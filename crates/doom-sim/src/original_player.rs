//! One genuine source player body; the actor store remains its sole instance owner.
use crate::{identity::*, *};
use std::{error::Error, fmt};

/// Fixed settings/ownership contract, separate from the historical unbound body.
pub const PLAYER_BODY_CONTEXT_SCHEMA: &[u8] = b"PLAYER-BODY-1-MEDIUM-NOTHRUST-v1";
/// Same DSIM22 field layout, explicitly selected grounded physical execution.
pub const PLAYER_KNOCKBACK_CONTEXT_SCHEMA: &[u8] = b"PLAYER-BODY-1-MEDIUM-GROUNDED-v1";
// doomdata.h / p_mobj.c / p_inter.c. Raw fixed-point admission proof, not clamps.
const ML_BLOCKING: u16 = 1;
pub(crate) const MAX_MOVE_BITS: u32 = 30 * 65536;
// Existing bounded direct-damage domain, distinct from the SARG4..40 formula.
const MAX_PLAYER_DAMAGE: i32 = 1000;
const THRUST_FRACTION: i32 = 65536 >> 3;
const THRUST_MASS_SCALE: i32 = 100;
// tables.h ANGLETOFINESHIFT and finecosine=finesine+FINEANGLES/4.
const ANGLE_TO_FINE_SHIFT: u32 = 19;
const FINE_COSINE_OFFSET: usize = 8192 / 4;
// The conservative axis/friction proof is derived in M3G3_REFERENCE.md.
const TRAVEL_NUMERATOR: i64 = 160;
const TRAVEL_DENOMINATOR: i64 = 3;
const MF_NOCLIP: u32 = 0x1000;
const MF_MISSILE: u32 = 0x10000;
const MF_SKULLFLY: u32 = 0x1000000;
const MF_FLOAT: u32 = 0x4000;
const MF_NOGRAVITY: u32 = 0x200;
// p_mobj.h. These describe source behavior, not storage availability.
pub(crate) const MF_SOLID: u32 = 0x2;
pub(crate) const MF_JUSTHIT: u32 = 0x40;
pub(crate) const MF_SHOOTABLE: u32 = 0x4;
pub(crate) const MF_CORPSE: u32 = 0x100000;
// g_game.c G_PlayerReborn/MAXHEALTH, and p_inter.c P_DamageMobj.
pub(crate) const INITIAL_PLAYER_HEALTH: i32 = 100;
pub(crate) const MAX_DAMAGE_COUNT: i32 = 100;
pub(crate) const TACTILE_BASE: i32 = 40;
// Existing shared map/target pre-tic proof, not an original-C coordinate limit.
pub(crate) const PLAYER_BODY_COORDINATE_LIMIT: u32 = 994;

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
    /// Source `frags[0]`; other player slots are unavailable.
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
// Store the selected capability once beside source state. ActorLife remains the
// sole actor definition/state storage; this is not a second player/actor owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PlayerRuntime {
    pub source: OriginalPlayer,
    pub policy: OriginalExecutionProfile,
    pub weapon: Option<crate::player_weapons::PlayerWeaponRuntime>,
    pub pickups: Option<PlayerPickupState>,
    pub armor: Option<PlayerArmorState>,
}
impl std::ops::Deref for PlayerRuntime {
    type Target = OriginalPlayer;
    fn deref(&self) -> &Self::Target {
        &self.source
    }
}
impl std::ops::DerefMut for PlayerRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.source
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
    /// Unsupported controls, physical domain, AI, settings or direct state mutation.
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
    /// body death-state entry. Body-only policies observe the request; weapon
    /// capabilities also execute source psprite lowering.
    Died {
        /// Source `frags[0]`.
        frags: i32,
        /// Original requested ready-weapon downstate, never a mobj state.
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
    /// Opt-in grounded physical player body, sharing damage, movement and thinkers.
    /// Static flat axis walls, zero commands and checked future travel only.
    pub fn original_player_knockback_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let registry =
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerBodyKnockbackV1)?;
        Self::with_original_registry(map, limit, registry, ActorOptions::default(), context)
            .map_err(Into::into)
    }
    /// Explicit original-ID policy; the legacy Profile enum denotes the service family.
    pub fn original_execution_profile(&self) -> Option<OriginalExecutionProfile> {
        (self.profile == Profile::OriginalDemon).then_some(
            self.original_player
                .map_or(OriginalExecutionProfile::DemonNormalV1, |p| p.policy),
        )
    }
    /// Read source player fields without resolving, clearing or rebinding references.
    pub fn original_player(&self, _slot: PlayerSlot) -> Result<OriginalPlayer, PlayerBodyError> {
        self.original_player
            .map(|p| p.source)
            .ok_or(PlayerBodyError::Profile)
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
        position: [Fixed; 2],
        height: SpawnHeight,
    ) -> Result<(), SimError> {
        if self.original_player.is_some()
            && ty.0 == OriginalActorType::MT_PLAYER.ordinal()
            && (position
                .iter()
                .any(|v| !crate::sight::within(*v, PLAYER_BODY_COORDINATE_LIMIT))
                || height != SpawnHeight::Floor
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
            || a.position()
                .iter()
                .any(|v| !crate::sight::within(*v, PLAYER_BODY_COORDINATE_LIMIT))
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
                    || if p.policy.has_physical_player() {
                        travel_bounds(a.position(), a.momentum()).is_err()
                            || l.flags
                                & (MF_NOCLIP | MF_MISSILE | MF_SKULLFLY | MF_FLOAT | MF_NOGRAVITY)
                                != 0
                    } else {
                        a.momentum() != [Fixed::ZERO; 2]
                    }
                    || a.momz != Fixed::ZERO
                    || a.z != a.floor
                    || (p.policy.has_player_commands()
                        && (l.flags & player_commands::MF_JUSTATTACKED != 0
                            || !(0..=i32::from(u8::MAX)).contains(&l.reaction_time)))
                {
                    return Err(PlayerBodyError::Invariant.into());
                }
            } else if p.policy.has_physical_player() && a.momentum() != [Fixed::ZERO; 2] {
                if !p.policy.has_player_weapons()
                    || l.actor_type.0 != OriginalActorType::MT_SERGEANT.ordinal()
                    || a.z != a.floor
                    || a.momz != Fixed::ZERO
                {
                    return Err(PlayerBodyError::Unsupported.into());
                }
                travel_bounds(a.position(), a.momentum())?;
            } else if stepping
                && !self.uses_active_player_roster()
                && l.actor_type == original_registry::owner_slot()
                && l.health > 0
            {
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
        if p.policy == OriginalExecutionProfile::PlayerDemonArmorV1 {
            p.armor.ok_or(PlayerBodyError::Invariant)?.validate()?;
        }
        if p.policy.has_physical_player()
            && request.inflictor.is_some()
            && !self.owns_player_body(request.target)
            && !(p.policy.has_player_weapons()
                && request.source == p.mo
                && request.inflictor == p.mo
                && self
                    .actor(request.target)
                    .and_then(Actor::life)
                    .is_some_and(|l| l.actor_type.0 == OriginalActorType::MT_SERGEANT.ordinal()))
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        if p.policy.has_player_weapons()
            && request.inflictor == p.mo
            && p.mo.is_some()
            && !self.owns_player_body(request.target)
        {
            let target = self.actor(request.target).ok_or(CombatError::Reference)?;
            let origin = self
                .actor(p.mo.ok_or(PlayerBodyError::Binding)?)
                .ok_or(CombatError::Reference)?;
            if target
                .life()
                .is_some_and(|life| life.health > 0 && life.flags & MF_SHOOTABLE != 0)
            {
                preflight_grounded_impulse(
                    target,
                    origin,
                    request.amount,
                    OriginalActorType::MT_SERGEANT.descriptor().mass,
                )?;
            }
        }
        if self.owns_player_body(request.target) {
            if let Some(inflictor) = request.inflictor {
                if !p.policy.has_physical_player() {
                    return Err(PlayerBodyError::Unsupported.into());
                }
                self.preflight_player_thrust(request, inflictor)?;
            }
            if let Some(l) = self.actor(request.target).and_then(Actor::life) {
                if l.health > 0
                    && p.armor
                        .map(|a| ArmorEvent::plan(request.target, request.amount, a))
                        .transpose()?
                        .map_or(request.amount, |a| a.effective)
                        >= l.health
                    && (request.source.is_none() || request.source == p.mo)
                {
                    p.frags.checked_add(1).ok_or(PlayerBodyError::Statistics)?;
                }
            }
        } else if !p.policy.has_player_weapons()
            && request.source == p.mo
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

/// Conservative forward invariant from the source axis-wall/friction proof.
/// Wider temporaries validate support; actual gameplay still uses source Fixed.
pub(crate) fn travel_bounds(
    position: [Fixed; 2],
    momentum: [Fixed; 2],
) -> Result<(), PlayerBodyError> {
    let maximum = momentum
        .into_iter()
        .map(|m| m.bits().unsigned_abs())
        .max()
        .unwrap_or(0);
    if maximum > MAX_MOVE_BITS {
        return Err(PlayerBodyError::Unsupported);
    }
    let travel =
        (TRAVEL_NUMERATOR * i64::from(maximum) + TRAVEL_DENOMINATOR - 1) / TRAVEL_DENOMINATOR;
    let limit = i64::from(PLAYER_BODY_COORDINATE_LIMIT) * 65536;
    if position
        .into_iter()
        .any(|p| i64::from(p.bits()).abs() + travel > limit)
    {
        return Err(PlayerBodyError::Unsupported);
    }
    Ok(())
}
pub(crate) fn validate_grounded_geometry(map: &Map) -> Result<(), SimError> {
    for line in map.linedefs() {
        let [a, b] = line.vertices.map(usize::from);
        let from = map.vertices().get(a).ok_or(PlayerBodyError::Unsupported)?;
        let to = map.vertices().get(b).ok_or(PlayerBodyError::Unsupported)?;
        if line.sides[1].is_some()
            || line.flags != ML_BLOCKING
            || (from.x != to.x && from.y != to.y)
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
    }
    Ok(())
}
impl Simulation {
    fn preflight_player_thrust(
        &self,
        request: DamageRequest,
        inflictor: ActorId,
    ) -> Result<(), SimError> {
        if !(0..=1000).contains(&request.amount) {
            return Err(CombatError::Damage.into());
        }
        let target = self.actor(request.target).ok_or(CombatError::Reference)?;
        let origin = self.actor(inflictor).ok_or(CombatError::Reference)?;
        if self.actors.is_pending(inflictor) != Some(false) {
            return Err(CombatError::Reference.into());
        }
        let original = origin.life().ok_or(PlayerBodyError::Unsupported)?;
        if ![
            OriginalActorType::MT_PLAYER.ordinal(),
            OriginalActorType::MT_SERGEANT.ordinal(),
        ]
        .contains(&original.actor_type.0)
            || origin.z() != Fixed::ZERO
            || origin.vertical_momentum() != Fixed::ZERO
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        let life = target.life().ok_or(PlayerBodyError::Invariant)?;
        if life.health <= 0 || life.flags & MF_SHOOTABLE == 0 {
            return Ok(());
        }
        if life.actor_type.0 != OriginalActorType::MT_PLAYER.ordinal()
            || target.z() != Fixed::ZERO
            || target.vertical_momentum() != Fixed::ZERO
        {
            return Err(PlayerBodyError::Invariant.into());
        }
        // A physical hit cannot repair live, unlisted Demon AI. Require the
        // already admitted thinker closure before committing new momentum.
        self.preflight_player_body(true)?;
        preflight_player_impulse(target, origin, request.amount)
    }
}

/// Shared current-record proof; callers separately establish role/ownership and
/// grounded, no-power settings before using the source physical damage service.
pub(crate) fn preflight_player_impulse(
    target: &Actor,
    origin: &Actor,
    amount: i32,
) -> Result<(), SimError> {
    preflight_grounded_impulse(
        target,
        origin,
        amount,
        OriginalActorType::MT_PLAYER.descriptor().mass,
    )
}
pub(crate) fn preflight_grounded_impulse(
    target: &Actor,
    origin: &Actor,
    amount: i32,
    mass: i32,
) -> Result<(), SimError> {
    if !(0..=MAX_PLAYER_DAMAGE).contains(&amount) {
        return Err(CombatError::Damage.into());
    }
    if mass <= 0
        || target.z != Fixed::ZERO
        || target.momz != Fixed::ZERO
        || origin.z != Fixed::ZERO
        || origin.momz != Fixed::ZERO
    {
        return Err(PlayerBodyError::Unsupported.into());
    }
    let thrust = amount
        .checked_mul(THRUST_FRACTION)
        .and_then(|n| n.checked_mul(THRUST_MASS_SCALE))
        .and_then(|n| n.checked_div(mass))
        .ok_or(CombatError::Metadata)?;
    let mut delta = [Fixed::ZERO; 2];
    for (axis, d) in delta.iter_mut().enumerate() {
        *d = Fixed::from_bits(
            target.position()[axis]
                .bits()
                .checked_sub(origin.position()[axis].bits())
                .ok_or(CombatError::Coordinates)?,
        );
        d.bits().checked_abs().ok_or(CombatError::Coordinates)?;
    }
    let angle = movement::point_angle(delta);
    let phase =
        usize::try_from(angle >> ANGLE_TO_FINE_SHIFT).map_err(|_| CombatError::Coordinates)?;
    let cosine = *FINE_SINE
        .get(phase + FINE_COSINE_OFFSET)
        .ok_or(CombatError::Coordinates)?;
    let sine = *FINE_SINE.get(phase).ok_or(CombatError::Coordinates)?;
    let mut momentum = target.momentum();
    for (axis, component) in [cosine, sine].into_iter().enumerate() {
        let impulse = Fixed::from_bits(thrust) * Fixed::from_bits(component);
        momentum[axis] = Fixed::from_bits(
            momentum[axis]
                .bits()
                .checked_add(impulse.bits())
                .ok_or(CombatError::Coordinates)?,
        );
    }
    travel_bounds(target.position(), momentum)?;
    Ok(())
}

#[cfg(test)]
#[path = "original_player_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "original_player_knockback_tests.rs"]
mod knockback_tests;
