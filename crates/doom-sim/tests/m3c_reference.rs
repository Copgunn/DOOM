//! Independent original C metadata, countdown, callback and placement comparisons.
#[path = "../../../tests/support/m3c.rs"]
mod fixtures;
use doom_sim::*;
fn values(line: &str) -> Vec<i64> {
    line.split(',')
        .skip(1)
        .map(|s| s.parse().unwrap())
        .collect()
}
fn event(event: &Event) -> Option<(i64, i64, i64)> {
    let id = |id: ActorId| i64::from(id.slot());
    match event {
        Event::Spawned(a) => Some((1, id(*a), -2)),
        Event::Think(a) => Some((0, id(*a), -1)),
        Event::RemovalMarked(a) => Some((2, id(*a), -1)),
        Event::Retired(a) => Some((3, id(*a), -1)),
        Event::ActorState(e) => match e {
            ActorStateEvent::Entered { actor, state } => Some((4, id(*actor), i64::from(state.0))),
            ActorStateEvent::Action { actor, action } => Some((
                5,
                id(*actor),
                match action {
                    ActorAction::Original(id) => i64::from(id.0),
                    ActorAction::Probe(p) => match p {
                        ProbeAction::Random => 100,
                        ProbeAction::Enter(_) => 101,
                        ProbeAction::SpawnTarget(_) => 102,
                        ProbeAction::RemoveSelf => 103,
                        ProbeAction::RemoveTarget => 104,
                        ProbeAction::TargetSelf => 105,
                    },
                    ActorAction::None => panic!("no-action must not dispatch"),
                },
            )),
            ActorStateEvent::Target { actor, target } => {
                Some((6, id(*actor), target.map_or(-1, id)))
            }
            ActorStateEvent::Sound { actor, sound } => {
                Some((7, actor.map_or(-1, id), i64::from(*sound)))
            }
            ActorStateEvent::Random { actor, value } => Some((8, id(*actor), i64::from(*value))),
            _ => None,
        },
        _ => None,
    }
}
#[test]
fn all_original_metadata_matches_compiled_c() {
    let registry = ActorRegistry::vanilla();
    let states = include_str!("../../../tests/reference/m3c_states.csv")
        .lines()
        .skip(1)
        .collect::<Vec<_>>();
    assert_eq!(registry.states().len(), states.len());
    for row in states {
        let c = row.split(',').collect::<Vec<_>>();
        let index: usize = c[0].parse().unwrap();
        let s = &registry.states()[index];
        let action = match s.action {
            ActorAction::None => 0,
            ActorAction::Original(id) => i32::from(id.0),
            _ => panic!(),
        };
        assert_eq!(s.name, c[1]);
        assert_eq!(
            vec![
                i32::from(s.sprite),
                s.frame as i32,
                s.tics,
                action,
                i32::from(s.next.0),
                s.misc[0],
                s.misc[1]
            ],
            c[2..]
                .iter()
                .map(|s| s.parse::<i32>().unwrap())
                .collect::<Vec<_>>()
        );
    }
    let rows = include_str!("../../../tests/reference/m3c_types.csv")
        .lines()
        .skip(1)
        .collect::<Vec<_>>();
    assert_eq!(registry.types().len(), rows.len());
    for row in rows {
        let c = row.split(',').collect::<Vec<_>>();
        let t = &registry.types()[c[0].parse::<usize>().unwrap()];
        assert_eq!(t.name, c[1]);
        assert_eq!(
            t.properties.as_slice(),
            &c[2..]
                .iter()
                .map(|s| s.parse::<i32>().unwrap())
                .collect::<Vec<_>>()
        );
    }
}
#[test]
fn source_state_spawn_actions_thinker_and_map_traces_match() {
    let lines = include_str!("../../../tests/reference/m3c_trace.csv");
    let mut comparisons = 0;
    for case in 0..=8 {
        let mut sim = fixtures::setup(case);
        let mut all_events = Vec::new();
        let end = match case {
            0 => 80,
            6 => 3,
            7 => 5,
            8 => 2,
            _ => 0,
        };
        for tic in 0..=end {
            if tic > 0 {
                match case {
                    7 => sim
                        .enter_actor_state(ActorId::from_parts(0, 1), StateId(tic + 1))
                        .unwrap(),
                    8 if tic == 2 => sim
                        .enter_actor_state(ActorId::from_parts(0, 1), StateId(1))
                        .unwrap(),
                    _ => {
                        sim.step([TicCommand::default(); 4]).unwrap();
                        all_events.extend(sim.events().iter().filter_map(event));
                    }
                }
            }
            for row in lines.lines().filter(|s| s.starts_with("A,")) {
                let c = values(row);
                if c[0] != i64::from(case) || c[1] != i64::from(tic) {
                    continue;
                }
                let id = ActorId::from_parts(c[2] as u32, 1);
                let a = sim.actor(id);
                assert_eq!(
                    a.is_some(),
                    c[3] != 0,
                    "case {case} tic {tic} actor {}",
                    c[2]
                );
                if let Some(a) = a {
                    let life = a.life().unwrap();
                    let observed = vec![
                        i64::from(life.actor_type.0),
                        life.state.map_or(-1, |s| i64::from(s.0)),
                        i64::from(life.tics),
                        i64::from(life.sprite),
                        i64::from(life.frame),
                        i64::from(life.health),
                        i64::from(life.reaction_time),
                        i64::from(life.flags),
                        i64::from(a.lastlook()),
                        i64::from(a.z().bits()),
                        i64::from(a.radius().bits()),
                        i64::from(a.height().bits()),
                        i64::from(a.angle()),
                        life.target.map_or(-1, |id| i64::from(id.slot())),
                        i64::from(sim.rng().indices()[0]),
                    ];
                    assert_eq!(observed, c[4..], "case {case} tic {tic} actor {}", c[2]);
                    comparisons += observed.len();
                }
            }
        }
        // Flush pending-only setup/action observations without executing a tic.
        // Canonical event payloads are decoded by the independent golden test.
        if case == 0 || case == 6 {
            let expected = lines
                .lines()
                .filter(|s| s.starts_with("E,"))
                .map(values)
                .filter(|c| c[0] == i64::from(case))
                .map(|c| (c[2], c[3], if c[2] == 1 { -2 } else { c[4] }))
                .collect::<Vec<_>>();
            assert_eq!(all_events, expected, "ordered events case {case}");
            comparisons += expected.len();
        }
    }
    assert!(comparisons > 2500);
}
#[test]
fn independent_v4_digests_and_full_states_match() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
    let mut cases = (0..=8).map(fixtures::setup).collect::<Vec<_>>();
    let mut previous = [0_u16; 9];
    let mut count = 0;
    let mut full = 0;
    for row in include_str!("../../../tests/reference/m3c_snapshots.csv")
        .lines()
        .skip(1)
    {
        let c = row.split(',').collect::<Vec<_>>();
        let case = c[0].parse::<usize>().unwrap();
        let step = c[1].parse::<u16>().unwrap();
        let sim = &mut cases[case];
        if step > previous[case] {
            match case {
                7 => sim
                    .enter_actor_state(ActorId::from_parts(0, 1), StateId(step + 1))
                    .unwrap(),
                8 if step == 2 => sim
                    .enter_actor_state(ActorId::from_parts(0, 1), StateId(1))
                    .unwrap(),
                _ => {
                    sim.step([TicCommand::default(); 4]).unwrap();
                }
            }
            previous[case] = step;
        }
        let snapshot = sim.snapshot();
        if c[4] != "-" {
            let expected = std::fs::read(root.join(c[4])).unwrap();
            let actual = snapshot.canonical_bytes();
            if actual != expected {
                let mut cursor = 16;
                while cursor < expected.len() {
                    let length =
                        u32::from_le_bytes(expected[cursor..cursor + 4].try_into().unwrap())
                            as usize;
                    let path =
                        std::str::from_utf8(&expected[cursor + 4..cursor + 4 + length]).unwrap();
                    cursor += 4 + length;
                    let size = u64::from_le_bytes(expected[cursor..cursor + 8].try_into().unwrap())
                        as usize;
                    cursor += 8;
                    let field = snapshot
                        .fields()
                        .find(|f| f.path() == path)
                        .unwrap_or_else(|| panic!("missing field {path}"));
                    assert_eq!(
                        field.value(),
                        &expected[cursor..cursor + size],
                        "case {case} step {step} field {path}"
                    );
                    cursor += size;
                }
                panic!("schema field order/count mismatch case {case} step {step}");
            }
            full += 1;
        }
        assert_eq!(
            format!("{:016x}", snapshot.digest()),
            c[3],
            "case {case} step {step}"
        );
        count += 1;
    }
    assert_eq!(count, 99);
    assert!(full >= 20);
}
#[test]
fn all_67_admitted_source_types_spawn_at_three_heights_like_c() {
    use doom_types::Fixed;
    let mut sim = Simulation::actor_states(fixtures::map(&[])).unwrap();
    let types = sim
        .actor_registry()
        .unwrap()
        .types()
        .iter()
        .enumerate()
        .filter_map(|(i, _)| {
            sim.actor_registry()
                .unwrap()
                .validate_type(ActorTypeId(i as u16), sim.sectors())
                .ok()
                .map(|_| ActorTypeId(i as u16))
        })
        .collect::<Vec<_>>();
    assert_eq!(types.len(), 67);
    for ty in types {
        for height in [
            SpawnHeight::Floor,
            SpawnHeight::Ceiling,
            SpawnHeight::Absolute(Fixed::from_map_units(16)),
        ] {
            sim.spawn_type(ty, [Fixed::ZERO; 2], height).unwrap();
        }
    }
    let mut count = 0;
    for line in include_str!("../../../tests/reference/m3c_trace.csv")
        .lines()
        .filter(|l| l.starts_with("A,9,"))
    {
        let c = values(line);
        let a = sim.actor(ActorId::from_parts(c[2] as u32, 1)).unwrap();
        let l = a.life().unwrap();
        assert_eq!(
            vec![
                i64::from(l.actor_type.0),
                l.state.map_or(-1, |s| i64::from(s.0)),
                i64::from(l.tics),
                i64::from(l.sprite),
                i64::from(l.frame),
                i64::from(l.health),
                i64::from(l.reaction_time),
                i64::from(l.flags),
                i64::from(a.lastlook()),
                i64::from(a.z().bits()),
                i64::from(a.radius().bits()),
                i64::from(a.height().bits()),
                i64::from(a.angle()),
                -1,
                i64::from(sim.rng().indices()[0])
            ],
            c[4..]
        );
        count += 1;
    }
    assert_eq!(count, 201);
}
