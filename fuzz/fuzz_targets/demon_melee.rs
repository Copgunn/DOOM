//! Bounded attacks, states, ownership, references and complete failed-state checks.
#![no_main]
#[path = "../../tests/support/m3d3f.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    static CASES: std::sync::OnceLock<Vec<fixtures::Case>> = std::sync::OnceLock::new();
    let cases = CASES.get_or_init(fixtures::cases);
    let labels = [
        "ordinary",
        "shadow",
        "null",
        "outside",
        "thinker_cycles",
        "wall",
    ];
    let c = cases
        .iter()
        .find(|c| c.label == labels[usize::from((first & 0x7f) % 6)])
        .unwrap();
    // Mutate only public synthetic map bytes. Constructor rejection is separate
    // from authoritative operation atomicity; no production geometry override.
    let mut map_bytes = fixtures::map_bytes(usize::from((first & 0x7f) % 20)).to_vec();
    for (i, byte) in rest.iter().take(8).enumerate() {
        let offset = (usize::from(*byte) + i * 17) % map_bytes.len();
        map_bytes[offset] ^= *byte;
    }
    if let Ok(wad) = doom_wad::Wad::parse(&map_bytes, doom_wad::Limits::default()) {
        if let Ok(map) = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()) {
            let _ = Simulation::with_demon_melee_registry(
                map,
                64,
                fixtures::registry(c),
                ActorOptions::default(),
            );
        }
    }
    let mut s = fixtures::setup(c);
    // High selector bit adds a bounded capacity setup. Mutational operations
    // can now exercise zero-demand melee and rejected spawns at all 64 slots.
    if first & 0x80 != 0 {
        while s.actor_order().count() < 64 {
            s.spawn_type(
                ActorTypeId(135),
                [Fixed::from_map_units(-200), Fixed::from_map_units(-96)],
                SpawnHeight::Floor,
            )
            .unwrap();
        }
    }
    let initial = PursuitSetup::from_actor(s.actor(ActorId::from_parts(0, 1)).unwrap()).unwrap();
    for c in rest.chunks_exact(8).take(8) {
        let a = ActorId::from_parts(u32::from(c[1] % 8), u64::from(c[2] % 3));
        let b = ActorId::from_parts(u32::from(c[7] % 8), 1);
        let value = i32::from_le_bytes(c[3..7].try_into().unwrap());
        let before = s.snapshot();
        let result = match c[0] % 16 {
            0 => s.demon_melee_attack(a),
            1 => s.enter_actor_state(a, StateId(6)),
            2 => s.check_missile_range(a).map(|_| ()),
            3 => s.check_melee_range(a).map(|_| ()),
            4 => s.chase(a),
            5 => s.enter_actor_state(
                a,
                StateId([0, 1, 2, 3, 4, 5, 6, 7, 149, 255][usize::from(c[7] % 10)]),
            ),
            6 => s.mark_remove(a),
            7 => s.set_actor_target(a, if c[6] & 1 != 0 { None } else { Some(b) }),
            8 => {
                let mut p = s
                    .actor(a)
                    .and_then(PursuitSetup::from_actor)
                    .unwrap_or(initial);
                match c[7] % 7 {
                    0 => p.movedir = c[3],
                    1 => p.movecount = value,
                    2 => p.reaction_time = value,
                    3 => p.threshold = value,
                    4 => p.health = value,
                    5 => p.flags = value as u32,
                    _ => p.angle = value as u32,
                };
                s.configure_pursuit_actor(a, p)
            }
            9 => s.set_perception_player(
                usize::from(c[7] % 6),
                PerceptionPlayer {
                    active: c[3] & 1 != 0,
                    health: value,
                    actor: if c[4] & 1 != 0 { None } else { Some(a) },
                },
            ),
            10 => s
                .spawn_type(
                    ActorTypeId([0, 135, 136, 37, 38][usize::from(c[7] % 5)]),
                    [Fixed::from_bits(value), Fixed::ZERO],
                    SpawnHeight::Floor,
                )
                .map(|_| ()),
            11 => s.set_momentum(a, [Fixed::from_bits(value), Fixed::ZERO]),
            12 => s.set_vertical_momentum(a, Fixed::from_bits(value)),
            13 => {
                // Each individual tic has its own atomicity contract. At most
                // 24 tics across the eight operations reaches delayed strikes.
                for _ in 0..=c[7] % 3 {
                    let before_tic = s.snapshot();
                    if s.step([TicCommand::default(); 4]).is_err() {
                        assert_eq!(before_tic, s.snapshot());
                    }
                }
                Ok(())
            }
            14 => s.noise_alert(a, b),
            _ => s.face_target(a),
        };
        if result.is_err() {
            assert_eq!(before, s.snapshot());
        }
    }
});
