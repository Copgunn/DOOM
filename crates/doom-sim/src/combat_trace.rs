//! Bounded source BLOCKMAP/intercept tracing; no gameplay mutation.
use crate::{ActorId, CombatError, Fixed, Simulation};
/// A stable line or generation-aware actor identity in a trace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceObject {
    /// Immutable linedef index.
    Line(u16),
    /// Actor identity; never an allocator address.
    Actor(ActorId),
}
/// One source divline intercept, in stable fraction order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Intercept {
    /// Signed 16.16 fraction along the nudged trace.
    pub fraction: Fixed,
    /// Source line or thing.
    pub object: TraceObject,
}
/// Read-only collection evidence, distinct from traverser stopping decisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HitscanTrace {
    /// Source start after exact-block-boundary nudging.
    pub origin: [Fixed; 2],
    /// Original endpoint minus the nudged origin.
    pub delta: [Fixed; 2],
    /// Source DDA cell sequence, including out-of-grid cells.
    pub cells: Vec<[i32; 2]>,
    /// Candidate callback order, lines before things in each cell.
    pub candidates: Vec<TraceObject>,
    /// Collected nonnegative fractions, stable on ties; may exceed one.
    pub intercepts: Vec<Intercept>,
}
/// Original aim window/target result; querying it consumes no RNG or events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AimResult {
    /// Complete collection evidence.
    pub trace: HitscanTrace,
    /// First applicable shootable actor.
    pub target: Option<ActorId>,
    /// Zero when no target, otherwise source clipped-span midpoint.
    pub slope: Fixed,
    /// Final bottom/top window after line openings.
    pub window: [Fixed; 2],
    /// Stopping line or selected target, if any.
    pub stopped: Option<TraceObject>,
    /// Number of in-range intercept callbacks actually evaluated.
    pub traversed: u32,
}
#[derive(Clone, Copy)]
struct Div {
    start: [Fixed; 2],
    delta: [Fixed; 2],
}
fn shift(v: Fixed, bits: u32) -> Fixed {
    Fixed::from_bits(v.bits() >> bits)
}
fn side(d: Div, p: [Fixed; 2]) -> u8 {
    let [dx, dy] = d.delta;
    if dx == Fixed::ZERO {
        return u8::from(if p[0] <= d.start[0] {
            dy > Fixed::ZERO
        } else {
            dy < Fixed::ZERO
        });
    }
    if dy == Fixed::ZERO {
        return u8::from(if p[1] <= d.start[1] {
            dx < Fixed::ZERO
        } else {
            dx > Fixed::ZERO
        });
    }
    let x = p[0] - d.start[0];
    let y = p[1] - d.start[1];
    if (dy.bits() ^ dx.bits() ^ x.bits() ^ y.bits()) < 0 {
        return u8::from((dy.bits() ^ x.bits()) < 0);
    }
    u8::from(shift(y, 8) * shift(dx, 8) >= shift(dy, 8) * shift(x, 8))
}
fn fraction(trace: Div, line: Div) -> Fixed {
    let [dx, dy] = line.delta;
    let den = shift(dy, 8) * trace.delta[0] - shift(dx, 8) * trace.delta[1];
    if den == Fixed::ZERO {
        return Fixed::ZERO;
    }
    let num = shift(line.start[0] - trace.start[0], 8) * dy
        + shift(trace.start[1] - line.start[1], 8) * dx;
    num / den
}
fn checked_add(a: Fixed, b: Fixed) -> Result<Fixed, CombatError> {
    a.bits()
        .checked_add(b.bits())
        .map(Fixed::from_bits)
        .ok_or(CombatError::Coordinates)
}
fn checked_sub(a: Fixed, b: Fixed) -> Result<Fixed, CombatError> {
    a.bits()
        .checked_sub(b.bits())
        .map(Fixed::from_bits)
        .ok_or(CombatError::Coordinates)
}
pub(crate) fn validate_range(range: Fixed) -> Result<(), CombatError> {
    if range <= Fixed::ZERO || range > Fixed::from_map_units(2048) {
        Err(CombatError::Range)
    } else {
        Ok(())
    }
}
pub(crate) fn collect(
    sim: &Simulation,
    shooter: ActorId,
    angle: u32,
    range: Fixed,
) -> Result<HitscanTrace, CombatError> {
    validate_range(range)?;
    let actor = sim.actor(shooter).ok_or(CombatError::Reference)?;
    let mut start = actor.position();
    let phase = (angle >> 19) as usize;
    let units = range.bits() >> 16;
    let end = [
        checked_add(
            start[0],
            Fixed::from_bits(units * crate::FINE_SINE[phase + 2048]),
        )?,
        checked_add(start[1], Fixed::from_bits(units * crate::FINE_SINE[phase]))?,
    ];
    let origin = sim.world.map().blockmap().origin();
    let origin = [
        Fixed::from_map_units(origin.x),
        Fixed::from_map_units(origin.y),
    ];
    for axis in 0..2 {
        if checked_sub(start[axis], origin[axis])?.bits() & ((128 << 16) - 1) == 0 {
            start[axis] = checked_add(start[axis], Fixed::ONE)?;
        }
    }
    let trace = Div {
        start,
        delta: [
            checked_sub(end[0], start[0])?,
            checked_sub(end[1], start[1])?,
        ],
    };
    let a = [
        checked_sub(start[0], origin[0])?,
        checked_sub(start[1], origin[1])?,
    ];
    let b = [
        checked_sub(end[0], origin[0])?,
        checked_sub(end[1], origin[1])?,
    ];
    let first = a.map(|v| v.bits() >> 23);
    let last = b.map(|v| v.bits() >> 23);
    let mut cell = first;
    let mut steps = [0; 2];
    let mut increments = [Fixed::ZERO; 2];
    let mut crossings = [Fixed::ZERO; 2];
    for axis in 0..2 {
        let other = 1 - axis;
        let partial;
        let frac = Fixed::from_bits((a[axis].bits() >> 7) & 65535);
        if first[axis] != last[axis] {
            steps[axis] = if last[axis] > first[axis] { 1 } else { -1 };
            partial = if steps[axis] > 0 {
                Fixed::ONE - frac
            } else {
                frac
            };
            let distance = checked_sub(b[axis], a[axis])?
                .bits()
                .checked_abs()
                .ok_or(CombatError::Coordinates)?;
            increments[axis] = checked_sub(b[other], a[other])? / Fixed::from_bits(distance);
        } else {
            partial = Fixed::ONE;
            increments[axis] = Fixed::from_bits(256 << 16);
        }
        crossings[axis] = shift(a[other], 7) + partial * increments[axis];
    }
    let mut out = HitscanTrace {
        origin: start,
        delta: trace.delta,
        cells: Vec::new(),
        candidates: Vec::new(),
        intercepts: Vec::new(),
    };
    let mut seen = [false; 128];
    let [width, height] = sim.world.map().blockmap().dimensions().map(i32::from);
    let mut complete = false;
    for _ in 0..64 {
        out.cells.push(cell);
        if cell[0] >= 0 && cell[0] < width && cell[1] >= 0 && cell[1] < height {
            for line_id in sim
                .world
                .line_candidates(cell[0] as usize, cell[1] as usize)
            {
                let i = usize::from(line_id);
                if seen[i] {
                    continue;
                }
                seen[i] = true;
                let object = TraceObject::Line(line_id);
                out.candidates.push(object);
                let line = &sim.world.lines[i];
                let other = [line.start[0] + line.delta[0], line.start[1] + line.delta[1]];
                let sides = if trace
                    .delta
                    .iter()
                    .any(|v| v.bits() > 16 * 65536 || v.bits() < -16 * 65536)
                {
                    [side(trace, line.start), side(trace, other)]
                } else {
                    [line.point_side(start), line.point_side(end)]
                };
                if sides[0] != sides[1] {
                    add(
                        &mut out,
                        fraction(
                            trace,
                            Div {
                                start: line.start,
                                delta: line.delta,
                            },
                        ),
                        object,
                    )?;
                }
            }
            for id in &sim.spatial.order {
                let actor = sim.actor(*id).expect("linked live actor");
                if !crate::lifecycle::blocks_cell_index(actor) {
                    continue;
                }
                let position = actor.position();
                let here = [
                    (position[0] - origin[0]).bits() >> 23,
                    (position[1] - origin[1]).bits() >> 23,
                ];
                if here != cell {
                    continue;
                }
                let object = TraceObject::Actor(*id);
                out.candidates.push(object);
                if out.candidates.len() > 64 * (128 + 1024) {
                    return Err(CombatError::WorkLimit);
                }
                let r = actor.radius();
                let low = [checked_sub(position[0], r)?, checked_sub(position[1], r)?];
                let high = [checked_add(position[0], r)?, checked_add(position[1], r)?];
                let (p, q) = if (trace.delta[0].bits() ^ trace.delta[1].bits()) > 0 {
                    ([low[0], high[1]], [high[0], low[1]])
                } else {
                    (low, high)
                };
                if side(trace, p) != side(trace, q) {
                    add(
                        &mut out,
                        fraction(
                            trace,
                            Div {
                                start: p,
                                delta: [q[0] - p[0], q[1] - p[1]],
                            },
                        ),
                        object,
                    )?;
                }
            }
        }
        if cell == last {
            complete = true;
            break;
        }
        if crossings[0].bits() >> 16 == cell[1] {
            crossings[0] = crossings[0] + increments[0];
            cell[0] += steps[0];
        } else if crossings[1].bits() >> 16 == cell[0] {
            crossings[1] = crossings[1] + increments[1];
            cell[1] += steps[1];
        }
    }
    if !complete {
        return Err(CombatError::TraversalLimit);
    }
    out.intercepts.sort_by_key(|i| i.fraction);
    Ok(out)
}
fn add(out: &mut HitscanTrace, frac: Fixed, object: TraceObject) -> Result<(), CombatError> {
    if frac >= Fixed::ZERO {
        if out.intercepts.len() == 128 {
            return Err(CombatError::InterceptLimit);
        }
        out.intercepts.push(Intercept {
            fraction: frac,
            object,
        });
    }
    Ok(())
}
pub(crate) fn aim(
    sim: &Simulation,
    shooter: ActorId,
    angle: u32,
    range: Fixed,
) -> Result<AimResult, CombatError> {
    let trace = collect(sim, shooter, angle, range)?;
    let a = sim.actor(shooter).unwrap();
    let z = a.z() + Fixed::from_bits(a.height().bits() >> 1) + Fixed::from_map_units(8);
    let mut result = AimResult {
        trace,
        target: None,
        slope: Fixed::ZERO,
        window: [Fixed::from_bits(-40960), Fixed::from_bits(40960)],
        stopped: None,
        traversed: 0,
    };
    for intercept in &result.trace.intercepts {
        if intercept.fraction > Fixed::ONE {
            break;
        }
        result.traversed += 1;
        let dist = range * intercept.fraction;
        match intercept.object {
            TraceObject::Line(index) => {
                let open =
                    crate::multisector::opening(&sim.world, &sim.sectors, usize::from(index));
                let mut blocked = true;
                if let Some(open) = open {
                    if open.bottom < open.top {
                        let [front, back] = line_sectors(sim, index);
                        if front.floor != back.floor {
                            result.window[0] = result.window[0].max((open.bottom - z) / dist);
                        }
                        if front.ceiling != back.ceiling {
                            result.window[1] = result.window[1].min((open.top - z) / dist);
                        }
                        blocked = result.window[1] <= result.window[0];
                    }
                }
                if blocked {
                    result.stopped = Some(intercept.object);
                    break;
                }
            }
            TraceObject::Actor(id) => {
                if id == shooter {
                    continue;
                }
                let actor = sim.actor(id).unwrap();
                if actor.life().unwrap().flags & 4 == 0 {
                    continue;
                }
                let top = (actor.z() + actor.height() - z) / dist;
                let bottom = (actor.z() - z) / dist;
                if top < result.window[0] || bottom > result.window[1] {
                    continue;
                }
                let sum = top.min(result.window[1]) + bottom.max(result.window[0]);
                result.slope = Fixed::from_bits(sum.bits() / 2);
                result.target = Some(id);
                result.stopped = Some(intercept.object);
                break;
            }
        }
    }
    Ok(result)
}
pub(crate) fn hit(
    sim: &Simulation,
    shooter: ActorId,
    trace: &HitscanTrace,
    range: Fixed,
    slope: Fixed,
) -> (Option<TraceObject>, Option<[Fixed; 3]>, u32) {
    let a = sim.actor(shooter).unwrap();
    let z = a.z() + Fixed::from_bits(a.height().bits() >> 1) + Fixed::from_map_units(8);
    let mut visited = 0;
    for intercept in &trace.intercepts {
        if intercept.fraction > Fixed::ONE {
            break;
        }
        visited += 1;
        let dist = range * intercept.fraction;
        let hit = match intercept.object {
            TraceObject::Line(index) => {
                match crate::multisector::opening(&sim.world, &sim.sectors, usize::from(index)) {
                    None => true,
                    Some(open) => {
                        let [f, b] = line_sectors(sim, index);
                        f.floor != b.floor && (open.bottom - z) / dist > slope
                            || f.ceiling != b.ceiling && (open.top - z) / dist < slope
                    }
                }
            }
            TraceObject::Actor(id) => {
                if id == shooter {
                    false
                } else {
                    let a = sim.actor(id).unwrap();
                    a.life().unwrap().flags & 4 != 0
                        && (a.z() + a.height() - z) / dist >= slope
                        && (a.z() - z) / dist <= slope
                }
            }
        };
        if hit {
            let back = Fixed::from_map_units(if matches!(intercept.object, TraceObject::Line(_)) {
                4
            } else {
                10
            }) / range;
            let frac = intercept.fraction - back;
            let point = [
                trace.origin[0] + trace.delta[0] * frac,
                trace.origin[1] + trace.delta[1] * frac,
                z + slope * (frac * range),
            ];
            return (Some(intercept.object), Some(point), visited);
        }
    }
    (None, None, visited)
}

fn line_sectors(sim: &Simulation, index: u16) -> [crate::SectorState; 2] {
    sim.world.map().linedefs()[usize::from(index)]
        .sides
        .map(|side| {
            sim.sectors[usize::from(sim.world.map().sidedefs()[usize::from(side.unwrap())].sector)]
        })
}
