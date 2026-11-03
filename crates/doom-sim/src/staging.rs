//! Sector and line-special writes staged inside one checked tic plan (M3h8 note §9).
//!
//! A plan never writes `Simulation::sectors` or the runtime line specials directly.
//! It records each written sector (and each line-special change) once, in a small
//! copy-on-write overlay; every later query of the same pass reads through it with
//! [`Sectors`]; a committed plan applies the overlay and a refused one drops it, so the
//! committed tables stay byte-identical. The overlay of the last committed plan is
//! kept as the next plan's buffer, so a tic allocates nothing for it once its capacity
//! is reached; without writes it is empty and never allocates (every policy before
//! the movers policy writes neither table).
use crate::{GeometryError, SectorState, SimError, StoreError};
use std::ops::Index;

/// Read-through view of the runtime sector heights: the committed table, with the
/// entries written earlier in the same plan taking precedence. Copyable, like the
/// slice it replaces; a view without writes is exactly the committed table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sectors<'a> {
    committed: &'a [SectorState],
    written: &'a [(u16, SectorState)],
}
impl<'a> Sectors<'a> {
    /// The committed table alone (no plan, or a plan that wrote nothing).
    pub(crate) const fn committed(committed: &'a [SectorState]) -> Self {
        Self {
            committed,
            written: &[],
        }
    }
    /// Sector count (the committed table's; writes never add sectors).
    pub(crate) const fn len(self) -> usize {
        self.committed.len()
    }
    /// The current state of `index`, or `None` past the table.
    pub(crate) fn get(self, index: usize) -> Option<&'a SectorState> {
        // A linear scan of at most the sectors written this tic (`≤ M`, §8).
        match self.written.iter().find(|(s, _)| usize::from(*s) == index) {
            Some((_, state)) => Some(state),
            None => self.committed.get(index),
        }
    }
}
impl Index<usize> for Sectors<'_> {
    type Output = SectorState;
    fn index(&self, index: usize) -> &SectorState {
        match self.get(index) {
            Some(state) => state,
            None => panic!("sector {index} out of range of {}", self.len()),
        }
    }
}

/// The sparse overlay of one plan: each sector whose heights were written, and each
/// line whose runtime special was changed, at most once, in first-write order. Not
/// canonical state: committed into the tables, or dropped with a refused plan.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct StagedWrites {
    sectors: Vec<(u16, SectorState)>,
    lines: Vec<(u16, i16)>,
}
impl StagedWrites {
    /// Empties the overlay, keeping its capacity for the next plan.
    pub(crate) fn clear(&mut self) {
        self.sectors.clear();
        self.lines.clear();
    }
    /// Whether the plan wrote nothing.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "read by the M3h8 plane movers (plan step 7)")
    )]
    pub(crate) fn is_empty(&self) -> bool {
        self.sectors.is_empty() && self.lines.is_empty()
    }
    /// The read-through view of `committed` under this overlay.
    pub(crate) fn sectors<'a>(&'a self, committed: &'a [SectorState]) -> Sectors<'a> {
        Sectors {
            committed,
            written: &self.sectors,
        }
    }
    /// Stages new heights for `sector` (`T_MovePlane`); a second write of the same
    /// sector replaces the first. Refused past the table or when the overlay cannot
    /// grow; the overlay is then unchanged.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "written by the M3h8 plane movers (plan step 7)")
    )]
    pub(crate) fn write_sector(
        &mut self,
        committed: &[SectorState],
        sector: u16,
        state: SectorState,
    ) -> Result<(), SimError> {
        if usize::from(sector) >= committed.len() {
            return Err(GeometryError::InvalidReference.into());
        }
        stage(&mut self.sectors, sector, state)
    }
    /// The runtime special of `line` under this overlay (`committed` holds every
    /// line's runtime special), or `None` past the table.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "read by the M3h8 crossing and use bodies (plan steps 9–10)"
        )
    )]
    pub(crate) fn line_special(&self, committed: &[i16], line: u16) -> Option<i16> {
        match self.lines.iter().find(|(l, _)| *l == line) {
            Some((_, special)) => Some(*special),
            None => committed.get(usize::from(line)).copied(),
        }
    }
    /// Stages the runtime special of `line` (a W1 or S1 line cleared to 0).
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "written by the M3h8 crossing and use bodies (plan steps 9–10)"
        )
    )]
    pub(crate) fn write_line_special(
        &mut self,
        committed: &[i16],
        line: u16,
        special: i16,
    ) -> Result<(), SimError> {
        if usize::from(line) >= committed.len() {
            return Err(GeometryError::InvalidReference.into());
        }
        stage(&mut self.lines, line, special)
    }
    /// Applies the overlay to the committed tables. Every staged index was checked
    /// against the same tables when it was written.
    pub(crate) fn commit(&self, sectors: &mut [SectorState], lines: &mut [i16]) {
        for &(sector, state) in &self.sectors {
            sectors[usize::from(sector)] = state;
        }
        for &(line, special) in &self.lines {
            lines[usize::from(line)] = special;
        }
    }
}
fn stage<V>(entries: &mut Vec<(u16, V)>, index: u16, value: V) -> Result<(), SimError> {
    if let Some(entry) = entries.iter_mut().find(|(i, _)| *i == index) {
        entry.1 = value;
        return Ok(());
    }
    entries.try_reserve(1).map_err(|_| StoreError::Capacity)?;
    entries.push((index, value));
    Ok(())
}

#[cfg(test)]
#[path = "staging_tests.rs"]
mod tests;
