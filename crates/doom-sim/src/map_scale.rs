//! Explicit unbound static-geometry profile; no gameplay admission by version order.
use crate::line_work::MAX_LINES;
use crate::*;

/// M3f1 maximum linedefs; classic encoding and parser limits remain separate.
pub const MAP_SCALE_MAX_LINES: usize = MAX_LINES;
/// M3f1 maximum static sectors.
pub const MAP_SCALE_MAX_SECTORS: usize = 256;
/// Maximum total first-occurrence references across prepared collision cells.
pub const MAP_SCALE_MAX_REFERENCES: usize = 262_144;

/// Setup-only budgets. They do not alter successful runtime behavior or DSIM.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MapScaleLimits {
    /// At most 64 unbound bodies; no player or typed registry admission.
    pub max_actors: u32,
    /// At most MAP_SCALE_MAX_REFERENCES; shared lists count once per cell.
    pub max_prepared_references: usize,
}
impl Default for MapScaleLimits {
    fn default() -> Self {
        Self {
            max_actors: 64,
            max_prepared_references: MAP_SCALE_MAX_REFERENCES,
        }
    }
}

/// Derived storage observations, excluded from canonical authoritative state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MapScaleMetrics {
    /// Immutable linedef count.
    pub lines: usize,
    /// Runtime static sector count.
    pub sectors: usize,
    /// Collision-grid cells.
    pub cells: usize,
    /// Total compiled first-occurrence references.
    pub prepared_references: usize,
    /// Requested owned capacity bytes: compiled lines/lists, leaf sectors and
    /// runtime sector heights. Excludes parsed Map, snapshots, actors and allocator overhead.
    pub prepared_storage_bytes: usize,
}

impl Simulation {
    /// Explicit DSIM-v18 unbound XY/Z and read-only sight, at most 1024 lines /
    /// 256 static sectors. No player binding, typed actions, combat or map spawning.
    pub fn map_scale(map: Map) -> Result<Self, SimError> {
        Self::with_map_scale_limits(map, MapScaleLimits::default())
    }
    /// Failure-atomic setup with smaller actor/reference budgets for constrained hosts.
    pub fn with_map_scale_limits(map: Map, limits: MapScaleLimits) -> Result<Self, SimError> {
        if limits.max_actors > 64 || limits.max_prepared_references > MAP_SCALE_MAX_REFERENCES {
            return Err(GeometryError::WorkLimit.into());
        }
        Self::create_bounded(
            map,
            limits.max_actors,
            Profile::MapScale,
            limits.max_prepared_references,
        )
    }
    /// Read-only actual prepared capacities; no RNG or canonical-state effects.
    pub fn map_scale_metrics(&self) -> Result<MapScaleMetrics, SimError> {
        if self.profile != Profile::MapScale {
            return Err(GeometryError::Unsupported("requires map-scale profile").into());
        }
        Ok(self.prepared_geometry_metrics())
    }
    pub(crate) fn prepared_geometry_metrics(&self) -> MapScaleMetrics {
        let cells = self
            .world
            .collision_cells
            .as_ref()
            .expect("prepared map-scale lists");
        let prepared_references = cells.iter().map(Vec::len).sum();
        let prepared_storage_bytes = self.world.lines.capacity()
            * std::mem::size_of::<LineGeometry>()
            + cells.capacity() * std::mem::size_of::<Vec<u16>>()
            + cells
                .iter()
                .map(|v| v.capacity() * std::mem::size_of::<u16>())
                .sum::<usize>()
            + self
                .world
                .topology
                .as_ref()
                .expect("map-scale topology")
                .storage_bytes()
            + self.sectors.capacity() * std::mem::size_of::<SectorState>();
        MapScaleMetrics {
            lines: self.world.lines.len(),
            sectors: self.sectors.len(),
            cells: cells.len(),
            prepared_references,
            prepared_storage_bytes,
        }
    }
    pub(crate) fn preflight_map_scale_step(&self) -> Result<(), SimError> {
        for id in self.actor_order() {
            if self.actors.is_pending(id) == Some(false) {
                let a = self.actor(id).expect("live map-scale actor");
                bounded(a.position().into_iter().chain([a.z()]), 994)?;
                bounded(a.momentum().into_iter().chain([a.vertical_momentum()]), 64)?;
            }
        }
        Ok(())
    }
    pub(crate) fn map_scale_sight_actor(&self, id: ActorId) -> Result<Actor, SimError> {
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        let actor = *self.actor(id).ok_or(StoreError::InvalidHandle)?;
        crate::sight::actor_bounds(&actor)?;
        Ok(actor)
    }
}
pub(crate) fn bounded(values: impl IntoIterator<Item = Fixed>, units: u32) -> Result<(), SimError> {
    if values.into_iter().any(|v| !crate::sight::within(v, units)) {
        return Err(GeometryError::InvalidActor.into());
    }
    Ok(())
}
pub(crate) fn spawn_bounds(spec: ActorSpec, vertical: VerticalSpec) -> Result<(), SimError> {
    bounded(spec.position, 994)?;
    bounded(spec.momentum.into_iter().chain([vertical.momentum]), 64)?;
    if let SpawnHeight::Absolute(z) = vertical.height {
        bounded([z], 994)?;
    }
    Ok(())
}
