//! Source sprite-dispatched world contacts, using the existing player and store.
use crate::{identity::*, lifecycle::ActorAccess, player_weapons::PlayerWeaponRuntime, *};

/// Explicit pickup capability; historical no-pickup policies remain unchanged.
pub const PLAYER_DEMON_PICKUPS_CONTEXT_SCHEMA: &[u8] = b"PLAYER-DEMON-1-MEDIUM-GROUNDED-PICKUPS-v1";
/// New placed plus controlled-dropped item budget, within the existing64 slots.
pub const MAX_WORLD_PICKUPS: usize = 16;
pub(crate) const MF_COUNTITEM: u32 = 0x800000;
pub(crate) const MF_SPECIAL: u32 = 1;
pub(crate) const MF_COUNTKILL: u32 = 0x0040_0000;
pub(crate) const MF_DROPPED: u32 = 0x0002_0000;
pub(crate) const MF_AMBUSH: u32 = 0x20;
const MF_PICKUP: u32 = 0x800;
const BONUS_ADD: i32 = 6; // p_inter.c BONUSADD
pub(crate) const MAX_BONUS: i32 = BONUS_ADD * MAX_WORLD_PICKUPS as i32;

/// Executable item definitions; each remains its original canonical actor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PickupKind {
    /// MT_CLIP, DoomEd2007.
    Clip,
    /// MT_MISC17, DoomEd2048.
    BulletBox,
    /// MT_MISC22, DoomEd2008.
    Shells,
    /// MT_MISC23, DoomEd2049.
    ShellBox,
    /// MT_MISC10, DoomEd2011.
    Stimpack,
    /// MT_MISC11, DoomEd2012.
    Medikit,
    /// MT_SHOTGUN, DoomEd2001; distinct from MT_SHOTGUY.
    Shotgun,
    /// MT_CHAINGUN, DoomEd2002; distinct from MT_CHAINGUY.
    Chaingun,
    /// Original MT_MISC0, DoomEd2018, animated green armor.
    GreenArmor,
    /// Original MT_MISC1, DoomEd2019, animated blue armor.
    BlueArmor,
    /// Canonical MT_MISC2, DoomEd2014, SPR_BON1.
    HealthBonus,
    /// Canonical MT_MISC3, DoomEd2015, SPR_BON2.
    ArmorBonus,
}
impl PickupKind {
    /// Original definition, never a synthetic pickup alias.
    pub const fn original_type(self) -> OriginalActorType {
        match self {
            Self::Clip => OriginalActorType::MT_CLIP,
            Self::BulletBox => OriginalActorType::MT_MISC17,
            Self::Shells => OriginalActorType::MT_MISC22,
            Self::ShellBox => OriginalActorType::MT_MISC23,
            Self::Stimpack => OriginalActorType::MT_MISC10,
            Self::Medikit => OriginalActorType::MT_MISC11,
            Self::Shotgun => OriginalActorType::MT_SHOTGUN,
            Self::Chaingun => OriginalActorType::MT_CHAINGUN,
            Self::GreenArmor => OriginalActorType::MT_MISC0,
            Self::BlueArmor => OriginalActorType::MT_MISC1,
            Self::HealthBonus => OriginalActorType::MT_MISC2,
            Self::ArmorBonus => OriginalActorType::MT_MISC3,
        }
    }
    fn message(self) -> PickupMessage {
        match self {
            Self::Clip => PickupMessage::Clip,
            Self::BulletBox => PickupMessage::BulletBox,
            Self::Shells => PickupMessage::Shells,
            Self::ShellBox => PickupMessage::ShellBox,
            Self::Stimpack => PickupMessage::Stimpack,
            Self::Medikit => PickupMessage::Medikit,
            Self::Shotgun => PickupMessage::Shotgun,
            Self::Chaingun => PickupMessage::Chaingun,
            Self::GreenArmor => PickupMessage::GreenArmor,
            Self::BlueArmor => PickupMessage::BlueArmor,
            Self::HealthBonus => PickupMessage::HealthBonus,
            Self::ArmorBonus => PickupMessage::ArmorBonus,
        }
    }
    fn can_drop(self) -> bool {
        matches!(self, Self::Clip | Self::Shotgun | Self::Chaingun)
    }
    pub(crate) fn is_bonus(self) -> bool {
        matches!(self, Self::HealthBonus | Self::ArmorBonus)
    }
    pub(crate) fn is_animated(self) -> bool {
        self.is_armor() || self.is_bonus()
    }
    pub(crate) fn definition_flags(self) -> u32 {
        MF_SPECIAL | if self.is_bonus() { MF_COUNTITEM } else { 0 }
    }
    pub(crate) fn is_armor(self) -> bool {
        matches!(self, Self::GreenArmor | Self::BlueArmor)
    }
}
pub(crate) fn kind(ty: ActorTypeId) -> Option<PickupKind> {
    Some(match OriginalActorType::try_from(ty.0).ok()? {
        OriginalActorType::MT_CLIP => PickupKind::Clip,
        OriginalActorType::MT_MISC17 => PickupKind::BulletBox,
        OriginalActorType::MT_MISC22 => PickupKind::Shells,
        OriginalActorType::MT_MISC23 => PickupKind::ShellBox,
        OriginalActorType::MT_MISC10 => PickupKind::Stimpack,
        OriginalActorType::MT_MISC11 => PickupKind::Medikit,
        OriginalActorType::MT_SHOTGUN => PickupKind::Shotgun,
        OriginalActorType::MT_CHAINGUN => PickupKind::Chaingun,
        OriginalActorType::MT_MISC0 => PickupKind::GreenArmor,
        OriginalActorType::MT_MISC1 => PickupKind::BlueArmor,
        OriginalActorType::MT_MISC2 => PickupKind::HealthBonus,
        OriginalActorType::MT_MISC3 => PickupKind::ArmorBonus,
        _ => return None,
    })
}
pub(crate) fn kind_for(profile: OriginalExecutionProfile, ty: ActorTypeId) -> Option<PickupKind> {
    // Historical non-pickup execution must not walk item identity metadata.
    if !profile.has_world_pickups() {
        return None;
    }
    let k = kind(ty)?;
    (!k.is_armor() || profile.has_player_armor())
        .then_some(k)
        .filter(|k| !k.is_bonus() || profile.has_player_bonuses())
}
pub(crate) fn state_allowed(ty: ActorTypeId, state: OriginalState) -> bool {
    kind(ty).is_some_and(|k| {
        let root = k.original_type().states().spawn;
        if k.is_bonus() {
            state
                .ordinal()
                .checked_sub(root.ordinal())
                .is_some_and(|offset| offset < 6)
        } else {
            state == root || (k.is_armor() && state == root.descriptor().next)
        }
    })
}
/// Stable identifiers for actual dstrings.h messages; no HUD timeout is implied.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum PickupMessage {
    /// No source message assigned.
    #[default]
    None = 0,
    /// GOTCLIP.
    Clip = 1,
    /// GOTCLIPBOX.
    BulletBox = 2,
    /// GOTSHELLS.
    Shells = 3,
    /// GOTSHELLBOX.
    ShellBox = 4,
    /// GOTSTIM.
    Stimpack = 5,
    /// GOTMEDIKIT.
    Medikit = 6,
    /// GOTMEDINEED, source metadata only in this positive-health domain.
    MedikitNeeded = 7,
    /// GOTSHOTGUN.
    Shotgun = 8,
    /// GOTCHAINGUN.
    Chaingun = 9,
    /// GOTARMOR, ordinary green armor.
    GreenArmor = 10,
    /// GOTMEGA, blue armor; not a MegaSphere.
    BlueArmor = 11,
    /// GOTHTHBONUS.
    HealthBonus = 12,
    /// GOTARMBONUS.
    ArmorBonus = 13,
}
/// Source pickup-only fields; health, body and inventory retain their owners.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlayerPickupState {
    /// Source bonuscount; live admitted fragment decrements before thinkers.
    pub bonus_count: i32,
    /// Source itemcount; the admitted definitions lack MF_COUNTITEM.
    pub item_count: i32,
    /// Last assigned source message, without presentation-side clearing.
    pub message: PickupMessage,
}
impl PlayerPickupState {
    fn validate(self, profile: OriginalExecutionProfile) -> Result<(), SimError> {
        if !(0..=MAX_BONUS).contains(&self.bonus_count)
            || if profile.has_player_bonuses() {
                !(0..=MAX_WORLD_PICKUPS as i32).contains(&self.item_count)
            } else {
                self.item_count != 0
            }
            || self.message == PickupMessage::MedikitNeeded
            || (!profile.has_player_bonuses()
                && matches!(
                    self.message,
                    PickupMessage::HealthBonus | PickupMessage::ArmorBonus
                ))
            || (!profile.has_player_armor()
                && matches!(
                    self.message,
                    PickupMessage::GreenArmor | PickupMessage::BlueArmor
                ))
        {
            return Err(PlayerBodyError::Invariant.into());
        }
        Ok(())
    }
}
/// Source touch return classification, independent of handle lifetime policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PickupOutcome {
    /// Vertical reach rejected.
    OutOfReach = 0,
    /// Toucher health is nonpositive.
    Dead = 1,
    /// The resource was already full/owned with no ammo benefit.
    NotNeeded = 2,
    /// Resource granted and item marked for deferred retirement.
    Collected = 3,
}
/// Ordered source observations; DSIM25 event23 subtypes0–6, DSIM26 also7.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PickupEvent {
    /// Item/body/sprite, vertical delta, body height and signed health.
    Touch(ActorId, ActorId, u16, Fixed, Fixed, i32),
    /// Ammo kind, accepted, before/after, current ready and resulting pending.
    Ammo(u8, bool, i32, i32, Weapon, Option<Weapon>),
    /// Accepted and player before/after plus resulting mobj health.
    Body(bool, i32, i32, i32),
    /// Weapon, accepted, ownership before/after, pending before/after.
    Weapon(Weapon, bool, u16, u16, Option<Weapon>, Option<Weapon>),
    /// Assigned actual source message.
    Message(PickupMessage),
    /// Resulting source bonuscount and itemcount.
    Bonus(i32, i32),
    /// Actual source touch return classification.
    Result(ActorId, PickupOutcome),
    /// Requested source class, accepted, armor before and after P_GiveArmor.
    Armor(ArmorType, bool, PlayerArmorState, PlayerArmorState),
    /// Direct BON1 effect, without entering P_GiveBody.
    HealthBonus(i32, i32, i32),
    /// Direct BON2 effect; ordinary armor grant thresholds remain separate.
    ArmorBonus(PlayerArmorState, PlayerArmorState),
    /// Original COUNTITEM increment, after message and before source removal.
    Counted(i32, i32),
}
pub(crate) enum ContactEvent {
    Pickup(PickupEvent),
    Removed(ActorId),
    Sound(u16),
}
impl ContactEvent {
    pub(crate) fn event(self) -> Event {
        match self {
            Self::Pickup(e) => Event::Pickup(e),
            Self::Removed(id) => Event::RemovalMarked(id),
            Self::Sound(sound) => Event::ActorState(ActorStateEvent::Sound { actor: None, sound }),
        }
    }
}
/// Pure function of the type ordinal and the compile-time identity tables, so its
/// result is computed once per process and reused by every later preflight.
pub(crate) fn validate_definition(ty: ActorTypeId) -> Result<(), SimError> {
    static MEMO: [std::sync::OnceLock<Result<(), SimError>>; 256] =
        [const { std::sync::OnceLock::new() }; 256];
    match MEMO.get(usize::from(ty.0)) {
        Some(cell) => *cell.get_or_init(|| validate_definition_uncached(ty)),
        None => validate_definition_uncached(ty),
    }
}
fn validate_definition_uncached(ty: ActorTypeId) -> Result<(), SimError> {
    let k = kind(ty).ok_or(ActorError::InvalidType)?;
    let d = k.original_type().descriptor();
    let s = d.states.spawn.descriptor();
    let timing = if k.is_bonus() {
        let mut state = d.states.spawn;
        let mut valid = true;
        for frame in [0, 1, 2, 3, 2, 1] {
            let descriptor = state.descriptor();
            valid &= descriptor.tics == 6
                && descriptor.sprite == s.sprite
                && descriptor.frame == frame
                && descriptor.action.is_none();
            state = descriptor.next;
        }
        valid && state == d.states.spawn
    } else if k.is_armor() {
        let next = s.next.descriptor();
        s.tics > 0
            && next.tics > 0
            && next.next == d.states.spawn
            && next.sprite == s.sprite
            && next.action.is_none()
    } else {
        s.tics == -1 && s.next == OriginalState::S_NULL
    };
    if d.flags != k.definition_flags() || !timing || s.action.is_some() {
        return Err(ActorError::InvalidType.into());
    }
    Ok(())
}
pub(crate) fn validate_item(
    room: Option<crate::player_map_scale::RoomBounds>,
    a: Actor,
) -> Result<(), SimError> {
    let l = a.life().ok_or(ActorError::InvalidType)?;
    let k = kind(l.actor_type).ok_or(ActorError::InvalidType)?;
    validate_definition(l.actor_type)?;
    let state = l
        .state
        .and_then(|v| OriginalState::try_from(v.0).ok())
        .ok_or(ActorError::InvalidState)?;
    let s = state.descriptor();
    let dropped = l.flags & MF_DROPPED != 0;
    if !state_allowed(l.actor_type, state)
        || l.sprite != s.sprite.ordinal()
        || l.frame != s.frame
        || (if k.is_animated() {
            !(1..=s.tics).contains(&l.tics)
        } else {
            l.tics != -1
        })
        || l.health != 1000
        || l.target.is_some()
        || l.reaction_time != 8
        || l.threshold != 0
        || l.movedir != 0
        || l.movecount != 0
        || l.death_requested
        || a.lastlook() > 3
        || a.radius() != Fixed::from_map_units(20)
        || a.height() != Fixed::from_map_units(16)
        || a.momentum() != [Fixed::ZERO; 2]
        || a.momz != Fixed::ZERO
        || a.z != a.floor
        || a.floor != Fixed::ZERO
        || a.ceiling != Fixed::from_map_units(128)
        || !crate::player_map_scale::position_admitted(room, a.position(), 978)
        || l.flags & !(k.definition_flags() | MF_AMBUSH | MF_DROPPED) != 0
        || l.flags & k.definition_flags() != k.definition_flags()
        || (dropped && (!k.can_drop() || l.spawnpoint.is_some() || l.flags & MF_AMBUSH != 0))
        || (!dropped
            && l.spawnpoint.is_none_or(|p| {
                i32::from(p.kind) != k.original_type().descriptor().doomed_number
                    || a.position()
                        != [
                            Fixed::from_map_units(p.position.x),
                            Fixed::from_map_units(p.position.y),
                        ]
            }))
    {
        return Err(PlayerBodyError::Invariant.into());
    }
    Ok(())
}
pub(crate) fn bonus_tick(
    player: Option<&mut original_player::PlayerRuntime>,
) -> Result<(), SimError> {
    if let Some(p) = player {
        if p.policy.has_world_pickups() {
            let live = p.state == PlayerBodyState::Live;
            let state = p.pickups.as_mut().ok_or(PlayerBodyError::Invariant)?;
            state.validate(p.policy)?;
            if live && state.bonus_count > 0 {
                state.bonus_count -= 1;
            }
        }
    }
    Ok(())
}
fn observe(events: &mut Vec<movement::MotionEvent>, event: PickupEvent) {
    events.push(movement::MotionEvent::Contact(ContactEvent::Pickup(event)));
}
fn ammo(
    p: &mut PlayerWeaponState,
    shell: bool,
    loads: u16,
    events: &mut Vec<movement::MotionEvent>,
) -> Result<bool, SimError> {
    let before = if shell { p.shells } else { p.bullets };
    let cap = u16::try_from(
        *weapons::AMMO_CAPACITIES
            .get(usize::from(shell))
            .ok_or(WeaponError::Inventory)?,
    )
    .map_err(|_| WeaponError::Inventory)?;
    if before > cap {
        return Err(WeaponError::Inventory.into());
    }
    let accepted = before != cap;
    if accepted {
        let count = if loads == 0 {
            5
        } else {
            loads
                .checked_mul(if shell { 4 } else { 10 })
                .ok_or(WeaponError::Inventory)?
        };
        // Private callers supply at most five loads: before<=200 and count<=50,
        // so the checked source addition is representable before capping.
        let after = before
            .checked_add(count)
            .ok_or(WeaponError::Inventory)?
            .min(cap);
        if shell {
            p.shells = after;
        } else {
            p.bullets = after;
        }
        if before == 0 {
            if shell
                && matches!(p.ready, Weapon::Fist | Weapon::Pistol)
                && p.owned & (1 << Weapon::Shotgun as u8) != 0
            {
                p.pending = Some(Weapon::Shotgun);
            }
            if !shell && p.ready == Weapon::Fist {
                p.pending = Some(if p.owned & (1 << Weapon::Chaingun as u8) != 0 {
                    Weapon::Chaingun
                } else {
                    Weapon::Pistol
                });
            }
        }
    }
    observe(
        events,
        PickupEvent::Ammo(
            u8::from(shell),
            accepted,
            i32::from(before),
            i32::from(if shell { p.shells } else { p.bullets }),
            p.ready,
            p.pending,
        ),
    );
    Ok(accepted)
}
/// Same actor access and player state used by the shared thinker transaction.
pub(crate) struct Contacts<'a, A> {
    pub access: &'a mut A,
    pub player: &'a mut original_player::PlayerRuntime,
}
impl<A: ActorAccess> movement::MotionAccess for Contacts<'_, A> {
    type Error = SimError;
    fn get(&self, id: ActorId) -> Option<Actor> {
        self.access.get(id).copied()
    }
    fn contacts(
        &mut self,
        id: ActorId,
        body: &mut Actor,
        attempt: &ActorMoveAttempt,
        spatial: &mut movement::SpatialIndex,
        events: &mut Vec<movement::MotionEvent>,
    ) -> Result<(), SimError> {
        if self.player.mo != Some(id) || !self.player.policy.has_world_pickups() {
            return Err(PlayerBodyError::Binding.into());
        }
        for &item_id in &attempt.actors {
            if item_id == id {
                continue;
            }
            let item = self.get(item_id).ok_or(StoreError::InvalidHandle)?;
            let l = item.life().ok_or(ActorError::InvalidType)?;
            if l.flags & MF_SPECIAL == 0 {
                continue;
            }
            let distance = i64::from(body.radius().bits()) + i64::from(item.radius().bits());
            if (0..2).any(|axis| {
                (i64::from(item.position()[axis].bits()) - i64::from(attempt.target[axis].bits()))
                    .abs()
                    >= distance
            }) {
                continue;
            }
            if body.life().is_none_or(|v| v.flags & MF_PICKUP == 0) {
                continue;
            }
            validate_item(self.player.room, item)?;
            let k = kind_for(self.player.policy, l.actor_type).ok_or(ActorError::InvalidType)?;
            let delta = item.z - body.z;
            let health = body.life().ok_or(ActorError::InvalidType)?.health;
            observe(
                events,
                PickupEvent::Touch(item_id, id, l.sprite, delta, body.height(), health),
            );
            let mut outcome = if delta > body.height() || delta < Fixed::from_map_units(-8) {
                PickupOutcome::OutOfReach
            } else if health <= 0 {
                PickupOutcome::Dead
            } else {
                PickupOutcome::NotNeeded
            };
            if outcome == PickupOutcome::NotNeeded {
                let weapon = self.player.weapon.as_mut().ok_or(WeaponError::Player)?;
                let PlayerWeaponRuntime::Pack(w) = weapon else {
                    return Err(WeaponError::Inventory.into());
                };
                let accepted = match k {
                    PickupKind::HealthBonus => {
                        let before = self.player.health;
                        let after = before
                            .checked_add(1)
                            .ok_or(PlayerBodyError::Invariant)?
                            .min(200);
                        self.player.health = after;
                        body.life.as_mut().ok_or(ActorError::InvalidType)?.health = after;
                        self.access
                            .get_mut(id)
                            .and_then(|a| a.life.as_mut())
                            .ok_or(StoreError::InvalidHandle)?
                            .health = after;
                        observe(events, PickupEvent::HealthBonus(before, after, after));
                        true
                    }
                    PickupKind::ArmorBonus => {
                        let policy = self.player.policy;
                        let armor = self
                            .player
                            .armor
                            .as_mut()
                            .ok_or(PlayerBodyError::Invariant)?;
                        armor.validate_for(policy)?;
                        let before = *armor;
                        armor.points = armor
                            .points
                            .checked_add(1)
                            .ok_or(PlayerBodyError::Invariant)?
                            .min(200);
                        if armor.kind == ArmorType::None {
                            armor.kind = ArmorType::Green;
                        }
                        armor.validate_for(policy)?;
                        observe(events, PickupEvent::ArmorBonus(before, *armor));
                        true
                    }
                    PickupKind::GreenArmor | PickupKind::BlueArmor => {
                        let requested = if k == PickupKind::GreenArmor {
                            ArmorType::Green
                        } else {
                            ArmorType::Blue
                        };
                        let armor = self
                            .player
                            .armor
                            .as_mut()
                            .ok_or(PlayerBodyError::Invariant)?;
                        let before = *armor;
                        let accepted = armor.grant_for(requested, self.player.policy)?;
                        observe(
                            events,
                            PickupEvent::Armor(requested, accepted, before, *armor),
                        );
                        accepted
                    }
                    PickupKind::Clip => ammo(
                        w,
                        false,
                        if l.flags & MF_DROPPED != 0 { 0 } else { 1 },
                        events,
                    )?,
                    PickupKind::BulletBox => ammo(w, false, 5, events)?,
                    PickupKind::Shells => ammo(w, true, 1, events)?,
                    PickupKind::ShellBox => ammo(w, true, 5, events)?,
                    PickupKind::Stimpack | PickupKind::Medikit => {
                        let before = self.player.health;
                        let accepted = before < 100;
                        if accepted {
                            let after = before
                                .checked_add(if k == PickupKind::Stimpack { 10 } else { 25 })
                                .ok_or(PlayerBodyError::Invariant)?
                                .min(100);
                            self.player.health = after;
                            body.life.as_mut().ok_or(ActorError::InvalidType)?.health = after;
                            self.access
                                .get_mut(id)
                                .and_then(|a| a.life.as_mut())
                                .ok_or(StoreError::InvalidHandle)?
                                .health = after;
                        }
                        observe(
                            events,
                            PickupEvent::Body(
                                accepted,
                                before,
                                self.player.health,
                                body.life().ok_or(ActorError::InvalidType)?.health,
                            ),
                        );
                        accepted
                    }
                    PickupKind::Shotgun | PickupKind::Chaingun => {
                        let weapon = if k == PickupKind::Shotgun {
                            Weapon::Shotgun
                        } else {
                            Weapon::Chaingun
                        };
                        let before = w.owned;
                        let pending = w.pending;
                        let gave = ammo(
                            w,
                            weapon == Weapon::Shotgun,
                            if l.flags & MF_DROPPED != 0 { 1 } else { 2 },
                            events,
                        )?;
                        let fresh = before & (1 << weapon as u8) == 0;
                        if fresh {
                            w.owned |= 1 << weapon as u8;
                            w.pending = Some(weapon);
                        }
                        observe(
                            events,
                            PickupEvent::Weapon(
                                weapon,
                                gave || fresh,
                                before,
                                w.owned,
                                pending,
                                w.pending,
                            ),
                        );
                        gave || fresh
                    }
                };
                if accepted {
                    let state = self
                        .player
                        .pickups
                        .as_mut()
                        .ok_or(PlayerBodyError::Invariant)?;
                    state.message = k.message();
                    observe(events, PickupEvent::Message(state.message));
                    if l.flags & MF_COUNTITEM != 0 {
                        if !self.player.policy.has_player_bonuses() {
                            return Err(PlayerBodyError::Invariant.into());
                        }
                        let before = state.item_count;
                        state.item_count =
                            before.checked_add(1).ok_or(PlayerBodyError::Invariant)?;
                        observe(events, PickupEvent::Counted(before, state.item_count));
                    }
                    if !self.access.mark(item_id)? {
                        return Err(StoreError::InvalidHandle.into());
                    }
                    spatial.unlink(item_id);
                    events.push(movement::MotionEvent::Contact(ContactEvent::Removed(
                        item_id,
                    )));
                    state.bonus_count = state
                        .bonus_count
                        .checked_add(BONUS_ADD)
                        .ok_or(PlayerBodyError::Invariant)?;
                    state.validate(self.player.policy)?;
                    observe(
                        events,
                        PickupEvent::Bonus(state.bonus_count, state.item_count),
                    );
                    let sound = if matches!(k, PickupKind::Shotgun | PickupKind::Chaingun) {
                        "sfx_wpnup"
                    } else {
                        "sfx_itemup"
                    };
                    events.push(movement::MotionEvent::Contact(ContactEvent::Sound(
                        lifecycle::source_sound(sound).ok_or(ActorError::InvalidType)?,
                    )));
                    outcome = PickupOutcome::Collected;
                }
            }
            observe(events, PickupEvent::Result(item_id, outcome));
        }
        Ok(())
    }
}
impl Simulation {
    /// One genuine player/Demon world with source ammo, health and weapon items.
    pub fn original_player_pickups_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        Self::with_original_registry(
            map,
            limit,
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonPickupsV1)?,
            ActorOptions::default(),
            context,
        )
        .map_err(Into::into)
    }
    /// Source pickup observations; no mutable player/inventory alias is exposed.
    pub fn original_player_pickups(
        &self,
        _slot: PlayerSlot,
    ) -> Result<PlayerPickupState, PlayerBodyError> {
        self.original_player
            .as_ref()
            .and_then(|p| p.pickups)
            .ok_or(PlayerBodyError::Profile)
    }
    pub(crate) fn preflight_world_pickups(&self) -> Result<(), SimError> {
        if !self.player_policy().has_world_pickups() {
            return Ok(());
        }
        self.original_player
            .as_ref()
            .and_then(|p| p.pickups)
            .ok_or(PlayerBodyError::Invariant)?
            .validate(self.player_policy())?;
        if self.player_policy().has_player_armor() {
            self.original_player_armor(PlayerSlot::One)?;
        }
        let mut count = 0;
        let mut counted = 0;
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            let a = *self.actor(id).ok_or(StoreError::InvalidHandle)?;
            if a.life()
                .is_some_and(|l| kind_for(self.player_policy(), l.actor_type).is_some())
            {
                count += 1;
                counted += usize::from(a.life().is_some_and(|l| l.flags & MF_COUNTITEM != 0));
                validate_item(self.room(), a)?;
            }
        }
        if self.player_policy().has_player_bonuses() {
            let total = self
                .statistics
                .ok_or(PlayerBodyError::Invariant)?
                .total_items;
            let collected = self
                .original_player
                .as_ref()
                .and_then(|p| p.pickups)
                .ok_or(PlayerBodyError::Invariant)?
                .item_count;
            if !(0..=MAX_WORLD_PICKUPS as i32).contains(&total)
                || collected > total
                || counted > (total - collected) as usize
                || (total > 0 && total as usize != counted_map_things(self.world.map().things()))
            {
                return Err(PlayerBodyError::Invariant.into());
            }
        }
        if count > max_world_items(self.player_policy()) {
            return Err(MapSpawnError::WorkLimit.into());
        }
        Ok(())
    }
    /// Controlled actual dropped Clip/Shotgun/Chaingun spawn, before body/tic0.
    /// This is setup, not monster loot, respawn or dynamic item creation.
    pub fn spawn_dropped_pickup(
        &mut self,
        k: PickupKind,
        position: [Fixed; 2],
    ) -> Result<ActorId, SimError> {
        if !self.player_policy().has_world_pickups()
            || self.tic != 0
            || self.original_player.as_ref().is_none_or(|p| p.mo.is_some())
            || !k.can_drop()
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.preflight_world_pickups()?;
        let items = self
            .actors
            .order()
            .filter(|id| {
                self.actor(*id)
                    .and_then(Actor::life)
                    .is_some_and(|l| kind_for(self.player_policy(), l.actor_type).is_some())
            })
            .count();
        if items >= MAX_WORLD_PICKUPS {
            return Err(MapSpawnError::WorkLimit.into());
        }
        if !self.spawn_admitted(
            position,
            Fixed::from_bits(k.original_type().descriptor().radius),
            978,
        ) {
            return Err(MonsterError::Arithmetic.into());
        }
        self.reserve_actor_spawns(1)?;
        let id = lifecycle::LifeHost {
            player: None,
            statistics: self.statistics.as_mut(),
            perception: None,
            access: &mut self.actors,
            registry: self.registry.as_deref().ok_or(ActorError::InvalidType)?,
            world: &self.world,
            sectors: &self.sectors,
            options: self.actor_options,
            rng: &mut self.rng,
            spatial: &mut self.spatial,
            bindings: &mut self.bindings,
            events: &mut self.pending,
        }
        .spawn(
            ActorTypeId(k.original_type().ordinal()),
            position,
            SpawnHeight::Floor,
        );
        self.actors
            .get_mut(id)
            .and_then(|a| a.life.as_mut())
            .ok_or(StoreError::InvalidHandle)?
            .flags |= MF_DROPPED;
        Ok(id)
    }
}

/// Live world-item bound: placed/controlled items plus, under PlayerMonstersV1,
/// at most one P_KillMobj drop per admitted roster monster.
pub(crate) const fn max_world_items(policy: OriginalExecutionProfile) -> usize {
    if policy.has_monsters() {
        MAX_WORLD_PICKUPS + crate::MAX_PLAYER_MONSTERS
    } else {
        MAX_WORLD_PICKUPS
    }
}
/// DoomEd numbers of the COUNTITEM health/armor bonuses (info.c MT_MISC2/MT_MISC3).
pub(crate) const COUNTED_ITEM_DOOMED: [i16; 2] = [2014, 2015];
/// DoomEd numbers of the admitted COUNTKILL monsters (info.c MT_SERGEANT,
/// MT_POSSESSED, MT_SHOTGUY, MT_CHAINGUY).
pub(crate) const COUNTED_KILL_DOOMED: [i16; 4] = [3002, 3004, 9, 65];
/// The same set plus MT_TROOP under `PlayerMonstersImpV1`.
pub(crate) const COUNTED_KILL_DOOMED_IMPS: [i16; 5] = [3001, 3002, 3004, 9, 65];
/// The single source placement filter for counted THINGs: p_mobj.c
/// P_SpawnMapThing skips `options & 16` (MTF_NOTSINGLE) in single player and
/// requires the Medium skill bit `options & 2`. Every caller (runtime
/// preflight and DSIM readers) counts through this one definition. Actor
/// admission itself stays in the existing batch validator.
pub(crate) fn counted_things(things: impl IntoIterator<Item = (i16, u16)>, kinds: &[i16]) -> usize {
    things
        .into_iter()
        .filter(|(kind, options)| kinds.contains(kind) && options & 16 == 0 && options & 2 != 0)
        .count()
}
pub(crate) fn counted_map_things(things: &[doom_map::Thing]) -> usize {
    counted_things(
        things.iter().map(|t| (t.kind, t.options)),
        &COUNTED_ITEM_DOOMED,
    )
}
