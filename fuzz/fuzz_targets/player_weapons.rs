//! Bounded command streams with whole-state recoverable-error checks.
#![no_main]
#[path = "../../tests/support/m3d2.rs"]
mod fixtures;
use doom_sim::*;
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let c = &fixtures::cases()[[0, 5, 24][usize::from(first % 3)]];
    let mut sim = fixtures::setup(c);
    for _ in 0..16 {
        sim.step([TicCommand::default(); 4]).unwrap();
    }
    for chunk in rest.chunks_exact(4).take(32) {
        let mut commands = [TicCommand::default(); 4];
        commands[usize::from(chunk[0] % 4)] = TicCommand {
            forward: (chunk[1] as i8) % 32,
            side: (chunk[2] as i8) % 32,
            angle_turn: 0,
            buttons: chunk[3],
        };
        let before = sim.snapshot();
        if sim.step(commands).is_err() {
            assert_eq!(sim.snapshot(), before);
        }
    }
});
