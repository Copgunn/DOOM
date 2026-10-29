//! Rust admission/atomicity policies, distinct from original-C equivalence.
#[path = "../../../tests/support/m3f2.rs"]
mod fixtures;
use doom_sim::{
    ActorId, ActorSpec, Gravity, MapExtentLimits, Profile, Simulation, Solidity, SpawnHeight,
    TicCommand, VerticalSpec,
};
use doom_types::Fixed;
fn u(v: i16) -> Fixed {
    Fixed::from_map_units(v)
}
fn commands() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}
fn spawn(sim: &mut Simulation, x: i16, y: i16) -> ActorId {
    sim.spawn(ActorSpec {
        position: [u(x), u(y)],
        ..Default::default()
    })
    .unwrap()
}
fn grid(width: i16, height: i16) -> doom_map::Map {
    let mut lumps = fixtures::lumps(0);
    let cells = usize::from(width as u16) * usize::from(height as u16);
    let mut bytes = Vec::new();
    for n in [-2048_i16, -2048, width, height] {
        bytes.extend(n.to_le_bytes());
    }
    for _ in 0..cells {
        bytes.extend(((4 + cells) as i16).to_le_bytes());
    }
    for n in [0_i16, 3, 2, 1, 0, -1] {
        bytes.extend(n.to_le_bytes());
    }
    lumps[10].1 = bytes;
    fixtures::decode(&lumps).unwrap()
}
#[test]
fn historical_geometry_bound_remains_512_and_new_profile_is_explicit() {
    assert!(Simulation::map_scale(fixtures::map(1)).is_ok());
    assert!(Simulation::map_scale(fixtures::map(2)).is_err());
    let sim = Simulation::map_extent(fixtures::map(2)).unwrap();
    assert_eq!(sim.profile(), Profile::MapExtent);
    let snapshot = sim.snapshot();
    assert_eq!(snapshot.version(), 19);
    assert!(!snapshot.fields().any(|f| f.path() == "actors.options"));
    for i in [3, 4, 5, 6, 7, 8, 9, 10, 17, 18, 20, 29] {
        assert!(
            Simulation::map_scale(fixtures::map(i)).is_err(),
            "old geometry profile map{i}"
        );
        assert!(
            Simulation::map_extent(fixtures::map(i)).is_ok(),
            "new geometry map{i}"
        );
    }
}
#[test]
fn independent_coordinate_node_height_and_line_norm_bounds_reject_above_proof() {
    for (map, lump, offset, value) in [
        (5, 4, 8, 2049),
        (6, 4, 0, -2049),
        (3, 7, 0, 2049),
        (3, 7, 6, 2049),
        (18, 8, 2, 1025),
        (17, 8, 0, -1025),
        (0, 10, 0, 2049),
        (0, 10, 2, -2049),
    ] {
        let mut lumps = fixtures::lumps(map);
        fixtures::set_word(&mut lumps[lump].1, offset, value);
        assert!(
            Simulation::map_extent(fixtures::decode(&lumps).unwrap()).is_err(),
            "bound {map}/{lump}/{offset}"
        );
    }
    let mut lumps = fixtures::lumps(9); // source axis1536 is admitted
    let x = i16::from_le_bytes(lumps[4].1[8..10].try_into().unwrap());
    fixtures::set_word(&mut lumps[4].1, 8, x + 1);
    assert!(Simulation::map_extent(fixtures::decode(&lumps).unwrap()).is_err());
    let mut lumps = fixtures::lumps(3);
    fixtures::set_word(&mut lumps[7].1, 0, 2048);
    fixtures::set_word(&mut lumps[7].1, 6, 2048);
    assert!(Simulation::map_extent(fixtures::decode(&lumps).unwrap()).is_ok());
    // The wider interaction domain is not a blanket superset of old long diagonals.
    let mut lumps = fixtures::lumps(0);
    for (offset, v) in [(0, -512), (2, -512), (4, 512), (6, 512)] {
        fixtures::set_word(&mut lumps[4].1, offset, v);
    }
    let map = fixtures::decode(&lumps).unwrap();
    assert!(Simulation::map_scale(map.clone()).is_ok());
    assert!(Simulation::map_extent(map).is_err());
}
#[test]
fn raw_unit_actor_destination_and_momentum_boundaries_are_atomic() {
    let mut sim = Simulation::map_extent(fixtures::map(7)).unwrap();
    for sign in [-1_i32, 1] {
        let spec = ActorSpec {
            position: [Fixed::from_bits(sign * 2018 * 65536), Fixed::ZERO],
            ..Default::default()
        };
        let id = sim.spawn(spec).unwrap();
        let before = sim.snapshot();
        let mut outside = spec;
        outside.position[0] = Fixed::from_bits(sign * (2018 * 65536 + 1));
        assert!(sim.spawn(outside).is_err());
        assert_eq!(before, sim.snapshot());
        for raw in [sign * 2048 * 65536, sign * (2048 * 65536 - 1)] {
            let before = sim.snapshot();
            sim.check_multisector_position(id, [Fixed::from_bits(raw), Fixed::ZERO])
                .unwrap();
            sim.world()
                .locate([Fixed::from_bits(raw), Fixed::ZERO])
                .unwrap();
            assert_eq!(before, sim.snapshot());
        }
        for raw in [
            sign * (2048 * 65536 + 1),
            if sign < 0 { i32::MIN } else { i32::MAX },
        ] {
            let before = sim.snapshot();
            assert!(
                sim.check_multisector_position(id, [Fixed::from_bits(raw), Fixed::ZERO])
                    .is_err()
            );
            assert!(
                sim.world()
                    .locate([Fixed::from_bits(raw), Fixed::ZERO])
                    .is_err()
            );
            assert_eq!(before, sim.snapshot());
        }
        sim.set_momentum(id, [Fixed::from_bits(sign * 64 * 65536), Fixed::ZERO])
            .unwrap();
        sim.set_vertical_momentum(id, Fixed::from_bits(sign * 64 * 65536))
            .unwrap();
        let before = sim.snapshot();
        assert!(
            sim.set_momentum(id, [Fixed::from_bits(sign * (64 * 65536 + 1)), Fixed::ZERO])
                .is_err()
        );
        assert!(
            sim.set_vertical_momentum(id, Fixed::from_bits(sign * (64 * 65536 + 1)))
                .is_err()
        );
        assert_eq!(before, sim.snapshot());
        let vertical = VerticalSpec {
            height: SpawnHeight::Absolute(Fixed::from_bits(sign * 1984 * 65536)),
            ..Default::default()
        };
        sim.spawn_vertical(ActorSpec::default(), Solidity::NonSolid, vertical)
            .unwrap();
        let before = sim.snapshot();
        assert!(
            sim.spawn_vertical(
                ActorSpec::default(),
                Solidity::NonSolid,
                VerticalSpec {
                    height: SpawnHeight::Absolute(Fixed::from_bits(sign * (1984 * 65536 + 1))),
                    ..vertical
                }
            )
            .is_err()
        );
        assert_eq!(before, sim.snapshot());
    }
}
#[test]
fn full_height_and_negative_floors_are_bounded_independently_of_xy() {
    let mut sim = Simulation::map_extent(fixtures::map(19)).unwrap();
    let a = sim
        .spawn(ActorSpec {
            height: u(2048),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(sim.actor(a).unwrap().z(), u(-1024));
    let b = spawn(&mut sim, 64, 0);
    let before = sim.snapshot();
    sim.check_sight(a, b).unwrap();
    assert_eq!(before, sim.snapshot());
    let mut falling = Simulation::map_extent(fixtures::map(19)).unwrap();
    let id = falling
        .spawn_vertical(
            ActorSpec::default(),
            Solidity::NonSolid,
            VerticalSpec {
                height: SpawnHeight::Absolute(u(512)),
                momentum: u(-64),
                ..Default::default()
            },
        )
        .unwrap();
    falling.step(commands()).unwrap();
    assert_eq!(falling.actor(id).unwrap().vertical_momentum(), u(-65));
    let before = falling.snapshot();
    assert!(falling.step(commands()).is_err());
    assert_eq!(before, falling.snapshot());
    assert!(
        sim.spawn(ActorSpec {
            height: Fixed::from_bits(2048 * 65536 + 1),
            ..Default::default()
        })
        .is_err()
    );
    assert_eq!(before, sim.snapshot());
}
#[test]
fn new_cell_limit_is_executed_at_high_indexes_and_keeps_order() {
    let mut sim = Simulation::map_extent(fixtures::map(20)).unwrap();
    let a = spawn(&mut sim, 1476, 1500);
    let b = spawn(&mut sim, 1508, 1500);
    let metrics = sim.map_extent_metrics().unwrap();
    assert_eq!((metrics.cells, metrics.prepared_references), (8192, 262144));
    let map = sim.world().map();
    let origin = map.blockmap().origin();
    let cell_x = (1476 - i32::from(origin.x)) / 128;
    let cell_y = (1500 - i32::from(origin.y)) / 128;
    assert!(cell_y * 256 + cell_x > 4096);
    let movement = sim
        .check_multisector_position(a, [u(1482), u(1500)])
        .unwrap();
    assert_eq!(&movement.movement.lines.candidates[..3], &[0, 31, 30]);
    assert_eq!(movement.movement.actors, vec![b, a]);
    sim.set_momentum(a, [u(1), Fixed::ZERO]).unwrap();
    sim.step(commands()).unwrap();
    assert_eq!(sim.spatial_order(), &[a, b]);
    assert_eq!(sim.actor_order().collect::<Vec<_>>(), vec![a, b]);
    assert!(Simulation::map_scale(grid(256, 32)).is_err());
    assert!(Simulation::map_extent(grid(256, 32)).is_ok());
    assert!(Simulation::map_extent(grid(8193, 1)).is_err());
    for index in [21, 22] {
        assert_eq!(
            Simulation::map_extent(fixtures::map(index))
                .unwrap()
                .map_extent_metrics()
                .unwrap()
                .cells,
            8192
        );
    }
}
#[test]
fn cell_references_raw_words_memory_and_cpu_budgets_are_independent() {
    assert!(
        Simulation::with_map_extent_limits(
            fixtures::map(20),
            MapExtentLimits {
                max_prepared_references: 262143,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        Simulation::with_map_extent_limits(
            fixtures::map(0),
            MapExtentLimits {
                max_preparation_bytes: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        Simulation::with_map_extent_limits(
            fixtures::map(0),
            MapExtentLimits {
                max_collision_work: 271,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        Simulation::with_map_extent_limits(
            fixtures::map(0),
            MapExtentLimits {
                max_collision_work: 272,
                ..Default::default()
            }
        )
        .is_ok()
    );
    for limits in [
        MapExtentLimits {
            max_actors: 65,
            ..Default::default()
        },
        MapExtentLimits {
            max_preparation_bytes: usize::MAX,
            ..Default::default()
        },
        MapExtentLimits {
            max_collision_work: usize::MAX,
            ..Default::default()
        },
        MapExtentLimits {
            max_prepared_references: usize::MAX,
            ..Default::default()
        },
    ] {
        assert!(Simulation::with_map_extent_limits(fixtures::map(0), limits).is_err());
    }
    let mut lumps = fixtures::lumps(0);
    lumps[10].1.resize(2 * 65536, 255);
    assert!(Simulation::map_extent(fixtures::decode(&lumps).unwrap()).is_ok());
    lumps[10].1.extend([255, 255]);
    assert!(Simulation::map_extent(fixtures::decode(&lumps).unwrap()).is_err());
}
#[test]
fn repeated_child_cost_counts_full_expansion_not_unique_nodes() {
    assert!(Simulation::map_scale(fixtures::map(26)).is_ok());
    assert!(Simulation::map_scale(fixtures::map(27)).is_err());
    let mut sim = Simulation::map_extent(fixtures::map(27)).unwrap();
    let a = spawn(&mut sim, -64, 0);
    let b = spawn(&mut sim, 64, 0);
    let before = sim.snapshot();
    let sight = sim.check_sight(a, b).unwrap();
    let work = sight
        .visits
        .iter()
        .filter(|v| {
            matches!(
                v,
                doom_sim::SightVisit::NodeStart { .. }
                    | doom_sim::SightVisit::NodeEnd { .. }
                    | doom_sim::SightVisit::Subsector(_)
                    | doom_sim::SightVisit::Seg { .. }
            )
        })
        .count();
    assert_eq!(work, 16382);
    assert!(sight.visible);
    assert_eq!(before, sim.snapshot());
    let mut lumps = fixtures::lumps(27);
    let mut node = lumps[7].1[..28].to_vec();
    fixtures::set_word(&mut node, 24, 11);
    fixtures::set_word(&mut node, 26, 11);
    lumps[7].1.extend(node);
    assert!(Simulation::map_extent(fixtures::decode(&lumps).unwrap()).is_err());
    assert_eq!(before, sim.snapshot());
}
#[test]
fn late_actor_step_error_preserves_all_earlier_work_pending_events_and_rng() {
    let mut sim = Simulation::map_extent(fixtures::map(7)).unwrap();
    let mut airborne = |x, mx| {
        sim.spawn_vertical(
            ActorSpec {
                position: [u(x), Fixed::ZERO],
                momentum: [u(mx), Fixed::ZERO],
                ..Default::default()
            },
            Solidity::NonSolid,
            VerticalSpec {
                height: SpawnHeight::Absolute(u(16)),
                gravity: Gravity::None,
                allow_dropoff: true,
                ..Default::default()
            },
        )
        .unwrap()
    };
    let first = airborne(0, 6);
    let late = airborne(2018, 30);
    sim.step(commands()).unwrap();
    assert!(sim.actor(late).unwrap().position()[0] > u(2018));
    sim.gameplay_random();
    let before = sim.snapshot();
    assert!(sim.step(commands()).is_err());
    assert_eq!(before, sim.snapshot());
    sim.mark_remove(late).unwrap();
    sim.step(commands()).unwrap();
    assert!(sim.actor(first).unwrap().position()[0] > u(6));
    let new = spawn(&mut sim, 64, 0);
    assert_eq!(late.slot(), new.slot());
    assert_ne!(late.generation(), new.generation());
    let before = sim.snapshot();
    assert!(sim.check_sight(late, new).is_err());
    assert!(
        sim.check_multisector_position(late, [Fixed::ZERO; 2])
            .is_err()
    );
    assert!(sim.set_momentum(late, [Fixed::ZERO; 2]).is_err());
    assert_eq!(before, sim.snapshot());
}
#[test]
fn capacity_pending_removal_and_zero_capacity_do_not_mutate_on_failure() {
    let mut sim = Simulation::with_map_extent_limits(
        fixtures::map(0),
        MapExtentLimits {
            max_actors: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let a = spawn(&mut sim, 0, 0);
    let before = sim.snapshot();
    assert!(sim.spawn(ActorSpec::default()).is_err());
    assert_eq!(before, sim.snapshot());
    sim.mark_remove(a).unwrap();
    let before = sim.snapshot();
    assert!(sim.check_sight(a, a).is_err());
    assert!(sim.check_multisector_position(a, [Fixed::ZERO; 2]).is_err());
    assert_eq!(before, sim.snapshot());
    let mut empty = Simulation::with_map_extent_limits(
        fixtures::map(0),
        MapExtentLimits {
            max_actors: 0,
            ..Default::default()
        },
    )
    .unwrap();
    let before = empty.snapshot();
    assert!(empty.spawn(ActorSpec::default()).is_err());
    assert_eq!(before, empty.snapshot());
    assert_eq!(empty.rng().indices(), [0, 0]);
    let mut full = Simulation::map_extent(fixtures::map(0)).unwrap();
    let ids: Vec<_> = (0..64).map(|_| spawn(&mut full, 0, 0)).collect();
    let before = full.snapshot();
    assert!(full.spawn(ActorSpec::default()).is_err());
    assert_eq!(before, full.snapshot());
    full.step(commands()).unwrap();
    let order: Vec<_> = full
        .events()
        .iter()
        .filter_map(|event| {
            if let doom_sim::Event::Think(id) = event {
                Some(*id)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(order, ids);
}
#[test]
fn map_specials_and_malformed_references_reject_before_execution() {
    let sim = Simulation::map_extent(fixtures::map(0)).unwrap();
    let before = sim.snapshot();
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[2].1, 0, 32767);
    assert!(fixtures::decode(&lumps).is_err());
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[2].1, 6, 1);
    assert!(Simulation::map_extent(fixtures::decode(&lumps).unwrap()).is_err());
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[8].1, 22, 1);
    assert!(Simulation::map_extent(fixtures::decode(&lumps).unwrap()).is_err());
    assert_eq!(before, sim.snapshot());
}
#[test]
fn expanded_queries_pause_reads_and_smaller_budgets_preserve_independent_instances() {
    let mut a = Simulation::map_extent(fixtures::map(7)).unwrap();
    let mut b = Simulation::with_map_extent_limits(
        fixtures::map(7),
        MapExtentLimits {
            max_preparation_bytes: 2 * 1024 * 1024,
            max_collision_work: 1000,
            ..Default::default()
        },
    )
    .unwrap();
    let first = spawn(&mut a, -1500, 0);
    let target = spawn(&mut a, 1500, 0);
    assert_eq!(first, spawn(&mut b, -1500, 0));
    assert_eq!(target, spawn(&mut b, 1500, 0));
    for sim in [&mut a, &mut b] {
        sim.set_momentum(first, [u(6), Fixed::ZERO]).unwrap();
    }
    for _ in 0..24 {
        let before = a.snapshot();
        a.map_extent_metrics().unwrap();
        a.world().locate([u(1500), Fixed::ZERO]).unwrap();
        a.check_sight(first, target).unwrap();
        a.check_multisector_position(first, [u(1500), Fixed::ZERO])
            .unwrap();
        assert_eq!(before, a.snapshot());
        assert_eq!(a.step(commands()).unwrap(), b.step(commands()).unwrap());
    }
    a.set_paused(true);
    let before = a.snapshot();
    let mut invalid = commands();
    invalid[0].buttons = 255;
    assert_eq!(a.step(invalid).unwrap(), before);
    assert_eq!(before, a.snapshot());
}

#[test]
fn new_geometry_profile_rejects_player_typed_combat_ai_and_map_spawning() {
    let mut sim = Simulation::map_extent(fixtures::map(7)).unwrap();
    let a = spawn(&mut sim, 1500, 0);
    let before = sim.snapshot();
    assert!(sim.bind_player(0, a).is_err());
    assert_eq!(before, sim.snapshot());
    assert!(
        sim.damage_actor(doom_sim::DamageRequest {
            target: a,
            source: None,
            inflictor: None,
            amount: 1
        })
        .is_err()
    );
    assert_eq!(before, sim.snapshot());
    assert!(
        sim.spawn_type(
            doom_sim::ActorTypeId(12),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor
        )
        .is_err()
    );
    assert_eq!(before, sim.snapshot());
    assert!(sim.noise_alert(a, a).is_err());
    assert_eq!(before, sim.snapshot());
    assert!(sim.look(a).is_err());
    assert_eq!(before, sim.snapshot());
    assert!(sim.spawn_map_things().is_err());
    assert_eq!(before, sim.snapshot());
    let mut invalid = commands();
    invalid[0].buttons = 255;
    assert!(sim.step(invalid).is_err());
    assert_eq!(before, sim.snapshot());
}
