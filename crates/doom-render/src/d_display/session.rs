//! M6a: the display across a single-player Episode 1 session (owner ruling U3 (i); M6a note §1.6,
//! §1.9, §1.11, §1.16, §4.4, §4.5), saved as `DDSP` version 3 ([`SessionDisplayState`]).
//!
//! A session display ([`Display::new_session`]) keeps M5c's random accounting
//! ([`RandomContract::WithSound`]) and adds what a whole session needs:
//!
//! - **the game state** `D_Display` switches on (`gamestate`, d_main.c:233-261): a level, the
//!   intermission or the finale, with `oldgamestate` (d_main.c:196, 300) for the palette rule of
//!   d_main.c:274-275 (`I_SetPalette(PLAYPAL)`, palette 0, on the first frame of a state other
//!   than the level);
//! - **level starts that keep the `M_Random` index** ([`Display::begin_next_level`]), next to
//!   [`Display::begin_level`] (a new game, `M_ClearRandom`, g_game.c:1414): `G_DoWorldDone` and the
//!   single-player reborn reload clear neither random index (m_random.c:69-72 has one caller,
//!   `G_InitNew`), and `P_SpawnPlayer` runs `ST_Start` and `HU_Start` (p_mobj.c:693-699);
//! - **every wipe of the session** (§1.11): `gamestate != wipegamestate` (d_main.c:223) at the
//!   session start, level to intermission, intermission to level, the reborn reload
//!   (`wipegamestate = -1`, g_game.c:472-473), level to finale and the finale's text to art screen
//!   (`wipegamestate = -1`, f_finale.c:245); each wipe's 320 draws ([`WIPE_DRAWS`],
//!   f_wipe.c:159-166) come after the frame of the first tic in the new state (decision T6,
//!   extended by §4.5);
//! - **the index through tics outside a level**: [`Display::random_index`] hands it out to the
//!   game layer's intermission logic and its sound, and [`Display::observe_non_level`] takes back
//!   the number of draws they made (as [`Display::observe_after`] does in a level);
//! - **the persistent `screens[0]`** across states: a level frame is composed over the previous
//!   frame (decision R8's persistent framebuffer), so the first frame of a level after an
//!   intermission keeps the intermission's pixels wherever the level frame draws nothing, and
//!   `fuzzpos` and the r_segs.c wall steps, globals never reset (r_draw.c:274, r_segs.c:81-84), run
//!   on from level to level.
//!
//! **Not yet** (after the owner's hand test; typed refusals, never a silent no-op): the finale
//! drawer ([`Display::draw_finale`] refuses [`NotYet::FinaleDrawer`]) and the melt pictures
//! (a saved melt in progress is refused on restore as [`NotYet::MeltPictures`]). `DDSP` version 3
//! already has room for both, so neither needs another version.
use super::{
    Display, DisplayObserveError, DisplayRestoreError, HudState, RandomContract, StatusBarState,
    WIPE_DRAWS, advanced, read_tail, render_config, status_inputs, stored_message, value_error,
    write_tail,
};
use crate::st_stuff::{self, StatusInputs};
use crate::state::WallSteps;
use crate::v_video::{Canvas, Screen};
use crate::wi_stuff::{
    self, IntermissionGraphics, IntermissionLoadError, IntermissionRefusal, IntermissionView,
};
use crate::{Frame, Level, RenderState, Renderer, SCREENHEIGHT, SCREENWIDTH, StateDecodeError};
use doom_sim::Simulation;
use std::fmt;
use std::sync::Arc;

const PIXELS: usize = SCREENWIDTH * SCREENHEIGHT;

/// `gamestate_t` (doomdef.h:127-133) as far as a session reaches it (`GS_DEMOSCREEN`, the title
/// loop, is a later milestone, to be named by the owner).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameState {
    /// `GS_LEVEL` (0).
    Level,
    /// `GS_INTERMISSION` (1).
    Intermission,
    /// `GS_FINALE` (2).
    Finale,
}

impl GameState {
    fn code(self) -> u8 {
        match self {
            Self::Level => 0,
            Self::Intermission => 1,
            Self::Finale => 2,
        }
    }
    fn from_code(c: u8) -> Option<Self> {
        [Self::Level, Self::Intermission, Self::Finale]
            .get(usize::from(c))
            .copied()
    }
}

/// How a level starts inside a session without a new game (both keep the `M_Random` index).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LevelChange {
    /// `G_DoWorldDone` (g_game.c:1172-1179): the next map, from the intermission.
    WorldDone,
    /// The single-player reborn reload (`G_DoReborn`'s `ga_loadlevel`, g_game.c:924-932): the
    /// same map, from the level.
    Reborn,
}

/// Original behaviour that is not built yet (after the owner's hand test, M6a note §6.5). Each
/// is a typed refusal, never a silent no-op.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotYet {
    /// `F_Drawer` (f_finale.c:700-731): the text crawl and the art screen.
    FinaleDrawer,
    /// The melt's pictures (`wipe_doMelt`, f_wipe.c:184-222), one per tic under the declared
    /// schedule (M6a note §1.16.1).
    MeltPictures,
}

/// What `F_Drawer`'s Doom 1 path will read (f_finale.c:50-54, 700-731): `finalestage` and
/// `finalecount`. For Episode 1 the flat (`FLOOR4_8`) and the text (`E1TEXT`) are fixed by
/// `F_StartFinale` (f_finale.c:117-121), and the art screen by the game mode (f_finale.c:714-720).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FinaleView {
    /// `finalestage`: 0 the text, 1 the art screen.
    pub stage: i32,
    /// `finalecount`.
    pub count: i32,
}

/// One frame outside a level beside its pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NonLevelFrame {
    /// The PLAYPAL index in force: 0, set by `I_SetPalette(PLAYPAL)` on entering the state
    /// (d_main.c:274-275) and changed by nothing outside a level.
    pub palette: u8,
    /// The palette `I_SetPalette` received during this frame: `Some(0)` when the state differs
    /// from `oldgamestate`, which is the last displayed state, or -1 after a screen size change
    /// (d_main.c:215-220, 274-275).
    pub palette_set: Option<u8>,
    /// `snl_pointeron` after the frame (intermission only; see
    /// [`crate::wi_stuff::IntermissionFrame`]).
    pub snl_pointeron: bool,
}

/// `screens[0]` with its taint, and the renderer globals that outlive a level (`fuzzpos`,
/// r_draw.c:274; the wall steps, r_segs.c:81-84), held while no level is loaded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Carried {
    /// `screens[0]`: the last composed frame (64,000 palette indices).
    pub screen: Vec<u8>,
    /// Per pixel, the origin of an undefined read that produced it (0: defined; decision R4).
    pub taint: Vec<u32>,
    /// `fuzzpos`, 0..50.
    pub fuzzpos: u32,
    /// The r_segs.c wall-step globals.
    pub wall_steps: WallSteps,
}

impl Carried {
    fn from_render(r: &RenderState) -> Self {
        Self {
            screen: r.screen.clone(),
            taint: r.taint.clone(),
            fuzzpos: r.fuzzpos,
            wall_steps: r.wall_steps,
        }
    }
}

/// What `screens[0]` and the renderer hold, by game state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Scene {
    /// A level: the render state (`DRST` version 2), whose screen is the composed frame.
    Level(Box<RenderState>),
    /// The intermission.
    Intermission(Carried),
    /// The finale.
    Finale(Carried),
}

impl Scene {
    /// The game state of the scene.
    pub fn game_state(&self) -> GameState {
        match self {
            Self::Level(_) => GameState::Level,
            Self::Intermission(_) => GameState::Intermission,
            Self::Finale(_) => GameState::Finale,
        }
    }
}

/// A melt in progress (M6a note §1.16): `step` melt steps of `ticks = 1` shown of the wipe that
/// the last frame started. No display produces one yet ([`Display::session_state`] always gives
/// `None`; the melt pictures are [`NotYet::MeltPictures`]); the layout is fixed now so that they
/// need no new version. The format itself carries one: [`SessionDisplayState::encode`] writes tag
/// 1 for a state built by hand with `Some`, [`SessionDisplayState::decode`] keeps it, and
/// [`SessionDisplayState::check`] and every restore refuse it.
///
/// Stored, not reconstructible: the start screen (`wipe_StartScreen`, f_wipe.c:237-245, the
/// previous frame, which may belong to a level no longer loaded) and the step count. Not stored,
/// because they are functions of what is stored: the 160 column offsets `y[]`, from the 320
/// `M_Random` draws of `wipe_initMelt` (f_wipe.c:156-168) at the index before the wipe (the
/// state's index minus 320, since no tic runs during a melt) and the step count (f_wipe.c:196-208);
/// the end screen (`wipe_EndScreen`, f_wipe.c:249-258), which is the composed frame the scene
/// holds; the number of steps N = 28 + max(-y0) (§1.16.1); and the palette (the end frame's,
/// §1.16.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeltState {
    /// Melt steps already shown, 1..=42 (at most N - 1 of N <= 43).
    pub step: u8,
    /// The start screen.
    pub start: Vec<u8>,
    /// The start screen's taint.
    pub start_taint: Vec<u32>,
}

/// The saved state of a session display, `DDSP` version 3 (docs/STATE_FORMAT.md, the layout is in
/// [`SessionDisplayState::encode`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionDisplayState {
    /// `screenblocks` (10 or 11).
    pub screenblocks: u8,
    /// `D_Display`'s `fullscreen` (d_main.c:198, 247).
    pub fullscreen: bool,
    /// A wipe is due: `gamestate != wipegamestate` (d_main.c:223).
    pub wipe: bool,
    /// The status bar, its `M_Random` index included.
    pub status: StatusBarState,
    /// The HUD.
    pub hud: HudState,
    /// `oldgamestate` (d_main.c:196): None for -1.
    pub old_gamestate: Option<GameState>,
    /// Session tics observed: `gametic` after the last tic (d_net.c:745-746).
    pub tic: u64,
    /// The session tic count at the latest state entry, level start or forced wipe.
    pub entered: u64,
    /// The scene.
    pub scene: Scene,
    /// A melt in progress (never written yet).
    pub melt: Option<MeltState>,
}

/// The session part of a [`Display`].
#[derive(Clone, Debug)]
pub(crate) struct Session {
    pub(crate) gamestate: GameState,
    pub(crate) old_gamestate: Option<GameState>,
    pub(crate) tic: u64,
    pub(crate) entered: u64,
    /// Some exactly outside a level.
    carried: Option<Carried>,
    /// The intermission graphics, in the intermission.
    wi: Option<Arc<IntermissionGraphics>>,
}

impl Session {
    /// A fresh process at the session start: no tic, `oldgamestate` -1 (d_main.c:196), the level
    /// state, whose start wipes (`wipegamestate` is `GS_DEMOSCREEN`, d_main.c:187).
    pub(crate) fn fresh() -> Self {
        Self {
            gamestate: GameState::Level,
            old_gamestate: None,
            tic: 0,
            entered: 0,
            carried: None,
            wi: None,
        }
    }
}

/// A refused session call. The display is unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionError {
    /// The display is not a session display ([`Display::new_session`]).
    NotSession,
    /// The call does not fit the display's game state (or no level has been started).
    WrongGameState(GameState),
    /// The simulation could not be read or the renderer could not start the level.
    Observe(DisplayObserveError),
    /// `WI_loadData` would stop (a missing name) or touch memory it does not own.
    Load(IntermissionLoadError),
    /// The intermission frame was refused.
    Intermission(IntermissionRefusal),
    /// Not built yet.
    NotYet(NotYet),
}

/// A session state that cannot be restored. The display keeps its state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionRestoreError {
    /// The display is not a session display.
    NotSession,
    /// As for `DDSP` versions 1 and 2 (`Value`, `Inconsistent`, `Tic`, `Render`, `Observe`,
    /// `Config`).
    Display(DisplayRestoreError),
    /// A level scene needs its level and the simulation at the same tic; a scene outside a level
    /// takes neither.
    Level,
    /// The intermission graphics could not be loaded.
    Load(IntermissionLoadError),
    /// Not built yet: a melt in progress.
    NotYet(NotYet),
}

macro_rules! session_error {
    ($($t:ty),*) => {$(
        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "session display: {self:?}")
            }
        }
        impl std::error::Error for $t {}
    )*};
}
session_error!(SessionError, SessionRestoreError);

impl Display {
    /// A session display over loaded resources and the first level (M6a; `DDSP` version 3): the
    /// M5c accounting ([`RandomContract::WithSound`]) throughout, a fresh process's `D_Display`
    /// statics and a zeroed `screens[0]` (the first melt melts from zeros, M6a note §1.16.2).
    /// Start it with [`Self::begin_level`] (the session's new game).
    pub fn new_session(
        res: Arc<crate::Resources>,
        level: Arc<Level>,
        cfg: super::DisplayConfig,
    ) -> Result<Self, crate::ResourceError> {
        let mut d = Self::new(res, level, cfg)?;
        d.contract = RandomContract::WithSound;
        d.session = Some(Box::new(Session::fresh()));
        Ok(d)
    }

    /// The game state of a session display (None otherwise).
    pub fn game_state(&self) -> Option<GameState> {
        self.session.as_ref().map(|s| s.gamestate)
    }

    /// Session tics observed (None for a display that is not a session display).
    pub fn session_tic(&self) -> Option<u64> {
        self.session.as_ref().map(|s| s.tic)
    }

    /// The `M_Random` index (m_random.c:63-67) after the last observed tic and drawn frame: the
    /// value the game layer's intermission logic and its sound start from in a tic outside a
    /// level; their draws come back through [`Self::observe_non_level`].
    pub fn random_index(&self) -> u8 {
        self.status.rndindex
    }

    /// Whether a wipe is due: `gamestate != wipegamestate` (d_main.c:223), so the next frame is
    /// followed by the melt's [`WIPE_DRAWS`] (M6a step 7 reports where each wipe falls).
    pub fn wipe_due(&self) -> bool {
        self.wipe
    }

    /// A level start inside a session that keeps the `M_Random` index (`change` says which, and
    /// must fit the current state: [`LevelChange::WorldDone`] from the intermission,
    /// [`LevelChange::Reborn`] from a level), over `level` and the simulation constructed for it
    /// (before its tic 0). Call it before observing the load tic (the new level's tic 0 runs in
    /// the same `G_Ticker` as the load, g_game.c:664-669). As the C does:
    ///
    /// - `ST_Start` (st_stuff.c:1444-1454): `ST_initData` resets `st_firsttime`, `st_faceindex`,
    ///   `st_palette` (-1), `st_oldhealth` (-1), `oldweaponsowned` (the player's) and the key
    ///   boxes (st_stuff.c:1250-1278), `ST_createWidgets` the widgets' old values; **kept**:
    ///   `st_facecount` (st_stuff.c:384), the face `priority` and `lastattackdown` (function
    ///   statics, st_stuff.c:757-758), the `M_Random` index (m_random.c:54) and `screens[4]`
    ///   (allocated once, st_stuff.c:1470). `ST_Stop`'s `I_SetPalette` (st_stuff.c:1457-1464)
    ///   happens inside the tic and is replaced by the frame's own (`st_palette` -1);
    /// - `HU_Start` (hu_stuff.c:419-484): `message_on`, `message_dontfuckwithme`,
    ///   `message_nottobefuckedwith` cleared and the message line re-initialised
    ///   (`HUlib_initSText`, hu_lib.c:177-198); **kept**: `message_counter` (hu_stuff.c:103) and
    ///   `showMessages`;
    /// - `D_Display`'s `fullscreen` is kept (a static, d_main.c:198);
    /// - a wipe is due (the state changed, or `wipegamestate = -1` for a reload, g_game.c:472-473);
    /// - the renderer starts the level (decision R8) over the carried `screens[0]`, its taint,
    ///   `fuzzpos` and wall steps instead of a fresh process's zeros.
    ///
    /// A load-time sound start (a carried chainsaw's `sfx_sawup`, M6a note §1.1 item 8) draws
    /// before `ST_Ticker` in the same tic: pass it with the tic's sound draws to
    /// [`Self::observe_after`]. On an error nothing changes.
    pub fn begin_next_level(
        &mut self,
        level: Arc<Level>,
        sim: &Simulation,
        change: LevelChange,
    ) -> Result<(), SessionError> {
        self.begin_next_level_carried(level, sim, change, 0)
    }

    /// [`Self::begin_next_level`] with the `deltaviewheight` the new level's player starts with
    /// (M6a step 7). `P_SpawnPlayer` sets `viewheight = VIEWHEIGHT` (p_mobj.c:683) but leaves
    /// `deltaviewheight` alone, and neither `G_PlayerFinishLevel` (g_game.c:779-792) nor
    /// `G_DoLoadLevel` (g_game.c:445-497) touches it, so a carried start keeps the exit tic's
    /// value (a landing just before the exit squats at the next start, through `P_CalcHeight`,
    /// p_user.c:113-136); a reborn player's is 0 (the `memset` of `G_PlayerReborn`,
    /// g_game.c:815). [`Self::begin_next_level`] is this with 0. On an error nothing changes.
    pub fn begin_next_level_carried(
        &mut self,
        level: Arc<Level>,
        sim: &Simulation,
        change: LevelChange,
        deltaviewheight: i32,
    ) -> Result<(), SessionError> {
        let s = self.session.as_deref().ok_or(SessionError::NotSession)?;
        let from = match change {
            LevelChange::WorldDone => GameState::Intermission,
            LevelChange::Reborn => GameState::Level,
        };
        if s.gamestate != from {
            return Err(SessionError::WrongGameState(s.gamestate));
        }
        let carried = match &s.carried {
            Some(c) => c.clone(),
            None => Carried::from_render(
                self.renderer
                    .state()
                    .ok_or(SessionError::WrongGameState(s.gamestate))?,
            ),
        };
        let inputs = status_inputs(sim).map_err(SessionError::Observe)?;
        let stored = stored_message(sim).map_err(SessionError::Observe)?;
        let mut renderer = Renderer::new(self.res.clone(), level, self.renderer.config())
            .expect("the display's own configuration");
        renderer
            .begin_level(sim)
            .map_err(|e| SessionError::Observe(DisplayObserveError::Render(e)))?;
        let mut start = renderer.state().expect("begun").clone();
        start.screen = carried.screen;
        start.taint = carried.taint;
        start.fuzzpos = carried.fuzzpos;
        start.wall_steps = carried.wall_steps;
        start.view.deltaviewheight = deltaviewheight;
        renderer.restore(start).expect("the level's own start");
        let old = self.status;
        let mut status = StatusBarState::level_start(inputs.weaponowned);
        status.rndindex = old.rndindex;
        status.facecount = old.facecount;
        status.priority = old.priority;
        status.lastattackdown = old.lastattackdown;
        status.bg_drawn = old.bg_drawn;
        let mut hud = HudState::level_start(stored);
        hud.message_counter = self.hud.message_counter;
        hud.show_messages = self.hud.show_messages;
        self.renderer = renderer;
        self.status = status;
        self.hud = hud;
        self.inputs = Some(inputs);
        self.wipe = true;
        let s = self.session.as_deref_mut().expect("checked");
        s.gamestate = GameState::Level;
        s.entered = s.tic;
        s.carried = None;
        s.wi = None;
        Ok(())
    }

    /// The view shadow (decision R6: `viewheight`, `deltaviewheight` and the rest) after the last
    /// observed level tic of a session display, or `None` outside a level (M6a step 7: the
    /// session carries the exit tic's `deltaviewheight` to the next level start).
    pub fn level_view(&self) -> Option<&crate::view::ViewShadow> {
        let s = self.session.as_deref()?;
        (s.gamestate == GameState::Level)
            .then(|| self.renderer.state().map(|r| &r.view))
            .flatten()
    }

    /// `G_DoCompleted`'s `gamestate = GS_INTERMISSION` and `WI_Start` (g_game.c:1133-1140) for
    /// Episode 1: loads the intermission graphics (`WI_loadData`, which stops the genuine engine
    /// at a missing name: [`SessionError::Load`]), keeps `screens[0]` and the renderer globals,
    /// and makes a wipe due. Call it in the tic that completes the level (the first intermission
    /// tic, M6a note §1.1 item 2), before [`Self::observe_non_level`]. On an error nothing
    /// changes.
    pub fn begin_intermission(&mut self) -> Result<(), SessionError> {
        let carried = self.leave_level()?;
        let g = IntermissionGraphics::load(&self.res, 0).map_err(SessionError::Load)?;
        self.enter(GameState::Intermission, carried, Some(Arc::new(g)));
        Ok(())
    }

    /// `F_StartFinale`'s `gamestate = GS_FINALE` (f_finale.c:96-191), entered from the level in the
    /// `G_Ticker` that completed E1M8 (g_game.c:639-641, 1033-1038): keeps `screens[0]` and the
    /// renderer globals and makes a wipe due. Its frames are not drawn yet
    /// ([`Self::draw_finale`]).
    pub fn begin_finale(&mut self) -> Result<(), SessionError> {
        let carried = self.leave_level()?;
        self.enter(GameState::Finale, carried, None);
        Ok(())
    }

    /// `F_Ticker`'s `wipegamestate = -1` when stage 1 begins (f_finale.c:241-247): a wipe in the
    /// finale, which does not change the state (so no palette is set, d_main.c:274).
    pub fn begin_finale_art(&mut self) -> Result<(), SessionError> {
        let s = self
            .session
            .as_deref_mut()
            .ok_or(SessionError::NotSession)?;
        if s.gamestate != GameState::Finale {
            return Err(SessionError::WrongGameState(s.gamestate));
        }
        s.entered = s.tic;
        self.wipe = true;
        Ok(())
    }

    fn leave_level(&self) -> Result<Carried, SessionError> {
        let s = self.session.as_deref().ok_or(SessionError::NotSession)?;
        if s.gamestate != GameState::Level {
            return Err(SessionError::WrongGameState(s.gamestate));
        }
        self.renderer
            .state()
            .map(Carried::from_render)
            .ok_or(SessionError::WrongGameState(s.gamestate))
    }

    fn enter(&mut self, state: GameState, carried: Carried, wi: Option<Arc<IntermissionGraphics>>) {
        let s = self.session.as_deref_mut().expect("checked");
        s.gamestate = state;
        s.entered = s.tic;
        s.carried = Some(carried);
        s.wi = wi;
        self.wipe = true;
    }

    /// Observes one session tic outside a level: no `ST_Ticker` or `HU_Ticker` runs
    /// (g_game.c:727-747), and the `M_Random` index advances by `draws`, the number of draws the
    /// tic made: the intermission's animation draws (`WI_initAnimatedBack`, ten per call,
    /// wi_stuff.c:514-528) and one per sound start (s_sound.c:327-346); only their number matters
    /// for the index (m_random.c:63-67). On an error nothing changes.
    pub fn observe_non_level(&mut self, draws: u16) -> Result<(), SessionError> {
        let s = self
            .session
            .as_deref_mut()
            .ok_or(SessionError::NotSession)?;
        if s.gamestate == GameState::Level {
            return Err(SessionError::WrongGameState(s.gamestate));
        }
        self.status.rndindex = advanced(self.status.rndindex, draws);
        s.tic = s.tic.saturating_add(1);
        Ok(())
    }

    /// `D_Display` in the intermission (d_main.c:250-252, 274-275, 300, 320-344): `WI_Drawer` from
    /// `view` into `out` ([`crate::wi_stuff::draw`]), palette 0, then, when a wipe is due, its
    /// [`WIPE_DRAWS`] after the frame (the melt's pictures are not drawn: the frame shown is the
    /// composed frame, the melt's last picture). Whole-frame and independent of screenblocks. On
    /// an error nothing changes and `out` is unspecified.
    pub fn draw_intermission(
        &mut self,
        view: &IntermissionView,
        out: &mut Frame,
    ) -> Result<NonLevelFrame, SessionError> {
        let s = self.session.as_deref().ok_or(SessionError::NotSession)?;
        let g = match (&s.wi, s.gamestate) {
            (Some(g), GameState::Intermission) => g,
            _ => return Err(SessionError::WrongGameState(s.gamestate)),
        };
        let f = wi_stuff::draw(&self.res, g, view, out.pixels_mut())
            .map_err(SessionError::Intermission)?;
        let palette_set = (s.old_gamestate != Some(GameState::Intermission)).then_some(0);
        if self.wipe {
            self.status.rndindex = advanced(self.status.rndindex, WIPE_DRAWS);
        }
        self.wipe = false;
        let s = self.session.as_deref_mut().expect("checked");
        let c = s.carried.as_mut().expect("outside a level");
        c.screen.copy_from_slice(out.pixels());
        c.taint.fill(0);
        s.old_gamestate = Some(GameState::Intermission);
        Ok(NonLevelFrame {
            palette: 0,
            palette_set,
            snl_pointeron: f.snl_pointeron,
        })
    }

    /// `D_Display` in the finale: refused as [`NotYet::FinaleDrawer`] (in the finale; elsewhere
    /// [`SessionError::WrongGameState`]). Nothing changes, so the wipe and its draws stay due.
    pub fn draw_finale(
        &mut self,
        view: &FinaleView,
        out: &mut Frame,
    ) -> Result<NonLevelFrame, SessionError> {
        let _ = (view, out);
        let s = self.session.as_deref().ok_or(SessionError::NotSession)?;
        if s.gamestate != GameState::Finale {
            return Err(SessionError::WrongGameState(s.gamestate));
        }
        Err(SessionError::NotYet(NotYet::FinaleDrawer))
    }

    /// The saveable state of a session display (`DDSP` version 3); None for another display or
    /// before [`Self::begin_level`].
    pub fn session_state(&self) -> Option<SessionDisplayState> {
        let s = self.session.as_deref()?;
        let scene = match s.gamestate {
            GameState::Level => Scene::Level(Box::new(self.renderer.state()?.clone())),
            GameState::Intermission => Scene::Intermission(s.carried.clone()?),
            GameState::Finale => Scene::Finale(s.carried.clone()?),
        };
        Some(SessionDisplayState {
            screenblocks: self.renderer.config().screenblocks,
            fullscreen: self.fullscreen,
            wipe: self.wipe,
            status: self.status,
            hud: self.hud.clone(),
            old_gamestate: s.old_gamestate,
            tic: s.tic,
            entered: s.entered,
            scene,
            melt: None,
        })
    }

    /// Restores a session state after [`SessionDisplayState::check`]: a level scene with `level`
    /// = its level and the simulation at the scene's tic (the status bar inputs are read from it
    /// and the face and HUD checked against it), a scene outside a level with `level` None (the
    /// intermission graphics are loaded again). On an error the display keeps its state.
    pub fn restore_session(
        &mut self,
        state: SessionDisplayState,
        level: Option<(Arc<Level>, &Simulation)>,
    ) -> Result<(), SessionRestoreError> {
        if self.session.is_none() {
            return Err(SessionRestoreError::NotSession);
        }
        state.check()?;
        let d = SessionRestoreError::Display;
        let cfg = render_config(state.screenblocks).map_err(|_| {
            d(DisplayRestoreError::Config(
                crate::ConfigError::ScreenBlocks(state.screenblocks),
            ))
        })?;
        let mut next = self.clone();
        let mut session = Session {
            gamestate: state.scene.game_state(),
            old_gamestate: state.old_gamestate,
            tic: state.tic,
            entered: state.entered,
            carried: None,
            wi: None,
        };
        match (&state.scene, level) {
            (Scene::Level(render), Some((lvl, sim))) => {
                if render.tic != sim.tic() {
                    return Err(d(DisplayRestoreError::Tic {
                        state: render.tic,
                        simulation: sim.tic(),
                    }));
                }
                let mut r = Renderer::new(next.res.clone(), lvl, cfg)
                    .map_err(|e| d(DisplayRestoreError::Config(e)))?;
                r.restore((**render).clone())
                    .map_err(|e| d(DisplayRestoreError::Render(e)))?;
                let inputs = status_inputs(sim).map_err(|e| d(DisplayRestoreError::Observe(e)))?;
                if let Some(e) = level_consistency(&state.status, &state.hud, render.tic, &inputs) {
                    return Err(d(DisplayRestoreError::Inconsistent(e)));
                }
                next.renderer = r;
                next.inputs = Some(inputs);
            }
            (Scene::Intermission(c), None) => {
                let g =
                    IntermissionGraphics::load(&next.res, 0).map_err(SessionRestoreError::Load)?;
                session.carried = Some(c.clone());
                session.wi = Some(Arc::new(g));
                next.inputs = None;
            }
            (Scene::Finale(c), None) => {
                session.carried = Some(c.clone());
                next.inputs = None;
            }
            _ => return Err(SessionRestoreError::Level),
        }
        next.bg.fill(0);
        next.bg_defined.fill(false);
        if state.status.bg_drawn {
            // As `Display::restore`: screens[4] once drawn is STBAR (st_stuff.c:504).
            let (mut fg, mut taint) = (vec![0; PIXELS], vec![0; PIXELS]);
            let mut c = Canvas {
                res: &next.res,
                fg: &mut fg,
                taint: &mut taint,
                bg: &mut next.bg,
                bg_defined: &mut next.bg_defined,
            };
            c.patch(0, 0, Screen::Bg, &next.gfx.sbar)
                .map_err(|_| d(DisplayRestoreError::Value("bg_drawn")))?;
        }
        next.status = state.status;
        next.hud = state.hud;
        next.fullscreen = state.fullscreen;
        next.wipe = state.wipe;
        next.contract = RandomContract::WithSound;
        next.session = Some(Box::new(session));
        *self = next;
        Ok(())
    }
}

/// The face of `ST_updateFaceWidget` by `priority` (st_stuff.c:760-921), as `DDSP` version 2
/// checks it, or the carried form a session adds: `ST_initData` sets `st_faceindex` to 0 at a
/// level start but keeps `st_facecount` and `priority`, so faceindex 0 stays on screen with the
/// carried priority until the count runs out or a stronger face comes (st_stuff.c:384, 757-758,
/// 1262).
fn face_ok(s: &StatusBarState) -> bool {
    let (band, offset) = (s.faceindex / 8, s.faceindex % 8);
    let plain = s.faceindex < st_stuff::GODFACE;
    let count_ok = match s.priority {
        0 => s.facecount < 17,
        4 | 5 | 9 => s.facecount == 0,
        6 | 7 => s.facecount < 35,
        8 => s.facecount < 70,
        _ => false,
    };
    let table = match s.priority {
        0 => plain && offset <= 2,
        4 => s.faceindex == st_stuff::GODFACE,
        5 | 6 => plain && offset == 7,
        7 => plain && (matches!(offset, 3 | 4 | 7) || offset == 5 && band <= 3),
        8 => plain && offset == 6,
        9 => s.faceindex == st_stuff::DEADFACE,
        _ => false,
    };
    count_ok && (table || s.faceindex == 0)
}

/// The cross-field checks of a level scene against the simulation (`tic` is the level's), as
/// `DDSP` version 2's with what a session changes: `message_counter` survives `HU_Start`, so the
/// counter may run with the message off (only `message_on` implies a running counter); the face
/// may be in its carried form ([`face_ok`]); and a carried dead face (priority 9) may stand at the
/// level's tic 0, before the first `ST_Ticker` resets it. The wipe flag is checked against the
/// session tics in [`SessionDisplayState::check`].
fn level_consistency(
    s: &StatusBarState,
    h: &HudState,
    tic: u64,
    i: &StatusInputs,
) -> Option<&'static str> {
    let mut ticked = *s;
    if tic != 0 {
        ticked.tick_widgets(i);
    }
    let checks: [(bool, &'static str); 7] = [
        (
            !h.message_on || h.message_counter != 0,
            "message_on and message_counter",
        ),
        (!h.must_show && !h.protected, "must_show or protected"),
        (
            s.oldhealth == if tic == 0 { -1 } else { i.health } && s.oldhealth <= 200,
            "oldhealth",
        ),
        (
            ticked.keyboxes == s.keyboxes && ticked.ready_ammo == s.ready_ammo,
            "keyboxes or ready_ammo",
        ),
        (
            s.lastattackdown == -1 || (1..=70).contains(&s.lastattackdown),
            "lastattackdown",
        ),
        (face_ok(s), "face"),
        (
            tic == 0 || (s.priority == 9) == (i.health == 0),
            "dead face",
        ),
    ];
    checks.iter().find(|(ok, _)| !ok).map(|(_, name)| *name)
}

/// `DDSP` version 3.
pub const VERSION_SESSION: u32 = 3;

impl SessionDisplayState {
    /// The checks that need no simulation ([`Display::restore_session`] adds the level's):
    ///
    /// - every status bar and HUD value in its range (as versions 1 and 2), `message_on` only
    ///   with a running counter, `must_show` and `protected` clear;
    /// - no melt in progress ([`NotYet::MeltPictures`]);
    /// - `entered` at most `tic`; **the wipe flag keyed on the session tics since `entered`**: set
    ///   at 0 (a state entry, level start or forced wipe always wipes, d_main.c:223; g_game.c:
    ///   472-473; f_finale.c:245), either at 1 (between the tic's observation and its frame), clear
    ///   from 2 (the frame clears it, d_main.c:300);
    /// - a level scene's tic equal to the session tics since `entered` (the level's tic 0 runs in
    ///   the load's `G_Ticker`, g_game.c:664-669);
    /// - with no wipe due, `oldgamestate` is the state (the frame set it, d_main.c:300) or -1 (a
    ///   screen size change since, d_main.c:215-220);
    /// - outside a level, the carried screen and taint 64,000 entries, `fuzzpos` below 50, and
    ///   with no wipe due no tainted pixel (a frame outside a level overwrites every pixel:
    ///   `WI_slamBackground`, wi_stuff.c:406-410);
    /// - outside a level, `st_oldhealth` 0 to 200: the finished level's last `ST_Ticker` wrote
    ///   the player's health (st_stuff.c:994), at most 200 (p_inter.c:385-386, 402-403, 412),
    ///   and nothing writes it until the next `ST_Start` (in a level
    ///   [`Display::restore_session`] checks it against the simulation).
    pub fn check(&self) -> Result<(), SessionRestoreError> {
        let d = SessionRestoreError::Display;
        if let Some(v) = value_error(&self.status, &self.hud) {
            return Err(d(DisplayRestoreError::Value(v)));
        }
        if self.melt.is_some() {
            return Err(SessionRestoreError::NotYet(NotYet::MeltPictures));
        }
        let bad = |n| Err(d(DisplayRestoreError::Inconsistent(n)));
        let h = &self.hud;
        if h.message_on && h.message_counter == 0 {
            return bad("message_on and message_counter");
        }
        if h.must_show || h.protected {
            return bad("must_show or protected");
        }
        let Some(since) = self.tic.checked_sub(self.entered) else {
            return bad("entered");
        };
        let wipe_ok = match since {
            0 => self.wipe,
            1 => true,
            _ => !self.wipe,
        };
        if !wipe_ok {
            return bad("wipe");
        }
        let state = self.scene.game_state();
        if !self.wipe && !self.old_gamestate.is_none_or(|o| o == state) {
            return bad("old gamestate");
        }
        match &self.scene {
            Scene::Level(r) if r.tic != since => bad("level tic"),
            Scene::Level(_) => Ok(()),
            Scene::Intermission(c) | Scene::Finale(c) => {
                if c.screen.len() != PIXELS || c.taint.len() != PIXELS {
                    return bad("carried screen");
                }
                if c.fuzzpos as usize >= crate::r_draw::FUZZTABLE {
                    return Err(d(DisplayRestoreError::Value("fuzzpos")));
                }
                if !self.wipe && c.taint.iter().any(|t| *t != 0) {
                    return bad("carried taint");
                }
                if !(0..=200).contains(&self.status.oldhealth) {
                    return bad("oldhealth");
                }
                Ok(())
            }
        }
    }

    /// The `DDSP` version 3 encoding (all integers little endian):
    ///
    /// | Field | Encoding |
    /// | --- | --- |
    /// | magic, version | 4 bytes `DDSP`, u32 = 3 |
    /// | session tic, entered | 2 x u64 |
    /// | game state | u8: 0 level, 1 intermission, 2 finale |
    /// | old game state | u8: 0..=2, or 255 for -1 |
    /// | scene, a level | u32 length n, n bytes of `DRST` version 2 |
    /// | scene, outside a level | u32 `fuzzpos`, 4 x i32 wall steps, 64,000 bytes screen, taint runs |
    /// | melt | u8: 0 none, 1 in progress, then u8 step, 64,000 bytes start screen, its taint runs |
    /// | status bar and HUD | exactly version 1's rows from `screenblocks` to the message line |
    ///
    /// Taint runs are as in `DRST`: u32 run count n, n x (u32 count > 0, u32 origin), the counts
    /// summing to 64,000.
    pub fn encode(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(PIXELS + 4096);
        o.extend_from_slice(super::MAGIC);
        o.extend(VERSION_SESSION.to_le_bytes());
        o.extend(self.tic.to_le_bytes());
        o.extend(self.entered.to_le_bytes());
        o.push(self.scene.game_state().code());
        o.push(self.old_gamestate.map_or(0xff, GameState::code));
        match &self.scene {
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
                o.extend_from_slice(&m.start);
                write_runs(&mut o, &m.start_taint);
            }
        }
        write_tail(
            &mut o,
            self.screenblocks,
            self.fullscreen,
            self.wipe,
            &self.status,
            &self.hud,
        );
        o
    }

    /// Decodes a `DDSP` version 3 encoding: every count and range ([`StateDecodeError::Invalid`]
    /// names the field), the embedded `DRST` with its own decoder, and version 1's rows with
    /// version 1's checks. Versions 1 and 2 are refused as `Version` (their decoders are
    /// `DisplayState::decode` and `decode_any`, which in turn refuse version 3). A melt in
    /// progress decodes, and is refused by [`Self::check`].
    pub fn decode(bytes: &[u8]) -> Result<Self, StateDecodeError> {
        let mut r = Reader { b: bytes, at: 0 };
        if r.take(4)? != super::MAGIC {
            return Err(StateDecodeError::Magic);
        }
        let version = r.u32()?;
        if version != VERSION_SESSION {
            return Err(StateDecodeError::Version(version));
        }
        let tic = r.u64()?;
        let entered = r.u64()?;
        let state = GameState::from_code(r.u8()?).ok_or(StateDecodeError::Invalid("gamestate"))?;
        let old = r.u8()?;
        let old_gamestate = match old {
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
                if !(1..=42).contains(&step) {
                    return Err(StateDecodeError::Invalid("melt step"));
                }
                let start = r.take(PIXELS)?.to_vec();
                let start_taint = r.runs()?;
                Some(MeltState {
                    step,
                    start,
                    start_taint,
                })
            }
            _ => return Err(StateDecodeError::Invalid("melt")),
        };
        let tail = read_tail(&bytes[r.at..])?;
        Ok(Self {
            screenblocks: tail.screenblocks,
            fullscreen: tail.fullscreen,
            wipe: tail.wipe,
            status: tail.status,
            hud: tail.hud,
            old_gamestate,
            tic,
            entered,
            scene,
            melt,
        })
    }
}

fn write_runs(o: &mut Vec<u8>, taint: &[u32]) {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for t in taint {
        match runs.last_mut() {
            Some((n, v)) if v == t => *n += 1,
            _ => runs.push((1, *t)),
        }
    }
    o.extend((runs.len() as u32).to_le_bytes());
    for (n, v) in runs {
        o.extend(n.to_le_bytes());
        o.extend(v.to_le_bytes());
    }
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], StateDecodeError> {
        let end = self.at.checked_add(n).ok_or(StateDecodeError::Truncated)?;
        let b = self
            .b
            .get(self.at..end)
            .ok_or(StateDecodeError::Truncated)?;
        self.at = end;
        Ok(b)
    }
    fn u8(&mut self) -> Result<u8, StateDecodeError> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, StateDecodeError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("4")))
    }
    fn i32(&mut self) -> Result<i32, StateDecodeError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().expect("4")))
    }
    fn u64(&mut self) -> Result<u64, StateDecodeError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("8")))
    }
    /// Taint runs covering the screen exactly.
    fn runs(&mut self) -> Result<Vec<u32>, StateDecodeError> {
        let n = self.u32()?;
        if n as usize > PIXELS {
            return Err(StateDecodeError::Invalid("taint runs"));
        }
        let mut taint = Vec::with_capacity(PIXELS);
        for _ in 0..n {
            let count = self.u32()? as usize;
            let v = self.u32()?;
            // `taint.len() <= PIXELS` here, so the subtraction cannot wrap; adding the file's
            // count to the length first could, where `usize` is 32 bits.
            if count == 0 || count > PIXELS - taint.len() {
                return Err(StateDecodeError::Invalid("taint runs"));
            }
            taint.resize(taint.len() + count, v);
        }
        if taint.len() != PIXELS {
            return Err(StateDecodeError::Invalid("taint runs"));
        }
        Ok(taint)
    }
}

#[cfg(test)]
#[path = "session_reader_tests.rs"]
mod reader_tests;
