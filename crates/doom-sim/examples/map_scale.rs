//! Asserted original-C geometry example; optional host-only measurements.
#[path = "../../../tests/support/m3f1.rs"]
mod fixtures;
use doom_sim::{ActorSpec, Event, Profile, Simulation, Solidity, TicCommand};
use doom_types::Fixed;
use std::{hint::black_box, time::Instant};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--measure") {
        let index = args.get(2).unwrap().parse::<usize>().unwrap();
        measure(index, args.get(3).map(String::as_str) == Some("legacy"));
        return;
    }
    let case = &fixtures::cases()[14];
    let (mut sim, actor, _) = case.setup(false);
    assert_eq!(sim.profile(), Profile::MapScale);
    let metrics = sim.map_scale_metrics().unwrap();
    assert_eq!((metrics.lines, metrics.sectors), (769, 256));
    let mut ordered = false;
    for tic in 1..=24 {
        let snapshot = sim.step(case.commands()).unwrap();
        for e in sim.events() {
            if let Event::SectorMovement { actor: id, attempt } = e {
                if *id == actor && attempt.movement.lines.candidates.len() > 2 {
                    assert_eq!(&attempt.movement.lines.candidates[..2], &[0, 768]);
                    ordered = true;
                }
            }
        }
        if tic == 1 {
            fixtures::assert_canonical(
                include_bytes!("../../../tests/reference/m3f1_state_14_1.bin"),
                &snapshot,
            );
        }
        if tic == 24 {
            fixtures::assert_canonical(
                include_bytes!("../../../tests/reference/m3f1_state_14_24.bin"),
                &snapshot,
            );
        }
    }
    assert!(ordered);
    assert!(sim.actor(actor).unwrap().location().unwrap().sector > 100);
    let (mut repeated, _, _) = case.setup(false);
    for _ in 0..24 {
        repeated.step(case.commands()).unwrap();
    }
    assert_eq!(sim.snapshot(), repeated.snapshot());
    let capacity = Simulation::map_scale(fixtures::map(8))
        .unwrap()
        .map_scale_metrics()
        .unwrap();
    assert_eq!(capacity.lines, 1024);
    println!(
        "map_scale: 1024-line capacity; 769 lines / 256 sectors exercised; DSIM-v18 {:016x}",
        sim.snapshot().digest()
    );
}
fn measure(index: usize, legacy: bool) {
    let mut preparation = std::time::Duration::ZERO;
    for _ in 0..100 {
        let map = fixtures::map(index);
        let start = Instant::now();
        let sim = if legacy {
            Simulation::multisector(map)
        } else {
            Simulation::map_scale(map)
        }
        .unwrap();
        preparation += start.elapsed();
        black_box(sim);
    }
    let mut sim = if legacy {
        Simulation::multisector(fixtures::map(index))
    } else {
        Simulation::map_scale(fixtures::map(index))
    }
    .unwrap();
    let a = sim
        .spawn(ActorSpec {
            position: [Fixed::from_map_units(-64), Fixed::ZERO],
            ..Default::default()
        })
        .unwrap();
    let b = sim
        .spawn_with_solidity(
            ActorSpec {
                position: [Fixed::from_map_units(64), Fixed::ZERO],
                height: Fixed::from_map_units(16),
                ..Default::default()
            },
            Solidity::NonSolid,
        )
        .unwrap();
    let start = Instant::now();
    let mut candidates = 0;
    for _ in 0..1000 {
        candidates += black_box(
            sim.check_multisector_position(a, [Fixed::from_map_units(-58), Fixed::ZERO])
                .unwrap(),
        )
        .movement
        .lines
        .candidates
        .len();
    }
    let collision = start.elapsed();
    let start = Instant::now();
    let mut visits = 0;
    if !legacy {
        for _ in 0..1000 {
            visits += black_box(sim.check_sight(a, b).unwrap()).visits.len();
        }
    }
    let sight = start.elapsed();
    sim.set_momentum(a, [Fixed::from_map_units(6), Fixed::ZERO])
        .unwrap();
    let start = Instant::now();
    for _ in 0..100 {
        black_box(sim.step([TicCommand::default(); 4]).unwrap());
    }
    let stepping = start.elapsed();
    println!(
        "map={index} legacy={legacy} prepare100_us={} collision1000_us={} candidates={candidates} sight1000_us={} sight_visits={visits} step100_us={} metrics={:?}",
        preparation.as_micros(),
        collision.as_micros(),
        sight.as_micros(),
        stepping.as_micros(),
        if legacy {
            None
        } else {
            Some(sim.map_scale_metrics().unwrap())
        }
    );
}
