//! M6a part 2: the finale-and-melt configuration of the session ([`Configuration::FinaleAndMelt`],
//! owner's rulings V1 to V3 of 2026-11-23 and the melt rulings of 2026-11-21), saved as `DSES`
//! version 2 ([`super::state_v2`]).
//!
//! The frozen configuration ([`Configuration::RefusesFinaleAndMelt`], [`Session::new`],
//! [`Session::tic`], `DSES` version 1) keeps every refusal of part 1. This one runs the same
//! session layer over wire 27 (doom-sim's `PlayerMonstersEpisode1V5`, captures DSIM43: wire 26 with
//! the traces' wrapping `P_InterceptVector`, which the real E1M8 needs to reach the victory; the
//! owner's ruling of 2026-11-24, which replaces the "wire 26, DSIM42" of ruling V2; the frozen
//! configuration stays on wire 26 and DSIM42), with doom-render's `SessionConfig::FinaleAndMelt` display (`DDSP` version 4) and doom-sound's
//! `ChainMode::FinaleAndMelt` sound (`DSND` version 3), and adds:
//!
//! - **The victory and the finale** (M6a note §1.8). On the tic after E1M8's exit, `G_Ticker`'s
//!   action loop runs `G_DoCompleted` (g_game.c:1020-1038: `G_PlayerFinishLevel` on the player,
//!   then on map 8 `gameaction = ga_victory` and return, with no `wminfo` and no `WI_Start`), then
//!   in the same loop `F_StartFinale` (g_game.c:639-641; f_finale.c:96-191: `gamestate =
//!   GS_FINALE`, `S_ChangeMusic(mus_victor)`, Episode 1's flat and text, stage 0, count 0), and
//!   the game-state switch at the end of the same `G_Ticker` runs the first `F_Ticker`
//!   (g_game.c:740-742). Every later tic is one `F_Ticker` (f_finale.c:207-249: `finalecount++`;
//!   stage 1 with a forced wipe when `finalecount > 440 * 3 + 250`, f_finale.c:241-248, the
//!   finale's 1,571st tic). The Doom 1 finale reads no command (the skip test is Doom II's,
//!   f_finale.c:212-227) and never ends by itself (quirk Q-f_finale-207; ruling U5): the session's
//!   tic cap (S21) is its only bound. A missing `D_VICTOR` is the genuine stop on the victory tic
//!   (ruling U10 (i)), refused before anything changes. A frame the genuine engine would stop on
//!   (registered mode without `HELP2` at the first stage-1 frame, ruling U4 (3)) is a display
//!   refusal under rule 2: `F_Ticker`'s tic stays committed, the sound and the display stay at the
//!   last good tic (the frame too), and the session takes no further tic.
//! - **The melt of every wipe** (M6a note §1.16; the owner's rulings of 2026-11-21). A tic whose
//!   display wipes leaves the session **in a melt** ([`Session::melt_in_progress`]): the app then
//!   calls [`Session::melt_step`] once per tic of wall time, each one `wipe_ScreenWipe` with
//!   `ticks = 1` (d_main.c:329-344; f_wipe.c:184-222) giving the next picture and its palette;
//!   during melt steps no game tic runs (a tic is refused, [`SessionWitness::MeltInProgress`]),
//!   no command is read or written (a melt step takes none), and no sound is updated or mixed;
//!   the buffer owed by the tic, if one was due, is mixed after the melt's last picture
//!   (`I_UpdateSound` and `I_SubmitSound` after `D_Display`, d_main.c:395-405), and is handed out
//!   with that last step ([`MeltStepReport::buffer`]). A tic whose display does not wipe mixes its
//!   buffer before it returns, so its report carries it as the frozen session's does.
//!
//! **Driving it** (what the app does, one pass of `D_DoomLoop` at a time): call
//! [`Session::tic_finale_and_melt`] with the tic's command; show the report's frame
//! ([`Session::frame`]) under its palette and queue its buffer, if any; then while
//! [`Session::melt_in_progress`], one [`Session::melt_step`] per tic of wall time, each picture
//! shown under [`MeltStepReport::palette`], the last step's [`MeltStepReport::buffer`] queued;
//! then the next tic. Save with [`Session::save_v2`] between any two calls (in a melt too);
//! restore with [`Session::restore_v2`]. Every refusal is typed and leaves the session as before,
//! except the rule-2 display refusal above.
use super::finale;
use super::intermission::Latches;
use super::sound::{self as session_sound, Music};
use super::{
    BT_SPECIAL, CarriedPlayer, PlantedFault, Reader, Refusal, Session, SessionData,
    SessionSettings, SessionWitness, StartError, TicKind, TicReport, command_domain,
    command_domain_wire29, dead_latches,
};
use doom_map::Map;
use doom_render::{Display, FinaleView, Frame, GameMode, GameState, SessionError};
use doom_sim::{
    GameParameters, LevelExit, LevelStart, LevelStartError, MapContextOptions, Simulation, Skill,
    StartKind, TicCommand,
};
use doom_sound::chain::{ChainSound, ChainTic, IntermissionSound};
use doom_sound::chain_melt::{ChainFault, ChainMode};
use doom_sound::mixer::Buffer;
use std::sync::Arc;

/// The `M_Random` draws of one wipe (f_wipe.c:159-166), modulo the 256 entries of the index.
const WIPE_INDEX_STEP: u8 = (320 % 256) as u8;

/// Which session (owner's ruling V1 of 2026-11-23: the new behaviour is a second configuration
/// beside the frozen one, which keeps every refusal).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Configuration {
    /// M6a part 1, frozen ([`Session::new`], [`Session::tic`], `DSES` version 1): the victory,
    /// the finale and the easy skills are typed "not yet" refusals; every wipe's melt takes no
    /// time.
    RefusesFinaleAndMelt,
    /// M6a part 2 ([`Session::new_finale_and_melt`], [`Session::tic_finale_and_melt`],
    /// [`Session::melt_step`], `DSES` version 2): the victory and the Episode 1 finale, and every
    /// wipe's melt one step per tic (module documentation), over wire 27 (DSIM43). The easy
    /// skills stay refused (a later pull request, wire 28).
    FinaleAndMelt,
    /// Wire 28 (decision D3 of 2026-11-25, recorded for the owner;
    /// [`Session::new_finale_and_melt_wire28`], [`Session::tic_finale_and_melt`],
    /// [`Session::melt_step`], `DSES` version 3 in [`super::state_v3`]): [`Self::FinaleAndMelt`]'s
    /// finale, melt, display and sound in every respect, over wire 28 (doom-sim's
    /// `PlayerMonstersEpisode1V6`, captures DSIM44): skills 0 and 1 admitted besides 2 and 3
    /// (ruling U12 (iii); p_mobj.c:743-751, p_inter.c:95-101, 799-800) and spawn rule S'' with
    /// the monster escape witness, a refuse-the-tic stop (M6a note §1.17, option 1).
    /// Nightmare (4) stays refused ([`super::StartError::Domain`]).
    FinaleAndMeltWire28,
    /// Wire 29 (M6b step 6; [`Session::new_finale_and_melt_wire29`],
    /// [`Session::tic_finale_and_melt`], [`Session::melt_step`], `DSES` version 4 in
    /// [`super::state_v4`]): [`Self::FinaleAndMeltWire28`] over wire 29 (doom-sim's
    /// `PlayerMonstersEpisode1V7`, captures DSIM45): skill 4 admitted besides 0 to 3, the `-respawn`
    /// and `-fast` parameters (rulings N1, N10), and the original's pause (rulings N5, N6:
    /// the session owns `paused`; module documentation of [`super`] and [`Session::paused`]).
    FinaleAndMeltWire29,
}

impl Configuration {
    /// doom-sound's configuration of the session's sound.
    pub(crate) fn chain_mode(self) -> ChainMode {
        match self {
            Self::RefusesFinaleAndMelt => ChainMode::RefusesMelt,
            Self::FinaleAndMelt | Self::FinaleAndMeltWire28 | Self::FinaleAndMeltWire29 => {
                ChainMode::FinaleAndMelt
            }
        }
    }

    /// Whether the session shows the finale and melts (both configurations but the frozen one):
    /// its tics run through [`Session::tic_finale_and_melt`] and [`Session::melt_step`].
    pub fn melts(self) -> bool {
        self != Self::RefusesFinaleAndMelt
    }

    /// The skill of the settings' skill byte in this configuration: 2 and 3 in every one, 0
    /// (`sk_baby`) and 1 (`sk_easy`) under wires 28 and 29 only; the two older configurations
    /// refuse them as before (`NotYet::EasySkill`); 4 (`sk_nightmare`) under wire 29 only, and
    /// every other configuration refuses 4 and above (`StartError::Domain`). A byte above 4 is the
    /// demo reader's to clamp (g_game.c:1379-1380), not the session's.
    pub(crate) fn skill(self, byte: u8) -> Result<Skill, StartError> {
        match (self, byte) {
            (Self::FinaleAndMeltWire28 | Self::FinaleAndMeltWire29, 0) => Ok(Skill::Baby),
            (Self::FinaleAndMeltWire28 | Self::FinaleAndMeltWire29, 1) => Ok(Skill::Easy),
            (Self::FinaleAndMeltWire29, 4) => Ok(Skill::Nightmare),
            _ => super::skill_of(byte),
        }
    }

    /// The level of this configuration loaded in the `G_Ticker` whose `gametic` is `gametic`:
    /// wire 26 (`PlayerMonstersEpisode1V4`, DSIM42) for the frozen configuration, wire 27
    /// (`PlayerMonstersEpisode1V5`, DSIM43) for the finale and the melt, wire 28
    /// (`PlayerMonstersEpisode1V6`, DSIM44) for [`Self::FinaleAndMeltWire28`] and wire 29
    /// (`PlayerMonstersEpisode1V7`, DSIM45, with `parameters`) for [`Self::FinaleAndMeltWire29`];
    /// the older configurations take no parameters (always the default).
    pub(crate) fn level_at(
        self,
        map: Map,
        map_number: u8,
        (skill, parameters): (Skill, GameParameters),
        start: LevelStart,
        gametic: u64,
    ) -> Result<Simulation, LevelStartError> {
        let build = match self {
            Self::RefusesFinaleAndMelt => Simulation::original_player_episode1_v4_level_at,
            Self::FinaleAndMelt => Simulation::original_player_episode1_v5_level_at,
            Self::FinaleAndMeltWire28 => Simulation::original_player_episode1_v6_level_at,
            Self::FinaleAndMeltWire29 => {
                return Simulation::original_player_episode1_v7_level_at(
                    map,
                    map_number,
                    skill,
                    parameters,
                    MapContextOptions::default(),
                    start,
                    gametic,
                );
            }
        };
        build(
            map,
            map_number,
            skill,
            MapContextOptions::default(),
            start,
            gametic,
        )
    }

    /// doom-render's configuration of the session display in game mode `mode`.
    pub(crate) fn display_config(self, mode: GameMode) -> doom_render::SessionConfig {
        match self {
            Self::RefusesFinaleAndMelt => doom_render::SessionConfig::RefusesFinaleAndMelt,
            Self::FinaleAndMelt | Self::FinaleAndMeltWire28 | Self::FinaleAndMeltWire29 => {
                doom_render::SessionConfig::FinaleAndMelt { mode }
            }
        }
    }
}

/// What a tic of a finale-and-melt session was: the frozen session's kinds and the two it
/// refuses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FinaleMeltTicKind {
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
    /// `G_DoCompleted` on map 8, `F_StartFinale` and the first `F_Ticker` (finale tic 1).
    Victory,
    /// A later finale tic (one `F_Ticker`).
    Finale,
}

impl FinaleMeltTicKind {
    /// The frozen session's kind of the same tic (None for the two kinds it refuses).
    pub fn frozen(self) -> Option<TicKind> {
        Some(match self {
            Self::Level => TicKind::Level,
            Self::Load { map, start } => TicKind::Load { map, start },
            Self::Completion => TicKind::Completion,
            Self::Intermission => TicKind::Intermission,
            Self::Victory | Self::Finale => return None,
        })
    }

    fn of(kind: TicKind) -> Self {
        match kind {
            TicKind::Level => Self::Level,
            TicKind::Load { map, start } => Self::Load { map, start },
            TicKind::Completion => Self::Completion,
            TicKind::Intermission => Self::Intermission,
        }
    }
}

/// What one tic of a finale-and-melt session presented: everything the frozen session's
/// [`TicReport`] holds, with the same meaning, and the finale's state.
#[derive(Clone, Debug)]
pub struct FinaleMeltTicReport {
    /// The session tic (`gametic` before it).
    pub tic: u64,
    /// What the tic was.
    pub kind: FinaleMeltTicKind,
    /// The frame's palette number.
    pub palette: u8,
    /// The palette `I_SetPalette` received in the frame.
    pub palette_set: Option<u8>,
    /// The display wiped after the frame: its 320 `M_Random` draws are made and **the session is
    /// now in a melt** ([`Session::melt_in_progress`]).
    pub wiped: bool,
    /// A level frame's report (None outside a level).
    pub level_frame: Option<doom_render::DisplayFrame>,
    /// The intermission's animation draws of `M_Random`.
    pub wi_draws: u16,
    /// The intermission's sound starts, in call order.
    pub wi_starts: Vec<IntermissionSound>,
    /// What the sound did. `sound.buffer` is the buffer mixed in this call: the tic's due buffer
    /// when its display did not melt (as in the frozen session); None when it melts (the buffer,
    /// if one was due, comes with the melt's last step, [`MeltStepReport::buffer`]).
    pub sound: ChainTic,
    /// `finalestage` and `finalecount` after the tic (None outside the finale).
    pub finale: Option<FinaleView>,
    /// [`Configuration::FinaleAndMeltWire29`]: `paused` after the tic's `G_Ticker`, so that its
    /// frame is one the C draws `M_PAUSE` over (d_main.c:303-311). **Temporary (M6b step 7 draws
    /// the picture):** this frame lacks it. Always false in the other configurations.
    pub paused: bool,
}

impl FinaleMeltTicReport {
    fn of(r: TicReport) -> Self {
        Self {
            tic: r.tic,
            kind: FinaleMeltTicKind::of(r.kind),
            palette: r.palette,
            palette_set: r.palette_set,
            wiped: r.wiped,
            level_frame: r.level_frame,
            wi_draws: r.wi_draws,
            wi_starts: r.wi_starts,
            sound: r.sound,
            finale: None,
            paused: false,
        }
    }
}

/// What one melt step showed ([`Session::melt_step`]).
#[derive(Clone, Debug, PartialEq)]
pub struct MeltStepReport {
    /// The session tic whose display melts (`gametic` - 1).
    pub tic: u64,
    /// The melt step just shown, from 1.
    pub step: u8,
    /// `wipe_doMelt`'s `done`: the last step, which ends the melt (its picture equals the tic's
    /// composed frame, as does the one before it).
    pub done: bool,
    /// The PLAYPAL index every picture of the melt is shown under (the end frame's, M6a note
    /// §1.16.3).
    pub palette: u8,
    /// On the last step: the tic's owed buffer, mixed now (None when the tic had no buffer due,
    /// and on every other step).
    pub buffer: Option<Buffer>,
}

impl Session {
    /// A new game in the finale-and-melt configuration (module documentation): as
    /// [`Session::new`], with the display in doom-render's `SessionConfig::FinaleAndMelt` (the
    /// declared game mode picks the finale's art screen) and the sound in
    /// `ChainMode::FinaleAndMelt`. Skills 0 and 1 are refused as the frozen session refuses them.
    pub fn new_finale_and_melt(
        data: Arc<SessionData>,
        settings: SessionSettings,
    ) -> Result<Self, StartError> {
        Self::new_in(data, settings, Configuration::FinaleAndMelt)
    }

    /// A new game in the wire 28 configuration ([`Configuration::FinaleAndMeltWire28`]): as
    /// [`Session::new_finale_and_melt`], with the levels on wire 28 and skills 0 to 3 admitted.
    pub fn new_finale_and_melt_wire28(
        data: Arc<SessionData>,
        settings: SessionSettings,
    ) -> Result<Self, StartError> {
        Self::new_in(data, settings, Configuration::FinaleAndMeltWire28)
    }

    /// A new game in the wire 29 configuration ([`Configuration::FinaleAndMeltWire29`]): as
    /// [`Session::new_finale_and_melt_wire28`] with the levels on wire 29, skills 0 to 4 admitted
    /// and the `-respawn` and `-fast` `parameters`, constant for the session as `respawnparm` and
    /// `fastparm` are for a game (g_game.c:1416-1428).
    pub fn new_finale_and_melt_wire29(
        data: Arc<SessionData>,
        settings: SessionSettings,
        parameters: GameParameters,
    ) -> Result<Self, StartError> {
        Self::new_in_with(
            data,
            settings,
            Configuration::FinaleAndMeltWire29,
            parameters,
        )
    }

    /// `respawnparm` and `fastparm` (the default outside [`Configuration::FinaleAndMeltWire29`]).
    pub fn parameters(&self) -> GameParameters {
        self.parameters
    }

    /// `paused` after the last tic (always false outside [`Configuration::FinaleAndMeltWire29`]).
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// The configuration.
    pub fn configuration(&self) -> Configuration {
        self.config
    }

    /// `finalestage` and `finalecount` (None outside the finale).
    pub fn finale(&self) -> Option<FinaleView> {
        self.finale
    }

    /// Whether the last tic's display started a melt that has not ended: the next call must be
    /// [`Self::melt_step`] (always false in the frozen configuration).
    pub fn melt_in_progress(&self) -> bool {
        self.display.melt_in_progress()
    }

    /// Runs one session tic of the finale-and-melt configuration with `cmd` (module
    /// documentation; the frozen [`Session::tic`]'s order, plus the victory and the finale).
    /// Refused, with nothing changed, in the frozen configuration
    /// ([`SessionWitness::Configuration`]) and during a melt ([`SessionWitness::MeltInProgress`]).
    pub fn tic_finale_and_melt(&mut self, cmd: TicCommand) -> Result<FinaleMeltTicReport, Refusal> {
        let tic = self.gametic;
        let refuse = |witness| Refusal { tic, witness };
        if !self.config.melts() {
            return Err(refuse(SessionWitness::Configuration(self.config)));
        }
        if self.ended.is_some() {
            return Err(refuse(SessionWitness::Ended));
        }
        if self.display.melt_in_progress() {
            if self.fault != Some(PlantedFault::TicInMelt) {
                return Err(refuse(SessionWitness::MeltInProgress));
            }
            let mut unseen = Frame::new();
            while self.display.melt_in_progress() {
                self.melt_step(&mut unseen)?;
            }
        }
        if self.config == Configuration::FinaleAndMeltWire29 {
            return self.tic_wire29(cmd).map_err(refuse);
        }
        let check = |reader| command_domain(&cmd, reader);
        let frozen = FinaleMeltTicReport::of;
        let r = match self.gamestate() {
            GameState::Level => {
                if self.level.reborn_requested() {
                    let p_random = self.level.rng().indices()[0];
                    check(Reader::Level)
                        .and_then(|()| {
                            self.load_tic(
                                cmd,
                                self.map,
                                doom_sim::LevelStart::Reborn { p_random },
                                doom_render::LevelChange::Reborn,
                                0,
                                false,
                            )
                        })
                        .map(frozen)
                } else if let Some(exit) = self.level.level_exit_request() {
                    check(Reader::NonLevel).and_then(|()| {
                        if self.map == 8 {
                            self.victory_tic(exit)
                        } else {
                            self.completion_tic(cmd, exit).map(frozen)
                        }
                    })
                } else {
                    check(Reader::Level)
                        .and_then(|()| self.level_tic(cmd, false))
                        .map(frozen)
                }
            }
            GameState::Intermission if self.world_done => check(Reader::Level)
                .and_then(|()| self.world_done_tic(cmd, false))
                .map(frozen),
            GameState::Intermission => check(Reader::NonLevel)
                .and_then(|()| self.intermission_tic(cmd))
                .map(frozen),
            GameState::Finale => check(Reader::NonLevel).and_then(|()| self.finale_tic()),
        };
        r.map_err(refuse)
    }

    /// One tic of [`Configuration::FinaleAndMeltWire29`]: [`Self::tic_finale_and_melt`]'s order
    /// with `G_Ticker`'s special buttons between the game actions and the game-state switch
    /// (g_game.c:605-748; the pause in [`super`]'s module documentation). `paused` changes only
    /// with a tic that succeeds.
    fn tic_wire29(&mut self, cmd: TicCommand) -> Result<FinaleMeltTicReport, SessionWitness> {
        let fault = self.fault;
        let state = self.gamestate();
        let reborn = state == GameState::Level && self.level.reborn_requested();
        let exit = (state == GameState::Level && !reborn)
            .then(|| self.level.level_exit_request())
            .flatten();
        let loads = reborn || (state == GameState::Intermission && self.world_done);
        // G_DoLoadLevel clears `paused` (g_game.c:494) before the special buttons (698-724).
        let before = self.paused && !(loads && fault != Some(PlantedFault::PauseKeep));
        let toggle = if fault == Some(PlantedFault::PauseMask) {
            cmd.buttons == 0x81
        } else {
            doom_sim::toggles_pause(cmd.buttons)
        };
        let paused = before != toggle;
        // P_Ticker runs in a level tic and on a load tic's new level, unless paused.
        let level = (state == GameState::Level && exit.is_none()) || loads;
        let hold = paused && fault != Some(PlantedFault::PauseRun);
        command_domain_wire29(&cmd, level && !hold)?;
        let wi_cmd = if fault == Some(PlantedFault::PauseWi) && cmd.buttons & BT_SPECIAL != 0 {
            TicCommand {
                buttons: cmd.buttons & !1,
                ..cmd
            }
        } else {
            cmd
        };
        let frozen = FinaleMeltTicReport::of;
        let mut r = if reborn {
            let p_random = self.level.rng().indices()[0];
            let start = LevelStart::Reborn { p_random };
            let change = doom_render::LevelChange::Reborn;
            self.load_tic(cmd, self.map, start, change, 0, hold)
                .map(frozen)?
        } else if let Some(exit) = exit {
            if self.map == 8 {
                self.victory_tic(exit)?
            } else {
                self.completion_tic(wi_cmd, exit).map(frozen)?
            }
        } else if state == GameState::Level {
            self.level_tic(cmd, hold).map(frozen)?
        } else if loads {
            self.world_done_tic(cmd, hold).map(frozen)?
        } else if state == GameState::Intermission {
            self.intermission_tic(wi_cmd).map(frozen)?
        } else {
            self.finale_tic()?
        };
        self.paused = paused;
        self.shown_paused |= paused;
        r.paused = paused;
        Ok(r)
    }

    /// One melt step (`wipe_ScreenWipe` with `ticks = 1`, d_main.c:335-340; f_wipe.c:184-222):
    /// the next picture into `out` (64,000 palette indices), shown under the melt's palette; on
    /// the last step the melt ends and the tic's owed buffer is mixed (d_main.c:397-405). No tic
    /// runs, no command is read and no sound is updated (d_main.c:329-344). Refused, with nothing
    /// changed: in the frozen configuration ([`SessionWitness::Configuration`]), after a rule-2
    /// refusal ([`SessionWitness::Ended`]) and with no melt in progress
    /// ([`SessionWitness::NoMelt`]).
    pub fn melt_step(&mut self, out: &mut Frame) -> Result<MeltStepReport, Refusal> {
        let tic = self.gametic;
        let refuse = |witness| Refusal { tic, witness };
        if !self.config.melts() {
            return Err(refuse(SessionWitness::Configuration(self.config)));
        }
        if self.ended.is_some() {
            return Err(refuse(SessionWitness::Ended));
        }
        if !self.display.melt_in_progress() {
            return Err(refuse(SessionWitness::NoMelt));
        }
        // Preflight: the sound's display is pending exactly while a melt is in progress (the
        // session mixes when the melt ends), so neither call below can refuse.
        let mut sound = self.sound.clone();
        sound
            .melt_step()
            .map_err(|e| refuse(SessionWitness::Sound(e.into())))?;
        let shown = self
            .display
            .melt_step(out)
            .map_err(|e| refuse(SessionWitness::Display(e)))?;
        let buffer = if shown.done {
            sound
                .mix_after_display()
                .map_err(|e| refuse(SessionWitness::Sound(e.into())))?
        } else {
            None
        };
        self.sound = sound;
        Ok(MeltStepReport {
            tic: tic.saturating_sub(1),
            step: shown.step,
            done: shown.done,
            palette: shown.palette,
            buffer,
        })
    }

    /// In the finale-and-melt configuration, after the frame: when the display did not start a
    /// melt, the tic's owed buffer is mixed now and put in the report (the order of
    /// d_main.c:392-405 with no melt in between). The frozen configuration mixes inside the tic.
    pub(super) fn mix_unless_melting(
        &self,
        display: &Display,
        sound: &mut ChainSound,
        ts: &mut ChainTic,
    ) -> Result<(), SessionWitness> {
        if self.config.melts() && !display.melt_in_progress() {
            ts.sound.buffer = sound
                .mix_after_display()
                .map_err(|e| SessionWitness::Sound(e.into()))?;
        }
        Ok(())
    }

    /// `G_DoCompleted` on map 8 (g_game.c:1020-1038), `F_StartFinale` (g_game.c:639-641;
    /// f_finale.c:96-191) and the first `F_Ticker` (g_game.c:740-742): finale tic 1. Everything
    /// on the side, committed at the end; every refusal leaves the session as before.
    fn victory_tic(&mut self, exit: LevelExit) -> Result<FinaleMeltTicReport, SessionWitness> {
        // G_PlayerFinishLevel (g_game.c:779-792) on the player, as for an intermission; on map 8
        // no `wminfo` is written and `WI_Start` does not run (g_game.c:1033-1038).
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
        // F_StartFinale: S_ChangeMusic(mus_victor) (f_finale.c:114) stops the genuine engine when
        // D_VICTOR is missing (ruling U10 (i)).
        if session_sound::music_lookup_fails(&self.sound, Music::Victor) {
            return Err(self.music_missing(Music::Victor));
        }
        let mut display = self.display.clone();
        display.begin_finale().map_err(SessionWitness::Display)?;
        let entered = display.clone();
        // `gameepisode` is 1 in every session (Episode 1).
        let mut view = finale::start(1).map_err(SessionWitness::Finale)?;
        if self.fault != Some(PlantedFault::FinaleLate) {
            // On the victory tic stage 1 cannot begin (finalecount becomes 1).
            finale::tick(&mut view).map_err(SessionWitness::Finale)?;
        }
        let mut sound = self.sound.clone();
        let ts = session_sound::finale_tic(&mut sound, display.random_index(), &self.level)
            .map_err(SessionWitness::Sound)?;
        let r = self.present_finale(display, sound, view, ts, FinaleMeltTicKind::Victory);
        if self.ended.is_some() {
            // Rule 2 here (only a refused frame ends the session, and this tic began not ended):
            // G_DoCompleted and F_StartFinale ran (g_game.c:636-641, f_finale.c:99).
            self.display = entered;
        } else if r.is_err() {
            return r;
        }
        self.secretexit = exit == LevelExit::Secret;
        self.carried = Some(carried);
        r
    }

    /// A finale tic after the first: one `F_Ticker` (f_finale.c:207-249), its forced wipe at
    /// stage 1 (f_finale.c:241-248), the sound and `D_Display`.
    fn finale_tic(&mut self) -> Result<FinaleMeltTicReport, SessionWitness> {
        let mut view = self
            .finale
            .ok_or(SessionWitness::Display(SessionError::WrongGameState(
                GameState::Finale,
            )))?;
        let mut tick = finale::tick(&mut view).map_err(SessionWitness::Finale)?;
        if self.fault == Some(PlantedFault::NoArtWipe) {
            tick.wipe = false;
        }
        let mut display = self.display.clone();
        if tick.wipe {
            // wipegamestate = -1 (f_finale.c:245).
            display
                .begin_finale_art()
                .map_err(SessionWitness::Display)?;
        }
        let mut sound = self.sound.clone();
        let ts = session_sound::finale_tic(&mut sound, display.random_index(), &self.level)
            .map_err(SessionWitness::Sound)?;
        self.present_finale(display, sound, view, ts, FinaleMeltTicKind::Finale)
    }

    /// The display of a finale tic (`F_Drawer`, d_main.c:254-256) and its commit. A frame
    /// refused by the display is rule 2 (the finale's state committed, the sound and the display
    /// at the last good tic, the session ended: ruling U4 (3)), the crawl's (stage 0) witnessed as
    /// [`SessionWitness::FinaleText`]; [`Self::victory_tic`] also commits `F_StartFinale`'s.
    #[allow(
        clippy::too_many_arguments,
        reason = "the staged parts of one tic, all moved once"
    )]
    fn present_finale(
        &mut self,
        mut display: Display,
        mut sound: ChainSound,
        view: FinaleView,
        mut ts: ChainTic,
        kind: FinaleMeltTicKind,
    ) -> Result<FinaleMeltTicReport, SessionWitness> {
        // The Episode 1 finale starts no sound and draws no random number (M6a note §1.8), so
        // the sound's draws are the update's (none); they are checked as in every other tic.
        let before = display.random_index();
        let expected = before.wrapping_add((ts.sound.draws & 0xff) as u8);
        if expected != ts.sound.rndindex {
            return Err(SessionWitness::RandomIndex {
                expected,
                display: ts.sound.rndindex,
            });
        }
        display
            .observe_non_level(ts.sound.draws)
            .map_err(SessionWitness::Display)?;
        // Due after `begin_finale` (the state changed) or `begin_finale_art` (`tick.wipe`).
        let wiped = display.wipe_due();
        let drawn = display.draw_finale(&view, &mut self.frame);
        let f = match drawn {
            Ok(f) => f,
            Err(e) => {
                let w = match e {
                    SessionError::Finale(r) if view.stage == 0 => SessionWitness::FinaleText(r),
                    e => SessionWitness::Display(e),
                };
                // Rule 2: F_Ticker ran in the C before D_Display stopped (d_main.c:383-395).
                self.finale = Some(view);
                self.ended = Some(Refusal {
                    tic: self.gametic,
                    witness: w.clone(),
                });
                return Err(w);
            }
        };
        let after = ts
            .sound
            .rndindex
            .wrapping_add(if wiped { WIPE_INDEX_STEP } else { 0 });
        if display.random_index() != after {
            return Err(SessionWitness::RandomIndex {
                expected: after,
                display: display.random_index(),
            });
        }
        self.mix_unless_melting(&display, &mut sound, &mut ts)?;
        self.display = display;
        self.sound = sound;
        self.finale = Some(view);
        self.gametic += 1;
        Ok(FinaleMeltTicReport {
            tic: self.gametic - 1,
            kind,
            palette: f.palette,
            palette_set: f.palette_set,
            wiped,
            level_frame: None,
            wi_draws: 0,
            wi_starts: Vec::new(),
            sound: ts,
            finale: Some(view),
            paused: false,
        })
    }

    /// The fault the session forwards to its sound (`PlantedFault::MixInMelt`).
    pub(super) fn plant_sound_fault(&mut self, fault: PlantedFault) {
        if fault == PlantedFault::MixInMelt {
            self.sound.plant_fault(Some(ChainFault::MixInMelt));
        }
    }
}
