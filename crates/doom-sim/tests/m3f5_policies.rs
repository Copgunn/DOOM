//! Rust policy tests, separate from independent original-C comparisons.
#[path = "../../../tests/support/m3f5.rs"]
mod fixtures;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
fn setup() -> Simulation {
    fixtures::setup(&fixtures::cases()[0])
}
fn atomic<T>(
    sim: &mut Simulation,
    action: impl FnOnce(&mut Simulation) -> Result<T, CanonicalDemonError>,
) {
    let before = sim.snapshot();
    assert!(action(sim).is_err());
    assert_eq!(sim.snapshot(), before);
}
#[test]
fn binding_is_bijective_without_spectre_shotgunner_or_raise_admission() {
    let registry = ActorRegistry::demon_map_context_fixture();
    let b = DemonExecutionBinding::admit(&registry).unwrap();
    assert_eq!(b.original_type(), OriginalActorType::MT_SERGEANT);
    assert_eq!(b.runtime_definition().properties[22], 0);
    assert_eq!(
        OriginalActorType::MT_SERGEANT.states().raise,
        OriginalState::S_SARG_RAISE1
    );
    assert!(registry.original_actor_type(ActorTypeId(136)).is_err());
    assert_eq!(
        registry.original_actor_type(ActorTypeId(12)),
        Ok(OriginalActorType::MT_SERGEANT)
    );
    let mut canonical = std::collections::BTreeSet::new();
    let mut native = std::collections::BTreeSet::new();
    for (original, runtime) in b.states() {
        assert!(canonical.insert(original));
        assert!(native.insert(runtime));
        assert_eq!(b.runtime_state(original), Ok(runtime));
        assert_eq!(b.original_state(runtime), Ok(original));
    }
    assert_eq!(canonical.len(), 21);
    for state in OriginalState::all() {
        assert_eq!(b.runtime_state(state).is_ok(), canonical.contains(&state));
    }
    for ty in OriginalActorType::all() {
        assert_eq!(
            b.runtime_type(ty).is_ok(),
            ty == OriginalActorType::MT_SERGEANT
        );
    }
    for raw in [0, 22, 475, 496, 967, u16::MAX] {
        assert!(b.original_state(StateId(raw)).is_err());
    }
}
#[test]
fn all_source_and_runtime_actor_properties_are_part_of_the_proof() {
    let original = ActorRegistry::demon_map_context_fixture();
    for ty in [12, 136] {
        for field in 0..23 {
            let mut types = original.types().to_vec();
            types[ty].properties[field] ^= 1;
            let registry = ActorRegistry::new(original.states().to_vec(), types).unwrap();
            assert!(
                matches!(
                    DemonExecutionBinding::admit(&registry),
                    Err(CanonicalDemonError::Registry)
                ),
                "type{ty} field{field}"
            );
        }
    }
    assert!(DemonExecutionBinding::admit(&ActorRegistry::vanilla()).is_err());
}
#[test]
fn all_bound_source_and_runtime_state_fields_are_part_of_the_proof() {
    let original = ActorRegistry::demon_map_context_fixture();
    let b = DemonExecutionBinding::admit(&original).unwrap();
    for (canonical, native) in b.states() {
        for index in [canonical.ordinal(), native.0] {
            for field in 0..8 {
                let mut states = original.states().to_vec();
                let s = &mut states[usize::from(index)];
                match field {
                    0 => s.name.push('_'),
                    1 => s.sprite ^= 1,
                    2 => s.frame ^= 1,
                    3 => s.tics += 1,
                    4 => s.action = ActorAction::Probe(ProbeAction::Random),
                    5 => s.next = StateId(if s.next.0 == 0 { 1 } else { 0 }),
                    6 => s.misc[0] ^= 1,
                    7 => s.misc[1] ^= 1,
                    _ => unreachable!(),
                }
                let registry = ActorRegistry::new(states, original.types().to_vec()).unwrap();
                assert!(
                    DemonExecutionBinding::admit(&registry).is_err(),
                    "state{index} field{field}"
                );
            }
        }
    }
}
#[test]
fn canonical_requests_reject_wrong_owners_states_types_and_profiles_atomically() {
    let mut sim = setup();
    let owner = ActorId::from_parts(0, 1);
    let body = ActorId::from_parts(1, 1);
    for ty in [
        OriginalActorType::MT_SHADOWS,
        OriginalActorType::MT_SHOTGUY,
        OriginalActorType::MT_PLAYER,
    ] {
        atomic(&mut sim, |s| s.spawn_original_demons(ty));
    }
    for state in [
        OriginalState::S_NULL,
        OriginalState::S_SARG_RAISE1,
        OriginalState::S_SARG_RAISE6,
        OriginalState::S_POSS_STND,
        OriginalState::S_SARG_PAIN,
        OriginalState::S_SARG_DIE1,
    ] {
        atomic(&mut sim, |s| s.enter_original_demon_state(owner, state));
    }
    atomic(&mut sim, |s| {
        s.enter_original_demon_state(body, OriginalState::S_SARG_ATK1)
    });
    atomic(&mut sim, |s| {
        s.enter_original_demon_state(ActorId::from_parts(0, 2), OriginalState::S_SARG_ATK1)
    });
    assert!(sim.original_demon_identity(body).is_err());
    let mut old = Simulation::actor_states(fixtures::map(0)).unwrap();
    atomic(&mut old, |s| {
        s.spawn_original_demons(OriginalActorType::MT_SERGEANT)
    });
    atomic(&mut old, |s| {
        s.enter_original_demon_state(owner, OriginalState::S_SARG_STND)
    });
}
#[test]
fn queries_and_replays_preserve_complete_state_and_rng() {
    let c = &fixtures::cases()[0];
    let mut queried = fixtures::setup(c);
    let mut control = fixtures::setup(c);
    let owner = ActorId::from_parts(0, 1);
    for op in 1..=80 {
        let before = queried.snapshot();
        let b = queried.canonical_demon_binding().unwrap();
        for (original, native) in b.states() {
            assert_eq!(b.runtime_state(original), Ok(native));
        }
        let (ty, state) = queried.original_demon_identity(owner).unwrap();
        assert_eq!(ty, OriginalActorType::MT_SERGEANT);
        assert!(state.is_some());
        assert_eq!(before, queried.snapshot());
        fixtures::advance(&mut queried, c, op);
        fixtures::advance(&mut control, c, op);
        assert_eq!(queried.snapshot(), control.snapshot());
        fixtures::assert_golden(&queried, 0, op);
    }
}
#[test]
fn corpses_cannot_be_resurrected_by_canonical_state_requests() {
    let c = &fixtures::cases()[1];
    let mut sim = fixtures::setup(c);
    let owner = ActorId::from_parts(0, 1);
    for op in 1..=c.v("tics") as usize {
        fixtures::advance(&mut sim, c, op);
    }
    assert_eq!(
        sim.original_demon_identity(owner).unwrap().1,
        Some(OriginalState::S_SARG_DIE6)
    );
    for state in [
        OriginalState::S_SARG_STND,
        OriginalState::S_SARG_RUN1,
        OriginalState::S_SARG_RAISE1,
    ] {
        atomic(&mut sim, |s| s.enter_original_demon_state(owner, state));
    }
    let rng = sim.rng();
    for _ in 0..40 {
        sim.step([TicCommand::default(); 4]).unwrap();
    }
    assert_eq!(sim.rng(), rng);
    assert_eq!(sim.actor(owner).unwrap().life().unwrap().tics, -1);
}
#[test]
fn retirement_and_slot_reuse_never_redirect_a_canonical_owner_request() {
    let mut sim = setup();
    let owner = ActorId::from_parts(0, 1);
    sim.mark_remove(owner).unwrap();
    assert_eq!(
        sim.original_demon_identity(owner).unwrap().0,
        OriginalActorType::MT_SERGEANT
    );
    atomic(&mut sim, |s| {
        s.enter_original_demon_state(owner, OriginalState::S_SARG_STND)
    });
    sim.step([TicCommand::default(); 4]).unwrap();
    assert!(sim.original_demon_identity(owner).is_err());
    let body = sim
        .spawn_type(
            ActorTypeId(135),
            [Fixed::from_map_units(32), Fixed::from_map_units(64)],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(body.slot(), owner.slot());
    assert_ne!(body.generation(), owner.generation());
    assert!(sim.original_demon_identity(body).is_err());
    atomic(&mut sim, |s| {
        s.enter_original_demon_state(owner, OriginalState::S_SARG_STND)
    });
    atomic(&mut sim, |s| {
        s.enter_original_demon_state(body, OriginalState::S_SARG_STND)
    });
}
fn thing_map(kind: i16) -> doom_map::Map {
    let mut bytes = fixtures::map_bytes(0).to_vec();
    let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let directory = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    for i in 0..count {
        let entry = directory + 16 * i;
        if &bytes[entry + 8..entry + 16] == b"THINGS\0\0" {
            let start = u32::from_le_bytes(bytes[entry..entry + 4].try_into().unwrap()) as usize;
            bytes[start + 6..start + 8].copy_from_slice(&kind.to_le_bytes());
        }
    }
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
#[test]
fn original_map_filters_precede_selected_type_admission_without_granting_spectre() {
    for kind in [58, 9] {
        for no_monsters in [false, true] {
            let mut sim = Simulation::canonical_demon_map(
                thing_map(kind),
                64,
                ActorOptions {
                    no_monsters,
                    ..Default::default()
                },
                MapContextOptions::default(),
            )
            .unwrap();
            if no_monsters {
                let r = sim
                    .spawn_original_demons(OriginalActorType::MT_SERGEANT)
                    .unwrap();
                assert_eq!(r.filtered, 1);
                assert!(r.spawned.is_empty());
            } else {
                atomic(&mut sim, |s| {
                    s.spawn_original_demons(OriginalActorType::MT_SERGEANT)
                });
            }
            assert_eq!(sim.rng(), doom_types::DoomRng::default());
        }
    }
    let mut full = Simulation::canonical_demon_map(
        fixtures::map(0),
        0,
        ActorOptions::default(),
        MapContextOptions::default(),
    )
    .unwrap();
    atomic(&mut full, |s| {
        s.spawn_original_demons(OriginalActorType::MT_SERGEANT)
    });
}
