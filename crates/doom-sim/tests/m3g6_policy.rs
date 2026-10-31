//! Rust ownership/admission/rollback policy. Independent C evidence is separate.
#[path = "../../../tests/support/m3g6.rs"]
mod source;
use doom_sim::{identity::*, *};
use doom_types::Fixed;

// p_mobj.h and the existing STATE_FORMAT MonsterEvent damage-draw category.
const MF_SOLID: u32 = 0x2;
const MF_SHOOTABLE: u32 = 0x4;
const MF_CORPSE: u32 = 0x100000;
const DAMAGE_DRAW: u8 = 4;
fn setup(index: usize) -> (Simulation, source::Actors) {
    source::setup(&source::cases()[index])
}
fn advance(s: &mut Simulation, a: source::Actors, index: usize, op: usize) {
    let c = &source::cases()[index];
    source::operations(s, a, c, op);
    s.advance(source::commands(c, op)).unwrap();
}
fn through(index: usize, tics: usize) -> (Simulation, source::Actors) {
    let (mut s, a) = setup(index);
    for op in 1..=tics {
        advance(&mut s, a, index, op);
    }
    (s, a)
}
fn reject(s: &mut Simulation, op: impl FnOnce(&mut Simulation) -> Result<(), SimError>) {
    let before = s.snapshot();
    let roster = s.original_player_roster();
    assert!(op(s).is_err());
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.original_player_roster(), roster);
}
fn damages(s: &Simulation) -> Vec<DamageRequest> {
    s.events()
        .iter()
        .filter_map(|e| match e {
            Event::Combat(CombatEvent::Damage(request)) => Some(*request),
            _ => None,
        })
        .collect()
}

#[test]
fn explicit_capability_preserves_original_ids_and_acquisition_only_rejection() {
    let r = OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonMeleeV1).unwrap();
    assert_eq!(OriginalExecutionProfile::PlayerDemonMeleeV1.wire_id(), 6);
    for ty in OriginalActorType::all() {
        assert_eq!(
            r.admit_definition(DefinitionIdentity::Original(ty)).is_ok(),
            [
                OriginalActorType::MT_PLAYER,
                OriginalActorType::MT_SERGEANT,
                OriginalActorType::MT_PUFF,
                OriginalActorType::MT_BLOOD
            ]
            .contains(&ty)
        );
    }
    assert!(
        r.admit_definition(DefinitionIdentity::Context(ContextRole::UnboundBody))
            .is_err()
    );
    let old =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonAcquisitionV1).unwrap();
    for state in [
        OriginalState::S_SARG_ATK1,
        OriginalState::S_SARG_ATK2,
        OriginalState::S_SARG_ATK3,
    ] {
        let definition = DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT);
        assert_eq!(
            r.admit_state(definition, state).unwrap(),
            StateId(state.ordinal())
        );
        assert!(old.admit_state(definition, state).is_err());
    }
    assert!(
        r.admit_state(
            DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT),
            OriginalState::S_SARG_RAISE1
        )
        .is_err()
    );
    assert!(
        r.admit_state(
            DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
            OriginalState::S_PLAY_ATK1
        )
        .is_err()
    );
}

#[test]
fn normal_thinkers_delay_damage_until_the_source_attack_action() {
    let (mut s, a) = setup(1);
    let demon = a.origin.unwrap();
    for op in 1..=28 {
        advance(&mut s, a, 1, op);
        if op < 28 {
            assert!(damages(&s).is_empty());
            assert_eq!(s.original_player(PlayerSlot::One).unwrap().health, 100);
        }
        if [12, 20, 28].contains(&op) {
            let state = [
                OriginalState::S_SARG_ATK1,
                OriginalState::S_SARG_ATK2,
                OriginalState::S_SARG_ATK3,
            ][(op - 12) / 8];
            let life = s.actor(demon).unwrap().life().unwrap();
            assert_eq!((life.state, life.tics), (Some(StateId(state.ordinal())), 8));
        }
    }
    assert_eq!(
        damages(&s),
        vec![DamageRequest {
            target: a.body,
            inflictor: Some(demon),
            source: Some(demon),
            amount: 32
        }]
    );
    let p = s.original_player(PlayerSlot::One).unwrap();
    assert_eq!((p.health, p.attacker), (68, Some(demon)));
    assert_eq!(s.actor(a.body).unwrap().life().unwrap().health, 68);
    assert!(s.actor(a.body).unwrap().position()[0] > Fixed::ZERO);
    assert!(s.actor(a.body).unwrap().momentum()[0] > Fixed::ZERO);
    assert_eq!(s.actor(demon).unwrap().life().unwrap().health, 150);
    assert_eq!(s.actor_availability(demon), ActorAvailability::Live);
    for op in 29..=64 {
        advance(&mut s, a, 1, op);
    }
}

#[test]
fn moving_target_miss_rechecks_range_without_damage_or_damage_rng() {
    let (mut s, a) = through(2, 31);
    let rng = s.rng();
    advance(&mut s, a, 2, 32);
    assert_eq!(s.rng(), rng);
    assert!(damages(&s).is_empty());
    assert_eq!(s.original_player(PlayerSlot::One).unwrap().health, 100);
    assert!(s.events().iter().any(|e| matches!(e,
        Event::Monster(MonsterEvent::Range {kind: 0, decision, ..}) if !decision.accepted)));
    assert!(!s.events().iter().any(|e| matches!(
        e,
        Event::Monster(MonsterEvent::Random {
            kind: DAMAGE_DRAW,
            ..
        })
    )));
    assert_eq!(
        s.actor(a.origin.unwrap()).unwrap().life().unwrap().state,
        Some(StateId(OriginalState::S_SARG_ATK3.ordinal()))
    );
}

#[test]
fn absent_target_returns_without_facing_range_or_rng_then_reacquires() {
    let (mut s, a) = through(9, 27);
    let rng = s.rng();
    advance(&mut s, a, 9, 28);
    assert_eq!(s.rng(), rng);
    assert!(damages(&s).is_empty());
    assert!(!s.events().iter().any(|e| matches!(
        e,
        Event::Monster(MonsterEvent::Face { .. } | MonsterEvent::Range { .. })
    )));
    for op in 29..=54 {
        advance(&mut s, a, 9, op);
    }
    assert_eq!(
        s.actor(a.origin.unwrap()).unwrap().life().unwrap().target,
        Some(a.body)
    );
    assert_eq!(damages(&s)[0].amount, 32);
}

#[test]
fn dead_allocated_target_can_pass_range_draw_and_then_ignore_damage() {
    let (mut s, a) = through(10, 27);
    let player = s.original_player(PlayerSlot::One).unwrap();
    let mut rng = s.rng();
    rng.p_random();
    advance(&mut s, a, 10, 28);
    assert_eq!(s.rng(), rng);
    assert_eq!(s.original_player(PlayerSlot::One).unwrap(), player);
    assert_eq!(damages(&s)[0].amount, 24);
    assert!(s.events().iter().any(|e| matches!(e,
        Event::Combat(CombatEvent::Ignored(id)) if *id == a.body)));
    assert!(!s.events().iter().any(|e| matches!(
        e,
        Event::Combat(CombatEvent::Knockback { .. } | CombatEvent::Health { .. })
    )));
}

#[test]
fn independently_observed_damage_extrema_and_nonpain_draw_remain_exact() {
    for (index, tic, expected) in [(0, 92, 4), (6, 122, 40)] {
        let (s, a) = through(index, tic);
        assert_eq!(damages(&s)[0].amount, expected);
        assert_eq!(damages(&s)[0].source, a.origin);
        assert_eq!(damages(&s)[0].inflictor, a.origin);
    }
    let (s, _) = through(11, 17);
    assert!(s.events().iter().any(|e| matches!(
        e,
        Event::Combat(CombatEvent::Random {
            kind: 1,
            value: 255
        })
    )));
    assert!(
        !s.events()
            .iter()
            .any(|e| matches!(e, Event::Combat(CombatEvent::Pain { .. })))
    );
    assert_eq!(s.original_player(PlayerSlot::One).unwrap().health, 68);
}

#[test]
fn autonomous_lethal_chain_retains_active_owned_terminal_corpse_and_attacker() {
    // No injected damage, state entry, target assignment or attack helper.
    let (mut s, a) = setup(0);
    let mut hits = Vec::new();
    for op in 1..=416 {
        advance(&mut s, a, 0, op);
        for hit in damages(&s) {
            assert_eq!(
                (hit.target, hit.source, hit.inflictor),
                (a.body, a.origin, a.origin)
            );
            hits.push((op, hit.amount));
        }
    }
    assert_eq!(
        hits,
        [
            (92, 4),
            (116, 24),
            (146, 12),
            (174, 24),
            (228, 12),
            (280, 8),
            (330, 24)
        ]
    );
    let p = s.original_player(PlayerSlot::One).unwrap();
    assert_eq!(
        (p.mo, p.health, p.state, p.attacker, p.frags),
        (Some(a.body), 0, PlayerBodyState::Dead, a.origin, 0)
    );
    let body = s.actor(a.body).unwrap();
    let life = body.life().unwrap();
    assert_eq!(
        (life.health, life.state, life.tics),
        (-8, Some(StateId(OriginalState::S_PLAY_DIE7.ordinal())), -1)
    );
    assert_eq!(life.flags & (MF_SOLID | MF_SHOOTABLE), 0);
    assert_ne!(life.flags & MF_CORPSE, 0);
    assert_eq!(body.momentum(), [Fixed::ZERO; 2]);
    assert_eq!(s.original_player_roster().unwrap()[0].actor, Some(a.body));
    assert!(s.original_player_roster().unwrap()[0].active);
    assert_eq!(
        s.actor(a.origin.unwrap()).unwrap().life().unwrap().target,
        Some(a.body)
    );
    assert_eq!(s.level_statistics().unwrap().single_player_kill_count, 0);
    reject(&mut s, |s| s.mark_remove(a.body));
    reject(&mut s, |s| {
        s.spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
            .map(|_| ())
    });
}

#[test]
fn damage_resets_reaction_delay_then_commands_compose_with_impulse_before_thinkers() {
    let (mut s, a) = through(16, 27);
    assert_eq!(s.actor(a.body).unwrap().life().unwrap().reaction_time, 13);
    advance(&mut s, a, 16, 28);
    assert_eq!(s.actor(a.body).unwrap().life().unwrap().reaction_time, 0);
    let position = s.actor(a.body).unwrap().position();
    advance(&mut s, a, 16, 29);
    assert!(s.actor(a.body).unwrap().position()[0] > position[0]);
    let command = s
        .events()
        .iter()
        .position(|e| matches!(e, Event::Player { slot: 0, .. }))
        .unwrap();
    let thinker = s
        .events()
        .iter()
        .position(|e| matches!(e, Event::Think(_)))
        .unwrap();
    assert!(command < thinker);
    source::check_commands_and_motion(&s, a, &s.snapshot(), source::rows(16, 29));
}

#[test]
fn fixed_roster_invalid_commands_wrong_roles_and_stale_targets_reject_atomically() {
    let (mut s, a) = setup(1);
    let demon = a.origin.unwrap();
    for (slot, forward, buttons) in [(1, 1, 0), (0, 51, 0), (0, 0, 1)] {
        let mut input = [TicCommand::default(); 4];
        input[slot] = TicCommand {
            forward,
            buttons,
            ..Default::default()
        };
        reject(&mut s, |s| s.advance(input));
    }
    assert!(PlayerSlot::try_from(1).is_err());
    reject(&mut s, |s| {
        s.bind_original_player_body(PlayerSlot::One, demon)
    });
    reject(&mut s, |s| {
        s.bind_original_player_body(PlayerSlot::One, a.body)
    });
    reject(&mut s, |s| {
        s.set_actor_target(
            demon,
            Some(ActorId::from_parts(a.body.slot(), a.body.generation() + 1)),
        )
    });
    reject(&mut s, |s| {
        s.set_actor_target(demon, Some(ActorId::from_parts(u32::MAX, 1)))
    });
    reject(&mut s, |s| s.set_actor_target(demon, Some(demon)));
    reject(&mut s, |s| {
        s.set_perception_player(0, PerceptionPlayer::default())
    });
    reject(&mut s, |s| s.demon_melee_attack(demon));
    for ty in [OriginalActorType::MT_SHADOWS, OriginalActorType::MT_SHOTGUY] {
        reject(&mut s, |s| {
            s.spawn_type(
                ActorTypeId(ty.ordinal()),
                [Fixed::ZERO; 2],
                SpawnHeight::Floor,
            )
            .map(|_| ())
        });
    }
}

#[test]
fn missing_body_cannot_run_an_empty_or_fabricated_roster() {
    let c = &source::cases()[1];
    let mut s = Simulation::original_player_demon_melee_map(
        source::case_map(c.map),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    let d = s
        .spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap()
        .spawned[0];
    reject(&mut s, |s| s.advance([TicCommand::default(); 4]));
    reject(&mut s, |s| s.look_for_players(d, false).map(|_| ()));
    reject(&mut s, |s| s.bind_original_player_body(PlayerSlot::One, d));
}

#[test]
fn retiring_live_attacker_does_not_rebind_retained_references_to_reused_slot() {
    let (mut s, a) = through(1, 28);
    let d = a.origin.unwrap();
    s.mark_remove(d).unwrap();
    s.advance([TicCommand::default(); 4]).unwrap();
    assert!(s.actor(d).is_none());
    let replacement = s
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(replacement.slot(), d.slot());
    assert_ne!(replacement.generation(), d.generation());
    assert_eq!(
        s.original_player(PlayerSlot::One).unwrap().attacker,
        Some(d)
    );
    assert_eq!(s.actor(a.body).unwrap().life().unwrap().target, Some(d));
    reject(&mut s, |s| s.set_actor_target(d, Some(a.body)));
    s.advance([TicCommand::default(); 4]).unwrap();
}

#[test]
fn foreign_game_instance_references_remain_rejected_under_melee_capability() {
    let (s, a) = setup(1);
    let mut left = GameInstance::new(s.clone());
    let mut right = GameInstance::new(s);
    let exported = left
        .with_execution_context(|ctx| {
            let token = ctx.admit_local(
                a.body,
                DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
            )?;
            Ok(ctx.export_actor(token).unwrap())
        })
        .unwrap();
    let before = right.simulation().snapshot();
    right
        .with_execution_context(|ctx| {
            assert_eq!(
                ctx.import_actor(
                    &exported,
                    DefinitionIdentity::Original(OriginalActorType::MT_PLAYER)
                ),
                Err(ProvenanceError::InstanceMismatch)
            );
            Ok(())
        })
        .unwrap();
    assert_eq!(right.simulation().snapshot(), before);
}

// Rust-only schema corruption, not the independent expected-fixture encoder.
fn altered(snapshot: &Snapshot, policy: u32, context: &[u8]) -> Vec<u8> {
    let fields: Vec<_> = snapshot.fields().collect();
    let mut bytes = b"DSIM".to_vec();
    bytes.extend(22_u32.to_le_bytes());
    bytes.extend(u64::try_from(fields.len()).unwrap().to_le_bytes());
    for field in fields {
        let policy_bytes = policy.to_le_bytes();
        let value = match field.path() {
            "profile" => policy_bytes.as_slice(),
            "context.schema" => context,
            _ => field.value(),
        };
        bytes.extend(u32::try_from(field.path().len()).unwrap().to_le_bytes());
        bytes.extend(field.path().as_bytes());
        bytes.extend(u64::try_from(value.len()).unwrap().to_le_bytes());
        bytes.extend(value);
    }
    bytes
}
#[test]
fn diagnostics_keep_v22_layout_and_reject_mixed_or_no_melee_attack_schemas() {
    let (s, _) = through(1, 12);
    let snapshot = s.snapshot();
    let bytes = snapshot
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    let view = Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).unwrap();
    assert_eq!(view.version(), 22);
    assert_eq!(
        view.field("context.schema"),
        Some(PLAYER_DEMON_MELEE_CONTEXT_SCHEMA)
    );
    assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
    assert!(Diagnostic::decode(&bytes, DiagnosticLimits::default()).is_err());
    for (policy, context) in [
        (6, PLAYER_DEMON_CONTEXT_SCHEMA),
        (5, PLAYER_DEMON_MELEE_CONTEXT_SCHEMA),
        (7, PLAYER_DEMON_MELEE_CONTEXT_SCHEMA),
    ] {
        assert!(
            Diagnostic::decode_player(
                &altered(&snapshot, policy, context),
                DiagnosticLimits::default()
            )
            .is_err()
        );
    }
    assert_eq!(
        Diagnostic::decode_player(
            &altered(&snapshot, 5, PLAYER_DEMON_CONTEXT_SCHEMA),
            DiagnosticLimits::default()
        )
        .unwrap_err(),
        DiagnosticError::Identity
    );
    let old = include_bytes!("../../../tests/reference/m3g5_state_1_10.bin");
    assert!(Diagnostic::decode_player(old, DiagnosticLimits::default()).is_ok());
}

#[test]
fn pause_rejections_and_stepping_equivalence_hold_through_hit_death_and_corpse() {
    for tic in [27, 28, 330, 416] {
        let (mut s, _) = through(0, tic);
        let mut stepped = s.clone();
        let mut bad = [TicCommand::default(); 4];
        bad[3].buttons = 255;
        s.set_paused(true);
        stepped.set_paused(true);
        let before = s.snapshot();
        s.advance(bad).unwrap();
        assert_eq!(s.snapshot(), before);
        assert_eq!(s.snapshot(), stepped.step(bad).unwrap());
        s.set_paused(false);
        stepped.set_paused(false);
        let before = s.snapshot();
        assert_eq!(s.advance(bad), stepped.step(bad).map(|_| ()));
        assert_eq!(s.snapshot(), before);
        assert_eq!(s.snapshot(), stepped.snapshot());
    }
}
