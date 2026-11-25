//! M6a part 2, mutation survivors of the CI run on 9e3734ae: each test names the mutants it kills.
//! Rust-only rules already cited in the source (the doc comments of `state_v2.rs`, the module
//! documentation of `app/finale_melt.rs`); no expectation comes from Rust output.
#![allow(clippy::unwrap_used, reason = "test code")]
mod m6a;

use doom_game::app::{App, AppFault, FinaleMeltSessionTicReport, GameData, Host, MeltStepShown};
use doom_game::app::{SessionConfig, Timestamp, Update};
use doom_game::input::{Event, NUMKEYS, keys};
use doom_game::session::demo::SessionDemo;
use doom_game::session::finale::FinaleRefusal;
use doom_game::session::intermission::Intermission;
use doom_game::session::state::{DsesError, MAX_BYTES};
use doom_game::session::state_v2::SessionStateV2;
use doom_game::session::{ModeSetting, Session, SessionSettings};
use doom_render::GameState;
use doom_sim::TicCommand;
use std::collections::BTreeMap;

#[derive(Default)]
struct H {
    files: BTreeMap<String, Vec<u8>>,
    tics: Vec<(Vec<Event>, TicCommand)>,
    steps: usize,
}
impl Host for H {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp {
            year: 2026,
            month: 11,
            day: 24,
            hour: 12,
            minute: 0,
            second: 0,
        }
    }
    fn exists(&mut self, n: &str) -> bool {
        self.files.contains_key(n)
    }
    fn save(&mut self, n: &str, b: &[u8]) -> Result<(), String> {
        self.files.insert(n.into(), b.to_vec());
        Ok(())
    }
    fn finale_melt_tic(&mut self, r: &FinaleMeltSessionTicReport<'_>) {
        self.tics.push((r.events.to_vec(), r.executed));
    }
    fn melt_step(&mut self, _: &MeltStepShown<'_>) {
        self.steps += 1;
    }
}

/// An app on CHAIN, one update per tic of wall time (35 Hz as microseconds, rounded up).
struct D {
    app: App,
    h: H,
    k: u64,
    base: u64,
}
impl D {
    fn new(fault: Option<AppFault>, tic_cap: usize) -> Self {
        let item = m6a::item("CHAIN");
        let files: Vec<Vec<u8>> = item
            .wads
            .iter()
            .map(|w| std::fs::read(m6a::root(w)).unwrap())
            .collect();
        let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
        let data =
            GameData::load_finale_and_melt_session(&refs, ModeSetting::Probe, item.map, item.skill)
                .unwrap();
        let cfg = SessionConfig {
            screenblocks: 10,
            tic_cap,
            ..SessionConfig::default()
        };
        let mut h = H::default();
        let mut app = App::new(data, cfg, &mut h);
        assert_eq!(app.update(0, &mut h).ran, 0);
        app.plant_fault(fault);
        Self {
            app,
            h,
            k: 0,
            base: 0,
        }
    }
    fn update(&mut self) -> Update {
        self.k += 1;
        self.app
            .update(self.base + (self.k * 1_000_000).div_ceil(35), &mut self.h)
    }
    /// Updates until the melt in progress ends (at most 200).
    fn finish_melt(&mut self) {
        for _ in 0..200 {
            if !self.app.melt_in_progress() {
                return;
            }
            self.update();
        }
        panic!("the melt did not end in 200 updates");
    }
}

/// Kills `finale_melt.rs:143:9` (`AppMelt::fresh` -> `Default::default()`) and `:144:13` (delete
/// field `fault`): a restart (Esc, R) keeps the planted fault ("keeping a planted fault"), seen as
/// `DropKeyInMelt` still dropping the key pressed in the restarted session's first melt.
#[test]
fn a_restart_keeps_the_planted_fault() {
    let mut d = D::new(Some(AppFault::DropKeyInMelt), 100_000);
    d.update();
    d.finish_melt();
    d.app.key(keys::KEY_ESCAPE, true, &mut d.h);
    d.app.key(b'r', true, &mut d.h);
    d.base += (d.k * 1_000_000).div_ceil(35) + 1;
    d.k = 0;
    assert_eq!(d.app.update(d.base, &mut d.h).ran, 0);
    assert_eq!(d.update().ran, 1);
    assert!(d.app.melt_in_progress());
    d.app.key(keys::KEY_UPARROW, true, &mut d.h);
    d.finish_melt();
    d.update();
    assert_eq!(d.h.tics.last().unwrap().1.forward, 0, "key dropped");
}

/// Kills `finale_melt.rs:276:87` (`%` -> `/`), `:276:91` (`==` -> `!=`), `:279:23` (`+=` -> `*=`
/// and `-=`): `SkipMeltStep` ("every second melt step runs in the same slot as the step before
/// it") runs a second step after an odd one: tic 0's update shows steps 1 and 2 and reports both.
#[test]
fn the_skip_fault_runs_the_second_step_of_an_odd_one_and_counts_it() {
    let mut d = D::new(Some(AppFault::SkipMeltStep), 100_000);
    let u = d.update();
    assert_eq!((u.ran, u.melt_steps, d.h.steps), (1, 2, 2));
}

/// Kills `finale_melt.rs:356:9` (`App::melt_step` -> `()`): the melt's steps are shown to the host
/// and the melt ends (bounded here; the C-derived 33-step minimum is in `m6a2_play.rs`).
///
/// Also `finale_melt.rs:260:19` (`slots += 1` -> `*=`, a TIMEOUT in CI): with `slots` stuck at 0
/// the `while` of `update_finale_and_melt` never reaches its bound and runs tics until the session
/// ends (here the tic cap of 50), so one slot due ran many tics instead of exactly one.
#[test]
fn a_melt_step_is_shown_and_one_slot_runs_one_tic() {
    let mut d = D::new(None, 50);
    d.update();
    assert_eq!(d.h.steps, 1);
    d.finish_melt();
    assert!(d.h.steps >= 33);
    let u = d.update();
    assert_eq!((u.ran, u.melt_steps, u.dropped), (1, 0, 0));
}

/// Kills `finale_melt.rs:53:39` (`4 * NUMKEYS` -> `4 + NUMKEYS`) and `:409:34` (`==` -> `!=` in
/// `release_held`): up to 4 x `NUMKEYS` inputs are held in a melt and one more forgets them and
/// holds `ReleaseAll`, which releases a key held down before the melt when the melt ends
/// (module documentation; `EventLayer::release_all`).
#[test]
fn the_held_bound_and_the_release_all_at_the_melts_end() {
    let mut d = D::new(None, 100_000);
    d.app.key(keys::KEY_UPARROW, true, &mut d.h);
    d.update();
    assert!(d.app.melt_in_progress());
    for i in 0..4 * NUMKEYS {
        d.app.key(b'x', i % 2 == 0, &mut d.h);
    }
    assert_eq!(d.app.held_inputs(), 4 * NUMKEYS);
    d.app.key(b'x', true, &mut d.h);
    assert_eq!(d.app.held_inputs(), 2, "ReleaseAll and the new input");
    d.finish_melt();
    d.update();
    let (events, _) = d.h.tics.last().unwrap();
    assert!(
        events.contains(&Event::KeyUp(keys::KEY_UPARROW)),
        "{events:?}"
    );
}

/// Kills `finale.rs:46:9` (`Display for FinaleRefusal` -> `Ok(Default::default())`).
#[test]
fn a_finale_refusal_displays() {
    assert_eq!(
        FinaleRefusal::Count.to_string(),
        "finale tic refused: Count"
    );
}

/// Kills `state_v2.rs:228:24` (`>` -> `==` and `>=`): a state of exactly `MAX_BYTES` is read (and
/// refused for its magic), one byte more is `TooLarge`.
#[test]
fn the_reader_size_bound_is_exact() {
    let ok = SessionStateV2::decode(&vec![0; MAX_BYTES]);
    assert!(matches!(ok, Err(DsesError::Magic)), "{ok:?}");
    let big = SessionStateV2::decode(&vec![0; MAX_BYTES + 1]);
    assert!(matches!(big, Err(DsesError::TooLarge)), "{big:?}");
}

/// A CHAIN session on its demo until `pred` holds with no melt in progress.
fn session_where(pred: impl Fn(&Session, &SessionStateV2) -> bool) -> (Session, SessionStateV2) {
    let item = m6a::item("CHAIN");
    let demo = SessionDemo::parse(&std::fs::read(m6a::root(&item.lmp)).unwrap()).unwrap();
    let settings = SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: 10,
    };
    let mut s = Session::new_finale_and_melt(m6a::data(&item, None), settings).unwrap();
    for c in demo.commands {
        while s.melt_in_progress() {
            s.melt_step(&mut doom_render::Frame::new()).unwrap();
        }
        let st = s.save_v2().unwrap();
        if pred(&s, &st) {
            return (s, st);
        }
        s.tic_finale_and_melt(c).unwrap();
    }
    panic!("no such state");
}

type Forge = fn(&mut SessionStateV2);

fn refused(st: &SessionStateV2, name: &str) {
    let r = st.check();
    assert!(
        matches!(&r, Err(DsesError::Inconsistent(n)) if *n == name),
        "want {name}: {r:?}"
    );
}

/// Kills `state_v2.rs:415:17` and `:416:17` (delete match arm 6 / 8), `:484:9`
/// (`check_outside_intermission` -> `Ok(())`), `:485:44` (`&&` -> `||` in `finished`) and `:489:22`
/// (`&&` -> `||`), by forged states each breaking exactly one rule of `check`'s doc comment:
/// gameaction 6 only in a level and 8 only in the intermission; outside the intermission a fresh
/// `wi` has a fresh `wminfo`, otherwise the intermission is finished (`NoState`, `cnt` 0) and its
/// `next` led to the current map. The genuine states are accepted.
#[test]
fn forged_states_outside_the_intermission() {
    let (_, level) = session_where(|s, _| s.gamestate() == GameState::Level && s.gametic() == 5);
    level.check().unwrap();
    let mut f = level.clone();
    f.gameaction = 8;
    refused(&f, "gameaction and gamestate");
    let mut f = level.clone();
    f.wminfo.next = 3;
    refused(&f, "the last intermission's statics");
    let (_, inter) =
        session_where(|_, st| st.gamestate == GameState::Intermission && st.wi.state == 0);
    inter.check().unwrap();
    let mut f = inter.clone();
    f.gameaction = 6;
    refused(&f, "gameaction and gamestate");
    // a level after a finished intermission
    let (_, after) = session_where(|s, st| {
        s.gamestate() == GameState::Level && s.map() == 2 && st.wi != Intermission::default()
    });
    assert_eq!((after.wi.state, after.wi.cnt), (-1, 0));
    after.check().unwrap();
    for (what, forge) in [
        (
            "cnt",
            (|s: &mut SessionStateV2| s.wi.cnt = 1) as fn(&mut SessionStateV2),
        ),
        ("state", |s| s.wi.state = 0),
        ("next", |s| s.wminfo.next += 1),
    ] {
        let mut f = after.clone();
        forge(&mut f);
        assert!(
            f.wi.check().is_ok(),
            "{what}: the forgery is within wi's own domain"
        );
        refused(&f, "the last intermission's statics");
    }
}

/// Kills `state_v2.rs:500:9` (`check_intermission` -> `Ok(())`), `:513:26` (`&&` -> `||`),
/// `:514:38` (`&&` -> `||`) and `:521:29`, `:521:48` (`&&` -> `||`): forged intermissions, each
/// breaking one of version 1's rules (doc comment of `check_intermission`): `wminfo.epsd` 0 and
/// `last` the map just finished; `last` in 1..=9 and not 8 (a map-8 intermission does not exist:
/// the victory runs the finale); each of the three maxima at least 1 (`WI_initVariables`).
#[test]
fn forged_intermissions() {
    let (_, inter) =
        session_where(|_, st| st.gamestate == GameState::Intermission && st.wi.state == 0);
    let table: [(&str, &str, Forge); 5] = [
        ("epsd", "wminfo map", |s| s.wminfo.epsd = 1),
        ("last", "wminfo map", |s| s.wminfo.last += 1),
        ("maxkills", "wminfo maxima", |s| s.wminfo.maxkills = 0),
        ("maxitems", "wminfo maxima", |s| s.wminfo.maxitems = 0),
        ("maxsecret", "wminfo maxima", |s| s.wminfo.maxsecret = 0),
    ];
    for (_, name, forge) in table {
        let mut f = inter.clone();
        forge(&mut f);
        refused(&f, name);
    }
    // A map-8 intermission: `last` 8 equals the map (so rule 1 holds); the level capture's map and
    // the sound's `gamemap` are forged to 8 as well, so only `wminfo last` can refuse.
    let mut f = inter.clone();
    let key = b"\x09\0\0\0level.map\x01\0\0\0\0\0\0\0";
    let at = f.level.windows(key.len()).position(|w| w == key).unwrap() + key.len();
    f.level[at] = 8;
    f.map = 8;
    f.sound.state.gamemap = 8;
    f.wminfo.last = 7;
    refused(&f, "wminfo last");
}

/// Kills `state_v2.rs:646:25` and `:647:25` (`&&` -> `||` in `restore_v2`): the intermission's
/// `wminfo` must equal the finished level's maxima, counts and time (all three); a state whose
/// kill count alone differs (still within `check`'s rules) is refused when restored.
#[test]
fn restore_refuses_wminfo_counts_of_another_level() {
    let (s, mut st) =
        session_where(|_, st| st.gamestate == GameState::Intermission && st.wi.state == 0);
    let data = m6a::data(&m6a::item("CHAIN"), None);
    Session::restore_v2(data.clone(), &st, s.level().clone()).unwrap();
    st.wminfo.plyr.skills += 1;
    st.check().unwrap();
    let r = Session::restore_v2(data, &st, s.level().clone());
    assert!(
        matches!(
            &r,
            Err(DsesError::Inconsistent(
                "wminfo and the finished level's counts"
            ))
        ),
        "{:?}",
        r.err()
    );
}
