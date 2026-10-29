//! Bounded commercial single-player THING admission, reusing the profile15 fixture.
use crate::*;

/// Maximum validated classic THING records accepted by profile16.
pub const MAX_DEMON_MAP_THINGS: usize = 256;
/// Maximum native map Demons before the unbound target/effect thinker suffix.
pub const MAX_MAP_DEMONS: usize = 4;

/// Map-spawn input or explicit actor-admission restriction, before mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapSpawnError {
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
