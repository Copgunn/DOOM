//! M6a step 8: the session's level state against the genuine engine, whole state, every level
//! tic of every level-chain item.
//!
//! Each of the 18 level-chain items ([`m6a::CHAIN_ITEMS`]) is run by the Rust session from its
//! committed demo (the same `Session::tic` as play, screenblocks 10), and after every session
//! tic that ends in a level (a level tic, or a load with the new level's tic 0) the level's
//! `DSIM` 42 capture (`Simulation::snapshot`) is reduced to the state digest of
//! `tests/reference/m6a_state.txt` (DOOM-M6A-STATE-v1, written by `tools/reference/m6a_state.py`
//! from the genuine linuxdoom-1.10 runner's dumps through the independent DSIM42 encoder
//! `tools/reference/level_snapshot_m6a.py`; no Rust output feeds it):
//!
//! - **state digest**: SHA-256 of the capture without `map.*` and `events.*` in the canonical
//!   framing (`DSIM`, u32 version 42, u64 field count, then per field u32 path length, path,
//!   u64 value length, value), followed by its `events.sounds.*` fields in the same per-field
//!   framing. Every actor (position, momentum, state, tics, health, flags, targets), every
//!   thinker (movers, lights), sectors, lines and buttons, the player (its mobj, command,
//!   weapons, psprites, `bob`), the level counters, both random indices, the exit request and
//!   wire 26's start group (`level.start.*`, the carried `bob`, weapon x and flash included) are
//!   in it. Left out: `events.last` and `events.pending` (Rust lists the tic's events, the
//!   encoder writes them empty, M5c note 4.4.4); `map.*` is compared once per level (below);
//! - per level: the **map digest** (the `map.*` fields of capture 1), the number of captures
//!   and the **sequence digest** over every capture's state digest in order, so every level
//!   tic is compared; the D rows (capture 1, every 35th level tic, each level's last capture)
//!   place a first difference within 35 tics.
//!
//! Capture 0 of a level after the first is no session state: its load and its tic 0 share one
//! `G_Ticker` (g_game.c:617-650, 727-729), so no session tic ends between them.
mod m6a;

use doom_game::session::demo::SessionDemo;
use doom_game::session::{PlantedFault, Session, SessionSettings, StartError, TicKind};
use doom_sim::Snapshot;
use doom_test_evidence::sha256;
use m6a::Item;
use std::collections::BTreeMap;

/// One level of the golden: its L row and its D rows by capture.
#[derive(Debug, Default)]
struct GoldenLevel {
    tic: i64,
    map: u8,
    kind: String,
    captures: usize,
    map_sha: String,
    sequence: String,
    points: BTreeMap<usize, (i64, String, String)>,
}

/// The golden levels of every item, in load order.
fn golden() -> BTreeMap<String, Vec<GoldenLevel>> {
    let text = std::fs::read_to_string(m6a::reference("m6a_state.txt")).expect("golden");
    let mut out: BTreeMap<String, Vec<GoldenLevel>> = BTreeMap::new();
    assert!(text.starts_with("#schema,DOOM-M6A-STATE-v1\n"));
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        let levels = out.entry(f[1].to_owned()).or_default();
        let load: usize = f[2].parse().expect("load");
        match f[0] {
            "L" => {
                assert_eq!(levels.len(), load, "{line}");
                levels.push(GoldenLevel {
                    tic: f[3].parse().expect("tic"),
                    map: f[4].parse().expect("map"),
                    kind: f[5].to_owned(),
                    captures: f[6].parse().expect("captures"),
                    map_sha: f[7].to_owned(),
                    sequence: f[8].to_owned(),
                    points: BTreeMap::new(),
                });
            }
            "D" => {
                let leveltime: usize = f[4].parse().expect("leveltime");
                let point = (f[3].parse().expect("tic"), f[5].to_owned(), f[6].to_owned());
                levels[load].points.insert(leveltime, point);
            }
            _ => panic!("row {line}"),
        }
    }
    out
}

fn framed(out: &mut Vec<u8>, path: &str, value: &[u8]) {
    out.extend((path.len() as u32).to_le_bytes());
    out.extend(path.as_bytes());
    out.extend((value.len() as u64).to_le_bytes());
    out.extend(value);
}

/// The state digest of a capture (module documentation), as 32 bytes, and the map digest.
fn digests(s: &Snapshot) -> ([u8; 32], String) {
    assert_eq!(s.version(), 42, "wire 26 captures are DSIM 42");
    let kept: Vec<_> = s
        .fields()
        .filter(|f| !f.path().starts_with("map.") && !f.path().starts_with("events."))
        .collect();
    let mut body = b"DSIM".to_vec();
    body.extend(42_u32.to_le_bytes());
    body.extend((kept.len() as u64).to_le_bytes());
    for f in &kept {
        framed(&mut body, f.path(), f.value());
    }
    let mut map = Vec::new();
    for f in s.fields() {
        if f.path().starts_with("events.sounds.") {
            framed(&mut body, f.path(), f.value());
        } else if f.path().starts_with("map.") {
            framed(&mut map, f.path(), f.value());
        }
    }
    (unhex(&sha256(&body)), sha256(&map))
}

fn unhex(h: &str) -> [u8; 32] {
    std::array::from_fn(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).expect("hex"))
}

/// One Rust level: its load tic, map, captures' state digests and map digest.
#[derive(Debug, Default)]
struct RustLevel {
    tic: i64,
    map: u8,
    kind: String,
    captures: Vec<([u8; 32], i64)>,
    map_sha: Option<String>,
}

/// What one item compared, or its first difference.
#[derive(Debug, Default)]
struct Outcome {
    levels: usize,
    captures: usize,
    points: usize,
    first_difference: Option<String>,
}

fn run_levels(item: &Item, fault: Option<PlantedFault>) -> Vec<RustLevel> {
    let data = m6a::data(item, None);
    let lmp = std::fs::read(m6a::root(&item.lmp)).expect("demo");
    let demo = SessionDemo::parse(&lmp).expect("session demo");
    let settings = SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: 10,
    };
    let mut session = match Session::new(data, settings) {
        Ok(s) => s,
        Err(StartError::Sound(_)) if item.session_tics == 0 => return Vec::new(),
        Err(e) => panic!("{}: start {e:?}", item.name),
    };
    if let Some(f) = fault {
        session.plant_fault(f);
    }
    let mut levels = vec![RustLevel {
        tic: -1,
        map: item.map,
        kind: "new".into(),
        ..RustLevel::default()
    }];
    for (t, &cmd) in demo.commands.iter().enumerate() {
        let Ok(r) = session.tic(cmd) else {
            break;
        };
        match r.kind {
            TicKind::Load { map, start } => levels.push(RustLevel {
                tic: t as i64,
                map,
                kind: format!("{start:?}").to_lowercase(),
                ..RustLevel::default()
            }),
            TicKind::Level => {}
            TicKind::Completion | TicKind::Intermission => continue,
        }
        let level = levels.last_mut().expect("level");
        let (state, map) = digests(&session.level().snapshot());
        level.map_sha.get_or_insert(map);
        level.captures.push((state, t as i64));
    }
    levels
}

fn compare(item: &Item, fault: Option<PlantedFault>, golden: &[GoldenLevel]) -> Outcome {
    let rust = run_levels(item, fault);
    let mut o = Outcome::default();
    let name = &item.name;
    if rust.len() != golden.len() {
        o.first_difference = Some(format!(
            "{name}: {} levels, the genuine run {}",
            rust.len(),
            golden.len()
        ));
        return o;
    }
    for (k, (r, g)) in rust.iter().zip(golden).enumerate() {
        let head = (r.tic, r.map, r.kind.as_str(), r.captures.len());
        if head != (g.tic, g.map, g.kind.as_str(), g.captures) {
            o.first_difference = Some(format!(
                "{name} load {k}: (tic, map, kind, captures) {head:?}, genuine {:?}",
                (g.tic, g.map, &g.kind, g.captures)
            ));
            return o;
        }
        if r.map_sha.as_deref() != Some(g.map_sha.as_str()) {
            o.first_difference = Some(format!("{name} load {k}: map group differs"));
            return o;
        }
        for (&lt, (tic, why, want)) in &g.points {
            let (got, rtic) = r.captures[lt - 1];
            assert_eq!(rtic, *tic, "{name} load {k}: capture {lt} at session tic");
            o.points += 1;
            if hex(&got) != *want {
                o.first_difference = Some(format!(
                    "{name} load {k}: session tic {tic} (level tic {}, {why}): state differs",
                    lt - 1
                ));
                return o;
            }
        }
        let all: Vec<u8> = r.captures.iter().flat_map(|c| c.0).collect();
        if sha256(&all) != g.sequence {
            o.first_difference = Some(format!(
                "{name} load {k}: the sequence differs between the D rows (every point equal)"
            ));
            return o;
        }
        o.levels += 1;
        o.captures += r.captures.len();
    }
    o
}

fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn the_session_level_state_equals_the_genuine_engine() {
    let golden = golden();
    assert_eq!(
        golden
            .keys()
            .filter(|k| m6a::CHAIN_ITEMS.contains(&k.as_str()))
            .count(),
        m6a::CHAIN_ITEMS.len() - 1,
        "every chain item but NOE1M2AT (no level) has levels in the golden"
    );
    let mut total = Outcome::default();
    for name in m6a::CHAIN_ITEMS {
        let item = m6a::item(name);
        let g = golden.get(name).map_or(&[][..], Vec::as_slice);
        let o = compare(&item, None, g);
        eprintln!(
            "{name}: {} levels, {} captures, {} points{}",
            o.levels,
            o.captures,
            o.points,
            o.first_difference
                .as_deref()
                .map_or(String::new(), |d| format!("; {d}"))
        );
        assert!(o.first_difference.is_none(), "{:?}", o.first_difference);
        total.levels += o.levels;
        total.captures += o.captures;
        total.points += o.points;
    }
    eprintln!(
        "total: {} levels, {} captures (every level tic), {} points",
        total.levels, total.captures, total.points
    );
    assert_eq!((total.levels, total.captures), (55, 4_889));
}

/// A carried start's `bob` one unit off (`PlantedFault::CarriedBob`): the step-7 gate does not
/// see it (`tests/m6a_session.rs`, `a_carried_bob_fault_passes_the_row_gate`: rows, frames,
/// sound and mixer all equal the genuine goldens), and this gate catches it at the first carried
/// load's first capture.
#[test]
fn the_state_comparison_catches_what_the_rows_miss() {
    let golden = golden();
    let o = compare(
        &m6a::item("CHAIN"),
        Some(PlantedFault::CarriedBob),
        &golden["CHAIN"],
    );
    let d = o.first_difference.expect("caught");
    eprintln!("CHAIN CarriedBob: {d}");
    assert!(
        d.contains("CHAIN load 1: session tic 142 (level tic 0, load): state differs"),
        "{d}"
    );
}

/// The items of the owner's rulings of 2026-11-22 ([`m6a::RULING_ITEMS`]: TURBO, the turbo
/// message on the C's `gametic` across loads, and INTCMD, intermission commands outside the
/// level domain), every level tic, whole state; and the golden holds exactly the chain items and
/// these.
#[test]
fn the_ruling_items_level_state_equals_the_genuine_engine() {
    let golden = golden();
    let mut names: Vec<&str> = m6a::CHAIN_ITEMS
        .iter()
        .chain(m6a::RULING_ITEMS.iter())
        .copied()
        .filter(|n| *n != "NOE1M2AT")
        .collect();
    names.sort_unstable();
    assert_eq!(golden.keys().map(String::as_str).collect::<Vec<_>>(), names);
    for name in m6a::RULING_ITEMS {
        let o = compare(&m6a::item(name), None, &golden[name]);
        eprintln!(
            "{name}: {} levels, {} captures, {} points",
            o.levels, o.captures, o.points
        );
        assert!(o.first_difference.is_none(), "{:?}", o.first_difference);
        assert_eq!(o.levels, golden[name].len());
    }
}

/// `PlantedFault::TurboLeveltime` (every load at `gametic` 0): the state gate catches it at
/// TURBO's first later load, where `level.start.gametic` and the turbo message both differ.
#[test]
fn the_state_comparison_catches_the_turbo_leveltime_fault() {
    let golden = golden();
    let o = compare(
        &m6a::item("TURBO"),
        Some(PlantedFault::TurboLeveltime),
        &golden["TURBO"],
    );
    let d = o.first_difference.expect("caught");
    eprintln!("TURBO TurboLeveltime: {d}");
    assert!(
        d.contains("TURBO load 1: session tic 192 (level tic 0, load): state differs"),
        "{d}"
    );
}

/// Local only (never in CI): every run of the real-map goldens of
/// `tools/local/m6a_session_real.py` (the genuine engine on a scratch copy of the owner's IWAD),
/// through this same state gate. Run with `M6A_REFERENCE_DIR=<its --out> cargo test --release
/// -p doom-game --test m6a_state -- --ignored real_maps`; without the variable it does nothing.
#[test]
#[ignore = "local only: needs M6A_REFERENCE_DIR from tools/local/m6a_session_real.py"]
fn real_maps_state_local() {
    if !m6a::real_maps() {
        eprintln!("skip: M6A_REFERENCE_DIR is not set");
        return;
    }
    let golden = golden();
    let mut total = Outcome::default();
    let mut failures = Vec::new();
    let items = m6a::real_items();
    for item in &items {
        let g = golden.get(&item.name).map_or(&[][..], Vec::as_slice);
        let o = compare(item, None, g);
        eprintln!(
            "{}: {} levels, {} captures, {} points{}",
            item.name,
            o.levels,
            o.captures,
            o.points,
            o.first_difference
                .as_deref()
                .map_or(String::new(), |d| format!(" DIFFERENCE {d}"))
        );
        failures.extend(o.first_difference);
        total.levels += o.levels;
        total.captures += o.captures;
        total.points += o.points;
    }
    eprintln!(
        "real maps, {} runs: {} levels, {} captures, {} points",
        items.len(),
        total.levels,
        total.captures,
        total.points
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Local only (never in CI): the real-map state gate can fail where the row gate cannot. A
/// carried start's `bob` one unit off (`PlantedFault::CarriedBob`, which passes the session
/// gate's rows, frames, sound and mixer: `a_carried_bob_fault_passes_the_row_gate`) is caught
/// in every real run whose golden has a carried start, and there is at least one. Run as
/// [`real_maps_state_local`]; `M6A_REAL_ONLY` narrows it.
#[test]
#[ignore = "local only: needs M6A_REFERENCE_DIR from tools/local/m6a_session_real.py"]
fn real_maps_state_local_catches_a_planted_fault() {
    if !m6a::real_maps() {
        eprintln!("skip: M6A_REFERENCE_DIR is not set");
        return;
    }
    let golden = golden();
    let carried = m6a::carried_items();
    let mut caught = 0;
    let mut missed = Vec::new();
    for item in m6a::real_items() {
        if !carried.contains(&item.name) {
            continue;
        }
        let o = compare(&item, Some(PlantedFault::CarriedBob), &golden[&item.name]);
        match o.first_difference {
            Some(d) => {
                eprintln!("{} CarriedBob: {d}", item.name);
                caught += 1;
            }
            None => missed.push(item.name.clone()),
        }
    }
    eprintln!("real maps, CarriedBob: caught in {caught} runs, missed in {missed:?}");
    assert!(caught > 0, "no real run with a carried start");
    assert!(missed.is_empty(), "not caught: {missed:?}");
}

/// The stale-attacker item `ATTACK` (owner's ruling of 2026-11-22): every level tic of both
/// levels, whole state, equal to the genuine digests. The genuine E1M2 load holds
/// `players[0].attacker` as a pointer into the freed E1M1, which the independent encoder projects
/// as NULL (`tools/reference/m6a_state.py`'s rule); the carried start's NULL equals it.
#[test]
fn the_stale_attacker_item_level_state_equals_the_genuine_engine() {
    let golden = golden();
    let g = &golden["ATTACK"];
    assert_eq!(
        g.iter()
            .map(|l| (l.tic, l.map, l.kind.as_str()))
            .collect::<Vec<_>>(),
        [(-1, 1, "new"), (235, 2, "carried")]
    );
    let o = compare(&m6a::item("ATTACK"), None, g);
    eprintln!(
        "ATTACK: {} levels, {} captures, {} points",
        o.levels, o.captures, o.points
    );
    assert!(o.first_difference.is_none(), "{:?}", o.first_difference);
    assert_eq!((o.levels, o.captures), (2, 254));
}

/// The stale-attacker rule's record (`tests/reference/m6a_stale_attacker.txt`, written by
/// `tools/reference/m6a_state.py` from the genuine dumps): it fired on exactly one load of all
/// the committed items, ATTACK's carried load into E1M2 at session tic 235, which the state
/// golden holds as a carried level.
#[test]
fn the_stale_attacker_rule_fired_on_attacks_carried_load_only() {
    if m6a::real_maps() {
        eprintln!("skip: M6A_REFERENCE_DIR is set (the record is the synthetic set's)");
        return;
    }
    let text =
        std::fs::read_to_string(m6a::reference("m6a_stale_attacker.txt")).expect("fired record");
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("#schema,DOOM-M6A-STALE-ATTACKER-v1"));
    let rows: Vec<&str> = lines.filter(|l| !l.starts_with('#')).collect();
    assert_eq!(rows, ["A,ATTACK,1,235", "T,1"]);
    let golden = golden();
    let level = &golden["ATTACK"][1];
    assert_eq!((level.tic, level.kind.as_str()), (235, "carried"));
}
