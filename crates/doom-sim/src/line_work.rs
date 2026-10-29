//! Bounded line membership; bit order never determines candidate order.
use crate::GeometryError;

pub(crate) const MAX_LINES: usize = 1024;

pub(crate) fn reserved<T>(count: usize) -> Result<Vec<T>, GeometryError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| GeometryError::Allocation)?;
    Ok(values)
}

/// Query-local validcount equivalent. No allocation during a movement decision.
/// All callers have already admitted at most MAX_LINES immutable linedefs.
#[derive(Clone, Default)]
pub(crate) struct SeenLines([u64; MAX_LINES / 64]);
impl SeenLines {
    pub fn insert(&mut self, index: usize) -> bool {
        let word = &mut self.0[index / 64];
        let bit = 1_u64 << (index % 64);
        let fresh = *word & bit == 0;
        *word |= bit;
        fresh
    }
}

/// Preparation-only dense membership, sized from both actual table dimensions.
/// Ordered collision lists remain separate and retain source first encounters.
pub(crate) struct CellMembership {
    words: Vec<u64>,
    stride: usize,
}
impl CellMembership {
    pub fn new(cells: &[Vec<u16>], lines: usize) -> Result<Self, GeometryError> {
        let stride = lines.div_ceil(64);
        let length = cells
            .len()
            .checked_mul(stride)
            .ok_or(GeometryError::CoordinateOverflow)?;
        let mut words = reserved(length)?;
        words.resize(length, 0_u64);
        for (cell, entries) in cells.iter().enumerate() {
            for &line in entries {
                let line = usize::from(line);
                words[cell * stride + line / 64] |= 1_u64 << (line % 64);
            }
        }
        Ok(Self { words, stride })
    }
    pub fn contains(&self, cell: usize, line: usize) -> bool {
        self.words[cell * self.stride + line / 64] & (1_u64 << (line % 64)) != 0
    }
}
