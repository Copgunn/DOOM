//! Independent original C damage, hitscan/intercept, effect and lifecycle evidence.
#[path = "../../../tests/support/m3d1.rs"]
mod fixtures;
use doom_sim::*;
fn numbers(row: &str) -> Vec<i64> {
    row.split(',').skip(1).map(|s| s.parse().unwrap()).collect()
}
fn object(o: TraceObject) -> [i64; 2] {
    match o {
        TraceObject::Line(i) => [1, i64::from(i)],
        TraceObject::Actor(id) => [0, i64::from(id.slot())],
    }
}
#[test]
fn damage_hitscan_impacts_and_complete_death_progressions_match_c() {
    let source = include_str!("../../../tests/reference/m3d1_trace.csv");
    let cases = fixtures::cases();
    let mut comparisons = 0;
    for (which, case) in cases.iter().enumerate() {
        let mut sim = fixtures::setup(case);
        let mut timeline = sim.pending_events().to_vec();
        let rows = source
            .lines()
            .filter(|r| r.split(',').nth(1).unwrap().parse::<usize>().unwrap() == which)
            .collect::<Vec<_>>();
        for operation in 0..=case.n(19) + 1 {
            let pending = sim.pending_events().len();
            if operation == 1 {
                if case.n(0) == 4 {
                    let starts = rows
                        .iter()
                        .enumerate()
                        .filter_map(|(i, r)| r.starts_with("D,").then_some(i))
                        .collect::<Vec<_>>();
                    assert_eq!(starts.len(), 100);
                    for (i, &start) in starts.iter().enumerate() {
                        let end = starts.get(i + 1).copied().unwrap_or(rows.len());
                        let result = sim
                            .line_attack(
                                ActorId::from_parts(0, 1),
                                LineAttack {
                                    angle: case.n(8) as u32,
                                    range: case.f(9),
                                    slope: case.f(10),
                                    damage: 0,
                                },
                            )
                            .unwrap();
                        check_trace(&result.trace, &rows[start..end], 1, result.traversed);
                    }
                } else if case.n(0) == 2 {
                    let before = sim.snapshot();
                    let aim = sim
                        .aim_line_attack(ActorId::from_parts(0, 1), case.n(8) as u32, case.f(9))
                        .unwrap();
                    assert_eq!(sim.snapshot(), before);
                    let q = rows
                        .iter()
                        .find(|r| r.starts_with("Q,"))
                        .map(|r| numbers(r))
                        .unwrap();
                    assert_eq!(
                        vec![
                            aim.target.map_or(-1, |id| i64::from(id.slot())),
                            i64::from(aim.slope.bits()),
                            i64::from(aim.window[0].bits()),
                            i64::from(aim.window[1].bits())
                        ],
                        q[2..],
                        "aim {}",
                        case.label
                    );
                    check_trace(&aim.trace, &rows, 0, aim.traversed);
                } else {
                    let result = fixtures::execute(&mut sim, case);
                    if let Some(result) = result {
                        let h = rows
                            .iter()
                            .find(|r| r.starts_with("H,"))
                            .map(|r| numbers(r))
                            .unwrap();
                        let hit = result.hit.map_or([-1, -1], object);
                        let point = result
                            .position
                            .map_or([0; 3], |p| p.map(|v| i64::from(v.bits())));
                        assert_eq!(
                            vec![hit[0], hit[1], point[0], point[1], point[2]],
                            h[2..],
                            "hit {}",
                            case.label
                        );
                        check_trace(&result.trace, &rows, 1, result.traversed);
                    }
                }
            } else if operation > 1 {
                sim.step([TicCommand::default(); 4]).unwrap();
            }
            if operation == 1 {
                timeline.extend_from_slice(&sim.pending_events()[pending..]);
            } else if operation > 1 {
                timeline.extend_from_slice(&sim.events()[pending..]);
            }
            for row in rows.iter().filter(|r| r.starts_with("A,")) {
                let c = numbers(row);
                if c[1] != i64::from(operation) {
                    continue;
                }
                let a = sim.actor(ActorId::from_parts(c[2] as u32, 1));
                assert_eq!(
                    a.is_some(),
                    c[3] != 0,
                    "{} op {operation} actor {}",
                    case.label,
                    c[2]
                );
                if let Some(a) = a {
                    let life = a.life().unwrap();
                    let p = a.position();
                    let m = a.momentum();
                    let constraints = a.constraints();
                    let got = vec![
                        i64::from(life.actor_type.0),
                        i64::from(p[0].bits()),
                        i64::from(p[1].bits()),
                        i64::from(a.z().bits()),
                        i64::from(m[0].bits()),
                        i64::from(m[1].bits()),
                        i64::from(a.vertical_momentum().bits()),
                        i64::from(life.health),
                        life.state.map_or(-1, |s| i64::from(s.0)),
                        i64::from(life.tics),
                        i64::from(life.flags),
                        i64::from(a.height().bits()),
                        i64::from(a.radius().bits()),
                        i64::from(life.reaction_time),
                        i64::from(life.threshold),
                        life.target.map_or(-1, |id| i64::from(id.slot())),
                        i64::from(a.lastlook()),
                        i64::from(constraints[0].bits()),
                        i64::from(constraints[1].bits()),
                        i64::from(a.location().unwrap().subsector),
                        i64::from(sim.rng().indices()[0]),
                    ];
                    assert_eq!(got, c[4..], "{} op {operation} actor {}", case.label, c[2]);
                    comparisons += got.len();
                }
            }
        }
        let actual = timeline.iter().filter_map(life_event).collect::<Vec<_>>();
        let expected = rows
            .iter()
            .filter(|r| r.starts_with("E,"))
            .map(|r| numbers(r))
            .filter(|c| c[2] != 8)
            .map(|c| [c[2], c[3], c[4]])
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "life event order {}", case.label);
    }
    assert_eq!(comparisons, 38_052);
}
fn check_trace(trace: &HitscanTrace, rows: &[&str], stage: i64, traversed: u32) {
    let geometry = rows
        .iter()
        .find(|r| r.starts_with("D,"))
        .map(|r| numbers(r))
        .unwrap();
    assert_eq!(
        trace
            .origin
            .into_iter()
            .chain(trace.delta)
            .map(|v| i64::from(v.bits()))
            .collect::<Vec<_>>(),
        geometry[3..]
    );
    let cells = rows
        .iter()
        .filter(|r| r.starts_with("B,"))
        .map(|r| numbers(r)[3..].to_vec())
        .collect::<Vec<_>>();
    assert_eq!(
        trace
            .cells
            .iter()
            .map(|c| c.map(i64::from).to_vec())
            .collect::<Vec<_>>(),
        cells
    );
    let expected = rows
        .iter()
        .filter(|r| r.starts_with("C,"))
        .map(|r| numbers(r))
        .filter(|c| c[2] == stage)
        .map(|c| [c[3], c[4]])
        .collect::<Vec<_>>();
    assert_eq!(
        trace
            .candidates
            .iter()
            .copied()
            .map(object)
            .collect::<Vec<_>>(),
        expected,
        "candidate order"
    );
    let mut expected = rows
        .iter()
        .filter(|r| r.starts_with("I,"))
        .map(|r| numbers(r))
        .filter(|c| c[2] == stage)
        .map(|c| [c[3], c[4], c[5]])
        .collect::<Vec<_>>();
    expected.sort_by_key(|c| c[2]);
    let got = trace
        .intercepts
        .iter()
        .map(|i| {
            let o = object(i.object);
            [o[0], o[1], i64::from(i.fraction.bits())]
        })
        .collect::<Vec<_>>();
    assert_eq!(got, expected, "intercept fractions/order");
    let expected = rows
        .iter()
        .filter(|r| r.starts_with("T,"))
        .map(|r| numbers(r))
        .filter(|c| c[2] == stage)
        .map(|c| [c[3], c[4], c[5]])
        .collect::<Vec<_>>();
    assert_eq!(traversed as usize, expected.len());
    assert_eq!(
        &got[..expected.len()],
        expected.as_slice(),
        "callback traversal order"
    );
}

fn life_event(event: &Event) -> Option<[i64; 3]> {
    match event {
        Event::Think(id) => Some([0, i64::from(id.slot()), 0]),
        Event::RemovalMarked(id) => Some([2, i64::from(id.slot()), 0]),
        Event::Retired(id) => Some([3, i64::from(id.slot()), 0]),
        Event::ActorState(event) => match event {
            ActorStateEvent::Initialized { actor, actor_type } => {
                Some([1, i64::from(actor.slot()), i64::from(actor_type.0)])
            }
            ActorStateEvent::Entered { actor, state } => {
                Some([4, i64::from(actor.slot()), i64::from(state.0)])
            }
            ActorStateEvent::Action {
                actor,
                action: ActorAction::Original(action),
            } => Some([5, i64::from(actor.slot()), i64::from(action.0)]),
            ActorStateEvent::Sound { actor, sound } => Some([
                7,
                actor.map_or(-1, |id| i64::from(id.slot())),
                i64::from(*sound),
            ]),
            _ => None,
        },
        _ => None,
    }
}
