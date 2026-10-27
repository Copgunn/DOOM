//! Original synthetic fixtures, GPL-2.0-only. Shared by tests and seed generator.
#![allow(dead_code)] // Each consumer uses a different subset of these helpers.

pub type Lump = ([u8; 8], Vec<u8>);

pub fn name(value: &str) -> [u8; 8] {
    let mut out = [0; 8];
    out[..value.len()].copy_from_slice(value.as_bytes());
    out
}

pub fn words(values: &[i16]) -> Vec<u8> {
    values.iter().flat_map(|n| n.to_le_bytes()).collect()
}

pub fn set_word(bytes: &mut [u8], offset: usize, value: i16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub fn set_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn wad(magic: [u8; 4], lumps: &[Lump]) -> Vec<u8> {
    let mut out = Vec::from(magic);
    out.extend_from_slice(&(lumps.len() as i32).to_le_bytes());
    out.extend_from_slice(&0_i32.to_le_bytes());
    let mut directory = Vec::new();
    for (name, bytes) in lumps {
        directory.extend_from_slice(&(out.len() as i32).to_le_bytes());
        directory.extend_from_slice(&(bytes.len() as i32).to_le_bytes());
        directory.extend_from_slice(name);
        out.extend_from_slice(bytes);
    }
    let offset = out.len() as i32;
    set_i32(&mut out, 8, offset);
    out.extend_from_slice(&directory);
    out
}

pub fn node(children: [u16; 2]) -> Vec<u8> {
    let mut b = words(&[0, 0, 128, 0, 64, -64, -64, 64, 64, -64, -64, 64]);
    b.extend(children.into_iter().flat_map(u16::to_le_bytes));
    b
}

/// A square, one sector, four walls/segs, one leaf, a player-start definition.
pub fn classic_map(marker: &str) -> Vec<Lump> {
    let mut lines = Vec::new();
    let mut sides = Vec::new();
    let mut segs = Vec::new();
    for i in 0..4_i16 {
        lines.extend(words(&[i, (i + 1) % 4, 1, 0, 0, i, -1]));
        sides.extend(words(&[-8, 16]));
        sides.extend_from_slice(b"-\0\0\0\0\0\0\0-\0\0\0\0\0\0\0WALLTEST");
        sides.extend(words(&[0]));
        segs.extend(words(&[i, (i + 1) % 4, i16::MIN, i, 0, -4]));
    }
    let mut sector = words(&[-16, 128]);
    sector.extend_from_slice(b"FLOOR0\0\0CEILING0");
    sector.extend(words(&[160, 0, 7]));
    vec![
        (name(marker), vec![]),
        (name("THINGS"), words(&[-16, 16, 90, 1, 7])),
        (name("LINEDEFS"), lines),
        (name("SIDEDEFS"), sides),
        (
            name("VERTEXES"),
            words(&[-64, -64, 64, -64, 64, 64, -64, 64]),
        ),
        (name("SEGS"), segs),
        (name("SSECTORS"), words(&[4, 0])),
        (name("NODES"), vec![]),
        (name("SECTORS"), sector),
        (name("REJECT"), vec![0]),
        (
            name("BLOCKMAP"),
            words(&[-128, -128, 2, 2, 8, 8, 8, 8, 0, 0, 1, 2, 3, -1]),
        ),
    ]
}

/// Clockwise room with optional northeast bevel, used only by M3a fixtures.
pub fn collision_arena(angled: bool) -> Vec<Lump> {
    let mut lumps = classic_map("MAP01");
    let vertices: &[[i16; 2]] = if angled {
        &[[-64, -64], [-64, 64], [16, 64], [64, 16], [64, -64]]
    } else {
        &[[-64, -64], [-64, 64], [64, 64], [64, -64]]
    };
    lumps[4].1 = vertices.iter().flat_map(|v| words(v)).collect();
    let side = lumps[3].1[..30].to_vec();
    lumps[2].1.clear();
    lumps[3].1.clear();
    lumps[5].1.clear();
    for i in 0..vertices.len() as i16 {
        let next = (i + 1) % vertices.len() as i16;
        lumps[2].1.extend(words(&[i, next, 1, 0, 0, i, -1]));
        lumps[3].1.extend(&side);
        lumps[5].1.extend(words(&[i, next, 0, i, 0, 0]));
    }
    lumps[6].1 = words(&[vertices.len() as i16, 0]);
    let mut block = vec![-128, -128, 2, 2, 8, 8, 8, 8, 0];
    block.extend(0..vertices.len() as i16);
    block.push(-1);
    lumps[10].1 = words(&block);
    lumps
}
