#![no_main]
#[path = "../../tests/support/m3f9.rs"]
mod fixture;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
use fixture::fixtures::{BODY, OWNER, id};
use libfuzzer_sys::fuzz_target;

const MAX_INPUT: usize = 128;
const REQUEST_BYTES: usize = 16;

fn operation(
    game: &mut GameInstance,
    initial: &InstanceActorRef,
    foreign: &InstanceId,
    b: &[u8; REQUEST_BYTES],
) -> Result<(), ProvenanceError> {
    let op = b[15] % 10;
    if op == 4 {
        if b[0] & 16 == 0 {
            game.restart().map_err(ContextError::from)?;
        } else {
            *game = GameInstance::new(game.simulation().clone());
        }
        return Ok(());
    }
    let slot = if b[0] & 128 == 0 {
        u32::from(b[1] % 8)
    } else {
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    };
    let generation = if b[0] & 64 == 0 {
        1 + u64::from(b[3] % 3)
    } else {
        u64::from_le_bytes([b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11]])
    };
    let provenance = match b[14] % 4 {
        0 => Some(game.id()),
        1 => Some(foreign.clone()),
        2 => None,
        _ => initial.provenance().cloned(),
    };
    let candidate = InstanceActorRef::candidate(provenance, ActorId::from_parts(slot, generation));
    game.with_execution_context(|ctx| {
        if op == 3 {
            ctx.step([TicCommand::default(); 4])?;
            return Ok(());
        }
        if op == 5 {
            ctx.spawn_body(
                [
                    Fixed::from_map_units(i16::from_le_bytes([b[8], b[9]])),
                    Fixed::from_map_units(i16::from_le_bytes([b[10], b[11]])),
                ],
                SpawnHeight::Floor,
            )?;
            return Ok(());
        }
        if op == 9 {
            ctx.set_perception_player(usize::from(b[14] % 6), None, false, 100)?;
            return Ok(());
        }
        let raw = u16::from_le_bytes([b[12], b[13]]);
        let definition = if b[0] & 32 != 0 {
            DefinitionIdentity::Original(
                OriginalActorType::try_from(raw)
                    .map_err(|_| ContextError::Identity(OriginalRegistryError::Definition(raw)))?,
            )
        } else if slot == 0 {
            OWNER
        } else {
            BODY
        };
        if op == 6 {
            // Deliberate local readmission is separate from provenance import.
            let local = ctx.admit_local(candidate.local_id(), definition)?;
            let exported = ctx.export_actor(local)?;
            assert_eq!(
                ctx.import_actor(&exported, definition)?.raw_id(),
                local.raw_id()
            );
            return Ok(());
        }
        let actor = ctx.import_actor(&candidate, definition)?;
        match op {
            0 => {
                let _ = ctx.actor(actor)?;
                let _ = ctx.target(actor)?;
            }
            1 => ctx.mark_remove(actor)?,
            2 => ctx.set_target(actor, None)?,
            7 => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                ctx.set_target(owner, Some(actor))?;
            }
            _ => {
                ctx.damage(
                    actor,
                    None,
                    None,
                    i32::from_le_bytes([b[8], b[9], b[10], b[11]]),
                )?;
            }
        }
        Ok(())
    })
}

fuzz_target!(|data: &[u8]| {
    if data.len() < REQUEST_BYTES || data.len() > MAX_INPUT {
        return;
    }
    let (mut a, mut b) = (
        GameInstance::new(fixture::setup()),
        GameInstance::new(fixture::setup()),
    );
    let (initial_a, initial_b) = (
        fixture::export(&mut a, id(0), OWNER),
        fixture::export(&mut b, id(0), OWNER),
    );
    // The key retains no world; even a dropped foreign owner must not alias.
    let foreign = GameInstance::new(fixture::setup()).id();
    for chunk in data.chunks_exact(REQUEST_BYTES) {
        let Ok(request) = <&[u8; REQUEST_BYTES]>::try_from(chunk) else {
            return;
        };
        let before_a = (a.simulation().snapshot(), a.id());
        let before_b = (b.simulation().snapshot(), b.id());
        let ra = operation(&mut a, &initial_a, &foreign, request);
        let rb = operation(&mut b, &initial_b, &foreign, request);
        assert_eq!(ra, rb);
        if ra.is_err() {
            assert_eq!(a.simulation().snapshot(), before_a.0);
            assert_eq!(a.id(), before_a.1);
            assert_eq!(b.simulation().snapshot(), before_b.0);
            assert_eq!(b.id(), before_b.1);
        }
        assert_eq!(a.simulation().snapshot(), b.simulation().snapshot());
        assert_eq!(a.simulation().rng(), b.simulation().rng());
        assert_ne!(a.id(), b.id());
    }
});
