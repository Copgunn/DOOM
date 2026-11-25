//! The session demo across levels (ruling U7; M6a note §1.10, §4.7): one version-110 demo per
//! session, its header written once with the start map and the skill, then one command for every
//! session tic in every game state, intermission tics included (`G_Ticker` reads or writes a
//! command for every player before the game-state switch, g_game.c:656-670; there is no per-level
//! header or marker). The recording is the app's existing [`crate::app::Recorder`] unchanged:
//! started with the session's start map and skill it writes exactly that file, and it never
//! records `BT_SPECIAL` (its command domain is wire 24's). This module adds the reading side
//! ([`SessionDemo::parse`], which takes the header's map as the start and does not stop at a level
//! boundary; the single-level `DemoCommands::parse` and its files are unchanged), the replay
//! ([`replay`], the same session layer to the demo's end or a refusal, a replayed `BT_SPECIAL`
//! refused there), and the game mode's place on the file (ruling U4).
//!
//! **Where the game mode is recorded.** The demo header has no room for it (version, skill,
//! episode, map, four option bytes, the console player and `playeringame`, g_game.c:1555-1566).
//! It is recorded twice: as a declared field of the saved session (`DSES` v1) and as a token in
//! the session demo's file name, [`session_mode_name`]
//! (`E1M<map>-skill<n>-<retail|registered>-<yyyymmdd-hhmmss>.lmp`), next to the unchanged
//! single-level [`crate::app::session_name`]. A replay or check tool takes the mode from the token
//! ([`mode_of_session_name`]); an explicit flag, when given, must agree with it
//! ([`replay_mode`]), or the check refuses.
use super::finale_melt::{FinaleMeltTicReport, MeltStepReport};
use super::{Refusal, Session, SessionMode, TicReport};
use crate::app::Timestamp;
use doom_render::Frame;
use doom_sim::TicCommand;
use doom_sim::demo::{
    DEMO_HEADER, DEMO_MAP_INDEX, DemoError, EPISODE1_DEMO_MAPS, parse_demo_episode1_v2,
    write_demo_episode1_v2,
};

/// A parsed session demo.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SessionDemo {
    /// The start map (the header's map byte, g_game.c:1592).
    pub map: u8,
    /// The skill byte (g_game.c:1590).
    pub skill: u8,
    /// One command per session tic, in order.
    pub commands: Vec<TicCommand>,
}

impl SessionDemo {
    /// Reads a session demo: the header's map (1..=9) is the start, the skill 2 or 3 (the easy
    /// skills are M6a step 10), every other header byte as `DEMO_HEADER`, then the commands to
    /// the marker (`G_ReadDemoTiccmd`, g_game.c:1491-1503), with no level boundary. The commands'
    /// buttons are not judged here: a replayed `BT_SPECIAL` is refused at its tic by the session.
    pub fn parse(bytes: &[u8]) -> Result<Self, DemoError> {
        let header = bytes.get(..DEMO_HEADER.len()).ok_or(DemoError::Truncated)?;
        let map = header[DEMO_MAP_INDEX];
        if !EPISODE1_DEMO_MAPS.contains(&map) {
            return Err(DemoError::Header {
                index: DEMO_MAP_INDEX,
                value: map,
            });
        }
        let (h, demo) = parse_demo_episode1_v2(bytes, map)?;
        Ok(Self {
            map: h.map,
            skill: h.skill,
            commands: demo.commands,
        })
    }

    /// The demo's bytes, written by doom-sim's writer (the recorder's): refused as it refuses
    /// (`BT_SPECIAL`, a forward or side of -128, a turn with a low byte).
    pub fn bytes(&self) -> Result<Vec<u8>, DemoError> {
        write_demo_episode1_v2(self.map, self.skill, &self.commands)
    }
}

/// The file name of a session demo (ruling U4, module documentation):
/// `E1M<map>-skill<n>-<retail|registered>-<yyyymmdd-hhmmss>.lmp`.
pub fn session_mode_name(map: u8, skill: u8, mode: SessionMode, t: Timestamp) -> String {
    format!(
        "E1M{map}-skill{skill}-{}-{:04}{:02}{:02}-{:02}{:02}{:02}.lmp",
        mode.word(),
        t.year,
        t.month,
        t.day,
        t.hour,
        t.minute,
        t.second
    )
}

/// The game mode a session demo's file name declares: the token after `skill<n>-`, which must be
/// exactly `retail` or `registered`; `None` for any other name (a single-level session file has
/// no token).
pub fn mode_of_session_name(name: &str) -> Option<SessionMode> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let mut parts = base.split('-');
    let (map, skill, mode) = (parts.next()?, parts.next()?, parts.next()?);
    let map_ok = map.len() == 4 && map.starts_with("E1M");
    let skill_ok = skill.len() == 6 && skill.starts_with("skill");
    if !(map_ok && skill_ok) {
        return None;
    }
    match mode {
        "retail" => Some(SessionMode::Retail),
        "registered" => Some(SessionMode::Registered),
        _ => None,
    }
}

/// Why a replay tool cannot choose the game mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModeTokenError {
    /// The file name has no mode token and no flag was given.
    Missing,
    /// The flag disagrees with the file name's token.
    Disagrees {
        /// The token's mode.
        token: SessionMode,
        /// The flag's mode.
        flag: SessionMode,
    },
}

/// The mode a replay or check of the session demo `name` uses: the token, and an explicit `flag`
/// only when it agrees with the token (or when the name has no token).
pub fn replay_mode(name: &str, flag: Option<SessionMode>) -> Result<SessionMode, ModeTokenError> {
    match (mode_of_session_name(name), flag) {
        (Some(token), Some(flag)) if token != flag => {
            Err(ModeTokenError::Disagrees { token, flag })
        }
        (Some(token), _) => Ok(token),
        (None, Some(flag)) => Ok(flag),
        (None, None) => Err(ModeTokenError::Missing),
    }
}

/// Replays `commands` through a finale-and-melt `session` (a fresh one of the demo's start map
/// and skill, [`Session::new_finale_and_melt`]): one command per session tic in every game state,
/// the finale's included (g_game.c:656-670), and after each tic every step of the melt its
/// display started, which reads no command (the melt loop runs no `G_Ticker`, d_main.c:329-344;
/// M6a note §1.16.5). Each committed tic goes to `on_tic` and each melt step, with its picture,
/// to `on_melt_step`. Ends at the demo's end (`None`) or the first refusal.
pub fn replay_finale_and_melt(
    session: &mut Session,
    commands: &[TicCommand],
    mut on_tic: impl FnMut(&Session, &FinaleMeltTicReport),
    mut on_melt_step: impl FnMut(&Session, &MeltStepReport, &Frame),
) -> Option<Refusal> {
    let mut picture = Frame::new();
    for &cmd in commands {
        match session.tic_finale_and_melt(cmd) {
            Ok(r) => on_tic(session, &r),
            Err(e) => return Some(e),
        }
        while session.melt_in_progress() {
            match session.melt_step(&mut picture) {
                Ok(m) => on_melt_step(session, &m, &picture),
                Err(e) => return Some(e),
            }
        }
    }
    None
}

/// Replays `commands` through `session` (a fresh session of the demo's start map and skill),
/// handing each committed tic to `on_tic`: the same session layer as play, to the demo's end
/// (`None`) or the first refusal (the session keeps its state before the refused tic, except
/// after a rule-2 refusal, `Session::tic`).
pub fn replay(
    session: &mut Session,
    commands: &[TicCommand],
    mut on_tic: impl FnMut(&Session, &TicReport),
) -> Option<Refusal> {
    for &cmd in commands {
        match session.tic(cmd) {
            Ok(r) => on_tic(session, &r),
            Err(e) => return Some(e),
        }
    }
    None
}
