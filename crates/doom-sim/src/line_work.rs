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

/// Query-local validcount equivalent. Worlds of at most `MAX_LINES` linedefs (every
/// policy before the real-map one) keep the inline bitset and allocate nothing
/// during a movement or combat decision; larger worlds, admitted only by the
/// real-map policy, use one heap bitset sized from the line count (the WAD format
/// bounds it at 32,768 lines, 4 KiB), the same way the trace already grows its
/// intercept list. Bit order never determines candidate order.
#[derive(Clone)]
pub(crate) enum SeenLines {
    Inline([u64; MAX_LINES / 64]),
    Heap(Vec<u64>),
}
impl SeenLines {
    /// A clear set able to hold every index below `lines`.
    pub fn for_lines(lines: usize) -> Self {
        if lines <= MAX_LINES {
            Self::Inline([0; MAX_LINES / 64])
        } else {
            Self::Heap(vec![0; lines.div_ceil(64)])
        }
    }
    fn words(&mut self) -> &mut [u64] {
        match self {
            Self::Inline(words) => words,
            Self::Heap(words) => words,
        }
    }
    /// Mark `index` and report whether it was fresh.
    pub fn insert(&mut self, index: usize) -> bool {
        let word = &mut self.words()[index / 64];
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
