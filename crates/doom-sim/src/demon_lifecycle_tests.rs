//! Private fault injection and a separately injected original-C scalar component.
use crate::*;
fn sim() -> Simulation {
    let data = include_bytes!("../../../tests/reference/m3d3g_map_0.bin");
    let wad = doom_wad::Wad::parse(data, doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut s = Simulation::with_demon_lifecycle_registry(
        map,
        64,
        ActorRegistry::demon_lifecycle_fixture(),
        ActorOptions::default(),
    )
    .unwrap();
    s.spawn_type(
        ActorTypeId(136),
        [Fixed::from_map_units(-64), Fixed::ZERO],
        SpawnHeight::Floor,
    )
    .unwrap();
    s.spawn_type(
        ActorTypeId(135),
        [Fixed::from_map_units(96), Fixed::ZERO],
        SpawnHeight::Floor,
    )
    .unwrap();
    s
}
#[test]
fn original_c_injected_pain_boundary_component() {
    for row in include_str!("../../../tests/reference/m3d3g_pain_boundary.csv")
        .lines()
        .skip(1)
    {
        let v = row
            .split(',')
            .map(|v| v.parse::<i32>().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(crate::combat::pain_decision(v[0] as u8, 180), v[2] == 7);
        assert_eq!(v[1], 149);
        assert_eq!(v[4], 0);
    }
}
#[test]
fn bound_demon_and_hitscan_targets_reject_before_any_effect() {
    let mut s = sim();
    let owner = ActorId::from_parts(0, 1);
    s.bindings[0] = Some(owner);
    let before = s.snapshot();
    assert_eq!(
        s.damage_actor(DamageRequest {
            target: owner,
            source: None,
            inflictor: None,
            amount: 150
        }),
        Err(CombatError::Player.into())
    );
    assert_eq!(s.snapshot(), before);
    assert!(
        s.line_attack(
            ActorId::from_parts(1, 1),
            LineAttack {
                angle: 0x80000000,
                range: Fixed::from_map_units(512),
                slope: Fixed::ZERO,
                damage: 150
            }
        )
        .is_err()
    );
    assert_eq!(s.snapshot(), before);
}
#[test]
fn unsupported_later_death_action_rejects_before_health_rng_or_sound() {
    let mut s = sim();
    let r = s.registry.as_ref().unwrap();
    let mut states = r.states().to_vec();
    states[9].action = ActorAction::Original(lifecycle::source_action_id("A_VileChase"));
    s.registry = Some(std::sync::Arc::new(
        ActorRegistry::new(states, r.types().to_vec()).unwrap(),
    ));
    let before = s.snapshot();
    assert!(
        s.damage_actor(DamageRequest {
            target: ActorId::from_parts(0, 1),
            source: None,
            inflictor: None,
            amount: 150
        })
        .is_err()
    );
    assert_eq!(s.snapshot(), before);
}
#[test]
fn moving_strike_preflights_bound_target_before_facing() {
    let mut s = sim();
    let owner = ActorId::from_parts(0, 1);
    let target = ActorId::from_parts(1, 1);
    s.set_actor_target(owner, Some(target)).unwrap();
    s.bindings[0] = Some(target);
    let before = s.snapshot();
    assert!(s.demon_melee_attack(owner).is_err());
    assert_eq!(s.snapshot(), before);
}
