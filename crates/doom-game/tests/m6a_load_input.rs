//! `G_DoLoadLevel` clears the held keys (g_game.c:490-496) at every session load after the first.
//! A key held across a load is in the load tic's command, built before `G_Ticker` (d_net.c:408,
//! 745), and in no later command until a keydown arrives. The event layer forgets its held keys
//! at the load, so an operating-system key repeat is that keydown, and it posts the run key again
//! under always-run. Rust-only tests of the app's event layer; the C facts are cited.
#![allow(clippy::unwrap_used, reason = "test code")]

mod m6a;

use doom_game::app::{App, GameData, Host, SessionConfig, SessionTicReport, Timestamp};
use doom_game::input::{Event, Preset, keys};
use doom_game::session::{ModeSetting, Session, TicKind};

/// Each session tic: whether it was a load, its command's forward move, and whether the input
/// log of the tic holds a keydown of the run key.
#[derive(Default)]
struct Cmds(Vec<(bool, i8, bool)>);

impl Host for Cmds {
    fn timestamp(&mut self) -> Timestamp {
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
        let run = r.events.contains(&Event::KeyDown(keys::KEY_RSHIFT));
        self.0.push((load, r.executed.forward, run));
    }
}

/// Plays `name` by the preset's keys: forward for `up` tics, then use tapped until a load is
/// `pending`. Then the forward key goes down once and stays down: the load tic, a tic with no
/// input, and a tic before which the keyboard's repeat of the forward key arrives. Returns those
/// three tics. Under always-run the physical run key is held (walking, so the route is the
/// vanilla one) until the forward key's last press, where it is released (running).
fn held_over_load(
    name: &str,
    preset: Preset,
    up: usize,
    pending: fn(&Session) -> bool,
) -> Vec<(bool, i8, bool)> {
    let item = m6a::item(name);
    let read = |w: &String| std::fs::read(m6a::root(w)).unwrap();
    let files: Vec<Vec<u8>> = item.wads.iter().map(read).collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let data = GameData::load_session(&refs, ModeSetting::Probe, item.map, item.skill).unwrap();
    let mut host = Cmds::default();
    let cfg = SessionConfig {
        preset,
        ..SessionConfig::default()
    };
    let mut app = App::new(data, cfg, &mut host);
    app.update(0, &mut host);
    let tic = |app: &mut App, host: &mut Cmds| {
        let now = (host.0.len() as u64 + 1) * 1_000_000;
        assert_eq!(app.update(now.div_ceil(35), host).ran, 1);
    };
    let (forward, use_key) = (preset.controls().key_up(), preset.controls().key_use());
    let always_run = preset.layer_options().always_run;
    if always_run {
        app.key(keys::KEY_RSHIFT, true, &mut host);
    }
    app.key(forward, true, &mut host);
    (0..up).for_each(|_| tic(&mut app, &mut host));
    app.key(forward, false, &mut host);
    while !pending(app.session().unwrap()) {
        app.key(use_key, host.0.len() % 2 == 1, &mut host);
        tic(&mut app, &mut host);
    }
    app.key(use_key, false, &mut host);
    if always_run {
        app.key(keys::KEY_RSHIFT, false, &mut host);
    }
    app.key(forward, true, &mut host);
    (0..2).for_each(|_| tic(&mut app, &mut host));
    app.key(forward, true, &mut host); // the keyboard's repeat
    tic(&mut app, &mut host);
    host.0.split_off(host.0.len() - 3)
}

const EXIT: fn(&Session) -> bool = |s| s.gameaction() == 8;
const REBORN: fn(&Session) -> bool = |s| s.level().reborn_requested();

#[test]
fn forward_held_across_an_exit_load_is_dropped_until_a_repeat_arrives() {
    let got = held_over_load("CHAIN", Preset::Vanilla, 125, EXIT);
    assert_eq!(
        got,
        [(true, 25, false), (false, 0, false), (false, 25, false)]
    );
}

#[test]
fn forward_held_across_a_reborn_load_is_dropped_until_a_repeat_arrives() {
    let got = held_over_load("REBHELD", Preset::Vanilla, 22, REBORN);
    assert_eq!(
        got,
        [(true, 25, false), (false, 0, false), (false, 25, false)]
    );
}

/// Always-run: the load tic's log has the run key's keydown from the physical release, the tic
/// after the load has it again from the layer, and the repeat of the forward key moves at the
/// run speed (50, g_game.c:175, 258).
#[test]
fn always_run_survives_an_exit_load_and_the_log_holds_the_run_key() {
    let got = held_over_load("CHAIN", Preset::Modern, 125, EXIT);
    assert_eq!(
        got,
        [(true, 50, true), (false, 0, true), (false, 50, false)]
    );
}

#[test]
fn always_run_survives_a_reborn_load_and_the_log_holds_the_run_key() {
    let got = held_over_load("REBHELD", Preset::Modern, 22, REBORN);
    assert_eq!(
        got,
        [(true, 50, true), (false, 0, true), (false, 50, false)]
    );
}
