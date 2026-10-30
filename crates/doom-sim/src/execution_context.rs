//! Scoped access to the existing actor owner; no second lifetime manager.
use crate::{identity::*, *};
use std::{error::Error, fmt, marker::PhantomData};

// p_mobj.h: named source flag, independent of actor role and storage liveness.
const MF_SHOOTABLE: u32 = 0x4;
type Brand<'ctx> = PhantomData<fn(&'ctx ()) -> &'ctx ()>;

/// An instance reference branded to one exclusive execution scope. Not a liveness
/// lease: every access still validates its generation. No definition is stored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextActor<'ctx> {
    id: ActorId,
    brand: Brand<'ctx>,
}
impl ContextActor<'_> {
    /// Explicitly erase the scope for legacy diagnostics/local APIs. Equal raw
    /// values from different simulations do not establish shared provenance.
    pub const fn raw_id(self) -> ActorId {
        self.id
    }
}

/// Context admission or a delegated operation failed. Earlier successful calls
/// inside a scope remain applied; a scope is not an aggregate transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextError {
    /// The selected simulation has no admitted original-ID execution context.
    Profile,
    /// Candidate identity and its derived unavailable storage status.
    Unavailable(ActorId, ActorAvailability),
    /// Explicit expected definition differs from the current actual definition.
    DefinitionMismatch(DefinitionIdentity, DefinitionIdentity),
    /// Existing original definition/state admission failed.
    Identity(OriginalRegistryError),
    /// Existing gameplay service rejected before its documented commit boundary.
    Runtime(SimError),
}
impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "execution context: {self:?}")
    }
}
impl Error for ContextError {}
impl From<SimError> for ContextError {
    fn from(value: SimError) -> Self {
        Self::Runtime(value)
    }
}
impl From<OriginalRegistryError> for ContextError {
    fn from(value: OriginalRegistryError) -> Self {
        Self::Identity(value)
    }
}

/// Pure target observation. Availability does not grant combat permission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetReference<'ctx> {
    /// The owner stores no reference.
    Absent,
    /// Current nonpending body, including a possible corpse.
    Available {
        /// Checked instance in this same scope.
        actor: ContextActor<'ctx>,
        /// Current health, independent of storage availability.
        health: i32,
        /// Current source MF_SHOOTABLE bit; other action guards still apply.
        shootable: bool,
    },
    /// Stored identity is retained without resolving to another occupant.
    Unavailable {
        /// Original stored identity, never silently cleared or rebound.
        stored: ActorId,
        /// Pending, retired, stale, vacant, invalid or out-of-range status.
        status: ActorAvailability,
    },
    /// Current storage has a definition outside this profile's target role.
    Incompatible {
        /// Stored local instance.
        stored: ActorId,
        /// Its actual definition, separate from instance identity.
        definition: DefinitionIdentity,
    },
}

/// Exclusive generative view over one Simulation. ContextRole describes a
/// definition; this scope instead identifies access to one concrete actor owner.
/// Tokens cannot escape, be widened, or mix between scopes. Raw admission is an
/// explicit local interpretation, not a global-world provenance check.
///
/// A token cannot escape the callback:
/// ```compile_fail
/// use doom_sim::{*, identity::OriginalActorType};
/// fn escape(sim: &mut Simulation) {
///     let escaped = sim.with_execution_context(|ctx| {
///         ctx.admit_local(ActorId::from_parts(0, 1),
///             DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT))
///     });
/// }
/// ```
/// Distinct contexts cannot exchange typed instance references:
/// ```compile_fail
/// use doom_sim::{*, identity::OriginalActorType};
/// fn mix(a: &mut Simulation, b: &mut Simulation) -> Result<(), ContextError> {
///     a.with_execution_context(|outer| {
///         let owner = outer.admit_local(ActorId::from_parts(0, 1),
///             DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT))?;
///         b.with_execution_context(|inner| {
///             let target = inner.admit_local(ActorId::from_parts(1, 1),
///                 DefinitionIdentity::Context(ContextRole::UnboundBody))?;
///             inner.set_target(owner, Some(target))
///         })
///     })
/// }
/// ```
pub struct ExecutionContext<'ctx> {
    simulation: &'ctx mut Simulation,
    instance: Option<&'ctx InstanceId>,
    brand: Brand<'ctx>,
}
impl Simulation {
    /// Read storage availability in this store only, without mutation or RNG.
    pub fn actor_availability(&self, id: ActorId) -> ActorAvailability {
        self.actors.availability(id)
    }
    /// Admit an exclusive original-ID scope. Each delegated operation preserves
    /// its own atomicity; successful earlier operations survive callback errors.
    /// No persistent context identifier, allocation or world restoration occurs.
    pub fn with_execution_context<R>(
        &mut self,
        operation: impl for<'ctx> FnOnce(&mut ExecutionContext<'ctx>) -> Result<R, ContextError>,
    ) -> Result<R, ContextError> {
        operation(&mut ExecutionContext::new(self, None)?)
    }
}
impl<'ctx> ExecutionContext<'ctx> {
    pub(crate) fn new(
        simulation: &'ctx mut Simulation,
        instance: Option<&'ctx InstanceId>,
    ) -> Result<Self, ContextError> {
        if simulation.profile != Profile::OriginalDemon {
            return Err(ContextError::Profile);
        }
        // Construction already certified the immutable registry and profile.
        Ok(Self {
            simulation,
            instance,
            brand: PhantomData,
        })
    }
    /// Export a checked current actor for another scope of this same incarnation.
    /// Pending storage remains observable. The reference retains only a small key,
    /// not actor storage/liveness. Raw Simulation scopes reject as unbound.
    pub fn export_actor(
        &self,
        actor: ContextActor<'ctx>,
    ) -> Result<InstanceActorRef, ProvenanceError> {
        let instance = self.instance.ok_or(ProvenanceError::UnboundContext)?;
        self.actor(actor)?;
        Ok(InstanceActorRef::candidate(
            Some(instance.clone()),
            actor.id,
        ))
    }
    /// Revalidate matching provenance before existing local generation/state/
    /// expected-definition admission. No mutation, allocation or RNG consumption.
    /// Pending/corpse observation does not authorize any gameplay operation.
    pub fn import_actor(
        &self,
        reference: &InstanceActorRef,
        expected: DefinitionIdentity,
    ) -> Result<ContextActor<'ctx>, ProvenanceError> {
        let instance = self.instance.ok_or(ProvenanceError::UnboundContext)?;
        let provenance = reference
            .provenance()
            .ok_or(ProvenanceError::MissingProvenance)?;
        if provenance != instance {
            return Err(ProvenanceError::InstanceMismatch);
        }
        self.admit_local(reference.local_id(), expected)
            .map_err(Into::into)
    }
    fn token(&self, id: ActorId) -> ContextActor<'ctx> {
        ContextActor {
            id,
            brand: PhantomData,
        }
    }
    fn current(&self, id: ActorId) -> Result<Actor, ContextError> {
        let status = self.simulation.actor_availability(id);
        if !matches!(
            status,
            ActorAvailability::Live | ActorAvailability::PendingRemoval
        ) {
            return Err(ContextError::Unavailable(id, status));
        }
        self.simulation.original_actor_identity(id)?;
        self.simulation
            .actor(id)
            .copied()
            .ok_or(ContextError::Unavailable(id, status))
    }
    /// Interpret a raw candidate locally with an explicit expected definition.
    /// Pending storage may be observed; this does not authorize target acquisition.
    pub fn admit_local(
        &self,
        id: ActorId,
        expected: DefinitionIdentity,
    ) -> Result<ContextActor<'ctx>, ContextError> {
        self.current(id)?;
        let actual = self.simulation.original_actor_identity(id)?.0;
        if actual != expected {
            return Err(ContextError::DefinitionMismatch(expected, actual));
        }
        Ok(self.token(id))
    }
    /// A checked copy, never a borrowed actor that survives validated access.
    pub fn actor(&self, actor: ContextActor<'ctx>) -> Result<Actor, ContextError> {
        self.current(actor.id)
    }
    /// Pure status, including retirement/reuse since this token was admitted.
    pub fn availability(&self, actor: ContextActor<'ctx>) -> ActorAvailability {
        self.simulation.actor_availability(actor.id)
    }
    /// Observe a retained reference without clearing, rebinding or drawing RNG.
    pub fn target(&self, owner: ContextActor<'ctx>) -> Result<TargetReference<'ctx>, ContextError> {
        let actor = self.actor(owner)?;
        let Some(stored) = actor.life().and_then(|life| life.target) else {
            return Ok(TargetReference::Absent);
        };
        let status = self.simulation.actor_availability(stored);
        if status != ActorAvailability::Live {
            return Ok(TargetReference::Unavailable { stored, status });
        }
        let definition = self.simulation.original_actor_identity(stored)?.0;
        let body = self.current(stored)?;
        let life = body.life().ok_or(ContextError::Identity(
            OriginalRegistryError::ProfileOrActor,
        ))?;
        // Observation needs storage/state/definition-role validity, not another
        // allocating immutable type-admission traversal. Acquisition retains the
        // existing complete physical/action checks and reuses this same role guard.
        match self
            .simulation
            .validate_perception_target_type(life.actor_type)
        {
            Ok(()) => {}
            Err(SimError::MapSpawn(MapSpawnError::Target)) => {
                return Ok(TargetReference::Incompatible { stored, definition });
            }
            Err(error) => return Err(error.into()),
        }
        Ok(TargetReference::Available {
            actor: self.token(stored),
            health: life.health,
            shootable: life.flags & MF_SHOOTABLE != 0,
        })
    }
    /// Assign/replace/clear through existing role, health and lifetime guards.
    /// Clearing never needs to resolve an old unavailable reference. Demon self
    /// targets reject by role; synthetic body self-policy remains unchanged.
    pub fn set_target(
        &mut self,
        owner: ContextActor<'ctx>,
        target: Option<ContextActor<'ctx>>,
    ) -> Result<(), ContextError> {
        self.actor(owner)?;
        if let Some(target) = target {
            self.actor(target)?;
        }
        self.simulation
            .set_actor_target(owner.id, target.map(ContextActor::raw_id))
            .map_err(Into::into)
    }
    /// Immediate spatial unlink and deferred retirement; duplicates are idempotent.
    pub fn mark_remove(&mut self, actor: ContextActor<'ctx>) -> Result<(), ContextError> {
        self.actor(actor)?;
        self.simulation.mark_remove(actor.id).map_err(Into::into)
    }
    /// Shared checked damage. Source and inflictor remain independent references.
    pub fn damage(
        &mut self,
        target: ContextActor<'ctx>,
        inflictor: Option<ContextActor<'ctx>>,
        source: Option<ContextActor<'ctx>>,
        amount: i32,
    ) -> Result<DamageResult, ContextError> {
        self.actor(target)?;
        for reference in [inflictor, source].into_iter().flatten() {
            self.actor(reference)?;
        }
        self.simulation
            .damage_actor(DamageRequest {
                target: target.id,
                inflictor: inflictor.map(ContextActor::raw_id),
                source: source.map(ContextActor::raw_id),
                amount,
            })
            .map_err(Into::into)
    }
    /// Enter only states already admitted by the existing original execution policy.
    pub fn enter_state(
        &mut self,
        actor: ContextActor<'ctx>,
        state: OriginalState,
    ) -> Result<(), ContextError> {
        self.actor(actor)?;
        self.simulation
            .enter_original_state(actor.id, state)
            .map_err(Into::into)
    }
    /// Spawn the existing explicit body; no original-player admission is implied.
    pub fn spawn_body(
        &mut self,
        position: [doom_types::Fixed; 2],
        height: SpawnHeight,
    ) -> Result<ContextActor<'ctx>, ContextError> {
        let id = self.simulation.spawn_context_body(position, height)?;
        // A successful existing spawn certifies this current owned instance.
        Ok(self.token(id))
    }
    /// Spawn and bind the sole genuine player through the existing actor owner.
    pub fn spawn_player(
        &mut self,
        slot: PlayerSlot,
        position: [doom_types::Fixed; 2],
    ) -> Result<ContextActor<'ctx>, ContextError> {
        let id = self.simulation.spawn_player_body(slot, position)?;
        Ok(self.token(id))
    }
    /// Bind a checked scoped initial MT_PLAYER; qualified imports retain instance checks.
    /// A body from another scope cannot be bound, even with equal raw slots:
    /// ```compile_fail
    /// use doom_sim::*;
    /// fn cross(a: &mut Simulation, b: &mut Simulation) -> Result<(), ContextError> {
    ///     a.with_execution_context(|outer| {
    ///         let player = outer.spawn_player(PlayerSlot::One, Default::default())?;
    ///         b.with_execution_context(|inner| inner.bind_original_player(PlayerSlot::One, player))
    ///     })
    /// }
    /// ```
    pub fn bind_original_player(
        &mut self,
        slot: PlayerSlot,
        body: ContextActor<'ctx>,
    ) -> Result<(), ContextError> {
        self.actor(body)?;
        self.simulation
            .bind_original_player_body(slot, body.id)
            .map_err(Into::into)
    }
    /// Source record with retained opaque handles, not proof of reference liveness.
    pub fn original_player(&self, slot: PlayerSlot) -> Result<OriginalPlayer, ContextError> {
        self.simulation
            .original_player(slot)
            .map_err(|e| ContextError::Runtime(e.into()))
    }
    /// Set/repair the existing four-slot synthetic perception input. This is not
    /// player gameplay or a new ownership role. Existing roster guards apply.
    pub fn set_perception_player(
        &mut self,
        slot: usize,
        actor: Option<ContextActor<'ctx>>,
        active: bool,
        health: i32,
    ) -> Result<(), ContextError> {
        if let Some(actor) = actor {
            self.actor(actor)?;
        }
        self.simulation
            .set_perception_player(
                slot,
                PerceptionPlayer {
                    actor: actor.map(ContextActor::raw_id),
                    active,
                    health,
                },
            )
            .map_err(Into::into)
    }
    /// Existing checked tic and thinker order; no second traversal or scheduler.
    pub fn step(&mut self, commands: [TicCommand; 4]) -> Result<Snapshot, ContextError> {
        self.simulation.step(commands).map_err(Into::into)
    }
    /// Existing checked tic without implicit Snapshot capture. Event recording,
    /// per-operation atomicity and this context's ownership rules are unchanged.
    pub fn advance(&mut self, commands: [TicCommand; 4]) -> Result<(), ContextError> {
        self.simulation.advance(commands).map_err(Into::into)
    }
    /// Existing diagnostics, with unchanged field meanings and v21 bytes.
    pub fn snapshot(&self) -> Snapshot {
        self.simulation.snapshot()
    }
    /// Copy of both RNG streams; observation cannot consume gameplay draws.
    pub fn rng(&self) -> doom_types::DoomRng {
        self.simulation.rng()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhausted_generation_is_retired_without_wrapping_or_aliasing_earlier_occupants() {
        let mut store = ThinkerStore::new(2).unwrap();
        let initial = store.spawn(()).unwrap();
        let last = ActorId::from_parts(initial.slot(), u64::MAX);
        // Bounded white-box setup of the otherwise impractically distant limit.
        store.slots.get_mut(0).unwrap().generation = u64::MAX;
        store.head = Some(last);
        store.tail = Some(last);
        store.mark_remove(last).unwrap();
        store
            .run(2, |visit, _| assert_eq!(visit, Visit::Retired(last)))
            .unwrap();
        assert_eq!(store.availability(last), ActorAvailability::Retired);
        assert_eq!(
            store.availability(ActorId::from_parts(0, u64::MAX - 1)),
            ActorAvailability::StaleGeneration
        );
        assert_eq!(
            store.availability(initial),
            ActorAvailability::StaleGeneration
        );
        let next = store.spawn(()).unwrap();
        assert_eq!(next.slot(), 1);
        assert_eq!(store.availability(last), ActorAvailability::Retired);
    }
}
