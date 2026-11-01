//! Original bonus capability, reusing existing player, item and thinker owners.
use crate::*;

/// Explicit original-player bonus execution context; historical contexts stay frozen.
pub const PLAYER_DEMON_BONUSES_CONTEXT_SCHEMA: &[u8] = b"PLAYER-DEMON-1-MEDIUM-GROUNDED-BONUSES-v1";

impl Simulation {
    /// Original animated health/armor bonus world, sharing bounded player combat.
    pub fn original_player_bonuses_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            limit,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonBonusesV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
}
