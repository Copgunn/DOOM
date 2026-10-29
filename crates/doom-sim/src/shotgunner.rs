//! Source SPOS10/10/10 with bounded sequential shared-combat projection.
use crate::{
    lifecycle::{ActorAccess, LifeHost},
    *,
};
/// Chronological source shotgun observations, separate from persistent state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShotgunEvent {
    /// Non-null A_SPosAttack entry, before shotgun sound.
    Begin {
        /// Native owner.
        actor: ActorId,
    },
    /// Loop boundary before this pellet's three attack draws.
    PelletStart {
        /// Native owner.
        actor: ActorId,
        /// Source ordinal1..3.
        index: u8,
    },
    /// Individual source parameters, after spread/damage draws and before tracing.
    Pellet {
        /// Native owner.
        actor: ActorId,
        /// Source ordinal1..3.
        index: u8,
        /// Wrapping BAM angle.
        angle: u32,
        /// Shared original aim slope.
        slope: Fixed,
        /// Independent damage3..15.
        damage: i32,
    },
    /// All impact/damage effects completed before the next pellet.
    PelletDone {
        /// Native owner.
        actor: ActorId,
        /// Source ordinal1..3.
        index: u8,
        /// Gameplay RNG index after this pellet.
        rng: u8,
        /// Actual effect, absent for a miss.
        effect: Option<ActorId>,
    },
    /// Complete original three-pellet action.
    Completed {
        /// Native owner.
        actor: ActorId,
        /// Final gameplay index.
        rng: u8,
    },
}
#[derive(Clone, Debug)]
struct PelletPlan {
    start: DoomRng,
    end: DoomRng,
    attack: crate::combat::PreparedAttack,
}
#[derive(Clone, Debug)]
pub(crate) struct ShotgunPlan {
    owner: ActorId,
    initial: DoomRng,
    angle: u32,
    aim: AimResult,
    pellets: [PelletPlan; 3],
}
/// Only touched bodies and projected NOBLOCKMAP impacts are copied. No store clone.
struct Projection<'a> {
    base: &'a ThinkerStore<Actor>,
    touched: Vec<(ActorId, Actor)>,
    born: Vec<(ActorId, Actor)>,
    ids: [ActorId; 3],
}
impl Projection<'_> {
    fn read(&self, id: ActorId) -> Option<&Actor> {
        self.born
            .iter()
            .chain(&self.touched)
            .find(|(i, _)| *i == id)
            .map(|(_, a)| a)
            .or_else(|| self.base.get(id))
    }
}
impl ActorAccess for &mut Projection<'_> {
    fn get(&self, id: ActorId) -> Option<&Actor> {
        self.read(id)
    }
    fn get_mut(&mut self, id: ActorId) -> Option<&mut Actor> {
        if let Some(i) = self.born.iter().position(|(i, _)| *i == id) {
            return Some(&mut self.born[i].1);
        }
        let i = if let Some(i) = self.touched.iter().position(|(i, _)| *i == id) {
            i
        } else {
            let a = *self.base.get(id)?;
            assert!(
                self.touched.len() < 3,
                "only three hit bodies can be touched"
            );
            self.touched.push((id, a));
            self.touched.len() - 1
        };
        Some(&mut self.touched[i].1)
    }
    fn spawn(&mut self, a: Actor) -> Result<ActorId, StoreError> {
        assert!(self.born.len() < 3);
        let id = self.ids[self.born.len()];
        self.born.push((id, a));
        Ok(id)
    }
    fn mark(&mut self, _: ActorId) -> Result<bool, StoreError> {
        unreachable!(
            "strict source-body and positive impact entry states never remove within a blast"
        )
    }
}
fn projected_ids(store: &ThinkerStore<Actor>) -> [ActorId; 3] {
    let mut ids = store
        .slots
        .iter()
        .enumerate()
        .filter(|(_, s)| s.entry.is_none() && !s.retired)
        .map(|(i, s)| ActorId::from_parts(i as u32, s.generation))
        .take(3)
        .collect::<Vec<_>>();
    let mut next = store.slots.len() as u32;
    while ids.len() < 3 {
        assert!(next < store.max_slots);
        ids.push(ActorId::from_parts(next, 1));
        next += 1;
    }
    ids.try_into().unwrap()
}
fn observe<A: ActorAccess>(h: &mut LifeHost<'_, A>, event: ShotgunEvent) {
    h.events.push(Event::Shotgun(event));
}
pub(crate) fn fire<A: ActorAccess>(h: &mut LifeHost<'_, A>, id: ActorId) {
    if h.access.get(id).unwrap().life.unwrap().target.is_none() {
        return;
    }
    let plan = h
        .perception
        .unwrap()
        .shotgun
        .expect("complete blast preflight precedes mutation")
        .clone();
    assert_eq!(plan.owner, id);
    assert_eq!(plan.initial, *h.rng);
    observe(h, ShotgunEvent::Begin { actor: id });
    h.sound(Some(id), lifecycle::source_sound("sfx_shotgn").unwrap());
    crate::monsters::face(h, id);
    assert_eq!(h.access.get(id).unwrap().angle(), plan.angle);
    h.events.push(Event::Monster(MonsterEvent::Aim {
        actor: id,
        result: Box::new(plan.aim.clone()),
    }));
    for (i, pellet) in plan.pellets.into_iter().enumerate() {
        let index = i as u8 + 1;
        assert_eq!(*h.rng, pellet.start);
        observe(h, ShotgunEvent::PelletStart { actor: id, index });
        let left = i32::from(crate::monsters::draw(h, id, 2));
        let right = i32::from(crate::monsters::draw(h, id, 3));
        let angle = plan.angle.wrapping_add(((left - right) * 1048576) as u32);
        let damage = (i32::from(crate::monsters::draw(h, id, 4)) % 5 + 1) * 3;
        let params = pellet.attack.parameters();
        assert_eq!(
            (angle, damage, plan.aim.slope),
            (params.angle, params.damage, params.slope)
        );
        observe(
            h,
            ShotgunEvent::Pellet {
                actor: id,
                index,
                angle,
                slope: plan.aim.slope,
                damage,
            },
        );
        let result = crate::combat::execute_attack(h, pellet.attack);
        assert_eq!(*h.rng, pellet.end);
        observe(
            h,
            ShotgunEvent::PelletDone {
                actor: id,
                index,
                rng: h.rng.indices()[0],
                effect: result.effect,
            },
        );
    }
    observe(
        h,
        ShotgunEvent::Completed {
            actor: id,
            rng: h.rng.indices()[0],
        },
    );
}
impl Simulation {
    /// Explicit bounded original SPosAttack fixtures; no full Shotgun Guy admission.
    pub fn with_shotgunner_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if limit > 64 {
            return Err(PerceptionError::WorkLimit.into());
        }
        if map.linedefs().iter().any(|l| l.special != 0)
            || map.sectors().iter().any(|s| s.special != 0)
        {
            return Err(PursuitError::Specials.into());
        }
        crate::sight::validate_map(&map)?;
        validate_registry(&registry)?;
        let mut sim = Self::with_combat_registry(map, limit, registry, options)?;
        sim.profile = Profile::Shotgunner;
        sim.sound_targets = vec![None; sim.sectors.len()];
        Ok(sim)
    }
    pub(crate) fn validate_shotgun_type(&self, ty: ActorTypeId) -> Result<(), SimError> {
        if ![37, 38, 135, 136].contains(&ty.0) {
            return Err(MonsterError::Definition.into());
        }
        let info = self
            .registry
            .as_deref()
            .unwrap()
            .actor_type(ty)
            .ok_or(MonsterError::Definition)?;
        if ty.0 == 136 && info.name != "FIXTURE_SHOTGUNNER_136"
            || ty.0 == 135 && info.name != "FIXTURE_DAMAGE_TARGET"
        {
            return Err(MonsterError::Definition.into());
        }
        self.registry
            .as_deref()
            .unwrap()
            .validate_type_shotgunner(ty, &self.sectors)?;
        Ok(())
    }
    pub(crate) fn shotgun_state_role(
        &self,
        ty: ActorTypeId,
        state: StateId,
    ) -> Result<(), SimError> {
        let valid = match ty.0 {
            136 => (149..=173).contains(&state.0) || [1, 2, 4, 5, 6].contains(&state.0),
            135 => (149..=173).contains(&state.0),
            37 | 38 => (90..=96).contains(&state.0),
            _ => false,
        };
        if !valid {
            return Err(MonsterError::Definition.into());
        }
        Ok(())
    }
    pub(crate) fn preflight_shotgun_step(&mut self) -> Result<Option<ShotgunPlan>, SimError> {
        self.preflight_monster_owner()?
            .map(|id| self.prepare_shotgun(id))
            .transpose()
            .map(Option::flatten)
    }
    fn prepare_shotgun(&mut self, id: ActorId) -> Result<Option<ShotgunPlan>, SimError> {
        self.preflight_monster_callback(id)?;
        let a = *self.actor(id).unwrap();
        let Some(target) = a.life.unwrap().target else {
            return Ok(None);
        };
        let t = *self.actor(target).unwrap();
        // Conservative source-capacity policy: no pending-removal credit, even on misses.
        self.reserve_actor_spawns(3)?;
        let initial = self.rng;
        let mut rng = initial;
        let angle = crate::monsters::face_angle(a, t, &mut rng);
        let range = Fixed::from_map_units(2048);
        let aim = crate::combat_trace::aim(self, id, angle, range)?;
        let mut projection = Projection {
            base: &self.actors,
            touched: Vec::new(),
            born: Vec::new(),
            ids: projected_ids(&self.actors),
        };
        let mut spatial = movement::SpatialIndex::default();
        let mut events = Vec::new();
        let mut bindings = [None; 4];
        let mut pellets = Vec::new();
        for _ in 0..3 {
            let start = rng;
            let left = i32::from(rng.p_random());
            let right = i32::from(rng.p_random());
            let shot_angle = angle.wrapping_add(((left - right) * 1048576) as u32);
            let damage = (i32::from(rng.p_random()) % 5 + 1) * 3;
            // Body positions/radii/index flags cannot change in this closure; hit/damage
            // eligibility and state DO change, and are read from the projected overlay.
            let attack = self.prepare_attack_lookup(
                id,
                LineAttack {
                    angle: shot_angle,
                    range,
                    slope: aim.slope,
                    damage,
                },
                None,
                Some(&|id| projection.read(id).copied()),
            )?;
            if attack.result.position.is_some_and(|p| {
                !crate::sight::within(p[2], 1018)
                    || p[..2].iter().any(|v| !crate::sight::within(*v, 994))
            }) {
                return Err(MonsterError::Arithmetic.into());
            }
            let mut host = LifeHost {
                statistics: self.statistics.as_mut(),
                perception: None,
                access: &mut projection,
                registry: self.registry.as_deref().unwrap(),
                world: &self.world,
                sectors: &self.sectors,
                options: self.actor_options,
                rng: &mut rng,
                spatial: &mut spatial,
                bindings: &mut bindings,
                events: &mut events,
            };
            crate::combat::execute_attack(&mut host, attack.clone());
            events.clear();
            pellets.push(PelletPlan {
                start,
                end: rng,
                attack,
            });
        }
        Ok(Some(ShotgunPlan {
            owner: id,
            initial,
            angle,
            aim,
            pellets: pellets.try_into().unwrap(),
        }))
    }
    /// Atomic source A_SPosAttack: one shared aim and three sequential real pellets.
    pub fn shotgun_attack(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.profile != Profile::Shotgunner {
            return Err(MonsterError::Profile.into());
        }
        let plan = self.prepare_shotgun(id)?;
        let mut host = LifeHost {
            statistics: self.statistics.as_mut(),
            perception: Some(crate::perception::PerceptionInputs {
                players: &self.perception_players,
                sound_targets: &self.sound_targets,
                pursuit: true,
                monsters: true,
                shotgunner: true,
                chaingunner: false,
                demon: false,
                map_demons: false,
                checked: None,
                melee: None,
                shotgun: plan.as_ref(),
                shot: None,
            }),
            access: &mut self.actors,
            registry: self.registry.as_deref().unwrap(),
            world: &self.world,
            sectors: &self.sectors,
            options: self.actor_options,
            rng: &mut self.rng,
            spatial: &mut self.spatial,
            bindings: &mut self.bindings,
            events: &mut self.pending,
        };
        host.events.push(Event::ActorState(ActorStateEvent::Action {
            actor: id,
            action: ActorAction::Original(lifecycle::source_action_id("A_SPosAttack")),
        }));
        fire(&mut host, id);
        Ok(())
    }
    pub(crate) fn enter_shotgun_state(
        &mut self,
        id: ActorId,
        state: StateId,
    ) -> Result<(), SimError> {
        let a = self.monster_actor(id, 1008)?;
        self.shotgun_state_role(a.life.unwrap().actor_type, state)?;
        if self.monster_is_owner(id) {
            self.preflight_monster_callback(id)?;
        }
        self.validate_entry(state)?;
        let plan = if state == StateId(5) {
            self.prepare_shotgun(id)?
        } else {
            None
        };
        self.enter_attack_planned(id, state, None, plan.as_ref());
        Ok(())
    }
}
fn validate_registry(r: &ActorRegistry) -> Result<(), SimError> {
    let source = ActorRegistry::vanilla();
    if r.states() == source.states() && r.types() == source.types() {
        return Ok(());
    }
    if r.states().len() != source.states().len() || r.types().len() != source.types().len() {
        return Err(MonsterError::Definition.into());
    }
    for (i, s) in r.states().iter().enumerate() {
        let expected = match i {
            1 | 2 => ActorStateDef {
                name: if i == 1 {
                    "FIXTURE_SPOS_LOOK"
                } else {
                    "FIXTURE_SPOS_CHASE"
                }
                .into(),
                sprite: 30,
                frame: 0,
                tics: s.tics,
                action: ActorAction::Original(lifecycle::source_action_id(if i == 1 {
                    "A_Look"
                } else {
                    "A_Chase"
                })),
                next: StateId(i as u16),
                misc: [0; 2],
            },
            4..=6 => {
                let original = source
                    .states()
                    .iter()
                    .find(|s| s.name == format!("S_SPOS_ATK{}", i - 3))
                    .unwrap();
                let mut expected = original.clone();
                expected.name = format!("FIXTURE_SPOS_ATK{}", i - 3);
                expected.next = StateId(if i == 6 { 2 } else { i as u16 + 1 });
                expected
            }
            _ => source.states()[i].clone(),
        };
        if *s != expected || [1, 2].contains(&i) && !(1..=35).contains(&s.tics) {
            return Err(MonsterError::Definition.into());
        }
    }
    for (i, t) in r.types().iter().enumerate() {
        if ![135, 136].contains(&i) {
            if t != &source.types()[i] {
                return Err(MonsterError::Definition.into());
            }
            continue;
        }
        let owner = i == 136;
        let mut expected = source.types()[0].clone();
        expected.name = if owner {
            "FIXTURE_SHOTGUNNER_136"
        } else {
            "FIXTURE_DAMAGE_TARGET"
        }
        .into();
        expected.properties[0] = -1;
        let variable: &[usize] = if owner {
            &[4, 15, 16, 17, 20, 21]
        } else {
            &[2, 8, 16, 17, 21]
        };
        for &key in variable {
            expected.properties[key] = t.properties[key];
        }
        if owner {
            expected.properties[1] = 1;
            expected.properties[3] = 2;
            expected.properties[5] = 8;
            expected.properties[10] = 0;
            expected.properties[11] = 4;
        }
        let flags = if owner {
            2 | 32 | 64 | 128 | 1024 | 2048 | 2097152 | 33554432
        } else {
            2 | 4 | 32 | 64 | 1024 | 2048 | 262144 | 524288 | 33554432
        };
        if *t != expected
            || t.radius() < Fixed::ZERO
            || t.radius() > Fixed::from_map_units(32)
            || t.height() <= Fixed::ZERO
            || t.flags() & !flags != 0
            || owner && !(1..=16).contains(&t.speed())
            || !owner
                && (!(1..=1000).contains(&t.spawn_health())
                    || !(0..=256).contains(&t.pain_chance()))
        {
            return Err(MonsterError::Definition.into());
        }
    }
    Ok(())
}
pub(crate) fn event_bytes(b: &mut Vec<u8>, e: &ShotgunEvent) {
    fn id(b: &mut Vec<u8>, id: ActorId) {
        b.extend(id.slot().to_le_bytes());
        b.extend(id.generation().to_le_bytes());
    }
    match e {
        ShotgunEvent::Begin { actor } => {
            b.push(0);
            id(b, *actor);
        }
        ShotgunEvent::PelletStart { actor, index } => {
            b.push(1);
            id(b, *actor);
            b.push(*index);
        }
        ShotgunEvent::Pellet {
            actor,
            index,
            angle,
            slope,
            damage,
        } => {
            b.push(2);
            id(b, *actor);
            b.push(*index);
            b.extend(angle.to_le_bytes());
            b.extend(slope.bits().to_le_bytes());
            b.extend(damage.to_le_bytes());
        }
        ShotgunEvent::PelletDone {
            actor,
            index,
            rng,
            effect,
        } => {
            b.push(3);
            id(b, *actor);
            b.extend([*index, *rng, u8::from(effect.is_some())]);
            id(b, effect.unwrap_or(ActorId::from_parts(0, 0)));
        }
        ShotgunEvent::Completed { actor, rng } => {
            b.push(4);
            id(b, *actor);
            b.push(*rng);
        }
    }
}
