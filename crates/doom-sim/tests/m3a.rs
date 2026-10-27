//! Grounded collision policies, recoverable errors, boundaries and DSIM-v2 replay.
#[path = "../../../tests/support/mod.rs"]
mod support;
use doom_map::{Map, MapLimits};
use doom_sim::{
    ActorId, ActorSpec, Event, GeometryError, Profile, SimError, Simulation, Snapshot, Solidity,
    StoreError, TicCommand, compare_traces,
};
use doom_types::Fixed;
use doom_wad::{Limits, Wad};
fn map(lumps: &[support::Lump]) -> Map {
    let bytes = support::wad(*b"PWAD", lumps);
    let wad = Wad::parse(&bytes, Limits::default()).unwrap();
    Map::from_wad(&wad, "MAP01", MapLimits::default()).unwrap()
}
fn simulation(angled: bool) -> Simulation {
    Simulation::player_collision(map(&support::collision_arena(angled))).unwrap()
}
fn pos(x: i16, y: i16) -> ActorSpec {
    ActorSpec {
        position: [Fixed::from_map_units(x), Fixed::from_map_units(y)],
        ..Default::default()
    }
}
fn commands() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}

#[test]
fn overlap_is_strict_axis_square_and_nonsolid_movers_still_block() {
    let mut sim = simulation(false);
    let solid = sim
        .spawn_with_solidity(pos(20, 0), Solidity::Solid)
        .unwrap();
    let mover = sim.spawn(pos(0, 0)).unwrap();
    let at = |x, y| [Fixed::from_bits(x), Fixed::from_bits(y)];
    assert!(
        sim.check_actor_position(mover, at(4 * 65536, 0))
            .unwrap()
            .allowed()
    );
    assert_eq!(
        sim.check_actor_position(mover, at(4 * 65536 + 1, 0))
            .unwrap()
            .blocked_by_actor,
        Some(solid)
    );
    // A circle controller would allow this diagonal; original axis-square test blocks.
    assert_eq!(
        sim.check_actor_position(mover, at(5 * 65536, 15 * 65536))
            .unwrap()
            .blocked_by_actor,
        Some(solid)
    );
    assert!(
        sim.check_actor_position(mover, at(5 * 65536, 16 * 65536))
            .unwrap()
            .allowed()
    );
    let overlapping = sim
        .spawn_with_solidity(pos(20, 0), Solidity::Solid)
        .unwrap();
    assert_eq!(
        sim.check_actor_position(mover, at(20 * 65536, 0))
            .unwrap()
            .blocked_by_actor,
        Some(overlapping)
    );
}

#[test]
fn cell_order_precedes_global_recency_and_offmap_centers_are_excluded() {
    let mut sim = simulation(false);
    let first = sim
        .spawn_with_solidity(pos(-4, -4), Solidity::Solid)
        .unwrap();
    sim.spawn_with_solidity(pos(4, 4), Solidity::Solid).unwrap();
    sim.spawn_with_solidity(pos(-4, 4), Solidity::Solid)
        .unwrap();
    sim.spawn_with_solidity(pos(4, -4), Solidity::Solid)
        .unwrap();
    let mover = sim.spawn(pos(0, 0)).unwrap();
    let check = sim.check_actor_position(mover, [Fixed::ZERO; 2]).unwrap();
    assert_eq!(check.actors, [first]);
    assert_eq!(check.blocked_by_actor, Some(first));
    let mut sim = simulation(false);
    let outside = sim
        .spawn_with_solidity(pos(-129, 0), Solidity::Solid)
        .unwrap();
    let mover = sim.spawn(pos(-127, 0)).unwrap();
    let check = sim
        .check_actor_position(mover, sim.actor(mover).unwrap().position())
        .unwrap();
    assert!(!check.actors.contains(&outside));
    assert!(sim.spatial_order().contains(&outside));
}

#[test]
fn relink_same_cell_removal_and_reuse_preserve_distinct_orders() {
    let mut sim = simulation(false);
    let a = sim
        .spawn_with_solidity(pos(20, 0), Solidity::Solid)
        .unwrap();
    let b = sim
        .spawn_with_solidity(pos(20, 0), Solidity::Solid)
        .unwrap();
    let mover = sim.spawn(pos(0, 0)).unwrap();
    let target = [Fixed::from_map_units(20), Fixed::ZERO];
    assert_eq!(
        sim.check_actor_position(mover, target)
            .unwrap()
            .blocked_by_actor,
        Some(b)
    );
    sim.mark_remove(b).unwrap(); // unlinks now, not when storage retires
    assert!(sim.actor(b).is_some());
    assert!(!sim.spatial_order().contains(&b));
    assert_eq!(
        sim.check_actor_position(mover, target)
            .unwrap()
            .blocked_by_actor,
        Some(a)
    );
    sim.set_momentum(a, [Fixed::ONE, Fixed::ZERO]).unwrap();
    sim.step(commands()).unwrap();
    assert_eq!(sim.spatial_order()[0], a); // successful same-cell move linked at head
    let reused = sim
        .spawn_with_solidity(pos(20, 0), Solidity::Solid)
        .unwrap();
    assert_eq!(reused.slot(), b.slot());
    assert_ne!(reused.generation(), b.generation());
    assert_eq!(sim.actor_order().collect::<Vec<_>>(), [a, mover, reused]);
    assert_eq!(sim.spatial_order()[0], reused);
    assert_eq!(
        sim.check_actor_position(mover, target)
            .unwrap()
            .blocked_by_actor,
        Some(reused)
    );
    assert_eq!(
        sim.check_actor_position(b, target),
        Err(SimError::Store(StoreError::InvalidHandle))
    );
}

#[test]
fn errors_are_atomic_with_pending_events_rng_and_spatial_state() {
    let mut sim =
        Simulation::with_player_collision_limit(map(&support::collision_arena(false)), 1).unwrap();
    let actor = sim.spawn_with_solidity(pos(0, 0), Solidity::Solid).unwrap();
    sim.bind_player(0, actor).unwrap();
    sim.gameplay_random();
    let before = sim.snapshot();
    assert_eq!(
        sim.spawn(pos(0, 0)),
        Err(SimError::Store(StoreError::Capacity))
    );
    assert_eq!(sim.snapshot(), before);
    assert!(
        sim.spawn(ActorSpec {
            radius: Fixed::MIN,
            ..Default::default()
        })
        .is_err()
    );
    assert_eq!(sim.snapshot(), before);
    assert_eq!(sim.bind_player(1, actor), Err(SimError::InvalidPlayer));
    assert_eq!(sim.snapshot(), before);
    assert_eq!(
        sim.mark_remove(ActorId::from_parts(actor.slot(), actor.generation() + 1)),
        Err(SimError::Store(StoreError::InvalidHandle))
    );
    assert_eq!(sim.snapshot(), before);
    let mut input = commands();
    input[3].buttons = 255;
    assert_eq!(sim.step(input), Err(SimError::UnsupportedButtons));
    assert_eq!(sim.snapshot(), before);
    sim.set_paused(true);
    let before = sim.snapshot();
    assert_eq!(sim.step(input).unwrap(), before);
    sim.set_paused(false);
    sim.step(commands()).unwrap();
    assert!(sim.events().contains(&Event::Spawned(actor)));
    assert_eq!(sim.rng().indices(), [2, 0]);
}

#[test]
fn missing_coverage_degenerate_overflow_and_work_limits_reject_at_setup() {
    let mut lumps = support::collision_arena(false);
    // Structurally valid list, geometrically missing lines 1..3.
    lumps[10].1 = support::words(&[-128, -128, 2, 2, 8, 8, 8, 8, 0, -1]);
    assert!(matches!(
        Simulation::player_collision(map(&lumps)),
        Err(SimError::Geometry(GeometryError::Unsupported(
            "M3a incomplete collision BLOCKMAP"
        )))
    ));
    // Foundation preserves its source-faithful list policy, without claiming coverage.
    assert!(Simulation::new(map(&lumps)).is_ok());
    let mut lumps = support::collision_arena(false);
    support::set_word(&mut lumps[2].1, 2, 0);
    assert!(matches!(
        Simulation::player_collision(map(&lumps)),
        Err(SimError::Geometry(GeometryError::Unsupported(
            "M3a degenerate line"
        )))
    ));
    let mut lumps = support::collision_arena(false);
    support::set_word(&mut lumps[4].1, 0, i16::MIN);
    support::set_word(&mut lumps[4].1, 4, i16::MAX);
    assert!(matches!(
        Simulation::player_collision(map(&lumps)),
        Err(SimError::Geometry(GeometryError::CoordinateOverflow))
    ));
    assert!(matches!(
        Simulation::with_player_collision_limit(map(&support::collision_arena(false)), 1025),
        Err(SimError::Store(StoreError::InvalidLimit))
    ));
    let mut lumps = support::collision_arena(false);
    let line = lumps[2].1[..14].to_vec();
    for _ in 0..125 {
        lumps[2].1.extend(&line);
    }
    assert!(matches!(
        Simulation::player_collision(map(&lumps)),
        Err(SimError::Geometry(GeometryError::Unsupported(
            "M3a geometry work limit"
        )))
    ));
}

#[test]
fn duplicate_block_references_keep_first_encounter_motion() {
    let original = support::collision_arena(true);
    let mut duplicate = original.clone();
    duplicate[10].1 =
        support::words(&[-128, -128, 2, 2, 8, 8, 8, 8, 0, 0, 0, 1, 2, 2, 3, 4, 4, -1]);
    let mut a = Simulation::player_collision(map(&original)).unwrap();
    let mut b = Simulation::player_collision(map(&duplicate)).unwrap();
    for sim in [&mut a, &mut b] {
        let id = sim
            .spawn_with_solidity(pos(20, 20), Solidity::Solid)
            .unwrap();
        sim.bind_player(0, id).unwrap();
        sim.set_momentum(id, [Fixed::from_map_units(20), Fixed::from_map_units(8)])
            .unwrap();
    }
    for _ in 0..24 {
        a.step(commands()).unwrap();
        b.step(commands()).unwrap();
        assert_eq!(a.events(), b.events());
        for id in a.actor_order() {
            assert_eq!(a.actor(id), b.actor(id));
        }
    }
}

#[test]
fn dsim_v2_matches_independent_python_bytes_and_all_coasting_goldens() {
    let mut sim = simulation(false);
    let id = sim
        .spawn_with_solidity(
            ActorSpec {
                momentum: [Fixed::ONE, Fixed::ZERO],
                ..Default::default()
            },
            Solidity::Solid,
        )
        .unwrap();
    sim.bind_player(0, id).unwrap();
    for (i, row) in include_str!("../../../tests/reference/m3a_snapshots.csv")
        .lines()
        .skip(1)
        .enumerate()
    {
        let snapshot = sim.step(commands()).unwrap();
        assert_eq!(snapshot.version(), 2);
        if i == 0 {
            assert_eq!(
                snapshot.canonical_bytes(),
                include_bytes!("../../../tests/reference/m3a_coast_first.bin")
            );
        }
        assert_eq!(
            snapshot.digest(),
            u64::from_str_radix(row.split(',').nth(1).unwrap(), 16).unwrap(),
            "DSIM-v2 tic {}",
            i + 1
        );
    }
    let old = Simulation::new(map(&support::collision_arena(false))).unwrap();
    assert_eq!(old.profile(), Profile::Foundation);
    assert_eq!(old.snapshot().version(), 1);
    assert_eq!(
        old.snapshot()
            .first_difference(&sim.snapshot())
            .unwrap()
            .field,
        "schema.version"
    );
}

fn trace(presentation: bool, perturb: bool) -> Vec<Snapshot> {
    let mut sim = simulation(true);
    let player = sim
        .spawn_with_solidity(
            ActorSpec {
                momentum: [Fixed::from_map_units(20), Fixed::from_map_units(8)],
                ..pos(0, 0)
            },
            Solidity::Solid,
        )
        .unwrap();
    let mut obstacle = sim
        .spawn_with_solidity(pos(20, 0), Solidity::Solid)
        .unwrap();
    sim.bind_player(0, player).unwrap();
    let mut trace = Vec::new();
    let mut wall_hits = 0;
    let mut actor_hits = 0;
    let mut slides = 0;
    for tic in 0..768 {
        if tic % 97 == 20 {
            sim.mark_remove(obstacle).unwrap();
        }
        if tic % 97 == 21 {
            obstacle = sim
                .spawn_with_solidity(pos(20, 0), Solidity::Solid)
                .unwrap();
        }
        sim.set_paused(tic == 300);
        if !sim.paused() {
            sim.gameplay_random();
        }
        if perturb && tic == 400 {
            sim.gameplay_random();
        }
        if presentation {
            for _ in 0..tic % 7 {
                let _ = sim.snapshot().digest();
                let _ = sim.events();
                let mut copy = sim.rng();
                copy.p_random();
                copy.m_random();
            }
        }
        let mut input = commands();
        input[0] = TicCommand {
            forward: 50,
            side: if tic % 96 < 48 { -24 } else { 24 },
            angle_turn: if tic % 128 < 32 { 512 } else { 0 },
            buttons: 0,
        };
        if tic < 12 {
            input = commands();
        }
        trace.push(sim.step(input).unwrap());
        for e in sim.events() {
            match e {
                Event::ActorMovement { attempt, .. } => {
                    wall_hits += usize::from(attempt.lines.blocked_by.is_some());
                    actor_hits += usize::from(attempt.blocked_by_actor.is_some());
                }
                Event::Slide { .. } => slides += 1,
                _ => {}
            }
        }
    }
    assert!(
        wall_hits > 0 && actor_hits > 0 && slides > 0,
        "wall={wall_hits} actor={actor_hits} slides={slides}"
    );
    assert_eq!(sim.tic(), 767);
    trace
}

#[test]
fn long_player_wall_actor_lifecycle_traces_are_read_schedule_independent() {
    let expected = trace(false, false);
    let actual = trace(true, false);
    assert_eq!(compare_traces(&expected, &actual), None);
    assert_eq!(expected, actual);
    for (a, b) in expected.iter().zip(&actual) {
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
        assert_eq!(a.digest(), b.digest());
    }
    let actual = trace(false, true);
    let difference = compare_traces(&expected, &actual).unwrap();
    assert_eq!(
        (difference.tic, difference.field.as_str()),
        (400, "rng.gameplay")
    );
}

#[test]
fn bounded_generated_extremes_never_panic_or_drift_across_instances() {
    let extremes = [
        i32::MIN,
        i32::MIN + 1,
        -128 * 65536,
        -1,
        0,
        1,
        128 * 65536,
        i32::MAX - 1,
        i32::MAX,
    ];
    for (i, x) in extremes.into_iter().enumerate() {
        let mut a = simulation(true);
        let id = a
            .spawn_with_solidity(
                ActorSpec {
                    position: [Fixed::from_bits(x), Fixed::from_bits(extremes[8 - i])],
                    momentum: [Fixed::MIN, Fixed::MAX],
                    radius: Fixed::from_bits(if i % 2 == 0 { 0 } else { 32 * 65536 }),
                    ..Default::default()
                },
                Solidity::Solid,
            )
            .unwrap();
        a.bind_player(0, id).unwrap();
        let mut b = a.clone();
        for turn in [i16::MIN, -1, 0, 1, i16::MAX] {
            let mut input = commands();
            input[0] = TicCommand {
                forward: i8::MIN,
                side: i8::MAX,
                angle_turn: turn,
                buttons: 0,
            };
            assert_eq!(a.step(input).unwrap(), b.step(input).unwrap());
        }
    }
    let mut seed = 1_u32;
    for _ in 0..128 {
        let mut a = simulation(true);
        for index in 0..4 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let mut actor = pos(
                ((seed & 127) as i16) - 64,
                (((seed >> 8) & 127) as i16) - 64,
            );
            actor.momentum = [
                Fixed::from_bits(seed as i32),
                Fixed::from_bits(seed.rotate_left(17) as i32),
            ];
            let id = a
                .spawn_with_solidity(
                    actor,
                    if index % 2 == 0 {
                        Solidity::Solid
                    } else {
                        Solidity::NonSolid
                    },
                )
                .unwrap();
            if index == 0 {
                a.bind_player(0, id).unwrap();
            }
        }
        let mut b = a.clone();
        for _ in 0..12 {
            assert_eq!(a.step(commands()).unwrap(), b.step(commands()).unwrap());
        }
    }
}

#[test]
fn shared_long_lists_compile_without_repeated_raw_list_scans() {
    let mut lumps = support::collision_arena(false);
    let mut words = vec![-128, -128, 64, 64];
    words.extend(std::iter::repeat_n(4100, 4096));
    words.push(0);
    for _ in 0..15000 {
        words.extend([0, 1, 2, 3]);
    }
    words.push(-1);
    lumps[10].1 = support::words(&words);
    let mut sim = Simulation::player_collision(map(&lumps)).unwrap();
    let actor = sim
        .spawn(ActorSpec {
            momentum: [Fixed::ONE, Fixed::ZERO],
            ..Default::default()
        })
        .unwrap();
    sim.bind_player(0, actor).unwrap();
    sim.step(commands()).unwrap();
    let attempts: Vec<_> = sim
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::ActorMovement { attempt, .. } => Some(attempt),
            _ => None,
        })
        .collect();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].lines.candidates, [0, 1, 2, 3]);
    words.resize(65537, 0); // Harmless unused storage still exceeds the M3a work bound.
    lumps[10].1 = support::words(&words);
    assert!(matches!(
        Simulation::player_collision(map(&lumps)),
        Err(SimError::Geometry(GeometryError::Unsupported(
            "M3a geometry work limit"
        )))
    ));
}
