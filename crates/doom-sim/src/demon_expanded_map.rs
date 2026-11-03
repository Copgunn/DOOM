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
    /// Format-sized static geometry with derived work values (real-map policy).
    pub real_map: bool,
    /// Sky ceilings and the static sector specials (specials policy).
    pub specials: bool,
    /// Doors, lifts, floors, switches and the exit (movers policy, a superset of
    /// the specials policy: `specials` is set too).
    pub movers: bool,
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
                real_map: false,
                specials: false,
                movers: false,
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
            real_map,
            specials,
            movers,
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
        // The real-map policy takes the THING count from the lump (the parser's
        // format bound); spawned actors stay bounded by the roster.
        if !real_map && map.things().len() > MAX_DEMON_MAP_THINGS {
            return Err(MapSpawnError::WorkLimit.into());
        }
        let movers = if movers {
            Some(mover_rules::validate_movers(&map)?)
        } else {
            if specials {
                validate_specials(&map)?;
            }
            None
        };
        if !specials
            && (map.linedefs().iter().any(|l| l.special != 0)
                || map.sectors().iter().any(|s| s.special != 0))
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
        if !specials
            && map
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
        let mut sim = if real_map {
            let admission = map_extent::validate_real_map(&map, actor_cap)?;
            let mut sim = Self::create_bounded_with(
                map,
                limit,
                profile,
                admission.references,
                GeometryCapacity {
                    specials,
                    ..GeometryCapacity::FORMAT
                },
            )?;
            sim.world.extent_sight_work = Some(admission.sight_work);
            sim.world.sight_work_budget = admission.sight_budget;
            if specials {
                sim.install_specials(limit, movers)?;
            }
            sim
        } else {
            let limits = MapExtentLimits {
                max_actors: limit,
                ..Default::default()
            };
            let work = map_extent::validate_with_actor_cap(&map, limits, actor_cap)?;
            let mut sim =
                Self::create_bounded(map, limit, profile, limits.max_prepared_references)?;
            sim.world.extent_sight_work = Some(work);
            sim
        };
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

    /// The specials policy's load-time state: a sector-thinker slot class bounded by
    /// the number of light sectors `L` (the only sector thinkers this policy
    /// creates, all at load), so the visit budget is the 320 actor slots plus that
    /// count; the per-sector sky identity and `blockbox`; and the runtime lights and
    /// specials, still the map's until `spawn_level_specials` runs.
    ///
    /// Under the movers policy the slot class holds `L + 2M` (`M` mover sectors: a
    /// removed mover keeps its slot until the next pass while a replacement for its
    /// sector can exist, decision N8), so the visit budget is `320 + L + 2M`; the
    /// runtime line specials start as the map's, and the admission is kept.
    fn install_specials(
        &mut self,
        limit: u32,
        movers: Option<mover_rules::MoverAdmission>,
    ) -> Result<(), SimError> {
        let map = self.world.map();
        let lights = map
            .sectors()
            .iter()
            .filter(|s| matches!(s.special, 1 | 2 | 3 | 4 | 8 | 12 | 13 | 17))
            .count();
        let movers_slots = movers.as_ref().map_or(Some(0), |m| m.movers.checked_mul(2));
        let lights = movers_slots
            .and_then(|m| m.checked_add(lights))
            .and_then(|slots| u32::try_from(slots).ok())
            .ok_or(StoreError::InvalidLimit)?;
        let sky = map
            .sectors()
            .iter()
            .map(|s| is_sky_flat(&s.flats[1]))
            .collect();
        let sectors = map
            .sectors()
            .iter()
            .map(|s| sector_thinkers::SectorSpecial {
                light: s.light,
                special: s.special,
            })
            .collect();
        let blockboxes = multisector::sector_block_boxes(map)?;
        let line_specials = movers
            .is_some()
            .then(|| map.linedefs().iter().map(|l| l.special).collect());
        self.actors = ThinkerStore::with_sector_capacity(limit, lights)?;
        self.world.sky_ceilings = Some(sky);
        self.world.sector_block_boxes = Some(blockboxes);
        self.specials = Some(sector_thinkers::LevelSpecials {
            sectors,
            spawned: false,
        });
        if let Some(lines) = line_specials {
            self.line_specials = lines;
        }
        self.movers = movers;
        Ok(())
    }
    /// Derived prepared storage metrics; excluded from canonical state.
    pub fn demon_expanded_map_metrics(&self) -> Result<MapScaleMetrics, SimError> {
        if self.profile != Profile::DemonExpandedMap {
            return Err(MonsterError::Profile.into());
        }
        Ok(self.prepared_geometry_metrics())
    }
}

/// `ceilingpic == skyflatnum`: `skyflatnum = R_FlatNumForName("F_SKY1")` and a
/// sector's `ceilingpic = R_FlatNumForName(name)` are the same flat exactly when
/// `W_CheckNumForName` resolves both names to one lump, that is when the eight
/// name bytes up to the first NUL equal `F_SKY1` ignoring ASCII case
/// (`strncpy` then `strupr`, w_wad.c). The declared flat table of the C evidence
/// (`skyflatnum` 2, the index of `F_SKY1`) is one instance of this rule.
pub(crate) fn is_sky_flat(name: &doom_types::LumpName) -> bool {
    let bytes = name.as_bytes();
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    bytes[..end].eq_ignore_ascii_case(b"F_SKY1")
}
/// The specials policy's load rules (M3h7 note §4.2): every sector special is one
/// of `ADMITTED_SECTOR_SPECIALS` or 0 (10, 11, 14 and unknown values stay
/// rejected); every line special is 0 or 48, a 48 is one-sided (a two-sided
/// special line would enter `spechit`, Stage B) and there are at most 64 of them
/// (`MAXLINEANIMS`, filled without a bound check by the source): all
/// `PursuitError::Specials`. Every sector light lies in `LIGHT_RANGE`
/// (`MapScaleError::Light`). Tags are kept: no admitted special reads them.
pub(crate) fn validate_specials(map: &Map) -> Result<(), SimError> {
    use sector_thinkers::{MAX_LINE_ANIMS, SCROLLING_WALL};
    validate_sector_specials(map)?;
    let mut scrollers = 0;
    for line in map.linedefs() {
        match line.special {
            0 => {}
            SCROLLING_WALL if line.sides[1].is_none() => scrollers += 1,
            _ => return Err(PursuitError::Specials.into()),
        }
    }
    if scrollers > MAX_LINE_ANIMS {
        return Err(PursuitError::Specials.into());
    }
    validate_lights(map)
}
/// Every sector special is one of `ADMITTED_SECTOR_SPECIALS` or 0
/// (`PursuitError::Specials`): the sector rule of the specials and movers policies.
pub(crate) fn validate_sector_specials(map: &Map) -> Result<(), SimError> {
    use sector_thinkers::ADMITTED_SECTOR_SPECIALS;
    if map
        .sectors()
        .iter()
        .any(|s| s.special != 0 && !ADMITTED_SECTOR_SPECIALS.contains(&s.special))
    {
        return Err(PursuitError::Specials.into());
    }
    Ok(())
}
/// Every sector light lies in `LIGHT_RANGE` (`MapScaleError::Light`).
pub(crate) fn validate_lights(map: &Map) -> Result<(), SimError> {
    if map
        .sectors()
        .iter()
        .any(|s| !sector_thinkers::LIGHT_RANGE.contains(&s.light))
    {
        return Err(MapScaleError::Light.into());
    }
    Ok(())
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
    sectors: crate::staging::Sectors<'_>,
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
        if count > 64 || work > world.sight_work_budget {
            return Err(PerceptionError::WorkLimit.into());
        }
        control.queries.set(count);
        control.work.set(work);
    }
    sight::checked_query(world, sectors, observer, target)
}

/// Only the bounded mutable actor projection is staged. World, map, registry,
/// sectors, sound targets, roster, canonical caches and old events are borrowed.
/// Staged per-sector sound targets: the flat policies stage their one sector without
/// allocating (the tic plan keeps exactly its two historical allocations); the
/// heights policy copies every sector's target.
enum StagedSound {
    One(Option<ActorId>),
    Many(Vec<Option<ActorId>>),
}
impl StagedSound {
    fn as_slice(&self) -> &[Option<ActorId>] {
        match self {
            Self::One(target) => std::slice::from_ref(target),
            Self::Many(targets) => targets,
        }
    }
    fn as_mut_slice(&mut self) -> &mut [Option<ActorId>] {
        match self {
            Self::One(target) => std::slice::from_mut(target),
            Self::Many(targets) => targets,
        }
    }
}
struct DemonPlan {
    actors: ThinkerStore<Actor>,
    spatial: movement::SpatialIndex,
    rng: DoomRng,
    statistics: Option<LevelStatistics>,
    player: Option<original_player::PlayerRuntime>,
    bindings: [Option<ActorId>; 4],
    /// Sound targets staged by the weapon noise.
    sound_targets: StagedSound,
    /// Under the specials policy, the runtime sector lights and specials written
    /// by the light thinkers and secrets of this tic (empty otherwise).
    specials: Vec<sector_thinkers::SectorSpecial>,
    /// The sector heights and line specials written in this tic, each table in a
    /// lazy full copy taken on its first write (M3h8 note §9), read through by every
    /// later query of the plan; nothing staged under every policy without movers.
    writes: staging::StagedWrites,
    events: Vec<Event>,
}
impl Simulation {
    fn demon_plan(&mut self) -> Result<DemonPlan, SimError> {
        let mut slots = line_work::reserved(self.actors.slots.len())?;
        slots.extend_from_slice(&self.actors.slots);
        // The shared order lives in the slot links plus head/tail, so staging both
        // classes stages it. The sector class is staged in the spare buffer of the
        // last committed plan, as for the sound targets: no allocation once its
        // capacity matches the sector-thinker slot count. Without sector thinkers it
        // is empty and never allocates: the tic plan keeps its two historical
        // allocations.
        let mut sector_slots = std::mem::take(&mut self.spare_sector_slots);
        sector_slots.clear();
        sector_slots
            .try_reserve(self.actors.sector_slots.len())
            .map_err(|_| StoreError::Capacity)?;
        sector_slots.extend_from_slice(&self.actors.sector_slots);
        let mut order = line_work::reserved(self.actors.max_slots as usize)?;
        order.extend_from_slice(&self.spatial.order);
        let sound_targets = if self.player_policy().has_heights() {
            // The spare buffer swapped out by the last committed plan: no
            // allocation once its capacity matches the sector count. It is never
            // read, so a failed tic drops it and the next plan allocates once.
            let mut targets = std::mem::take(&mut self.spare_sound_targets);
            targets.clear();
            targets
                .try_reserve(self.sound_targets.len())
                .map_err(|_| StoreError::Capacity)?;
            targets.extend_from_slice(&self.sound_targets);
            StagedSound::Many(targets)
        } else {
            StagedSound::One(self.sound_targets.first().copied().flatten())
        };
        // The spare buffer of the last committed plan, as for the sound targets:
        // no allocation once its capacity matches the sector count; empty (and
        // never allocating) without the specials policy.
        let mut specials = std::mem::take(&mut self.spare_specials);
        specials.clear();
        if let Some(level) = &self.specials {
            specials
                .try_reserve(level.sectors.len())
                .map_err(|_| StoreError::Capacity)?;
            specials.extend_from_slice(&level.sectors);
        }
        // The staging buffers of the last committed plan: copied into only on a
        // plan's first write, so no allocation without writes and none once their
        // capacity matches the tables.
        let mut writes = std::mem::take(&mut self.spare_writes);
        writes.clear();
        Ok(DemonPlan {
            actors: ThinkerStore {
                slots,
                sector_slots,
                head: self.actors.head,
                tail: self.actors.tail,
                max_slots: self.actors.max_slots,
                max_sector_slots: self.actors.max_sector_slots,
            },
            spatial: movement::SpatialIndex { order },
            rng: self.rng,
            statistics: self.statistics,
            player: self.original_player,
            bindings: self.bindings,
            sound_targets,
            specials,
            writes,
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
            if policy.has_heights()
                && !original_registry::is_impact(life.actor_type)
                && !unlinked_blast
            {
                // Invariant V: impacts spawn at the source's absolute z, and a
                // fireball exploded by its own half-step is clipped by its next
                // P_ZMovement (in the same pass for an in-tic launch), so only
                // their z/momz witnesses apply.
                player_map_scale::vertical_admitted(actor)?;
            } else {
                map_scale::bounded([actor.momz], player_map_scale::momz_bound(policy))?;
                if policy.has_heights() {
                    map_scale::bounded([actor.z], player_map_scale::HEIGHTS_Z_UNITS)?;
                }
            }
        }
        self.preflight_player_body(stepping)?;
        self.preflight_monster_owner()?;
        self.preflight_live_missiles()?;
        Ok(())
    }
    fn commit_demon_plan(&mut self, plan: DemonPlan) -> Vec<Event> {
        let committed = std::mem::replace(&mut self.actors, plan.actors);
        self.spare_sector_slots = committed.sector_slots;
        self.spatial = plan.spatial;
        self.rng = plan.rng;
        self.statistics = plan.statistics;
        self.original_player = plan.player;
        if self.player_policy().has_player_weapons() {
            match plan.sound_targets {
                StagedSound::One(target) => {
                    if let Some(first) = self.sound_targets.first_mut() {
                        *first = target;
                    }
                }
                StagedSound::Many(targets) => {
                    self.spare_sound_targets = std::mem::replace(&mut self.sound_targets, targets);
                }
            }
        }
        self.bindings = plan.bindings;
        if let Some(level) = &mut self.specials {
            self.spare_specials = std::mem::replace(&mut level.sectors, plan.specials);
        }
        let mut writes = plan.writes;
        writes.commit(&mut self.sectors, &mut self.line_specials);
        self.spare_writes = writes;
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
            sectors: plan.writes.sectors(&self.sectors),
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
            sectors: plan.writes.sectors(&self.sectors),
            options: self.actor_options,
            rng: &mut plan.rng,
            spatial: &mut plan.spatial,
            bindings: &mut plan.bindings,
            events: &mut plan.events,
        });
        if let Some(error) = control.failure.get() {
            return Err(self.policy_error(error));
        }
        let events = self.commit_demon_plan(plan);
        self.pending.extend(events);
        Ok(result)
    }
    /// The real-map policy names the source's 129th intercept of a trace, reached
    /// by a hitscan or a slide inside a refused tic, `MapScaleError::Intercepts`
    /// (M3h6 note §4.4); every other error and policy is unchanged.
    fn policy_error(&self, error: SimError) -> SimError {
        if self.player_policy().has_real_map()
            && error == SimError::Combat(CombatError::InterceptLimit)
        {
            MapScaleError::Intercepts.into()
        } else {
            error
        }
    }
    pub(crate) fn advance_expanded_demons(
        &mut self,
        commands: [TicCommand; 4],
    ) -> Result<(), SimError> {
        let command_policy = self.player_policy().has_player_commands();
        let weapon_policy = self.player_policy().has_player_weapons();
        let specials = self.specials.is_some();
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
                    sectors: plan.writes.sectors(&self.sectors),
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
                    )
                    .map_err(|error| self.policy_error(error))?;
                }
                let live = host
                    .player
                    .as_ref()
                    .is_some_and(|p| p.state == PlayerBodyState::Live);
                if weapon_policy && specials && live {
                    // P_PlayerThink: P_CalcHeight, then P_PlayerInSpecialSector,
                    // then the weapon change and P_MovePsprites (M3h7 step 3 order).
                    // The source leveltime of this tic is the completed-tic count.
                    let command = commands[usize::from(PlayerSlot::One.ordinal())];
                    player_pistol::stage_operation(
                        &mut host,
                        weapons::PlayerWeaponOperation::CalcHeight,
                        command,
                        self.tic,
                        plan.sound_targets.as_mut_slice(),
                    )
                    .map_err(|error| self.policy_error(error))?;
                    sector_thinkers::player_in_special_sector(
                        &mut host,
                        &mut plan.specials,
                        self.tic,
                    )?;
                    if let Some(error) = control.failure.get() {
                        return Err(self.policy_error(error));
                    }
                    player_pistol::stage_operation(
                        &mut host,
                        weapons::PlayerWeaponOperation::AfterSpecials,
                        command,
                        self.tic,
                        plan.sound_targets.as_mut_slice(),
                    )
                    .map_err(|error| self.policy_error(error))?;
                } else if weapon_policy {
                    player_pistol::stage(
                        &mut host,
                        commands[usize::from(PlayerSlot::One.ordinal())],
                        self.tic,
                        plan.sound_targets.as_mut_slice(),
                    )
                    .map_err(|error| self.policy_error(error))?;
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
            perception_inputs.sound_targets = plan.sound_targets.as_slice();
        }
        thinkers::run(
            &mut plan.actors,
            thinkers::Inputs {
                world: &self.world,
                sectors: plan.writes.sectors(&self.sectors),
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
                boundary: self.boundary.as_deref(),
                specials: specials.then_some(plan.specials.as_mut_slice()),
            },
        );
        if let Some(error) = control.failure.get() {
            return Err(self.policy_error(error));
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
