//! Bounded attack sequences, extreme arithmetic and recoverable-error atomicity.
#![no_main]
#[path = "../../tests/support/m3d1.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let case = &fixtures::cases()[0];
    let mut sim = Simulation::with_combat_registry(
        fixtures::map(0),
        8,
        fixtures::registry(case),
        ActorOptions::default(),
    )
    .unwrap();
    for x in [-80, 64] {
        sim.spawn_type(
            ActorTypeId(if x < 0 { 0 } else { 136 }),
            [Fixed::from_map_units(x), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    }
    for chunk in data.as_chunks::<8>().0.iter().take(32) {
        let before = sim.snapshot();
        let raw = i32::from_le_bytes(chunk[..4].try_into().unwrap());
        let id = ActorId::from_parts(u32::from(chunk[4] % 10), u64::from(chunk[5] % 3));
        let result = match chunk[6] % 4 {
            0 => sim
                .damage_actor(DamageRequest {
                    target: id,
                    inflictor: None,
                    source: None,
                    amount: raw,
                })
                .map(|_| ()),
            1 => sim
                .line_attack(
                    id,
                    LineAttack {
                        angle: raw as u32,
                        range: Fixed::from_bits(raw),
                        slope: Fixed::from_bits(raw.rotate_left(7)),
                        damage: i32::from(chunk[7]),
                    },
                )
                .map(|_| ()),
            2 => sim
                .aim_line_attack(id, raw as u32, Fixed::from_bits(raw))
                .map(|_| ()),
            _ => sim.step([TicCommand::default(); 4]).map(|_| ()),
        };
        if result.is_err() {
            assert_eq!(sim.snapshot(), before);
        }
    }
});
