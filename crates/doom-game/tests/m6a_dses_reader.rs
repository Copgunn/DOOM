//! `DSES` version 1's reader after the M6a reader review: what decode and restore refuse of a
//! state no session produces (items 1, 2, 7 and 8), which `secretexit` the state holds (item 4),
//! and the proof that the tightened rules refuse nothing a session saves.
//!
//! - **Every saved state is still read** (`every_saved_state_decodes_and_restores`): every
//!   committed M6a item the session runs, saved before each tic and after the last, encoded,
//!   decoded, restored against its level and encoded again to the same bytes. Rust against Rust:
//!   it proves the rules admit what the session produces, not that the session is right (the
//!   genuine comparison is `m6a_session.rs` and `m6a_state.rs`).
//!   `real_maps_saved_states_decode_and_restore_local` is the same on the local real-map sets
//!   (`M6A_REFERENCE_DIR`, never in CI), decoding every state and restoring every 35th.
//! - **Forgeries**: each differs from a saved state in the one thing named, and is refused with
//!   the rule's name. Where the review found the state accepted, the test says so.

mod m6a;

use doom_game::session::demo::SessionDemo;
use doom_game::session::state::{DsesError, SessionState};
use doom_game::session::{Session, SessionData, SessionMode, SessionSettings};
use doom_render::GameState;
use doom_sim::{Simulation, TicCommand};
use std::sync::Arc;

fn commands(item: &m6a::Item) -> Option<Vec<TicCommand>> {
    let bytes = std::fs::read(m6a::root(&item.lmp)).expect("demo");
    SessionDemo::parse(&bytes).ok().map(|d| d.commands)
}

fn new_session(item: &m6a::Item, data: Arc<SessionData>) -> Option<Session> {
    let settings = SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: 10,
    };
    Session::new(data, settings).ok()
}

/// Saves `s`, requires the bytes to decode to the same encoding and, with `restore`, to restore
/// against the session's own level to a session that saves the same bytes. Returns whether the
/// session could be saved (one ended by a refusal cannot).
fn saved_state_is_read(s: &Session, restore: bool, what: &str) -> bool {
    let Ok(state) = s.save() else { return false };
    let bytes = state.encode();
    let back = SessionState::decode(&bytes).unwrap_or_else(|e| panic!("{what}: decode: {e:?}"));
    assert_eq!(back.encode(), bytes, "{what}: round trip");
    if restore {
        let r = Session::restore(s.data().clone(), &back, s.level().clone())
            .unwrap_or_else(|e| panic!("{what}: restore: {e:?}"));
        assert_eq!(r.save().expect("save").encode(), bytes, "{what}: restored");
    }
    true
}

/// Runs `item` and checks the saved state every `decode_every` tics; every `restore_every`-th of
/// those is also restored. Returns (tics run, states checked).
fn drive(item: &m6a::Item, restore_every: usize) -> Option<(usize, usize)> {
    let cmds = commands(item)?;
    let mut s = new_session(item, m6a::data(item, None))?;
    let mut states = 0;
    let mut tics = 0;
    for (t, &c) in cmds.iter().enumerate() {
        let what = format!("{} before tic {t}", item.name);
        states += usize::from(saved_state_is_read(&s, t % restore_every == 0, &what));
        if s.tic(c).is_err() {
            break;
        }
        tics += 1;
    }
    let what = format!("{} after {tics} tics", item.name);
    states += usize::from(saved_state_is_read(&s, true, &what));
    Some((tics, states))
}

#[test]
fn every_saved_state_decodes_and_restores() {
    let (mut items, mut tics, mut states) = (0, 0, 0);
    let mut skipped = Vec::new();
    for item in m6a::items() {
        match drive(&item, 1) {
            Some((t, n)) => {
                items += 1;
                tics += t;
                states += n;
            }
            None => skipped.push(item.name.clone()),
        }
    }
    eprintln!(
        "DSES every tic: {items} items, {tics} tics, {states} states decoded and restored; \
         not started (a refused start or demo): {skipped:?}"
    );
    assert!(
        items >= 20 && states > tics,
        "{items} items, {states} states"
    );
}

#[test]
#[ignore = "local only: needs M6A_REFERENCE_DIR from tools/local/m6a_session_real.py"]
fn real_maps_saved_states_decode_and_restore_local() {
    if !m6a::real_maps() {
        eprintln!("skip: M6A_REFERENCE_DIR is not set");
        return;
    }
    let (mut tics, mut states) = (0, 0);
    let items = m6a::real_items();
    for item in &items {
        let (t, n) = drive(item, 35).unwrap_or_else(|| panic!("{}: not started", item.name));
        eprintln!("{}: {t} tics, {n} states decoded", item.name);
        tics += t;
        states += n;
    }
    eprintln!(
        "real maps, {} runs: {tics} tics, {states} states decoded, every 35th restored",
        items.len()
    );
}

struct Points {
    data: Arc<SessionData>,
    /// INTFAST in its first level (30 tics).
    level: (SessionState, Simulation),
    /// INTIDLE in the tally, a counter counting.
    tally: (SessionState, Simulation),
    /// INTFAST after its intermission's last tic, `ga_worlddone` pending.
    done: (SessionState, Simulation),
}

/// The states of `name` that `pick` names, by label, with their levels, and the item's data.
fn states_of(
    name: &str,
    pick: impl Fn(usize, &Session) -> Option<&'static str>,
) -> (
    Arc<SessionData>,
    Vec<(&'static str, SessionState, Simulation)>,
) {
    let item = m6a::item(name);
    let data = m6a::data(&item, Some(SessionMode::Registered));
    let mut s = new_session(&item, data.clone()).expect("session");
    let mut out: Vec<(&'static str, SessionState, Simulation)> = Vec::new();
    for (t, &c) in commands(&item).expect("demo").iter().enumerate() {
        s.tic(c).expect("tic");
        if let Some(label) = pick(t + 1, &s) {
            if !out.iter().any(|(l, _, _)| *l == label) {
                out.push((label, s.save().expect("save"), s.level().clone()));
            }
        }
    }
    (data, out)
}

/// States of INTFAST (its level, its intermission's end) and INTIDLE (its tally) with their
/// levels; both items play the same WAD set, so one session data restores all three.
fn points() -> Points {
    let (data, fast) = states_of("INTFAST", |tics, s| {
        if tics == 30 {
            Some("level")
        } else {
            (s.gameaction() == 8).then_some("done")
        }
    });
    let (_, idle) = states_of("INTIDLE", |_, s| {
        let wi = s.intermission();
        (s.gamestate() == GameState::Intermission && wi.state == 0 && wi.sp_state == 8)
            .then_some("tally")
    });
    let take = |v: &[(&'static str, SessionState, Simulation)], label: &str| {
        let (_, state, level) = v.iter().find(|(l, _, _)| *l == label).expect(label);
        (state.clone(), level.clone())
    };
    let p = Points {
        data,
        level: take(&fast, "level"),
        tally: take(&idle, "tally"),
        done: take(&fast, "done"),
    };
    assert_eq!(p.level.0.gamestate, GameState::Level);
    assert_eq!(p.tally.0.gameaction, 0);
    assert_eq!((p.done.0.wi.state, p.done.0.wi.cnt), (-1, 0));
    p
}

/// The rule that refuses `s` when decoded from its own bytes.
fn rule(s: &SessionState) -> &'static str {
    match SessionState::decode(&s.encode()) {
        Err(DsesError::Inconsistent(rule) | DsesError::Value(rule)) => rule,
        other => panic!("not refused by a rule: {:?}", other.map(|_| "decoded")),
    }
}

fn restore_rule(p: &Points, s: &SessionState, level: &Simulation) -> &'static str {
    let s = SessionState::decode(&s.encode()).expect("decodes");
    match Session::restore(p.data.clone(), &s, level.clone()) {
        Err(DsesError::Inconsistent(rule)) => rule,
        other => panic!("not refused by a rule: {:?}", other.map(|_| "restored")),
    }
}

/// Items 1 and 2: values from the file that the code added to before checking them. Each was a
/// debug-build overflow panic (a wrapped value in a release build); each is now a refusal, and
/// the arithmetic itself no longer overflows.
#[test]
fn file_values_are_checked_before_arithmetic() {
    let p = points();
    let (tally, _) = &p.tally;
    // Item 1: `wminfo.last + 1` on the file's value.
    let mut s = tally.clone();
    s.wminfo.last = i32::MAX;
    assert_eq!(rule(&s), "wminfo map");
    // Item 2: `bcnt + ...` with a `bcnt` near the top of `int` and an animation due.
    let mut s = tally.clone();
    s.wi.bcnt = i32::MAX;
    s.wi.anims[0].nexttic = i32::MAX;
    assert_eq!(rule(&s), "anim nexttic");
    let mut s = tally.clone();
    s.wi.bcnt += 1;
    s.wi.anims.iter_mut().for_each(|a| a.nexttic += 1);
    assert_eq!(rule(&s), "bcnt and the intermission's tics");
    // Any `nexttic` was read: one behind `bcnt` never comes due.
    let mut s = tally.clone();
    s.wi.anims[3].nexttic = s.wi.bcnt;
    assert_eq!(rule(&s), "anim nexttic");
    // The tally's multiplication by 100 and its counters.
    let mut s = tally.clone();
    s.wminfo.plyr.skills = i32::MAX / 100 + 1;
    assert_eq!(rule(&s), "wminfo tally");
    let mut s = tally.clone();
    s.wi.cnt_kills = i32::MAX;
    assert_eq!(rule(&s), "tally counter");
    let mut s = tally.clone();
    s.wi.cnt_time = i32::MAX - 1;
    assert_eq!(rule(&s), "time counters");
}

/// Item 7: `gameaction` 8 during the tally, carried view heights, `wminfo` counts and a carried
/// player that are not the finished level's.
#[test]
fn the_intermission_state_is_the_finished_levels() {
    let p = points();
    let (tally, frozen) = &p.tally;
    let (done, done_level) = &p.done;
    assert!(Session::restore(p.data.clone(), tally, frozen.clone()).is_ok());
    assert!(Session::restore(p.data.clone(), done, done_level.clone()).is_ok());
    // `ga_worlddone` is set by the tic that ends the intermission, and only by it.
    let mut s = tally.clone();
    s.gameaction = 8;
    assert_eq!(rule(&s), "gameaction 8 and the intermission's end");
    let mut s = done.clone();
    s.gameaction = 0;
    assert_eq!(rule(&s), "gameaction 8 and the intermission's end");
    // A view height above VIEWHEIGHT (41 units) is no player's.
    let mut s = tally.clone();
    s.carried.as_mut().expect("carried").viewheight = (41 << 16) + 1;
    assert_eq!(rule(&s), "carried viewheight");
    // Counts that tally to the same percentages decode; the finished level has others.
    let mut s = tally.clone();
    s.wminfo.maxkills *= 2;
    s.wminfo.plyr.skills *= 2;
    assert!(SessionState::decode(&s.encode()).is_ok());
    assert_eq!(
        restore_rule(&p, &s, frozen),
        "wminfo and the finished level's counts"
    );
    let mut s = tally.clone();
    s.wminfo.plyr.stime += 1;
    assert_eq!(
        restore_rule(&p, &s, frozen),
        "wminfo and the finished level's counts"
    );
    // A carried inventory that is valid alone and not the finished player's.
    let mut s = tally.clone();
    let start = s
        .carried
        .as_mut()
        .and_then(|c| c.start.as_mut())
        .expect("alive");
    assert!(start.ammo[0] > 0);
    start.ammo[0] -= 1;
    assert_eq!(
        restore_rule(&p, &s, frozen),
        "carried player and the finished level's"
    );
    // A carried health that is valid alone: the status bar's last health already contradicts it.
    let mut s = tally.clone();
    let start = s
        .carried
        .as_mut()
        .and_then(|c| c.start.as_mut())
        .expect("alive");
    start.health -= 1;
    assert_eq!(rule(&s), "carried health and st_oldhealth");
}

/// Item 8: the embedded level capture is tied to the outer state (map, tic, start `gametic`),
/// and the display's entry tic to the sound's level tics.
#[test]
fn the_embedded_states_are_tied_to_the_session() {
    let p = points();
    let (level, sim) = &p.level;
    let (tally, _) = &p.tally;
    assert!(Session::restore(p.data.clone(), level, sim.clone()).is_ok());
    // The capture of the same level one tic later.
    let mut later = sim.clone();
    let none = TicCommand::default();
    later.advance([none; 4]).expect("tic");
    let mut s = level.clone();
    s.level = later.snapshot().canonical_bytes();
    assert_eq!(rule(&s), "the level capture's tic");
    // The same lumps loaded as another map number, at the same tic.
    let item = m6a::item("INTFAST");
    let wad_bytes = std::fs::read(m6a::root(&item.wads[0])).expect("WAD");
    let wad = doom_wad::Wad::parse(&wad_bytes, doom_wad::Limits::default()).expect("wad");
    let lump = format!("E1M{}", item.map);
    let map = doom_map::Map::from_wad(&wad, &lump, doom_map::MapLimits::default()).expect("map");
    let other = if item.map == 1 { 2 } else { item.map - 1 };
    let mut renumbered = Simulation::original_player_episode1_v4_level(
        map,
        other,
        doom_sim::Skill::Medium,
        doom_sim::MapContextOptions::default(),
        doom_sim::LevelStart::NewGame,
    )
    .expect("level");
    for &c in &commands(&item).expect("demo")[..30] {
        renumbered.advance([c, none, none, none]).expect("tic");
    }
    let mut s = level.clone();
    s.level = renumbered.snapshot().canonical_bytes();
    assert_eq!(rule(&s), "the level capture's map");
    // The display's `entered` against the sound's level end (intermission) and start (level).
    let mut s = tally.clone();
    s.display.entered -= 1;
    assert_eq!(rule(&s), "display entered and the sound's level tics");
    // The display's own rules hold for a decoded state: a wipe still due after a whole tic.
    let mut s = tally.clone();
    s.display.wipe = true;
    assert!(SessionState::decode(&s.encode()).is_err());
    let mut s = tally.clone();
    s.display.status.oldhealth = 201;
    assert!(matches!(
        SessionState::decode(&s.encode()),
        Err(DsesError::DisplayRestore(_))
    ));
}

/// Item 4: the saved `secretexit` byte is the flag as the last completed level left it, and
/// `Session::secretexit` reports the C's `secretexit`, which the exit tic has already rewritten
/// (g_game.c:1000-1012). On a tic with a secret exit pending the two differ; the pending exit is
/// in the level capture, and a restored session reports and completes it as the original does.
#[test]
fn the_saved_secretexit_is_the_last_completed_exits() {
    let item = m6a::item("SECRET");
    let data = m6a::data(&item, None);
    let mut s = new_session(&item, data.clone()).expect("session");
    let cmds = commands(&item).expect("demo");
    let mut seen = false;
    for (t, &c) in cmds.iter().enumerate() {
        s.tic(c).unwrap_or_else(|e| panic!("tic {t}: {e}"));
        if s.gameaction() == 6 && s.secretexit() && !seen {
            seen = true;
            let state = s.save().expect("save");
            assert!(
                !state.secretexit,
                "the saved flag is the last completed exit's"
            );
            let bytes = state.encode();
            let back = SessionState::decode(&bytes).expect("decode");
            let mut r = Session::restore(data.clone(), &back, s.level().clone()).expect("restore");
            assert!(
                r.secretexit(),
                "the restored session reports the pending exit"
            );
            let next = cmds[t + 1];
            r.tic(next).expect("completion");
            let mut whole = s.clone();
            whole.tic(next).expect("completion");
            let (a, b) = (r.save().expect("save"), whole.save().expect("save"));
            assert!(
                a.secretexit && a.wminfo.next == 8,
                "completed as a secret exit"
            );
            assert_eq!(a.encode(), b.encode());
        }
    }
    assert!(seen, "SECRET takes a secret exit");
}

/// Seeds for the four M6a fuzz targets (`fuzz/fuzz_targets/m6a_*.rs`): the three states of
/// [`points`] and their embedded states, written under the directory `M6A_FUZZ_SEEDS` names
/// (outside the repository), one folder per target. Local only.
#[test]
#[ignore = "local only: writes fuzz seeds under M6A_FUZZ_SEEDS"]
fn write_fuzz_seeds_local() {
    let Some(dir) = std::env::var_os("M6A_FUZZ_SEEDS") else {
        return;
    };
    let p = points();
    for (name, (s, _)) in [("level", &p.level), ("tally", &p.tally), ("done", &p.done)] {
        for (target, bytes) in [
            ("m6a_dses", s.encode()),
            ("m6a_ddsp3", s.display.encode()),
            ("m6a_dsnd2", s.sound.encode()),
            ("m6a_dsim42", s.level.clone()),
        ] {
            let folder = std::path::Path::new(&dir).join(target);
            std::fs::create_dir_all(&folder).expect("folder");
            std::fs::write(folder.join(name), bytes).expect("seed");
        }
    }
}
