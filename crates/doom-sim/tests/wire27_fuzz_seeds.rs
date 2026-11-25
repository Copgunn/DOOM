//! Seed writer for the fuzz target `fuzz/fuzz_targets/wire27_dsim43.rs` (the
//! `DSIM` 43 reader, wire 27). Local only: it writes outside the repository and
//! asserts nothing about the genuine engine.
use doom_sim::{demo::parse_demo_episode1_v2, *};
use std::path::{Path, PathBuf};

fn reference(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(path)
}

/// Three seeds: the wire-27 captures of the item AIMSHOT (map slot E1M3 of
/// `tests/reference/wire27_wads/wire27.wad`) at session tics 0, 35 (the end of
/// its demo) and 70 (35 idle tics later), written as `t0`, `t35` and `t70`
/// under `$WIRE27_FUZZ_SEEDS/wire27_dsim43`. Each is checked to decode as
/// `DSIM` 43 before it is written.
#[test]
#[ignore = "local only: writes fuzz seeds under WIRE27_FUZZ_SEEDS"]
fn write_fuzz_seeds_local() {
    let Some(dir) = std::env::var_os("WIRE27_FUZZ_SEEDS") else {
        return;
    };
    let folder = Path::new(&dir).join("wire27_dsim43");
    std::fs::create_dir_all(&folder).expect("folder");
    let bytes = std::fs::read(reference("wire27_wads/wire27.wad")).expect("wad");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("wad parse");
    let map = doom_map::Map::from_wad(&wad, "E1M3", doom_map::MapLimits::default()).expect("map");
    let mut s = Simulation::original_player_episode1_v5_level(
        map,
        3,
        Skill::Medium,
        MapContextOptions::default(),
        LevelStart::NewGame,
    )
    .expect("the slot loads under wire 27");
    let lmp = std::fs::read(reference("wire27_scripts/AIMSHOT.lmp")).expect("demo");
    let demo = parse_demo_episode1_v2(&lmp, lmp[3]).expect("demo parse").1;
    let none = TicCommand::default();
    for tic in 0..=70_usize {
        if tic % 35 == 0 {
            let seed = s.snapshot().canonical_bytes();
            let capture = Diagnostic::decode_episode1_v5(&seed, DiagnosticLimits::default());
            assert_eq!(capture.expect("a DSIM 43 capture").version(), 43);
            std::fs::write(folder.join(format!("t{tic}")), seed).expect("seed");
        }
        let c = demo.commands.get(tic).copied().unwrap_or(none);
        s.advance([c, none, none, none]).expect("the tic runs");
    }
}
