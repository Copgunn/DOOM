//! Sector and line-special writes staged inside one checked tic plan (M3h8 note §9).
//!
//! A plan never writes `Simulation::sectors` or the runtime line specials directly.
//! The first sector write of a plan copies the committed sector table into a buffer
//! reused across tics, every later write changes the copy, and every later query of
//! the same pass reads the copy through [`Sectors`]; a committed plan swaps the copy
//! in and a refused one drops it, so the committed tables stay byte-identical. Line
//! specials are staged the same way, in a copy of the runtime line-special vector.
//! The §9 measurement selected this lazy full copy over a sparse overlay. A plan that
//! writes nothing copies nothing; its buffers then never allocate (every policy before
//! the movers policy writes neither table), and once a buffer has the table's capacity
//! a tic allocates nothing for it.
use crate::{GeometryError, SectorState, SimError, StoreError};
use std::ops::Index;

/// Read-through view of the runtime sector heights: the plan's staged copy once it
/// has written a sector, the committed table before. Copyable, like the slice it
/// replaces; a view without writes is exactly the committed table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sectors<'a> {
    table: &'a [SectorState],
}
impl<'a> Sectors<'a> {
    /// The committed table alone (no plan, or a plan that wrote nothing).
    pub(crate) const fn committed(committed: &'a [SectorState]) -> Self {
        Self { table: committed }
    }
    /// Sector count (the committed table's; writes never add sectors).
    pub(crate) const fn len(self) -> usize {
        self.table.len()
    }
    /// The current state of `index`, or `None` past the table.
    pub(crate) fn get(self, index: usize) -> Option<&'a SectorState> {
        self.table.get(index)
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

/// One table staged by a lazy full copy: `copy` holds the plan's table only while
/// `live`; otherwise it is a spare buffer whose contents mean nothing.
#[derive(Clone, Debug)]
struct LazyCopy<T> {
    copy: Vec<T>,
    live: bool,
}
impl<T> Default for LazyCopy<T> {
    fn default() -> Self {
        Self {
            copy: Vec::new(),
            live: false,
        }
    }
}
impl<T: Copy> LazyCopy<T> {
    fn view<'a>(&'a self, committed: &'a [T]) -> &'a [T] {
        if self.live { &self.copy } else { committed }
    }
    /// Sets `index` to `value`, copying `committed` first on the plan's first write.
    /// Refused past the table or when the copy cannot be allocated; the staged table
    /// is then unchanged.
    fn write(&mut self, committed: &[T], index: u16, value: T) -> Result<(), SimError> {
        let index = usize::from(index);
        if index >= committed.len() {
            return Err(GeometryError::InvalidReference.into());
        }
        if !self.live {
            self.copy.clear();
            self.copy
                .try_reserve(committed.len())
                .map_err(|_| StoreError::Capacity)?;
            self.copy.extend_from_slice(committed);
            self.live = true;
        }
        self.copy[index] = value;
        Ok(())
    }
    /// Swaps a live copy into `committed`; the replaced table becomes the spare.
    fn commit(&mut self, committed: &mut Vec<T>) {
        if self.live {
            debug_assert_eq!(self.copy.len(), committed.len());
            std::mem::swap(&mut self.copy, committed);
            self.live = false;
        }
    }
}
impl<T: PartialEq> PartialEq for LazyCopy<T> {
    /// Equal when both stage nothing or stage equal tables; a spare buffer's stale
    /// contents and capacity are not part of the staged state.
    fn eq(&self, other: &Self) -> bool {
        self.live == other.live && (!self.live || self.copy == other.copy)
    }
}
impl<T: Eq> Eq for LazyCopy<T> {}

/// The staged tables of one plan: lazy full copies of the sector table and of the
/// runtime line specials, each taken on its first write. Not canonical state:
/// swapped into the tables on commit, or dropped with a refused plan.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct StagedWrites {
    sectors: LazyCopy<SectorState>,
    lines: LazyCopy<i16>,
}
impl StagedWrites {
    /// Drops anything staged, keeping the buffers for the next plan.
    pub(crate) fn clear(&mut self) {
        self.sectors.live = false;
        self.lines.live = false;
    }
    /// Whether the plan wrote nothing.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "read by the M3h8 plane movers (plan step 7)")
    )]
    pub(crate) fn is_empty(&self) -> bool {
        !self.sectors.live && !self.lines.live
    }
    /// The capacities of the sector and line-special buffers (tests only).
    #[cfg(test)]
    pub(crate) fn buffer_capacities(&self) -> (usize, usize) {
        (self.sectors.copy.capacity(), self.lines.copy.capacity())
    }
    /// The read-through view of `committed` under this plan's writes.
    pub(crate) fn sectors<'a>(&'a self, committed: &'a [SectorState]) -> Sectors<'a> {
        Sectors {
            table: self.sectors.view(committed),
        }
    }
    /// Stages new heights for `sector` (`T_MovePlane`); a second write of the same
    /// sector replaces the first. Refused past the table or when the copy cannot be
    /// allocated; the staged state is then unchanged.
    pub(crate) fn write_sector(
        &mut self,
        committed: &[SectorState],
        sector: u16,
        state: SectorState,
    ) -> Result<(), SimError> {
        self.sectors.write(committed, sector, state)
    }
    /// The runtime special of `line` under this plan's writes (`committed` holds every
    /// line's runtime special), or `None` past the table.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "read by the M3h8 crossing and use bodies (plan steps 9–10)"
        )
    )]
    pub(crate) fn line_special(&self, committed: &[i16], line: u16) -> Option<i16> {
        self.lines.view(committed).get(usize::from(line)).copied()
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
        self.lines.write(committed, line, special)
    }
    /// Swaps each staged copy into its committed table (the same tables every write
    /// was checked against); the replaced tables become the next plan's buffers. A
    /// table without writes is untouched.
    pub(crate) fn commit(&mut self, sectors: &mut Vec<SectorState>, lines: &mut Vec<i16>) {
        self.sectors.commit(sectors);
        self.lines.commit(lines);
    }
}

#[cfg(test)]
#[path = "staging_tests.rs"]
mod tests;
