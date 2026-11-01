#![no_main]
#[path = "../../tests/support/m3g1_profiles.rs"]
mod inputs;
use doom_sim::*;
use libfuzzer_sys::fuzz_target;
const MAX_INPUT: usize = 97;
const REQUEST_BYTES: usize = 8;
fuzz_target!(|data: &[u8]| {
    let Some((&first, remaining)) = data.split_first() else {
        return;
    };
    let Some(profile) = inputs::PROFILES
        .get(usize::from(first) % inputs::PROFILES.len())
        .copied()
    else {
        return;
    };
    let mut observed = inputs::setup(profile);
    let mut direct = observed.clone();
    for chunk in remaining[..remaining.len().min(MAX_INPUT - 1)]
        .as_chunks::<REQUEST_BYTES>()
        .0
    {
        let mut command = [TicCommand::default(); 4];
        let slot = usize::from(chunk[0] % 4);
        if let Some(value) = command.get_mut(slot) {
            *value = TicCommand {
                forward: chunk[1] as i8,
                side: chunk[2] as i8,
                angle_turn: i16::from_le_bytes([chunk[3], chunk[4]]),
                buttons: chunk[5],
            };
        }
        let paused = chunk[6] & 1 != 0;
        observed.set_paused(paused);
        direct.set_paused(paused);
        if chunk[6] & 2 != 0 {
            assert_eq!(observed.gameplay_random(), direct.gameplay_random());
        }
        if chunk[6] & 4 != 0 {
            let candidate = if chunk[7] & 1 == 0 {
                observed.actor_order().next()
            } else {
                Some(ActorId::from_parts(u32::MAX, u64::from(chunk[7])))
            };
            if let Some(actor) = candidate {
                assert_eq!(observed.mark_remove(actor), direct.mark_remove(actor));
            }
        }
        let before = direct.snapshot();
        let expected = observed.step(command).map(drop);
        let actual = direct.advance(command);
        assert_eq!(expected, actual);
        if actual.is_err() {
            assert_eq!(direct.snapshot(), before);
        }
        let a = observed.snapshot();
        let b = direct.snapshot();
        assert_eq!(a, b);
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    }
});
