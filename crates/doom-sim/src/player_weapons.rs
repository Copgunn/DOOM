//! Weapon-only source state; OriginalPlayer and the actor store retain ownership.
use crate::{identity::*, *};

/// Four-weapon context for DSIM24/policy8; the pickup policy composes it separately.
pub const PLAYER_DEMON_WEAPONS_CONTEXT_SCHEMA: &[u8] = b"PLAYER-DEMON-1-MEDIUM-GROUNDED-WEAPONS-v1";
/// Original wp_nochange (doomdef.h); distinct from NUMWEAPONS.
pub(crate) const NO_CHANGE: u8 = 10;

/// Dynamic source weapon fields, without duplicate body, health or lifetime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlayerWeaponState {
    /// Actual source readyweapon.
    pub ready: Weapon,
    /// Pending weapon; None represents wp_nochange.
    pub pending: Option<Weapon>,
    /// Source ownership bits, limited to the admitted four weapons.
    pub owned: u16,
    /// am_clip (maximum200).
    pub bullets: u16,
    /// am_shell (maximum50).
    pub shells: u16,
    /// am_cell; 0 under every policy before the Episode 1 policy (DSIM39).
    pub cells: u16,
    /// am_misl (rockets); 0 under every policy before the Episode 1 policy.
    pub rockets: u16,
    /// Source `player->backpack`; false under every policy before the Episode 1
    /// policy. The maxima double once it is set (p_inter.c:589–595).
    pub backpack: bool,
    /// Source attackdown.
    pub attack_down: bool,
    /// Source refire, checked on increment.
    pub refire: i32,
    /// Source pre-friction momentum bob; not a camera implementation.
    pub bob: Fixed,
    /// Source weapon lighting0..2.
    pub extra_light: i32,
    /// Original weapon and flash psprites; never actors.
    pub psprites: [Psprite; 2],
}
impl PlayerWeaponState {
    pub(crate) fn initial(inventory: WeaponInventory) -> Result<Self, SimError> {
        inventory.validate()?;
        Ok(Self {
            ready: inventory.ready,
            pending: None,
            owned: inventory.owned,
            bullets: u16::try_from(inventory.ammo[0]).map_err(|_| WeaponError::Inventory)?,
            shells: u16::try_from(inventory.ammo[1]).map_err(|_| WeaponError::Inventory)?,
            cells: u16::try_from(inventory.ammo[2]).map_err(|_| WeaponError::Inventory)?,
            rockets: u16::try_from(inventory.ammo[3]).map_err(|_| WeaponError::Inventory)?,
            backpack: false,
            attack_down: true,
            refire: 0,
            bob: Fixed::ZERO,
            extra_light: 0,
            psprites: [Psprite::default(); 2],
        })
    }
    pub(crate) fn adapter(self, id: ActorId, health: i32) -> WeaponPlayer {
        WeaponPlayer {
            slot: PlayerSlot::One.ordinal(),
            actor: id,
            health,
            ready: self.ready,
            pending: self.pending,
            owned: self.owned,
            ammo: self.ammo(),
            max_ammo: self.max_ammo(),
            attack_down: self.attack_down,
            refire: self.refire,
            bob: self.bob,
            extra_light: self.extra_light,
            psprites: self.psprites,
        }
    }
    /// The state after one weapon operation on `self`'s adapter: the backpack
    /// is not a weapon field, so it is kept, and the maxima must be `self`'s.
    /// `episode1` (the selected policy, never inferred from the values) admits
    /// the cells, rockets and the two Episode 1 weapons.
    pub(crate) fn after_adapter(
        self,
        value: WeaponPlayer,
        id: ActorId,
        episode1: bool,
    ) -> Result<Self, SimError> {
        if value.actor != id
            || value.slot != PlayerSlot::One.ordinal()
            || value.max_ammo != self.max_ammo()
        {
            return Err(WeaponError::Inventory.into());
        }
        WeaponInventory {
            ready: value.ready,
            owned: value.owned,
            ammo: value.ammo,
        }
        .validate_with(value.max_ammo, episode1)?;
        Ok(Self {
            ready: value.ready,
            pending: value.pending,
            owned: value.owned,
            bullets: u16::try_from(value.ammo[0]).map_err(|_| WeaponError::Inventory)?,
            shells: u16::try_from(value.ammo[1]).map_err(|_| WeaponError::Inventory)?,
            cells: u16::try_from(value.ammo[2]).map_err(|_| WeaponError::Inventory)?,
            rockets: u16::try_from(value.ammo[3]).map_err(|_| WeaponError::Inventory)?,
            backpack: self.backpack,
            attack_down: value.attack_down,
            refire: value.refire,
            bob: value.bob,
            extra_light: value.extra_light,
            psprites: value.psprites,
        })
    }
    /// `player->ammo` in `ammotype_t` order (clip, shell, cell, misl).
    pub(crate) fn ammo(self) -> [i32; 4] {
        [
            i32::from(self.bullets),
            i32::from(self.shells),
            i32::from(self.cells),
            i32::from(self.rockets),
        ]
    }
    /// `player->maxammo`: the `G_PlayerReborn` maxima, doubled once by the first
    /// backpack (p_inter.c:589–595).
    pub(crate) fn max_ammo(self) -> [i32; 4] {
        weapons::AMMO_CAPACITIES.map(|m| m << u32::from(self.backpack))
    }
    /// The state's domain; `episode1` admits the backpack and the cell and
    /// rocket counts the Episode 1 policy's pickups give (p_inter.c:553–599).
    pub(crate) fn validate_for(self, episode1: bool) -> Result<(), SimError> {
        // No policy before the Episode 1 policy gives a backpack.
        if self.backpack && !episode1 {
            return Err(WeaponError::Inventory.into());
        }
        WeaponInventory {
            ready: self.ready,
            owned: self.owned,
            ammo: self.ammo(),
        }
        .validate_with(self.max_ammo(), episode1)?;
        if let Some(pending) = self.pending {
            Weapon::from_number_for(pending as u8, episode1)?;
            if self.owned & (1 << pending as u8) == 0 {
                return Err(WeaponError::Inventory.into());
            }
        }
        if self.refire < 0
            || !(0..=weapons::MAX_BOB_BITS).contains(&self.bob.bits())
            || !(0..=2).contains(&self.extra_light)
        {
            return Err(WeaponError::Inventory.into());
        }
        for (channel, ps) in self.psprites.into_iter().enumerate() {
            if !(-15 * 65536..=17 * 65536).contains(&ps.position[0].bits())
                || !(0..=128 * 65536).contains(&ps.position[1].bits())
            {
                return Err(WeaponError::Definition.into());
            }
            if channel == 1 && ps.state.is_none() {
                if ps.tics != 0 || self.extra_light != 0 {
                    return Err(WeaponError::Definition.into());
                }
                continue;
            }
            let state = OriginalState::try_from(ps.state.ok_or(WeaponError::Definition)?.0)
                .map_err(|_| WeaponError::Definition)?;
            let valid = if channel == 0 {
                weapon_state(self.ready, state)
            } else {
                flash_light(state) == Some(self.extra_light)
            };
            if !valid || !(1..=state.descriptor().tics).contains(&ps.tics) {
                return Err(WeaponError::Definition.into());
            }
        }
        Ok(())
    }
}
pub(crate) fn weapon_state(weapon: Weapon, state: OriginalState) -> bool {
    let (first, last) = match weapon {
        Weapon::Fist => (OriginalState::S_PUNCH, OriginalState::S_PUNCH5),
        Weapon::Pistol => (OriginalState::S_PISTOL, OriginalState::S_PISTOL4),
        Weapon::Shotgun => (OriginalState::S_SGUN, OriginalState::S_SGUN9),
        Weapon::Chaingun => (OriginalState::S_CHAIN, OriginalState::S_CHAIN2),
        // info.c:193–209: contiguous ordinals, the up and down states included.
        Weapon::Rocket => (OriginalState::S_MISSILE, OriginalState::S_MISSILE3),
        Weapon::Chainsaw => (OriginalState::S_SAW, OriginalState::S_SAW3),
        _ => return false,
    };
    (first.ordinal()..=last.ordinal()).contains(&state.ordinal())
}
pub(crate) fn flash_light(state: OriginalState) -> Option<i32> {
    match state {
        OriginalState::S_PISTOLFLASH
        | OriginalState::S_SGUNFLASH1
        | OriginalState::S_CHAINFLASH1 => Some(1),
        OriginalState::S_SGUNFLASH2 | OriginalState::S_CHAINFLASH2 => Some(2),
        // info.c:199–202: `A_Light1`, no action (still 1), then `A_Light2` twice.
        OriginalState::S_MISSILEFLASH1 | OriginalState::S_MISSILEFLASH2 => Some(1),
        OriginalState::S_MISSILEFLASH3 | OriginalState::S_MISSILEFLASH4 => Some(2),
        _ => None,
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlayerWeaponRuntime {
    Pistol(PistolState),
    Pack(PlayerWeaponState),
}
impl PlayerWeaponRuntime {
    pub(crate) fn pistol(self) -> Option<PistolState> {
        if let Self::Pistol(p) = self {
            Some(p)
        } else {
            None
        }
    }
    #[cfg(test)]
    pub(crate) fn pistol_mut(&mut self) -> Option<&mut PistolState> {
        if let Self::Pistol(p) = self {
            Some(p)
        } else {
            None
        }
    }
    pub(crate) fn pack(self) -> Option<PlayerWeaponState> {
        if let Self::Pack(p) = self {
            Some(p)
        } else {
            None
        }
    }
    /// The runtime's domain; `episode1` selects the Episode 1 policy's wider
    /// pack domain.
    pub(crate) fn validate_for(self, episode1: bool) -> Result<(), SimError> {
        match self {
            Self::Pistol(p) => p.validate(),
            Self::Pack(p) => p.validate_for(episode1),
        }
    }
    pub(crate) fn adapter(self, id: ActorId, health: i32) -> WeaponPlayer {
        match self {
            Self::Pistol(p) => p.adapter(id, health),
            Self::Pack(p) => p.adapter(id, health),
        }
    }
    pub(crate) fn with_adapter_result(
        self,
        p: WeaponPlayer,
        id: ActorId,
        episode1: bool,
    ) -> Result<Self, SimError> {
        match self {
            Self::Pistol(_) => PistolState::from_adapter(p, id).map(Self::Pistol),
            Self::Pack(s) => s.after_adapter(p, id, episode1).map(Self::Pack),
        }
    }
}
/// The weapon bits of a command; `episode1` admits every three-bit selection
/// with `BT_CHANGE` (5 and 6 are ignored by the change: never owned).
pub(crate) fn validate_command(c: TicCommand, episode1: bool) -> Result<(), SimError> {
    if c.buttons & BT_CHANGE != 0 {
        if !episode1 {
            Weapon::from_number((c.buttons & BT_WEAPONMASK) >> 3)?;
        }
    } else if c.buttons & BT_WEAPONMASK != 0 {
        return Err(SimError::UnsupportedButtons);
    }
    Ok(())
}
impl Simulation {
    /// Explicit four-weapon/player/Demon capability; setup still requires a body
    /// and controlled initial inventory. No pickups or other weapons are admitted.
    pub fn original_player_demon_weapons_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let registry =
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonWeaponsV1)?;
        Self::with_original_registry(map, limit, registry, ActorOptions::default(), context)
            .map_err(Into::into)
    }
    /// Source-compatible controlled loadout, once, after body binding before tic0.
    pub fn initialize_player_weapons(
        &mut self,
        _slot: PlayerSlot,
        inventory: WeaponInventory,
    ) -> Result<(), SimError> {
        let p = self
            .original_player
            .as_ref()
            .ok_or(PlayerBodyError::Profile)?;
        if !p.policy.has_weapon_pack() {
            return Err(PlayerBodyError::Profile.into());
        }
        if self.tic != 0
            || p.weapon.is_some()
            || p.state != PlayerBodyState::Live
            || p.health != original_player::INITIAL_PLAYER_HEALTH
        {
            return Err(WeaponError::Inventory.into());
        }
        let initial = PlayerWeaponRuntime::Pack(PlayerWeaponState::initial(inventory)?);
        self.checked_demon_operation(|host| {
            let result = player_pistol::run(
                host,
                weapons::PlayerWeaponOperation::Setup,
                TicCommand::default(),
                0,
                None,
                None,
                Some(initial),
            );
            if let Err(error) = result {
                host.fail_sight(error);
            }
            result
        })?
    }
    /// Copy of weapon-specific observations; cannot mutate health or ownership.
    pub fn original_player_weapons(
        &self,
        _slot: PlayerSlot,
    ) -> Result<Option<PlayerWeaponState>, PlayerBodyError> {
        self.original_player
            .as_ref()
            .filter(|p| p.policy.has_weapon_pack())
            .map(|p| p.weapon.and_then(PlayerWeaponRuntime::pack))
            .ok_or(PlayerBodyError::Profile)
    }
}

#[cfg(test)]
#[path = "player_weapon_tests.rs"]
mod tests;
