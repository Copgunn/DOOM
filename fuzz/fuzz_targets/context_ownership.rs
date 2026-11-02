#![no_main]
#[path = "../../tests/support/m3f8.rs"]
mod fixtures;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
fn operation(sim: &mut Simulation, bytes: &[u8]) -> Result<(), ContextError> {
    let slot = if bytes[0] & 128 == 0 {
        u32::from(bytes[1] % 5)
    } else {
        u32::from_le_bytes(bytes[..4].try_into().unwrap())
    };
    let generation = if bytes[0] & 64 == 0 {
        1 + u64::from(bytes[3] % 3)
    } else {
        u64::from_le_bytes(bytes[4..12].try_into().unwrap())
    };
    let candidate = ActorId::from_parts(slot, generation);
    let raw = u16::from_le_bytes([bytes[12], bytes[13]]);
    sim.with_execution_context(|ctx| {
        let operation = bytes[15] % 9;
        if operation == 3 {
            ctx.step([TicCommand::default(); 4])?;
            return Ok(());
        }
        if operation == 6 {
            ctx.spawn_body(
                [
                    Fixed::from_map_units(i16::from_le_bytes([bytes[8], bytes[9]])),
                    Fixed::from_map_units(i16::from_le_bytes([bytes[10], bytes[11]])),
                ],
                SpawnHeight::Floor,
            )?;
            return Ok(());
        }
        if operation == 8 {
            ctx.set_perception_player(usize::from(bytes[14] % 6), None, false, 100)?;
            return Ok(());
        }
        let definition = if bytes[0] & 32 != 0 {
            DefinitionIdentity::Original(
                OriginalActorType::try_from(raw)
                    .map_err(|_| ContextError::Identity(OriginalRegistryError::Definition(raw)))?,
            )
        } else if slot == 0 {
            fixtures::OWNER
        } else {
            fixtures::BODY
        };
        let actor = ctx.admit_local(candidate, definition)?;
        match operation {
            0 => {
                let body = ctx.admit_local(
                    ActorId::from_parts(u32::from(bytes[14] % 5), 1),
                    fixtures::BODY,
                )?;
                ctx.set_target(actor, Some(body))?;
            }
            1 => ctx.set_target(actor, None)?,
            2 => ctx.mark_remove(actor)?,
            4 => ctx.enter_state(
                actor,
                OriginalState::try_from(raw)
                    .map_err(|_| ContextError::Identity(OriginalRegistryError::State(raw)))?,
            )?,
            5 => {
                ctx.damage(
                    actor,
                    None,
                    None,
                    i32::from_le_bytes(bytes[12..16].try_into().unwrap()),
                )?;
            }
            _ => {
                let before = ctx.snapshot();
                let _ = ctx.target(actor)?;
                let _ = ctx.availability(actor);
                assert_eq!(ctx.snapshot(), before);
                let bytes = before
                    .diagnostic_bytes(DiagnosticLimits::default())
                    .unwrap();
                let view = Diagnostic::decode(&bytes, DiagnosticLimits::default()).unwrap();
                assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
            }
        }
        Ok(())
    })
}
fuzz_target!(|data: &[u8]| {
    if data.len() > 128 {
        return;
    }
    let c = &fixtures::cases()[0];
    let (mut a, mut b) = (fixtures::setup(0, c), fixtures::setup(0, c));
    for bytes in data.as_chunks::<16>().0.iter().take(8) {
        let before = a.snapshot();
        let ra = operation(&mut a, bytes);
        let rb = operation(&mut b, bytes);
        assert_eq!(ra, rb);
        assert_eq!(a.snapshot(), b.snapshot());
        if ra.is_err() {
            assert_eq!(a.snapshot(), before);
        }
    }
});
