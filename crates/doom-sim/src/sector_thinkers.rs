//! Sector thinkers (`P_SpawnSpecials` lights now, movers later) share the actor
//! processing order but live in their own slot class (M3h7 owner decision 4).
use crate::{StoreError, ThinkerContext, Visit};

/// Payload of the sector-thinker slot class. No admitted policy creates a sector
/// thinker yet, so the type is uninhabited: every simulation store is built with
/// sector capacity zero and the dispatch below is statically unreachable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SectorThinker {}

/// The thinker pass's sector-thinker hook. It refuses explicitly instead of
/// skipping: an actor visit here, or a sector visit of a storage that cannot
/// exist, is reported as [`StoreError::UnsupportedThinker`].
pub(crate) fn visit<T>(
    context: &mut ThinkerContext<'_, T, SectorThinker>,
    visit: Visit,
) -> Result<(), StoreError> {
    match visit {
        Visit::SectorThink(id) => match context.sector_get(id) {
            Some(&thinker) => match thinker {},
            None => Err(StoreError::UnsupportedThinker),
        },
        Visit::SectorRetired(_) | Visit::Think(_) | Visit::Retired(_) => {
            Err(StoreError::UnsupportedThinker)
        }
    }
}
