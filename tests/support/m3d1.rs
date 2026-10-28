//! Synthetic inputs shared by combat tests/example; expected results come from C.
#![allow(dead_code)]
use doom_map::{Map, MapLimits};
use doom_sim::*;
use doom_types::Fixed;
use doom_wad::{Limits, Wad};
#[derive(Clone, Debug)]
pub struct Case {
    pub label: String,
    pub values: Vec<i64>,
}
impl Case {
    pub fn n(&self, index: usize) -> i32 {
        self.values[index] as i32
    }
    pub fn f(&self, index: usize) -> Fixed {
        Fixed::from_bits(self.n(index))
    }
}
pub fn cases() -> Vec<Case> {
    include_str!("../reference/m3d1_cases.csv")
        .lines()
        .skip(1)
        .map(|line| {
            let c = line.split(',').collect::<Vec<_>>();
            Case {
                label: c[1].into(),
                values: c[2..].iter().map(|s| s.parse().unwrap()).collect(),
            }
        })
        .collect()
}
pub fn map(index: usize) -> Map {
    let maps: [&[u8]; 9] = [
        include_bytes!("../reference/m3d1_map_0.bin"),
        include_bytes!("../reference/m3d1_map_1.bin"),
        include_bytes!("../reference/m3d1_map_2.bin"),
        include_bytes!("../reference/m3d1_map_3.bin"),
        include_bytes!("../reference/m3d1_map_4.bin"),
        include_bytes!("../reference/m3d1_map_5.bin"),
        include_bytes!("../reference/m3d1_map_6.bin"),
        include_bytes!("../reference/m3d1_map_7.bin"),
        include_bytes!("../reference/m3d1_map_8.bin"),
    ];
    let wad = Wad::parse(maps[index], Limits::default()).unwrap();
    Map::from_wad(&wad, "MAP01", MapLimits::default()).unwrap()
}
pub fn registry(c: &Case) -> ActorRegistry {
    let r = ActorRegistry::vanilla();
    let mut types = r.types().to_vec();
    let mut info = types[0].clone();
    info.name = "FIXTURE_PAWN".into();
    info.properties[0] = -1;
    info.properties[2] = c.n(13);
    info.properties[8] = c.n(12);
    info.properties[18] = c.n(14);
    info.properties[21] = c.n(15);
    info.properties[5] = 8;
    info.properties[17] = c.n(20);
    types[136] = info;
    ActorRegistry::new(r.states().to_vec(), types).unwrap()
}
pub fn setup(c: &Case) -> Simulation {
    let mut sim = Simulation::with_combat_registry(
        map(c.n(1) as usize),
        1024,
        registry(c),
        ActorOptions::default(),
    )
    .unwrap();
    sim.spawn_type(
        ActorTypeId(0),
        [c.f(2), c.f(3)],
        SpawnHeight::Absolute(c.f(4)),
    )
    .unwrap();
    sim.spawn_type(
        ActorTypeId(136),
        [c.f(5), c.f(6)],
        SpawnHeight::Absolute(c.f(7)),
    )
    .unwrap();
    if c.n(16) != 0 {
        sim.spawn_type(
            ActorTypeId(136),
            [
                if c.n(16) == 1 {
                    Fixed::from_map_units(16)
                } else {
                    c.f(5)
                },
                c.f(6),
            ],
            SpawnHeight::Absolute(c.f(7)),
        )
        .unwrap();
    }
    sim
}
pub fn execute(sim: &mut Simulation, c: &Case) -> Option<AttackResult> {
    let shooter = ActorId::from_parts(0, 1);
    match c.n(0) {
        3 => {
            for source in [shooter, ActorId::from_parts(2, 1)] {
                sim.damage_actor(DamageRequest {
                    target: ActorId::from_parts(1, 1),
                    inflictor: Some(source),
                    source: Some(source),
                    amount: c.n(11),
                })
                .unwrap();
            }
            None
        }
        4 => {
            for _ in 0..100 {
                sim.line_attack(
                    shooter,
                    LineAttack {
                        angle: c.n(8) as u32,
                        range: c.f(9),
                        slope: c.f(10),
                        damage: 0,
                    },
                )
                .unwrap();
            }
            None
        }
        1 => {
            sim.damage_actor(DamageRequest {
                target: ActorId::from_parts(1, 1),
                inflictor: if c.n(17) != 0 { Some(shooter) } else { None },
                source: match c.n(18) {
                    0 => None,
                    2 => Some(ActorId::from_parts(1, 1)),
                    _ => Some(shooter),
                },
                amount: c.n(11),
            })
            .unwrap();
            None
        }
        2 => {
            sim.aim_line_attack(shooter, c.n(8) as u32, c.f(9)).unwrap();
            None
        }
        _ => Some(
            sim.line_attack(
                shooter,
                LineAttack {
                    angle: c.n(8) as u32,
                    range: c.f(9),
                    slope: c.f(10),
                    damage: c.n(11),
                },
            )
            .unwrap(),
        ),
    }
}
/// Compare named fields first so a byte-fixture mismatch identifies its origin.
pub fn assert_golden(sim: &Simulation, which: usize, operation: i32) {
    let index = include_str!("../reference/m3d1_snapshots.csv");
    let row = index
        .lines()
        .skip(1)
        .find(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<i32>().unwrap() == operation
        })
        .unwrap();
    let columns = row.split(',').collect::<Vec<_>>();
    let snapshot = sim.snapshot();
    if columns[4] != "-" {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/reference")
            .join(columns[4]);
        let expected = std::fs::read(path).unwrap();
        let mut offset = 16;
        let count = u64::from_le_bytes(expected[8..16].try_into().unwrap()) as usize;
        assert_eq!(
            snapshot.fields().count(),
            count,
            "field count case {which} op {operation}"
        );
        for field in snapshot.fields() {
            let size =
                u32::from_le_bytes(expected[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            let name = std::str::from_utf8(&expected[offset..offset + size]).unwrap();
            offset += size;
            let size =
                u64::from_le_bytes(expected[offset..offset + 8].try_into().unwrap()) as usize;
            offset += 8;
            assert_eq!(field.path(), name, "case {which} op {operation}");
            assert_eq!(
                field.value(),
                &expected[offset..offset + size],
                "case {which} op {operation}, {name}"
            );
            offset += size;
        }
        assert_eq!(snapshot.canonical_bytes(), expected);
    }
    assert_eq!(
        format!("{:016x}", snapshot.digest()),
        columns[3],
        "case {which} op {operation}"
    );
}
