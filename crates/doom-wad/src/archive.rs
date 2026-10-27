use std::{error::Error, fmt, ops::Range};

use doom_types::LumpName;

use crate::WadKind;

/// Per-archive resource policy, checked before directory allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Maximum input bytes; the caller controls acquisition of these bytes.
    pub max_archive_bytes: usize,
    /// Maximum directory entries.
    pub max_lumps: usize,
    /// Maximum size of an individual lump.
    pub max_lump_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_archive_bytes: 512 * 1024 * 1024,
            max_lumps: 1_000_000,
            max_lump_bytes: 128 * 1024 * 1024,
        }
    }
}

/// Malformed archive or resource-policy failure. No filesystem paths are stored.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WadError {
    /// Fewer than twelve bytes are available.
    TruncatedHeader,
    /// The first four bytes are neither IWAD nor PWAD.
    InvalidMagic,
    /// A signed field was negative; `entry = None` denotes the header.
    NegativeField {
        /// Directory entry, if applicable.
        entry: Option<usize>,
        /// Field name.
        field: &'static str,
        /// Original signed value.
        value: i32,
    },
    /// A checked size calculation could not be represented on this platform.
    ArithmeticOverflow,
    /// A configured resource bound was exceeded.
    LimitExceeded {
        /// Resource name.
        resource: &'static str,
        /// Requested quantity.
        actual: usize,
        /// Configured maximum.
        limit: usize,
    },
    /// A directory or lump region falls outside the input.
    OutOfBounds {
        /// Directory entry; None denotes the directory itself.
        entry: Option<usize>,
        /// Region start.
        offset: usize,
        /// Region length.
        size: usize,
    },
    /// Directory storage could not be reserved.
    AllocationFailed,
}

impl fmt::Display for WadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WAD validation failed: {self:?}")
    }
}
impl Error for WadError {}

/// Validated directory entry. Its range cannot be changed after parsing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Lump {
    name: LumpName,
    range: Range<usize>,
}

impl Lump {
    /// Exact on-disk name, including padding and non-text bytes.
    pub const fn name(&self) -> LumpName {
        self.name
    }
    /// Validated byte range within the archive; ranges may overlap.
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
    /// ASCII-insensitive lookup equivalence, truncated/padded to eight bytes.
    pub fn is_named(&self, query: &str) -> bool {
        normalized(self.name.as_bytes()) == normalized(query.as_bytes())
    }
}

fn normalized(bytes: &[u8]) -> [u8; 8] {
    let mut name = [0; 8];
    for (out, byte) in name
        .iter_mut()
        .zip(bytes.iter().copied().take_while(|b| *b != 0))
    {
        *out = byte.to_ascii_uppercase();
    }
    name
}

fn unsigned(value: i32, field: &'static str, entry: Option<usize>) -> Result<usize, WadError> {
    usize::try_from(value).map_err(|_| WadError::NegativeField {
        entry,
        field,
        value,
    })
}

fn bound(actual: usize, limit: usize, resource: &'static str) -> Result<(), WadError> {
    if actual > limit {
        Err(WadError::LimitExceeded {
            resource,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

fn region(
    offset: usize,
    size: usize,
    len: usize,
    entry: Option<usize>,
) -> Result<Range<usize>, WadError> {
    let end = offset
        .checked_add(size)
        .ok_or(WadError::ArithmeticOverflow)?;
    if end > len {
        return Err(WadError::OutOfBounds {
            entry,
            offset,
            size,
        });
    }
    Ok(offset..end)
}

fn signed(bytes: &[u8]) -> i32 {
    i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Immutable archive borrowing its source bytes; no lump data is copied.
#[derive(Debug)]
pub struct Wad<'a> {
    bytes: &'a [u8],
    kind: WadKind,
    lumps: Vec<Lump>,
}

impl<'a> Wad<'a> {
    /// Validates the full header and directory before returning any data views.
    ///
    /// Negative fields and ranges past EOF are rejected, including empty lumps
    /// with offsets past EOF. Zero-size markers at zero/EOF and arbitrary shared,
    /// overlapping, header/directory-overlapping in-bounds regions are permitted.
    ///
    /// ```
    /// use doom_wad::{Limits, Wad, WadKind};
    /// let bytes = b"IWAD\0\0\0\0\x0c\0\0\0";
    /// let archive = Wad::parse(bytes, Limits::default())?;
    /// assert_eq!(archive.kind(), WadKind::Iwad);
    /// assert!(archive.lumps().is_empty());
    /// # Ok::<(), doom_wad::WadError>(())
    /// ```
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, WadError> {
        bound(bytes.len(), limits.max_archive_bytes, "archive bytes")?;
        if bytes.len() < 12 {
            return Err(WadError::TruncatedHeader);
        }
        let kind = WadKind::from_magic([bytes[0], bytes[1], bytes[2], bytes[3]])
            .ok_or(WadError::InvalidMagic)?;
        let count = unsigned(signed(&bytes[4..8]), "lump count", None)?;
        let offset = unsigned(signed(&bytes[8..12]), "directory offset", None)?;
        bound(count, limits.max_lumps, "lump count")?;
        let size = count.checked_mul(16).ok_or(WadError::ArithmeticOverflow)?;
        let directory = region(offset, size, bytes.len(), None)?;
        let mut lumps = Vec::new();
        lumps
            .try_reserve_exact(count)
            .map_err(|_| WadError::AllocationFailed)?;
        for (index, record) in bytes[directory].chunks_exact(16).enumerate() {
            let offset = unsigned(signed(&record[..4]), "lump offset", Some(index))?;
            let size = unsigned(signed(&record[4..8]), "lump size", Some(index))?;
            bound(size, limits.max_lump_bytes, "lump bytes")?;
            let range = region(offset, size, bytes.len(), Some(index))?;
            let mut name = [0; 8];
            name.copy_from_slice(&record[8..16]);
            lumps.push(Lump {
                name: LumpName::from_bytes(name),
                range,
            });
        }
        Ok(Self { bytes, kind, lumps })
    }
    /// Archive signature kind.
    pub const fn kind(&self) -> WadKind {
        self.kind
    }
    /// Entries in original on-disk order, without deduplication.
    pub fn lumps(&self) -> &[Lump] {
        &self.lumps
    }
    /// Returns the last matching directory index, as in patch lookup.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.lumps.iter().rposition(|l| l.is_named(name))
    }
    /// Safely retrieves bytes by directory index; invalid indexes return None.
    pub fn data(&self, index: usize) -> Option<&'a [u8]> {
        self.bytes.get(self.lumps.get(index)?.range.clone())
    }
}

/// An archive/index pair returned by ordered resource lookup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LumpLocation {
    /// Archive position in load order.
    pub archive: usize,
    /// Directory position within the archive.
    pub lump: usize,
}

/// Borrowed collection in load order; later archives take precedence.
#[derive(Clone, Copy, Debug)]
pub struct WadSet<'w, 'a> {
    archives: &'w [Wad<'a>],
}

impl<'w, 'a> WadSet<'w, 'a> {
    /// Uses the supplied sequence without sorting or copying it.
    pub const fn new(archives: &'w [Wad<'a>]) -> Self {
        Self { archives }
    }
    /// Archives in load order.
    pub const fn archives(&self) -> &'w [Wad<'a>] {
        self.archives
    }
    /// Finds the last match across and within archives.
    pub fn find(&self, name: &str) -> Option<LumpLocation> {
        self.archives
            .iter()
            .enumerate()
            .rev()
            .find_map(|(archive, wad)| wad.find(name).map(|lump| LumpLocation { archive, lump }))
    }
    /// Safely reads a location, including caller-constructed locations.
    pub fn data(&self, location: LumpLocation) -> Option<&'a [u8]> {
        self.archives.get(location.archive)?.data(location.lump)
    }
}

#[cfg(test)]
mod tests {
    use super::{WadError, region};

    #[test]
    fn range_arithmetic_never_wraps() {
        assert_eq!(
            region(usize::MAX, 1, usize::MAX, None),
            Err(WadError::ArithmeticOverflow)
        );
        assert_eq!(
            region(usize::MAX, 0, usize::MAX, None),
            Ok(usize::MAX..usize::MAX)
        );
    }
}
