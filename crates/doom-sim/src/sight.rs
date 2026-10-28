//! Source P_CheckSight traversal; separate from BLOCKMAP combat tracing.
use crate::*;
use doom_map::{BspChild, Map};
pub(crate) const WORK_LIMIT: usize = 8192;
/// Ordered local observations from original BSP/seg sight traversal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SightVisit {
    /// Start-side classification, before the first child; raw 2 normalizes to 0.
    NodeStart {
        /// Source node.
        node: u16,
        /// Raw source side.
        side: u8,
    },
    /// Target side evaluated only after the starting child passed.
    NodeEnd {
        /// Source node.
        node: u16,
        /// Raw target side.
        side: u8,
    },
    /// Enter a source leaf, including repeated leaves in valid bounded DAGs.
    Subsector(u16),
    /// Seg visit before line duplicate suppression.
    Seg {
        /// Leaf containing this seg.
        subsector: u16,
        /// Seg index.
        seg: u32,
        /// Parent linedef.
        line: u16,
        /// Already marked by an earlier seg in this query.
        duplicate: bool,
    },
    /// Two side tests: stage 0=line endpoints, stage 1=observer/target.
    Sides {
        /// Parent linedef.
        line: u16,
        /// Source test stage.
        stage: u8,
        /// Source three-way classifications.
        sides: [u8; 2],
    },
    /// Non-identical-height two-sided opening, before closed-door rejection.
    Opening {
        /// Parent linedef.
        line: u16,
        /// Raw fixed opening ceiling/floor.
        heights: [Fixed; 2],
    },
    /// Intercept and resulting bottom/top slope interval.
    Clip {
        /// Parent linedef.
        line: u16,
        /// Source fixed intercept fraction.
        fraction: Fixed,
        /// Updated bottom/top slopes.
        slopes: [Fixed; 2],
    },
    /// Crossing blocked by 0=one sided, 1=closed opening, 2=closed slope interval.
    Blocked {
        /// Parent linedef.
        line: u16,
        /// Source rejection branch.
        reason: u8,
    },
}
/// Pure sight result. Rejected queries expose no stale source-global geometry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SightResult {
    /// Source final decision.
    pub visible: bool,
    /// Observer-sector * sector-count + target-sector.
    pub reject_index: u32,
    /// Whether REJECT returned before traversal.
    pub rejected: bool,
    /// Original eye height; absent on an early REJECT.
    pub eye: Option<Fixed>,
    /// Final bottom/top slope interval; absent on an early REJECT.
    pub slopes: Option<[Fixed; 2]>,
    /// Source traversal/decision order. No persistent stamps or counters.
    pub visits: Vec<SightVisit>,
}
#[derive(Clone, Copy)]
pub(crate) struct Divline {
    pub start: [Fixed; 2],
    pub delta: [Fixed; 2],
}
pub(crate) fn side(p: [Fixed; 2], d: Divline) -> u8 {
    if d.delta[0] == Fixed::ZERO {
        if p[0] == d.start[0] {
            return 2;
        }
        return u8::from(if p[0] <= d.start[0] {
            d.delta[1] > Fixed::ZERO
        } else {
            d.delta[1] < Fixed::ZERO
        });
    }
    if d.delta[1] == Fixed::ZERO {
        // Preserve Linux 1.10's x==node->y test, including its surprising ties.
        if p[0] == d.start[1] {
            return 2;
        }
        return u8::from(if p[1] <= d.start[1] {
            d.delta[0] < Fixed::ZERO
        } else {
            d.delta[0] > Fixed::ZERO
        });
    }
    let dx = (p[0] - d.start[0]).bits() >> 16;
    let dy = (p[1] - d.start[1]).bits() >> 16;
    let left = (d.delta[1].bits() >> 16) * dx;
    let right = dy * (d.delta[0].bits() >> 16);
    if right < left {
        0
    } else if left == right {
        2
    } else {
        1
    }
}
pub(crate) fn fraction(v2: Divline, v1: Divline) -> Fixed {
    let shift = |v: Fixed| Fixed::from_bits(v.bits() >> 8);
    let den = shift(v1.delta[1]) * v2.delta[0] - shift(v1.delta[0]) * v2.delta[1];
    if den == Fixed::ZERO {
        return Fixed::ZERO;
    }
    let num = shift(v1.start[0] - v2.start[0]) * v1.delta[1]
        + shift(v2.start[1] - v1.start[1]) * v1.delta[0];
    num / den
}
pub(crate) fn within(v: Fixed, units: u32) -> bool {
    v.bits().unsigned_abs() <= units * 65536
}
pub(crate) fn actor_bounds(a: &Actor) -> Result<(), SimError> {
    if a.position()
        .into_iter()
        .chain([a.z])
        .any(|v| !within(v, 1024))
    {
        return Err(PerceptionError::Arithmetic.into());
    }
    Ok(())
}
/// Bound the fully expanded source traversal, including repeated children/segs.
pub(crate) fn validate_map(map: &Map) -> Result<(), SimError> {
    if map
        .vertices()
        .iter()
        .any(|v| v.x.unsigned_abs() > 512 || v.y.unsigned_abs() > 512)
        || map.nodes().iter().any(|n| {
            [n.origin.x, n.origin.y, n.delta.x, n.delta.y]
                .iter()
                .any(|v| v.unsigned_abs() > 512)
        })
        || map
            .sectors()
            .iter()
            .any(|s| s.heights.iter().any(|h| h.unsigned_abs() > 512))
    {
        return Err(PerceptionError::Arithmetic.into());
    }
    let leaf = |i: u16| 1 + usize::from(map.subsectors()[usize::from(i)].seg_count);
    if map.nodes().is_empty() {
        if leaf(0) > WORK_LIMIT {
            return Err(PerceptionError::WorkLimit.into());
        }
        return Ok(());
    }
    let mut costs = vec![None; map.nodes().len()];
    let mut stack = vec![(map.nodes().len() - 1, false)];
    while let Some((i, done)) = stack.pop() {
        if costs[i].is_some() {
            continue;
        }
        let node = &map.nodes()[i];
        if !done {
            stack.push((i, true));
            for c in node.children {
                if let BspChild::Node(n) = c {
                    stack.push((usize::from(n), false));
                }
            }
        } else {
            let mut cost = 2_usize;
            for c in node.children {
                let n = match c {
                    BspChild::Node(n) => costs[usize::from(n)].unwrap(),
                    BspChild::Subsector(n) => leaf(n),
                };
                cost = cost
                    .checked_add(n)
                    .filter(|v| *v <= WORK_LIMIT)
                    .ok_or(PerceptionError::WorkLimit)?;
            }
            costs[i] = Some(cost);
        }
    }
    Ok(())
}
#[derive(Clone, Copy)]
enum Task {
    Child(BspChild),
    After(u16, u8),
}
fn node_line(map: &Map, i: u16) -> Divline {
    let n = &map.nodes()[usize::from(i)];
    Divline {
        start: [
            Fixed::from_map_units(n.origin.x),
            Fixed::from_map_units(n.origin.y),
        ],
        delta: [
            Fixed::from_map_units(n.delta.x),
            Fixed::from_map_units(n.delta.y),
        ],
    }
}
pub(crate) fn query(
    world: &StaticWorld,
    sectors: &[SectorState],
    observer: Actor,
    target: Actor,
) -> SightResult {
    let map = world.map();
    let pair = usize::from(observer.location.unwrap().sector) * sectors.len()
        + usize::from(target.location.unwrap().sector);
    let rejected = map.reject()[pair >> 3] & (1 << (pair & 7)) != 0;
    let mut out = SightResult {
        visible: !rejected,
        reject_index: pair as u32,
        rejected,
        eye: None,
        slopes: None,
        visits: Vec::new(),
    };
    if rejected {
        return out;
    }
    let eye = observer.z + observer.height() - Fixed::from_bits(observer.height().bits() >> 2);
    let mut slopes = [target.z - eye, target.z + target.height() - eye];
    out.eye = Some(eye);
    let ray = Divline {
        start: observer.position(),
        delta: [
            target.position()[0] - observer.position()[0],
            target.position()[1] - observer.position()[1],
        ],
    };
    let root = if map.nodes().is_empty() {
        BspChild::Subsector(0)
    } else {
        BspChild::Node((map.nodes().len() - 1) as u16)
    };
    let mut tasks = vec![Task::Child(root)];
    let mut seen = 0_u128;
    let mut work = 0;
    'walk: while let Some(task) = tasks.pop() {
        work += 1;
        assert!(work <= WORK_LIMIT, "admitted expanded sight work");
        match task {
            Task::After(i, start) => {
                let end = side(target.position(), node_line(map, i));
                out.visits.push(SightVisit::NodeEnd { node: i, side: end });
                if start != end {
                    tasks.push(Task::Child(
                        map.nodes()[usize::from(i)].children[usize::from(start ^ 1)],
                    ));
                }
            }
            Task::Child(BspChild::Node(i)) => {
                let raw = side(observer.position(), node_line(map, i));
                let start = if raw == 2 { 0 } else { raw };
                out.visits
                    .push(SightVisit::NodeStart { node: i, side: raw });
                tasks.push(Task::After(i, start));
                tasks.push(Task::Child(
                    map.nodes()[usize::from(i)].children[usize::from(start)],
                ));
            }
            Task::Child(BspChild::Subsector(i)) => {
                out.visits.push(SightVisit::Subsector(i));
                let leaf = &map.subsectors()[usize::from(i)];
                for seg_index in usize::from(leaf.first_seg)
                    ..usize::from(leaf.first_seg) + usize::from(leaf.seg_count)
                {
                    work += 1;
                    assert!(work <= WORK_LIMIT, "admitted seg work");
                    let seg = &map.segs()[seg_index];
                    let line_id = seg.linedef;
                    let duplicate = seen & (1_u128 << line_id) != 0;
                    out.visits.push(SightVisit::Seg {
                        subsector: i,
                        seg: seg_index as u32,
                        line: line_id,
                        duplicate,
                    });
                    if duplicate {
                        continue;
                    }
                    seen |= 1_u128 << line_id;
                    let line = &map.linedefs()[usize::from(line_id)];
                    let p = line.vertices.map(|id| {
                        let v = map.vertices()[usize::from(id)];
                        [Fixed::from_map_units(v.x), Fixed::from_map_units(v.y)]
                    });
                    let sides = p.map(|p| side(p, ray));
                    out.visits.push(SightVisit::Sides {
                        line: line_id,
                        stage: 0,
                        sides,
                    });
                    if sides[0] == sides[1] {
                        continue;
                    }
                    let div = Divline {
                        start: p[0],
                        delta: [p[1][0] - p[0][0], p[1][1] - p[0][1]],
                    };
                    let sides = [side(observer.position(), div), side(target.position(), div)];
                    out.visits.push(SightVisit::Sides {
                        line: line_id,
                        stage: 1,
                        sides,
                    });
                    if sides[0] == sides[1] {
                        continue;
                    }
                    if line.flags & 4 == 0 {
                        out.visits.push(SightVisit::Blocked {
                            line: line_id,
                            reason: 0,
                        });
                        out.visible = false;
                        break 'walk;
                    }
                    let [front, back] = [seg.side, seg.side ^ 1].map(|s| {
                        let side = line.sides[usize::from(s)].unwrap();
                        sectors[usize::from(map.sidedefs()[usize::from(side)].sector)]
                    });
                    if front == back {
                        continue;
                    }
                    let top = front.ceiling.min(back.ceiling);
                    let bottom = front.floor.max(back.floor);
                    out.visits.push(SightVisit::Opening {
                        line: line_id,
                        heights: [top, bottom],
                    });
                    if bottom >= top {
                        out.visits.push(SightVisit::Blocked {
                            line: line_id,
                            reason: 1,
                        });
                        out.visible = false;
                        break 'walk;
                    }
                    let frac = fraction(ray, div);
                    if front.floor != back.floor {
                        slopes[0] = slopes[0].max((bottom - eye) / frac);
                    }
                    if front.ceiling != back.ceiling {
                        slopes[1] = slopes[1].min((top - eye) / frac);
                    }
                    out.visits.push(SightVisit::Clip {
                        line: line_id,
                        fraction: frac,
                        slopes,
                    });
                    if slopes[1] <= slopes[0] {
                        out.visits.push(SightVisit::Blocked {
                            line: line_id,
                            reason: 2,
                        });
                        out.visible = false;
                        break 'walk;
                    }
                }
            }
        }
    }
    out.slopes = Some(slopes);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn f(v: i64) -> Fixed {
        Fixed::from_bits(v as i32)
    }
    #[test]
    fn source_three_way_sides_including_horizontal_quirk() {
        let mut count = 0;
        for line in include_str!("../../../tests/reference/m3d3a_sides.csv")
            .lines()
            .skip(1)
        {
            let n = line
                .split(',')
                .map(|s| s.parse::<i64>().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                i64::from(side(
                    [f(n[4]), f(n[5])],
                    Divline {
                        start: [f(n[0]), f(n[1])],
                        delta: [f(n[2]), f(n[3])]
                    }
                )),
                n[6],
                "source divline row {count}"
            );
            count += 1;
        }
        assert_eq!(count, 1536);
    }
    #[test]
    fn source_intercept_vector2_fractions() {
        let mut count = 0;
        for line in include_str!("../../../tests/reference/m3d3a_fractions.csv")
            .lines()
            .skip(1)
        {
            let n = line
                .split(',')
                .map(|s| s.parse::<i64>().unwrap())
                .collect::<Vec<_>>();
            let a = Divline {
                start: [f(n[0]), f(n[1])],
                delta: [f(n[2]), f(n[3])],
            };
            let b = Divline {
                start: [f(n[4]), f(n[5])],
                delta: [f(n[6]), f(n[7])],
            };
            assert_eq!(
                i64::from(fraction(a, b).bits()),
                n[8],
                "source intercept row {count}"
            );
            count += 1;
        }
        assert_eq!(count, 131);
    }
}
