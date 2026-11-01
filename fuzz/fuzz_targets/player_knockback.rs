#![no_main]
#[path = "../../tests/support/m3g3.rs"]
mod inputs;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
const REQUEST_BYTES: usize = 10;
const MAX_INPUT: usize = 129;
fn candidate(sim: &Simulation, kind: u8, slot: u8, generation: u8) -> Option<ActorId> {
    match kind % 4 {
        0 => None,
        1 => sim.original_player(PlayerSlot::One).ok().and_then(|p| p.mo),
        2 => Some(ActorId::from_parts(0, 1)),
        _ => Some(ActorId::from_parts(u32::from(slot), u64::from(generation))),
    }
}
fn apply(sim: &mut Simulation, b: &[u8], capture: bool) -> Result<(), SimError> {
    let id = candidate(sim, b[1] % 3 + 1, b[2], b[3])
        .unwrap_or(ActorId::from_parts(u32::from(b[2]), u64::from(b[3])));
    match b[0] % 12 {
        0 => sim
            .spawn_player_body(
                PlayerSlot::try_from(b[4]).map_err(SimError::from)?,
                [
                    Fixed::from_map_units(i16::from(b[5] as i8) * 8),
                    Fixed::from_map_units(i16::from(b[6] as i8) * 8),
                ],
            )
            .map(|_| ()),
        1 => sim.bind_original_player_body(PlayerSlot::One, id),
        2 => sim
            .damage_actor(DamageRequest {
                target: id,
                inflictor: candidate(sim, b[4], b[5], b[6]),
                source: candidate(sim, b[7], b[5], b[6]),
                amount: i32::from(i16::from_le_bytes([b[8], b[9]])),
            })
            .map(|_| ()),
        3 => sim.mark_remove(id),
        4 => sim
            .spawn_type(
                ActorTypeId(if b[4] & 1 == 0 {
                    OriginalActorType::MT_PUFF.ordinal()
                } else {
                    OriginalActorType::MT_PLAYER.ordinal()
                }),
                [Fixed::ZERO; 2],
                SpawnHeight::Floor,
            )
            .map(|_| ()),
        5 => sim.enter_actor_state(id, StateId(u16::from_le_bytes([b[4], b[5]]))),
        6 => sim.set_momentum(
            id,
            [
                Fixed::from_bits(i32::from_le_bytes([b[4], b[5], b[6], b[7]])),
                Fixed::ZERO,
            ],
        ),
        7 => {
            let commands = [TicCommand {
                forward: if b[4] & 1 == 0 { 0 } else { 1 },
                buttons: b[5] & 1,
                ..Default::default()
            }; 4];
            if capture {
                sim.step(commands).map(|_| ())
            } else {
                sim.advance(commands)
            }
        }
        8 => {
            let mut local = GameInstance::new(sim.clone());
            let mut foreign = GameInstance::new(sim.clone());
            if let Ok(reference) = local.with_execution_context(|ctx| {
                let actor = ctx.admit_local(
                    id,
                    DefinitionIdentity::Original(OriginalActorType::MT_PLAYER),
                )?;
                ctx.export_actor(actor)
            }) {
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
            Ok(())
        }
        9 => sim.set_vertical_momentum(
            id,
            Fixed::from_bits(i32::from_le_bytes([b[4], b[5], b[6], b[7]])),
        ),
        10 => sim
            .damage_actor(DamageRequest {
                target: id,
                source: None,
                inflictor: None,
                amount: i32::from(i16::from_le_bytes([b[8], b[9]])),
            })
            .map(|_| ()),
        _ => {
            sim.set_paused(b[4] & 1 != 0);
            Ok(())
        }
    }
}
fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let mut sim = if first & 128 == 0 {
        inputs::setup(&inputs::cases()[usize::from(first) % inputs::cases().len()]).0
    } else {
        let mut sim = Simulation::original_player_knockback_map(
            inputs::map(),
            8,
            MapContextOptions::default(),
        )
        .unwrap();
        if first & 64 != 0 {
            sim.spawn_original_actor(OriginalActorType::MT_SERGEANT)
                .unwrap();
        }
        sim
    };
    let mut captured = sim.clone();
    for b in rest[..rest.len().min(MAX_INPUT - 1)]
        .as_chunks::<REQUEST_BYTES>()
        .0
    {
        let before = sim.snapshot();
        let actual = apply(&mut sim, b, false);
        let expected = apply(&mut captured, b, true);
        assert_eq!(actual, expected);
        if actual.is_err() {
            assert_eq!(sim.snapshot(), before);
        }
        assert_eq!(sim.snapshot(), captured.snapshot());
        let bytes = sim
            .snapshot()
            .diagnostic_bytes(DiagnosticLimits::default())
            .unwrap();
        let read = Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(read.encode(DiagnosticLimits::default()).unwrap(), bytes);
        let mut changed = bytes;
        let at = usize::from(b[8]) % changed.len();
        changed[at] ^= b[9];
        let _ = Diagnostic::decode_player(&changed, DiagnosticLimits::default());
    }
});
