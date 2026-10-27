//! Shared leaf types for the Rust rewrite.
//!
//! Explicit fixed-point arithmetic, owned vanilla RNG and raw resource names.
//! This crate performs no I/O and has no dependencies.

mod fixed;
mod rng;

pub use fixed::{Fixed, FixedDivError};
pub use rng::DoomRng;

/// Logical simulation tics per second, from Linux DOOM 1.10's `TICRATE`.
pub const TIC_RATE: u32 = 35;

/// An uninterpreted eight-byte WAD lump name.
///
/// Names need not be UTF-8 or NUL-terminated. Preserve all bytes at the file
/// boundary; archive lookup and its case/override rules belong to `doom-wad`.
/// Equality here compares raw bytes, not archive lookup equivalence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LumpName([u8; 8]);

impl LumpName {
    /// Retains an on-disk name exactly, without normalization or validation.
    pub const fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }

    /// Returns all eight original bytes, including any padding.
    pub const fn as_bytes(&self) -> &[u8; 8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::LumpName;

    #[test]
    fn retains_full_width_and_non_text_names() {
        for bytes in [*b"SIDEDEFS", *b"map01\0\0\0", [0, 255, 128, 1, 2, 3, 4, 5]] {
            assert_eq!(LumpName::from_bytes(bytes).as_bytes(), &bytes);
        }
    }

    #[test]
    fn raw_equality_does_not_apply_lookup_rules() {
        assert_ne!(
            LumpName::from_bytes(*b"map01\0\0\0"),
            LumpName::from_bytes(*b"MAP01\0\0\0")
        );
    }
}
