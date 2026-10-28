//! Explicit unbound-actor combat services on the accepted state/lifecycle host.
use crate::{
    lifecycle::{ActorAccess, Demand, LifeHost},
    *,
};
use std::{error::Error, fmt};
/// A bounded combat prerequisite failed before authoritative mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatError {
    /// Requires the explicit profile 5.
    Profile,
    /// Combat identity is invalid, stale or pending retirement.
    Reference,
    /// Full player inventory/armor/weapon/scoring behavior is unavailable.
    Player,
    /// Damage must be in 0..=1000 in this declared slice.
    Damage,
    /// Source mass/spawn health or checked health arithmetic is invalid.
    Metadata,
    /// Selected source type requires item drops or another excluded kill branch.
    KillBranch,
    /// Native definitions changed the required original impact behavior.
    ImpactDefinition,
    /// Original sky suppression is not implemented by this profile.
    Sky,
    /// Range must be positive and at most 2048 fixed map units.
    Range,
    /// Trace or thrust coordinate setup exceeds the defined arithmetic slice.
    Coordinates,
    /// Source's 128-intercept storage bound would be exceeded.
    InterceptLimit,
    /// Source's 64-cell traversal did not reach its endpoint.
    TraversalLimit,
    /// Conservative callback/trace work exceeded the supported bound.
    WorkLimit,
}
impl fmt::Display for CombatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "combat: {self:?}")
    }
}
impl Error for CombatError {}
/// Direct P_DamageMobj subset request; source and inflictor are independent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamageRequest {
    /// Actor receiving damage.
    pub target: ActorId,
    /// Physical damage origin, or None for environment damage.
    pub inflictor: Option<ActorId>,
    /// Actor to target in response, or None.
    pub source: Option<ActorId>,
    /// Supplied source damage, not a weapon damage/spread generator.
    pub amount: i32,
}
/// Outcome of one completed direct damage operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamageResult {
    /// Nonshootable/already-dead source early return.
    pub ignored: bool,
    /// Health before/after.
    pub health: [i32; 2],
    /// Pain chance succeeded; its state may itself remove the actor.
    pub pain: bool,
    /// P_KillMobj subset ran.
    pub killed: bool,
    /// Extreme-death state selected by the strict source threshold.
    pub extreme: bool,
}
/// Explicit source P_LineAttack parameters; weapon buttons/ammo are later work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineAttack {
    /// Unsigned binary angle.
    pub angle: u32,
    /// Positive fixed attack range, including its fractional bits.
    pub range: Fixed,
    /// Fixed vertical slope; use aim_line_attack for source autoaim.
    pub slope: Fixed,
    /// Damage zero still spawns an impact and consumes its RNG.
    pub damage: i32,
}
/// Trace and real gameplay effects of a successful explicit attack request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttackResult {
    /// Original candidate/fraction collection.
    pub trace: HitscanTrace,
    /// First applicable line or actor.
    pub hit: Option<TraceObject>,
    /// Impact position before source random Z jitter.
    pub position: Option<[Fixed; 3]>,
    /// Number of intercept callbacks evaluated before stopping.
    pub traversed: u32,
    /// Real puff/blood thinker, not a renderer-only substitute.
    pub effect: Option<ActorId>,
    /// Damage result, absent for walls, misses or zero attack damage.
    pub damage: Option<DamageResult>,
}
/// Deterministic combat observations in source side-effect order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatEvent {
    /// Explicit shot parameters before trace observations/effects.
    Attack {
        /// Valid shooter.
        shooter: ActorId,
        /// Source request.
        attack: LineAttack,
    },
    /// Complete read-only decision, recorded before impacts/damage.
    Trace {
        /// Source collection and stopping result.
        result: Box<AttackResult>,
    },
    /// Direct damage invocation before thrust/health/RNG side effects.
    Damage(DamageRequest),
    /// Source implicit draw; kind 0=fall-forward, 1=pain, 2=kill timer,
    /// 3/4=impact Z draws, 5=impact timer (spawn lastlook stays implicit).
    Random {
        /// Defined service draw category.
        kind: u8,
        /// Original P_Random lookup.
        value: u8,
    },
    /// Source XY impulse applied before health reduction.
    Knockback {
        /// Receiving actor.
        target: ActorId,
        /// Binary angle after any reversal.
        angle: u32,
        /// Fixed impulse magnitude.
        thrust: Fixed,
        /// Resulting XY momentum.
        momentum: [Fixed; 2],
    },
    /// Health subtraction before pain/kill decisions.
    Health {
        /// Receiving actor.
        target: ActorId,
        /// Before and after health.
        values: [i32; 2],
    },
    /// JUSTHIT set before entering pain state.
    Pain {
        /// Receiving actor.
        target: ActorId,
        /// Registered pain state.
        state: StateId,
    },
    /// Source kill flag/height changes, before death actions/timer draw.
    Kill {
        /// Killed actor.
        target: ActorId,
        /// Selected death state.
        state: StateId,
        /// Extreme-death threshold succeeded.
        extreme: bool,
    },
    /// Source post-action randomized/clamped death tics.
    DeathTimer {
        /// Killed actor.
        target: ActorId,
        /// Remaining state duration.
        tics: i32,
    },
    /// Nonlethal damage reset reactiontime after pain callbacks.
    Reaction(ActorId),
    /// Source target/threshold assignment, before any see-state entry.
    Retarget {
        /// Damaged owner.
        target: ActorId,
        /// Valid source identity (may be pending after a callback).
        source: ActorId,
    },
    /// Real impact initialization completed before damage.
    Impact {
        /// Spawned effect.
        actor: ActorId,
    },
    /// Source ignored target; no gameplay RNG/health/momentum changed.
    Ignored(ActorId),
}
#[derive(Clone, Copy, Debug)]
struct DamagePlan {
    request: DamageRequest,
    health: [i32; 2],
    killed: bool,
    extreme: bool,
    state: StateId,
    demand: Demand,
    ignored: bool,
}
#[derive(Clone, Debug)]
pub(crate) struct PreparedAttack {
    shooter: ActorId,
    attack: LineAttack,
    pub result: AttackResult,
    plan: Option<DamagePlan>,
}
impl PreparedAttack {
    pub fn spawn_count(&self) -> usize {
        usize::from(self.result.hit.is_some()) + self.plan.map_or(0, |p| p.demand.spawns)
    }
}
impl Simulation {
    /// Profile 5 using recognized source definitions; no THINGS spawned implicitly.
    pub fn combat(map: Map) -> Result<Self, SimError> {
        Self::with_combat_registry(map, 1024, ActorRegistry::vanilla(), ActorOptions::default())
    }
    /// Immutable native/source definitions for the declared unbound combat slice.
    pub fn with_combat_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        registry.validate_impact_source()?;
        if map
            .sectors()
            .iter()
            .any(|s| s.flats[1].as_bytes().eq_ignore_ascii_case(b"F_SKY1\0\0"))
        {
            return Err(CombatError::Sky.into());
        }
        let mut sim = Self::with_actor_registry(map, limit, registry, options)?;
        sim.profile = Profile::Combat;
        // Impacts are spawned through the infallible validated lifecycle host.
        // Prove their dimensions/closures fit before admitting any attack.
        for name in ["MT_PUFF", "MT_BLOOD"] {
            sim.validate_combat_type(lifecycle::source_type_id(name).unwrap())?;
        }
        Ok(sim)
    }
    pub(crate) fn validate_combat_type(&self, ty: ActorTypeId) -> Result<(), SimError> {
        let registry = self.registry.as_deref().ok_or(CombatError::Profile)?;
        if self.profile == Profile::MonsterAttacks {
            return self.validate_monster_type(ty);
        }
        if self.profile == Profile::Projectiles {
            self.validate_projectile_type(ty)?;
        } else {
            registry.validate_type_combat(ty, &self.sectors)?;
        }
        let info = registry.actor_type(ty).ok_or(ActorError::InvalidType)?;
        if self.profile == Profile::PlayerWeapons {
            self.validate_weapon_type(ty)?;
        }
        if info.flags() & 4 != 0 {
            if info.mass() <= 0
                || info.spawn_health() <= 0
                || !(0..=256).contains(&info.pain_chance())
            {
                return Err(CombatError::Metadata.into());
            }
            for state in [
                info.states()[1],
                info.states()[2],
                info.states()[5],
                info.states()[6],
            ] {
                self.validate_entry(state)?;
            }
        }
        Ok(())
    }
    fn combat_actor(&self, id: ActorId) -> Result<Actor, SimError> {
        self.combat_actor_with(id, None)
    }
    fn combat_actor_with(&self, id: ActorId, shooter: Option<ActorId>) -> Result<Actor, SimError> {
        if !matches!(
            self.profile,
            Profile::Combat
                | Profile::PlayerWeapons
                | Profile::Projectiles
                | Profile::MonsterAttacks
        ) {
            return Err(CombatError::Profile.into());
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(CombatError::Reference.into());
        }
        if self.bindings.contains(&Some(id))
            && !(self.profile == Profile::PlayerWeapons && shooter == Some(id))
        {
            return Err(CombatError::Player.into());
        }
        self.actor(id).copied().ok_or(CombatError::Reference.into())
    }
    /// Pure source autoaim query. This is not general P_CheckSight or AI perception.
    pub fn aim_line_attack(
        &self,
        shooter: ActorId,
        angle: u32,
        range: Fixed,
    ) -> Result<AimResult, SimError> {
        if self.profile == Profile::MonsterAttacks {
            return Err(CombatError::Profile.into());
        }
        self.combat_actor(shooter)?;
        Ok(crate::combat_trace::aim(self, shooter, angle, range)?)
    }
    fn plan_damage(&self, request: DamageRequest) -> Result<DamagePlan, SimError> {
        self.plan_damage_with(request, None)
    }
    pub(crate) fn plan_weapon_damage(
        &self,
        request: DamageRequest,
        shooter: ActorId,
    ) -> Result<(), SimError> {
        self.plan_damage_with(request, Some(shooter)).map(|_| ())
    }
    fn plan_damage_with(
        &self,
        request: DamageRequest,
        shooter: Option<ActorId>,
    ) -> Result<DamagePlan, SimError> {
        self.plan_damage_lookup(request, shooter, None)
    }
    fn plan_damage_lookup(
        &self,
        request: DamageRequest,
        shooter: Option<ActorId>,
        lookup: Option<&dyn Fn(ActorId) -> Option<Actor>>,
    ) -> Result<DamagePlan, SimError> {
        let base = self.combat_actor(request.target)?;
        let actor = match lookup {
            Some(f) => f(request.target).ok_or(CombatError::Reference)?,
            None => base,
        };
        for id in [request.inflictor, request.source].into_iter().flatten() {
            self.combat_actor_with(id, shooter)?;
        }
        if !(0..=1000).contains(&request.amount) {
            return Err(CombatError::Damage.into());
        }
        let life = actor.life.unwrap();
        let mut plan = DamagePlan {
            request,
            health: [life.health; 2],
            killed: false,
            extreme: false,
            state: StateId(0),
            demand: Demand::default(),
            ignored: life.flags & 4 == 0 || life.health <= 0,
        };
        if plan.ignored {
            return Ok(plan);
        }
        self.validate_combat_type(life.actor_type)?;
        let registry = self.registry.as_deref().unwrap();
        let info = registry.actor_type(life.actor_type).unwrap();
        plan.health[1] = life
            .health
            .checked_sub(request.amount)
            .ok_or(CombatError::Metadata)?;
        plan.killed = plan.health[1] <= 0;
        if let Some(inflictor) = request.inflictor {
            let from = match lookup {
                Some(f) => f(inflictor).ok_or(CombatError::Reference)?,
                None => *self.actor(inflictor).unwrap(),
            }
            .position();
            for (a, b) in actor.position().into_iter().zip(from) {
                let delta = a
                    .bits()
                    .checked_sub(b.bits())
                    .ok_or(CombatError::Coordinates)?;
                delta.checked_abs().ok_or(CombatError::Coordinates)?;
            }
        }
        if plan.killed {
            if ["MT_POSSESSED", "MT_WOLFSS", "MT_SHOTGUY", "MT_CHAINGUY"]
                .iter()
                .any(|name| lifecycle::source_type_is(life.actor_type, name))
            {
                return Err(CombatError::KillBranch.into());
            }
            plan.extreme = plan.health[1] < -info.spawn_health() && info.states()[6].0 != 0;
            plan.state = info.states()[if plan.extreme { 6 } else { 5 }];
            self.validate_entry(plan.state)?;
            plan.demand = registry.demand(plan.state, life.tics)?;
        } else {
            if info.pain_chance() > 0 {
                plan.demand
                    .add(registry.demand(info.states()[2], life.tics)?)?;
            }
            if request.source.is_some() {
                plan.demand
                    .add(registry.demand(info.states()[1], life.tics)?)?;
            }
        }
        Ok(plan)
    }
    /// Atomic supported P_DamageMobj operation. No full-simulation clone is used.
    pub fn damage_actor(&mut self, request: DamageRequest) -> Result<DamageResult, SimError> {
        if self.profile == Profile::MonsterAttacks {
            return Err(CombatError::Profile.into());
        }
        let plan = self.plan_damage(request)?;
        self.reserve_actor_spawns(plan.demand.spawns)?;
        Ok(self.combat_host().apply_damage(plan))
    }
    /// Explicit supported P_LineAttack. Impact thinkers/RNG precede damage;
    /// zero damage still creates the source impact. Errors preserve canonical state.
    pub fn line_attack(
        &mut self,
        shooter: ActorId,
        attack: LineAttack,
    ) -> Result<AttackResult, SimError> {
        if self.profile == Profile::MonsterAttacks {
            return Err(CombatError::Profile.into());
        }
        self.line_attack_with(shooter, attack, None)
    }
    pub(crate) fn weapon_line_attack(
        &mut self,
        shooter: ActorId,
        attack: LineAttack,
    ) -> Result<AttackResult, SimError> {
        self.line_attack_with(shooter, attack, Some(shooter))
    }
    fn line_attack_with(
        &mut self,
        shooter: ActorId,
        attack: LineAttack,
        permit: Option<ActorId>,
    ) -> Result<AttackResult, SimError> {
        let prepared = self.prepare_attack(shooter, attack, permit)?;
        self.reserve_actor_spawns(prepared.spawn_count())?;
        Ok(self.combat_host().execute_attack(prepared))
    }
    pub(crate) fn prepare_attack(
        &self,
        shooter: ActorId,
        attack: LineAttack,
        permit: Option<ActorId>,
    ) -> Result<PreparedAttack, SimError> {
        self.prepare_attack_lookup(shooter, attack, permit, None)
    }
    pub(crate) fn prepare_attack_lookup(
        &self,
        shooter: ActorId,
        attack: LineAttack,
        permit: Option<ActorId>,
        lookup: Option<&dyn Fn(ActorId) -> Option<Actor>>,
    ) -> Result<PreparedAttack, SimError> {
        self.combat_actor_with(shooter, permit)?;
        if !(0..=1000).contains(&attack.damage) {
            return Err(CombatError::Damage.into());
        }
        let trace = crate::combat_trace::collect(self, shooter, attack.angle, attack.range)?;
        let (hit, position, traversed) = match lookup {
            Some(f) => crate::combat_trace::hit_with(
                self,
                shooter,
                &trace,
                attack.range,
                attack.slope,
                &|id| f(id),
            ),
            None => crate::combat_trace::hit(self, shooter, &trace, attack.range, attack.slope),
        };
        let plan = match hit {
            Some(TraceObject::Actor(target)) if attack.damage != 0 => {
                Some(self.plan_damage_lookup(
                    DamageRequest {
                        target,
                        inflictor: Some(shooter),
                        source: Some(shooter),
                        amount: attack.damage,
                    },
                    permit,
                    lookup,
                )?)
            }
            _ => None,
        };

        Ok(PreparedAttack {
            shooter,
            attack,
            plan,
            result: AttackResult {
                trace,
                hit,
                position,
                traversed,
                effect: None,
                damage: None,
            },
        })
    }
    fn combat_host(&mut self) -> LifeHost<'_, &mut ThinkerStore<Actor>> {
        LifeHost {
            perception: None,
            access: &mut self.actors,
            registry: self.registry.as_deref().unwrap(),
            world: &self.world,
            sectors: &self.sectors,
            options: self.actor_options,
            rng: &mut self.rng,
            spatial: &mut self.spatial,
            bindings: &mut self.bindings,
            events: &mut self.pending,
        }
    }
}
impl<A: ActorAccess> LifeHost<'_, A> {
    fn combat_event(&mut self, event: CombatEvent) {
        self.events.push(Event::Combat(event));
    }
    fn combat_random(&mut self, kind: u8) -> u8 {
        let value = self.rng.p_random();
        self.combat_event(CombatEvent::Random { kind, value });
        value
    }
    fn apply_damage(&mut self, plan: DamagePlan) -> DamageResult {
        let DamagePlan {
            request,
            health,
            killed,
            extreme,
            state,
            ignored,
            ..
        } = plan;
        let id = request.target;
        self.combat_event(CombatEvent::Damage(request));
        let mut result = DamageResult {
            ignored,
            health,
            pain: false,
            killed,
            extreme,
        };
        if ignored {
            self.combat_event(CombatEvent::Ignored(id));
            return result;
        }
        let old = *self.access.get(id).unwrap();
        let life = old.life.unwrap();
        let info = self.registry.actor_type(life.actor_type).unwrap().clone();
        if let Some(inflictor) = request.inflictor {
            let origin = *self.access.get(inflictor).unwrap();
            let delta = [
                old.position()[0] - origin.position()[0],
                old.position()[1] - origin.position()[1],
            ];
            let mut angle = movement::point_angle(delta);
            let mut thrust = request.amount * 8192 * 100 / info.mass();
            if request.amount < 40
                && request.amount > life.health
                && old.z() - origin.z() > Fixed::from_map_units(64)
                && self.combat_random(0) & 1 != 0
            {
                angle = angle.wrapping_add(0x80000000);
                thrust *= 4;
            }
            let thrust = Fixed::from_bits(thrust);
            let phase = (angle >> 19) as usize;
            let a = self.access.get_mut(id).unwrap();
            a.spec.momentum[0] =
                a.spec.momentum[0] + thrust * Fixed::from_bits(FINE_SINE[phase + 2048]);
            a.spec.momentum[1] = a.spec.momentum[1] + thrust * Fixed::from_bits(FINE_SINE[phase]);
            let momentum = a.spec.momentum;
            self.combat_event(CombatEvent::Knockback {
                target: id,
                angle,
                thrust,
                momentum,
            });
        }
        self.access
            .get_mut(id)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .health = health[1];
        self.combat_event(CombatEvent::Health {
            target: id,
            values: health,
        });
        if killed {
            let a = self.access.get_mut(id).unwrap();
            let life = a.life.as_mut().unwrap();
            life.flags &= !(4 | 16384 | 16777216);
            if !lifecycle::source_type_is(life.actor_type, "MT_SKULL") {
                life.flags &= !512;
            }
            life.flags |= 1048576 | 1024;
            a.spec.height = Fixed::from_bits(a.spec.height.bits() >> 2);
            a.allow_dropoff = true;
            a.gravity = if life.flags & 512 != 0 {
                Gravity::None
            } else {
                Gravity::Normal
            };
            self.combat_event(CombatEvent::Kill {
                target: id,
                state,
                extreme,
            });
            self.enter(id, state);
            let random = self.combat_random(2);
            let life = self.access.get_mut(id).unwrap().life.as_mut().unwrap();
            life.tics = (life.tics - i32::from(random & 3)).max(1);
            let tics = life.tics;
            self.combat_event(CombatEvent::DeathTimer { target: id, tics });
            return result;
        }
        let random = self.combat_random(1);
        if i32::from(random) < info.pain_chance() {
            result.pain = true;
            self.access
                .get_mut(id)
                .unwrap()
                .life
                .as_mut()
                .unwrap()
                .flags |= 64;
            self.combat_event(CombatEvent::Pain {
                target: id,
                state: info.states()[2],
            });
            self.enter(id, info.states()[2]);
        }
        self.access
            .get_mut(id)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .reaction_time = 0;
        self.combat_event(CombatEvent::Reaction(id));
        if let Some(source) = request.source {
            let current = self.access.get(id).unwrap().life.unwrap();
            let source_type = self.access.get(source).unwrap().life.unwrap().actor_type;
            if (current.threshold == 0 || lifecycle::source_type_is(current.actor_type, "MT_VILE"))
                && source != id
                && !lifecycle::source_type_is(source_type, "MT_VILE")
            {
                self.target(id, Some(source));
                self.access
                    .get_mut(id)
                    .unwrap()
                    .life
                    .as_mut()
                    .unwrap()
                    .threshold = 100;
                self.combat_event(CombatEvent::Retarget { target: id, source });
                if self.access.get(id).unwrap().life.unwrap().state == Some(info.states()[0])
                    && info.states()[1].0 != 0
                {
                    self.enter(id, info.states()[1]);
                }
            }
        }
        result
    }
    fn execute_attack(&mut self, prepared: PreparedAttack) -> AttackResult {
        let PreparedAttack {
            shooter,
            attack,
            mut result,
            plan,
        } = prepared;
        let hit = result.hit;
        let position = result.position;
        self.combat_event(CombatEvent::Attack { shooter, attack });
        self.combat_event(CombatEvent::Trace {
            result: Box::new(result.clone()),
        });
        if let Some(mut position) = position {
            let blood = matches!(hit,Some(TraceObject::Actor(id)) if self.access.get(id).unwrap().life.unwrap().flags&524288==0);
            let first = self.combat_random(3);
            let second = self.combat_random(4);
            position[2] =
                position[2] + Fixed::from_bits((i32::from(first) - i32::from(second)) * 1024);
            let ty = lifecycle::source_type_id(if blood { "MT_BLOOD" } else { "MT_PUFF" }).unwrap();
            let id = self.spawn(
                ty,
                [position[0], position[1]],
                SpawnHeight::Absolute(position[2]),
            );
            self.access.get_mut(id).unwrap().momz =
                Fixed::from_map_units(if blood { 2 } else { 1 });
            let random = self.combat_random(5);
            let life = self.access.get_mut(id).unwrap().life.as_mut().unwrap();
            life.tics = (life.tics - i32::from(random & 3)).max(1);
            let state = if blood && attack.damage < 9 {
                Some("S_BLOOD3")
            } else if blood && attack.damage <= 12 {
                Some("S_BLOOD2")
            } else if !blood && attack.range == Fixed::from_map_units(64) {
                Some("S_PUFF3")
            } else {
                None
            };
            if let Some(name) = state {
                self.enter(id, lifecycle::source_state_id(name).unwrap());
            }
            self.combat_event(CombatEvent::Impact { actor: id });
            result.effect = Some(id);
        }
        if let Some(plan) = plan {
            result.damage = Some(self.apply_damage(plan));
        }
        result
    }
}
pub(crate) fn execute_attack<A: ActorAccess>(
    host: &mut LifeHost<'_, A>,
    prepared: PreparedAttack,
) -> AttackResult {
    host.execute_attack(prepared)
}
