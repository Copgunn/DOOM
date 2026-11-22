//! M6a: one single-player Episode 1 session across levels in the game layer (owner's ruling
//! U2 = B1; M6a note §4.1, §4.2): the `G_Ticker` of linuxdoom-1.10 (g_game.c:605-748) for the
//! tics in which no level runs, the loads and the reborns, over wire 26 (doom-sim's carried
//! level start), the session display (doom-render, `DDSP` version 3) and the session's sound.
//!
//! **One session tic** ([`Session::tic`]), in the C's order:
//!
//! 1. **The reborn first** (g_game.c:611-614): when the level's dead player pressed use
//!    (`PST_REBORN`, p_user.c:227-228), `G_DoReborn` sets `gameaction = ga_loadlevel`
//!    (g_game.c:924-932), overwriting a `ga_completed` of the same tic, so a reborn beats a
//!    same-tic exit (U8 item 12, observed by the genuine items `USEAT131`, `USEAT130`,
//!    `USEAT132`, `PROBE`); the map is reloaded with a fresh player and the carried `P_Random`
//!    index (`G_PlayerReborn` from `P_SpawnPlayer`, p_mobj.c:659-660) and its tic 0 runs with this
//!    tic's command (g_game.c:664-669, 729-734).
//! 2. **A pending completion** (`ga_completed`, set by the level's exit): `G_DoCompleted`
//!    (g_game.c:1020-1141): `G_PlayerFinishLevel` (779-792) on the carried player; on map 8
//!    `ga_victory`, whose `F_StartFinale` is **not yet** built (a typed refusal of this tic with
//!    nothing changed: the finale comes after the owner's hand test); on map 9 `didsecret`
//!    (1039-1042); `wminfo` (1064-1131); `WI_Start`; then this tic's `WI_Ticker` with this tic's
//!    command (736-738). This is the first intermission tic.
//! 3. **A pending world-done** (`ga_worlddone`, set by the previous tic's `WI_Ticker` through
//!    `G_WorldDone`, g_game.c:1147-1152): `G_DoWorldDone` (1172-1179) loads map `next + 1` with the
//!    carried player, whose latches are what the intermission's last tic left
//!    (`WI_checkForAccelerate`, wi_stuff.c:1470-1498), or, for a player dead at the exit, a
//!    reborn start (g_game.c:477-480); then its tic 0 with this tic's command.
//! 4. Otherwise the level's tic (`advance`) or `WI_Ticker`.
//!
//! Every load passes wire 26 the session's `gametic` of its tic (`levelstarttic = gametic`,
//! g_game.c:470), so the level's turbo test reads `gametic` as the C does (below).
//!
//! Then the sound (`S_UpdateSounds` and the due buffer, with the finished level's listener and
//! origins outside a level, d_main.c:392) and the display (`D_Display`, d_main.c:395, the wipe's
//! 320 `M_Random` draws after the frame of the first tic in a new state), as M5c orders them.
//!
//! **The `M_Random` order outside a level** (wi_stuff.c, s_sound.c:327-346): on the first
//! intermission tic, `WI_Start`'s ten animation draws (`WI_initStats`, inside `G_DoCompleted`),
//! then `WI_Ticker`'s sound starts; on the tic of the press in state 10, the `sfx_sgcock` start,
//! then `WI_initShowNextLoc`'s ten draws; in every other tic only the sound starts. Then the wipe,
//! after the frame. It is checked against the genuine session rows' `rndindex` on every tic.
//!
//! **Refusals** (note §4.8): every refusal of the session layer itself (a `BT_SPECIAL` or other
//! inadmissible command, see below, a map that does not load or is not admitted at its load
//! tic, a missing music lump, a missing intermission picture, the E1M8 victory) is made before
//! anything changes: the whole session is as before the tic. A load tic
//! builds and runs the new level on the side and commits only when everything succeeded. In a
//! level tic the simulation commits its tic before the sound and the display run; a sound or
//! display refusal there leaves the simulation's tic committed and the sound and display at the
//! last good tic (M5c's second-set rule 2), and the session takes no further tic.
//!
//! **The command domain** (owner's ruling 2 of 2026-11-22): a command is judged by what reads it
//! in its `G_Ticker`. `BT_SPECIAL` (the original's pause and save, g_game.c:698-724) is refused in
//! every game state. A command that reaches `P_Ticker` (a level tic, or the new level's tic 0 of
//! a load tic) must be in wire 24's domain ([`crate::app::wire24_admits`]), as before. A command
//! read by `WI_Ticker` (the completion tic and every later intermission tic) is otherwise
//! accepted whatever it holds: `WI_checkForAccelerate` reads only bits 0 (`BT_ATTACK`) and 1
//! (`BT_USE`) of its buttons (wi_stuff.c:1470-1498), and nothing else in the intermission reads
//! the command but the turbo test below. The finale is still a typed "not yet" refusal; the Doom 1
//! finale reads no command (`F_Ticker`, f_finale.c:207-249, reads one for Doom II only), so the
//! same rule (`BT_SPECIAL` refused, everything else accepted) applies there when it is built
//! (`command_domain`).
//!
//! **The turbo message** (g_game.c:671-679; owner's ruling 1 of 2026-11-22) tests `gametic`, the
//! session's tic count: wire 26 reads the load's `gametic` plus the level tic, so a level after a
//! load gets the message on the C's tics, a load tic included (the load's `P_SpawnPlayer` clears
//! `message`, p_mobj.c:676, before the test sets it in the same `G_Ticker`, g_game.c:617-650 then
//! 671-679). Outside a level the test still runs (it precedes the game-state switch) and sets the
//! finished level's player's `message`, which nothing reads there (`HU_Ticker` and `HU_Drawer` run
//! in a level only, g_game.c:729-733, d_main.c:270-271) and which the next load's `P_SpawnPlayer`
//! clears before any `HU_Ticker`; the session therefore keeps no message outside a level.
pub mod demo;
pub mod intermission;
pub mod sound;
pub mod state;

use doom_map::Map;
use doom_render::{
    Display, DisplayConfig, DisplayFrame, DisplayRefusal, Frame, GameMode, GameState, Level,
    LevelChange, LumpDirectory, ResourceError, Resources, SessionError,
};
use doom_sim::{
    CarriedStart, LevelExit, LevelStart, LevelStartError, MapContextOptions, SimError, Simulation,
    Skill, StartKind, TicCommand,
};
use doom_sound::chain::{ChainConfig, ChainRefusal, ChainSound, ChainTic, IntermissionSound};
use doom_sound::lumps::{SoundCaps, SoundLoadError, SoundSet};
use doom_types::{DoomRng, Fixed};
use intermission::{Intermission, Latches, PARS_E1, WbPlayer, WbStart, next_map};
use sound::{Music, MusicLumps, SoundWitness};
use std::fmt;
use std::sync::Arc;

/// `BT_SPECIAL` (d_event.h:78).
pub const BT_SPECIAL: u8 = 0x80;
/// The `M_Random` draws of one wipe (f_wipe.c:159-166), modulo the 256 entries of the index.
const WIPE_INDEX_STEP: u8 = (320 % 256) as u8;

/// The game mode of a session (owner's ruling U4): an explicit, declared setting.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SessionMode {
    /// `registered` (d_main.c:697-702: what linuxdoom makes of a file named `doom.wad`).
    Registered,
    /// `retail` (d_main.c:690-695: `doomu.wad`).
    Retail,
}

impl SessionMode {
    /// doom-render's game mode.
    pub fn game_mode(self) -> GameMode {
        match self {
            Self::Registered => GameMode::Registered,
            Self::Retail => GameMode::Retail,
        }
    }

    /// The word used in file names and on the exit line: `registered` or `retail`.
    pub fn word(self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Retail => "retail",
        }
    }
}

/// How the mode was chosen (recorded with the session, `DSES` v1).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ModeSource {
    /// The default: [`probe_game_mode`] on the IWAD.
    Probed,
    /// Forced by the caller's flag.
    Forced,
}

/// The declared game mode of a session and how it was chosen.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct DeclaredMode {
    /// The mode.
    pub mode: SessionMode,
    /// How it was chosen.
    pub source: ModeSource,
}

/// The caller's choice of game mode: the probe (the default) or a forced mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum ModeSetting {
    /// [`probe_game_mode`] on the IWAD.
    #[default]
    Probe,
    /// Forced.
    Force(SessionMode),
}

/// The declared probe of ruling U4 (linuxdoom makes no such probe: `IdentifyVersion`,
/// d_main.c:563-717, tests file names only): **retail when the IWAD, the first WAD given, has a
/// lump named exactly `E4M1`** (the first map of a fourth episode) in its own directory,
/// registered otherwise. Later WADs are not looked at.
pub fn probe_game_mode(iwad: &[u8]) -> Result<SessionMode, doom_wad::WadError> {
    let wad = doom_wad::Wad::parse(iwad, doom_wad::Limits::default())?;
    let fourth = wad
        .lumps()
        .iter()
        .any(|l| l.name().as_bytes() == b"E4M1\0\0\0\0");
    Ok(if fourth {
        SessionMode::Retail
    } else {
        SessionMode::Registered
    })
}

/// A map that could not be loaded (the genuine engine's `W_GetNumForName` or set-up stop at its
/// load).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapLoadError {
    /// The map's lumps are missing or malformed.
    Map(doom_map::MapError),
    /// The renderer's level tables could not be built.
    Level(ResourceError),
}

/// Why the session data could not be loaded. Nothing is built.
#[derive(Debug)]
pub enum DataError {
    /// No WAD was given.
    NoWad,
    /// WAD `index` (0 is the IWAD) is not a valid WAD.
    Wad {
        /// The file's position.
        index: usize,
        /// The parser's error.
        error: doom_wad::WadError,
    },
    /// The renderer's resources could not be loaded.
    Resources(ResourceError),
    /// The sound data could not be loaded.
    SoundData(SoundLoadError),
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "session data: {self:?}")
    }
}

impl std::error::Error for DataError {}

/// Everything a session loads once from the WADs: the declared game mode, the renderer's
/// resources, the nine Episode 1 maps (each loaded, or the error its load tic will refuse with),
/// the sound data and the music lumps present.
#[derive(Clone, Debug)]
pub struct SessionData {
    mode: DeclaredMode,
    res: Arc<Resources>,
    maps: Vec<Result<(Map, Arc<Level>), MapLoadError>>,
    sound_set: Arc<SoundSet>,
    textures: u32,
    music: MusicLumps,
}

impl SessionData {
    /// Loads `wads` (the IWAD first; later files take precedence, as `W_AddFile` order does) with
    /// the game mode `setting` (the probe by default; ruling U4).
    pub fn load(wads: &[&[u8]], setting: ModeSetting) -> Result<Self, DataError> {
        Self::load_with(wads, &[], setting)
    }

    /// [`Self::load`] with extra single-lump files after the WADs (`W_AddFile` of a non-WAD file,
    /// as `-playdemo` adds its demo: `(file name, bytes)`).
    pub fn load_with(
        wads: &[&[u8]],
        lumps: &[(&str, &[u8])],
        setting: ModeSetting,
    ) -> Result<Self, DataError> {
        let iwad = wads.first().ok_or(DataError::NoWad)?;
        let mode = match setting {
            ModeSetting::Force(mode) => DeclaredMode {
                mode,
                source: ModeSource::Forced,
            },
            ModeSetting::Probe => DeclaredMode {
                mode: probe_game_mode(iwad).map_err(|error| DataError::Wad { index: 0, error })?,
                source: ModeSource::Probed,
            },
        };
        let parsed = wads
            .iter()
            .enumerate()
            .map(|(index, bytes)| {
                doom_wad::Wad::parse(bytes, doom_wad::Limits::default())
                    .map_err(|error| DataError::Wad { index, error })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut dir = LumpDirectory::new();
        for wad in &parsed {
            dir.add_wad(wad).map_err(DataError::Resources)?;
        }
        for (name, bytes) in lumps {
            dir.add_single_lump(name, bytes)
                .map_err(DataError::Resources)?;
        }
        let music = MusicLumps::from_directory(&dir);
        let sound_set =
            Arc::new(SoundSet::load(&dir, SoundCaps::default()).map_err(DataError::SoundData)?);
        let res =
            Arc::new(Resources::load(dir, mode.mode.game_mode()).map_err(DataError::Resources)?);
        // numtextures (r_data.c:480), for the layout detector of decision T12.
        let textures = u32::try_from(res.textures.len()).unwrap_or(u32::MAX);
        let set = doom_wad::WadSet::new(&parsed);
        let maps = (1..=9u8)
            .map(|n| {
                let map = Map::load(&set, &format!("E1M{n}"), doom_map::MapLimits::default())
                    .map_err(MapLoadError::Map)?;
                let level = Level::load(&res, &map, 1).map_err(MapLoadError::Level)?;
                Ok((map, Arc::new(level)))
            })
            .collect();
        Ok(Self {
            mode,
            res,
            maps,
            sound_set,
            textures,
            music,
        })
    }

    /// The declared game mode.
    pub fn mode(&self) -> DeclaredMode {
        self.mode
    }

    /// The renderer's resources.
    pub fn resources(&self) -> &Arc<Resources> {
        &self.res
    }

    /// The music lumps present.
    pub fn music(&self) -> MusicLumps {
        self.music
    }

    pub(crate) fn map(&self, n: u8) -> Result<&(Map, Arc<Level>), MapLoadError> {
        self.maps[usize::from(n - 1)].as_ref().map_err(Clone::clone)
    }
}

/// The session's fixed settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionSettings {
    /// The start map, 1..=9 (any, as vanilla's warp; ruling U14).
    pub map: u8,
    /// The skill byte: 2 or 3 (the easy skills are M6a step 10, after the hand test).
    pub skill: u8,
    /// Screenblocks, 10 or 11 (decision S14).
    pub screenblocks: u8,
}

/// Original behaviour not built yet (after the owner's hand test, M6a note §6.5): each is a
/// typed refusal with the whole session preserved, never a silent no-op.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum NotYet {
    /// `ga_victory` → `F_StartFinale` (g_game.c:639-641, 1033-1038; f_finale.c:96-191): the
    /// tic after E1M8's exit is refused before anything changes.
    Victory,
    /// A saved session in the finale (`DSES` v1 has room for it).
    Finale,
    /// The easy skills 0 and 1 (M6a step 10, ruling U12).
    EasySkill,
}

/// Why a session could not start (before tic 0). Nothing is built.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StartError {
    /// The map is outside 1..=9, the skill outside 0..=3, or screenblocks outside 10..=11.
    Domain,
    /// Not yet admitted.
    NotYet(NotYet),
    /// The start map does not load.
    Map(MapLoadError),
    /// Wire 26 refuses the start map.
    Admission(LevelStartError),
    /// The start map's music lump is missing: the genuine engine stops at its first load
    /// (U10 (i)).
    Sound(SoundWitness),
    /// The display could not be built.
    Display(String),
}

/// What refused a session tic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionWitness {
    /// The command holds `BT_SPECIAL` (g_game.c:698-724: the original's pause, M6b, and save,
    /// a later milestone): never recorded, refused in a replayed demo.
    Special,
    /// A command that reaches `P_Ticker` (a level tic or a load tic) is outside wire 24's domain
    /// (module documentation; the intermission accepts it).
    Command,
    /// Not yet built.
    NotYet(NotYet),
    /// The map of a load tic does not load.
    Map {
        /// The map.
        map: u8,
        /// Why.
        error: MapLoadError,
    },
    /// Wire 26 refuses the map of a load tic.
    Admission {
        /// The map.
        map: u8,
        /// Why.
        error: LevelStartError,
    },
    /// The simulation refused the tic.
    Simulation(SimError),
    /// The sound refused the tic (a missing music lump included).
    Sound(SoundWitness),
    /// The display refused the tic (a missing intermission picture included: `WI_loadData`'s
    /// `I_Error`).
    Display(SessionError),
    /// The level frame was refused (an M5a render witness or a 2D drawer witness).
    Draw(DisplayRefusal),
    /// The frame's palette is not in PLAYPAL.
    Palette(u8),
    /// The two views of the `M_Random` index disagree (a broken seam).
    RandomIndex {
        /// The index the session expected.
        expected: u8,
        /// The display's.
        display: u8,
    },
    /// The session already refused a tic after committing the simulation's (rule 2).
    Ended,
}

impl fmt::Display for SessionWitness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sound(s) => write!(f, "sound: {s}"),
            Self::Simulation(e) => write!(f, "{e}"),
            other => write!(f, "{other:?}"),
        }
    }
}

/// A refused session tic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refusal {
    /// The session tic (`gametic` when it was attempted).
    pub tic: u64,
    /// What refused it.
    pub witness: SessionWitness,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "session tic {}: {}", self.tic, self.witness)
    }
}

impl std::error::Error for Refusal {}

/// What a session tic was.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TicKind {
    /// A level tic.
    Level,
    /// A load and the new level's tic 0.
    Load {
        /// The map loaded.
        map: u8,
        /// How the player started.
        start: StartKind,
    },
    /// `G_DoCompleted` and the first intermission tic.
    Completion,
    /// An intermission tic.
    Intermission,
}

/// What one session tic presented.
#[derive(Clone, Debug)]
pub struct TicReport {
    /// The session tic (`gametic` before it).
    pub tic: u64,
    /// What the tic was.
    pub kind: TicKind,
    /// The frame's palette number.
    pub palette: u8,
    /// The palette `I_SetPalette` received in the frame.
    pub palette_set: Option<u8>,
    /// Whether the frame was followed by a wipe's draws.
    pub wiped: bool,
    /// A level frame's report (None outside a level).
    pub level_frame: Option<DisplayFrame>,
    /// The intermission's animation draws of `M_Random` (`WI_initAnimatedBack`, ten per call;
    /// the sound starts' draws are in `sound.draws`).
    pub wi_draws: u16,
    /// The intermission's sound starts, in call order (empty in a level tic; the level's are
    /// the simulation's records).
    pub wi_starts: Vec<IntermissionSound>,
    /// What the sound did.
    pub sound: ChainTic,
}

/// The carried player between levels: what `G_PlayerFinishLevel` (g_game.c:779-792) leaves of the
/// finished level's player, kept from the completion tic to the next load.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CarriedPlayer {
    /// What a living player carries (doom-sim's [`CarriedStart`], its latches and `P_Random`
    /// index as at the exit; the session's [`CarriedPlayer::latches`] replace the latches at the
    /// load); `None` for a player dead at the exit, who starts the next map reborn
    /// (g_game.c:477-480).
    pub start: Option<CarriedStart>,
    /// `attackdown` and `usedown`, rewritten by every intermission tic (wi_stuff.c:1476-1497).
    pub latches: Latches,
    /// `viewheight` after the exit tic (`P_SpawnPlayer` resets it, p_mobj.c:683).
    pub viewheight: i32,
    /// `deltaviewheight` after the exit tic: carried to the next start (p_mobj.c:642-700 leaves
    /// it alone).
    pub deltaviewheight: i32,
}

/// A fault planted by the evidence self-tests (`tests/m6a_session.rs`), to prove that the
/// comparison with the genuine goldens fails when the session is wrong. Never set by the app;
/// a session holding one cannot be saved.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlantedFault {
    /// A carried start with one point of health less.
    CarriedHealth,
    /// The exit checked before the reborn (the reverse of g_game.c:611-623).
    ExitBeforeReborn,
    /// The tenth animation draw of `WI_Start` skipped.
    SkippedAnimationDraw,
    /// The kill count of the tally one too high after each step in `sp_state` 2.
    TallyOffByOne,
    /// A carried start's `bob` one unit off (one lower, or 1 for 0): `P_CalcHeight` rewrites
    /// the player's `bob` in tic 0 (p_user.c:83-87), so only the `DSIM` 42 start group keeps it.
    CarriedBob,
    /// Every load at `gametic` 0, so the turbo test reads `leveltime` (step 7's model, without
    /// its refusal).
    TurboLeveltime,
    /// Wire 24's command domain applied in every game state (step 7's rule).
    IntermissionDomain,
}

/// Which domain judges a command, by the part of `G_Ticker` that reads it (module
/// documentation).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Reader {
    /// `P_Ticker`: a level tic, or a load tic's tic 0.
    Level,
    /// `WI_Ticker` (and, when it is built, the Doom 1 `F_Ticker`).
    NonLevel,
}

/// The command check of ruling 2: `BT_SPECIAL` in every game state, wire 24's domain only for a
/// command `P_Ticker` reads.
fn command_domain(cmd: &TicCommand, reader: Reader) -> Result<(), SessionWitness> {
    if cmd.buttons & BT_SPECIAL != 0 {
        return Err(SessionWitness::Special);
    }
    if reader == Reader::Level && !crate::app::wire24_admits(cmd) {
        return Err(SessionWitness::Command);
    }
    Ok(())
}

/// One single-player Episode 1 session (module documentation).
#[derive(Clone, Debug)]
pub struct Session {
    data: Arc<SessionData>,
    settings: SessionSettings,
    skill: Skill,
    /// `gametic`: the session tics run (d_net.c:745-746).
    gametic: u64,
    /// `gamemap` (the current level's, or the finished level's outside a level).
    map: u8,
    /// `secretexit` (g_game.c:1000): rewritten at every exit, kept otherwise.
    secretexit: bool,
    /// The console player's `didsecret` (d_player.h:164).
    didsecret: bool,
    /// `wminfo` (g_game.c:1064-1131), as `WI_initVariables` left it.
    wminfo: WbStart,
    /// The wi_stuff.c statics.
    wi: Intermission,
    /// `ga_worlddone` pending (set by `G_WorldDone`).
    world_done: bool,
    /// The carried player, outside a level.
    carried: Option<CarriedPlayer>,
    /// The current level, or outside a level the finished one (read only).
    level: Simulation,
    display: Display,
    sound: ChainSound,
    frame: Frame,
    /// Set by a rule-2 refusal (module documentation).
    ended: Option<Refusal>,
    /// A planted fault (self-tests only).
    fault: Option<PlantedFault>,
}

fn skill_of(byte: u8) -> Result<Skill, StartError> {
    match byte {
        2 => Ok(Skill::Medium),
        3 => Ok(Skill::Hard),
        0 | 1 => Err(StartError::NotYet(NotYet::EasySkill)),
        _ => Err(StartError::Domain),
    }
}

impl Session {
    /// A new game on `settings.map` (`G_InitNew` → `G_DoLoadLevel`, g_game.c:1412-1484): the
    /// map's music looked up, wire 26's new-game start, a fresh session display (`DDSP` version
    /// 3) and sound. Nothing runs yet: [`Self::tic`] runs tic 0.
    pub fn new(data: Arc<SessionData>, settings: SessionSettings) -> Result<Self, StartError> {
        if !(1..=9).contains(&settings.map) || !(10..=11).contains(&settings.screenblocks) {
            return Err(StartError::Domain);
        }
        let skill = skill_of(settings.skill)?;
        let sound = ChainSound::new(
            data.sound_set.clone(),
            data.music,
            ChainConfig::fresh(data.textures),
            i32::from(settings.map),
        )
        .map_err(|e| StartError::Sound(SoundWitness::Chain(e)))?;
        let (map, level) = data.map(settings.map).map_err(StartError::Map)?;
        let sim = Simulation::original_player_episode1_v4_level(
            map.clone(),
            settings.map,
            skill,
            MapContextOptions::default(),
            LevelStart::NewGame,
        )
        .map_err(StartError::Admission)?;
        let cfg = DisplayConfig {
            screenblocks: settings.screenblocks,
        };
        let mut display = Display::new_session(data.res.clone(), level.clone(), cfg)
            .map_err(|e| StartError::Display(format!("{e:?}")))?;
        display
            .begin_level(&sim)
            .map_err(|e| StartError::Display(format!("{e:?}")))?;
        Ok(Self {
            settings,
            skill,
            gametic: 0,
            map: settings.map,
            secretexit: false,
            didsecret: false,
            wminfo: WbStart::default(),
            wi: Intermission::default(),
            world_done: false,
            carried: None,
            level: sim,
            display,
            sound,
            frame: Frame::new(),
            ended: None,
            fault: None,
            data,
        })
    }

    /// The settings.
    pub fn settings(&self) -> SessionSettings {
        self.settings
    }

    /// The declared game mode.
    pub fn mode(&self) -> DeclaredMode {
        self.data.mode
    }

    /// The session data.
    pub fn data(&self) -> &Arc<SessionData> {
        &self.data
    }

    /// `gametic`: the session tics run.
    pub fn gametic(&self) -> u64 {
        self.gametic
    }

    /// `gamestate`.
    pub fn gamestate(&self) -> GameState {
        self.display.game_state().unwrap_or(GameState::Level)
    }

    /// `gameaction` after the last tic, as `gameaction_t` numbers it (d_event.h:54-63): 6
    /// (`ga_completed`) after an exit tic, 8 (`ga_worlddone`) after the intermission's last tic,
    /// 0 otherwise (`ga_loadlevel` is consumed in the tic that sets it).
    pub fn gameaction(&self) -> u8 {
        match self.gamestate() {
            GameState::Level if self.level.level_exit_request().is_some() => 6,
            GameState::Intermission if self.world_done => 8,
            _ => 0,
        }
    }

    /// `gamemap`.
    pub fn map(&self) -> u8 {
        self.map
    }

    /// `secretexit`: in a level with an exit requested, that exit's; otherwise the last exit's.
    pub fn secretexit(&self) -> bool {
        match (self.gamestate(), self.level.level_exit_request()) {
            (GameState::Level, Some(e)) => e == LevelExit::Secret,
            _ => self.secretexit,
        }
    }

    /// The console player's `didsecret`.
    pub fn didsecret(&self) -> bool {
        self.didsecret
    }

    /// `wminfo`.
    pub fn wminfo(&self) -> &WbStart {
        &self.wminfo
    }

    /// The wi_stuff.c statics.
    pub fn intermission(&self) -> &Intermission {
        &self.wi
    }

    /// The carried player (outside a level).
    pub fn carried(&self) -> Option<&CarriedPlayer> {
        self.carried.as_ref()
    }

    /// The current level's simulation, or outside a level the finished one.
    pub fn level(&self) -> &Simulation {
        &self.level
    }

    /// The display.
    pub fn display(&self) -> &Display {
        &self.display
    }

    /// The sound.
    pub fn sound(&self) -> &ChainSound {
        &self.sound
    }

    /// `levelstarttic` (g_game.c:470): the session tic of the current (or finished) level's load.
    pub fn levelstarttic(&self) -> u64 {
        self.sound.level_start()
    }

    /// The last frame.
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    /// The refusal that ended the session after its simulation committed a tic, if any.
    pub fn ended(&self) -> Option<&Refusal> {
        self.ended.as_ref()
    }

    /// Plants a fault (self-tests only, [`PlantedFault`]).
    #[doc(hidden)]
    pub fn plant_fault(&mut self, fault: PlantedFault) {
        self.fault = Some(fault);
    }

    /// Runs one session tic with `cmd` (module documentation).
    pub fn tic(&mut self, cmd: TicCommand) -> Result<TicReport, Refusal> {
        let tic = self.gametic;
        let refuse = |witness| Refusal { tic, witness };
        if self.ended.is_some() {
            return Err(refuse(SessionWitness::Ended));
        }
        let fault = self.fault;
        let check = |reader| {
            let reader = if fault == Some(PlantedFault::IntermissionDomain) {
                Reader::Level
            } else {
                reader
            };
            command_domain(&cmd, reader)
        };
        let r = match self.gamestate() {
            GameState::Level => {
                let exit_first = self.fault == Some(PlantedFault::ExitBeforeReborn)
                    && self.level.level_exit_request().is_some();
                if self.level.reborn_requested() && !exit_first {
                    let p_random = self.level.rng().indices()[0];
                    check(Reader::Level).and_then(|()| {
                        self.load_tic(
                            cmd,
                            self.map,
                            LevelStart::Reborn { p_random },
                            LevelChange::Reborn,
                            0,
                        )
                    })
                } else if let Some(exit) = self.level.level_exit_request() {
                    check(Reader::NonLevel).and_then(|()| self.completion_tic(cmd, exit))
                } else {
                    check(Reader::Level).and_then(|()| self.level_tic(cmd))
                }
            }
            GameState::Intermission if self.world_done => {
                check(Reader::Level).and_then(|()| self.world_done_tic(cmd))
            }
            GameState::Intermission => {
                check(Reader::NonLevel).and_then(|()| self.intermission_tic(cmd))
            }
            // When the finale is built its tics take `Reader::NonLevel` (module documentation).
            GameState::Finale => {
                check(Reader::NonLevel).and(Err(SessionWitness::NotYet(NotYet::Victory)))
            }
        };
        r.map_err(refuse)
    }

    /// A level tic: `P_Ticker`, then sound and display (rule 2 on their refusal).
    fn level_tic(&mut self, cmd: TicCommand) -> Result<TicReport, SessionWitness> {
        let none = TicCommand::default();
        self.level
            .advance([cmd, none, none, none])
            .map_err(SessionWitness::Simulation)?;
        let r = self.present_level(cmd, None, None, TicKind::Level);
        if let Err(w) = &r {
            self.ended = Some(Refusal {
                tic: self.gametic,
                witness: w.clone(),
            });
        }
        r
    }

    /// The sound and display of a level tic of `self.level` (already advanced); with `load`, the
    /// display `prepared` by `begin_next_level_carried` and the map loaded. Commits sound and
    /// display only when both succeed.
    fn present_level(
        &mut self,
        cmd: TicCommand,
        load: Option<u8>,
        prepared: Option<Display>,
        kind: TicKind,
    ) -> Result<TicReport, SessionWitness> {
        let mut display = prepared.unwrap_or_else(|| self.display.clone());
        let before = display.random_index();
        let mut sound = self.sound.clone();
        let ts = sound::level_tic(&mut sound, load, &self.level, before)
            .map_err(SessionWitness::Sound)?;
        let expected = before.wrapping_add((ts.sound.draws & 0xff) as u8);
        if expected != ts.sound.rndindex {
            return Err(SessionWitness::RandomIndex {
                expected,
                display: ts.sound.rndindex,
            });
        }
        display
            .observe_after(&self.level, &cmd, ts.sound.draws)
            .map_err(|e| SessionWitness::Display(SessionError::Observe(e)))?;
        let wiped = display.wipe_due();
        let mut frame = std::mem::take(&mut self.frame);
        let drawn = display.draw(&self.level, &mut frame);
        self.frame = frame;
        let f = drawn.map_err(SessionWitness::Draw)?;
        if display.playpal().len() / 768 <= usize::from(f.palette) {
            return Err(SessionWitness::Palette(f.palette));
        }
        self.display = display;
        self.sound = sound;
        self.gametic += 1;
        Ok(TicReport {
            tic: self.gametic - 1,
            kind,
            palette: f.palette,
            palette_set: f.palette_set,
            wiped,
            level_frame: Some(f),
            wi_draws: 0,
            wi_starts: Vec::new(),
            sound: ts,
        })
    }

    /// A load and the new level's tic 0 (a reborn reload or `G_DoWorldDone`): everything on the
    /// side, committed at the end.
    fn load_tic(
        &mut self,
        cmd: TicCommand,
        map: u8,
        start: LevelStart,
        change: LevelChange,
        deltaviewheight: i32,
    ) -> Result<TicReport, SessionWitness> {
        // S_Start's music lookup comes first in P_SetupLevel (p_setup.c:607), before the map's
        // lumps are read (p_setup.c:618-650).
        if sound::music_lookup_fails(&self.sound, Music::Level(map)) {
            return Err(self.music_missing(Music::Level(map)));
        }
        let (m, level) = self
            .data
            .map(map)
            .map_err(|error| SessionWitness::Map { map, error })?;
        // `levelstarttic = gametic` (g_game.c:470): this tic's `gametic`.
        let gametic = if self.fault == Some(PlantedFault::TurboLeveltime) {
            0
        } else {
            self.gametic
        };
        let mut sim = Simulation::original_player_episode1_v4_level_at(
            m.clone(),
            map,
            self.skill,
            MapContextOptions::default(),
            start,
            gametic,
        )
        .map_err(|error| SessionWitness::Admission { map, error })?;
        let mut display = self.display.clone();
        display
            .begin_next_level_carried(level.clone(), &sim, change, deltaviewheight)
            .map_err(SessionWitness::Display)?;
        let none = TicCommand::default();
        sim.advance([cmd, none, none, none])
            .map_err(SessionWitness::Simulation)?;
        let kind = TicKind::Load {
            map,
            start: start.kind(),
        };
        // Commit the level only with its presentation (transactional load).
        let old = std::mem::replace(&mut self.level, sim);
        match self.present_level(cmd, Some(map), Some(display), kind) {
            Ok(r) => {
                if let Some(e) = old.level_exit_request() {
                    // A reborn that beat a same-tic exit: the exit's `secretexit` stands.
                    self.secretexit = e == LevelExit::Secret;
                }
                if start.kind() == StartKind::Reborn {
                    // G_PlayerReborn's memset (g_game.c:815).
                    self.didsecret = false;
                }
                self.map = map;
                self.carried = None;
                self.world_done = false;
                Ok(r)
            }
            Err(w) => {
                self.level = old;
                Err(w)
            }
        }
    }

    /// The draws of an intermission tic through the display and its frame; commits the display
    /// and the sound.
    #[allow(
        clippy::too_many_arguments,
        reason = "the staged parts of one tic, all moved or borrowed once"
    )]
    fn present_intermission(
        &mut self,
        mut display: Display,
        sound: ChainSound,
        wi: &mut Intermission,
        wminfo: &WbStart,
        rng_after: u8,
        draws: u16,
        kind: TicKind,
        ts: ChainTic,
        wi_starts: Vec<IntermissionSound>,
    ) -> Result<TicReport, SessionWitness> {
        display
            .observe_non_level(draws)
            .map_err(SessionWitness::Display)?;
        if display.random_index() != rng_after {
            return Err(SessionWitness::RandomIndex {
                expected: rng_after,
                display: display.random_index(),
            });
        }
        let wiped = display.wipe_due();
        let view = wi.view(wminfo);
        let mut frame = std::mem::take(&mut self.frame);
        let drawn = display.draw_intermission(&view, &mut frame);
        self.frame = frame;
        let f = drawn.map_err(SessionWitness::Display)?;
        wi.snl_pointeron = f.snl_pointeron;
        let expected = rng_after.wrapping_add(if wiped { WIPE_INDEX_STEP } else { 0 });
        if display.random_index() != expected {
            return Err(SessionWitness::RandomIndex {
                expected,
                display: display.random_index(),
            });
        }
        self.display = display;
        self.sound = sound;
        self.gametic += 1;
        Ok(TicReport {
            tic: self.gametic - 1,
            kind,
            palette: f.palette,
            palette_set: f.palette_set,
            wiped,
            level_frame: None,
            wi_draws: draws - ts.sound.draws,
            wi_starts,
            sound: ts,
        })
    }

    /// `G_DoCompleted` and the first intermission tic.
    fn completion_tic(
        &mut self,
        cmd: TicCommand,
        exit: LevelExit,
    ) -> Result<TicReport, SessionWitness> {
        if self.map == 8 {
            return Err(SessionWitness::NotYet(NotYet::Victory));
        }
        let finish = self
            .level
            .level_finish()
            .map_err(|e| SessionWitness::Simulation(e.into()))?;
        let view = self.display.level_view().ok_or(SessionWitness::Display(
            SessionError::WrongGameState(GameState::Level),
        ))?;
        let latches = match finish.carried {
            Some(c) => Latches {
                attackdown: c.attack_down,
                usedown: c.use_down,
            },
            None => dead_latches(&self.level)?,
        };
        let carried = CarriedPlayer {
            start: finish.carried,
            latches,
            viewheight: view.viewheight,
            deltaviewheight: view.deltaviewheight,
        };
        let secretexit = exit == LevelExit::Secret;
        // Map 9: every player's didsecret (g_game.c:1039-1042), then wminfo's (1064).
        let didsecret = self.didsecret || self.map == 9;
        let n = |v: i32| v;
        let mut wminfo = WbStart {
            epsd: 0,
            didsecret,
            last: i32::from(self.map) - 1,
            next: next_map(self.map, secretexit),
            maxkills: n(finish.totals[0]),
            maxitems: n(finish.totals[1]),
            maxsecret: n(finish.totals[2]),
            partime: 35 * PARS_E1[usize::from(self.map) - 1],
            plyr: WbPlayer {
                skills: finish.counts[0],
                sitems: finish.counts[1],
                ssecret: finish.counts[2],
                stime: i32::try_from(finish.level_time).unwrap_or(i32::MAX),
            },
        };
        // WI_Start's WI_loadData (inside G_DoCompleted) stops before WI_Ticker's music lookup.
        let mut display = self.display.clone();
        display
            .begin_intermission()
            .map_err(SessionWitness::Display)?;
        if sound::music_lookup_fails(&self.sound, Music::Inter) {
            return Err(self.music_missing(Music::Inter));
        }
        let mut wi = self.wi;
        let mut rng = DoomRng::from_indices(0, display.random_index());
        let mut pre = wi.start(&mut wminfo, &mut rng);
        if self.fault == Some(PlantedFault::SkippedAnimationDraw) {
            rng = DoomRng::from_indices(0, rng.indices()[1].wrapping_sub(1));
            pre -= 1;
        }
        let mut latches = carried.latches;
        let out = wi.ticker(&wminfo, cmd.buttons, &mut latches, &mut rng);
        let mut sound = self.sound.clone();
        let ts = sound::intermission_tic(&mut sound, &out.starts, out.sound_index, &self.level)
            .map_err(SessionWitness::Sound)?;
        let r = self.present_intermission(
            display,
            sound,
            &mut wi,
            &wminfo,
            rng.indices()[1],
            pre + out.draws,
            TicKind::Completion,
            ts,
            out.starts,
        )?;
        self.wi = wi;
        self.wminfo = wminfo;
        self.secretexit = secretexit;
        self.didsecret = didsecret;
        self.carried = Some(CarriedPlayer { latches, ..carried });
        self.apply_world_done(out.world_done);
        Ok(r)
    }

    /// `G_WorldDone` (g_game.c:1145-1153): `ga_worlddone`, and after a secret exit the console
    /// player's `didsecret`.
    fn apply_world_done(&mut self, world_done: bool) {
        if world_done {
            self.world_done = true;
            if self.secretexit {
                self.didsecret = true;
            }
        }
    }

    /// A later intermission tic.
    fn intermission_tic(&mut self, cmd: TicCommand) -> Result<TicReport, SessionWitness> {
        let carried = self
            .carried
            .ok_or(SessionWitness::Display(SessionError::WrongGameState(
                GameState::Intermission,
            )))?;
        let mut wi = self.wi;
        let mut rng = DoomRng::from_indices(0, self.display.random_index());
        let mut latches = carried.latches;
        let out = wi.ticker(&self.wminfo, cmd.buttons, &mut latches, &mut rng);
        if self.fault == Some(PlantedFault::TallyOffByOne) && wi.sp_state == 2 {
            wi.cnt_kills += 1;
        }
        let mut sound = self.sound.clone();
        let ts = sound::intermission_tic(&mut sound, &out.starts, out.sound_index, &self.level)
            .map_err(SessionWitness::Sound)?;
        let wminfo = self.wminfo;
        let display = self.display.clone();
        let r = self.present_intermission(
            display,
            sound,
            &mut wi,
            &wminfo,
            rng.indices()[1],
            out.draws,
            TicKind::Intermission,
            ts,
            out.starts,
        )?;
        self.wi = wi;
        self.carried = Some(CarriedPlayer { latches, ..carried });
        self.apply_world_done(out.world_done);
        Ok(r)
    }

    /// The genuine engine's stop at a missing music lump in this tic (U10 (i)).
    fn music_missing(&self, music: Music) -> SessionWitness {
        SessionWitness::Sound(SoundWitness::Chain(ChainRefusal::MusicMissing {
            music,
            tic: Some(self.gametic),
        }))
    }

    /// `G_DoWorldDone` (g_game.c:1172-1179) and the new level's tic 0.
    fn world_done_tic(&mut self, cmd: TicCommand) -> Result<TicReport, SessionWitness> {
        let carried = self
            .carried
            .ok_or(SessionWitness::Display(SessionError::WrongGameState(
                GameState::Intermission,
            )))?;
        let p_random = self.level.rng().indices()[0];
        let map = u8::try_from(self.wminfo.next + 1).unwrap_or(0);
        if !(1..=9).contains(&map) {
            return Err(SessionWitness::Map {
                map,
                error: MapLoadError::Level(ResourceError::LumpNotFound(format!("E1M{map}"))),
            });
        }
        let planted = i32::from(self.fault == Some(PlantedFault::CarriedHealth));
        let (start, dvh) = match carried.start {
            Some(c) => (
                LevelStart::Carried(CarriedStart {
                    attack_down: carried.latches.attackdown,
                    use_down: carried.latches.usedown,
                    p_random,
                    health: c.health - planted,
                    bob: if self.fault == Some(PlantedFault::CarriedBob) {
                        let b = c.bob.bits();
                        Fixed::from_bits(if b > 0 { b - 1 } else { 1 })
                    } else {
                        c.bob
                    },
                    ..c
                }),
                carried.deltaviewheight,
            ),
            None => (LevelStart::Reborn { p_random }, 0),
        };
        self.load_tic(cmd, map, start, LevelChange::WorldDone, dvh)
    }
}

/// The latches of a player dead at the exit: `attackdown` from the weapon state (no
/// `A_WeaponReady` runs for a dead player) and `usedown` as the canonical capture holds it
/// (`P_PlayerThink` returns through `P_DeathThink` before its use block, p_user.c:249-262,
/// 322-331, so it keeps the value of the tic of death). Read once, at the completion.
fn dead_latches(sim: &Simulation) -> Result<Latches, SessionWitness> {
    let profile = || SessionWitness::Simulation(doom_sim::PlayerBodyError::Profile.into());
    let w = sim
        .original_player_weapons(doom_sim::PlayerSlot::One)
        .ok()
        .flatten()
        .ok_or_else(profile)?;
    let usedown = sim
        .snapshot()
        .fields()
        .find(|f| f.path() == "original_player.0.use_down")
        .map(|f| f.value() == [1])
        .ok_or_else(profile)?;
    Ok(Latches {
        attackdown: w.attack_down,
        usedown,
    })
}
