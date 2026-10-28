//! Bounded graph, failure atomicity and safe lifecycle policy regressions.
#[path = "../../../tests/support/m3c.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
fn native(limit: u32) -> Simulation {
    Simulation::with_actor_registry(
        fixtures::map(&[]),
        limit,
        fixtures::native_registry(6),
        ActorOptions::default(),
    )
    .unwrap()
}
fn spawn(sim: &mut Simulation, ty: u16) -> ActorId {
    sim.spawn_type(ActorTypeId(ty), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap()
}
fn change_registry(
    edit: impl FnOnce(&mut Vec<ActorStateDef>, &mut Vec<ActorTypeDef>),
) -> Result<ActorRegistry, ActorError> {
    let r = fixtures::native_registry(6);
    let mut s = r.states().to_vec();
    let mut t = r.types().to_vec();
    edit(&mut s, &mut t);
    ActorRegistry::new(s, t)
}
#[test]
fn invalid_metadata_and_every_reference_are_rejected() {
    for field in [1, 3, 7, 10, 11, 12, 13, 22] {
        assert_eq!(
            change_registry(|_, t| t[131].properties[field] = 65536).unwrap_err(),
            ActorError::InvalidState
        );
    }
    for field in [4, 6, 9, 14, 20] {
        assert_eq!(
            change_registry(|_, t| t[131].properties[field] = -1).unwrap_err(),
            ActorError::InvalidMetadata
        );
    }
    for (field, error) in [
        (0, ActorError::InvalidMetadata),
        (1, ActorError::InvalidState),
        (2, ActorError::InvalidMetadata),
        (3, ActorError::InvalidState),
    ] {
        assert_eq!(
            change_registry(|s, _| match field {
                0 => s[1].tics = -2,
                1 => s[1].next = StateId(65535),
                2 => s[1].action = ActorAction::Original(ActionId(65535)),
                _ => s[0].tics = 0,
            })
            .unwrap_err(),
            error
        );
    }
}
#[test]
fn unsupported_spawns_entries_and_bad_handles_preserve_all_state() {
    let mut sim = native(1);
    let id = spawn(&mut sim, 132);
    let before = sim.snapshot();
    for result in [
        sim.spawn_type(ActorTypeId(65535), [Fixed::ZERO; 2], SpawnHeight::Floor),
        sim.spawn_type(ActorTypeId(1), [Fixed::ZERO; 2], SpawnHeight::Floor),
        sim.spawn_type(ActorTypeId(132), [Fixed::ZERO; 2], SpawnHeight::Floor),
    ] {
        assert!(result.is_err());
    }
    assert!(sim.enter_actor_state(id, StateId(65535)).is_err());
    assert!(
        sim.enter_actor_state(
            id,
            sim.actor_registry()
                .unwrap()
                .find_state("S_POSS_STND")
                .unwrap()
        )
        .is_err()
    );
    assert!(
        sim.set_actor_target(id, Some(ActorId::from_parts(0, 99)))
            .is_err()
    );
    assert!(sim.request_actor_death(id).is_err());
    assert!(sim.spawn(ActorSpec::default()).is_err());
    assert_eq!(sim.snapshot(), before);
}
#[test]
fn zero_and_callback_cycles_and_long_entry_work_fail_before_rng() {
    for callback in [false, true] {
        let registry = change_registry(|s, _| {
            s[1].tics = 0;
            s[1].next = StateId(1);
            if callback {
                s[1].tics = 1;
                s[1].action = ActorAction::Probe(ProbeAction::Enter(StateId(1)));
            }
        })
        .unwrap();
        let mut sim = Simulation::with_actor_registry(
            fixtures::map(&[]),
            8,
            registry,
            ActorOptions::default(),
        )
        .unwrap();
        let before = sim.snapshot();
        assert_eq!(
            sim.spawn_type(ActorTypeId(131), [Fixed::ZERO; 2], SpawnHeight::Floor),
            Err(SimError::Actor(ActorError::ImmediateCycle))
        );
        assert_eq!(sim.snapshot(), before);
    }
    let registry = change_registry(|s, _| {
        for (i, state) in s.iter_mut().enumerate().take(131).skip(1) {
            state.tics = 0;
            state.action = ActorAction::None;
            state.next = StateId(if i == 130 { 0 } else { i as u16 + 1 });
        }
    })
    .unwrap();
    assert_eq!(
        registry.validate_state(StateId(1)),
        Err(ActorError::WorkLimit)
    );
    let r = change_registry(|s, _| {
        s[1].tics = 1;
        s[1].next = StateId(1);
        s[1].action = ActorAction::None;
    })
    .unwrap();
    assert!(r.validate_state(StateId(1)).is_ok());
}
#[test]
fn step_and_external_entry_capacity_errors_are_atomic() {
    let mut sim = native(1);
    let id = spawn(&mut sim, 131);
    let before = sim.snapshot();
    assert_eq!(
        sim.step([TicCommand::default(); 4]),
        Err(SimError::Store(StoreError::Capacity))
    );
    assert_eq!(sim.snapshot(), before);
    assert_eq!(
        sim.enter_actor_state(id, StateId(2)),
        Err(SimError::Store(StoreError::Capacity))
    );
    assert_eq!(sim.snapshot(), before);
    sim.set_paused(true);
    let before = sim.snapshot();
    assert_eq!(sim.step([TicCommand::default(); 4]).unwrap(), before);
}
#[test]
fn batch_map_failure_is_atomic_and_filters_precede_admission() {
    // P_SpawnMapThing compares the source type identity, not its display name.
    let original = ActorRegistry::vanilla();
    let skull = original.find_type("MT_SKULL").unwrap();
    let mut types = original.types().to_vec();
    let info = &mut types[usize::from(skull.0)];
    info.name = "RENAMED_SKULL".into();
    info.properties[21] = 0;
    info.properties[1] = 149;
    info.properties[12] = 0;
    let registry = ActorRegistry::new(original.states().to_vec(), types).unwrap();
    let mut identity = Simulation::with_actor_registry(
        fixtures::map(&[[0, 0, 0, 3006, 7]]),
        4,
        registry,
        ActorOptions {
            no_monsters: true,
            ..Default::default()
        },
    )
    .unwrap();
    let report = identity.spawn_map_things().unwrap();
    assert!(report.spawned.is_empty());
    assert_eq!(report.filtered, 1);
    assert_eq!(identity.rng().indices(), [0, 0]);

    let things = [[0, 0, 0, 44, 7], [0, 0, 0, 3004, 7]];
    let mut sim = Simulation::actor_states(fixtures::map(&things)).unwrap();
    let before = sim.snapshot();
    assert!(sim.spawn_map_things().is_err());
    assert_eq!(sim.snapshot(), before);
    let mut sim = Simulation::with_actor_registry(
        fixtures::map(&things),
        1,
        ActorRegistry::vanilla(),
        ActorOptions {
            no_monsters: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(sim.spawn_map_things().unwrap().spawned.len(), 1);
    let before = sim.snapshot();
    assert!(sim.spawn_map_things().is_err());
    assert_eq!(sim.snapshot(), before);
    let sim = Simulation::with_actor_registry(
        fixtures::map(&[]),
        10,
        ActorRegistry::vanilla(),
        ActorOptions {
            mode: ActorGameMode::Deathmatch,
            ..Default::default()
        },
    );
    assert!(matches!(
        sim,
        Err(SimError::Actor(ActorError::UnsupportedMode))
    ));
}
#[test]
fn retained_self_mutual_and_stale_references_never_alias_reuse() {
    let mut sim = Simulation::actor_states(fixtures::map(&[])).unwrap();
    let a = spawn(&mut sim, 0);
    let b = spawn(&mut sim, 0);
    sim.set_actor_target(a, Some(a)).unwrap();
    assert!(sim.actor_target(a).is_some());
    sim.set_actor_target(a, Some(b)).unwrap();
    sim.set_actor_target(b, Some(a)).unwrap();
    sim.mark_remove(b).unwrap();
    assert!(sim.actor_target(a).is_some());
    sim.step([TicCommand::default(); 4]).unwrap();
    assert!(sim.actor_target(a).is_none());
    assert_eq!(sim.actor(a).unwrap().life().unwrap().target, Some(b));
    let c = spawn(&mut sim, 0);
    assert_eq!(c.slot(), b.slot());
    assert_ne!(c.generation(), b.generation());
    assert!(sim.actor_target(a).is_none());
    let before = sim.snapshot();
    assert!(sim.set_actor_target(a, Some(b)).is_err());
    assert_eq!(sim.snapshot(), before);
}
#[test]
fn actions_run_after_xy_and_z_and_children_think_in_same_pass() {
    let mut sim = native(4);
    let id = sim
        .spawn_type(
            ActorTypeId(131),
            [Fixed::ZERO; 2],
            SpawnHeight::Absolute(Fixed::from_map_units(32)),
        )
        .unwrap();
    sim.set_momentum(id, [Fixed::ONE, Fixed::ZERO]).unwrap();
    sim.step([TicCommand::default(); 4]).unwrap();
    let events = sim.events();
    let xy = events
        .iter()
        .position(|e| matches!(e,Event::SectorMovement{actor,..} if *actor==id))
        .unwrap();
    let z = events
        .iter()
        .position(|e| matches!(e,Event::Vertical{actor,..} if *actor==id))
        .unwrap();
    let state = events
        .iter()
        .position(
            |e| matches!(e,Event::ActorState(ActorStateEvent::Entered{actor,..}) if *actor==id),
        )
        .unwrap();
    assert!(xy < z && z < state);
    let child = sim.actor(id).unwrap().life().unwrap().target.unwrap();
    assert_eq!(
        sim.actor(child).unwrap().position(),
        sim.actor(id).unwrap().position()
    );
    assert_eq!(sim.actor(child).unwrap().life().unwrap().target, Some(id));
    assert!(events.contains(&Event::Think(child)));
    assert!(sim.spatial_order().is_empty());
    sim.step([TicCommand::default(); 4]).unwrap();
    assert!(sim.actor(id).is_none());
    assert!(sim.actor(child).is_none());
}
#[test]
fn controlled_death_persists_and_null_removal_preserves_source_timers() {
    let mut sim = Simulation::actor_states(fixtures::map(&[])).unwrap();
    let id = spawn(&mut sim, 0);
    sim.bind_player(0, id).unwrap();
    let before = sim.snapshot();
    assert_eq!(
        sim.request_actor_death(id),
        Err(SimError::Actor(ActorError::BoundDeath))
    );
    assert_eq!(sim.snapshot(), before);
    let other = spawn(&mut sim, 0);
    sim.request_actor_death(other).unwrap();
    for _ in 0..65 {
        sim.step([TicCommand::default(); 4]).unwrap();
    }
    let a = sim.actor(other).unwrap();
    assert_eq!(a.life().unwrap().tics, -1);
    assert_eq!(a.life().unwrap().health, 100);
    assert_eq!(a.solidity(), Solidity::NonSolid);
    assert_eq!(a.life().unwrap().flags & 2, 0);
    assert_eq!(a.height(), Fixed::from_map_units(56));
    let before = *a;
    sim.enter_actor_state(other, StateId(0)).unwrap();
    let a = sim.actor(other).unwrap();
    assert_eq!(a.life().unwrap().state, None);
    assert_eq!(a.life().unwrap().tics, before.life().unwrap().tics);
    assert_eq!(a.life().unwrap().sprite, before.life().unwrap().sprite);
    assert!(!sim.spatial_order().contains(&other));
    sim.step([TicCommand::default(); 4]).unwrap();
    assert!(sim.actor(other).is_none());
}
#[test]
fn source_height_sentinels_and_no_sector_or_blockmap_flags_are_explicit() {
    let mut sim = Simulation::actor_states(fixtures::map(&[])).unwrap();
    let fog = sim
        .spawn_type(ActorTypeId(39), [Fixed::ZERO; 2], SpawnHeight::Ceiling)
        .unwrap();
    assert_eq!(sim.actor(fog).unwrap().z(), Fixed::from_map_units(112));
    let p = spawn(&mut sim, 0);
    let attempt = sim.check_multisector_position(p, [Fixed::ZERO; 2]).unwrap();
    assert!(!attempt.movement.actors.contains(&fog));
    let registry = change_registry(|s, t| {
        s[1].tics = -1;
        s[1].next = StateId(0);
        t[131].properties[21] = 512 | 8;
    })
    .unwrap();
    let mut sim =
        Simulation::with_actor_registry(fixtures::map(&[]), 4, registry, ActorOptions::default())
            .unwrap();
    let id = spawn(&mut sim, 131);
    assert!(sim.spatial_order().contains(&id));
    assert!(sim.sector_actor_order(0).unwrap().is_empty());
}
#[test]
fn repeat_traces_and_named_first_divergence_include_new_state() {
    let mut a = fixtures::setup(0);
    let mut b = a.clone();
    for _ in 0..300 {
        let x = a.step([TicCommand::default(); 4]).unwrap();
        let y = b.step([TicCommand::default(); 4]).unwrap();
        assert!(x.first_difference(&y).is_none());
    }
    let id = ActorId::from_parts(0, 1);
    let before = a.snapshot();
    a.set_actor_target(id, Some(id)).unwrap();
    let difference = before.first_difference(&a.snapshot()).unwrap();
    assert_eq!(difference.field, "actor.0.target");
}
#[test]
fn bounded_synthetic_graph_property_regression() {
    let mut seed = 1_u32;
    let mut accepted = 0;
    let mut rejected = 0;
    for _ in 0..256 {
        let r = change_registry(|s, t| {
            t[131].properties[1] = 1;
            for state in s.iter_mut().take(9).skip(1) {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                state.tics = (seed % 4) as i32 - 1;
                state.next = StateId(((seed >> 8) % 9) as u16);
                state.action = if seed & 16 == 0 {
                    ActorAction::None
                } else {
                    ActorAction::Probe(ProbeAction::Random)
                };
            }
        })
        .unwrap();
        let mut a =
            Simulation::with_actor_registry(fixtures::map(&[]), 8, r, ActorOptions::default())
                .unwrap();
        let before = a.snapshot();
        match a.spawn_type(ActorTypeId(131), [Fixed::ZERO; 2], SpawnHeight::Floor) {
            Ok(_) => {
                accepted += 1;
                let mut b = a.clone();
                for _ in 0..12 {
                    assert_eq!(
                        a.step([TicCommand::default(); 4]),
                        b.step([TicCommand::default(); 4])
                    );
                }
            }
            Err(_) => {
                rejected += 1;
                assert_eq!(a.snapshot(), before);
            }
        }
    }
    assert!(accepted > 100 && rejected > 0);
}
#[test]
fn aggregate_step_work_and_spawn_cycles_are_bounded_before_mutation() {
    let registry = change_registry(|s, _| {
        s[1].tics = 1;
        s[1].next = StateId(2);
        s[1].action = ActorAction::None;
        for (i, state) in s.iter_mut().enumerate().take(130).skip(2) {
            state.tics = if i == 129 { -1 } else { 0 };
            state.next = StateId(if i == 129 { 0 } else { i as u16 + 1 });
            state.action = ActorAction::Probe(ProbeAction::Random);
        }
    })
    .unwrap();
    let mut sim =
        Simulation::with_actor_registry(fixtures::map(&[]), 128, registry, ActorOptions::default())
            .unwrap();
    for _ in 0..65 {
        spawn(&mut sim, 131);
    }
    let before = sim.snapshot();
    assert_eq!(
        sim.step([TicCommand::default(); 4]),
        Err(SimError::Actor(ActorError::WorkLimit))
    );
    assert_eq!(sim.snapshot(), before);
    let registry = change_registry(|s, _| {
        s[3].action = ActorAction::Probe(ProbeAction::SpawnTarget(ActorTypeId(131)))
    })
    .unwrap();
    assert_eq!(
        registry.validate_type(ActorTypeId(131), sim.sectors()),
        Err(ActorError::SpawnCycle)
    );
}
#[test]
fn source_admission_matrix_is_explicit() {
    let sim = Simulation::actor_states(fixtures::map(&[])).unwrap();
    let registry = sim.actor_registry().unwrap();
    let admitted = registry
        .types()
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            registry
                .validate_type(ActorTypeId(i as u16), sim.sectors())
                .ok()
                .map(|_| t.name.as_str())
        })
        .collect::<Vec<_>>();
    assert_eq!(admitted.len(), 67);
}
