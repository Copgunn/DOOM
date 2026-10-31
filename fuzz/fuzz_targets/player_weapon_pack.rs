#![no_main]
#[path = "../../tests/support/m3g8.rs"]
mod inputs;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;

const REQUEST_BYTES: usize = 10;
const MAX_INPUT: usize = 162;
// Declared C timelines seed selection, hits, misses, death and terminal corpses.
const WARMUPS: [usize; 20] = [
    0, 14, 18, 19, 24, 28, 33, 39, 64, 128, 160, 214, 215, 256, 287, 383, 400, 494, 495, 511,
];
fn apply(s: &mut Simulation, a: inputs::Actors, b: &[u8], capture: bool) -> Result<(), SimError> {
    let candidate = ActorId::from_parts(u32::from(b[7]), u64::from(b[8]));
    let demon = if b[9] & 1 == 0 {
        a.origin.unwrap_or(candidate)
    } else {
        candidate
    };
    match b[0] % 16 {
        0..=5 => {
            let mut c = [TicCommand::default(); 4];
            c[usize::from(b[1] % 4)] = TicCommand {
                forward: i8::from_le_bytes([b[2]]),
                side: i8::from_le_bytes([b[3]]),
                angle_turn: i16::from_le_bytes([b[4], b[5]]),
                buttons: b[6],
            };
            if capture {
                s.step(c).map(|_| ())
            } else {
                s.advance(c)
            }
        }
        6 => s
            .damage_actor(DamageRequest {
                target: if b[3] & 1 == 0 { a.body } else { demon },
                source: match b[1] % 3 {
                    0 => None,
                    1 => Some(demon),
                    _ => Some(a.body),
                },
                inflictor: match b[2] % 3 {
                    0 => None,
                    1 => Some(demon),
                    _ => Some(a.body),
                },
                amount: i32::from(i16::from_le_bytes([b[4], b[5]])),
            })
            .map(|_| ()),
        7 => s.mark_remove(if b[1] & 1 == 0 { a.body } else { demon }),
        8 => s.set_actor_target(
            demon,
            match b[1] % 3 {
                0 => None,
                1 => Some(a.body),
                _ => Some(candidate),
            },
        ),
        9 => s.look(demon).map(|_| ()),
        10 => s.bind_original_player_body(PlayerSlot::try_from(b[1])?, candidate),
        11 => {
            s.set_paused(b[1] & 1 != 0);
            Ok(())
        }
        12 => s.set_initial_player_reaction_time(PlayerSlot::try_from(b[1])?, b[2]),
        13 => s
            .spawn_type(
                ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
                [Fixed::ZERO; 2],
                SpawnHeight::Floor,
            )
            .map(|_| ()),
        14 => s
            .enter_original_state(
                demon,
                OriginalState::try_from(u16::from_le_bytes([b[4], b[5]]))
                    .map_err(|_| MonsterError::Definition)?,
            )
            .map_err(|error| match error {
                OriginalRegistryError::Runtime(error) => error,
                _ => MonsterError::Definition.into(),
            }),
        _ => s.check_melee_range(demon).map(|_| ()),
    }
}
fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let Some((&warm, rest)) = rest.split_first() else {
        return;
    };
    let cases = inputs::cases();
    let c = &cases[usize::from(first) % cases.len()];
    let (mut s, a) = inputs::setup(c);
    for op in 1..=WARMUPS[usize::from(warm) % WARMUPS.len()].min(c.tics) {
        if s.advance(inputs::commands(c, op)).is_err() {
            break;
        }
    }
    let mut captured = s.clone();
    for b in rest[..rest.len().min(MAX_INPUT - 2)].chunks_exact(REQUEST_BYTES) {
        let before = s.snapshot();
        let roster = s.original_player_roster();
        let actual = apply(&mut s, a, b, false);
        let expected = apply(&mut captured, a, b, true);
        assert_eq!(actual, expected);
        if actual.is_err() {
            assert_eq!(s.snapshot(), before);
            assert_eq!(s.original_player_roster(), roster);
        }
        let snapshot = s.snapshot();
        assert_eq!(snapshot, captured.snapshot());
        let mut bytes = snapshot
            .diagnostic_bytes(DiagnosticLimits::default())
            .unwrap();
        let view = Diagnostic::decode_weapons(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
        let at = usize::from(u16::from_le_bytes([b[7], b[8]])) % bytes.len();
        bytes[at] ^= b[9];
        let _ = Diagnostic::decode_weapons(&bytes, DiagnosticLimits::default());
    }
});
