//! Wire 29 restore and continue (M6b phase 2 step 9, note section 6.2 item 4). A `DSES` 4 state is
//! saved and restored at every tic of four windows, and the restored session is stepped beside the
//! unbroken one on the same commands: the windows are a paused stretch, the tics around a corpse's
//! respawn gate, the fogs' lifetimes after it, and the load that clears a pause. Compared on every
//! tic: the report (kind, paused, sound with its mixed buffer), the frame, the music calls, the
//! level's whole `DSIM` 45 state and every melt step; and at the end of each continuation the two
//! `DSES` 4 encodings are equal. Where the windows lie comes from facts already cited: the genuine
//! engine's `R` rows (the tic a corpse's gate passed, `m6b_genuine.txt`) and its session stream's
//! `paused` column (`m6b_session/`, `wire29_session/`), never from the port's output.
//!
//! Below the public API: none. `Session::restore_v4` is given the live level (a `DSIM` is checked,
//! not read back into a simulation), so "`DSIM` 45 alone at the simulation level" is the level
//! compared here and the clone `restore_v4` takes; there is no `Simulation` constructor from bytes.
mod m6a;

use doom_game::session::demo::SessionDemo;
use doom_game::session::{Session, SessionSettings};
use doom_render::Frame;
use doom_sim::{GameParameters, TicCommand};
use m6a::{Rows, int};

const SHARED: [&str; 5] = [
    "tests/reference/m5b_gfx.wad",
    "tests/reference/m5c_sfx.wad",
    "tests/reference/m6a_wads/m6a_gfx.wad",
    "tests/reference/m6a_wads/m6a_music.wad",
    "tests/reference/m6b_wads/m6b_gfx.wad",
];

/// The item (WADs, demo, parameters) as `wire29_session.rs` builds it.
fn item(name: &str) -> (m6a::Item, GameParameters, Vec<TicCommand>) {
    let new = matches!(name, "DEADSPEC" | "LOADP" | "TURBOP");
    let episode = matches!(name, "PAUSEWI" | "LOADP");
    let first = if episode {
        "tests/reference/m6a_wads/episode.wad"
    } else {
        "tests/reference/m6b_wads/m6b.wad"
    };
    let lmp = if new {
        format!("tests/reference/wire29_session/{name}.lmp")
    } else {
        format!("tests/reference/m6b_scripts/{name}.lmp")
    };
    let bytes = std::fs::read(m6a::root(&lmp)).expect("demo");
    let (demo, parameters) = SessionDemo::parse_v4(&bytes).expect("the wire 29 demo reader");
    let item = m6a::Item {
        name: name.into(),
        mode: "registered".into(),
        wads: std::iter::once(first)
            .chain(SHARED)
            .map(String::from)
            .collect(),
        lmp,
        map: demo.map,
        skill: demo.skill,
        end: "marker".into(),
        session_tics: demo.commands.len(),
    };
    (item, parameters, demo.commands)
}

/// The genuine session stream's `paused` column at each tic.
fn genuine_paused(name: &str) -> Vec<bool> {
    let frozen = m6a::root(&format!("tests/reference/m6b_session/{name}.session"));
    let path = if frozen.exists() {
        format!("m6b_session/{name}.session")
    } else {
        format!("wire29_session/{name}.session")
    };
    let g = Rows::read(&path);
    g.of("G").iter().map(|r| int(r, "paused") == 1).collect()
}

/// Restores a session at each tic of `points` and runs it `horizon` tics beside the unbroken
/// one. Returns the numbers of restores taken at a paused and at an unpaused genuine tic.
fn windows(name: &str, points: &[usize], horizon: usize) -> (usize, usize) {
    let (item, parameters, cmds) = item(name);
    let paused = genuine_paused(name);
    let data = m6a::data(&item, None);
    let settings = SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: 10,
    };
    let mut main = Session::new_finale_and_melt_wire29(data.clone(), settings, parameters).unwrap();
    let mut active: Vec<(usize, Session)> = Vec::new();
    let (mut at_paused, mut at_running) = (0, 0);
    let mut unseen = Frame::new();
    for (t, &c) in cmds.iter().enumerate() {
        if points.contains(&t) {
            let state = main.save_v4().expect("saveable");
            let bytes = state.encode();
            let decoded = doom_game::session::state_v4::SessionStateV4::decode(&bytes).unwrap();
            assert_eq!(decoded, state, "{name} {t}");
            let r = Session::restore_v4(data.clone(), &decoded, main.level().clone())
                .unwrap_or_else(|e| panic!("{name} {t}: {e:?}"));
            assert_eq!(r.save_v4().unwrap().encode(), bytes, "{name} {t}");
            active.push((t, r));
            if paused[t] {
                at_paused += 1;
            } else {
                at_running += 1;
            }
        }
        let a = main.tic_finale_and_melt(c).expect("a tic");
        for (from, r) in &mut active {
            let b = r.tic_finale_and_melt(c).expect("a restored tic");
            let at = format!("{name}: restored at {from}, tic {t}");
            assert_eq!(
                (a.kind, a.paused, a.wiped, &a.sound),
                (b.kind, b.paused, b.wiped, &b.sound),
                "{at}"
            );
            assert_eq!(main.paused(), r.paused(), "{at}");
            assert_eq!(main.frame().pixels(), r.frame().pixels(), "{at}");
            assert_eq!(main.sound().music_calls(), r.sound().music_calls(), "{at}");
            assert_eq!(main.level().snapshot(), r.level().snapshot(), "{at}");
        }
        while main.melt_in_progress() {
            let m = main.melt_step(&mut unseen).expect("a melt step");
            for (from, r) in &mut active {
                let n = r.melt_step(&mut unseen).expect("a restored melt step");
                assert_eq!(m, n, "{name}: restored at {from}, melt after tic {t}");
            }
        }
        active.retain(|(from, r)| {
            if t + 1 < from + horizon {
                return true;
            }
            assert_eq!(
                r.save_v4().unwrap().encode(),
                main.save_v4().unwrap().encode(),
                "{name}: restored at {from}, state after tic {t}"
            );
            false
        });
    }
    (at_paused, at_running)
}

/// A paused stretch of PAUSE (the genuine `paused` column holds from tic 72 to 112): every tic
/// from just before the first pause command to just after the unpausing one.
#[test]
fn a_session_restored_at_every_tic_of_a_pause_continues_as_the_unbroken_one() {
    let points: Vec<usize> = (68..=118).collect();
    let (paused, running) = windows("PAUSE", &points, 20);
    assert!(paused >= 30, "restores at paused tics: {paused}");
    assert!(running >= 5, "restores at unpaused tics: {running}");
}

/// The corpse's gate passes at the tic of the genuine `R` row's `respawn` outcome (RESP4: skill 4;
/// RESP3R: `-respawn` at skill 3); the restore points run from three tics before it through the
/// fogs' lifetimes (both fogs and the monster are spawned in the gate's tic, order `TFOG TFOG`
/// and the type, `m6b_genuine.txt`) in steps, so a restore falls between the fogs' states too.
#[test]
fn a_session_restored_around_a_respawn_gate_and_the_fogs_continues_as_the_unbroken_one() {
    // `R,item,tic,leveltime,type,gibs,..,value,outcome,order` (the file's header names them).
    let genuine = std::fs::read_to_string(m6a::root("tests/reference/m6b_genuine.txt")).unwrap();
    for name in ["RESP4", "RESP3R"] {
        let row = genuine
            .lines()
            .map(|l| l.split(',').collect::<Vec<_>>())
            .find(|f| f[0] == "R" && f[1] == name && f[13] == "respawn")
            .expect("the genuine gate row");
        assert_eq!(row[14], "TFOG TFOG SERGEANT");
        let gate: usize = row[2].parse().unwrap();
        let mut points: Vec<usize> = (gate - 3..gate + 16).collect();
        points.extend((gate + 16..gate + 80).step_by(4));
        let (paused, running) = windows(name, &points, 25);
        assert_eq!(
            (paused, running),
            (0, points.len()),
            "{name}: no pause here"
        );
    }
}

/// The load that clears a pause: PAUSEWI pauses in the intermission and the next level loads at
/// tic 142 (the genuine `paused` column clears there), LOADP pauses on its load tic and runs
/// tic 0 at 146 (`wire29_session/LOADP.session`). Every tic around each load.
#[test]
fn a_session_restored_around_the_load_that_clears_a_pause_continues_as_the_unbroken_one() {
    let (paused, _) = windows("PAUSEWI", &(120..=150).collect::<Vec<_>>(), 20);
    assert_eq!(
        paused, 9,
        "PAUSEWI's genuine paused tics are 9 (facts of wire29_session)"
    );
    let (paused, running) = windows("LOADP", &(138..=150).collect::<Vec<_>>(), 20);
    assert!(
        paused >= 3 && running >= 3,
        "LOADP: {paused} paused, {running} running"
    );
}
