//! Public source-C perception inputs and ordered evidence.
#![allow(dead_code)]
#[path = "m3d3b.rs"]
mod sight_support;
use doom_sim::*;
use doom_types::Fixed;
#[derive(Clone, Debug)]
pub struct Case {
    pub label: String,
    pub values: Vec<i64>,
}
impl Case {
    pub fn v(&self, name: &str) -> i32 {
        let i = include_str!("../reference/m3d3g_cases.csv")
            .lines()
            .next()
            .unwrap()
            .split(',')
            .position(|k| k == name)
            .unwrap()
            - 2;
        self.n(i)
    }
    pub fn set(&mut self, name: &str, value: i32) {
        let i = include_str!("../reference/m3d3g_cases.csv")
            .lines()
            .next()
            .unwrap()
            .split(',')
            .position(|k| k == name)
            .unwrap()
            - 2;
        self.values[i] = i64::from(value);
    }
    pub fn n(&self, i: usize) -> i32 {
        self.values[i] as i32
    }
    pub fn f(&self, i: usize) -> Fixed {
        Fixed::from_bits(self.n(i))
    }
}
pub fn cases() -> Vec<Case> {
    include_str!("../reference/m3d3g_cases.csv")
        .lines()
        .skip(1)
        .map(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            Case {
                label: c[1].into(),
                values: c[2..].iter().map(|v| v.parse().unwrap()).collect(),
            }
        })
        .collect()
}
pub fn map_bytes(index: usize) -> &'static [u8] {
    match index {
        0 => include_bytes!("../reference/m3d3g_map_0.bin"),
        1 => include_bytes!("../reference/m3d3g_map_1.bin"),
        2 => include_bytes!("../reference/m3d3g_map_2.bin"),
        3 => include_bytes!("../reference/m3d3g_map_3.bin"),
        4 => include_bytes!("../reference/m3d3g_map_4.bin"),
        5 => include_bytes!("../reference/m3d3g_map_5.bin"),
        6 => include_bytes!("../reference/m3d3g_map_6.bin"),
        7 => include_bytes!("../reference/m3d3g_map_7.bin"),
        8 => include_bytes!("../reference/m3d3g_map_8.bin"),
        9 => include_bytes!("../reference/m3d3g_map_9.bin"),
        10 => include_bytes!("../reference/m3d3g_map_10.bin"),
        11 => include_bytes!("../reference/m3d3g_map_11.bin"),
        12 => include_bytes!("../reference/m3d3g_map_12.bin"),
        13 => include_bytes!("../reference/m3d3g_map_13.bin"),
        14 => include_bytes!("../reference/m3d3g_map_14.bin"),
        15 => include_bytes!("../reference/m3d3g_map_15.bin"),
        16 => include_bytes!("../reference/m3d3g_map_16.bin"),
        17 => include_bytes!("../reference/m3d3g_map_17.bin"),
        18 => include_bytes!("../reference/m3d3g_map_18.bin"),
        19 => include_bytes!("../reference/m3d3g_map_19.bin"),
        20 => include_bytes!("../reference/m3d3g_map_20.bin"),
        _ => panic!("fixture map"),
    }
}
pub fn map(index: usize) -> doom_map::Map {
    let bytes = map_bytes(index);
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn registry(_c: &Case) -> ActorRegistry {
    ActorRegistry::demon_lifecycle_fixture()
}
pub fn setup(c: &Case) -> Simulation {
    let mut sim = Simulation::with_demon_lifecycle_registry(
        map(c.v("map") as usize),
        64,
        registry(c),
        ActorOptions::default(),
    )
    .unwrap();
    for _ in 0..c.v("skip") {
        sim.gameplay_random();
    }
    sim.spawn_type(
        ActorTypeId(136),
        [Fixed::from_bits(c.v("ox")), Fixed::from_bits(c.v("oy"))],
        SpawnHeight::Floor,
    )
    .unwrap();
    for i in 0..4 {
        sim.spawn_type(
            ActorTypeId(135),
            [c.f(19 + i * 5), c.f(20 + i * 5)],
            SpawnHeight::Absolute(c.f(21 + i * 5)),
        )
        .unwrap();
    }
    let id = ActorId::from_parts(0, 1);
    let mut p = PursuitSetup::from_actor(sim.actor(id).unwrap()).unwrap();
    p.health = c.v("owner_health");
    p.movedir = c.v("direction") as u8;
    p.movecount = c.v("count");
    p.reaction_time = c.v("reaction");
    p.threshold = c.v("threshold");
    p.angle = c.v("angle") as u32;
    sim.configure_pursuit_actor(id, p).unwrap();
    if c.v("target") >= 0 {
        sim.set_actor_target(id, Some(ActorId::from_parts(c.v("target") as u32, 1)))
            .unwrap();
    }
    for i in 0..4 {
        sim.set_perception_player(
            i,
            PerceptionPlayer {
                active: c.n(22 + i * 5) != 0,
                health: c.n(23 + i * 5),
                actor: Some(ActorId::from_parts(i as u32 + 1, 1)),
            },
        )
        .unwrap();
    }
    if c.v("enter") != 0 {
        sim.enter_actor_state(id, StateId(c.v("enter") as u16))
            .unwrap();
    }
    if c.v("initial_damage") >= 0 {
        hit(&mut sim, c, c.v("initial_damage"));
    }
    sim
}
pub fn hit(sim: &mut Simulation, c: &Case, amount: i32) {
    if c.v("kind") != 0 {
        sim.line_attack(
            ActorId::from_parts(1, 1),
            LineAttack {
                angle: 0x80000000,
                range: Fixed::from_map_units(512),
                slope: Fixed::ZERO,
                damage: amount,
            },
        )
        .unwrap();
    } else {
        sim.damage_actor(DamageRequest {
            target: ActorId::from_parts(0, 1),
            source: (c.v("source") >= 0).then(|| ActorId::from_parts(c.v("source") as u32, 1)),
            inflictor: (c.v("inflictor") >= 0)
                .then(|| ActorId::from_parts(c.v("inflictor") as u32, 1)),
            amount,
        })
        .unwrap();
    }
}
pub fn advance(sim: &mut Simulation, c: &Case, op: usize) {
    if op == c.v("repeat_tick") as usize {
        hit(sim, c, c.v("repeat_damage"));
    }
    if c.v("body_push") != 0 {
        sim.set_momentum(
            ActorId::from_parts(1, 1),
            [Fixed::from_map_units(c.v("body_push") as i16), Fixed::ZERO],
        )
        .unwrap();
    }
    if op == c.v("retire_tick") as usize {
        sim.set_perception_player(
            0,
            PerceptionPlayer {
                active: false,
                health: 100,
                actor: None,
            },
        )
        .unwrap();
        sim.mark_remove(ActorId::from_parts(1, 1)).unwrap();
    }
    if c.v("retire_tick") >= 0 && op == c.v("retire_tick") as usize + 2 {
        let actor = sim
            .spawn_type(
                ActorTypeId(135),
                [c.f(19), c.f(20)],
                SpawnHeight::Absolute(c.f(21)),
            )
            .unwrap();
        assert_eq!(actor, ActorId::from_parts(1, 2));
        sim.set_perception_player(
            0,
            PerceptionPlayer {
                active: true,
                health: 100,
                actor: Some(actor),
            },
        )
        .unwrap();
    }
    sim.step([TicCommand::default(); 4])
        .unwrap_or_else(|e| panic!("{} op{op}: {e}", c.label));
}
pub fn assert_golden(sim: &Simulation, which: usize, op: usize) {
    let row = include_str!("../reference/m3d3g_snapshots.csv")
        .lines()
        .skip(1)
        .find(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<usize>().unwrap() == op
        })
        .unwrap();
    let c = row.split(',').collect::<Vec<_>>();
    let snapshot = sim.snapshot();
    if c[4] != "-" {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/reference")
            .join(c[4]);
        let bytes = std::fs::read(path).unwrap();
        let count = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
        assert_eq!(
            snapshot.fields().count(),
            count,
            "case {which} op {op} field count"
        );
        let mut offset = 16;
        for f in snapshot.fields() {
            let size = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            let name = std::str::from_utf8(&bytes[offset..offset + size]).unwrap();
            offset += size;
            let size = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
            offset += 8;
            assert_eq!(f.path(), name, "case {which} op {op}");
            assert_eq!(
                f.value(),
                &bytes[offset..offset + size],
                "case {which} op {op} {name}"
            );
            offset += size;
        }
        assert_eq!(snapshot.canonical_bytes(), bytes);
    }
    assert_eq!(
        format!("{:016x}", snapshot.digest()),
        c[3],
        "case {which} op {op}"
    );
}
pub fn has_golden(which: usize, op: usize) -> bool {
    include_str!("../reference/m3d3g_snapshots.csv")
        .lines()
        .skip(1)
        .any(|r| {
            let c = r.split(',').collect::<Vec<_>>();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<usize>().unwrap() == op
        })
}

pub fn source_rows(which: usize, op: usize) -> Vec<(String, Vec<i64>)> {
    type Rows = std::collections::BTreeMap<(usize, usize), Vec<(String, Vec<i64>)>>;
    static ROWS: std::sync::OnceLock<Rows> = std::sync::OnceLock::new();
    ROWS.get_or_init(|| {
        let mut rows = Rows::new();
        for line in include_str!("../reference/m3d3g_trace.csv").lines() {
            let mut values = line.split(',');
            let tag = values.next().unwrap().to_string();
            let which = values.next().unwrap().parse().unwrap();
            let op = values.next().unwrap().parse().unwrap();
            rows.entry((which, op))
                .or_default()
                .push((tag, values.map(|s| s.parse().unwrap()).collect()));
        }
        rows
    })
    .get(&(which, op))
    .unwrap()
    .clone()
}
#[derive(Default)]
pub struct SourceTrace {
    births: Vec<ActorId>,
    rng: doom_types::DoomRng,
    pub actor_fields: usize,
    pub draws: usize,
    pub observations: usize,
}
impl SourceTrace {
    pub fn check(&mut self, sim: &Simulation, which: usize, op: usize, events: &[Event]) {
        for event in events {
            if let Event::Spawned(id) = event {
                self.births.push(*id);
            }
        }
        let rows = source_rows(which, op);
        for (_, c) in rows.iter().filter(|(tag, c)| tag == "E" && c[0] == 8) {
            assert_eq!(
                i64::from(self.rng.p_random()),
                c[2],
                "case {which} op {op} source RNG"
            );
            self.draws += 1;
        }
        assert_eq!(
            sim.rng().indices(),
            self.rng.indices(),
            "case {which} op {op} RNG index"
        );
        self.observations += rows
            .iter()
            .filter(|(tag, _)| {
                [
                    "E", "J", "G", "X", "H", "V", "K", "D", "B", "U", "I", "T", "M", "C", "O", "Z",
                    "Y",
                ]
                .contains(&tag.as_str())
            })
            .count();
        for (_, c) in rows.iter().filter(|(tag, _)| tag == "A") {
            let id = self.births[c[0] as usize];
            if c[1] == 0 {
                assert!(sim.actor(id).is_none());
                continue;
            }
            let a = sim.actor(id).unwrap();
            let l = a.life().unwrap();
            let actual = [
                i64::from(l.actor_type.0),
                i64::from(a.position()[0].bits()),
                i64::from(a.position()[1].bits()),
                i64::from(a.z().bits()),
                i64::from(a.momentum()[0].bits()),
                i64::from(a.momentum()[1].bits()),
                i64::from(a.vertical_momentum().bits()),
                i64::from(l.health),
                l.state.map_or(-1, |s| i64::from(s.0)),
                i64::from(l.tics),
                i64::from(l.flags),
                i64::from(a.height().bits()),
                i64::from(a.radius().bits()),
                i64::from(l.reaction_time),
                i64::from(l.threshold),
                l.target.map_or(-1, |id| {
                    self.births.iter().position(|b| *b == id).unwrap() as i64
                }),
                i64::from(a.lastlook()),
                i64::from(a.constraints()[0].bits()),
                i64::from(a.constraints()[1].bits()),
                i64::from(a.location().unwrap().subsector),
                i64::from(sim.rng().indices()[0]),
                i64::from(a.angle()),
                i64::from(l.sprite),
                i64::from(l.frame),
                i64::from(l.movedir),
                i64::from(l.movecount),
            ];
            for (field, (got, want)) in actual.iter().zip(&c[2..]).enumerate() {
                assert_eq!(
                    got, want,
                    "case {which} op {op} birth {} source actor field {field}",
                    c[0]
                );
                self.actor_fields += 1;
            }
        }
    }
}
