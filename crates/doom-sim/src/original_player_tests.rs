//! White-box atomicity and structural stepping checks; Rust policy only.
use super::*;
#[path = "../../../tests/support/m3g2.rs"]
mod inputs;
#[test]
fn player_fields_rollback_with_actor_rng_statistics_and_events_on_late_failure() {
    let mut sim = inputs::setup(2, &inputs::cases()[2]);
    let plan = sim
        .plan_damage(DamageRequest {
            target: inputs::id(1),
            inflictor: None,
            source: None,
            amount: 100,
        })
        .unwrap();
    let before = sim.snapshot();
    let p = sim.original_player;
    let result = sim.checked_demon_operation(|host| {
        host.apply_damage(plan);
        host.fail_sight(PerceptionError::WorkLimit.into());
    });
    assert_eq!(result, Err(PerceptionError::WorkLimit.into()));
    assert_eq!(sim.snapshot(), before);
    assert_eq!(sim.original_player, p);
}
#[test]
fn source_frags_overflow_rejects_before_mutation_but_noncredited_death_is_valid() {
    let mut sim =
        Simulation::original_player_map(inputs::map(), 64, MapContextOptions::default()).unwrap();
    sim.spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap();
    let body = sim
        .spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap();
    sim.original_player.as_mut().unwrap().frags = i32::MAX;
    for source in [None, Some(body)] {
        let before = sim.snapshot();
        assert_eq!(
            sim.damage_actor(DamageRequest {
                target: body,
                inflictor: None,
                source,
                amount: 100
            }),
            Err(PlayerBodyError::Statistics.into())
        );
        assert_eq!(sim.snapshot(), before);
    }
    sim.damage_actor(DamageRequest {
        target: body,
        inflictor: None,
        source: Some(inputs::id(0)),
        amount: 100,
    })
    .unwrap();
    assert_eq!(sim.original_player.unwrap().frags, i32::MAX);
}
#[test]
fn genuine_player_advance_has_no_implicit_snapshot_and_rejects_without_capture() {
    let mut sim = inputs::setup(6, &inputs::cases()[6]);
    let count = crate::snapshot::capture_count();
    inputs::operations(&mut sim, 6, 1);
    sim.advance(inputs::COMMANDS).unwrap();
    assert_eq!(crate::snapshot::capture_count(), count);
    sim.step(inputs::COMMANDS).unwrap();
    assert_eq!(crate::snapshot::capture_count(), count + 1);
    assert!(
        sim.advance(
            [TicCommand {
                forward: 1,
                ..Default::default()
            }; 4]
        )
        .is_err()
    );
    assert_eq!(crate::snapshot::capture_count(), count + 1);
    sim.set_paused(true);
    sim.advance(inputs::COMMANDS).unwrap();
    assert_eq!(crate::snapshot::capture_count(), count + 1);
}
