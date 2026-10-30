#![no_main]
#[path = "../../tests/support/m3g2.rs"]
mod inputs;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
const REQUEST_BYTES: usize = 8;
const MAX_INPUT: usize = 97;
fn apply(sim: &mut Simulation, bytes: &[u8], snapshot: bool) -> Result<(), SimError> {
    let player = sim.original_player(PlayerSlot::One).ok().and_then(|p| p.mo);
    let candidate = if bytes[1] & 1 == 0 {
        player.unwrap_or(ActorId::from_parts(
            u32::from(bytes[2]),
            u64::from(bytes[3]),
        ))
    } else {
        ActorId::from_parts(u32::from(bytes[2]), u64::from(bytes[3]))
    };
    match bytes[0] % 10 {
        0 => {
            let slot = PlayerSlot::try_from(bytes[4]).map_err(SimError::from)?;
            sim.spawn_player_body(
                slot,
                [
                    Fixed::from_map_units(i16::from(bytes[5] as i8) * 8),
                    Fixed::ZERO,
                ],
            )
            .map(|_| ())
        }
        1 => sim.bind_original_player_body(PlayerSlot::One, candidate),
        2 => sim
            .damage_actor(DamageRequest {
                target: candidate,
                source: match bytes[4] % 4 {
                    0 => None,
                    1 => player,
                    2 => Some(ActorId::from_parts(0, 1)),
                    _ => Some(ActorId::from_parts(
                        u32::from(bytes[6]),
                        u64::from(bytes[7]),
                    )),
                },
                inflictor: if bytes[5] & 1 == 0 { None } else { player },
                amount: i32::from(i16::from_le_bytes([bytes[6], bytes[7]])),
            })
            .map(|_| ()),
        3 => sim.mark_remove(candidate),
        4 => sim
            .spawn_type(
                ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
                [Fixed::from_map_units(-96), Fixed::ZERO],
                SpawnHeight::Floor,
            )
            .map(|_| ()),
        5 if bytes[4] & 1 != 0 => sim.face_target(candidate),
        5 => sim.enter_actor_state(candidate, StateId(u16::from_le_bytes([bytes[4], bytes[5]]))),
        6 => sim.set_momentum(
            candidate,
            [
                Fixed::from_map_units(i16::from(bytes[4] as i8)),
                Fixed::ZERO,
            ],
        ),
        7 => {
            let commands = [TicCommand {
                forward: if bytes[4] & 1 == 0 { 0 } else { 1 },
                ..Default::default()
            }; 4];
            if snapshot {
                sim.step(commands).map(|_| ())
            } else {
                sim.advance(commands)
            }
        }
        8 => {
            let mut game = GameInstance::new(sim.clone());
            let mut foreign = GameInstance::new(sim.clone());
            if let Some(id) = player {
                let reference = game.with_execution_context(|ctx| {
                    let actor = ctx.admit_local(
                        id,
                        DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
                    )?;
                    Ok(ctx.export_actor(actor))
                });
                if let Ok(Ok(reference)) = reference {
                    foreign
                        .with_execution_context(|ctx| {
                            assert!(
                                ctx.import_actor(
                                    &reference,
                                    DefinitionIdentity::Original(OriginalActorType::MT_PLAYER)
                                )
                                .is_err()
                            );
                            Ok(())
                        })
                        .unwrap();
                }
            }
            Ok(())
        }
        _ => sim
            .damage_actor(DamageRequest {
                target: candidate,
                source: None,
                inflictor: None,
                amount: i32::from(bytes[4]),
            })
            .map(|_| ()),
    }
}
fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let mut sim =
        Simulation::original_player_map(inputs::map(), 8, MapContextOptions::default()).unwrap();
    if first & 1 != 0 {
        sim.spawn_original_actor(OriginalActorType::MT_SERGEANT)
            .unwrap();
    }
    let mut captured = sim.clone();
    for request in rest[..rest.len().min(MAX_INPUT - 1)].chunks_exact(REQUEST_BYTES) {
        let before = sim.snapshot();
        let actual = apply(&mut sim, request, false);
        let expected = apply(&mut captured, request, true);
        assert_eq!(actual, expected);
        if actual.is_err() {
            assert_eq!(before, sim.snapshot());
        }
        assert_eq!(captured.snapshot(), sim.snapshot());
        let bytes = sim
            .snapshot()
            .diagnostic_bytes(DiagnosticLimits::default())
            .unwrap();
        let read = Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(read.encode(DiagnosticLimits::default()).unwrap(), bytes);
        assert!(Diagnostic::decode(&bytes, DiagnosticLimits::default()).is_err());
        if !bytes.is_empty() {
            let mut malformed = bytes;
            let at = usize::from(request[6]) % malformed.len();
            malformed[at] ^= request[7];
            let _ = Diagnostic::decode_player(&malformed, DiagnosticLimits::default());
        }
    }
});
