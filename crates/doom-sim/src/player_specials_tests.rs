//! M3h7 sky and static specials: source replay of every committed scenario of the
//! `pools` and `sky` rooms, the specials admission and its rejections, the
//! `spawn_level_specials` load order, the transaction and the policy identity.
//! Expected behavior comes from the original source reading in
//! M3H7_REFERENCE_INVESTIGATION.md and the committed C rows, never from Rust output.
use super::*;
use crate::specials_evidence;

// Source replay: every committed scenario against the compiled original rows,
// the independent DSIM34 captures and ordered events; step equals advance.
#[test]
fn specials_source_case_00_pools_lights_soak() {
    specials_evidence::verify_case(0);
}
#[test]
fn specials_source_case_01_pools_nukage_damage() {
    specials_evidence::verify_case(1);
}
#[test]
fn specials_source_case_02_pools_nukage_death() {
    specials_evidence::verify_case(2);
}
#[test]
fn specials_source_case_03_pools_ledge_edge() {
    specials_evidence::verify_case(3);
}
#[test]
fn specials_source_case_04_pools_secret() {
    specials_evidence::verify_case(4);
}
#[test]
fn specials_source_case_05_pools_strobe_hurt() {
    specials_evidence::verify_case(5);
}
#[test]
fn specials_source_case_06_pools_light_and_fireball() {
    specials_evidence::verify_case(6);
}
#[test]
fn specials_source_case_07_sky_hack_wall() {
    specials_evidence::verify_case(7);
}
#[test]
fn specials_source_case_08_sky_one_sided_wall() {
    specials_evidence::verify_case(8);
}
#[test]
fn specials_source_case_09_sky_wall_after_hack() {
    specials_evidence::verify_case(9);
}
#[test]
fn specials_source_case_10_sky_ceiling_fireball() {
    specials_evidence::verify_case(10);
}
#[test]
fn specials_source_case_11_sky_shot_high() {
    specials_evidence::verify_case(11);
}
#[test]
fn specials_source_case_12_sky_shot_hack() {
    specials_evidence::verify_case(12);
}
#[test]
fn specials_source_case_13_sky_shot_window() {
    specials_evidence::verify_case(13);
}
#[test]
fn specials_source_case_14_sky_window_order() {
    specials_evidence::verify_case(14);
}
#[test]
fn specials_source_case_15_sky_window_strict() {
    specials_evidence::verify_case(15);
}
#[test]
fn specials_source_case_16_sky_window_back() {
    specials_evidence::verify_case(16);
}
/// Every scenario together: the committed capture count, every scripted tic and
/// nonzero source, event and sector comparisons (empty evidence cannot pass).
#[test]
fn specials_source_all_captures_and_tics() {
    specials_evidence::verify();
}

use crate::diagonal_room::Lumps;
use crate::sector_thinkers::SCROLLING_WALL;

fn options() -> MapContextOptions {
    MapContextOptions {
        initial_rng: DoomRng::from_indices(6, 0),
        respawn_monsters: false,
    }
}
/// The committed `pools` room (map 0): every admitted sector special, three
/// one-sided scrollers (lines 16, 18, 44), line 0 two-sided.
fn pools() -> Lumps {
    Lumps::parse(specials_evidence::MAP_BYTES[0])
}
fn lump<'a>(lumps: &'a mut Lumps, name: &str) -> &'a mut Vec<u8> {
    &mut lumps.lumps.iter_mut().find(|(n, _)| n == name).unwrap().1
}
fn set_sector_field(lumps: &mut Lumps, sector: usize, offset: usize, value: i16) {
    let s = lump(lumps, "SECTORS");
    s[26 * sector + offset..26 * sector + offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn set_ceiling(lumps: &mut Lumps, sector: usize, name: &[u8; 8]) {
    lump(lumps, "SECTORS")[26 * sector + 12..26 * sector + 20].copy_from_slice(name);
}
fn specials(lumps: &Lumps) -> Result<Simulation, OriginalRegistryError> {
    Simulation::original_player_specials_map(lumps.map(), options())
}
fn rejects(result: Result<Simulation, OriginalRegistryError>, expected: SimError) {
    match result {
        Err(OriginalRegistryError::Runtime(error)) => assert_eq!(error, expected),
        Err(other) => panic!("unexpected error {other:?}, expected {expected:?}"),
        Ok(_) => panic!("admitted, expected {expected:?}"),
    }
}
/// The committed setup of one scenario, before its first tic.
fn scenario(which: usize) -> Simulation {
    specials_evidence::setup(&specials_evidence::cases()[which]).0
}

#[test]
fn specials_admit_every_committed_room_and_reject_every_other_special() {
    for c in specials_evidence::cases() {
        // Every committed room loads under the specials policy; the real-map
        // policy (wire 17) still rejects each at load (a special or a sky ceiling).
        specials_evidence::setup(&c);
        let older =
            Simulation::original_player_real_map_map(specials_evidence::case_map(c.map), options());
        match older {
            Err(OriginalRegistryError::Runtime(
                SimError::Pursuit(PursuitError::Specials) | SimError::Combat(CombatError::Sky),
            )) => {}
            other => panic!("{}: {other:?}", c.label),
        }
    }
    // The superset: every committed M3h6 room loads unchanged.
    for c in crate::real_map_evidence::cases() {
        Simulation::original_player_specials_map(
            crate::real_map_evidence::case_map(c.map),
            options(),
        )
        .unwrap_or_else(|e| panic!("{}: {e:?}", c.label));
    }
    let specials_error = SimError::Pursuit(PursuitError::Specials);
    // Sector specials 10, 11 and 14 and values outside the source switch.
    for special in [6, 10, 11, 14, 15, 18, -1, 255] {
        let mut lumps = pools();
        set_sector_field(&mut lumps, 1, 22, special);
        rejects(specials(&lumps), specials_error);
    }
    // Every line special but 48, on a one-sided and on a two-sided line.
    for special in [1, 11, 24, 36, 46, 47, 49, 88, -1] {
        for line in [1, 0] {
            let mut lumps = pools();
            lumps.set_line_special(line, special);
            rejects(specials(&lumps), specials_error);
        }
    }
    // A two-sided scroller would enter `spechit` (Stage B).
    let mut lumps = pools();
    lumps.set_line_special(0, SCROLLING_WALL);
    rejects(specials(&lumps), specials_error);
    // A one-sided scroller is admitted.
    let mut lumps = pools();
    lumps.set_line_special(1, SCROLLING_WALL);
    specials(&lumps).unwrap();
    // A light level beyond the range where `short lightlevel` arithmetic is exact.
    for light in [i16::MAX - 63, i16::MIN + 63] {
        let mut lumps = pools();
        set_sector_field(&mut lumps, 14, 20, light);
        rejects(specials(&lumps), SimError::MapScale(MapScaleError::Light));
    }
    for light in [i16::MAX - 64, i16::MIN + 64] {
        let mut lumps = pools();
        set_sector_field(&mut lumps, 14, 20, light);
        specials(&lumps).unwrap();
    }
    // The real-map rules still apply: heights beyond 512.
    let mut lumps = pools();
    set_sector_field(&mut lumps, 13, 2, 513);
    assert!(specials(&lumps).is_err());
}

#[test]
fn at_most_sixty_four_one_sided_scrollers() {
    // `linespeciallist[MAXLINEANIMS]` is filled without a bound check: the 65th
    // scroller is refused at load. Copies of one-sided wall 1 reach the bound.
    let mut lumps = pools();
    let wall = lump(&mut lumps, "LINEDEFS")[14..28].to_vec();
    for line in [16, 18, 44] {
        lumps.set_line_special(line, 0);
    }
    for _ in 0..63 {
        lump(&mut lumps, "LINEDEFS").extend_from_slice(&wall);
    }
    let count = lumps.line_count();
    for line in 1..count {
        if line == 1 || line >= count - 63 {
            lumps.set_line_special(line, SCROLLING_WALL);
        }
    }
    let map = lumps.map();
    assert_eq!(
        map.linedefs()
            .iter()
            .filter(|l| l.special == SCROLLING_WALL)
            .count(),
        64
    );
    crate::demon_expanded_map::validate_specials(&map).unwrap();
    lump(&mut lumps, "LINEDEFS").extend_from_slice(&wall);
    lumps.set_line_special(count, SCROLLING_WALL);
    assert_eq!(
        crate::demon_expanded_map::validate_specials(&lumps.map()),
        Err(SimError::Pursuit(PursuitError::Specials))
    );
}

#[test]
fn sky_identity_is_the_f_sky1_lookup_name() {
    use crate::demon_expanded_map::is_sky_flat;
    use doom_types::LumpName;
    for (name, sky) in [
        (*b"F_SKY1\0\0", true),
        (*b"f_sky1\0\0", true),
        (*b"F_Sky1\0X", true),
        (*b"F_SKY10\0", false),
        (*b"F_SKY1  ", false),
        (*b"F_SKY2\0\0", false),
        (*b"TESTCEIL", false),
    ] {
        assert_eq!(is_sky_flat(&LumpName::from_bytes(name)), sky, "{name:?}");
    }
    // A pools ceiling renamed with trailing bytes after its NUL is sky.
    let mut lumps = pools();
    set_ceiling(&mut lumps, 13, b"F_SKY1\0Z");
    let s = specials(&lumps).unwrap();
    let sky = s.world.sky_ceilings.as_ref().unwrap();
    assert_eq!(
        sky.iter()
            .enumerate()
            .filter(|(_, s)| **s)
            .map(|(i, _)| i)
            .collect::<Vec<_>>(),
        [13]
    );
}

#[test]
fn spawn_level_specials_follows_the_body_runs_once_and_gates_stepping() {
    let c = &specials_evidence::cases()[0];
    let mut s =
        Simulation::original_player_specials_map(specials_evidence::case_map(c.map), options())
            .unwrap();
    let refused = SimError::MapSpawn(MapSpawnError::LevelSpecials);
    let before = s.snapshot();
    // Stepping before the load operation, and the operation before the body.
    assert_eq!(s.advance([TicCommand::default(); 4]), Err(refused));
    assert_eq!(s.spawn_level_specials(), Err(refused));
    assert_eq!(s.snapshot(), before);
    s.spawn_map_things().unwrap();
    assert_eq!(s.spawn_level_specials(), Err(refused));
    let mid = s.snapshot();
    assert_eq!(s.step([TicCommand::default(); 4]).map(|_| ()), Err(refused));
    assert_eq!(s.snapshot(), mid);
    s.spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
        .unwrap();
    assert!(!s.level_specials_spawned());
    s.spawn_level_specials().unwrap();
    assert!(s.level_specials_spawned());
    // Load only: every light sector's special is cleared but 4, total secrets 1.
    let after = s.snapshot();
    assert_eq!(s.level_statistics().unwrap().total_secrets, 1);
    let specials: Vec<_> = s
        .sector_lights_and_specials()
        .unwrap()
        .map(|(_, sp)| sp)
        .collect();
    assert_eq!(specials, [0, 7, 5, 16, 4, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    // A second call, a map-THING batch or another body after it: refused.
    assert_eq!(s.spawn_level_specials(), Err(refused));
    assert_eq!(s.spawn_map_things().map(|_| ()), Err(refused));
    assert_eq!(
        s.spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2]),
        Err(refused)
    );
    assert_eq!(s.snapshot(), after);
    // The light thinkers follow every actor; the store holds exactly them.
    let order: Vec<_> = s.shared_thinker_order().collect();
    let first = order.iter().position(|t| t.sector().is_some()).unwrap();
    assert!(order[..first].iter().all(|t| t.actor().is_some()));
    assert!(order[first..].iter().all(|t| t.sector().is_some()));
    assert_eq!(order.len() - first, 8);
    s.initialize_player_weapons(
        PlayerSlot::One,
        WeaponInventory {
            ready: Weapon::Pistol,
            owned: 3,
            ammo: [50, 0, 0, 0],
        },
    )
    .unwrap();
    s.step([TicCommand::default(); 4]).unwrap();
    assert_eq!(s.spawn_level_specials(), Err(refused));
    // Every other policy refuses the operation.
    let mut older =
        Simulation::original_player_real_map_map(crate::real_map_evidence::case_map(0), options())
            .unwrap();
    older.spawn_map_things().unwrap();
    assert_eq!(older.spawn_level_specials(), Err(refused));
    assert!(older.sector_lights_and_specials().is_none());
    assert!(older.original_player_secret_count().is_err());
}

#[test]
fn use_stays_rejected_with_state_unchanged() {
    let mut s = scenario(0);
    let before = s.snapshot();
    let mut c = [TicCommand::default(); 4];
    c[0].buttons = 2; // BT_USE
    assert_eq!(s.advance(c), Err(SimError::UnsupportedButtons));
    assert_eq!(s.snapshot(), before);
}

/// Points the last sector thinker of the shared order at a sector that does not
/// exist: its visit refuses with `StoreError::UnsupportedThinker`, after every
/// earlier thinker of that pass already ran.
fn break_last_light(s: &mut Simulation) {
    let last = s
        .actors
        .shared_order()
        .filter_map(|t| t.sector())
        .last()
        .unwrap();
    match s.actors.sector_get_mut(last).unwrap() {
        SectorThinker::Flash(t) => t.sector = u16::MAX,
        SectorThinker::Strobe(t) => t.sector = u16::MAX,
        SectorThinker::Glow(t) => t.sector = u16::MAX,
        SectorThinker::Flicker(t) => t.sector = u16::MAX,
    }
}

#[test]
fn refused_tic_mid_soak_keeps_sectors_lights_and_thinkers() {
    let mut s = scenario(0);
    for _ in 0..300 {
        s.advance([TicCommand::default(); 4]).unwrap();
    }
    let mut intact = s.clone();
    break_last_light(&mut s);
    let before = s.snapshot();
    let lights_before: Vec<_> = s.sector_lights_and_specials().unwrap().collect();
    let rng = s.rng();
    let error = SimError::Store(StoreError::UnsupportedThinker);
    assert_eq!(s.advance([TicCommand::default(); 4]), Err(error));
    assert_eq!(s.step([TicCommand::default(); 4]).map(|_| ()), Err(error));
    // The same tic without the fault changes the glow (every tic) and draws for
    // the flicker or flash; the refused one keeps everything.
    intact.advance([TicCommand::default(); 4]).unwrap();
    assert_ne!(
        intact
            .sector_lights_and_specials()
            .unwrap()
            .collect::<Vec<_>>(),
        lights_before
    );
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.rng(), rng);
    assert_eq!(s.tic(), 300);
    assert_eq!(
        s.sector_lights_and_specials().unwrap().collect::<Vec<_>>(),
        lights_before
    );
}

#[test]
fn refused_tic_keeps_an_unfound_secret() {
    // pools_secret enters sector 5 (special 9) in the step completing tic 30.
    let which = 4;
    let c = &specials_evidence::cases()[which];
    let (mut s, a, things) = specials_evidence::setup(c);
    for op in 1..30 {
        specials_evidence::controlled(&mut s, c, a, &things, op).unwrap();
        s.advance(specials_evidence::commands(c, op)).unwrap();
    }
    let mut intact = s.clone();
    break_last_light(&mut s);
    let before = s.snapshot();
    assert_eq!(
        s.advance(specials_evidence::commands(c, 30)),
        Err(SimError::Store(StoreError::UnsupportedThinker))
    );
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.original_player_secret_count(), Ok(0));
    assert_eq!(s.sector_lights_and_specials().unwrap().nth(5).unwrap().1, 9);
    intact.advance(specials_evidence::commands(c, 30)).unwrap();
    assert_eq!(intact.original_player_secret_count(), Ok(1));
    assert_eq!(
        intact
            .sector_lights_and_specials()
            .unwrap()
            .nth(5)
            .unwrap()
            .1,
        0
    );
}

#[test]
fn specials_policy_identity_is_distinct_from_the_real_map_policy() {
    let new = OriginalExecutionProfile::PlayerMonstersSpecialsV1;
    let old = OriginalExecutionProfile::PlayerMonstersRealMapV1;
    assert_eq!((new.wire_id(), old.wire_id()), (18, 17));
    assert_eq!(new.context_schema(), PLAYER_SPECIALS_CONTEXT_SCHEMA);
    assert_eq!(
        PLAYER_SPECIALS_CONTEXT_SCHEMA,
        b"PLAYER-MONSTERS-SPECIALS-1-MEDIUM-v1"
    );
    assert_ne!(new.context_schema(), old.context_schema());
    // A superset of the real-map predicates; only the new one has specials.
    assert!(new.has_specials() && !old.has_specials());
    for policy in [new, old] {
        assert!(policy.has_real_map() && policy.has_diagonals() && policy.has_heights());
        assert!(policy.has_imps() && policy.has_world_pickups() && policy.has_weapon_pack());
    }
    let mut s = scenario(0);
    assert_eq!(s.original_execution_profile(), Some(new));
    s.advance([TicCommand::default(); 4]).unwrap();
    let v34 = s
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    let real = crate::real_map_evidence::setup(&crate::real_map_evidence::cases()[0]).0;
    let v33 = real
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    let limits = DiagnosticLimits::default();
    assert_eq!(s.snapshot().version(), 34);
    assert_eq!(
        Diagnostic::decode_real_map(&v34, limits).map(|_| ()),
        Err(DiagnosticError::Version(34))
    );
    assert_eq!(
        Diagnostic::decode_specials(&v33, limits).map(|_| ()),
        Err(DiagnosticError::Version(33))
    );
    for reader in [
        Diagnostic::decode,
        Diagnostic::decode_player,
        Diagnostic::decode_diagonal,
        Diagnostic::decode_heights,
        Diagnostic::decode_bonuses,
    ] {
        assert_eq!(
            reader(&v34, limits).map(|_| ()),
            Err(DiagnosticError::Version(34))
        );
    }
    let view = Diagnostic::decode_specials(&v34, limits).unwrap();
    assert_eq!(view.field("profile"), Some(18u32.to_le_bytes().as_slice()));
    assert_eq!(view.encode(limits).unwrap(), v34);
    Diagnostic::decode_real_map(&v33, limits).unwrap();
}

#[test]
fn decode_specials_rejects_malformed_specials_fields() {
    let mut s = scenario(4);
    s.advance([TicCommand::default(); 4]).unwrap();
    let snapshot = s.snapshot();
    let limits = DiagnosticLimits::default();
    type Edit<'a> = &'a dyn Fn(&str, &[u8]) -> Option<Vec<u8>>;
    let encode = |edit: Edit<'_>| {
        let mut out = b"DSIM".to_vec();
        out.extend(34u32.to_le_bytes());
        let fields: Vec<_> = snapshot.fields().collect();
        out.extend((fields.len() as u64).to_le_bytes());
        for f in fields {
            let value = edit(f.path(), f.value()).unwrap_or_else(|| f.value().to_vec());
            out.extend((f.path().len() as u32).to_le_bytes());
            out.extend(f.path().as_bytes());
            out.extend((value.len() as u64).to_le_bytes());
            out.extend(value);
        }
        out
    };
    Diagnostic::decode_specials(&encode(&|_, _| None), limits).unwrap();
    let cases: [(&str, Vec<u8>); 7] = [
        ("sector_thinker.0.kind", vec![5]),
        ("sector_thinker.0.sector", 99u16.to_le_bytes().to_vec()),
        ("sector_thinker.0.occupied", vec![2]),
        ("sector_thinker.0.generation", 0u64.to_le_bytes().to_vec()),
        (
            "original_player.0.secret_count",
            2i32.to_le_bytes().to_vec(),
        ),
        ("sector.3.light", vec![0]),
        ("thinker.shared_order", vec![2; 13]),
    ];
    for (path, value) in cases {
        let bytes = encode(&|p, _| (p == path).then(|| value.clone()));
        assert!(
            Diagnostic::decode_specials(&bytes, limits).is_err(),
            "{path} accepted"
        );
    }
    // Shared order: a dropped sector entry, a duplicated one, reordered actors.
    let order = snapshot
        .fields()
        .find(|f| f.path() == "thinker.shared_order")
        .unwrap()
        .value()
        .to_vec();
    let entries: Vec<_> = order.chunks(13).map(<[u8]>::to_vec).collect();
    let mut dropped = entries.clone();
    dropped.pop();
    let mut duplicated = entries.clone();
    duplicated.push(entries.last().unwrap().clone());
    let mut swapped = entries.clone();
    swapped.swap(0, 1);
    for (name, edited) in [
        ("dropped", dropped),
        ("duplicated", duplicated),
        ("swapped", swapped),
    ] {
        let edited = edited.concat();
        let bytes = encode(&|p, _| (p == "thinker.shared_order").then(|| edited.clone()));
        assert!(
            Diagnostic::decode_specials(&bytes, limits).is_err(),
            "{name}"
        );
    }
}
