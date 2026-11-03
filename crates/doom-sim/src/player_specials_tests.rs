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
fn a_secret_is_found_on_landing_not_while_above_its_floor() {
    // A5, Rust-only: pools_secret's walk with the secret sector 5 sunk 48 units, so
    // the body enters it in the air ("Falling, not all the way down yet?",
    // p_spec.c:1007). No secret while its z is above the sector floor; one on landing.
    let which = 4;
    let c = &specials_evidence::cases()[which];
    let mut lumps = pools();
    let floor = i16::from_le_bytes(
        lump(&mut lumps, "SECTORS")[26 * 5..26 * 5 + 2]
            .try_into()
            .unwrap(),
    );
    set_sector_field(&mut lumps, 5, 0, floor - 48);
    let (mut s, a, things) = specials_evidence::setup_map(c, lumps.map());
    // P_PlayerInSpecialSector runs in the player phase, before the thinker pass
    // moves the body, so a tic tests the z the previous tic ended with.
    let resting = |s: &Simulation| {
        let body = s.actor(a.body).unwrap();
        (
            body.location.unwrap().sector == 5,
            body.z == s.sectors()[5].floor,
        )
    };
    let (mut airborne, mut found_at, mut events) = (0, None, 0);
    for op in 1..=c.tics {
        let (in_secret, on_floor) = resting(&s);
        specials_evidence::controlled(&mut s, c, a, &things, op).unwrap();
        s.advance(specials_evidence::commands(c, op)).unwrap();
        events += s
            .events()
            .iter()
            .filter(|e| matches!(e, Event::Special(SpecialEvent::Secret { .. })))
            .count();
        let count = s.original_player_secret_count().unwrap();
        let special = s.sector_lights_and_specials().unwrap().nth(5).unwrap().1;
        match found_at {
            None if count == 0 => {
                assert_eq!(special, 9, "op{op}");
                assert!(!(in_secret && on_floor), "on the floor, no secret, op{op}");
                airborne += usize::from(in_secret);
            }
            None => {
                assert!(in_secret && on_floor, "found off the floor, op{op}");
                assert_eq!((count, special), (1, 0), "op{op}");
                found_at = Some(op);
            }
            Some(_) => assert_eq!((count, special), (1, 0), "op{op}"),
        }
    }
    assert!(airborne > 0, "the body never entered the sector in the air");
    assert!(found_at.is_some());
    assert_eq!(events, 1);
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

/// DSIM34 bytes of `snapshot`, each field value replaced where `edit` returns one.
type FieldEdit<'a> = &'a dyn Fn(&str, &[u8]) -> Option<Vec<u8>>;
fn edited_bytes(snapshot: &Snapshot, edit: FieldEdit<'_>) -> Vec<u8> {
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
}
fn snapshot_value<'a>(snapshot: &'a Snapshot, path: &str) -> &'a [u8] {
    snapshot
        .fields()
        .find(|f| f.path() == path)
        .unwrap()
        .value()
}
/// The first snapshot after a tic of the committed scenario `label` whose
/// `events.last` holds a tag-25 event of `sub_tag`, with that event's index.
fn snapshot_with_special_event(label: &str, sub_tag: u8) -> (Snapshot, usize) {
    let c = specials_evidence::cases()
        .into_iter()
        .find(|c| c.label == label)
        .unwrap();
    let (mut s, a, things) = specials_evidence::setup(&c);
    for op in 1..=c.tics {
        specials_evidence::controlled(&mut s, &c, a, &things, op).unwrap();
        s.advance(specials_evidence::commands(&c, op)).unwrap();
        let snapshot = s.snapshot();
        let count = u64::from_le_bytes(
            snapshot_value(&snapshot, "events.last.count")
                .try_into()
                .unwrap(),
        );
        let found = (0..count as usize).find(|i| {
            snapshot_value(&snapshot, &format!("events.last.{i}")).get(..2) == Some(&[25, sub_tag])
        });
        if let Some(i) = found {
            return (snapshot, i);
        }
    }
    panic!("{label}: no tag-25 sub-tag {sub_tag} event");
}

#[test]
fn decode_specials_cross_checks_thinkers_specials_and_sky_events_against_the_map() {
    // Rust-only safety test: each mutation below breaks one property every DSIM34
    // stream of this policy has; the unmutated streams decode.
    let limits = DiagnosticLimits::default();
    let mut s = scenario(4);
    s.advance([TicCommand::default(); 4]).unwrap();
    let snapshot = s.snapshot();
    Diagnostic::decode_specials(&edited_bytes(&snapshot, &|_, _| None), limits).unwrap();
    let map = snapshot_value(&snapshot, "map.sectors").to_vec();
    let map_special = |i: usize| i16::from_le_bytes([map[26 * i + 22], map[26 * i + 23]]);
    let sectors = map.len() / 26;
    let light = |special: i16| matches!(special, 1 | 2 | 3 | 4 | 8 | 12 | 13 | 17);
    let find = |want: &dyn Fn(i16) -> bool| (0..sectors).find(|&i| want(map_special(i))).unwrap();
    let thinker = |j: usize, name: &str| {
        snapshot_value(&snapshot, &format!("sector_thinker.{j}.{name}")).to_vec()
    };
    let slots = u64::from_le_bytes(
        snapshot_value(&snapshot, "sector_thinkers.slot_count")
            .try_into()
            .unwrap(),
    ) as usize;
    let strobes: Vec<_> = (0..slots).filter(|&j| thinker(j, "kind") == [2]).collect();
    assert!(strobes.len() >= 2);
    let flash = find(&|v| v == 1);
    let flicker_damage = find(&|v| v == 4);
    let plain = find(&|v| v == 0);
    let damage = find(&|v| matches!(v, 5 | 7 | 16));
    let mut extra_light = map.clone();
    extra_light[26 * plain + 22..26 * plain + 24].copy_from_slice(&17i16.to_le_bytes());
    let short_map = map[..map.len() - 26].to_vec();
    let path = |name: String| name;
    let cases: Vec<(&str, String, Vec<u8>)> = vec![
        // Two thinkers of one sector (same kind, so only the duplicate is wrong).
        (
            "duplicate sector",
            path(format!("sector_thinker.{}.sector", strobes[1])),
            thinker(strobes[0], "sector"),
        ),
        // A thinker of a sector whose map special spawns no thinker.
        (
            "non-light sector",
            path("sector_thinker.0.sector".into()),
            (plain as u16).to_le_bytes().to_vec(),
        ),
        // One more light sector in the map than thinkers in the store.
        ("light count", path("map.sectors".into()), extra_light),
        ("map sector count", path("map.sectors".into()), short_map),
        // A light sector keeps its special after the spawners cleared it.
        (
            "flash special",
            path(format!("sector.{flash}.special")),
            1i16.to_le_bytes().to_vec(),
        ),
        // Special 4 is set back to 4 by its spawner.
        (
            "special 4 cleared",
            path(format!("sector.{flicker_damage}.special")),
            0i16.to_le_bytes().to_vec(),
        ),
        // Only a secret clears a non-light special.
        (
            "damage special cleared",
            path(format!("sector.{damage}.special")),
            0i16.to_le_bytes().to_vec(),
        ),
    ];
    assert!(!light(map_special(plain)) && !light(map_special(damage)));
    let mut accepted = Vec::new();
    for (name, target, value) in &cases {
        let bytes = edited_bytes(&snapshot, &|p, _| (p == target).then(|| value.clone()));
        if Diagnostic::decode_specials(&bytes, limits).is_ok() {
            accepted.push(*name);
        }
    }
    // Before the load operation the store is empty, which only tic 0 allows.
    let mut pre = specials(&pools()).unwrap();
    pre.spawn_map_things().unwrap();
    pre.spawn_player_body(PlayerSlot::One, [768, -200].map(Fixed::from_map_units))
        .unwrap();
    let before_load = pre.snapshot();
    Diagnostic::decode_specials(&edited_bytes(&before_load, &|_, _| None), limits).unwrap();
    let bytes = edited_bytes(&before_load, &|p, _| {
        (p == "tic").then(|| 1u64.to_le_bytes().to_vec())
    });
    if Diagnostic::decode_specials(&bytes, limits).is_ok() {
        accepted.push("empty store after tic 0");
    }
    // Sky events: the line exists; a sky removal names an occupied fireball.
    let lines_of =
        |snapshot: &Snapshot| (snapshot_value(snapshot, "map.linedefs").len() / 14) as u16;
    let (removal, i) = snapshot_with_special_event("sky_hack_wall", 1);
    let (shot, k) = snapshot_with_special_event("sky_shot_high", 2);
    let removal_event = format!("events.last.{i}");
    let shot_event = format!("events.last.{k}");
    for (snapshot, path) in [(&removal, &removal_event), (&shot, &shot_event)] {
        Diagnostic::decode_specials(&edited_bytes(snapshot, &|_, _| None), limits).unwrap();
        let event = snapshot_value(snapshot, path).to_vec();
        let mut bad_line = event.clone();
        let at = if event[1] == 1 { 14 } else { 3 };
        bad_line[at..at + 2].copy_from_slice(&lines_of(snapshot).to_le_bytes());
        let bytes = edited_bytes(snapshot, &|p, _| (p == path).then(|| bad_line.clone()));
        if Diagnostic::decode_specials(&bytes, limits).is_ok() {
            accepted.push(if event[1] == 1 {
                "removal line"
            } else {
                "shot line"
            });
        }
    }
    let event = snapshot_value(&removal, &removal_event).to_vec();
    let slot = u32::from_le_bytes(event[2..6].try_into().unwrap());
    let mut stale = event.clone();
    let generation = u64::from_le_bytes(event[6..14].try_into().unwrap());
    stale[6..14].copy_from_slice(&(generation + 1).to_le_bytes());
    let bytes = edited_bytes(&removal, &|p, _| {
        (p == removal_event).then(|| stale.clone())
    });
    if Diagnostic::decode_specials(&bytes, limits).is_ok() {
        accepted.push("removal generation");
    }
    // The event names another occupied actor (a map monster, not a fireball).
    let other = (0..)
        .find(|j| *j != slot && snapshot_value(&removal, &format!("actor.{j}.occupied")) == [1])
        .unwrap();
    let mut monster = event.clone();
    monster[2..6].copy_from_slice(&other.to_le_bytes());
    monster[6..14].copy_from_slice(snapshot_value(
        &removal,
        &format!("actor.{other}.generation"),
    ));
    let bytes = edited_bytes(&removal, &|p, _| {
        (p == removal_event).then(|| monster.clone())
    });
    if Diagnostic::decode_specials(&bytes, limits).is_ok() {
        accepted.push("removal of a non-fireball");
    }
    assert!(accepted.is_empty(), "accepted: {accepted:?}");
}

/// The M3h3 scripted command sequence over a seed: runs, strafes, turns and fires.
fn scripted(seed: u64, op: usize) -> [TicCommand; 4] {
    let mut x = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(op as u64);
    x ^= x >> 29;
    let mut c = [TicCommand::default(); 4];
    c[0] = TicCommand {
        forward: if x & 3 == 0 { 0 } else { 50 },
        side: match (x >> 2) & 3 {
            0 => -40,
            1 => 40,
            _ => 0,
        },
        angle_turn: match (x >> 4) & 7 {
            0 => 1024,
            1 => -1024,
            2 => 4096,
            _ => 0,
        },
        buttons: u8::from((x >> 7) & 7 == 0),
    };
    c
}
/// Invariants Ω (M3h5) and V (M3h4) at a tic boundary, unchanged by the specials
/// policy (no height changes in Stage A): every live actor's center is in the
/// vertex rectangle; every XY mover that is not an impact or an item is inside the
/// one-sided boundary with its open box meeting no wall; every actor that is not
/// an impact satisfies `floor <= z <= ceiling - height`.
fn assert_wall_and_vertical_invariants(s: &Simulation, op: usize) {
    let boundary = s.boundary.as_deref().unwrap();
    let room = s.room().unwrap();
    for id in s.actor_order() {
        let a = s.actor(id).unwrap();
        let Some(l) = a.life() else { continue };
        assert!(
            room.contains(a.position()),
            "tic {op} type {}",
            l.actor_type.0
        );
        if original_registry::is_impact(l.actor_type) {
            continue;
        }
        assert!(
            a.floor <= a.z && a.z + a.height() <= a.ceiling,
            "tic {op} type {} z {} floor {} ceiling {}",
            l.actor_type.0,
            a.z.bits(),
            a.floor.bits(),
            a.ceiling.bits()
        );
        if l.flags & 1 != 0 {
            continue;
        }
        assert!(
            boundary.inside(a.position()),
            "tic {op} type {} at {:?} outside",
            l.actor_type.0,
            a.position().map(Fixed::bits)
        );
        assert!(
            !boundary.box_meets_wall(a.position(), a.radius() - Fixed::from_bits(1)),
            "tic {op} type {} at {:?} meets a wall",
            l.actor_type.0,
            a.position().map(Fixed::bits)
        );
    }
}
/// Every light thinker keeps its sector's level inside `[minlight, maxlight]`
/// (ordered, since a fire flicker's `minlight + 16` may exceed its `maxlight`);
/// a sector has at most one light thinker; every sector without one keeps its
/// map light; the sector thinkers are one contiguous run after the load actors
/// and before every later actor (A1); `secretcount <= totalsecret` (A5).
fn assert_specials_invariants(s: &Simulation, load: &[ActorId], op: usize) {
    let lights: Vec<_> = s.sector_lights_and_specials().unwrap().collect();
    let mut owned = vec![false; lights.len()];
    let mut phase = 0;
    for entry in s.shared_thinker_order() {
        match entry {
            ThinkerId::Actor(id) => {
                let at_load = load.contains(&id);
                if phase == 0 && !at_load {
                    phase = 2;
                }
                if phase == 1 {
                    phase = 2;
                }
                assert!(phase == 0 || !at_load, "tic {op}: load actor after a light");
            }
            ThinkerId::Sector(id) => {
                assert!(phase < 2, "tic {op}: light after a later actor");
                phase = 1;
                let t = s.sector_thinker(id).unwrap();
                let (lo, hi) = match *t {
                    SectorThinker::Flash(t) => (t.minlight, t.maxlight),
                    SectorThinker::Strobe(t) => (t.minlight, t.maxlight),
                    SectorThinker::Glow(t) => (t.minlight, t.maxlight),
                    SectorThinker::Flicker(t) => (t.minlight, t.maxlight),
                };
                let sector = usize::from(t.sector());
                assert!(!owned[sector], "tic {op}: two thinkers on sector {sector}");
                owned[sector] = true;
                let light = i32::from(lights[sector].0);
                assert!(
                    lo.min(hi) <= light && light <= lo.max(hi),
                    "tic {op} sector {sector} light {light} outside [{lo}, {hi}]"
                );
            }
        }
    }
    for (i, (light, _)) in lights.iter().enumerate() {
        if !owned[i] {
            assert_eq!(*light, s.world.map().sectors()[i].light, "tic {op} {i}");
        }
    }
    let secrets = s.original_player_secret_count().unwrap();
    assert!(0 <= secrets && secrets <= s.level_statistics().unwrap().total_secrets);
}
/// 1,200 scripted tics in a committed room with its roster; returns the refused
/// tics (only the two legal source witnesses may refuse, keeping the tic).
fn specials_soak(label: &str, seed: u64) -> usize {
    let cases = specials_evidence::cases();
    let c = cases.iter().find(|c| c.label == label).unwrap();
    let (mut s, a, things) = specials_evidence::setup(c);
    let mut load = things.clone();
    load.push(a.body);
    assert_specials_invariants(&s, &load, 0);
    let mut refused = 0;
    for op in 1..=1200 {
        let before = s.tic();
        let live = s.original_player(PlayerSlot::One).unwrap().state == PlayerBodyState::Live;
        let commands = if live {
            scripted(seed, op)
        } else {
            [TicCommand::default(); 4]
        };
        match s.advance(commands) {
            Ok(()) => assert_eq!(s.tic(), before + 1),
            Err(SimError::MapScale(MapScaleError::Escape | MapScaleError::Intercepts)) => {
                refused += 1;
                assert_eq!(s.tic(), before);
            }
            Err(e) => panic!("{label} seed {seed} tic {op}: {e}"),
        }
        assert_wall_and_vertical_invariants(&s, op);
        assert_specials_invariants(&s, &load, op);
    }
    refused
}
#[test]
fn pools_soak_keeps_the_invariants_and_light_envelopes() {
    let mut refused = 0;
    for seed in [1, 2, 3, 4] {
        refused += specials_soak("pools_light_and_fireball", seed);
    }
    assert_eq!(refused, 0);
}
#[test]
fn sky_soak_keeps_the_invariants_under_the_sky() {
    let mut refused = 0;
    for seed in [1, 2, 3, 4] {
        refused += specials_soak("sky_hack_wall", seed);
    }
    assert_eq!(refused, 0);
}

/// Replays the committed `sky_shot_window` pistol script with the sill S (sector 3,
/// sky, map 7) lowered to `ceiling`; returns (sky shots, puffs spawned).
fn window_shots(ceiling: i16) -> (Vec<(u8, u16, Fixed)>, usize) {
    let cases = specials_evidence::cases();
    let c = cases.iter().find(|c| c.label == "sky_shot_window").unwrap();
    let mut lumps = Lumps::parse(specials_evidence::MAP_BYTES[c.map]);
    set_sector_field(&mut lumps, 3, 2, ceiling);
    let (mut s, a, things) = specials_evidence::setup_map(c, lumps.map());
    let (mut shots, mut puffs) = (Vec::new(), 0);
    for op in 1..=c.tics {
        specials_evidence::controlled(&mut s, c, a, &things, op).unwrap();
        s.advance(specials_evidence::commands(c, op)).unwrap();
        for event in s.events() {
            match *event {
                Event::Special(SpecialEvent::SkyShot { reason, line, z }) => {
                    shots.push((reason, line, z));
                }
                Event::Spawned(id) => {
                    let ty = s.actor(id).and_then(|a| a.life()).map(|l| l.actor_type);
                    puffs += usize::from(
                        ty.map(|t| t.0)
                            == Some(crate::identity::OriginalActorType::MT_PUFF.ordinal()),
                    );
                }
                _ => {}
            }
        }
    }
    (shots, puffs)
}
#[test]
fn sky_shot_at_exactly_the_front_ceiling_is_not_sky_ended() {
    // p_map.c:960 `if (z > li->frontsector->ceilingheight) return false;` is strict.
    // The pistol's would-be puff z is the body's z + 28 + 8 = 36 (slope 0, no
    // target). With the sky sill at ceiling 36 the court-to-sill line's opening
    // top equals z, so the shot passes it and stops at the sill-to-window line
    // (front: the sill, sky; back: the window, not sky) with z == front ceiling:
    // not sky-ended, a puff. One unit lower and the court-to-sill line (front: the
    // sill) stops the shot with z above its sky ceiling: sky-ended, no puff.
    let (shots, puffs) = window_shots(36);
    assert!(shots.is_empty(), "{shots:?}");
    assert!(puffs > 0);
    let (shots, puffs) = window_shots(35);
    assert!(!shots.is_empty());
    assert_eq!(puffs, 0);
    for (reason, _, z) in shots {
        assert_eq!((reason, z), (0, Fixed::from_map_units(36)));
    }
}

#[test]
fn sector_thinker_slots_and_visit_budget_follow_the_light_sectors() {
    // A7: the slot class holds exactly the light sectors (specials 1, 2, 3, 4, 8,
    // 12, 13, 17); the visit budget is the 320 actor slots plus that count.
    let s = scenario(0);
    assert_eq!(s.actors.sector_capacity(), 8);
    assert_eq!(s.actors.visit_bound(), 320 + 8);
    assert_eq!(
        s.shared_thinker_order()
            .filter(|t| t.sector().is_some())
            .count(),
        8
    );
    // The store is full: one more sector thinker is refused explicitly.
    let mut full = s.clone();
    let extra = *full
        .sector_thinker(
            full.shared_thinker_order()
                .find_map(|t| t.sector())
                .unwrap(),
        )
        .unwrap();
    assert!(full.actors.spawn_sector(extra).is_err());
    // The sky room has no light sector: capacity zero.
    let sky = scenario(7);
    assert_eq!(sky.actors.sector_capacity(), 0);
    assert_eq!(sky.actors.visit_bound(), 320);
    // A ninth light sector (the plain corridor 13 as special 17) raises both.
    let mut lumps = pools();
    set_sector_field(&mut lumps, 13, 22, 17);
    let mut s = specials(&lumps).unwrap();
    assert_eq!(s.actors.sector_capacity(), 9);
    assert_eq!(s.actors.visit_bound(), 329);
    s.spawn_map_things().unwrap();
    s.spawn_player_body(PlayerSlot::One, [768, -200].map(Fixed::from_map_units))
        .unwrap();
    s.spawn_level_specials().unwrap();
    assert_eq!(
        s.shared_thinker_order()
            .filter(|t| t.sector().is_some())
            .count(),
        9
    );
}
#[test]
fn every_scenario_keeps_the_specials_invariants_after_every_tic() {
    // The committed scripts (the secret found, damage, death, fireballs and sky
    // removals) under the same per-tic checks as the soaks.
    for c in specials_evidence::cases() {
        let (mut s, a, things) = specials_evidence::setup(&c);
        let mut load = things.clone();
        load.push(a.body);
        for op in 1..=c.tics {
            specials_evidence::controlled(&mut s, &c, a, &things, op).unwrap();
            s.advance(specials_evidence::commands(&c, op)).unwrap();
            assert_wall_and_vertical_invariants(&s, op);
            assert_specials_invariants(&s, &load, op);
        }
        let found = s.original_player_secret_count().unwrap();
        assert_eq!(found, i32::from(c.label == "pools_secret"), "{}", c.label);
    }
}
