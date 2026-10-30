//! Reuse one frozen C input through qualified references; no new source adapter.
#![allow(dead_code)]
#[path = "m3f8.rs"]
pub mod fixtures;
use doom_sim::*;
use fixtures::{BODY, OWNER, id};

pub fn setup() -> Simulation {
    fixtures::setup(0, &fixtures::cases()[0])
}
pub fn export(
    game: &mut GameInstance,
    local: ActorId,
    definition: DefinitionIdentity,
) -> InstanceActorRef {
    game.with_execution_context(|ctx| ctx.export_actor(ctx.admit_local(local, definition)?))
        .unwrap()
}
pub fn import(
    game: &mut GameInstance,
    reference: &InstanceActorRef,
    definition: DefinitionIdentity,
) -> Result<(), ProvenanceError> {
    game.with_execution_context(|ctx| {
        ctx.import_actor(reference, definition)?;
        Ok(())
    })
}

/// Existing replacement/clear case, including all seven C-derived captures.
/// Counts are reused comparisons, not additional original-C observations.
pub fn source_case() -> ([usize; 5], Snapshot) {
    let case = &fixtures::cases()[0];
    let mut game = GameInstance::new(setup());
    let owner = export(&mut game, id(0), OWNER);
    let replacement = export(&mut game, id(2), BODY);
    let mut source = fixtures::SourceTrace::default();
    source.check(game.simulation(), 0, 0, game.simulation().pending_events());
    let mut events = fixtures::assert_golden(game.simulation(), 0, 0);
    let tics = usize::try_from(case.v("tics")).unwrap();
    for op in 1..=tics {
        let pending = game.simulation().pending_events().len();
        game.with_execution_context(|ctx| {
            let actor = ctx.import_actor(&owner, OWNER)?;
            match op {
                1 => ctx.set_target(actor, Some(ctx.import_actor(&replacement, BODY)?))?,
                2 => ctx.set_target(actor, None)?,
                _ => {}
            }
            ctx.step([TicCommand::default(); 4])?;
            Ok(())
        })
        .unwrap();
        source.check(
            game.simulation(),
            0,
            op,
            &game.simulation().events()[pending..],
        );
        events += fixtures::assert_golden(game.simulation(), 0, op);
    }
    (
        [
            source.actor_fields,
            source.draws,
            source.statistics_fields,
            events,
            tics + 1,
        ],
        game.simulation().snapshot(),
    )
}
