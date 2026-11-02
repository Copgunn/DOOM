//! Bounded perception/roster/state sequences and complete error atomicity.
#![no_main]
#[path = "../../tests/support/m3d3a.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let c = &fixtures::cases()
        [[37, 38, 39, 40, 51, 52, 53, 54, 55, 56, 57, 58][usize::from(first % 12)]];
    let mut sim = fixtures::setup(c);
    for chunk in rest.as_chunks::<4>().0.iter().take(12) {
        let a = ActorId::from_parts(u32::from(chunk[1] % 8), u64::from(chunk[2] % 3));
        let b = ActorId::from_parts(u32::from(chunk[3] % 8), 1);
        let before = sim.snapshot();
        let result = match chunk[0] % 12 {
            0 => sim.check_sight(a, b).map(|_| ()),
            1 => sim.look_for_players(a, chunk[3] & 1 != 0).map(|_| ()),
            2 => sim.look(a).map(|_| ()),
            3 => sim.noise_alert(a, b),
            4 => sim.set_perception_player(
                usize::from(chunk[1] % 6),
                PerceptionPlayer {
                    active: chunk[2] & 1 != 0,
                    health: i32::from(chunk[3] as i8),
                    actor: if chunk[2] & 4 != 0 { None } else { Some(a) },
                },
            ),
            5 => sim.mark_remove(a),
            6 => sim.step([TicCommand::default(); 4]).map(|_| ()),
            7 => sim.enter_actor_state(a, StateId(u16::from(chunk[3]))),
            8 => {
                sim.configure_perception_observer(a, u32::from(chunk[3]) << 24, i32::from(chunk[2]))
            }
            9 => {
                sim.set_paused(chunk[3] & 1 != 0);
                Ok(())
            }
            10 => sim.set_momentum(
                a,
                [
                    Fixed::from_bits(i32::from(chunk[3] as i8) * 64 * 65536),
                    Fixed::ZERO,
                ],
            ),
            _ => sim
                .spawn_type(
                    ActorTypeId(if chunk[3] & 1 == 0 { 0 } else { 136 }),
                    [Fixed::ZERO; 2],
                    SpawnHeight::Floor,
                )
                .map(|_| ()),
        };
        if result.is_err() {
            assert_eq!(sim.snapshot(), before);
        }
    }
});
