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
        if self.player_policy().has_world_pickups()
            && world_pickups::kind_for(self.player_policy(), ty).is_some()
        {
            return Err(MapSpawnError::DirectSpawn.into());
        }
        self.player_spawn_guard(ty, position, height)?;
        if self.profile.uses_demon_map() && ty == self.demon_owner_type() {
            return Err(MapSpawnError::DirectSpawn.into());
        }
        if self.profile.uses_perception()
            && (!self.spawn_admitted(
                position,
                self.registry
                    .as_deref()
                    .and_then(|r| r.actor_type(ty))
                    .map_or(Fixed::ZERO, ActorTypeDef::radius),
                1024,
            ) || matches!(height,SpawnHeight::Absolute(z) if !crate::sight::within(z,1024)))
        {
            return Err(PerceptionError::Arithmetic.into());
        }
        if self.player_policy().has_imps() && crate::original_registry::is_fireball(ty) {
            return Err(ProjectileError::Controlled.into());
        }
        if self.profile.uses_perception() {
            self.validate_perception_type(ty)?;
        } else if self.profile == Profile::Projectiles {
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
        if self.profile.uses_monsters() {
            self.preflight_monster_spawn(ty, position, height)?;
        }
        self.reserve_actor_spawns(1)?;
        Ok(LifeHost {
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
        .spawn(ty, position, height))
    }
    pub(crate) fn validate_entry(&self, state: StateId) -> Result<(), SimError> {
        let registry = self.registry_required()?;
        // The Imp's melee root is direct-entry, unlike any former-human attack.
        if self.player_policy().has_imps() {
            registry.validate_state_original_monsters(state)?;
            return Ok(());
        }
        if self.profile.uses_demon() {
            registry.validate_state_demon_melee(state)?;
        } else if self.profile == Profile::Chaingunner {
            registry.validate_state_chaingunner(state)?;
        } else if self.profile == Profile::Shotgunner {
            registry.validate_state_shotgunner(state)?;
        } else if self.profile.uses_monsters() {
            registry.validate_state_monsters(state)?;
        } else if self.profile == Profile::Pursuit {
            registry.validate_state_pursuit(state)?;
        } else if self.profile.uses_perception() {
            registry.validate_state_perception(state)?;
        } else {
            registry.validate_state(state)?;
        }
        for ty in registry.spawn_dependencies(state)? {
            if self.profile.uses_perception() {
                self.validate_perception_type(ty)?;
            } else if self.profile == Profile::Projectiles {
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
        self.enter_monster_planned(id, state, None);
    }
    pub(crate) fn enter_monster_planned(
        &mut self,
        id: ActorId,
        state: StateId,
        shot: Option<&crate::monsters::MonsterShotPlan>,
    ) {
        self.enter_attack_planned(id, state, shot, None);
    }
    pub(crate) fn enter_attack_planned(
        &mut self,
        id: ActorId,
        state: StateId,
        shot: Option<&crate::monsters::MonsterShotPlan>,
        shotgun: Option<&crate::shotgunner::ShotgunPlan>,
    ) {
        let perception =
            (self.profile.uses_perception()).then_some(crate::perception::PerceptionInputs {
                pursuit: (self.profile == Profile::Pursuit || self.profile.uses_monsters()),
                monsters: self.profile.uses_monsters(),
                shotgunner: self.profile == Profile::Shotgunner,
                chaingunner: self.profile == Profile::Chaingunner,
                demon: self.profile.uses_demon(),
                map_demons: self.profile.uses_demon_map(),
                checked: None,
                melee: None,
                shotgun,
                shot,
                players: crate::player_roster::PerceptionRoster::Historical(
                    &self.perception_players,
                ),
                sound_targets: &self.sound_targets,
            });
        LifeHost {
            player: None,
            statistics: self.statistics.as_mut(),
            perception,
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
        if self.owns_player_body(id) {
            return Err(PlayerBodyError::Unsupported.into());
        }
        let life = self.active_typed(id)?;
        if self.player_policy().has_world_pickups()
            && world_pickups::kind_for(self.player_policy(), life.actor_type).is_some()
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.projectile_control(id)?;
        if self.profile == Profile::Shotgunner {
            return self.enter_shotgun_state(id, state);
        }
        if self.profile.uses_monsters() {
            return self.enter_monster_state(id, state);
        }
        if self.profile == Profile::Pursuit {
            self.preflight_pursuit_entry(id, state)?;
        } else if self.profile.uses_perception() {
            self.preflight_perception_entry(id, state)?;
        }
        self.validate_entry(state)?;
        let demand = self.registry_required()?.demand(state, life.tics)?;
        self.reserve_actor_spawns(demand.spawns)?;
        self.enter_validated(id, state);
        Ok(())
    }
    /// Controlled death-state entry for an unbound actor. This does not apply
    /// damage, alter health/height, set corpse flags, award kills or drop items.
    pub fn request_actor_death(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.profile.uses_demon_lifecycle() {
            return Err(MonsterError::Profile.into());
        }
        let life = self.active_typed(id)?;
        if self.player_policy().has_world_pickups()
            && world_pickups::kind_for(self.player_policy(), life.actor_type).is_some()
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
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
    /// Assign a validated store-local target. Role/lifetime rules depend on the
    /// profile: canonical Demons require a current, nonpending context body.
    /// References are retained on retirement; clearing never resolves the old target.
    pub fn set_actor_target(
        &mut self,
        id: ActorId,
        target: Option<ActorId>,
    ) -> Result<(), SimError> {
        if self.owns_player_body(id) {
            return Err(PlayerBodyError::Unsupported.into());
        }
        let life = self.active_typed(id)?;
        if self.player_policy().has_world_pickups()
            && world_pickups::kind_for(self.player_policy(), life.actor_type).is_some()
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        self.projectile_control(id)?;
        if self.profile == Profile::Pursuit || self.profile.uses_monsters() {
            if let Some(t) = target {
                self.validate_perception_reference(t)?;
            }
        }
        if target.is_some_and(|target| self.actor(target).is_none()) {
            return Err(StoreError::InvalidHandle.into());
        }
        self.actors
            .get_mut(id)
            .and_then(|actor| actor.life.as_mut())
            .ok_or(StoreError::InvalidHandle)?
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
    /// Atomic ordinary placement in source order. The general lifecycle profile
    /// reports starts without creating players and allows explicit repetition.
    /// Profile 16 instead requires a fresh batch and rejects starts/unsupported
    /// enabled types; see `with_demon_map_spawn_registry` for its admission.
    pub fn spawn_map_things(&mut self) -> Result<MapSpawnReport, SimError> {
        let registry = self.registry_required()?;
        let map_demons = self.profile.uses_demon_map();
        if map_demons && (self.tic != 0 || !self.actors.slots.is_empty()) {
            return Err(MapSpawnError::Setup.into());
        }
        let mut report = MapSpawnReport::default();
        let mut selected = Vec::new();
        for thing in self.world.map().things() {
            if map_demons && (thing.kind <= 4 || thing.kind == 11) {
                return Err(MapSpawnError::StartOrInvalidType(thing.kind).into());
            }
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
            let mut ty = registry
                .find_doomed(i32::from(thing.kind))
                .ok_or(ActorError::InvalidType)?;
            let info = registry.actor_type(ty).unwrap();
            if self.actor_options.no_monsters
                && (info.flags() & 4194304 != 0 || lifecycle::source_type_is(ty, "MT_SKULL"))
            {
                report.filtered += 1;
                continue;
            }
            if map_demons {
                // Lookup and nomonsters use original metadata, including COUNTKILL,
                // before the explicit executable fixture adaptation.
                let pickup = self.player_policy().has_world_pickups()
                    && world_pickups::kind_for(self.player_policy(), ty).is_some();
                let former = crate::original_registry::is_extended_monster(ty)
                    && crate::original_registry::is_policy_monster(self.player_policy(), ty);
                if !pickup
                    && !former
                    && ty
                        != crate::identity::OriginalActorType::MT_SERGEANT
                            .resolve(registry)
                            .map_err(|_| MonsterError::Definition)?
                {
                    return Err(MapSpawnError::UnsupportedType(thing.kind).into());
                }
                if self.profile.uses_checked_demons() && !(-4..=3).contains(&(thing.angle / 45)) {
                    return Err(MapSpawnError::Angle(thing.angle).into());
                }
                if (!pickup
                    && selected
                        .iter()
                        .filter(|(_, ty)| self.is_roster_monster(*ty))
                        .count()
                        == self.map_demon_limit())
                    || (pickup
                        && selected
                            .iter()
                            .filter(|(_, ty)| {
                                world_pickups::kind_for(self.player_policy(), *ty).is_some()
                            })
                            .count()
                            == MAX_WORLD_PICKUPS)
                {
                    return Err(MapSpawnError::WorkLimit.into());
                }
                if !self.spawn_admitted(
                    [
                        Fixed::from_map_units(thing.position.x),
                        Fixed::from_map_units(thing.position.y),
                    ],
                    info.radius(),
                    978,
                ) {
                    return Err(MonsterError::Arithmetic.into());
                }
                if !pickup && !former {
                    ty = self.demon_owner_type();
                }
            }
            if self.profile.uses_perception() {
                if !self.spawn_admitted(
                    [
                        Fixed::from_map_units(thing.position.x),
                        Fixed::from_map_units(thing.position.y),
                    ],
                    info.radius(),
                    1024,
                ) {
                    return Err(PerceptionError::Arithmetic.into());
                }
                self.validate_perception_type(ty)?;
            } else if self.profile == Profile::Projectiles {
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
        if let Some(stats) = self.statistics {
            let count = i32::try_from(
                selected
                    .iter()
                    .filter(|(_, ty)| {
                        registry
                            .actor_type(*ty)
                            .is_some_and(|t| t.flags() & world_pickups::MF_COUNTKILL != 0)
                    })
                    .count(),
            )
            .map_err(|_| MapSpawnError::StatisticsOverflow)?;
            stats
                .total_kills
                .checked_add(count)
                .ok_or(MapSpawnError::StatisticsOverflow)?;
        }
        if self.player_policy().has_player_bonuses() {
            let total = self
                .statistics
                .ok_or(PlayerBodyError::Invariant)?
                .total_items;
            let count = selected
                .iter()
                .filter(|(_, ty)| {
                    registry
                        .actor_type(*ty)
                        .is_some_and(|t| t.flags() & world_pickups::MF_COUNTITEM != 0)
                })
                .count();
            let count = i32::try_from(count).map_err(|_| MapSpawnError::StatisticsOverflow)?;
            if !(0..=MAX_WORLD_PICKUPS as i32).contains(&total)
                || total
                    .checked_add(count)
                    .is_none_or(|n| n > MAX_WORLD_PICKUPS as i32)
            {
                return Err(MapSpawnError::StatisticsOverflow.into());
            }
            report.spawned = self.checked_map_spawn_batch(selected.len(), |host| {
                let mut ids = line_work::reserved(selected.len())?;
                for (thing, ty) in selected {
                    ids.push(host.spawn_placed_thing(thing, ty, true));
                    #[cfg(test)]
                    BONUS_MAP_FAIL_AFTER.with(|counter| {
                        if let Some(left) = counter.get() {
                            if left <= 1 {
                                counter.set(None);
                                let last = *ids.last().unwrap();
                                BONUS_MAP_FAILURE_OBSERVATION.with(|o| {
                                    o.set(Some((
                                        host.rng.indices()[0],
                                        host.statistics.as_ref().unwrap().total_items,
                                        host.access.order().count(),
                                        host.access.get(last).unwrap().life().unwrap().tics,
                                    )))
                                });
                                return Err(MapSpawnError::WorkLimit.into());
                            }
                            counter.set(Some(left - 1));
                        }
                        Ok::<(), SimError>(())
                    })?;
                }
                Ok(ids)
            })?;
        } else {
            self.reserve_actor_spawns(selected.len())?;
            // Legacy batches still have no recoverable work after reservation.
            let mut host = LifeHost {
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
            };
            for (thing, ty) in selected {
                report
                    .spawned
                    .push(host.spawn_placed_thing(thing, ty, false));
            }
        }
        Ok(report)
    }
}

impl<A: lifecycle::ActorAccess> LifeHost<'_, A> {
    // All definitions and complete counter additions are proved before calling.
    // Both legacy and staged batches use this one original placement service.
    fn spawn_placed_thing(
        &mut self,
        thing: doom_map::Thing,
        ty: ActorTypeId,
        count_items: bool,
    ) -> ActorId {
        let info = self.registry.actor_type(ty).unwrap();
        let height = if info.flags() & 256 != 0 {
            SpawnHeight::Ceiling
        } else {
            SpawnHeight::Floor
        };
        let id = self.spawn(
            ty,
            [
                Fixed::from_map_units(thing.position.x),
                Fixed::from_map_units(thing.position.y),
            ],
            height,
        );
        let actor = self.access.get_mut(id).unwrap();
        let life = actor.life.as_mut().unwrap();
        life.spawnpoint = Some(thing);
        if life.tics > 0 {
            life.tics = 1 + i32::from(self.rng.p_random()) % life.tics;
        }
        if let Some(stats) = self.statistics.as_deref_mut() {
            if life.flags & world_pickups::MF_COUNTKILL != 0 {
                stats.total_kills += 1;
                self.events
                    .push(Event::Statistics(StatisticsEvent::MapKillTotal {
                        actor: id,
                        total: stats.total_kills,
                    }));
            }
            if count_items && life.flags & world_pickups::MF_COUNTITEM != 0 {
                stats.total_items += 1;
                self.events
                    .push(Event::Statistics(StatisticsEvent::MapItemTotal {
                        actor: id,
                        total: stats.total_items,
                    }));
            }
        }
        actor.spec.angle = ((i32::from(thing.angle) / 45) as u32).wrapping_mul(0x20000000);
        if thing.options & 8 != 0 {
            life.flags |= 32;
        }
        id
    }
}
#[cfg(test)]
std::thread_local! {
    pub(crate) static BONUS_MAP_FAILURE_OBSERVATION: std::cell::Cell<Option<(u8,i32,usize,i32)>> = const { std::cell::Cell::new(None) };
    pub(crate) static BONUS_MAP_FAIL_AFTER: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}
