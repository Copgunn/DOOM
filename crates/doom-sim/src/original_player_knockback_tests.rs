//! White-box rollback, representation and capture contracts, not C evidence.
use super::*;
#[path = "../../../tests/support/m3g3.rs"]
mod inputs;
#[test]
fn physical_momentum_player_fields_rng_and_events_rollback_on_late_failure() {
    let c = &inputs::cases()[1];
    let (mut sim, a) = inputs::setup(c);
    let plan = sim
        .plan_damage(DamageRequest {
            target: a.body,
            inflictor: Some(a.origin),
            source: None,
            amount: 100,
        })
        .unwrap();
    let before = sim.snapshot();
    let player = sim.original_player;
    let result = sim.checked_demon_operation(|host| {
        host.apply_damage(plan);
        assert_ne!(
            host.access.get(a.body).unwrap().momentum(),
            [Fixed::ZERO; 2]
        );
        host.fail_sight(PerceptionError::WorkLimit.into());
    });
    assert_eq!(result, Err(PerceptionError::WorkLimit.into()));
    assert_eq!(sim.snapshot(), before);
    assert_eq!(sim.original_player, player);
}
#[test]
fn corrupted_physics_and_accumulation_limits_reject_without_panics_or_partial_effects() {
    let c = &inputs::cases()[1];
    let (mut sim, a) = inputs::setup(c);
    for value in [i32::MIN, i32::MAX, 30 * 65536 + 1] {
        sim.actors.get_mut(a.body).unwrap().spec.momentum[0] = Fixed::from_bits(value);
        let before = sim.snapshot();
        assert!(
            sim.damage_actor(DamageRequest {
                target: a.body,
                inflictor: Some(a.origin),
                source: None,
                amount: 20
            })
            .is_err()
        );
        assert_eq!(sim.snapshot(), before);
    }
    sim.actors.get_mut(a.body).unwrap().spec.momentum = [Fixed::ZERO; 2];
    let flags = sim.actors.get(a.body).unwrap().life.unwrap().flags;
    for unsupported in [MF_NOCLIP, MF_MISSILE, MF_SKULLFLY, MF_FLOAT, MF_NOGRAVITY] {
        sim.actors
            .get_mut(a.body)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .flags = flags | unsupported;
        let before = sim.snapshot();
        assert!(sim.advance(inputs::COMMANDS).is_err());
        assert_eq!(sim.snapshot(), before);
    }
}
#[test]
fn moving_player_advance_never_captures_and_pause_error_precedence_stays_intact() {
    let c = &inputs::cases()[1];
    let (mut sim, a) = inputs::setup(c);
    inputs::hit(&mut sim, a, c, 20).unwrap();
    let count = crate::snapshot::capture_count();
    sim.advance(inputs::COMMANDS).unwrap();
    assert_eq!(crate::snapshot::capture_count(), count);
    sim.step(inputs::COMMANDS).unwrap();
    assert_eq!(crate::snapshot::capture_count(), count + 1);
    let invalid = [TicCommand {
        forward: 1,
        ..Default::default()
    }; 4];
    assert!(sim.advance(invalid).is_err());
    assert_eq!(crate::snapshot::capture_count(), count + 1);
    sim.set_paused(true);
    sim.advance(invalid).unwrap();
    assert_eq!(crate::snapshot::capture_count(), count + 1);
}
