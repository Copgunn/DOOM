#![no_main]
#[path = "../../tests/support/m3g4.rs"]
mod inputs;
use doom_sim::*;
use libfuzzer_sys::fuzz_target;
const REQUEST_BYTES: usize = 8;
const MAX_INPUT: usize = 129;
fn apply(s: &mut Simulation, b: &[u8], capture: bool) -> Result<(), SimError> {
    let id = s
        .original_player(PlayerSlot::One)?
        .mo
        .unwrap_or(ActorId::from_parts(u32::MAX, 0));
    match b[0] % 8 {
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
                target: id,
                inflictor: None,
                source: if b[1] & 1 == 0 {
                    None
                } else {
                    Some(ActorId::from_parts(u32::from(b[2]), u64::from(b[3])))
                },
                amount: i32::from(i16::from_le_bytes([b[4], b[5]])),
            })
            .map(|_| ()),
        5 => s.mark_remove(id),
        6 => {
            s.set_paused(b[1] & 1 != 0);
            Ok(())
        }
        _ => s.bind_original_player_body(
            PlayerSlot::One,
            ActorId::from_parts(u32::from(b[1]), u64::from(b[2])),
        ),
    }
}
fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let cases = inputs::cases();
    let (mut s, _) = inputs::setup(&cases[usize::from(first) % cases.len()]);
    let mut captured = s.clone();
    for b in rest[..rest.len().min(MAX_INPUT - 1)]
        .as_chunks::<REQUEST_BYTES>()
        .0
    {
        let before = s.snapshot();
        let a = apply(&mut s, b, false);
        let expected = apply(&mut captured, b, true);
        assert_eq!(a, expected);
        if a.is_err() {
            assert_eq!(s.snapshot(), before);
        }
        assert_eq!(s.snapshot(), captured.snapshot());
        let bytes = s
            .snapshot()
            .diagnostic_bytes(DiagnosticLimits::default())
            .unwrap();
        let decoded = Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(decoded.encode(DiagnosticLimits::default()).unwrap(), bytes);
        let mut changed = bytes;
        let at = usize::from(b[6]) % changed.len();
        changed[at] ^= b[7];
        let _ = Diagnostic::decode_player(&changed, DiagnosticLimits::default());
    }
});
