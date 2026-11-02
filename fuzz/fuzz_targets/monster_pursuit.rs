//! Bounded pursuit/state/roster/identity operations; full error atomicity.
#![no_main]
#[path = "../../tests/support/m3d3b.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    static CASES: std::sync::OnceLock<Vec<fixtures::Case>> = std::sync::OnceLock::new();
    let label = [
        "thinker_acquire_and_pursue",
        "thinker_wall_alternates",
        "fully_trapped",
        "chase_missing_reacquires",
        "chase_count_zero",
        "move_headroom",
    ][usize::from(first % 6)];
    let c = CASES
        .get_or_init(fixtures::cases)
        .iter()
        .find(|c| c.label == label)
        .unwrap();
    let mut sim = fixtures::setup(c);
    let initial = PursuitSetup::from_actor(sim.actor(ActorId::from_parts(0, 1)).unwrap()).unwrap();
    for chunk in rest.as_chunks::<8>().0.iter().take(8) {
        let a = ActorId::from_parts(u32::from(chunk[1] % 8), u64::from(chunk[2] % 3));
        let b = ActorId::from_parts(u32::from(chunk[7] % 8), 1);
        let value = i32::from_le_bytes(chunk[3..7].try_into().unwrap());
        let before = sim.snapshot();
        let result = match chunk[0] % 14 {
            0 => sim.move_pursuer(a).map(|_| ()),
            1 => sim.try_walk(a).map(|_| ()),
            2 => sim.new_chase_direction(a),
            3 => sim.chase(a),
            4 => sim.look(a).map(|_| ()),
            5 => sim.step([TicCommand::default(); 4]).map(|_| ()),
            6 => sim.set_perception_player(
                usize::from(chunk[7] % 6),
                PerceptionPlayer {
                    active: chunk[3] & 1 != 0,
                    health: value,
                    actor: if chunk[4] & 1 != 0 { None } else { Some(a) },
                },
            ),
            7 => sim.mark_remove(a),
            8 => sim.set_actor_target(a, if chunk[6] & 1 != 0 { None } else { Some(b) }),
            9 => {
                let mut p = sim
                    .actor(a)
                    .and_then(PursuitSetup::from_actor)
                    .unwrap_or(initial);
                match chunk[7] % 7 {
                    0 => p.movedir = chunk[3],
                    1 => p.movecount = value,
                    2 => p.reaction_time = value,
                    3 => p.threshold = value,
                    4 => p.flags = value as u32,
                    5 => p.health = value,
                    _ => p.angle = value as u32,
                };
                sim.configure_pursuit_actor(a, p)
            }
            10 => sim.enter_actor_state(
                a,
                StateId([0, 1, 2, 149, 155, 173, 255][usize::from(chunk[7] % 7)]),
            ),
            11 => sim.noise_alert(a, b),
            12 => sim
                .spawn_type(
                    ActorTypeId(if chunk[7] & 1 != 0 { 136 } else { 0 }),
                    [Fixed::ZERO; 2],
                    SpawnHeight::Floor,
                )
                .map(|_| ()),
            _ => sim.set_momentum(a, [Fixed::from_bits(value), Fixed::ZERO]),
        };
        if result.is_err() {
            assert_eq!(sim.snapshot(), before);
        }
    }
});
