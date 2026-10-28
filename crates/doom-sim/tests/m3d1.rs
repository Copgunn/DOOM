//! Safe combat boundaries and determinism policies, separate from C comparisons.
#[path = "../../../tests/support/m3d1.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
fn case() -> fixtures::Case {
    fixtures::cases()[0].clone()
}
fn ids() -> (ActorId, ActorId) {
    (ActorId::from_parts(0, 1), ActorId::from_parts(1, 1))
}
fn request() -> DamageRequest {
    let (source, target) = ids();
    DamageRequest {
        target,
        source: Some(source),
        inflictor: Some(source),
        amount: 20,
    }
}
fn attack() -> LineAttack {
    LineAttack {
        angle: 0,
        range: Fixed::from_map_units(160),
        slope: Fixed::ZERO,
        damage: 20,
    }
}
#[test]
fn failed_damage_and_trace_requests_preserve_complete_state() {
    let mut sim = fixtures::setup(&case());
    let before = sim.snapshot();
    for id in [ActorId::from_parts(100, 1), ActorId::from_parts(1, 99)] {
        for r in [
            DamageRequest {
                target: id,
                ..request()
            },
            DamageRequest {
                source: Some(id),
                ..request()
            },
            DamageRequest {
                inflictor: Some(id),
                ..request()
            },
        ] {
            assert!(sim.damage_actor(r).is_err());
            assert_eq!(sim.snapshot(), before);
        }
    }
    for amount in [-1, 1001, i32::MAX] {
        assert_eq!(
            sim.damage_actor(DamageRequest {
                amount,
                ..request()
            }),
            Err(SimError::Combat(CombatError::Damage))
        );
        assert_eq!(sim.snapshot(), before);
    }
    for range in [
        Fixed::ZERO,
        Fixed::from_bits(-1),
        Fixed::from_map_units(2049),
    ] {
        assert!(
            sim.line_attack(ids().0, LineAttack { range, ..attack() })
                .is_err()
        );
        assert_eq!(sim.snapshot(), before);
    }
    let mut old = Simulation::actor_states(fixtures::map(0)).unwrap();
    let before = old.snapshot();
    assert!(old.damage_actor(request()).is_err());
    assert_eq!(old.snapshot(), before);
}
#[test]
fn pending_retired_and_reused_combat_handles_remain_safe() {
    let mut sim = fixtures::setup(&case());
    let (shooter, target) = ids();
    sim.set_actor_target(shooter, Some(target)).unwrap();
    sim.mark_remove(target).unwrap();
    let before = sim.snapshot();
    assert_eq!(
        sim.damage_actor(request()),
        Err(SimError::Combat(CombatError::Reference))
    );
    assert_eq!(sim.snapshot(), before);
    sim.step([TicCommand::default(); 4]).unwrap();
    let replacement = sim
        .spawn_type(
            ActorTypeId(136),
            [Fixed::from_map_units(64), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert_eq!(replacement.slot(), target.slot());
    assert_ne!(replacement.generation(), target.generation());
    assert!(sim.actor_target(shooter).is_none());
    let before = sim.snapshot();
    assert!(sim.damage_actor(request()).is_err());
    assert_eq!(sim.snapshot(), before);
    assert_eq!(
        sim.aim_line_attack(shooter, 0, attack().range)
            .unwrap()
            .target,
        Some(replacement)
    );
}
#[test]
fn equal_fraction_order_follows_relink_history_after_slot_reuse() {
    let mut c = case();
    c.values[16] = 2;
    let mut sim = fixtures::setup(&c);
    let first = ActorId::from_parts(1, 1);
    let second = ActorId::from_parts(2, 1);
    let shooter = ids().0;
    assert_eq!(
        sim.aim_line_attack(shooter, 0, attack().range)
            .unwrap()
            .target,
        Some(second)
    );
    sim.mark_remove(first).unwrap();
    sim.step([TicCommand::default(); 4]).unwrap();
    let new = sim
        .spawn_type(
            ActorTypeId(136),
            [Fixed::from_map_units(64), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    assert!(new.slot() < second.slot());
    assert_eq!(
        sim.aim_line_attack(shooter, 0, attack().range)
            .unwrap()
            .target,
        Some(new)
    );
    sim.line_attack(shooter, attack()).unwrap();
    assert_eq!(sim.actor(second).unwrap().life().unwrap().health, 100);
    assert_eq!(sim.actor(new).unwrap().life().unwrap().health, 80);
}
#[test]
fn bound_player_armor_weapon_and_scoring_paths_reject_before_effects() {
    let mut sim = fixtures::setup(&case());
    sim.bind_player(0, ids().0).unwrap();
    let before = sim.snapshot();
    assert_eq!(
        sim.damage_actor(request()),
        Err(SimError::Combat(CombatError::Player))
    );
    assert!(sim.line_attack(ids().0, attack()).is_err());
    assert!(sim.aim_line_attack(ids().0, 0, attack().range).is_err());
    assert_eq!(sim.snapshot(), before);
}
#[test]
fn mass_and_native_impact_changes_reject_before_spawn_rng() {
    for index in [2, 8, 18] {
        let mut r = fixtures::registry(&case());
        let mut types = r.types().to_vec();
        types[136].properties[index] = if index == 8 { 257 } else { 0 };
        r = ActorRegistry::new(r.states().to_vec(), types).unwrap();
        let mut sim =
            Simulation::with_combat_registry(fixtures::map(0), 8, r, ActorOptions::default())
                .unwrap();
        let before = sim.snapshot();
        assert!(
            sim.spawn_type(ActorTypeId(136), [Fixed::ZERO; 2], SpawnHeight::Floor)
                .is_err()
        );
        assert_eq!(sim.snapshot(), before);
    }
    let original = ActorRegistry::vanilla();
    let mut states = original.states().to_vec();
    let puff = original.find_state("S_PUFF3").unwrap();
    states[usize::from(puff.0)].action = ActorAction::Probe(ProbeAction::Random);
    let registry = ActorRegistry::new(states, original.types().to_vec()).unwrap();
    assert!(matches!(
        Simulation::with_combat_registry(fixtures::map(0), 8, registry, ActorOptions::default()),
        Err(SimError::Combat(CombatError::ImpactDefinition))
    ));
}
#[test]
fn excluded_item_drop_kill_branch_is_atomic() {
    let original = fixtures::registry(&case());
    let mut types = original.types().to_vec();
    types[1] = types[136].clone();
    let registry = ActorRegistry::new(original.states().to_vec(), types).unwrap();
    let mut sim =
        Simulation::with_combat_registry(fixtures::map(0), 8, registry, ActorOptions::default())
            .unwrap();
    let target = sim
        .spawn_type(ActorTypeId(1), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap();
    let before = sim.snapshot();
    assert_eq!(
        sim.damage_actor(DamageRequest {
            target,
            source: None,
            inflictor: None,
            amount: 100
        }),
        Err(SimError::Combat(CombatError::KillBranch))
    );
    assert_eq!(sim.snapshot(), before);
}
#[test]
fn impact_and_callback_capacity_preflight_is_atomic() {
    let mut c = case();
    c.values[12] = 256;
    let original = fixtures::registry(&c);
    let mut states = original.states().to_vec();
    let pain = original.find_state("S_PLAY_PAIN").unwrap();
    states[usize::from(pain.0)].action =
        ActorAction::Probe(ProbeAction::SpawnTarget(ActorTypeId(40)));
    let registry = ActorRegistry::new(states, original.types().to_vec()).unwrap();
    let mut sim =
        Simulation::with_combat_registry(fixtures::map(0), 3, registry, ActorOptions::default())
            .unwrap();
    sim.spawn_type(
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
    let before = sim.snapshot();
    assert_eq!(
        sim.line_attack(ids().0, attack()),
        Err(SimError::Store(StoreError::Capacity))
    );
    assert_eq!(sim.snapshot(), before);
    let before = sim.snapshot();
    sim.damage_actor(request()).unwrap();
    assert_ne!(sim.snapshot(), before);
    assert!(sim.actor(ActorId::from_parts(2, 1)).is_some());
}
#[test]
fn damage_pain_callbacks_can_remove_self_without_invalid_storage() {
    let mut c = case();
    c.values[12] = 256;
    let original = fixtures::registry(&c);
    let mut states = original.states().to_vec();
    let pain = original.find_state("S_PLAY_PAIN").unwrap();
    states[usize::from(pain.0)].action = ActorAction::Probe(ProbeAction::RemoveSelf);
    let registry = ActorRegistry::new(states, original.types().to_vec()).unwrap();
    let mut sim =
        Simulation::with_combat_registry(fixtures::map(0), 8, registry, ActorOptions::default())
            .unwrap();
    sim.spawn_type(ActorTypeId(0), [Fixed::ZERO; 2], SpawnHeight::Floor)
        .unwrap();
    sim.spawn_type(
        ActorTypeId(136),
        [Fixed::from_map_units(64), Fixed::ZERO],
        SpawnHeight::Floor,
    )
    .unwrap();
    sim.damage_actor(request()).unwrap();
    let target = sim.actor(ids().1).unwrap();
    assert_eq!(target.life().unwrap().health, 80);
    assert_eq!(target.life().unwrap().reaction_time, 0);
    assert_eq!(target.life().unwrap().target, Some(ids().0));
    assert!(!sim.spatial_order().contains(&ids().1));
    sim.step([TicCommand::default(); 4]).unwrap();
    assert!(sim.actor(ids().1).is_none());
}
#[test]
fn intercept_overflow_and_extreme_setup_reject_atomically() {
    let mut sim = Simulation::with_combat_registry(
        fixtures::map(0),
        256,
        fixtures::registry(&case()),
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
    for _ in 0..128 {
        sim.spawn_type(
            ActorTypeId(136),
            [Fixed::from_map_units(64), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    }
    let before = sim.snapshot();
    assert_eq!(
        sim.line_attack(shooter, attack()),
        Err(SimError::Combat(CombatError::InterceptLimit))
    );
    assert_eq!(sim.snapshot(), before);
    let far = sim
        .spawn_type(
            ActorTypeId(0),
            [Fixed::from_bits(i32::MAX), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    let before = sim.snapshot();
    assert!(sim.line_attack(far, attack()).is_err());
    assert_eq!(sim.snapshot(), before);
}
#[test]
fn zero_damage_shots_have_real_impacts_and_query_does_not() {
    let mut sim = fixtures::setup(&case());
    let before = sim.snapshot();
    let aim = sim.aim_line_attack(ids().0, 0, attack().range).unwrap();
    assert_eq!(aim.target, Some(ids().1));
    assert_eq!(sim.snapshot(), before);
    let rng = sim.rng().indices()[0];
    let result = sim
        .line_attack(
            ids().0,
            LineAttack {
                damage: 0,
                ..attack()
            },
        )
        .unwrap();
    assert!(result.effect.is_some());
    assert!(result.damage.is_none());
    assert_eq!(sim.actor(ids().1).unwrap().life().unwrap().health, 100);
    assert_eq!(sim.rng().indices()[0], rng.wrapping_add(4));
    assert_eq!(
        sim.actor(result.effect.unwrap())
            .unwrap()
            .vertical_momentum(),
        Fixed::from_map_units(2)
    );
}
#[test]
fn repeated_attack_traces_and_new_field_divergence_are_deterministic() {
    let c = case();
    let mut a = fixtures::setup(&c);
    let mut b = a.clone();
    for tic in 0..160 {
        if tic % 17 == 0 {
            assert_eq!(
                a.line_attack(
                    ids().0,
                    LineAttack {
                        damage: 0,
                        ..attack()
                    }
                ),
                b.line_attack(
                    ids().0,
                    LineAttack {
                        damage: 0,
                        ..attack()
                    }
                )
            );
        }
        assert_eq!(
            a.step([TicCommand::default(); 4]),
            b.step([TicCommand::default(); 4])
        );
        let _ = a.events();
        let _ = a.snapshot().digest();
    }
    let before = a.snapshot();
    a.damage_actor(request()).unwrap();
    let diff = before.first_difference(&a.snapshot()).unwrap();
    assert_eq!(diff.field, "rng.gameplay");
    assert_eq!(a.actor(ids().1).unwrap().life().unwrap().threshold, 100);
    assert_eq!(a.snapshot().canonical_bytes()[4..8], 5_u32.to_le_bytes());
}
#[test]
fn unsupported_sky_flags_and_insufficient_impact_clearance_reject() {
    use doom_map::{Map, MapLimits};
    use doom_wad::{Limits, Wad};
    for sky in [false, true] {
        let mut bytes = include_bytes!("../../../tests/reference/m3d1_map_0.bin").to_vec();
        let count = i32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        let directory = i32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let sector = (0..count)
            .find_map(|i| {
                let p = directory + i * 16;
                (&bytes[p + 8..p + 16] == b"SECTORS\0")
                    .then(|| i32::from_le_bytes(bytes[p..p + 4].try_into().unwrap()) as usize)
            })
            .unwrap();
        if sky {
            bytes[sector + 12..sector + 20].copy_from_slice(b"F_SKY1\0\0");
        } else {
            bytes[sector + 2..sector + 4].copy_from_slice(&8_i16.to_le_bytes());
        }
        let wad = Wad::parse(&bytes, Limits::default()).unwrap();
        let map = Map::from_wad(&wad, "MAP01", MapLimits::default()).unwrap();
        assert!(Simulation::combat(map).is_err());
    }
    for flags in [4096, 16777216, 4194304, 16384] {
        let mut c = case();
        c.values[15] |= flags;
        let mut sim = Simulation::with_combat_registry(
            fixtures::map(0),
            8,
            fixtures::registry(&c),
            ActorOptions::default(),
        )
        .unwrap();
        let before = sim.snapshot();
        assert!(
            sim.spawn_type(ActorTypeId(136), [Fixed::ZERO; 2], SpawnHeight::Floor)
                .is_err()
        );
        assert_eq!(sim.snapshot(), before);
    }
}
