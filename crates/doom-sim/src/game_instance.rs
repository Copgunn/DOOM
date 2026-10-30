//! Process-local provenance at the boundary, outside deterministic actor state.
use crate::{ActorId, ContextError, ExecutionContext, SimError, Simulation};
use std::{error::Error, fmt, sync::Arc};

struct Incarnation;

/// Opaque identity of one runtime incarnation. Equality means the same allocation,
/// not equal game state. Retained tokens prevent allocation reuse from aliasing an
/// earlier incarnation. No numeric/byte conversion or persistent identity exists.
/// Cloning this token does not clone or retain a Simulation or actor.
#[derive(Clone)]
pub struct InstanceId(Arc<Incarnation>);
impl InstanceId {
    fn fresh() -> Self {
        Self(Arc::new(Incarnation))
    }
}
impl PartialEq for InstanceId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for InstanceId {}
impl fmt::Debug for InstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("InstanceId(<opaque>)")
    }
}

/// Boundary reference candidate, not an actor lease or authenticated message.
/// Checked export supplies provenance; explicit candidate construction is local
/// routing/readmission policy. Every import still checks provenance, generation,
/// current state and the expected definition. Never serialize an InstanceId.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstanceActorRef {
    provenance: Option<InstanceId>,
    local: ActorId,
}
impl InstanceActorRef {
    /// Explicit candidate assembly, without claiming origin or availability.
    /// Missing provenance is representable and rejects on import. Attaching a
    /// known local key is an explicit local interpretation of the raw value; it
    /// cannot recover an erased origin or authenticate untrusted transport data.
    pub fn candidate(provenance: Option<InstanceId>, local: ActorId) -> Self {
        Self { provenance, local }
    }
    /// Process-local provenance, if the boundary supplied it.
    pub fn provenance(&self) -> Option<&InstanceId> {
        self.provenance.as_ref()
    }
    /// Explicitly observe the unqualified local handle. This erases provenance.
    pub const fn local_id(&self) -> ActorId {
        self.local
    }
}

/// Provenance failures are separate from existing actor/role/service failures.
/// Validation order is bound context, present key, matching key, then existing
/// local admission. Rejected import/export never changes authoritative state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvenanceError {
    /// A raw Simulation scope has no owning game-instance provenance.
    UnboundContext,
    /// A candidate carries only a local handle, including decoded diagnostics.
    MissingProvenance,
    /// Another instance/incarnation; includes references predating a restart.
    InstanceMismatch,
    /// Current generation, definition, profile or delegated service rejected.
    Context(ContextError),
}
impl From<ContextError> for ProvenanceError {
    fn from(error: ContextError) -> Self {
        Self::Context(error)
    }
}
impl fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "instance provenance: {self:?}")
    }
}
impl Error for ProvenanceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Context(error) => Some(error),
            _ => None,
        }
    }
}

/// Owns one unchanged Simulation and its boundary identity. Moves preserve that
/// identity. Wrapping cloned/extracted state creates a distinct incarnation.
/// There is no mutable Simulation escape or caller-supplied identity constructor.
/// Existing local APIs remain available through the immutable view/consuming
/// extraction; their raw handles do not gain provenance.
///
/// The owner cannot be cloned together with its identity:
/// ```compile_fail
/// use doom_sim::GameInstance;
/// fn require_clone<T: Clone>() {}
/// require_clone::<GameInstance>();
/// ```
/// Scoped actor tokens still cannot escape:
/// ```compile_fail
/// use doom_sim::{*, identity::OriginalActorType};
/// fn escape(game: &mut GameInstance) {
///     let escaped = game.with_execution_context(|ctx| {
///         Ok(ctx.admit_local(ActorId::from_parts(0, 1),
///             DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT))?)
///     });
/// }
/// ```
pub struct GameInstance {
    simulation: Simulation,
    id: InstanceId,
}
impl GameInstance {
    /// Create a fresh process-local incarnation without changing game state/RNG.
    /// One small Arc allocation has the existing process-level OOM policy.
    /// Wrapping a profile does not grant it executable context admission.
    pub fn new(simulation: Simulation) -> Self {
        Self {
            simulation,
            id: InstanceId::fresh(),
        }
    }
    /// Opaque routing token; not an authentication credential or serialized key.
    pub fn id(&self) -> InstanceId {
        self.id.clone()
    }
    /// Read-only local observations; diagnostics exclude the boundary identity.
    pub fn simulation(&self) -> &Simulation {
        &self.simulation
    }
    /// Consume this owner. Rewrapping the returned state always creates a new
    /// incarnation; old qualified references cannot attach to it automatically.
    pub fn into_simulation(self) -> Simulation {
        self.simulation
    }
    /// Use the existing generative scope and gameplay services. Only the current
    /// original-ID profile admits this scope. Successful earlier calls remain
    /// applied if a later callback call fails; this is not a scope transaction.
    pub fn with_execution_context<R>(
        &mut self,
        operation: impl for<'ctx> FnOnce(&mut ExecutionContext<'ctx>) -> Result<R, ProvenanceError>,
    ) -> Result<R, ProvenanceError> {
        operation(&mut ExecutionContext::new(
            &mut self.simulation,
            Some(&self.id),
        )?)
    }
    /// Existing fresh level setup/spawn with continuing RNG. Only after setup
    /// succeeds are both state and incarnation replaced. Old reference keys stay
    /// distinct even when new local handles have identical numerical values.
    pub fn restart(&mut self) -> Result<(), SimError> {
        let fresh = self.simulation.restart_demon_map_context()?;
        let id = InstanceId::fresh();
        self.simulation = fresh;
        self.id = id;
        Ok(())
    }
}
