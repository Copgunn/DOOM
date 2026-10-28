//! Project safety/determinism policies; independent C comparisons are separate.
#[path = "../../../tests/support/m3d2.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
fn actor() -> ActorId {
    ActorId::from_parts(0, 1)
}
fn commands(buttons: u8) -> [TicCommand; 4] {
    let mut c = [TicCommand::default(); 4];
    c[0].buttons = buttons;
    c
}
#[test]
fn unsupported_buttons_and_selections_reject_in_every_slot_atomically() {
    let mut sim = fixtures::setup(&fixtures::cases()[0]);
    for slot in 0..4 {
        for buttons in [2, 64, 128, 8, 4 + (4 << 3), 4 + (7 << 3)] {
            let before = sim.snapshot();
            let mut c = [TicCommand::default(); 4];
            c[slot].buttons = buttons;
            assert!(sim.step(c).is_err());
            assert_eq!(sim.snapshot(), before);
        }
    }
    let before = sim.snapshot();
    assert!(
        sim.step({
            let mut c = commands(1);
            c[3].buttons = 1;
            c
        })
        .is_err()
    );
    assert_eq!(sim.snapshot(), before);
}
#[test]
fn inventory_and_player_setup_failures_preserve_state() {
    let c = &fixtures::cases()[0];
    let mut sim = Simulation::with_weapon_registry(
        fixtures::map(0),
        64,
        fixtures::registry(c),
        ActorOptions::default(),
    )
    .unwrap();
    let id = sim
        .spawn_type(ActorTypeId(0), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap();
    for inventory in [
        WeaponInventory {
            ready: Weapon::Rocket,
            ..WeaponInventory::default()
        },
        WeaponInventory {
            ready: Weapon::Shotgun,
            ..WeaponInventory::default()
        },
        WeaponInventory {
            owned: 1,
            ..WeaponInventory::default()
        },
        WeaponInventory {
            ammo: [201, 0, 0, 0],
            ..WeaponInventory::default()
        },
        WeaponInventory {
            ammo: [50, -1, 0, 0],
            ..WeaponInventory::default()
        },
        WeaponInventory {
            ammo: [50, 0, 1, 0],
            ..WeaponInventory::default()
        },
    ] {
        let before = sim.snapshot();
        assert!(sim.initialize_weapon_player(0, id, inventory).is_err());
        assert_eq!(sim.snapshot(), before);
    }
    for bad in [ActorId::from_parts(0, 99), ActorId::from_parts(99, 1)] {
        let before = sim.snapshot();
        assert!(
            sim.initialize_weapon_player(0, bad, WeaponInventory::default())
                .is_err()
        );
        assert_eq!(sim.snapshot(), before);
    }
    sim.initialize_weapon_player(0, id, WeaponInventory::default())
        .unwrap();
    let before = sim.snapshot();
    assert!(
        sim.initialize_weapon_player(1, id, WeaponInventory::default())
            .is_err()
    );
    assert!(sim.bind_player(1, id).is_err());
    assert_eq!(sim.snapshot(), before);
}
#[test]
fn modified_psprite_actions_and_impact_metadata_reject_admission() {
    let source = ActorRegistry::vanilla();
    for name in ["S_PISTOL2", "S_SGUN2", "S_PLAY_ATK2", "S_LIGHTDONE"] {
        let mut states = source.states().to_vec();
        let id = source.find_state(name).unwrap();
        states[usize::from(id.0)].action = ActorAction::Probe(ProbeAction::RemoveSelf);
        let r = ActorRegistry::new(states, source.types().to_vec()).unwrap();
        assert!(
            Simulation::with_weapon_registry(fixtures::map(0), 64, r, ActorOptions::default())
                .is_err()
        );
    }
    let mut types = source.types().to_vec();
    types[0].properties[18] = 0;
    let r = ActorRegistry::new(source.states().to_vec(), types).unwrap();
    assert!(
        Simulation::with_weapon_registry(fixtures::map(0), 64, r, ActorOptions::default()).is_err()
    );
}
#[test]
fn shotgun_capacity_failure_preserves_ammo_rng_psprites_and_targets() {
    let c = &fixtures::cases()[5];
    let mut sim = Simulation::with_weapon_registry(
        fixtures::map(0),
        3,
        fixtures::registry(c),
        ActorOptions::default(),
    )
    .unwrap();
    let shooter = sim
        .spawn_type(
            ActorTypeId(0),
            [Fixed::from_map_units(-80), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    sim.spawn_type(
        ActorTypeId(136),
        [Fixed::from_map_units(64), Fixed::ZERO],
        SpawnHeight::Floor,
    )
    .unwrap();
    sim.initialize_weapon_player(
        0,
        shooter,
        WeaponInventory {
            ready: Weapon::Shotgun,
            owned: 7,
            ammo: [50, 8, 0, 0],
        },
    )
    .unwrap();
    let stream = fixtures::commands(5);
    let mut failed = false;
    for cmd in stream {
        let before = sim.snapshot();
        if let Err(e) = sim.step(cmd) {
            assert_eq!(e, SimError::Store(StoreError::Capacity));
            assert_eq!(sim.snapshot(), before);
            assert_eq!(sim.weapon_player().unwrap().ammo[1], 8);
            failed = true;
            break;
        }
    }
    assert!(failed);
}
#[test]
fn private_weapon_shooter_route_does_not_enable_direct_player_damage() {
    let mut sim = fixtures::setup(&fixtures::cases()[0]);
    let before = sim.snapshot();
    for request in [
        DamageRequest {
            target: actor(),
            inflictor: None,
            source: None,
            amount: 5,
        },
        DamageRequest {
            target: ActorId::from_parts(1, 1),
            inflictor: Some(actor()),
            source: Some(actor()),
            amount: 5,
        },
    ] {
        assert!(sim.damage_actor(request).is_err());
        assert_eq!(sim.snapshot(), before);
    }
    assert!(
        sim.line_attack(
            actor(),
            LineAttack {
                angle: 0,
                range: Fixed::from_map_units(2048),
                slope: Fixed::ZERO,
                damage: 5
            }
        )
        .is_err()
    );
    assert_eq!(sim.snapshot(), before);
    for cmd in fixtures::commands(0) {
        sim.step(cmd).unwrap();
    }
    assert!(
        sim.actor(ActorId::from_parts(1, 1))
            .unwrap()
            .life()
            .unwrap()
            .health
            < 1000
    );
}
#[test]
fn pending_retirement_reuse_and_noise_targets_keep_generations() {
    let mut sim = fixtures::setup(&fixtures::cases()[0]);
    for cmd in fixtures::commands(0) {
        sim.step(cmd).unwrap();
    }
    assert_eq!(sim.sound_targets()[0], Some(actor()));
    sim.mark_remove(actor()).unwrap();
    let before = sim.snapshot();
    assert!(sim.step(commands(1)).is_err());
    assert_eq!(sim.snapshot(), before);
    sim.detach_weapon_player().unwrap();
    sim.step(commands(0)).unwrap();
    assert!(sim.actor(actor()).is_none());
    let replacement = sim
        .spawn_type(
            ActorTypeId(0),
            [Fixed::from_map_units(-80), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(replacement.slot(), actor().slot());
    assert_ne!(replacement.generation(), actor().generation());
    assert_eq!(sim.sound_targets()[0], Some(actor()));
    assert!(sim.actor(sim.sound_targets()[0].unwrap()).is_none());
    sim.initialize_weapon_player(3, replacement, WeaponInventory::default())
        .unwrap();
    assert_eq!(sim.weapon_player().unwrap().actor, replacement);
}
#[test]
fn pause_and_presentation_reads_do_not_advance_weapons() {
    let mut a = fixtures::setup(&fixtures::cases()[22]);
    let mut b = a.clone();
    a.set_paused(true);
    let before = a.snapshot();
    for _ in 0..3 {
        assert_eq!(a.step(commands(255)).unwrap(), before);
    }
    a.set_paused(false);
    for cmd in fixtures::commands(22) {
        let got = a.step(cmd).unwrap();
        let expected = b.step(cmd).unwrap();
        let _ = a.events();
        let _ = a.pending_events();
        let _ = a.weapon_player();
        let _ = a.sound_targets();
        let _ = got.digest();
        assert_eq!(got, expected);
    }
    assert!(a.weapon_player().unwrap().refire > 0);
}
#[test]
fn weapon_field_divergence_reports_the_earliest_persistent_field() {
    let a = fixtures::setup(&fixtures::cases()[0]);
    let mut c = fixtures::cases()[0].clone();
    c.values[2] = 49;
    let b = fixtures::setup(&c);
    let d = a.snapshot().first_difference(&b.snapshot()).unwrap();
    assert_eq!(d.field, "weapons.player.ammo");
    assert_eq!(a.snapshot().version(), 6);
}
#[test]
fn extreme_bob_setup_rejects_before_motion_or_weapon_changes() {
    let mut sim = fixtures::setup(&fixtures::cases()[0]);
    sim.set_momentum(actor(), [Fixed::from_bits(i32::MAX), Fixed::ZERO])
        .unwrap();
    let before = sim.snapshot();
    assert_eq!(
        sim.step(commands(1)),
        Err(SimError::Weapon(WeaponError::Momentum))
    );
    assert_eq!(sim.snapshot(), before);
}
