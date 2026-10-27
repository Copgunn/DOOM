//! Exact Linux DOOM 1.10 table and independently owned stream indexes.

include!("rng_table.rs");

/// Owned gameplay and miscellaneous RNG streams. No process-global state.
#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub struct DoomRng {
    gameplay: u8,
    miscellaneous: u8,
}
impl DoomRng {
    /// Restores exact indexes, without consuming a lookup.
    pub const fn from_indices(gameplay: u8, miscellaneous: u8) -> Self {
        Self {
            gameplay,
            miscellaneous,
        }
    }
    /// Gameplay index followed by miscellaneous index.
    pub const fn indices(self) -> [u8; 2] {
        [self.gameplay, self.miscellaneous]
    }
    /// Increment gameplay index before reading the original table.
    pub fn p_random(&mut self) -> u8 {
        self.gameplay = self.gameplay.wrapping_add(1);
        TABLE[usize::from(self.gameplay)]
    }
    /// Increment only the non-gameplay stream. Rendering cannot consume P_Random.
    pub fn m_random(&mut self) -> u8 {
        self.miscellaneous = self.miscellaneous.wrapping_add(1);
        TABLE[usize::from(self.miscellaneous)]
    }
    /// Original M_ClearRandom behavior: both indexes become zero.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
