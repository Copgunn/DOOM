//! Genuine pistol inputs and independently committed source/schema evidence.
#![allow(dead_code)]
// Frozen evidence readers are intentionally shared by independent test binaries.
#[allow(clippy::duplicate_mod)]
#[path = "player_execution_evidence.rs"]
mod evidence_reader;
#[allow(clippy::duplicate_mod)]
#[path = "m3g6.rs"]
mod shared;
use doom_sim::{identity::*, *};
use doom_types::{DoomRng, Fixed};
pub use shared::{Actors, SourceTrace};
use std::{collections::BTreeMap, sync::OnceLock};
include!("../reference/m3g7_maps.rs");
#[derive(Clone, Debug)]
pub struct Case {
    pub which: usize,
    pub label: String,
    pub tics: usize,
    pub position: [i16; 2],
    pub seed: u8,
    pub pre_damage: i32,
    pub reaction: u8,
    pub clips: u16,
    pub thing: u8,
    pub map: usize,
    pub commands: Vec<[TicCommand; 4]>,
}
pub fn cases() -> Vec<Case> {
    let mut lines = include_str!("../reference/m3g7_cases.csv").lines();
    let header: Vec<_> = lines.next().unwrap().split(',').collect();
    lines
        .enumerate()
        .map(|(which, line)| {
            let v: BTreeMap<_, _> = header.iter().copied().zip(line.split(',')).collect();
            let tics = v["tics"].parse().unwrap();
            let commands = (0..=tics)
                .map(|op| {
                    let c = &rows(which, op)
                        .iter()
                        .find(|(tag, _)| tag == "UC")
                        .unwrap()
                        .1;
                    let mut result = [TicCommand::default(); 4];
                    result[0] = TicCommand {
                        forward: c[0].try_into().unwrap(),
                        side: c[1].try_into().unwrap(),
                        angle_turn: c[2].try_into().unwrap(),
                        buttons: c[3].try_into().unwrap(),
                    };
                    result
                })
                .collect();
            Case {
                which,
                label: v["label"].into(),
                tics,
                position: [v["px"].parse().unwrap(), v["py"].parse().unwrap()],
                seed: v["seed"].parse().unwrap(),
                pre_damage: v["pre_damage"].parse().unwrap(),
                reaction: v["reaction"].parse().unwrap(),
                clips: v["clips"].parse().unwrap(),
                thing: v["thing"].parse().unwrap(),
                map: v["map"].parse().unwrap(),
                commands,
            }
        })
        .collect()
}
pub fn case_map(index: usize) -> doom_map::Map {
    let wad = doom_wad::Wad::parse(MAP_BYTES[index], doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn setup(c: &Case) -> (Simulation, Actors) {
    let mut s = Simulation::original_player_demon_pistol_map(
        case_map(c.map),
        64,
        MapContextOptions {
            initial_rng: DoomRng::from_indices(c.seed, 0),
            respawn_monsters: false,
        },
    )
    .unwrap();
    let spawned = s
        .spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap()
        .spawned;
    assert_eq!(spawned.len(), usize::from(c.thing != 0));
    let origin = spawned.first().copied();
    let body = s
        .spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
        .unwrap();
    s.initialize_player_pistol(PlayerSlot::One, c.clips)
        .unwrap();
    if c.pre_damage != 0 {
        s.damage_actor(DamageRequest {
            target: body,
            inflictor: None,
            source: origin,
            amount: c.pre_damage,
        })
        .unwrap();
    }
    s.set_initial_player_reaction_time(PlayerSlot::One, c.reaction)
        .unwrap();
    (s, Actors { body, origin })
}
pub fn commands(c: &Case, op: usize) -> [TicCommand; 4] {
    c.commands[op]
}
pub fn evidence() -> &'static evidence_reader::PlayerEvidence {
    static DATA: OnceLock<evidence_reader::PlayerEvidence> = OnceLock::new();
    DATA.get_or_init(|| {
        evidence_reader::PlayerEvidence::new(
            include_str!("../reference/m3g7_trace.csv"),
            "DOOM-PLAYER-DEMON-PISTOL-v1",
            include_bytes!("../reference/m3g7_events.bin"),
            "case,operation,tic,fnv1a64,bytes_fixture,bytes,fields,event_fields\n",
            &[],
        )
    })
}
pub fn rows(which: usize, op: usize) -> &'static [(String, Vec<i64>)] {
    &evidence().rows[&(which, op)]
}
pub fn assert_snapshot(snapshot: &Snapshot, which: usize, op: usize) -> bool {
    let Some(line) = include_str!("../reference/m3g7_snapshots.csv")
        .lines()
        .skip(1)
        .find(|line| {
            let mut f = line.split(',');
            f.next().unwrap().parse::<usize>().unwrap() == which
                && f.next().unwrap().parse::<usize>().unwrap() == op
        })
    else {
        return false;
    };
    let v: Vec<_> = line.split(',').collect();
    let bytes = snapshot
        .diagnostic_bytes(DiagnosticLimits::default())
        .unwrap();
    if v[4] != "-" {
        let full: &[(&str, &[u8])] = &[
            (
                "m3g7_state_0_0.bin",
                include_bytes!("../reference/m3g7_state_0_0.bin"),
            ),
            (
                "m3g7_state_0_19.bin",
                include_bytes!("../reference/m3g7_state_0_19.bin"),
            ),
            (
                "m3g7_state_0_215.bin",
                include_bytes!("../reference/m3g7_state_0_215.bin"),
            ),
            (
                "m3g7_state_0_288.bin",
                include_bytes!("../reference/m3g7_state_0_288.bin"),
            ),
            (
                "m3g7_state_10_19.bin",
                include_bytes!("../reference/m3g7_state_10_19.bin"),
            ),
            (
                "m3g7_state_15_25.bin",
                include_bytes!("../reference/m3g7_state_15_25.bin"),
            ),
            (
                "m3g7_state_2_25.bin",
                include_bytes!("../reference/m3g7_state_2_25.bin"),
            ),
            (
                "m3g7_state_3_39.bin",
                include_bytes!("../reference/m3g7_state_3_39.bin"),
            ),
            (
                "m3g7_state_6_25.bin",
                include_bytes!("../reference/m3g7_state_6_25.bin"),
            ),
            (
                "m3g7_state_9_26.bin",
                include_bytes!("../reference/m3g7_state_9_26.bin"),
            ),
        ];
        let full = full.iter().find(|(name, _)| *name == v[4]).unwrap().1;
        let view = Diagnostic::decode_pistol(full, DiagnosticLimits::default()).unwrap();
        for f in snapshot.fields() {
            assert_eq!(
                Some(f.value()),
                view.field(f.path()),
                "case{which} op{op} {}",
                f.path()
            );
        }
        assert_eq!(bytes, full, "full bytes case{which} op{op}");
    }
    assert_eq!(
        snapshot.digest(),
        u64::from_str_radix(v[3], 16).unwrap(),
        "digest case{which} op{op}"
    );
    assert_eq!(bytes.len(), v[5].parse::<usize>().unwrap());
    assert_eq!(snapshot.fields().count(), v[6].parse::<usize>().unwrap());
    let view = Diagnostic::decode_pistol(&bytes, DiagnosticLimits::default())
        .unwrap_or_else(|e| panic!("decode case{which} op{op}: {e:?}"));
    assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
    true
}
pub fn verify() -> u64 {
    let mut captures = 0;
    let mut payload = 0;
    let mut digest = 0;
    let mut counts = [0; 5];
    let mut weapon_fields = 0;
    for (which, c) in cases().iter().enumerate() {
        let (mut s, a) = setup(c);
        let mut stepped = s.clone();
        let mut trace = SourceTrace::seeded(c.seed);
        for op in 0..=c.tics {
            let pending = s.pending_events().len();
            if op != 0 {
                s.advance(commands(c, op))
                    .unwrap_or_else(|e| panic!("{} op{op}: {e}", c.label));
                let snap = stepped.step(commands(c, op)).unwrap();
                assert_eq!(s.snapshot(), snap, "step/advance {} op{op}", c.label);
            }
            let events = if op == 0 {
                s.pending_events()
            } else {
                &s.events()[pending..]
            };
            let rows = rows(which, op);
            trace.check(&s, which, op, events, rows);
            let snap = s.snapshot();
            shared::check_commands_and_motion(&s, a, &snap, rows);
            shared::check_roster_and_demon(&s, &trace, a, rows);
            let p = s.original_player_pistol(PlayerSlot::One).unwrap().unwrap();
            if let Some((_, v)) = rows.iter().find(|(tag, _)| tag == "WP") {
                let mut got = vec![
                    1,
                    10,
                    3,
                    i64::from(p.bullets),
                    200,
                    i64::from(p.attack_down),
                    i64::from(p.refire),
                    i64::from(p.bob.bits()),
                    i64::from(p.extra_light),
                ];
                for ps in p.psprites {
                    got.extend([
                        ps.state.map_or(-1, |s| i64::from(s.0)),
                        i64::from(ps.tics),
                        i64::from(ps.position[0].bits()),
                        i64::from(ps.position[1].bits()),
                    ]);
                }
                assert_eq!(&got, v, "weapon {} op{op}", c.label);
                weapon_fields += got.len();
            }
            payload += evidence().assert_events(&snap, which, op);
            captures += usize::from(assert_snapshot(&snap, which, op));
        }
        digest ^= s.snapshot().digest();
        for (sum, v) in counts.iter_mut().zip(trace.counts) {
            *sum += v;
        }
    }
    assert_eq!((captures, payload, weapon_fields), (464, 35028, 42585));
    assert_eq!(counts[0], 30914);
    assert_eq!(counts[1], 2784);
    assert_eq!(counts[3], 1574);
    digest
}
