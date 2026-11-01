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
    /// One grounded original player with bounded physical thrust and axis-wall motion.
    PlayerBodyKnockbackV1,
    /// One original player's bounded commands and source reaction-time gate.
    PlayerCommandMovementV1,
    /// Fixed active player0 and one canonical Demon: visual acquisition/nonmelee chase.
    PlayerDemonAcquisitionV1,
    /// The same fixed roster with source-timed autonomous Demon physical melee.
    PlayerDemonMeleeV1,
    /// The fixed combat roster with genuine source pistol/psprite execution.
    PlayerDemonPistolV1,
    /// Four source weapons sharing the genuine player and autonomous Demon.
    PlayerDemonWeaponsV1,
    /// World pickups composed with the existing player/four-weapon/Demon closure.
    PlayerDemonPickupsV1,
    /// Original animated armor and source protection in the shared combat world.
    PlayerDemonArmorV1,
    /// Original animated bonuses and counted items in the shared player world.
    PlayerDemonBonusesV1,
    /// The bonus world plus original former humans, a bounded mixed roster,
    /// source infighting and P_KillMobj drops.
    PlayerMonstersV1,
}
impl OriginalExecutionProfile {
    /// Stable wire policy ID; not the Rust Profile discriminant or DSIM version.
    pub const fn wire_id(self) -> u32 {
        match self {
            Self::DemonNormalV1 => 1,
            Self::PlayerBodyLifecycleV1 => 2,
            Self::PlayerBodyKnockbackV1 => 3,
            Self::PlayerCommandMovementV1 => 4,
            Self::PlayerDemonAcquisitionV1 => 5,
            Self::PlayerDemonMeleeV1 => 6,
            Self::PlayerDemonPistolV1 => 7,
            Self::PlayerDemonWeaponsV1 => 8,
            Self::PlayerDemonPickupsV1 => 9,
            Self::PlayerDemonArmorV1 => 10,
            Self::PlayerDemonBonusesV1 => 11,
            Self::PlayerMonstersV1 => 12,
        }
    }
    /// Exact immutable execution-context schema; not an actor-instance identity.
    pub const fn context_schema(self) -> &'static [u8] {
        match self {
            Self::DemonNormalV1 => ORIGINAL_CONTEXT_SCHEMA,
            Self::PlayerBodyLifecycleV1 => PLAYER_BODY_CONTEXT_SCHEMA,
            Self::PlayerBodyKnockbackV1 => PLAYER_KNOCKBACK_CONTEXT_SCHEMA,
            Self::PlayerCommandMovementV1 => PLAYER_COMMAND_CONTEXT_SCHEMA,
            Self::PlayerDemonAcquisitionV1 => PLAYER_DEMON_CONTEXT_SCHEMA,
            Self::PlayerDemonMeleeV1 => PLAYER_DEMON_MELEE_CONTEXT_SCHEMA,
            Self::PlayerDemonPistolV1 => PLAYER_DEMON_PISTOL_CONTEXT_SCHEMA,
            Self::PlayerDemonWeaponsV1 => PLAYER_DEMON_WEAPONS_CONTEXT_SCHEMA,
            Self::PlayerDemonPickupsV1 => PLAYER_DEMON_PICKUPS_CONTEXT_SCHEMA,
            Self::PlayerDemonArmorV1 => PLAYER_DEMON_ARMOR_CONTEXT_SCHEMA,
            Self::PlayerDemonBonusesV1 => PLAYER_DEMON_BONUSES_CONTEXT_SCHEMA,
            Self::PlayerMonstersV1 => PLAYER_MONSTERS_CONTEXT_SCHEMA,
        }
    }
    pub(crate) const fn has_player_armor(self) -> bool {
        matches!(
            self,
            Self::PlayerDemonArmorV1 | Self::PlayerDemonBonusesV1 | Self::PlayerMonstersV1
        )
    }
    pub(crate) const fn has_player_bonuses(self) -> bool {
        matches!(self, Self::PlayerDemonBonusesV1 | Self::PlayerMonstersV1)
    }
    /// Original former humans, a bounded mixed roster, infighting and drops.
    pub(crate) const fn has_monsters(self) -> bool {
        matches!(self, Self::PlayerMonstersV1)
    }
    pub(crate) const fn has_world_pickups(self) -> bool {
        matches!(
            self,
            Self::PlayerDemonPickupsV1
                | Self::PlayerDemonArmorV1
                | Self::PlayerDemonBonusesV1
                | Self::PlayerMonstersV1
        )
    }
    pub(crate) const fn has_player_body(self) -> bool {
        matches!(
            self,
            Self::PlayerBodyLifecycleV1
                | Self::PlayerBodyKnockbackV1
                | Self::PlayerCommandMovementV1
                | Self::PlayerDemonAcquisitionV1
                | Self::PlayerDemonMeleeV1
                | Self::PlayerDemonPistolV1
                | Self::PlayerDemonWeaponsV1
                | Self::PlayerDemonPickupsV1
                | Self::PlayerDemonArmorV1
                | Self::PlayerDemonBonusesV1
                | Self::PlayerMonstersV1
        )
    }
    pub(crate) const fn has_physical_player(self) -> bool {
        matches!(
            self,
            Self::PlayerBodyKnockbackV1
                | Self::PlayerCommandMovementV1
                | Self::PlayerDemonAcquisitionV1
                | Self::PlayerDemonMeleeV1
                | Self::PlayerDemonPistolV1
                | Self::PlayerDemonWeaponsV1
                | Self::PlayerDemonPickupsV1
                | Self::PlayerDemonArmorV1
                | Self::PlayerDemonBonusesV1
                | Self::PlayerMonstersV1
        )
    }
    pub(crate) const fn has_player_weapons(self) -> bool {
        matches!(
            self,
            Self::PlayerDemonPistolV1
                | Self::PlayerDemonWeaponsV1
                | Self::PlayerDemonPickupsV1
                | Self::PlayerDemonArmorV1
                | Self::PlayerDemonBonusesV1
                | Self::PlayerMonstersV1
        )
    }
    pub(crate) const fn has_player_commands(self) -> bool {
        matches!(
            self,
            Self::PlayerCommandMovementV1
                | Self::PlayerDemonAcquisitionV1
                | Self::PlayerDemonMeleeV1
                | Self::PlayerDemonPistolV1
                | Self::PlayerDemonWeaponsV1
                | Self::PlayerDemonPickupsV1
                | Self::PlayerDemonArmorV1
                | Self::PlayerDemonBonusesV1
                | Self::PlayerMonstersV1
        )
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
/// p_mobj.h/info.c former humans admitted only by `PlayerMonstersV1`.
pub(crate) const FORMER_HUMANS: [OriginalActorType; 3] = [
    OriginalActorType::MT_POSSESSED,
    OriginalActorType::MT_SHOTGUY,
    OriginalActorType::MT_CHAINGUY,
];
/// Original attack callbacks reachable from each former human's normal closure.
fn former_human_attacks(ty: OriginalActorType) -> &'static [&'static str] {
    match ty {
        OriginalActorType::MT_POSSESSED => &["A_PosAttack"],
        OriginalActorType::MT_SHOTGUY => &["A_SPosAttack"],
        _ => &["A_CPosAttack", "A_CPosRefire"],
    }
}
/// Normal spawn/see/pain/missile/death/xdeath closure; raise metadata stays inert.
fn former_human_policy(
    ty: OriginalActorType,
) -> Result<&'static ActorPolicy, OriginalRegistryError> {
    static POLICIES: [OnceLock<Result<ActorPolicy, OriginalRegistryError>>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let index = FORMER_HUMANS
        .iter()
        .position(|t| *t == ty)
        .ok_or(OriginalRegistryError::Graph)?;
    POLICIES[index]
        .get_or_init(|| {
            let p = ActorPolicy::source(ty)?;
            for s in OriginalState::all().filter(|s| p.normal.contains(*s)) {
                let action = s.descriptor().action;
                if !(matches!(
                    action,
                    None | Some(
                        "A_Look"
                            | "A_Chase"
                            | "A_FaceTarget"
                            | "A_Pain"
                            | "A_Scream"
                            | "A_XScream"
                            | "A_Fall"
                    )
                ) || action.is_some_and(|a| former_human_attacks(ty).contains(&a)))
                {
                    return Err(OriginalRegistryError::Graph);
                }
            }
            if p.normal.contains(ty.states().raise) {
                return Err(OriginalRegistryError::Graph);
            }
            Ok(p)
        })
        .as_ref()
        .map_err(|e| *e)
}
pub(crate) fn is_former_human(ty: ActorTypeId) -> bool {
    FORMER_HUMANS.iter().any(|t| t.ordinal() == ty.0)
}
/// Autonomous monster definitions executable by the selected player policy.
pub(crate) fn is_policy_monster(policy: OriginalExecutionProfile, ty: ActorTypeId) -> bool {
    ty == owner_slot() || (policy.has_monsters() && is_former_human(ty))
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
            OriginalExecutionProfile::PlayerBodyLifecycleV1
            | OriginalExecutionProfile::PlayerBodyKnockbackV1
            | OriginalExecutionProfile::PlayerCommandMovementV1
            | OriginalExecutionProfile::PlayerDemonAcquisitionV1
            | OriginalExecutionProfile::PlayerDemonMeleeV1
            | OriginalExecutionProfile::PlayerDemonPistolV1
            | OriginalExecutionProfile::PlayerDemonWeaponsV1
            | OriginalExecutionProfile::PlayerDemonPickupsV1
            | OriginalExecutionProfile::PlayerDemonArmorV1
            | OriginalExecutionProfile::PlayerDemonBonusesV1
            | OriginalExecutionProfile::PlayerMonstersV1 => {
                demon_policy()?;
                player_policy()?;
                if profile.has_monsters() {
                    for ty in FORMER_HUMANS {
                        former_human_policy(ty)?;
                    }
                }
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
            OriginalExecutionProfile::PlayerBodyLifecycleV1
            | OriginalExecutionProfile::PlayerBodyKnockbackV1
            | OriginalExecutionProfile::PlayerCommandMovementV1
            | OriginalExecutionProfile::PlayerDemonAcquisitionV1
            | OriginalExecutionProfile::PlayerDemonMeleeV1
            | OriginalExecutionProfile::PlayerDemonPistolV1
            | OriginalExecutionProfile::PlayerDemonWeaponsV1
            | OriginalExecutionProfile::PlayerDemonPickupsV1
            | OriginalExecutionProfile::PlayerDemonArmorV1
            | OriginalExecutionProfile::PlayerDemonBonusesV1
            | OriginalExecutionProfile::PlayerMonstersV1 => {
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
    if world_pickups::kind_for(profile, slot).is_some() {
        return Ok(());
    }
    if profile.has_player_body() {
        return match identity(slot)? {
            DefinitionIdentity::Original(
                OriginalActorType::MT_PLAYER
                | OriginalActorType::MT_SERGEANT
                | OriginalActorType::MT_PUFF
                | OriginalActorType::MT_BLOOD,
            ) => Ok(()),
            DefinitionIdentity::Original(ty)
                if profile.has_monsters() && FORMER_HUMANS.contains(&ty) =>
            {
                Ok(())
            }
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
    let allowed = if world_pickups::kind_for(profile, ty).is_some() {
        world_pickups::state_allowed(ty, s)
    } else if ty.0 == OriginalActorType::MT_PLAYER.ordinal() {
        player_policy()?.normal.contains(s)
            || (profile.has_player_weapons()
                && matches!(s, OriginalState::S_PLAY_ATK1 | OriginalState::S_PLAY_ATK2))
    } else if is_former_human(ty) {
        former_human_policy(
            OriginalActorType::try_from(ty.0).map_err(|_| OriginalRegistryError::Graph)?,
        )?
        .normal
        .contains(s)
    } else if ty == owner_slot() {
        demon_policy()?.normal.contains(s)
            && !(profile == OriginalExecutionProfile::PlayerDemonAcquisitionV1
                && matches!(
                    s,
                    OriginalState::S_SARG_ATK1
                        | OriginalState::S_SARG_ATK2
                        | OriginalState::S_SARG_ATK3
                ))
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
pub(crate) fn direct(ty: ActorTypeId, state: StateId) -> bool {
    OriginalState::try_from(state.0).ok().is_some_and(|s| {
        if is_former_human(ty) {
            OriginalActorType::try_from(ty.0)
                .ok()
                .and_then(|t| former_human_policy(t).ok())
                .is_some_and(|p| p.direct.contains(s))
        } else {
            demon_policy().is_ok_and(|p| p.direct.contains(s))
        }
    })
}
pub(crate) fn death_phase(
    ty: ActorTypeId,
    state: StateId,
) -> Result<(bool, bool), OriginalRegistryError> {
    let s = OriginalState::try_from(state.0).map_err(|_| OriginalRegistryError::State(state.0))?;
    let p = if is_former_human(ty) {
        former_human_policy(
            OriginalActorType::try_from(ty.0).map_err(|_| OriginalRegistryError::Graph)?,
        )?
    } else {
        demon_policy()?
    };
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
        if policy.has_player_body()
            && (options != ActorOptions::default()
                || map.sectors().len() != 1
                || map.sectors().iter().any(|s| s.heights != [0, 128]))
        {
            return Err(PlayerBodyError::Unsupported.into());
        }
        if policy.has_physical_player() {
            original_player::validate_grounded_geometry(&map)?;
        }
        if policy.has_player_weapons() && map.linedefs().len() > 128 {
            return Err(CombatError::InterceptLimit.into());
        }
        let mut sim = Self::with_demon_expanded_profile(
            map,
            limit,
            registry.into_registry(),
            options,
            context,
            Profile::OriginalDemon,
        )?;
        if policy.has_player_body() {
            sim.original_player = Some(original_player::PlayerRuntime {
                source: OriginalPlayer::default(),
                weapon: None,
                armor: policy
                    .has_player_armor()
                    .then_some(PlayerArmorState::default()),
                pickups: policy
                    .has_world_pickups()
                    .then_some(PlayerPickupState::default()),
                policy,
            });
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
    /// Run the existing atomic THING batch for the canonical Demon context.
    /// The pickup policy also admits its declared world items in that same batch;
    /// use `spawn_map_things` directly when setting up that mixed world.
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
