//! Bounded projectile API/step sequences and complete recoverable-error atomicity.
#![no_main]
#[path = "../../tests/support/m3d2b.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let c = &fixtures::cases()[[0, 3, 5, 10, 11, 23, 24, 26, 33][usize::from(first % 9)]];
    let mut sim = fixtures::setup(c);
    for chunk in rest.as_chunks::<3>().0.iter().take(12) {
        let id = ActorId::from_parts(u32::from(chunk[1] % 6), u64::from(chunk[2] % 3));
        let before = sim.snapshot();
        let result = match chunk[0] % 8 {
            0 => sim
                .launch_projectile(ProjectileRequest {
                    source: id,
                    ..fixtures::request(c)
                })
                .map(|_| ()),
            1 => sim
                .launch_projectile(ProjectileRequest {
                    aim: ProjectileAim::Actor(id),
                    ..fixtures::request(c)
                })
                .map(|_| ()),
            2 => sim.mark_remove(id),
            3 => sim.set_momentum(
                id,
                [
                    Fixed::from_bits(i32::from(chunk[2] as i8) * 1048576),
                    Fixed::ZERO,
                ],
            ),
            4 => sim.set_vertical_momentum(id, Fixed::from_bits(i32::from(chunk[2] as i8) * 65536)),
            5 => sim.bind_player(0, id),
            6 => sim
                .spawn_type(
                    ActorTypeId(u16::from(chunk[1])),
                    [Fixed::ZERO; 2],
                    SpawnHeight::Floor,
                )
                .map(|_| ()),
            _ => sim.step([TicCommand::default(); 4]).map(|_| ()),
        };
        if result.is_err() {
            assert_eq!(sim.snapshot(), before);
        }
    }
});
