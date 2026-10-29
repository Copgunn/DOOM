//! Rust-only bounded admission and atomicity, separate from original-C equivalence.
#[path = "../../../tests/support/m3f1.rs"]
mod fixtures;
use doom_sim::{
    ActorSpec, ActorTypeId, DamageRequest, GeometryError, MapScaleLimits, Profile, Simulation,
    Solidity, SpawnHeight, TicCommand,
};
use doom_types::Fixed;
fn units(n: i16) -> Fixed {
    Fixed::from_map_units(n)
}
fn zero() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}
fn spawn(sim: &mut Simulation, x: i16) -> doom_sim::ActorId {
    sim.spawn_with_solidity(
        ActorSpec {
            position: [units(x), Fixed::ZERO],
            ..Default::default()
        },
        Solidity::Solid,
    )
    .unwrap()
}
#[test]
fn counts_at_old_bound_one_more_and_new_bounds_are_explicit() {
    for (map, lines, sectors) in [
        (2, 128, 1),
        (4, 129, 1),
        (6, 256, 1),
        (8, 1024, 1),
        (10, 385, 128),
        (12, 388, 129),
        (14, 769, 256),
    ] {
        let sim = Simulation::map_scale(fixtures::map(map)).unwrap();
        let m = sim.map_scale_metrics().unwrap();
        assert_eq!((m.lines, m.sectors), (lines, sectors));
        assert_eq!(sim.profile(), Profile::MapScale);
        assert_eq!(sim.snapshot().version(), 18);
        assert_eq!(sim.world().conservative_omissions(), Some(0));
        assert_eq!(
            Simulation::multisector(fixtures::map(map)).is_ok(),
            lines <= 128 && sectors <= 128
        );
    }
}
#[test]
fn prepared_reference_budget_checks_the_expanded_lists_not_only_raw_words() {
    let exact = Simulation::map_scale(fixtures::map(8))
        .unwrap()
        .map_scale_metrics()
        .unwrap()
        .prepared_references;
    assert_eq!(exact, 8192);
    let mut sim = Simulation::with_map_scale_limits(
        fixtures::map(8),
        MapScaleLimits {
            max_prepared_references: exact,
            ..Default::default()
        },
    )
    .unwrap();
    let id = spawn(&mut sim, -24);
    let before = sim.snapshot();
    assert!(
        Simulation::with_map_scale_limits(
            fixtures::map(8),
            MapScaleLimits {
                max_prepared_references: exact - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(before, sim.snapshot());
    assert!(
        sim.check_multisector_position(id, [units(-16), Fixed::ZERO])
            .is_ok()
    );
    assert!(
        Simulation::with_map_scale_limits(
            fixtures::map(0),
            MapScaleLimits {
                max_actors: 65,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        Simulation::with_map_scale_limits(
            fixtures::map(0),
            MapScaleLimits {
                max_prepared_references: usize::MAX,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn line_and_sector_capacity_plus_one_reject_without_changing_an_existing_instance() {
    let sim = Simulation::map_scale(fixtures::map(0)).unwrap();
    let before = sim.snapshot();
    let mut lumps = fixtures::lumps(8);
    let line = lumps[2].1[..14].to_vec();
    lumps[2].1.extend(line);
    assert!(Simulation::map_scale(fixtures::decode(&lumps).unwrap()).is_err());
    let mut lumps = fixtures::lumps(14);
    let sector = lumps[8].1[..26].to_vec();
    lumps[8].1.extend(sector);
    lumps[9].1.resize((257_usize * 257).div_ceil(8), 0);
    assert!(Simulation::map_scale(fixtures::decode(&lumps).unwrap()).is_err());
    assert_eq!(before, sim.snapshot());
}
#[test]
fn raw_word_cell_and_sight_expansion_budgets_remain_bounded() {
    let mut lumps = fixtures::lumps(0);
    lumps[10].1.resize(2 * 65537, 255);
    assert!(Simulation::map_scale(fixtures::decode(&lumps).unwrap()).is_err());
    let mut lumps = fixtures::lumps(0);
    let mut block = Vec::new();
    for n in [-256_i16, -128, 4097, 1] {
        block.extend(n.to_le_bytes());
    }
    for _ in 0..4097 {
        block.extend(4101_i16.to_le_bytes());
    }
    for n in [0_i16, 0, -1] {
        block.extend(n.to_le_bytes());
    }
    lumps[10].1 = block;
    assert!(Simulation::map_scale(fixtures::decode(&lumps).unwrap()).is_err());
    let mut lumps = fixtures::lumps(16);
    let node = lumps[7].1[..28].to_vec();
    for i in 1..15 {
        let mut next = node.clone();
        fixtures::set_word(&mut next, 24, i - 1);
        fixtures::set_word(&mut next, 26, i - 1);
        lumps[7].1.extend(next);
    }
    assert!(Simulation::map_scale(fixtures::decode(&lumps).unwrap()).is_err());
}
#[test]
fn malformed_indexes_specials_and_arithmetic_geometry_reject() {
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[2].1, 0, 32767);
    assert!(fixtures::decode(&lumps).is_err());
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[2].1, 6, 1);
    assert!(Simulation::map_scale(fixtures::decode(&lumps).unwrap()).is_err());
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[4].1, 0, 513);
    assert!(Simulation::map_scale(fixtures::decode(&lumps).unwrap()).is_err());
}
#[test]
fn actor_capacity_and_invalid_controls_preserve_every_canonical_field() {
    let mut sim = Simulation::with_map_scale_limits(
        fixtures::map(8),
        MapScaleLimits {
            max_actors: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let actor = spawn(&mut sim, -24);
    let before = sim.snapshot();
    assert!(sim.spawn(ActorSpec::default()).is_err());
    assert_eq!(before, sim.snapshot());
    assert!(sim.set_momentum(actor, [units(65), Fixed::ZERO]).is_err());
    assert_eq!(before, sim.snapshot());
    assert!(sim.set_vertical_momentum(actor, units(-65)).is_err());
    assert_eq!(before, sim.snapshot());
    assert!(
        sim.spawn(ActorSpec {
            position: [units(995), Fixed::ZERO],
            ..Default::default()
        })
        .is_err()
    );
    assert_eq!(before, sim.snapshot());
    assert!(
        sim.check_multisector_position(actor, [units(1025), Fixed::ZERO])
            .is_err()
    );
    assert_eq!(before, sim.snapshot());
    let mut command = zero();
    command[0].buttons = 1;
    assert!(sim.step(command).is_err());
    assert_eq!(before, sim.snapshot());
}
#[test]
fn new_profile_does_not_admit_player_combat_sound_or_typed_spawning() {
    let mut sim = Simulation::map_scale(fixtures::map(14)).unwrap();
    let a = spawn(&mut sim, -24);
    let before = sim.snapshot();
    assert!(sim.bind_player(0, a).is_err());
    assert_eq!(before, sim.snapshot());
    assert!(
        sim.damage_actor(DamageRequest {
            target: a,
            source: None,
            inflictor: None,
            amount: 1
        })
        .is_err()
    );
    assert_eq!(before, sim.snapshot());
    assert!(
        sim.spawn_type(ActorTypeId(12), [Fixed::ZERO; 2], SpawnHeight::Floor)
            .is_err()
    );
    assert_eq!(before, sim.snapshot());
    assert!(sim.noise_alert(a, a).is_err());
    assert_eq!(before, sim.snapshot());
    assert!(sim.look(a).is_err());
    assert_eq!(before, sim.snapshot());
    assert!(sim.spawn_map_things().is_err());
    assert_eq!(before, sim.snapshot());
}
#[test]
fn pending_retirement_and_generation_reuse_never_redirect_queries() {
    let mut sim = Simulation::map_scale(fixtures::map(14)).unwrap();
    let old = spawn(&mut sim, -24);
    let other = spawn(&mut sim, 24);
    sim.mark_remove(old).unwrap();
    let pending = sim.snapshot();
    assert!(sim.check_sight(old, other).is_err());
    assert!(
        sim.check_multisector_position(old, [Fixed::ZERO; 2])
            .is_err()
    );
    assert_eq!(pending, sim.snapshot());
    sim.step(zero()).unwrap();
    let replacement = spawn(&mut sim, -24);
    assert_eq!(old.slot(), replacement.slot());
    assert_ne!(old, replacement);
    let before = sim.snapshot();
    assert!(sim.check_sight(old, replacement).is_err());
    assert!(sim.set_momentum(old, [Fixed::ZERO; 2]).is_err());
    assert_eq!(before, sim.snapshot());
}
#[test]
fn late_actor_arithmetic_rejection_preflights_all_actors_before_motion() {
    let mut sim = Simulation::map_scale(fixtures::map(14)).unwrap();
    let first = spawn(&mut sim, -24);
    let edge = spawn(&mut sim, 994);
    sim.set_momentum(first, [units(6), Fixed::ZERO]).unwrap();
    sim.set_momentum(edge, [units(30), Fixed::ZERO]).unwrap();
    sim.step(zero()).unwrap();
    assert_eq!(sim.actor(edge).unwrap().position()[0], units(1024));
    let before = sim.snapshot();
    assert!(sim.step(zero()).is_err());
    assert_eq!(before, sim.snapshot());
}
#[test]
fn budget_choices_pause_and_presentation_reads_do_not_change_outcomes() {
    let mut a = Simulation::map_scale(fixtures::map(14)).unwrap();
    let mut b = Simulation::with_map_scale_limits(
        fixtures::map(14),
        MapScaleLimits {
            max_prepared_references: 8192,
            ..Default::default()
        },
    )
    .unwrap();
    let x = spawn(&mut a, -24);
    let y = spawn(&mut a, 24);
    assert_eq!(x, spawn(&mut b, -24));
    assert_eq!(y, spawn(&mut b, 24));
    a.set_momentum(x, [units(6), Fixed::ZERO]).unwrap();
    b.set_momentum(x, [units(6), Fixed::ZERO]).unwrap();
    for _ in 0..32 {
        let before = a.snapshot();
        a.map_scale_metrics().unwrap();
        a.check_sight(x, y).unwrap();
        a.check_multisector_position(x, [Fixed::ZERO; 2]).unwrap();
        assert_eq!(before, a.snapshot());
        assert_eq!(a.step(zero()).unwrap(), b.step(zero()).unwrap());
    }
    a.set_paused(true);
    let before = a.snapshot();
    let mut invalid = zero();
    invalid[0].buttons = 255;
    assert_eq!(a.step(invalid).unwrap(), before);
    assert_eq!(a.snapshot(), before);
}
#[test]
fn zero_actor_limit_rejects_before_rng_or_spatial_linking() {
    let mut sim = Simulation::with_map_scale_limits(
        fixtures::map(0),
        MapScaleLimits {
            max_actors: 0,
            ..Default::default()
        },
    )
    .unwrap();
    let before = sim.snapshot();
    assert!(sim.spawn(ActorSpec::default()).is_err());
    assert_eq!(before, sim.snapshot());
    assert!(sim.spatial_order().is_empty());
    assert_eq!(sim.rng().indices(), [0, 0]);
    assert!(matches!(
        Simulation::with_map_scale_limits(
            fixtures::map(0),
            MapScaleLimits {
                max_prepared_references: 0,
                ..Default::default()
            }
        ),
        Err(doom_sim::SimError::Geometry(GeometryError::WorkLimit))
    ));
}

#[test]
fn shared_suffix_and_maximum_prepared_work_are_bounded() {
    let shared = Simulation::map_scale(fixtures::map(26)).unwrap();
    assert_eq!(
        shared.map_scale_metrics().unwrap().prepared_references,
        8192
    );
    let suffix = Simulation::map_scale(fixtures::map(27)).unwrap();
    assert!(suffix.world().conservative_omissions().unwrap() > 0);
    let max = Simulation::map_scale(fixtures::map(28)).unwrap();
    assert_eq!(max.map_scale_metrics().unwrap().prepared_references, 262144);
    assert!(
        Simulation::with_map_scale_limits(
            fixtures::map(28),
            MapScaleLimits {
                max_prepared_references: 262143,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut lumps = fixtures::lumps(8);
    let mut blocks = Vec::new();
    for n in [-256_i16, -128, 64, 64] {
        blocks.extend(n.to_le_bytes());
    }
    for _ in 0..4096 {
        blocks.extend(4100_i16.to_le_bytes());
    }
    blocks.extend(0_i16.to_le_bytes());
    for n in (0..1024_i16).rev() {
        blocks.extend(n.to_le_bytes());
    }
    blocks.extend((-1_i16).to_le_bytes());
    lumps[10].1 = blocks;
    assert!(matches!(
        Simulation::map_scale(fixtures::decode(&lumps).unwrap()),
        Err(doom_sim::SimError::Geometry(GeometryError::WorkLimit))
    ));
    let mut lumps = fixtures::lumps(0);
    fixtures::set_word(&mut lumps[10].1, 0, 513);
    assert!(Simulation::map_scale(fixtures::decode(&lumps).unwrap()).is_err());
}
#[test]
fn actor_capacity_sixty_four_is_not_an_implicit_expansion_of_other_profiles() {
    let mut sim = Simulation::map_scale(fixtures::map(0)).unwrap();
    for _ in 0..64 {
        spawn(&mut sim, -24);
    }
    let before = sim.snapshot();
    assert!(sim.spawn(ActorSpec::default()).is_err());
    assert_eq!(before, sim.snapshot());
    sim.step(zero()).unwrap();
    assert_eq!(sim.actor_order().count(), 64);
}
