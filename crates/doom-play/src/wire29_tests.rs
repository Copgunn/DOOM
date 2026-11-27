//! Wire 29 in doom-play (M6b rulings N1, N4, N5, N7): the `--skill 4`, `--respawn` and `--fast`
//! flags, the help's account of them and of the two pauses, and sessions through the app on
//! doom-play's loader: the Pause key's command recorded as the original builds it (g_game.c:426-430)
//! and replayed by a fresh session to the same state, Esc's pause outside the demo, a restart as a
//! new game, and a missing `M_PAUSE` refused at the first paused tic (decision D24 of step 7).
//! Rust-only checks; the genuine engine plays such recordings in `tools/local/wire29_play_check.py`.

use crate::melt_tests::at;
use crate::{HELP, parse};
use doom_game::app::{App, Ending, GameData, GameParameters, Host, Mode, SessionConfig};
use doom_game::app::{PAUSE_COMMAND, Timestamp, Witness};
use doom_game::input::keys::{KEY_ESCAPE, KEY_PAUSE, KEY_UPARROW};
use doom_game::session::demo::{SessionDemo, replay_finale_and_melt};
use doom_game::session::{ModeSetting, Session, SessionSettings, SessionWitness};
use std::path::Path;
use std::sync::Arc;

fn args(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

const BOTH: GameParameters = GameParameters {
    respawn: true,
    fast: true,
};

#[test]
fn skill_4_and_the_respawn_and_fast_flags() {
    for (given, want) in [("4", 4), ("nightmare", 4), ("0", 0), ("uv", 3)] {
        let o = parse(args(&format!("--iwad a.wad --skill {given}")))
            .unwrap()
            .unwrap();
        assert_eq!((o.skill, o.parameters), (want, GameParameters::default()));
    }
    for (flags, respawn, fast) in [
        ("--respawn", true, false),
        ("--fast", false, true),
        ("--fast --respawn --skill 2", true, true),
    ] {
        let o = parse(args(&format!("--iwad a.wad {flags}")))
            .unwrap()
            .unwrap();
        assert_eq!(o.parameters, GameParameters { respawn, fast }, "{flags}");
    }
    for bad in ["--skill 5", "--skill nm", "--respawn 1", "--fast=1"] {
        assert!(
            parse(args(&format!("--iwad a.wad {bad}"))).is_err(),
            "{bad}"
        );
    }
    for want in [
        "4 or nightmare: Nightmare!",
        "--respawn            the original's -respawn",
        "--fast               the original's -fast",
        "Keys: Pause is the original's pause",
        "both presses are recorded in the demo",
        "Pause does nothing\nthere",
        "skills 0 to 4 with --respawn and --fast",
    ] {
        assert!(HELP.contains(want), "{want}");
    }
    assert!(
        !HELP.contains("Esc or Pause")
            && !HELP.to_lowercase().contains("nightmare (skill 4), which")
    );
}

#[derive(Default)]
struct FileHost {
    saved: Vec<(String, Vec<u8>)>,
}

impl Host for FileHost {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, _: &str) -> bool {
        false
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.saved.push((name.to_owned(), bytes.to_vec()));
        Ok(())
    }
}

/// The synthetic episode's WADs, with M6b's own `M_PAUSE` (`m6b_gfx.wad`) or without it.
fn data(with_pause_picture: bool) -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
    let mut names = vec![
        "m6a_wads/episode.wad",
        "m5b_gfx.wad",
        "m5c_sfx.wad",
        "m6a_wads/m6a_gfx.wad",
        "m6a_wads/m6a_music.wad",
    ];
    if with_pause_picture {
        names.push("m6b_wads/m6b_gfx.wad");
    }
    let files: Vec<Vec<u8>> = names
        .iter()
        .map(|w| std::fs::read(dir.join(w)).expect("WAD"))
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    GameData::load_wire29_session(&refs, ModeSetting::Probe, 1, 4, BOTH).expect("skill 4")
}

/// Runs slots until the session has run `tics` tics.
fn run_to(app: &mut App, host: &mut FileHost, k: &mut u64, tics: u64) {
    while app.tic() < tics && app.mode() == &Mode::Playing {
        *k += 1;
        assert!(*k < 10_000, "{tics} tics not reached");
        app.update(at(*k), host);
    }
}

/// The Pause key (ruling N7): its command replaces the buttons and keeps the movement of the
/// command built with the up arrow held (g_game.c:426-430), toggles the session's `paused`
/// (g_game.c:698-708) while `gametic` runs on; Esc pauses the app on top of it, the Pause key
/// does nothing while the app is paused, Esc resumes, Pause unpauses. The demo holds exactly the
/// two pause commands, skill 4 and both parameter bytes (g_game.c:1556, 1560-1561), reads back by
/// the wire 29 reader alone, and replays through a fresh wire 29 session to the same saved state
/// (`DSES` 4). A restart is a fresh session of the same skill and parameters (ruling N4).
#[test]
fn the_pause_key_records_the_original_command_and_replays() {
    let mut host = FileHost::default();
    let mut app = App::new(data(true), SessionConfig::default(), &mut host);
    let mut k = 0;
    app.key(KEY_UPARROW, true, &mut host);
    run_to(&mut app, &mut host, &mut k, 40);
    let t0 = app.tic();
    app.key(KEY_PAUSE, true, &mut host);
    app.key(KEY_PAUSE, false, &mut host);
    run_to(&mut app, &mut host, &mut k, t0 + 1);
    let s = app.session().expect("session");
    let leveltime = s.level().tic();
    let first = app.recorder().commands()[t0 as usize];
    assert_eq!(
        (first.buttons, first.forward),
        (PAUSE_COMMAND, 25),
        "buttons replaced, movement kept"
    );
    assert!(s.paused());
    run_to(&mut app, &mut host, &mut k, t0 + 20);
    let s = app.session().expect("session");
    assert_eq!(
        (s.paused(), s.level().tic()),
        (true, leveltime),
        "the level stands still"
    );
    app.key(KEY_ESCAPE, true, &mut host);
    assert_eq!(
        app.mode(),
        &Mode::Paused,
        "Esc stops the app on top of the original's pause"
    );
    app.key(KEY_PAUSE, true, &mut host);
    assert_eq!(
        app.mode(),
        &Mode::Paused,
        "the Pause key does nothing while the app is stopped"
    );
    app.key(KEY_ESCAPE, true, &mut host);
    assert_eq!(app.mode(), &Mode::Playing);
    run_to(&mut app, &mut host, &mut k, t0 + 30);
    assert!(
        app.session().unwrap().paused(),
        "no command was sent by the stopped app's Pause"
    );
    let t1 = app.tic();
    app.key(KEY_PAUSE, true, &mut host);
    run_to(&mut app, &mut host, &mut k, t1 + 20);
    let s = app.session().expect("session");
    assert!(!s.paused() && s.level().tic() > leveltime);
    let pauses: Vec<u64> = (app.recorder().commands().iter().enumerate())
        .filter_map(|(t, c)| (c.buttons == PAUSE_COMMAND).then_some(t as u64))
        .collect();
    assert_eq!(pauses, [t0, t1]);
    app.quit(&mut host);
    let (name, bytes) = host.saved.last().expect("a demo file").clone();
    assert!(name.starts_with("E1M1-skill4-"), "{name}");
    assert_eq!(
        bytes[..7],
        [110, 4, 1, 1, 0, 1, 1],
        "version, skill, episode, map, the parameters"
    );
    let (demo, parameters) = SessionDemo::parse_v4(&bytes).expect("the wire 29 reader");
    assert_eq!((demo.map, demo.skill, parameters), (1, 4, BOTH));
    assert_eq!(demo.commands, app.recorder().commands());
    assert_eq!(demo.bytes_v4(parameters).expect("written"), bytes);
    assert!(
        SessionDemo::parse_v3(&bytes).is_err() && demo.bytes_v3().is_err(),
        "the earlier ones"
    );
    let played = app.session().expect("session");
    let session_data = Arc::clone(played.data());
    let settings = SessionSettings {
        map: 1,
        skill: 4,
        screenblocks: 10,
    };
    let fresh = |p| Session::new_finale_and_melt_wire29(Arc::clone(&session_data), settings, p);
    let mut replayed = fresh(parameters).expect("a fresh session");
    let end = replay_finale_and_melt(&mut replayed, &demo.commands, |_, _| {}, |_, _, _| {});
    assert_eq!((end, replayed.gametic()), (None, played.gametic()));
    let state = |s: &Session| s.save_v4().expect("saveable").encode();
    assert_eq!(state(&replayed), state(played));
    // a restart: a new game of the same skill and parameters, unpaused, its own demo
    app.restart(&mut host);
    let s = app.session().expect("session");
    assert_eq!((s.gametic(), s.paused(), s.parameters()), (0, false, BOTH));
    assert_eq!(state(s), state(&fresh(BOTH).expect("a new game")));
    assert!(app.recorder().is_empty());
}

/// Without `M_PAUSE` the first paused tic is refused with the state kept (decision D24 of step 7;
/// the genuine engine stops in `W_CacheLumpName` on its first paused frame, d_main.c:307): the
/// app ends there and the refused tic's pause command is not in the demo.
#[test]
fn a_missing_pause_picture_refuses_the_first_paused_tic() {
    let mut host = FileHost::default();
    let mut app = App::new(data(false), SessionConfig::default(), &mut host);
    let mut k = 0;
    run_to(&mut app, &mut host, &mut k, 20);
    let t0 = app.tic();
    app.key(KEY_PAUSE, true, &mut host);
    run_to(&mut app, &mut host, &mut k, 30);
    let Mode::Ended(Ending::Refused { tic, witness }) = app.mode().clone() else {
        panic!("not refused: {:?}", app.mode());
    };
    assert_eq!(tic, t0);
    let Witness::Session(w) = witness else {
        panic!("{witness:?}")
    };
    assert!(
        matches!(
            *w,
            SessionWitness::Display(doom_render::SessionError::PausePicture(None))
        ),
        "{w:?}"
    );
    assert_eq!(
        app.recorder().len() as u64,
        t0,
        "the refused tic is not recorded"
    );
    assert!(!app.session().unwrap().paused());
}
