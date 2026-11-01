//! Rust ownership/atomicity policies; source equivalence is tested independently.
use super::*;
use crate::weapon_pack_evidence as source;
fn fresh() -> Simulation {
    Simulation::original_player_demon_weapons_map(
        source::case_map(0),
        64,
        MapContextOptions::default(),
    )
    .unwrap()
}
fn inventory() -> WeaponInventory {
    WeaponInventory::default()
}
fn warm(index: usize, tic: usize) -> (Simulation, source::Actors) {
    let c = &source::cases()[index];
    let (mut s, a) = source::setup(c);
    for op in 1..=tic {
        s.advance(source::commands(c, op)).unwrap();
    }
    (s, a)
}
#[test]
fn initial_loadout_is_checked_once_and_does_not_grant_unowned_weapons() {
    let inputs = [
        WeaponInventory {
            ready: Weapon::Shotgun,
            ..inventory()
        },
        WeaponInventory {
            ready: Weapon::Rocket,
            ..inventory()
        },
        WeaponInventory {
            owned: 1,
            ..inventory()
        },
        WeaponInventory {
            owned: 31,
            ..inventory()
        },
        WeaponInventory {
            ammo: [201, 0, 0, 0],
            ..inventory()
        },
        WeaponInventory {
            ammo: [0, 51, 0, 0],
            ..inventory()
        },
        WeaponInventory {
            ammo: [0, 0, 1, 0],
            ..inventory()
        },
        WeaponInventory {
            ammo: [-1, 0, 0, 0],
            ..inventory()
        },
    ];
    for input in inputs {
        let mut s = fresh();
        s.spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
            .unwrap();
        let before = s.snapshot();
        assert!(s.initialize_player_weapons(PlayerSlot::One, input).is_err());
        assert_eq!(s.snapshot(), before);
    }
    let mut s = fresh();
    let before = s.snapshot();
    assert!(
        s.initialize_player_weapons(PlayerSlot::One, inventory())
            .is_err()
    );
    assert_eq!(s.snapshot(), before);
    s.spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap();
    s.initialize_player_weapons(PlayerSlot::One, inventory())
        .unwrap();
    let before = s.snapshot();
    assert!(
        s.initialize_player_weapons(PlayerSlot::One, inventory())
            .is_err()
    );
    assert_eq!(s.snapshot(), before);
}
#[test]
fn historical_pistol_policy_and_weapon_observations_remain_isolated() {
    let mut s = Simulation::original_player_demon_pistol_map(
        source::case_map(0),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    s.spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap();
    s.initialize_player_pistol(PlayerSlot::One, 1).unwrap();
    let before = s.snapshot();
    assert!(
        s.initialize_player_weapons(PlayerSlot::One, inventory())
            .is_err()
    );
    let mut c = [TicCommand::default(); 4];
    c[0].buttons = BT_CHANGE;
    assert!(s.advance(c).is_err());
    assert_eq!(s.snapshot(), before);
    assert!(s.original_player_weapons(PlayerSlot::One).is_err());
    let bytes = before
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    assert!(Diagnostic::decode_weapons(&bytes, DiagnosticLimits::default()).is_err());
}
#[test]
fn malformed_weapon_state_rejects_without_mutation() {
    for mode in 0..12 {
        let (mut s, _) = warm(0, 0);
        let PlayerWeaponRuntime::Pack(state) =
            s.original_player.as_mut().unwrap().weapon.as_mut().unwrap()
        else {
            panic!("pack")
        };
        match mode {
            0 => state.bullets = 201,
            1 => state.shells = 51,
            2 => state.refire = -1,
            3 => state.owned = 0,
            4 => state.ready = Weapon::Rocket,
            5 => state.pending = Some(Weapon::Shotgun),
            6 => state.pending = Some(Weapon::Bfg),
            7 => state.extra_light = 3,
            8 => state.psprites[0].state = Some(StateId(OriginalState::S_CHAIN3.ordinal())),
            9 => state.psprites[0].tics = 0,
            10 => state.bob = Fixed::from_bits(weapons::MAX_BOB_BITS + 1),
            _ => state.psprites[0].position[1] = Fixed::from_map_units(129),
        }
        let before = s.snapshot();
        assert!(s.advance([TicCommand::default(); 4]).is_err(), "mode{mode}");
        assert_eq!(s.snapshot(), before);
    }
}
#[test]
fn unsupported_commands_and_slots_are_atomic() {
    let (mut s, _) = warm(0, 0);
    for (slot, buttons) in [
        (0, 2),
        (0, 128),
        (0, 8),
        (0, BT_CHANGE | 32),
        (0, BT_CHANGE | 56),
        (1, BT_ATTACK),
        (3, BT_CHANGE),
    ] {
        let mut commands = [TicCommand::default(); 4];
        commands[slot].buttons = buttons;
        let before = s.snapshot();
        let captures = snapshot::capture_count();
        assert!(s.advance(commands).is_err());
        assert_eq!(snapshot::capture_count(), captures);
        assert_eq!(s.snapshot(), before);
    }
}
#[test]
fn pending_retired_and_reused_bodies_never_rebind_weapon_ownership() {
    let (mut s, a) = warm(0, 0);
    assert!(s.mark_remove(a.body).is_err());
    s.actors.mark_remove(a.body).unwrap();
    let before = s.snapshot();
    assert!(s.advance([TicCommand::default(); 4]).is_err());
    assert_eq!(s.snapshot(), before);
    s.spatial.unlink(a.body);
    s.actors.run(64, |_, _| {}).unwrap();
    let effect = s
        .spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::ZERO; 2],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(effect.slot(), a.body.slot());
    assert_ne!(effect.generation(), a.body.generation());
    let before = s.snapshot();
    assert!(s.advance([TicCommand::default(); 4]).is_err());
    assert_eq!(s.snapshot(), before);
}
#[test]
fn late_shotgun_capacity_failure_discards_earlier_pellets_damage_rng_and_ammo() {
    let c = &source::cases()[12];
    let (mut s, a) = warm(12, 17);
    s.actors.max_slots = 4;
    let before = s.snapshot();
    let result = s.checked_demon_operation(|host| {
        let mut sound = Some(a.body);
        let result = player_pistol::stage(
            host,
            source::commands(c, 18)[0],
            17,
            std::slice::from_mut(&mut sound),
        );
        assert!(result.is_err());
        assert!(
            host.events
                .iter()
                .filter(|e| matches!(e, Event::Weapon(WeaponEvent::Bullet { .. })))
                .count()
                >= 3
        );
        assert!(
            host.access
                .get(a.origin.unwrap())
                .unwrap()
                .life()
                .unwrap()
                .health
                < 150
        );
        host.fail_sight(result.unwrap_err());
    });
    assert_eq!(result, Err(StoreError::Capacity.into()));
    assert_eq!(s.snapshot(), before);
    let captures = snapshot::capture_count();
    assert_eq!(
        s.advance(source::commands(c, 18)),
        Err(StoreError::Capacity.into())
    );
    assert_eq!(snapshot::capture_count(), captures);
    assert_eq!(s.snapshot(), before);
}
#[test]
fn completed_shotgun_then_late_error_rolls_back_all_seven_pellets() {
    let c = &source::cases()[12];
    let (mut s, a) = warm(12, 17);
    let before = s.snapshot();
    let result = s.checked_demon_operation(|host| {
        let mut sound = Some(a.body);
        player_pistol::stage(
            host,
            source::commands(c, 18)[0],
            17,
            std::slice::from_mut(&mut sound),
        )
        .unwrap();
        assert_eq!(
            host.events
                .iter()
                .filter(|e| matches!(e, Event::Weapon(WeaponEvent::Bullet { .. })))
                .count(),
            7
        );
        assert_eq!(
            host.player
                .as_ref()
                .unwrap()
                .weapon
                .unwrap()
                .pack()
                .unwrap()
                .shells,
            4
        );
        host.fail_sight(PerceptionError::WorkLimit.into());
    });
    assert_eq!(result, Err(PerceptionError::WorkLimit.into()));
    assert_eq!(s.snapshot(), before);
}
#[test]
fn chaingun_immediate_fire_then_noise_is_one_atomic_transaction() {
    let c = &source::cases()[20];
    let (mut s, a) = warm(20, 14);
    let before = s.snapshot();
    let result = s.checked_demon_operation(|host| {
        let mut sound = None;
        player_pistol::stage(
            host,
            source::commands(c, 15)[0],
            14,
            std::slice::from_mut(&mut sound),
        )
        .unwrap();
        assert_eq!(sound, Some(a.body));
        let bullet = host
            .events
            .iter()
            .position(|e| matches!(e, Event::Weapon(WeaponEvent::Bullet { .. })))
            .unwrap();
        let noise = host
            .events
            .iter()
            .position(|e| matches!(e, Event::Weapon(WeaponEvent::Noise { .. })))
            .unwrap();
        assert!(bullet < noise);
        assert_eq!(
            host.player
                .as_ref()
                .unwrap()
                .weapon
                .unwrap()
                .pack()
                .unwrap()
                .bullets,
            49
        );
        host.fail_sight(PerceptionError::WorkLimit.into());
    });
    assert_eq!(result, Err(PerceptionError::WorkLimit.into()));
    assert_eq!(s.snapshot(), before);
}
#[test]
fn source_fist_fallback_switching_and_four_weapon_kills_are_real_command_actions() {
    for (index, killed) in [
        (1, false),
        (8, false),
        (13, true),
        (18, true),
        (25, true),
        (26, true),
    ] {
        let c = &source::cases()[index];
        let (mut s, a) = source::setup(c);
        let mut fists = 0;
        let mut fist_hits = 0;
        for op in 1..=c.tics {
            let health = s.actor(a.origin.unwrap()).unwrap().life().unwrap().health;
            s.advance(source::commands(c, op)).unwrap();
            if s.original_player_weapons(PlayerSlot::One)
                .unwrap()
                .unwrap()
                .ready
                == Weapon::Fist
            {
                fist_hits += usize::from(
                    s.actor(a.origin.unwrap()).unwrap().life().unwrap().health < health,
                );
                fists += s
                .events()
                .iter()
                .filter(
                    |e| matches!(e,Event::Weapon(WeaponEvent::Bullet{damage,..}) if damage%2==0),
                )
                .count();
            }
        }
        if killed {
            assert!(s.actor(a.origin.unwrap()).unwrap().life().unwrap().health <= 0);
            assert_eq!(s.level_statistics().unwrap().single_player_kill_count, 1);
        }
        if index == 1 || index == 26 {
            assert!(fists > 0);
            assert!(fist_hits > 0);
            assert_eq!(
                s.original_player_weapons(PlayerSlot::One)
                    .unwrap()
                    .unwrap()
                    .ready,
                Weapon::Fist
            );
        }
    }
}
#[test]
fn no_implicit_snapshot_in_every_weapon_and_dead_or_paused_continuation() {
    for index in [0, 1, 4, 10, 13, 18, 22, 25, 26] {
        let c = &source::cases()[index];
        let (mut s, _) = source::setup(c);
        let mut captured = s.clone();
        for op in 1..=c.tics {
            let count = snapshot::capture_count();
            s.advance(source::commands(c, op)).unwrap();
            assert_eq!(snapshot::capture_count(), count);
            assert_eq!(
                captured.step(source::commands(c, op)).unwrap(),
                s.snapshot()
            );
        }
        s.set_paused(true);
        captured.set_paused(true);
        let count = snapshot::capture_count();
        s.advance([TicCommand::default(); 4]).unwrap();
        assert_eq!(snapshot::capture_count(), count);
        assert_eq!(
            captured.step([TicCommand::default(); 4]).unwrap(),
            s.snapshot()
        );
    }
}
#[test]
fn dsim24_round_trip_rejects_historical_readers_and_incompatible_policy() {
    let (s, _) = warm(25, 64);
    let snapshot = s.snapshot();
    let limits = DiagnosticLimits::default();
    let bytes = snapshot.diagnostic_bytes(limits).unwrap();
    assert!(Diagnostic::decode(&bytes, limits).is_err());
    assert!(Diagnostic::decode_player(&bytes, limits).is_err());
    assert!(Diagnostic::decode_pistol(&bytes, limits).is_err());
    let decoded = Diagnostic::decode_weapons(&bytes, limits).unwrap();
    assert_eq!(decoded.encode(limits).unwrap(), bytes);
    for key in [
        "profile",
        "original_player.0.weapons.ready",
        "original_player.0.weapons.owned",
        "original_player.0.weapons.shells",
        "original_player.0.weapons.attack_down",
    ] {
        let target = decoded.field(key).unwrap();
        let offset = target.as_ptr() as usize - bytes.as_ptr() as usize;
        let mut bad = bytes.clone();
        bad[offset] = 255;
        assert!(Diagnostic::decode_weapons(&bad, limits).is_err(), "{key}");
    }
}

#[test]
fn foreign_instance_and_wrong_definition_cannot_become_the_weapon_body() {
    let (s, a) = warm(1, 0);
    let mut one = GameInstance::new(s.clone());
    let mut two = GameInstance::new(s);
    let expected = DefinitionIdentity::Original(OriginalActorType::MT_PLAYER);
    let reference = one
        .with_execution_context(|ctx| ctx.export_actor(ctx.admit_local(a.body, expected)?))
        .unwrap();
    let before = two.simulation().snapshot();
    two.with_execution_context(|ctx| {
        assert!(matches!(
            ctx.import_actor(&reference, expected),
            Err(ProvenanceError::InstanceMismatch)
        ));
        assert!(
            ctx.admit_local(
                a.body,
                DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT)
            )
            .is_err()
        );
        Ok(())
    })
    .unwrap();
    assert_eq!(two.simulation().snapshot(), before);
}
#[test]
fn inconsistent_player_body_binding_and_health_reject_atomically() {
    for mode in 0..3 {
        let (mut s, a) = warm(1, 0);
        let p = s.original_player.as_mut().unwrap();
        match mode {
            0 => p.source.mo = a.origin,
            1 => p.source.health = 99,
            _ => p.source.mo = Some(ActorId::from_parts(a.body.slot(), a.body.generation() + 1)),
        }
        let before = s.snapshot();
        assert!(s.advance([TicCommand::default(); 4]).is_err());
        assert_eq!(s.snapshot(), before);
    }
}
#[test]
fn completed_lethal_shotgun_then_late_error_restores_statistics_and_corpse_fields() {
    let c = &source::cases()[13];
    let (mut s, a) = warm(13, 165);
    let before = s.snapshot();
    let result = s.checked_demon_operation(|host| {
        let mut sound = Some(a.body);
        player_pistol::stage(
            host,
            source::commands(c, 166)[0],
            165,
            std::slice::from_mut(&mut sound),
        )
        .unwrap();
        assert!(
            host.access
                .get(a.origin.unwrap())
                .unwrap()
                .life()
                .unwrap()
                .health
                <= 0
        );
        assert_eq!(
            host.statistics.as_deref().unwrap().single_player_kill_count,
            1
        );
        host.fail_sight(PerceptionError::WorkLimit.into());
    });
    assert_eq!(result, Err(PerceptionError::WorkLimit.into()));
    assert_eq!(s.snapshot(), before);
}

#[test]
fn weapon_storage_variant_must_match_the_explicit_execution_policy() {
    let (mut s, _) = warm(0, 0);
    s.original_player.as_mut().unwrap().weapon =
        Some(PlayerWeaponRuntime::Pistol(crate::PistolState {
            bullets: 50,
            attack_down: true,
            refire: 0,
            bob: Fixed::ZERO,
            extra_light: 0,
            psprites: [
                Psprite {
                    state: Some(StateId(OriginalState::S_PISTOLUP.ordinal())),
                    tics: 1,
                    position: [Fixed::ZERO, Fixed::from_map_units(122)],
                },
                Psprite::default(),
            ],
        }));
    let before = s.snapshot();
    assert_eq!(
        s.advance([TicCommand::default(); 4]),
        Err(WeaponError::Inventory.into())
    );
    assert_eq!(s.snapshot(), before);
}
