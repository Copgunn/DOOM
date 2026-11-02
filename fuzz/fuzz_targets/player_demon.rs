#![no_main]
#[path = "../../tests/support/m3g5.rs"]
mod inputs;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
const REQUEST_BYTES: usize = 10;
const MAX_INPUT: usize = 161;
fn apply(s: &mut Simulation, a: inputs::Actors, b: &[u8], capture: bool) -> Result<(), SimError> {
    let candidate = ActorId::from_parts(u32::from(b[7]), u64::from(b[8]));
    let demon = if b[9] & 1 == 0 {
        a.origin.unwrap_or(candidate)
    } else {
        candidate
    };
    match b[0] % 12 {
        0..=2 => {
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
        3 => s.set_initial_player_reaction_time(PlayerSlot::try_from(b[1])?, b[2]),
        4 => s
            .damage_actor(DamageRequest {
                target: a.body,
                source: if b[1] & 1 == 0 { None } else { Some(demon) },
                inflictor: if b[1] & 2 == 0 { None } else { Some(demon) },
                amount: i32::from(i16::from_le_bytes([b[4], b[5]])),
            })
            .map(|_| ()),
        5 => s.mark_remove(if b[1] & 1 == 0 { a.body } else { demon }),
        6 => {
            s.set_paused(b[1] & 1 != 0);
            Ok(())
        }
        7 => s.bind_original_player_body(PlayerSlot::try_from(b[1])?, candidate),
        8 => s.look_for_players(demon, b[1] & 1 != 0).map(|_| ()),
        9 => s.look(demon).map(|_| ()),
        10 => s.set_actor_target(
            demon,
            match b[1] % 3 {
                0 => None,
                1 => Some(a.body),
                _ => Some(candidate),
            },
        ),
        _ => s
            .spawn_type(
                ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
                [Fixed::ZERO; 2],
                SpawnHeight::Floor,
            )
            .map(|_| ()),
    }
}
fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let cases = inputs::cases();
    let (mut s, a) = inputs::setup(&cases[usize::from(first) % cases.len()]);
    let mut captured = s.clone();
    for b in rest[..rest.len().min(MAX_INPUT - 1)]
        .as_chunks::<REQUEST_BYTES>()
        .0
    {
        let before = s.snapshot();
        let roster = s.original_player_roster();
        let actual = apply(&mut s, a, b, false);
        let expected = apply(&mut captured, a, b, true);
        assert_eq!(actual, expected);
        if actual.is_err() {
            assert_eq!(s.snapshot(), before);
            assert_eq!(s.original_player_roster(), roster);
        }
        assert_eq!(s.snapshot(), captured.snapshot());
        let mut bytes = s
            .snapshot()
            .diagnostic_bytes(DiagnosticLimits::default())
            .unwrap();
        let view = Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
        let at = usize::from(u16::from_le_bytes([b[7], b[8]])) % bytes.len();
        bytes[at] ^= b[9];
        let _ = Diagnostic::decode_player(&bytes, DiagnosticLimits::default());
    }
});
