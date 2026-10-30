//! Rust instance-provenance policy; no C pointer/authentication equivalence.
#[path = "../../../tests/support/m3f9.rs"]
mod fixture;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
use fixture::{
    export,
    fixtures::{BODY, OWNER, id},
    import, setup,
};

fn fresh() -> GameInstance {
    GameInstance::new(setup())
}
fn unavailable(local: ActorId, status: ActorAvailability) -> ProvenanceError {
    ProvenanceError::Context(ContextError::Unavailable(local, status))
}

#[test]
fn independent_same_map_worlds_with_equal_local_ids_reject_foreign_references() {
    let (mut a, mut b) = (fresh(), fresh());
    assert_eq!(a.simulation().snapshot(), b.simulation().snapshot());
    assert_ne!(a.id(), b.id());
    let (ar, br) = (export(&mut a, id(0), OWNER), export(&mut b, id(0), OWNER));
    assert_eq!(ar.local_id(), br.local_id());
    assert_ne!(ar, br);
    let before = b.simulation().snapshot();
    assert_eq!(
        import(&mut b, &ar, OWNER),
        Err(ProvenanceError::InstanceMismatch)
    );
    assert_eq!(b.simulation().snapshot(), before);
}

#[test]
fn references_cross_scopes_and_moves_without_becoming_liveness_leases() {
    let mut a = fresh();
    let key = a.id();
    let reference = export(&mut a, id(1), BODY);
    let mut moved = a;
    assert_eq!(key, moved.id());
    let before = moved.simulation().snapshot();
    for _ in 0..100 {
        import(&mut moved, &reference, BODY).unwrap();
    }
    assert_eq!(moved.simulation().snapshot(), before);
}

#[test]
fn wrapping_cloned_or_extracted_state_never_copies_the_incarnation() {
    let mut a = fresh();
    let reference = export(&mut a, id(0), OWNER);
    let mut branch = GameInstance::new(a.simulation().clone());
    assert_eq!(a.simulation().snapshot(), branch.simulation().snapshot());
    assert_ne!(a.id(), branch.id());
    assert_eq!(
        import(&mut branch, &reference, OWNER),
        Err(ProvenanceError::InstanceMismatch)
    );
    let state = a.into_simulation();
    let mut rewrapped = GameInstance::new(state);
    assert_eq!(
        import(&mut rewrapped, &reference, OWNER),
        Err(ProvenanceError::InstanceMismatch)
    );
    assert_eq!(
        rewrapped.simulation().snapshot(),
        branch.simulation().snapshot()
    );
}

#[test]
fn restart_invalidates_old_live_and_retired_refs_even_when_new_ids_match() {
    let mut game = fresh();
    let owner = export(&mut game, id(0), OWNER);
    let body = export(&mut game, id(1), BODY);
    let old_key = game.id();
    game.with_execution_context(|ctx| {
        let actor = ctx.import_actor(&owner, OWNER)?;
        let target = ctx.import_actor(&body, BODY)?;
        ctx.set_target(actor, None)?;
        ctx.set_perception_player(0, None, false, 100)?;
        ctx.mark_remove(target)?;
        ctx.step([TicCommand::default(); 4])?;
        Ok(())
    })
    .unwrap();
    assert_eq!(
        import(&mut game, &body, BODY),
        Err(unavailable(id(1), ActorAvailability::Retired))
    );
    let expected = game.simulation().restart_demon_map_context().unwrap();
    game.restart().unwrap();
    assert_ne!(old_key, game.id());
    assert_eq!(game.simulation().snapshot(), expected.snapshot());
    assert_eq!(
        game.simulation().actor_order().next(),
        Some(owner.local_id())
    );
    assert_eq!(
        import(&mut game, &owner, OWNER),
        Err(ProvenanceError::InstanceMismatch)
    );
    let replacement = game
        .with_execution_context(|ctx| {
            let body = ctx.spawn_body(
                [Fixed::from_map_units(-200), Fixed::from_map_units(64)],
                SpawnHeight::Floor,
            )?;
            ctx.export_actor(body)
        })
        .unwrap();
    assert_eq!(replacement.local_id(), body.local_id());
    assert_eq!(
        import(&mut game, &body, BODY),
        Err(ProvenanceError::InstanceMismatch)
    );
    import(&mut game, &replacement, BODY).unwrap();
}

#[test]
fn failed_original_profile_restart_preserves_state_rng_and_incarnation() {
    // A valid zero-capacity world has no actors; its selected map THING cannot
    // spawn on restart. This exercises a real recoverable setup failure.
    let map = fixture::fixtures::prior::historical::map(0);
    let sim = Simulation::original_id_map(
        map,
        0,
        ActorOptions::default(),
        MapContextOptions::default(),
    )
    .unwrap();
    let mut game = GameInstance::new(sim);
    let (before, key, rng) = (
        game.simulation().snapshot(),
        game.id(),
        game.simulation().rng(),
    );
    assert!(game.restart().is_err());
    assert_eq!(game.id(), key);
    assert_eq!(game.simulation().rng(), rng);
    assert_eq!(game.simulation().snapshot(), before);
}

#[test]
fn pending_storage_is_observable_but_existing_target_guards_still_apply() {
    let mut game = fresh();
    let owner = export(&mut game, id(0), OWNER);
    let body = export(&mut game, id(1), BODY);
    game.with_execution_context(|ctx| {
        let owner = ctx.import_actor(&owner, OWNER)?;
        let body = ctx.import_actor(&body, BODY)?;
        ctx.mark_remove(body)?;
        let pending = ctx.export_actor(body)?;
        let read = ctx.import_actor(&pending, BODY)?;
        assert_eq!(ctx.availability(read), ActorAvailability::PendingRemoval);
        let before = ctx.snapshot();
        assert!(ctx.set_target(owner, Some(read)).is_err());
        assert_eq!(ctx.snapshot(), before);
        ctx.set_target(owner, None)?;
        ctx.set_perception_player(0, None, false, 100)?;
        ctx.step([TicCommand::default(); 4])?;
        assert_eq!(
            ctx.export_actor(body),
            Err(unavailable(id(1), ActorAvailability::Retired))
        );
        assert_eq!(
            ctx.import_actor(&pending, BODY),
            Err(unavailable(id(1), ActorAvailability::Retired))
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn slot_reuse_keeps_instance_key_but_rejects_the_old_generation() {
    let mut game = fresh();
    let key = game.id();
    let old = export(&mut game, id(1), BODY);
    let replacement = game
        .with_execution_context(|ctx| {
            let owner = ctx.admit_local(id(0), OWNER)?;
            let body = ctx.import_actor(&old, BODY)?;
            ctx.set_target(owner, None)?;
            ctx.set_perception_player(0, None, false, 100)?;
            ctx.mark_remove(body)?;
            ctx.step([TicCommand::default(); 4])?;
            let next = ctx.spawn_body(
                [Fixed::from_map_units(-200), Fixed::from_map_units(64)],
                SpawnHeight::Floor,
            )?;
            ctx.export_actor(next)
        })
        .unwrap();
    assert_eq!(game.id(), key);
    assert_eq!(replacement.local_id(), ActorId::from_parts(1, 2));
    assert_eq!(
        game.simulation().actor_order().last(),
        Some(replacement.local_id())
    );
    assert_eq!(
        import(&mut game, &old, BODY),
        Err(unavailable(id(1), ActorAvailability::StaleGeneration))
    );
    import(&mut game, &replacement, BODY).unwrap();
}

#[test]
fn death_is_not_retirement_and_import_does_not_grant_state_entry_permission() {
    let mut game = fresh();
    let reference = export(&mut game, id(0), OWNER);
    game.with_execution_context(|ctx| {
        let actor = ctx.import_actor(&reference, OWNER)?;
        ctx.damage(actor, None, None, 150)?;
        let corpse = ctx.import_actor(&reference, OWNER)?;
        assert_eq!(ctx.availability(corpse), ActorAvailability::Live);
        assert!(ctx.actor(corpse)?.life().unwrap().health <= 0);
        let before = ctx.snapshot();
        assert!(ctx.enter_state(corpse, OriginalState::S_SARG_RUN1).is_err());
        assert!(
            ctx.enter_state(corpse, OriginalState::S_SARG_RAISE1)
                .is_err()
        );
        assert_eq!(ctx.snapshot(), before);
        Ok(())
    })
    .unwrap();
}

#[test]
fn missing_and_foreign_provenance_reject_before_any_local_handle_lookup() {
    let (mut a, b) = (fresh(), fresh());
    let before = a.simulation().snapshot();
    for local in [id(0), ActorId::from_parts(u32::MAX, 0)] {
        assert_eq!(
            import(&mut a, &InstanceActorRef::candidate(None, local), OWNER),
            Err(ProvenanceError::MissingProvenance)
        );
        assert_eq!(
            import(
                &mut a,
                &InstanceActorRef::candidate(Some(b.id()), local),
                OWNER
            ),
            Err(ProvenanceError::InstanceMismatch)
        );
    }
    assert_eq!(a.simulation().snapshot(), before);
}

#[test]
fn bounded_malformed_handles_and_all_wrong_definition_keys_reject_purely() {
    let mut game = fresh();
    let key = game.id();
    let before = game.simulation().snapshot();
    for slot in (0..=65).chain([u32::MAX]) {
        for generation in [0, 1, 2, u64::MAX] {
            let local = ActorId::from_parts(slot, generation);
            let result = import(
                &mut game,
                &InstanceActorRef::candidate(Some(key.clone()), local),
                OWNER,
            );
            assert_eq!(result.is_ok(), local == id(0));
        }
    }
    let owner = export(&mut game, id(0), OWNER);
    for ty in OriginalActorType::all() {
        assert_eq!(
            import(&mut game, &owner, DefinitionIdentity::Original(ty)).is_ok(),
            ty == OriginalActorType::MT_SERGEANT
        );
    }
    assert!(import(&mut game, &owner, BODY).is_err());
    assert_eq!(game.simulation().snapshot(), before);
}

#[test]
fn an_unissued_vacant_generation_is_not_an_importable_actor() {
    let mut game = fresh();
    game.with_execution_context(|ctx| {
        let owner = ctx.admit_local(id(0), OWNER)?;
        let body = ctx.admit_local(id(1), BODY)?;
        ctx.set_target(owner, None)?;
        ctx.set_perception_player(0, None, false, 100)?;
        ctx.mark_remove(body)?;
        ctx.step([TicCommand::default(); 4])?;
        Ok(())
    })
    .unwrap();
    let unissued = ActorId::from_parts(1, 2);
    let reference = InstanceActorRef::candidate(Some(game.id()), unissued);
    assert_eq!(
        import(&mut game, &reference, BODY),
        Err(unavailable(unissued, ActorAvailability::Vacant))
    );
}

#[test]
fn raw_contexts_have_no_provenance_and_historical_profiles_gain_no_admission() {
    let mut game = fresh();
    let reference = export(&mut game, id(0), OWNER);
    let mut local = setup();
    let before = local.snapshot();
    local
        .with_execution_context(|ctx| {
            let actor = ctx.admit_local(id(0), OWNER)?;
            assert_eq!(
                ctx.export_actor(actor),
                Err(ProvenanceError::UnboundContext)
            );
            assert_eq!(
                ctx.import_actor(&reference, OWNER),
                Err(ProvenanceError::UnboundContext)
            );
            Ok(())
        })
        .unwrap();
    assert_eq!(local.snapshot(), before);
    let legacy = fixture::fixtures::prior::historical::setup(
        &fixture::fixtures::prior::historical::cases()[0],
    );
    let mut legacy = GameInstance::new(legacy);
    let before = legacy.simulation().snapshot();
    let mut called = false;
    assert_eq!(
        legacy.with_execution_context(|_| {
            called = true;
            Ok(())
        }),
        Err(ProvenanceError::Context(ContextError::Profile))
    );
    assert!(!called);
    assert_eq!(legacy.simulation().snapshot(), before);
}

#[test]
fn deliberate_local_readmission_does_not_authenticate_erased_origin() {
    let (mut a, mut b) = (fresh(), fresh());
    let foreign = export(&mut b, id(1), BODY);
    assert_eq!(
        import(&mut a, &foreign, BODY),
        Err(ProvenanceError::InstanceMismatch)
    );
    // Attaching a known local key explicitly changes the interpretation. No API
    // can recover an erased origin or authenticate this local caller's assertion.
    let interpreted = InstanceActorRef::candidate(Some(a.id()), foreign.local_id());
    import(&mut a, &interpreted, BODY).unwrap();
    assert_eq!(format!("{:?}", a.id()), "InstanceId(<opaque>)");
}

#[test]
fn failed_import_does_not_roll_back_successful_earlier_scope_operations() {
    let (mut game, foreign) = (fresh(), fresh());
    let candidate = InstanceActorRef::candidate(Some(foreign.id()), id(0));
    let mut expected = setup();
    expected.set_actor_target(id(0), None).unwrap();
    assert_eq!(
        game.with_execution_context(|ctx| {
            let owner = ctx.admit_local(id(0), OWNER)?;
            ctx.set_target(owner, None)?;
            ctx.import_actor(&candidate, OWNER)?;
            Ok(())
        }),
        Err(ProvenanceError::InstanceMismatch)
    );
    assert_eq!(game.simulation().snapshot(), expected.snapshot());
}

#[test]
fn provenance_reads_and_rejections_preserve_replay_events_rng_and_v21_bytes() {
    let mut baseline = setup();
    let mut game = GameInstance::new(baseline.clone());
    let owner = export(&mut game, id(0), OWNER);
    let bad = InstanceActorRef::candidate(None, id(0));
    for _ in 0..100 {
        let snapshot = baseline.snapshot();
        game.with_execution_context(|ctx| {
            let actor = ctx.import_actor(&owner, OWNER)?;
            let roundtrip = ctx.export_actor(actor)?;
            assert_eq!(roundtrip, owner);
            assert_eq!(
                ctx.import_actor(&bad, OWNER),
                Err(ProvenanceError::MissingProvenance)
            );
            assert_eq!(ctx.snapshot().canonical_bytes(), snapshot.canonical_bytes());
            assert_eq!(ctx.rng(), baseline.rng());
            ctx.step([TicCommand::default(); 4])?;
            Ok(())
        })
        .unwrap();
        let expected = baseline.step([TicCommand::default(); 4]).unwrap();
        assert_eq!(game.simulation().snapshot(), expected);
    }
}

#[test]
fn decoded_v20_v21_local_handles_cannot_restore_instance_provenance() {
    let mut game = fresh();
    let before = game.simulation().snapshot();
    let bytes = before
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    let view = Diagnostic::decode(&bytes, DiagnosticLimits::default()).unwrap();
    assert!(view.actor(id(0)).unwrap().is_some());
    let candidate = InstanceActorRef::candidate(None, id(0));
    assert_eq!(
        import(&mut game, &candidate, OWNER),
        Err(ProvenanceError::MissingProvenance)
    );
    let legacy = fixture::fixtures::prior::historical::setup(
        &fixture::fixtures::prior::historical::cases()[0],
    );
    let bytes = legacy
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    let view = Diagnostic::decode(&bytes, DiagnosticLimits::default()).unwrap();
    assert_eq!(view.version(), 20);
    assert!(view.actor(id(0)).unwrap().is_some());
    assert_eq!(
        import(&mut game, &candidate, OWNER),
        Err(ProvenanceError::MissingProvenance)
    );
    assert_eq!(game.simulation().snapshot(), before);
}
