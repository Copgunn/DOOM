//! Real-map-scale admission for `PlayerMonstersMapScaleV1`: the walled vertex
//! rectangle derived from the loaded map, the budgets derived from fireball flight
//! time and attack cadence, and the policy-aware coordinate checks that replace the
//! historical 978/994/1,008-unit margins and the `160M/3` travel envelope for this
//! policy only. See `docs/M3H3_REFERENCE_INVESTIGATION.md` for the proof.
use crate::*;
use std::{error::Error, fmt};

/// Actor slots of the map-scale policy: `1 + 8 + 16 + 26k + (8-k) + 8(8-k) + 8 <= 241`
/// live actors for `k` Imps, plus at most `39 - 3k` spawns in one pass, is at most
/// 256 visits, so the store never fills and the visit budget keeps a 64-visit margin.
pub const MAP_SCALE_ACTOR_SLOTS: u32 = 320;
/// Live fireballs: one Imp launches at most every 23 tics (its attack state cut
/// short by pain, then the JUSTHIT shortcut of `P_CheckMissileRange`) and a fireball
/// holds its slot for at most `ceil(5776/10) + 20 = 598` passes in the largest room,
/// so `ceil(598/23) = 26` per Imp and 208 for eight. Unreachable in play; kept as
/// the schema bound and launch witness.
pub const MAP_SCALE_MAX_LIVE_PROJECTILES: usize = 208;
/// Linedefs plus the at most 33 BLOCKMAP-linked things never exceed the source
/// `MAXINTERCEPTS` (128) in one trace.
pub const MAP_SCALE_WALL_LINES: usize = 95;
/// XY momentum bound per component. In-world impulses add at most about 650 units
/// in one pass and `P_XYMovement` clamps to 30 every tic; `2^28` raw leaves two
/// orders of magnitude before any checked addition could overflow.
pub const MAP_SCALE_MOMENTUM_UNITS: u32 = 4096;
/// Absolute vertex, BLOCKMAP-origin and actor coordinate bound: the M3f2 intercept
/// proof with `G = A = 2048` and line norm 1,536.
pub const MAP_SCALE_EXTENT_UNITS: u32 = map_extent::QUERY_UNITS;
const MAX_LINKED_THINGS: usize = 1 + MAX_PLAYER_MONSTERS + MAX_WORLD_PICKUPS + MAX_PLAYER_MONSTERS;

/// Map geometry or actor state outside the proven real-map-scale domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapScaleError {
    /// A side of the vertex rectangle is not completely covered by one-sided walls.
    Walls,
    /// The BLOCKMAP does not cover the rectangle or omits a wall from a cell it touches.
    Blockmap,
    /// More linedefs than `MAP_SCALE_WALL_LINES`.
    Lines,
    /// An actor center outside the rectangle before a tic (unreachable by the proof).
    Position,
    /// An XY momentum component beyond `MAP_SCALE_MOMENTUM_UNITS`.
    Momentum,
}
impl fmt::Display for MapScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "map scale: {self:?}")
    }
}
impl Error for MapScaleError {}
impl From<MapScaleError> for SimError {
    fn from(value: MapScaleError) -> Self {
        Self::MapScale(value)
    }
}

/// The closed vertex rectangle of an admitted walled room, in raw fixed point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoomBounds {
    /// Minimum x and y over all VERTEXES.
    pub min: [Fixed; 2],
    /// Maximum x and y over all VERTEXES.
    pub max: [Fixed; 2],
}
impl RoomBounds {
    /// Closed containment of an actor center.
    pub fn contains(&self, position: [Fixed; 2]) -> bool {
        (0..2).all(|axis| position[axis] >= self.min[axis] && position[axis] <= self.max[axis])
    }
    /// Closed containment of a whole bounding box: the spawn condition under which
    /// every later `P_TryMove` keeps the open box inside the open rectangle.
    /// The box edges are formed in i64, so a coordinate near `i32::MIN` cannot wrap
    /// into the rectangle.
    pub fn contains_box(&self, position: [Fixed; 2], radius: Fixed) -> bool {
        (0..2).all(|axis| {
            let center = i64::from(position[axis].bits());
            let radius = i64::from(radius.bits());
            center - radius >= i64::from(self.min[axis].bits())
                && center + radius <= i64::from(self.max[axis].bits())
        })
    }
    /// Derive and check the rectangle of a grounded single-sector map: four fully
    /// walled sides, a BLOCKMAP that covers the rectangle and lists every wall in
    /// every cell its bbox touches, and at most `MAP_SCALE_WALL_LINES` linedefs. The
    /// caller has already required one-sided axis-aligned blocking walls and the
    /// 2,048-unit vertex envelope. Admission only; nothing here runs per tic.
    pub fn derive(map: &Map) -> Result<Self, SimError> {
        if map.linedefs().len() > MAP_SCALE_WALL_LINES {
            return Err(MapScaleError::Lines.into());
        }
        let vertices = map.vertices();
        let (Some(first), Some(_)) = (vertices.first(), map.linedefs().first()) else {
            return Err(MapScaleError::Walls.into());
        };
        let mut min = [i32::from(first.x), i32::from(first.y)];
        let mut max = min;
        for v in vertices {
            for (axis, value) in [v.x, v.y].into_iter().enumerate() {
                min[axis] = min[axis].min(i32::from(value));
                max[axis] = max[axis].max(i32::from(value));
            }
        }
        // Each side of the rectangle must be covered by the union of the wall
        // segments lying on it (closed integer intervals along the other axis).
        for axis in 0..2 {
            let other = 1 - axis;
            for edge in [min[axis], max[axis]] {
                let mut intervals: Vec<[i32; 2]> = map
                    .linedefs()
                    .iter()
                    .filter_map(|line| {
                        let [a, b] = line.vertices.map(|i| vertices[usize::from(i)]);
                        let (pa, pb) = (
                            [i32::from(a.x), i32::from(a.y)],
                            [i32::from(b.x), i32::from(b.y)],
                        );
                        (pa[axis] == edge && pb[axis] == edge)
                            .then(|| [pa[other].min(pb[other]), pa[other].max(pb[other])])
                    })
                    .collect();
                intervals.sort_unstable();
                let mut covered = min[other];
                for [start, end] in intervals {
                    if start > covered {
                        return Err(MapScaleError::Walls.into());
                    }
                    covered = covered.max(end);
                }
                if covered < max[other] {
                    return Err(MapScaleError::Walls.into());
                }
            }
        }
        let blockmap = map.blockmap();
        let origin = [
            i32::from(blockmap.origin().x),
            i32::from(blockmap.origin().y),
        ];
        let [width, height] = blockmap.dimensions().map(i32::from);
        if origin[0] > min[0]
            || origin[1] > min[1]
            || max[0] >= origin[0] + 128 * width
            || max[1] >= origin[1] + 128 * height
        {
            return Err(MapScaleError::Blockmap.into());
        }
        let words = blockmap.words();
        for (index, line) in map.linedefs().iter().enumerate() {
            let [a, b] = line.vertices.map(|i| vertices[usize::from(i)]);
            let xl = (i32::from(a.x.min(b.x)) - origin[0]) >> 7;
            let xh = (i32::from(a.x.max(b.x)) - origin[0]) >> 7;
            let yl = (i32::from(a.y.min(b.y)) - origin[1]) >> 7;
            let yh = (i32::from(a.y.max(b.y)) - origin[1]) >> 7;
            for x in xl..=xh {
                for y in yl..=yh {
                    let range = blockmap.cell_ranges()[(y * width + x) as usize].clone();
                    if !words[range].contains(&(index as u16)) {
                        return Err(MapScaleError::Blockmap.into());
                    }
                }
            }
        }
        Ok(Self {
            min: min.map(|v| Fixed::from_map_units(v as i16)),
            max: max.map(|v| Fixed::from_map_units(v as i16)),
        })
    }
}

/// Position admission under the policy: the derived room, or the historical
/// absolute margin when no room is admitted.
pub(crate) fn position_admitted(
    room: Option<RoomBounds>,
    position: [Fixed; 2],
    margin: u32,
) -> bool {
    match room {
        Some(room) => room.contains(position),
        None => position.iter().all(|v| sight::within(*v, margin)),
    }
}
/// Spawn admission: the whole closed box inside the room, or the historical margin.
pub(crate) fn spawn_admitted(
    room: Option<RoomBounds>,
    position: [Fixed; 2],
    radius: Fixed,
    margin: u32,
) -> bool {
    match room {
        Some(room) => room.contains_box(position, radius),
        None => position.iter().all(|v| sight::within(*v, margin)),
    }
}
/// Travel admission before or after a tic: inside a room, the center stays in
/// the rectangle by the wall invariant and momentum stays checked; otherwise the
/// historical `160M/3` envelope.
pub(crate) fn travel_admitted(
    room: Option<RoomBounds>,
    position: [Fixed; 2],
    momentum: [Fixed; 2],
) -> Result<(), SimError> {
    match room {
        Some(room) => {
            if !room.contains(position) {
                return Err(MapScaleError::Position.into());
            }
            momentum_admitted(momentum)
        }
        None => original_player::travel_bounds(position, momentum).map_err(Into::into),
    }
}
pub(crate) fn momentum_admitted(momentum: [Fixed; 2]) -> Result<(), SimError> {
    if momentum
        .iter()
        .all(|m| sight::within(*m, MAP_SCALE_MOMENTUM_UNITS))
    {
        Ok(())
    } else {
        Err(MapScaleError::Momentum.into())
    }
}
impl Simulation {
    /// The admitted room of the map-scale policy, if selected.
    pub(crate) fn room(&self) -> Option<RoomBounds> {
        self.original_player.and_then(|p| p.room)
    }
    pub(crate) fn position_admitted(&self, position: [Fixed; 2], margin: u32) -> bool {
        position_admitted(self.room(), position, margin)
    }
    pub(crate) fn spawn_admitted(&self, position: [Fixed; 2], radius: Fixed, margin: u32) -> bool {
        spawn_admitted(self.room(), position, radius, margin)
    }
    /// Absolute XY/Z bound of every live actor for sight and perception reads.
    pub(crate) fn sight_margin(&self) -> u32 {
        if self.room().is_some() {
            MAP_SCALE_EXTENT_UNITS
        } else {
            1024
        }
    }
    /// Live fireball bound of the selected policy.
    pub(crate) fn live_projectile_bound(&self) -> usize {
        live_projectile_bound(self.player_policy())
    }
}
pub(crate) const fn live_projectile_bound(policy: OriginalExecutionProfile) -> usize {
    if policy.has_map_scale() {
        MAP_SCALE_MAX_LIVE_PROJECTILES
    } else {
        MAX_LIVE_PROJECTILES
    }
}
pub(crate) const fn admitted_lines(policy: OriginalExecutionProfile) -> usize {
    if policy.has_map_scale() {
        MAP_SCALE_WALL_LINES
    } else {
        128
    }
}
const _: () = assert!(MAP_SCALE_WALL_LINES + MAX_LINKED_THINGS <= 128);
