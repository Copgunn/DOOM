//! Explicit intersection of expanded static geometry and the native Demon closure.
use crate::*;

/// Which expanded-map service family a construction selects and the two
/// policy-decided admissions it needs before any type validation runs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ExpandedSelection {
    pub profile: Profile,
    /// Actor slots the proof needs (64 historically, 320 at map scale).
    pub actor_cap: u32,
    /// Admit type dimensions against the tallest sector (heights policy).
    pub type_envelope: bool,
}
impl Simulation {
    /// Profile 20: larger static maps with the exact map-context Demon registry.
    /// Geometry uses the extent proof; actors retain the narrower gameplay bounds.
    /// Selected THING angles require a source-defined signed BAM multiplication.
    pub fn with_demon_expanded_map_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
        context: MapContextOptions,
    ) -> Result<Self, SimError> {
        Self::with_demon_expanded_profile(
            map,
            limit,
            registry,
            options,
            context,
            ExpandedSelection {
                profile: Profile::DemonExpandedMap,
                actor_cap: 64,
                type_envelope: false,
            },
        )
    }
    pub(crate) fn with_demon_expanded_profile(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
        context: MapContextOptions,
        selection: ExpandedSelection,
    ) -> Result<Self, SimError> {
        let ExpandedSelection {
            profile,
            actor_cap,
            type_envelope,
        } = selection;
        if context.respawn_monsters {
            return Err(MapSpawnError::Respawn.into());
        }
        if options.mode != ActorGameMode::SinglePlayer {
            return Err(ActorError::UnsupportedMode.into());
        }
        if options.skill == Skill::Nightmare {
            return Err(MonsterError::Definition.into());
        }
        if map.things().len() > MAX_DEMON_MAP_THINGS {
            return Err(MapSpawnError::WorkLimit.into());
        }
        if map.linedefs().iter().any(|l| l.special != 0)
            || map.sectors().iter().any(|s| s.special != 0)
        {
            return Err(PursuitError::Specials.into());
        }
        if map
            .sectors()
            .iter()
            .any(|s| s.heights.iter().any(|h| h.unsigned_abs() > 512))
        {
            return Err(PerceptionError::Arithmetic.into());
        }
        if map
            .sectors()
            .iter()
            .any(|s| s.flats[1].as_bytes().eq_ignore_ascii_case(b"F_SKY1\0\0"))
        {
            return Err(CombatError::Sky.into());
        }
        if profile == Profile::OriginalDemon {
            OriginalExecutionRegistry::validate(&registry, OriginalExecutionProfile::DemonNormalV1)
                .map_err(|_| MonsterError::Definition)?;
        } else {
            let expected = ActorRegistry::demon_map_context_fixture();
            if registry.states() != expected.states() || registry.types() != expected.types() {
                return Err(MonsterError::Definition.into());
            }
            DemonExecutionBinding::admit(&registry).map_err(|_| MonsterError::Definition)?;
        }
        registry.validate_impact_source()?;
        let limits = MapExtentLimits {
            max_actors: limit,
            ..Default::default()
        };
        let work = map_extent::validate_with_actor_cap(&map, limits, actor_cap)?;
        let mut sim = Self::create_bounded(map, limit, profile, limits.max_prepared_references)?;
        sim.world.extent_sight_work = Some(work);
        if type_envelope {
            // Heights policy: windows and closed sectors may be shorter than any
            // actor; dimensions are admitted against the tallest sector and every
            // spawn proves its own fit. Earlier policies keep the map-wide fit.
            sim.type_envelope = sim
                .sectors
                .iter()
                .copied()
                .max_by_key(|s| s.ceiling - s.floor);
        }
        if profile != Profile::OriginalDemon {
            snapshot::registry_fields(Arc::make_mut(&mut sim.map_fields), &registry);
        }
        sim.registry = Some(Arc::new(registry));
        sim.actor_options = options;
        sim.rng = context.initial_rng;
        sim.statistics = Some(LevelStatistics::default());
        sim.sound_targets = vec![None; sim.sectors.len()];
        for name in ["MT_PUFF", "MT_BLOOD"] {
            sim.validate_combat_type(lifecycle::source_type_id(name).unwrap())?;
        }
        Ok(sim)
    }

    /// Derived prepared storage metrics; excluded from canonical state.
    pub fn demon_expanded_map_metrics(&self) -> Result<MapScaleMetrics, SimError> {
        if self.profile != Profile::DemonExpandedMap {
            return Err(MonsterError::Profile.into());
        }
        Ok(self.prepared_geometry_metrics())
    }
}

use crate::lifecycle::LifeHost;
use std::cell::Cell;

/// Transient per-operation work/error control. Never part of canonical state.
pub(crate) struct SightControl {
    pub failure: Cell<Option<SimError>>,
    queries: Cell<usize>,
    work: Cell<usize>,
}
impl Default for SightControl {
    fn default() -> Self {
        Self {
            failure: Cell::new(None),
            queries: Cell::new(0),
            work: Cell::new(0),
        }
    }
}
pub(crate) fn action_sight(
    control: Option<&SightControl>,
    world: &StaticWorld,
    sectors: &[SectorState],
    observer: Actor,
    target: Actor,
) -> Result<SightResult, SimError> {
    if let Some(control) = control {
        if let Some(error) = control.failure.get() {
            return Err(error);
        }
        let count = control.queries.get() + 1;
        let work = control
            .work
            .get()
            .checked_add(world.extent_sight_work.expect("checked geometry") + 5 * world.lines.len())
            .ok_or(PerceptionError::WorkLimit)?;
        if count > 64 || work > 1_048_576 {
            return Err(PerceptionError::WorkLimit.into());
        }
        control.queries.set(count);
        control.work.set(work);
    }
    sight::checked_query(world, sectors, observer, target)
}

/// Only the bounded mutable actor projection is staged. World, map, registry,
/// sectors, sound targets, roster, canonical caches and old events are borrowed.
struct DemonPlan {
    actors: ThinkerStore<Actor>,
    spatial: movement::SpatialIndex,
    rng: DoomRng,
    statistics: Option<LevelStatistics>,
    player: Option<original_player::PlayerRuntime>,
    bindings: [Option<ActorId>; 4],
    /// Per-sector sound targets staged by the weapon noise (one sector in the
    /// flat world, every sector under heights).
    sound_targets: Vec<Option<ActorId>>,
    events: Vec<Event>,
}
impl Simulation {
    fn demon_plan(&self) -> Result<DemonPlan, SimError> {
        let mut slots = line_work::reserved(self.actors.slots.len())?;
        slots.extend_from_slice(&self.actors.slots);
        let mut order = line_work::reserved(self.actors.max_slots as usize)?;
        order.extend_from_slice(&self.spatial.order);
        let mut sound_targets = line_work::reserved(self.sound_targets.len())?;
        sound_targets.extend_from_slice(&self.sound_targets);
        Ok(DemonPlan {
            actors: ThinkerStore {
                slots,
                head: self.actors.head,
                tail: self.actors.tail,
                max_slots: self.actors.max_slots,
            },
            spatial: movement::SpatialIndex { order },
            rng: self.rng,
            statistics: self.statistics,
            player: self.original_player,
            bindings: self.bindings,
            sound_targets,
            events: Vec::new(),
        })
    }
    fn checked_demon_inputs<'a>(
        &'a self,
        control: &'a SightControl,
    ) -> perception::PerceptionInputs<'a> {
        perception::PerceptionInputs {
            checked: Some(control),
            players: if self.uses_active_player_roster() {
                player_roster::PerceptionRoster::OriginalPlayer(self.player_policy())
            } else {
                player_roster::PerceptionRoster::Historical(&self.perception_players)
            },
            sound_targets: &self.sound_targets,
            pursuit: true,
            monsters: true,
            shotgunner: false,
            chaingunner: false,
            demon: true,
            map_demons: true,
            melee: None,
            shotgun: None,
            shot: None,
        }
    }
    fn preflight_expanded_demons(&self) -> Result<(), SimError> {
        self.preflight_expanded_demons_for(false)
    }
    fn preflight_expanded_demons_for(&self, stepping: bool) -> Result<(), SimError> {
        if self.statistics.is_none() || self.sound_targets.len() != self.sectors.len() {
            return Err(MapSpawnError::Ownership.into());
        }
        let mut linked = std::collections::BTreeSet::new();
        for &id in &self.spatial.order {
            if self.actors.is_pending(id) != Some(false) || !linked.insert(id) {
                return Err(MapSpawnError::Ownership.into());
            }
        }
        for id in self.actors.order() {
            if self.actors.is_pending(id) != Some(false) {
                continue;
            }
            let actor = self.actor(id).unwrap();
            sight::actor_bounds_within(actor, self.sight_margin())?;
            let life = actor.life.ok_or(MonsterError::Definition)?;
            if life.tics < -1 {
                return Err(ActorError::InvalidState.into());
            }
            // P_CheckMissileSpawn keeps a blocked half-step without relinking: an
            // exploding fireball's coordinates may lead its source location.
            let unlinked_blast = original_registry::is_fireball(life.actor_type)
                && life.flags & original_player::MF_MISSILE == 0;
            if !linked.contains(&id)
                || (!unlinked_blast
                    && actor.location != Some(self.world.locate_validated(actor.position())))
                || actor.floor > actor.ceiling
                || actor.height() <= Fixed::ZERO
            {
                return Err(MapSpawnError::Ownership.into());
            }
            if self.room().is_some() {
                player_map_scale::momentum_admitted(actor.momentum())?;
            } else {
                map_scale::bounded(actor.momentum(), 64)?;
            }
            let policy = self.player_policy();
            if policy.has_heights() && !original_registry::is_impact(life.actor_type) {
                // Invariant V: impacts spawn at the source's absolute z and are
                // clipped by P_ZMovement, so only their momz witness applies.
                player_map_scale::vertical_admitted(actor)?;
            } else {
                map_scale::bounded([actor.momz], player_map_scale::momz_bound(policy))?;
            }
        }
        self.preflight_player_body(stepping)?;
        self.preflight_monster_owner()?;
        self.preflight_live_missiles()?;
        Ok(())
    }
    fn commit_demon_plan(&mut self, plan: DemonPlan) -> Vec<Event> {
        self.actors = plan.actors;
        self.spatial = plan.spatial;
        self.rng = plan.rng;
        self.statistics = plan.statistics;
        self.original_player = plan.player;
        if self.player_policy().has_player_weapons() {
            self.sound_targets = plan.sound_targets;
        }
        self.bindings = plan.bindings;
        plan.events
    }
    /// New-policy THING setup stages the same bounded actor/RNG/statistics plan.
    /// It runs before player binding, so it does not require a live-player preflight.
    pub(crate) fn checked_map_spawn_batch(
        &mut self,
        count: usize,
        action: impl FnOnce(
            &mut LifeHost<'_, &mut ThinkerStore<Actor>>,
        ) -> Result<Vec<ActorId>, SimError>,
    ) -> Result<Vec<ActorId>, SimError> {
        if !self.player_policy().has_player_bonuses() || self.statistics.is_none() {
            return Err(MapSpawnError::Context.into());
        }
        let mut plan = self.demon_plan()?;
        plan.actors.reserve_spawns(count)?;
        plan.spatial
            .order
            .try_reserve(count)
            .map_err(|_| StoreError::Capacity)?;
        let result = action(&mut LifeHost {
            player: None,
            statistics: plan.statistics.as_mut(),
            perception: None,
            access: &mut plan.actors,
            registry: self.registry.as_deref().ok_or(ActorError::InvalidType)?,
            world: &self.world,
            sectors: &self.sectors,
            options: self.actor_options,
            rng: &mut plan.rng,
            spatial: &mut plan.spatial,
            bindings: &mut plan.bindings,
            events: &mut plan.events,
        })?;
        let events = self.commit_demon_plan(plan);
        self.pending.extend(events);
        Ok(result)
    }
    /// One bounded shared action execution; successful plans are committed once.
    pub(crate) fn checked_demon_operation<T>(
        &mut self,
        action: impl FnOnce(&mut LifeHost<'_, &mut ThinkerStore<Actor>>) -> T,
    ) -> Result<T, SimError> {
        self.preflight_expanded_demons()?;
        let mut plan = self.demon_plan()?;
        let control = SightControl::default();
        let result = action(&mut LifeHost {
            player: plan.player.as_mut(),
            statistics: plan.statistics.as_mut(),
            perception: Some(self.checked_demon_inputs(&control)),
            access: &mut plan.actors,
            registry: self.registry.as_deref().unwrap(),
            world: &self.world,
            sectors: &self.sectors,
            options: self.actor_options,
            rng: &mut plan.rng,
            spatial: &mut plan.spatial,
            bindings: &mut plan.bindings,
            events: &mut plan.events,
        });
        if let Some(error) = control.failure.get() {
            return Err(error);
        }
        let events = self.commit_demon_plan(plan);
        self.pending.extend(events);
        Ok(result)
    }
    pub(crate) fn advance_expanded_demons(
        &mut self,
        commands: [TicCommand; 4],
    ) -> Result<(), SimError> {
        let command_policy = self.player_policy().has_player_commands();
        let weapon_policy = self.player_policy().has_player_weapons();
        if weapon_policy
            && self
                .original_player
                .as_ref()
                .is_none_or(|p| p.weapon.is_none())
        {
            return Err(WeaponError::Player.into());
        }
        let command_body = if command_policy {
            self.preflight_player_commands(&commands)?
        } else {
            None
        };
        self.preflight_expanded_demons_for(true)?;
        self.preflight_actor_step()?;
        let mut plan = self.demon_plan()?;
        let control = SightControl::default();
        if command_policy {
            plan.events.push(Event::Phase(Phase::Players));
            {
                let mut host = LifeHost {
                    player: plan.player.as_mut(),
                    statistics: plan.statistics.as_mut(),
                    perception: Some(self.checked_demon_inputs(&control)),
                    access: &mut plan.actors,
                    registry: self.registry.as_deref().ok_or(ActorError::InvalidType)?,
                    world: &self.world,
                    sectors: &self.sectors,
                    options: self.actor_options,
                    rng: &mut plan.rng,
                    spatial: &mut plan.spatial,
                    bindings: &mut plan.bindings,
                    events: &mut plan.events,
                };
                if let Some(id) = command_body {
                    player_commands::stage(
                        &mut host,
                        id,
                        commands[usize::from(PlayerSlot::One.ordinal())],
                    )?;
                }
                if weapon_policy {
                    player_pistol::stage(
                        &mut host,
                        commands[usize::from(PlayerSlot::One.ordinal())],
                        self.tic,
                        &mut plan.sound_targets,
                    )?;
                }
            }
            world_pickups::bonus_tick(plan.player.as_mut())?;
            plan.events.push(Event::Phase(Phase::Thinkers));
        } else {
            plan.events
                .extend([Event::Phase(Phase::Players), Event::Phase(Phase::Thinkers)]);
        }
        let mut perception_inputs = self.checked_demon_inputs(&control);
        if weapon_policy {
            perception_inputs.sound_targets = &plan.sound_targets;
        }
        thinkers::run(
            &mut plan.actors,
            thinkers::Inputs {
                world: &self.world,
                sectors: &self.sectors,
                profile: self.profile,
                bindings: plan.bindings,
                live_bindings: &mut plan.bindings,
                registry: &self.registry,
                options: self.actor_options,
                statistics: &mut plan.statistics,
                player: plan.player.as_mut(),
                rng: &mut plan.rng,
                spatial: &mut plan.spatial,
                events: &mut plan.events,
                commands,
                perception_inputs: Some(perception_inputs),
            },
        );
        if let Some(error) = control.failure.get() {
            return Err(error);
        }
        if command_policy {
            player_commands::validate_result(&plan.actors, plan.player.as_ref())?;
            if weapon_policy {
                for id in plan.actors.order() {
                    if let Some(actor) = plan.actors.get(id) {
                        if actor.life().is_some_and(|l| {
                            original_registry::is_policy_monster(self.player_policy(), l.actor_type)
                        }) {
                            player_map_scale::travel_admitted(
                                self.room(),
                                actor.position(),
                                actor.momentum(),
                            )?;
                        }
                    }
                }
            }
        }
        let events = self.commit_demon_plan(plan);
        self.events = std::mem::take(&mut self.pending);
        self.events.extend(events);
        self.events
            .extend([Event::Phase(Phase::Specials), Event::Phase(Phase::Respawn)]);
        self.commands = commands;
        self.tic += 1;
        Ok(())
    }
}

#[cfg(test)]
#[path = "demon_expanded_map_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "transaction_cost_tests.rs"]
mod transaction_cost_tests;
