//! Rust-only ownership, safety and API policy. These are not C pointer proofs.
#[path = "../../../tests/support/m3f8.rs"]
mod fixtures;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
use fixtures::{BODY, OWNER, id};
fn setup() -> Simulation {
    fixtures::setup(0, &fixtures::cases()[0])
}

#[test]
fn availability_covers_invalid_pending_retired_vacant_stale_and_reused_slots() {
    let mut store = ThinkerStore::new(2).unwrap();
    assert_eq!(store.availability(id(0)), ActorAvailability::OutOfRange);
    let first = store.spawn(10).unwrap();
    for slot in [0, 1, 2, u32::MAX] {
        assert_eq!(
            store.availability(ActorId::from_parts(slot, 0)),
            ActorAvailability::InvalidGeneration
        );
    }
    assert_eq!(store.availability(first), ActorAvailability::Live);
    assert!(store.mark_remove(first).unwrap());
    assert!(!store.mark_remove(first).unwrap());
    assert_eq!(store.availability(first), ActorAvailability::PendingRemoval);
    let mut visits = Vec::new();
    store
        .run(2, |visit, _| {
            visits.push(visit);
        })
        .unwrap();
    assert_eq!(visits, vec![Visit::Retired(first)]);
    assert_eq!(store.availability(first), ActorAvailability::Retired);
    assert_eq!(
        store.availability(ActorId::from_parts(first.slot(), 2)),
        ActorAvailability::Vacant
    );
    let second = store.spawn(20).unwrap();
    assert_eq!(second, ActorId::from_parts(first.slot(), 2));
    assert_eq!(
        store.availability(first),
        ActorAvailability::StaleGeneration
    );
    assert_eq!(store.get(first), None);
    assert_eq!(store.get(second), Some(&20));
    assert!(store.mark_remove(first).is_err());
}

#[test]
fn local_admission_exhausts_definition_keys_and_bounded_invalid_candidates() {
    let mut sim = setup();
    let before = sim.snapshot();
    sim.with_execution_context(|ctx| {
        for ty in OriginalActorType::all() {
            assert_eq!(
                ctx.admit_local(id(0), DefinitionIdentity::Original(ty))
                    .is_ok(),
                ty == OriginalActorType::MT_SERGEANT
            );
        }
        assert!(ctx.admit_local(id(0), BODY).is_err());
        assert!(ctx.admit_local(id(1), OWNER).is_err());
        for slot in (0..=65).chain([u32::MAX]) {
            for generation in [0, 1, 2, u64::MAX] {
                let raw = ActorId::from_parts(slot, generation);
                assert_eq!(ctx.admit_local(raw, OWNER).is_ok(), raw == id(0));
            }
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(sim.snapshot(), before);
}

#[test]
fn observations_are_pure_and_tokens_add_no_persistent_identity() {
    assert_eq!(
        std::mem::size_of::<ContextActor<'static>>(),
        std::mem::size_of::<ActorId>()
    );
    let mut sim = setup();
    let before = sim.snapshot();
    let rng = sim.rng();
    sim.with_execution_context(|ctx| {
        let owner = ctx.admit_local(id(0), OWNER)?;
        let body = ctx.admit_local(id(1), BODY)?;
        for _ in 0..1000 {
            assert_eq!(ctx.availability(owner), ActorAvailability::Live);
            assert_eq!(
                ctx.actor(owner)?.life().unwrap().actor_type,
                ActorTypeId(OriginalActorType::MT_SERGEANT.ordinal())
            );
            assert_eq!(
                ctx.target(owner)?,
                TargetReference::Available {
                    actor: body,
                    health: 1000,
                    shootable: true
                }
            );
        }
        assert_eq!(ctx.rng(), rng);
        Ok(())
    })
    .unwrap();
    assert_eq!(sim.snapshot(), before);
}

#[test]
fn replacement_clear_and_role_dependent_self_policy_match_existing_service() {
    let (mut scoped, mut raw) = (setup(), setup());
    scoped
        .with_execution_context(|ctx| {
            let owner = ctx.admit_local(id(0), OWNER)?;
            let body = ctx.admit_local(id(2), BODY)?;
            ctx.set_target(owner, Some(body))?;
            let before = ctx.snapshot();
            assert!(ctx.set_target(owner, Some(owner)).is_err());
            assert_eq!(ctx.snapshot(), before);
            ctx.set_target(owner, None)?;
            assert_eq!(ctx.target(owner)?, TargetReference::Absent);
            ctx.set_target(body, Some(body))?;
            assert!(
                matches!(ctx.target(body)?,TargetReference::Available {actor,..} if actor==body)
            );
            Ok(())
        })
        .unwrap();
    raw.set_actor_target(id(0), Some(id(2))).unwrap();
    raw.set_actor_target(id(0), None).unwrap();
    raw.set_actor_target(id(2), Some(id(2))).unwrap();
    assert_eq!(scoped.snapshot(), raw.snapshot());
}

#[test]
fn pending_target_retention_failed_tick_and_explicit_repair_are_atomic() {
    let mut sim = setup();
    sim.with_execution_context(|ctx| {
        let owner = ctx.admit_local(id(0), OWNER)?;
        let body = ctx.admit_local(id(1), BODY)?;
        ctx.mark_remove(body)?;
        let before = ctx.snapshot();
        ctx.mark_remove(body)?;
        assert_eq!(ctx.snapshot(), before);
        assert_eq!(
            ctx.target(owner)?,
            TargetReference::Unavailable {
                stored: body.raw_id(),
                status: ActorAvailability::PendingRemoval
            }
        );
        assert!(ctx.set_target(owner, Some(body)).is_err());
        assert!(ctx.step([TicCommand::default(); 4]).is_err());
        assert_eq!(ctx.snapshot(), before);
        ctx.set_target(owner, None)?;
        ctx.set_perception_player(0, None, false, 100)?;
        ctx.step([TicCommand::default(); 4])?;
        assert_eq!(ctx.availability(body), ActorAvailability::Retired);
        assert!(ctx.actor(body).is_err());
        Ok(())
    })
    .unwrap();
}

#[test]
fn inert_corpse_reference_never_redirects_after_target_slot_reuse() {
    let mut sim = setup();
    sim.with_execution_context(|ctx| {
        let owner = ctx.admit_local(id(0), OWNER)?;
        let old = ctx.admit_local(id(1), BODY)?;
        ctx.damage(owner, None, Some(old), 150)?;
        ctx.set_perception_player(0, None, false, 100)?;
        ctx.mark_remove(old)?;
        ctx.step([TicCommand::default(); 4])?;
        assert_eq!(
            ctx.target(owner)?,
            TargetReference::Unavailable {
                stored: old.raw_id(),
                status: ActorAvailability::Retired
            }
        );
        let new = ctx.spawn_body([Fixed::from_map_units(96), Fixed::ZERO], SpawnHeight::Floor)?;
        assert_eq!(new.raw_id(), ActorId::from_parts(old.raw_id().slot(), 2));
        assert_eq!(
            ctx.target(owner)?,
            TargetReference::Unavailable {
                stored: old.raw_id(),
                status: ActorAvailability::StaleGeneration
            }
        );
        let before = ctx.snapshot();
        assert!(ctx.set_target(owner, Some(old)).is_err());
        assert!(ctx.mark_remove(old).is_err());
        assert!(ctx.damage(new, None, Some(old), 1).is_err());
        assert_eq!(ctx.snapshot(), before);
        assert_eq!(ctx.actor(new)?.life().unwrap().health, 1000);
        ctx.set_target(owner, None)?;
        assert_eq!(ctx.target(owner)?, TargetReference::Absent);
        Ok(())
    })
    .unwrap();
}

#[test]
fn corpse_is_available_storage_but_not_shootable() {
    let mut sim = setup();
    sim.with_execution_context(|ctx| {
        let owner = ctx.admit_local(id(0), OWNER)?;
        let body = ctx.admit_local(id(1), BODY)?;
        ctx.damage(body, None, Some(owner), 1000)?;
        assert_eq!(ctx.availability(body), ActorAvailability::Live);
        assert_eq!(
            ctx.target(owner)?,
            TargetReference::Available {
                actor: body,
                health: 0,
                shootable: false
            }
        );
        // Current body corpse acquisition retains the established role policy.
        ctx.set_target(owner, Some(body))?;
        Ok(())
    })
    .unwrap();
}

#[test]
fn retained_damage_source_can_be_observed_without_authorizing_it_as_a_target() {
    let mut sim = setup();
    sim.with_execution_context(|ctx| {
        let owner = ctx.admit_local(id(0), OWNER)?;
        let body = ctx.admit_local(id(1), BODY)?;
        // The shared damage service can retain the source on an inert body.
        // That does not grant the body's reference a Demon gameplay target role.
        ctx.damage(body, None, Some(owner), 1)?;
        let before = ctx.snapshot();
        assert_eq!(
            ctx.target(body)?,
            TargetReference::Incompatible {
                stored: owner.raw_id(),
                definition: OWNER
            }
        );
        assert!(ctx.set_target(body, Some(owner)).is_err());
        assert_eq!(ctx.snapshot(), before);
        Ok(())
    })
    .unwrap();
}

#[test]
fn rejected_state_damage_and_roster_requests_preserve_complete_diagnostics() {
    let mut sim = setup();
    sim.with_execution_context(|ctx| {
        let owner = ctx.admit_local(id(0), OWNER)?;
        let body = ctx.admit_local(id(1), BODY)?;
        let before = ctx.snapshot();
        for state in [
            OriginalState::S_NULL,
            OriginalState::S_SARG_PAIN,
            OriginalState::S_SARG_DIE1,
            OriginalState::S_SARG_RAISE1,
            OriginalState::S_PLAY,
        ] {
            assert!(ctx.enter_state(owner, state).is_err());
            assert_eq!(ctx.snapshot(), before);
        }
        for amount in [i32::MIN, -1, i32::MAX] {
            assert!(ctx.damage(owner, None, Some(body), amount).is_err());
            assert_eq!(ctx.snapshot(), before);
        }
        assert!(ctx.set_perception_player(4, Some(body), true, 100).is_err());
        assert!(
            ctx.set_perception_player(0, Some(owner), true, 100)
                .is_err()
        );
        assert!(ctx.set_perception_player(0, None, true, 100).is_err());
        assert_eq!(ctx.snapshot(), before);
        Ok(())
    })
    .unwrap();
}

#[test]
fn scope_is_not_an_aggregate_transaction() {
    let (mut sim, mut expected) = (setup(), setup());
    let result = sim.with_execution_context(|ctx| {
        let owner = ctx.admit_local(id(0), OWNER)?;
        ctx.set_target(owner, None)?;
        ctx.admit_local(ActorId::from_parts(u32::MAX, 1), BODY)?;
        Ok(())
    });
    assert!(result.is_err());
    expected.set_actor_target(id(0), None).unwrap();
    assert_eq!(sim.snapshot(), expected.snapshot());
}

#[test]
fn raw_ids_are_explicitly_local_and_do_not_prove_foreign_provenance() {
    let (mut first, mut second) = (setup(), setup());
    let erased = first
        .with_execution_context(|ctx| Ok(ctx.admit_local(id(1), BODY)?.raw_id()))
        .unwrap();
    second
        .with_execution_context(|ctx| {
            let local = ctx.admit_local(erased, BODY)?;
            assert_eq!(local.raw_id(), id(1));
            Ok(())
        })
        .unwrap();
    // Typed cross-context mixing and escaping are compile-fail rustdoc cases.
    assert_eq!(first.snapshot(), second.snapshot());
}

#[test]
fn legacy_profile_rejects_scope_before_invoking_callback() {
    let mut sim = fixtures::prior::historical::setup(&fixtures::prior::historical::cases()[0]);
    let before = sim.snapshot();
    let mut called = false;
    assert_eq!(
        sim.with_execution_context(|_| {
            called = true;
            Ok(())
        }),
        Err(ContextError::Profile)
    );
    assert!(!called);
    assert_eq!(sim.snapshot(), before);
}

#[test]
fn lowest_reusable_slot_is_appended_to_thinker_order_and_replay_is_stable() {
    let c = &fixtures::cases()[5];
    let (mut a, mut b) = (fixtures::setup(5, c), fixtures::setup(5, c));
    for op in 1..=6 {
        fixtures::advance(&mut a, 5, c, op);
        fixtures::advance(&mut b, 5, c, op);
        assert_eq!(a.snapshot(), b.snapshot());
        if op == 2 {
            assert_eq!(
                a.actor_order().collect::<Vec<_>>(),
                vec![id(0), id(2), id(3), id(4), ActorId::from_parts(1, 2)]
            );
        }
    }
}
