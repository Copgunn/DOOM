//! Original definitions and explicit execution roles; no native fixture aliases.
use crate::{identity::*, *};
use std::{error::Error, fmt, sync::OnceLock};

/// Immutable original information package identified by DSIM-v21.
pub const ORIGINAL_REGISTRY_SCHEMA: &[u8] = b"DOOM-INFO-1.10-v1";
/// Explicit synthetic context, separate from original actor identities.
pub const ORIGINAL_CONTEXT_SCHEMA: &[u8] = b"UNBOUND-BODY-v1";

/// Execution policy identifier, independent of diagnostic format version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OriginalExecutionProfile {
    /// Existing normal Demon closure with bounded unbound bodies and effects.
    DemonNormalV1,
    /// One genuine Medium/commercial player body; no input/inflictor/roster execution.
    PlayerBodyLifecycleV1,
}
impl OriginalExecutionProfile {
    /// Stable wire policy ID; not the Rust Profile discriminant or DSIM version.
    pub const fn wire_id(self) -> u32 {
        match self {
            Self::DemonNormalV1 => 1,
            Self::PlayerBodyLifecycleV1 => 2,
        }
    }
}
/// A non-source definition supplied by the explicit execution context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextRole {
    /// Unbound damage/perception body; never an original player identity.
    UnboundBody,
}
/// Definition identity. An actor's generational handle is a separate namespace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionIdentity {
    /// Original mobjtype_t, with pristine metadata at its original ordinal.
    Original(OriginalActorType),
    /// An explicitly identified context definition, not a canonical count sentinel.
    Context(ContextRole),
}
/// Registry/role admission failed; recognition alone never grants execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OriginalRegistryError {
    /// Complete source or context records differ from their identified package.
    Registry,
    /// Candidate is neither an original nor a declared context definition slot.
    Definition(u16),
    /// A valid definition has no executable role in this profile.
    Unsupported(DefinitionIdentity),
    /// State is outside the definition's admitted normal closure.
    State(u16),
    /// Unsupported original graph/action or policy configuration.
    Graph,
    /// Operation requires the explicit original-ID profile or a current actor.
    ProfileOrActor,
    /// Existing shared simulation rejected the operation.
    Runtime(SimError),
}
impl fmt::Display for OriginalRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "original registry: {self:?}")
    }
}
impl Error for OriginalRegistryError {}
impl From<SimError> for OriginalRegistryError {
    fn from(value: SimError) -> Self {
        Self::Runtime(value)
    }
}

#[derive(Clone, Copy)]
struct StateSet([u64; 16]);
impl StateSet {
    const fn empty() -> Self {
        Self([0; 16])
    }
    fn contains(self, state: OriginalState) -> bool {
        let n = usize::from(state.ordinal());
        self.0
            .get(n / 64)
            .is_some_and(|word| word & (1_u64 << (n % 64)) != 0)
    }
    fn insert(&mut self, state: OriginalState) -> Result<(), OriginalRegistryError> {
        let n = usize::from(state.ordinal());
        let word = self.0.get_mut(n / 64).ok_or(OriginalRegistryError::Graph)?;
        *word |= 1_u64 << (n % 64);
        Ok(())
    }
    fn graph(
        roots: impl IntoIterator<Item = OriginalState>,
    ) -> Result<Self, OriginalRegistryError> {
        let mut set = Self::empty();
        for mut current in roots {
            let mut work = 0;
            while current != OriginalState::S_NULL && !set.contains(current) {
                work += 1;
                if work > usize::from(OriginalState::COUNT) {
                    return Err(OriginalRegistryError::Graph);
                }
                set.insert(current)?;
                current = current.descriptor().next;
            }
        }
        Ok(set)
    }
}
struct ActorPolicy {
    normal: StateSet,
    direct: StateSet,
    death: StateSet,
    before_fall: StateSet,
}
impl ActorPolicy {
    fn source(ty: OriginalActorType) -> Result<Self, OriginalRegistryError> {
        let r = ty.states();
        let normal = StateSet::graph([
            r.spawn,
            r.see,
            r.pain,
            r.melee,
            r.missile,
            r.death,
            r.extreme_death,
        ])?;
        let direct = StateSet::graph([r.spawn, r.see, r.melee])?;
        let death = StateSet::graph([r.death, r.extreme_death])?;
        let mut before_fall = StateSet::empty();
        let mut current = r.death;
        for _ in 0..OriginalState::COUNT {
            if current == OriginalState::S_NULL || current.descriptor().action == Some("A_Fall") {
                break;
            }
            if before_fall.contains(current) {
                return Err(OriginalRegistryError::Graph);
            }
            before_fall.insert(current)?;
            current = current.descriptor().next;
        }
        Ok(Self {
            normal,
            direct,
            death,
            before_fall,
        })
    }
}
fn demon_policy() -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
    POLICY
        .get_or_init(|| {
            let p = ActorPolicy::source(OriginalActorType::MT_SERGEANT)?;
            for s in OriginalState::all().filter(|s| p.normal.contains(*s)) {
                if !matches!(
                    s.descriptor().action,
                    None | Some(
                        "A_Look"
                            | "A_Chase"
                            | "A_FaceTarget"
                            | "A_SargAttack"
                            | "A_Pain"
                            | "A_Scream"
                            | "A_Fall"
                    )
                ) {
                    return Err(OriginalRegistryError::Graph);
                }
            }
            if p.normal
                .contains(OriginalActorType::MT_SERGEANT.states().raise)
            {
                return Err(OriginalRegistryError::Graph);
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
pub(crate) fn body_slot() -> ActorTypeId {
    ActorTypeId(OriginalActorType::COUNT)
}
pub(crate) fn owner_slot() -> ActorTypeId {
    ActorTypeId(OriginalActorType::MT_SERGEANT.ordinal())
}
fn body_properties() -> [i32; 23] {
    let mut properties = *OriginalActorType::MT_PLAYER.properties();
    for (field, value) in [(0, -1), (2, 1000), (8, 0), (16, 16 * 65536), (21, 6)] {
        properties[field] = value;
    }
    properties
}
fn body_definition() -> ActorTypeDef {
    ActorTypeDef {
        name: "CONTEXT_UNBOUND_BODY".into(),
        properties: body_properties(),
    }
}
fn definitions() -> Result<ActorRegistry, OriginalRegistryError> {
    let mut source = ActorRegistry::vanilla();
    source.types.push(body_definition());
    ActorRegistry::new(source.states, source.types).map_err(|_| OriginalRegistryError::Registry)
}
/// Owned immutable source registry and its explicitly selected role contract.
///
/// Every original record is pristine. Context definitions are separate keys,
/// appended after the original portion; this never expands live actor capacity.
#[derive(Clone, Debug)]
pub struct OriginalExecutionRegistry {
    registry: ActorRegistry,
    profile: OriginalExecutionProfile,
}
impl OriginalExecutionRegistry {
    /// Construct the immutable source package and explicitly selected execution policy.
    pub fn new(profile: OriginalExecutionProfile) -> Result<Self, OriginalRegistryError> {
        match profile {
            OriginalExecutionProfile::DemonNormalV1 => {
                demon_policy()?;
            }
            OriginalExecutionProfile::PlayerBodyLifecycleV1 => {
                demon_policy()?;
                player_policy()?;
            }
        }
        Ok(Self {
            registry: definitions()?,
            profile,
        })
    }
    /// Validate all source/context records before accepting externally built data.
    pub fn admit(
        registry: ActorRegistry,
        profile: OriginalExecutionProfile,
    ) -> Result<Self, OriginalRegistryError> {
        Self::validate(&registry, profile)?;
        Ok(Self { registry, profile })
    }
    pub(crate) fn validate(
        registry: &ActorRegistry,
        profile: OriginalExecutionProfile,
    ) -> Result<(), OriginalRegistryError> {
        match profile {
            OriginalExecutionProfile::DemonNormalV1 => {
                demon_policy()?;
            }
            OriginalExecutionProfile::PlayerBodyLifecycleV1 => {
                demon_policy()?;
                player_policy()?;
            }
        }
        if registry.states().len() != usize::from(OriginalState::COUNT)
            || registry.types().len() != usize::from(OriginalActorType::COUNT) + 1
            || OriginalActorType::all().any(|ty| ty.resolve(registry).is_err())
            || OriginalState::all().any(|state| state.resolve(registry).is_err())
        {
            return Err(OriginalRegistryError::Registry);
        }
        let body = registry
            .actor_type(body_slot())
            .ok_or(OriginalRegistryError::Registry)?;
        if body.name != "CONTEXT_UNBOUND_BODY" || body.properties != body_properties() {
            return Err(OriginalRegistryError::Registry);
        }
        Ok(())
    }
    /// Immutable definitions. Metadata lookup does not authorize simulation behavior.
    pub fn registry(&self) -> &ActorRegistry {
        &self.registry
    }
    /// Resolve a typed definition key without native actor/state projection.
    pub fn resolve(&self, identity: DefinitionIdentity) -> ActorTypeId {
        match identity {
            DefinitionIdentity::Original(ty) => ActorTypeId(ty.ordinal()),
            DefinitionIdentity::Context(ContextRole::UnboundBody) => body_slot(),
        }
    }
    /// Identify a registry slot, keeping original count sentinels separate from context.
    pub fn identity(&self, slot: ActorTypeId) -> Result<DefinitionIdentity, OriginalRegistryError> {
        identity(slot)
    }
    /// Check executable capability independently of metadata recognition.
    pub fn admit_definition(
        &self,
        identity: DefinitionIdentity,
    ) -> Result<ActorTypeId, OriginalRegistryError> {
        let slot = self.resolve(identity);
        require_role_for(self.profile, slot)?;
        Ok(slot)
    }
    /// Check normal state closure; raise metadata never gains execution permission.
    pub fn admit_state(
        &self,
        identity: DefinitionIdentity,
        state: OriginalState,
    ) -> Result<StateId, OriginalRegistryError> {
        let slot = self.admit_definition(identity)?;
        state_role_for(self.profile, slot, StateId(state.ordinal()))?;
        Ok(StateId(state.ordinal()))
    }
    pub(crate) fn into_registry(self) -> ActorRegistry {
        self.registry
    }
}
pub(crate) fn identity(slot: ActorTypeId) -> Result<DefinitionIdentity, OriginalRegistryError> {
    if slot == body_slot() {
        return Ok(DefinitionIdentity::Context(ContextRole::UnboundBody));
    }
    OriginalActorType::try_from(slot.0)
        .map(DefinitionIdentity::Original)
        .map_err(|_| OriginalRegistryError::Definition(slot.0))
}
pub(crate) fn require_role_for(
    profile: OriginalExecutionProfile,
    slot: ActorTypeId,
) -> Result<(), OriginalRegistryError> {
    if profile == OriginalExecutionProfile::PlayerBodyLifecycleV1 {
        return match identity(slot)? {
            DefinitionIdentity::Original(
                OriginalActorType::MT_PLAYER
                | OriginalActorType::MT_SERGEANT
                | OriginalActorType::MT_PUFF
                | OriginalActorType::MT_BLOOD,
            ) => Ok(()),
            other => Err(OriginalRegistryError::Unsupported(other)),
        };
    }
    match identity(slot)? {
        DefinitionIdentity::Original(
            OriginalActorType::MT_SERGEANT
            | OriginalActorType::MT_PUFF
            | OriginalActorType::MT_BLOOD,
        )
        | DefinitionIdentity::Context(ContextRole::UnboundBody) => Ok(()),
        other => Err(OriginalRegistryError::Unsupported(other)),
    }
}
pub(crate) fn state_role_for(
    profile: OriginalExecutionProfile,
    ty: ActorTypeId,
    state: StateId,
) -> Result<(), OriginalRegistryError> {
    require_role_for(profile, ty)?;
    let s = OriginalState::try_from(state.0).map_err(|_| OriginalRegistryError::State(state.0))?;
    let allowed = if ty.0 == OriginalActorType::MT_PLAYER.ordinal() {
        player_policy()?.normal.contains(s)
    } else if ty == owner_slot() {
        demon_policy()?.normal.contains(s)
    } else {
        let source = if ty == body_slot() {
            OriginalActorType::MT_PLAYER
        } else {
            OriginalActorType::try_from(ty.0)
                .map_err(|_| OriginalRegistryError::Definition(ty.0))?
        };
        // Derived once; no graph traversal or allocation on a state-entry read.
        static BODY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
        static PUFF: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
        static BLOOD: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
        let cache = match source {
            OriginalActorType::MT_PLAYER => &BODY,
            OriginalActorType::MT_PUFF => &PUFF,
            OriginalActorType::MT_BLOOD => &BLOOD,
            _ => return Err(OriginalRegistryError::Graph),
        };
        cache
            .get_or_init(|| ActorPolicy::source(source))
            .as_ref()
            .map_err(|e| *e)?
            .normal
            .contains(s)
    };
    if allowed {
        Ok(())
    } else {
        Err(OriginalRegistryError::State(state.0))
    }
}
pub(crate) fn direct(state: StateId) -> bool {
    OriginalState::try_from(state.0)
        .ok()
        .is_some_and(|s| demon_policy().is_ok_and(|p| p.direct.contains(s)))
}
pub(crate) fn death_phase(state: StateId) -> Result<(bool, bool), OriginalRegistryError> {
    let s = OriginalState::try_from(state.0).map_err(|_| OriginalRegistryError::State(state.0))?;
    let p = demon_policy()?;
    Ok((p.death.contains(s), p.before_fall.contains(s)))
}
impl Simulation {
    /// Opt-in original-ID policy using shared services. The selected registry policy
    /// determines Demon-only or bounded genuine-player execution.
    pub fn with_original_registry(
        map: Map,
        limit: u32,
        registry: OriginalExecutionRegistry,
        options: ActorOptions,
        context: MapContextOptions,
    ) -> Result<Self, SimError> {
        let policy = registry.profile;
        if policy == OriginalExecutionProfile::PlayerBodyLifecycleV1
            && (options != ActorOptions::default()
                || map.sectors().len() != 1
                || map.sectors().iter().any(|s| s.heights != [0, 128]))
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        let mut sim = Self::with_demon_expanded_profile(
            map,
            limit,
            registry.into_registry(),
            options,
            context,
            Profile::OriginalDemon,
        )?;
        if policy == OriginalExecutionProfile::PlayerBodyLifecycleV1 {
            sim.original_player = Some(OriginalPlayer::default());
        }
        Ok(sim)
    }
    /// Construct the identified original package and bounded Demon context.
    pub fn original_id_map(
        map: Map,
        limit: u32,
        options: ActorOptions,
        context: MapContextOptions,
    ) -> Result<Self, OriginalRegistryError> {
        let registry = OriginalExecutionRegistry::new(OriginalExecutionProfile::DemonNormalV1)?;
        Self::with_original_registry(map, limit, registry, options, context).map_err(Into::into)
    }
    /// Spawn the already admitted source actor through the existing atomic THING batch.
    pub fn spawn_original_actor(
        &mut self,
        ty: OriginalActorType,
    ) -> Result<MapSpawnReport, OriginalRegistryError> {
        self.require_original_profile()?;
        if ty != OriginalActorType::MT_SERGEANT {
            return Err(OriginalRegistryError::Unsupported(
                DefinitionIdentity::Original(ty),
            ));
        }
        self.spawn_map_things().map_err(Into::into)
    }
    /// Spawn the explicit non-player context body through ordinary bounded ownership.
    pub fn spawn_context_body(
        &mut self,
        position: [Fixed; 2],
        height: SpawnHeight,
    ) -> Result<ActorId, OriginalRegistryError> {
        self.require_original_profile()?;
        self.spawn_type(body_slot(), position, height)
            .map_err(Into::into)
    }
    /// Enter a source state; current role/health and damage-controlled guards still apply.
    pub fn enter_original_state(
        &mut self,
        actor: ActorId,
        state: OriginalState,
    ) -> Result<(), OriginalRegistryError> {
        self.require_original_profile()?;
        self.enter_actor_state(actor, StateId(state.ordinal()))
            .map_err(Into::into)
    }
    /// Observe actual definition/state storage, with context keys kept distinct.
    pub fn original_actor_identity(
        &self,
        actor: ActorId,
    ) -> Result<(DefinitionIdentity, Option<OriginalState>), OriginalRegistryError> {
        self.require_original_profile()?;
        let life = self
            .actor(actor)
            .and_then(Actor::life)
            .ok_or(OriginalRegistryError::ProfileOrActor)?;
        require_role_for(self.player_policy(), life.actor_type)?;
        if let Some(s) = life.state {
            state_role_for(self.player_policy(), life.actor_type, s)?;
        }
        Ok((
            identity(life.actor_type)?,
            life.state
                .map(|s| {
                    OriginalState::try_from(s.0).map_err(|_| OriginalRegistryError::State(s.0))
                })
                .transpose()?,
        ))
    }
    pub(crate) fn with_original_actor_registry(
        map: Map,
        limit: u32,
        registry: ActorRegistry,
        options: ActorOptions,
        context: MapContextOptions,
    ) -> Result<Self, SimError> {
        let registry =
            OriginalExecutionRegistry::admit(registry, OriginalExecutionProfile::DemonNormalV1)
                .map_err(|_| MonsterError::Definition)?;
        Self::with_original_registry(map, limit, registry, options, context)
    }
    fn require_original_profile(&self) -> Result<(), OriginalRegistryError> {
        if self.profile == Profile::OriginalDemon {
            Ok(())
        } else {
            Err(OriginalRegistryError::ProfileOrActor)
        }
    }
}

fn player_policy() -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICY: OnceLock<Result<ActorPolicy, OriginalRegistryError>> = OnceLock::new();
    POLICY
        .get_or_init(|| {
            let roots = OriginalActorType::MT_PLAYER.states();
            let mut p = ActorPolicy::source(OriginalActorType::MT_PLAYER)?;
            p.normal = StateSet::graph([
                roots.spawn,
                roots.see,
                roots.pain,
                roots.death,
                roots.extreme_death,
            ])?;
            for s in OriginalState::all().filter(|s| p.normal.contains(*s)) {
                if !matches!(
                    s.descriptor().action,
                    None | Some("A_Pain" | "A_PlayerScream" | "A_XScream" | "A_Fall")
                ) {
                    return Err(OriginalRegistryError::Graph);
                }
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
