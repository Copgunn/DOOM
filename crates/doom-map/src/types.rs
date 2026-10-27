use doom_types::LumpName;
use std::ops::Range;

/// Signed map coordinates, without runtime fixed-point conversion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Vertex {
    /// Horizontal coordinate.
    pub x: i16,
    /// Vertical coordinate.
    pub y: i16,
}
/// Unspawned map thing; no gameplay interpretation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Thing {
    /// Signed position.
    pub position: Vertex,
    /// Signed degrees.
    pub angle: i16,
    /// Signed editor type.
    pub kind: i16,
    /// Option bits.
    pub options: u16,
}
/// Linedef with validated indexes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineDef {
    /// Start/end vertex indexes.
    pub vertices: [u16; 2],
    /// Flag bits, including two-sided bit 4.
    pub flags: u16,
    /// Signed action special.
    pub special: i16,
    /// Signed tag.
    pub tag: i16,
    /// Front/back sidedefs; disk -1 becomes None. Front is always present.
    pub sides: [Option<u16>; 2],
}
/// Wall definition, without resource resolution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SideDef {
    /// Signed horizontal/vertical offsets.
    pub offsets: [i16; 2],
    /// Exact upper/lower/middle texture names.
    pub textures: [LumpName; 3],
    /// Validated sector index.
    pub sector: u16,
}
/// Static sector, separate from runtime state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sector {
    /// Signed floor/ceiling heights.
    pub heights: [i16; 2],
    /// Exact floor/ceiling flat names.
    pub flats: [LumpName; 2],
    /// Signed light level.
    pub light: i16,
    /// Signed special.
    pub special: i16,
    /// Signed tag.
    pub tag: i16,
}
/// Split segment with validated references.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Seg {
    /// Start/end vertices.
    pub vertices: [u16; 2],
    /// Original binary angle bit pattern.
    pub angle: u16,
    /// Parent linedef.
    pub linedef: u16,
    /// Side selector, 0 or 1 with an existing sidedef.
    pub side: u16,
    /// Signed texture offset in map units.
    pub offset: i16,
}
/// Nonempty contiguous seg range with a consistent front sector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Subsector {
    /// Number of consecutive segs.
    pub seg_count: u16,
    /// First seg index.
    pub first_seg: u16,
}
/// Node or leaf, distinguished by the disk high bit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BspChild {
    /// Validated node index.
    Node(u16),
    /// Validated subsector index.
    Subsector(u16),
}
/// BSP node; root is last. One-leaf maps can omit nodes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Node {
    /// Signed partition origin.
    pub origin: Vertex,
    /// Signed partition delta.
    pub delta: Vertex,
    /// Per-child boxes in top/bottom/left/right order.
    pub bounds: [[i16; 4]; 2],
    /// Acyclic children.
    pub children: [BspChild; 2],
}
/// Validated grid sharing list storage across cells.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Blockmap {
    pub(crate) origin: Vertex,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) words: Vec<u16>,
    pub(crate) ranges: Vec<Range<usize>>,
}
impl Blockmap {
    /// Immutable decoded storage, including header, cell table and unused words.
    /// Referenced lists are validated; unused words need not be valid line indexes.
    pub fn words(&self) -> &[u16] {
        &self.words
    }
    /// Validated row-major cell ranges into `words`, excluding terminators.
    /// Shared ranges are retained for linear canonicalization without list copies.
    pub fn cell_ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }
    /// Signed grid origin.
    pub const fn origin(&self) -> Vertex {
        self.origin
    }
    /// Width/height in 128-map-unit cells.
    pub const fn dimensions(&self) -> [u16; 2] {
        [self.width, self.height]
    }
    /// Safe cell list. Includes the leading zero as linedef 0, matching Linux
    /// 1.10's iterator, which does not skip that conventional dummy word.
    pub fn cell_lines(
        &self,
        x: usize,
        y: usize,
    ) -> Option<impl ExactSizeIterator<Item = u16> + '_> {
        if x >= usize::from(self.width) || y >= usize::from(self.height) {
            return None;
        }
        let index = y.checked_mul(usize::from(self.width))?.checked_add(x)?;
        Some(
            self.words
                .get(self.ranges.get(index)?.clone())?
                .iter()
                .copied(),
        )
    }
}
