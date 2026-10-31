//! Bounded commercial single-player THING admission, reusing the profile15 fixture.
use crate::*;

/// Maximum validated classic THING records accepted by profile16.
pub const MAX_DEMON_MAP_THINGS: usize = 256;
/// Maximum native map Demons before the unbound target/effect thinker suffix.
pub const MAX_MAP_DEMONS: usize = 4;

/// Map-spawn input or explicit actor-admission restriction, before mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapSpawnError {
    /// New combined profile excludes an overflowing original signed BAM product.
    Angle(i16),
    /// Requested source respawn behavior is not admitted.
    Respawn,
    /// Operation requires the explicit map-context profile.
    Context,
    /// Checked source statistics would exceed signed 32-bit range.
    StatisticsOverflow,
    /// Batch setup requires tic zero and a never-allocated actor store.
    Setup,
    /// The bounded record or Demon count would be exceeded.
    WorkLimit,
    /// Player/deathmatch starts and invalid nonpositive types are excluded.
    StartOrInvalidType(i16),
    /// A recognized enabled type is outside Demon admission.
    UnsupportedType(i16),
    /// Native Demons enter only through the atomic THING batch.
    DirectSpawn,
    /// A live Demon or perception input needs an unbound body target.
    Target,
    /// Generalized cross-Demon damage/retargeting is outside this slice.
    Infighting,
    /// The owner-prefix or bounded late-damage planning proof does not hold.
    Ownership,
}
impl fmt::Display for MapSpawnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Demon map spawning: {self:?}")
    }
}
impl Error for MapSpawnError {}

impl Simulation {
    /// Empty profile16 using the exact profile15 native fixture and source
    /// DoomEd lookup. Call `spawn_map_things` explicitly before adding targets.
    /// Single-player DOOM II, normal timings; no player starts/scoring/raise.
    pub fn with_demon_map_spawn_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if map.things().len() > MAX_DEMON_MAP_THINGS {
            return Err(MapSpawnError::WorkLimit.into());
        }
        Self::with_demon_lifecycle_profile(map, limit, registry, options, Profile::DemonMapSpawn)
    }
}

impl Simulation {
    /// No recursion into owner callbacks: this bounded global proof also runs
    /// for direct calls before any action, state, RNG, movement or sound change.
    pub(crate) fn preflight_map_demons(&self) -> Result<(), SimError> {
        if !self.profile.uses_demon_map() {
            return Ok(());
        }
        if self.bindings.iter().any(Option::is_some) || self.weapon_player.is_some() {
            return Err(CombatError::Player.into());
        }
        self.preflight_perception_roster()?;
        for target in self.sound_targets.iter().flatten() {
            self.validate_perception_reference(*target)?;
        }
        self.preflight_world_pickups()?;
        let mut suffix = false;
        let mut owners = 0;
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            let a = self.monster_actor(id, 1008)?;
            let life = a.life.unwrap();
            // Validate complete future actions, not only this tic's next state.
            self.validate_combat_type(life.actor_type)?;
            if self.player_policy() == OriginalExecutionProfile::PlayerDemonPickupsV1
                && world_pickups::kind(life.actor_type).is_some()
            {
                world_pickups::validate_item(a)?;
                if suffix {
                    return Err(MapSpawnError::Ownership.into());
                }
            } else if life.actor_type == self.demon_owner_type() {
                owners += 1;
                if suffix || owners > self.map_demon_limit() {
                    return Err(MapSpawnError::Ownership.into());
                }
                if life.health > 0 {
                    if let Some(target) = life.target {
                        self.validate_perception_reference(target)?;
                    }
                }
            } else {
                suffix = true;
                if a.position().iter().any(|v| !crate::sight::within(*v, 994)) {
                    return Err(MonsterError::Arithmetic.into());
                }
                if life.actor_type == self.demon_body_type()
                    && a.momentum().iter().any(|v| !crate::sight::within(*v, 64))
                {
                    return Err(MapSpawnError::Ownership.into());
                }
            }
            if !crate::sight::within(a.momz, 64) || !crate::sight::within(a.z + a.momz, 1024) {
                return Err(MonsterError::Arithmetic.into());
            }
        }
        Ok(())
    }
    pub(crate) fn preflight_map_damage(&self, request: DamageRequest) -> Result<(), SimError> {
        if !self.profile.uses_demon_map() {
            return Ok(());
        }
        if self
            .actor(request.target)
            .and_then(Actor::life)
            .is_some_and(|l| l.actor_type == self.demon_owner_type())
            && request.source.is_some_and(|source| {
                source != request.target
                    && self
                        .actor(source)
                        .and_then(Actor::life)
                        .is_some_and(|l| l.actor_type == self.demon_owner_type())
            })
        {
            let target = self.actor(request.target).unwrap().life.unwrap();
            // Lethal/ignored damage returns before source retargeting. Only this
            // narrow synthetic branch is admitted; nonlethal infighting rejects.
            if !self.profile.uses_map_statistics()
                || target.flags & 4 != 0 && target.health > 0 && request.amount < target.health
            {
                return Err(MapSpawnError::Infighting.into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "demon_map_spawn_tests.rs"]
mod tests;
