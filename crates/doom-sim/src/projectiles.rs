//! Bounded original plasma launch, missile collision and shared actor lifecycle.
use crate::{lifecycle::LifeHost, *};
use std::{error::Error, fmt};
const MISSILE: u32 = 65536;
const PLASMA: ActorTypeId = ActorTypeId(34);
/// Rejected projectile prerequisite; recoverable errors precede mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectileError {
    /// Requires explicit profile 7.
    Profile,
    /// Only original MT_PLASMA is executable.
    UnsupportedType,
    /// Original immutable definitions or fixture constraints were violated.
    Definition,
    /// Invalid, pending, retired or stale launch/source identity.
    Reference,
    /// A live missile requires an active owner of its original generation.
    Owner,
    /// Actual bound player damage/weapon integration remains unsupported.
    Player,
    /// Use launch/impact services rather than bypassing missile initialization.
    Controlled,
    /// Coordinate/momentum setup is outside the declared arithmetic slice.
    Arithmetic,
    /// At most 64 actors are admitted; no new engine-limit claim.
    WorkLimit,
}
impl fmt::Display for ProjectileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "projectile: {self:?}")
    }
}
impl Error for ProjectileError {}
/// Exact source launch helper selection, not generic ballistic parameters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectileAim {
    /// P_SpawnMissile toward a validated actor's current position.
    Actor(ActorId),
    /// P_SpawnPlayerMissile autoaim from the source's current angle.
    PlayerAutoAim,
}
/// Explicit between-tic launch; weapon/monster callbacks remain future work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectileRequest {
    /// Valid active source; retained in the missile's source target field.
    pub source: ActorId,
    /// Only source type 34 (MT_PLASMA) is admitted.
    pub actor_type: ActorTypeId,
    /// Source helper/destination; destination is not retained as a tracer.
    pub aim: ProjectileAim,
}
/// Original projectile observations, interleaved with shared movement/state/damage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectileEvent {
    /// Before explosion clears momentum/enters death state.
    Exploded(ActorId),
    /// Source launch velocity before first spawn check.
    Velocity {
        /// Missile identity.
        actor: ActorId,
        /// Binary angle after any source shadow spread.
        angle: u32,
        /// Raw fixed XYZ momentum.
        momentum: [Fixed; 3],
    },
    /// Projectile-specific draw; spawn lastlook remains implicit in initialization.
    Random {
        /// 0 spawn tics, 1/2 shadow left/right, 3 contact damage, 4 explosion tics.
        kind: u8,
        /// Original lookup result.
        value: u8,
    },
    /// Direct initial half-step, before P_TryMove and any relinking.
    HalfStep {
        /// Missile identity.
        actor: ActorId,
        /// Advanced XYZ.
        position: [Fixed; 3],
        /// Randomized initial state duration.
        tics: i32,
    },
    /// Validated source helper request before actor/RNG/sound changes.
    Launch(ProjectileRequest),
}
fn within(v: Fixed, units: u32) -> bool {
    v.bits().unsigned_abs() <= units * 65536
}
impl Simulation {
    /// Profile 7, exact original metadata and at most 64 actor slots.
    pub fn projectiles(map: Map) -> Result<Self, SimError> {
        Self::with_projectile_registry(map, 64, ActorRegistry::vanilla(), ActorOptions::default())
    }
    /// Exact original states/types, except labelled player-state fixtures 15/17/136.
    pub fn with_projectile_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if limit > 64 {
            return Err(ProjectileError::WorkLimit.into());
        }
        let source = ActorRegistry::vanilla();
        if registry.states() != source.states() || registry.types().len() != source.types().len() {
            return Err(ProjectileError::Definition.into());
        }
        for (i, t) in registry.types().iter().enumerate() {
            if t == &source.types()[i] {
                continue;
            }
            if ![15, 17, 136].contains(&i) {
                return Err(ProjectileError::Definition.into());
            }
            for j in 0..23 {
                if ![0, 2, 5, 8, 16, 17, 18, 21].contains(&j)
                    && t.properties[j] != source.types()[0].properties[j]
                {
                    return Err(ProjectileError::Definition.into());
                }
            }
            if t.states() != lifecycle::source_player_states()
                || !(1..=1000).contains(&t.spawn_health())
                || t.mass() < 10
                || !(0..=256).contains(&t.pain_chance())
            {
                return Err(ProjectileError::Definition.into());
            }
        }
        if map
            .vertices()
            .iter()
            .any(|v| v.x.unsigned_abs() > 4096 || v.y.unsigned_abs() > 4096)
            || map
                .sectors()
                .iter()
                .any(|s| s.heights.iter().any(|v| v.unsigned_abs() > 4096))
            || map.nodes().iter().any(|n| {
                [n.origin.x, n.origin.y, n.delta.x, n.delta.y]
                    .iter()
                    .any(|v| v.unsigned_abs() > 4096)
            })
            || map.blockmap().origin().x.unsigned_abs() > 4096
            || map.blockmap().origin().y.unsigned_abs() > 4096
        {
            return Err(ProjectileError::Arithmetic.into());
        }
        let mut sim = Self::with_combat_registry(map, limit, registry, options)?;
        sim.profile = Profile::Projectiles;
        Ok(sim)
    }
    pub(crate) fn validate_projectile_type(&self, ty: ActorTypeId) -> Result<(), SimError> {
        let r = self.registry.as_deref().unwrap();
        r.validate_type_projectile(ty, &self.sectors)?;
        let info = r.actor_type(ty).ok_or(ActorError::InvalidType)?;
        if info.flags() & MISSILE != 0 && ty != PLASMA {
            return Err(ProjectileError::UnsupportedType.into());
        }
        if info.flags() & 4 != 0
            && (![0, 15, 17, 136].contains(&ty.0)
                || info.states() != lifecycle::source_player_states()
                || info.spawn_health() > 1000
                || info.mass() < 10)
        {
            return Err(ProjectileError::Definition.into());
        }
        Ok(())
    }
    pub(crate) fn projectile_setup_position(
        &self,
        position: [Fixed; 2],
        height: SpawnHeight,
    ) -> Result<(), SimError> {
        if position.into_iter().any(|v| !within(v, 4096))
            || matches!(height,SpawnHeight::Absolute(z) if !within(z,4096))
        {
            return Err(ProjectileError::Arithmetic.into());
        }
        Ok(())
    }
    pub(crate) fn preflight_projectile_step(&self) -> Result<(), SimError> {
        if self.bindings.iter().any(Option::is_some) || self.weapon_player.is_some() {
            return Err(ProjectileError::Player.into());
        }
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            let a = self.actor(id).unwrap();
            if a.position()
                .into_iter()
                .chain([a.z])
                .any(|v| !within(v, 8192))
                || a.momentum().into_iter().any(|v| !within(v, 8192))
                || !within(a.momz, 1024)
            {
                return Err(ProjectileError::Arithmetic.into());
            }
            let life = a.life.unwrap();
            self.validate_projectile_type(life.actor_type)?;
            if life.flags & MISSILE != 0 {
                let owner = life.target.ok_or(ProjectileError::Owner)?;
                if self.actors.is_pending(owner) != Some(false) {
                    return Err(ProjectileError::Owner.into());
                }
            }
        }
        Ok(())
    }
    pub(crate) fn projectile_control(&self, id: ActorId) -> Result<(), SimError> {
        if self.profile == Profile::Projectiles
            && self
                .actor(id)
                .is_some_and(|a| a.life.unwrap().actor_type == PLASMA)
        {
            return Err(ProjectileError::Controlled.into());
        }
        Ok(())
    }
    fn projectile_source(&self, id: ActorId) -> Result<Actor, SimError> {
        if self.actors.is_pending(id) != Some(false) {
            return Err(ProjectileError::Reference.into());
        }
        let a = *self.actor(id).unwrap();
        if self.bindings.contains(&Some(id)) {
            return Err(ProjectileError::Player.into());
        }
        if a.position()
            .into_iter()
            .chain([a.z])
            .any(|v| !within(v, 4096))
        {
            return Err(ProjectileError::Arithmetic.into());
        }
        Ok(a)
    }
    fn projectile_random(&mut self, kind: u8) -> u8 {
        let value = self.rng.p_random();
        self.pending
            .push(Event::Projectile(ProjectileEvent::Random { kind, value }));
        value
    }
    /// Atomic source launch/check. A blocked initial check returns a real explosion actor.
    pub fn launch_projectile(&mut self, request: ProjectileRequest) -> Result<ActorId, SimError> {
        if self.profile != Profile::Projectiles {
            return Err(ProjectileError::Profile.into());
        }
        if request.actor_type != PLASMA {
            return Err(ProjectileError::UnsupportedType.into());
        }
        self.preflight_projectile_step()?;
        let source = self.projectile_source(request.source)?;
        self.validate_projectile_type(PLASMA)?;
        let speed = Fixed::from_bits(
            self.registry
                .as_deref()
                .unwrap()
                .actor_type(PLASMA)
                .unwrap()
                .speed(),
        );
        let (mut angle, momz, shadow) = match request.aim {
            ProjectileAim::Actor(id) => {
                let dest = self.projectile_source(id)?;
                let delta = [
                    dest.position()[0] - source.position()[0],
                    dest.position()[1] - source.position()[1],
                ];
                let angle = movement::point_angle(delta);
                let x = delta[0].bits().unsigned_abs() as i32;
                let y = delta[1].bits().unsigned_abs() as i32;
                let dist = x + y - x.min(y) / 2;
                let time = (dist / speed.bits()).max(1);
                (
                    angle,
                    Fixed::from_bits((dest.z - source.z).bits() / time),
                    dest.life.unwrap().flags & 262144 != 0,
                )
            }
            ProjectileAim::PlayerAutoAim => {
                let base = source.angle();
                let mut chosen = base;
                let mut slope = Fixed::ZERO;
                for an in [base, base.wrapping_add(1 << 26), base.wrapping_sub(1 << 26)] {
                    let a = crate::combat_trace::aim(
                        self,
                        request.source,
                        an,
                        Fixed::from_map_units(1024),
                    )?;
                    if a.target.is_some() {
                        chosen = an;
                        slope = a.slope;
                        break;
                    }
                }
                (chosen, speed * slope, false)
            }
        };
        if !within(momz, 64) {
            return Err(ProjectileError::Arithmetic.into());
        }
        self.reserve_actor_spawns(1)?;
        self.pending
            .push(Event::Projectile(ProjectileEvent::Launch(request)));
        let id = LifeHost {
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
        .spawn(
            PLASMA,
            source.position(),
            SpawnHeight::Absolute(source.z + Fixed::from_map_units(32)),
        );
        let sound = self
            .registry
            .as_deref()
            .unwrap()
            .actor_type(PLASMA)
            .unwrap()
            .properties[4] as u16;
        self.projectile_sound(id, sound);
        self.actors
            .get_mut(id)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .target = Some(request.source);
        self.pending
            .push(Event::ActorState(ActorStateEvent::Target {
                actor: id,
                target: Some(request.source),
            }));
        if shadow {
            let left = self.projectile_random(1);
            let right = self.projectile_random(2);
            angle = angle.wrapping_add(((i32::from(left) - i32::from(right)) * 1048576) as u32);
        }
        let phase = (angle >> 19) as usize;
        let momentum = [
            speed * Fixed::from_bits(FINE_SINE[phase + 2048]),
            speed * Fixed::from_bits(FINE_SINE[phase]),
        ];
        let a = self.actors.get_mut(id).unwrap();
        a.spec.angle = angle;
        a.spec.momentum = momentum;
        a.momz = momz;
        self.pending
            .push(Event::Projectile(ProjectileEvent::Velocity {
                actor: id,
                angle,
                momentum: [momentum[0], momentum[1], momz],
            }));
        let random = self.projectile_random(0);
        let a = self.actors.get_mut(id).unwrap();
        let life = a.life.as_mut().unwrap();
        life.tics = (life.tics - i32::from(random & 3)).max(1);
        let tics = life.tics;
        for axis in 0..2 {
            a.spec.position[axis] =
                a.spec.position[axis] + Fixed::from_bits(a.spec.momentum[axis].bits() >> 1);
        }
        a.z = a.z + Fixed::from_bits(a.momz.bits() >> 1);
        self.pending
            .push(Event::Projectile(ProjectileEvent::HalfStep {
                actor: id,
                position: [a.position()[0], a.position()[1], a.z],
                tics,
            }));
        if !self.try_projectile_move(id, self.actor(id).unwrap().position()) {
            self.explode_projectile(id);
        }
        Ok(id)
    }
    fn projectile_sound(&mut self, id: ActorId, sound: u16) {
        if sound != 0 {
            self.pending.push(Event::ActorState(ActorStateEvent::Sound {
                actor: Some(id),
                sound,
            }));
        }
    }
    fn explode_projectile(&mut self, id: ActorId) {
        self.pending
            .push(Event::Projectile(ProjectileEvent::Exploded(id)));
        let a = self.actors.get_mut(id).unwrap();
        a.spec.momentum = [Fixed::ZERO; 2];
        a.momz = Fixed::ZERO;
        let info = self
            .registry
            .as_deref()
            .unwrap()
            .actor_type(PLASMA)
            .unwrap();
        let death = info.states()[5];
        let sound = info.properties[14] as u16;
        self.enter_validated(id, death);
        let r = self.projectile_random(4);
        let life = self.actors.get_mut(id).unwrap().life.as_mut().unwrap();
        life.tics = (life.tics - i32::from(r & 3)).max(1);
        life.flags &= !MISSILE;
        self.projectile_sound(id, sound);
    }
    pub(crate) fn query_projectile_move(
        &self,
        id: ActorId,
        target: [Fixed; 2],
    ) -> (SectorMoveAttempt, Option<ActorId>) {
        let a = *self.actor(id).unwrap();
        let missile = a.life.unwrap().flags & MISSILE != 0;
        if !missile {
            return (
                multisector::check_move(
                    (&self.world, &self.sectors),
                    id,
                    &a,
                    target,
                    &self.spatial,
                    &|id| self.actor(id).copied(),
                    false,
                ),
                None,
            );
        }
        let owner = a.life.unwrap().target.unwrap();
        let owner_type = self
            .actor(owner)
            .expect("preflight owner")
            .life
            .unwrap()
            .actor_type;
        let mut movement = ActorMoveAttempt {
            target,
            lines: Collision {
                blocked_by: None,
                fits: true,
                candidates: Vec::new(),
            },
            actors: Vec::new(),
            blocked_by_actor: None,
        };
        let mut damage = None;
        movement::for_things(
            &self.world,
            id,
            &a,
            target,
            &self.spatial,
            &|id| self.actor(id).copied(),
            |candidate, other| {
                movement.actors.push(candidate);
                let life = other.life.unwrap();
                if life.flags & (2 | 1 | 4) == 0 {
                    return true;
                }
                let distance = a.radius().bits() + other.radius().bits();
                if (other.position()[0] - target[0]).bits().unsigned_abs() >= distance as u32
                    || (other.position()[1] - target[1]).bits().unsigned_abs() >= distance as u32
                    || candidate == id
                {
                    return true;
                }
                if a.z > other.z + other.height() || a.z + a.height() < other.z {
                    return true;
                }
                let same = owner_type == life.actor_type
                    || [owner_type.0, life.actor_type.0] == [15, 17]
                    || [owner_type.0, life.actor_type.0] == [17, 15];
                if same {
                    if candidate == owner {
                        return true;
                    }
                    if life.actor_type != ActorTypeId(0) {
                        movement.blocked_by_actor = Some(candidate);
                        return false;
                    }
                }
                if life.flags & 4 == 0 {
                    if life.flags & 2 == 0 {
                        return true;
                    }
                } else {
                    damage = Some(candidate)
                }
                movement.blocked_by_actor = Some(candidate);
                false
            },
        );
        (
            multisector::check_move_from_things(
                (&self.world, &self.sectors),
                &a,
                target,
                movement,
                false,
                true,
            ),
            damage,
        )
    }
    fn try_projectile_move(&mut self, id: ActorId, target: [Fixed; 2]) -> bool {
        let (attempt, damage) = self.query_projectile_move(id, target);
        if let Some(target) = damage {
            let damage = self
                .registry
                .as_deref()
                .unwrap()
                .actor_type(self.actor(id).unwrap().life.unwrap().actor_type)
                .unwrap()
                .damage();
            let amount = (i32::from(self.projectile_random(3) % 8) + 1) * damage;
            let owner = self.actor(id).unwrap().life.unwrap().target;
            self.damage_actor(DamageRequest {
                target,
                inflictor: Some(id),
                source: owner,
                amount,
            })
            .expect("prevalidated projectile damage");
        }
        let allowed = attempt.allowed();
        if allowed {
            let a = self.actors.get_mut(id).unwrap();
            a.spec.position = target;
            a.floor = attempt.floor;
            a.ceiling = attempt.ceiling;
            a.location = Some(attempt.location);
            self.spatial.link(id);
        }
        self.pending
            .push(Event::SectorMovement { actor: id, attempt });
        allowed
    }
    fn projectile_xy(&mut self, id: ActorId) {
        let a = self.actors.get_mut(id).unwrap();
        let max = Fixed::from_map_units(30);
        a.spec.momentum = a.spec.momentum.map(|m| m.clamp(-max, max));
        let mut left = a.spec.momentum;
        while left != [Fixed::ZERO; 2] {
            let mut target = self.actor(id).unwrap().position();
            if left[0] > Fixed::from_map_units(15) || left[1] > Fixed::from_map_units(15) {
                for axis in 0..2 {
                    target[axis] = target[axis] + Fixed::from_bits(left[axis].bits() / 2);
                    left[axis] = Fixed::from_bits(left[axis].bits() >> 1);
                }
            } else {
                for axis in 0..2 {
                    target[axis] = target[axis] + left[axis];
                }
                left = [Fixed::ZERO; 2];
            }
            if !self.try_projectile_move(id, target) {
                if self.actor(id).unwrap().life.unwrap().flags & MISSILE != 0 {
                    self.explode_projectile(id);
                } else {
                    self.actors.get_mut(id).unwrap().spec.momentum = [Fixed::ZERO; 2];
                }
            }
        }
    }
    fn projectile_z(&mut self, id: ActorId) {
        let a = self.actors.get_mut(id).unwrap();
        if a.z == a.floor && a.momz == Fixed::ZERO {
            return;
        }
        let before = [a.z, a.momz];
        let constraints = [a.floor, a.ceiling];
        let missile = a.life.unwrap().flags & MISSILE != 0;
        let mut floor = false;
        let mut ceiling = false;
        let mut gravity = false;
        let mut explode = false;
        a.z = a.z + a.momz;
        if a.z <= a.floor {
            floor = true;
            if a.momz < Fixed::ZERO {
                a.momz = Fixed::ZERO
            }
            a.z = a.floor;
            explode = missile;
        } else if a.gravity == Gravity::Normal {
            gravity = true;
            a.momz = if a.momz == Fixed::ZERO {
                -Fixed::from_map_units(2)
            } else {
                a.momz - Fixed::ONE
            };
        }
        if !explode && a.z + a.height() > a.ceiling {
            ceiling = true;
            if a.momz > Fixed::ZERO {
                a.momz = Fixed::ZERO
            }
            a.z = a.ceiling - a.height();
            explode = missile;
        }
        if explode {
            self.explode_projectile(id);
        }
        let a = self.actor(id).unwrap();
        self.pending.push(Event::Vertical {
            actor: id,
            update: VerticalEvent {
                before,
                after: [a.z, a.momz],
                constraints,
                floor_contact: floor,
                ceiling_contact: ceiling,
                gravity_applied: gravity,
            },
        });
    }
    pub(crate) fn projectile_thinkers_with(&mut self, mut before: impl FnMut(&mut Self, ActorId)) {
        let mut cursor = self.actors.cursor(self.actors.max_slots as usize);
        while let Some(visit) = self
            .actors
            .next_visit(&mut cursor)
            .expect("bounded validated traversal")
        {
            match visit {
                Visit::Retired(id) => self.events.push(Event::Retired(id)),
                Visit::Think(id) => {
                    self.events.push(Event::Think(id));
                    before(self, id);
                    if self.actor(id).unwrap().life.unwrap().actor_type == PLASMA {
                        if self.actor(id).unwrap().momentum() != [Fixed::ZERO; 2] {
                            self.projectile_xy(id);
                        }
                        self.projectile_z(id);
                    } else {
                        let mut a = *self.actor(id).unwrap();
                        let motion = movement::move_xy_multisector(
                            &self.world,
                            id,
                            &mut a,
                            &mut self.spatial,
                            |id| self.actors.get(id).copied(),
                            None,
                            &self.sectors,
                        );
                        for m in motion {
                            self.pending.push(match m {
                                movement::MotionEvent::Sector(attempt) => {
                                    Event::SectorMovement { actor: id, attempt }
                                }
                                movement::MotionEvent::Slide(line) => {
                                    Event::Slide { actor: id, line }
                                }
                                movement::MotionEvent::Attempt(attempt) => {
                                    Event::ActorMovement { actor: id, attempt }
                                }
                            });
                        }
                        if let Some(update) = multisector::move_z(&mut a) {
                            self.pending.push(Event::Vertical { actor: id, update });
                        }
                        *self.actors.get_mut(id).unwrap() = a;
                    }
                    if self.actors.is_pending(id) == Some(false) {
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
                        .tick(id);
                    }
                    self.events.append(&mut self.pending);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/support/m3d2b.rs"]
mod fixtures;
#[cfg(test)]
mod tests {
    use super::*;
    fn unchanged<T: std::fmt::Debug>(
        sim: &mut Simulation,
        op: impl FnOnce(&mut Simulation) -> Result<T, SimError>,
    ) {
        let before = sim.snapshot();
        assert!(op(sim).is_err());
        assert_eq!(sim.snapshot(), before);
    }
    #[test]
    fn same_pass_append_matches_original_runthinkers_with_synthetic_launch_wrapper() {
        let c = &fixtures::cases()[35];
        let mut sim = fixtures::setup(c);
        let mut trace = fixtures::Trace::default();
        trace.check(&sim, 35, 0, sim.pending_events());
        fixtures::assert_golden(&sim, 35, 0);
        trace.check(&sim, 35, 1, &[]);
        let mut fields = 0;
        for op in 2..=c.n(16) as usize + 1 {
            let pending = sim.pending.len();
            if op == 2 {
                sim.events = std::mem::take(&mut sim.pending);
                sim.events
                    .extend([Event::Phase(Phase::Players), Event::Phase(Phase::Thinkers)]);
                let mut launched = false;
                sim.projectile_thinkers_with(|sim, id| {
                    if id == ActorId::from_parts(0, 1) && !launched {
                        launched = true;
                        fixtures::launch(sim, c);
                    }
                });
                sim.events
                    .extend([Event::Phase(Phase::Specials), Event::Phase(Phase::Respawn)]);
                sim.tic += 1;
            } else {
                sim.step([TicCommand::default(); 4]).unwrap();
            }
            trace.check(&sim, 35, op, &sim.events()[pending..]);
            fields = trace.actor_fields;
        }
        println!(
            "same-pass C: {} actor fields, {} events, {} moves, {} candidates, {} openings, {} RNG draws",
            trace.actor_fields,
            trace.events,
            trace.moves,
            trace.candidates,
            trace.openings,
            trace.draws
        );
        assert_eq!(
            [
                fields,
                trace.events,
                trace.moves,
                trace.candidates,
                trace.openings,
                trace.draws
            ],
            [2520, 198, 43, 216, 0, 7]
        ); // Includes setup and 40 original thinker passes.
    }
    #[test]
    fn invalid_stale_pending_and_capacity_launches_preserve_full_state() {
        let c = &fixtures::cases()[0];
        let mut sim = fixtures::setup(c);
        let req = fixtures::request(c);
        unchanged(&mut sim, |s| {
            s.launch_projectile(ProjectileRequest {
                source: ActorId::from_parts(999, 1),
                ..req
            })
        });
        unchanged(&mut sim, |s| {
            s.launch_projectile(ProjectileRequest {
                aim: ProjectileAim::Actor(ActorId::from_parts(1, 2)),
                ..req
            })
        });
        unchanged(&mut sim, |s| {
            s.launch_projectile(ProjectileRequest {
                actor_type: ActorTypeId(33),
                ..req
            })
        });
        let old_z = sim.actor(ActorId::from_parts(1, 1)).unwrap().z;
        sim.actors.get_mut(ActorId::from_parts(1, 1)).unwrap().z = Fixed::from_map_units(4096);
        unchanged(&mut sim, |s| s.launch_projectile(req));
        sim.actors.get_mut(ActorId::from_parts(1, 1)).unwrap().z = old_z;
        sim.actors.max_slots = 2;
        unchanged(&mut sim, |s| s.launch_projectile(req));
        sim.actors.max_slots = 64;
        sim.mark_remove(ActorId::from_parts(1, 1)).unwrap();
        unchanged(&mut sim, |s| s.launch_projectile(req));
        sim.step([TicCommand::default(); 4]).unwrap();
        let replacement = sim
            .spawn_type(ActorTypeId(136), [c.f(5), c.f(6)], SpawnHeight::Floor)
            .unwrap();
        assert_eq!(replacement, ActorId::from_parts(1, 2));
        unchanged(&mut sim, |s| s.launch_projectile(req));
    }
    #[test]
    fn owner_lifetime_preflight_and_explosion_retention_are_generation_aware() {
        let c = &fixtures::cases()[0];
        let mut sim = fixtures::setup(c);
        let missile = sim.launch_projectile(fixtures::request(c)).unwrap();
        sim.mark_remove(ActorId::from_parts(0, 1)).unwrap();
        unchanged(&mut sim, |s| s.step([TicCommand::default(); 4]));
        sim.mark_remove(missile).unwrap();
        sim.step([TicCommand::default(); 4]).unwrap();
        let replacement = sim
            .spawn_type(ActorTypeId(0), [c.f(2), c.f(3)], SpawnHeight::Floor)
            .unwrap();
        assert_eq!(replacement, ActorId::from_parts(0, 2));
        assert!(sim.actor(ActorId::from_parts(0, 1)).is_none());
        let c = &fixtures::cases()[3];
        let mut sim = fixtures::setup(c);
        let missile = sim.launch_projectile(fixtures::request(c)).unwrap();
        assert_eq!(
            sim.actor(missile).unwrap().life().unwrap().flags & MISSILE,
            0
        );
        sim.mark_remove(ActorId::from_parts(0, 1)).unwrap();
        sim.step([TicCommand::default(); 4]).unwrap();
        let old = sim.actor(missile).unwrap().life().unwrap().target.unwrap();
        let replacement = sim
            .spawn_type(ActorTypeId(0), [c.f(2), c.f(3)], SpawnHeight::Floor)
            .unwrap();
        assert_ne!(replacement, old);
        assert!(sim.actor_target(missile).is_none());
        assert_eq!(
            sim.actor(missile).unwrap().life().unwrap().target,
            Some(old)
        );
    }
    #[test]
    fn setup_bounds_and_missile_initialization_cannot_be_bypassed() {
        let c = &fixtures::cases()[0];
        let mut sim = fixtures::setup(c);
        let id = ActorId::from_parts(0, 1);
        unchanged(&mut sim, |s| {
            s.spawn_type(PLASMA, [Fixed::ZERO; 2], SpawnHeight::Floor)
        });
        unchanged(&mut sim, |s| {
            s.spawn_type(
                ActorTypeId(0),
                [Fixed::from_map_units(4097), Fixed::ZERO],
                SpawnHeight::Floor,
            )
        });
        unchanged(&mut sim, |s| {
            s.set_momentum(id, [Fixed::from_bits(i32::MAX), Fixed::ZERO])
        });
        unchanged(&mut sim, |s| {
            s.set_vertical_momentum(id, Fixed::from_map_units(65))
        });
        unchanged(&mut sim, |s| s.bind_player(0, id));
        let missile = sim.launch_projectile(fixtures::request(c)).unwrap();
        let before = sim.snapshot();
        assert!(
            sim.check_multisector_position(missile, [Fixed::from_bits(i32::MAX), Fixed::ZERO])
                .is_err()
        );
        assert_eq!(sim.snapshot(), before);
        unchanged(&mut sim, |s| s.enter_actor_state(missile, StateId(0)));
        unchanged(&mut sim, |s| s.request_actor_death(missile));
        unchanged(&mut sim, |s| s.set_actor_target(missile, None));
        unchanged(&mut sim, |s| s.set_momentum(missile, [Fixed::ZERO; 2]));
        unchanged(&mut sim, |s| s.set_vertical_momentum(missile, Fixed::ZERO));
    }
    #[test]
    fn metadata_action_flags_and_unbounded_work_reject_before_mutation() {
        let c = &fixtures::cases()[0];
        let r = fixtures::registry(c);
        let mut states = r.states().to_vec();
        states[107].action = ActorAction::Probe(ProbeAction::RemoveTarget);
        let altered = ActorRegistry::new(states, r.types().to_vec()).unwrap();
        assert!(
            Simulation::with_projectile_registry(
                fixtures::map(0),
                64,
                altered,
                ActorOptions::default()
            )
            .is_err()
        );
        let mut types = r.types().to_vec();
        types[34].properties[15] = 1;
        let altered = ActorRegistry::new(r.states().to_vec(), types).unwrap();
        assert!(
            Simulation::with_projectile_registry(
                fixtures::map(0),
                64,
                altered,
                ActorOptions::default()
            )
            .is_err()
        );
        assert!(
            Simulation::with_projectile_registry(
                fixtures::map(0),
                65,
                r.clone(),
                ActorOptions::default()
            )
            .is_err()
        );
        let mut sim = fixtures::setup(c);
        unchanged(&mut sim, |s| {
            s.spawn_type(ActorTypeId(11), [Fixed::ZERO; 2], SpawnHeight::Floor)
        });
        // Even a corrupted internal binding is rejected before launch/step; public setup
        // cannot create this state, and no player damage guard has been removed.
        sim.bindings[0] = Some(ActorId::from_parts(1, 1));
        unchanged(&mut sim, |s| s.launch_projectile(fixtures::request(c)));
        unchanged(&mut sim, |s| s.step([TicCommand::default(); 4]));
        unchanged(&mut sim, |s| {
            s.damage_actor(DamageRequest {
                target: ActorId::from_parts(1, 1),
                source: None,
                inflictor: None,
                amount: 5,
            })
        });
    }
    #[test]
    fn exact_contact_height_and_radius_boundaries_follow_source_predicates() {
        let c = &fixtures::cases()[0];
        let mut sim = fixtures::setup(c);
        let missile = sim.launch_projectile(fixtures::request(c)).unwrap();
        let target = ActorId::from_parts(1, 1);
        let a = *sim.actor(missile).unwrap();
        let b = *sim.actor(target).unwrap();
        let edge = b.position()[0] - a.radius() - b.radius();
        let pure = sim.snapshot();
        assert!(
            sim.check_multisector_position(missile, [edge, b.position()[1]])
                .unwrap()
                .allowed()
        );
        assert_eq!(sim.snapshot(), pure);
        assert_eq!(
            sim.query_projectile_move(missile, [edge + Fixed::from_bits(1), b.position()[1]])
                .1,
            Some(target)
        );
        sim.actors.get_mut(missile).unwrap().z = b.z + b.height();
        assert_eq!(
            sim.query_projectile_move(missile, b.position()).1,
            Some(target)
        );
        sim.actors.get_mut(missile).unwrap().z = b.z + b.height() + Fixed::from_bits(1);
        assert!(sim.query_projectile_move(missile, b.position()).0.allowed());
        sim.actors.get_mut(missile).unwrap().z = b.z - a.height();
        assert_eq!(
            sim.query_projectile_move(missile, b.position()).1,
            Some(target)
        );
        sim.actors.get_mut(missile).unwrap().z = b.z - a.height() - Fixed::from_bits(1);
        assert!(sim.query_projectile_move(missile, b.position()).0.allowed());
    }
    #[test]
    fn independent_runs_presentation_reads_pause_and_first_difference() {
        let c = &fixtures::cases()[2];
        let mut a = fixtures::setup(c);
        let mut b = fixtures::setup(c);
        fixtures::launch(&mut a, c);
        fixtures::launch(&mut b, c);
        for _ in 0..45 {
            let before = b.snapshot();
            for _ in 0..3 {
                let _ = b.snapshot().canonical_bytes();
                let _ = b.events();
                let _ = b.pending_events();
                let _ = b.actor_order().collect::<Vec<_>>();
            }
            assert_eq!(b.snapshot(), before);
            assert_eq!(
                a.step([TicCommand::default(); 4]).unwrap(),
                b.step([TicCommand::default(); 4]).unwrap()
            );
        }
        b.set_paused(true);
        let before = b.snapshot();
        assert_eq!(
            b.step(
                [TicCommand {
                    buttons: 255,
                    ..Default::default()
                }; 4]
            )
            .unwrap(),
            before
        );
        let d = a.snapshot().first_difference(&b.snapshot()).unwrap();
        assert_eq!(d.field, "paused");
        assert_eq!(a.snapshot().version(), 7);
    }
}
