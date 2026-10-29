//! Canonical Demon definition requests through the frozen Profile20 representation.
//!
//! This checked binding is deliberately separate from M3f4's same-record resolver.
//! ActorLife/DSIM retain native slots; canonical IDs name the source definition.
use crate::identity::{OriginalActorType as Type, OriginalState as State};
use crate::*;
use std::{error::Error, fmt};

/// Existing Profile20 owner role, not original mobjtype_t 136.
pub(crate) const RUNTIME_OWNER: ActorTypeId = ActorTypeId(136);
const RUNTIME_NAME: &str = "FIXTURE_DEMON_MAP_CONTEXT_136";
// The historical native layout is explicit policy, not original enum ordering.
pub(crate) const STATES: &[(State, StateId)] = &[
    (State::S_SARG_STND, StateId(1)),
    (State::S_SARG_STND2, StateId(3)),
    (State::S_SARG_RUN1, StateId(2)),
    (State::S_SARG_RUN2, StateId(15)),
    (State::S_SARG_RUN3, StateId(16)),
    (State::S_SARG_RUN4, StateId(17)),
    (State::S_SARG_RUN5, StateId(18)),
    (State::S_SARG_RUN6, StateId(19)),
    (State::S_SARG_RUN7, StateId(20)),
    (State::S_SARG_RUN8, StateId(21)),
    (State::S_SARG_ATK1, StateId(4)),
    (State::S_SARG_ATK2, StateId(5)),
    (State::S_SARG_ATK3, StateId(6)),
    (State::S_SARG_PAIN, StateId(7)),
    (State::S_SARG_PAIN2, StateId(8)),
    (State::S_SARG_DIE1, StateId(9)),
    (State::S_SARG_DIE2, StateId(10)),
    (State::S_SARG_DIE3, StateId(11)),
    (State::S_SARG_DIE4, StateId(12)),
    (State::S_SARG_DIE5, StateId(13)),
    (State::S_SARG_DIE6, StateId(14)),
];

/// A canonical request failed before delegation, or the existing service rejected it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalDemonError {
    /// Canonical execution binding is selected only by Profile20.
    Profile,
    /// Required source or projected runtime metadata differs from its definition.
    Registry,
    /// Only MT_SERGEANT is admitted; sharing SARG states grants no other type.
    UnsupportedType(Type),
    /// This canonical state has no executable binding, including raise and null.
    UnsupportedState(State),
    /// This registry-local candidate is outside the admitted state binding.
    RuntimeState(StateId),
    /// The handle does not identify the current runtime Demon owner record.
    Owner,
    /// Existing simulation admission or execution rejection, with its atomicity contract.
    Runtime(SimError),
}
impl fmt::Display for CanonicalDemonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "canonical Demon: {self:?}")
    }
}
impl Error for CanonicalDemonError {}
impl From<SimError> for CanonicalDemonError {
    fn from(value: SimError) -> Self {
        Self::Runtime(value)
    }
}
fn runtime_state(original: State) -> Result<StateId, CanonicalDemonError> {
    NativeDemonState::from_original(original)
        .map(NativeDemonState::runtime)
        .ok_or(CanonicalDemonError::UnsupportedState(original))
}
fn reference(original: State) -> Result<StateId, CanonicalDemonError> {
    if original == State::S_NULL {
        Ok(StateId(0))
    } else {
        runtime_state(original)
    }
}
fn actor_properties() -> Result<[i32; 23], CanonicalDemonError> {
    let mut p = *Type::MT_SERGEANT.properties();
    // info.h state-reference fields, except the deliberately unadmitted raise root.
    for field in [1, 3, 7, 10, 11, 12, 13] {
        let original = State::try_from(p[field]).map_err(|_| CanonicalDemonError::Registry)?;
        p[field] = i32::from(reference(original)?.0);
    }
    p[0] = -1; // Ordinary THINGS lookup resolves the original source record first.
    p[22] = 0; // Executable resurrection remains excluded; source metadata is intact.
    Ok(p)
}

/// Borrowed proof of the canonical Demon-to-native definition correspondence.
///
/// This validates the actor, null sentinel and all 21 bound source/runtime states.
/// Other registry records and game context retain their separate admission checks.
/// No allocation, mutation or gameplay RNG is involved.
#[derive(Clone, Copy, Debug)]
pub struct DemonExecutionBinding<'a> {
    registry: &'a ActorRegistry,
}
impl<'a> DemonExecutionBinding<'a> {
    /// Validate complete source records and the exact declared runtime projection.
    pub fn admit(registry: &'a ActorRegistry) -> Result<Self, CanonicalDemonError> {
        Type::MT_SERGEANT
            .resolve(registry)
            .map_err(|_| CanonicalDemonError::Registry)?;
        State::S_NULL
            .resolve(registry)
            .map_err(|_| CanonicalDemonError::Registry)?;
        let actor = registry
            .actor_type(RUNTIME_OWNER)
            .ok_or(CanonicalDemonError::Registry)?;
        if actor.name != RUNTIME_NAME || actor.properties != actor_properties()? {
            return Err(CanonicalDemonError::Registry);
        }
        for &(original, runtime) in STATES {
            let source_id = original
                .resolve(registry)
                .map_err(|_| CanonicalDemonError::Registry)?;
            let source = registry
                .state(source_id)
                .ok_or(CanonicalDemonError::Registry)?;
            let projected = registry
                .state(runtime)
                .ok_or(CanonicalDemonError::Registry)?;
            let next = State::try_from(source.next.0).map_err(|_| CanonicalDemonError::Registry)?;
            if projected.name.strip_prefix("FIXTURE_") != original.name().strip_prefix("S_")
                || projected.sprite != source.sprite
                || projected.frame != source.frame
                || projected.tics != source.tics
                || projected.action != source.action
                || projected.next != reference(next)?
                || projected.misc != source.misc
            {
                return Err(CanonicalDemonError::Registry);
            }
        }
        Ok(Self { registry })
    }
    /// Canonical source actor, retaining its original ordinal 12.
    pub const fn original_type(self) -> Type {
        Type::MT_SERGEANT
    }
    /// Resolve an explicit executable source type; Spectre and Shotgun Guy reject.
    pub fn runtime_type(self, original: Type) -> Result<ActorTypeId, CanonicalDemonError> {
        self.native_type(original).map(NativeDemonType::runtime)
    }
    /// Resolve the source type into the explicitly named native namespace.
    pub fn native_type(self, original: Type) -> Result<NativeDemonType, CanonicalDemonError> {
        if original != self.original_type() {
            return Err(CanonicalDemonError::UnsupportedType(original));
        }
        self.check_native_type(RUNTIME_OWNER)
    }
    /// Check a native type candidate, without interpreting it as an original ordinal.
    pub fn check_native_type(
        self,
        candidate: ActorTypeId,
    ) -> Result<NativeDemonType, CanonicalDemonError> {
        NativeDemonType::checked(candidate).ok_or(CanonicalDemonError::Owner)
    }
    /// Resolve a source state into a native representation; entry still needs role checks.
    pub fn native_state(self, original: State) -> Result<NativeDemonState, CanonicalDemonError> {
        NativeDemonState::from_original(original)
            .ok_or(CanonicalDemonError::UnsupportedState(original))
    }
    /// Check a native slot, rejecting canonical SARG ordinals and unrelated slots.
    pub fn check_native_state(
        self,
        candidate: StateId,
    ) -> Result<NativeDemonState, CanonicalDemonError> {
        NativeDemonState::checked(candidate).ok_or(CanonicalDemonError::RuntimeState(candidate))
    }
    /// Runtime definition, visibly distinct from the pristine canonical descriptor.
    pub fn runtime_definition(self) -> &'a ActorTypeDef {
        self.registry
            .actor_type(RUNTIME_OWNER)
            .expect("borrowed validated binding")
    }
    /// The complete bound state correspondence, in original sequence order.
    pub fn states(self) -> impl ExactSizeIterator<Item = (State, StateId)> {
        STATES.iter().copied()
    }
    /// Resolve a canonical state. Existing entry/health/role checks still apply.
    pub fn runtime_state(self, original: State) -> Result<StateId, CanonicalDemonError> {
        self.native_state(original).map(NativeDemonState::runtime)
    }
    /// Observe the source definition behind a bound native state candidate.
    pub fn original_state(self, runtime: StateId) -> Result<State, CanonicalDemonError> {
        self.check_native_state(runtime)
            .map(NativeDemonState::original)
    }
}

/// Construct the existing byte-exact Profile17/20 fixture from canonical records.
pub(crate) fn registry() -> ActorRegistry {
    let source = ActorRegistry::vanilla();
    let mut states = source.states().to_vec();
    for &(original, runtime) in STATES {
        let source_id = original.resolve(&source).expect("original SARG metadata");
        let mut state = source
            .state(source_id)
            .expect("resolved source state")
            .clone();
        state.name = format!("FIXTURE_{}", &original.name()[2..]);
        state.next = reference(State::try_from(state.next.0).expect("original next reference"))
            .expect("closed admitted SARG graph");
        states[usize::from(runtime.0)] = state;
    }
    let mut types = source.types().to_vec();
    Type::MT_SERGEANT
        .resolve(&source)
        .expect("original Demon descriptor");
    types[usize::from(RUNTIME_OWNER.0)] = ActorTypeDef {
        name: RUNTIME_NAME.into(),
        properties: actor_properties().expect("source-defined projection"),
    };
    let mut target = types[0].clone();
    target.name = "FIXTURE_DAMAGE_TARGET".into();
    for (field, value) in [(0, -1), (2, 1000), (8, 0), (16, 16 * 65536), (21, 6)] {
        target.properties[field] = value;
    }
    types[135] = target;
    ActorRegistry::new(states, types).expect("bounded canonical projection")
}

impl Simulation {
    /// Existing Profile20 services, selected with canonical Demon-derived definitions.
    /// No implicit THING batch, real player, Spectre, raise or respawn admission.
    pub fn canonical_demon_map(
        map: Map,
        limit: u32,
        options: ActorOptions,
        context: MapContextOptions,
    ) -> Result<Self, SimError> {
        Self::with_demon_expanded_map_registry(
            map,
            limit,
            ActorRegistry::demon_map_context_fixture(),
            options,
            context,
        )
    }
    /// Obtain the checked definition binding; this does not change canonical state.
    pub fn canonical_demon_binding(
        &self,
    ) -> Result<DemonExecutionBinding<'_>, CanonicalDemonError> {
        if self.profile != Profile::DemonExpandedMap {
            return Err(CanonicalDemonError::Profile);
        }
        DemonExecutionBinding::admit(
            self.registry
                .as_deref()
                .ok_or(CanonicalDemonError::Registry)?,
        )
    }
    /// Execute the existing atomic THING batch for the explicitly requested Demon.
    /// Source filtering and original DoomEd lookup retain their existing ordering.
    pub fn spawn_original_demons(
        &mut self,
        original: Type,
    ) -> Result<MapSpawnReport, CanonicalDemonError> {
        self.canonical_demon_binding()?.runtime_type(original)?;
        self.spawn_map_things().map_err(Into::into)
    }
    /// Enter an original stand/run/attack state through existing owner/lifecycle checks.
    /// Pain/death remain damage-controlled; canonical raise/null are not entry bindings.
    pub fn enter_original_demon_state(
        &mut self,
        actor: ActorId,
        original: State,
    ) -> Result<(), CanonicalDemonError> {
        let runtime = self.canonical_demon_binding()?.runtime_state(original)?;
        if !self
            .actor(actor)
            .and_then(Actor::life)
            .is_some_and(|l| l.actor_type == RUNTIME_OWNER)
        {
            return Err(CanonicalDemonError::Owner);
        }
        self.enter_actor_state(actor, runtime).map_err(Into::into)
    }
    /// Source definition identity of a current owner record, including a removed state.
    /// This is an explicit execution projection; raw ActorLife/DSIM retain native IDs.
    /// Pending records remain observable until retirement; stale handles reject.
    pub fn original_demon_identity(
        &self,
        actor: ActorId,
    ) -> Result<(Type, Option<State>), CanonicalDemonError> {
        let (ty, state) = self.native_demon_identity(actor)?;
        Ok((ty.original(), state.map(NativeDemonState::original)))
    }
    /// Checked view of the raw native owner/state stored by Profile20.
    ///
    /// A value is representation evidence, not permission to enter pain/death,
    /// reuse a stale handle or execute in another registry. No state or RNG changes.
    pub fn native_demon_identity(
        &self,
        actor: ActorId,
    ) -> Result<(NativeDemonType, Option<NativeDemonState>), CanonicalDemonError> {
        let binding = self.canonical_demon_binding()?;
        let life = self
            .actor(actor)
            .and_then(Actor::life)
            .ok_or(CanonicalDemonError::Owner)?;
        Ok((
            binding.check_native_type(life.actor_type)?,
            life.state
                .map(|s| binding.check_native_state(s))
                .transpose()?,
        ))
    }
}
