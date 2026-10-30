//! Rust ownership, rejection and representation policies; not C compatibility evidence.
#[path = "../../../tests/support/m3g2.rs"]
mod inputs;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
fn empty() -> Simulation {
    Simulation::original_player_map(inputs::map(), 64, MapContextOptions::default()).unwrap()
}
fn body(sim: &mut Simulation) -> ActorId {
    sim.spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap()
}
fn damage(
    sim: &mut Simulation,
    target: ActorId,
    source: Option<ActorId>,
    amount: i32,
) -> Result<DamageResult, SimError> {
    sim.damage_actor(DamageRequest {
        target,
        source,
        inflictor: None,
        amount,
    })
}
fn rejected(sim: &mut Simulation, op: impl FnOnce(&mut Simulation) -> Result<(), SimError>) {
    let before = sim.snapshot();
    assert!(op(sim).is_err());
    assert_eq!(sim.snapshot(), before);
}
#[test]
fn explicit_registry_policy_admits_only_the_source_player_closure() {
    let registry =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerBodyLifecycleV1).unwrap();
    let old = OriginalExecutionRegistry::new(OriginalExecutionProfile::DemonNormalV1).unwrap();
    let player = DefinitionIdentity::Original(OriginalActorType::MT_PLAYER);
    assert!(old.admit_definition(player).is_err());
    assert!(
        registry
            .admit_definition(DefinitionIdentity::Context(ContextRole::UnboundBody))
            .is_err()
    );
    for ty in OriginalActorType::all() {
        assert_eq!(
            registry
                .admit_definition(DefinitionIdentity::Original(ty))
                .is_ok(),
            [
                OriginalActorType::MT_PLAYER,
                OriginalActorType::MT_SERGEANT,
                OriginalActorType::MT_PUFF,
                OriginalActorType::MT_BLOOD
            ]
            .contains(&ty)
        );
    }
    for state in OriginalState::all() {
        assert_eq!(
            registry.admit_state(player, state).is_ok(),
            (OriginalState::S_PLAY.ordinal()..=OriginalState::S_PLAY_RUN4.ordinal())
                .contains(&state.ordinal())
                || (OriginalState::S_PLAY_PAIN.ordinal()..=OriginalState::S_PLAY_XDIE9.ordinal())
                    .contains(&state.ordinal())
        );
    }
    for n in 1..=u8::MAX {
        assert_eq!(PlayerSlot::try_from(n), Err(PlayerBodyError::Slot));
    }
    assert_eq!(PlayerSlot::try_from(0).unwrap().ordinal(), 0);
}
#[test]
fn binding_requires_a_current_initial_original_body_and_cannot_be_duplicated() {
    let mut sim = empty();
    let puff = sim
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    rejected(&mut sim, |s| {
        s.bind_original_player_body(PlayerSlot::One, puff)
    });
    for id in [
        ActorId::from_parts(u32::MAX, 1),
        ActorId::from_parts(0, 0),
        ActorId::from_parts(0, 2),
    ] {
        rejected(&mut sim, |s| {
            s.bind_original_player_body(PlayerSlot::One, id)
        });
    }
    let original = sim
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PLAYER.ordinal()),
            [Fixed::from_map_units(96), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    rejected(&mut sim, |s| s.advance(inputs::COMMANDS)); // unbound is represented, not executable
    sim.with_execution_context(|ctx| {
        let token = ctx.admit_local(
            original,
            DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
        )?;
        ctx.bind_original_player(PlayerSlot::One, token)
    })
    .unwrap();
    assert_ne!(original.slot(), u32::from(PlayerSlot::One.ordinal()));
    rejected(&mut sim, |s| {
        s.bind_original_player_body(PlayerSlot::One, original)
    });
    rejected(&mut sim, |s| {
        s.spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
            .map(|_| ())
    });
    sim.advance(inputs::COMMANDS).unwrap();
}
#[test]
fn pending_body_cannot_bind_and_the_retained_binding_never_reuses_a_generation() {
    let mut sim = empty();
    let unbound = sim
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PLAYER.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    sim.mark_remove(unbound).unwrap();
    rejected(&mut sim, |s| {
        s.bind_original_player_body(PlayerSlot::One, unbound)
    });
    sim.advance(inputs::COMMANDS).unwrap();
    let current = body(&mut sim);
    assert_eq!(current.slot(), unbound.slot());
    assert_ne!(current.generation(), unbound.generation());
    sim.mark_remove(current).unwrap();
    assert_eq!(
        sim.original_player(PlayerSlot::One).unwrap().mo,
        Some(current)
    );
    sim.advance(inputs::COMMANDS).unwrap();
    let puff = sim
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(puff.slot(), current.slot());
    assert_ne!(puff.generation(), current.generation());
    assert_eq!(
        sim.actor_availability(current),
        ActorAvailability::StaleGeneration
    );
    assert_eq!(
        sim.original_player(PlayerSlot::One).unwrap().mo,
        Some(current)
    );
    rejected(&mut sim, |s| {
        s.bind_original_player_body(PlayerSlot::One, puff)
    });
    rejected(&mut sim, |s| damage(s, current, None, 1).map(|_| ()));
}
#[test]
fn qualified_player_binding_rejects_other_instances_and_wrong_definitions() {
    let mut left = empty();
    let raw = body(&mut left);
    let mut a = GameInstance::new(left.clone());
    let mut b = GameInstance::new(left);
    let reference = a
        .with_execution_context(|ctx| {
            let body = ctx.admit_local(
                raw,
                DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
            )?;
            Ok(ctx.export_actor(body).unwrap())
        })
        .unwrap();
    let before = b.simulation().snapshot();
    let err = b.with_execution_context(|ctx| {
        assert!(matches!(
            ctx.import_actor(
                &reference,
                DefinitionIdentity::Original(OriginalActorType::MT_PLAYER)
            ),
            Err(ProvenanceError::InstanceMismatch)
        ));
        Ok(())
    });
    assert!(err.is_ok());
    assert_eq!(before, b.simulation().snapshot());
    a.with_execution_context(|ctx| {
        assert!(
            ctx.import_actor(
                &reference,
                DefinitionIdentity::Context(ContextRole::UnboundBody)
            )
            .is_err()
        );
        Ok(())
    })
    .unwrap();
}
#[test]
fn invalid_and_pending_attackers_reject_even_when_damage_would_be_ignored() {
    let mut sim = empty();
    sim.spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap();
    let target = body(&mut sim);
    let attacker = inputs::id(0);
    damage(&mut sim, target, Some(attacker), 1).unwrap();
    sim.mark_remove(attacker).unwrap();
    rejected(&mut sim, |s| {
        damage(s, target, Some(attacker), 1).map(|_| ())
    });
    sim.advance(inputs::COMMANDS).unwrap();
    assert_eq!(
        sim.original_player(PlayerSlot::One).unwrap().attacker,
        Some(attacker)
    );
    let replacement = sim
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::from_map_units(-96), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(replacement.slot(), attacker.slot());
    for invalid in [
        attacker,
        replacement,
        ActorId::from_parts(u32::MAX, 1),
        ActorId::from_parts(1, 0),
    ] {
        rejected(&mut sim, |s| {
            damage(s, target, Some(invalid), 1).map(|_| ())
        });
    }
    damage(&mut sim, target, None, 1000).unwrap();
    rejected(&mut sim, |s| {
        damage(s, target, Some(attacker), 0).map(|_| ())
    });
}
#[test]
fn source_player_damage_retains_health_relationship_and_allocated_corpse() {
    let mut sim = empty();
    let target = body(&mut sim);
    damage(&mut sim, target, Some(target), 20).unwrap();
    let p = sim.original_player(PlayerSlot::One).unwrap();
    assert_eq!(
        (p.health, p.attacker, p.damage_count),
        (80, Some(target), 20)
    );
    damage(&mut sim, target, None, 0).unwrap();
    assert_eq!(sim.original_player(PlayerSlot::One).unwrap().attacker, None);
    damage(&mut sim, target, None, 181).unwrap();
    let p = sim.original_player(PlayerSlot::One).unwrap();
    assert_eq!((p.health, p.state, p.frags), (0, PlayerBodyState::Dead, 1));
    assert_eq!(sim.actor(target).unwrap().life().unwrap().health, -101);
    assert_eq!(sim.actor(target).unwrap().solidity(), Solidity::NonSolid);
    for _ in 0..80 {
        sim.advance(inputs::COMMANDS).unwrap();
    }
    let rng = sim.rng();
    for _ in 0..64 {
        sim.advance(inputs::COMMANDS).unwrap();
    }
    assert_eq!(sim.rng(), rng);
    assert_eq!(sim.actor_availability(target), ActorAvailability::Live);
    assert_eq!(
        sim.original_player(PlayerSlot::One).unwrap().mo,
        Some(target)
    );
    let p = sim.original_player(PlayerSlot::One).unwrap();
    let result = damage(&mut sim, target, Some(target), 1000).unwrap();
    assert!(result.ignored);
    assert_eq!(sim.original_player(PlayerSlot::One).unwrap(), p);
    assert_eq!(sim.rng(), rng);
}
#[test]
fn unsupported_player_dependencies_and_manual_state_entry_are_atomic() {
    let mut sim = empty();
    let target = body(&mut sim);
    for state in [
        OriginalState::S_NULL,
        OriginalState::S_PLAY,
        OriginalState::S_PLAY_ATK1,
        OriginalState::S_PLAY_PAIN,
        OriginalState::S_PLAY_DIE1,
        OriginalState::S_SARG_STND,
    ] {
        rejected(&mut sim, |s| {
            s.enter_actor_state(target, StateId(state.ordinal()))
        });
    }
    rejected(&mut sim, |s| {
        s.set_momentum(target, [Fixed::from_map_units(1), Fixed::ZERO])
    });
    rejected(&mut sim, |s| s.set_vertical_momentum(target, Fixed::ZERO));
    rejected(&mut sim, |s| s.bind_player(0, target));
    rejected(&mut sim, |s| s.request_actor_death(target));
    rejected(&mut sim, |s| s.configure_perception_observer(target, 0, 1));
    rejected(&mut sim, |s| s.set_actor_target(target, None));
    rejected(&mut sim, |s| {
        s.damage_actor(DamageRequest {
            target,
            source: Some(target),
            inflictor: Some(target),
            amount: 1,
        })
        .map(|_| ())
    });
    for amount in [-1, 1001, i32::MAX] {
        rejected(&mut sim, |s| damage(s, target, None, amount).map(|_| ()));
    }
    rejected(&mut sim, |s| {
        s.set_perception_player(
            0,
            PerceptionPlayer {
                actor: Some(target),
                active: true,
                health: 100,
            },
        )
    });
    rejected(&mut sim, |s| {
        s.advance(
            [TicCommand {
                forward: 1,
                ..Default::default()
            }; 4],
        )
    });
    rejected(&mut sim, |s| s.restart_demon_map_context().map(|_| ()));
    sim.set_paused(true);
    let before = sim.snapshot();
    sim.advance(
        [TicCommand {
            forward: 1,
            buttons: 255,
            ..Default::default()
        }; 4],
    )
    .unwrap();
    assert_eq!(sim.snapshot(), before);
}
#[test]
fn live_demon_ai_requires_a_future_roster_but_direct_source_damage_is_real() {
    let mut sim = empty();
    sim.spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap();
    let player = body(&mut sim);
    let demon = inputs::id(0);
    rejected(&mut sim, |s| s.advance(inputs::COMMANDS));
    rejected(&mut sim, |s| s.look(demon).map(|_| ()));
    rejected(&mut sim, |s| s.chase(demon));
    damage(&mut sim, player, Some(demon), 5).unwrap();
    assert_eq!(
        sim.original_player(PlayerSlot::One).unwrap().attacker,
        Some(demon)
    );
    rejected(&mut sim, |s| damage(s, demon, Some(player), 1).map(|_| ()));
    damage(&mut sim, demon, Some(player), 150).unwrap();
    assert_eq!(sim.level_statistics().unwrap().single_player_kill_count, 1);
    sim.advance(inputs::COMMANDS).unwrap();
    damage(&mut sim, player, Some(demon), 1).unwrap();
    assert_eq!(
        sim.original_player(PlayerSlot::One).unwrap().attacker,
        Some(demon)
    );
}
#[test]
fn every_new_input_matches_step_and_advance_with_explicit_snapshot() {
    for (which, c) in inputs::cases().iter().enumerate() {
        let mut a = inputs::setup(which, c);
        let mut b = a.clone();
        for op in 1..=c.tics {
            inputs::operations(&mut a, which, op);
            inputs::operations(&mut b, which, op);
            let expected = a.step(inputs::COMMANDS).unwrap();
            b.advance(inputs::COMMANDS).unwrap();
            assert_eq!(expected, b.snapshot());
        }
    }
}
#[test]
fn old_and_new_diagnostic_readers_negotiate_without_reinterpreting_history() {
    let mut sim = empty();
    let player = body(&mut sim);
    let b = sim
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    assert_eq!(
        Diagnostic::decode(&b, DiagnosticLimits::default()).unwrap_err(),
        DiagnosticError::Version(22)
    );
    let read = Diagnostic::decode_player(&b, DiagnosticLimits::default()).unwrap();
    assert_eq!(
        read.actor(player).unwrap(),
        Some(DiagnosticIdentity::Original {
            definition: DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
            state: Some(OriginalState::S_PLAY)
        })
    );
    assert_eq!(read.encode(DiagnosticLimits::default()).unwrap(), b);
    for bytes in [
        include_bytes!("../../../tests/reference/m3f7_state_0_0.bin").as_slice(),
        include_bytes!("../../../tests/reference/m3f8_state_4_0.bin").as_slice(),
    ] {
        assert!(Diagnostic::decode(bytes, DiagnosticLimits::default()).is_ok());
        assert!(Diagnostic::decode_player(bytes, DiagnosticLimits::default()).is_err());
    }
    for len in 0..b.len().min(64) {
        assert!(Diagnostic::decode_player(&b[..len], DiagnosticLimits::default()).is_err());
    }
    assert!(
        Diagnostic::decode_player(
            &b,
            DiagnosticLimits {
                bytes: b.len() - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}

fn edit_field(input: &[u8], name: &str, replacement: &[u8]) -> Vec<u8> {
    let mut out = input[..16].to_vec();
    let mut at = 16;
    while at < input.len() {
        let start = at;
        let len = u32::from_le_bytes(input[at..at + 4].try_into().unwrap()) as usize;
        at += 4;
        let path = std::str::from_utf8(&input[at..at + len]).unwrap();
        at += len;
        let len = u64::from_le_bytes(input[at..at + 8].try_into().unwrap()) as usize;
        at += 8;
        let value = if path == name {
            replacement
        } else {
            &input[at..at + len]
        };
        out.extend_from_slice(&input[start..start + 4 + path.len()]);
        out.extend_from_slice(&(value.len() as u64).to_le_bytes());
        out.extend_from_slice(value);
        at += len;
    }
    out
}
#[test]
fn player_diagnostic_forgery_and_malformed_new_events_reject_safely() {
    let mut sim = empty();
    body(&mut sim);
    let original = sim.snapshot().canonical_bytes();
    for (field, value) in [
        ("context.schema", b"UNBOUND-BODY-v1".to_vec()),
        ("profile", 1_u32.to_le_bytes().to_vec()),
        ("original_player.0.mo", vec![]),
        ("original_player.0.mo", vec![1; 13]),
        ("original_player.0.attacker", vec![1; 13]),
        ("original_player.0.health", 99_i32.to_le_bytes().to_vec()),
        ("original_player.0.state", vec![1]),
        ("actor.0.player", vec![255]),
        ("events.pending.2", vec![22, 99]),
        ("events.pending.2", vec![22, 1]),
        ("events.pending.2", vec![22, 0]),
    ] {
        assert!(
            Diagnostic::decode_player(
                &edit_field(&original, field, &value),
                DiagnosticLimits::default()
            )
            .is_err(),
            "{field}"
        );
    }
    // Bound is pending event2: Spawned, Initialized, Bound.
    let mut event = vec![22, 2];
    event.extend(0_i32.to_le_bytes());
    event.extend(OriginalState::S_PLAY.ordinal().to_le_bytes());
    assert!(
        Diagnostic::decode_player(
            &edit_field(&original, "events.pending.2", &event),
            DiagnosticLimits::default()
        )
        .is_err()
    );
}
