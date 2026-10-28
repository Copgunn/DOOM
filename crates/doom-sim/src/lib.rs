//! Deterministic, headless Linux DOOM simulation foundations.
//!
//! Each [`Simulation::step`] is one logical 35 Hz tic, never elapsed wall time.
//! Explicit profiles preserve M2 foundations and M3a grounded player collision,
//! and add M3b static-sector BSP/openings and XY/Z/gravity. None implements full
//! player thinking or establishes complete vanilla gameplay synchronization.
//! Profile 4 adds admitted source actor states, spawning and controlled death.
//! Profile 5 adds bounded unbound-actor damage, autoaim and hitscan combat.
//! Profile 6 adds command-driven player weapons, ammo and source psprites.
//! Profile 7 adds explicit original plasma launch, flight, direct impact and removal.

#[cfg(test)]
extern crate self as doom_sim;

mod actor;
mod perception;
mod sight;
pub use perception::PerceptionError;
pub use sight::{SightResult, SightVisit};
mod projectiles;
pub use projectiles::{ProjectileAim, ProjectileError, ProjectileEvent, ProjectileRequest};
mod weapons;
pub use weapons::{
    BT_ATTACK, BT_CHANGE, BT_WEAPONMASK, Psprite, Weapon, WeaponError, WeaponEvent,
    WeaponInventory, WeaponPlayer,
};
mod combat;
mod combat_trace;
pub use combat::{AttackResult, CombatError, CombatEvent, DamageRequest, DamageResult, LineAttack};
pub use combat_trace::{AimResult, HitscanTrace, Intercept, TraceObject};
mod lifecycle;
mod spawning;
pub use lifecycle::{
    ActionId, ActorAction, ActorError, ActorGameMode, ActorLife, ActorOptions, ActorRegistry,
    ActorStateDef, ActorStateEvent, ActorTypeDef, ActorTypeId, ProbeAction, Skill, StateId,
};
pub use spawning::MapSpawnReport;
mod geometry;
mod movement;
mod multisector;
pub use movement::{ActorMoveAttempt, Solidity};
pub use multisector::{
    Gravity, Location, MoveFailure, Opening, OpeningVisit, SectorMoveAttempt, SectorState,
    SpawnHeight, VerticalEvent, VerticalSpec,
};
mod snapshot;
mod store;
include!("fine_sine.rs");

pub use actor::{Actor, ActorSpec, TicCommand};
use doom_map::Map;
use doom_types::{DoomRng, Fixed};
pub use geometry::{Collision, GeometryError, LineGeometry, MoveAttempt, StaticWorld};
pub use snapshot::{Divergence, Snapshot, StateField, compare_traces};
use std::{error::Error, fmt, sync::Arc};
pub use store::{ActorId, StoreError, ThinkerContext, ThinkerStore, Visit};

/// Explicit behavior/schema profile; M2 remains frozen for regression consumers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Profile {
    /// M2 non-solid non-player response, command foundations, DSIM-v1.
    Foundation,
    /// M3a grounded player sliding and zero/MF_SOLID actors, DSIM-v2.
    PlayerCollision,
    /// M3b static connected sectors, BSP/XY/Z/gravity and reference lists, DSIM-v3.
    MultiSector,
    /// M3c typed actor lifecycle on static-sector physics, DSIM-v4.
    ActorStates,
    /// M3d1 unbound actor damage/hitscan subset, DSIM-v5.
    Combat,
    /// M3d2 player weapons and psprites, DSIM-v6.
    PlayerWeapons,
    /// M3d2b bounded original plasma projectiles, DSIM-v7.
    Projectiles,
    /// M3d3a sight, player scanning and source awareness, DSIM-v8.
    Perception,
}

impl Profile {
    pub(crate) fn uses_sectors(self) -> bool {
        matches!(
            self,
            Self::MultiSector
                | Self::ActorStates
                | Self::Combat
                | Self::PlayerWeapons
                | Self::Projectiles
                | Self::Perception
        )
    }
}

/// Deterministic stepping/setup failure. Unsupported gameplay is explicit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimError {
    /// Actor storage/scheduling failure.
    Store(StoreError),
    /// Actor definition, action admission or bounded preflight failure.
    Actor(ActorError),
    /// Bounded combat validation failure, before mutation.
    Combat(CombatError),
    /// Weapon profile, command, admission or preflight failure.
    Weapon(WeaponError),
    /// Projectile admission, ownership or arithmetic failure.
    Projectile(ProjectileError),
    /// Perception profile, identity, metadata or bounded-work rejection.
    Perception(PerceptionError),
    /// Unsupported or invalid map/profile.
    Geometry(GeometryError),
    /// Input slot is outside 0..4 or duplicates an existing binding.
    InvalidPlayer,
    /// Button behavior is not implemented in either profile.
    UnsupportedButtons,
    /// Logical u64 tic counter exhausted; no state is advanced.
    TicOverflow,
}
impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "simulation: {self:?}")
    }
}
impl Error for SimError {}
impl From<PerceptionError> for SimError {
    fn from(e: PerceptionError) -> Self {
        Self::Perception(e)
    }
}
impl From<ProjectileError> for SimError {
    fn from(e: ProjectileError) -> Self {
        Self::Projectile(e)
    }
}
impl From<WeaponError> for SimError {
    fn from(e: WeaponError) -> Self {
        Self::Weapon(e)
    }
}
impl From<CombatError> for SimError {
    fn from(e: CombatError) -> Self {
        Self::Combat(e)
    }
}
impl From<ActorError> for SimError {
    fn from(e: ActorError) -> Self {
        Self::Actor(e)
    }
}
impl From<StoreError> for SimError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}
impl From<GeometryError> for SimError {
    fn from(e: GeometryError) -> Self {
        Self::Geometry(e)
    }
}

/// Visible phase order, from P_Ticker. Specials/respawn are phase markers only.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    /// Input slots in numerical order, before thinkers.
    Players,
    /// Append-sensitive thinker traversal.
    Thinkers,
    /// No map-special implementation in M2.
    Specials,
    /// No respawn implementation in M2.
    Respawn,
}

/// Deterministic observations. Presentation may read but never consume/mutate them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    /// Ordered source projectile observations in profile 7.
    Projectile(ProjectileEvent),
    /// Ordered profile-6 player weapon observations.
    Weapon(WeaponEvent),
    /// Profile-5 combat evidence in source side-effect order.
    Combat(CombatEvent),
    /// Typed state/action/lifecycle evidence in execution order.
    ActorState(ActorStateEvent),
    /// Phase boundary.
    Phase(Phase),
    /// Actor appended, after its initial lastlook RNG draw.
    Spawned(ActorId),
    /// Logical removal mark, before physical retirement.
    RemovalMarked(ActorId),
    /// Storage retired and old generation invalidated.
    Retired(ActorId),
    /// Ground command applied in player-slot order.
    Player {
        /// Input slot.
        slot: u8,
        /// Controlled mover.
        actor: ActorId,
    },
    /// Actor update in linked order.
    Think(ActorId),
    /// One destination attempt, including candidate order and blocking result.
    Movement {
        /// Moving actor.
        actor: ActorId,
        /// Attempt details.
        attempt: MoveAttempt,
    },
    /// Explicit gameplay random request, separate from implicit spawn lastlook.
    GameplayRandom(u8),
    /// M3a things-before-lines destination attempt.
    ActorMovement {
        /// Moving actor.
        actor: ActorId,
        /// Ordered collision evidence.
        attempt: ActorMoveAttempt,
    },
    /// M3b destination and opening/clearance decision.
    SectorMovement {
        /// Moving actor.
        actor: ActorId,
        /// Ordered collision/height evidence.
        attempt: SectorMoveAttempt,
    },
    /// M3b Z update after this actor's XY movement.
    Vertical {
        /// Updated actor.
        actor: ActorId,
        /// Physical floor/ceiling/gravity observation.
        update: VerticalEvent,
    },
    /// Selected wall projection, in slide retry order.
    Slide {
        /// Sliding player.
        actor: ActorId,
        /// Selected linedef.
        line: u16,
    },
}

/// Ground thrust/angle part of P_MovePlayer, using exact original sine data.
/// The caller supplies grounded state. It does not implement player state changes,
/// airborne movement, weapon buttons or the player's blocked-move sliding response.
pub fn command_thrust(
    angle: u32,
    momentum: [Fixed; 2],
    command: TicCommand,
) -> Result<(u32, [Fixed; 2]), SimError> {
    if command.buttons != 0 {
        return Err(SimError::UnsupportedButtons);
    }
    let angle = angle.wrapping_add((i32::from(command.angle_turn) as u32).wrapping_shl(16));
    let mut momentum = momentum;
    for (heading, amount) in [
        (angle, command.forward),
        (angle.wrapping_sub(0x40000000), command.side),
    ] {
        if amount == 0 {
            continue;
        }
        let index = (heading >> 19) as usize;
        let force = Fixed::from_bits(i32::from(amount) * 2048);
        momentum[0] = momentum[0] + force * Fixed::from_bits(FINE_SINE[index + 2048]);
        momentum[1] = momentum[1] + force * Fixed::from_bits(FINE_SINE[index]);
    }
    Ok((angle, momentum))
}

/// Owned deterministic state. Map data is immutable and all runtime state is explicit.
#[derive(Clone, Debug)]
pub struct Simulation {
    pub(crate) profile: Profile,
    pub(crate) registry: Option<Arc<ActorRegistry>>,
    pub(crate) weapon_player: Option<WeaponPlayer>,
    pub(crate) sound_targets: Vec<Option<ActorId>>,
    pub(crate) actor_options: ActorOptions,
    pub(crate) spatial: movement::SpatialIndex,
    pub(crate) sectors: Vec<SectorState>,
    pub(crate) world: StaticWorld,
    pub(crate) actors: ThinkerStore<Actor>,
    pub(crate) rng: DoomRng,
    pub(crate) tic: u64,
    pub(crate) paused: bool,
    pub(crate) bindings: [Option<ActorId>; 4],
    pub(crate) commands: [TicCommand; 4],
    pub(crate) events: Vec<Event>,
    pub(crate) pending: Vec<Event>,
    pub(crate) map_fields: Arc<Vec<StateField>>,
}
impl Simulation {
    /// Creates a fresh supported world with capacity for 1024 logical actor slots.
    /// Map THINGS are retained as data; complete type-driven spawning is M3 work.
    pub fn new(map: Map) -> Result<Self, SimError> {
        Self::with_actor_limit(map, 1024)
    }
    /// Creates a fresh world with an explicit maximum of 65536 logical slots.
    pub fn with_actor_limit(map: Map, max_actors: u32) -> Result<Self, SimError> {
        Self::create(map, max_actors, Profile::Foundation)
    }
    /// M3a static grounded player/solid collision profile (DSIM-v2), 1024 slots.
    pub fn player_collision(map: Map) -> Result<Self, SimError> {
        Self::with_player_collision_limit(map, 1024)
    }
    /// M3a profile with at most 1024 logical slots and bounded geometry.
    pub fn with_player_collision_limit(map: Map, max_actors: u32) -> Result<Self, SimError> {
        if max_actors > 1024 {
            return Err(StoreError::InvalidLimit.into());
        }
        Self::create(map, max_actors, Profile::PlayerCollision)
    }
    /// Explicit M3b static multi-sector and XY/Z profile (DSIM-v3). Supplied
    /// BLOCKMAP lists define collision candidates; missing spatial coverage is
    /// diagnosed conservatively, never silently rebuilt.
    pub fn multisector(map: Map) -> Result<Self, SimError> {
        Self::with_multisector_limit(map, 1024)
    }
    /// M3b profile with an explicit maximum of 1024 logical actor slots.
    pub fn with_multisector_limit(map: Map, max_actors: u32) -> Result<Self, SimError> {
        if max_actors > 1024 {
            return Err(StoreError::InvalidLimit.into());
        }
        Self::create(map, max_actors, Profile::MultiSector)
    }
    fn create(map: Map, max_actors: u32, profile: Profile) -> Result<Self, SimError> {
        let mut world = if profile.uses_sectors() {
            StaticWorld::new_multisector(map)?
        } else {
            StaticWorld::new(map)?
        };
        if profile == Profile::PlayerCollision {
            movement::validate_profile(&mut world)?;
        }
        if profile.uses_sectors() {
            movement::prepare_reference_profile(&mut world)?;
            world.topology = Some(multisector::Topology::new(&world)?);
        }
        let sectors = if profile.uses_sectors() {
            world
                .map()
                .sectors()
                .iter()
                .map(|s| SectorState {
                    floor: Fixed::from_map_units(s.heights[0]),
                    ceiling: Fixed::from_map_units(s.heights[1]),
                })
                .collect()
        } else {
            Vec::new()
        };
        let map_fields = Arc::new(snapshot::map_fields(world.map()));
        Ok(Self {
            profile,
            registry: None,
            weapon_player: None,
            sound_targets: Vec::new(),
            actor_options: ActorOptions::default(),
            sectors,
            spatial: movement::SpatialIndex::default(),
            world,
            actors: ThinkerStore::new(max_actors)?,
            rng: DoomRng::default(),
            tic: 0,
            paused: false,
            bindings: [None; 4],
            commands: [TicCommand::default(); 4],
            events: Vec::new(),
            pending: Vec::new(),
            map_fields,
        })
    }
    /// Selected behavior and canonical schema profile.
    pub const fn profile(&self) -> Profile {
        self.profile
    }
    /// Most-recent spatial link order in M3a, including off-map live actors.
    pub fn spatial_order(&self) -> &[ActorId] {
        &self.spatial.order
    }
    /// Read-only M3a collision query at a destination; no occupancy side effects.
    pub fn check_actor_position(
        &self,
        id: ActorId,
        target: [Fixed; 2],
    ) -> Result<ActorMoveAttempt, SimError> {
        if self.profile != Profile::PlayerCollision {
            return Err(GeometryError::Unsupported("requires M3a profile").into());
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        let actor = self.actors.get(id).ok_or(StoreError::InvalidHandle)?;
        Ok(movement::check(
            &self.world,
            id,
            actor,
            target,
            &self.spatial,
            &|id| self.actors.get(id).copied(),
        ))
    }
    /// Current read-only runtime sector state (empty in older profiles).
    pub fn sectors(&self) -> &[SectorState] {
        &self.sectors
    }
    /// Current sector-head ordering, independent of thinker or handle order.
    pub fn sector_actor_order(&self, sector: u16) -> Result<Vec<ActorId>, SimError> {
        if !self.profile.uses_sectors() {
            return Err(GeometryError::Unsupported("sector order requires M3b profile").into());
        }
        if usize::from(sector) >= self.sectors.len() {
            return Err(GeometryError::InvalidReference.into());
        }
        Ok(self
            .spatial
            .order
            .iter()
            .copied()
            .filter(|id| lifecycle::in_sector_list(self.actors.get(*id).expect("linked actor")))
            .filter(|id| {
                self.actors
                    .get(*id)
                    .expect("live spatial handle")
                    .location
                    .expect("M3b membership")
                    .sector
                    == sector
            })
            .collect())
    }
    /// P_LineOpening calculation from current runtime heights. One-sided lines
    /// return None; unsupported profile/index errors have no state effects.
    pub fn line_opening(&self, index: usize) -> Result<Option<Opening>, SimError> {
        if !self.profile.uses_sectors() {
            return Err(GeometryError::Unsupported("openings require M3b profile").into());
        }
        if index >= self.world.lines.len() {
            return Err(GeometryError::InvalidReference.into());
        }
        Ok(multisector::opening(&self.world, &self.sectors, index))
    }
    /// Read-only source TryMove decision, including step/dropoff/headroom checks.
    pub fn check_multisector_position(
        &self,
        id: ActorId,
        target: [Fixed; 2],
    ) -> Result<SectorMoveAttempt, SimError> {
        if !self.profile.uses_sectors() {
            return Err(GeometryError::Unsupported("sector collision requires M3b profile").into());
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        if self.profile == Profile::Projectiles {
            self.preflight_projectile_step()?;
            if target
                .iter()
                .any(|v| v.bits().unsigned_abs() > 8192 * 65536)
            {
                return Err(ProjectileError::Arithmetic.into());
            }
            return Ok(self.query_projectile_move(id, target).0);
        }
        let actor = self.actors.get(id).ok_or(StoreError::InvalidHandle)?;
        Ok(multisector::check_move(
            (&self.world, &self.sectors),
            id,
            actor,
            target,
            &self.spatial,
            &|id| self.actors.get(id).copied(),
            self.bindings.contains(&Some(id)),
        ))
    }
    /// Static immutable geometry.
    pub const fn world(&self) -> &StaticWorld {
        &self.world
    }
    /// Number of completed logical tics, independent of presentation rate.
    pub const fn tic(&self) -> u64 {
        self.tic
    }
    /// Whether stepping is frozen before all gameplay phases.
    pub const fn paused(&self) -> bool {
        self.paused
    }
    /// Explicit pause control. Paused step calls consume no commands/RNG/events.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }
    /// Read-only actor lookup; marked storage remains readable until retirement.
    pub fn actor(&self, id: ActorId) -> Option<&Actor> {
        self.actors.get(id)
    }
    /// Linked processing order, independent of slot reuse.
    pub fn actor_order(&self) -> impl Iterator<Item = ActorId> + '_ {
        self.actors.order()
    }
    /// Copy of RNG state for observation; mutating the copy cannot affect simulation.
    pub const fn rng(&self) -> DoomRng {
        self.rng
    }
    /// Last completed tic's events; never drained by presentation.
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    /// Read-only observations from explicit operations since the last tic.
    /// These are retained in the next completed tic and in canonical pending state.
    pub fn pending_events(&self) -> &[Event] {
        &self.pending
    }
    /// Explicit deterministic gameplay draw, queued as an observation for next tic.
    pub fn gameplay_random(&mut self) -> u8 {
        let value = self.rng.p_random();
        self.pending.push(Event::GameplayRandom(value));
        value
    }
    /// Spawns a parameterized grounded mover, consuming exactly one P_Random for
    /// lastlook after storage succeeds. Full P_SpawnMobj type/state setup is absent.
    pub fn spawn(&mut self, spec: ActorSpec) -> Result<ActorId, SimError> {
        self.spawn_with_solidity(spec, Solidity::NonSolid)
    }
    /// Parameterized setup with zero/MF_SOLID flags in M3a. Like P_SpawnMobj,
    /// overlapping initial placements are allowed; query occupancy explicitly.
    /// Validation/capacity errors preserve logical state, index order and RNG.
    pub fn spawn_with_solidity(
        &mut self,
        spec: ActorSpec,
        solidity: Solidity,
    ) -> Result<ActorId, SimError> {
        if self.profile == Profile::ActorStates {
            return Err(ActorError::UnsupportedMode.into());
        }
        if self.profile.uses_sectors() {
            return self.spawn_vertical(spec, solidity, VerticalSpec::default());
        }
        self.world.validate_actor(spec.radius, spec.height)?;
        if self.profile == Profile::Foundation && solidity != Solidity::NonSolid {
            return Err(GeometryError::Unsupported("solid actors require M3a profile").into());
        }
        if self.profile == Profile::PlayerCollision {
            self.spatial.reserve_spawn()?;
        }
        let actor = Actor {
            spec,
            floor: self.world.floor(),
            ceiling: self.world.ceiling(),
            z: self.world.floor(),
            lastlook: 0,
            solidity,
            momz: Fixed::ZERO,
            gravity: Gravity::Normal,
            allow_dropoff: false,
            location: None,
            life: None,
        };
        let id = self.actors.spawn(actor)?;
        self.actors
            .get_mut(id)
            .expect("newly spawned actor handle")
            .lastlook = self.rng.p_random() % 4;
        if self.profile == Profile::PlayerCollision {
            self.spatial.link(id);
        }
        self.pending.push(Event::Spawned(id));
        Ok(id)
    }
    /// M3b parameterized spawn with source floor/ceiling/absolute Z, vertical
    /// momentum and explicit MF_NOGRAVITY/MF_DROPOFF exceptions. No state/type,
    /// teleport, floating monster or collision-side-effect spawning is implied.
    pub fn spawn_vertical(
        &mut self,
        spec: ActorSpec,
        solidity: Solidity,
        vertical: VerticalSpec,
    ) -> Result<ActorId, SimError> {
        if self.profile != Profile::MultiSector {
            return Err(GeometryError::Unsupported(
                "parameterized vertical spawn requires M3b profile",
            )
            .into());
        }
        let location = self.world.locate_validated(spec.position);
        let sector = self.sectors[usize::from(location.sector)];
        if spec.radius < Fixed::ZERO
            || spec.radius > Fixed::from_map_units(32)
            || spec.height <= Fixed::ZERO
            || spec.height > sector.ceiling - sector.floor
        {
            return Err(GeometryError::InvalidActor.into());
        }
        let z = match vertical.height {
            SpawnHeight::Floor => sector.floor,
            SpawnHeight::Ceiling => sector.ceiling - spec.height,
            SpawnHeight::Absolute(z) => z,
        };
        self.spatial.reserve_spawn()?;
        let actor = Actor {
            spec,
            floor: sector.floor,
            ceiling: sector.ceiling,
            z,
            lastlook: 0,
            solidity,
            momz: vertical.momentum,
            gravity: vertical.gravity,
            allow_dropoff: vertical.allow_dropoff,
            location: Some(location),
            life: None,
        };
        let id = self.actors.spawn(actor)?;
        self.actors.get_mut(id).expect("new M3b handle").lastlook = self.rng.p_random() % 4;
        self.spatial.link(id);
        self.pending.push(Event::Spawned(id));
        Ok(id)
    }
    /// Changes M3b vertical momentum between tics; invalid/profile errors preserve state.
    pub fn set_vertical_momentum(&mut self, id: ActorId, momentum: Fixed) -> Result<(), SimError> {
        if !self.profile.uses_sectors() {
            return Err(
                GeometryError::Unsupported("vertical momentum requires M3b profile").into(),
            );
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        self.projectile_control(id)?;
        if self.profile == Profile::Projectiles && momentum.bits().unsigned_abs() > 64 * 65536 {
            return Err(ProjectileError::Arithmetic.into());
        }
        self.actors.get_mut(id).unwrap().momz = momentum;
        Ok(())
    }
    /// Marks removal and unbinds input slots immediately. Storage retires at traversal.
    pub fn mark_remove(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.actors.mark_remove(id)? {
            self.spatial.unlink(id);
            self.pending.push(Event::RemovalMarked(id));
        }
        for binding in &mut self.bindings {
            if *binding == Some(id) {
                *binding = None;
            }
        }
        Ok(())
    }
    /// Changes explicit momentum between tics; next update clamps it like the reference.
    pub fn set_momentum(&mut self, id: ActorId, momentum: [Fixed; 2]) -> Result<(), SimError> {
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        self.projectile_control(id)?;
        if self.profile == Profile::Projectiles
            && momentum
                .iter()
                .any(|v| v.bits().unsigned_abs() > 4096 * 65536)
        {
            return Err(ProjectileError::Arithmetic.into());
        }
        self.actors.get_mut(id).unwrap().spec.momentum = momentum;
        Ok(())
    }
    /// Binds one of four ordered command slots. Controlled movers keep the
    /// M2 non-player response or M3a player XY response, according to the profile.
    /// This does not implement full type/state-driven player spawning.
    pub fn bind_player(&mut self, slot: usize, id: ActorId) -> Result<(), SimError> {
        if self.profile == Profile::Projectiles {
            return Err(ProjectileError::Player.into());
        }
        if self.profile == Profile::PlayerWeapons {
            return Err(WeaponError::Player.into());
        }
        if slot >= 4
            || self
                .bindings
                .iter()
                .enumerate()
                .any(|(i, v)| i != slot && *v == Some(id))
        {
            return Err(SimError::InvalidPlayer);
        }
        if self.actors.is_pending(id) != Some(false) {
            return Err(StoreError::InvalidHandle.into());
        }
        self.bindings[slot] = Some(id);
        Ok(())
    }
    /// Advances exactly one 35 Hz logical tic. No wall-clock/frame-time parameter.
    /// Recoverable errors (unsupported buttons/tic exhaustion) preserve all state.
    /// Actor/map invariants are validated by setup, so movement has no late
    /// recoverable error path. Allocation exhaustion is the process-level Rust
    /// policy, not SimError. Internal invariant assertions indicate engine bugs.
    pub fn step(&mut self, commands: [TicCommand; 4]) -> Result<Snapshot, SimError> {
        if self.paused {
            return Ok(self.snapshot());
        }
        if self.tic == u64::MAX {
            return Err(SimError::TicOverflow);
        }
        if self.profile == Profile::PlayerWeapons {
            self.preflight_weapon_step(&commands)?;
        } else if commands.iter().any(|c| c.buttons != 0) {
            return Err(SimError::UnsupportedButtons);
        }
        if self.profile == Profile::Projectiles {
            self.preflight_projectile_step()?;
        }
        self.preflight_actor_step()?;
        self.events = std::mem::take(&mut self.pending);
        self.commands = commands;
        self.events.push(Event::Phase(Phase::Players));
        if self.profile == Profile::PlayerWeapons {
            self.weapon_tick(commands);
            self.events.append(&mut self.pending);
        } else {
            for (slot, command) in commands.into_iter().enumerate() {
                if let Some(id) = self.bindings[slot] {
                    if self.actors.is_pending(id) != Some(false) {
                        self.bindings[slot] = None;
                        continue;
                    }
                    let actor = self.actors.get_mut(id).expect("validated active binding");
                    let physical_command = if self.profile.uses_sectors() && actor.z > actor.floor {
                        TicCommand {
                            forward: 0,
                            side: 0,
                            ..command
                        }
                    } else {
                        command
                    };
                    let (angle, momentum) =
                        command_thrust(actor.spec.angle, actor.spec.momentum, physical_command)
                            .expect("buttons prevalidated");
                    actor.spec.angle = angle;
                    actor.spec.momentum = momentum;
                    self.events.push(Event::Player {
                        slot: slot as u8,
                        actor: id,
                    });
                }
            }
        }
        self.events.push(Event::Phase(Phase::Thinkers));
        if self.profile == Profile::Projectiles {
            self.projectile_thinkers_with(|_, _| {});
        } else {
            let world = &self.world;
            let events = &mut self.events;
            let profile = self.profile;
            let bindings = self.bindings;
            let live_bindings = &mut self.bindings;
            let registry = &self.registry;
            let options = self.actor_options;
            let rng = &mut self.rng;
            let spatial = &mut self.spatial;
            let sectors = &self.sectors;
            // Profile 4 preflights all bounded action work and same-pass child allocation.
            self.actors
                .run(
                    self.actors.max_slots as usize,
                    |visit, context| match visit {
                        Visit::Retired(id) => events.push(Event::Retired(id)),
                        Visit::Think(id) => {
                            events.push(Event::Think(id));
                            let mut actor = *context.get(id).expect("live thinker handle");
                            let had_xy_momentum =
                                actor.momentum().iter().any(|m| *m != Fixed::ZERO);
                            if profile == Profile::Foundation {
                                events.extend(
                                    world
                                        .move_xy(&mut actor)
                                        .into_iter()
                                        .map(|attempt| Event::Movement { actor: id, attempt }),
                                );
                            } else {
                                let command = bindings
                                    .iter()
                                    .position(|binding| *binding == Some(id))
                                    .map(|slot| commands[slot]);
                                let motion = if profile.uses_sectors() {
                                    movement::move_xy_multisector(
                                        world,
                                        id,
                                        &mut actor,
                                        spatial,
                                        |id| context.get(id).copied(),
                                        command,
                                        sectors,
                                    )
                                } else {
                                    movement::move_xy(
                                        world,
                                        id,
                                        &mut actor,
                                        spatial,
                                        |id| context.get(id).copied(),
                                        command,
                                    )
                                };
                                for event in motion {
                                    events.push(match event {
                                        movement::MotionEvent::Attempt(attempt) => {
                                            Event::ActorMovement { actor: id, attempt }
                                        }
                                        movement::MotionEvent::Slide(line) => {
                                            Event::Slide { actor: id, line }
                                        }
                                        movement::MotionEvent::Sector(attempt) => {
                                            Event::SectorMovement { actor: id, attempt }
                                        }
                                    });
                                }
                            }
                            if profile == Profile::PlayerWeapons
                                && had_xy_momentum
                                && actor.z <= actor.floor
                                && actor.momentum() == [Fixed::ZERO; 2]
                                && bindings.iter().position(|b| *b == Some(id)).is_some_and(
                                    |slot| commands[slot].forward == 0 && commands[slot].side == 0,
                                )
                            {
                                let run = lifecycle::source_state_id("S_PLAY_RUN1").unwrap().0;
                                if actor
                                    .life
                                    .unwrap()
                                    .state
                                    .is_some_and(|s| s.0.wrapping_sub(run) < 4)
                                {
                                    let play = lifecycle::source_state_id("S_PLAY").unwrap();
                                    *context.get_mut(id).unwrap() = actor;
                                    lifecycle::LifeHost {
                                        access: &mut *context,
                                        registry: registry.as_deref().unwrap(),
                                        world,
                                        sectors,
                                        options,
                                        rng,
                                        spatial,
                                        bindings: live_bindings,
                                        events,
                                    }
                                    .enter(id, play);
                                    actor = *context.get(id).unwrap();
                                }
                            }
                            if profile.uses_sectors() {
                                if let Some(update) = multisector::move_z(&mut actor) {
                                    events.push(Event::Vertical { actor: id, update });
                                }
                            }
                            *context.get_mut(id).expect("same live thinker handle") = actor;
                            if let Some(registry) = registry {
                                lifecycle::LifeHost {
                                    access: context,
                                    registry,
                                    world,
                                    sectors,
                                    options,
                                    rng,
                                    spatial,
                                    bindings: live_bindings,
                                    events,
                                }
                                .tick(id);
                            }
                        }
                    },
                )
                .expect("built-in traversal preserves links and is bounded by max slots");
        }
        self.events.push(Event::Phase(Phase::Specials));
        self.events.push(Event::Phase(Phase::Respawn));
        self.tic += 1;
        Ok(self.snapshot())
    }
    /// Captures every implemented state field and immutable map data canonically.
    pub fn snapshot(&self) -> Snapshot {
        snapshot::capture(self)
    }
}

/// Runs a command stream headlessly and records one canonical snapshot per call.
/// Setup/spawn/removal/random requests, if needed, are explicit caller operations.
pub fn run_commands(
    simulation: &mut Simulation,
    commands: impl IntoIterator<Item = [TicCommand; 4]>,
) -> Result<Vec<Snapshot>, SimError> {
    commands
        .into_iter()
        .map(|commands| simulation.step(commands))
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn logical_tic_exhaustion_rejects_before_mutation() {
        let wad = doom_wad::Wad::parse(
            include_bytes!("../../../fuzz/corpus/map/classic_square"),
            doom_wad::Limits::default(),
        )
        .unwrap();
        let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
        for profile in [
            super::Profile::Foundation,
            super::Profile::PlayerCollision,
            super::Profile::MultiSector,
        ] {
            let mut simulation = super::Simulation::create(map.clone(), 1024, profile).unwrap();
            simulation.tic = u64::MAX;
            let before = simulation.snapshot();
            assert_eq!(
                simulation.step([super::TicCommand::default(); 4]),
                Err(super::SimError::TicOverflow)
            );
            assert_eq!(simulation.snapshot(), before);
        }
    }
    #[test]
    fn command_button_rejection_is_explicit() {
        assert_eq!(
            super::command_thrust(
                0,
                [doom_types::Fixed::ZERO; 2],
                super::TicCommand {
                    buttons: 1,
                    ..Default::default()
                }
            ),
            Err(super::SimError::UnsupportedButtons)
        );
    }
    #[test]
    fn allocator_capacity_does_not_change_multisector_state_or_results() {
        let wad = doom_wad::Wad::parse(
            include_bytes!("../../../tests/reference/m3b_map_2.bin"),
            doom_wad::Limits::default(),
        )
        .unwrap();
        let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
        let mut expected = super::Simulation::multisector(map).unwrap();
        let id = expected.spawn(super::ActorSpec::default()).unwrap();
        expected.bind_player(0, id).unwrap();
        let mut actual = expected.clone();
        actual.actors.slots.try_reserve(257).unwrap();
        actual.spatial.order.try_reserve(131).unwrap();
        actual.pending.try_reserve(127).unwrap();
        actual.events.try_reserve(255).unwrap();
        actual.sectors.try_reserve(127).unwrap();
        actual.world.lines.try_reserve(127).unwrap();
        for cell in actual.world.collision_cells.as_mut().unwrap() {
            cell.try_reserve(127).unwrap();
        }
        assert_eq!(expected.snapshot(), actual.snapshot());
        for _ in 0..64 {
            let mut command = [super::TicCommand::default(); 4];
            command[0].forward = 50;
            assert_eq!(
                expected.step(command).unwrap(),
                actual.step(command).unwrap()
            );
        }
    }
}
