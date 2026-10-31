//! Rust admission, lifetime, decoding and rollback policy, separate from C evidence.
#[path = "../../../tests/support/m3g7.rs"]
mod source;
use doom_sim::{identity::*, *};
use doom_types::Fixed;

fn through(index: usize, tics: usize) -> (Simulation, source::Actors) {
    let c = &source::cases()[index];
    let (mut sim, actors) = source::setup(c);
    for op in 1..=tics {
        sim.advance(source::commands(c, op)).unwrap();
    }
    (sim, actors)
}
fn rejected(sim: &mut Simulation, action: impl FnOnce(&mut Simulation) -> Result<(), SimError>) {
    let before = sim.snapshot();
    let roster = sim.original_player_roster();
    assert!(action(sim).is_err());
    assert_eq!(sim.snapshot(), before);
    assert_eq!(sim.original_player_roster(), roster);
}
#[test]
fn capability_admits_only_current_original_roles_and_body_attack_states() {
    let new =
        OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonPistolV1).unwrap();
    let old = OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonMeleeV1).unwrap();
    assert_eq!(OriginalExecutionProfile::PlayerDemonPistolV1.wire_id(), 7);
    for ty in OriginalActorType::all() {
        assert_eq!(
            new.admit_definition(DefinitionIdentity::Original(ty))
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
    for state in [OriginalState::S_PLAY_ATK1, OriginalState::S_PLAY_ATK2] {
        let player = DefinitionIdentity::Original(OriginalActorType::MT_PLAYER);
        assert_eq!(
            new.admit_state(player, state).unwrap(),
            StateId(state.ordinal())
        );
        assert!(old.admit_state(player, state).is_err());
    }
    assert!(
        new.admit_definition(DefinitionIdentity::Context(ContextRole::UnboundBody))
            .is_err()
    );
    assert!(
        new.admit_state(
            DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT),
            OriginalState::S_SARG_RAISE1
        )
        .is_err()
    );
    assert!(
        new.admit_state(
            DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
            OriginalState::S_PISTOL
        )
        .is_err()
    );
}
#[test]
fn setup_requires_a_current_bound_body_and_only_bounded_initial_ammo() {
    let c = &source::cases()[0];
    let mut sim = Simulation::original_player_demon_pistol_map(
        source::case_map(c.map),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    rejected(&mut sim, |s| {
        s.initialize_player_pistol(PlayerSlot::One, 50)
    });
    sim.spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap();
    let body = sim
        .spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap();
    rejected(&mut sim, |s| {
        s.initialize_player_pistol(PlayerSlot::One, 201)
    });
    let snap = sim.snapshot();
    let bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
    Diagnostic::decode_pistol(&bytes, DiagnosticLimits::default()).unwrap();
    sim.initialize_player_pistol(PlayerSlot::One, 50).unwrap();
    assert_eq!(sim.original_player(PlayerSlot::One).unwrap().mo, Some(body));
    rejected(&mut sim, |s| {
        s.initialize_player_pistol(PlayerSlot::One, 50)
    });
}
#[test]
fn unsupported_buttons_slots_and_command_magnitudes_preserve_complete_state() {
    for kind in 0..7 {
        let (mut s, _) = through(0, 18);
        let mut commands = [TicCommand::default(); 4];
        match kind {
            0 => commands[0].buttons = 2,
            1 => commands[0].buttons = BT_CHANGE,
            2 => commands[0].buttons = u8::MAX,
            3 => commands[1].buttons = BT_ATTACK,
            4 => commands[3].forward = 1,
            5 => commands[0].forward = 127,
            _ => commands[0].side = -128,
        }
        rejected(&mut s, |s| s.advance(commands));
        rejected(&mut s, |s| s.step(commands).map(|_| ()));
    }
    assert!(PlayerSlot::try_from(1).is_err());
}
#[test]
fn historical_melee_policy_keeps_its_no_weapon_contract() {
    let c = &source::cases()[0];
    let mut s = Simulation::original_player_demon_melee_map(
        source::case_map(c.map),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    s.spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap();
    s.spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap();
    rejected(&mut s, |s| s.initialize_player_pistol(PlayerSlot::One, 50));
    let mut commands = [TicCommand::default(); 4];
    commands[0].buttons = BT_ATTACK;
    assert_eq!(s.advance(commands), Err(SimError::UnsupportedButtons));
    assert_eq!(s.snapshot().version(), 22);
}
#[test]
fn initial_setup_and_first_shot_do_not_consume_ammo_at_attack_selection() {
    let c = &source::cases()[0];
    let (mut s, _) = source::setup(c);
    let initial = s.original_player_pistol(PlayerSlot::One).unwrap().unwrap();
    assert_eq!(initial.bullets, 50);
    assert!(initial.attack_down);
    assert_eq!(
        initial.psprites[0].state,
        Some(StateId(OriginalState::S_PISTOLUP.ordinal()))
    );
    assert_eq!(initial.psprites[0].position[1], Fixed::from_map_units(122));
    for op in 1..=19 {
        s.advance(source::commands(c, op)).unwrap();
        assert_eq!(
            s.original_player_pistol(PlayerSlot::One)
                .unwrap()
                .unwrap()
                .bullets,
            if op < 19 { 50 } else { 49 }
        );
        assert_eq!(
            s.events()
                .iter()
                .any(|e| matches!(e, Event::Weapon(WeaponEvent::Bullet { .. }))),
            op == 19
        );
    }
}
#[test]
fn source_held_cadence_and_release_repress_use_the_shared_psprite_machine() {
    for (index, expected) in [
        (3, vec![25, 39, 53, 67, 81, 95, 109, 123, 137, 151]),
        (4, vec![25, 74, 88]),
    ] {
        let c = &source::cases()[index];
        let (mut s, _) = source::setup(c);
        let mut shots = Vec::new();
        for op in 1..=c.tics {
            s.advance(source::commands(c, op)).unwrap();
            if s.events()
                .iter()
                .any(|e| matches!(e, Event::Weapon(WeaponEvent::Bullet { .. })))
            {
                shots.push(op);
            }
        }
        assert_eq!(shots, expected);
    }
}
#[test]
fn initial_accuracy_and_refire_spread_have_exact_source_draw_counts() {
    let c = &source::cases()[3];
    let (mut s, _) = source::setup(c);
    let mut shots = 0;
    for op in 1..=39 {
        s.advance(source::commands(c, op)).unwrap();
        if s.events()
            .iter()
            .any(|e| matches!(e, Event::Weapon(WeaponEvent::Bullet { .. })))
        {
            let kinds: Vec<_> = s
                .events()
                .iter()
                .filter_map(|e| match e {
                    Event::Weapon(WeaponEvent::Random { kind, .. }) => Some(*kind),
                    _ => None,
                })
                .collect();
            assert_eq!(kinds, if shots == 0 { vec![0] } else { vec![0, 1, 2] });
            shots += 1;
        }
    }
    assert_eq!(shots, 2);
}
#[test]
fn blocked_and_fallback_shots_make_real_puffs_without_fabricated_demon_damage() {
    for index in [6, 7, 15, 16] {
        let c = &source::cases()[index];
        let (mut s, a) = source::setup(c);
        let mut puffs = 0;
        for op in 1..=c.tics {
            s.advance(source::commands(c, op)).unwrap();
            assert!(!s.events().iter().any(|e|matches!(e,Event::Combat(CombatEvent::Damage(r))if Some(r.target)==a.origin && r.source==Some(a.body))));
            puffs+=s.events().iter().filter(|e|matches!(e,Event::ActorState(ActorStateEvent::Initialized{actor_type,..})if actor_type.0==OriginalActorType::MT_PUFF.ordinal())).count();
        }
        assert!(puffs > 0);
    }
}
#[test]
fn empty_ammo_fallback_rejects_at_the_source_action_and_rolls_back() {
    let c = &source::cases()[11];
    let (mut idle, _) = source::setup(c);
    for op in 1..=15 {
        idle.advance(source::commands(c, op)).unwrap();
    }
    let mut command = [TicCommand::default(); 4];
    command[0].buttons = BT_ATTACK;
    rejected(&mut idle, |s| s.advance(command));
    let (mut last, _) = through(10, 28);
    assert_eq!(
        last.original_player_pistol(PlayerSlot::One)
            .unwrap()
            .unwrap()
            .bullets,
        0
    );
    // ReFire checks fallback even when attack has just been released.
    rejected(&mut last, |s| s.advance([TicCommand::default(); 4]));
    rejected(&mut last, |s| s.advance(command));
}
#[test]
fn actual_two_way_combat_credits_the_player_and_keeps_the_demon_corpse() {
    let c = &source::cases()[0];
    let (mut s, a) = source::setup(c);
    let mut shots = 0;
    let mut pain = 0;
    let mut hits = 0;
    let mut bites = 0;
    for op in 1..=c.tics {
        s.advance(source::commands(c, op)).unwrap();
        for event in s.events() {
            match event {
                Event::Weapon(WeaponEvent::Bullet { actor, damage, .. }) => {
                    assert_eq!(*actor, a.body);
                    assert!([5, 10, 15].contains(damage));
                    shots += 1;
                }
                Event::Combat(CombatEvent::Damage(r)) if Some(r.target) == a.origin => {
                    assert_eq!((r.source, r.inflictor), (Some(a.body), Some(a.body)));
                    hits += 1;
                }
                Event::Combat(CombatEvent::Damage(r)) if r.target == a.body => {
                    assert_eq!((r.source, r.inflictor), (a.origin, a.origin));
                    bites += 1;
                }
                Event::Combat(CombatEvent::Pain { target, .. }) if Some(*target) == a.origin => {
                    pain += 1
                }
                _ => {}
            }
        }
    }
    assert!(shots > 0 && hits > 0 && bites > 0 && pain > 0);
    assert_eq!(s.original_player(PlayerSlot::One).unwrap().health, 60);
    assert_eq!(s.level_statistics().unwrap().single_player_kill_count, 1);
    assert_eq!(
        s.actor(a.origin.unwrap()).unwrap().life().unwrap().state,
        Some(StateId(OriginalState::S_SARG_DIE6.ordinal()))
    );
    assert_eq!(
        s.snapshot()
            .fields()
            .find(|f| f.path() == format!("actor.{}.pending_removal", a.origin.unwrap().slot()))
            .unwrap()
            .value(),
        [0]
    );
    assert_ne!(
        s.original_player_pistol(PlayerSlot::One)
            .unwrap()
            .unwrap()
            .psprites[0]
            .state,
        Some(StateId(OriginalState::S_PISTOLDOWN.ordinal()))
    );
}
#[test]
fn death_during_windup_lowers_weapon_and_retains_owned_active_corpse() {
    let c = &source::cases()[9];
    let (mut s, a) = source::setup(c);
    for op in 1..=c.tics {
        s.advance(source::commands(c, op)).unwrap();
        assert!(
            !s.events()
                .iter()
                .any(|e| matches!(e, Event::Weapon(WeaponEvent::Bullet { .. })))
        );
    }
    let p = s.original_player(PlayerSlot::One).unwrap();
    assert_eq!(p.state, PlayerBodyState::Dead);
    assert_eq!((p.mo, p.attacker), (Some(a.body), a.origin));
    assert!(s.original_player_roster().unwrap()[0].active);
    assert_eq!(
        s.snapshot()
            .fields()
            .find(|f| f.path() == format!("actor.{}.pending_removal", a.body.slot()))
            .unwrap()
            .value(),
        [0]
    );
    assert!(s.actor(a.body).unwrap().life().unwrap().health <= 0);
    let pistol = s.original_player_pistol(PlayerSlot::One).unwrap().unwrap();
    assert_eq!(pistol.bullets, 50);
    assert_eq!(
        pistol.psprites[0].state,
        Some(StateId(OriginalState::S_PISTOLDOWN.ordinal()))
    );
    assert_eq!(pistol.psprites[0].position[1], Fixed::from_map_units(128));
    let mut command = [TicCommand::default(); 4];
    command[0].buttons = BT_ATTACK;
    rejected(&mut s, |s| s.advance(command));
}
#[test]
fn bound_body_cannot_be_retired_or_rebound_while_pistol_ownership_is_active() {
    let (mut s, a) = through(0, 18);
    rejected(&mut s, |s| s.mark_remove(a.body));
    let mut command = [TicCommand::default(); 4];
    command[0].buttons = BT_ATTACK;
    s.advance(command).unwrap();
    rejected(&mut s, |s| {
        s.bind_original_player_body(PlayerSlot::One, a.origin.unwrap())
    });
    assert_eq!(s.original_player(PlayerSlot::One).unwrap().mo, Some(a.body));
}
#[test]
fn paused_weapon_tics_and_repeated_rejections_preserve_rng_and_all_diagnostics() {
    let (mut s, _) = through(0, 18);
    s.set_paused(true);
    let before = s.snapshot();
    let mut command = [TicCommand::default(); 4];
    command[0].buttons = u8::MAX;
    s.advance(command).unwrap();
    assert_eq!(s.step(command).unwrap(), before);
    assert_eq!(s.snapshot(), before);
    s.set_paused(false);
    for _ in 0..3 {
        rejected(&mut s, |s| s.advance(command));
    }
}
#[test]
fn explicit_new_schema_preserves_all_historical_reader_boundaries() {
    let (s, _) = through(0, 19);
    let snap = s.snapshot();
    assert_eq!(snap.version(), 23);
    let bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
    assert!(Diagnostic::decode(&bytes, DiagnosticLimits::default()).is_err());
    assert!(Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).is_err());
    Diagnostic::decode_pistol(&bytes, DiagnosticLimits::default()).unwrap();
    let old = include_bytes!("../../../tests/reference/m3g6_state_0_92.bin");
    Diagnostic::decode_player(old, DiagnosticLimits::default()).unwrap();
    assert!(Diagnostic::decode_pistol(old, DiagnosticLimits::default()).is_err());
    for (path, replacement) in [
        ("profile", vec![6, 0, 0, 0]),
        ("original_player.0.pistol.bullets", vec![201, 0]),
        (
            "original_player.0.pistol.psprite.0.state",
            OriginalState::S_SGUN.ordinal().to_le_bytes().to_vec(),
        ),
    ] {
        let mut bad = bytes.clone();
        // Replace only a framed value; preserve all lengths to test schema guards.
        let mut offset = 16;
        while offset < bad.len() {
            let size = u32::from_le_bytes(bad[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            let matches = &bad[offset..offset + size] == path.as_bytes();
            offset += size;
            let size = u64::from_le_bytes(bad[offset..offset + 8].try_into().unwrap()) as usize;
            offset += 8;
            if matches {
                assert_eq!(size, replacement.len());
                bad[offset..offset + size].copy_from_slice(&replacement);
                break;
            }
            offset += size;
        }
        assert!(Diagnostic::decode_pistol(&bad, DiagnosticLimits::default()).is_err());
    }
}

#[test]
fn foreign_instance_body_cannot_be_imported_as_this_players_weapon_owner() {
    let (s, a) = through(0, 18);
    let mut left = GameInstance::new(s.clone());
    let mut right = GameInstance::new(s);
    let reference = left
        .with_execution_context(|ctx| {
            let body = ctx.admit_local(
                a.body,
                DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
            )?;
            Ok(ctx.export_actor(body).unwrap())
        })
        .unwrap();
    let before = right.simulation().snapshot();
    right
        .with_execution_context(|ctx| {
            assert_eq!(
                ctx.import_actor(
                    &reference,
                    DefinitionIdentity::Original(OriginalActorType::MT_PLAYER)
                ),
                Err(ProvenanceError::InstanceMismatch)
            );
            Ok(())
        })
        .unwrap();
    assert_eq!(right.simulation().snapshot(), before);
}
