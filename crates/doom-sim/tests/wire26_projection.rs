//! Wire 26 (`PlayerMonstersEpisode1V4`, DSIM42) started as a new game against wire
//! 25 (DSIM41), Rust against Rust (M6a step 4, the M5c method of M6a note §4.3): on
//! every committed synthetic item wire 25 runs (the 64 items of
//! tests/reference/m5c_sound, `wire25::ITEMS`, and the T4 truncation map), and on the
//! first level of every committed M6a chain item whose map wire 25 loads (a map holding
//! the walk exits 52 or 124, which wire 26 alone admits, is counted), every wire-26
//! capture with its start
//! group removed and its names set back to wire 25's has the wire-25 capture's bytes
//! and FNV-1a digest, the sound records are equal, both wires refuse at the same tic
//! with the same error, every DSIM42 capture is read by `decode_episode1_v4` and
//! refused by `decode_episode1_v3`. This is consistency evidence that wire 26 changes
//! nothing but the start; it is not genuine evidence for the carried start (that is
//! `wire26_chain.rs`).

mod wire25;
mod wire26;

use doom_sim::demo::parse_demo_episode1_v2;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// (lump, the WAD holding its maps, demo) of every committed M6a chain item; the
/// map is the demo header's.
const M6A_ITEMS: [(&str, &str); 18] = [
    ("CHAIN", "m6a_wads/episode.wad"),
    ("INTSTA", "m6a_wads/episode.wad"),
    ("INTSTB", "m6a_wads/episode.wad"),
    ("INTIDLE", "m6a_wads/episode.wad"),
    ("INTHELD", "m6a_wads/episode.wad"),
    ("INTFAST", "m6a_wads/episode.wad"),
    ("REBHELD", "m6a_wads/episode.wad"),
    ("REBLATE", "m6a_wads/episode.wad"),
    ("SECRET", "m6a_wads/episode.wad"),
    ("REBSECRT", "m6a_wads/episode.wad"),
    ("PROBE", "m6a_wads/rbx.wad"),
    ("USEAT130", "m6a_wads/rbx.wad"),
    ("USEAT131", "m6a_wads/rbx.wad"),
    ("USEAT132", "m6a_wads/rbx.wad"),
    ("RBXDEAD", "m6a_wads/rbx.wad"),
    ("NOINTER", "m6a_wads/episode.wad"),
    ("NOE1M2", "m6a_wads/episode.wad"),
    ("NOE1M2AT", "m6a_wads/episode.wad"),
];

fn run(wad: &str, lmp: &str) -> wire26::Outcome {
    let reference = root().join("tests/reference");
    let bytes = std::fs::read(reference.join(wad)).expect("wad");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("wad parse");
    let demo = std::fs::read(reference.join(lmp)).expect("demo");
    let number = demo[3];
    let map = doom_map::Map::from_wad(
        &wad,
        &format!("E1M{number}"),
        doom_map::MapLimits::default(),
    )
    .expect("map");
    let (header, demo) = parse_demo_episode1_v2(&demo, number).expect("demo parse");
    wire26::run_pair(&map, number, wire26::skill(header.skill), &demo.commands)
        .unwrap_or_else(|e| panic!("{lmp}: {e}"))
}

#[test]
fn wire26_new_game_projects_to_wire25_on_every_m5c_item() {
    let (mut captures, mut refused) = (0, 0);
    for (lump, wad, lmp) in wire25::ITEMS {
        let out = run(wad, lmp);
        assert_eq!(out.load_refused, None, "{lump}");
        assert_eq!(out.wire25_only_refused, None, "{lump}");
        captures += out.captures;
        refused += usize::from(out.refused.is_some());
        println!(
            "{lump}: {} captures, refused {:?}",
            out.captures, out.refused
        );
    }
    let t4 = run("m5c_t4/truncate.wad", "m5c_t4/TRUNC.lmp");
    captures += t4.captures;
    println!(
        "wire26_projection (M5c): {} items + TRUNC ({} captures), {captures} captures equal, \
         {refused} refused runs",
        wire25::ITEMS.len(),
        t4.captures
    );
}

/// The table above is the committed set of M6a session goldens.
#[test]
fn the_m6a_item_table_is_the_session_golden_set() {
    let mut goldens: Vec<String> = std::fs::read_dir(root().join("tests/reference/m6a_session"))
        .expect("goldens")
        .map(|e| e.expect("entry").file_name().into_string().expect("name"))
        .filter_map(|n| n.strip_suffix(".session").map(str::to_owned))
        .collect();
    goldens.sort();
    let mut lumps: Vec<String> = M6A_ITEMS.iter().map(|i| i.0.to_owned()).collect();
    lumps.sort();
    assert_eq!(goldens, lumps);
}

/// The first level of every M6a chain item (its demo runs on past the level; both
/// wires refuse the tic after the exit or the reborn with the same error).
#[test]
fn wire26_new_game_projects_to_wire25_on_every_m6a_first_level() {
    let (mut captures, mut refused, mut loads, mut walk) = (0, 0, 0, 0);
    for (lump, wad) in M6A_ITEMS {
        let out = run(wad, &format!("m6a_scripts/{lump}.lmp"));
        captures += out.captures;
        refused += usize::from(out.refused.is_some());
        loads += usize::from(out.load_refused.is_some());
        walk += usize::from(out.wire25_only_refused.is_some());
        println!(
            "{lump}: {} captures, refused {:?}, load refused {:?}, wire 25 alone refused {:?}",
            out.captures, out.refused, out.load_refused, out.wire25_only_refused
        );
    }
    println!(
        "wire26_projection (M6a first levels): {} items, {captures} captures equal, {refused} \
         runs end in a refusal, {loads} loads refused alike by both wires, {walk} maps with a \
         walk exit refused by wire 25 alone",
        M6A_ITEMS.len()
    );
}
