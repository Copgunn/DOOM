//! Wire 23 (`PlayerMonstersEpisode1V1`, DSIM39) on the M3h9c synthetic WAD (M4a plan
//! step 3): before the Episode 1 mechanics land, the policy must run every wire-22
//! workload exactly as wire 22 does. Each synthetic script's demo is replayed under
//! both policies in lockstep; every tic's DSIM39 capture is the DSIM38 capture with
//! the DSIM39 fields inserted (plan §A.6) and the policy labels changed, and the two
//! runs refuse the same tic with the same error. The DSIM38 side is itself pinned
//! to the independent encoder's goldens by `level_fixtures.rs`; this test compares
//! Rust with Rust and only shows the new policy adds nothing yet.
use doom_sim::{demo::parse_demo, *};
use std::path::{Path, PathBuf};

fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}

fn map() -> doom_map::Map {
    let bytes = std::fs::read(reference("m3h9c_synthetic.wad")).expect("synthetic WAD");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("WAD");
    doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).expect("E1M1")
}

fn start(episode1: bool) -> Simulation {
    let mut s = if episode1 {
        Simulation::original_player_episode1_map(map(), MapContextOptions::default())
    } else {
        Simulation::original_player_level_map(map(), MapContextOptions::default())
    }
    .expect("admission");
    s.spawn_map_things().expect("THINGs");
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .expect("weapons");
    s.spawn_level_specials().expect("specials");
    s
}

/// The DSIM39 additions of plan §A.6; a plat `type` is the one addition whose
/// name also exists on DSIM38 door and floor records, so it is recognised by
/// absence from the DSIM38 capture.
fn new_in_39(path: &str) -> bool {
    path == "level.secret_exit"
        || path == "original_player.0.backpack"
        || path.starts_with("original_player.0.powers.")
        || path.starts_with("original_player.0.cards.")
        || path.starts_with("original_player.0.weapons.max_ammo.")
        || path == "original_player.0.weapons.cells"
        || path == "original_player.0.weapons.rockets"
        || (path.starts_with("sector.") && path.ends_with(".floorpic"))
        || (path.starts_with("sector_thinker.")
            && (path.ends_with(".direction")
                || path.ends_with(".speed")
                || path.ends_with(".type")))
}

fn compare(old: &Snapshot, new: &Snapshot, tic: usize) {
    assert_eq!(old.version(), 38);
    assert_eq!(new.version(), 39);
    let w22: Vec<_> = old.fields().map(|f| (f.path(), f.value())).collect();
    let paths: std::collections::BTreeSet<&str> = w22.iter().map(|(p, _)| *p).collect();
    let kept: Vec<_> = new
        .fields()
        .filter(|f| {
            let known = paths.contains(f.path());
            assert!(known || new_in_39(f.path()), "tic {tic}: {}", f.path());
            known
        })
        .map(|f| (f.path(), f.value()))
        .collect();
    assert_eq!(kept.len(), w22.len(), "tic {tic}");
    for ((p, a), (q, b)) in w22.iter().zip(&kept) {
        assert_eq!(p, q, "tic {tic}");
        match *p {
            "profile" => assert_eq!(
                (*a, *b),
                (&22u32.to_le_bytes()[..], &23u32.to_le_bytes()[..])
            ),
            "context.schema" => assert_eq!(*b, PLAYER_EPISODE1_CONTEXT_SCHEMA),
            "store.max_slots" => {
                assert_eq!(
                    (*a, *b),
                    (&384u32.to_le_bytes()[..], &1024u32.to_le_bytes()[..])
                )
            }
            _ => assert_eq!(a, b, "tic {tic}: {p}"),
        }
    }
}

fn lockstep(script: &str) -> usize {
    let demo = parse_demo(
        &std::fs::read(reference(&format!("m3h9c_scripts/{script}.lmp"))).expect("demo"),
    )
    .expect("version-110 demo");
    let (mut old, mut new) = (start(false), start(true));
    compare(&old.snapshot(), &new.snapshot(), 0);
    let limits = DiagnosticLimits::default();
    for (i, command) in demo.commands.iter().enumerate() {
        let commands = [
            *command,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ];
        let (a, b) = (old.advance(commands), new.advance(commands));
        assert_eq!(a, b, "{script} tic {}", i + 1);
        if a.is_err() {
            return i + 1;
        }
        // A divergence persists in canonical state (RNG, positions, thinkers),
        // so every 25th capture is compared in full (debug-build cost).
        if i % 25 == 0 {
            let snap = new.snapshot();
            compare(&old.snapshot(), &snap, i + 1);
            if i % 200 == 0 {
                Diagnostic::decode_episode1(&snap.canonical_bytes(), limits)
                    .unwrap_or_else(|e| panic!("{script} tic {}: {e:?}", i + 1));
            }
        }
    }
    let snap = new.snapshot();
    compare(&old.snapshot(), &snap, demo.commands.len());
    Diagnostic::decode_episode1(&snap.canonical_bytes(), limits).expect("final capture");
    demo.commands.len()
}

#[test]
fn synthetic_load_under_wire_23_is_the_wire_22_load_plus_dsim39() {
    let (old, new) = (start(false), start(true));
    compare(&old.snapshot(), &new.snapshot(), 0);
    let bytes = new.snapshot().canonical_bytes();
    Diagnostic::decode_episode1(&bytes, DiagnosticLimits::default()).expect("DSIM39 load");
    let bounds = new.episode1_bounds().expect("per-map bounds");
    assert_eq!(bounds, Episode1Bounds::from_map(&map()));
    assert_eq!(old.episode1_bounds(), None);
}

#[test]
fn every_synthetic_script_runs_identically_under_wire_23() {
    for script in [
        "SYNIDLE", "SYNWALK", "SYNFIGHT", "SYNBARRL", "SYNDIE", "SYNCHAOS",
    ] {
        assert!(lockstep(script) > 0, "{script}");
    }
}
