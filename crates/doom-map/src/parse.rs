use crate::{
    Blockmap, BspChild, ErrorKind as K, LineDef, MapError, Node, Sector, Seg, SideDef, Subsector,
    Thing, Vertex,
};
use doom_types::LumpName;
use doom_wad::{Wad, WadSet};
use std::mem::size_of;

const NAMES: [&str; 10] = [
    "THINGS", "LINEDEFS", "SIDEDEFS", "VERTEXES", "SEGS", "SSECTORS", "NODES", "SECTORS", "REJECT",
    "BLOCKMAP",
];

/// Decoding/allocation policy; ordinary indexes follow signed Linux 1.10 fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MapLimits {
    /// Maximum bytes in one map lump.
    pub max_lump_bytes: usize,
    /// Maximum records per table, additionally capped at 32768 by the format.
    pub max_records: usize,
    /// Maximum grid cells.
    pub max_blockmap_cells: usize,
    /// Maximum BLOCKMAP words, including header/table/lists.
    pub max_blockmap_words: usize,
    /// Total requested decoded and scratch allocation bytes during one parse.
    pub max_decoded_bytes: usize,
}
impl Default for MapLimits {
    fn default() -> Self {
        Self {
            max_lump_bytes: 64 * 1024 * 1024,
            max_records: 32768,
            max_blockmap_cells: 262144,
            max_blockmap_words: 4 * 1024 * 1024,
            max_decoded_bytes: 128 * 1024 * 1024,
        }
    }
}

struct Budget {
    used: usize,
    limits: MapLimits,
}
impl Budget {
    fn vector<T>(&mut self, count: usize, lump: &'static str) -> Result<Vec<T>, MapError> {
        let size = count
            .checked_mul(size_of::<T>())
            .and_then(|n| self.used.checked_add(n))
            .ok_or_else(|| MapError::new(lump, None, K::ArithmeticOverflow))?;
        if size > self.limits.max_decoded_bytes {
            return Err(MapError::new(lump, None, K::LimitExceeded("decoded bytes")));
        }
        let mut v = Vec::new();
        v.try_reserve_exact(count)
            .map_err(|_| MapError::new(lump, None, K::AllocationFailed))?;
        self.used = size;
        Ok(v)
    }
}

/// Recognizes E#M# or MAP##, with ASCII-insensitive letters and NUL padding.
pub fn is_map_marker(name: LumpName) -> bool {
    let b = name.as_bytes();
    let end = b.iter().position(|b| *b == 0).unwrap_or(8);
    marker_bytes(&b[..end])
}
fn marker_bytes(b: &[u8]) -> bool {
    (b.len() == 4
        && b[0].eq_ignore_ascii_case(&b'E')
        && b[1].is_ascii_digit()
        && b[2].eq_ignore_ascii_case(&b'M')
        && b[3].is_ascii_digit())
        || (b.len() == 5
            && b[..3].eq_ignore_ascii_case(b"MAP")
            && b[3..].iter().all(u8::is_ascii_digit))
}

/// Immutable owned map. Slice accessors preserve validated relationships.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Map {
    things: Vec<Thing>,
    lines: Vec<LineDef>,
    sides: Vec<SideDef>,
    vertices: Vec<Vertex>,
    segs: Vec<Seg>,
    subsectors: Vec<Subsector>,
    nodes: Vec<Node>,
    sectors: Vec<Sector>,
    reject: Vec<u8>,
    blockmap: Blockmap,
}
impl Map {
    /// Loads the last matching marker across archives. Map lumps must belong to
    /// that archive's contiguous group; malformed overrides do not fall back.
    pub fn load(wads: &WadSet<'_, '_>, marker: &str, limits: MapLimits) -> Result<Self, MapError> {
        if !marker_bytes(marker.as_bytes()) {
            return Err(MapError::new("MAP", None, K::InvalidMarker));
        }
        let location = wads
            .find(marker)
            .ok_or_else(|| MapError::new("MAP", None, K::MarkerNotFound))?;
        Self::from_index(&wads.archives()[location.archive], location.lump, limits)
    }
    /// Loads a classic ordered map group in one archive.
    pub fn from_wad(wad: &Wad<'_>, marker: &str, limits: MapLimits) -> Result<Self, MapError> {
        Self::load(&WadSet::new(std::slice::from_ref(wad)), marker, limits)
    }
    fn from_index(wad: &Wad<'_>, index: usize, limits: MapLimits) -> Result<Self, MapError> {
        if wad
            .lumps()
            .get(index + 1)
            .is_some_and(|l| l.is_named("TEXTMAP"))
        {
            return Err(MapError::new("MAP", None, K::UnsupportedFormat("UDMF")));
        }
        if wad
            .lumps()
            .get(index + 11)
            .is_some_and(|l| l.is_named("BEHAVIOR"))
        {
            return Err(MapError::new("MAP", None, K::UnsupportedFormat("Hexen")));
        }
        let mut data = [&[][..]; 10];
        for (i, name) in NAMES.iter().enumerate() {
            let entry = wad
                .lumps()
                .get(index + i + 1)
                .ok_or_else(|| MapError::new("MAP", None, K::MissingLump(name)))?;
            if !entry.is_named(name) {
                return Err(MapError::new("MAP", None, K::MissingLump(name)));
            }
            data[i] = wad
                .data(index + i + 1)
                .ok_or_else(|| MapError::new("MAP", None, K::MissingLump(name)))?;
            if data[i].len() > limits.max_lump_bytes {
                return Err(MapError::new(name, None, K::LimitExceeded("lump bytes")));
            }
        }
        for magic in [b"XNOD", b"ZNOD", b"XGLN", b"ZGLN", b"xNd4", b"xNd5"] {
            if data[6].starts_with(magic) {
                return Err(MapError::new(
                    "NODES",
                    None,
                    K::UnsupportedFormat("extended nodes"),
                ));
            }
        }
        let mut budget = Budget { used: 0, limits };
        let things = records(data[0], 10, "THINGS", &mut budget, |b| Thing {
            position: vertex(b, 0),
            angle: s(b, 4),
            kind: s(b, 6),
            options: u(b, 8),
        })?;
        let lines = records(data[1], 14, "LINEDEFS", &mut budget, |b| LineDef {
            vertices: [u(b, 0), u(b, 2)],
            flags: u(b, 4),
            special: s(b, 6),
            tag: s(b, 8),
            sides: [side(u(b, 10)), side(u(b, 12))],
        })?;
        let sides = records(data[2], 30, "SIDEDEFS", &mut budget, |b| SideDef {
            offsets: [s(b, 0), s(b, 2)],
            textures: [name(b, 4), name(b, 12), name(b, 20)],
            sector: u(b, 28),
        })?;
        let vertices = records(data[3], 4, "VERTEXES", &mut budget, |b| vertex(b, 0))?;
        let segs = records(data[4], 12, "SEGS", &mut budget, |b| Seg {
            vertices: [u(b, 0), u(b, 2)],
            angle: u(b, 4),
            linedef: u(b, 6),
            side: u(b, 8),
            offset: s(b, 10),
        })?;
        let subsectors = records(data[5], 4, "SSECTORS", &mut budget, |b| Subsector {
            seg_count: u(b, 0),
            first_seg: u(b, 2),
        })?;
        let nodes = records(data[6], 28, "NODES", &mut budget, |b| Node {
            origin: vertex(b, 0),
            delta: vertex(b, 4),
            bounds: [
                [s(b, 8), s(b, 10), s(b, 12), s(b, 14)],
                [s(b, 16), s(b, 18), s(b, 20), s(b, 22)],
            ],
            children: [child(u(b, 24)), child(u(b, 26))],
        })?;
        let sectors = records(data[7], 26, "SECTORS", &mut budget, |b| Sector {
            heights: [s(b, 0), s(b, 2)],
            flats: [name(b, 4), name(b, 12)],
            light: s(b, 20),
            special: s(b, 22),
            tag: s(b, 24),
        })?;
        for (len, name) in [
            (lines.len(), "LINEDEFS"),
            (sides.len(), "SIDEDEFS"),
            (vertices.len(), "VERTEXES"),
            (segs.len(), "SEGS"),
            (subsectors.len(), "SSECTORS"),
            (sectors.len(), "SECTORS"),
        ] {
            if len == 0 {
                return Err(MapError::new(name, None, K::EmptyGeometry));
            }
        }
        let expected = sectors
            .len()
            .checked_mul(sectors.len())
            .and_then(|n| n.checked_add(7))
            .map(|n| n / 8)
            .ok_or_else(|| MapError::new("REJECT", None, K::ArithmeticOverflow))?;
        if data[8].len() < expected {
            return Err(MapError::new(
                "REJECT",
                None,
                K::RejectTooShort {
                    expected,
                    actual: data[8].len(),
                },
            ));
        }
        let mut reject = budget.vector(data[8].len(), "REJECT")?;
        reject.extend_from_slice(data[8]);
        let blockmap = blockmap(data[9], lines.len(), &mut budget)?;
        let map = Self {
            things,
            lines,
            sides,
            vertices,
            segs,
            subsectors,
            nodes,
            sectors,
            reject,
            blockmap,
        };
        map.validate(&mut budget)?;
        Ok(map)
    }
    fn validate(&self, budget: &mut Budget) -> Result<(), MapError> {
        for (i, side) in self.sides.iter().enumerate() {
            reference(side.sector, self.sectors.len(), "SIDEDEFS", i, "sector")?;
        }
        for (i, line) in self.lines.iter().enumerate() {
            for v in line.vertices {
                reference(v, self.vertices.len(), "LINEDEFS", i, "vertex")?;
            }
            for side in line.sides.into_iter().flatten() {
                reference(side, self.sides.len(), "LINEDEFS", i, "sidedef")?;
            }
            if line.sides[0].is_none() || (line.flags & 4 != 0 && line.sides[1].is_none()) {
                return Err(MapError::new(
                    "LINEDEFS",
                    Some(i),
                    K::InvalidValue {
                        field: "required sidedef",
                        value: -1,
                    },
                ));
            }
        }
        for (i, seg) in self.segs.iter().enumerate() {
            for v in seg.vertices {
                reference(v, self.vertices.len(), "SEGS", i, "vertex")?;
            }
            reference(seg.linedef, self.lines.len(), "SEGS", i, "linedef")?;
            if seg.side > 1
                || self.lines[usize::from(seg.linedef)].sides[usize::from(seg.side)].is_none()
            {
                return Err(MapError::new(
                    "SEGS",
                    Some(i),
                    K::InvalidValue {
                        field: "side",
                        value: i64::from(seg.side),
                    },
                ));
            }
        }
        // Prefix transition counts bound validation even for overlapping ranges.
        let mut changes = budget.vector(self.segs.len(), "SSECTORS")?;
        let mut count = 0_u32;
        let mut previous = None;
        for seg in &self.segs {
            let side = self.lines[usize::from(seg.linedef)].sides[usize::from(seg.side)]
                .ok_or_else(|| MapError::new("SEGS", None, K::InvalidSubsector))?;
            let sector = self.sides[usize::from(side)].sector;
            if previous.is_some_and(|p| p != sector) {
                count += 1;
            }
            changes.push(count);
            previous = Some(sector);
        }
        for (i, ss) in self.subsectors.iter().enumerate() {
            let first = usize::from(ss.first_seg);
            let end = first + usize::from(ss.seg_count);
            if ss.seg_count == 0
                || ss.seg_count > i16::MAX as u16
                || ss.first_seg > i16::MAX as u16
                || end > self.segs.len()
                || changes[first] != changes[end - 1]
            {
                return Err(MapError::new("SSECTORS", Some(i), K::InvalidSubsector));
            }
        }
        for (i, node) in self.nodes.iter().enumerate() {
            if node.delta.x == 0 && node.delta.y == 0 {
                return Err(MapError::new(
                    "NODES",
                    Some(i),
                    K::InvalidValue {
                        field: "partition delta",
                        value: 0,
                    },
                ));
            }
            for child in node.children {
                match child {
                    BspChild::Node(n) => reference(n, self.nodes.len(), "NODES", i, "node child")?,
                    BspChild::Subsector(n) => {
                        reference(n, self.subsectors.len(), "NODES", i, "subsector child")?
                    }
                }
            }
        }
        if self.nodes.is_empty() && self.subsectors.len() != 1 {
            return Err(MapError::new(
                "NODES",
                None,
                K::InvalidValue {
                    field: "subsectors without nodes",
                    value: self.subsectors.len() as i64,
                },
            ));
        }
        acyclic(&self.nodes, budget)
    }
    /// Unspawned things in original order.
    pub fn things(&self) -> &[Thing] {
        &self.things
    }
    /// Validated linedefs.
    pub fn linedefs(&self) -> &[LineDef] {
        &self.lines
    }
    /// Validated sidedefs.
    pub fn sidedefs(&self) -> &[SideDef] {
        &self.sides
    }
    /// Signed map vertices.
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }
    /// Validated split segments.
    pub fn segs(&self) -> &[Seg] {
        &self.segs
    }
    /// Nonempty validated ranges.
    pub fn subsectors(&self) -> &[Subsector] {
        &self.subsectors
    }
    /// Acyclic nodes; last is root.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    /// Static sectors.
    pub fn sectors(&self) -> &[Sector] {
        &self.sectors
    }
    /// Full visibility bytes, including allowed trailing data.
    pub fn reject(&self) -> &[u8] {
        &self.reject
    }
    /// Visibility bit; invalid sector indexes return None.
    pub fn is_rejected(&self, from: usize, to: usize) -> Option<bool> {
        if from >= self.sectors.len() || to >= self.sectors.len() {
            return None;
        }
        let bit = from.checked_mul(self.sectors.len())?.checked_add(to)?;
        Some(self.reject.get(bit / 8)? & (1 << (bit % 8)) != 0)
    }
    /// Validated cell grid with shared lists.
    pub const fn blockmap(&self) -> &Blockmap {
        &self.blockmap
    }
}

fn u(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([b[p], b[p + 1]])
}
fn s(b: &[u8], p: usize) -> i16 {
    i16::from_le_bytes([b[p], b[p + 1]])
}
fn vertex(b: &[u8], p: usize) -> Vertex {
    Vertex {
        x: s(b, p),
        y: s(b, p + 2),
    }
}
fn name(b: &[u8], p: usize) -> LumpName {
    let mut n = [0; 8];
    n.copy_from_slice(&b[p..p + 8]);
    LumpName::from_bytes(n)
}
fn side(n: u16) -> Option<u16> {
    if n == u16::MAX { None } else { Some(n) }
}
fn child(n: u16) -> BspChild {
    if n & 0x8000 == 0 {
        BspChild::Node(n)
    } else {
        BspChild::Subsector(n & 0x7fff)
    }
}
fn reference(
    n: u16,
    bound: usize,
    lump: &'static str,
    record: usize,
    field: &'static str,
) -> Result<(), MapError> {
    if n > i16::MAX as u16 || usize::from(n) >= bound {
        Err(MapError::new(
            lump,
            Some(record),
            K::InvalidReference {
                field,
                value: i64::from(n as i16),
                bound,
            },
        ))
    } else {
        Ok(())
    }
}
fn records<T>(
    bytes: &[u8],
    width: usize,
    lump: &'static str,
    budget: &mut Budget,
    decode: impl Fn(&[u8]) -> T,
) -> Result<Vec<T>, MapError> {
    if bytes.len() % width != 0 {
        return Err(MapError::new(
            lump,
            None,
            K::RecordSize {
                size: bytes.len(),
                width,
            },
        ));
    }
    let count = bytes.len() / width;
    if count > budget.limits.max_records || count > 32768 {
        return Err(MapError::new(lump, None, K::LimitExceeded("records")));
    }
    let mut records = budget.vector(count, lump)?;
    records.extend(bytes.chunks_exact(width).map(decode));
    Ok(records)
}
fn acyclic(nodes: &[Node], budget: &mut Budget) -> Result<(), MapError> {
    let mut colors = budget.vector(nodes.len(), "NODES")?;
    colors.resize(nodes.len(), 0_u8);
    let mut stack: Vec<(usize, usize)> = budget.vector(nodes.len(), "NODES")?;
    for start in 0..nodes.len() {
        if colors[start] != 0 {
            continue;
        }
        colors[start] = 1;
        stack.push((start, 0));
        while let Some((index, next)) = stack.last_mut() {
            if *next == 2 {
                colors[*index] = 2;
                stack.pop();
                continue;
            }
            let child = nodes[*index].children[*next];
            *next += 1;
            if let BspChild::Node(n) = child {
                let n = usize::from(n);
                if colors[n] == 1 {
                    return Err(MapError::new("NODES", Some(n), K::BspCycle));
                }
                if colors[n] == 0 {
                    colors[n] = 1;
                    stack.push((n, 0));
                }
            }
        }
    }
    Ok(())
}
fn blockmap(bytes: &[u8], lines: usize, budget: &mut Budget) -> Result<Blockmap, MapError> {
    let error = |reason| MapError::new("BLOCKMAP", None, K::InvalidBlockmap(reason));
    if bytes.len() < 8 || bytes.len() % 2 != 0 {
        return Err(error("header/word size"));
    }
    let width = s(bytes, 4);
    let height = s(bytes, 6);
    if width <= 0 || height <= 0 {
        return Err(error("nonpositive dimensions"));
    }
    let cells = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| error("grid size overflow"))?;
    if cells > budget.limits.max_blockmap_cells
        || bytes.len() / 2 > budget.limits.max_blockmap_words
    {
        return Err(MapError::new(
            "BLOCKMAP",
            None,
            K::LimitExceeded("blockmap"),
        ));
    }
    let table_end = 4_usize
        .checked_add(cells)
        .ok_or_else(|| error("table size overflow"))?;
    if table_end > bytes.len() / 2 {
        return Err(error("truncated cell table"));
    }
    let mut words = budget.vector(bytes.len() / 2, "BLOCKMAP")?;
    words.extend(bytes.chunks_exact(2).map(|b| u(b, 0)));
    // Each suffix is validated once: shared offsets cannot multiply work.
    let mut ends = budget.vector(words.len(), "BLOCKMAP")?;
    ends.resize(words.len(), usize::MAX);
    for i in (table_end..words.len()).rev() {
        if words[i] == u16::MAX {
            ends[i] = i;
        } else if words[i] <= i16::MAX as u16
            && usize::from(words[i]) < lines
            && i + 1 < words.len()
        {
            ends[i] = ends[i + 1];
        }
    }
    let mut ranges = budget.vector(cells, "BLOCKMAP")?;
    for i in 0..cells {
        let offset = words[4 + i];
        let start = usize::from(offset);
        if offset > i16::MAX as u16 || start < table_end || start >= words.len() {
            return Err(MapError::new(
                "BLOCKMAP",
                Some(i),
                K::InvalidBlockmap("list offset"),
            ));
        }
        if words[start] != 0 || ends[start] == usize::MAX {
            return Err(MapError::new(
                "BLOCKMAP",
                Some(i),
                K::InvalidBlockmap("dummy, line index, or terminator"),
            ));
        }
        ranges.push(start..ends[start]);
    }
    Ok(Blockmap {
        origin: vertex(bytes, 0),
        width: width as u16,
        height: height as u16,
        words,
        ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::{Budget, K, MapLimits};

    #[test]
    fn decoded_allocation_size_overflow_is_rejected_before_reserving() {
        let mut budget = Budget {
            used: 0,
            limits: MapLimits {
                max_decoded_bytes: usize::MAX,
                ..MapLimits::default()
            },
        };
        assert_eq!(
            budget.vector::<u64>(usize::MAX, "MAP").unwrap_err().kind,
            K::ArithmeticOverflow
        );
    }
}
