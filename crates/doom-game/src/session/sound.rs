//! The session's sound across levels: **the one integration point with doom-sound's `DSND`
//! version 2** (M6a step 6, being built at the same time as this module).
//!
//! Until that session type is merged, [`SessionSound`] runs the M5c session
//! ([`doom_sound::session::Sound`], unchanged) across the levels with what its public interface
//! allows, in the C's order (M6a note §1.1, §1.6, §4.6):
//!
//! - **a load** (`G_DoWorldDone` or `G_DoReborn`'s `G_DoLoadLevel` → `P_SetupLevel` → `S_Start`,
//!   p_setup.c:607): every occupied channel is freed (s_sound.c:209-211; `I_StopSound` is empty in
//!   linuxdoom, i_sound.c:505-514, so the mixer plays on) and `gamemap` becomes the new map (the
//!   map-8 rule of `S_AdjustSoundParams`, s_sound.c:773-777, 800-808, reads it); the level's music
//!   is looked up unless it is already playing (s_sound.c:235-245, 649-686): a missing lump is the
//!   genuine engine's stop at that tic. The load-time record (a carried chainsaw's `sfx_sawup`,
//!   p_pspr.c:145-146) is the head of tic 0's records (wire 26), so it runs first;
//! - **a level tic**: the simulation's records, then `S_UpdateSounds` and the due buffer, as M5c;
//! - **an intermission tic**: on the first, `S_ChangeMusic(mus_inter)` (wi_stuff.c:1508-1515);
//!   the tic's starts with no origin, and the update with the listener and the origins of the
//!   finished level, which stays valid memory until the next load's `Z_FreeTags` (p_setup.c:618).
//!
//! What waits for `DSND` version 2 (the next agent's integration, about one module): the saved
//! state. This interim keeps it as the M5c state's bytes ([`SOUND_SLOT_INTERIM`]), which is not a
//! `DSND` version 2 encoding; replacing the inside of this type with doom-sound's chain session
//! and writing [`SOUND_SLOT_DSND2`] is the remaining step, and no caller changes.
use doom_render::LumpDirectory;
use doom_sim::Simulation;
use doom_sound::logic::{SoundOrigin, SoundRecord};
use doom_sound::lumps::SoundSet;
use doom_sound::session::{
    SessionRefusal, SimulationPositions, Sound, SoundConfig, TicSound, simulation_listener,
};
use doom_sound::state::{SoundState, StateError};
use std::fmt;
use std::sync::Arc;

/// The `DSES` v1 sound slot kind of this interim: the M5c session's state bytes (`DSND`
/// version 1's layout), its map the session's current `gamemap` and its tic count `gametic`.
/// Written only until `DSND` version 2 is merged (see the module documentation).
pub const SOUND_SLOT_INTERIM: u8 = 1;
/// The `DSES` v1 sound slot kind of `DSND` version 2 (doom-sound, M6a step 6): the final kind.
pub const SOUND_SLOT_DSND2: u8 = 2;

/// The eleven music lumps of the Episode 1 path (`S_music`, sounds.c; looked up as
/// `d_<name>`, s_sound.c:671-676).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Music {
    /// `D_E1M1` to `D_E1M9` (`mus_e1m1 + gamemap - 1` for episode 1, s_sound.c:235-245).
    Level(u8),
    /// `D_INTER` (wi_stuff.c:1514).
    Inter,
    /// `D_VICTOR` (f_finale.c:114).
    Victor,
}

impl Music {
    /// The lump name looked up.
    pub fn lump_name(self) -> String {
        match self {
            Self::Level(m) => format!("D_E1M{m}"),
            Self::Inter => "D_INTER".into(),
            Self::Victor => "D_VICTOR".into(),
        }
    }
}

/// Which of the eleven music lumps the loaded WADs hold (`W_CheckNumForName`, case-folded as
/// `W_GetNumForName` folds, w_wad.c:345-409).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MusicLumps {
    present: [bool; 11],
}

impl MusicLumps {
    /// The presence of each lump in `dir`.
    pub fn from_directory(dir: &LumpDirectory) -> Self {
        let mut present = [false; 11];
        for (i, p) in present.iter_mut().enumerate() {
            *p = dir
                .check_num_for_name(Self::music(i).lump_name().as_bytes())
                .is_some();
        }
        Self { present }
    }

    fn music(i: usize) -> Music {
        match i {
            0..=8 => Music::Level(i as u8 + 1),
            9 => Music::Inter,
            _ => Music::Victor,
        }
    }

    fn index(m: Music) -> usize {
        match m {
            Music::Level(n) => usize::from(n.clamp(1, 9)) - 1,
            Music::Inter => 9,
            Music::Victor => 10,
        }
    }

    /// Whether the lump of `m` is present.
    pub fn present(&self, m: Music) -> bool {
        self.present[Self::index(m)]
    }
}

/// Why the session's sound refused. Nothing changed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SoundWitness {
    /// The M5c session refused (a sound, the update, the tic limit, the draw count).
    Session(SessionRefusal),
    /// `W_GetNumForName` finds no music lump: the genuine engine stops here (s_sound.c:671-676,
    /// w_wad.c:399-409; U10 (i)).
    MusicMissing(Music),
    /// The intermission starts did not draw one `M_Random` value each (the module
    /// documentation of `intermission`): a broken invariant of the seam, refused.
    Draws {
        /// The starts.
        starts: usize,
        /// The draws the sound session made.
        draws: u16,
    },
    /// The finished level gives no listener (no player body).
    NoListener,
    /// The saved slot could not be read or does not fit.
    Slot(StateError),
    /// A slot kind this build does not read (`DSND` version 2 waits for its merge).
    SlotKind(u8),
}

impl fmt::Display for SoundWitness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session(e) => write!(f, "{e}"),
            Self::MusicMissing(m) => write!(f, "W_GetNumForName: {} not found!", m.lump_name()),
            Self::Draws { starts, draws } => write!(
                f,
                "{starts} intermission sound starts made {draws} random draws"
            ),
            Self::NoListener => write!(f, "the finished level has no listener"),
            Self::Slot(e) => write!(f, "sound state: {e:?}"),
            Self::SlotKind(k) => write!(f, "sound state kind {k} is not read by this build"),
        }
    }
}

/// The session's sound (the interim; module documentation).
#[derive(Clone, Debug, PartialEq)]
pub struct SessionSound {
    sound: Sound,
    music: MusicLumps,
    /// The music playing (`mus_playing`).
    playing: Music,
}

impl SessionSound {
    /// A new game on `map` (`G_InitNew` → `P_SetupLevel` → `S_Start`): a fresh process's session
    /// on the map (volume 8, 3 channels, M5c T9) and the map's music looked up: a missing lump
    /// stops the genuine engine before any tic.
    pub fn new(
        set: Arc<SoundSet>,
        textures: u32,
        music: MusicLumps,
        map: u8,
    ) -> Result<Self, SoundWitness> {
        let playing = Music::Level(map);
        if !music.present(playing) {
            return Err(SoundWitness::MusicMissing(playing));
        }
        let sound = Sound::new(set, SoundConfig::fresh(i32::from(map), textures)).map_err(|e| {
            SoundWitness::Session(SessionRefusal::Sound {
                tic: 0,
                at: doom_sound::session::RefusedAt::Update,
                refusal: e,
            })
        })?;
        Ok(Self {
            sound,
            music,
            playing,
        })
    }

    /// The M5c session inside (its channel table, mixer, schedule and detector).
    pub fn sound(&self) -> &Sound {
        &self.sound
    }

    /// The music playing.
    pub fn playing(&self) -> Music {
        self.playing
    }

    /// `S_ChangeMusic(target)`'s lookup (s_sound.c:665-676): none when it is playing.
    fn change_music(&mut self, target: Music) -> Result<(), SoundWitness> {
        if target != self.playing {
            if !self.music.present(target) {
                return Err(SoundWitness::MusicMissing(target));
            }
            self.playing = target;
        }
        Ok(())
    }

    /// Only the music lookup of a load of `map` (the preflight a session makes before it builds
    /// the map): `Err` exactly when [`Self::level_tic`] with this load would refuse for music.
    pub fn check_load_music(&self, map: u8) -> Result<(), SoundWitness> {
        self.clone().change_music(Music::Level(map))
    }

    /// Only the music lookup of an intermission's first tic.
    pub fn check_intermission_music(&self) -> Result<(), SoundWitness> {
        self.clone().change_music(Music::Inter)
    }

    /// One level tic of `sim` (which has just completed it), with `load` the map loaded in this
    /// tic (`S_Start` first, module documentation). Transactional.
    pub fn level_tic(
        &mut self,
        load: Option<u8>,
        sim: &Simulation,
        rndindex: u8,
    ) -> Result<TicSound, SoundWitness> {
        let mut next = self.clone();
        if let Some(map) = load {
            next.change_music(Music::Level(map))?;
            let mut st = next.sound.state();
            st.channels.iter_mut().for_each(|c| *c = None);
            st.config.logic.gamemap = i32::from(map);
            next.sound = Sound::from_state(next.sound.mixer().sound_set().clone(), &st, st.config)
                .map_err(SoundWitness::Slot)?;
        }
        let listener = simulation_listener(sim).ok_or(SoundWitness::NoListener)?;
        let t = next
            .sound
            .tic(
                sim.sound_records(),
                rndindex,
                &listener,
                &SimulationPositions(sim),
            )
            .map_err(SoundWitness::Session)?;
        *self = next;
        Ok(t)
    }

    /// One intermission tic: `entering` on the first (the `G_Ticker` of `G_DoCompleted`), the
    /// starts in call order with the index before the first, and the finished level `frozen`
    /// for the listener and the origins. Transactional.
    pub fn intermission_tic(
        &mut self,
        entering: bool,
        starts: &[u16],
        rndindex: u8,
        frozen: &Simulation,
    ) -> Result<TicSound, SoundWitness> {
        let mut next = self.clone();
        if entering {
            next.change_music(Music::Inter)?;
        }
        let records: Vec<SoundRecord> = starts
            .iter()
            .map(|&sfx| SoundRecord::Start {
                sfx,
                origin: SoundOrigin::None,
                at: None,
            })
            .collect();
        let listener = simulation_listener(frozen).ok_or(SoundWitness::NoListener)?;
        let t = next
            .sound
            .tic(&records, rndindex, &listener, &SimulationPositions(frozen))
            .map_err(SoundWitness::Session)?;
        if usize::from(t.draws) != starts.len() {
            return Err(SoundWitness::Draws {
                starts: starts.len(),
                draws: t.draws,
            });
        }
        *self = next;
        Ok(t)
    }

    /// The `DSES` v1 sound slot: (kind, bytes).
    pub fn slot(&self) -> (u8, Vec<u8>) {
        (SOUND_SLOT_INTERIM, self.sound.state().encode())
    }

    /// The sound rebuilt from a `DSES` v1 slot over `set`, checked against the session: the
    /// map `map`, `textures`, the music lumps, the music playing `playing`, the tic count
    /// `gametic`, and every actor or sector origin resolving in `level` (the current level or
    /// the finished one). On an error nothing is built.
    #[allow(clippy::too_many_arguments, reason = "every input is checked")]
    pub fn from_slot(
        kind: u8,
        bytes: &[u8],
        set: Arc<SoundSet>,
        textures: u32,
        music: MusicLumps,
        map: u8,
        playing: Music,
        gametic: u64,
        level: &Simulation,
    ) -> Result<Self, SoundWitness> {
        if kind != SOUND_SLOT_INTERIM {
            return Err(SoundWitness::SlotKind(kind));
        }
        let st = SoundState::decode(bytes).map_err(SoundWitness::Slot)?;
        if st.schedule.tic() != gametic {
            return Err(SoundWitness::Slot(StateError::TicMismatch {
                state: st.schedule.tic(),
                simulation: gametic,
            }));
        }
        for (cnum, e) in st.channels.iter().enumerate() {
            match e.map(|e| e.origin) {
                Some(SoundOrigin::Actor(a)) if level.sound_origin_xy(a).is_none() => {
                    return Err(SoundWitness::Slot(StateError::ActorGone { cnum }));
                }
                Some(SoundOrigin::Sector(s)) if level.sector_sound_origin(s).is_none() => {
                    return Err(SoundWitness::Slot(StateError::Sector { cnum }));
                }
                _ => {}
            }
        }
        let expected = SoundConfig::fresh(i32::from(map), textures);
        let sound = Sound::from_state(set, &st, expected).map_err(SoundWitness::Slot)?;
        if !music.present(playing) {
            return Err(SoundWitness::MusicMissing(playing));
        }
        Ok(Self {
            sound,
            music,
            playing,
        })
    }
}
