//! Explicit intersection of expanded static geometry and the native Demon closure.
use crate::*;

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
            Profile::DemonExpandedMap,
        )
    }
    pub(crate) fn with_demon_expanded_profile(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
        context: MapContextOptions,
        profile: Profile,
    ) -> Result<Self, SimError> {
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
        let work = map_extent::validate(&map, limits)?;
        let mut sim = Self::create_bounded(map, limit, profile, limits.max_prepared_references)?;
        sim.world.extent_sight_work = Some(work);
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
    events: Vec<Event>,
}
impl Simulation {
    fn demon_plan(&self) -> Result<DemonPlan, SimError> {
        let mut slots = line_work::reserved(self.actors.slots.len())?;
        slots.extend_from_slice(&self.actors.slots);
        let mut order = line_work::reserved(self.actors.max_slots as usize)?;
        order.extend_from_slice(&self.spatial.order);
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
            events: Vec::new(),
        })
    }
    fn checked_demon_inputs<'a>(
        &'a self,
        control: &'a SightControl,
    ) -> perception::PerceptionInputs<'a> {
        perception::PerceptionInputs {
            checked: Some(control),
            players: &self.perception_players,
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
            sight::actor_bounds(actor)?;
            let life = actor.life.ok_or(MonsterError::Definition)?;
            if life.tics < -1 {
                return Err(ActorError::InvalidState.into());
            }
            if !linked.contains(&id)
                || actor.location != Some(self.world.locate_validated(actor.position()))
                || actor.floor > actor.ceiling
                || actor.height() <= Fixed::ZERO
            {
                return Err(MapSpawnError::Ownership.into());
            }
            map_scale::bounded(actor.momentum().into_iter().chain([actor.momz]), 64)?;
        }
        self.preflight_player_body(stepping)?;
        self.preflight_monster_owner()?;
        Ok(())
    }
    fn commit_demon_plan(&mut self, plan: DemonPlan) -> Vec<Event> {
        self.actors = plan.actors;
        self.spatial = plan.spatial;
        self.rng = plan.rng;
        self.statistics = plan.statistics;
        self.original_player = plan.player;
        self.bindings = plan.bindings;
        plan.events
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
            player: plan.player.as_deref_mut(),
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
        let command_policy =
            self.player_policy() == OriginalExecutionProfile::PlayerCommandMovementV1;
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
            if let Some(id) = command_body {
                player_commands::stage(
                    &mut LifeHost {
                        player: plan.player.as_deref_mut(),
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
                    },
                    id,
                    commands[usize::from(PlayerSlot::One.ordinal())],
                )?;
            }
            plan.events.push(Event::Phase(Phase::Thinkers));
        } else {
            plan.events
                .extend([Event::Phase(Phase::Players), Event::Phase(Phase::Thinkers)]);
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
                player: plan.player.as_deref_mut(),
                rng: &mut plan.rng,
                spatial: &mut plan.spatial,
                events: &mut plan.events,
                commands,
                perception_inputs: Some(self.checked_demon_inputs(&control)),
            },
        );
        if let Some(error) = control.failure.get() {
            return Err(error);
        }
        if command_policy {
            player_commands::validate_result(&plan.actors, plan.player.as_deref())?;
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
