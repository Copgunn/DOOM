//! Original missile launch, flight, contact and explosion. One algorithm serves the
//! frozen plasma profile (M3d2b) and the checked player world (`PlayerMonstersImpV1`).
use crate::{lifecycle::LifeHost, *};
use crate::{
    original_player::{MF_MISSILE as MISSILE, MF_SHADOW, MF_SHOOTABLE, MF_SOLID},
    world_pickups::MF_SPECIAL,
};
use std::{error::Error, fmt};
const PLASMA: ActorTypeId = ActorTypeId(34);
const FIREBALL: ActorTypeId = ActorTypeId(identity::OriginalActorType::MT_TROOPSHOT.ordinal());
/// p_mobj.c P_SpawnMissile: launch height above the source origin.
const MISSILE_SPAWN_HEIGHT: i16 = 32;
/// p_local.h MAXMOVE: 30 map units per tic.
const MAX_MOVE: i16 = 30;
/// p_mobj.c P_XYMovement: larger components split at MAXMOVE/2.
const MAX_STEP: i16 = MAX_MOVE / 2;
/// p_map.c PIT_CheckThing: `((P_Random()%8)+1)*info->damage`.
const CONTACT_DAMAGE_CHOICES: u8 = 8;
/// `ProjectileEvent::Random` draw kinds.
const DRAW_SPAWN: u8 = 0;
const DRAW_SHADOW_LEFT: u8 = 1;
const DRAW_SHADOW_RIGHT: u8 = 2;
const DRAW_CONTACT: u8 = 3;
const DRAW_EXPLODE: u8 = 4;
/// Live fireball bound for `PlayerMonstersImpV1`. Flying and exploding fireballs
/// both hold a thinker slot. One player, eight monsters, 16 items and eight drops
/// use 33 of the 64 slots; these 24 leave 7 for bullet impacts. Unscripted soaks of
/// eight Imps peaked at 19, so 16 froze ordinary runs. This is a measured margin, not
/// a proof: the source has no cap, and a launch past it rejects its tic, repeatedly,
/// until the caller changes the state.
pub const MAX_LIVE_PROJECTILES: usize = 24;
/// Source missile services shared by the frozen plasma profile and the checked
/// player world. Each host owns its RNG stream, event queue and damage path; the
/// algorithms below never branch on which host runs them.
pub(crate) trait MissileWorld {
    fn missile(&self, id: ActorId) -> Actor;
    fn missile_mut(&mut self, id: ActorId) -> &mut Actor;
    fn definition(&self, ty: ActorTypeId) -> &ActorTypeDef;
    fn emit(&mut self, event: Event);
    fn gameplay_draw(&mut self) -> u8;
    /// PIT_CheckThing/PIT_CheckLine result, with any thing to damage.
    fn contact(&self, id: ActorId, target: [Fixed; 2]) -> (SectorMoveAttempt, Option<ActorId>);
    /// P_DamageMobj from inside the block iterator.
    fn strike(&mut self, request: DamageRequest);
    fn enter_missile_state(&mut self, id: ActorId, state: StateId);
    fn relink(&mut self, id: ActorId);
}
fn observed_draw<W: MissileWorld>(w: &mut W, kind: u8) -> u8 {
    let value = w.gameplay_draw();
    w.emit(Event::Projectile(ProjectileEvent::Random { kind, value }));
    value
}
fn missile_sound<W: MissileWorld>(w: &mut W, id: ActorId, sound: u16) {
    if sound != 0 {
        w.emit(Event::ActorState(ActorStateEvent::Sound {
            actor: Some(id),
            sound,
        }));
    }
}
/// P_SpawnMissile aim: angle, `(dest z - source z) / flight time`, shadow.
fn actor_aim(source: &Actor, dest: &Actor, speed: Fixed) -> (u32, Fixed, bool) {
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
        dest.life.unwrap().flags & MF_SHADOW != 0,
    )
}
/// Everything after P_SpawnMobj in P_SpawnMissile, then P_CheckMissileSpawn.
fn finish_launch<W: MissileWorld>(
    w: &mut W,
    id: ActorId,
    source: ActorId,
    mut angle: u32,
    momz: Fixed,
    shadow: bool,
) {
    let ty = w.missile(id).life.unwrap().actor_type;
    let info = w.definition(ty);
    let (speed, sound) = (Fixed::from_bits(info.speed()), info.sounds()[0]);
    missile_sound(w, id, sound);
    w.missile_mut(id).life.as_mut().unwrap().target = Some(source);
    w.emit(Event::ActorState(ActorStateEvent::Target {
        actor: id,
        target: Some(source),
    }));
    if shadow {
        let left = observed_draw(w, DRAW_SHADOW_LEFT);
        let right = observed_draw(w, DRAW_SHADOW_RIGHT);
        angle = angle.wrapping_add(((i32::from(left) - i32::from(right)) * 1048576) as u32);
    }
    let phase = (angle >> 19) as usize;
    let momentum = [
        speed * Fixed::from_bits(FINE_SINE[phase + 2048]),
        speed * Fixed::from_bits(FINE_SINE[phase]),
    ];
    let a = w.missile_mut(id);
    a.spec.angle = angle;
    a.spec.momentum = momentum;
    a.momz = momz;
    w.emit(Event::Projectile(ProjectileEvent::Velocity {
        actor: id,
        angle,
        momentum: [momentum[0], momentum[1], momz],
    }));
    let random = observed_draw(w, DRAW_SPAWN);
    let a = w.missile_mut(id);
    let life = a.life.as_mut().unwrap();
    life.tics = (life.tics - i32::from(random & 3)).max(1);
    let tics = life.tics;
    for axis in 0..2 {
        a.spec.position[axis] =
            a.spec.position[axis] + Fixed::from_bits(a.spec.momentum[axis].bits() >> 1);
    }
    a.z = a.z + Fixed::from_bits(a.momz.bits() >> 1);
    let position = [a.position()[0], a.position()[1], a.z];
    w.emit(Event::Projectile(ProjectileEvent::HalfStep {
        actor: id,
        position,
        tics,
    }));
    let target = w.missile(id).position();
    if !try_move(w, id, target) {
        explode(w, id);
    }
}
/// P_ExplodeMissile.
fn explode<W: MissileWorld>(w: &mut W, id: ActorId) {
    w.emit(Event::Projectile(ProjectileEvent::Exploded(id)));
    let a = w.missile_mut(id);
    a.spec.momentum = [Fixed::ZERO; 2];
    a.momz = Fixed::ZERO;
    let ty = a.life.unwrap().actor_type;
    let info = w.definition(ty);
    let (death, sound) = (info.states()[5], info.sounds()[3]);
    w.enter_missile_state(id, death);
    let r = observed_draw(w, DRAW_EXPLODE);
    let life = w.missile_mut(id).life.as_mut().unwrap();
    life.tics = (life.tics - i32::from(r & 3)).max(1);
    life.flags &= !MISSILE;
    missile_sound(w, id, sound);
}
/// P_TryMove for a missile. Contact damage precedes the move result, as it runs
/// inside the source block iterator.
fn try_move<W: MissileWorld>(w: &mut W, id: ActorId, target: [Fixed; 2]) -> bool {
    let (attempt, hit) = w.contact(id, target);
    if let Some(victim) = hit {
        let life = w.missile(id).life.unwrap();
        let damage = w.definition(life.actor_type).damage();
        let draw = observed_draw(w, DRAW_CONTACT);
        w.strike(DamageRequest {
            target: victim,
            inflictor: Some(id),
            source: life.target,
            amount: (i32::from(draw % CONTACT_DAMAGE_CHOICES) + 1) * damage,
        });
    }
    let allowed = attempt.allowed();
    if allowed {
        let a = w.missile_mut(id);
        a.spec.position = target;
        a.floor = attempt.floor;
        a.ceiling = attempt.ceiling;
        a.location = Some(attempt.location);
        w.relink(id);
    }
    w.emit(Event::SectorMovement { actor: id, attempt });
    allowed
}
/// P_XYMovement for a thing that may stop being a missile mid-loop.
fn move_xy<W: MissileWorld>(w: &mut W, id: ActorId) {
    let a = w.missile_mut(id);
    let max = Fixed::from_map_units(MAX_MOVE);
    a.spec.momentum = a.spec.momentum.map(|m| m.clamp(-max, max));
    let mut left = a.spec.momentum;
    while left != [Fixed::ZERO; 2] {
        let mut target = w.missile(id).position();
        if left[0] > Fixed::from_map_units(MAX_STEP) || left[1] > Fixed::from_map_units(MAX_STEP) {
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
        if !try_move(w, id, target) {
            if w.missile(id).life.unwrap().flags & MISSILE != 0 {
                explode(w, id);
            } else {
                w.missile_mut(id).spec.momentum = [Fixed::ZERO; 2];
            }
        }
    }
}
/// P_ZMovement for a missile.
fn move_z<W: MissileWorld>(w: &mut W, id: ActorId) {
    let a = w.missile_mut(id);
    if a.z == a.floor && a.momz == Fixed::ZERO {
        return;
    }
    let before = [a.z, a.momz];
    let constraints = [a.floor, a.ceiling];
    let missile = a.life.unwrap().flags & MISSILE != 0;
    let mut floor = false;
    let mut ceiling = false;
    let mut gravity = false;
    let mut blast = false;
    a.z = a.z + a.momz;
    if a.z <= a.floor {
        floor = true;
        if a.momz < Fixed::ZERO {
            a.momz = Fixed::ZERO
        }
        a.z = a.floor;
        blast = missile;
    } else if a.gravity == Gravity::Normal {
        gravity = true;
        a.momz = if a.momz == Fixed::ZERO {
            -Fixed::from_map_units(2)
        } else {
            a.momz - Fixed::ONE
        };
    }
    if !blast && a.z + a.height() > a.ceiling {
        ceiling = true;
        if a.momz > Fixed::ZERO {
            a.momz = Fixed::ZERO
        }
        a.z = a.ceiling - a.height();
        blast = missile;
    }
    if blast {
        explode(w, id);
    }
    let a = w.missile(id);
    w.emit(Event::Vertical {
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
/// P_MobjThinker movement half: state countdown belongs to the caller.
pub(crate) fn fly<W: MissileWorld>(w: &mut W, id: ActorId) {
    if w.missile(id).momentum() != [Fixed::ZERO; 2] {
        move_xy(w, id);
    }
    move_z(w, id);
}
/// PIT_CheckThing/PIT_CheckLine over any actor lookup. Pure: the source draw
/// and P_DamageMobj in the missile branch are the caller's, after this returns.
fn missile_contact(
    (world, sectors): (&StaticWorld, &[SectorState]),
    spatial: &movement::SpatialIndex,
    lookup: &impl Fn(ActorId) -> Option<Actor>,
    id: ActorId,
    target: [Fixed; 2],
) -> (SectorMoveAttempt, Option<ActorId>) {
    let a = lookup(id).expect("live missile");
    let missile = a.life.unwrap().flags & MISSILE != 0;
    if !missile {
        return (
            multisector::check_move((world, sectors), id, &a, target, spatial, lookup, false),
            None,
        );
    }
    let owner = a.life.unwrap().target.unwrap();
    let owner_type = lookup(owner)
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
        world,
        id,
        &a,
        target,
        spatial,
        lookup,
        |candidate, other| {
            movement.actors.push(candidate);
            let life = other.life.unwrap();
            if life.flags & (MF_SOLID | MF_SPECIAL | MF_SHOOTABLE) == 0 {
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
            // MT_KNIGHT(17) and MT_BRUISER(15) are one species to the source.
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
            if life.flags & MF_SHOOTABLE == 0 {
                if life.flags & MF_SOLID == 0 {
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
        multisector::check_move_from_things((world, sectors), &a, target, movement, false, true),
        damage,
    )
}
impl MissileWorld for Simulation {
    fn missile(&self, id: ActorId) -> Actor {
        *self.actor(id).expect("live missile")
    }
    fn missile_mut(&mut self, id: ActorId) -> &mut Actor {
        self.actors.get_mut(id).expect("live missile")
    }
    fn definition(&self, ty: ActorTypeId) -> &ActorTypeDef {
        self.registry.as_deref().unwrap().actor_type(ty).unwrap()
    }
    fn emit(&mut self, event: Event) {
        self.pending.push(event);
    }
    fn gameplay_draw(&mut self) -> u8 {
        self.rng.p_random()
    }
    fn contact(&self, id: ActorId, target: [Fixed; 2]) -> (SectorMoveAttempt, Option<ActorId>) {
        self.query_projectile_move(id, target)
    }
    fn strike(&mut self, request: DamageRequest) {
        self.damage_actor(request)
            .expect("prevalidated projectile damage");
    }
    fn enter_missile_state(&mut self, id: ActorId, state: StateId) {
        self.enter_validated(id, state);
    }
    fn relink(&mut self, id: ActorId) {
        self.spatial.link(id);
    }
}
impl<A: lifecycle::ActorAccess> MissileWorld for LifeHost<'_, A> {
    fn missile(&self, id: ActorId) -> Actor {
        *self.access.get(id).expect("live missile")
    }
    fn missile_mut(&mut self, id: ActorId) -> &mut Actor {
        self.access.get_mut(id).expect("live missile")
    }
    fn definition(&self, ty: ActorTypeId) -> &ActorTypeDef {
        self.registry.actor_type(ty).expect("admitted missile")
    }
    fn emit(&mut self, event: Event) {
        self.events.push(event);
    }
    fn gameplay_draw(&mut self) -> u8 {
        self.rng.p_random()
    }
    fn contact(&self, id: ActorId, target: [Fixed; 2]) -> (SectorMoveAttempt, Option<ActorId>) {
        missile_contact(
            (self.world, self.sectors),
            self.spatial,
            &|i| self.access.get(i).copied(),
            id,
            target,
        )
    }
    fn strike(&mut self, request: DamageRequest) {
        match combat::DamagePlan::current_original(self, request).and_then(|plan| {
            self.access.reserve(plan.spawn_count())?;
            Ok(plan)
        }) {
            Ok(plan) => {
                self.apply_damage(plan);
            }
            // Keep the first latched cause; the staged tic is discarded either way.
            Err(error) if !self.sight_failed() => self.fail_sight(error),
            Err(_) => {}
        }
    }
    fn enter_missile_state(&mut self, id: ActorId, state: StateId) {
        self.enter(id, state);
    }
    fn relink(&mut self, id: ActorId) {
        self.spatial.link(id);
    }
}
/// A_TroopAttack's P_SpawnMissile inside the checked transaction. The live cap
/// and slot reservation reject before this launch changes anything; the caller
/// latches the error, which discards the tic's earlier staged effects.
pub(crate) fn launch_fireball<A: lifecycle::ActorAccess>(
    h: &mut LifeHost<'_, A>,
    source: ActorId,
    dest: ActorId,
) -> Result<(), SimError> {
    let bound = h.player.as_deref().map_or(MAX_LIVE_PROJECTILES, |p| {
        crate::player_map_scale::live_projectile_bound(p.policy)
    });
    if h.access.live_of_type(FIREBALL) >= bound {
        return Err(ProjectileError::WorkLimit.into());
    }
    h.access.reserve(1)?;
    let from = *h.access.get(source).ok_or(ProjectileError::Reference)?;
    let to = *h.access.get(dest).ok_or(ProjectileError::Reference)?;
    let speed = Fixed::from_bits(h.definition(FIREBALL).speed());
    let (angle, momz, shadow) = actor_aim(&from, &to, speed);
    if !within(momz, 64) {
        return Err(ProjectileError::Arithmetic.into());
    }
    h.emit(Event::Projectile(ProjectileEvent::Launch(
        ProjectileRequest {
            source,
            actor_type: FIREBALL,
            aim: ProjectileAim::Actor(dest),
        },
    )));
    let id = h.spawn(
        FIREBALL,
        from.position(),
        SpawnHeight::Absolute(from.z + Fixed::from_map_units(MISSILE_SPAWN_HEIGHT)),
    );
    finish_launch(h, id, source, angle, momz, shadow);
    Ok(())
}
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
    /// `PlayerMonstersImpV1`: the live cap and each flying fireball's owner.
    pub(crate) fn preflight_live_missiles(&self) -> Result<(), SimError> {
        if !self.player_policy().has_imps() {
            return Ok(());
        }
        if self.actors.live_of_type(FIREBALL) > self.live_projectile_bound() {
            return Err(ProjectileError::WorkLimit.into());
        }
        for id in self.actors.order() {
            let life = self.actor(id).and_then(Actor::life).unwrap();
            if life.actor_type == FIREBALL
                && life.flags & MISSILE != 0
                && self.actors.is_pending(id) == Some(false)
            {
                let owner = life.target.ok_or(ProjectileError::Owner)?;
                if self.actors.is_pending(owner) != Some(false) {
                    return Err(ProjectileError::Owner.into());
                }
            }
        }
        Ok(())
    }
    pub(crate) fn preflight_owner_removal(&self, owner: ActorId) -> Result<(), SimError> {
        if !self.player_policy().has_imps() {
            return Ok(());
        }
        let owns_flight = self.actors.order().any(|id| {
            self.actors.is_pending(id) == Some(false)
                && self.actor(id).and_then(Actor::life).is_some_and(|l| {
                    l.actor_type == FIREBALL && l.flags & MISSILE != 0 && l.target == Some(owner)
                })
        });
        if owns_flight {
            return Err(ProjectileError::Owner.into());
        }
        Ok(())
    }
    pub(crate) fn projectile_control(&self, id: ActorId) -> Result<(), SimError> {
        let ty = self.actor(id).and_then(Actor::life).map(|l| l.actor_type);
        if (self.profile == Profile::Projectiles && ty == Some(PLASMA))
            || (self.player_policy().has_imps() && ty == Some(FIREBALL))
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
        let (angle, momz, shadow) = match request.aim {
            ProjectileAim::Actor(id) => actor_aim(&source, &self.projectile_source(id)?, speed),
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
            player: None,
            statistics: self.statistics.as_mut(),
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
            SpawnHeight::Absolute(source.z + Fixed::from_map_units(MISSILE_SPAWN_HEIGHT)),
        );
        finish_launch(self, id, request.source, angle, momz, shadow);
        Ok(id)
    }
    pub(crate) fn query_projectile_move(
        &self,
        id: ActorId,
        target: [Fixed; 2],
    ) -> (SectorMoveAttempt, Option<ActorId>) {
        missile_contact(
            (&self.world, &self.sectors),
            &self.spatial,
            &|id| self.actor(id).copied(),
            id,
            target,
        )
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
                        fly(self, id);
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
                                movement::MotionEvent::Contact(event) => event.event(),
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
                            player: None,
                            statistics: self.statistics.as_mut(),
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
