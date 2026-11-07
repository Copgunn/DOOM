//! M3f2 arithmetic domain and independent setup/work admission.
use crate::*;
use std::mem::size_of;

/// Maximum prepared grid cells in the explicit M3f2 profile.
pub const MAP_EXTENT_MAX_CELLS: usize = 8192;
/// Fully expanded BSP/leaf/seg work, counting repeated children.
pub const MAP_EXTENT_SIGHT_WORK: usize = 16384;
/// Conservative preparation allocation requests; excludes parsed Map/canonical caches.
pub const MAP_EXTENT_PREPARATION_BYTES: usize = 4 * 1024 * 1024;
/// Collision setup proof budget: BSP nodes + four line lists + four actor scans.
pub const MAP_EXTENT_COLLISION_WORK: usize = 16384;
pub(crate) const QUERY_UNITS: u32 = 2048;
const LINE_NORM_UNITS: u32 = 1536;
const HEIGHT_UNITS: u16 = 1024;
const MOMENTUM_UNITS: u32 = 64;
const STEP_XY_UNITS: u32 = QUERY_UNITS - 30;
const STEP_Z_UNITS: u32 = QUERY_UNITS - MOMENTUM_UNITS;

/// Smaller setup budgets do not change the behavior of an admitted simulation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MapExtentLimits {
    /// At most 64 unbound bodies.
    pub max_actors: u32,
    /// At most 262144 first-occurrence references, counting shared lists per cell.
    pub max_prepared_references: usize,
    /// At most 4 MiB of conservatively charged preparation requests.
    pub max_preparation_bytes: usize,
    /// At most 16384 conservative collision work units per destination query.
    pub max_collision_work: usize,
}
impl Default for MapExtentLimits {
    fn default() -> Self {
        Self {
            max_actors: 64,
            max_prepared_references: MAP_SCALE_MAX_REFERENCES,
            max_preparation_bytes: MAP_EXTENT_PREPARATION_BYTES,
            max_collision_work: MAP_EXTENT_COLLISION_WORK,
        }
    }
}

pub(crate) fn validate(map: &Map, limits: MapExtentLimits) -> Result<usize, SimError> {
    validate_with_actor_cap(map, limits, 64)
}
/// The same proof with the actor capacity of the selected policy; the collision
/// work budget charges four scans of that capacity per destination query.
pub(crate) fn validate_with_actor_cap(
    map: &Map,
    limits: MapExtentLimits,
    actor_cap: u32,
) -> Result<usize, SimError> {
    let cells = map.blockmap().cell_ranges().len();
    if limits.max_actors > actor_cap
        || limits.max_prepared_references > MAP_SCALE_MAX_REFERENCES
        || limits.max_preparation_bytes > MAP_EXTENT_PREPARATION_BYTES
        || limits.max_collision_work > MAP_EXTENT_COLLISION_WORK
        || map.linedefs().len() > MAP_SCALE_MAX_LINES
        || map.sectors().len() > MAP_SCALE_MAX_SECTORS
        || map.nodes().len() > 4096
        || map.subsectors().len() > 4096
        || cells > MAP_EXTENT_MAX_CELLS
        || map.blockmap().words().len() > 65536
    {
        return Err(GeometryError::WorkLimit.into());
    }
    let origin = map.blockmap().origin();
    if [origin.x, origin.y]
        .iter()
        .any(|v| u32::from(v.unsigned_abs()) > QUERY_UNITS)
        || map.vertices().iter().any(|v| {
            u32::from(v.x.unsigned_abs()) > QUERY_UNITS
                || u32::from(v.y.unsigned_abs()) > QUERY_UNITS
        })
        || map.nodes().iter().any(|n| {
            [n.origin.x, n.origin.y, n.delta.x, n.delta.y]
                .iter()
                .any(|v| u32::from(v.unsigned_abs()) > QUERY_UNITS)
        })
        || map
            .sectors()
            .iter()
            .any(|s| s.heights.iter().any(|v| v.unsigned_abs() > HEIGHT_UNITS))
    {
        return Err(GeometryError::CoordinateOverflow.into());
    }
    for line in map.linedefs() {
        let [a, b] = line.vertices.map(|i| map.vertices()[usize::from(i)]);
        // Intercept num/den both fit: 4096 * (|dx|+|dy|) * 256,
        // plus at most 1538 raw rounding units, strictly below i32::MAX.
        let norm = (i32::from(b.x) - i32::from(a.x)).unsigned_abs()
            + (i32::from(b.y) - i32::from(a.y)).unsigned_abs();
        if norm > LINE_NORM_UNITS {
            return Err(GeometryError::CoordinateOverflow.into());
        }
    }
    let work = map
        .nodes()
        .len()
        .checked_add(4 * map.linedefs().len())
        .and_then(|n| n.checked_add(4 * limits.max_actors as usize))
        .ok_or(GeometryError::WorkLimit)?;
    if work > limits.max_collision_work {
        return Err(GeometryError::WorkLimit.into());
    }
    // Charge all stage scratch together, even though its lifetimes do not overlap.
    // Reserved reference capacity is charged, rather than assuming short lists.
    let stride = map.linedefs().len().div_ceil(64);
    let requests = [
        (cells, size_of::<Vec<u16>>() + size_of::<(usize, usize)>()),
        (cells * stride, size_of::<u64>()),
        (limits.max_prepared_references, size_of::<u16>()),
        (
            map.linedefs().len(),
            size_of::<LineGeometry>() + size_of::<usize>() + size_of::<(usize, u16)>(),
        ),
        (
            map.nodes().len(),
            size_of::<Option<usize>>() + 2 * size_of::<(usize, bool)>(),
        ),
        (map.subsectors().len(), size_of::<u16>()),
        (map.sectors().len(), size_of::<SectorState>()),
        (16, size_of::<Vec<u8>>()),
    ];
    let bytes = requests
        .into_iter()
        .try_fold(0_usize, |total, (n, size)| {
            n.checked_mul(size).and_then(|n| total.checked_add(n))
        })
        .ok_or(GeometryError::WorkLimit)?;
    if bytes > limits.max_preparation_bytes {
        return Err(GeometryError::WorkLimit.into());
    }
    sight::expanded_work(map, MAP_EXTENT_SIGHT_WORK)
}

/// Prepared-table bounds the real-map policy keeps as allocation guards (M3h6 note
/// §4.1 row 7): compiled cell references and preparation bytes are functions of
/// the lump, recorded, and refused only beyond these, which no `short`-indexed map
/// of the two IWADs approaches (at most 13,435 BLOCKMAP words).
pub const REAL_MAP_MAX_PREPARED_REFERENCES: usize = 1 << 24;
pub const REAL_MAP_PREPARATION_BYTES: usize = 64 * 1024 * 1024;
/// What the real-map admission derives from the loaded map instead of checking
/// against constants: every value is recorded, none is a fixed threshold.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RealMapAdmission {
    /// Fully expanded BSP/leaf/seg sight work of one query (`sight::expanded_work`).
    pub sight_work: usize,
    /// Per-operation sight work budget: `64 · (sight_work + 5 · lines)`.
    pub sight_budget: usize,
    /// BLOCKMAP list words the preparation may compile into cell references.
    pub references: usize,
    /// Conservative collision work of one destination query:
    /// `nodes + 4 · lines + 4 · actors`.
    pub collision_work: usize,
    /// Conservatively charged preparation requests, as the M3f2 proof charges them.
    pub preparation_bytes: usize,
}
/// The real-map admission (M3h6 note §4.1): format bounds, heights within the M3f2
/// vertical envelope, and the derived work values. The arithmetic certificate,
/// the boundary and the BLOCKMAP rules are checked by the caller.
/// `sight_queries` is the per-operation sight-query allowance the work budget is
/// sized for: 64 for every older policy, `8·(K + 1)` under the Episode 1 policy
/// (decision D8).
pub(crate) fn validate_real_map(
    map: &Map,
    actor_cap: u32,
    sight_queries: usize,
) -> Result<RealMapAdmission, SimError> {
    let records = crate::player_map_scale::REAL_MAP_FORMAT_RECORDS;
    let blockmap = map.blockmap();
    let cells = blockmap.cell_ranges().len();
    if map.linedefs().len() > records
        || map.sectors().len() > records
        || map.nodes().len() > records
        || map.subsectors().len() > records
        || map.segs().len() > records
        || map.vertices().len() > records
        || map.sidedefs().len() > records
        || blockmap.words().len() > 32_768
        || cells >= blockmap.words().len()
    {
        return Err(GeometryError::WorkLimit.into());
    }
    if map
        .sectors()
        .iter()
        .any(|s| s.heights.iter().any(|v| v.unsigned_abs() > HEIGHT_UNITS))
    {
        return Err(GeometryError::CoordinateOverflow.into());
    }
    // The sight expansion of a BSP that is a tree is at most 2 per node plus one
    // per leaf plus its segs; a larger expansion means shared children.
    let tree_bound = 2 * map.nodes().len() + map.subsectors().len() + map.segs().len();
    let sight_work = sight::expanded_work(map, tree_bound)?;
    let lines = map.linedefs().len();
    let sight_budget = sight_queries
        .checked_mul(
            sight_work
                .checked_add(5 * lines)
                .ok_or(GeometryError::WorkLimit)?,
        )
        .ok_or(GeometryError::WorkLimit)?;
    let references = blockmap
        .cell_ranges()
        .iter()
        .try_fold(0_usize, |total, range| total.checked_add(range.len()))
        .filter(|n| *n <= REAL_MAP_MAX_PREPARED_REFERENCES)
        .ok_or(GeometryError::WorkLimit)?;
    let collision_work = map
        .nodes()
        .len()
        .checked_add(4 * lines)
        .and_then(|n| n.checked_add(4 * actor_cap as usize))
        .ok_or(GeometryError::WorkLimit)?;
    let stride = lines.div_ceil(64);
    let requests = [
        (cells, size_of::<Vec<u16>>() + size_of::<(usize, usize)>()),
        (cells * stride, size_of::<u64>()),
        (references, size_of::<u16>()),
        (
            lines,
            size_of::<LineGeometry>() + size_of::<usize>() + size_of::<(usize, u16)>(),
        ),
        (
            map.nodes().len(),
            size_of::<Option<usize>>() + 2 * size_of::<(usize, bool)>(),
        ),
        (map.subsectors().len(), size_of::<u16>()),
        (map.sectors().len(), size_of::<SectorState>()),
        (16, size_of::<Vec<u8>>()),
    ];
    let preparation_bytes = requests
        .into_iter()
        .try_fold(0_usize, |total, (n, size)| {
            n.checked_mul(size).and_then(|n| total.checked_add(n))
        })
        .filter(|bytes| *bytes <= REAL_MAP_PREPARATION_BYTES)
        .ok_or(GeometryError::WorkLimit)?;
    Ok(RealMapAdmission {
        sight_work,
        sight_budget,
        references,
        collision_work,
        preparation_bytes,
    })
}

pub(crate) fn position(
    values: impl IntoIterator<Item = Fixed>,
    units: u32,
) -> Result<(), SimError> {
    map_scale::bounded(values, units)
}
pub(crate) fn spawn_bounds(spec: ActorSpec, vertical: VerticalSpec) -> Result<(), SimError> {
    position(spec.position, STEP_XY_UNITS)?;
    position(
        crate::sight::xyz(spec.momentum, vertical.momentum),
        MOMENTUM_UNITS,
    )?;
    if let SpawnHeight::Absolute(z) = vertical.height {
        position([z], STEP_Z_UNITS)?;
    }
    Ok(())
}
impl Simulation {
    /// Explicit DSIM-v19 static unbound geometry, checked expanded coordinates,
    /// 8192 collision cells and 16384 expanded sight work. No player/AI/combat.
    pub fn map_extent(map: Map) -> Result<Self, SimError> {
        Self::with_map_extent_limits(map, MapExtentLimits::default())
    }
    /// Failure-atomic construction with independently smaller allocation/work budgets.
    pub fn with_map_extent_limits(map: Map, limits: MapExtentLimits) -> Result<Self, SimError> {
        let work = validate(&map, limits)?;
        let mut sim = Self::create_bounded(
            map,
            limits.max_actors,
            Profile::MapExtent,
            limits.max_prepared_references,
        )?;
        sim.world.extent_sight_work = Some(work);
        Ok(sim)
    }
    /// Actual prepared capacities; derived observations are excluded from DSIM.
    pub fn map_extent_metrics(&self) -> Result<MapScaleMetrics, SimError> {
        if self.profile != Profile::MapExtent {
            return Err(GeometryError::Unsupported("requires map-extent profile").into());
        }
        Ok(self.prepared_geometry_metrics())
    }
    pub(crate) fn preflight_map_extent_step(&self) -> Result<(), SimError> {
        for id in self.actor_order() {
            if self.actors.is_pending(id) == Some(false) {
                let a = self.actor(id).expect("live map-extent actor");
                position(a.position(), STEP_XY_UNITS)?;
                position([a.z()], STEP_Z_UNITS)?;
                position(
                    crate::sight::xyz(a.momentum(), a.vertical_momentum()),
                    MOMENTUM_UNITS,
                )?;
            }
        }
        Ok(())
    }
    pub(crate) fn map_extent_sight_actor(&self, id: ActorId) -> Result<Actor, SimError> {
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        let a = *self.actor(id).ok_or(StoreError::InvalidHandle)?;
        position(crate::sight::xyz(a.position(), a.z()), QUERY_UNITS)?;
        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_c_extreme_intercept_and_divline_outputs_match() {
        for row in include_str!("../../../tests/reference/m3f2_arithmetic.csv")
            .lines()
            .skip(1)
        {
            let n: Vec<i32> = row.split(',').map(|v| v.parse().unwrap()).collect();
            let line = |i| sight::Divline {
                start: [Fixed::from_bits(n[i]), Fixed::from_bits(n[i + 1])],
                delta: [Fixed::from_bits(n[i + 2]), Fixed::from_bits(n[i + 3])],
            };
            assert_eq!(sight::fraction(line(4), line(0)).bits(), n[8]);
            assert_eq!(i32::from(sight::side(line(4).start, line(0))), n[9]);
            assert_eq!(i32::from(sight::side(line(4).start, line(10))), n[14]);
        }
        // A defined original intercept result whose later C clipping divisor
        // would invoke abs(INT_MIN). This component does not execute that UB.
        let row = include_str!("../../../tests/reference/m3f2_boundary.csv")
            .lines()
            .nth(1)
            .unwrap();
        let n: Vec<i32> = row.split(',').map(|v| v.parse().unwrap()).collect();
        let line = sight::Divline {
            start: [Fixed::from_bits(n[0]), Fixed::from_bits(n[1])],
            delta: [Fixed::from_bits(n[2]), Fixed::from_bits(n[3])],
        };
        let ray = sight::Divline {
            start: [Fixed::from_bits(n[4]), Fixed::from_bits(n[5])],
            delta: [Fixed::from_bits(n[6]), Fixed::from_bits(n[7])],
        };
        assert_eq!(i32::from(sight::side(line.start, ray)), n[8]);
        assert_eq!(
            i32::from(sight::side(
                [line.start[0] + line.delta[0], line.start[1] + line.delta[1]],
                ray
            )),
            n[9]
        );
        assert_eq!(i32::from(sight::side(ray.start, line)), n[10]);
        assert_eq!(
            i32::from(sight::side(
                [ray.start[0] + ray.delta[0], ray.start[1] + ray.delta[1]],
                line
            )),
            n[11]
        );
        assert_eq!(sight::fraction(ray, line).bits(), n[12]);
    }
}
