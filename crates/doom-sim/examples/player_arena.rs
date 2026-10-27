//! Synthetic bevel arena, two solid blockers and a C-verified command sequence.
#[path = "../../../tests/support/mod.rs"]
mod support;
use doom_map::{Map, MapLimits};
use doom_sim::{ActorSpec, Event, Simulation, Solidity, TicCommand};
use doom_types::Fixed;
use doom_wad::{Limits, Wad};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = support::wad(*b"PWAD", &support::collision_arena(true));
    let wad = Wad::parse(&bytes, Limits::default())?;
    let mut sim =
        Simulation::player_collision(Map::from_wad(&wad, "MAP01", MapLimits::default())?)?;
    // Source spawning permits overlap. Two coincident blockers also exercise order.
    for _ in 0..2 {
        sim.spawn_with_solidity(
            ActorSpec {
                position: [Fixed::from_map_units(20), Fixed::ZERO],
                ..Default::default()
            },
            Solidity::Solid,
        )?;
    }
    let player = sim.spawn_with_solidity(ActorSpec::default(), Solidity::Solid)?;
    sim.bind_player(0, player)?;
    let mut input = [TicCommand::default(); 4];
    input[0] = TicCommand {
        forward: 50,
        side: -24,
        angle_turn: 512,
        buttons: 0,
    };
    let expected = include_str!("../../../tests/reference/m3a_movement.csv")
        .lines()
        .skip(1)
        .filter(|row| row.starts_with("19,"));
    let (mut actor_blocks, mut wall_blocks, mut slides) = (0, 0, 0);
    for row in expected {
        let snapshot = sim.step(input)?;
        let columns: Vec<_> = row.split(',').collect();
        let actor = sim.actor(player).expect("live player");
        assert_eq!(
            actor.position().map(Fixed::bits),
            [columns[11].parse::<i32>()?, columns[12].parse::<i32>()?]
        );
        assert_eq!(
            actor.momentum().map(Fixed::bits),
            [columns[13].parse::<i32>()?, columns[14].parse::<i32>()?]
        );
        assert_eq!(actor.angle(), columns[15].parse::<u32>()?);
        for event in sim.events() {
            match event {
                Event::ActorMovement { attempt, .. } => {
                    actor_blocks += usize::from(attempt.blocked_by_actor.is_some());
                    wall_blocks += usize::from(attempt.lines.blocked_by.is_some());
                }
                Event::Slide { .. } => slides += 1,
                _ => {}
            }
        }
        println!(
            "tic={} x={} y={} momx={} momy={} digest={:016x}",
            snapshot.tic(),
            actor.position()[0].bits(),
            actor.position()[1].bits(),
            actor.momentum()[0].bits(),
            actor.momentum()[1].bits(),
            snapshot.digest()
        );
    }
    assert_eq!(sim.tic(), 12);
    assert!(actor_blocks > 0 && wall_blocks > 0 && slides > 0);
    println!(
        "verified actor_blocks={actor_blocks} wall_blocks={wall_blocks} slide_projections={slides}"
    );
    Ok(())
}
