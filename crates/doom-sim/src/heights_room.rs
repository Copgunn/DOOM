//! Test-input assembly only: walled rooms of axis-aligned sectors with static
//! heights, mirroring `tools/reference/m3h4_geometry.py` so rejection variants exist
//! without committed bytes. A strip of rectangular sectors along x inside a walled
//! rectangle: one-sided `ML_BLOCKING` boundary walls with the room on their right,
//! split into pieces of at most `segment` units; one two-sided line between adjacent
//! sectors (`ML_TWOSIDED` plus the caller's extra flags); one subsector per sector
//! with segs in the source side convention; a guillotine BSP whose root is the last
//! node; a REJECT table with optional set pairs; and a cell-exact BLOCKMAP whose
//! origin is the rectangle's minimum corner. Neither builder reads the other's output.
use std::collections::BTreeMap;

/// A deliberate departure from the admitted domain, applied while assembling the WAD.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fault {
    /// Move the top vertex of this interior boundary 32 units east: a diagonal portal.
    DiagonalPortal(usize),
    /// Replace this linedef's flags.
    Flags(usize, u16),
    /// Give the west boundary wall a back side and `ML_TWOSIDED`: a two-sided line on
    /// the boundary that no one-sided wall covers.
    TwoSidedBoundary,
    /// Keep this linedef but list it in no BLOCKMAP cell.
    Unlisted(usize),
    /// A line special on this linedef.
    LineSpecial(usize),
    /// A sector special on this sector.
    SectorSpecial(usize),
    /// An `F_SKY1` ceiling on this sector.
    Sky(usize),
    /// Replace this sector's heights (beyond ±512 or inverted).
    Heights(usize, [i16; 2]),
    /// Drop `ML_TWOSIDED` from this linedef while keeping its back side.
    InconsistentTwoSided(usize),
}

pub(crate) struct HeightsRoom {
    /// The `n + 1` x boundaries of `n` sectors.
    pub spans: Vec<i16>,
    /// `[floor, ceiling]` per sector.
    pub heights: Vec<[i16; 2]>,
    /// The y extent shared by every sector.
    pub y: [i16; 2],
    /// Extra linedef flags of an interior boundary `1..n`.
    pub portals: BTreeMap<usize, u16>,
    /// REJECT pairs, set in both directions.
    pub reject: Vec<[usize; 2]>,
    pub things: Vec<[i16; 5]>,
    pub segment: i16,
    pub faults: Vec<Fault>,
}
struct Line {
    a: usize,
    b: usize,
    front: usize,
    back: Option<usize>,
    flags: u16,
    special: i16,
}
impl HeightsRoom {
    pub fn new(spans: &[i16], heights: &[[i16; 2]]) -> Self {
        assert_eq!(spans.len(), heights.len() + 1);
        Self {
            spans: spans.to_vec(),
            heights: heights.to_vec(),
            y: [-512, 512],
            portals: BTreeMap::new(),
            reject: Vec::new(),
            things: Vec::new(),
            segment: 1024,
            faults: Vec::new(),
        }
    }
    pub fn with_portals(mut self, portals: &[(usize, u16)]) -> Self {
        self.portals = portals.iter().copied().collect();
        self
    }
    pub fn with_things(mut self, things: &[[i16; 5]]) -> Self {
        self.things = things.to_vec();
        self
    }
    pub fn with_fault(mut self, fault: Fault) -> Self {
        self.faults.push(fault);
        self
    }
    fn has(&self, fault: Fault) -> bool {
        self.faults.contains(&fault)
    }
    pub fn wad(&self) -> Vec<u8> {
        let n = self.heights.len();
        let [y0, y1] = self.y;
        let mut vertices: Vec<[i16; 2]> = Vec::new();
        let mut vertex = |p: [i16; 2]| -> usize {
            if let Some(i) = vertices.iter().position(|v| *v == p) {
                i
            } else {
                vertices.push(p);
                vertices.len() - 1
            }
        };
        let mut lines: Vec<Line> = Vec::new();
        let segment = i32::from(self.segment);
        let mut add = |a: [i16; 2], b: [i16; 2], front: usize, back: Option<usize>, flags: u16| {
            let length = (i32::from(b[0]) - i32::from(a[0])).abs()
                + (i32::from(b[1]) - i32::from(a[1])).abs();
            let pieces = ((length + segment - 1) / segment).max(1);
            let at = |p: i16, q: i16, k: i32| {
                let d = (i32::from(q) - i32::from(p)) * k;
                // Truncating division, as the Python builder's `tdiv`.
                (i32::from(p) + d / pieces) as i16
            };
            for k in 0..pieces {
                lines.push(Line {
                    a: vertex([at(a[0], b[0], k), at(a[1], b[1], k)]),
                    b: vertex([at(a[0], b[0], k + 1), at(a[1], b[1], k + 1)]),
                    front,
                    back,
                    flags,
                    special: 0,
                });
            }
        };
        // Clockwise perimeter keeps each sector on its walls' right (front) side.
        add([self.spans[0], y0], [self.spans[0], y1], 0, None, 1);
        for i in 0..n {
            add([self.spans[i], y1], [self.spans[i + 1], y1], i, None, 1);
        }
        add([self.spans[n], y1], [self.spans[n], y0], n - 1, None, 1);
        for i in (0..n).rev() {
            add([self.spans[i + 1], y0], [self.spans[i], y0], i, None, 1);
        }
        // Interior boundaries run +y, so their right (front) side is the eastern sector.
        let portal_lines: Vec<usize> = (1..n)
            .map(|k| {
                let flags = 4 | self.portals.get(&k).copied().unwrap_or(0);
                add(
                    [self.spans[k], y0],
                    [self.spans[k], y1],
                    k,
                    Some(k - 1),
                    flags,
                );
                lines.len() - 1
            })
            .collect();
        drop(add);
        for fault in &self.faults {
            match *fault {
                Fault::DiagonalPortal(k) => {
                    let line = &lines[portal_lines[k - 1]];
                    let top = line.b;
                    let moved = [vertices[top][0] + 32, vertices[top][1]];
                    vertices.push(moved);
                    lines[portal_lines[k - 1]].b = vertices.len() - 1;
                }
                Fault::Flags(i, flags) => lines[i].flags = flags,
                Fault::TwoSidedBoundary => {
                    lines[0].back = Some(0);
                    lines[0].flags |= 4;
                }
                Fault::LineSpecial(i) => lines[i].special = 1,
                Fault::InconsistentTwoSided(i) => lines[i].flags &= !4,
                Fault::Unlisted(_)
                | Fault::SectorSpecial(_)
                | Fault::Sky(_)
                | Fault::Heights(_, _) => {}
            }
        }
        let words = |v: &[i16]| v.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>();
        let name = |s: &str| {
            let mut b = s.as_bytes().to_vec();
            b.resize(8, 0);
            b
        };
        let mut linedefs = Vec::new();
        let mut sidedefs = Vec::new();
        let mut sides = 0_i16;
        for l in &lines {
            let mut indexes = [-1_i16; 2];
            for (slot, sector) in [Some(l.front), l.back].into_iter().enumerate() {
                let Some(sector) = sector else { continue };
                indexes[slot] = sides;
                sides += 1;
                sidedefs.extend(words(&[0, 0]));
                sidedefs.extend(name("-"));
                sidedefs.extend(name("-"));
                sidedefs.extend(name("TESTWALL"));
                sidedefs.extend(words(&[sector as i16]));
            }
            linedefs.extend(words(&[
                l.a as i16,
                l.b as i16,
                l.flags as i16,
                l.special,
                0,
                indexes[0],
                indexes[1],
            ]));
        }
        let mut segs = Vec::new();
        let mut subsectors = Vec::new();
        let mut first = 0_i16;
        for sector in 0..n {
            let mut count = 0_i16;
            for (i, l) in lines.iter().enumerate() {
                let (side, a, b) = if l.front == sector {
                    (0, l.a, l.b)
                } else if l.back == Some(sector) {
                    (1, l.b, l.a)
                } else {
                    continue;
                };
                segs.extend(words(&[a as i16, b as i16, 0, i as i16, side, 0]));
                count += 1;
            }
            subsectors.extend(words(&[count, first]));
            first += count;
        }
        let mut nodes = Vec::new();
        let mut node_count = 0_u16;
        fn build(
            spans: &[i16],
            y: [i16; 2],
            lo: usize,
            hi: usize,
            nodes: &mut Vec<u8>,
            count: &mut u16,
        ) -> u16 {
            if lo == hi {
                return 0x8000 | lo as u16;
            }
            let mid = (lo + hi + 1) / 2;
            let left = build(spans, y, lo, mid - 1, nodes, count);
            let right = build(spans, y, mid, hi, nodes, count);
            let x = spans[mid];
            let words = |v: &[i16]| v.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>();
            nodes.extend(words(&[
                x,
                y[0],
                0,
                y[1] - y[0],
                y[1],
                y[0],
                x,
                spans[hi + 1],
                y[1],
                y[0],
                spans[lo],
                x,
            ]));
            nodes.extend(right.to_le_bytes());
            nodes.extend(left.to_le_bytes());
            *count += 1;
            *count - 1
        }
        build(&self.spans, self.y, 0, n - 1, &mut nodes, &mut node_count);
        let mut sectors = Vec::new();
        for (i, h) in self.heights.iter().enumerate() {
            let mut heights = *h;
            let mut ceiling = "TESTCEIL";
            let mut special = 0;
            for fault in &self.faults {
                match *fault {
                    Fault::Heights(s, v) if s == i => heights = v,
                    Fault::Sky(s) if s == i => ceiling = "F_SKY1",
                    Fault::SectorSpecial(s) if s == i => special = 9,
                    _ => {}
                }
            }
            sectors.extend(words(&heights));
            sectors.extend(name("TESTFLR0"));
            sectors.extend(name(ceiling));
            sectors.extend(words(&[160, special, 0]));
        }
        let mut reject = vec![0_u8; (n * n).div_ceil(8)];
        for [a, b] in &self.reject {
            for pair in [a * n + b, b * n + a] {
                reject[pair >> 3] |= 1 << (pair & 7);
            }
        }
        let [ox, oy] = [self.spans[0], y0];
        let width = ((i32::from(self.spans[n]) - i32::from(ox)) / 128 + 1) as i16;
        let height = ((i32::from(y1) - i32::from(oy)) / 128 + 1) as i16;
        let cells = usize::from(width as u16) * usize::from(height as u16);
        let mut lists: Vec<u16> = Vec::new();
        let mut offsets = Vec::with_capacity(cells);
        for cell in 0..cells {
            let cx = i32::from(ox) + (cell % usize::from(width as u16)) as i32 * 128;
            let cy = i32::from(oy) + (cell / usize::from(width as u16)) as i32 * 128;
            offsets.push((4 + cells + lists.len()) as u16);
            lists.push(0);
            for (i, l) in lines.iter().enumerate() {
                let (a, b) = (vertices[l.a], vertices[l.b]);
                let (lx0, lx1) = (i32::from(a[0].min(b[0])), i32::from(a[0].max(b[0])));
                let (ly0, ly1) = (i32::from(a[1].min(b[1])), i32::from(a[1].max(b[1])));
                if lx1 >= cx
                    && lx0 <= cx + 128
                    && ly1 >= cy
                    && ly0 <= cy + 128
                    && i != 0
                    && !self.has(Fault::Unlisted(i))
                {
                    lists.push(i as u16);
                }
            }
            lists.push(u16::MAX);
        }
        let mut blockmap = words(&[ox, oy, width, height]);
        for o in offsets.iter().chain(lists.iter()) {
            blockmap.extend(o.to_le_bytes());
        }
        let lumps: Vec<(&str, Vec<u8>)> = vec![
            ("MAP01", Vec::new()),
            (
                "THINGS",
                self.things.iter().flat_map(|t| words(t)).collect(),
            ),
            ("LINEDEFS", linedefs),
            ("SIDEDEFS", sidedefs),
            ("VERTEXES", vertices.iter().flat_map(|v| words(v)).collect()),
            ("SEGS", segs),
            ("SSECTORS", subsectors),
            ("NODES", nodes),
            ("SECTORS", sectors),
            ("REJECT", reject),
            ("BLOCKMAP", blockmap),
        ];
        let mut data = b"PWAD".to_vec();
        data.extend((lumps.len() as u32).to_le_bytes());
        data.extend(0_u32.to_le_bytes());
        let mut directory = Vec::new();
        for (n, b) in &lumps {
            directory.extend((data.len() as u32).to_le_bytes());
            directory.extend((b.len() as u32).to_le_bytes());
            directory.extend(name(n));
            data.extend(b);
        }
        let at = (data.len() as u32).to_le_bytes();
        data[8..12].copy_from_slice(&at);
        data.extend(directory);
        data
    }
    pub fn map(&self) -> doom_map::Map {
        let bytes = self.wad();
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
        doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
    }
}
