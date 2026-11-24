//! Auto-repeated keydowns through [`key_event`], the window's key path. `G_DoLoadLevel` clears
//! the held keys at a level load (g_game.c:490-496) and a repeating keyboard sets them again, so
//! a repeat must reach the app while it plays; on the pause and end screens a held Esc, R or Q
//! must act once. Rust-only tests of doom-play's key path on the synthetic episode (CHAIN's WADs).

use crate::keys::{Mute, key_event};
use doom_game::app::{App, GameData, Host, Mode, SessionConfig, SessionTicReport, Timestamp};
use doom_game::session::{ModeSetting, TicKind};
use miniquad::KeyCode;
use std::path::Path;

/// Counts the sessions begun and the quits' saves, and keeps each session tic's kind and move.
#[derive(Default)]
struct TestHost {
    begun: usize,
    tics: Vec<(bool, i8)>,
}

impl Host for TestHost {
    fn timestamp(&mut self) -> Timestamp {
        self.begun += 1;
        Timestamp::default()
    }
    fn exists(&mut self, _: &str) -> bool {
        false
    }
    fn save(&mut self, _: &str, _: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn session_tic(&mut self, r: &SessionTicReport<'_>) {
        let load = matches!(r.report.kind, TicKind::Load { .. });
        self.tics.push((load, r.executed.forward));
    }
}

impl Mute for TestHost {
    fn toggle_mute(&mut self) {}
}

fn start(tic_cap: usize) -> (App, TestHost) {
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
    let data = GameData::load_session(&refs, ModeSetting::Probe, 1, 2).expect("session data");
    let mut host = TestHost::default();
    let cfg = SessionConfig {
        tic_cap,
        ..SessionConfig::default()
    };
    let mut app = App::new(data, cfg, &mut host);
    app.update(0, &mut host);
    (app, host)
}

/// Runs the next tic, if the app is playing; the clock is the count of updates, each a little
/// over a tic's 28,571.4 microseconds, so exactly one tic is due whenever the clock was started.
fn tic(app: &mut App, host: &mut TestHost, clock: &mut u64) -> u64 {
    *clock += 1;
    app.update(*clock * 28_572, host).ran
}

/// CHAIN's first exit by keys (as doom-game's `m6a_load_input`), every key through `key_event`:
/// the up arrow goes down once before the load tic and stays down; a tic with no event; then
/// the window's auto-repeat of it arrives.
#[test]
fn a_repeat_through_key_event_sets_a_held_key_again_after_a_load() {
    let (mut app, mut host) = start(100_000);
    let mut clock = 0;
    key_event(&mut app, &mut host, KeyCode::Up, true, false);
    for _ in 0..125 {
        assert_eq!(tic(&mut app, &mut host, &mut clock), 1);
        key_event(&mut app, &mut host, KeyCode::Up, true, true); // dropped by the layer
    }
    key_event(&mut app, &mut host, KeyCode::Up, false, false);
    while app.session().expect("session").gameaction() != 8 {
        let down = host.tics.len() % 2 == 1;
        key_event(&mut app, &mut host, KeyCode::Space, down, false);
        assert_eq!(tic(&mut app, &mut host, &mut clock), 1);
    }
    key_event(&mut app, &mut host, KeyCode::Space, false, false);
    key_event(&mut app, &mut host, KeyCode::Up, true, false);
    assert_eq!(tic(&mut app, &mut host, &mut clock), 1);
    assert_eq!(tic(&mut app, &mut host, &mut clock), 1);
    key_event(&mut app, &mut host, KeyCode::Up, true, true);
    assert_eq!(tic(&mut app, &mut host, &mut clock), 1);
    let last = host.tics.split_off(host.tics.len() - 3);
    assert_eq!(last, [(true, 25), (false, 0), (false, 25)]);
    assert!(host.tics[..125].iter().all(|t| *t == (false, 25)));
}

/// Runs updates until one tic has run (the first update after a resume only starts the clock).
fn next_tic(app: &mut App, host: &mut TestHost, clock: &mut u64) -> (bool, i8) {
    let ran = (0..3).map(|_| tic(app, host, clock)).find(|ran| *ran != 0);
    assert_eq!(ran, Some(1));
    host.tics[host.tics.len() - 1]
}

/// The up arrow is held through a pause (`focus`: by a focus loss, else by Esc) and the resume
/// by Esc, with the keyboard repeating it throughout. Every pause releases the held keys through
/// the event layer; a repeat during the pause is dropped; the first repeat after the resume is a
/// keydown again. Returns the forward moves of the tic before the pause, the first tic after
/// the resume (no repeat yet) and the tic after the next repeat.
fn held_through_a_pause(focus: bool) -> [i8; 3] {
    let (mut app, mut host) = start(100_000);
    let mut clock = 0;
    key_event(&mut app, &mut host, KeyCode::Up, true, false);
    let before = next_tic(&mut app, &mut host, &mut clock).1;
    if focus {
        app.focus_lost();
    } else {
        key_event(&mut app, &mut host, KeyCode::Escape, true, false);
        key_event(&mut app, &mut host, KeyCode::Escape, false, false);
    }
    assert_eq!(app.mode(), &Mode::Paused);
    key_event(&mut app, &mut host, KeyCode::Up, true, true);
    assert_eq!(tic(&mut app, &mut host, &mut clock), 0);
    key_event(&mut app, &mut host, KeyCode::Escape, true, false);
    key_event(&mut app, &mut host, KeyCode::Escape, false, false);
    assert_eq!(app.mode(), &Mode::Playing);
    let released = next_tic(&mut app, &mut host, &mut clock).1;
    key_event(&mut app, &mut host, KeyCode::Up, true, true);
    [
        before,
        released,
        next_tic(&mut app, &mut host, &mut clock).1,
    ]
}

#[test]
fn a_key_held_through_a_pause_is_down_again_at_its_next_repeat() {
    assert_eq!(held_through_a_pause(false), [25, 0, 25]);
}

#[test]
fn a_key_held_through_a_focus_loss_is_down_again_at_its_next_repeat() {
    assert_eq!(held_through_a_pause(true), [25, 0, 25]);
}

fn repeats(app: &mut App, host: &mut TestHost, key: KeyCode) {
    for _ in 0..3 {
        key_event(app, host, key, true, true);
    }
    key_event(app, host, key, false, false);
}

/// A held Esc pauses once and, pressed again and held, resumes once; a held R on the pause
/// screen restarts once; a held Q quits and the app stays paused.
#[test]
fn a_held_app_key_acts_once_on_the_pause_screen() {
    let (mut app, mut host) = start(100_000);
    let mut clock = 0;
    assert_eq!(tic(&mut app, &mut host, &mut clock), 1);
    key_event(&mut app, &mut host, KeyCode::Escape, true, false);
    repeats(&mut app, &mut host, KeyCode::Escape);
    assert_eq!(app.mode(), &Mode::Paused);
    key_event(&mut app, &mut host, KeyCode::Escape, true, false);
    repeats(&mut app, &mut host, KeyCode::Escape);
    assert_eq!(app.mode(), &Mode::Playing);
    key_event(&mut app, &mut host, KeyCode::Pause, true, false);
    repeats(&mut app, &mut host, KeyCode::Pause);
    assert_eq!(app.mode(), &Mode::Paused);
    assert_eq!(host.begun, 1);
    key_event(&mut app, &mut host, KeyCode::R, true, false);
    repeats(&mut app, &mut host, KeyCode::R);
    assert_eq!((host.begun, app.mode()), (2, &Mode::Playing));
    key_event(&mut app, &mut host, KeyCode::Escape, true, false);
    key_event(&mut app, &mut host, KeyCode::Q, true, false);
    repeats(&mut app, &mut host, KeyCode::Q);
    assert_eq!((host.begun, app.mode()), (2, &Mode::Paused));
    assert_eq!(tic(&mut app, &mut host, &mut clock), 0);
}

/// On the end screen (the tic cap) a held R restarts once, and a held Esc does not pause or
/// resume anything: it quits, and no tic runs.
#[test]
fn a_held_app_key_acts_once_on_the_end_screen() {
    let (mut app, mut host) = start(2);
    let mut clock = 0;
    while app.mode() == &Mode::Playing {
        tic(&mut app, &mut host, &mut clock);
    }
    assert!(matches!(app.mode(), Mode::Ended(_)));
    key_event(&mut app, &mut host, KeyCode::R, true, false);
    repeats(&mut app, &mut host, KeyCode::R);
    assert_eq!((host.begun, app.mode()), (2, &Mode::Playing));
    while app.mode() == &Mode::Playing {
        tic(&mut app, &mut host, &mut clock);
    }
    key_event(&mut app, &mut host, KeyCode::Escape, true, false);
    repeats(&mut app, &mut host, KeyCode::Escape);
    assert!(matches!(app.mode(), Mode::Ended(_)));
    assert_eq!((host.begun, tic(&mut app, &mut host, &mut clock)), (2, 0));
}
