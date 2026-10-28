//! Bounded native state-graph fuzzing; no original or commercial game assets.
#![no_main]
#[path = "../../tests/support/m3c.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let original = fixtures::native_registry(6);
    let mut states = original.states().to_vec();
    for (i, chunk) in data.chunks(3).take(8).enumerate() {
        if chunk.len() != 3 {
            break;
        }
        let s = &mut states[i + 1];
        s.tics = i32::from(chunk[0] % 4) - 1;
        s.next = StateId(u16::from(chunk[1] % 9));
        s.action = match chunk[2] % 3 {
            0 => ActorAction::None,
            1 => ActorAction::Probe(ProbeAction::Random),
            _ => ActorAction::Probe(ProbeAction::Enter(StateId(u16::from(chunk[2] % 9)))),
        };
    }
    let registry = ActorRegistry::new(states, original.types().to_vec()).unwrap();
    let mut sim =
        Simulation::with_actor_registry(fixtures::map(&[]), 4, registry, ActorOptions::default())
            .unwrap();
    let before = sim.snapshot();
    match sim.spawn_type(ActorTypeId(131), [Fixed::ZERO; 2], SpawnHeight::Floor) {
        Ok(_) => {
            for _ in 0..8 {
                let before = sim.snapshot();
                if sim.step([TicCommand::default(); 4]).is_err() {
                    assert_eq!(sim.snapshot(), before);
                    break;
                }
            }
        }
        Err(_) => assert_eq!(sim.snapshot(), before),
    }
});
