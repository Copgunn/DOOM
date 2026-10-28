//! Component C vectors and deliberately invalid private setups; no production override.
use super::*;
#[path = "../../../tests/support/m3d3f.rs"]
mod fixtures;
fn id(n: u32) -> ActorId {
    ActorId::from_parts(n, 1)
}
fn setup(label: &str) -> Simulation {
    fixtures::setup(
        &fixtures::cases()
            .into_iter()
            .find(|c| c.label == label)
            .unwrap(),
    )
}
#[test]
fn original_component_range_uses_metadata_not_current_collision_radius() {
    let mut count = 0;
    for line in include_str!("../../../tests/reference/m3d3f_melee_radius.csv")
        .lines()
        .skip(1)
    {
        let row = line
            .split(',')
            .map(|s| s.parse::<i32>().unwrap())
            .collect::<Vec<_>>();
        let mut c = fixtures::cases()
            .into_iter()
            .find(|c| c.label == "ordinary")
            .unwrap();
        c.set("target_radius", row[0]);
        let s = fixtures::setup(&c);
        let mut a = *s.actor(id(0)).unwrap();
        let mut b = *s.actor(id(1)).unwrap();
        a.spec.position = [Fixed::ZERO; 2];
        b.spec.position = [Fixed::from_bits(row[2]), Fixed::ZERO];
        b.spec.radius = Fixed::from_bits(row[1]);
        let result = crate::monsters::melee(
            &s.world,
            &s.sectors,
            s.registry.as_deref().unwrap(),
            a,
            Some(b),
        );
        assert_eq!(result.accepted, row[4] != 0, "original C component {line}");
        count += 1;
    }
    assert_eq!(count, 36);
}
#[test]
fn later_damage_rejection_precedes_real_shadow_facing_for_direct_call() {
    let mut s = setup("shadow");
    s.bindings[0] = Some(id(1)); // Test-only invalid setup; public binding is rejected.
    assert!(s.check_melee_range(id(0)).unwrap().accepted);
    let before = s.snapshot();
    assert_eq!(s.demon_melee_attack(id(0)), Err(CombatError::Player.into()));
    assert_eq!(s.snapshot(), before);
}
#[test]
fn later_damage_rejection_preserves_complete_expiring_tic() {
    let mut s = setup("shadow");
    s.enter_actor_state(id(0), StateId(5)).unwrap();
    for _ in 0..7 {
        s.step([TicCommand::default(); 4]).unwrap();
    }
    assert_eq!(s.actor(id(0)).unwrap().life.unwrap().tics, 1);
    s.bindings[0] = Some(id(1));
    assert!(s.check_melee_range(id(0)).unwrap().accepted);
    let before = s.snapshot();
    assert_eq!(
        s.step([TicCommand::default(); 4]),
        Err(CombatError::Player.into())
    );
    assert_eq!(s.snapshot(), before);
}
