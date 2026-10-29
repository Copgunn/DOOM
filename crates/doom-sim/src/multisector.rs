//! Explicit static multi-sector topology, height constraints and vertical physics.
use crate::{Actor, ActorId, ActorMoveAttempt, GeometryError, Solidity, StaticWorld, movement};
use doom_map::BspChild;
use doom_types::Fixed;

/// Validated BSP leaf and its source first-seg sector association.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Location {
    /// Subsector table index.
    pub subsector: u16,
    /// Sector table index.
    pub sector: u16,
}
/// Current runtime sector heights, separate from immutable map definitions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SectorState {
    /// Floor height.
    pub floor: Fixed,
    /// Ceiling height.
    pub ceiling: Fixed,
}
/// Supported source gravity policy; this is not flying/monster behavior.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Gravity {
    /// Original one-unit gravity, including the initial two-unit impulse.
    #[default]
    Normal,
    /// MF_NOGRAVITY only; no MF_FLOAT, air control, or automatic flight.
    None,
}
/// Source-supported initial height selection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SpawnHeight {
    /// ONFLOORZ behavior.
    #[default]
    Floor,
    /// ONCEILINGZ behavior, minus actor height.
    Ceiling,
    /// Explicit raw Z. May be outside constraints; next Z update clips like source.
    Absolute(Fixed),
}
/// Explicit M3b vertical setup; arbitrary unsupported mobj flags are not accepted.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VerticalSpec {
    /// Initial Z selection.
    pub height: SpawnHeight,
    /// Initial vertical momentum; not a jump button or elapsed-time input.
    pub momentum: Fixed,
    /// Normal gravity or the source MF_NOGRAVITY exception.
    pub gravity: Gravity,
    /// MF_DROPOFF permission, independent of input binding/player identity.
    pub allow_dropoff: bool,
}
/// P_LineOpening result for a two-sided line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Opening {
    /// Minimum of adjacent ceilings.
    pub top: Fixed,
    /// Maximum of adjacent floors.
    pub bottom: Fixed,
    /// Top minus bottom; can be zero or negative.
    pub range: Fixed,
    /// Minimum of adjacent floors.
    pub low_floor: Fixed,
}
/// Opening visited while accumulating one destination's height constraints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpeningVisit {
    /// Linedef index in first-encounter order.
    pub line: u16,
    /// Adjacent runtime heights.
    pub opening: Opening,
}
/// First failing P_TryMove condition, in original evaluation order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MoveFailure {
    /// Destination is allowed.
    None = 0,
    /// A solid actor blocked before lines.
    Actor = 1,
    /// A one-sided wall or applicable blocking flag.
    Line = 2,
    /// Available interval is shorter than actor height.
    Fit = 3,
    /// Current Z leaves insufficient headroom.
    Headroom = 4,
    /// Raised floor is more than 24 units above current Z.
    Step = 5,
    /// Floor minus dropoff exceeds 24 without MF_DROPOFF.
    Dropoff = 6,
}
/// Observational destination decision, with source-order constraint accumulation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SectorMoveAttempt {
    /// Ordered thing and line candidate evidence.
    pub movement: ActorMoveAttempt,
    /// BSP location of the proposed center, even when blocked.
    pub location: Location,
    /// Highest contacted floor.
    pub floor: Fixed,
    /// Lowest contacted ceiling.
    pub ceiling: Fixed,
    /// Lowest contacted floor across openings.
    pub dropoff: Fixed,
    /// Source floatok flag (not implemented floating-monster behavior).
    pub float_ok: bool,
    /// First rejection or None.
    pub failure: MoveFailure,
    /// Openings actually evaluated in candidate order.
    pub openings: Vec<OpeningVisit>,
}
impl SectorMoveAttempt {
    /// Whether the destination passes all supported P_TryMove tests.
    pub const fn allowed(&self) -> bool {
        matches!(self.failure, MoveFailure::None)
    }
}
/// One executed Z update; view/animation/audio are not simulated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerticalEvent {
    /// Z/momz before update.
    pub before: [Fixed; 2],
    /// Z/momz after update.
    pub after: [Fixed; 2],
    /// Floor/ceiling constraints used.
    pub constraints: [Fixed; 2],
    /// Floor branch ran, including step-up clipping.
    pub floor_contact: bool,
    /// Ceiling branch ran.
    pub ceiling_contact: bool,
    /// Normal airborne gravity branch ran.
    pub gravity_applied: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Topology {
    leaf_sectors: Vec<u16>,
    pub conservative_omissions: u32,
}
impl Topology {
    pub fn new(world: &StaticWorld) -> Result<Self, GeometryError> {
        Self::new_bounded(world, 128)
    }
    pub fn new_bounded(world: &StaticWorld, sectors: usize) -> Result<Self, GeometryError> {
        let map = world.map();
        if map.sectors().len() > sectors
            || map.subsectors().len() > 4096
            || map.nodes().len() > 4096
        {
            return Err(GeometryError::Unsupported("M3b topology work limit"));
        }
        if map
            .linedefs()
            .iter()
            .any(|line| line.sides[1].is_some() != (line.flags & 4 != 0))
        {
            return Err(GeometryError::Unsupported(
                "M3b inconsistent two-sided flags",
            ));
        }
        let mut lowest = i32::MAX;
        let mut highest = i32::MIN;
        for sector in map.sectors() {
            let [floor, ceiling] = sector.heights.map(|h| Fixed::from_map_units(h).bits());
            if ceiling < floor {
                return Err(GeometryError::Unsupported("M3b inverted sector heights"));
            }
            lowest = lowest.min(floor);
            highest = highest.max(ceiling);
        }
        if highest.checked_sub(lowest).is_none() {
            return Err(GeometryError::CoordinateOverflow);
        }
        let mut leaf_sectors = crate::line_work::reserved(map.subsectors().len())?;
        leaf_sectors.extend(map.subsectors().iter().map(|leaf| {
            let seg = &map.segs()[usize::from(leaf.first_seg)];
            let line = &map.linedefs()[usize::from(seg.linedef)];
            map.sidedefs()
                [usize::from(line.sides[usize::from(seg.side)].expect("M1 validates seg side"))]
            .sector
        }));
        let [w, h] = map.blockmap().dimensions().map(i64::from);
        let origin = map.blockmap().origin();
        let masks = crate::line_work::CellMembership::new(
            world
                .collision_cells
                .as_ref()
                .expect("prepared collision lists"),
            world.lines.len(),
        )?;
        let mut omissions = 0;
        for (i, line) in world.lines.iter().enumerate() {
            let bounds = line.bounds.map(Fixed::bits);
            let xl = ((i64::from(bounds[2]) - (i64::from(origin.x) << 16)) >> 23).max(0);
            let xh = ((i64::from(bounds[3]) - (i64::from(origin.x) << 16)) >> 23).min(w - 1);
            let yl = ((i64::from(bounds[1]) - (i64::from(origin.y) << 16)) >> 23).max(0);
            let yh = ((i64::from(bounds[0]) - (i64::from(origin.y) << 16)) >> 23).min(h - 1);
            for x in xl..=xh {
                for y in yl..=yh {
                    if !masks.contains(y as usize * w as usize + x as usize, i) {
                        omissions += 1;
                    }
                }
            }
        }
        Ok(Self {
            leaf_sectors,
            conservative_omissions: omissions,
        })
    }
    pub fn storage_bytes(&self) -> usize {
        self.leaf_sectors.capacity() * std::mem::size_of::<u16>()
    }
    pub fn locate(&self, world: &StaticWorld, point: [Fixed; 2]) -> Location {
        let map = world.map();
        let mut child = if map.nodes().is_empty() {
            BspChild::Subsector(0)
        } else {
            BspChild::Node((map.nodes().len() - 1) as u16)
        };
        // M1 proves acyclicity, including unreachable nodes. Each branch visits
        // at most node_count nodes, without recursion or allocator-dependent order.
        for _ in 0..=map.nodes().len() {
            match child {
                BspChild::Subsector(subsector) => {
                    return Location {
                        subsector,
                        sector: self.leaf_sectors[usize::from(subsector)],
                    };
                }
                BspChild::Node(index) => {
                    let node = &map.nodes()[usize::from(index)];
                    child = node.children[usize::from(node_side(point, node))];
                }
            }
        }
        unreachable!("immutable M1-validated acyclic BSP must reach a leaf")
    }
}
fn node_side(point: [Fixed; 2], node: &doom_map::Node) -> u8 {
    let start = [
        Fixed::from_map_units(node.origin.x),
        Fixed::from_map_units(node.origin.y),
    ];
    let [dx, dy] = [
        Fixed::from_map_units(node.delta.x),
        Fixed::from_map_units(node.delta.y),
    ];
    if dx == Fixed::ZERO {
        return u8::from(if point[0] <= start[0] {
            dy > Fixed::ZERO
        } else {
            dy < Fixed::ZERO
        });
    }
    if dy == Fixed::ZERO {
        return u8::from(if point[1] <= start[1] {
            dx < Fixed::ZERO
        } else {
            dx > Fixed::ZERO
        });
    }
    let x = point[0] - start[0];
    let y = point[1] - start[1];
    if (dy.bits() ^ dx.bits() ^ x.bits() ^ y.bits()) < 0 {
        return u8::from((dy.bits() ^ x.bits()) < 0);
    }
    let left = Fixed::from_bits(dy.bits() >> 16) * x;
    let right = y * Fixed::from_bits(dx.bits() >> 16);
    u8::from(right >= left)
}

pub(crate) fn opening(
    world: &StaticWorld,
    sectors: &[SectorState],
    index: usize,
) -> Option<Opening> {
    let line = &world.map().linedefs()[index];
    let back = line.sides[1]?;
    let front = line.sides[0].expect("validated front side");
    let a = sectors[usize::from(world.map().sidedefs()[usize::from(front)].sector)];
    let b = sectors[usize::from(world.map().sidedefs()[usize::from(back)].sector)];
    let top = a.ceiling.min(b.ceiling);
    let bottom = a.floor.max(b.floor);
    Some(Opening {
        top,
        bottom,
        range: top - bottom,
        low_floor: a.floor.min(b.floor),
    })
}

pub(crate) fn check_move(
    (world, sectors): (&StaticWorld, &[SectorState]),
    id: ActorId,
    actor: &Actor,
    target: [Fixed; 2],
    spatial: &movement::SpatialIndex,
    get: &impl Fn(ActorId) -> Option<Actor>,
    player: bool,
) -> SectorMoveAttempt {
    let movement = movement::check_things(world, id, actor, target, spatial, get);
    check_move_from_things((world, sectors), actor, target, movement, player, false)
}
pub(crate) fn check_move_from_things(
    (world, sectors): (&StaticWorld, &[SectorState]),
    actor: &Actor,
    target: [Fixed; 2],
    movement: ActorMoveAttempt,
    player: bool,
    missile: bool,
) -> SectorMoveAttempt {
    let location = world.locate_validated(target);
    let sector = sectors[usize::from(location.sector)];
    let mut result = SectorMoveAttempt {
        movement,
        location,
        floor: sector.floor,
        ceiling: sector.ceiling,
        dropoff: sector.floor,
        float_ok: false,
        failure: MoveFailure::None,
        openings: Vec::new(),
    };
    if result.movement.blocked_by_actor.is_some() {
        result.failure = MoveFailure::Actor;
        return result;
    }
    let b = crate::geometry::bbox(target, actor.spec.radius);
    let origin = world.map().blockmap().origin();
    let ox = Fixed::from_map_units(origin.x);
    let oy = Fixed::from_map_units(origin.y);
    let [w, h] = world.map().blockmap().dimensions().map(i32::from);
    let xl = ((b[2] - ox).bits() >> 23).max(0);
    let xh = ((b[3] - ox).bits() >> 23).min(w - 1);
    let yl = ((b[1] - oy).bits() >> 23).max(0);
    let yh = ((b[0] - oy).bits() >> 23).min(h - 1);
    let mut seen = crate::line_work::SeenLines::default();
    for x in xl..=xh {
        for y in yl..=yh {
            for index in world.line_candidates(x as usize, y as usize) {
                let i = usize::from(index);
                if !seen.insert(i) {
                    continue;
                }
                result.movement.lines.candidates.push(index);
                if !world.lines[i].crosses(b) {
                    continue;
                }
                let line = &world.map().linedefs()[i];
                if line.sides[1].is_none()
                    || (!missile && (line.flags & 1 != 0 || (!player && line.flags & 2 != 0)))
                {
                    result.movement.lines.blocked_by = Some(index);
                    result.failure = MoveFailure::Line;
                    return result;
                }
                let opening = opening(world, sectors, i).expect("two-sided line has an opening");
                result.ceiling = result.ceiling.min(opening.top);
                result.floor = result.floor.max(opening.bottom);
                result.dropoff = result.dropoff.min(opening.low_floor);
                result.openings.push(OpeningVisit {
                    line: index,
                    opening,
                });
            }
        }
    }
    result.movement.lines.fits = result.ceiling - result.floor >= actor.spec.height;
    result.failure = if !result.movement.lines.fits {
        MoveFailure::Fit
    } else {
        result.float_ok = true;
        if result.ceiling - actor.z < actor.spec.height {
            MoveFailure::Headroom
        } else if result.floor - actor.z > Fixed::from_map_units(24) {
            MoveFailure::Step
        } else if !actor.allow_dropoff && result.floor - result.dropoff > Fixed::from_map_units(24)
        {
            MoveFailure::Dropoff
        } else {
            MoveFailure::None
        }
    };
    result
}
pub(crate) fn slide_blocks(
    world: &StaticWorld,
    sectors: &[SectorState],
    index: usize,
    actor: &Actor,
) -> bool {
    if let Some(open) = opening(world, sectors, index) {
        open.range < actor.spec.height
            || open.top - actor.z < actor.spec.height
            || open.bottom - actor.z > Fixed::from_map_units(24)
    } else {
        world.lines[index].point_side(actor.spec.position) == 0
    }
}
pub(crate) fn move_z(actor: &mut Actor) -> Option<VerticalEvent> {
    if actor.z == actor.floor && actor.momz == Fixed::ZERO {
        return None;
    }
    let before = [actor.z, actor.momz];
    let mut event = VerticalEvent {
        before,
        after: before,
        constraints: [actor.floor, actor.ceiling],
        floor_contact: false,
        ceiling_contact: false,
        gravity_applied: false,
    };
    actor.z = actor.z + actor.momz;
    if actor.z <= actor.floor {
        event.floor_contact = true;
        if actor.momz < Fixed::ZERO {
            actor.momz = Fixed::ZERO;
        }
        actor.z = actor.floor;
    } else if actor.gravity == Gravity::Normal {
        event.gravity_applied = true;
        actor.momz = if actor.momz == Fixed::ZERO {
            -Fixed::from_map_units(2)
        } else {
            actor.momz - Fixed::ONE
        };
    }
    if actor.z + actor.spec.height > actor.ceiling {
        event.ceiling_contact = true;
        if actor.momz > Fixed::ZERO {
            actor.momz = Fixed::ZERO;
        }
        actor.z = actor.ceiling - actor.spec.height;
    }
    event.after = [actor.z, actor.momz];
    Some(event)
}
pub(crate) fn flags(actor: &Actor) -> u32 {
    (if actor.solidity == Solidity::Solid {
        2
    } else {
        0
    }) | (if actor.gravity == Gravity::None {
        512
    } else {
        0
    }) | (if actor.allow_dropoff { 1024 } else { 0 })
}

#[cfg(test)]
mod extent_tests {
    use super::*;
    #[test]
    fn original_c_large_partition_side_products_match() {
        for row in include_str!("../../../tests/reference/m3f2_arithmetic.csv")
            .lines()
            .skip(1)
        {
            let n: Vec<i32> = row.split(',').map(|v| v.parse().unwrap()).collect();
            let node = doom_map::Node {
                origin: doom_map::Vertex {
                    x: (n[10] >> 16) as i16,
                    y: (n[11] >> 16) as i16,
                },
                delta: doom_map::Vertex {
                    x: (n[12] >> 16) as i16,
                    y: (n[13] >> 16) as i16,
                },
                bounds: [[0; 4]; 2],
                children: [BspChild::Subsector(0); 2],
            };
            assert_eq!(
                i32::from(node_side(
                    [Fixed::from_bits(n[4]), Fixed::from_bits(n[5])],
                    &node
                )),
                n[15]
            );
        }
    }
}
