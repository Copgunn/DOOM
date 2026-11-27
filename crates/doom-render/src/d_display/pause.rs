//! M6b wire 29: the original's pause in the session display (rulings N5 and N6;
//! docs/M6B_REFERENCE_INVESTIGATION.md §4.4), saved as `DDSP` version 5
//! ([`SessionDisplayStateV5`]).
//!
//! A pause display ([`Display::new_session_pause`]) is the finale-and-melt display
//! ([`SessionConfig::FinaleAndMelt`]) with what the pause changes:
//!
//! - **The picture.** `D_Display` draws the frame as always, then, while `paused`, the patch
//!   `M_PAUSE` at x = `viewwindowx + (scaledviewwidth - 68) / 2`, y = `viewwindowy + 4`
//!   (d_main.c:303-311; y = 4 with the automap up, which no session has), in every game state,
//!   with the patch's offsets subtracted and its range check (`V_DrawPatchDirect` calls
//!   `V_DrawPatch`, v_video.c:219-232, 336-343). It goes into `screens[0]` after the HUD and before
//!   `wipe_EndScreen` (d_main.c:303-327), so a melt's end screen holds it and the next frame is
//!   composed over it. The session gives `paused` after each tic's toggle
//!   ([`Display::set_paused`]); a level tic is observed paused ([`Display::observe_paused`])
//!   exactly when it is set.
//! - **A missing `M_PAUSE`.** The genuine engine stops in `W_CacheLumpName` on the first paused
//!   frame (w_wad.c:399-409). The display refuses to be set paused without a usable picture
//!   ([`SessionError::PausePicture`] with `None`), so the session refuses the tic whose frame would
//!   be the first paused one, its state kept.
//! - **Paused level tics.** A paused level tic is observed (`ST_Ticker` and `HU_Ticker` run,
//!   g_game.c:729-734) without the renderer, which follows the simulation (p_tick.c:135-136): the
//!   level scene's tic is the session tics since the state was entered less the paused ones,
//!   which the state counts, and the level may be drawn before its tic 0 has run (a paused load
//!   tic).
//!
//! **`DDSP` version 5** is version 4's layout ([`SessionDisplayStateV4::encode`]) with the version
//! 5 and nine bytes inserted at offset 8, after the version: u8 `paused` (0 or 1) and u64 the
//! level's paused tics since the state was entered. Rules: version 4's, with a level scene's tic
//! plus the paused tics equal to the session tics since `entered` (version 4: the tic alone) and
//! the status bar checks of a restore made at that many observed tics; no paused tic outside a
//! level; a level scene shown paused after a tic has at least one paused tic. Versions 1 to 4
//! keep their bytes and readers, and each refuses version 5.
use super::finale_melt::{SessionDisplayStateV4, SessionStateRefusal};
use super::{GameState, Scene, SessionConfig, SessionError, SessionRestoreError};
use crate::d_display::{Display, DisplayConfig, DisplayRestoreError, MAGIC};
use crate::v_video::{Canvas, DrawError, Patch, Screen};
use crate::{GameMode, Level, Resources, StateDecodeError};
use doom_sim::Simulation;
use std::sync::Arc;

/// `DDSP` version 5: the session display state of a pause display.
pub const VERSION_PAUSE: u32 = 5;
/// The width d_main.c:309 centres (`M_PAUSE`'s in the original WADs).
const CENTRED_WIDTH: i32 = 68;
/// Where version 5's fields begin (after the magic and the version) and their length.
const AT: usize = 8;
const LEN: usize = 9;

/// The pause part of a pause display.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pause {
    /// `paused` after the last tic.
    pub(crate) paused: bool,
    /// The level's paused tics since the state was entered.
    pub(crate) level_paused: u64,
    /// `M_PAUSE`, if the lump exists and is a patch.
    picture: Option<Patch>,
    /// The planted fault of the evidence self-test: the picture one row lower.
    lower: bool,
}

impl Pause {
    /// The same picture with nothing paused (a new game, a state entry).
    pub(crate) fn fresh(self) -> Self {
        Self {
            paused: false,
            level_paused: 0,
            ..self
        }
    }
}

/// The fields `DDSP` version 5 adds.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PauseFields {
    /// `paused` after the last tic: the next frame draws `M_PAUSE`.
    pub paused: bool,
    /// The level's paused tics since the state was entered (0 outside a level).
    pub level_paused: u64,
}

/// The saved state of a pause display, `DDSP` version 5 (module documentation).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionDisplayStateV5 {
    /// Everything version 4 holds.
    pub display: SessionDisplayStateV4,
    /// The fields version 5 adds.
    pub pause: PauseFields,
}

impl Display {
    /// A pause display over loaded resources and the first level: the finale-and-melt display of
    /// game mode `mode` with the pause (module documentation). A missing or malformed `M_PAUSE`
    /// is not refused here: the genuine engine reads it on the first paused frame only.
    pub fn new_session_pause(
        res: Arc<Resources>,
        level: Arc<Level>,
        cfg: DisplayConfig,
        mode: GameMode,
    ) -> Result<Self, crate::ResourceError> {
        let picture = res
            .directory()
            .check_num_for_name(b"M_PAUSE")
            .and_then(|lump| Patch::load(&res, lump));
        let mut d = Self::new_session_with(res, level, cfg, SessionConfig::FinaleAndMelt { mode })?;
        if let Some(s) = d.session.as_deref_mut() {
            s.pause = Some(Pause {
                paused: false,
                level_paused: 0,
                picture,
                lower: false,
            });
        }
        Ok(d)
    }

    pub(crate) fn pause_part(&self) -> Option<Pause> {
        self.session.as_deref().and_then(|s| s.pause)
    }

    /// `paused` and the level's paused tics of a pause display (None for any other display).
    pub fn pause_fields(&self) -> Option<PauseFields> {
        self.pause_part().map(|p| PauseFields {
            paused: p.paused,
            level_paused: p.level_paused,
        })
    }

    /// Sets `paused` after the tic's toggle (g_game.c:704-709, a load's clear at g_game.c:494), for
    /// the tic about to be observed and its frame. Refused, with nothing changed: on a display
    /// that is not a pause display ([`SessionError::NotPause`]), during a melt
    /// ([`SessionError::MeltInProgress`]), and when paused without a usable `M_PAUSE`
    /// ([`SessionError::PausePicture`] with `None`: the genuine engine's stop on the first paused
    /// frame, w_wad.c:399-409).
    pub fn set_paused(&mut self, paused: bool) -> Result<(), SessionError> {
        let s = self
            .session
            .as_deref_mut()
            .ok_or(SessionError::NotSession)?;
        if s.melt.is_some() {
            return Err(SessionError::MeltInProgress);
        }
        let p = s.pause.as_mut().ok_or(SessionError::NotPause)?;
        if paused && p.picture.is_none() {
            return Err(SessionError::PausePicture(None));
        }
        p.paused = paused;
        Ok(())
    }

    /// Where `M_PAUSE` is drawn before its offsets (d_main.c:303-311), at this display's view
    /// size; None for a display that is not a pause display.
    pub fn pause_origin(&self) -> Option<(i32, i32)> {
        let p = self.pause_part()?;
        let (x, y) = pause_origin_for(self.renderer.view_size());
        Some((x, y + i32::from(p.lower)))
    }

    /// The picture and its origin when the next frame draws it (paused), else None.
    pub(crate) fn pause_draw(&self) -> Option<(i32, i32, Patch)> {
        let p = self.pause_part().filter(|p| p.paused)?;
        let (x, y) = self.pause_origin()?;
        Some((
            x,
            y,
            p.picture.expect("set_paused refuses a missing picture"),
        ))
    }

    /// Draws `M_PAUSE` over the composed frame when paused (module documentation); nothing on a
    /// display that is not paused.
    pub(crate) fn draw_pause(&self, fg: &mut [u8], taint: &mut [u32]) -> Result<(), DrawError> {
        draw_pause_at(&self.res, fg, taint, self.pause_draw())
    }

    /// Plants the evidence self-test's fault: the picture one row lower (a display holding it
    /// cannot be saved). Ignored by a display that is not a pause display.
    #[doc(hidden)]
    pub fn plant_pause_fault(&mut self) {
        if let Some(p) = self.session.as_deref_mut().and_then(|s| s.pause.as_mut()) {
            p.lower = true;
        }
    }

    /// The saveable state of a pause display (`DDSP` version 5); None for another display.
    pub fn session_state_v5(&self) -> Option<SessionDisplayStateV5> {
        self.session_state_v5_checked().ok()
    }

    /// [`Self::session_state_v5`] with the reason for a refusal named.
    pub fn session_state_v5_checked(&self) -> Result<SessionDisplayStateV5, SessionStateRefusal> {
        let p = self
            .pause_part()
            .ok_or(SessionStateRefusal::NotPauseDisplay)?;
        if p.lower {
            return Err(SessionStateRefusal::PlantedFault);
        }
        let display = self.session_state_v4_any()?;
        Ok(SessionDisplayStateV5 {
            display,
            pause: PauseFields {
                paused: p.paused,
                level_paused: p.level_paused,
            },
        })
    }

    /// Restores a `DDSP` version 5 state after [`SessionDisplayStateV5::check`], as
    /// [`Self::restore_session_v4`] restores version 4. Refused with
    /// [`SessionRestoreError::WrongConfig`] unless the display is a pause display of the state's
    /// game mode. On an error the display keeps its state.
    pub fn restore_session_v5(
        &mut self,
        state: SessionDisplayStateV5,
        level: Option<(Arc<Level>, &Simulation)>,
    ) -> Result<(), SessionRestoreError> {
        if self.pause_part().is_none() {
            return Err(SessionRestoreError::WrongConfig);
        }
        let pause = state.pause;
        self.restore_v4_with(state.display, level, Some(pause))
    }
}

impl SessionDisplayStateV5 {
    /// The checks that need no simulation (module documentation).
    pub fn check(&self) -> Result<(), SessionRestoreError> {
        Self::check_parts(&self.display, self.pause)
    }

    /// [`Self::check`] of a version 4 state and the version 5 fields given apart.
    pub fn check_parts(
        display: &SessionDisplayStateV4,
        pause: PauseFields,
    ) -> Result<(), SessionRestoreError> {
        display.check_paused(pause.level_paused)?;
        let s = &display.session;
        let bad = |n| {
            Err(SessionRestoreError::Display(
                DisplayRestoreError::Inconsistent(n),
            ))
        };
        match s.scene.game_state() {
            GameState::Level => {
                let since = s.tic - s.entered;
                if pause.paused && since >= 1 && pause.level_paused == 0 {
                    return bad("paused without a paused tic");
                }
                Ok(())
            }
            _ if pause.level_paused != 0 => bad("paused tics outside a level"),
            _ => Ok(()),
        }
    }

    /// The `DDSP` version 5 encoding (module documentation).
    pub fn encode(&self) -> Vec<u8> {
        Self::encode_parts(&self.display, self.pause)
    }

    /// [`Self::encode`] of a version 4 state and the version 5 fields given apart.
    pub fn encode_parts(display: &SessionDisplayStateV4, pause: PauseFields) -> Vec<u8> {
        let mut o = display.encode();
        o[4..AT].copy_from_slice(&VERSION_PAUSE.to_le_bytes());
        let mut extra = vec![u8::from(pause.paused)];
        extra.extend_from_slice(&pause.level_paused.to_le_bytes());
        o.splice(AT..AT, extra);
        o
    }

    /// Decodes a `DDSP` version 5 encoding: the two fields, then the rest with version 4's
    /// decoder (every count and range it checks). Every other version is refused as `Version`.
    /// The rules across fields are [`Self::check`]'s.
    pub fn decode(bytes: &[u8]) -> Result<Self, StateDecodeError> {
        if bytes.get(..4).ok_or(StateDecodeError::Truncated)? != MAGIC {
            return Err(StateDecodeError::Magic);
        }
        let version = u32::from_le_bytes(
            bytes
                .get(4..AT)
                .ok_or(StateDecodeError::Truncated)?
                .try_into()
                .expect("4 bytes"),
        );
        if version != VERSION_PAUSE {
            return Err(StateDecodeError::Version(version));
        }
        let extra = bytes.get(AT..AT + LEN).ok_or(StateDecodeError::Truncated)?;
        let paused = match extra[0] {
            0 => false,
            1 => true,
            _ => return Err(StateDecodeError::Invalid("paused")),
        };
        let level_paused = u64::from_le_bytes(extra[1..].try_into().expect("8 bytes"));
        let mut rest = Vec::with_capacity(bytes.len() - LEN);
        rest.extend_from_slice(MAGIC);
        rest.extend_from_slice(&super::finale_melt::VERSION_FINALE_MELT.to_le_bytes());
        rest.extend_from_slice(&bytes[AT + LEN..]);
        Ok(Self {
            display: SessionDisplayStateV4::decode(&rest)?,
            pause: PauseFields {
                paused,
                level_paused,
            },
        })
    }
}

/// Where `M_PAUSE` is drawn before its offsets for the view size `v`, the automap down
/// (d_main.c:303-311): x = `viewwindowx + (scaledviewwidth - 68) / 2`, y = `viewwindowy + 4`.
pub fn pause_origin_for(v: &crate::r_main::ViewSize) -> (i32, i32) {
    (
        v.viewwindowx + (v.scaledviewwidth - CENTRED_WIDTH) / 2,
        v.viewwindowy + 4,
    )
}

/// `V_DrawPatchDirect(x, y, 0, M_PAUSE)` (v_video.c:336-343) when `at` is given.
pub(crate) fn draw_pause_at(
    res: &Resources,
    fg: &mut [u8],
    taint: &mut [u32],
    at: Option<(i32, i32, Patch)>,
) -> Result<(), DrawError> {
    let Some((x, y, picture)) = at else {
        return Ok(());
    };
    let (mut bg, mut bg_defined) = (Vec::new(), Vec::new());
    let mut c = Canvas {
        res,
        fg,
        taint,
        bg: &mut bg,
        bg_defined: &mut bg_defined,
    };
    c.patch(x, y, Screen::Fg, &picture).map(|_| ())
}

/// Whether a level scene's tic plus the paused tics equals the session tics since `entered`.
pub(crate) fn level_tic_ok(scene: &Scene, since: u64, level_paused: u64) -> bool {
    match scene {
        Scene::Level(r) => r.tic.checked_add(level_paused) == Some(since),
        _ => true,
    }
}
