//! Versioned state, boundary policies, topology bounds and long XY/Z regressions.
#[path = "../../../tests/support/m3b.rs"]
mod fixtures;
use doom_sim::{
    ActorId, ActorSpec, Event, GeometryError, Gravity, MoveFailure, Profile, SimError, Simulation,
    Snapshot, Solidity, SpawnHeight, StoreError, TicCommand, VerticalSpec, compare_traces,
};
use doom_types::Fixed;
fn empty() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}
fn spec(x: i16, y: i16) -> ActorSpec {
    ActorSpec {
        position: [Fixed::from_map_units(x), Fixed::from_map_units(y)],
        ..Default::default()
    }
}
fn spawn(sim: &mut Simulation, actor: ActorSpec, z: Fixed, dropoff: bool) -> ActorId {
    sim.spawn_vertical(
        actor,
        Solidity::Solid,
        VerticalSpec {
            height: SpawnHeight::Absolute(z),
            allow_dropoff: dropoff,
            ..Default::default()
        },
    )
    .unwrap()
}
fn probe(map: usize, z: Fixed, dropoff: bool) -> MoveFailure {
    let mut sim = Simulation::multisector(fixtures::map(map)).unwrap();
    let id = spawn(&mut sim, spec(-16, 0), z, dropoff);
    sim.check_multisector_position(id, [Fixed::from_map_units(-4), Fixed::ZERO])
        .unwrap()
        .failure
}
#[test]
fn exact_step_headroom_and_dropoff_thresholds_are_source_strict() {
    assert_eq!(probe(2, Fixed::ZERO, false), MoveFailure::None);
    assert_eq!(probe(2, Fixed::from_bits(-1), false), MoveFailure::Step);
    assert_eq!(probe(3, Fixed::ZERO, false), MoveFailure::Step);
    assert_eq!(probe(5, Fixed::ZERO, false), MoveFailure::None);
    assert_eq!(probe(5, Fixed::from_bits(1), false), MoveFailure::Headroom);
    assert_eq!(probe(4, Fixed::ZERO, false), MoveFailure::Fit);
    assert_eq!(probe(7, Fixed::ZERO, false), MoveFailure::None);
    assert_eq!(probe(8, Fixed::ZERO, false), MoveFailure::Dropoff);
    assert_eq!(probe(8, Fixed::ZERO, true), MoveFailure::None);
    assert_eq!(probe(13, Fixed::ZERO, true), MoveFailure::Fit);
}
#[test]
fn gravity_initial_impulse_and_no_gravity_ceiling_cases_are_explicit() {
    let cases = fixtures::cases();
    let (mut sim, id, _) = cases[14].setup(true);
    for (z, momz) in [
        (32, -2),
        (30, -3),
        (27, -4),
        (23, -5),
        (18, -6),
        (12, -7),
        (5, -8),
        (0, 0),
    ] {
        sim.step(empty()).unwrap();
        let a = sim.actor(id).unwrap();
        assert_eq!(
            [a.z(), a.vertical_momentum()],
            [Fixed::from_map_units(z), Fixed::from_map_units(momz)]
        );
    }
    let (mut sim, id, _) = cases[15].setup(true);
    sim.step(empty()).unwrap();
    assert_eq!(sim.actor(id).unwrap().z(), Fixed::from_map_units(72));
    assert_eq!(sim.actor(id).unwrap().vertical_momentum(), Fixed::ZERO);
    assert!(
        sim.events()
            .iter()
            .any(|e| matches!(e,Event::Vertical {update,..} if update.ceiling_contact))
    );
    let (mut sim, id, _) = cases[17].setup(true);
    let mut command = empty();
    command[0] = TicCommand {
        forward: 127,
        side: -128,
        angle_turn: 1,
        buttons: 0,
    };
    for _ in 0..16 {
        sim.step(command).unwrap();
    }
    assert_eq!(sim.actor(id).unwrap().z(), Fixed::from_map_units(32));
    assert_eq!(sim.actor(id).unwrap().momentum(), [Fixed::ZERO; 2]);
    assert_eq!(sim.actor(id).unwrap().angle(), 16 << 16);
}
#[test]
fn contacted_heights_and_center_sector_have_distinct_meanings() {
    let mut sim = Simulation::multisector(fixtures::map(1)).unwrap();
    let id = spawn(&mut sim, spec(-16, 0), Fixed::ZERO, true);
    sim.bind_player(0, id).unwrap();
    sim.set_momentum(id, [Fixed::from_map_units(10), Fixed::ZERO])
        .unwrap();
    sim.step(empty()).unwrap();
    let a = sim.actor(id).unwrap();
    assert_eq!(a.location().unwrap().sector, 0);
    assert_eq!(a.constraints()[0], Fixed::from_map_units(8));
    assert_eq!(a.z(), Fixed::from_map_units(8));
    assert_eq!(sim.sectors()[0].floor, Fixed::ZERO);
    sim.step(empty()).unwrap();
    assert_eq!(sim.actor(id).unwrap().location().unwrap().sector, 1);
    assert_eq!(sim.sector_actor_order(1).unwrap(), [id]);
}
#[test]
fn reference_lists_accept_sparse_diagonals_and_expose_missing_collision_data() {
    let full = Simulation::multisector(fixtures::map(10)).unwrap();
    let sparse = Simulation::multisector(fixtures::map(11)).unwrap();
    assert_eq!(full.world().conservative_omissions(), Some(0));
    assert!(sparse.world().conservative_omissions().unwrap() > 0);
    let mut full = Simulation::multisector(fixtures::map(0)).unwrap();
    let mut missing = Simulation::multisector(fixtures::map(15)).unwrap();
    let a = spawn(&mut full, spec(120, 0), Fixed::ZERO, true);
    let b = spawn(&mut missing, spec(120, 0), Fixed::ZERO, true);
    let target = [Fixed::from_map_units(124), Fixed::ZERO];
    assert_eq!(
        full.check_multisector_position(a, target).unwrap().failure,
        MoveFailure::Line
    );
    assert!(
        missing
            .check_multisector_position(b, target)
            .unwrap()
            .allowed()
    );
    assert!(missing.world().conservative_omissions().unwrap() > 0);
    assert_eq!(missing.profile(), Profile::MultiSector);
}
#[test]
fn sector_head_relinks_immediate_removal_and_reuse_are_generation_safe() {
    let mut sim = Simulation::multisector(fixtures::map(0)).unwrap();
    let a = sim.spawn(spec(-64, 0)).unwrap();
    let b = sim.spawn(spec(-64, 0)).unwrap();
    let c = sim.spawn(spec(64, 0)).unwrap();
    assert_eq!(sim.sector_actor_order(0).unwrap(), [b, a]);
    assert_eq!(sim.sector_actor_order(1).unwrap(), [c]);
    sim.set_momentum(a, [Fixed::ONE, Fixed::ZERO]).unwrap();
    sim.step(empty()).unwrap();
    assert_eq!(sim.sector_actor_order(0).unwrap(), [a, b]);
    sim.mark_remove(a).unwrap();
    assert!(sim.actor(a).is_some());
    assert_eq!(sim.sector_actor_order(0).unwrap(), [b]);
    sim.step(empty()).unwrap();
    let reused = sim.spawn(spec(32, 0)).unwrap();
    assert_eq!(reused.slot(), a.slot());
    assert_ne!(reused.generation(), a.generation());
    assert_eq!(sim.sector_actor_order(1).unwrap(), [reused, c]);
    assert_eq!(sim.actor_order().collect::<Vec<_>>(), [b, c, reused]);
    assert_eq!(
        sim.set_vertical_momentum(a, Fixed::ONE),
        Err(SimError::Store(StoreError::InvalidHandle))
    );
}
#[test]
fn setup_and_step_errors_preserve_rng_pending_events_and_all_v3_state() {
    let mut sim = Simulation::with_multisector_limit(fixtures::map(0), 1).unwrap();
    let id = spawn(&mut sim, spec(-64, 0), Fixed::from_map_units(32), true);
    sim.bind_player(0, id).unwrap();
    sim.gameplay_random();
    let before = sim.snapshot();
    assert!(
        sim.world()
            .check_position([Fixed::ZERO; 2], Fixed::ONE, Fixed::ONE)
            .is_err()
    );
    assert!(sim.world().validate_actor(Fixed::ONE, Fixed::ONE).is_err());
    assert_eq!(
        sim.spawn(spec(0, 0)),
        Err(SimError::Store(StoreError::Capacity))
    );
    assert_eq!(sim.snapshot(), before);
    assert!(
        sim.spawn_vertical(
            ActorSpec {
                radius: Fixed::MAX,
                ..spec(0, 0)
            },
            Solidity::Solid,
            VerticalSpec::default()
        )
        .is_err()
    );
    assert_eq!(sim.snapshot(), before);
    assert_eq!(sim.bind_player(1, id), Err(SimError::InvalidPlayer));
    assert_eq!(sim.snapshot(), before);
    assert!(sim.line_opening(usize::MAX).is_err());
    assert!(sim.sector_actor_order(u16::MAX).is_err());
    assert_eq!(sim.snapshot(), before);
    let mut command = empty();
    command[2].buttons = 255;
    assert_eq!(sim.step(command), Err(SimError::UnsupportedButtons));
    assert_eq!(sim.snapshot(), before);
    sim.set_paused(true);
    let paused = sim.snapshot();
    assert_eq!(sim.step(command).unwrap(), paused);
    sim.set_paused(false);
    sim.step(empty()).unwrap();
    assert_eq!(sim.rng().indices(), [2, 0]);
    let mut old = Simulation::player_collision({
        let wad = doom_wad::Wad::parse(
            include_bytes!("../../../fuzz/corpus/map/classic_square"),
            doom_wad::Limits::default(),
        )
        .unwrap();
        doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
    })
    .unwrap();
    let before = old.snapshot();
    assert!(
        old.spawn_vertical(spec(0, 0), Solidity::NonSolid, VerticalSpec::default())
            .is_err()
    );
    assert_eq!(old.snapshot(), before);
}
#[test]
fn malformed_topology_and_unsupported_spans_reject_before_movement() {
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[3].1, 28, i16::MAX);
    assert!(fixtures::decode(&lumps).is_err());
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[7].1, 24, 0);
    assert!(fixtures::decode(&lumps).is_err());
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[2].1, 6 * 14 + 4, 0);
    assert!(matches!(
        Simulation::multisector(fixtures::decode(&lumps).unwrap()),
        Err(SimError::Geometry(GeometryError::Unsupported(
            "M3b inconsistent two-sided flags"
        )))
    ));
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[8].1, 0, 128);
    fixtures::set_word(&mut lumps[8].1, 2, 0);
    assert!(matches!(
        Simulation::multisector(fixtures::decode(&lumps).unwrap()),
        Err(SimError::Geometry(GeometryError::Unsupported(
            "M3b inverted sector heights"
        )))
    ));
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[8].1, 0, i16::MIN);
    fixtures::set_word(&mut lumps[8].1, 28, i16::MAX);
    assert!(matches!(
        Simulation::multisector(fixtures::decode(&lumps).unwrap()),
        Err(SimError::Geometry(GeometryError::CoordinateOverflow))
    ));
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[2].1, 2, 0);
    assert!(matches!(
        Simulation::multisector(fixtures::decode(&lumps).unwrap()),
        Err(SimError::Geometry(GeometryError::Unsupported(
            "M3b degenerate line"
        )))
    ));
    let mut lumps = fixtures::lumps(0);
    let sector = lumps[8].1[..26].to_vec();
    for _ in 0..127 {
        lumps[8].1.extend(&sector);
    }
    lumps[9].1.resize((129_usize * 129).div_ceil(8), 0);
    assert!(matches!(
        Simulation::multisector(fixtures::decode(&lumps).unwrap()),
        Err(SimError::Geometry(GeometryError::Unsupported(
            "M3b topology work limit"
        )))
    ));
}
#[test]
fn deep_bsp_is_iterative_bounded_and_keeps_original_child_order() {
    let mut lumps = fixtures::lumps(0);
    let template = lumps[7].1.clone();
    lumps[7].1.clear();
    for i in 0..4096_u16 {
        let mut node = template.clone();
        node[24..26].copy_from_slice(&(if i == 0 { 0x8001 } else { i - 1 }).to_le_bytes());
        lumps[7].1.extend(node);
    }
    let sim = Simulation::multisector(fixtures::decode(&lumps).unwrap()).unwrap();
    assert_eq!(
        sim.world()
            .locate([Fixed::from_map_units(64), Fixed::ZERO])
            .unwrap()
            .sector,
        1
    );
    assert_eq!(
        sim.world()
            .locate([Fixed::from_map_units(-64), Fixed::ZERO])
            .unwrap()
            .sector,
        0
    );
    let mut node = template.clone();
    node[24..26].copy_from_slice(&4095_u16.to_le_bytes());
    lumps[7].1.extend(node);
    assert!(matches!(
        Simulation::multisector(fixtures::decode(&lumps).unwrap()),
        Err(SimError::Geometry(GeometryError::Unsupported(
            "M3b topology work limit"
        )))
    ));
}
fn check_binary_fields(snapshot: &Snapshot, expected: &[u8]) {
    assert_eq!(&expected[..8], b"DSIM\x03\0\0\0");
    let count = u64::from_le_bytes(expected[8..16].try_into().unwrap()) as usize;
    let mut offset = 16;
    let fields = snapshot.fields().collect::<Vec<_>>();
    assert_eq!(count, fields.len());
    for actual in fields {
        let size = u32::from_le_bytes(expected[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        let path = std::str::from_utf8(&expected[offset..offset + size]).unwrap();
        offset += size;
        let size = u64::from_le_bytes(expected[offset..offset + 8].try_into().unwrap()) as usize;
        offset += 8;
        assert_eq!(actual.path(), path);
        assert_eq!(
            actual.value(),
            &expected[offset..offset + size],
            "independent DSIM-v3 tic {} field {path}",
            snapshot.tic()
        );
        offset += size;
    }
    assert_eq!(offset, expected.len());
    assert_eq!(snapshot.canonical_bytes(), expected);
}
#[test]
fn independent_v3_goldens_cover_steps_membership_drop_air_and_landing() {
    let cases = fixtures::cases();
    let (mut sim, _, _) = cases[2].setup(true);
    let mut current = usize::MAX;
    for row in include_str!("../../../tests/reference/m3b_snapshots.csv")
        .lines()
        .skip(1)
    {
        let c: Vec<_> = row.split(',').collect();
        let case = c[0].parse::<usize>().unwrap();
        if case != current {
            (sim, _, _) = cases[case].setup(true);
            current = case;
        }
        let snapshot = sim.step(cases[case].commands()).unwrap();
        assert_eq!(snapshot.version(), 3);
        assert_eq!(snapshot.tic(), c[1].parse::<u64>().unwrap());
        assert_eq!(
            snapshot.digest(),
            u64::from_str_radix(c[2], 16).unwrap(),
            "C-derived independent DSIM-v3 case {case} tic {}",
            snapshot.tic()
        );
        let binary: &[u8] = match c[6] {
            "-" => continue,
            "m3b_fall_first.bin" => include_bytes!("../../../tests/reference/m3b_fall_first.bin"),
            "m3b_step.bin" => include_bytes!("../../../tests/reference/m3b_step.bin"),
            "m3b_transition.bin" => include_bytes!("../../../tests/reference/m3b_transition.bin"),
            "m3b_drop_air.bin" => include_bytes!("../../../tests/reference/m3b_drop_air.bin"),
            "m3b_landing.bin" => include_bytes!("../../../tests/reference/m3b_landing.bin"),
            _ => panic!("unknown original synthetic golden"),
        };
        check_binary_fields(&snapshot, binary);
    }
}
fn long_trace(presentation: bool, perturb: bool) -> Vec<Snapshot> {
    let mut sim = Simulation::multisector(fixtures::map(12)).unwrap();
    let player = spawn(&mut sim, spec(-64, 0), Fixed::from_map_units(64), true);
    sim.bind_player(0, player).unwrap();
    let mut other = spawn(&mut sim, spec(48, 0), Fixed::from_map_units(24), true);
    let mut trace = Vec::new();
    let mut sectors = [false; 3];
    let (mut falls, mut contacts) = (0, 0);
    for tic in 0..768 {
        if tic % 97 == 20 {
            sim.mark_remove(other).unwrap();
        }
        if tic % 97 == 50 {
            other = spawn(&mut sim, spec(48, 0), Fixed::from_map_units(24), true);
        }
        sim.set_paused(tic == 300);
        if !sim.paused() {
            sim.gameplay_random();
        }
        if perturb && tic == 400 {
            sim.gameplay_random();
        }
        if presentation {
            for _ in 0..tic % 5 {
                let _ = sim.snapshot().digest();
                let _ = sim.events();
                let _ = sim.sector_actor_order(0).unwrap();
                let mut rng = sim.rng();
                rng.p_random();
            }
        }
        let mut commands = empty();
        commands[0] = TicCommand {
            forward: 50,
            angle_turn: if tic % 128 == 64 || tic % 128 == 0 && tic != 0 {
                -32768
            } else {
                0
            },
            ..Default::default()
        };
        trace.push(sim.step(commands).unwrap());
        sectors[usize::from(sim.actor(player).unwrap().location().unwrap().sector)] = true;
        for event in sim.events() {
            if let Event::Vertical { update, .. } = event {
                falls += usize::from(update.gravity_applied);
                contacts += usize::from(update.floor_contact);
            }
        }
    }
    assert!(
        sectors.into_iter().all(|seen| seen),
        "visited sectors: {sectors:?}"
    );
    assert!(falls > 0 && contacts > 0);
    assert_eq!(sim.tic(), 767);
    trace
}
#[test]
fn long_multisector_gravity_lifecycle_pause_and_rng_traces_are_deterministic() {
    let expected = long_trace(false, false);
    let actual = long_trace(true, false);
    assert_eq!(compare_traces(&expected, &actual), None);
    assert_eq!(expected, actual);
    for (a, b) in expected.iter().zip(&actual) {
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    }
    let actual = long_trace(false, true);
    let diff = compare_traces(&expected, &actual).unwrap();
    assert_eq!((diff.tic, diff.field.as_str()), (400, "rng.gameplay"));
}
#[test]
fn bounded_raw_extremes_and_generated_static_worlds_do_not_panic_or_drift() {
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for (i, x) in extremes.into_iter().enumerate() {
        let mut a = Simulation::multisector(fixtures::map(0)).unwrap();
        let id = a
            .spawn_vertical(
                ActorSpec {
                    position: [fixtures::raw(x), fixtures::raw(extremes[6 - i])],
                    momentum: [Fixed::MIN, Fixed::MAX],
                    ..Default::default()
                },
                Solidity::Solid,
                VerticalSpec {
                    height: SpawnHeight::Absolute(fixtures::raw(x)),
                    momentum: fixtures::raw(extremes[6 - i]),
                    gravity: if i % 2 == 0 {
                        Gravity::None
                    } else {
                        Gravity::Normal
                    },
                    allow_dropoff: true,
                },
            )
            .unwrap();
        a.bind_player(0, id).unwrap();
        let mut b = a.clone();
        for _ in 0..12 {
            assert_eq!(a.step(empty()).unwrap(), b.step(empty()).unwrap());
        }
    }
    let mut seed = 1_u32;
    for index in 0..128 {
        let mut a = Simulation::multisector(fixtures::map([0, 1, 10, 12, 18][index % 5])).unwrap();
        for i in 0..3 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let actor = ActorSpec {
                momentum: [
                    fixtures::raw(seed as i32),
                    fixtures::raw(seed.rotate_left(7) as i32),
                ],
                ..spec(
                    ((seed & 255) as i16) - 128,
                    (((seed >> 8) & 127) as i16) - 64,
                )
            };
            let id = a
                .spawn_vertical(
                    actor,
                    if i % 2 == 0 {
                        Solidity::Solid
                    } else {
                        Solidity::NonSolid
                    },
                    VerticalSpec {
                        height: SpawnHeight::Absolute(fixtures::raw(seed as i32)),
                        momentum: fixtures::raw(seed.rotate_left(15) as i32),
                        allow_dropoff: true,
                        ..Default::default()
                    },
                )
                .unwrap();
            if i == 0 {
                a.bind_player(0, id).unwrap();
            }
        }
        let mut b = a.clone();
        for _ in 0..12 {
            assert_eq!(a.step(empty()).unwrap(), b.step(empty()).unwrap());
        }
    }
}

#[test]
fn equivalent_motion_is_independent_of_retired_slot_history() {
    let mut fresh = Simulation::multisector(fixtures::map(0)).unwrap();
    let mut reused = Simulation::multisector(fixtures::map(0)).unwrap();
    let retired = reused.spawn(spec(-120, 60)).unwrap();
    reused.mark_remove(retired).unwrap();
    reused.step(empty()).unwrap();
    let a = spawn(&mut fresh, spec(-64, 0), Fixed::ZERO, true);
    let b = spawn(&mut reused, spec(-64, 0), Fixed::ZERO, true);
    fresh.bind_player(0, a).unwrap();
    reused.bind_player(0, b).unwrap();
    assert_eq!(a.slot(), b.slot());
    assert_ne!(a.generation(), b.generation());
    assert_ne!(fresh.snapshot(), reused.snapshot());
    for _ in 0..32 {
        let mut command = empty();
        command[0].forward = 25;
        fresh.step(command).unwrap();
        reused.step(command).unwrap();
        let a = fresh.actor(a).unwrap();
        let b = reused.actor(b).unwrap();
        assert_eq!(
            (
                a.position(),
                a.z(),
                a.momentum(),
                a.vertical_momentum(),
                a.location(),
                a.constraints()
            ),
            (
                b.position(),
                b.z(),
                b.momentum(),
                b.vertical_momentum(),
                b.location(),
                b.constraints()
            )
        );
    }
    // Whole canonical states intentionally differ: tic/RNG/identity histories are
    // logical state. This checks the physical trajectory, not false hash equality.
}
