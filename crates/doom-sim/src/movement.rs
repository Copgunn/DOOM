//! Bounded, source-derived grounded player and solid-actor XY profile.
use crate::{
    Actor, ActorId, Collision, GeometryError, LineGeometry, StaticWorld, StoreError, TicCommand,
};
use doom_types::Fixed;
include!("tan_to_angle.rs");

/// Collision participation supported independently of player input binding.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Solidity {
    /// Flags zero: does not block others, but still collides with solid actors.
    #[default]
    NonSolid,
    /// MF_SOLID only; no pickup, missile, shootable or Z-overlap mechanics.
    Solid,
}

/// One grounded movement attempt, including things-before-lines observations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorMoveAttempt {
    /// Requested destination.
    pub target: [Fixed; 2],
    /// Static fit and line candidates; empty if an actor blocked first.
    pub lines: Collision,
    /// Cell traversal and cell-head order, including non-solid candidates and self.
    pub actors: Vec<ActorId>,
    /// First strictly overlapping solid candidate, excluding self.
    pub blocked_by_actor: Option<ActorId>,
}
impl ActorMoveAttempt {
    /// Whether the entire destination check succeeded.
    pub const fn allowed(&self) -> bool {
        self.blocked_by_actor.is_none() && self.lines.allowed()
    }
}

/// Most-recent-link order. Filtering by center cell equals per-cell head lists.
#[derive(Clone, Debug, Default)]
pub(crate) struct SpatialIndex {
    pub order: Vec<ActorId>,
}
impl SpatialIndex {
    pub fn reserve_spawn(&mut self) -> Result<(), StoreError> {
        self.order.try_reserve(1).map_err(|_| StoreError::Capacity)
    }
    pub fn unlink(&mut self, id: ActorId) {
        self.order.retain(|entry| *entry != id);
    }
    pub fn link(&mut self, id: ActorId) {
        self.unlink(id);
        // Spawn reserves before state changes; relinking never grows active count.
        self.order.insert(0, id);
    }
}

pub(crate) fn validate_profile(world: &mut StaticWorld) -> Result<(), GeometryError> {
    prepare_cells(world, true, 128, 128 * 4096, 4096)
}
pub(crate) fn prepare_reference_profile(world: &mut StaticWorld) -> Result<(), GeometryError> {
    prepare_cells(world, false, 128, 128 * 4096, 4096)
}
pub(crate) fn prepare_map_scale(
    world: &mut StaticWorld,
    budget: usize,
) -> Result<(), GeometryError> {
    prepare_cells(world, false, crate::line_work::MAX_LINES, budget, 4096)
}
pub(crate) fn prepare_map_extent(
    world: &mut StaticWorld,
    budget: usize,
) -> Result<(), GeometryError> {
    prepare_cells(
        world,
        false,
        crate::line_work::MAX_LINES,
        budget,
        crate::MAP_EXTENT_MAX_CELLS,
    )
}
fn prepare_cells(
    world: &mut StaticWorld,
    conservative: bool,
    max_lines: usize,
    budget: usize,
    max_cells: usize,
) -> Result<(), GeometryError> {
    let map = world.map();
    let [w, h] = map.blockmap().dimensions().map(usize::from);
    if world.lines.len() > max_lines || w * h > max_cells || map.blockmap().words().len() > 65536 {
        return Err(GeometryError::Unsupported(if conservative {
            "M3a geometry work limit"
        } else {
            "M3b geometry work limit"
        }));
    }
    let origin = map.blockmap().origin();
    if conservative {
        for v in map.vertices() {
            if i64::from(v.x) < i64::from(origin.x)
                || i64::from(v.y) < i64::from(origin.y)
                || i64::from(v.x) >= i64::from(origin.x) + w as i64 * 128
                || i64::from(v.y) >= i64::from(origin.y) + h as i64 * 128
            {
                return Err(GeometryError::Unsupported("M3a vertices outside BLOCKMAP"));
            }
        }
    }
    // Compile first-occurrence order for each requested suffix in bounded work.
    // Shared/overlapping long lists must not multiply scans of raw words.
    let mut starts = crate::line_work::reserved(w * h)?;
    for (cell, range) in map.blockmap().cell_ranges().iter().enumerate() {
        starts.push((range.start, cell));
    }
    starts.sort_unstable(); // Preparation lookup only; never candidate traversal.
    let mut cells = crate::line_work::reserved(w * h)?;
    cells.resize_with(w * h, Vec::new);
    let mut next = crate::line_work::reserved(world.lines.len())?;
    next.resize(world.lines.len(), usize::MAX);
    let mut references = 0_usize;
    for (position, word) in map.blockmap().words().iter().copied().enumerate().rev() {
        if word == u16::MAX {
            next.fill(usize::MAX);
        } else if usize::from(word) < world.lines.len() {
            next[usize::from(word)] = position;
        }
        let first = starts.partition_point(|&(start, _)| start < position);
        let end = starts.partition_point(|&(start, _)| start <= position);
        if first == end {
            continue;
        }
        let mut order = crate::line_work::reserved(next.len())?;
        for (line, &word_position) in next.iter().enumerate() {
            if word_position != usize::MAX {
                order.push((word_position, line as u16));
            }
        }
        order.sort_unstable(); // Original word positions, never linedef IDs.
        for &(_, row) in &starts[first..end] {
            references = references
                .checked_add(order.len())
                .filter(|n| *n <= budget)
                .ok_or(GeometryError::WorkLimit)?;
            let mut list = crate::line_work::reserved(order.len())?;
            list.extend(order.iter().map(|&(_, line)| line));
            cells[row] = list;
        }
    }
    let masks = crate::line_work::CellMembership::new(&cells, world.lines.len())?;
    for (i, line) in world.lines.iter().enumerate() {
        if line.delta == [Fixed::ZERO; 2] {
            return Err(GeometryError::Unsupported(if conservative {
                "M3a degenerate line"
            } else {
                "M3b degenerate line"
            }));
        }
        if !conservative {
            continue;
        }
        let xl = ((i64::from(line.bounds[2].bits()) - (i64::from(origin.x) << 16)) >> 23) as usize;
        let xh = ((i64::from(line.bounds[3].bits()) - (i64::from(origin.x) << 16)) >> 23) as usize;
        let yl = ((i64::from(line.bounds[1].bits()) - (i64::from(origin.y) << 16)) >> 23) as usize;
        let yh = ((i64::from(line.bounds[0].bits()) - (i64::from(origin.y) << 16)) >> 23) as usize;
        for x in xl..=xh {
            for y in yl..=yh {
                if !masks.contains(y * w + x, i) {
                    return Err(GeometryError::Unsupported(
                        "M3a incomplete collision BLOCKMAP",
                    ));
                }
            }
        }
    }
    world.collision_cells = Some(cells);
    Ok(())
}

fn cell(world: &StaticWorld, position: [Fixed; 2]) -> [i32; 2] {
    let o = world.map().blockmap().origin();
    [
        (position[0] - Fixed::from_map_units(o.x)).bits() >> 23,
        (position[1] - Fixed::from_map_units(o.y)).bits() >> 23,
    ]
}

pub(crate) fn check(
    world: &StaticWorld,
    id: ActorId,
    actor: &Actor,
    target: [Fixed; 2],
    spatial: &SpatialIndex,
    get: &impl Fn(ActorId) -> Option<Actor>,
) -> ActorMoveAttempt {
    let mut result = check_things(world, id, actor, target, spatial, get);
    if result.blocked_by_actor.is_none() {
        result.lines = world.check_validated_position(target, actor.spec.radius, actor.spec.height);
    }
    result
}

pub(crate) fn check_things(
    world: &StaticWorld,
    id: ActorId,
    actor: &Actor,
    target: [Fixed; 2],
    spatial: &SpatialIndex,
    get: &impl Fn(ActorId) -> Option<Actor>,
) -> ActorMoveAttempt {
    let mut result = ActorMoveAttempt {
        target,
        lines: Collision {
            blocked_by: None,
            fits: true,
            candidates: Vec::new(),
        },
        actors: Vec::new(),
        blocked_by_actor: None,
    };
    for_things(
        world,
        id,
        actor,
        target,
        spatial,
        get,
        |candidate, other| {
            result.actors.push(candidate);
            if other.solidity == Solidity::NonSolid || candidate == id {
                return true;
            }
            let distance =
                i64::from(actor.spec.radius.bits()) + i64::from(other.spec.radius.bits());
            if (i64::from(other.spec.position[0].bits()) - i64::from(target[0].bits())).abs()
                < distance
                && (i64::from(other.spec.position[1].bits()) - i64::from(target[1].bits())).abs()
                    < distance
            {
                result.blocked_by_actor = Some(candidate);
                return false;
            }
            true
        },
    );
    result
}
pub(crate) fn for_things(
    world: &StaticWorld,
    id: ActorId,
    actor: &Actor,
    target: [Fixed; 2],
    spatial: &SpatialIndex,
    get: &impl Fn(ActorId) -> Option<Actor>,
    mut visit: impl FnMut(ActorId, Actor) -> bool,
) {
    let margin = actor.spec.radius + Fixed::from_map_units(32);
    let [xl, yl] = cell(world, [target[0] - margin, target[1] - margin]);
    let [xh, yh] = cell(world, [target[0] + margin, target[1] + margin]);
    let [w, h] = world.map().blockmap().dimensions().map(i32::from);
    for x in xl.max(0)..=xh.min(w - 1) {
        for y in yl.max(0)..=yh.min(h - 1) {
            for candidate in &spatial.order {
                let other = if *candidate == id {
                    *actor
                } else {
                    get(*candidate).expect("valid spatial handle")
                };
                if !crate::lifecycle::blocks_cell_index(&other)
                    || cell(world, other.spec.position) != [x, y]
                {
                    continue;
                }
                if !visit(*candidate, other) {
                    return;
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Divline {
    start: [Fixed; 2],
    delta: [Fixed; 2],
}
fn shifted(value: Fixed, bits: u32) -> Fixed {
    Fixed::from_bits(value.bits() >> bits)
}
fn div_side(line: Divline, point: [Fixed; 2]) -> u8 {
    let [dx, dy] = line.delta;
    if dx == Fixed::ZERO {
        return u8::from(if point[0] <= line.start[0] {
            dy > Fixed::ZERO
        } else {
            dy < Fixed::ZERO
        });
    }
    if dy == Fixed::ZERO {
        return u8::from(if point[1] <= line.start[1] {
            dx < Fixed::ZERO
        } else {
            dx > Fixed::ZERO
        });
    }
    let x = point[0] - line.start[0];
    let y = point[1] - line.start[1];
    if (dy.bits() ^ dx.bits() ^ x.bits() ^ y.bits()) < 0 {
        return u8::from((dy.bits() ^ x.bits()) < 0);
    }
    u8::from(shifted(y, 8) * shifted(dx, 8) >= shifted(dy, 8) * shifted(x, 8))
}
fn intercept(trace: Divline, line: &LineGeometry) -> Fixed {
    let [dx, dy] = line.delta;
    let den = shifted(dy, 8) * trace.delta[0] - shifted(dx, 8) * trace.delta[1];
    if den == Fixed::ZERO {
        return Fixed::ZERO;
    }
    let num = shifted(line.start[0] - trace.start[0], 8) * dy
        + shifted(trace.start[1] - line.start[1], 8) * dx;
    num / den
}

fn path(world: &StaticWorld, mut start: [Fixed; 2], end: [Fixed; 2]) -> Vec<(Fixed, usize)> {
    let o = world.map().blockmap().origin();
    let origin = [Fixed::from_map_units(o.x), Fixed::from_map_units(o.y)];
    for axis in 0..2 {
        if (start[axis] - origin[axis]).bits() & ((128 << 16) - 1) == 0 {
            start[axis] = start[axis] + Fixed::ONE;
        }
    }
    let trace = Divline {
        start,
        delta: [end[0] - start[0], end[1] - start[1]],
    };
    let a = [start[0] - origin[0], start[1] - origin[1]];
    let b = [end[0] - origin[0], end[1] - origin[1]];
    let first = a.map(|p| p.bits() >> 23);
    let last = b.map(|p| p.bits() >> 23);
    let mut map = first;
    let mut steps = [0; 2];
    let mut increments = [Fixed::ZERO; 2];
    let mut crossings = [Fixed::ZERO; 2];
    for axis in 0..2 {
        let other = 1 - axis;
        let fraction = Fixed::from_bits((a[axis].bits() >> 7) & 0xffff);
        let partial;
        if last[axis] > first[axis] {
            steps[axis] = 1;
            partial = Fixed::ONE - fraction;
            increments[axis] =
                (b[other] - a[other]) / Fixed::from_bits((b[axis] - a[axis]).bits().wrapping_abs());
        } else if last[axis] < first[axis] {
            steps[axis] = -1;
            partial = fraction;
            increments[axis] =
                (b[other] - a[other]) / Fixed::from_bits((b[axis] - a[axis]).bits().wrapping_abs());
        } else {
            partial = Fixed::ONE;
            increments[axis] = Fixed::from_bits(256 << 16);
        }
        crossings[axis] = shifted(a[other], 7) + partial * increments[axis];
    }
    let [w, h] = world.map().blockmap().dimensions().map(i32::from);
    let mut seen = crate::line_work::SeenLines::default();
    let mut hits = Vec::new();
    for _ in 0..64 {
        if map[0] >= 0 && map[1] >= 0 && map[0] < w && map[1] < h {
            for n in world.line_candidates(map[0] as usize, map[1] as usize) {
                let i = usize::from(n);
                if !seen.insert(i) {
                    continue;
                }
                let line = &world.lines[i];
                let sides = if trace
                    .delta
                    .iter()
                    .any(|d| d.bits() > 16 * 65536 || d.bits() < -16 * 65536)
                {
                    [
                        div_side(trace, line.start),
                        div_side(
                            trace,
                            [line.start[0] + line.delta[0], line.start[1] + line.delta[1]],
                        ),
                    ]
                } else {
                    [line.point_side(start), line.point_side(end)]
                };
                if sides[0] != sides[1] {
                    let frac = intercept(trace, line);
                    if frac >= Fixed::ZERO {
                        hits.push((frac, i));
                    }
                }
            }
        }
        if map == last {
            break;
        }
        if crossings[0].bits() >> 16 == map[1] {
            crossings[0] = crossings[0] + increments[0];
            map[0] += steps[0];
        } else if crossings[1].bits() >> 16 == map[0] {
            crossings[1] = crossings[1] + increments[1];
            map[1] += steps[1];
        }
    }
    // Stable sort reproduces the C repeated strict-minimum scan on ties.
    hits.sort_by_key(|hit| hit.0);
    hits
}

fn slope_div(num: u32, den: u32) -> usize {
    if den < 512 {
        return 2048;
    }
    ((num.wrapping_shl(3) / (den >> 8)).min(2048)) as usize
}
pub(crate) fn point_angle(delta: [Fixed; 2]) -> u32 {
    let [x, y] = delta.map(Fixed::bits);
    if x == 0 && y == 0 {
        return 0;
    }
    let ax = x.unsigned_abs();
    let ay = y.unsigned_abs();
    let value = if ax > ay {
        TAN_TO_ANGLE[slope_div(ay, ax)]
    } else {
        TAN_TO_ANGLE[slope_div(ax, ay)]
    };
    match (x >= 0, y >= 0, ax > ay) {
        (true, true, true) => value,
        (true, true, false) => 0x3fffffff_u32.wrapping_sub(value),
        (true, false, true) => value.wrapping_neg(),
        (true, false, false) => 0xc0000000_u32.wrapping_add(value),
        (false, true, true) => 0x7fffffff_u32.wrapping_sub(value),
        (false, true, false) => 0x40000000_u32.wrapping_add(value),
        (false, false, true) => 0x80000000_u32.wrapping_add(value),
        (false, false, false) => 0xbfffffff_u32.wrapping_sub(value),
    }
}
fn project(line: &LineGeometry, position: [Fixed; 2], motion: [Fixed; 2]) -> [Fixed; 2] {
    if line.slope == 0 {
        return [motion[0], Fixed::ZERO];
    }
    if line.slope == 1 {
        return [Fixed::ZERO, motion[1]];
    }
    let mut angle = point_angle(line.delta);
    if line.point_side(position) == 1 {
        angle = angle.wrapping_add(0x80000000);
    }
    let mut delta = point_angle(motion).wrapping_sub(angle);
    if delta > 0x80000000 {
        delta = delta.wrapping_add(0x80000000);
    }
    let [x, y] = motion.map(|m| m.bits().unsigned_abs());
    let length = Fixed::from_bits(x.wrapping_add(y).wrapping_sub(x.min(y) >> 1) as i32);
    let new_length = length * Fixed::from_bits(crate::FINE_SINE[(delta >> 19) as usize + 2048]);
    let index = (angle >> 19) as usize;
    [
        new_length * Fixed::from_bits(crate::FINE_SINE[index + 2048]),
        new_length * Fixed::from_bits(crate::FINE_SINE[index]),
    ]
}

pub(crate) enum MotionEvent {
    Attempt(ActorMoveAttempt),
    Slide(u16),
    Sector(crate::SectorMoveAttempt),
}
struct Mover<'a, F> {
    world: &'a StaticWorld,
    id: ActorId,
    actor: &'a mut Actor,
    spatial: &'a mut SpatialIndex,
    get: F,
    events: Vec<MotionEvent>,
    profile: MotionProfile<'a>,
}
impl<F: Fn(ActorId) -> Option<Actor>> Mover<'_, F> {
    fn attempt(&mut self, delta: [Fixed; 2]) -> bool {
        let target = [
            self.actor.spec.position[0] + delta[0],
            self.actor.spec.position[1] + delta[1],
        ];
        if let Some(sectors) = self.profile.sectors {
            let attempt = crate::multisector::check_move(
                (self.world, sectors),
                self.id,
                self.actor,
                target,
                self.spatial,
                &self.get,
                self.profile.command.is_some(),
            );
            let allowed = attempt.allowed();
            if allowed {
                self.actor.spec.position = target;
                self.actor.floor = attempt.floor;
                self.actor.ceiling = attempt.ceiling;
                self.actor.location = Some(attempt.location);
                self.spatial.link(self.id);
            }
            self.events.push(MotionEvent::Sector(attempt));
            return allowed;
        }
        let attempt = check(
            self.world,
            self.id,
            self.actor,
            target,
            self.spatial,
            &self.get,
        );
        let allowed = attempt.allowed();
        self.events.push(MotionEvent::Attempt(attempt));
        if allowed {
            self.actor.spec.position = target;
            self.spatial.link(self.id);
        }
        allowed
    }
    fn stairstep(&mut self) {
        if !self.attempt([Fixed::ZERO, self.actor.spec.momentum[1]]) {
            self.attempt([self.actor.spec.momentum[0], Fixed::ZERO]);
        }
    }
    fn slide(&mut self) {
        for _ in 0..2 {
            let pos = self.actor.spec.position;
            let radius = self.actor.spec.radius;
            let mom = self.actor.spec.momentum;
            let lead = [
                if mom[0] > Fixed::ZERO {
                    pos[0] + radius
                } else {
                    pos[0] - radius
                },
                if mom[1] > Fixed::ZERO {
                    pos[1] + radius
                } else {
                    pos[1] - radius
                },
            ];
            let trail = [
                if mom[0] > Fixed::ZERO {
                    pos[0] - radius
                } else {
                    pos[0] + radius
                },
                if mom[1] > Fixed::ZERO {
                    pos[1] - radius
                } else {
                    pos[1] + radius
                },
            ];
            let mut best = Fixed::from_bits(65537);
            let mut blocker = None;
            for start in [lead, [trail[0], lead[1]], [lead[0], trail[1]]] {
                for (frac, i) in path(self.world, start, [start[0] + mom[0], start[1] + mom[1]]) {
                    if frac > Fixed::ONE {
                        break;
                    }
                    let blocks = if let Some(sectors) = self.profile.sectors {
                        crate::multisector::slide_blocks(self.world, sectors, i, self.actor)
                    } else {
                        self.world.lines[i].point_side(pos) == 0
                    };
                    if !blocks {
                        continue;
                    }
                    if frac < best {
                        best = frac;
                        blocker = Some(i);
                    }
                    break;
                }
            }
            let Some(i) = blocker else {
                self.stairstep();
                return;
            };
            let flush = best - Fixed::from_bits(0x800);
            if flush > Fixed::ZERO && !self.attempt([mom[0] * flush, mom[1] * flush]) {
                self.stairstep();
                return;
            }
            let remainder = (Fixed::ONE - best).min(Fixed::ONE);
            if remainder <= Fixed::ZERO {
                return;
            }
            self.events.push(MotionEvent::Slide(i as u16));
            let motion = project(
                &self.world.lines[i],
                self.actor.spec.position,
                [mom[0] * remainder, mom[1] * remainder],
            );
            self.actor.spec.momentum = motion;
            if self.attempt(motion) {
                return;
            }
        }
        self.stairstep();
    }
}

pub(crate) fn move_xy(
    world: &StaticWorld,
    id: ActorId,
    actor: &mut Actor,
    spatial: &mut SpatialIndex,
    get: impl Fn(ActorId) -> Option<Actor>,
    command: Option<TicCommand>,
) -> Vec<MotionEvent> {
    move_xy_profile(
        world,
        id,
        actor,
        spatial,
        get,
        MotionProfile {
            command,
            sectors: None,
        },
    )
}
pub(crate) fn move_xy_multisector(
    world: &StaticWorld,
    id: ActorId,
    actor: &mut Actor,
    spatial: &mut SpatialIndex,
    get: impl Fn(ActorId) -> Option<Actor>,
    command: Option<TicCommand>,
    sectors: &[crate::SectorState],
) -> Vec<MotionEvent> {
    move_xy_profile(
        world,
        id,
        actor,
        spatial,
        get,
        MotionProfile {
            command,
            sectors: Some(sectors),
        },
    )
}
struct MotionProfile<'a> {
    command: Option<TicCommand>,
    sectors: Option<&'a [crate::SectorState]>,
}
fn move_xy_profile(
    world: &StaticWorld,
    id: ActorId,
    actor: &mut Actor,
    spatial: &mut SpatialIndex,
    get: impl Fn(ActorId) -> Option<Actor>,
    profile: MotionProfile<'_>,
) -> Vec<MotionEvent> {
    let command = profile.command;
    let mut mover = Mover {
        world,
        id,
        actor,
        spatial,
        get,
        events: Vec::new(),
        profile,
    };
    if mover.actor.spec.momentum == [Fixed::ZERO; 2] {
        return mover.events;
    }
    for m in &mut mover.actor.spec.momentum {
        *m = Fixed::from_bits(m.bits().clamp(-30 * 65536, 30 * 65536));
    }
    let [mut x, mut y] = mover.actor.spec.momentum.map(Fixed::bits);
    loop {
        let delta = if x > 15 * 65536 || y > 15 * 65536 {
            let d = [x / 2, y / 2];
            x >>= 1;
            y >>= 1;
            d
        } else {
            let d = [x, y];
            x = 0;
            y = 0;
            d
        };
        if !mover.attempt(delta.map(Fixed::from_bits)) {
            if command.is_some() {
                mover.slide();
            } else {
                mover.actor.spec.momentum = [Fixed::ZERO; 2];
            }
        }
        if x == 0 && y == 0 {
            break;
        }
    }
    if mover.profile.sectors.is_some() && mover.actor.z > mover.actor.floor {
        return mover.events;
    }
    if mover
        .actor
        .life
        .is_some_and(|life| life.flags & 1048576 != 0)
        && mover
            .actor
            .spec
            .momentum
            .iter()
            .any(|m| m.bits() > 16384 || m.bits() < -16384)
    {
        if let Some(sectors) = mover.profile.sectors {
            if mover.actor.floor != sectors[usize::from(mover.actor.location.unwrap().sector)].floor
            {
                return mover.events;
            }
        }
    }
    if mover
        .actor
        .spec
        .momentum
        .iter()
        .all(|m| m.bits() > -0x1000 && m.bits() < 0x1000)
        && command.is_none_or(|c| c.forward == 0 && c.side == 0)
    {
        mover.actor.spec.momentum = [Fixed::ZERO; 2];
    } else {
        for m in &mut mover.actor.spec.momentum {
            *m = *m * Fixed::from_bits(0xe800);
        }
    }
    mover.events
}

/// Original P_AproxDistance within the admitted signed coordinate bounds.
pub(crate) fn approx_distance(delta: [Fixed; 2]) -> Fixed {
    let x = delta[0].bits().abs();
    let y = delta[1].bits().abs();
    Fixed::from_bits(x + y - (x.min(y) >> 1))
}
