use super::*;
#[test]
fn independent_original_world_ammo() {
    assert_ne!(pickup_evidence::verify_parts(0), 0);
}

#[test]
fn independent_original_world_health() {
    assert_ne!(pickup_evidence::verify_parts(1), 0);
}
#[test]
fn independent_original_world_weapon_acquisition_and_sustained_combat() {
    assert_ne!(pickup_evidence::verify_parts(3), 0);
}

use crate::{identity::*, pickup_evidence as source};
fn fresh(which: usize) -> (Simulation, source::Actors) {
    source::setup(&source::cases()[which])
}
fn item(s: &Simulation) -> ActorId {
    s.actor_order()
        .find(|id| {
            s.actor(*id)
                .and_then(Actor::life)
                .is_some_and(|l| world_pickups::kind(l.actor_type).is_some())
        })
        .unwrap()
}
fn rejected(s: &mut Simulation, commands: [TicCommand; 4]) {
    let before = s.snapshot();
    let count = snapshot::capture_count();
    let roster = s.original_player_roster();
    assert!(s.advance(commands).is_err());
    assert_eq!(snapshot::capture_count(), count);
    assert_eq!(s.snapshot(), before);
    assert_eq!(s.original_player_roster(), roster);
    assert!(s.step(commands).is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn pickup_definition_admission_is_explicit_and_static() {
    let r = OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonPickupsV1).unwrap();
    for (k, ordinal, state) in [
        (PickupKind::Clip, 63, 870),
        (PickupKind::BulletBox, 64, 871),
        (PickupKind::Shells, 69, 876),
        (PickupKind::ShellBox, 70, 877),
        (PickupKind::Stimpack, 53, 840),
        (PickupKind::Medikit, 54, 841),
        (PickupKind::Shotgun, 77, 884),
        (PickupKind::Chaingun, 73, 880),
    ] {
        assert_eq!(k.original_type().ordinal(), ordinal);
        let d = DefinitionIdentity::Original(k.original_type());
        assert_eq!(r.admit_definition(d).unwrap(), ActorTypeId(ordinal));
        assert_eq!(
            r.admit_state(d, k.original_type().states().spawn).unwrap(),
            StateId(state)
        );
        assert!(r.admit_state(d, OriginalState::S_SARG_RUN1).is_err());
    }
    for ty in [
        OriginalActorType::MT_SHOTGUY,
        OriginalActorType::MT_CHAINGUY,
        OriginalActorType::MT_SHADOWS,
    ] {
        assert!(
            r.admit_definition(DefinitionIdentity::Original(ty))
                .is_err()
        );
    }
}
#[test]
fn old_weapon_policy_still_rejects_world_items_atomically() {
    let mut s = Simulation::original_player_demon_weapons_map(
        source::case_map(source::cases()[1].map),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    let before = s.snapshot();
    assert!(s.spawn_map_things().is_err());
    assert_eq!(s.snapshot(), before);
    assert!(
        s.spawn_dropped_pickup(PickupKind::Clip, [Fixed::ZERO; 2])
            .is_err()
    );
    assert_eq!(s.snapshot(), before);
}
#[test]
fn item_raw_spawn_state_target_and_damage_are_not_accidental_capabilities() {
    let (mut s, a) = fresh(1);
    let i = item(&s);
    let before = s.snapshot();
    assert!(
        s.spawn_type(ActorTypeId(63), [Fixed::ZERO; 2], SpawnHeight::Floor)
            .is_err()
    );
    assert!(s.enter_original_state(i, OriginalState::S_CLIP).is_err());
    assert!(s.set_actor_target(i, Some(a.body)).is_err());
    assert!(
        s.damage_actor(DamageRequest {
            target: i,
            inflictor: None,
            source: Some(a.body),
            amount: 1
        })
        .is_err()
    );
    assert!(
        s.damage_actor(DamageRequest {
            target: a.body,
            inflictor: None,
            source: Some(i),
            amount: 1
        })
        .is_err()
    );
    assert_eq!(s.snapshot(), before);
}
#[test]
fn map_filtering_and_item_statistics_keep_original_meaning() {
    let c = &source::cases()[17];
    let mut s = Simulation::original_player_pickups_map(
        source::case_map(c.map),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    let report = s.spawn_map_things().unwrap();
    assert_eq!(report.filtered, 3);
    assert!(report.spawned.is_empty());
    assert_eq!(s.level_statistics().unwrap().total_kills, 0);
    assert_eq!(s.level_statistics().unwrap().total_items, 0);
    let (s, _) = fresh(32);
    assert_eq!(s.actor_order().count(), 17);
    assert_eq!(s.level_statistics().unwrap().total_items, 0);
}
#[test]
fn batch_actor_capacity_failure_preserves_rng_order_events_and_totals() {
    let c = &source::cases()[32];
    let mut s = Simulation::original_player_pickups_map(
        source::case_map(c.map),
        8,
        MapContextOptions::default(),
    )
    .unwrap();
    let before = s.snapshot();
    assert!(s.spawn_map_things().is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn controlled_dropped_setup_checks_kind_order_bounds_and_capacity() {
    let mut s = Simulation::original_player_pickups_map(
        source::case_map(source::cases()[17].map),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    s.spawn_map_things().unwrap();
    for k in [
        PickupKind::Stimpack,
        PickupKind::BulletBox,
        PickupKind::Shells,
    ] {
        let before = s.snapshot();
        assert!(s.spawn_dropped_pickup(k, [Fixed::ZERO; 2]).is_err());
        assert_eq!(s.snapshot(), before);
    }
    let before = s.snapshot();
    assert!(
        s.spawn_dropped_pickup(PickupKind::Clip, [Fixed::from_map_units(979), Fixed::ZERO])
            .is_err()
    );
    assert_eq!(s.snapshot(), before);
    for _ in 0..MAX_WORLD_PICKUPS {
        s.spawn_dropped_pickup(PickupKind::Clip, [Fixed::ZERO; 2])
            .unwrap();
    }
    let before = s.snapshot();
    assert!(
        s.spawn_dropped_pickup(PickupKind::Clip, [Fixed::ZERO; 2])
            .is_err()
    );
    assert_eq!(s.snapshot(), before);
    let (mut s, _) = fresh(1);
    let before = s.snapshot();
    assert!(
        s.spawn_dropped_pickup(PickupKind::Clip, [Fixed::ZERO; 2])
            .is_err()
    );
    assert_eq!(s.snapshot(), before);
}
#[test]
fn invalid_static_item_fields_reject_before_any_command_or_rng() {
    for mode in 0..12 {
        let (mut s, a) = fresh(1);
        let i = item(&s);
        let actor = s.actors.get_mut(i).unwrap();
        match mode {
            0 => actor.life.as_mut().unwrap().sprite = 0,
            1 => actor.life.as_mut().unwrap().frame = 1,
            2 => actor.life.as_mut().unwrap().tics = 0,
            3 => actor.life.as_mut().unwrap().state = Some(StateId(0)),
            4 => actor.life.as_mut().unwrap().flags |= 2,
            5 => actor.life.as_mut().unwrap().health = 999,
            6 => actor.life.as_mut().unwrap().target = Some(a.body),
            7 => actor.spec.momentum[0] = Fixed::ONE,
            8 => actor.z = Fixed::ONE,
            9 => actor.spec.radius = Fixed::ONE,
            10 => actor.life.as_mut().unwrap().spawnpoint = None,
            _ => actor.life.as_mut().unwrap().flags |= 0x20000,
        };
        rejected(&mut s, source::commands(&source::cases()[1], 1));
    }
}
#[test]
fn invalid_player_generation_health_or_missing_pickup_state_rejects() {
    for mode in 0..6 {
        let (mut s, a) = fresh(1);
        let p = s.original_player.as_mut().unwrap();
        match mode {
            0 => p.mo = None,
            1 => p.mo = Some(ActorId::from_parts(a.body.slot(), a.body.generation() + 1)),
            2 => p.health = 99,
            3 => p.pickups = None,
            4 => p.pickups.as_mut().unwrap().bonus_count = -1,
            _ => p.pickups.as_mut().unwrap().item_count = 1,
        };
        rejected(&mut s, source::commands(&source::cases()[1], 1));
    }
}
#[test]
fn unsupported_commands_slots_and_pending_body_preserve_complete_state() {
    for mode in 0..5 {
        let (mut s, a) = fresh(1);
        let mut c = source::commands(&source::cases()[1], 1);
        match mode {
            0 => c[0].buttons = 2,
            1 => c[1].forward = 1,
            2 => c[0].forward = 51,
            3 => {
                s.actors.mark_remove(a.body).unwrap();
            }
            _ => {
                s.spatial.order.push(a.body);
            }
        };
        rejected(&mut s, c);
    }
}
#[test]
fn ordinary_wall_block_commits_source_collection_and_exact_slide() {
    let (mut s, a) = fresh(28);
    let i = item(&s);
    let pos = s.actor(a.body).unwrap().position();
    s.advance(source::commands(&source::cases()[28], 1))
        .unwrap();
    assert_eq!(s.actor(a.body).unwrap().position()[0], pos[0]);
    assert_eq!(s.actor(a.body).unwrap().position()[1], Fixed::from_bits(3));
    assert_eq!(s.actor_availability(i), ActorAvailability::PendingRemoval);
    assert_eq!(
        s.original_player_weapons(PlayerSlot::One)
            .unwrap()
            .unwrap()
            .bullets,
        60
    );
    assert!(s.events().iter().any(|e| matches!(
        e,
        Event::Pickup(PickupEvent::Result(_, PickupOutcome::Collected))
    )));
}
#[test]
fn full_resource_refusal_keeps_item_and_does_not_draw_rng() {
    let (mut s, _) = fresh(7);
    let i = item(&s);
    let rng = s.rng().indices();
    s.advance(source::commands(&source::cases()[7], 1)).unwrap();
    assert_eq!(s.actor_availability(i), ActorAvailability::Live);
    assert_eq!(s.rng().indices(), rng);
    assert_eq!(
        s.original_player_pickups(PlayerSlot::One).unwrap(),
        PlayerPickupState::default()
    );
    assert!(s.events().iter().any(|e| matches!(
        e,
        Event::Pickup(PickupEvent::Result(_, PickupOutcome::NotNeeded))
    )));
}
#[test]
fn collection_retirement_and_effect_slot_reuse_never_redirect_item_references() {
    let c = &source::cases()[48];
    let (mut s, _) = fresh(48);
    let i = ActorId::from_parts(0, 1);
    for op in 1..=54 {
        s.advance(source::commands(c, op)).unwrap();
        if op == 1 {
            assert_eq!(s.actor_availability(i), ActorAvailability::PendingRemoval);
            assert!(!s.spatial.order.contains(&i));
        }
    }
    assert_eq!(s.actor_availability(i), ActorAvailability::StaleGeneration);
    let reuse = s.actor_order().find(|id| id.slot() == i.slot()).unwrap();
    assert_ne!(reuse.generation(), i.generation());
    assert!(s.actor(i).is_none());
    let before = s.snapshot();
    assert!(s.set_actor_target(i, None).is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn healing_preserves_pain_attacker_and_damage_counter() {
    let c = &source::cases()[18];
    let (mut s, a) = fresh(18);
    let before = s.original_player(PlayerSlot::One).unwrap();
    s.advance(source::commands(c, 1)).unwrap();
    let after = s.original_player(PlayerSlot::One).unwrap();
    assert_eq!(after.health, 11);
    assert_eq!(after.attacker, before.attacker);
    assert_eq!(after.damage_count, before.damage_count);
    assert_eq!(s.actor(a.body).unwrap().life().unwrap().health, 11);
    assert_eq!(
        s.original_player_pickups(PlayerSlot::One).unwrap().message,
        PickupMessage::Stimpack
    );
}
#[test]
fn dead_moving_body_cannot_collect_or_resurrect() {
    let c = &source::cases()[27];
    let (mut s, a) = fresh(27);
    let i = item(&s);
    let before = s.original_player(PlayerSlot::One).unwrap();
    s.advance(source::commands(c, 1)).unwrap();
    assert_eq!(s.original_player(PlayerSlot::One).unwrap(), before);
    assert_eq!(s.actor_availability(i), ActorAvailability::Live);
    assert_eq!(s.actor(a.body).unwrap().life().unwrap().health, 0);
    assert!(s.events().iter().any(|e| matches!(
        e,
        Event::Pickup(PickupEvent::Result(_, PickupOutcome::Dead))
    )));
}
#[test]
fn late_bonus_rejection_rolls_back_real_grant_removal_and_prior_command() {
    for index in [1, 33, 34] {
        let (mut s, _) = fresh(index);
        s.original_player
            .as_mut()
            .unwrap()
            .pickups
            .as_mut()
            .unwrap()
            .bonus_count = 96;
        // Range-valid but causally inconsistent private state: +6 fails after
        // ammo/ownership/pending changes and removal in the same transaction.
        rejected(&mut s, source::commands(&source::cases()[index], 1));
        let w = s.original_player_weapons(PlayerSlot::One).unwrap().unwrap();
        assert_eq!((w.owned, w.pending, w.bullets, w.shells), (3, None, 50, 0));
    }
}

#[test]
fn paused_advance_and_step_keep_pickups_pending_and_snapshot_capture_explicit() {
    let (mut s, _) = fresh(1);
    s.set_paused(true);
    let mut captured = s.clone();
    let before = s.original_player_pickups(PlayerSlot::One).unwrap();
    let count = snapshot::capture_count();
    s.advance(source::commands(&source::cases()[1], 1)).unwrap();
    assert_eq!(snapshot::capture_count(), count);
    assert_eq!(
        s.snapshot(),
        captured
            .step(source::commands(&source::cases()[1], 1))
            .unwrap()
    );
    assert_eq!(s.original_player_pickups(PlayerSlot::One).unwrap(), before);
}
#[test]
fn sustained_default_loadout_collects_supplies_after_real_damage_and_wins() {
    let c = &source::cases()[46];
    let (mut s, a) = fresh(46);
    let mut healed = false;
    let mut hurt = false;
    let mut shots = 0;
    for op in 1..=c.tics {
        s.advance(source::commands(c, op)).unwrap();
        hurt |= s
            .events()
            .iter()
            .any(|e| matches!(e, Event::PlayerBody(PlayerBodyEvent::Damaged { .. })));
        healed |= s
            .events()
            .iter()
            .any(|e| matches!(e, Event::Pickup(PickupEvent::Body(true, 92, 100, 100))));
        shots += s
            .events()
            .iter()
            .filter(|e| matches!(e, Event::Weapon(WeaponEvent::Bullet { .. })))
            .count();
    }
    assert!(hurt && healed);
    assert!(shots > 3);
    assert_eq!(s.original_player(PlayerSlot::One).unwrap().health, 64);
    assert!(s.actor(a.origin.unwrap()).unwrap().life().unwrap().health <= 0);
}
#[test]
fn sustained_losing_route_retains_player_corpse_and_roster() {
    let c = &source::cases()[49];
    let (mut s, a) = fresh(49);
    for op in 1..=c.tics {
        s.advance(source::commands(c, op)).unwrap();
    }
    let p = s.original_player(PlayerSlot::One).unwrap();
    assert_eq!(p.state, PlayerBodyState::Dead);
    assert_eq!(p.mo, Some(a.body));
    assert_eq!(p.attacker, a.origin);
    assert!(s.original_player_roster().unwrap()[0].active);
    assert!(s.actor(a.body).unwrap().life().unwrap().health <= 0);
}
#[test]
fn pickup_refs_reject_foreign_game_instances() {
    let (s, _) = fresh(1);
    let i = item(&s);
    let mut left = GameInstance::new(s.clone());
    let mut right = GameInstance::new(s);
    let reference = left
        .with_execution_context(|ctx| {
            let i = ctx.admit_local(i, DefinitionIdentity::Original(OriginalActorType::MT_CLIP))?;
            ctx.export_actor(i)
        })
        .unwrap();
    let before = right.simulation().snapshot();
    assert!(
        right
            .with_execution_context(|ctx| {
                ctx.import_actor(
                    &reference,
                    DefinitionIdentity::Original(OriginalActorType::MT_CLIP),
                )?;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(right.simulation().snapshot(), before);
}
#[test]
fn dsim25_is_explicit_and_old_readers_cannot_reinterpret_it() {
    let (s, _) = fresh(1);
    let b = s
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    assert!(Diagnostic::decode(&b, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_player(&b, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_pistol(&b, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_weapons(&b, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_pickups(&b, DiagnosticLimits::default()).is_ok());
}

fn replace_field(bytes: &mut [u8], name: &str, value: &[u8]) {
    let mut at = 16;
    while at < bytes.len() {
        let n = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        at += 4;
        let yes = &bytes[at..at + n] == name.as_bytes();
        at += n;
        let len = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
        at += 8;
        if yes {
            assert_eq!(len, value.len());
            bytes[at..at + len].copy_from_slice(value);
            return;
        }
        at += len;
    }
    panic!("missing committed schema field {name}");
}
#[test]
fn malformed_pickup_schema_fields_and_static_records_reject_safely() {
    let (s, _) = fresh(1);
    let bytes = s
        .snapshot()
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    for (name, value) in [
        ("profile", 8_u32.to_le_bytes().to_vec()),
        (
            "original_player.0.pickups.bonus_count",
            97_i32.to_le_bytes().to_vec(),
        ),
        (
            "original_player.0.pickups.item_count",
            1_i32.to_le_bytes().to_vec(),
        ),
        ("original_player.0.pickups.message", vec![7]),
        ("actor.0.flags", 3_u32.to_le_bytes().to_vec()),
        ("actor.0.sprite", 0_u16.to_le_bytes().to_vec()),
        ("actor.0.tics", 0_i32.to_le_bytes().to_vec()),
        ("actor.0.spawnpoint", vec![0; 11]),
    ] {
        let mut bad = bytes.clone();
        replace_field(&mut bad, name, &value);
        assert!(
            Diagnostic::decode_pickups(&bad, DiagnosticLimits::default()).is_err(),
            "{name}"
        );
    }
    for length in [0, 4, 16, bytes.len() - 1] {
        assert!(Diagnostic::decode_pickups(&bytes[..length], DiagnosticLimits::default()).is_err());
    }
}
#[test]
fn malformed_pickup_event_payloads_reject_with_bounded_parsing() {
    let (mut s, _) = fresh(1);
    s.advance(source::commands(&source::cases()[1], 1)).unwrap();
    let snap = s.snapshot();
    let bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
    for field in snap
        .fields()
        .filter(|f| f.path().starts_with("events.last.") && f.value().first() == Some(&23))
    {
        let mut value = field.value().to_vec();
        value[1] = 255;
        let mut bad = bytes.clone();
        replace_field(&mut bad, field.path(), &value);
        assert!(Diagnostic::decode_pickups(&bad, DiagnosticLimits::default()).is_err());
    }
}
#[test]
fn late_health_failure_discards_healing_and_retirement() {
    let (mut s, _) = fresh(26);
    s.original_player
        .as_mut()
        .unwrap()
        .pickups
        .as_mut()
        .unwrap()
        .bonus_count = 96;
    rejected(&mut s, source::commands(&source::cases()[26], 1));
    assert_eq!(s.original_player(PlayerSlot::One).unwrap().health, 1);
}
