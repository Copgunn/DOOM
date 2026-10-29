//! Reused original-C observations checked against actual original-ID storage.
//! Only the explicitly synthetic observer body135 is bound to context137.
#![allow(dead_code)]
#[path = "m3f5.rs"]
pub mod historical;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
#[allow(unused_imports)] // Reusable harness consumers select different operations.
pub use historical::{Case, advance, cases, map, options, rows};
use std::{collections::BTreeMap, sync::OnceLock};
pub fn map_only(c: &Case) -> Simulation {
    let mut sim = Simulation::original_id_map(
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
    sim.spawn_original_actor(OriginalActorType::MT_SERGEANT)
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
            .spawn_context_body(
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
            sim.enter_original_state(actor, OriginalState::try_from(c.v("enter_all")).unwrap())
                .unwrap();
        }
    }
    sim
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
            // Actual storage, never the legacy canonical projection API.
            let ty = i64::from(l.actor_type.0);
            let state = l.state.map_or(-1, |s| i64::from(s.0));
            if c[2] == i64::from(OriginalActorType::MT_SERGEANT.ordinal()) {
                self.canonical_fields += 2;
                assert_eq!(
                    sim.original_actor_identity(id).unwrap().0,
                    DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT)
                );
            }
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
                let expected = if field == 0 && *want == 135 {
                    i64::from(OriginalActorType::COUNT) // explicit observer context binding
                } else {
                    *want
                };
                assert_eq!(
                    *got, expected,
                    "case{which} op{op} actor{} C field{field}",
                    c[0]
                );
                self.actor_fields += 1;
            }
        }
    }
}

pub fn assert_golden(sim: &Simulation, which: usize, op: usize) -> usize {
    let row = include_str!("../reference/m3f7_snapshots.csv")
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
        let legacy = &bytes[at..at + n];
        let expected = original_event(legacy, which, op, path);
        assert_eq!(f.path(), path, "case{which} op{op} event order");
        if f.value() != expected.as_slice() {
            let first = f.value().iter().zip(&expected).position(|(a, b)| a != b);
            panic!("case{which} op{op} {path} first differing byte{first:?}");
        }
        at += n;
        actual += 1;
    }
    assert_eq!(actual, count);
    assert_eq!(at, end);
    count - 2
}

// The old EV20 stream supplies all unchanged payload bytes. Identity payloads are
// read from raw C observations in chronology order, not native state projections.
fn original_event(legacy: &[u8], which: usize, op: usize, path: &str) -> Vec<u8> {
    if path.ends_with(".count") {
        return legacy.to_vec();
    }
    let key = match legacy.get(..2) {
        Some([12, 0]) => Some(("E", 1, 2, 14)),
        Some([12, 1]) => Some(("E", 4, 2, 14)),
        Some([16, 8]) => Some(("K", 8, 2, 14)),
        Some([18, 3]) => Some(("J", 3, 3, 15)),
        Some([13, 6]) => Some(("G", 6, 2, 14)),
        Some([13, 7]) => Some(("G", 7, 2, 14)),
        _ => None,
    };
    let mut result = legacy.to_vec();
    if let Some((tag, code, field, offset)) = key {
        // Locate the occurrence among preceding fields of this same event kind.
        let event_index: usize = path.rsplit('.').next().unwrap().parse().unwrap();
        let prefix = path.rsplit_once('.').unwrap().0;
        let before = legacy_event_values(which, op, prefix);
        let occurrence = before[..event_index]
            .iter()
            .filter(|b| b.get(..2) == legacy.get(..2))
            .count();
        let mut source = Vec::new();
        if op <= 1 {
            source.extend_from_slice(rows(which, 0, false));
        }
        if op > 0 {
            source.extend_from_slice(rows(which, op, false));
        }
        let value = source
            .iter()
            .filter(|(t, c)| t == tag && c[0] == code)
            .nth(occurrence)
            .unwrap()
            .1[field];
        let value = if tag == "E" && code == 1 && value == 135 {
            i64::from(OriginalActorType::COUNT)
        } else {
            value
        };
        result[offset..offset + 2].copy_from_slice(&u16::try_from(value).unwrap().to_le_bytes());
    }
    result
}
fn legacy_event_values(which: usize, op: usize, prefix: &str) -> Vec<&'static [u8]> {
    type Index = BTreeMap<(usize, usize), Vec<(&'static str, &'static [u8])>>;
    static INDEX: OnceLock<Index> = OnceLock::new();
    let index = INDEX.get_or_init(|| {
        let bytes: &'static [u8] = include_bytes!("../reference/m3f5_events.bin");
        let mut index = Index::new();
        let mut at = 4;
        while at < bytes.len() {
            let w = u32_at(bytes, &mut at);
            let o = u32_at(bytes, &mut at);
            let count = u32_at(bytes, &mut at);
            let mut values = Vec::new();
            for _ in 0..count {
                let n = u32_at(bytes, &mut at);
                let path = std::str::from_utf8(&bytes[at..at + n]).unwrap();
                at += n;
                let n = u64_at(bytes, &mut at);
                let v = &bytes[at..at + n];
                at += n;
                values.push((path, v));
            }
            assert!(index.insert((w, o), values).is_none());
        }
        index
    });
    index[&(which, op)]
        .iter()
        .filter(|(path, _)| path.starts_with(prefix) && !path.ends_with(".count"))
        .map(|(_, v)| *v)
        .collect()
}
