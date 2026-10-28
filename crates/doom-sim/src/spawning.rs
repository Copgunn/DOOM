//! Profile-4 setup. Recoverable validation always precedes gameplay mutation.
use crate::{
    lifecycle::{Demand, LifeHost},
    *,
};

/// Results for the supported single-player THINGS subset, in source order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MapSpawnReport {
    /// Admitted ordinary actors.
    pub spawned: Vec<ActorId>,
    /// Deferred starts; complete P_SpawnPlayer remains unsupported.
    pub player_starts: Vec<doom_map::Thing>,
    /// First ten deathmatch starts, collected before ordinary filters.
    pub deathmatch_starts: Vec<doom_map::Thing>,
    /// Placements skipped by single-player, skill or nomonsters rules.
    pub filtered: usize,
}

impl Simulation {
    /// Profile 4, all recognized source metadata, medium single-player skill,
    /// 1024 slots. Map THINGS are not spawned implicitly.
    pub fn actor_states(map: Map) -> Result<Self, SimError> {
        Self::with_actor_registry(map, 1024, ActorRegistry::vanilla(), ActorOptions::default())
    }
    /// Explicit immutable definitions and options. Native probe definitions do
    /// not implement a scripting language or mod compatibility layer.
    pub fn with_actor_registry(
        map: Map,
        max_actors: u32,
        registry: ActorRegistry,
        options: ActorOptions,
    ) -> Result<Self, SimError> {
        if map.things().len() > 4096 {
            return Err(ActorError::WorkLimit.into());
        }
        if max_actors > 1024 {
            return Err(StoreError::InvalidLimit.into());
        }
        if options.mode != ActorGameMode::SinglePlayer {
            return Err(ActorError::UnsupportedMode.into());
        }
        let mut sim = Self::create(map, max_actors, Profile::ActorStates)?;
        snapshot::registry_fields(Arc::make_mut(&mut sim.map_fields), &registry);
        sim.registry = Some(Arc::new(registry));
        sim.actor_options = options;
        Ok(sim)
    }
    /// Read-only registry, absent in earlier physics profiles.
    pub fn actor_registry(&self) -> Option<&ActorRegistry> {
        self.registry.as_deref()
    }
    /// Immutable placement options.
    pub const fn actor_options(&self) -> ActorOptions {
        self.actor_options
    }
    fn registry_required(&self) -> Result<&ActorRegistry, SimError> {
        self.registry
            .as_deref()
            .ok_or(ActorError::UnsupportedMode.into())
    }
    pub(crate) fn active_typed(&self, id: ActorId) -> Result<ActorLife, SimError> {
        self.registry_required()?;
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        self.actor(id)
            .and_then(Actor::life)
            .ok_or(ActorError::InvalidType.into())
    }
    pub(crate) fn reserve_actor_spawns(&mut self, count: usize) -> Result<(), SimError> {
        self.actors.reserve_spawns(count)?;
        self.spatial
            .order
            .try_reserve(count)
            .map_err(|_| StoreError::Capacity)?;
        Ok(())
    }
    /// Supported P_SpawnMobj: assign the spawn state directly (no action), set
    /// type properties/height, draw lastlook once, link and append the thinker.
    pub fn spawn_type(
        &mut self,
        ty: ActorTypeId,
        position: [Fixed; 2],
        height: SpawnHeight,
    ) -> Result<ActorId, SimError> {
        if self.profile == Profile::Projectiles {
            if ty == ActorTypeId(34) {
                return Err(ProjectileError::Controlled.into());
            }
            self.projectile_setup_position(position, height)?;
            self.validate_combat_type(ty)?;
        } else if matches!(self.profile, Profile::Combat | Profile::PlayerWeapons) {
            self.validate_combat_type(ty)?;
        } else {
            self.registry_required()?.validate_type(ty, &self.sectors)?;
        }
        self.reserve_actor_spawns(1)?;
        Ok(LifeHost {
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
        .spawn(ty, position, height))
    }
    pub(crate) fn validate_entry(&self, state: StateId) -> Result<(), SimError> {
        let registry = self.registry_required()?;
        registry.validate_state(state)?;
        for ty in registry.spawn_dependencies(state)? {
            if self.profile == Profile::Projectiles {
                registry.validate_type_projectile(ty, &self.sectors)?;
            } else if matches!(self.profile, Profile::Combat | Profile::PlayerWeapons) {
                registry.validate_type_combat(ty, &self.sectors)?;
            } else {
                registry.validate_type(ty, &self.sectors)?;
            }
        }
        Ok(())
    }
    pub(crate) fn enter_validated(&mut self, id: ActorId, state: StateId) {
        LifeHost {
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
        .enter(id, state);
    }
    /// Enter a validated state immediately between tics. Work, action availability
    /// and child capacity are checked before any callback or gameplay RNG draw.
    pub fn enter_actor_state(&mut self, id: ActorId, state: StateId) -> Result<(), SimError> {
        let life = self.active_typed(id)?;
        self.projectile_control(id)?;
        self.validate_entry(state)?;
        let demand = self.registry_required()?.demand(state, life.tics)?;
        self.reserve_actor_spawns(demand.spawns)?;
        self.enter_validated(id, state);
        Ok(())
    }
    /// Controlled death-state entry for an unbound actor. This does not apply
    /// damage, alter health/height, set corpse flags, award kills or drop items.
    pub fn request_actor_death(&mut self, id: ActorId) -> Result<(), SimError> {
        let life = self.active_typed(id)?;
        self.projectile_control(id)?;
        if self.bindings.contains(&Some(id)) {
            return Err(ActorError::BoundDeath.into());
        }
        let state = self
            .registry_required()?
            .actor_type(life.actor_type)
            .unwrap()
            .states()[5];
        if state.0 == 0 {
            return Err(ActorError::NoDeathState.into());
        }
        self.validate_entry(state)?;
        let demand = self.registry_required()?.demand(state, life.tics)?;
        self.reserve_actor_spawns(demand.spawns)?;
        self.actors
            .get_mut(id)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .death_requested = true;
        self.pending
            .push(Event::ActorState(ActorStateEvent::Death(id)));
        self.enter_validated(id, state);
        Ok(())
    }
    /// Assign a validated target. Self and pending targets are permitted. Stored
    /// references remain unchanged when their targets retire, as in the source.
    pub fn set_actor_target(
        &mut self,
        id: ActorId,
        target: Option<ActorId>,
    ) -> Result<(), SimError> {
        self.active_typed(id)?;
        self.projectile_control(id)?;
        if target.is_some_and(|target| self.actor(target).is_none()) {
            return Err(StoreError::InvalidHandle.into());
        }
        self.actors
            .get_mut(id)
            .unwrap()
            .life
            .as_mut()
            .unwrap()
            .target = target;
        self.pending
            .push(Event::ActorState(ActorStateEvent::Target {
                actor: id,
                target,
            }));
        Ok(())
    }
    /// Resolve by the stored generation. Pending storage is readable; retired or
    /// reused storage resolves to None, without silently clearing the identity.
    pub fn actor_target(&self, id: ActorId) -> Option<&Actor> {
        self.actor(self.actor(id)?.life?.target?)
    }
    pub(crate) fn preflight_actor_step(&mut self) -> Result<(), SimError> {
        let Some(registry) = &self.registry else {
            return Ok(());
        };
        let mut demand = Demand::default();
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            let life = self.actors.get(id).unwrap().life.unwrap();
            if life.tics == 1 {
                demand
                    .add(registry.demand(registry.state(life.state.unwrap()).unwrap().next, 0)?)?;
            }
        }
        self.reserve_actor_spawns(demand.spawns)
    }
    /// Atomic ordinary placement in source order. Starts are reported without
    /// creating players; repeated calls explicitly repeat placement.
    pub fn spawn_map_things(&mut self) -> Result<MapSpawnReport, SimError> {
        let registry = self.registry_required()?;
        let mut report = MapSpawnReport::default();
        let mut selected = Vec::new();
        for thing in self.world.map().things() {
            if thing.kind == 11 {
                if report.deathmatch_starts.len() < 10 {
                    report.deathmatch_starts.push(*thing);
                }
                continue;
            }
            if (1..=4).contains(&thing.kind) {
                report.player_starts.push(*thing);
                continue;
            }
            let bit = match self.actor_options.skill {
                Skill::Baby | Skill::Easy => 1,
                Skill::Medium => 2,
                Skill::Hard | Skill::Nightmare => 4,
            };
            if thing.options & 16 != 0 || thing.options & bit == 0 {
                report.filtered += 1;
                continue;
            }
            let ty = registry
                .find_doomed(i32::from(thing.kind))
                .ok_or(ActorError::InvalidType)?;
            let info = registry.actor_type(ty).unwrap();
            if self.actor_options.no_monsters
                && (info.flags() & 4194304 != 0 || lifecycle::source_type_is(ty, "MT_SKULL"))
            {
                report.filtered += 1;
                continue;
            }
            if self.profile == Profile::Projectiles {
                if ty == ActorTypeId(34) {
                    return Err(ProjectileError::Controlled.into());
                }
                self.projectile_setup_position(
                    [
                        Fixed::from_map_units(thing.position.x),
                        Fixed::from_map_units(thing.position.y),
                    ],
                    SpawnHeight::Floor,
                )?;
                self.validate_combat_type(ty)?;
            } else if matches!(self.profile, Profile::Combat | Profile::PlayerWeapons) {
                registry.validate_type_combat(ty, &self.sectors)?;
            } else {
                registry.validate_type(ty, &self.sectors)?;
            }
            if matches!(self.profile, Profile::Combat | Profile::PlayerWeapons) {
                self.validate_combat_type(ty)?;
            }
            selected.push((*thing, ty));
        }
        self.reserve_actor_spawns(selected.len())?;
        // No recoverable operation follows reservation: all selected definitions
        // were validated together. This preserves batch failure atomicity.
        for (thing, ty) in selected {
            let info = self.registry.as_deref().unwrap().actor_type(ty).unwrap();
            let height = if info.flags() & 256 != 0 {
                SpawnHeight::Ceiling
            } else {
                SpawnHeight::Floor
            };
            let id = LifeHost {
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
                ty,
                [
                    Fixed::from_map_units(thing.position.x),
                    Fixed::from_map_units(thing.position.y),
                ],
                height,
            );
            let actor = self.actors.get_mut(id).unwrap();
            let life = actor.life.as_mut().unwrap();
            life.spawnpoint = Some(thing);
            if life.tics > 0 {
                life.tics = 1 + i32::from(self.rng.p_random()) % life.tics;
            }
            actor.spec.angle = ((i32::from(thing.angle) / 45) as u32).wrapping_mul(0x20000000_u32);
            if thing.options & 8 != 0 {
                life.flags |= 32;
            }
            report.spawned.push(id);
        }
        Ok(report)
    }
}
