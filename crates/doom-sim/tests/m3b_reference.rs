//! Original C-derived lookup, opening, destination and multi-tic XY/Z evidence.
#[path = "../../../tests/support/m3b.rs"]
mod fixtures;
use doom_sim::{ActorId, Event, SectorMoveAttempt, Simulation};
use doom_types::Fixed;
fn numbers(row: &str) -> Vec<i64> {
    row.split(',').map(|n| n.parse().unwrap()).collect()
}
fn label(id: ActorId, actor: ActorId, other: Option<ActorId>) -> i32 {
    if id == actor {
        0
    } else {
        assert_eq!(Some(id), other);
        1
    }
}
fn decision(a: &SectorMoveAttempt, actor: ActorId, other: Option<ActorId>) -> Vec<i32> {
    vec![
        a.movement.target[0].bits(),
        a.movement.target[1].bits(),
        i32::from(a.allowed()),
        a.floor.bits(),
        a.ceiling.bits(),
        a.dropoff.bits(),
        i32::from(a.float_ok),
        a.failure as i32,
        i32::from(a.location.subsector),
        i32::from(a.location.sector),
        a.movement.lines.blocked_by.map_or(-1, i32::from),
        a.movement
            .blocked_by_actor
            .map_or(-1, |id| label(id, actor, other)),
        a.movement.lines.candidates.len() as i32,
        a.movement.actors.len() as i32,
        a.openings.len() as i32,
    ]
}
#[test]
fn bsp_side_boundary_and_leaf_sector_lookup_matches_c() {
    let worlds: Vec<_> = (0..fixtures::map_count())
        .map(|i| Simulation::multisector(fixtures::map(i)).unwrap())
        .collect();
    for (row, line) in include_str!("../../../tests/reference/m3b_bsp.csv")
        .lines()
        .skip(1)
        .enumerate()
    {
        let n = numbers(line);
        let location = worlds[n[0] as usize]
            .world()
            .locate([fixtures::raw(n[1] as i32), fixtures::raw(n[2] as i32)])
            .unwrap();
        assert_eq!(
            [i64::from(location.subsector), i64::from(location.sector)],
            [n[3], n[4]],
            "C BSP row {}",
            row + 2
        );
    }
}
#[test]
fn every_two_sided_opening_matches_original_c() {
    let worlds: Vec<_> = (0..fixtures::map_count())
        .map(|i| Simulation::multisector(fixtures::map(i)).unwrap())
        .collect();
    for (row, line) in include_str!("../../../tests/reference/m3b_openings.csv")
        .lines()
        .skip(1)
        .enumerate()
    {
        let n = numbers(line);
        let open = worlds[n[0] as usize]
            .line_opening(n[1] as usize)
            .unwrap()
            .unwrap();
        assert_eq!(
            [open.top, open.bottom, open.range, open.low_floor].map(|f| i64::from(f.bits())),
            n[2..],
            "C opening row {}",
            row + 2
        );
    }
}
#[test]
fn all_step_dropoff_clearance_and_blocking_probes_match_c_without_mutation() {
    let cases = fixtures::cases();
    for (row, line) in include_str!("../../../tests/reference/m3b_probes.csv")
        .lines()
        .skip(1)
        .enumerate()
    {
        let n = numbers(line);
        let (sim, actor, other) = cases[n[0] as usize].setup(false);
        let before = sim.snapshot();
        let actual = sim
            .check_multisector_position(
                actor,
                [fixtures::raw(n[1] as i32), fixtures::raw(n[2] as i32)],
            )
            .unwrap();
        assert_eq!(
            decision(&actual, actor, other)
                .into_iter()
                .map(i64::from)
                .collect::<Vec<_>>(),
            n[1..],
            "C destination row {}: {line}",
            row + 2
        );
        assert_eq!(before, sim.snapshot());
    }
}
#[test]
fn all_multitic_xy_z_constraints_order_and_membership_match_c() {
    let cases = fixtures::cases();
    let (mut sim, mut actor, mut other) = cases[0].setup(true);
    for row in include_str!("../../../tests/reference/m3b_movement.csv")
        .lines()
        .skip(1)
    {
        let c: Vec<_> = row.split(',').collect();
        let n: Vec<i64> = c[..13].iter().map(|n| n.parse().unwrap()).collect();
        let case = &cases[n[0] as usize];
        let tic = n[1];
        if tic == 0 {
            (sim, actor, other) = case.setup(true);
        }
        sim.step(case.commands()).unwrap();
        let a = sim.actor(actor).unwrap();
        let loc = a.location().unwrap();
        let actual = vec![
            i64::from(a.position()[0].bits()),
            i64::from(a.position()[1].bits()),
            i64::from(a.z().bits()),
            i64::from(a.momentum()[0].bits()),
            i64::from(a.momentum()[1].bits()),
            i64::from(a.vertical_momentum().bits()),
            i64::from(a.constraints()[0].bits()),
            i64::from(a.constraints()[1].bits()),
            i64::from(loc.subsector),
            i64::from(loc.sector),
            i64::from(a.angle()),
        ];
        assert_eq!(
            actual,
            n[2..],
            "C case {} ({}) tic {tic}: first physical field mismatch",
            case.index,
            case.label
        );
        let attempts: Vec<_> = sim
            .events()
            .iter()
            .filter_map(|e| match e {
                Event::SectorMovement { actor: id, attempt } if *id == actor => Some(attempt),
                _ => None,
            })
            .collect();
        let expected: Vec<_> = if c[13] == "-" {
            Vec::new()
        } else {
            c[13].split(';').map(fixtures::integers).collect()
        };
        assert_eq!(
            attempts
                .iter()
                .map(|a| decision(a, actor, other))
                .collect::<Vec<_>>(),
            expected,
            "C case {} tic {tic}: attempted target/constraints/decision",
            case.index
        );
        let lines: Vec<_> = attempts
            .iter()
            .flat_map(|a| a.movement.lines.candidates.iter().map(|n| i32::from(*n)))
            .collect();
        assert_eq!(
            lines,
            fixtures::integers(c[14]),
            "C case {} tic {tic}: line order",
            case.index
        );
        let things: Vec<_> = attempts
            .iter()
            .flat_map(|a| &a.movement.actors)
            .map(|id| label(*id, actor, other))
            .collect();
        assert_eq!(
            things,
            fixtures::integers(c[15]),
            "C case {} tic {tic}: thing order",
            case.index
        );
        let slides: Vec<_> = sim
            .events()
            .iter()
            .filter_map(|e| match e {
                Event::Slide { actor: id, line } if *id == actor => Some(i32::from(*line)),
                _ => None,
            })
            .collect();
        assert_eq!(
            slides,
            fixtures::integers(c[16]),
            "C case {} tic {tic}: slide wall",
            case.index
        );
        let openings: Vec<_> = attempts
            .iter()
            .flat_map(|a| &a.openings)
            .map(|visit| {
                vec![
                    i32::from(visit.line),
                    visit.opening.top.bits(),
                    visit.opening.bottom.bits(),
                    visit.opening.range.bits(),
                    visit.opening.low_floor.bits(),
                ]
            })
            .collect();
        let expected: Vec<_> = if c[17] == "-" {
            Vec::new()
        } else {
            c[17].split(';').map(fixtures::integers).collect()
        };
        assert_eq!(
            openings, expected,
            "C case {} tic {tic}: opening accumulation",
            case.index
        );
        let members: Vec<Vec<i32>> = (0..sim.sectors().len())
            .map(|sector| {
                sim.sector_actor_order(sector as u16)
                    .unwrap()
                    .into_iter()
                    .map(|id| label(id, actor, other))
                    .collect()
            })
            .collect();
        assert_eq!(
            members,
            c[18].split('|').map(fixtures::integers).collect::<Vec<_>>(),
            "C case {} tic {tic}: sector-head order",
            case.index
        );
        let mut attempt_index = 0;
        let motion = sim
            .events()
            .iter()
            .filter_map(|e| match e {
                Event::SectorMovement { actor: id, .. } if *id == actor => {
                    let value = format!("T{attempt_index}");
                    attempt_index += 1;
                    Some(value)
                }
                Event::Slide { actor: id, line } if *id == actor => Some(format!("S{line}")),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            if motion.is_empty() {
                "-".to_owned()
            } else {
                motion.join(";")
            },
            c[20],
            "C case {} tic {tic}: chronological motion order",
            case.index
        );
        let vertical: Vec<_> = sim
            .events()
            .iter()
            .filter_map(|e| match e {
                Event::Vertical { actor: id, update } if *id == actor => {
                    Some(update.before.map(Fixed::bits).to_vec())
                }
                _ => None,
            })
            .collect();
        let expected = if c[19] == "-" {
            Vec::new()
        } else {
            vec![fixtures::integers(c[19])]
        };
        assert_eq!(
            vertical, expected,
            "C case {} tic {tic}: XY then guarded Z",
            case.index
        );
    }
}

#[test]
fn bsp_axis_reversals_diagonals_and_sign_fast_paths_match_c() {
    let mut worlds = Vec::new();
    for [dx, dy] in [
        [0, 128],
        [0, -128],
        [128, 0],
        [-128, 0],
        [128, 128],
        [128, -128],
        [-128, 128],
        [-128, -128],
    ] {
        let mut lumps = fixtures::lumps(0);
        fixtures::set_word(&mut lumps[7].1, 4, dx);
        fixtures::set_word(&mut lumps[7].1, 6, dy);
        worlds.push(Simulation::multisector(fixtures::decode(&lumps).unwrap()).unwrap());
    }
    for (i, row) in include_str!("../../../tests/reference/m3b_sides.csv")
        .lines()
        .skip(1)
        .enumerate()
    {
        let n = numbers(row);
        let location = worlds[i / 64]
            .world()
            .locate([fixtures::raw(n[2] as i32), fixtures::raw(n[3] as i32)])
            .unwrap();
        assert_eq!(
            i64::from(1 - location.subsector),
            n[4],
            "C BSP partition-side row {}",
            i + 2
        );
    }
}
