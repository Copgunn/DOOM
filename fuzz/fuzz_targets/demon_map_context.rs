//! Bounded original Demon THING batches and ordered gameplay, perception, melee/lifecycle and atomicity.
#![no_main]
#[path = "../../tests/support/m3d3j.rs"]
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
        "one_idle",
        "multiple_4",
        "shared_target_death",
        "one_wake",
        "multi_rng_0",
        "spawn_lifecycle",
        "corpse_source_reuse",
        "nonzero_floor",
    ];
    let c = cases
        .iter()
        .find(|c| c.label == labels[usize::from((first & 0x7f) % 8)])
        .unwrap();
    // Mutate only public synthetic map bytes. Constructor rejection is separate
    // from authoritative operation atomicity; no production geometry override.
    let mut map_bytes = fixtures::map_bytes(usize::from((first & 0x7f) % 54)).to_vec();
    for (i, byte) in rest.iter().take(8).enumerate() {
        let offset = (usize::from(*byte) + i * 17) % map_bytes.len();
        map_bytes[offset] ^= *byte;
    }
    if let Ok(wad) = doom_wad::Wad::parse(&map_bytes, doom_wad::Limits::default()) {
        if let Ok(map) = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()) {
            if let Ok(mut candidate) = Simulation::with_demon_map_context_registry(
                map,
                64,
                fixtures::registry(c),
                ActorOptions::default(),
                MapContextOptions::default(),
            ) {
                let before = candidate.snapshot();
                if candidate.spawn_map_things().is_err() {
                    assert_eq!(candidate.snapshot(), before);
                }
            }
        }
    }
    let mut s = fixtures::setup(c);
    // High selector bit adds a bounded capacity setup. Mutational operations
    // can now exercise zero-demand damage/melee and rejected spawns at all 64 slots.
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
        let result = match c[0] % 22 {
            0 => s
                .damage_actor(DamageRequest {
                    target: a,
                    source: if c[4] & 1 == 0 { None } else { Some(b) },
                    inflictor: if c[5] & 1 == 0 { None } else { Some(b) },
                    amount: value,
                })
                .map(|_| ()),
            1 => s
                .line_attack(
                    a,
                    LineAttack {
                        angle: value as u32,
                        range: Fixed::from_map_units([0, 64, 512, 2048][usize::from(c[7] % 4)]),
                        slope: Fixed::from_bits(value),
                        damage: i32::from(c[3]) * 4,
                    },
                )
                .map(|_| ()),
            2 => s.demon_melee_attack(a),
            3 => s.check_melee_range(a).map(|_| ()),
            4 => s.chase(a),
            5 => s.enter_actor_state(
                a,
                StateId(
                    [
                        0, 1, 3, 2, 15, 16, 17, 18, 19, 20, 21, 4, 5, 6, 7, 8, 9, 14, 149, 496,
                    ][usize::from(c[7] % 20)],
                ),
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
                    ActorTypeId([12, 13, 34, 135, 136, 37, 38][usize::from(c[7] % 7)]),
                    [Fixed::from_bits(value), Fixed::ZERO],
                    SpawnHeight::Floor,
                )
                .map(|_| ()),
            11 => s.set_momentum(a, [Fixed::from_bits(value), Fixed::ZERO]),
            12 => s.request_actor_death(a),
            13 => {
                // Each individual tic has its own atomicity contract. At most
                // 32 tics across the eight operations reaches delayed strikes.
                for _ in 0..=c[7] % 4 {
                    let before_tic = s.snapshot();
                    if s.step([TicCommand::default(); 4]).is_err() {
                        assert_eq!(before_tic, s.snapshot());
                    }
                }
                Ok(())
            }
            14 => s.noise_alert(a, b),
            15 => s.pos_attack(a),
            16 => {
                s.set_paused(c[7] & 1 != 0);
                Ok(())
            }
            17 => s.shotgun_attack(a),
            18 => s.chaingunner_attack(a),
            19 => s.spawn_map_things().map(|_| ()),
            20 => s.set_vertical_momentum(a, Fixed::from_bits(value)),
            _ => {
                let old = s.snapshot();
                let fresh = s.restart_demon_map_context();
                assert_eq!(s.snapshot(), old);
                fresh.map(|next| {
                    assert_eq!(next.level_statistics().unwrap().single_player_kill_count, 0);
                })
            }
        };
        if result.is_err() {
            assert_eq!(before, s.snapshot());
        }
    }
});
