//! M6a part 2, the part 2 golden set (owner's addition of 2026-11-23 to ruling V9): every melt
//! at screenblocks 11. The finale-and-melt display (`SessionConfig::FinaleAndMelt`) against the
//! genuine linuxdoom-1.10 engine's melt goldens at screenblocks 11
//! (`tests/reference/m6a2_melt/<ITEM>.sb11.melt`, written by `tools/reference/m6a2_goldens.py`
//! from the part 1 items' committed demos and WADs; no Rust output feeds an expectation), driven
//! exactly as `m6a_finale_melt_display.rs` drives the part 1 items at screenblocks 10.
//!
//! Every part 1 item is compared at screenblocks 11 on every frame, the `M_Random` index, the
//! finale's stage and count and every melt: start and end screens, palette, the step count and
//! every step's picture digest and `done` flag (the owner's ruling V4: one SHA-256 per melt step
//! over the 64,000 palette indices, and the palette per melt). The frames and session rows are
//! the part 1 goldens, unchanged; only the melt rows are new. The two full-crawl items' melts at
//! screenblocks 11 are compared by `m6a2_full_crawl`.
mod m6a;
mod m6a2;

use doom_render::MeltFault;
use m6a2::{Counts, Golden, drive, melt_sb11_names};

/// Items whose demo the session refuses before any tic: the easy skills (skill bytes 0 and 1,
/// the next pull request).
const NOT_DRIVEN: [&str; 2] = ["SKILL0", "SKILL1"];

/// Every part 1 item at screenblocks 11 with its melts.
#[test]
fn every_part1_item_at_screenblocks_11_with_its_melts() {
    let full = ["FULLREG", "FULLRET"];
    let names = melt_sb11_names();
    let mut total = Counts::default();
    let (mut golden_melts, mut golden_steps) = (0usize, 0usize);
    let (mut uncovered_melts, mut uncovered_steps) = (0usize, 0usize);
    let mut not_driven = Vec::new();
    let mut kinds = std::collections::BTreeMap::<&str, usize>::new();
    let mut items = 0usize;
    for item in m6a::items() {
        assert!(
            names.contains(&item.name),
            "{} has an sb11 melt golden",
            item.name
        );
        let golden = Golden::part1_sb11(&item.name);
        let gm = golden.melts.as_ref().expect("melts");
        let steps: usize = gm.values().map(|m| m.steps.len()).sum();
        golden_melts += gm.len();
        golden_steps += steps;
        items += 1;
        match drive(&item, 11, &golden, None, None) {
            Ok((n, _)) => {
                eprintln!("{} sb11: {n:?}", item.name);
                for &t in gm.keys() {
                    let now = golden.frames[&t].gamestate;
                    let kind = match ((t > 0).then(|| golden.frames[&(t - 1)].gamestate), now) {
                        (None, _) => "session start",
                        (Some(0), 1) => "level to intermission",
                        (Some(1), 0) => "intermission to level",
                        (Some(0), 0) => "reborn reload",
                        (Some(0), 2) => "level to finale",
                        (Some(2), 2) => "finale text to art",
                        _ => panic!("{} tic {t}: an unexpected melt", item.name),
                    };
                    *kinds.entry(kind).or_insert(0) += 1;
                }
                total.tics += n.tics;
                total.level_frames += n.level_frames;
                total.intermission_frames += n.intermission_frames;
                total.finale_frames += n.finale_frames;
                total.finale_states += n.finale_states;
                total.index += n.index;
                total.melts += n.melts;
                total.melt_steps += n.melt_steps;
            }
            Err(e) if item.session_tics == 0 => {
                eprintln!("{}: {e}", item.name);
                assert!(gm.is_empty());
            }
            Err(e) if NOT_DRIVEN.contains(&item.name.as_str()) => {
                eprintln!("{} sb11: not driven: {e}", item.name);
                not_driven.push(item.name.clone());
                uncovered_melts += gm.len();
                uncovered_steps += steps;
            }
            Err(e) => panic!("{e}"),
        }
    }
    eprintln!(
        "sb11 {total:?}; melts compared {} of {golden_melts}, steps {} of {golden_steps}; \
         kinds {kinds:?}; not driven {not_driven:?} ({uncovered_melts} melts, \
         {uncovered_steps} steps)",
        total.melts, total.melt_steps
    );
    assert_eq!(items + full.len(), names.len());
    assert_eq!(not_driven, NOT_DRIVEN);
    assert_eq!(
        (
            total.melts + uncovered_melts,
            total.melt_steps + uncovered_steps
        ),
        (golden_melts, golden_steps)
    );
    assert_eq!(kinds.len(), 6, "every kind of melt: {kinds:?}");
    assert_eq!(total.index, total.tics);
    // The frames and indices are those the part 1 test compares at screenblocks 11
    // (m6a_finale_melt_display: 16,197 tics, 8,185 level, 3,182 intermission, 4,830 finale).
    assert_eq!(
        (
            total.tics,
            total.level_frames,
            total.intermission_frames,
            total.finale_frames
        ),
        (16197, 8185, 3182, 4830)
    );
}

/// The planted faults at screenblocks 11, each caught in the first melt of CHAIN: a melt
/// column's offset off by one, a dropped step and a wrong melt palette.
#[test]
fn the_planted_faults_are_caught_at_screenblocks_11() {
    let item = m6a::item("CHAIN");
    let golden = Golden::part1_sb11("CHAIN");
    for (fault, want) in [
        (MeltFault::ColumnOffByOne, "CHAIN sb11 tic 0: melt step"),
        (
            MeltFault::DroppedStep,
            "CHAIN sb11 tic 0: melt step 5 picture differs",
        ),
        (MeltFault::WrongPalette, "CHAIN sb11 tic 0: melt palette"),
    ] {
        let e = drive(&item, 11, &golden, Some(fault), None)
            .map(|_| ())
            .expect_err("a planted fault must be caught");
        eprintln!("CHAIN sb11 {fault:?}: {e}");
        assert!(e.starts_with(want), "{fault:?}: {e}");
    }
}
