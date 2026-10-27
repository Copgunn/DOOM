//! Bounded, borrowing IWAD/PWAD reader. Parsing performs no filesystem I/O.
//!
//! [`Wad`] retains directory order and exact names. [`WadSet`] searches archives
//! in reverse load order. Lookup folds ASCII case and ignores bytes after NUL;
//! raw [`LumpName`] equality remains byte equality. Overlapping ranges are legal.

mod archive;

pub use archive::{Limits, Lump, LumpLocation, Wad, WadError, WadSet};

use doom_types::LumpName;

/// Archive kind encoded by the four-byte WAD signature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WadKind {
    /// Base game archive (`IWAD`).
    Iwad,
    /// Patch archive (`PWAD`).
    Pwad,
}

impl WadKind {
    /// Recognizes an exact signature; this does not validate a WAD header.
    pub const fn from_magic(magic: [u8; 4]) -> Option<Self> {
        match &magic {
            b"IWAD" => Some(Self::Iwad),
            b"PWAD" => Some(Self::Pwad),
            _ => None,
        }
    }
}

/// Unvalidated directory metadata, retaining the format's signed 32-bit fields.
///
/// Construction does not establish that an entry is safe to access. M1 must
/// reject negative fields and check archive bounds before exposing lump data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LumpInfo {
    /// Raw name from the directory.
    pub name: LumpName,
    /// Byte offset from the beginning of the archive.
    pub offset: i32,
    /// Byte length of the lump.
    pub size: i32,
}
