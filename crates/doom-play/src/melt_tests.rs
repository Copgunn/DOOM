//! M6a part 2: doom-play's side of the melt (the app is doom-game's finale-and-melt
//! configuration, compared with the genuine engine in doom-game's `tests/m6a2_play.rs`). Rust-only
//! checks of the presentation: the sound queue during a melt (silence, never a repeated buffer,
//! counted apart and never as an underrun, the output started again at the start threshold after
//! it), the exit lines, the input log's lines for motion ignored and inputs held during a melt,
//! and the tic statistics' melt intervals. Headless: no device, no window, the fake clock.

use crate::audio::{BUFFER_FRAMES, DeviceSink, MeltCounters, Output, Queue, Volume, melt_line};
use crate::input_log::{InputLog, LoggedHost, logged_key_event};
use crate::keys::Mute;
use crate::mouse_forward::Forward;
use crate::tic_stats::TicStats;
use doom_game::app::{App, AudioSink, GameData, Host, SessionConfig, Timestamp};
use doom_game::input::MouseButton;
use doom_game::session::ModeSetting;
use miniquad::KeyCode;
use std::io::Write;
use std::path::Path;

fn buffer(base: i16) -> [[i16; 2]; BUFFER_FRAMES] {
    std::array::from_fn(|i| {
        let v = base.wrapping_add(i as i16);
        [v, v.wrapping_neg()]
    })
}

fn pull(q: &mut Queue, frames: usize) -> Vec<i16> {
    let mut out = vec![0i16; 2 * frames];
    q.pull(&mut out);
    out
}

/// The queue during a melt: what it holds plays out, then silence (zeros, never a repeated
/// buffer), counted as melt silence and not as an underrun; after the melt the output waits for
/// the start threshold again and then plays the new buffers from their first frame.
#[test]
fn the_queue_plays_silence_during_a_melt_and_counts_no_underrun() {
    let mut q = Queue::new(Volume::FULL);
    q.push(&buffer(100));
    q.push(&buffer(1000));
    let first = pull(&mut q, 600);
    assert_eq!(first[0], 100, "started at the threshold");
    q.melt_start();
    // 424 frames left (the second buffer from its frame 88): they play, then 600 frames of silence
    let out = pull(&mut q, 1024);
    assert_eq!((out[0], out[2 * 423]), (1088, 1511));
    assert!(out[2 * 424..].iter().all(|&v| v == 0), "silence, no repeat");
    // still in the melt: the queue waits for its threshold; silence again, counted
    let out = pull(&mut q, 512);
    assert!(out.iter().all(|&v| v == 0));
    assert_eq!(q.counters().underruns, 0, "a melt is no underrun");
    assert_eq!(
        q.melt_counters(),
        MeltCounters {
            melts: 1,
            ran_dry: 1,
            silence_frames: 600 + 512,
        }
    );
    q.melt_end();
    // the melting tic's owed buffer, then the next: output starts again at two buffers
    q.push(&buffer(5000));
    assert!(
        pull(&mut q, 128).iter().all(|&v| v == 0),
        "waiting for the threshold"
    );
    q.push(&buffer(7000));
    let out = pull(&mut q, 128);
    assert_eq!(out[0], 5000, "the new buffer from its first frame");
    assert_eq!(q.counters().underruns, 0);
    assert_eq!(
        q.melt_counters().silence_frames,
        1112,
        "waiting after the melt is not counted"
    );
    // a second melt is counted; a session start ends a melt
    q.melt_start();
    q.session_start();
    q.push(&buffer(1));
    q.push(&buffer(2));
    let _ = pull(&mut q, 1024);
    let _ = pull(&mut q, 10);
    assert_eq!(
        q.counters().underruns,
        1,
        "after the session start a dry queue is an underrun"
    );
    assert_eq!(q.melt_counters().melts, 2);
}

/// The melt exit line: none without a melt; with melts it names them and the silence, and says
/// it is not an underrun.
#[test]
fn the_melt_line_says_the_silence_is_no_underrun() {
    assert_eq!(melt_line(MeltCounters::default()), None);
    assert_eq!(
        melt_line(MeltCounters {
            melts: 3,
            ran_dry: 2,
            silence_frames: 25_000,
        })
        .unwrap(),
        "audio during melts: 3 melts; 25000 frames of silence played while a melt ran (the queue \
         ran dry 2 times); linuxdoom-1.10 built with its in-process sound code mixes and submits \
         nothing during a melt, so this is no underrun and is not counted as one"
    );
    assert!(
        melt_line(MeltCounters {
            melts: 1,
            ran_dry: 1,
            silence_frames: 1,
        })
        .unwrap()
        .starts_with("audio during melts: 1 melt; 1 frames of silence played while a melt ran (the queue ran dry 1 time)")
    );
}

/// A host with a headless device sink.
struct SinkHost {
    out: Output,
}

impl Host for SinkHost {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, _: &str) -> bool {
        false
    }
    fn save(&mut self, _: &str, _: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn audio(&mut self) -> Option<&mut dyn AudioSink> {
        Some(&mut self.out)
    }
}

impl Mute for SinkHost {
    fn toggle_mute(&mut self) {}
}

/// The finale-and-melt app (doom-play's) on CHAIN's synthetic WADs.
fn app<H: Host>(host: &mut H) -> App {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
    let wads = [
        "m6a_wads/episode.wad",
        "m5b_gfx.wad",
        "m5c_sfx.wad",
        "m6a_wads/m6a_gfx.wad",
        "m6a_wads/m6a_music.wad",
    ];
    let files: Vec<Vec<u8>> = wads
        .iter()
        .map(|w| std::fs::read(dir.join(w)).expect("WAD"))
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let data = GameData::load_finale_and_melt_session(&refs, ModeSetting::Probe, 1, 2)
        .expect("session data");
    let mut app = App::new(data, SessionConfig::default(), host);
    app.update(0, host);
    app
}

fn at(k: u64) -> u64 {
    (k * 1_000_000).div_ceil(35)
}

/// Through the app: CHAIN's first exit and intermission by keys (as doom-game's `m6a_load_input`:
/// the up arrow held for 125 tics, then presses of the space bar on the exit switch and through
/// the tally), the device pulled 315 frames (one tic at 11,025 Hz) after every
/// update, one update per tic of wall time. The session-start melt, the melt into the
/// intermission and the melt into E1M2 starve the queue, which plays silence: counted as melt
/// silence and never as an underrun, so the exit lines say "underruns 0" and give the melts on a
/// line of their own. No buffer reaches the queue during a melt.
#[test]
fn melts_through_the_app_are_no_underrun_in_the_exit_lines() {
    use doom_render::GameState;
    let mut host = SinkHost {
        out: Output::Device(DeviceSink::headless(Volume::FULL)),
    };
    let mut app = app(&mut host);
    let mut out = vec![0i16; 2 * 315];
    let mut melt_pulls_silent = 0;
    app.key(doom_game::input::keys::KEY_UPARROW, true, &mut host);
    let done = |app: &App| {
        app.session()
            .is_some_and(|s| s.map() == 2 && s.gamestate() == GameState::Level)
            && !app.melt_in_progress()
    };
    let mut k = 0;
    while !done(&app) {
        k += 1;
        assert!(
            k < 2_000,
            "E1M2 not reached: {:?}",
            app.session().map(|s| (
                s.map(),
                s.gamestate(),
                s.gametic(),
                app.melts(),
                app.mode().clone()
            ))
        );
        if app.tic() == 125 {
            app.key(doom_game::input::keys::KEY_UPARROW, false, &mut host);
        }
        if app.tic() >= 125 {
            // presses of the space bar: the exit switch, then the tally (a rising edge every
            // other update)
            app.key(b' ', k % 2 == 0, &mut host);
        }
        let u = app.update(at(k), &mut host);
        assert!(u.ran <= 1 && u.dropped == 0, "{u:?}");
        let melting = app.melt_in_progress();
        let Output::Device(d) = &host.out else {
            unreachable!()
        };
        d.pull(&mut out);
        if melting && out.iter().all(|&v| v == 0) {
            melt_pulls_silent += 1;
        }
    }
    let Output::Device(d) = &host.out else {
        unreachable!()
    };
    let m = d.melt_counters();
    assert_eq!(m.melts, app.melts());
    assert!(app.melts() >= 3, "{}", app.melts());
    assert!(m.ran_dry >= 2 && m.silence_frames > 0, "{m:?}");
    assert!(melt_pulls_silent > 60, "{melt_pulls_silent}");
    let lines = d.exit_lines();
    assert!(
        lines[3].contains("underruns 0 (0 frames of silence)"),
        "{}",
        lines[3]
    );
    assert!(
        !lines.iter().any(|l| l.starts_with("audio underruns")),
        "{lines:?}"
    );
    assert_eq!(lines.last(), melt_line(m).as_ref());
    eprintln!("{}", lines.join("\n"));
}

#[derive(Clone, Default)]
struct Shared(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Shared {
    fn text(&self) -> String {
        String::from_utf8(self.0.borrow().clone()).expect("UTF-8")
    }
}

/// The input log during a melt (the owner's rulings of 2026-11-21 and 2026-11-23): raw motion is
/// logged `ignored (melt)` the way paused motion is logged `ignored`, and no `mouse-motion` reaches
/// the app; a key and a button the app holds get an `app ... held for the melt's end` line; the
/// update's `pump` line names its melt steps; after the melt the held events are the first tic's
/// `game` lines, in order, and the motion is not among them.
#[test]
fn the_input_log_records_held_inputs_and_ignored_motion_during_a_melt() {
    let text = Shared::default();
    let mut host = LoggedHost {
        host: SinkHost {
            out: Output::Discard(crate::audio::Discard::default()),
        },
        log: InputLog::to_writer(Box::new(text.clone())),
    };
    let mut app = app(&mut host);
    let mut fwd = Forward::default();
    fwd.sync_capture(&app, &mut host.log, || (960.0, 720.0));
    let u = app.update(at(1), &mut host);
    assert_eq!((u.ran, u.melt_steps), (1, 1));
    host.log.melt_steps(u.melt_steps);
    host.log.pump_end(&app, u.ran, u.dropped, fwd.captured());
    assert!(app.melt_in_progress());
    fwd.raw_motion(&mut app, &mut host.log, 5.0, 5.0);
    logged_key_event(&mut app, &mut host, KeyCode::Up, true, false);
    fwd.button(
        &mut app,
        &mut host.log,
        Some(MouseButton::Left),
        true,
        (1.0, 1.0),
    );
    assert_eq!(app.held_inputs(), 2);
    let mut k = 1;
    while app.melt_in_progress() {
        k += 1;
        let u = app.update(at(k), &mut host);
        host.log.melt_steps(u.melt_steps);
        host.log.pump_end(&app, u.ran, u.dropped, fwd.captured());
    }
    k += 1;
    let u = app.update(at(k), &mut host);
    assert_eq!((u.ran, u.melt_steps), (1, 0));
    host.log.pump_end(&app, u.ran, u.dropped, fwd.captured());
    let log = text.text();
    for want in [
        "raw 2 1 motion dx=5 dy=5 ignored (melt)",
        "app 2 1 held for the melt's end: key-down Up",
        "app 2 1 held for the melt's end: mouse-button Left down",
        "pump 1 1 ran=1 dropped=0 mode=playing capture=on melt-steps=1",
        "pump 2 1 ran=0 dropped=0 mode=playing capture=on melt-steps=1",
    ] {
        assert!(log.contains(want), "{want} not in:\n{log}");
    }
    assert!(!log.contains("mouse-motion"), "{log}");
    // `game PUMP TIC TEXT`: the held events, in order, at tic 1, the first after the melt
    let game: Vec<(String, String)> = log
        .lines()
        .filter(|l| l.starts_with("game "))
        .map(|l| {
            let f: Vec<&str> = l.splitn(4, ' ').collect();
            (f[2].to_string(), f[3].to_string())
        })
        .collect();
    let want = [("1", "keydown 0xad"), ("1", "mouse buttons=1 dx=0 dy=0")];
    assert_eq!(
        game,
        want.map(|(t, e)| (t.to_string(), e.to_string())),
        "{log}"
    );
}

/// The tic statistics leave the interval across a melt out and report it apart; with no melt the
/// summary is unchanged.
#[test]
fn the_tic_statistics_report_melt_intervals_apart() {
    let mut t = TicStats::default();
    t.record(0, 1, 0);
    t.record(28_571, 1, 0);
    t.melt();
    t.record(28_571 + 1_000_000, 1, 0);
    t.record(28_571 + 1_028_571, 1, 0);
    let s = t.summary();
    assert!(s.contains("executed 4, dropped 0"), "{s}");
    assert!(
        s.contains(
            "(first to last tic; the 1 interval across a melt (one melt step per tic of wall \
             time, no game tic), 1.000 s, not counted)"
        ),
        "{s}"
    );
    assert!(s.contains("max 28571"), "{s}");
    let mut plain = TicStats::default();
    plain.record(0, 1, 0);
    plain.record(28_571, 1, 0);
    plain.record(57_142, 1, 0);
    assert!(!plain.summary().contains("melt"));
}
