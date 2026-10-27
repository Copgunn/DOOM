//! Deterministic, headless Linux DOOM simulation foundations.
//!
//! Each [`Simulation::step`] is one logical 35 Hz tic, never elapsed wall time.
//! Supported movement is grounded non-player XY motion in a static one-sector
//! map. Command thrust is a separately verified player-input foundation, not
//! complete player movement/sliding or vanilla gameplay synchronization.

mod actor;
mod geometry;
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

/// Deterministic stepping/setup failure. Unsupported gameplay is explicit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimError {
    /// Actor storage/scheduling failure.
    Store(StoreError),
    /// Unsupported or invalid map/profile.
    Geometry(GeometryError),
    /// Input slot is outside 0..4 or duplicates an existing binding.
    InvalidPlayer,
    /// Button behavior is not implemented in M2.
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
        let world = StaticWorld::new(map)?;
        let map_fields = Arc::new(snapshot::map_fields(world.map()));
        Ok(Self {
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
    /// Explicit deterministic gameplay draw, queued as an observation for next tic.
    pub fn gameplay_random(&mut self) -> u8 {
        let value = self.rng.p_random();
        self.pending.push(Event::GameplayRandom(value));
        value
    }
    /// Spawns a parameterized grounded mover, consuming exactly one P_Random for
    /// lastlook after storage succeeds. Full P_SpawnMobj type/state setup is absent.
    pub fn spawn(&mut self, spec: ActorSpec) -> Result<ActorId, SimError> {
        self.world.validate_actor(spec.radius, spec.height)?;
        let actor = Actor {
            spec,
            floor: self.world.floor(),
            ceiling: self.world.ceiling(),
            z: self.world.floor(),
            lastlook: 0,
        };
        let id = self.actors.spawn(actor)?;
        self.actors
            .get_mut(id)
            .ok_or(StoreError::InvalidHandle)?
            .lastlook = self.rng.p_random() % 4;
        self.pending.push(Event::Spawned(id));
        Ok(id)
    }
    /// Marks removal and unbinds input slots immediately. Storage retires at traversal.
    pub fn mark_remove(&mut self, id: ActorId) -> Result<(), SimError> {
        if self.actors.mark_remove(id)? {
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
        self.actors
            .get_mut(id)
            .ok_or(StoreError::InvalidHandle)?
            .spec
            .momentum = momentum;
        Ok(())
    }
    /// Binds one of four ordered command slots. Controlled movers keep the
    /// supported non-player collision response; this is not full player spawning.
    pub fn bind_player(&mut self, slot: usize, id: ActorId) -> Result<(), SimError> {
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
    /// Unsupported buttons and tic exhaustion are rejected before mutation.
    pub fn step(&mut self, commands: [TicCommand; 4]) -> Result<Snapshot, SimError> {
        if self.paused {
            return Ok(self.snapshot());
        }
        if self.tic == u64::MAX {
            return Err(SimError::TicOverflow);
        }
        if commands.iter().any(|c| c.buttons != 0) {
            return Err(SimError::UnsupportedButtons);
        }
        self.events = std::mem::take(&mut self.pending);
        self.commands = commands;
        self.events.push(Event::Phase(Phase::Players));
        for (slot, command) in commands.into_iter().enumerate() {
            if let Some(id) = self.bindings[slot] {
                if self.actors.is_pending(id) != Some(false) {
                    self.bindings[slot] = None;
                    continue;
                }
                let actor = self.actors.get_mut(id).ok_or(StoreError::InvalidHandle)?;
                let (angle, momentum) =
                    command_thrust(actor.spec.angle, actor.spec.momentum, command)?;
                actor.spec.angle = angle;
                actor.spec.momentum = momentum;
                self.events.push(Event::Player {
                    slot: slot as u8,
                    actor: id,
                });
            }
        }
        self.events.push(Event::Phase(Phase::Thinkers));
        let world = &self.world;
        let events = &mut self.events;
        let mut failure = None;
        // Built-in updates never spawn; max_slots therefore bounds all visits.
        self.actors.run(
            self.actors.max_slots as usize,
            |visit, context| match visit {
                Visit::Retired(id) => events.push(Event::Retired(id)),
                Visit::Think(id) => {
                    events.push(Event::Think(id));
                    if let Some(actor) = context.get_mut(id) {
                        match world.move_xy(actor) {
                            Ok(attempts) => events.extend(
                                attempts
                                    .into_iter()
                                    .map(|attempt| Event::Movement { actor: id, attempt }),
                            ),
                            Err(error) => failure = Some(error),
                        }
                    }
                }
            },
        )?;
        if let Some(error) = failure {
            return Err(error.into());
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
        let mut simulation = super::Simulation::new(map).unwrap();
        simulation.tic = u64::MAX;
        let before = simulation.snapshot();
        assert_eq!(
            simulation.step([super::TicCommand::default(); 4]),
            Err(super::SimError::TicOverflow)
        );
        assert_eq!(simulation.snapshot(), before);
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
}
