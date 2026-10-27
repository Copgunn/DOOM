//! Independent coasting goldens, ordered phases, lifecycle and repeated long traces.
#[path = "../../../tests/support/mod.rs"]
mod support;
use doom_map::{Map, MapLimits};
use doom_sim::{
    ActorSpec, Event, GeometryError, Phase, SimError, Simulation, Snapshot, StoreError, TicCommand,
    compare_traces, run_commands,
};
use doom_types::Fixed;
use doom_wad::{Limits, Wad};

fn map(lumps: &[support::Lump]) -> Map {
    let bytes = support::wad(*b"PWAD", lumps);
    let wad = Wad::parse(&bytes, Limits::default()).unwrap();
    Map::from_wad(&wad, "MAP01", MapLimits::default()).unwrap()
}
fn simulation() -> Simulation {
    Simulation::new(map(&support::classic_map("MAP01"))).unwrap()
}
fn commands() -> [TicCommand; 4] {
    [TicCommand::default(); 4]
}

#[test]
fn independent_python_encoding_and_c_positions_match_every_golden_tic() {
    let mut sim = simulation();
    let actor = sim
        .spawn(ActorSpec {
            momentum: [Fixed::ONE, Fixed::ZERO],
            ..Default::default()
        })
        .unwrap();
    let trace = run_commands(&mut sim, (0..12).map(|_| commands())).unwrap();
    assert_eq!(
        trace[0].canonical_bytes(),
        include_bytes!("../../../tests/reference/coast_first.bin")
    );
    for (snapshot, line) in trace.iter().zip(
        include_str!("../../../tests/reference/snapshots.csv")
            .lines()
            .skip(1),
    ) {
        let columns = line.split(',').collect::<Vec<_>>();
        let tic = columns[0].parse::<u64>().unwrap();
        assert_eq!(snapshot.tic(), tic);
        assert_eq!(
            snapshot.digest(),
            u64::from_str_radix(columns[1], 16).unwrap(),
            "independent canonical golden at tic {tic}"
        );
        for (field, column) in [("x", 2), ("y", 3), ("momx", 4), ("momy", 5)] {
            let expected = columns[column].parse::<i32>().unwrap().to_le_bytes();
            let path = format!("actor.0.{field}");
            assert_eq!(
                snapshot
                    .fields()
                    .find(|f| f.path() == path)
                    .unwrap()
                    .value(),
                &expected,
                "C-derived field {path} at tic {tic}"
            );
        }
    }
    assert_eq!(
        sim.actor(actor).unwrap().position()[0].bits(),
        include_str!("../../../tests/reference/snapshots.csv")
            .lines()
            .last()
            .unwrap()
            .split(',')
            .nth(2)
            .unwrap()
            .parse::<i32>()
            .unwrap()
    );
}

#[test]
fn exact_phase_and_actor_order_with_reversed_input_bindings() {
    let mut sim = simulation();
    let a = sim.spawn(ActorSpec::default()).unwrap();
    let b = sim.spawn(ActorSpec::default()).unwrap();
    sim.bind_player(1, a).unwrap();
    sim.bind_player(0, b).unwrap();
    sim.step(commands()).unwrap();
    assert_eq!(
        sim.events(),
        [
            Event::Spawned(a),
            Event::Spawned(b),
            Event::Phase(Phase::Players),
            Event::Player { slot: 0, actor: b },
            Event::Player { slot: 1, actor: a },
            Event::Phase(Phase::Thinkers),
            Event::Think(a),
            Event::Think(b),
            Event::Phase(Phase::Specials),
            Event::Phase(Phase::Respawn),
        ]
    );
    assert_eq!(sim.tic(), 1);
    assert_eq!(sim.rng().indices(), [2, 0]);
}

#[test]
fn paused_steps_are_frozen_and_invalid_commands_do_not_mutate_state() {
    let mut sim = simulation();
    sim.spawn(ActorSpec::default()).unwrap();
    sim.set_paused(true);
    let before = sim.snapshot();
    let mut input = commands();
    input[0].buttons = 1;
    assert_eq!(sim.step(input).unwrap(), before);
    assert_eq!(sim.tic(), 0);
    sim.set_paused(false);
    let before = sim.snapshot();
    assert_eq!(sim.step(input), Err(SimError::UnsupportedButtons));
    assert_eq!(sim.snapshot(), before);
    sim.step(commands()).unwrap();
    assert!(matches!(sim.events()[0], Event::Spawned(_)));
    assert_eq!(sim.tic(), 1);
}

#[test]
fn removal_invalidates_handles_at_retirement_and_reuses_slots_at_the_tail() {
    let mut sim = simulation();
    let a = sim.spawn(ActorSpec::default()).unwrap();
    let b = sim.spawn(ActorSpec::default()).unwrap();
    sim.bind_player(0, a).unwrap();
    sim.mark_remove(a).unwrap();
    assert!(sim.actor(a).is_some());
    assert!(sim.set_momentum(a, [Fixed::ONE; 2]).is_err());
    sim.step(commands()).unwrap();
    assert!(sim.actor(a).is_none());
    assert!(sim.events().contains(&Event::Retired(a)));
    assert!(
        !sim.events()
            .iter()
            .any(|e| matches!(e,Event::Player {actor,..} if *actor==a))
    );
    let c = sim.spawn(ActorSpec::default()).unwrap();
    assert_eq!(a.slot(), c.slot());
    assert_ne!(a.generation(), c.generation());
    assert_eq!(sim.actor_order().collect::<Vec<_>>(), [b, c]);
    assert_eq!(
        sim.mark_remove(a),
        Err(SimError::Store(StoreError::InvalidHandle))
    );
}

#[test]
fn unsupported_maps_and_failed_spawns_are_explicit_without_rng_drift() {
    let mut two_sided = support::classic_map("MAP01");
    support::set_word(&mut two_sided[2].1, 4, 4);
    support::set_word(&mut two_sided[2].1, 12, 0);
    assert!(matches!(
        Simulation::new(map(&two_sided)),
        Err(SimError::Geometry(GeometryError::Unsupported(
            "two-sided lines"
        )))
    ));
    let mut sectors = support::classic_map("MAP01");
    let second = sectors[8].1.clone();
    sectors[8].1.extend(second);
    assert!(matches!(
        Simulation::new(map(&sectors)),
        Err(SimError::Geometry(GeometryError::Unsupported(
            "multiple sectors/portals"
        )))
    ));
    let mut special = support::classic_map("MAP01");
    support::set_word(&mut special[2].1, 6, 1);
    assert!(matches!(
        Simulation::new(map(&special)),
        Err(SimError::Geometry(GeometryError::Unsupported(
            "map specials"
        )))
    ));
    let mut overflow = support::classic_map("MAP01");
    support::set_word(&mut overflow[4].1, 0, i16::MIN);
    assert!(matches!(
        Simulation::new(map(&overflow)),
        Err(SimError::Geometry(GeometryError::CoordinateOverflow))
    ));
    let mut sim = Simulation::with_actor_limit(map(&support::classic_map("MAP01")), 1).unwrap();
    let before = sim.snapshot();
    assert!(
        sim.spawn(ActorSpec {
            radius: Fixed::from_map_units(33),
            ..Default::default()
        })
        .is_err()
    );
    assert_eq!(sim.snapshot(), before);
    sim.spawn(ActorSpec::default()).unwrap();
    let before = sim.snapshot();
    assert_eq!(
        sim.spawn(ActorSpec::default()),
        Err(SimError::Store(StoreError::Capacity))
    );
    assert_eq!(sim.snapshot(), before);
}

fn long_trace(presentation: bool, perturb: bool) -> Vec<Snapshot> {
    let mut sim = simulation();
    let mut a = sim.spawn(ActorSpec::default()).unwrap();
    let mut b = sim
        .spawn(ActorSpec {
            position: [Fixed::from_map_units(-20), Fixed::ZERO],
            ..Default::default()
        })
        .unwrap();
    sim.bind_player(0, a).unwrap();
    sim.bind_player(1, b).unwrap();
    let mut trace = Vec::new();
    for tic in 0..768 {
        if tic % 97 == 10 {
            sim.mark_remove(b).unwrap();
        }
        if tic % 97 == 11 {
            let old = b;
            b = sim.spawn(ActorSpec::default()).unwrap();
            assert!(sim.actor(old).is_none());
            sim.bind_player(1, b).unwrap();
        }
        if tic % 181 == 20 {
            sim.mark_remove(a).unwrap();
        }
        if tic % 181 == 21 {
            a = sim.spawn(ActorSpec::default()).unwrap();
            sim.bind_player(0, a).unwrap();
        }
        sim.set_paused(tic == 300);
        if !sim.paused() {
            sim.gameplay_random();
        }
        if perturb && tic == 400 {
            sim.gameplay_random();
        }
        if presentation {
            for _ in 0..tic % 7 {
                let snapshot = sim.snapshot();
                let _ = snapshot.digest();
                let _ = sim.events();
                let mut copy = sim.rng();
                copy.m_random();
                copy.p_random();
            }
        }
        let mut input = commands();
        input[0] = TicCommand {
            forward: if tic % 64 < 8 { 25 } else { 0 },
            side: if tic % 37 == 0 { 12 } else { 0 },
            angle_turn: if tic % 17 == 0 { 512 } else { 0 },
            buttons: 0,
        };
        input[1].forward = if tic % 23 < 3 { -12 } else { 0 };
        trace.push(sim.step(input).unwrap());
    }
    assert_eq!(sim.tic(), 767);
    trace
}

#[test]
fn fresh_instances_long_streams_and_presentation_schedules_are_identical() {
    let expected = long_trace(false, false);
    let actual = long_trace(true, false);
    assert_eq!(compare_traces(&expected, &actual), None);
    assert_eq!(expected, actual);
    for (e, a) in expected.iter().zip(&actual) {
        assert_eq!(e.canonical_bytes(), a.canonical_bytes());
        assert_eq!(e.digest(), a.digest());
    }
}

#[test]
fn diagnostics_identify_earliest_divergent_tic_and_rng_field() {
    let expected = long_trace(false, false);
    let actual = long_trace(false, true);
    let diff = compare_traces(&expected, &actual).unwrap();
    assert_eq!(diff.tic, 400);
    assert_eq!(diff.field, "rng.gameplay");
    assert_ne!(diff.expected, diff.actual);
    let short = compare_traces(&expected, &expected[..1]).unwrap();
    assert_eq!(short.field, "trace.length");
    assert_eq!(short.tic, 2);
}

#[test]
fn static_map_and_generation_metadata_participate_in_canonical_state() {
    let a = simulation();
    let mut changed = support::classic_map("MAP01");
    support::set_word(&mut changed[4].1, 0, -63);
    let b = Simulation::new(map(&changed)).unwrap();
    assert_eq!(
        a.snapshot().first_difference(&b.snapshot()).unwrap().field,
        "map.vertices"
    );
    let mut b = simulation();
    let id = b.spawn(ActorSpec::default()).unwrap();
    b.mark_remove(id).unwrap();
    b.step(commands()).unwrap();
    assert!(
        b.snapshot()
            .fields()
            .any(|f| f.path() == "actor.0.generation" && f.value() == 2_u64.to_le_bytes())
    );
    assert!(
        b.snapshot()
            .fields()
            .all(|f| !f.path().contains("address") && !f.path().contains("timestamp"))
    );
}
