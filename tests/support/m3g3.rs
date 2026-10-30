//! Explicit grounded-player inputs, frozen C rows and independent DSIM22 evidence.
#![allow(dead_code)]
#[path = "player_observations.rs"]
mod observations;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
#[allow(unused_imports)]
pub use observations::SourceTrace;
pub const COMMANDS: [TicCommand; 4] = [TicCommand {
    forward: 0,
    side: 0,
    angle_turn: 0,
    buttons: 0,
}; 4];
pub fn map() -> doom_map::Map {
    case_map(0)
}
include!("../reference/m3g3_maps.rs");
use std::{collections::BTreeMap, sync::OnceLock};
#[derive(Clone, Debug)]
pub struct Case {
    pub label: String,
    pub map: usize,
    pub tics: usize,
    pub skip: usize,
    pub origin: [i16; 2],
    pub position: [i16; 2],
    pub extra: u8,
    pub source: i8,
    pub physical: u8,
    pub amount: i32,
    pub repeat: u8,
    pub pre: i32,
    pub retire: bool,
}
pub fn cases() -> Vec<Case> {
    include_str!("../reference/m3g3_cases.csv")
        .lines()
        .skip(1)
        .map(|line| {
            let v: Vec<_> = line.split(',').collect();
            Case {
                label: v[1].into(),
                map: v[15].parse().unwrap(),
                tics: v[2].parse().unwrap(),
                skip: v[3].parse().unwrap(),
                origin: [v[4].parse().unwrap(), v[5].parse().unwrap()],
                position: [v[6].parse().unwrap(), v[7].parse().unwrap()],
                extra: v[8].parse().unwrap(),
                source: v[9].parse().unwrap(),
                physical: v[10].parse().unwrap(),
                amount: v[11].parse().unwrap(),
                repeat: v[12].parse().unwrap(),
                pre: v[13].parse().unwrap(),
                retire: v[14] == "1",
            }
        })
        .collect()
}
#[derive(Clone, Copy)]
pub struct Actors {
    pub origin: ActorId,
    pub other: Option<ActorId>,
    pub body: ActorId,
}
pub fn hit(
    sim: &mut Simulation,
    a: Actors,
    c: &Case,
    amount: i32,
) -> Result<DamageResult, SimError> {
    sim.damage_actor(DamageRequest {
        target: a.body,
        inflictor: match c.physical {
            0 => None,
            2 => Some(a.body),
            _ => Some(a.origin),
        },
        source: match c.source {
            -1 => None,
            0 => Some(a.origin),
            1 => a.other,
            _ => Some(a.body),
        },
        amount,
    })
}
pub fn case_map(which: usize) -> doom_map::Map {
    let wad = doom_wad::Wad::parse(MAP_BYTES[which], doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn setup(c: &Case) -> (Simulation, Actors) {
    let mut sim = Simulation::original_player_knockback_map(
        case_map(c.map),
        64,
        MapContextOptions::default(),
    )
    .unwrap();
    for _ in 0..c.skip {
        sim.gameplay_random();
    }
    let spawned = sim
        .spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap()
        .spawned;
    let origin = spawned[0];
    let other = spawned.get(1).copied();
    let body = sim
        .spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
        .unwrap();
    for target in [Some(origin), other].into_iter().flatten() {
        sim.damage_actor(DamageRequest {
            target,
            source: None,
            inflictor: None,
            amount: 150,
        })
        .unwrap();
    }
    if c.pre != 0 {
        sim.damage_actor(DamageRequest {
            target: body,
            source: None,
            inflictor: None,
            amount: c.pre,
        })
        .unwrap();
    }
    (
        sim,
        Actors {
            origin,
            other,
            body,
        },
    )
}
pub fn operations(sim: &mut Simulation, a: Actors, c: &Case, op: usize) {
    if op == 1 {
        hit(sim, a, c, c.amount).unwrap();
    }
    if c.repeat == 1 && op == 2 {
        hit(sim, a, c, 20).unwrap();
    }
    if c.repeat == 1 && op == 3 {
        hit(sim, a, c, 10).unwrap();
    }
    if c.repeat == 2 && op == 2 {
        hit(sim, a, c, 1).unwrap();
    }
    if c.retire && op == 3 {
        sim.mark_remove(a.body).unwrap();
    }
}
type Rows = BTreeMap<(usize, usize), Vec<(String, Vec<i64>)>>;
pub fn rows(which: usize, op: usize) -> &'static [(String, Vec<i64>)] {
    static ROWS: OnceLock<Rows> = OnceLock::new();
    ROWS.get_or_init(|| {
        let mut all = Rows::new();
        let mut lines = include_str!("../reference/m3g3_trace.csv").lines();
        assert_eq!(lines.next(), Some("DOOM-PLAYER-KNOCKBACK-v1"));
        for line in lines {
            let v: Vec<_> = line.split(',').collect();
            all.entry((v[1].parse().unwrap(), v[2].parse().unwrap()))
                .or_default()
                .push((
                    v[0].into(),
                    v[3..].iter().map(|v| v.parse().unwrap()).collect(),
                ));
        }
        all
    })
    .get(&(which, op))
    .unwrap()
}
type Frames = BTreeMap<(usize, usize), [Vec<Vec<u8>>; 2]>;
fn word(bytes: &[u8], at: &mut usize) -> usize {
    let n = u32::from_le_bytes(bytes[*at..*at + 4].try_into().unwrap()) as usize;
    *at += 4;
    n
}
pub fn assert_events(sim: &Simulation, which: usize, op: usize) -> usize {
    static FRAMES: OnceLock<Frames> = OnceLock::new();
    let frames = FRAMES.get_or_init(|| {
        let b = include_bytes!("../reference/m3g3_events.bin");
        assert_eq!(&b[..4], b"PBE1");
        let mut at = 4;
        let mut all = Frames::new();
        while at < b.len() {
            let which = word(b, &mut at);
            let op = word(b, &mut at);
            let counts = [word(b, &mut at), word(b, &mut at)];
            let mut frame = [Vec::new(), Vec::new()];
            for (family, count) in frame.iter_mut().zip(counts) {
                for _ in 0..count {
                    let n = word(b, &mut at);
                    family.push(b[at..at + n].to_vec());
                    at += n;
                }
            }
            assert!(all.insert((which, op), frame).is_none());
        }
        assert_eq!(at, b.len());
        all
    });
    let snapshot = sim.snapshot();
    let expected = &frames[&(which, op)];
    let mut actual = Vec::new();
    let mut fields = Vec::new();
    for (prefix, events) in ["events.last", "events.pending"].into_iter().zip(expected) {
        fields.push((
            format!("{prefix}.count"),
            (events.len() as u64).to_le_bytes().to_vec(),
        ));
        fields.extend(
            events
                .iter()
                .enumerate()
                .map(|(i, e)| (format!("{prefix}.{i}"), e.clone())),
        );
    }
    actual.extend(
        snapshot
            .fields()
            .filter(|f| f.path().starts_with("events."))
            .map(|f| (f.path().to_owned(), f.value().to_vec())),
    );
    assert_eq!(actual, fields, "complete event fields case{which} op{op}");
    actual.len() - 2
}
pub fn assert_snapshot(sim: &Simulation, which: usize, op: usize) -> bool {
    let line = include_str!("../reference/m3g3_snapshots.csv")
        .lines()
        .skip(1)
        .find(|line| {
            let v: Vec<_> = line.split(',').collect();
            v[0].parse::<usize>().unwrap() == which && v[1].parse::<usize>().unwrap() == op
        });
    let Some(line) = line else { return false };
    let v: Vec<_> = line.split(',').collect();
    let snapshot = sim.snapshot();
    let bytes = snapshot
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    assert_eq!(
        snapshot.digest(),
        u64::from_str_radix(v[3], 16).unwrap(),
        "snapshot case{which} op{op}"
    );
    if v[4] != "-" {
        let expected: &[u8] = match (which, op) {
            (8, 8) => include_bytes!("../reference/m3g3_state_8_8.bin"),
            (10, 1) => include_bytes!("../reference/m3g3_state_10_1.bin"),
            (11, 72) => include_bytes!("../reference/m3g3_state_11_72.bin"),
            (12, 72) => include_bytes!("../reference/m3g3_state_12_72.bin"),
            _ => panic!("undeclared independent full fixture"),
        };
        assert_eq!(bytes, expected, "full bytes case{which} op{op}");
    }
    let view = Diagnostic::decode_player(&bytes, DiagnosticLimits::default()).unwrap();
    assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
    true
}
