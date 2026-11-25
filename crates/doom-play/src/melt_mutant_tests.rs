//! M6a part 2 mutation-survivor tests (cargo-mutants 27.1.0 over doom-play's diff). Each pins a
//! guard the melt tests left open; headless, no device, no window, the fake clock.

use crate::audio::{BUFFER_FRAMES, DeviceSink, MeltCounters, Output, Queue, Volume};
use crate::closing::Play;
use crate::input_log::{InputLog, LoggedHost, logged_key_event};
use crate::melt_tests::{Shared, SinkHost, app, at, buffer};
use crate::mouse_forward::Forward;
use crate::session_writer::tests::{TestIo, at as wat, scratch, writer};
use doom_game::app::{AudioSink, Host, MeltStepShown, Timestamp};
use doom_render::GameState;
use miniquad::KeyCode;
use std::{cell::Cell, rc::Rc};

/// Kills audio.rs:696:20 (`missing > 0` to `>= 0` in `Queue::pull`): a pull during a melt that
/// takes exactly what the queue holds ran nothing dry, so no `ran_dry` and no silence is counted.
#[test]
fn a_pull_that_empties_the_queue_exactly_during_a_melt_does_not_run_dry() {
    let mut q = Queue::new(Volume::FULL);
    q.push(&buffer(100));
    q.push(&buffer(1000));
    q.melt_start();
    let mut out = vec![0i16; 2 * 2 * BUFFER_FRAMES];
    q.pull(&mut out);
    assert_eq!(out[0], 100);
    let none = MeltCounters {
        melts: 1,
        ran_dry: 0,
        silence_frames: 0,
    };
    assert_eq!(q.melt_counters(), none, "exactly empty is not dry");
    q.pull(&mut out[..2]);
    assert_eq!(q.melt_counters().ran_dry, 1, "one frame short is dry");
}

/// Kills audio.rs:1088:9 (`DeviceSink::melt_end` replaced with `()`) and audio.rs:1215:9
/// (`Output::melt_end` replaced with `()`): after `melt_end` a dry queue is an underrun, not melt
/// silence (with `melt_end` a no-op the queue stays melting: `ran_dry` 1 and no underrun).
#[test]
fn melt_end_reaches_the_queue_through_the_device_sink_and_the_output() {
    let mut out = vec![0i16; 2 * 1024];
    let mut d = DeviceSink::headless(Volume::FULL);
    d.buffer(&buffer(100));
    d.buffer(&buffer(1000));
    d.pull(&mut out[..2 * 600]);
    d.melt_start();
    d.melt_end();
    d.pull(&mut out);
    assert_eq!(d.melt_counters().ran_dry, 0, "DeviceSink::melt_end");
    assert!(
        d.exit_lines()[3].contains("underruns 1"),
        "{:?}",
        d.exit_lines()
    );
    let mut o = Output::Device(DeviceSink::headless(Volume::FULL));
    o.buffer(&buffer(100));
    o.buffer(&buffer(1000));
    let Output::Device(d) = &o else {
        unreachable!()
    };
    d.pull(&mut out[..2 * 600]);
    o.melt_start();
    o.melt_end();
    let Output::Device(d) = &o else {
        unreachable!()
    };
    d.pull(&mut out);
    assert_eq!(d.melt_counters().ran_dry, 0, "Output::melt_end");
    assert!(
        d.exit_lines()[3].contains("underruns 1"),
        "{:?}",
        d.exit_lines()
    );
}

/// A host that counts the melt steps it is told of.
struct Counting(Rc<Cell<u64>>);

impl Host for Counting {
    fn timestamp(&mut self) -> doom_game::app::Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, _: &str) -> bool {
        false
    }
    fn save(&mut self, _: &str, _: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn melt_step(&mut self, _: &MeltStepShown<'_>) {
        self.0.set(self.0.get() + 1);
    }
}

/// Kills input_log.rs:555:9 (`LoggedHost::melt_step` replaced with `()`): the logging wrapper
/// passes every melt step on to the host it wraps, as many as the app showed.
#[test]
fn the_logging_host_passes_every_melt_step_on() {
    let seen = Rc::new(Cell::new(0));
    let mut host = LoggedHost {
        host: Counting(seen.clone()),
        log: InputLog::off(),
    };
    let mut app = app(&mut host);
    let mut shown = 0;
    for k in 1..=20 {
        shown += app.update(at(k), &mut host).melt_steps;
    }
    assert!(shown >= 10, "{shown}");
    assert_eq!(seen.get(), shown);
}

fn logged() -> (Shared, LoggedHost<SinkHost>, doom_game::app::App) {
    let text = Shared::default();
    let mut host = LoggedHost {
        host: SinkHost {
            out: Output::Discard(crate::audio::Discard::default()),
        },
        log: InputLog::to_writer(Box::new(text.clone())),
    };
    let app = app(&mut host);
    (text, host, app)
}

/// Kills input_log.rs:434:25 (`&&` to `||`), 434:46 (`ran > 0` to `>= 0`), 434:59 (`steps > 0` to
/// `== 0`, `< 0` and `>= 0`) and 441:33 (`steps > 0` to `>= 0`) in `InputLog::pump_end`: an idle
/// pump (nothing ran, no melt step, nothing logged) writes no `pump` line yet still numbers the
/// next pump; a pump with only melt steps writes a line naming them; a pump with only a tic writes
/// a line that names no melt steps.
#[test]
fn pump_lines_are_written_only_when_something_happened_and_name_melt_steps_only_if_any() {
    let (text, mut host, app) = logged();
    // the session line made the log busy: the first pump ends with a line (pump 1); then idle
    host.log.pump_end(&app, 0, 0, false);
    host.log.pump_end(&app, 0, 0, false);
    host.log.melt_steps(1);
    host.log.pump_end(&app, 0, 0, false);
    host.log.pump_end(&app, 1, 0, false);
    let log = text.text();
    let lines: Vec<&str> = log.lines().filter(|l| l.starts_with("pump ")).collect();
    assert_eq!(lines.len(), 3, "idle pump 2 wrote nothing: {log}");
    assert!(lines[1].starts_with("pump 3 ") && lines[1].ends_with("capture=off melt-steps=1"));
    assert!(lines[2].starts_with("pump 4 ") && lines[2].ends_with("mode=playing capture=off"));
    assert!(!lines[2].contains("melt-steps"), "{}", lines[2]);
}

/// Kills input_log.rs:571:26 (`held_inputs() > before` to `>=` in `logged_key_event`): a key the
/// app takes at once (the melt is over, so it is not held) gets no `held for the melt's end` line.
#[test]
fn a_key_taken_at_once_is_not_logged_as_held() {
    let (text, mut host, mut app) = logged();
    let mut k = 0;
    while k == 0 || app.melt_in_progress() {
        k += 1;
        app.update(at(k), &mut host);
    }
    let before = app.held_inputs();
    logged_key_event(&mut app, &mut host, KeyCode::Up, true, false);
    assert_eq!(app.held_inputs(), before);
    assert!(!text.text().contains("held for"), "{}", text.text());
}

/// Kills closing.rs:127:29 (`u.melt_steps > 0` to `< 0` in `Play::update`): through a whole play
/// run (the exit, the intermission, E1M2 by keys as in the melt tests) `Play::update` tells the
/// statistics of each melt, so the summary reports the intervals across melts apart.
#[test]
fn play_update_tells_the_statistics_of_each_melt() {
    let dir = scratch("melt-mutant");
    let mut host = LoggedHost {
        host: writer(&dir, TestIo::default()),
        log: InputLog::off(),
    };
    let a = app(&mut host);
    let mut p = Play::new(a, host, Forward::default(), true);
    p.app
        .key(doom_game::input::keys::KEY_UPARROW, true, &mut p.host);
    let mut k = 0;
    let done = |p: &Play| {
        p.app
            .session()
            .is_some_and(|s| s.map() == 2 && s.gamestate() == GameState::Level)
            && !p.app.melt_in_progress()
    };
    while !done(&p) {
        k += 1;
        assert!(k < 2_000, "E1M2 not reached");
        if p.app.tic() == 125 {
            p.app
                .key(doom_game::input::keys::KEY_UPARROW, false, &mut p.host);
        }
        if p.app.tic() >= 125 {
            p.app.key(b' ', k % 2 == 0, &mut p.host);
        }
        p.update(wat(k));
    }
    let s = p.stats.as_ref().unwrap().summary();
    assert!(s.contains("interval") && s.contains("across a melt"), "{s}");
    p.quit();
    std::fs::remove_dir_all(&dir).unwrap();
}
