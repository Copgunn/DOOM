//! Rust admission/safety policy; independent C execution evidence is separate.
#[path = "../../../tests/support/m3f7.rs"]
mod fixtures;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
use fixtures::historical as frozen;
const POLICY: OriginalExecutionProfile = OriginalExecutionProfile::DemonNormalV1;
fn registry() -> OriginalExecutionRegistry {
    OriginalExecutionRegistry::new(POLICY).unwrap()
}
fn setup() -> Simulation {
    fixtures::setup(&fixtures::cases()[0])
}
fn key(ty: OriginalActorType) -> DefinitionIdentity {
    DefinitionIdentity::Original(ty)
}
#[test]
fn all_original_records_are_pristine_and_context_is_not_an_original_sentinel() {
    let r = registry();
    let original = ActorRegistry::vanilla();
    assert_eq!(r.registry().states(), original.states());
    assert_eq!(
        &r.registry().types()[..usize::from(OriginalActorType::COUNT)],
        original.types()
    );
    for ty in OriginalActorType::all() {
        assert_eq!(r.resolve(key(ty)), ActorTypeId(ty.ordinal()));
        assert_eq!(r.identity(r.resolve(key(ty))), Ok(key(ty)));
        assert_eq!(r.registry().original_actor_type(r.resolve(key(ty))), Ok(ty));
    }
    for state in OriginalState::all() {
        assert_eq!(
            r.registry().original_state(StateId(state.ordinal())),
            Ok(state)
        );
    }
    let context = DefinitionIdentity::Context(ContextRole::UnboundBody);
    assert_eq!(r.identity(r.resolve(context)), Ok(context));
    assert!(OriginalActorType::try_from(r.resolve(context).0).is_err());
    assert_eq!(r.registry().types().len(), 138);
    for ty in [OriginalActorType::MT_MISC85, OriginalActorType::MT_MISC86] {
        assert!(r.admit_definition(key(ty)).is_err());
    }
}
#[test]
fn recognition_and_execution_are_separate_for_every_identity() {
    let r = registry();
    for raw in 0..=u16::MAX {
        assert_eq!(OriginalActorType::try_from(raw).is_ok(), raw < 137);
        assert_eq!(OriginalState::try_from(raw).is_ok(), raw < 967);
        assert_eq!(r.identity(ActorTypeId(raw)).is_ok(), raw < 138);
    }
    for ty in OriginalActorType::all() {
        assert_eq!(
            r.admit_definition(key(ty)).is_ok(),
            matches!(
                ty,
                OriginalActorType::MT_SERGEANT
                    | OriginalActorType::MT_PUFF
                    | OriginalActorType::MT_BLOOD
            )
        );
    }
    for state in OriginalState::all() {
        assert_eq!(
            r.admit_state(key(OriginalActorType::MT_SERGEANT), state)
                .is_ok(),
            (475..=495).contains(&state.ordinal())
        );
    }
}
#[test]
fn complete_registry_validation_rejects_modified_records_names_and_graphs() {
    let r = registry();
    for ty in [0, 12, 135, 136, 137] {
        for field in 0..23 {
            let mut types = r.registry().types().to_vec();
            types[ty].properties[field] ^= 1;
            let candidate = ActorRegistry::new(r.registry().states().to_vec(), types).unwrap();
            assert!(OriginalExecutionRegistry::admit(candidate, POLICY).is_err());
        }
    }
    for index in [1, 149, 475, 488, 490, 495, 496, 966] {
        for field in 0..8 {
            let mut states = r.registry().states().to_vec();
            let s = &mut states[index];
            match field {
                0 => s.name.push('_'),
                1 => s.sprite ^= 1,
                2 => s.frame ^= 1,
                3 => s.tics += 1,
                4 => s.action = ActorAction::Probe(ProbeAction::Random),
                5 => s.next = StateId(0),
                6 => s.misc[0] ^= 1,
                _ => s.misc[1] ^= 1,
            }
            // If the next reference was already null, choose a different valid edge.
            if field == 5 && s.next == r.registry().states()[index].next {
                s.next = StateId(1);
            }
            let candidate = ActorRegistry::new(states, r.registry().types().to_vec()).unwrap();
            assert!(OriginalExecutionRegistry::admit(candidate, POLICY).is_err());
        }
    }
    assert!(OriginalExecutionRegistry::admit(ActorRegistry::vanilla(), POLICY).is_err());
    assert!(
        OriginalExecutionRegistry::admit(ActorRegistry::demon_map_context_fixture(), POLICY)
            .is_err()
    );
}
#[test]
fn direct_entry_never_grants_pain_death_raise_foreign_or_native_states() {
    let owner = ActorId::from_parts(0, 1);
    for raw in (0..22).chain(475..=501).chain([u16::MAX]) {
        let mut s = setup();
        let before = s.snapshot();
        let result = s.enter_actor_state(owner, StateId(raw));
        assert_eq!(result.is_ok(), (475..=487).contains(&raw), "raw{raw}");
        if result.is_err() {
            assert_eq!(s.snapshot(), before);
        }
    }
}
#[test]
fn unsupported_actors_and_wrong_profiles_reject_without_mutation() {
    let mut s = setup();
    for ty in [
        OriginalActorType::MT_SHADOWS,
        OriginalActorType::MT_SHOTGUY,
        OriginalActorType::MT_PLAYER,
        OriginalActorType::MT_MISC85,
        OriginalActorType::MT_MISC86,
    ] {
        let before = s.snapshot();
        assert!(s.spawn_original_actor(ty).is_err());
        assert_eq!(s.snapshot(), before);
        assert!(
            s.spawn_type(
                ActorTypeId(ty.ordinal()),
                [Fixed::ZERO; 2],
                SpawnHeight::Floor
            )
            .is_err()
        );
        assert_eq!(s.snapshot(), before);
    }
    let mut old = frozen::setup(&frozen::cases()[0]);
    let before = old.snapshot();
    assert!(
        old.spawn_original_actor(OriginalActorType::MT_SERGEANT)
            .is_err()
    );
    assert!(
        old.spawn_context_body([Fixed::ZERO; 2], SpawnHeight::Floor)
            .is_err()
    );
    assert!(
        old.original_actor_identity(ActorId::from_parts(0, 1))
            .is_err()
    );
    assert_eq!(old.snapshot(), before);
    assert!(
        s.original_demon_identity(ActorId::from_parts(0, 1))
            .is_err()
    );
    assert!(s.native_demon_identity(ActorId::from_parts(0, 1)).is_err());
}
#[test]
fn actual_source_storage_and_context_handle_reuse_are_distinct() {
    let mut s = setup();
    let owner = ActorId::from_parts(0, 1);
    assert_eq!(
        s.actor(owner).unwrap().life().unwrap().actor_type,
        ActorTypeId(12)
    );
    assert_eq!(
        s.actor(owner).unwrap().life().unwrap().state,
        Some(StateId(475))
    );
    s.mark_remove(owner).unwrap();
    s.step([TicCommand::default(); 4]).unwrap();
    let replacement = s
        .spawn_context_body([Fixed::from_map_units(64); 2], SpawnHeight::Floor)
        .unwrap();
    assert_eq!(replacement.slot(), owner.slot());
    assert_eq!(replacement.generation(), 2);
    assert!(s.original_actor_identity(owner).is_err());
    assert_eq!(
        s.original_actor_identity(replacement).unwrap().0,
        DefinitionIdentity::Context(ContextRole::UnboundBody)
    );
    let bytes = s
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    let read = Diagnostic::decode(&bytes, DiagnosticLimits::default()).unwrap();
    assert_eq!(read.actor(owner), Ok(None));
    assert!(matches!(
        read.actor(replacement),
        Ok(Some(DiagnosticIdentity::Original {
            definition: DefinitionIdentity::Context(_),
            ..
        }))
    ));
}
#[test]
fn original_observation_queries_preserve_replay_rng_and_order() {
    let (mut a, mut b) = (setup(), setup());
    for op in 0..=80 {
        let before = a.snapshot();
        for id in a.actor_order() {
            a.original_actor_identity(id).unwrap();
        }
        assert_eq!(before, a.snapshot());
        assert_eq!(a.snapshot(), b.snapshot());
        assert_eq!(
            a.actor_order().collect::<Vec<_>>(),
            b.actor_order().collect::<Vec<_>>()
        );
        if op < 80 {
            fixtures::advance(&mut a, &fixtures::cases()[0], op + 1);
            fixtures::advance(&mut b, &fixtures::cases()[0], op + 1);
        }
    }
}
#[test]
fn restart_keeps_original_registry_and_raw_spawn_contract() {
    let s = setup();
    let restart = s.restart_demon_map_context().unwrap();
    assert_eq!(restart.profile(), Profile::OriginalDemon);
    assert_eq!(
        restart
            .actor(ActorId::from_parts(0, 1))
            .unwrap()
            .life()
            .unwrap()
            .actor_type,
        ActorTypeId(12)
    );
}

#[test]
fn original_profile_keeps_existing_resource_and_context_exclusions() {
    for limit in [65, u32::MAX] {
        assert!(
            Simulation::original_id_map(
                fixtures::map(0),
                limit,
                ActorOptions::default(),
                MapContextOptions::default()
            )
            .is_err()
        );
    }
    assert!(
        Simulation::original_id_map(
            fixtures::map(0),
            64,
            ActorOptions {
                skill: Skill::Nightmare,
                ..Default::default()
            },
            MapContextOptions::default()
        )
        .is_err()
    );
    assert!(
        Simulation::original_id_map(
            fixtures::map(0),
            64,
            ActorOptions::default(),
            MapContextOptions {
                respawn_monsters: true,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut tiny = Simulation::original_id_map(
        fixtures::map(0),
        0,
        ActorOptions::default(),
        MapContextOptions::default(),
    )
    .unwrap();
    let before = tiny.snapshot();
    assert!(
        tiny.spawn_original_actor(OriginalActorType::MT_SERGEANT)
            .is_err()
    );
    assert_eq!(tiny.snapshot(), before);
}
#[test]
fn rejected_commands_and_hitscan_preserve_complete_original_diagnostics() {
    let mut s = setup();
    let before = s.snapshot();
    assert!(
        s.step(
            [TicCommand {
                buttons: 255,
                ..Default::default()
            }; 4]
        )
        .is_err()
    );
    assert_eq!(s.snapshot(), before);
    assert!(
        s.line_attack(
            ActorId::from_parts(1, 1),
            LineAttack {
                angle: 0,
                range: Fixed::from_map_units(512),
                slope: Fixed::ZERO,
                damage: 1
            }
        )
        .is_err()
    );
    assert_eq!(s.snapshot(), before);
    s.set_paused(true);
    let paused = s.snapshot();
    assert_eq!(
        s.step(
            [TicCommand {
                buttons: 255,
                ..Default::default()
            }; 4]
        )
        .unwrap(),
        paused
    );
}
