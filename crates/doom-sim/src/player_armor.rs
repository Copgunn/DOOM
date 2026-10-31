//! Original player-owned armor; definition, body and health keep their owners.
use crate::*;

/// Explicit original armor execution context, separate from historical pickups.
pub const PLAYER_DEMON_ARMOR_CONTEXT_SCHEMA: &[u8] = b"PLAYER-DEMON-1-MEDIUM-GROUNDED-ARMOR-v1";

/// Source player.armortype, not an original actor-definition ordinal.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ArmorType {
    /// No protection; points must be zero.
    #[default]
    None,
    /// Ordinary green armor, source type1.
    Green,
    /// Ordinary blue armor, source type2.
    Blue,
}
impl ArmorType {
    /// Explicit source scalar identity; never an actor type or handle.
    pub const fn wire_id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Green => 1,
            Self::Blue => 2,
        }
    }
    pub(crate) const fn capacity(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Green => 100,
            Self::Blue => 200,
        }
    }
}
impl TryFrom<u8> for ArmorType {
    type Error = PlayerBodyError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Green),
            2 => Ok(Self::Blue),
            _ => Err(PlayerBodyError::Invariant),
        }
    }
}
/// The sole source armor fields attached to the existing genuine player runtime.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlayerArmorState {
    /// Source armortype. This observation grants no actor ownership or execution.
    pub kind: ArmorType,
    /// Source armorpoints; ordinary green<=100, blue<=200, without bonuses.
    pub points: i32,
}
impl PlayerArmorState {
    /// P_GiveArmor compares points, rather than ordering armor classes.
    pub(crate) fn grant(&mut self, requested: ArmorType) -> Result<bool, PlayerBodyError> {
        self.validate()?;
        if requested == ArmorType::None {
            return Err(PlayerBodyError::Unsupported);
        }
        if self.points >= requested.capacity() {
            return Ok(false);
        }
        *self = Self {
            kind: requested,
            points: requested.capacity(),
        };
        Ok(true)
    }
    pub(crate) fn validate(self) -> Result<(), PlayerBodyError> {
        if self.points < 0
            || self.points > self.kind.capacity()
            || (self.kind == ArmorType::None) != (self.points == 0)
        {
            return Err(PlayerBodyError::Invariant);
        }
        Ok(())
    }
}
/// Source player armor block, after raw physical thrust and before health work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArmorEvent {
    /// Actual owned body receiving the damage.
    pub target: ActorId,
    /// Incoming source damage, retained for thrust and request observations.
    pub incoming: i32,
    /// Points consumed by the source integer armor calculation.
    pub saved: i32,
    /// Effective player/mobj health damage.
    pub effective: i32,
    /// Source armor state before the block.
    pub before: PlayerArmorState,
    /// Source armor state after the block.
    pub after: PlayerArmorState,
}
impl ArmorEvent {
    /// Pure bounded P_DamageMobj armor block. Physical thrust uses `incoming`.
    pub(crate) fn plan(
        target: ActorId,
        incoming: i32,
        before: PlayerArmorState,
    ) -> Result<Self, SimError> {
        before.validate()?;
        if !(0..=1000).contains(&incoming) {
            return Err(CombatError::Damage.into());
        }
        let mut after = before;
        let mut saved = match before.kind {
            ArmorType::None => 0,
            ArmorType::Green => incoming / 3,
            ArmorType::Blue => incoming / 2,
        };
        // The original clears armortype on equality as well as exhaustion.
        if before.kind != ArmorType::None && before.points <= saved {
            saved = before.points;
            after.kind = ArmorType::None;
        }
        after.points -= saved;
        after.validate()?;
        Ok(Self {
            target,
            incoming,
            saved,
            effective: incoming - saved,
            before,
            after,
        })
    }
}
impl Simulation {
    /// Explicit original armor world, using the existing player, store and core.
    pub fn original_player_armor_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            limit,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonArmorV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// Copy of source armor values; no mutable player or inventory alias escapes.
    pub fn original_player_armor(
        &self,
        _slot: PlayerSlot,
    ) -> Result<PlayerArmorState, PlayerBodyError> {
        let armor = self
            .original_player
            .as_ref()
            .and_then(|p| p.armor)
            .ok_or(PlayerBodyError::Profile)?;
        armor.validate()?;
        Ok(armor)
    }
}
