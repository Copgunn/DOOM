//! Original synthetic C-input fixtures, shared only by M3f2 tests/examples.
#![allow(dead_code)]
#[path = "mod.rs"]
mod base;
#[path = "../reference/m3f2_maps.rs"]
mod maps;
pub fn set_word(bytes: &mut [u8], offset: usize, value: i16) {
    base::set_word(bytes, offset, value);
}
use doom_map::{Map, MapLimits};
use doom_sim::{
    ActorId, ActorSpec, Gravity, Simulation, Solidity, SpawnHeight, TicCommand, VerticalSpec,
};
use doom_types::Fixed;
use doom_wad::{Limits, Wad};

pub fn map(index: usize) -> Map {
    let wad = Wad::parse(maps::MAP_BYTES[index], Limits::default()).unwrap();
    Map::from_wad(&wad, "MAP01", MapLimits::default()).unwrap()
}
pub fn map_count() -> usize {
    maps::MAP_BYTES.len()
}
pub fn raw(value: i32) -> Fixed {
    Fixed::from_bits(value)
}
pub fn integers(s: &str) -> Vec<i32> {
    if s == "-" {
        Vec::new()
    } else {
        s.split(':').map(|n| n.parse().unwrap()).collect()
    }
}
#[derive(Clone, Debug)]
pub struct Case {
    pub index: usize,
    pub label: &'static str,
    pub map: usize,
    pub player: bool,
    pub flags: u32,
    pub position: [Fixed; 2],
    pub z: Fixed,
    pub momentum: [Fixed; 2],
    pub momz: Fixed,
    pub command: TicCommand,
    pub obstacle: bool,
}
pub fn cases() -> Vec<Case> {
    include_str!("../reference/m3f2_cases.csv")
        .lines()
        .skip(1)
        .map(|row| {
            let c: Vec<_> = row.split(',').collect();
            let n = |i: usize| c[i].parse::<i32>().unwrap();
            Case {
                index: n(0) as usize,
                label: c[1],
                map: n(2) as usize,
                player: n(3) != 0,
                flags: n(4) as u32,
                position: [raw(n(5)), raw(n(6))],
                z: raw(n(7)),
                momentum: [raw(n(8)), raw(n(9))],
                momz: raw(n(10)),
                command: TicCommand {
                    forward: n(11) as i8,
                    side: n(12) as i8,
                    angle_turn: n(13) as i16,
                    buttons: 0,
                },
                obstacle: n(14) != 0,
            }
        })
        .collect()
}
impl Case {
    pub fn setup(&self, obstacle: bool) -> (Simulation, ActorId, Option<ActorId>) {
        assert_eq!(self.flags & !(2 | 512 | 1024), 0);
        let mut sim = Simulation::map_extent(map(self.map)).unwrap();
        let other = if self.obstacle && obstacle {
            Some(
                sim.spawn_vertical(
                    ActorSpec {
                        position: [
                            self.position[0] + Fixed::from_map_units(32),
                            self.position[1],
                        ],
                        ..Default::default()
                    },
                    Solidity::Solid,
                    VerticalSpec {
                        height: SpawnHeight::Absolute(Fixed::from_map_units(64)),
                        gravity: Gravity::None,
                        ..Default::default()
                    },
                )
                .unwrap(),
            )
        } else {
            None
        };
        let actor = sim
            .spawn_vertical(
                ActorSpec {
                    position: self.position,
                    momentum: self.momentum,
                    ..Default::default()
                },
                if self.flags & 2 != 0 {
                    Solidity::Solid
                } else {
                    Solidity::NonSolid
                },
                VerticalSpec {
                    height: SpawnHeight::Absolute(self.z),
                    momentum: self.momz,
                    gravity: if self.flags & 512 != 0 {
                        Gravity::None
                    } else {
                        Gravity::Normal
                    },
                    allow_dropoff: self.flags & 1024 != 0,
                },
            )
            .unwrap();
        if self.player {
            sim.bind_player(0, actor).unwrap();
        }
        (sim, actor, other)
    }
    pub fn commands(&self) -> [TicCommand; 4] {
        [
            self.command,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ]
    }
}

pub fn lumps(index: usize) -> Vec<base::Lump> {
    let wad = Wad::parse(maps::MAP_BYTES[index], Limits::default()).unwrap();
    wad.lumps()
        .iter()
        .enumerate()
        .map(|(i, l)| (*l.name().as_bytes(), wad.data(i).unwrap().to_vec()))
        .collect()
}
pub fn decode(lumps: &[base::Lump]) -> Result<Map, doom_map::MapError> {
    let bytes = base::wad(*b"PWAD", lumps);
    let wad = Wad::parse(&bytes, Limits::default()).unwrap();
    Map::from_wad(&wad, "MAP01", MapLimits::default())
}

/// Rust-only map for a defined C intercept whose later source clip is undefined.
pub fn source_clip_boundary_map() -> Map {
    let mut data = lumps(0);
    for (offset, value) in [(0, 0), (2, 0), (4, 90), (6, 234)] {
        set_word(&mut data[4].1, offset, value);
    }
    let mut sector = data[8].1[..26].to_vec();
    set_word(&mut sector, 0, 24);
    data[8].1.extend(sector);
    let side_index = (data[3].1.len() / 30) as i16;
    let mut side = data[3].1[..30].to_vec();
    set_word(&mut side, 28, 1);
    data[3].1.extend(side);
    set_word(&mut data[2].1, 4, 4);
    set_word(&mut data[2].1, 12, side_index);
    set_word(&mut data[6].1, 0, 1);
    decode(&data).unwrap()
}
/// Exact observer/target inputs from the original-C boundary component.
pub fn source_clip_boundary_positions() -> [[Fixed; 2]; 2] {
    let n: Vec<i32> = include_str!("../reference/m3f2_boundary.csv")
        .lines()
        .nth(1)
        .unwrap()
        .split(',')
        .map(|v| v.parse().unwrap())
        .collect();
    [[raw(n[4]), raw(n[5])], [raw(n[4] + n[6]), raw(n[5] + n[7])]]
}

/// Compare independently encoded expected fields, identifying the first divergence.
pub fn assert_canonical(expected: &[u8], actual: &doom_sim::Snapshot) {
    fn take<'a>(bytes: &mut &'a [u8], n: usize) -> &'a [u8] {
        let (a, b) = bytes.split_at(n);
        *bytes = b;
        a
    }
    let mut bytes = expected;
    assert_eq!(take(&mut bytes, 4), b"DSIM");
    let version = u32::from_le_bytes(take(&mut bytes, 4).try_into().unwrap());
    assert_eq!(version, actual.version(), "schema.version");
    let count = u64::from_le_bytes(take(&mut bytes, 8).try_into().unwrap()) as usize;
    let fields: Vec<_> = actual.fields().collect();
    assert_eq!(count, fields.len(), "schema.field_count");
    for field in fields {
        let n = u32::from_le_bytes(take(&mut bytes, 4).try_into().unwrap()) as usize;
        let name = std::str::from_utf8(take(&mut bytes, n)).unwrap();
        let n = u64::from_le_bytes(take(&mut bytes, 8).try_into().unwrap()) as usize;
        let value = take(&mut bytes, n);
        assert_eq!(name, field.path(), "first differing field path");
        assert_eq!(value, field.value(), "first differing field {name}");
    }
    assert!(bytes.is_empty());
    assert_eq!(expected, actual.canonical_bytes());
}
