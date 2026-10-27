//! Independently recorded C geometry, movement, thrust and scheduling comparisons.
#[path = "../../../tests/support/mod.rs"]
mod support;
use doom_map::{Map, MapLimits};
use doom_sim::{
    ActorSpec, Event, LineGeometry, Simulation, ThinkerStore, TicCommand, Visit, command_thrust,
};
use doom_types::Fixed;
use doom_wad::{Limits, Wad};

fn simulation_with_ordered_lists(ordered: bool) -> Simulation {
    let mut lumps = support::classic_map("MAP01");
    if ordered {
        lumps[10].1 = support::words(&[
            -128, -128, 2, 2, 8, 12, 16, 20, 0, 0, 3, -1, 0, 0, 1, -1, 0, 2, 3, -1, 0, 1, 2, -1,
        ]);
    }
    let bytes = support::wad(*b"PWAD", &lumps);
    let wad = Wad::parse(&bytes, Limits::default()).unwrap();
    Simulation::new(Map::from_wad(&wad, "MAP01", MapLimits::default()).unwrap()).unwrap()
}
fn raw(n: i32) -> Fixed {
    Fixed::from_bits(n)
}

#[test]
fn geometry_axis_diagonal_and_boundary_predicates_match_c() {
    for (i, line) in include_str!("../../../tests/reference/geometry.csv")
        .lines()
        .skip(1)
        .enumerate()
    {
        let c = line
            .split(',')
            .map(|v| v.parse::<i32>().unwrap())
            .collect::<Vec<_>>();
        let l = LineGeometry::new([raw(c[0]), raw(c[1])], [raw(c[2]), raw(c[3])]).unwrap();
        assert_eq!(
            i32::from(l.point_side([raw(c[4]), raw(c[5])])),
            c[7],
            "point predicate C row {}",
            i + 2
        );
        assert_eq!(
            i32::from(l.box_side([raw(c[4]), raw(c[5])], raw(c[6]))),
            c[8],
            "box predicate C row {}",
            i + 2
        );
    }
}

#[test]
fn ground_xy_positions_momentum_substeps_and_candidates_match_c() {
    for (data, ordered) in [
        (include_str!("../../../tests/reference/movement.csv"), false),
        (
            include_str!("../../../tests/reference/ordered_movement.csv"),
            true,
        ),
    ] {
        for (i, line) in data.lines().skip(1).enumerate() {
            let c = line.split(',').collect::<Vec<_>>();
            let n = c[..11]
                .iter()
                .map(|v| v.parse::<i32>().unwrap())
                .collect::<Vec<_>>();
            let mut sim = simulation_with_ordered_lists(ordered);
            let actor = sim
                .spawn(ActorSpec {
                    position: [raw(n[2]), raw(n[3])],
                    momentum: [raw(n[4]), raw(n[5])],
                    ..Default::default()
                })
                .unwrap();
            sim.step([TicCommand::default(); 4]).unwrap();
            let a = sim.actor(actor).unwrap();
            assert_eq!(
                a.position().map(Fixed::bits),
                [n[6], n[7]],
                "C XY position row {}: {line}",
                i + 2
            );
            assert_eq!(
                a.momentum().map(Fixed::bits),
                [n[8], n[9]],
                "C XY momentum row {}: {line}",
                i + 2
            );
            let attempts = sim
                .events()
                .iter()
                .filter_map(|e| {
                    if let Event::Movement { attempt, .. } = e {
                        Some(attempt)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(
                attempts.len(),
                n[10] as usize,
                "C XY substeps row {}",
                i + 2
            );
            let candidates = attempts
                .iter()
                .flat_map(|a| a.collision.candidates.iter().copied())
                .collect::<Vec<_>>();
            let expected = if c[11] == "-" {
                Vec::new()
            } else {
                c[11]
                    .split(':')
                    .map(|v| v.parse::<u16>().unwrap())
                    .collect()
            };
            assert_eq!(candidates, expected, "C candidate ordering row {}", i + 2);
        }
    }
}

#[test]
fn signed_commands_angle_wrap_and_exact_lookup_thrust_match_c() {
    for (i, line) in include_str!("../../../tests/reference/thrust.csv")
        .lines()
        .skip(1)
        .enumerate()
    {
        let c = line
            .split(',')
            .map(|v| v.parse::<i64>().unwrap())
            .collect::<Vec<_>>();
        let (angle, momentum) = command_thrust(
            c[0] as u32,
            [raw(c[4] as i32), raw(c[5] as i32)],
            TicCommand {
                angle_turn: c[1] as i16,
                forward: c[2] as i8,
                side: c[3] as i8,
                buttons: 0,
            },
        )
        .unwrap();
        assert_eq!(
            (angle, momentum.map(Fixed::bits)),
            (c[6] as u32, [c[7] as i32, c[8] as i32]),
            "C thrust row {}: {line}",
            i + 2
        );
    }
}

#[test]
fn thinker_observable_order_matches_retained_storage_c_adapter() {
    let mut actual = String::new();
    for scenario in 0..5 {
        let labels = [
            "remove_self_other_spawn",
            "spawn_by_tail",
            "remove_visited",
            "remove_current",
            "empty",
        ];
        let mut store = ThinkerStore::new(8).unwrap();
        let count = match scenario {
            0 => 3,
            2 => 2,
            4 => 0,
            _ => 1,
        };
        let mut handles = Vec::new();
        for value in 1..=count {
            handles.push(store.spawn(value).unwrap());
        }
        for phase in 0..2 {
            actual.push_str(&format!("{},{}:", labels[scenario], phase));
            store
                .run(8, |visit, context| {
                    if let Visit::Think(id) = visit {
                        let value = *context.get(id).unwrap();
                        actual.push_str(&format!(" {value}"));
                        if phase == 0 {
                            match scenario {
                                0 if value == 1 => {
                                    context.mark_remove(handles[1]).unwrap();
                                    context.mark_remove(id).unwrap();
                                    context.spawn(4).unwrap();
                                }
                                0 if value == 3 => {
                                    // The earlier retirement freed a lower arena slot;
                                    // processing must still append after value 4.
                                    context.spawn(5).unwrap();
                                }
                                1 if value < 3 => {
                                    context.spawn(value + 1).unwrap();
                                }
                                2 if value == 2 => {
                                    context.mark_remove(handles[0]).unwrap();
                                }
                                3 => {
                                    context.mark_remove(id).unwrap();
                                }
                                _ => {}
                            }
                        }
                    }
                })
                .unwrap();
            actual.push('\n');
        }
    }
    assert_eq!(
        actual,
        include_str!("../../../tests/reference/thinkers.txt"),
        "C thinker first-difference: compare case/tic rows"
    );
}
