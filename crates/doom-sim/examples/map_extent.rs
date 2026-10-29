//! Asserted expanded classic geometry; timing is optional host tooling only.
#[path = "../../../tests/support/m3f2.rs"]
mod fixtures;
use doom_sim::{ActorSpec, Event, Profile, SightVisit, Simulation, TicCommand};
use doom_types::Fixed;
use std::{hint::black_box, time::Instant};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--measure") {
        measure(
            args[2].parse().unwrap(),
            args.get(3).map(String::as_str) == Some("profile18"),
        );
        return;
    }
    let cases = fixtures::cases();
    let case = &cases[30];
    assert_eq!(case.label, "wide_positive");
    let (mut sim, actor, _) = case.setup(false);
    assert_eq!(sim.profile(), Profile::MapExtent);
    assert!(Simulation::map_scale(fixtures::map(case.map)).is_err());
    assert!(sim.world().map().vertices().iter().any(|v| v.x == 2048));
    let mut sectors = Vec::new();
    let mut ordering = false;
    for tic in 1..=128 {
        let snapshot = sim.step(case.commands()).unwrap();
        for event in sim.events() {
            if let Event::SectorMovement { actor: id, attempt } = event {
                if *id == actor {
                    assert!(attempt.allowed());
                    assert_eq!(&attempt.movement.lines.candidates[..3], &[0, 48, 47]);
                    ordering = true;
                }
            }
        }
        let sector = sim.actor(actor).unwrap().location().unwrap().sector;
        if sectors.last() != Some(&sector) {
            sectors.push(sector);
        }
        if tic == 1 {
            fixtures::assert_canonical(
                include_bytes!("../../../tests/reference/m3f2_state_30_1.bin"),
                &snapshot,
            );
        }
        if tic == 128 {
            fixtures::assert_canonical(
                include_bytes!("../../../tests/reference/m3f2_state_30_128.bin"),
                &snapshot,
            );
        }
    }
    assert!(ordering);
    assert_eq!(sectors, (0..16).collect::<Vec<_>>());
    assert_eq!(
        sim.actor(actor).unwrap().position(),
        [Fixed::from_map_units(1920), Fixed::ZERO]
    );
    let (mut repeated, _, _) = case.setup(false);
    for _ in 0..128 {
        repeated.step(case.commands()).unwrap();
    }
    assert_eq!(sim.snapshot(), repeated.snapshot());
    // Source-C high-cell script, with an independent complete byte fixture.
    let (mut grid, body, _) = cases[20].setup(false);
    let snapshot = grid.step(cases[20].commands()).unwrap();
    fixtures::assert_canonical(
        include_bytes!("../../../tests/reference/m3f2_state_20_1.bin"),
        &snapshot,
    );
    assert_eq!(grid.map_extent_metrics().unwrap().cells, 8192);
    let p = grid.actor(body).unwrap().position().map(Fixed::bits);
    let cell = ((p[1] / 65536 + 2048) / 128) * 256 + (p[0] / 65536 + 2048) / 128;
    assert!(cell > 4096);
    // Exact source-C DAG sight inputs: both original children really execute.
    let mut sight = Simulation::map_extent(fixtures::map(27)).unwrap();
    let mut spawn = |x, y| {
        sight
            .spawn(ActorSpec {
                position: [Fixed::from_map_units(x), Fixed::from_map_units(y)],
                height: Fixed::from_map_units(16),
                ..Default::default()
            })
            .unwrap()
    };
    let observer = spawn(-64, -16);
    let target = spawn(64, 16);
    let before = sight.snapshot();
    let result = sight.check_sight(observer, target).unwrap();
    assert!(result.visible);
    assert_eq!(
        result
            .visits
            .iter()
            .filter(|v| matches!(
                v,
                SightVisit::NodeStart { .. }
                    | SightVisit::NodeEnd { .. }
                    | SightVisit::Subsector(_)
                    | SightVisit::Seg { .. }
            ))
            .count(),
        16382
    );
    assert_eq!(before, sight.snapshot());
    // Digest comes from the independent C-derived encoder, not captured Rust output.
    assert_eq!(sim.snapshot().digest(), 0x2c13f52d732a12c4);
    println!(
        "map_extent: -1920 to +1920 through 16 sectors; cell {cell}; sight work 16382; DSIM-v19 {:016x}",
        sim.snapshot().digest()
    );
}
fn measure(index: usize, profile18: bool) {
    let case = &fixtures::cases()[index];
    assert_eq!(case.map, index);
    let mut parsing = std::time::Duration::ZERO;
    let mut preparation = std::time::Duration::ZERO;
    for _ in 0..100 {
        let start = Instant::now();
        let map = fixtures::map(index);
        parsing += start.elapsed();
        let start = Instant::now();
        let sim = if profile18 {
            Simulation::map_scale(map)
        } else {
            Simulation::map_extent(map)
        }
        .unwrap();
        preparation += start.elapsed();
        black_box(sim);
    }
    let mut sim = if profile18 {
        Simulation::map_scale(fixtures::map(index))
    } else {
        Simulation::map_extent(fixtures::map(index))
    }
    .unwrap();
    let a = sim
        .spawn(ActorSpec {
            position: case.position,
            height: Fixed::from_map_units(16),
            ..Default::default()
        })
        .unwrap();
    let b = sim
        .spawn(ActorSpec {
            position: [
                case.position[0] + Fixed::from_map_units(64),
                case.position[1],
            ],
            height: Fixed::from_map_units(16),
            ..Default::default()
        })
        .unwrap();
    let destination = [
        case.position[0] + Fixed::from_map_units(6),
        case.position[1],
    ];
    let start = Instant::now();
    let mut candidates = 0;
    for _ in 0..1000 {
        candidates += black_box(sim.check_multisector_position(a, destination).unwrap())
            .movement
            .lines
            .candidates
            .len();
    }
    let collision = start.elapsed();
    let start = Instant::now();
    let mut visits = 0;
    let mut work = 0;
    for _ in 0..1000 {
        let result = black_box(sim.check_sight(a, b).unwrap());
        visits += result.visits.len();
        work += result
            .visits
            .iter()
            .filter(|v| {
                matches!(
                    v,
                    SightVisit::NodeStart { .. }
                        | SightVisit::NodeEnd { .. }
                        | SightVisit::Subsector(_)
                        | SightVisit::Seg { .. }
                )
            })
            .count();
    }
    let sight = start.elapsed();
    sim.set_momentum(a, [Fixed::from_map_units(6), Fixed::ZERO])
        .unwrap();
    let start = Instant::now();
    for _ in 0..100 {
        black_box(sim.step([TicCommand::default(); 4]).unwrap());
    }
    let stepping = start.elapsed();
    let metrics = if profile18 {
        sim.map_scale_metrics()
    } else {
        sim.map_extent_metrics()
    }
    .unwrap();
    println!(
        "map={index} profile18={profile18} parse100_us={} prepare100_us={} collision1000_us={} candidates={candidates} sight1000_us={} sight_visits={visits} sight_work={work} step100_us={} lines={} sectors={} cells={} references={} prepared_bytes={}",
        parsing.as_micros(),
        preparation.as_micros(),
        collision.as_micros(),
        sight.as_micros(),
        stepping.as_micros(),
        metrics.lines,
        metrics.sectors,
        metrics.cells,
        metrics.prepared_references,
        metrics.prepared_storage_bytes
    );
}
