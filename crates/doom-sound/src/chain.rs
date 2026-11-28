//! One sound session across the levels of a single-player Episode 1 session (M6a ruling U3 (i);
//! M6a note §1.6, §1.7, §1.16.4 and §4.6): the M5c session ([`crate::session::Sound`]) carried
//! from level to level, through the intermission and into the finale, with `gametic` continuous.
//!
//! Per session tic, in the C order (g_game.c:605-748, then d_main.c:392-405), one of:
//!
//! - **a level tic** ([`ChainSound::level_tic`]): with a load (`G_DoWorldDone` after an
//!   intermission, g_game.c:1172-1179, or the single-player reborn, `G_DoReborn` → `G_DoLoadLevel`,
//!   g_game.c:924-932), first `S_Start` (p_setup.c:607): every occupied channel stopped
//!   (s_sound.c:209-211) and the level's music looked up (s_sound.c:235-245; [`crate::music`]);
//!   then the map number of the new level (g_game.c:1176); then the load-time starts, made before
//!   tic 0 (only the carried chainsaw's `sfx_sawup`: `P_SpawnPlayer` → `P_SetupPsprites` →
//!   `P_BringUpWeapon`, p_mobj.c:686, p_pspr.c:831-842, 145-146); then, with or without a load,
//!   the tic's records (`P_Ticker`, g_game.c:729-733);
//! - **an intermission tic** ([`ChainSound::intermission_tic`]): on the first, which is the
//!   `G_Ticker` that ran `G_DoCompleted` (g_game.c:1020-1141, 736-738), the intermission music is
//!   looked up (`WI_Ticker` at `bcnt == 1`, wi_stuff.c:1508-1515); then the tic's starts, each
//!   with no origin (`sfx_pistol`, `sfx_barexp`, `sfx_sgcock`, `WI_updateStats`,
//!   wi_stuff.c:1330-1434), requested by the game layer;
//! - **a finale tic** ([`ChainSound::finale_tic`]): on the first (`F_StartFinale` in the
//!   `G_Ticker` of `G_DoCompleted` on map 8, g_game.c:1033-1038, 639-641) the finale music is
//!   looked up (f_finale.c:114); the Doom 1 finale starts no sound;
//!
//! and after each, `S_UpdateSounds` with `gametic` + 1 (d_main.c:392) and the buffer due under the
//! declared schedule ([`crate::schedule`]). **Outside a level the listener and the origins are the
//! finished level's**: nothing stops the level's channels at completion (only the next `S_Start`
//! does), `S_UpdateSounds(players[consoleplayer].mo)` keeps running every frame, and the old mobjs
//! stay valid memory until `Z_FreeTags` at the next load (p_setup.c:618), so the level's sounds
//! play on into the intermission and the finale, heard from where the player stood (U8 quirk 9).
//! The game layer passes the finished level's simulation, read-only, for those tics.
//!
//! **The melt** (note §1.16.4 and the owner's ruling of 2026-11-21): no game tic, no
//! `S_UpdateSounds` and no buffer during melt steps; the buffer due after the tic whose display
//! melts is mixed after the melt, nothing skipped. The sequence of mixed buffers does not depend
//! on the melt. Until the melt pictures exist, a session runs the M5c schedule (a melt takes no
//! time, decision T6 as extended in step 5) and [`ChainSound::melt_step`] is a typed "not yet"
//! refusal; the saved state can represent a melt in progress ([`crate::chain_state`]).
//! (M6a part 2: that is the frozen configuration, [`ChainMode::RefusesMelt`]; the configuration
//! that runs the melt steps is [`ChainMode::FinaleAndMelt`], [`crate::chain_melt`].)
//!
//! Every tic is transactional: it runs on a copy of the session (a few hundred bytes; no
//! simulation is cloned) and a refusal leaves everything, the caller's random index included,
//! as it was. Every refusal the genuine engine has (a missing music lump) is made at the tic at
//! which the genuine engine stops.

use std::sync::Arc;

use crate::chain_melt::{ChainFault, ChainMode};
use crate::chain_pause::{Pause, PauseTic};
use crate::logic::{ChannelStop, Listener, OriginPositions, SoundOrigin, SoundRecord};
use crate::lumps::SoundSet;
use crate::music::{Music, MusicLumps};
use crate::schedule::Schedule;
use crate::session::{SessionRefusal, Sound, SoundConfig, TicSound};
use crate::sounds::{SFX_PISTOL, SFX_SAWUP};

/// `sfx_sgcock` (sounds.h:182).
const SFX_SGCOCK: u16 = 3;
/// `sfx_barexp` (sounds.h:261).
const SFX_BAREXP: u16 = 82;

/// The game state of the session (`gamestate_t`, doomdef.h:127-133; the demo screen is not part
/// of a session).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ChainState {
    /// `GS_LEVEL`.
    Level,
    /// `GS_INTERMISSION`.
    Intermission,
    /// `GS_FINALE`.
    Finale,
}

impl ChainState {
    /// The byte of the saved state: 0, 1, 2.
    pub fn byte(self) -> u8 {
        match self {
            Self::Level => 0,
            Self::Intermission => 1,
            Self::Finale => 2,
        }
    }
}

/// A start of the single-player intermission, always with no origin (`S_StartSound(0, ...)`,
/// wi_stuff.c:1330-1434). The deathmatch and network sounds (`sfx_slop`, `sfx_pldeth`) are not
/// reachable in a single-player session and cannot be requested.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum IntermissionSound {
    /// `sfx_pistol`: every fourth tic while a count runs (wi_stuff.c:1352-1393).
    Pistol,
    /// `sfx_barexp`: a count reaching its end, or a press that jumps every count (wi_stuff.c:1343-1408).
    Barexp,
    /// `sfx_sgcock`: the press in state 10 (wi_stuff.c:1413-1424).
    Sgcock,
}

impl IntermissionSound {
    /// The sound number.
    pub fn sfx(self) -> u16 {
        match self {
            Self::Pistol => SFX_PISTOL,
            Self::Barexp => SFX_BAREXP,
            Self::Sgcock => SFX_SGCOCK,
        }
    }
}

/// What a session cannot do yet (each is original behaviour; none is optional).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum NotYet {
    /// A melt step: comes with the melt pictures, after the owner's hand test of the level chain.
    MeltSteps,
    /// A saved state with a melt in progress: as [`NotYet::MeltSteps`].
    MeltInProgress,
}

/// Which call a [`ChainRefusal::WrongState`] refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ChainCall {
    /// [`ChainSound::level_tic`] without a load.
    Level,
    /// [`ChainSound::level_tic`] with a load.
    Load,
    /// [`ChainSound::intermission_tic`].
    Intermission,
    /// [`ChainSound::finale_tic`].
    Finale,
}

/// Why a session refused a tic or a construction. The session is unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainRefusal {
    /// The M5c session refused the tic (a sound, the update, the tic limit, the draw count).
    Session(SessionRefusal),
    /// The genuine engine stops here: `W_GetNumForName` finds no music lump (s_sound.c:671-676;
    /// `I_Error`, w_wad.c:399-409). `tic` is the session tic, or `None` at the session's load
    /// (before tic 0).
    MusicMissing {
        /// The music looked up.
        music: Music,
        /// The session tic.
        tic: Option<u64>,
    },
    /// A call the game state does not allow: a level tic without a load outside a level, a load
    /// from the finale (the Doom 1 finale never ends, f_finale.c:207-249), an intermission tic
    /// from the finale or a finale tic from the intermission (`ga_victory` comes from a level
    /// only, g_game.c:1033-1038).
    WrongState {
        /// The state.
        state: ChainState,
        /// The call.
        call: ChainCall,
    },
    /// A map number outside 1..=9 (episode 1).
    Map {
        /// The number given.
        map: i32,
    },
    /// A load from a level is the single-player reborn, which reloads the current map
    /// (`G_DoReborn` → `ga_loadlevel`, g_game.c:924-932).
    ReloadMap {
        /// The current map.
        current: i32,
        /// The map given.
        requested: i32,
    },
    /// The finale is entered from map 8 only (g_game.c:1033-1038).
    FinaleMap {
        /// The current map.
        map: i32,
    },
    /// A load-time record other than the one the C can make there (the carried chainsaw's
    /// `sfx_sawup` from the new body, with no placement, p_pspr.c:145-146), or more than one.
    LoadRecord {
        /// The record's index.
        index: usize,
    },
    /// The configuration: `snd_SfxVolume` outside 0..=127, more than 64 channels or too many
    /// textures.
    Config,
    /// Not yet ([`NotYet`]).
    NotYet(NotYet),
    /// [`ChainMode::FinaleAndMelt`]: a tic called while the previous tic's display is still
    /// pending (its melt steps, if any, and [`ChainSound::mix_after_display`] not yet done): the C
    /// mixes the buffer of a pass before the next pass runs its tics (d_main.c:374-405).
    DisplayPending {
        /// The call refused.
        call: ChainCall,
    },
    /// A melt step or [`ChainSound::mix_after_display`] with no tic's display pending: before
    /// the first tic, after the display's mix, or in [`ChainMode::RefusesMelt`] (which mixes in
    /// the tic) for the mix. A melt runs only inside the display of a tic (d_main.c:223-229,
    /// 320-344, 395).
    NoDisplayPending,
    /// [`ChainSound::pause_tic`] on a session that is not a pause session
    /// ([`ChainSound::new_pause`], `crate::chain_pause`).
    NotPause,
}

impl std::fmt::Display for ChainRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Session(s) => write!(f, "{s}"),
            Self::MusicMissing { music, tic: None } => {
                write!(
                    f,
                    "W_GetNumForName: {} not found! (at load)",
                    music.lump_name()
                )
            }
            Self::MusicMissing {
                music,
                tic: Some(t),
            } => write!(
                f,
                "W_GetNumForName: {} not found! (tic {t})",
                music.lump_name()
            ),
            Self::WrongState { state, call } => {
                write!(f, "the sound session cannot run {call:?} in {state:?}")
            }
            Self::Map { map } => write!(f, "map {map} is not a map of episode 1"),
            Self::ReloadMap { current, requested } => {
                write!(f, "a reborn reloads map {current}, not map {requested}")
            }
            Self::FinaleMap { map } => write!(f, "the finale follows map 8, not map {map}"),
            Self::LoadRecord { index } => {
                write!(f, "load-time sound {index} is not the carried chainsaw's")
            }
            Self::Config => write!(f, "the sound configuration is out of range"),
            Self::NotYet(n) => write!(f, "not yet: {n:?}"),
            Self::DisplayPending { call } => write!(
                f,
                "the sound session cannot run {call:?} before the previous tic's display is mixed"
            ),
            Self::NoDisplayPending => write!(f, "no tic's display is pending in the sound session"),
            Self::NotPause => write!(f, "the sound session has no pause"),
        }
    }
}

impl std::error::Error for ChainRefusal {}

impl From<SessionRefusal> for ChainRefusal {
    fn from(s: SessionRefusal) -> Self {
        Self::Session(s)
    }
}

/// A session's configuration: what a fresh process has and the loaded WADs give. The map is
/// session state, not configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ChainConfig {
    /// `snd_SfxVolume`: 8 in a fresh process (m_misc.c:237; M5c decision T9).
    pub sfx_volume: i32,
    /// `numChannels`: 3 in a fresh process (m_misc.c:282; M5c decision T9).
    pub channels: usize,
    /// The texture count of the loaded WADs, for the layout detector of M5c decision T12.
    pub textures: u32,
}

impl ChainConfig {
    /// A fresh process (volume 8, 3 channels) with `textures` textures loaded.
    pub fn fresh(textures: u32) -> Self {
        let f = SoundConfig::fresh(1, textures);
        Self {
            sfx_volume: f.logic.sfx_volume,
            channels: f.logic.channels,
            textures,
        }
    }

    /// The M5c configuration on map `gamemap`.
    pub(crate) fn sound(self, gamemap: i32) -> SoundConfig {
        let mut c = SoundConfig::fresh(gamemap, self.textures);
        c.logic.sfx_volume = self.sfx_volume;
        c.logic.channels = self.channels;
        c
    }
}

/// A level load in a [`ChainSound::level_tic`].
#[derive(Clone, Copy, Debug)]
pub struct LevelLoad<'a> {
    /// The map loaded (1..=9): `wminfo.next + 1` after an intermission, the current map for a reborn.
    pub map: i32,
    /// The sound records made during the load, before tic 0: at most the carried chainsaw's
    /// `sfx_sawup` from the new body (the listener), with no placement.
    pub records: &'a [SoundRecord],
}

/// What one session tic did.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainTic {
    /// The M5c tic report: `records` holds the load-time records' echoes first, then the tic's.
    pub sound: TicSound,
    /// For a load, the channels `S_Start` stopped (s_sound.c:209-211), in channel order.
    pub level_start: Vec<ChannelStop>,
    /// For a load, the number of load-time records at the front of `sound.records`.
    pub load_records: usize,
    /// The music looked up in this tic, if any (and found).
    pub music: Option<Music>,
}

/// One session's sound across levels.
#[derive(Clone, Debug, PartialEq)]
pub struct ChainSound {
    pub(crate) config: ChainConfig,
    pub(crate) music: MusicLumps,
    /// The M5c session; its configuration's map is the current `gamemap`.
    pub(crate) sound: Sound,
    pub(crate) state: ChainState,
    /// The session tic of the current (or finished) level's tic 0.
    pub(crate) level_start: u64,
    /// Outside a level, the session tic of the first tic outside it; 0 in a level.
    pub(crate) level_end: u64,
    /// The configuration (M6a part 2, ruling V1).
    pub(crate) mode: ChainMode,
    /// [`ChainMode::FinaleAndMelt`]: the last tic's display is pending (its buffer not yet mixed;
    /// a melt may be in progress). Always false in [`ChainMode::RefusesMelt`].
    pub(crate) pending: bool,
    /// While `pending`: the last tic had a buffer due, still owed (the schedule counts it).
    pub(crate) owed: bool,
    /// The `S_UpdateSounds` calls made (one per tic, d_main.c:392); the genuine runner counts
    /// them (`updates` in its Z row). Not saved: a reachable session has made one per tic run.
    pub(crate) updates: u64,
    /// A fault planted by the evidence self-tests ([`ChainFault`]); never set by the game.
    pub(crate) fault: Option<ChainFault>,
    /// A pause session's pause part (M6b wire 29, `crate::chain_pause`); None otherwise.
    pub(crate) pause: Option<Pause>,
}

impl ChainSound {
    /// A new game on map `map` (`G_InitNew` → `G_DoLoadLevel` → `P_SetupLevel` → `S_Start`,
    /// g_game.c:1412-1484, p_setup.c:607): `gametic` 0, every channel free, no buffer mixed, and
    /// the map's music looked up: a missing lump is the genuine engine's stop at its first load,
    /// before any tic ([`ChainRefusal::MusicMissing`] with no tic).
    pub fn new(
        set: Arc<SoundSet>,
        music: MusicLumps,
        config: ChainConfig,
        map: i32,
    ) -> Result<Self, ChainRefusal> {
        let level = Music::level(map).ok_or(ChainRefusal::Map { map })?;
        let sound = Sound::new(set, config.sound(map)).map_err(|_| ChainRefusal::Config)?;
        if !music.present(level) {
            return Err(ChainRefusal::MusicMissing {
                music: level,
                tic: None,
            });
        }
        Ok(Self {
            config,
            music,
            sound,
            state: ChainState::Level,
            level_start: 0,
            level_end: 0,
            mode: ChainMode::RefusesMelt,
            pending: false,
            owed: false,
            updates: 0,
            fault: None,
            pause: None,
        })
    }

    /// The configuration.
    pub fn config(&self) -> ChainConfig {
        self.config
    }

    /// The music lumps the session checks against.
    pub fn music_lumps(&self) -> MusicLumps {
        self.music
    }

    /// The M5c session inside: its logic, mixer, schedule and detector (read only).
    pub fn sound(&self) -> &Sound {
        &self.sound
    }

    /// The game state.
    pub fn state(&self) -> ChainState {
        self.state
    }

    /// `gamemap`: the current level's map, or outside a level the finished level's.
    pub fn gamemap(&self) -> i32 {
        self.sound.config.logic.gamemap
    }

    /// The schedule: the next session tic (`gametic`) and the buffers mixed. In
    /// [`ChainMode::FinaleAndMelt`], while a display is pending, it counts the last tic's due
    /// buffer although that buffer is still owed ([`Self::buffer_owed`], [`Self::buffers_mixed`]).
    pub fn schedule(&self) -> Schedule {
        self.sound.schedule
    }

    /// The session tic of the current (or finished) level's tic 0.
    pub fn level_start(&self) -> u64 {
        self.level_start
    }

    /// Outside a level, the session tic of its first tic (the completion tic); in a level, `None`.
    pub fn level_end(&self) -> Option<u64> {
        (self.state != ChainState::Level).then_some(self.level_end)
    }

    /// The music playing (`mus_playing`), which follows from the state: the level's music in a
    /// level, `D_INTER` in the intermission (changed on its first tic, wi_stuff.c:1512-1514),
    /// `D_VICTOR` in the finale (f_finale.c:114).
    pub fn music_playing(&self) -> Music {
        match self.state {
            ChainState::Level => Music::Level(self.gamemap() as u8),
            ChainState::Intermission => Music::Inter,
            ChainState::Finale => Music::Victor,
        }
    }

    /// The music lookup of `S_ChangeMusic(target)` (s_sound.c:665-676) at session tic `tic`: none
    /// when `target` is playing; otherwise a refusal when its lump is missing. Returns the music
    /// looked up.
    fn change_music(&self, target: Music, tic: u64) -> Result<Option<Music>, ChainRefusal> {
        if target == self.music_playing() {
            return Ok(None);
        }
        if !self.music.present(target) {
            return Err(ChainRefusal::MusicMissing {
                music: target,
                tic: Some(tic),
            });
        }
        Ok(Some(target))
    }

    /// [`ChainRefusal::DisplayPending`] while a display is pending (never in
    /// [`ChainMode::RefusesMelt`]).
    fn check_no_display(&self, call: ChainCall) -> Result<(), ChainRefusal> {
        if self.pending {
            return Err(ChainRefusal::DisplayPending { call });
        }
        Ok(())
    }

    /// Runs `sound`'s tic as the configuration does: [`ChainMode::RefusesMelt`] mixes the due
    /// buffer in the tic; [`ChainMode::FinaleAndMelt`] leaves it owed until the display is done
    /// ([`Self::mix_after_display`]).
    fn run_tic(
        &self,
        sound: &mut Sound,
        records: &[SoundRecord],
        rndindex: u8,
        listener: &Listener,
        positions: &dyn OriginPositions,
    ) -> Result<(TicSound, bool), ChainRefusal> {
        let mixes = self.mode == ChainMode::RefusesMelt;
        let owed = !mixes && sound.schedule.due() == 1;
        let t = sound.tic_mixing(records, rndindex, listener, positions, mixes)?;
        Ok((t, owed))
    }

    /// Commits a tic's sound session: the display of the tic becomes pending under
    /// [`ChainMode::FinaleAndMelt`], and one more `S_UpdateSounds` was made.
    fn commit(&mut self, sound: Sound, owed: bool) {
        self.sound = sound;
        self.pending = self.mode == ChainMode::FinaleAndMelt;
        self.owed = owed;
        self.updates += 1;
    }

    /// The next session tic, or the tic limit.
    fn next_tic(&self) -> Result<u64, ChainRefusal> {
        self.sound
            .schedule
            .gametics()
            .map(|(g, _)| g as u64)
            .map_err(|_| ChainRefusal::Session(SessionRefusal::TicLimit))
    }

    /// A level tic (see the module documentation): with `load`, `S_Start`, the music, the new map
    /// and the load-time records first. `records` are the tic's records (`P_Ticker`), and
    /// `listener` and `positions` are read after the tic from the level's simulation.
    pub fn level_tic(
        &mut self,
        load: Option<LevelLoad<'_>>,
        records: &[SoundRecord],
        rndindex: u8,
        listener: &Listener,
        positions: &dyn OriginPositions,
    ) -> Result<ChainTic, ChainRefusal> {
        let call = if load.is_some() {
            ChainCall::Load
        } else {
            ChainCall::Level
        };
        let wrong = || ChainRefusal::WrongState {
            state: self.state,
            call,
        };
        match (self.state, load.is_some()) {
            (ChainState::Level, _) | (ChainState::Intermission, true) => {}
            _ => return Err(wrong()),
        }
        self.check_no_display(call)?;
        let tic = self.next_tic()?;
        let Some(load) = load else {
            let mut sound = self.sound.clone();
            let (t, owed) = self.run_tic(&mut sound, records, rndindex, listener, positions)?;
            self.commit(sound, owed);
            self.commit_pause(PauseTic::Level { load: false });
            return Ok(ChainTic {
                sound: t,
                level_start: Vec::new(),
                load_records: 0,
                music: None,
            });
        };
        let level = Music::level(load.map).ok_or(ChainRefusal::Map { map: load.map })?;
        if self.state == ChainState::Level && load.map != self.gamemap() {
            return Err(ChainRefusal::ReloadMap {
                current: self.gamemap(),
                requested: load.map,
            });
        }
        let music = self.change_music(level, tic)?;
        if load.records.len() > 1 {
            return Err(ChainRefusal::LoadRecord { index: 1 });
        }
        for (index, r) in load.records.iter().enumerate() {
            let carried_chainsaw = matches!(
                *r,
                SoundRecord::Start { sfx: SFX_SAWUP, origin: SoundOrigin::Actor(a), at: None }
                    if a == listener.actor
            );
            if !carried_chainsaw {
                return Err(ChainRefusal::LoadRecord { index });
            }
        }
        let mut sound = self.sound.clone();
        // S_Start (s_sound.c:209-211) with gametic == tic, then gamemap (g_game.c:1176).
        let stops = sound.logic.level_start(tic as i32);
        sound.config.logic.gamemap = load.map;
        sound.logic.set_gamemap(load.map);
        let mut all = Vec::with_capacity(load.records.len() + records.len());
        all.extend_from_slice(load.records);
        all.extend_from_slice(records);
        let (t, owed) = self.run_tic(&mut sound, &all, rndindex, listener, positions)?;
        self.commit(sound, owed);
        self.commit_pause(PauseTic::Level { load: true });
        self.state = ChainState::Level;
        self.level_start = tic;
        self.level_end = 0;
        Ok(ChainTic {
            sound: t,
            level_start: stops,
            load_records: load.records.len(),
            music,
        })
    }

    /// An intermission tic (see the module documentation). From a level it is the first
    /// (`G_DoCompleted`'s `G_Ticker`): the intermission music is looked up before the starts.
    /// `starts` are the tic's starts in call order; `listener` and `positions` are the finished
    /// level's (its simulation after its last tic).
    pub fn intermission_tic(
        &mut self,
        starts: &[IntermissionSound],
        rndindex: u8,
        listener: &Listener,
        positions: &dyn OriginPositions,
    ) -> Result<ChainTic, ChainRefusal> {
        if self.state == ChainState::Finale {
            return Err(ChainRefusal::WrongState {
                state: self.state,
                call: ChainCall::Intermission,
            });
        }
        self.check_no_display(ChainCall::Intermission)?;
        let tic = self.next_tic()?;
        let entering = self.state == ChainState::Level;
        let music = if entering {
            self.change_music(Music::Inter, tic)?
        } else {
            None
        };
        let records: Vec<SoundRecord> = starts
            .iter()
            .map(|s| SoundRecord::Start {
                sfx: s.sfx(),
                origin: SoundOrigin::None,
                at: None,
            })
            .collect();
        let mut sound = self.sound.clone();
        let (t, owed) = self.run_tic(&mut sound, &records, rndindex, listener, positions)?;
        self.commit(sound, owed);
        self.commit_pause(if entering {
            PauseTic::EnterIntermission {
                music: music.is_some(),
            }
        } else {
            PauseTic::Other
        });
        if entering {
            self.state = ChainState::Intermission;
            self.level_end = tic;
        }
        Ok(ChainTic {
            sound: t,
            level_start: Vec::new(),
            load_records: 0,
            music,
        })
    }

    /// A finale tic (see the module documentation). From a level it is the first, which needs map 8
    /// and looks the finale music up. `listener` and `positions` are the finished level's.
    pub fn finale_tic(
        &mut self,
        rndindex: u8,
        listener: &Listener,
        positions: &dyn OriginPositions,
    ) -> Result<ChainTic, ChainRefusal> {
        if self.state == ChainState::Intermission {
            return Err(ChainRefusal::WrongState {
                state: self.state,
                call: ChainCall::Finale,
            });
        }
        self.check_no_display(ChainCall::Finale)?;
        let tic = self.next_tic()?;
        let entering = self.state == ChainState::Level;
        if entering && self.gamemap() != 8 {
            return Err(ChainRefusal::FinaleMap {
                map: self.gamemap(),
            });
        }
        // The planted fault `VictorLate` moves the lookup to the finale's second tic.
        let late = self.fault == Some(ChainFault::VictorLate);
        let looks_up = if late {
            !entering && self.level_end.checked_add(1) == Some(tic)
        } else {
            entering
        };
        let music = match (looks_up, late) {
            (false, _) => None,
            (true, false) => self.change_music(Music::Victor, tic)?,
            (true, true) if !self.music.present(Music::Victor) => {
                return Err(ChainRefusal::MusicMissing {
                    music: Music::Victor,
                    tic: Some(tic),
                });
            }
            (true, true) => Some(Music::Victor),
        };
        let mut sound = self.sound.clone();
        let (t, owed) = self.run_tic(&mut sound, &[], rndindex, listener, positions)?;
        self.commit(sound, owed);
        self.commit_pause(if entering {
            PauseTic::EnterFinale {
                music: music.is_some(),
            }
        } else {
            PauseTic::Other
        });
        if entering {
            self.state = ChainState::Finale;
            self.level_end = tic;
        }
        Ok(ChainTic {
            sound: t,
            level_start: Vec::new(),
            load_records: 0,
            music,
        })
    }

    /// A melt step (note §1.16.4): in the reference build nothing of the sound code runs during a
    /// melt step, and the buffer due after the melt's tic is mixed after its last step. In
    /// [`ChainMode::RefusesMelt`] (part 1): always [`NotYet::MeltSteps`], with nothing changed,
    /// never a silent no-op. In [`ChainMode::FinaleAndMelt`] it is accepted while a tic's display
    /// is pending and changes nothing ([`crate::chain_melt`]); otherwise
    /// [`ChainRefusal::NoDisplayPending`].
    pub fn melt_step(&mut self) -> Result<(), ChainRefusal> {
        match self.mode {
            ChainMode::RefusesMelt => Err(ChainRefusal::NotYet(NotYet::MeltSteps)),
            ChainMode::FinaleAndMelt => self.melt_step_finale_and_melt(),
        }
    }
}
