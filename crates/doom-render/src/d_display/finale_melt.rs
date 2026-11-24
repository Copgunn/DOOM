//! M6a part 2: the finale-and-melt configuration of the session display
//! ([`SessionConfig::FinaleAndMelt`], owner rulings V1, V3 and V4 of 2026-11-23), saved as
//! `DDSP` version 4 ([`SessionDisplayStateV4`]).
//!
//! Everything the frozen configuration ([`SessionConfig::RefusesFinaleAndMelt`]) does is done the
//! same way here, with the same `M_Random` accounting (each wipe's 320 draws after the frame of
//! the first tic in the new state, M6a note §1.11 and §4.5). What this configuration adds:
//!
//! - **The finale drawer.** [`Display::draw_finale`] runs `F_Drawer` for Episode 1
//!   ([`crate::f_finale::draw`], f_finale.c:700-736) over the persistent `screens[0]`: stage 0 the
//!   text crawl, stage 1 the art screen of the declared game mode (f_finale.c:714-720). A frame
//!   the genuine engine would stop on (a missing `HELP2` in registered mode, w_wad.c:399-409) is
//!   the typed refusal [`SessionError::Finale`]: the display and the last good frame are unchanged
//!   (owner ruling U4 (3)).
//! - **The melt of every wipe** (M6a note §1.16, the owner's melt rulings of 2026-11-21). A
//!   display that wipes (`gamestate != wipegamestate`, d_main.c:223) reads the start screen before
//!   drawing (`wipe_StartScreen`, d_main.c:226, f_wipe.c:237-245: the persistent `screens[0]`, all
//!   zeros before the session's first frame, §1.16.2), composes the frame, takes it as the end
//!   screen (`wipe_EndScreen`, d_main.c:327, f_wipe.c:249-258) and runs `wipe_initMelt`'s 320
//!   `M_Random` draws (f_wipe.c:156-166) at the index the frame leaves. The display is then **in a
//!   melt**: each [`Display::melt_step`] is one `wipe_ScreenWipe` call with `ticks = 1`
//!   (d_main.c:329-344; f_wipe.c:184-222; §1.16.1), and the call that returns `done` ends it, its
//!   picture equal to the end screen, as is the one before it. While a melt is in progress no tic
//!   is observed and no frame is drawn (the melt loop runs no `G_Ticker` and no drawer but
//!   `M_Drawer`, which draws nothing without a menu, d_main.c:331-344): those calls are refused
//!   ([`SessionError::MeltInProgress`], [`DisplayObserveError::MeltInProgress`],
//!   [`crate::DisplayRefusal::MeltInProgress`], [`crate::ConfigError::MeltInProgress`]) and change
//!   nothing. Every picture of a melt is shown under one palette, the one in force at the end of
//!   the display that started it (§1.16.3): the level frame's `st_palette` (st_stuff.c:1045-1050),
//!   or 0 outside a level (`I_SetPalette (PLAYPAL)` on entering the state, d_main.c:274-275, and
//!   nothing after it in the finale's own text-to-art wipe).
//! - **`DDSP` version 4**, which holds a melt in progress ([`MeltProgress`]: both screens, the
//!   start screen's taint, the 160 column positions, the steps shown and the palette) and the
//!   finale scene, beside the declared game mode. Versions 1 to 3 keep their bytes and readers.
//!
//! [`DisplayObserveError::MeltInProgress`]: crate::DisplayObserveError::MeltInProgress
use super::{
    Carried, GameState, NonLevelFrame, NotYet, PIXELS, Reader, Scene, SessionConfig,
    SessionDisplayState, SessionError, SessionRestoreError, write_runs,
};
use crate::d_display::{
    Display, DisplayConfig, DisplayRestoreError, MAGIC, WIPE_DRAWS, advanced, read_tail, write_tail,
};
use crate::f_wipe::{MELT_COLUMNS, Melt, melt_offsets};
use crate::state::WallSteps;
use crate::{Frame, GameMode, Level, RenderState, Resources, StateDecodeError};
use doom_sim::Simulation;
use std::sync::Arc;

/// `DDSP` version 4: the session display state of [`SessionConfig::FinaleAndMelt`].
pub const VERSION_FINALE_MELT: u32 = 4;

/// The most melt steps a saved melt in progress can have shown: a melt has N = 28 + max(-y0)
/// steps with every `y0` in -15..=0 (f_wipe.c:158-166, 196-208; M6a note §1.16.1), so N <= 43,
/// and the step that returns `done` ends the melt.
pub const MELT_MAX_SHOWN: u8 = 42;

/// The `M_Random` index moves by 320 mod 256 over `wipe_initMelt` (m_random.c:63-67).
const WIPE_INDEX_STEP: u8 = (WIPE_DRAWS & 0xff) as u8;

/// A melt in progress inside the display.
#[derive(Clone, Debug)]
pub(crate) struct MeltRun {
    melt: Melt,
    start_taint: Vec<u32>,
    end_taint: Vec<u32>,
    palette: u8,
    step: u8,
}

/// A fault planted by the evidence self-tests, to prove that the comparison with the genuine
/// melt goldens fails when the display is wrong. Never set by the app; a display holding one
/// cannot be saved ([`Display::session_state_v4`] gives `None`).
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeltFault {
    /// Column pair 80's initial offset one tic shorter (or, at 0, longer) at every melt.
    ColumnOffByOne,
    /// The fifth melt step of every melt not shown (two `wipe_doMelt` steps in one call).
    DroppedStep,
    /// Every melt shown under the next palette (mod 14) instead of the end frame's.
    WrongPalette,
}

/// What one [`Display::melt_step`] showed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeltStep {
    /// The melt step just shown, from 1.
    pub step: u8,
    /// `wipe_doMelt`'s `done` (f_wipe.c:184, 194, 217, 222): true on the last step, which ends
    /// the melt.
    pub done: bool,
    /// The PLAYPAL index the picture is shown under (the same for every step of a melt).
    pub palette: u8,
}

/// Why a checked session display state was refused ([`Display::session_state_v3_checked`],
/// [`Display::session_state_v4_checked`]). Nothing is changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionStateRefusal {
    /// The display is not a session display.
    NotSession,
    /// No level has been started ([`Display::begin_level`]).
    NotStarted,
    /// A [`SessionConfig::FinaleAndMelt`] display is saved as `DDSP` version 4 only.
    FinaleAndMeltSavesVersion4,
    /// A [`SessionConfig::RefusesFinaleAndMelt`] display is saved as `DDSP` version 3 only.
    RefusesFinaleAndMeltSavesVersion3,
    /// The display holds a planted fault (evidence self-tests only).
    PlantedFault,
}

impl std::fmt::Display for SessionStateRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "session display state refused: {self:?}")
    }
}

impl std::error::Error for SessionStateRefusal {}

/// A melt in progress in a saved state (`DDSP` version 4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeltProgress {
    /// Melt steps already shown, 0..=[`MELT_MAX_SHOWN`] (0: the display that started the melt
    /// has run, no step yet).
    pub step: u8,
    /// The palette of every picture of the melt, 0..=13.
    pub palette: u8,
    /// The 160 column positions `y[]` after `step` steps (f_wipe.c:138, 192-216): -15..=200.
    pub columns: [i32; MELT_COLUMNS],
    /// The start screen (`wipe_scr_start`), 64,000 palette indices.
    pub start: Vec<u8>,
    /// The start screen's taint (decision R4).
    pub start_taint: Vec<u32>,
    /// The end screen (`wipe_scr_end`), 64,000 palette indices: the scene's composed frame.
    pub end: Vec<u8>,
}

/// The saved state of a finale-and-melt session display, `DDSP` version 4 (docs/STATE_FORMAT.md;
/// the layout is in [`SessionDisplayStateV4::encode`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionDisplayStateV4 {
    /// The declared game mode (it picks the finale's art screen, f_finale.c:714-720).
    pub mode: GameMode,
    /// Everything version 3 holds, with the same meaning and rules. Its `melt` field is version
    /// 3's and must be `None` (not written; [`Self::check`] refuses a state with one).
    pub session: SessionDisplayState,
    /// A melt in progress.
    pub melt: Option<MeltProgress>,
}

impl Display {
    /// A session display in the configuration `session` over loaded resources and the first
    /// level: [`SessionConfig::RefusesFinaleAndMelt`] is exactly [`Self::new_session`];
    /// [`SessionConfig::FinaleAndMelt`] draws the finale and the melts (module documentation).
    /// Start it with [`Self::begin_level`].
    pub fn new_session_with(
        res: Arc<Resources>,
        level: Arc<Level>,
        cfg: DisplayConfig,
        session: SessionConfig,
    ) -> Result<Self, crate::ResourceError> {
        let mut d = Self::new_session(res, level, cfg)?;
        if let Some(s) = d.session.as_deref_mut() {
            s.config = session;
        }
        Ok(d)
    }

    /// The configuration of a session display (None for a display that is not one).
    pub fn session_config(&self) -> Option<SessionConfig> {
        self.session.as_deref().map(|s| s.config)
    }

    /// The start screen and its taint when this display wipes and melts: the persistent
    /// `screens[0]` before the frame (d_main.c:223-227).
    pub(crate) fn melt_start(&self) -> Option<(Vec<u8>, Vec<u32>)> {
        let s = self.session.as_deref()?;
        if !self.wipe || !matches!(s.config, SessionConfig::FinaleAndMelt { .. }) {
            return None;
        }
        match (s.gamestate, &s.carried) {
            (GameState::Level, _) => self
                .renderer
                .state()
                .map(|r| (r.screen.clone(), r.taint.clone())),
            (_, Some(c)) => Some((c.screen.clone(), c.taint.clone())),
            (_, None) => None,
        }
    }

    /// The scene's composed frame and its taint (the end screen of a melt).
    fn scene_screen(&self) -> Option<(&[u8], &[u32])> {
        let s = self.session.as_deref()?;
        match (s.gamestate, &s.carried) {
            (GameState::Level, _) => self
                .renderer
                .state()
                .map(|r| (r.screen.as_slice(), r.taint.as_slice())),
            (_, Some(c)) => Some((c.screen.as_slice(), c.taint.as_slice())),
            (_, None) => None,
        }
    }

    /// `wipe_EndScreen` and `wipe_initMelt` after a frame that wiped: the end screen is the frame
    /// just composed, the 320 draws start at `index` (the index the frame left; the caller has
    /// already advanced the display's index past them).
    pub(crate) fn begin_melt(
        &mut self,
        start: Vec<u8>,
        start_taint: Vec<u32>,
        palette: u8,
        index: u8,
    ) {
        // Every caller has just composed the scene's frame, so the scene has a screen.
        let (end, end_taint) = self.scene_screen().expect("a composed scene");
        let end_taint = end_taint.to_vec();
        let fault = self.session.as_deref().and_then(|s| s.fault);
        let (mut offsets, after) = melt_offsets(index);
        debug_assert_eq!(after, self.status.rndindex, "the wipe's 320 draws");
        if fault == Some(MeltFault::ColumnOffByOne) {
            let c = MELT_COLUMNS / 2;
            offsets[c] += if offsets[c] == 0 { -1 } else { 1 };
        }
        let palette = if fault == Some(MeltFault::WrongPalette) {
            (palette + 1) % 14
        } else {
            palette
        };
        let melt = Melt::from_offsets(&start, end, offsets).expect("64,000-pixel screens");
        if let Some(s) = self.session.as_deref_mut() {
            s.melt = Some(Box::new(MeltRun {
                melt,
                start_taint,
                end_taint,
                palette,
                step: 0,
            }));
        }
    }

    /// `F_Drawer` for [`SessionConfig::FinaleAndMelt`] (module documentation).
    pub(crate) fn draw_finale_frame(
        &mut self,
        mode: GameMode,
        view: &super::FinaleView,
        out: &mut Frame,
    ) -> Result<NonLevelFrame, SessionError> {
        let s = self.session.as_deref().ok_or(SessionError::NotSession)?;
        let c = s
            .carried
            .as_ref()
            .ok_or(SessionError::WrongGameState(s.gamestate))?;
        let (mut screen, mut taint) = (c.screen.clone(), c.taint.clone());
        crate::f_finale::draw(&self.res, &self.font, mode, view, &mut screen, &mut taint)
            .map_err(SessionError::Finale)?;
        // I_SetPalette(PLAYPAL) only on entering the state (d_main.c:274-275): the text-to-art
        // wipe sets none.
        let palette_set = (s.old_gamestate != Some(GameState::Finale)).then_some(0);
        let start = self.melt_start();
        let before = self.status.rndindex;
        if self.wipe {
            self.status.rndindex = advanced(self.status.rndindex, WIPE_DRAWS);
        }
        self.wipe = false;
        out.pixels_mut().copy_from_slice(&screen);
        let s = self.session.as_deref_mut().expect("checked");
        let c = s.carried.as_mut().expect("checked");
        c.screen = screen;
        c.taint = taint;
        s.old_gamestate = Some(GameState::Finale);
        if let Some((start, start_taint)) = start {
            self.begin_melt(start, start_taint, 0, before);
        }
        Ok(NonLevelFrame {
            palette: 0,
            palette_set,
            snl_pointeron: false,
        })
    }

    /// Whether a melt is in progress.
    pub fn melt_in_progress(&self) -> bool {
        self.session.as_deref().is_some_and(|s| s.melt.is_some())
    }

    /// One melt step (`wipe_ScreenWipe` with `ticks = 1`, d_main.c:335-340; f_wipe.c:184-222):
    /// the next picture into `out` (64,000 palette indices) and whether the melt is done; the step
    /// that returns `done` ends the melt, and the next tic can then be observed. Refused with
    /// [`SessionError::NoMelt`] when no melt is in progress, with
    /// [`SessionError::NotYet`]`(`[`NotYet::MeltPictures`]`)` by the frozen configuration, and
    /// with [`SessionError::NotSession`] by a display that is not a session display; a refusal
    /// changes nothing.
    pub fn melt_step(&mut self, out: &mut Frame) -> Result<MeltStep, SessionError> {
        let s = self
            .session
            .as_deref_mut()
            .ok_or(SessionError::NotSession)?;
        if s.config == SessionConfig::RefusesFinaleAndMelt {
            return Err(SessionError::NotYet(NotYet::MeltPictures));
        }
        let fault = s.fault;
        let run = s.melt.as_deref_mut().ok_or(SessionError::NoMelt)?;
        if fault == Some(MeltFault::DroppedStep) && run.step == 4 {
            run.melt.step();
        }
        let done = run.melt.step();
        run.step = run.step.saturating_add(1);
        out.pixels_mut().copy_from_slice(run.melt.picture());
        let shown = MeltStep {
            step: run.step,
            done,
            palette: run.palette,
        };
        if done {
            // wipe_exitMelt (f_wipe.c:226-233); screens[0] is the end screen, which the scene
            // already holds.
            s.melt = None;
        }
        Ok(shown)
    }

    /// The palette of the melt in progress.
    pub fn melt_palette(&self) -> Option<u8> {
        self.melt_run().map(|m| m.palette)
    }

    /// The melt steps already shown of the melt in progress.
    pub fn melt_steps_shown(&self) -> Option<u8> {
        self.melt_run().map(|m| m.step)
    }

    /// The start and end screens of the melt in progress.
    pub fn melt_screens(&self) -> Option<(&[u8], &[u8])> {
        self.melt_run().map(|m| (m.melt.start(), m.melt.end()))
    }

    /// The taint (decision R4) of the melt in progress's current picture: per pixel, the start or
    /// end screen's taint of the pixel shown there.
    pub fn melt_taint(&self) -> Option<Vec<u32>> {
        self.melt_run()
            .map(|m| compose(&m.start_taint, &m.end_taint, m.melt.columns()))
    }

    fn melt_run(&self) -> Option<&MeltRun> {
        self.session.as_deref()?.melt.as_deref()
    }

    /// Plants a fault (self-tests only, [`MeltFault`]); ignored by a display that is not a
    /// session display.
    #[doc(hidden)]
    pub fn plant_melt_fault(&mut self, fault: MeltFault) {
        if let Some(s) = self.session.as_deref_mut() {
            s.fault = Some(fault);
        }
    }

    /// The saveable state of a finale-and-melt session display (`DDSP` version 4); None for
    /// another display or configuration.
    pub fn session_state_v4(&self) -> Option<SessionDisplayStateV4> {
        let s = self.session.as_deref()?;
        let SessionConfig::FinaleAndMelt { mode } = s.config else {
            return None;
        };
        if s.fault.is_some() {
            return None;
        }
        let session = self.session_state_any()?;
        let melt = s.melt.as_deref().map(|m| MeltProgress {
            step: m.step,
            palette: m.palette,
            columns: *m.melt.columns(),
            start: m.melt.start().to_vec(),
            start_taint: m.start_taint.clone(),
            end: m.melt.end().to_vec(),
        });
        Some(SessionDisplayStateV4 {
            mode,
            session,
            melt,
        })
    }

    /// The `DDSP` version 3 state with the reason for a refusal named (owner's ruling 3 of
    /// 2026-11-23: a session display of [`SessionConfig::FinaleAndMelt`] is saved only as version
    /// 4, and an old-version save from it is a typed refusal). For the frozen configuration it is
    /// exactly [`Self::session_state`], whose `None` is kept as it was for its existing callers.
    pub fn session_state_v3_checked(&self) -> Result<SessionDisplayState, SessionStateRefusal> {
        let s = self
            .session
            .as_deref()
            .ok_or(SessionStateRefusal::NotSession)?;
        if s.config != SessionConfig::RefusesFinaleAndMelt {
            return Err(SessionStateRefusal::FinaleAndMeltSavesVersion4);
        }
        self.session_state_any()
            .ok_or(SessionStateRefusal::NotStarted)
    }

    /// [`Self::session_state_v4`] with the reason for a refusal named.
    pub fn session_state_v4_checked(&self) -> Result<SessionDisplayStateV4, SessionStateRefusal> {
        let s = self
            .session
            .as_deref()
            .ok_or(SessionStateRefusal::NotSession)?;
        match s.config {
            SessionConfig::RefusesFinaleAndMelt => {
                Err(SessionStateRefusal::RefusesFinaleAndMeltSavesVersion3)
            }
            SessionConfig::FinaleAndMelt { .. } if s.fault.is_some() => {
                Err(SessionStateRefusal::PlantedFault)
            }
            SessionConfig::FinaleAndMelt { .. } => self
                .session_state_v4()
                .ok_or(SessionStateRefusal::NotStarted),
        }
    }

    /// Restores a `DDSP` version 4 state after [`SessionDisplayStateV4::check`], as
    /// [`Self::restore_session`] restores version 3 (a level scene with its level and the
    /// simulation at the scene's tic, a scene outside a level with `level` None), and the melt in
    /// progress rebuilt from its screens and the `M_Random` index. Refused with
    /// [`SessionRestoreError::WrongConfig`] unless the display is [`SessionConfig::FinaleAndMelt`]
    /// with the state's game mode. On an error the display keeps its state.
    pub fn restore_session_v4(
        &mut self,
        state: SessionDisplayStateV4,
        level: Option<(Arc<Level>, &Simulation)>,
    ) -> Result<(), SessionRestoreError> {
        let s = self
            .session
            .as_deref()
            .ok_or(SessionRestoreError::NotSession)?;
        if s.config != (SessionConfig::FinaleAndMelt { mode: state.mode }) {
            return Err(SessionRestoreError::WrongConfig);
        }
        state.check()?;
        let run = match &state.melt {
            None => None,
            Some(m) => {
                let melt = replay(m, state.session.status.rndindex).map_err(inconsistent)?;
                let end_taint = match &state.session.scene {
                    Scene::Level(r) => r.taint.clone(),
                    Scene::Intermission(c) | Scene::Finale(c) => c.taint.clone(),
                };
                Some(Box::new(MeltRun {
                    melt,
                    start_taint: m.start_taint.clone(),
                    end_taint,
                    palette: m.palette,
                    step: m.step,
                }))
            }
        };
        let mut next = self.clone();
        next.restore_session_checked(state.session, level)?;
        next.session.as_deref_mut().expect("restored").melt = run;
        *self = next;
        Ok(())
    }
}

fn inconsistent(rule: &'static str) -> SessionRestoreError {
    SessionRestoreError::Display(DisplayRestoreError::Inconsistent(rule))
}

/// The melt of `m` rebuilt: `wipe_initMelt` at the index before the wipe's 320 draws (no tic
/// runs during a melt, so the state's index is the one after them), then `m.step` steps, none
/// of which may have returned `done`, ending on `m.columns`.
fn replay(m: &MeltProgress, rndindex: u8) -> Result<Melt, &'static str> {
    if m.step > MELT_MAX_SHOWN {
        return Err("melt step");
    }
    let before = rndindex.wrapping_sub(WIPE_INDEX_STEP);
    let (mut melt, _) = Melt::new(&m.start, &m.end, before).map_err(|_| "melt screens")?;
    for _ in 0..m.step {
        if melt.step() {
            return Err("melt step");
        }
    }
    if melt.columns() != &m.columns {
        return Err("melt columns");
    }
    Ok(melt)
}

/// The picture of a melt from its screens and column positions (f_wipe.c:192-216): column pair
/// i shows rows 0..y of `end` above rows 0..200-y of `start` (a waiting column, y <= 0, shows
/// `start`).
pub(crate) fn compose<T: Copy>(start: &[T], end: &[T], columns: &[i32; MELT_COLUMNS]) -> Vec<T> {
    let width = crate::SCREENWIDTH;
    let height = crate::SCREENHEIGHT;
    let mut out = start.to_vec();
    for (i, &y) in columns.iter().enumerate() {
        let y = usize::try_from(y.clamp(0, height as i32)).unwrap_or(0);
        for row in 0..height {
            for x in [2 * i, 2 * i + 1] {
                out[row * width + x] = if row < y {
                    end[row * width + x]
                } else {
                    start[(row - y) * width + x]
                };
            }
        }
    }
    out
}

impl SessionDisplayStateV4 {
    /// The checks that need no simulation ([`Display::restore_session_v4`] adds the level's):
    ///
    /// - every rule of version 3 ([`SessionDisplayState::check`]) on [`Self::session`], whose
    ///   own `melt` must be `None`;
    /// - with a melt in progress: the display that started it ran in the tic after the state was
    ///   entered and cleared the wipe (`entered` one session tic before `tic`, the wipe flag
    ///   clear, `oldgamestate` the scene's state: d_main.c:223-227, 300; a melt starts only in the
    ///   frame of the first tic in a new state, and nothing else runs until it ends); both
    ///   screens and the start screen's taint 64,000 entries; the end screen equal to the scene's
    ///   composed frame (the melt ends on it, f_wipe.c:184-222); the palette the end frame's (a
    ///   level's `st_palette`, else 0, M6a note §1.16.3); and the steps and column positions those
    ///   of `wipe_initMelt` at the index before the wipe's 320 draws followed by that many
    ///   `wipe_doMelt` steps, none of which returned `done`.
    ///
    /// Accepted on trust, as in version 3: the start screen and its taint (the previous frame,
    /// which may belong to a level no longer loaded).
    pub fn check(&self) -> Result<(), SessionRestoreError> {
        if self.session.melt.is_some() {
            return Err(inconsistent("version 3 melt"));
        }
        self.session.check()?;
        let Some(m) = &self.melt else {
            return Ok(());
        };
        let s = &self.session;
        if s.tic.checked_sub(s.entered) != Some(1) || s.wipe {
            return Err(inconsistent("melt wipe"));
        }
        let state = s.scene.game_state();
        if s.old_gamestate != Some(state) {
            return Err(inconsistent("melt old gamestate"));
        }
        if m.start.len() != PIXELS || m.start_taint.len() != PIXELS || m.end.len() != PIXELS {
            return Err(inconsistent("melt screens"));
        }
        let (screen, palette) = match &s.scene {
            Scene::Level(r) => (&r.screen, s.status.palette),
            Scene::Intermission(c) | Scene::Finale(c) => (&c.screen, 0),
        };
        if &m.end != screen {
            return Err(inconsistent("melt end"));
        }
        if i32::from(m.palette) != palette {
            return Err(inconsistent("melt palette"));
        }
        replay(m, s.status.rndindex)
            .map(|_| ())
            .map_err(inconsistent)
    }

    /// The `DDSP` version 4 encoding (all integers little endian):
    ///
    /// | Field | Encoding |
    /// | --- | --- |
    /// | magic, version | 4 bytes `DDSP`, u32 = 4 |
    /// | game mode | u8: 0 shareware, 1 registered, 2 retail |
    /// | session tic, entered | 2 x u64 |
    /// | game state | u8: 0 level, 1 intermission, 2 finale |
    /// | old game state | u8: 0..=2, or 255 for -1 |
    /// | scene, a level | u32 length n, n bytes of `DRST` version 2 |
    /// | scene, outside a level | u32 `fuzzpos`, 4 x i32 wall steps, 64,000 bytes screen, taint runs |
    /// | melt | u8: 0 none, 1 in progress, then u8 steps shown, u8 palette, 160 x i16 column positions, 64,000 bytes start screen, its taint runs, 64,000 bytes end screen |
    /// | status bar and HUD | exactly version 1's rows from `screenblocks` to the message line |
    ///
    /// Taint runs are as in `DRST`: u32 run count n, n x (u32 count > 0, u32 origin), the counts
    /// summing to 64,000. The version-3 `melt` field of [`Self::session`] is not written.
    pub fn encode(&self) -> Vec<u8> {
        let s = &self.session;
        let mut o = Vec::with_capacity(3 * PIXELS + 4096);
        o.extend_from_slice(MAGIC);
        o.extend(VERSION_FINALE_MELT.to_le_bytes());
        o.push(mode_code(self.mode));
        o.extend(s.tic.to_le_bytes());
        o.extend(s.entered.to_le_bytes());
        o.push(s.scene.game_state().code());
        o.push(s.old_gamestate.map_or(0xff, GameState::code));
        match &s.scene {
            Scene::Level(r) => {
                let drst = r.encode();
                o.extend((drst.len() as u32).to_le_bytes());
                o.extend(drst);
            }
            Scene::Intermission(c) | Scene::Finale(c) => {
                o.extend(c.fuzzpos.to_le_bytes());
                let w = &c.wall_steps;
                for x in [w.pixhigh, w.pixhighstep, w.pixlow, w.pixlowstep] {
                    o.extend(x.to_le_bytes());
                }
                o.extend_from_slice(&c.screen);
                write_runs(&mut o, &c.taint);
            }
        }
        match &self.melt {
            None => o.push(0),
            Some(m) => {
                o.push(1);
                o.push(m.step);
                o.push(m.palette);
                for y in m.columns {
                    // -15..=200 in every state a melt produces; anything else is refused by the
                    // decoder (a hand-built out-of-range value is written truncated).
                    o.extend((y as i16).to_le_bytes());
                }
                o.extend_from_slice(&m.start);
                write_runs(&mut o, &m.start_taint);
                o.extend_from_slice(&m.end);
            }
        }
        write_tail(
            &mut o,
            s.screenblocks,
            s.fullscreen,
            s.wipe,
            &s.status,
            &s.hud,
        );
        o
    }

    /// Decodes a `DDSP` version 4 encoding: every count and range ([`StateDecodeError::Invalid`]
    /// names the field: the game mode, the game states, `entered` at most the tic, `fuzzpos`, the
    /// taint runs, the melt tag, its steps shown at most [`MELT_MAX_SHOWN`], its palette at most
    /// 13 and its column positions in -15..=200), the embedded `DRST` with its own decoder, and
    /// version 1's rows with version 1's checks. Every other version is refused as `Version`
    /// (versions 1 and 2 are `DisplayState::decode_any`'s, version 3
    /// [`SessionDisplayState::decode`]'s, and each refuses version 4). The rules across fields
    /// are [`Self::check`]'s.
    pub fn decode(bytes: &[u8]) -> Result<Self, StateDecodeError> {
        let mut r = Reader { b: bytes, at: 0 };
        if r.take(4)? != MAGIC {
            return Err(StateDecodeError::Magic);
        }
        let version = r.u32()?;
        if version != VERSION_FINALE_MELT {
            return Err(StateDecodeError::Version(version));
        }
        let mode = match r.u8()? {
            0 => GameMode::Shareware,
            1 => GameMode::Registered,
            2 => GameMode::Retail,
            _ => return Err(StateDecodeError::Invalid("game mode")),
        };
        let tic = r.u64()?;
        let entered = r.u64()?;
        let state = GameState::from_code(r.u8()?).ok_or(StateDecodeError::Invalid("gamestate"))?;
        let old_gamestate = match r.u8()? {
            0xff => None,
            c => Some(GameState::from_code(c).ok_or(StateDecodeError::Invalid("old gamestate"))?),
        };
        if entered > tic {
            return Err(StateDecodeError::Invalid("entered"));
        }
        let scene = match state {
            GameState::Level => {
                let n = r.u32()? as usize;
                Scene::Level(Box::new(RenderState::decode(r.take(n)?)?))
            }
            _ => {
                let fuzzpos = r.u32()?;
                if fuzzpos as usize >= crate::r_draw::FUZZTABLE {
                    return Err(StateDecodeError::Invalid("fuzzpos"));
                }
                let wall_steps = WallSteps {
                    pixhigh: r.i32()?,
                    pixhighstep: r.i32()?,
                    pixlow: r.i32()?,
                    pixlowstep: r.i32()?,
                };
                let screen = r.take(PIXELS)?.to_vec();
                let taint = r.runs()?;
                let c = Carried {
                    screen,
                    taint,
                    fuzzpos,
                    wall_steps,
                };
                if state == GameState::Intermission {
                    Scene::Intermission(c)
                } else {
                    Scene::Finale(c)
                }
            }
        };
        let melt = match r.u8()? {
            0 => None,
            1 => {
                let step = r.u8()?;
                if step > MELT_MAX_SHOWN {
                    return Err(StateDecodeError::Invalid("melt step"));
                }
                let palette = r.u8()?;
                if palette > 13 {
                    return Err(StateDecodeError::Invalid("melt palette"));
                }
                let mut columns = [0i32; MELT_COLUMNS];
                for y in &mut columns {
                    let b = r.take(2)?;
                    *y = i32::from(i16::from_le_bytes([b[0], b[1]]));
                    if !(-15..=200).contains(y) {
                        return Err(StateDecodeError::Invalid("melt columns"));
                    }
                }
                let start = r.take(PIXELS)?.to_vec();
                let start_taint = r.runs()?;
                let end = r.take(PIXELS)?.to_vec();
                Some(MeltProgress {
                    step,
                    palette,
                    columns,
                    start,
                    start_taint,
                    end,
                })
            }
            _ => return Err(StateDecodeError::Invalid("melt")),
        };
        let tail = read_tail(&bytes[r.at..])?;
        Ok(Self {
            mode,
            session: SessionDisplayState {
                screenblocks: tail.screenblocks,
                fullscreen: tail.fullscreen,
                wipe: tail.wipe,
                status: tail.status,
                hud: tail.hud,
                old_gamestate,
                tic,
                entered,
                scene,
                melt: None,
            },
            melt,
        })
    }
}

fn mode_code(mode: GameMode) -> u8 {
    match mode {
        GameMode::Shareware => 0,
        GameMode::Registered => 1,
        GameMode::Retail => 2,
    }
}

#[cfg(test)]
#[path = "finale_melt_reader_tests.rs"]
mod reader_tests;
