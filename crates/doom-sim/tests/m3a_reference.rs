//! Original C results; report the first differing tic and observable field.
#[path = "../../../tests/support/mod.rs"]
mod support;
use doom_map::{Map, MapLimits};
use doom_sim::{ActorId, ActorSpec, Event, Simulation, Solidity, TicCommand};
use doom_types::Fixed;
use doom_wad::{Limits, Wad};

fn simulation(angled: bool) -> Simulation {
    simulation_ordered(angled, false)
}
fn simulation_ordered(angled: bool, ordered: bool) -> Simulation {
    let mut lumps = support::collision_arena(angled);
    if ordered {
        lumps[10].1 = support::words(&[
            -128, -128, 2, 2, 8, 14, 20, 26, 0, 0, 3, 2, 1, -1, 0, 0, 1, 3, 2, -1, 0, 0, 2, 3, 1,
            -1, 0, 0, 2, 1, 3, -1,
        ]);
    }
    let bytes = support::wad(*b"PWAD", &lumps);
    let wad = Wad::parse(&bytes, Limits::default()).unwrap();
    Simulation::player_collision(Map::from_wad(&wad, "MAP01", MapLimits::default()).unwrap())
        .unwrap()
}
fn values(s: &str) -> Vec<i32> {
    if s == "-" {
        Vec::new()
    } else {
        s.split(':').map(|s| s.parse().unwrap()).collect()
    }
}
fn number(n: i32) -> Fixed {
    Fixed::from_bits(n)
}
fn spec(x: i32, y: i32) -> ActorSpec {
    ActorSpec {
        position: [number(x), number(y)],
        ..Default::default()
    }
}

#[test]
fn all_c_player_slide_actor_order_and_lifecycle_tics_match() {
    let mut sim = simulation(false);
    let mut handles: [Option<ActorId>; 4] = [None; 4];
    for row in include_str!("../../../tests/reference/m3a_movement.csv")
        .lines()
        .skip(1)
    {
        let c: Vec<_> = row.split(',').collect();
        let n: Vec<i64> = c[..16].iter().map(|v| v.parse().unwrap()).collect();
        let case = n[0];
        let tic = n[1];
        if tic == 0 {
            sim = simulation_ordered(n[2] == 1, case == 20);
            handles = [None; 4];
            if (12..18).contains(&case) || case == 19 {
                let x = if case == 14 { 0 } else { 20 * 65536 };
                handles[1] = Some(
                    sim.spawn_with_solidity(spec(x, 0), Solidity::Solid)
                        .unwrap(),
                );
                handles[2] = Some(
                    sim.spawn_with_solidity(
                        spec(x, 0),
                        if case == 15 {
                            Solidity::NonSolid
                        } else {
                            Solidity::Solid
                        },
                    )
                    .unwrap(),
                );
            }
            let mut actor = spec(n[7] as i32, n[8] as i32);
            actor.momentum = [number(n[9] as i32), number(n[10] as i32)];
            handles[0] = Some(sim.spawn_with_solidity(actor, Solidity::Solid).unwrap());
            if n[3] == 1 {
                sim.bind_player(0, handles[0].unwrap()).unwrap();
            }
        }
        if case == 17 && tic == 2 {
            sim.set_momentum(handles[1].unwrap(), [Fixed::ONE, Fixed::ZERO])
                .unwrap();
        }
        if case == 17 && tic == 3 {
            sim.mark_remove(handles[1].unwrap()).unwrap();
        }
        if case == 17 && tic == 4 {
            handles[3] = Some(
                sim.spawn_with_solidity(spec(20 * 65536, 0), Solidity::Solid)
                    .unwrap(),
            );
        }
        let player = handles[0].unwrap();
        let actor = sim.actor(player).unwrap();
        assert_eq!(
            actor.position().map(Fixed::bits),
            [n[7] as i32, n[8] as i32],
            "case {case} tic {tic} initial position"
        );
        assert_eq!(
            actor.momentum().map(Fixed::bits),
            [n[9] as i32, n[10] as i32],
            "case {case} tic {tic} initial momentum"
        );
        let mut commands = [TicCommand::default(); 4];
        commands[0] = TicCommand {
            forward: n[4] as i8,
            side: n[5] as i8,
            angle_turn: n[6] as i16,
            buttons: 0,
        };
        sim.step(commands).unwrap();
        let a = sim.actor(player).unwrap();
        assert_eq!(
            a.position().map(Fixed::bits),
            [n[11] as i32, n[12] as i32],
            "case {case} tic {tic} position"
        );
        assert_eq!(
            a.momentum().map(Fixed::bits),
            [n[13] as i32, n[14] as i32],
            "case {case} tic {tic} momentum"
        );
        assert_eq!(i64::from(a.angle()), n[15], "case {case} tic {tic} angle");
        let attempts: Vec<_> = sim
            .events()
            .iter()
            .filter_map(|e| match e {
                Event::ActorMovement { actor, attempt } if *actor == player => Some(attempt),
                _ => None,
            })
            .collect();
        let expected: Vec<_> = if c[16] == "-" {
            Vec::new()
        } else {
            c[16].split(';').map(values).collect()
        };
        let actual: Vec<_> = attempts
            .iter()
            .map(|a| {
                vec![
                    a.target[0].bits(),
                    a.target[1].bits(),
                    i32::from(a.allowed()),
                ]
            })
            .collect();
        assert_eq!(
            actual, expected,
            "case {case} tic {tic} attempted destinations/allow"
        );
        let lines: Vec<_> = attempts
            .iter()
            .flat_map(|a| a.lines.candidates.iter().map(|n| i32::from(*n)))
            .collect();
        assert_eq!(lines, values(c[17]), "case {case} tic {tic} line order");
        let things: Vec<_> = attempts
            .iter()
            .flat_map(|a| &a.actors)
            .map(|id| handles.iter().position(|h| *h == Some(*id)).unwrap() as i32)
            .collect();
        assert_eq!(things, values(c[18]), "case {case} tic {tic} actor order");
        let slides: Vec<_> = sim
            .events()
            .iter()
            .filter_map(|e| match e {
                Event::Slide { actor, line } if *actor == player => Some(i32::from(*line)),
                _ => None,
            })
            .collect();
        assert_eq!(
            slides,
            values(c[19]),
            "case {case} tic {tic} slide line order"
        );
    }
}
