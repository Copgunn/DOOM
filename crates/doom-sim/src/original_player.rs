//! One genuine source player body; the actor store remains its sole instance owner.
use crate::{identity::*, *};
use std::{error::Error, fmt};

/// Fixed settings/ownership contract, separate from the historical unbound body.
pub const PLAYER_BODY_CONTEXT_SCHEMA: &[u8] = b"PLAYER-BODY-1-MEDIUM-NOTHRUST-v1";
// p_mobj.h. These describe source behavior, not storage availability.
pub(crate) const MF_SOLID: u32 = 0x2;
pub(crate) const MF_SHOOTABLE: u32 = 0x4;
pub(crate) const MF_CORPSE: u32 = 0x100000;
// g_game.c G_PlayerReborn/MAXHEALTH, and p_inter.c P_DamageMobj.
pub(crate) const INITIAL_PLAYER_HEALTH: i32 = 100;
pub(crate) const MAX_DAMAGE_COUNT: i32 = 100;
pub(crate) const TACTILE_BASE: i32 = 40;

/// Bounded source player index, independent of the body's actor slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerSlot {
    /// The only admitted player; not multiplayer or a four-player roster.
    One,
}
impl PlayerSlot {
    /// Source player-array ordinal.
    pub const fn ordinal(self) -> u8 {
        0
    }
}
impl TryFrom<u8> for PlayerSlot {
    type Error = PlayerBodyError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 0 {
            Ok(Self::One)
        } else {
            Err(PlayerBodyError::Slot)
        }
    }
}
/// Source PST_LIVE/PST_DEAD. Reborn is deliberately not executable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerBodyState {
    /// Source PST_LIVE.
    Live,
    /// Source PST_DEAD.
    Dead,
}
/// Copy of source player fields. Handles retain identity, not liveness leases.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OriginalPlayer {
    /// Source player.mo; retained after death/removal, never rebound on slot reuse.
    pub mo: Option<ActorId>,
    /// Source player.health, clamped at zero independently of signed mobj health.
    pub health: i32,
    /// Source playerstate; death is not actor retirement.
    pub state: PlayerBodyState,
    /// Last applied source, including None; retained if that actor retires.
    pub attacker: Option<ActorId>,
    /// Source damagecount, capped100; no PlayerThink decay in this body-only policy.
    pub damage_count: i32,
    /// Source frags[0]; other player slots are unavailable.
    pub frags: i32,
}
impl Default for OriginalPlayer {
    fn default() -> Self {
        Self {
            mo: None,
            health: INITIAL_PLAYER_HEALTH,
            state: PlayerBodyState::Live,
            attacker: None,
            damage_count: 0,
            frags: 0,
        }
    }
}
/// Checked ownership/settings/behavior rejection, before authoritative mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerBodyError {
    /// Requires the explicitly selected player-body execution policy.
    Profile,
    /// Only source player index0 is admitted.
    Slot,
    /// Missing/stale/pending, wrong definition, duplicate or noninitial binding.
    Binding,
    /// Unsupported controls, inflictor, AI, settings or direct state mutation.
    Unsupported,
    /// Player/body health, state, physics or ownership invariant is inconsistent.
    Invariant,
    /// Source frags would overflow; operation is rejected atomically.
    Statistics,
}
impl fmt::Display for PlayerBodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "player body: {self:?}")
    }
}
impl Error for PlayerBodyError {}
impl From<PlayerBodyError> for SimError {
    fn from(value: PlayerBodyError) -> Self {
        Self::PlayerBody(value)
    }
}
/// Source field/service observations. Presentation requests do not execute devices
/// or weapon psprites, and consume no gameplay RNG.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlayerBodyEvent {
    /// Source association fragment completed after raw mobj spawn.
    Bound(ActorId),
    /// Player fields and I_Tactile request, before mobj health subtraction.
    Damaged {
        /// Clamped source player health.
        health: i32,
        /// Assigned damage source, including NULL.
        attacker: Option<ActorId>,
        /// Capped source damagecount.
        damage_count: i32,
        /// I_Tactile amplitude; period40/duration10 are fixed by the contract.
        tactile: i32,
    },
    /// PST_DEAD/MF_SOLID clearing and P_DropWeapon's lowering request, before
    /// body death-state entry. Psprite animation is explicitly not executed.
    Died {
        /// Source frags[0].
        frags: i32,
        /// Original requested pistol-down state, not an executable mobj state.
        weapon_down: OriginalState,
    },
}
impl Simulation {
    /// Explicit one-player body policy, sharing the original registry and core.
    pub fn original_player_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let registry =
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerBodyLifecycleV1)?;
        Self::with_original_registry(map, limit, registry, ActorOptions::default(), context)
            .map_err(Into::into)
    }
    /// Explicit original-ID policy; the legacy Profile enum denotes the service family.
    pub fn original_execution_profile(&self) -> Option<OriginalExecutionProfile> {
        (self.profile == Profile::OriginalDemon).then_some(if self.original_player.is_some() {
            OriginalExecutionProfile::PlayerBodyLifecycleV1
        } else {
            OriginalExecutionProfile::DemonNormalV1
        })
    }
    /// Read source player fields without resolving, clearing or rebinding references.
    pub fn original_player(&self, _slot: PlayerSlot) -> Result<OriginalPlayer, PlayerBodyError> {
        self.original_player.ok_or(PlayerBodyError::Profile)
    }
    pub(crate) fn owns_player_body(&self, id: ActorId) -> bool {
        self.original_player.is_some_and(|p| p.mo == Some(id))
    }
    pub(crate) fn player_policy(&self) -> OriginalExecutionProfile {
        self.original_execution_profile()
            .unwrap_or(OriginalExecutionProfile::DemonNormalV1)
    }
    pub(crate) fn player_spawn_guard(
        &self,
        ty: ActorTypeId,
        height: SpawnHeight,
    ) -> Result<(), SimError> {
        if self.original_player.is_some()
            && ty.0 == OriginalActorType::MT_PLAYER.ordinal()
            && (height != SpawnHeight::Floor
                || self.original_player.is_some_and(|p| p.mo.is_some())
                || self.actors.order().any(|id| {
                    self.actor(id)
                        .and_then(Actor::life)
                        .is_some_and(|l| l.actor_type == ty)
                }))
        {
            return Err(PlayerBodyError::Binding.into());
        }
        Ok(())
    }
    fn install_player_binding(&mut self, id: ActorId) {
        if let Some(p) = &mut self.original_player {
            p.mo = Some(id);
        }
        self.pending
            .push(Event::PlayerBody(PlayerBodyEvent::Bound(id)));
    }
    /// Checked local binding of one initial genuine body. The full generation,
    /// current definition and source initial state are checked, not just its slot.
    pub fn bind_original_player_body(
        &mut self,
        _slot: PlayerSlot,
        id: ActorId,
    ) -> Result<(), SimError> {
        let p = self.original_player.ok_or(PlayerBodyError::Profile)?;
        let a = self.actor(id).ok_or(PlayerBodyError::Binding)?;
        let l = a.life().ok_or(PlayerBodyError::Binding)?;
        if p.mo.is_some()
            || self.actors.is_pending(id) != Some(false)
            || l.actor_type.0 != OriginalActorType::MT_PLAYER.ordinal()
            || l.state != Some(StateId(OriginalState::S_PLAY.ordinal()))
            || l.health != p.health
            || l.flags != OriginalActorType::MT_PLAYER.descriptor().flags
            || l.tics != OriginalState::S_PLAY.descriptor().tics
            || l.reaction_time != OriginalActorType::MT_PLAYER.descriptor().reaction_time
            || l.threshold != 0
            || l.target.is_some()
            || l.movedir != 0
            || l.movecount != 0
            || a.angle() != 0
            || a.momentum() != [Fixed::ZERO; 2]
            || a.momz != Fixed::ZERO
            || a.z != a.floor
        {
            return Err(PlayerBodyError::Binding.into());
        }
        self.install_player_binding(id);
        Ok(())
    }
    /// Atomic floor spawn and source association, with fixed fresh health100 and
    /// zero orientation. Not a full P_SpawnPlayer/HUD/weapon/respawn implementation.
    pub fn spawn_player_body(
        &mut self,
        _slot: PlayerSlot,
        position: [Fixed; 2],
    ) -> Result<ActorId, SimError> {
        self.original_player.ok_or(PlayerBodyError::Profile)?;
        let id = self.spawn_type(
            ActorTypeId(OriginalActorType::MT_PLAYER.ordinal()),
            position,
            SpawnHeight::Floor,
        )?;
        self.install_player_binding(id);
        Ok(id)
    }
    pub(crate) fn preflight_player_body(&self, stepping: bool) -> Result<(), SimError> {
        let Some(p) = self.original_player else {
            return Ok(());
        };
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            let a = self.actor(id).ok_or(PlayerBodyError::Invariant)?;
            let l = a.life().ok_or(PlayerBodyError::Invariant)?;
            if l.actor_type.0 == OriginalActorType::MT_PLAYER.ordinal() {
                if p.mo != Some(id)
                    || p.health != l.health.max(0)
                    || (p.state == PlayerBodyState::Dead) != (l.health <= 0)
                    || (l.flags & MF_CORPSE != 0) != (l.health <= 0)
                    || (l.flags & MF_SHOOTABLE != 0) != (l.health > 0)
                    || (l.flags & MF_SOLID != 0) != (l.health > 0)
                    || a.height()
                        != Fixed::from_bits(
                            OriginalActorType::MT_PLAYER.descriptor().height
                                >> if l.health > 0 { 0 } else { 2 },
                        )
                    || a.momentum() != [Fixed::ZERO; 2]
                    || a.momz != Fixed::ZERO
                    || a.z != a.floor
                {
                    return Err(PlayerBodyError::Invariant.into());
                }
            } else if stepping && l.actor_type == original_registry::owner_slot() && l.health > 0 {
                // Original all-inactive LookForPlayers does not terminate. Do not
                // replace it with unobserved player AI in this explicit policy.
                return Err(PlayerBodyError::Unsupported.into());
            }
        }
        Ok(())
    }
    pub(crate) fn preflight_player_damage(&self, request: DamageRequest) -> Result<(), SimError> {
        let Some(p) = self.original_player else {
            return Ok(());
        };
        if self.owns_player_body(request.target) {
            if request.inflictor.is_some() {
                return Err(PlayerBodyError::Unsupported.into());
            }
            if let Some(l) = self.actor(request.target).and_then(Actor::life) {
                if l.health > 0
                    && request.amount >= l.health
                    && (request.source.is_none() || request.source == p.mo)
                {
                    p.frags.checked_add(1).ok_or(PlayerBodyError::Statistics)?;
                }
            }
        } else if request.source == p.mo
            && p.mo.is_some()
            && self
                .actor(request.target)
                .and_then(Actor::life)
                .is_some_and(|l| l.health > 0 && request.amount < l.health)
        {
            // Player-to-live-monster retargeting needs the future roster/AI policy.
            return Err(PlayerBodyError::Unsupported.into());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "original_player_tests.rs"]
mod tests;
