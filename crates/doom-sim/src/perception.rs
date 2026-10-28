//! Bounded source sight and explicit perception profile composition.
use crate::*;
use std::{error::Error, fmt};
/// Admission/identity/work rejection before authoritative perception changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PerceptionError {
    /// Explicit profile 8 required.
    Profile,
    /// Invalid, pending, retired or stale actor.
    Reference,
    /// Unsupported source/fixture metadata or action handoff.
    Definition,
    /// Source-compatible arithmetic slice exceeded.
    Arithmetic,
    /// Actor or expanded traversal bound exceeded.
    WorkLimit,
    /// Perception roster configuration is invalid.
    Roster,
    /// Retained sound target is pending, retired or stale.
    SoundTarget,
}
impl fmt::Display for PerceptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "perception: {self:?}")
    }
}
impl Error for PerceptionError {}
impl Simulation {
    /// Explicit profile 8; no roster, weapon player, chase or attack is implicit.
    pub fn monster_perception(map: Map) -> Result<Self, SimError> {
        Self::with_perception_registry(map, 64, ActorRegistry::vanilla(), ActorOptions::default())
    }
    /// Immutable original definitions; fixture admission is explicitly constrained.
    pub fn with_perception_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if limit > 64 {
            return Err(PerceptionError::WorkLimit.into());
        }
        crate::sight::validate_map(&map)?;
        let source = ActorRegistry::vanilla();
        if registry.states() != source.states() || registry.types() != source.types() {
            return Err(PerceptionError::Definition.into());
        }
        let mut sim = Self::with_actor_registry(map, limit, registry, options)?;
        sim.profile = Profile::Perception;
        sim.sound_targets = vec![None; sim.sectors.len()];
        Ok(sim)
    }
    pub(crate) fn perception_actor(&self, id: ActorId) -> Result<Actor, SimError> {
        if self.profile != Profile::Perception {
            return Err(PerceptionError::Profile.into());
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(PerceptionError::Reference.into());
        }
        let a = *self.actor(id).ok_or(PerceptionError::Reference)?;
        crate::sight::actor_bounds(&a)?;
        Ok(a)
    }
    /// Original read-only REJECT/BSP sight, independent of roster/RNG/presentation.
    pub fn check_sight(&self, observer: ActorId, target: ActorId) -> Result<SightResult, SimError> {
        let observer = self.perception_actor(observer)?;
        let target = self.perception_actor(target)?;
        Ok(crate::sight::query(
            &self.world,
            &self.sectors,
            observer,
            target,
        ))
    }
}
