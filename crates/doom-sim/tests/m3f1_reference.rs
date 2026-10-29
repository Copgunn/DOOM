//! Original C-derived lookup, opening, destination and multi-tic XY/Z evidence.
#[path = "../../../tests/support/m3f1.rs"]
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
        .map(|i| Simulation::map_scale(fixtures::map(i)).unwrap())
        .collect();
    for (row, line) in include_str!("../../../tests/reference/m3f1_bsp.csv")
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
        .map(|i| Simulation::map_scale(fixtures::map(i)).unwrap())
        .collect();
    for (row, line) in include_str!("../../../tests/reference/m3f1_openings.csv")
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
    for (row, line) in include_str!("../../../tests/reference/m3f1_probes.csv")
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
    for row in include_str!("../../../tests/reference/m3f1_movement.csv")
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
        assert_eq!(sim.rng().indices()[0], c[22].parse::<u8>().unwrap());
        let thinker_order: Vec<_> = sim
            .events()
            .iter()
            .filter_map(|e| match e {
                Event::Think(id) => Some(label(*id, actor, other)),
                _ => None,
            })
            .collect();
        assert_eq!(thinker_order, fixtures::integers(c[21]), "C thinker order");
        if let Some(other_id) = other {
            let a = sim.actor(other_id).unwrap();
            let location = a.location().unwrap();
            let flags = u32::from(a.solidity() == doom_sim::Solidity::Solid) * 2
                + u32::from(a.gravity() == doom_sim::Gravity::None) * 512
                + u32::from(a.allows_dropoff()) * 1024;
            let physical = vec![
                a.position()[0].bits(),
                a.position()[1].bits(),
                a.z().bits(),
                a.momentum()[0].bits(),
                a.momentum()[1].bits(),
                a.vertical_momentum().bits(),
                a.constraints()[0].bits(),
                a.constraints()[1].bits(),
                i32::from(location.subsector),
                i32::from(location.sector),
                a.angle() as i32,
                i32::from(a.lastlook()),
                a.radius().bits(),
                a.height().bits(),
                flags as i32,
            ];
            assert_eq!(
                physical,
                fixtures::integers(c[27]),
                "C other actor complete physical fields"
            );
        } else {
            assert_eq!(c[27], "-");
        }
        let a = sim.actor(actor).unwrap();
        assert_eq!(a.lastlook(), c[23].parse::<u8>().unwrap());
        assert_eq!(a.radius().bits(), c[24].parse::<i32>().unwrap());
        assert_eq!(a.height().bits(), c[25].parse::<i32>().unwrap());
        let flags = u32::from(a.solidity() == doom_sim::Solidity::Solid) * 2
            + u32::from(a.gravity() == doom_sim::Gravity::None) * 512
            + u32::from(a.allows_dropoff()) * 1024;
        assert_eq!(flags, c[26].parse::<u32>().unwrap());
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
fn expanded_reject_bsp_and_ordered_seg_visits_match_original_c() {
    use doom_sim::{ActorSpec, SightVisit, Solidity};
    for (line_number, row) in include_str!("../../../tests/reference/m3f1_sight.csv")
        .lines()
        .skip(1)
        .enumerate()
    {
        let c: Vec<_> = row.split(',').collect();
        let n: Vec<i64> = c[..9].iter().map(|s| s.parse().unwrap()).collect();
        let mut sim = Simulation::map_scale(fixtures::map(n[0] as usize)).unwrap();
        let mut spawn = |x| {
            sim.spawn_with_solidity(
                ActorSpec {
                    position: [Fixed::from_bits(x), Fixed::ZERO],
                    height: Fixed::from_map_units(16),
                    ..Default::default()
                },
                Solidity::Solid,
            )
            .unwrap()
        };
        let observer = spawn(n[1] as i32);
        let target = spawn(n[2] as i32);
        let before = sim.snapshot();
        let result = sim.check_sight(observer, target).unwrap();
        assert_eq!(
            [
                i64::from(result.reject_index),
                i64::from(result.rejected),
                i64::from(result.visible),
                i64::from(result.eye.unwrap_or(Fixed::ZERO).bits()),
                i64::from(result.slopes.unwrap_or([Fixed::ZERO; 2])[0].bits()),
                i64::from(result.slopes.unwrap_or([Fixed::ZERO; 2])[1].bits())
            ],
            n[3..],
            "C sight row{}",
            line_number + 2
        );
        let entries: Vec<_> = result
            .visits
            .iter()
            .filter_map(|v| match v {
                SightVisit::Seg {
                    line, duplicate, ..
                } => Some(i32::from(*line) * 2 + i32::from(*duplicate)),
                _ => None,
            })
            .collect();
        assert_eq!(
            entries,
            fixtures::integers(c[9]),
            "C sight seg order row{}",
            line_number + 2
        );
        assert_eq!(sim.rng().indices()[0], c[10].parse::<u8>().unwrap());
        assert_eq!(before, sim.snapshot());
    }
}

#[path = "../../../tests/reference/m3f1_states.rs"]
mod independent;
#[test]
fn independent_dsim_v18_digests_and_complete_bytes_match() {
    let cases = fixtures::cases();
    let (mut sim, _, _) = cases[0].setup(false);
    for row in include_str!("../../../tests/reference/m3f1_snapshots.csv")
        .lines()
        .skip(1)
    {
        let c: Vec<_> = row.split(',').collect();
        let case = c[0].parse::<usize>().unwrap();
        let tic = c[1].parse::<usize>().unwrap();
        if tic == 1 {
            sim = cases[case].setup(false).0;
        }
        let snapshot = sim.step(cases[case].commands()).unwrap();
        if c[3] != "-" {
            let expected = independent::STATES
                .iter()
                .find(|&&(a, b, _)| a == case && b == tic)
                .unwrap()
                .2;
            fixtures::assert_canonical(expected, &snapshot);
        }
        assert_eq!(
            snapshot.digest(),
            u64::from_str_radix(c[2], 16).unwrap(),
            "C-derived DSIM-v18 case{case} tic{tic}"
        );
    }
}
