//! Explicit native namespace for the validated Profile20 execution binding.
use crate::{canonical_demon::STATES, identity::*, *};

/// Native Demon execution type, distinct from original mobjtype_t.
///
/// Obtained through a validated [`DemonExecutionBinding`]. This value describes
/// its representation, not a live owner or permission to execute in another profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct NativeDemonType(());
impl NativeDemonType {
    #[inline]
    pub(crate) fn checked(candidate: ActorTypeId) -> Option<Self> {
        (candidate == canonical_demon::RUNTIME_OWNER).then_some(Self(()))
    }
    /// Explicit registry-slot observation; this is not an original ordinal.
    #[inline]
    pub const fn runtime(self) -> ActorTypeId {
        canonical_demon::RUNTIME_OWNER
    }
    /// Canonical source definition represented by this native role.
    #[inline]
    pub const fn original(self) -> OriginalActorType {
        OriginalActorType::MT_SERGEANT
    }
}

// Derived only from the historical bijection. Parallel bounded counters avoid
// casts in const evaluation on the MSRV. No manually duplicated reverse mapping.
const fn reverse() -> [Option<OriginalState>; 22] {
    let mut result = [None; 22];
    let mut index = 0;
    let mut raw = 0;
    while index < result.len() {
        let mut source = 0;
        while source < STATES.len() {
            if STATES[source].1.0 == raw {
                assert!(result[index].is_none(), "duplicate native Demon slot");
                result[index] = Some(STATES[source].0);
            }
            source += 1;
        }
        index += 1;
        raw += 1;
    }
    let mut source = 0;
    while source < STATES.len() {
        assert!(STATES[source].1.0 > 0 && STATES[source].1.0 < 22);
        source += 1;
    }
    result
}
const ORIGINAL_BY_NATIVE: [Option<OriginalState>; 22] = reverse();

/// A checked native state in Profile20's normal Demon closure.
///
/// This is a transient view of a registry slot, not additional actor storage.
/// Pain/death observations are valid values but still reject direct entry.
/// S_NULL, raise states and canonical ordinals passed as native slots reject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeDemonState(StateId);
impl NativeDemonState {
    #[inline]
    pub(crate) fn checked(candidate: StateId) -> Option<Self> {
        ORIGINAL_BY_NATIVE
            .get(usize::from(candidate.0))
            .copied()
            .flatten()
            .map(|_| Self(candidate))
    }
    #[inline]
    pub(crate) fn from_original(original: OriginalState) -> Option<Self> {
        STATES
            .iter()
            .find(|(s, _)| *s == original)
            .map(|(_, r)| Self(*r))
    }
    /// Explicit registry-slot observation, preserving the historical alias.
    #[inline]
    pub const fn runtime(self) -> StateId {
        self.0
    }
    /// Canonical source definition, through the single declared bijection.
    #[inline]
    pub fn original(self) -> OriginalState {
        ORIGINAL_BY_NATIVE
            .get(usize::from(self.0.0))
            .copied()
            .flatten()
            .expect("private checked native state")
    }
    #[inline]
    pub(crate) fn allows_direct_entry(self) -> bool {
        matches!(
            self.original(),
            OriginalState::S_SARG_STND
                | OriginalState::S_SARG_STND2
                | OriginalState::S_SARG_RUN1
                | OriginalState::S_SARG_RUN2
                | OriginalState::S_SARG_RUN3
                | OriginalState::S_SARG_RUN4
                | OriginalState::S_SARG_RUN5
                | OriginalState::S_SARG_RUN6
                | OriginalState::S_SARG_RUN7
                | OriginalState::S_SARG_RUN8
                | OriginalState::S_SARG_ATK1
                | OriginalState::S_SARG_ATK2
                | OriginalState::S_SARG_ATK3
        )
    }
    #[inline]
    pub(crate) fn is_death(self) -> bool {
        matches!(
            self.original(),
            OriginalState::S_SARG_DIE1
                | OriginalState::S_SARG_DIE2
                | OriginalState::S_SARG_DIE3
                | OriginalState::S_SARG_DIE4
                | OriginalState::S_SARG_DIE5
                | OriginalState::S_SARG_DIE6
        )
    }
    #[inline]
    pub(crate) fn before_fall(self) -> bool {
        matches!(
            self.original(),
            OriginalState::S_SARG_DIE1 | OriginalState::S_SARG_DIE2 | OriginalState::S_SARG_DIE3
        )
    }
}

#[cfg(test)]
#[path = "native_demon_tests.rs"]
mod tests;
