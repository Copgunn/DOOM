//! The app in the session's finale-and-melt configuration (owner's ruling V1 of 2026-11-23:
//! [`crate::session::finale_melt::Configuration::FinaleAndMelt`], chosen by
//! [`super::GameData::load_finale_and_melt_session`]; the frozen app over
//! [`super::GameData::load_session`] is unchanged).
//!
//! **The schedule** (M6a note §1.16.1, declared and accepted by the owner on 2026-11-21). Each tic
//! of wall time (1/35 s, the app's tic clock) is one *slot*. A slot runs one game tic, or, while a
//! melt is in progress, one melt step (`wipe_ScreenWipe` with `ticks = 1`, d_main.c:329-344). The
//! tic whose display wipes is followed **in its own slot** by the melt's first step, as in the C,
//! where the melt loop's first call comes at once (`wipestart = I_GetTime () - 1`, d_main.c:329,
//! so the first busy wait ends at once): the frame of a wiping tic is never shown by itself, only
//! as the melt's last pictures. Every later step takes one slot of its own. A slot that ran a melt
//! step ends the update, so every melt picture is presented ([`App::picture`]); when the host is
//! late the surplus slots are dropped, as for game tics (decision S8), and the melt takes longer in
//! wall time, never fewer pictures. The tic after the melt runs in the next slot at the normal
//! pace: the melt's slots are consumed by its steps, so the tic clock owes nothing afterwards and
//! makes no catch-up burst (the original's real-time path would run up to five tics at once after
//! a melt, d_net.c:404-405, 666-677; the declared schedule is the `-timedemo` case, M6a note
//! §1.16.5).
//!
//! **No game work during a melt step** (d_main.c:329-344): no event is posted, no command is built,
//! nothing is recorded or written to the demo, no sound is mixed or handed to the sink; the
//! buffer the melting tic owed is handed over with the melt's last step (d_main.c:397-405). The
//! sink hears [`super::AudioSink::melt_start`] after the wiping tic and
//! [`super::AudioSink::melt_end`] before that buffer.
//!
//! **Input during a melt** (owner's rulings of 2026-11-21 and 2026-11-23). Every key, mouse
//! button and focus loss that reaches the app while a melt is in progress is **held** and handed
//! to the app's own input calls, in order, when the melt's last step has been shown, before the
//! first tic after it: the app then treats it exactly as if it had arrived at that moment, so the
//! existing rules decide (the event layer of M5b S4 to S6, the pause, the restart and quit keys of
//! the pause screen). A key pressed and released inside one melt therefore reaches the event
//! layer as a press and a release in the same pump, and S6's same-pump rule holds the release back
//! by one command: the press shows in the first command after the melt and the release in the
//! second (in the C both events are handled before the first command and the key never shows,
//! M6a note §1.16.5; the app's rule already differs from the C in the same way for a tap shorter
//! than a tic). The app's pause (Esc, Pause, a focus loss) asked for during a melt takes effect
//! when the melt ends, before the next tic. Mouse **motion** during a melt is **ignored**, not
//! accumulated, as while paused. At most [`MAX_HELD_IN_MELT`] inputs are held; past it the held
//! inputs are forgotten and the event layer releases everything when the melt ends, as the layer
//! itself does when its own bound is reached.
use super::{App, AudioNote, Ending, Host, Mode, Update, Witness};
use crate::input::{Event, MouseButton, NUMKEYS};
use crate::session::finale_melt::{FinaleMeltTicKind, FinaleMeltTicReport, MeltStepReport};
use crate::session::{Session, SessionMode};
use doom_render::{Frame, GameState};
use doom_sim::TicCommand;

/// The most inputs held during one melt (the event layer's own bound on raw inputs between two
/// builds, 4 × `NUMKEYS`). Past it the held inputs are dropped and the layer releases every key
/// when the melt ends ([`crate::input::EventLayer::release_all`]), so memory stays bounded and no
/// key is left stuck.
pub const MAX_HELD_IN_MELT: usize = 4 * NUMKEYS;

/// One committed tic of the finale-and-melt configuration, as [`Host::finale_melt_tic`]
/// receives it.
#[derive(Clone, Copy, Debug)]
pub struct FinaleMeltSessionTicReport<'a> {
    /// The events posted before the tic's command was built.
    pub events: &'a [Event],
    /// The executed command: the demo's re-read copy.
    pub executed: TicCommand,
    /// The session after the tic.
    pub session: &'a Session,
    /// What the session tic did.
    pub report: &'a FinaleMeltTicReport,
}

/// One melt step the app showed, as [`Host::melt_step`] receives it.
#[derive(Clone, Copy, Debug)]
pub struct MeltStepShown<'a> {
    /// The session after the step.
    pub session: &'a Session,
    /// What the step did.
    pub report: &'a MeltStepReport,
    /// The step's picture (64,000 palette indices), shown under `report.palette`.
    pub picture: &'a Frame,
}

/// A fault planted by the app's evidence tests (`tests/m6a2_play.rs`) to prove that they fail
/// when the app is wrong. Never set by doom-play.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppFault {
    /// A key that arrives during a melt is dropped instead of held.
    DropKeyInMelt,
    /// Mouse motion during a melt reaches the event layer (accumulated across the melt).
    MotionInMelt,
    /// A command is built and recorded at every melt step.
    CommandInMeltStep,
    /// Every second melt step runs in the same slot as the step before it, so its picture is
    /// never presented.
    SkipMeltStep,
    /// The melt's slots are owed again when it ends, so the tic clock runs a burst of tics.
    CatchUpAfterMelt,
}

/// An input held during a melt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Held {
    Key {
        code: u8,
        down: bool,
    },
    Button {
        button: MouseButton,
        down: bool,
    },
    FocusLost,
    /// [`MAX_HELD_IN_MELT`] was passed.
    ReleaseAll,
}

/// The finale-and-melt configuration's state in the app (reset with every session).
#[derive(Clone, Debug, Default)]
pub(super) struct AppMelt {
    /// The last melt step's picture.
    picture: Frame,
    /// Its palette.
    palette: u8,
    /// A melt step's picture is presented (the melt has not ended).
    showing: bool,
    /// The inputs held during the current melt, in order.
    held: Vec<Held>,
    /// The sink was told a melt started and not yet that it ended.
    sink_melting: bool,
    /// Melts begun this session.
    melts: u64,
    /// The session tic of the victory (`F_StartFinale`), once reached.
    finale_began: Option<u64>,
    /// The session tic of the first art screen frame (`finalestage` 1), once drawn.
    art_began: Option<u64>,
    /// The session tic at which the art screen's frame was refused (registered mode without
    /// `HELP2`, ruling U4 (3)).
    art_refused: Option<u64>,
    /// A planted fault (tests only).
    fault: Option<AppFault>,
}

impl AppMelt {
    /// The state of a new session, keeping a planted fault.
    pub(super) fn fresh(&self) -> Self {
        Self {
            fault: self.fault,
            ..Self::default()
        }
    }
}

impl App {
    /// Whether this app runs the finale-and-melt configuration.
    pub(super) fn finale_and_melt(&self) -> bool {
        self.data.finale_and_melt()
    }

    /// Whether a melt is in progress: inputs are held, motion ignored and the next slot is a
    /// melt step (always false in the frozen configuration).
    pub fn melt_in_progress(&self) -> bool {
        self.mode == Mode::Playing
            && self
                .session
                .as_deref()
                .is_some_and(Session::melt_in_progress)
    }

    /// The inputs held for the end of the current melt.
    pub fn held_inputs(&self) -> usize {
        self.melt.held.len()
    }

    /// The melts begun in this session (finale-and-melt configuration).
    pub fn melts(&self) -> u64 {
        self.melt.melts
    }

    /// The session tic of the victory (the first finale tic), once reached.
    pub fn finale_began(&self) -> Option<u64> {
        self.melt.finale_began
    }

    /// The session tic of the finale's first art screen frame, once drawn.
    pub fn art_screen_began(&self) -> Option<u64> {
        self.melt.art_began
    }

    /// The session tic at which the finale's art screen was refused (registered mode without
    /// `HELP2`), if it was.
    pub fn art_screen_refused(&self) -> Option<u64> {
        self.melt.art_refused
    }

    /// Plants a fault (the app's evidence tests only).
    #[doc(hidden)]
    pub fn plant_fault(&mut self, fault: Option<AppFault>) {
        self.melt.fault = fault;
    }

    /// The presented melt picture and its palette, while a melt step's picture is shown.
    pub(super) fn melt_picture(&self) -> Option<(&[u8], u8)> {
        self.melt
            .showing
            .then(|| (self.melt.picture.pixels(), self.melt.palette))
    }

    /// Holds a key, button or focus loss that arrived during a melt; `false` when no melt is in
    /// progress (the caller handles the input).
    pub(super) fn hold_key(&mut self, code: u8, down: bool) -> bool {
        if !self.melt_in_progress() {
            return false;
        }
        if self.melt.fault != Some(AppFault::DropKeyInMelt) {
            self.hold(Held::Key { code, down });
        }
        true
    }

    pub(super) fn hold_button(&mut self, button: MouseButton, down: bool) -> bool {
        self.melt_in_progress() && {
            self.hold(Held::Button { button, down });
            true
        }
    }

    pub(super) fn hold_focus_lost(&mut self) -> bool {
        self.melt_in_progress() && {
            self.hold(Held::FocusLost);
            true
        }
    }

    /// Whether motion is ignored because a melt is in progress.
    pub(super) fn motion_ignored_in_melt(&self) -> bool {
        self.melt_in_progress() && self.melt.fault != Some(AppFault::MotionInMelt)
    }

    fn hold(&mut self, input: Held) {
        if self.melt.held.len() >= MAX_HELD_IN_MELT {
            self.melt.held.clear();
            self.melt.held.push(Held::ReleaseAll);
        }
        self.melt.held.push(input);
    }

    /// The update of the finale-and-melt configuration with `due` slots due (module
    /// documentation).
    pub(super) fn update_finale_and_melt(&mut self, due: u64, host: &mut dyn Host) -> Update {
        let (mut ran, mut steps, mut slots) = (0, 0, 0);
        while slots < due.min(super::MAX_TICS_PER_UPDATE)
            && self.mode == Mode::Playing
            && !self.quit
        {
            slots += 1;
            if !self.melt_in_progress() {
                self.run_tic(host);
                ran += 1;
                if !self.melt_in_progress() {
                    continue;
                }
            }
            // A melt step's picture is presented before anything else runs.
            self.melt_step(host);
            steps += 1;
            if self.melt.fault == Some(AppFault::SkipMeltStep)
                && self.melt_in_progress()
                && self
                    .session
                    .as_deref()
                    .is_some_and(|s| s.display().melt_steps_shown().is_some_and(|n| n % 2 == 1))
            {
                self.melt_step(host);
                steps += 1;
            }
            break;
        }
        Update {
            ran,
            dropped: due - slots,
            melt_steps: steps,
        }
    }

    /// One committed tic of the finale-and-melt session with the executed command (the frozen
    /// [`App::session_tic`]'s rules: a refusal made before anything changed drops the recorded
    /// command; a rule-2 refusal keeps it).
    pub(super) fn finale_melt_tic(
        &mut self,
        executed: TicCommand,
        host: &mut dyn Host,
    ) -> Result<(), Ending> {
        let Some(s) = self.session.as_deref_mut() else {
            return Ok(());
        };
        match s.tic_finale_and_melt(executed) {
            Ok(r) => {
                self.palette = r.palette;
                self.drawn = true;
                self.melt.showing = false;
                // As the frozen app: `G_DoLoadLevel` clears the command-building state
                // (g_game.c:490-496) after this tic's command was built.
                if matches!(r.kind, FinaleMeltTicKind::Load { .. }) {
                    self.input.load_level();
                    self.layer.load_level();
                }
                if r.kind == FinaleMeltTicKind::Victory {
                    self.melt.finale_began.get_or_insert(r.tic);
                }
                if r.finale.is_some_and(|v| v.stage == 1) {
                    self.melt.art_began.get_or_insert(r.tic);
                }
                if let (Some(b), Some(sink)) = (r.sound.sound.buffer.as_ref(), host.audio()) {
                    sink.buffer(b);
                }
                host.finale_melt_tic(&FinaleMeltSessionTicReport {
                    events: &self.events,
                    executed,
                    session: s,
                    report: &r,
                });
                if r.wiped {
                    self.melt.melts += 1;
                    self.melt.sink_melting = true;
                    self.audio_notes.push(AudioNote::MeltStart);
                    self.flush_audio(host);
                }
                Ok(())
            }
            Err(refusal) => {
                if s.ended().is_none() {
                    self.recorder.drop_last();
                } else if s.gamestate() == GameState::Finale
                    && s.finale().is_some_and(|v| v.stage == 1)
                {
                    // Ruling U4 (3): the first stage-1 frame refused (HELP2 missing).
                    self.melt.art_refused = Some(refusal.tic);
                }
                Err(Ending::Refused {
                    tic: refusal.tic,
                    witness: Witness::Session(Box::new(refusal.witness)),
                })
            }
        }
    }

    /// One melt step: its picture presented, [`Host::melt_step`] told; on the last step the sink
    /// hears the melt's end and then the owed buffer, and the inputs held during the melt are
    /// handed to the app in order.
    fn melt_step(&mut self, host: &mut dyn Host) {
        if self.melt.fault == Some(AppFault::CommandInMeltStep) {
            if let Ok(r) = self.input.tic() {
                let _ = self.recorder.record(r);
            }
        }
        let Some(s) = self.session.as_deref_mut() else {
            return;
        };
        let m = match s.melt_step(&mut self.melt.picture) {
            Ok(m) => m,
            Err(refusal) => {
                let witness = Witness::Session(Box::new(refusal.witness));
                return self.end(
                    Ending::Refused {
                        tic: refusal.tic,
                        witness,
                    },
                    host,
                );
            }
        };
        self.melt.palette = m.palette;
        self.melt.showing = !m.done;
        self.drawn = true;
        host.melt_step(&MeltStepShown {
            session: s,
            report: &m,
            picture: &self.melt.picture,
        });
        if !m.done {
            return;
        }
        if self.melt.fault == Some(AppFault::CatchUpAfterMelt) {
            let steps = u64::from(m.step);
            self.clock.consumed = self.clock.consumed.saturating_sub(steps);
        }
        self.melt.sink_melting = false;
        self.audio_notes.push(AudioNote::MeltEnd);
        self.flush_audio(host);
        if let (Some(b), Some(sink)) = (m.buffer.as_ref(), host.audio()) {
            sink.buffer(b);
        }
        self.release_held(host);
    }

    /// Hands the inputs held during the melt to the app, in order, as if each arrived now.
    fn release_held(&mut self, host: &mut dyn Host) {
        for input in std::mem::take(&mut self.melt.held) {
            match input {
                Held::Key { code, down } => self.key(code, down, host),
                Held::Button { button, down } => self.mouse_button(button, down),
                Held::FocusLost => self.focus_lost(),
                Held::ReleaseAll => {
                    if self.mode == Mode::Playing {
                        self.layer.release_all();
                    }
                }
            }
        }
        self.flush_audio(host);
    }

    /// The sink is told a melt ended when a session ends during one.
    pub(super) fn end_melt_for_sink(&mut self) {
        if std::mem::take(&mut self.melt.sink_melting) {
            self.audio_notes.push(AudioNote::MeltEnd);
        }
        self.melt.showing = false;
    }

    /// The finale's part of the exit line (rulings U4, U5): where the finale began and whether
    /// its art screen was reached or refused.
    pub(super) fn finale_summary(&self, mode: SessionMode) -> String {
        let Some(began) = self.melt.finale_began else {
            return String::new();
        };
        let art = match mode {
            SessionMode::Retail => "CREDIT",
            SessionMode::Registered => "HELP2",
        };
        let then = match (self.melt.art_began, self.melt.art_refused) {
            (Some(t), _) => format!("its art screen ({art}) began at session tic {t}"),
            (None, Some(t)) => format!("its art screen ({art}) was refused at session tic {t}"),
            (None, None) => "its art screen had not begun".into(),
        };
        format!("; the finale began at session tic {began}, {then}")
    }
}
