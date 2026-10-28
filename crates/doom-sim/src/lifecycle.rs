//! Validated source actor metadata and bounded state/action execution.
use crate::{
    Actor, ActorId, ActorSpec, Gravity, SectorState, Solidity, SpawnHeight, StaticWorld,
    StoreError, ThinkerContext, ThinkerStore,
};
use doom_types::{DoomRng, Fixed};
use std::{collections::BTreeSet, error::Error, fmt};
include!("source_actors.rs");
/// Source/registry type identity; lookups validate candidate indexes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ActorTypeId(pub u16);
/// Source/registry state identity. Zero is S_NULL, not a normal entered state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct StateId(pub u16);
/// Original symbolic action identity, indexed by the registry action-name table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionId(pub u16);
/// Explicit synthetic reference-adapter capabilities, not vanilla AI or scripts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbeAction {
    /// Consume one gameplay draw, with an observation.
    Random,
    /// Recursively request state entry, preserving the outer source nextstate.
    Enter(StateId),
    /// Append a typed child, set parent target to child, child target to parent.
    SpawnTarget(ActorTypeId),
    /// Immediate spatial unlink and deferred thinker retirement of self.
    RemoveSelf,
    /// Mark the current resolvable target; stale targets are observed as unavailable.
    RemoveTarget,
    /// Assign a validated self-reference.
    TargetSelf,
}
/// Dispatch metadata; unsupported original callbacks never become no-ops.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorAction {
    /// Source NULL action pointer.
    None,
    /// An original symbolic action (admission requires implemented semantics).
    Original(ActionId),
    /// A visibly separate, bounded synthetic callback used in reference probes.
    Probe(ProbeAction),
}
/// Immutable definition of one actor state, including inert presentation data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorStateDef {
    /// Symbolic name, at most 64 UTF-8 bytes.
    pub name: String,
    /// Original sprite-table index; no sprite pixels are loaded.
    pub sprite: u16,
    /// Original frame/fullbright bit pattern.
    pub frame: u32,
    /// Duration: -1 infinite, zero immediate on entry, positive countdown.
    pub tics: i32,
    /// Typed dispatch identity.
    pub action: ActorAction,
    /// Next state, including S_NULL.
    pub next: StateId,
    /// Preserved original miscellaneous fields.
    pub misc: [i32; 2],
}
/// Complete original mobjinfo properties. Recognition does not imply execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorTypeDef {
    /// Symbolic type name.
    pub name: String,
    /// Source's 23 ordered mobjinfo integer properties; validated by Registry.
    /// Access named properties through the methods below; values retain source units.
    pub properties: [i32; 23],
}
impl ActorTypeDef {
    /// DoomEd number (-1 means not selected by ordinary THINGS lookup).
    pub const fn doomed_number(&self) -> i32 {
        self.properties[0]
    }
    /// Spawn/see/pain/melee/missile/death/xdeath/raise state references.
    pub fn states(&self) -> [StateId; 8] {
        [1, 3, 7, 10, 11, 12, 13, 22].map(|i| StateId(self.properties[i] as u16))
    }
    /// Initial health, without damage processing.
    pub const fn spawn_health(&self) -> i32 {
        self.properties[2]
    }
    /// Initial reaction time except nightmare (not a generic per-tic decrement).
    pub const fn reaction_time(&self) -> i32 {
        self.properties[5]
    }
    /// Source pain-chance metadata; no pain/damage calculation is implied.
    pub const fn pain_chance(&self) -> i32 {
        self.properties[8]
    }
    /// See/attack/pain/death/active sound identifiers, as inert metadata.
    pub fn sounds(&self) -> [u16; 5] {
        [4, 6, 9, 14, 20].map(|i| self.properties[i] as u16)
    }
    /// Source speed value, whose units depend on actor type.
    pub const fn speed(&self) -> i32 {
        self.properties[15]
    }
    /// Type collision radius.
    pub const fn radius(&self) -> Fixed {
        Fixed::from_bits(self.properties[16])
    }
    /// Type collision height.
    pub const fn height(&self) -> Fixed {
        Fixed::from_bits(self.properties[17])
    }
    /// Source mass metadata.
    pub const fn mass(&self) -> i32 {
        self.properties[18]
    }
    /// Source damage metadata; not executable combat behavior.
    pub const fn damage(&self) -> i32 {
        self.properties[19]
    }
    /// Complete original flag bits.
    pub const fn flags(&self) -> u32 {
        self.properties[21] as u32
    }
}
/// Actor/state admission or preflight failure; no private input locations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorError {
    /// Invalid type index.
    InvalidType,
    /// Invalid state/index/definition reference.
    InvalidState,
    /// Sprite, sound, name or duration is invalid.
    InvalidMetadata,
    /// Unsupported original action identity.
    UnsupportedAction(ActionId),
    /// Type flags require later behavior.
    UnsupportedFlags(u32),
    /// Immediate zero/callback state cycles are not safe to execute.
    ImmediateCycle,
    /// Bounded state/spawn closure exceeded supported work.
    WorkLimit,
    /// Spawn dependency cycle in synthetic callback graph.
    SpawnCycle,
    /// Type dimensions do not fit the declared static-sector profile.
    Dimensions,
    /// This type has no death state; controlled death is not arbitrary deletion.
    NoDeathState,
    /// Full bound-player death/respawn is not implemented here.
    BoundDeath,
    /// Operation requires profile 4 or unsupported game mode.
    UnsupportedMode,
}
impl fmt::Display for ActorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "actor-state: {self:?}")
    }
}
impl Error for ActorError {}
/// Single-player skill levels with source filtering/reaction-time rules.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum Skill {
    /// Baby uses THINGS bit 1.
    Baby = 0,
    /// Easy also uses bit 1.
    Easy = 1,
    /// Medium uses bit 2.
    #[default]
    Medium = 2,
    /// Hard uses bit 4.
    Hard = 3,
    /// Nightmare uses bit 4 and zero initial reaction time; no monster respawn.
    Nightmare = 4,
}
/// Explicit map-spawn mode. Only single-player DOOM II is executable in M3c.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ActorGameMode {
    /// Supported filtered map-prop spawning; full player-start creation deferred.
    #[default]
    SinglePlayer,
    /// Recognized but currently rejected before mutation.
    Cooperative,
    /// Recognized but currently rejected before mutation.
    Deathmatch,
}
/// Profile-4 source setup options, included in canonical state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActorOptions {
    /// Source skill.
    pub skill: Skill,
    /// Only SinglePlayer is currently executable.
    pub mode: ActorGameMode,
    /// Source nomonsters filtering precedes unsupported selected-type admission.
    pub no_monsters: bool,
}
/// Immutable validated source/native definitions, separate from dispatch services.
#[derive(Clone, Debug)]
pub struct ActorRegistry {
    pub(crate) states: Vec<ActorStateDef>,
    pub(crate) types: Vec<ActorTypeDef>,
}
const ALLOWED_FLAGS: u32 = 2 | 4 | 8 | 16 | 32 | 256 | 512 | 1024 | 2048 | 262144 | 33554432;
const ENTRY_LIMIT: usize = 128;
const STEP_LIMIT: usize = 8192;
impl ActorRegistry {
    /// Compile all original metadata; unsupported actions/types stay recognizable.
    pub fn vanilla() -> Self {
        let states = SOURCE_STATES
            .iter()
            .map(|(name, v)| ActorStateDef {
                name: (*name).into(),
                sprite: v[0] as u16,
                frame: v[1] as u32,
                tics: v[2],
                action: if v[3] == 0 {
                    ActorAction::None
                } else {
                    ActorAction::Original(ActionId(v[3] as u16))
                },
                next: StateId(v[4] as u16),
                misc: [v[5], v[6]],
            })
            .collect();
        let types = SOURCE_TYPES
            .iter()
            .map(|(name, v)| ActorTypeDef {
                name: (*name).into(),
                properties: *v,
            })
            .collect();
        Self::new(states, types).expect("generated original metadata is validated")
    }
    /// Validate owned metadata before any actor can use it. Original unsupported
    /// action identities may be recognized; execution admission is separate.
    pub fn new(states: Vec<ActorStateDef>, types: Vec<ActorTypeDef>) -> Result<Self, ActorError> {
        if states.is_empty() || states.len() > 2048 || types.len() > 256 {
            return Err(ActorError::WorkLimit);
        }
        if states[0].tics != -1
            || states[0].next != StateId(0)
            || states[0].action != ActorAction::None
        {
            return Err(ActorError::InvalidState);
        }
        for s in &states {
            if s.name.len() > 64 || s.tics < -1 || usize::from(s.sprite) >= SOURCE_SPRITES.len() {
                return Err(ActorError::InvalidMetadata);
            }
            if usize::from(s.next.0) >= states.len() {
                return Err(ActorError::InvalidState);
            }
            match s.action {
                ActorAction::Original(id)
                    if id.0 == 0 || usize::from(id.0) >= SOURCE_ACTIONS.len() =>
                {
                    return Err(ActorError::InvalidMetadata);
                }
                ActorAction::Probe(ProbeAction::Enter(id)) if usize::from(id.0) >= states.len() => {
                    return Err(ActorError::InvalidState);
                }
                ActorAction::Probe(ProbeAction::SpawnTarget(id))
                    if usize::from(id.0) >= types.len() =>
                {
                    return Err(ActorError::InvalidType);
                }
                _ => {}
            }
        }
        for t in &types {
            if t.name.len() > 64 {
                return Err(ActorError::InvalidMetadata);
            }
            for i in [1, 3, 7, 10, 11, 12, 13, 22] {
                if t.properties[i] < 0 || t.properties[i] as usize >= states.len() {
                    return Err(ActorError::InvalidState);
                }
            }
            for i in [4, 6, 9, 14, 20] {
                if t.properties[i] < 0 || t.properties[i] as usize >= SOURCE_SOUNDS.len() {
                    return Err(ActorError::InvalidMetadata);
                }
            }
        }
        Ok(Self { states, types })
    }
    /// Read validated state metadata by candidate ID.
    pub fn state(&self, id: StateId) -> Option<&ActorStateDef> {
        self.states.get(usize::from(id.0))
    }
    /// Read recognized actor metadata, including currently unsupported types.
    pub fn actor_type(&self, id: ActorTypeId) -> Option<&ActorTypeDef> {
        self.types.get(usize::from(id.0))
    }
    /// States in deterministic registry order.
    pub fn states(&self) -> &[ActorStateDef] {
        &self.states
    }
    /// Types in deterministic source/registry order.
    pub fn types(&self) -> &[ActorTypeDef] {
        &self.types
    }
    /// Source type by exact symbolic name.
    pub fn find_type(&self, name: &str) -> Option<ActorTypeId> {
        self.types
            .iter()
            .position(|t| t.name == name)
            .map(|i| ActorTypeId(i as u16))
    }
    /// Source state by exact symbolic name.
    pub fn find_state(&self, name: &str) -> Option<StateId> {
        self.states
            .iter()
            .position(|s| s.name == name)
            .map(|i| StateId(i as u16))
    }
    /// Last-match-free original ordered DoomEd lookup.
    pub fn find_doomed(&self, number: i32) -> Option<ActorTypeId> {
        self.types
            .iter()
            .position(|t| t.doomed_number() == number)
            .map(|i| ActorTypeId(i as u16))
    }
    /// Original symbolic callback name, without granting execution support.
    pub fn action_name(&self, id: ActionId) -> Option<&'static str> {
        SOURCE_ACTIONS.get(usize::from(id.0)).copied()
    }
    /// Whether a source callback has implemented, reference-testable semantics.
    pub fn action_supported(&self, id: ActionId) -> bool {
        matches!(
            self.action_name(id),
            Some("A_Fall" | "A_Pain" | "A_Scream" | "A_XScream" | "A_PlayerScream")
        )
    }
    /// Validate an executable state closure without changing simulation state.
    pub fn validate_state(&self, state: StateId) -> Result<(), ActorError> {
        let mut seen = BTreeSet::new();
        let mut pending = vec![state];
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            let s = self.state(id).ok_or(ActorError::InvalidState)?;
            if id.0 == 0 {
                continue;
            }
            if let ActorAction::Original(action) = s.action {
                if !self.action_supported(action) {
                    return Err(ActorError::UnsupportedAction(action));
                }
            }
            pending.push(s.next);
            if let ActorAction::Probe(ProbeAction::Enter(target)) = s.action {
                pending.push(target);
            }
        }
        for id in seen {
            let mut path = BTreeSet::new();
            self.immediate_cycle(id, &mut path, &mut BTreeSet::new())?;
            let mut tics = 0;
            let mut work = 0;
            self.flow(id, &mut tics, &mut work, &mut Vec::new())?;
        }
        Ok(())
    }
    fn immediate_cycle(
        &self,
        id: StateId,
        path: &mut BTreeSet<StateId>,
        done: &mut BTreeSet<StateId>,
    ) -> Result<(), ActorError> {
        if id.0 == 0 || done.contains(&id) {
            return Ok(());
        }
        if !path.insert(id) {
            return Err(ActorError::ImmediateCycle);
        }
        if path.len() > ENTRY_LIMIT {
            return Err(ActorError::WorkLimit);
        }
        let s = self.state(id).ok_or(ActorError::InvalidState)?;
        if s.tics == 0 {
            self.immediate_cycle(s.next, path, done)?;
        }
        if let ActorAction::Probe(ProbeAction::Enter(next)) = s.action {
            self.immediate_cycle(next, path, done)?;
        }
        path.remove(&id);
        done.insert(id);
        Ok(())
    }
    fn flow(
        &self,
        mut state: StateId,
        tics: &mut i32,
        work: &mut usize,
        spawns: &mut Vec<ActorTypeId>,
    ) -> Result<(), ActorError> {
        loop {
            *work += 1;
            if *work > ENTRY_LIMIT {
                return Err(ActorError::WorkLimit);
            }
            if state.0 == 0 {
                return Ok(());
            }
            let s = self.state(state).ok_or(ActorError::InvalidState)?;
            *tics = s.tics;
            match s.action {
                ActorAction::Probe(ProbeAction::Enter(next)) => {
                    self.flow(next, tics, work, spawns)?
                }
                ActorAction::Probe(ProbeAction::SpawnTarget(ty)) => spawns.push(ty),
                _ => {}
            }
            if *tics != 0 {
                return Ok(());
            }
            state = s.next;
        }
    }
    pub(crate) fn spawn_dependencies(&self, root: StateId) -> Result<Vec<ActorTypeId>, ActorError> {
        let mut seen = BTreeSet::new();
        let mut todo = vec![root];
        let mut children = BTreeSet::new();
        while let Some(id) = todo.pop() {
            if id.0 == 0 || !seen.insert(id) {
                continue;
            }
            let s = self.state(id).ok_or(ActorError::InvalidState)?;
            todo.push(s.next);
            match s.action {
                ActorAction::Probe(ProbeAction::Enter(next)) => todo.push(next),
                ActorAction::Probe(ProbeAction::SpawnTarget(ty)) => {
                    children.insert(ty);
                }
                _ => {}
            }
        }
        Ok(children.into_iter().collect())
    }
    /// Check source behavior admission and map-wide dimension fit. This rejects
    /// unsupported AI/combat/pickup paths before spawn/RNG or callback execution.
    pub fn validate_type(
        &self,
        ty: ActorTypeId,
        sectors: &[SectorState],
    ) -> Result<(), ActorError> {
        self.admit_type(
            ty,
            sectors,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut 0,
        )
    }
    fn admit_type(
        &self,
        ty: ActorTypeId,
        sectors: &[SectorState],
        path: &mut BTreeSet<ActorTypeId>,
        done: &mut BTreeSet<ActorTypeId>,
        work: &mut usize,
    ) -> Result<(), ActorError> {
        if done.contains(&ty) {
            return Ok(());
        }
        *work += 1;
        if *work > 32768 {
            return Err(ActorError::WorkLimit);
        }
        if !path.insert(ty) {
            return Err(ActorError::SpawnCycle);
        }
        let t = self.actor_type(ty).ok_or(ActorError::InvalidType)?;
        if t.flags() & !ALLOWED_FLAGS != 0 {
            return Err(ActorError::UnsupportedFlags(t.flags() & !ALLOWED_FLAGS));
        }
        if t.radius() < Fixed::ZERO
            || t.radius() > Fixed::from_map_units(32)
            || t.height() <= Fixed::ZERO
            || sectors.iter().any(|s| t.height() > s.ceiling - s.floor)
        {
            return Err(ActorError::Dimensions);
        }
        for root in [t.states()[0], t.states()[5]] {
            self.validate_state(root)?;
            let mut seen = BTreeSet::new();
            let mut todo = vec![root];
            while let Some(id) = todo.pop() {
                if id.0 == 0 || !seen.insert(id) {
                    continue;
                }
                *work += 1;
                if *work > 32768 {
                    return Err(ActorError::WorkLimit);
                }
                let s = self.state(id).ok_or(ActorError::InvalidState)?;
                todo.push(s.next);
                match s.action {
                    ActorAction::Probe(ProbeAction::Enter(next)) => todo.push(next),
                    ActorAction::Probe(ProbeAction::SpawnTarget(child)) => {
                        self.admit_type(child, sectors, path, done, work)?
                    }
                    _ => {}
                }
            }
        }
        path.remove(&ty);
        done.insert(ty);
        Ok(())
    }
    pub(crate) fn demand(&self, state: StateId, tics: i32) -> Result<Demand, ActorError> {
        let mut d = Demand::default();
        self.demand_entry(state, tics, &mut d, &mut BTreeSet::new())?;
        Ok(d)
    }
    fn demand_entry(
        &self,
        state: StateId,
        mut tics: i32,
        d: &mut Demand,
        path: &mut BTreeSet<ActorTypeId>,
    ) -> Result<(), ActorError> {
        let mut work = 0;
        let mut spawns = Vec::new();
        self.flow(state, &mut tics, &mut work, &mut spawns)?;
        d.work += work;
        for ty in spawns {
            d.spawns += 1;
            if d.spawns > 1024 || d.work > STEP_LIMIT {
                return Err(ActorError::WorkLimit);
            }
            if !path.insert(ty) {
                return Err(ActorError::SpawnCycle);
            }
            let t = self.actor_type(ty).ok_or(ActorError::InvalidType)?;
            let initial = self.state(t.states()[0]).ok_or(ActorError::InvalidState)?;
            if initial.tics == 1 {
                self.demand_entry(initial.next, 0, d, path)?;
            }
            path.remove(&ty);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Demand {
    pub spawns: usize,
    pub work: usize,
}
impl Demand {
    pub fn add(&mut self, other: Self) -> Result<(), ActorError> {
        self.spawns += other.spawns;
        self.work += other.work;
        if self.spawns > 1024 || self.work > STEP_LIMIT {
            Err(ActorError::WorkLimit)
        } else {
            Ok(())
        }
    }
}
/// Immutable actor state observation; mutating copies cannot change simulation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActorLife {
    /// Registered type identity.
    pub actor_type: ActorTypeId,
    /// Current state; None after explicit null-state entry. Some(StateId(0))
    /// remains distinct for direct source spawn-state assignment.
    pub state: Option<StateId>,
    /// Current state duration/countdown.
    pub tics: i32,
    /// Inert sprite ID.
    pub sprite: u16,
    /// Inert frame/fullbright bits.
    pub frame: u32,
    /// Source initialization health; controlled state death does not apply damage.
    pub health: i32,
    /// Initialized reaction time; no unsupported AI timer service.
    pub reaction_time: i32,
    /// Full admitted source flags.
    pub flags: u32,
    /// Validated assigned identity, retained even after target retirement.
    pub target: Option<ActorId>,
    /// Controlled death-state request marker, not a P_KillMobj/corpse flag.
    pub death_requested: bool,
    /// Source spawnpoint for map THINGS, or None for parameterized typed spawn.
    pub spawnpoint: Option<doom_map::Thing>,
}
/// State-machine observations, ordered relative to existing motion/lifecycle events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActorStateEvent {
    /// Direct source spawn-state assignment (no callback executed).
    Initialized {
        /// Actor identity.
        actor: ActorId,
        /// Source type.
        actor_type: ActorTypeId,
    },
    /// State/tics/sprite/frame assigned before any action; includes null entry.
    Entered {
        /// Actor identity.
        actor: ActorId,
        /// Entered state.
        state: StateId,
    },
    /// Supported action dispatched, before side effects.
    Action {
        /// Actor identity.
        actor: ActorId,
        /// Explicit original/probe identity.
        action: ActorAction,
    },
    /// Inert source sound request, with optional global origin.
    Sound {
        /// None means source boss-global origin.
        actor: Option<ActorId>,
        /// Source sound identifier.
        sound: u16,
    },
    /// Validated target assignment; stale identities are not auto-cleared.
    Target {
        /// Owning actor.
        actor: ActorId,
        /// Assigned identity.
        target: Option<ActorId>,
    },
    /// Explicit death-state request, distinct from damage/kill semantics.
    Death(ActorId),
    /// Synthetic probe observed an unavailable target; no stale access occurred.
    TargetUnavailable(ActorId),
    /// Implicit action gameplay lookup, separate from explicit external RNG calls.
    Random {
        /// Requesting actor.
        actor: ActorId,
        /// Source lookup result.
        value: u8,
    },
}

pub(crate) trait ActorAccess {
    fn get(&self, id: ActorId) -> Option<&Actor>;
    fn get_mut(&mut self, id: ActorId) -> Option<&mut Actor>;
    fn spawn(&mut self, actor: Actor) -> Result<ActorId, StoreError>;
    fn mark(&mut self, id: ActorId) -> Result<bool, StoreError>;
}
impl ActorAccess for &mut ThinkerStore<Actor> {
    fn get(&self, id: ActorId) -> Option<&Actor> {
        ThinkerStore::get(self, id)
    }
    fn get_mut(&mut self, id: ActorId) -> Option<&mut Actor> {
        ThinkerStore::get_mut(self, id)
    }
    fn spawn(&mut self, a: Actor) -> Result<ActorId, StoreError> {
        ThinkerStore::spawn(self, a)
    }
    fn mark(&mut self, id: ActorId) -> Result<bool, StoreError> {
        ThinkerStore::mark_remove(self, id)
    }
}
impl ActorAccess for &mut ThinkerContext<'_, Actor> {
    fn get(&self, id: ActorId) -> Option<&Actor> {
        ThinkerContext::get(self, id)
    }
    fn get_mut(&mut self, id: ActorId) -> Option<&mut Actor> {
        ThinkerContext::get_mut(self, id)
    }
    fn spawn(&mut self, a: Actor) -> Result<ActorId, StoreError> {
        ThinkerContext::spawn(self, a)
    }
    fn mark(&mut self, id: ActorId) -> Result<bool, StoreError> {
        ThinkerContext::mark_remove(self, id)
    }
}
pub(crate) fn make_actor(
    world: &StaticWorld,
    sectors: &[SectorState],
    registry: &ActorRegistry,
    options: ActorOptions,
    ty: ActorTypeId,
    position: [Fixed; 2],
    height: SpawnHeight,
) -> Actor {
    let info = registry.actor_type(ty).expect("admitted type");
    let initial = registry.state(info.states()[0]).expect("validated state");
    let flags = info.flags();
    let location = world.locate_validated(position);
    let sector = sectors[usize::from(location.sector)];
    let z = match height {
        SpawnHeight::Floor => sector.floor,
        SpawnHeight::Ceiling => sector.ceiling - info.height(),
        SpawnHeight::Absolute(z) => z,
    };
    Actor {
        spec: ActorSpec {
            position,
            radius: info.radius(),
            height: info.height(),
            ..Default::default()
        },
        floor: sector.floor,
        ceiling: sector.ceiling,
        z,
        lastlook: 0,
        solidity: if flags & 2 != 0 {
            Solidity::Solid
        } else {
            Solidity::NonSolid
        },
        momz: Fixed::ZERO,
        gravity: if flags & 512 != 0 {
            Gravity::None
        } else {
            Gravity::Normal
        },
        allow_dropoff: flags & 1024 != 0,
        location: Some(location),
        life: Some(ActorLife {
            actor_type: ty,
            state: Some(info.states()[0]),
            tics: initial.tics,
            sprite: initial.sprite,
            frame: initial.frame,
            health: info.spawn_health(),
            reaction_time: if options.skill == Skill::Nightmare {
                0
            } else {
                info.reaction_time()
            },
            flags,
            target: None,
            death_requested: false,
            spawnpoint: None,
        }),
    }
}
pub(crate) struct LifeHost<'a, A> {
    pub access: A,
    pub registry: &'a ActorRegistry,
    pub world: &'a StaticWorld,
    pub sectors: &'a [SectorState],
    pub options: ActorOptions,
    pub rng: &'a mut DoomRng,
    pub spatial: &'a mut crate::movement::SpatialIndex,
    pub bindings: &'a mut [Option<ActorId>; 4],
    pub events: &'a mut Vec<crate::Event>,
}
impl<A: ActorAccess> LifeHost<'_, A> {
    fn observe(&mut self, event: ActorStateEvent) {
        self.events.push(crate::Event::ActorState(event));
    }
    pub fn spawn(&mut self, ty: ActorTypeId, position: [Fixed; 2], height: SpawnHeight) -> ActorId {
        let actor = make_actor(
            self.world,
            self.sectors,
            self.registry,
            self.options,
            ty,
            position,
            height,
        );
        let id = self
            .access
            .spawn(actor)
            .expect("preflight reserved actor capacity");
        self.access.get_mut(id).expect("new typed actor").lastlook = self.rng.p_random() % 4;
        self.spatial.link(id);
        self.events.push(crate::Event::Spawned(id));
        self.observe(ActorStateEvent::Initialized {
            actor: id,
            actor_type: ty,
        });
        id
    }
    pub fn mark(&mut self, id: ActorId) {
        if self.access.mark(id).expect("validated callback identity") {
            self.spatial.unlink(id);
            for binding in &mut *self.bindings {
                if *binding == Some(id) {
                    *binding = None;
                }
            }
            self.events.push(crate::Event::RemovalMarked(id));
        }
    }
    pub fn target(&mut self, id: ActorId, target: Option<ActorId>) {
        self.access
            .get_mut(id)
            .expect("live reference owner")
            .life
            .as_mut()
            .expect("typed actor")
            .target = target;
        self.observe(ActorStateEvent::Target { actor: id, target });
    }
    fn draw(&mut self, id: ActorId) -> u8 {
        let value = self.rng.p_random();
        self.observe(ActorStateEvent::Random { actor: id, value });
        value
    }
    fn sound(&mut self, actor: Option<ActorId>, sound: u16) {
        self.observe(ActorStateEvent::Sound { actor, sound });
    }
    pub fn enter(&mut self, id: ActorId, state: StateId) {
        self.enter_inner(id, state, &mut 0);
    }
    fn enter_inner(&mut self, id: ActorId, mut state: StateId, work: &mut usize) {
        loop {
            *work += 1;
            assert!(*work <= ENTRY_LIMIT, "validated state-entry bound");
            self.observe(ActorStateEvent::Entered { actor: id, state });
            if state.0 == 0 {
                self.access
                    .get_mut(id)
                    .expect("pending storage remains readable")
                    .life
                    .as_mut()
                    .unwrap()
                    .state = None;
                self.mark(id);
                return;
            }
            let definition = self.registry.state(state).expect("validated state");
            let (tics, sprite, frame, action, next) = (
                definition.tics,
                definition.sprite,
                definition.frame,
                definition.action,
                definition.next,
            );
            {
                let life = self
                    .access
                    .get_mut(id)
                    .expect("typed actor")
                    .life
                    .as_mut()
                    .unwrap();
                life.state = Some(state);
                life.tics = tics;
                life.sprite = sprite;
                life.frame = frame;
            }
            if action != ActorAction::None {
                self.observe(ActorStateEvent::Action { actor: id, action });
                self.action(id, action, work);
            }
            if self
                .access
                .get(id)
                .expect("deferred storage")
                .life
                .unwrap()
                .tics
                != 0
            {
                return;
            }
            state = next;
        }
    }
    fn action(&mut self, id: ActorId, action: ActorAction, work: &mut usize) {
        match action {
            ActorAction::None => {}
            ActorAction::Original(action) => {
                let actor = *self.access.get(id).expect("typed actor");
                let life = actor.life.unwrap();
                let info = self.registry.actor_type(life.actor_type).unwrap();
                let sound_id = |name: &str| {
                    SOURCE_SOUNDS
                        .iter()
                        .position(|s| *s == name)
                        .expect("original sound") as u16
                };
                match self.registry.action_name(action).expect("original action") {
                    "A_Fall" => {
                        let actor = self.access.get_mut(id).unwrap();
                        actor.life.as_mut().unwrap().flags &= !2;
                        actor.solidity = Solidity::NonSolid;
                    }
                    "A_Pain" => {
                        let sound = info.sounds()[2];
                        if sound != 0 {
                            self.sound(Some(id), sound);
                        }
                    }
                    "A_XScream" => self.sound(Some(id), sound_id("sfx_slop")),
                    "A_PlayerScream" => self.sound(
                        Some(id),
                        sound_id(if life.health < -50 {
                            "sfx_pdiehi"
                        } else {
                            "sfx_pldeth"
                        }),
                    ),
                    "A_Scream" => {
                        let sound = info.sounds()[3];
                        if sound == 0 {
                            return;
                        }
                        let global = SOURCE_TYPES
                            .get(usize::from(life.actor_type.0))
                            .is_some_and(|(name, _)| matches!(*name, "MT_SPIDER" | "MT_CYBORG"));
                        let pod = sound_id("sfx_podth1");
                        let bg = sound_id("sfx_bgdth1");
                        let chosen = if (pod..=pod + 2).contains(&sound) {
                            pod + u16::from(self.draw(id) % 3)
                        } else if (bg..=bg + 1).contains(&sound) {
                            bg + u16::from(self.draw(id) % 2)
                        } else {
                            sound
                        };
                        self.sound(if global { None } else { Some(id) }, chosen);
                    }
                    _ => unreachable!("unsupported action rejected before mutation"),
                }
            }
            ActorAction::Probe(probe) => match probe {
                ProbeAction::Random => {
                    self.draw(id);
                }
                ProbeAction::Enter(state) => self.enter_inner(id, state, work),
                ProbeAction::RemoveSelf => self.mark(id),
                ProbeAction::TargetSelf => self.target(id, Some(id)),
                ProbeAction::RemoveTarget => {
                    let target = self.access.get(id).unwrap().life.unwrap().target;
                    if let Some(target) = target.filter(|target| self.access.get(*target).is_some())
                    {
                        self.mark(target);
                    } else {
                        self.observe(ActorStateEvent::TargetUnavailable(id));
                    }
                }
                ProbeAction::SpawnTarget(ty) => {
                    let actor = *self.access.get(id).unwrap();
                    let child = self.spawn(ty, actor.spec.position, SpawnHeight::Absolute(actor.z));
                    self.target(id, Some(child));
                    self.target(child, Some(id));
                }
            },
        }
    }
    pub fn tick(&mut self, id: ActorId) {
        let life = self
            .access
            .get_mut(id)
            .expect("live thinker")
            .life
            .as_mut()
            .expect("typed actor");
        if life.tics == -1 {
            return;
        }
        life.tics -= 1;
        if life.tics == 0 {
            let next = self
                .registry
                .state(life.state.expect("nonremoved state"))
                .unwrap()
                .next;
            self.enter(id, next);
        }
    }
}
pub(crate) fn blocks_cell_index(actor: &Actor) -> bool {
    actor.life.is_none_or(|life| life.flags & 16 == 0)
}
pub(crate) fn in_sector_list(actor: &Actor) -> bool {
    actor.life.is_none_or(|life| life.flags & 8 == 0)
}
