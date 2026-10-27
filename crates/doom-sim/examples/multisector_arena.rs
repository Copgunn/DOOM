//! Static connected-sector scenarios with C-asserted step/drop/XY/Z outcomes.
#[path = "../../../tests/support/m3b.rs"]
mod fixtures;
use doom_sim::{Event, MoveFailure};
use doom_types::Fixed;
fn main() {
    let cases = fixtures::cases();
    for index in [2, 3, 5, 4, 9, 13] {
        let case = &cases[index];
        let (mut sim, actor, _) = case.setup(true);
        let mut previous_sector = sim.actor(actor).unwrap().location().unwrap().sector;
        let (mut step_block, mut fit_block, mut gravity, mut landing) =
            (false, false, false, false);
        let mut visited = [false; 3];
        let mut reported_rejection = false;
        for row in include_str!("../../../tests/reference/m3b_movement.csv")
            .lines()
            .skip(1)
            .filter(|row| row.split(',').next().unwrap().parse::<usize>().unwrap() == index)
        {
            let snapshot = sim.step(case.commands()).unwrap();
            let a = sim.actor(actor).unwrap();
            let location = a.location().unwrap();
            visited[usize::from(location.sector)] = true;
            let c: Vec<_> = row.split(',').collect();
            let expected: Vec<i64> = c[2..13].iter().map(|n| n.parse().unwrap()).collect();
            let actual = vec![
                i64::from(a.position()[0].bits()),
                i64::from(a.position()[1].bits()),
                i64::from(a.z().bits()),
                i64::from(a.momentum()[0].bits()),
                i64::from(a.momentum()[1].bits()),
                i64::from(a.vertical_momentum().bits()),
                i64::from(a.constraints()[0].bits()),
                i64::from(a.constraints()[1].bits()),
                i64::from(location.subsector),
                i64::from(location.sector),
                i64::from(a.angle()),
            ];
            assert_eq!(
                actual,
                expected,
                "reference case {index} tic {}",
                snapshot.tic()
            );
            let mut rejected = false;
            let mut floor_contact = false;
            for event in sim.events() {
                match event {
                    Event::SectorMovement { attempt, .. } => {
                        step_block |= attempt.failure == MoveFailure::Step;
                        fit_block |= attempt.failure == MoveFailure::Fit;
                        rejected |= !attempt.allowed();
                    }
                    Event::Vertical { update, .. } => {
                        gravity |= update.gravity_applied;
                        floor_contact |= update.floor_contact;
                        landing |= update.floor_contact && update.before[1] < Fixed::ZERO;
                    }
                    _ => {}
                }
            }
            if snapshot.tic() == 1
                || snapshot.tic() == 24
                || location.sector != previous_sector
                || floor_contact
                || rejected && !reported_rejection
            {
                println!(
                    "{} tic={} xyz=[{},{},{}] momentum=[{},{},{}] sector={} digest={:016x}",
                    case.label,
                    snapshot.tic(),
                    a.position()[0].bits(),
                    a.position()[1].bits(),
                    a.z().bits(),
                    a.momentum()[0].bits(),
                    a.momentum()[1].bits(),
                    a.vertical_momentum().bits(),
                    location.sector,
                    snapshot.digest()
                );
            }
            reported_rejection |= rejected;
            previous_sector = location.sector;
        }
        assert_eq!(sim.tic(), 24);
        match index {
            2 => {
                assert!(visited[1]);
                assert_eq!(sim.actor(actor).unwrap().z(), Fixed::from_map_units(24));
            }
            3 => {
                assert!(!visited[1]);
                assert!(step_block);
            }
            5 => assert!(visited[1]),
            4 => {
                assert!(!visited[1]);
                assert!(fit_block);
            }
            9 => {
                assert!(visited[1] && gravity && landing);
                assert_eq!(sim.actor(actor).unwrap().z(), Fixed::from_map_units(-40));
            }
            13 => assert!(visited.into_iter().all(|seen| seen)),
            _ => unreachable!(),
        }
    }
    println!(
        "verified 6 connected-sector scenarios, 144 C-derived tics: step/clearance thresholds, transitions, allowed drop, gravity, landing and continued movement"
    );
}
