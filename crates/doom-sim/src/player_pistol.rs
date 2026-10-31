//! Canonical player weapon state, using the shared psprite/trace/damage services.
use crate::{
    identity::*,
    lifecycle::{ActorAccess, LifeHost},
    *,
};

/// DSIM23 policy7: fixed genuine player/Demon roster with bounded pistol combat.
pub const PLAYER_DEMON_PISTOL_CONTEXT_SCHEMA: &[u8] = b"PLAYER-DEMON-1-MEDIUM-GROUNDED-PISTOL-v1";
// p_pspr.c P_BulletSlope/P_GunShot and source weapon ownership bits.
const AUTOAIM_UNITS: i16 = 1024;
const BULLET_UNITS: i16 = 2048;
const MELEE_UNITS: i16 = 64;
const AUTOAIM_OFFSET: u32 = 1 << 26;
const OWNED_FIST_PISTOL: u16 = (1 << Weapon::Fist as u8) | (1 << Weapon::Pistol as u8);
/// Source initial ammunition, from G_PlayerReborn (not pickup gameplay).
pub const INITIAL_PISTOL_BULLETS: u16 = 50;
/// Dynamic source weapon fields only. The existing player owns identity/health.
/// Returned copies cannot change simulation ownership or ammunition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PistolState {
    /// Source am_clip, bounded by the original capacity200.
    pub bullets: u16,
    /// Source attackdown.
    pub attack_down: bool,
    /// Source consecutive refire count.
    pub refire: i32,
    /// Source pre-friction momentum bob; view/camera remain excluded.
    pub bob: Fixed,
    /// Source weapon light state.
    pub extra_light: i32,
    /// Original weapon then flash psprites, never actor instances.
    pub psprites: [Psprite; 2],
}
impl PistolState {
    pub(crate) fn validate(self) -> Result<(), SimError> {
        if self.bullets > 200
            || !(0..=200).contains(&self.refire)
            || !(0..=weapons::MAX_BOB_BITS).contains(&self.bob.bits())
            || !(0..=1).contains(&self.extra_light)
        {
            return Err(WeaponError::Inventory.into());
        }
        for (channel, ps) in self.psprites.into_iter().enumerate() {
            if !(-15 * 65536..=17 * 65536).contains(&ps.position[0].bits())
                || !(0..=128 * 65536).contains(&ps.position[1].bits())
            {
                return Err(WeaponError::Definition.into());
            }
            let limit = if channel == 0 {
                let state = ps
                    .state
                    .and_then(|s| OriginalState::try_from(s.0).ok())
                    .ok_or(WeaponError::Definition)?;
                if !matches!(
                    state,
                    OriginalState::S_PISTOL
                        | OriginalState::S_PISTOLUP
                        | OriginalState::S_PISTOLDOWN
                        | OriginalState::S_PISTOL1
                        | OriginalState::S_PISTOL2
                        | OriginalState::S_PISTOL3
                        | OriginalState::S_PISTOL4
                ) {
                    return Err(WeaponError::Definition.into());
                }
                state.descriptor().tics
            } else if ps.state.is_none() {
                if ps.tics != 0 || self.extra_light != 0 {
                    return Err(WeaponError::Definition.into());
                }
                continue;
            } else if ps.state == Some(StateId(OriginalState::S_PISTOLFLASH.ordinal()))
                && self.extra_light == 1
            {
                OriginalState::S_PISTOLFLASH.descriptor().tics
            } else {
                return Err(WeaponError::Definition.into());
            };
            if !(1..=limit).contains(&ps.tics) {
                return Err(WeaponError::Definition.into());
            }
        }
        Ok(())
    }
    pub(crate) fn initial(bullets: u16) -> Self {
        Self {
            bullets,
            attack_down: true,
            refire: 0,
            bob: Fixed::ZERO,
            extra_light: 0,
            psprites: [Psprite::default(); 2],
        }
    }
    pub(crate) fn adapter(self, id: ActorId, health: i32) -> WeaponPlayer {
        WeaponPlayer {
            slot: PlayerSlot::One.ordinal(),
            actor: id,
            health,
            ready: Weapon::Pistol,
            pending: None,
            owned: OWNED_FIST_PISTOL,
            ammo: [i32::from(self.bullets), 0, 0, 0],
            max_ammo: weapons::AMMO_CAPACITIES,
            attack_down: self.attack_down,
            refire: self.refire,
            bob: self.bob,
            extra_light: self.extra_light,
            psprites: self.psprites,
        }
    }
    pub(crate) fn from_adapter(value: WeaponPlayer, id: ActorId) -> Result<Self, SimError> {
        if value.actor != id
            || value.slot != PlayerSlot::One.ordinal()
            || value.ready != Weapon::Pistol
            || value.pending.is_some()
            || value.owned != OWNED_FIST_PISTOL
            || value.ammo[1..] != [0, 0, 0]
            || !(0..=200).contains(&value.ammo[0])
            || !(0..=200).contains(&value.refire)
        {
            return Err(WeaponError::Inventory.into());
        }
        Ok(Self {
            bullets: u16::try_from(value.ammo[0]).map_err(|_| WeaponError::Inventory)?,
            attack_down: value.attack_down,
            refire: value.refire,
            bob: value.bob,
            extra_light: value.extra_light,
            psprites: value.psprites,
        })
    }
}

impl Simulation {
    /// One genuine player, at most one map Demon, static flat axis-wall geometry,
    /// shared autonomous melee and pistol. Setup must explicitly initialize pistol.
    pub fn original_player_demon_pistol_map(
        map: Map,
        limit: u32,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let registry =
            OriginalExecutionRegistry::new(OriginalExecutionProfile::PlayerDemonPistolV1)?;
        Self::with_original_registry(map, limit, registry, ActorOptions::default(), context)
            .map_err(Into::into)
    }
    /// Source-equivalent fresh pistol setup after body binding, before any tic.
    /// Normal G_PlayerReborn uses50. Other0..200 values are explicit controlled
    /// inventory inputs; fist fallback, pickups and other weapons are unavailable.
    pub fn initialize_player_pistol(
        &mut self,
        _slot: PlayerSlot,
        bullets: u16,
    ) -> Result<(), SimError> {
        let player = self
            .original_player
            .as_ref()
            .ok_or(PlayerBodyError::Profile)?;
        if player.policy != OriginalExecutionProfile::PlayerDemonPistolV1 {
            return Err(PlayerBodyError::Profile.into());
        }
        if self.tic != 0
            || player.weapon.is_some()
            || bullets > 200
            || player.state != PlayerBodyState::Live
            || player.health != original_player::INITIAL_PLAYER_HEALTH
        {
            return Err(WeaponError::Inventory.into());
        }
        self.checked_demon_operation(|host| {
            let result = run(
                host,
                weapons::PistolOperation::Setup,
                TicCommand::default(),
                0,
                None,
                None,
                Some(player_weapons::PlayerWeaponRuntime::Pistol(
                    PistolState::initial(bullets),
                )),
            );
            if let Err(error) = result {
                host.fail_sight(error);
            }
            result
        })?
    }
    /// Read only weapon fields; the genuine player's existing body remains owner.
    pub fn original_player_pistol(
        &self,
        _slot: PlayerSlot,
    ) -> Result<Option<PistolState>, PlayerBodyError> {
        self.original_player
            .as_ref()
            .filter(|p| p.policy == OriginalExecutionProfile::PlayerDemonPistolV1)
            .map(|p| p.weapon.and_then(|w| w.pistol()))
            .ok_or(PlayerBodyError::Profile)
    }
}

type Fire<A> = fn(&mut LifeHost<'_, A>, ActorId, Weapon, bool) -> Result<(), SimError>;
struct Host<'a, 'b, A> {
    life: &'a mut LifeHost<'b, A>,
    id: ActorId,
    fire: Option<Fire<A>>,
    sound: Option<&'a mut Option<ActorId>>,
}
impl<A: ActorAccess> weapons::Host for Host<'_, '_, A> {
    fn actor(&self) -> Result<Actor, SimError> {
        self.life
            .access
            .get(self.id)
            .copied()
            .ok_or(WeaponError::Reference.into())
    }
    fn health(&self) -> i32 {
        self.life.player.as_ref().map_or(0, |p| p.health)
    }
    fn dead(&self) -> bool {
        self.life
            .player
            .as_ref()
            .is_some_and(|p| p.state == PlayerBodyState::Dead)
    }
    fn admit_weapon(&self, weapon: Weapon) -> Result<(), SimError> {
        if weapon == Weapon::Pistol
            || (self
                .life
                .player
                .as_ref()
                .is_some_and(|p| p.policy == OriginalExecutionProfile::PlayerDemonWeaponsV1)
                && matches!(weapon, Weapon::Fist | Weapon::Shotgun | Weapon::Chaingun))
        {
            Ok(())
        } else {
            Err(WeaponError::UnsupportedWeapon.into())
        }
    }
    fn motion(&mut self, _command: TicCommand) -> Result<(), SimError> {
        // Canonical commands already ran in the one authoritative player stage.
        Err(PlayerBodyError::Unsupported.into())
    }
    fn body_state(&mut self, state: StateId) -> Result<(), SimError> {
        original_registry::state_role_for(
            self.life
                .player
                .as_ref()
                .ok_or(PlayerBodyError::Profile)?
                .policy,
            ActorTypeId(OriginalActorType::MT_PLAYER.ordinal()),
            state,
        )
        .map_err(|_| WeaponError::Definition)?;
        self.life.enter(self.id, state);
        if self.life.sight_failed() {
            return Err(PlayerBodyError::Unsupported.into());
        }
        Ok(())
    }
    fn event(&mut self, event: WeaponEvent) {
        self.life.events.push(Event::Weapon(event));
    }
    fn sound(&mut self, sound: u16) {
        self.life
            .events
            .push(Event::ActorState(ActorStateEvent::Sound {
                actor: Some(self.id),
                sound,
            }));
    }
    fn fire(&mut self, weapon: Weapon, accurate: bool) -> Result<(), SimError> {
        self.admit_weapon(weapon)?;
        self.fire.ok_or(WeaponError::UnsupportedWeapon)?(self.life, self.id, weapon, accurate)
    }
    fn noise(&mut self) -> Result<(), SimError> {
        let sector = self.actor()?.location.ok_or(WeaponError::Reference)?.sector;
        if sector != 0 || self.life.sectors.len() != 1 {
            return Err(PlayerBodyError::Unsupported.into());
        }
        *self
            .sound
            .as_deref_mut()
            .ok_or(PlayerBodyError::Unsupported)? = Some(self.id);
        self.event(WeaponEvent::Noise {
            sector,
            blocks: 0,
            target: self.id,
        });
        Ok(())
    }
}

pub(crate) fn run<A: ActorAccess>(
    host: &mut LifeHost<'_, A>,
    operation: weapons::PistolOperation,
    command: TicCommand,
    leveltime: u64,
    fire: Option<Fire<A>>,
    sound: Option<&mut Option<ActorId>>,
    setup: Option<player_weapons::PlayerWeaponRuntime>,
) -> Result<(), SimError> {
    let player = host.player.as_deref().ok_or(PlayerBodyError::Profile)?;
    if !player.policy.has_player_weapons() {
        return Err(PlayerBodyError::Profile.into());
    }
    let id = player.mo.ok_or(PlayerBodyError::Binding)?;
    let actor = host.access.get(id).ok_or(WeaponError::Reference)?;
    if actor.life().is_none_or(|l| {
        l.actor_type.0 != OriginalActorType::MT_PLAYER.ordinal() || player.health != l.health.max(0)
    }) {
        return Err(PlayerBodyError::Invariant.into());
    }
    let state = match setup {
        Some(state) => {
            host.events.push(Event::Weapon(WeaponEvent::Initialized {
                slot: PlayerSlot::One.ordinal(),
                actor: id,
            }));
            state
        }
        None => {
            let state = player.weapon.ok_or(WeaponError::Player)?;
            state.validate()?;
            state
        }
    };
    let adapter = state.adapter(id, player.health);
    let registry = host.registry;
    let result = weapons::pistol(
        registry,
        adapter,
        Host {
            life: host,
            id,
            fire,
            sound,
        },
        operation,
        command,
        leveltime,
    )?;
    let result = state.with_adapter_result(result, id)?;
    result.validate()?;
    host.player
        .as_deref_mut()
        .ok_or(PlayerBodyError::Profile)?
        .weapon = Some(result);
    Ok(())
}

pub(crate) fn stage(
    host: &mut LifeHost<'_, &mut ThinkerStore<Actor>>,
    command: TicCommand,
    leveltime: u64,
    sound: &mut Option<ActorId>,
) -> Result<(), SimError> {
    let dead = host
        .player
        .as_ref()
        .is_some_and(|p| p.state == PlayerBodyState::Dead);
    run(
        host,
        if dead {
            weapons::PistolOperation::DeadAdvance
        } else {
            weapons::PistolOperation::Advance
        },
        command,
        leveltime,
        Some(fire_current),
        Some(sound),
        None,
    )
}
pub(crate) fn drop_weapon<A: ActorAccess>(host: &mut LifeHost<'_, A>) -> Result<(), SimError> {
    if host.player.as_ref().is_some_and(|p| p.weapon.is_some()) {
        run(
            host,
            weapons::PistolOperation::Drop,
            TicCommand::default(),
            0,
            None,
            None,
            None,
        )?;
    }
    Ok(())
}

fn fire_current(
    host: &mut LifeHost<'_, &mut ThinkerStore<Actor>>,
    id: ActorId,
    weapon: Weapon,
    accurate: bool,
) -> Result<(), SimError> {
    let angle = host.access.get(id).ok_or(WeaponError::Reference)?.angle();
    if weapon == Weapon::Fist {
        let mut random = |kind| {
            let value = host.rng.p_random();
            host.events
                .push(Event::Weapon(WeaponEvent::Random { kind, value }));
            value
        };
        let damage = 2 * (i32::from(random(0)) % 10 + 1);
        let spread = (i32::from(random(1)) - i32::from(random(2))) * 262144;
        let angle = angle.wrapping_add_signed(spread);
        let slope = aim_current(host, id, angle, MELEE_UNITS)?;
        return line_current(host, id, weapon, angle, slope, damage, MELEE_UNITS);
    }
    if !matches!(weapon, Weapon::Pistol | Weapon::Shotgun | Weapon::Chaingun) {
        return Err(WeaponError::UnsupportedWeapon.into());
    }
    let slope = aim_current(host, id, angle, AUTOAIM_UNITS)?;
    // Source shotgun computes aim once and resolves each complete GunShot in order.
    let pellets = if weapon == Weapon::Shotgun { 7 } else { 1 };
    for _ in 0..pellets {
        let mut random = |kind| {
            let value = host.rng.p_random();
            host.events
                .push(Event::Weapon(WeaponEvent::Random { kind, value }));
            value
        };
        let (bullet_angle, damage) =
            weapons::gun_shot(angle, weapon != Weapon::Shotgun && accurate, &mut random);
        line_current(host, id, weapon, bullet_angle, slope, damage, BULLET_UNITS)?;
    }
    Ok(())
}
fn line_current(
    host: &mut LifeHost<'_, &mut ThinkerStore<Actor>>,
    id: ActorId,
    weapon: Weapon,
    angle: u32,
    slope: Fixed,
    damage: i32,
    range: i16,
) -> Result<(), SimError> {
    let actor = *host.access.get(id).ok_or(WeaponError::Reference)?;
    host.events.push(Event::Weapon(WeaponEvent::Bullet {
        actor: id,
        angle,
        slope,
        damage,
    }));
    let attack = LineAttack {
        angle,
        range: Fixed::from_map_units(range),
        slope,
        damage,
    };
    let lookup = |id| host.access.get(id).copied();
    let trace =
        combat_trace::collect_in(host.world, host.spatial, &lookup, id, angle, attack.range)?;
    let (hit, position, traversed) = combat_trace::hit_trace(
        host.world,
        host.sectors,
        id,
        &trace,
        attack.range,
        slope,
        &lookup,
    )?;
    let plan = match hit {
        Some(TraceObject::Actor(target)) => {
            let victim = *host.access.get(target).ok_or(CombatError::Reference)?;
            original_player::preflight_grounded_impulse(
                &victim,
                &actor,
                damage,
                OriginalActorType::MT_SERGEANT.descriptor().mass,
            )?;
            Some(combat::DamagePlan::current_pistol(
                host.registry,
                victim,
                DamageRequest {
                    target,
                    inflictor: Some(id),
                    source: Some(id),
                    amount: damage,
                },
                host.statistics.as_deref().ok_or(MapSpawnError::Ownership)?,
            )?)
        }
        _ => None,
    };
    let result = AttackResult {
        trace,
        hit,
        position,
        traversed,
        effect: None,
        damage: None,
    };
    let prepared = combat::PreparedAttack::current(id, attack, result, plan);
    host.access.reserve_spawns(prepared.spawn_count())?;
    combat::execute_attack(host, prepared);
    if host.sight_failed() {
        return Err(PlayerBodyError::Unsupported.into());
    }
    if weapon == Weapon::Fist {
        if let Some(TraceObject::Actor(target)) = hit {
            host.events.push(Event::ActorState(ActorStateEvent::Sound {
                actor: Some(id),
                sound: lifecycle::source_sound("sfx_punch").ok_or(WeaponError::Definition)?,
            }));
            let target = host
                .access
                .get(target)
                .ok_or(WeaponError::Reference)?
                .position();
            let body = host.access.get_mut(id).ok_or(WeaponError::Reference)?;
            let delta = [0, 1].map(|axis| target[axis] - body.position()[axis]);
            body.spec.angle = movement::point_angle(delta);
        }
    }
    Ok(())
}

fn aim_current(
    host: &mut LifeHost<'_, &mut ThinkerStore<Actor>>,
    id: ActorId,
    angle: u32,
    units: i16,
) -> Result<Fixed, SimError> {
    let mut slope = Fixed::ZERO;
    for query in [
        angle,
        angle.wrapping_add(AUTOAIM_OFFSET),
        angle.wrapping_sub(AUTOAIM_OFFSET),
    ] {
        let range = Fixed::from_map_units(units);
        let lookup = |id| host.access.get(id).copied();
        let trace = combat_trace::collect_in(host.world, host.spatial, &lookup, id, query, range)?;
        let aim = combat_trace::aim_trace(host.world, host.sectors, id, trace, range, &lookup)?;
        #[cfg(test)]
        AIM_OBSERVATIONS.with(|capture| {
            if let Some(rows) = capture.borrow_mut().as_mut() {
                rows.push(aim.clone());
            }
        });
        slope = aim.slope;
        if aim.target.is_some() || units == MELEE_UNITS {
            break;
        }
    }
    Ok(slope)
}

#[cfg(test)]
thread_local! {
    static AIM_OBSERVATIONS: std::cell::RefCell<Option<Vec<AimResult>>> = const { std::cell::RefCell::new(None) };
}
#[cfg(test)]
#[path = "player_pistol_tests.rs"]
mod tests;
