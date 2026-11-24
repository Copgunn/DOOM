//! The saved state of a session across levels, `DSND` version 2 (M6a ruling U3 (i)), for
//! restore-and-continue at any session tic: in a level, in the intermission or in the finale.
//!
//! **Version 1 keeps its bytes and rules** ([`crate::state`]): [`SoundState::decode`] refuses
//! version 2 (`Version(2)`) and [`ChainSoundState::decode`] reads version 2 only (`Version(1)`
//! otherwise). What version 2 changes: the tic count is the session's `gametic`, not the level
//! simulation's; the map is state (it changes at every load) and not configuration; and the state
//! records where the session stands (the game state, the level's first session tic and, outside a
//! level, the session tic it was left) and whether a melt is in progress.
//!
//! Layout, little endian, exactly this many bytes (at most [`MAX_BYTES`]):
//!
//! | Offset | Size | Field |
//! | --- | --- | --- |
//! | 0 | 4 | magic `DSND` |
//! | 4 | 2 | version, 2 |
//! | 6 | 8 | session tics run (`gametic` after the last tic) |
//! | 14 | 8 | buffers mixed: `ceil(315 * tics / 512)`; with a melt in progress `ceil(315 * (tics - 1) / 512)` |
//! | 22 | 1 | `snd_SfxVolume` (0..=127) |
//! | 23 | 1 | `numChannels` N (0..=64) |
//! | 24 | 4 | texture count of the T12 detector (0..=65536) |
//! | 28 | 1 | game state: 0 level, 1 intermission, 2 finale |
//! | 29 | 4 | `gamemap` (i32, 1..=9; 8 in the finale): the level's map, or outside a level the finished level's |
//! | 33 | 8 | level start: the session tic of the level's tic 0 (below the tics run, or 0 with 0 tics run) |
//! | 41 | 8 | level end: 0 in a level; outside a level the session tic of its first tic, above the level start and below the tics run |
//! | 49 | 1 | melt: 0 none, 1 in progress (the logic and update of the last tic done, its buffer not yet mixed) |
//! | 50 | 1 | detector label: 0 none, 1 layout-dependent |
//! | 51 | 8 | the first tic it fired (0 without a label; below the tics run) |
//! | 59 | 8 | the number of firings (0 exactly without a label) |
//! | 67 | | the "lump looked up" flags, the N sound-logic channels, the mixer's `handlenums` and the 8 mixer channels, exactly as version 1 from its offset 49 |
//!
//! **Melt in progress** (M6a note §1.16.4): the reference build runs nothing of the sound code
//! during a melt step and mixes the buffer due after the melt's tic once the melt ends, so the
//! only sound state a melt adds is that buffer, still owed. Such a state decodes; restoring it is
//! [`NotYet::MeltInProgress`] until the melt steps exist, and no session writes one yet.
//!
//! Restoring ([`ChainSound::restore`]) checks the state against the caller's configuration and
//! music lumps (the bytes never choose them), the loaded sound set, and the level simulation the
//! session continues with (in a level) or holds read-only (outside one): its tic count and every
//! actor and sector origin. These are consistency checks, not a proof that a run reaches the state.

use std::sync::Arc;

use doom_sim::Simulation;

use crate::chain::{ChainConfig, ChainSound, ChainState, NotYet};
use crate::chain_melt::ChainMode;
use crate::detector::{LayoutDetector, MAX_TEXTURES};
use crate::logic::{ChannelEntry, MAX_CHANNELS, SoundOrigin};
use crate::lumps::SoundSet;
use crate::mixer::{MixChannelState, NUM_CHANNELS};
use crate::music::{Music, MusicLumps};
use crate::schedule::Schedule;
use crate::session::Sound;
use crate::sounds::NUMSFX;
use crate::state::{MAGIC, Reader, SoundState, StateError, Tail, read_tail, write_tail};

/// The version written and read here.
pub const VERSION: u16 = 2;
/// The fixed part's length.
const HEADER: usize = 81;
/// The largest valid encoding: the header, 64 occupied actor channels of 19 bytes, the counter
/// and 8 busy mixer channels of 28 bytes.
pub const MAX_BYTES: usize = HEADER + MAX_CHANNELS * 19 + 2 + NUM_CHANNELS * 28;

/// Why a version 2 state was refused. The target session is never changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainStateError {
    /// A check shared with version 1 (truncation, magic, version, the configuration bytes, the
    /// detector, the flags, the channels, the mixer, a busy channel before any tic, and on
    /// restore the sound set, a configuration mismatch, an origin missing from the simulation).
    State(StateError),
    /// A game state byte above 2.
    GameState(u8),
    /// A map outside 1..=9, or a finale on a map other than 8 (g_game.c:1033-1038).
    Map(i32),
    /// Level start and end inconsistent with the game state and the tics run.
    LevelTics,
    /// A melt byte other than 0 or 1, or a melt in progress before any tic.
    Melt(u8),
    /// Restore: the playing music's lump is missing from the caller's WADs (the genuine engine
    /// would have stopped when it was looked up).
    MusicMissing(Music),
    /// Restore: the level simulation has not run the state's level tics (`Simulation::tic`).
    TicMismatch {
        /// The level tics the state implies.
        state: u64,
        /// `Simulation::tic()`.
        simulation: u64,
    },
    /// Restore: not yet ([`NotYet::MeltInProgress`]).
    NotYet(NotYet),
    /// A state no session reaches, by the named rule (checked by version 2's reader only;
    /// version 1's reader is frozen): an actor origin of generation 0 (no actor has it), a
    /// labelled detector with no texture (it has no candidate to fire on), an occupied channel
    /// whose handle the last `S_UpdateSounds` would have freed (`gametic >= handle`,
    /// s_sound.c:560), or a channel playing a sound whose lump was never looked up.
    Unreachable(&'static str),
}

impl From<StateError> for ChainStateError {
    fn from(e: StateError) -> Self {
        Self::State(e)
    }
}

/// A session's saved state, version 2.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainSoundState {
    /// The configuration.
    pub config: ChainConfig,
    /// The session tics run (`gametic` after the last tic).
    pub tics: u64,
    /// The buffers mixed (see the layout: one fewer than the tics give when a melt is in progress
    /// and the last tic's buffer is due).
    pub buffers: u64,
    /// The game state.
    pub state: ChainState,
    /// `gamemap`.
    pub gamemap: i32,
    /// The session tic of the level's tic 0.
    pub level_start: u64,
    /// Outside a level, the session tic of its first tic; 0 in a level.
    pub level_end: u64,
    /// A melt in progress: the buffer due after the last tic is not yet mixed.
    pub melt: bool,
    /// The detector's label and count.
    pub detector: LayoutDetector,
    /// The "lump looked up" flags.
    pub looked_up: [bool; NUMSFX],
    /// The sound-logic channel table.
    pub channels: Vec<Option<ChannelEntry>>,
    /// The mixer's handle counter.
    pub handlenums: u16,
    /// The busy mixer channels.
    pub mixer: [Option<MixChannelState>; NUM_CHANNELS],
}

impl ChainSoundState {
    /// The `DSND` version 2 bytes. A state no session holds encodes to bytes that
    /// [`Self::decode`] refuses, never to another state (out-of-range values are written as the
    /// largest value of their field, which is out of range). The struct names the channel count
    /// twice (`config.channels` and the table's length) and the texture count twice
    /// (`config.textures` and the detector's); a hand-built state in which the two disagree is
    /// such a state, and its count is written as the largest value.
    pub fn encode(&self) -> Vec<u8> {
        self.encode_version(VERSION)
    }

    /// The bytes of version `version`: 2 here, 3 for [`crate::chain_melt::ChainSoundStateV3`]
    /// (the same layout).
    pub(crate) fn encode_version(&self, version: u16) -> Vec<u8> {
        let mut out = Vec::with_capacity(MAX_BYTES);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&version.to_le_bytes());
        out.extend_from_slice(&self.tics.to_le_bytes());
        out.extend_from_slice(&self.buffers.to_le_bytes());
        let volume = u8::try_from(self.config.sfx_volume)
            .ok()
            .filter(|v| *v <= 127);
        let channels = u8::try_from(self.channels.len())
            .ok()
            .filter(|n| usize::from(*n) <= MAX_CHANNELS && usize::from(*n) == self.config.channels);
        let textures = if self.detector.textures() == self.config.textures {
            self.config.textures
        } else {
            u32::MAX
        };
        out.push(volume.unwrap_or(u8::MAX));
        out.push(channels.unwrap_or(u8::MAX));
        out.extend_from_slice(&textures.to_le_bytes());
        out.push(self.state.byte());
        out.extend_from_slice(&self.gamemap.to_le_bytes());
        out.extend_from_slice(&self.level_start.to_le_bytes());
        out.extend_from_slice(&self.level_end.to_le_bytes());
        out.push(u8::from(self.melt));
        let first = self.detector.layout_dependent_from();
        out.push(u8::from(first.is_some()));
        out.extend_from_slice(&first.unwrap_or(0).to_le_bytes());
        out.extend_from_slice(&self.detector.firings().to_le_bytes());
        write_tail(
            &mut out,
            &self.looked_up,
            &self.channels,
            self.handlenums,
            &self.mixer,
        );
        out
    }

    /// Reads `DSND` version 2 bytes, checking everything that needs neither the sound set, the
    /// music lumps nor the simulation ([`ChainStateError`]). Never panics.
    pub fn decode(bytes: &[u8]) -> Result<Self, ChainStateError> {
        Self::decode_version(bytes, VERSION)
    }

    /// Reads bytes of version `want` only (2 here, 3 for
    /// [`crate::chain_melt::ChainSoundStateV3`]), with every rule of version 2; another version is
    /// [`StateError::Version`].
    pub(crate) fn decode_version(bytes: &[u8], want: u16) -> Result<Self, ChainStateError> {
        if bytes.len() > MAX_BYTES {
            return Err(StateError::TooLong.into());
        }
        let mut r = Reader { bytes, at: 0 };
        if r.take::<4>()? != MAGIC {
            return Err(StateError::Magic.into());
        }
        let version = r.u16()?;
        if version != want {
            return Err(StateError::Version(version).into());
        }
        let tic = r.u64()?;
        let buffers = r.u64()?;
        let sfx_volume = i32::from(r.u8()?);
        let channels = usize::from(r.u8()?);
        let textures = r.u32()?;
        if sfx_volume > 127 || channels > MAX_CHANNELS {
            return Err(StateError::Config.into());
        }
        let state = match r.u8()? {
            0 => ChainState::Level,
            1 => ChainState::Intermission,
            2 => ChainState::Finale,
            b => return Err(ChainStateError::GameState(b)),
        };
        let gamemap = r.i32()?;
        if !(1..=9).contains(&gamemap) || (state == ChainState::Finale && gamemap != 8) {
            return Err(ChainStateError::Map(gamemap));
        }
        let level_start = r.u64()?;
        let level_end = r.u64()?;
        let tics_ok = match state {
            ChainState::Level => level_end == 0 && (level_start < tic || level_start == 0),
            _ => level_start < level_end && level_end < tic,
        };
        if !tics_ok {
            return Err(ChainStateError::LevelTics);
        }
        let melt = match r.u8()? {
            0 => false,
            1 if tic > 0 => true,
            b => return Err(ChainStateError::Melt(b)),
        };
        // The schedule: the buffers of the tics run, or with a melt in progress those of the tics
        // before the last (its buffer still owed); the tics run must fit either way.
        Schedule::at(tic, Schedule::buffers_before(tic)).map_err(StateError::Schedule)?;
        let ran = if melt { tic - 1 } else { tic };
        Schedule::at(ran, buffers).map_err(StateError::Schedule)?;
        let labelled = r.tag().map_err(|_| StateError::Detector)?;
        let first = r.u64()?;
        let firings = r.u64()?;
        if textures > MAX_TEXTURES || (!labelled && first != 0) || (labelled && first >= tic) {
            return Err(StateError::Detector.into());
        }
        let detector = LayoutDetector::from_parts(textures, labelled.then_some(first), firings)
            .map_err(|_| StateError::Detector)?;
        let Tail {
            looked_up,
            channels: table,
            handlenums,
            mixer,
        } = read_tail(&mut r, channels, tic)?;
        if r.at != bytes.len() {
            return Err(StateError::Trailing.into());
        }
        if tic == 0 && (table.iter().any(Option::is_some) || mixer.iter().any(Option::is_some)) {
            return Err(StateError::BusyAtStart.into());
        }
        // Version 2 only (version 1's reader is frozen): states no session reaches.
        if labelled && textures == 0 {
            return Err(ChainStateError::Unreachable(
                "a labelled detector with no texture",
            ));
        }
        for e in table.iter().flatten() {
            if matches!(e.origin, SoundOrigin::Actor(a) if a.generation() == 0) {
                return Err(ChainStateError::Unreachable(
                    "an actor origin of generation 0",
                ));
            }
            // The last tic's `S_UpdateSounds` ran with `gametic` = the tics run and freed every
            // channel with `gametic >= handle` (s_sound.c:560, i_sound.c:517-521).
            if u64::try_from(e.handle).map_or(true, |h| h <= tic) {
                return Err(ChainStateError::Unreachable(
                    "a channel with an expired handle",
                ));
            }
            // `S_StartSound` sets the flag of every sound it starts (s_sound.c:363-365).
            if !looked_up[e.sfx.index()] {
                return Err(ChainStateError::Unreachable(
                    "a channel whose sound was never looked up",
                ));
            }
        }
        if mixer.iter().flatten().any(|m| !looked_up[m.sfx.index()]) {
            return Err(ChainStateError::Unreachable(
                "a mixer channel whose sound was never looked up",
            ));
        }
        Ok(Self {
            config: ChainConfig {
                sfx_volume,
                channels,
                textures,
            },
            tics: tic,
            buffers,
            state,
            gamemap,
            level_start,
            level_end,
            melt,
            detector,
            looked_up,
            channels: table,
            handlenums,
            mixer,
        })
    }

    /// The music this state has playing (see [`ChainSound::music_playing`]).
    pub fn music_playing(&self) -> Music {
        match self.state {
            ChainState::Level => Music::Level(self.gamemap as u8),
            ChainState::Intermission => Music::Inter,
            ChainState::Finale => Music::Victor,
        }
    }

    /// The tics the level simulation has run in this state: in a level the session tics since
    /// its tic 0, outside a level those it ran before it was left.
    pub fn level_tics(&self) -> u64 {
        match self.state {
            ChainState::Level => self.tics - self.level_start,
            _ => self.level_end - self.level_start,
        }
    }
}

impl ChainSound {
    /// The session's saved state. A session of [`crate::chain_melt::ChainMode::FinaleAndMelt`]
    /// saves as version 3 ([`Self::save_v3`]); called on one, this gives the same contents (a
    /// pending display as a melt in progress, its owed buffer not counted), which restore as
    /// version 2 rules say.
    pub fn save(&self) -> ChainSoundState {
        let s: SoundState = self.sound.state();
        ChainSoundState {
            config: self.config,
            tics: s.schedule.tic(),
            buffers: s.schedule.buffers() - u64::from(self.owed),
            state: self.state,
            gamemap: self.gamemap(),
            level_start: self.level_start,
            level_end: self.level_end,
            melt: self.pending,
            detector: s.detector,
            looked_up: s.looked_up,
            channels: s.channels,
            handlenums: s.handlenums,
            mixer: s.mixer,
        }
    }

    /// A session rebuilt from a saved state over a loaded sound set, checked against the caller's
    /// configuration (`expected`, [`StateError::ConfigMismatch`] otherwise) and music lumps (the
    /// playing music's lump must be present), the set (as [`Sound::from_state`] checks it) and
    /// `sim`: in a level, the level's simulation, which must have run the state's level tics;
    /// outside a level, the finished level's, which must have run the tics it ran before it was
    /// left. Every actor origin must resolve in it (live or marked) and every sector origin be one
    /// of its sectors. A melt in progress is [`NotYet::MeltInProgress`].
    pub fn restore(
        set: Arc<SoundSet>,
        music: MusicLumps,
        state: &ChainSoundState,
        sim: &Simulation,
        expected: ChainConfig,
    ) -> Result<Self, ChainStateError> {
        if state.melt {
            return Err(ChainStateError::NotYet(NotYet::MeltInProgress));
        }
        Self::restore_mode(set, music, state, sim, expected, ChainMode::RefusesMelt)
    }

    /// [`Self::restore`]'s checks and rebuild in configuration `mode`; `state.melt` is a pending
    /// display (only [`ChainMode::FinaleAndMelt`] reaches here with one).
    pub(crate) fn restore_mode(
        set: Arc<SoundSet>,
        music: MusicLumps,
        state: &ChainSoundState,
        sim: &Simulation,
        expected: ChainConfig,
        mode: ChainMode,
    ) -> Result<Self, ChainStateError> {
        if state.config != expected {
            return Err(StateError::ConfigMismatch {
                saved: state.config.sound(state.gamemap),
                expected: expected.sound(state.gamemap),
            }
            .into());
        }
        let playing = state.music_playing();
        if !music.present(playing) {
            return Err(ChainStateError::MusicMissing(playing));
        }
        if sim.tic() != state.level_tics() {
            return Err(ChainStateError::TicMismatch {
                state: state.level_tics(),
                simulation: sim.tic(),
            });
        }
        for (cnum, e) in state.channels.iter().enumerate() {
            match e.map(|e| e.origin) {
                Some(SoundOrigin::Actor(a)) if sim.sound_origin_xy(a).is_none() => {
                    return Err(StateError::ActorGone { cnum }.into());
                }
                Some(SoundOrigin::Sector(s)) if sim.sector_sound_origin(s).is_none() => {
                    return Err(StateError::Sector { cnum }.into());
                }
                _ => {}
            }
        }
        // The buffers mixed are those of the tics run, or with a pending display those of the tics
        // before the last; the last tic's buffer, if one was due, is then owed and the session's
        // schedule counts it (as a tic of `ChainMode::FinaleAndMelt` leaves it).
        let ran = if state.melt {
            state.tics.checked_sub(1).ok_or(ChainStateError::Melt(1))?
        } else {
            state.tics
        };
        Schedule::at(ran, state.buffers).map_err(StateError::Schedule)?;
        let schedule = Schedule::at(state.tics, Schedule::buffers_before(state.tics))
            .map_err(StateError::Schedule)?;
        let owed = schedule.buffers() != state.buffers;
        let v1 = SoundState {
            config: state.config.sound(state.gamemap),
            schedule,
            detector: state.detector,
            looked_up: state.looked_up,
            channels: state.channels.clone(),
            handlenums: state.handlenums,
            mixer: state.mixer,
        };
        let sound = Sound::from_state(set, &v1, v1.config)?;
        Ok(Self {
            config: state.config,
            music,
            sound,
            state: state.state,
            level_start: state.level_start,
            level_end: state.level_end,
            mode,
            pending: state.melt,
            owed,
            updates: state.tics,
            fault: None,
        })
    }
}
