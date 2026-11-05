//! Real E1M1 under the level policy (wire 22), from separately authorized local
//! assets only (`DOOM_IWAD_DIR`). Prints counts, booleans and digests only; no map
//! datum, lump byte or coordinate leaves the process.
use doom_sim::*;

fn field<'a>(snap: &'a Snapshot, path: &str) -> &'a [u8] {
    snap.fields()
        .find(|f| f.path() == path)
        .map_or(&[], StateField::value)
}

#[test]
#[ignore = "requires separately authorized assets via DOOM_IWAD_DIR"]
fn private_iwad_e1m1_level_policy() {
    let Some(directory) = std::env::var_os("DOOM_IWAD_DIR").filter(|v| !v.is_empty()) else {
        println!("DOOM_IWAD_DIR unset; nothing to run");
        return;
    };
    let mut paths: Vec<_> = std::fs::read_dir(directory)
        .expect("asset directory")
        .map(|e| e.expect("asset entry").path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("wad")))
        .collect();
    paths.sort();
    let mut runs = 0;
    for (index, path) in paths.iter().enumerate() {
        let bytes = std::fs::read(path).expect("asset bytes");
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("archive");
        let Ok(map) = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()) else {
            continue;
        };
        runs += 1;
        let mut s = Simulation::original_player_level_map(map, MapContextOptions::default())
            .expect("wire-22 admission");
        let report = s.spawn_map_things().expect("THINGs");
        s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
            .expect("weapons");
        s.spawn_level_specials().expect("specials");
        let snap = s.snapshot();
        assert_eq!(snap.version(), 38);
        assert_eq!(
            field(&snap, "rng.gameplay"),
            [144],
            "P_SetupLevel draws 144"
        );
        let body = report.player_body.expect("the body");
        let last = report.spawned.last().expect("spawned THINGs");
        let before_last = body.slot() < last.slot();
        if !before_last {
            println!("wad {index}: no spawned THING follows the type-1 record");
        }
        Diagnostic::decode_level(&snap.canonical_bytes(), DiagnosticLimits::default())
            .expect("DSIM38 at load");
        let mut refused = None;
        for tic in 1..=2100 {
            if let Err(e) = s.advance([TicCommand::default(); 4]) {
                refused = Some((tic, e));
                break;
            }
        }
        assert!(refused.is_none(), "refused: {refused:?}");
        let end = s.snapshot();
        Diagnostic::decode_level(&end.canonical_bytes(), DiagnosticLimits::default())
            .expect("DSIM38 after 2,100 tics");
        println!(
            "wad {index}: spawned {} actors {} body_before_last_spawned {before_last} \
             rng_load 144 idle_tics 2100 refused no rng_end {} final_fnv {:016x}",
            report.spawned.len(),
            s.actor_order().count(),
            field(&end, "rng.gameplay")[0],
            end.digest()
        );
    }
    println!("E1M1 runs: {runs}");
    assert!(runs > 0, "no E1M1 in the asset directory");
}
