//! Canonical C observations and separately declared frozen runtime projection.
#![allow(dead_code)]
#[path = "m3f3.rs"]
mod legacy;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
pub use legacy::Case;
#[allow(unused_imports)] // Some consumers use setup only; others drive the script.
pub use legacy::{advance, options};
use std::{collections::BTreeMap, sync::OnceLock};
mod maps {
    include!("../reference/m3f5_maps.rs");
}
pub fn cases() -> Vec<Case> {
    include_str!("../reference/m3f5_cases.csv")
        .lines()
        .skip(1)
        .map(|r| {
            let c: Vec<_> = r.split(',').collect();
            Case {
                label: c[1].into(),
                values: c[2..].iter().map(|s| s.parse().unwrap()).collect(),
            }
        })
        .collect()
}
pub fn map_bytes(which: usize) -> &'static [u8] {
    maps::MAP_BYTES[which]
}
pub fn map(which: usize) -> doom_map::Map {
    let wad = doom_wad::Wad::parse(maps::MAP_BYTES[which], doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn map_only(c: &Case) -> Simulation {
    let mut sim = Simulation::canonical_demon_map(
        map(c.v("map") as usize),
        64,
        options(c),
        MapContextOptions {
            initial_rng: doom_types::DoomRng::from_indices(c.v("seed") as u8, 0),
            respawn_monsters: false,
        },
    )
    .unwrap();
    for _ in 0..c.v("skip") {
        sim.gameplay_random();
    }
    sim.spawn_original_demons(OriginalActorType::MT_SERGEANT)
        .unwrap();
    sim
}
pub fn setup(c: &Case) -> Simulation {
    install_targets(map_only(c), c)
}
pub fn install_targets(mut sim: Simulation, c: &Case) -> Simulation {
    let demons = sim.level_statistics().unwrap().total_kills as usize;
    for i in 0..4 {
        let actor = sim
            .spawn_type(
                ActorTypeId(135),
                [
                    Fixed::from_bits(c.v(&format!("p{i}x"))),
                    Fixed::from_bits(c.v(&format!("p{i}y"))),
                ],
                SpawnHeight::Absolute(Fixed::from_bits(c.v(&format!("p{i}z")))),
            )
            .unwrap();
        let mut p = PursuitSetup::from_actor(sim.actor(actor).unwrap()).unwrap();
        p.health = c.v("body_health");
        sim.configure_pursuit_actor(actor, p).unwrap();
    }
    for i in 0..4 {
        sim.set_perception_player(
            i,
            PerceptionPlayer {
                active: c.v(&format!("p{i}active")) != 0,
                health: c.v(&format!("p{i}health")),
                actor: Some(ActorId::from_parts((demons + i) as u32, 1)),
            },
        )
        .unwrap();
    }
    if c.v("enter_all") != 0 {
        for i in 0..demons {
            let actor = ActorId::from_parts(i as u32, 1);
            sim.set_actor_target(actor, Some(ActorId::from_parts(demons as u32, 1)))
                .unwrap();
            sim.enter_original_demon_state(
                actor,
                OriginalState::try_from(c.v("enter_all")).unwrap(),
            )
            .unwrap();
        }
    }
    sim
}
pub type Row = (String, Vec<i64>);
type Rows = BTreeMap<(usize, usize), Vec<Row>>;
fn parse(text: &str) -> Rows {
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("DOOM-DEMON-EXEC-v1"));
    let mut rows = Rows::new();
    for line in lines {
        let c: Vec<_> = line.split(',').collect();
        rows.entry((c[1].parse().unwrap(), c[2].parse().unwrap()))
            .or_default()
            .push((
                c[0].into(),
                c[3..].iter().map(|s| s.parse().unwrap()).collect(),
            ));
    }
    rows
}
pub fn rows(which: usize, op: usize, spawn: bool) -> &'static [Row] {
    static TRACE: OnceLock<Rows> = OnceLock::new();
    static SPAWN: OnceLock<Rows> = OnceLock::new();
    let all = if spawn {
        SPAWN.get_or_init(|| parse(include_str!("../reference/m3f5_spawn.csv")))
    } else {
        TRACE.get_or_init(|| {
            parse(
                doom_test_evidence::text(
                    "tests/reference/m3f5_trace.csv",
                    include_bytes!("../reference/m3f5_trace.csv.zst"),
                    include_str!("../reference/zstd_manifest.json"),
                )
                .expect("verified historical zstd evidence"),
            )
        })
    };
    all.get(&(which, op)).unwrap()
}
#[derive(Default)]
pub struct SourceTrace {
    births: Vec<ActorId>,
    rng: doom_types::DoomRng,
    pub actor_fields: usize,
    pub canonical_fields: usize,
    pub draws: usize,
    pub statistics_fields: usize,
    pub spawnpoint_fields: usize,
}
impl SourceTrace {
    pub fn for_case(c: &Case) -> Self {
        Self {
            rng: doom_types::DoomRng::from_indices(c.v("seed") as u8, 0),
            ..Self::default()
        }
    }
    pub fn check(
        &mut self,
        sim: &Simulation,
        which: usize,
        op: usize,
        events: &[Event],
        spawn: bool,
    ) {
        for event in events {
            if let Event::Spawned(id) = event {
                self.births.push(*id);
            }
        }
        let rows = rows(which, op, spawn);
        for (_, c) in rows.iter().filter(|(tag, c)| tag == "E" && c[0] == 8) {
            assert_eq!(
                i64::from(self.rng.p_random()),
                c[2],
                "case{which} op{op} C RNG"
            );
            self.draws += 1;
        }
        assert_eq!(
            sim.rng().indices(),
            self.rng.indices(),
            "case{which} op{op} RNG consumption"
        );
        for (_, c) in rows.iter().filter(|(tag, _)| tag == "LS") {
            let s = sim.level_statistics().unwrap();
            assert_eq!(
                [
                    s.total_kills,
                    s.total_items,
                    s.total_secrets,
                    s.single_player_kill_count
                ]
                .map(i64::from)
                .as_slice(),
                c
            );
            self.statistics_fields += 4;
        }
        if spawn {
            for (_, c) in rows
                .iter()
                .filter(|(tag, c)| tag == "F" && c[0] >= 0 && c[7] > c[6])
            {
                let point = sim
                    .actor(self.births[c[6] as usize])
                    .unwrap()
                    .life()
                    .unwrap()
                    .spawnpoint
                    .unwrap();
                assert_eq!(
                    [
                        i64::from(point.position.x),
                        i64::from(point.position.y),
                        i64::from(point.angle),
                        i64::from(point.kind),
                        i64::from(point.options)
                    ]
                    .as_slice(),
                    &c[1..6]
                );
                self.spawnpoint_fields += 5;
            }
        }
        for (_, c) in rows.iter().filter(|(tag, _)| tag == "A") {
            let id = self.births[c[0] as usize];
            if c[1] == 0 {
                assert!(sim.actor(id).is_none());
                continue;
            }
            let a = sim.actor(id).unwrap();
            let l = a.life().unwrap();
            // Explicit canonical definition observation, not a claim about raw storage.
            let (ty, state) = if c[2] == i64::from(OriginalActorType::MT_SERGEANT.ordinal()) {
                let (ty, state) = sim.original_demon_identity(id).unwrap();
                self.canonical_fields += 2;
                (
                    i64::from(ty.ordinal()),
                    state.map_or(-1, |s| i64::from(s.ordinal())),
                )
            } else {
                (
                    i64::from(l.actor_type.0),
                    l.state.map_or(-1, |s| i64::from(s.0)),
                )
            };
            let actual = [
                ty,
                i64::from(a.position()[0].bits()),
                i64::from(a.position()[1].bits()),
                i64::from(a.z().bits()),
                i64::from(a.momentum()[0].bits()),
                i64::from(a.momentum()[1].bits()),
                i64::from(a.vertical_momentum().bits()),
                i64::from(l.health),
                state,
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
                assert_eq!(got, want, "case{which} op{op} actor{} C field{field}", c[0]);
                self.actor_fields += 1;
            }
        }
    }
}

pub fn assert_golden(sim: &Simulation, which: usize, op: usize) -> usize {
    let row = include_str!("../reference/m3f5_snapshots.csv")
        .lines()
        .skip(1)
        .find(|r| {
            let c: Vec<_> = r.split(',').collect();
            c[0].parse::<usize>().unwrap() == which && c[1].parse::<usize>().unwrap() == op
        })
        .unwrap();
    let c: Vec<_> = row.split(',').collect();
    let snapshot = sim.snapshot();
    if c[4] != "-" {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/reference")
            .join(c[4]);
        let bytes = std::fs::read(file).unwrap();
        assert_eq!(&bytes[..4], b"DSIM");
        let mut at = 8;
        assert_eq!(snapshot.fields().count(), u64_at(&bytes, &mut at));
        for field in snapshot.fields() {
            let n = u32_at(&bytes, &mut at);
            let path = std::str::from_utf8(&bytes[at..at + n]).unwrap();
            at += n;
            let n = u64_at(&bytes, &mut at);
            assert_eq!(field.path(), path, "case{which} op{op} field order");
            let expected = &bytes[at..at + n];
            if field.value() != expected {
                let first = field.value().iter().zip(expected).position(|(a, b)| a != b);
                panic!(
                    "case{which} op{op} {path} first differing byte{first:?}; lengths {}/{}",
                    field.value().len(),
                    expected.len()
                );
            }
            at += n;
        }
        assert_eq!(at, bytes.len());
        assert_eq!(snapshot.canonical_bytes(), bytes);
    }
    assert_eq!(
        format!("{:016x}", snapshot.digest()),
        c[3],
        "case{which} op{op} C-derived DSIM digest"
    );
    ordered_events(&snapshot, which, op)
}
fn u32_at(bytes: &[u8], at: &mut usize) -> usize {
    let v = u32::from_le_bytes(bytes[*at..*at + 4].try_into().unwrap());
    *at += 4;
    v as usize
}
fn u64_at(bytes: &[u8], at: &mut usize) -> usize {
    let v = u64::from_le_bytes(bytes[*at..*at + 8].try_into().unwrap());
    *at += 8;
    usize::try_from(v).unwrap()
}
fn ordered_events(snapshot: &Snapshot, which: usize, op: usize) -> usize {
    type Index = BTreeMap<(usize, usize), (usize, usize, usize)>;
    static INDEX: OnceLock<Index> = OnceLock::new();
    let bytes = include_bytes!("../reference/m3f5_events.bin");
    let index = INDEX.get_or_init(|| {
        assert_eq!(&bytes[..4], b"EV20");
        let mut index = Index::new();
        let mut at = 4;
        while at < bytes.len() {
            let which = u32_at(bytes, &mut at);
            let op = u32_at(bytes, &mut at);
            let count = u32_at(bytes, &mut at);
            let start = at;
            for _ in 0..count {
                let n = u32_at(bytes, &mut at);
                at += n;
                let n = u64_at(bytes, &mut at);
                at += n;
            }
            assert!(index.insert((which, op), (count, start, at)).is_none());
        }
        assert_eq!(at, bytes.len());
        index
    });
    let &(count, mut at, end) = index.get(&(which, op)).unwrap();
    let mut actual = 0;
    for f in snapshot
        .fields()
        .filter(|f| f.path().starts_with("events."))
    {
        let n = u32_at(bytes, &mut at);
        let path = std::str::from_utf8(&bytes[at..at + n]).unwrap();
        at += n;
        let n = u64_at(bytes, &mut at);
        let expected = &bytes[at..at + n];
        assert_eq!(f.path(), path, "case{which} op{op} event order");
        if f.value() != expected {
            let first = f.value().iter().zip(expected).position(|(a, b)| a != b);
            panic!("case{which} op{op} {path} first differing byte{first:?}");
        }
        at += n;
        actual += 1;
    }
    assert_eq!(actual, count);
    assert_eq!(at, end);
    count - 2
}
