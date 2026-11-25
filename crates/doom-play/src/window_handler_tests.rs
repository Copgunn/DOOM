//! M6a part 2 follow-up: the window's key-down, key-up and focus-lost handlers, whose logic is
//! `input_log::raw_key` and `input_log::raw_focus_lost` (`window.rs` only calls them). These kill
//! the six survivors recorded at window.rs:303:9, 313:9, 351:9 and 357:35 (three). Headless.

use crate::audio::{Discard, Output};
use crate::input_log::{InputLog, LoggedHost, raw_focus_lost, raw_key};
use crate::melt_tests::{Shared, SinkHost, app, at};
use doom_game::app::Mode;
use miniquad::KeyCode;

fn logged() -> (Shared, LoggedHost<SinkHost>, doom_game::app::App) {
    let text = Shared::default();
    let out = Output::Discard(Discard::default());
    let mut host = LoggedHost {
        host: SinkHost { out },
        log: InputLog::to_writer(Box::new(text.clone())),
    };
    let app = app(&mut host);
    (text, host, app)
}

/// During a melt (the first update starts one): each handler writes its `raw` line, the app
/// holds the input, and the `app` line says so (357:35 `>` to `<`; the handlers to `()`).
#[test]
fn during_a_melt_the_handlers_log_and_the_app_holds() {
    let (text, mut host, mut app) = logged();
    app.update(at(1), &mut host);
    assert!(app.melt_in_progress());
    raw_key(&mut app, &mut host, KeyCode::Up, true, true);
    raw_key(&mut app, &mut host, KeyCode::Up, false, false);
    raw_focus_lost(&mut app, &mut host);
    assert_eq!(app.held_inputs(), 3);
    let log = text.text();
    let lines: Vec<&str> = log.lines().collect();
    let want = [
        "raw 1 1 key-down Up repeat=true",
        "app 1 1 held for the melt's end: key-down Up",
        "raw 1 1 key-up Up",
        "app 1 1 held for the melt's end: key-up Up",
        "raw 1 1 focus-lost",
        "app 1 1 held for the melt's end: focus-lost",
    ];
    assert!(lines.ends_with(&want), "{log}");
}

/// Outside a melt: the `raw` lines, the key reaches the game, focus loss pauses at once, and
/// nothing is logged as held (357:35 `>` to `==` and `>=`).
#[test]
fn outside_a_melt_the_handlers_act_at_once() {
    let (text, mut host, mut app) = logged();
    let mut k = 0;
    while k == 0 || app.melt_in_progress() {
        k += 1;
        app.update(at(k), &mut host);
    }
    raw_key(&mut app, &mut host, KeyCode::Up, true, false);
    raw_key(&mut app, &mut host, KeyCode::Up, false, false);
    assert_eq!(app.mode(), &Mode::Playing);
    raw_focus_lost(&mut app, &mut host);
    assert_eq!(app.mode(), &Mode::Paused, "focus loss pauses");
    assert_eq!(app.held_inputs(), 0);
    let log = text.text();
    for want in ["key-down Up repeat=false", "key-up Up", "focus-lost"] {
        assert_eq!(
            log.matches(&format!("\nraw 1 1 {want}\n")).count(),
            1,
            "{log}"
        );
    }
    assert!(!log.contains("held for"), "{log}");
}
