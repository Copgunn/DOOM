//! Test-only lump patcher for the committed M3h5 rooms: rejection variants of the
//! diagonal admission are built by editing the committed WAD bytes (a reversed
//! wall, a detached wall end, a moved vertex, an unlisted line, a THING in the
//! void), so no synthetic geometry beyond the C harness's own enters the tests.
use std::convert::TryFrom;

/// A PWAD as an ordered list of named lumps.
#[derive(Clone, Debug)]
pub(crate) struct Lumps {
    pub lumps: Vec<(String, Vec<u8>)>,
}
fn i32_at(bytes: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn i16_at(bytes: &[u8], at: usize) -> i16 {
    i16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}
fn put_i16(bytes: &mut [u8], at: usize, value: i16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}
impl Lumps {
    pub fn parse(wad: &[u8]) -> Self {
        assert_eq!(&wad[..4], b"PWAD");
        let count = usize::try_from(i32_at(wad, 4)).unwrap();
        let directory = usize::try_from(i32_at(wad, 8)).unwrap();
        let lumps = (0..count)
            .map(|i| {
                let entry = directory + 16 * i;
                let at = usize::try_from(i32_at(wad, entry)).unwrap();
                let size = usize::try_from(i32_at(wad, entry + 4)).unwrap();
                let name = wad[entry + 8..entry + 16]
                    .iter()
                    .take_while(|b| **b != 0)
                    .map(|b| char::from(*b))
                    .collect();
                (name, wad[at..at + size].to_vec())
            })
            .collect();
        Self { lumps }
    }
    pub fn wad(&self) -> Vec<u8> {
        let mut data = b"PWAD".to_vec();
        data.extend((self.lumps.len() as i32).to_le_bytes());
        data.extend([0; 4]);
        let mut directory = Vec::new();
        for (name, bytes) in &self.lumps {
            directory.extend((data.len() as i32).to_le_bytes());
            directory.extend((bytes.len() as i32).to_le_bytes());
            let mut n = [0u8; 8];
            n[..name.len()].copy_from_slice(name.as_bytes());
            directory.extend(n);
            data.extend(bytes);
        }
        let at = (data.len() as i32).to_le_bytes();
        data[8..12].copy_from_slice(&at);
        data.extend(directory);
        data
    }
    pub fn map(&self) -> doom_map::Map {
        self.try_map().unwrap()
    }
    /// The parser's own verdict on the patched lumps.
    pub fn try_map(&self) -> Result<doom_map::Map, doom_map::MapError> {
        let bytes = self.wad();
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
        doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default())
    }
    /// Shorten the REJECT lump to `bytes`.
    pub fn truncate_reject(&mut self, bytes: usize) {
        self.lump("REJECT").truncate(bytes);
    }
    /// Count the linedefs listed by a BLOCKMAP cell.
    pub fn cell_lines(&mut self, cell: usize) -> Vec<u16> {
        let b = self.lump("BLOCKMAP");
        let words: Vec<u16> = b
            .chunks(2)
            .map(|w| u16::from_le_bytes([w[0], w[1]]))
            .collect();
        let mut at = usize::from(words[4 + cell]);
        let mut list = Vec::new();
        while words[at] != 0xFFFF {
            list.push(words[at]);
            at += 1;
        }
        list
    }
    /// Append `lines` to one cell's list (offsets rebuilt): extra listings the
    /// closed-cell rule allows, used to overfill a 2 × 2 window.
    pub fn list_lines_in_cell(&mut self, cell: usize, lines: &[u16]) {
        let b = self.lump("BLOCKMAP");
        let words: Vec<u16> = b
            .chunks(2)
            .map(|w| u16::from_le_bytes([w[0], w[1]]))
            .collect();
        let cells = usize::from(words[2]) * usize::from(words[3]);
        let mut lists: Vec<Vec<u16>> = Vec::new();
        for index in 0..cells {
            let mut at = usize::from(words[4 + index]);
            let mut list = Vec::new();
            while words[at] != 0xFFFF {
                list.push(words[at]);
                at += 1;
            }
            if index == cell {
                list.extend_from_slice(lines);
            }
            lists.push(list);
        }
        let mut out: Vec<u16> = words[..4].to_vec();
        let mut offsets = Vec::new();
        let mut body = Vec::new();
        for list in lists {
            offsets.push(u16::try_from(4 + cells + body.len()).unwrap());
            body.extend(list);
            body.push(0xFFFF);
        }
        out.extend(offsets);
        out.extend(body);
        *b = out.iter().flat_map(|w| w.to_le_bytes()).collect();
    }
    fn lump(&mut self, name: &str) -> &mut Vec<u8> {
        &mut self
            .lumps
            .iter_mut()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("lump {name}"))
            .1
    }
    pub fn line_count(&mut self) -> usize {
        self.lump("LINEDEFS").len() / 14
    }
    pub fn line_vertices(&mut self, line: usize) -> [i16; 2] {
        let l = self.lump("LINEDEFS");
        [i16_at(l, 14 * line), i16_at(l, 14 * line + 2)]
    }
    pub fn vertex(&mut self, index: i16) -> [i16; 2] {
        let v = self.lump("VERTEXES");
        let at = 4 * usize::try_from(index).unwrap();
        [i16_at(v, at), i16_at(v, at + 2)]
    }
    /// The first one-sided linedef whose endpoints are `a → b`.
    pub fn find_line(&mut self, a: [i16; 2], b: [i16; 2]) -> usize {
        (0..self.line_count())
            .find(|i| {
                let [va, vb] = self.line_vertices(*i);
                self.vertex(va) == a && self.vertex(vb) == b
            })
            .unwrap_or_else(|| panic!("line {a:?} -> {b:?}"))
    }
    /// Swap the linedef's endpoints: its front side now faces the other way (C3).
    pub fn reverse_line(&mut self, line: usize) {
        let l = self.lump("LINEDEFS");
        let [a, b] = [i16_at(l, 14 * line), i16_at(l, 14 * line + 2)];
        put_i16(l, 14 * line, b);
        put_i16(l, 14 * line + 2, a);
    }
    pub fn set_line_vertex(&mut self, line: usize, end: usize, vertex: i16) {
        put_i16(self.lump("LINEDEFS"), 14 * line + 2 * end, vertex);
    }
    pub fn set_line_flags(&mut self, line: usize, flags: u16) {
        put_i16(self.lump("LINEDEFS"), 14 * line + 4, flags as i16);
    }
    pub fn set_line_special(&mut self, line: usize, special: i16) {
        put_i16(self.lump("LINEDEFS"), 14 * line + 6, special);
    }
    pub fn push_vertex(&mut self, p: [i16; 2]) -> i16 {
        let v = self.lump("VERTEXES");
        let index = i16::try_from(v.len() / 4).unwrap();
        v.extend(p[0].to_le_bytes());
        v.extend(p[1].to_le_bytes());
        index
    }
    pub fn set_vertex(&mut self, index: i16, p: [i16; 2]) {
        let v = self.lump("VERTEXES");
        let at = 4 * usize::try_from(index).unwrap();
        put_i16(v, at, p[0]);
        put_i16(v, at + 2, p[1]);
    }
    /// `[floor, ceiling, light, special, tag]` with the ceiling flat name.
    pub fn set_sector(
        &mut self,
        sector: usize,
        heights: [i16; 2],
        special: i16,
        ceiling: &[u8; 8],
    ) {
        let s = self.lump("SECTORS");
        let at = 26 * sector;
        put_i16(s, at, heights[0]);
        put_i16(s, at + 2, heights[1]);
        s[at + 12..at + 20].copy_from_slice(ceiling);
        put_i16(s, at + 22, special);
    }
    /// Move the BLOCKMAP origin (header words 0 and 1).
    pub fn set_blockmap_origin(&mut self, origin: [i16; 2]) {
        let b = self.lump("BLOCKMAP");
        put_i16(b, 0, origin[0]);
        put_i16(b, 2, origin[1]);
    }
    /// Shift every node partition origin and bounding box along x.
    pub fn shift_nodes(&mut self, dx: i16) {
        let n = self.lump("NODES");
        for node in 0..n.len() / 28 {
            let at = 28 * node;
            for field in [0, 10, 12, 18, 20] {
                let v = i16_at(n, at + field);
                put_i16(n, at + field, v + dx);
            }
        }
    }
    pub fn add_thing(&mut self, thing: [i16; 5]) {
        let t = self.lump("THINGS");
        for v in thing {
            t.extend(v.to_le_bytes());
        }
    }
    pub fn clear_things(&mut self) {
        self.lump("THINGS").clear();
    }
    /// Remove the line from every BLOCKMAP cell list (offsets rebuilt).
    pub fn unlist_line(&mut self, line: usize) {
        let b = self.lump("BLOCKMAP");
        let words: Vec<u16> = b
            .chunks(2)
            .map(|w| u16::from_le_bytes([w[0], w[1]]))
            .collect();
        let cells = usize::from(words[2]) * usize::from(words[3]);
        let mut lists: Vec<Vec<u16>> = Vec::new();
        for cell in 0..cells {
            let mut at = usize::from(words[4 + cell]);
            let mut list = Vec::new();
            while words[at] != 0xFFFF {
                if usize::from(words[at]) != line || list.is_empty() {
                    list.push(words[at]);
                }
                at += 1;
            }
            lists.push(list);
        }
        let mut out: Vec<u16> = words[..4].to_vec();
        let mut offsets = Vec::new();
        let mut body = Vec::new();
        for list in lists {
            offsets.push(u16::try_from(4 + cells + body.len()).unwrap());
            body.extend(list);
            body.push(0xFFFF);
        }
        out.extend(offsets);
        out.extend(body);
        *b = out.iter().flat_map(|w| w.to_le_bytes()).collect();
    }
}
