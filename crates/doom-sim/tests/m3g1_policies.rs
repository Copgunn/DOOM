//! Rust-only ownership/atomicity policies for Snapshot-free stepping.
#[path = "../../../tests/support/m3f9.rs"]
mod fixture;
use doom_sim::*;
use doom_types::Fixed;
use fixture::fixtures::{BODY, OWNER, id};
const COMMANDS: [TicCommand; 4] = [TicCommand {
    forward: 0,
    side: 0,
    angle_turn: 0,
    buttons: 0,
}; 4];
#[test]
fn bound_context_advance_preserves_incarnation_and_reference_guards() {
    let mut game = GameInstance::new(fixture::setup());
    let mut control = fixture::setup();
    let key = game.id();
    let owner = fixture::export(&mut game, id(0), OWNER);
    for _ in 0..12 {
        let expected = control.step(COMMANDS).unwrap();
        game.with_execution_context(|ctx| {
            let actor = ctx.import_actor(&owner, OWNER)?;
            ctx.advance(COMMANDS)?;
            ctx.actor(actor)?;
            Ok(())
        })
        .unwrap();
        assert_eq!(game.id(), key);
        assert_eq!(game.simulation().snapshot(), expected);
    }
    let before = game.simulation().snapshot();
    game.with_execution_context(|ctx| {
        let commands = [TicCommand {
            buttons: u8::MAX,
            ..Default::default()
        }; 4];
        assert_eq!(
            ctx.advance(commands),
            Err(ContextError::Runtime(SimError::UnsupportedButtons))
        );
        assert_eq!(ctx.snapshot(), before);
        Ok(())
    })
    .unwrap();
}
#[test]
fn retirement_and_slot_reuse_do_not_redirect_qualified_references() {
    let mut game = GameInstance::new(fixture::setup());
    let stale = fixture::export(&mut game, id(1), BODY);
    game.with_execution_context(|ctx| {
        let owner = ctx.admit_local(id(0), OWNER)?;
        let target = ctx.import_actor(&stale, BODY)?;
        ctx.set_target(owner, None)?;
        ctx.set_perception_player(0, None, false, 100)?;
        ctx.mark_remove(target)?;
        ctx.advance(COMMANDS)?;
        let replacement =
            ctx.spawn_body([Fixed::from_map_units(96), Fixed::ZERO], SpawnHeight::Floor)?;
        assert_eq!(replacement.raw_id(), ActorId::from_parts(1, 2));
        assert!(ctx.import_actor(&stale, BODY).is_err());
        ctx.set_perception_player(0, Some(replacement), true, 100)?;
        ctx.set_target(owner, Some(replacement))?;
        ctx.advance(COMMANDS)?;
        Ok(())
    })
    .unwrap();
    assert_eq!(
        game.simulation().actor_availability(id(1)),
        ActorAvailability::StaleGeneration
    );
    assert_eq!(
        game.simulation()
            .actor(id(0))
            .unwrap()
            .life()
            .unwrap()
            .target,
        Some(ActorId::from_parts(1, 2))
    );
}
#[test]
fn pending_target_rejection_keeps_rng_events_order_and_state_on_both_paths() {
    let case = &fixture::fixtures::cases()[4];
    let mut direct = fixture::fixtures::setup(4, case);
    let mut observed = direct.clone();
    let before = direct.snapshot();
    let error = direct.advance(COMMANDS).unwrap_err();
    assert_eq!(observed.step(COMMANDS).unwrap_err(), error);
    assert_eq!(direct.snapshot(), before);
    assert_eq!(observed.snapshot(), before);
}
