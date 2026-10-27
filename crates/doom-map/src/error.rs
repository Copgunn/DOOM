use std::{error::Error, fmt};

/// Machine-readable map validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// Requested name is not E#M# or MAP##.
    InvalidMarker,
    /// No matching marker exists.
    MarkerNotFound,
    /// Required ordered lump is missing or misnamed.
    MissingLump(&'static str),
    /// Unsupported format signature.
    UnsupportedFormat(&'static str),
    /// Incomplete fixed-width record.
    RecordSize {
        /// Actual byte length.
        size: usize,
        /// Record width.
        width: usize,
    },
    /// Essential geometry table is empty.
    EmptyGeometry,
    /// Configured resource bound exceeded.
    LimitExceeded(&'static str),
    /// Checked size arithmetic overflowed.
    ArithmeticOverflow,
    /// Memory reservation failed.
    AllocationFailed,
    /// Index does not address the required table.
    InvalidReference {
        /// Index field.
        field: &'static str,
        /// Original signed value.
        value: i64,
        /// Referenced table length.
        bound: usize,
    },
    /// Invalid scalar or sentinel.
    InvalidValue {
        /// Field name.
        field: &'static str,
        /// Original value.
        value: i64,
    },
    /// Empty/invalid seg range or inconsistent front sectors.
    InvalidSubsector,
    /// BSP cycle, including in unreachable nodes.
    BspCycle,
    /// Visibility table does not cover all ordered sector pairs.
    RejectTooShort {
        /// Minimum byte count.
        expected: usize,
        /// Available byte count.
        actual: usize,
    },
    /// Invalid grid/header/list structure.
    InvalidBlockmap(&'static str),
}

/// Contextual error without private archive paths or contents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapError {
    /// Format lump name, or MAP for marker/group errors.
    pub lump: &'static str,
    /// Record/cell index, if applicable.
    pub record: Option<usize>,
    /// Failure category.
    pub kind: ErrorKind,
}
impl MapError {
    pub(crate) const fn new(lump: &'static str, record: Option<usize>, kind: ErrorKind) -> Self {
        Self { lump, record, kind }
    }
}
impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "map {} record {:?}: {:?}",
            self.lump, self.record, self.kind
        )
    }
}
impl Error for MapError {}
