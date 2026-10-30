//! Declared source inputs, shared evidence readers and canonical gameplay setup.
#![allow(dead_code)]
#[path = "player_execution_evidence.rs"]
mod evidence;
#[path = "actor_player_observations.rs"]
mod observations;
use doom_sim::{identity::*, *};
use doom_types::{DoomRng, Fixed};
pub use observations::SourceTrace;
use std::{collections::BTreeMap, sync::OnceLock};
include!("../reference/m3g5_maps.rs");
#[derive(Clone, Debug)]
pub struct Case {
    pub label: String,
    pub tics: usize,
    pub forward: i8,
    pub side: i8,
    pub turn: i16,
    pub move_hold: usize,
    pub turn_hold: usize,
    pub position: [i16; 2],
    pub reaction: u8,
    pub thing: u8,
    pub pre_damage: i32,
    pub hit_at: i32,
    pub hit_amount: i32,
    pub physical: bool,
    pub map: usize,
    pub seed: u8,
    pub threshold: i32,
    pub clear_at: i32,
    pub retire_at: i32,
    pub puff_at: i32,
    pub existing: bool,
    pub see: bool,
}
pub fn cases() -> Vec<Case> {
    let mut lines = include_str!("../reference/m3g5_cases.csv").lines();
    let header: Vec<_> = lines.next().unwrap().split(',').collect();
    lines
        .map(|line| {
            let v: BTreeMap<_, _> = header.iter().copied().zip(line.split(',')).collect();
            Case {
                label: v["label"].into(),
                tics: v["tics"].parse().unwrap(),
                forward: v["forward"].parse().unwrap(),
                side: v["side"].parse().unwrap(),
                turn: v["turn"].parse().unwrap(),
                move_hold: v["move_hold"].parse().unwrap(),
                turn_hold: v["turn_hold"].parse().unwrap(),
                position: [v["px"].parse().unwrap(), v["py"].parse().unwrap()],
                reaction: v["reaction"].parse().unwrap(),
                thing: v["thing"].parse().unwrap(),
                pre_damage: v["pre_damage"].parse().unwrap(),
                hit_at: v["hit_at"].parse().unwrap(),
                hit_amount: v["hit_amount"].parse().unwrap(),
                physical: v["physical"] == "1",
                map: v["map"].parse().unwrap(),
                seed: v["seed"].parse().unwrap(),
                threshold: v["threshold"].parse().unwrap(),
                clear_at: v["clear_at"].parse().unwrap(),
                retire_at: v["retire_at"].parse().unwrap(),
                puff_at: v["puff_at"].parse().unwrap(),
                existing: v["existing"] == "1",
                see: v["see"] == "1",
            }
        })
        .collect()
}
pub fn case_map(index: usize) -> doom_map::Map {
    let wad = doom_wad::Wad::parse(MAP_BYTES[index], doom_wad::Limits::default()).unwrap();
    doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap()
}
pub fn map() -> doom_map::Map {
    case_map(0)
}
#[derive(Clone, Copy)]
pub struct Actors {
    pub body: ActorId,
    pub origin: Option<ActorId>,
}
pub fn hit(sim: &mut Simulation, a: Actors, c: &Case) -> Result<DamageResult, SimError> {
    sim.damage_actor(DamageRequest {
        target: a.body,
        inflictor: if c.physical { a.origin } else { None },
        source: a.origin,
        amount: c.hit_amount,
    })
}
pub fn check_commands_and_motion(
    sim: &Simulation,
    a: Actors,
    snapshot: &Snapshot,
    rows: &[(String, Vec<i64>)],
) -> [usize; 2] {
    observations::check_motion(sim, a.body, snapshot, rows)
}
pub fn setup(c: &Case) -> (Simulation, Actors) {
    let mut sim = Simulation::original_player_demon_map(
        case_map(c.map),
        64,
        MapContextOptions {
            initial_rng: DoomRng::from_indices(c.seed, 0),
            respawn_monsters: false,
        },
    )
    .unwrap();
    let spawned = sim
        .spawn_original_actor(OriginalActorType::MT_SERGEANT)
        .unwrap()
        .spawned;
    assert_eq!(spawned.len(), usize::from(c.thing != 0));
    let origin = spawned.first().copied();
    let body = sim
        .spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
        .unwrap();
    if let Some(id) = origin {
        let angle = sim.actor(id).unwrap().angle();
        sim.configure_perception_observer(id, angle, c.threshold)
            .unwrap();
    }
    if c.pre_damage != 0 {
        sim.damage_actor(DamageRequest {
            target: body,
            inflictor: None,
            source: origin,
            amount: c.pre_damage,
        })
        .unwrap();
    }
    let a = Actors { body, origin };
    if c.hit_at == 0 {
        hit(&mut sim, a, c).unwrap();
    }
    if sim.original_player(PlayerSlot::One).unwrap().state == PlayerBodyState::Live {
        sim.set_initial_player_reaction_time(PlayerSlot::One, c.reaction)
            .unwrap();
    } else {
        assert_eq!(sim.actor(body).unwrap().life().unwrap().reaction_time, 0);
    }
    if c.existing {
        sim.set_actor_target(origin.unwrap(), Some(body)).unwrap();
    }
    if c.see {
        sim.enter_original_state(origin.unwrap(), OriginalState::S_SARG_RUN1)
            .unwrap();
    }
    (sim, a)
}
pub fn commands(c: &Case, op: usize) -> [TicCommand; 4] {
    let mut result = [TicCommand::default(); 4];
    result[usize::from(PlayerSlot::One.ordinal())] = TicCommand {
        forward: if op <= c.move_hold { c.forward } else { 0 },
        side: if op <= c.move_hold { c.side } else { 0 },
        angle_turn: if op <= c.turn_hold { c.turn } else { 0 },
        buttons: 0,
    };
    result
}
pub fn operations(sim: &mut Simulation, a: Actors, c: &Case, op: usize) {
    if i32::try_from(op).unwrap() == c.hit_at {
        hit(sim, a, c).unwrap();
    }
    let op = i32::try_from(op).unwrap();
    if op == c.clear_at {
        sim.set_actor_target(a.origin.unwrap(), None).unwrap();
    }
    if op == c.retire_at {
        sim.mark_remove(a.origin.unwrap()).unwrap();
    }
    if op == c.puff_at {
        sim.spawn_type(
            ActorTypeId(OriginalActorType::MT_PUFF.ordinal()),
            [Fixed::from_map_units(-512), Fixed::ZERO],
            SpawnHeight::Floor,
        )
        .unwrap();
    }
}

pub fn evidence() -> &'static evidence::PlayerEvidence {
    static DATA: OnceLock<evidence::PlayerEvidence> = OnceLock::new();
    DATA.get_or_init(|| {
        evidence::PlayerEvidence::new(
            include_str!("../reference/m3g5_trace.csv"),
            "DOOM-PLAYER-DEMON-v1",
            include_bytes!("../reference/m3g5_events.bin"),
            include_str!("../reference/m3g5_snapshots.csv"),
            &[
                (
                    "m3g5_state_1_10.bin",
                    include_bytes!("../reference/m3g5_state_1_10.bin"),
                ),
                (
                    "m3g5_state_2_10.bin",
                    include_bytes!("../reference/m3g5_state_2_10.bin"),
                ),
                (
                    "m3g5_state_2_11.bin",
                    include_bytes!("../reference/m3g5_state_2_11.bin"),
                ),
                (
                    "m3g5_state_7_10.bin",
                    include_bytes!("../reference/m3g5_state_7_10.bin"),
                ),
                (
                    "m3g5_state_10_32.bin",
                    include_bytes!("../reference/m3g5_state_10_32.bin"),
                ),
                (
                    "m3g5_state_17_128.bin",
                    include_bytes!("../reference/m3g5_state_17_128.bin"),
                ),
                (
                    "m3g5_state_19_32.bin",
                    include_bytes!("../reference/m3g5_state_19_32.bin"),
                ),
                (
                    "m3g5_state_23_32.bin",
                    include_bytes!("../reference/m3g5_state_23_32.bin"),
                ),
            ],
        )
    })
}
pub fn rows(which: usize, op: usize) -> &'static [(String, Vec<i64>)] {
    &evidence().rows[&(which, op)]
}

pub fn check_roster_and_demon(
    sim: &Simulation,
    trace: &SourceTrace,
    a: Actors,
    rows: &[(String, Vec<i64>)],
) -> [usize; 2] {
    let mut counts = [0, 0];
    for (tag, c) in rows {
        if tag == "R" {
            let roster = sim.original_player_roster().unwrap();
            let slot = usize::try_from(c[0]).unwrap();
            let p = roster[slot];
            assert_eq!(
                [
                    c[0],
                    i64::from(p.active),
                    i64::from(p.health),
                    trace.birth(p.actor)
                ]
                .as_slice(),
                c
            );
            counts[0] += 4;
        } else if tag == "DM" {
            let actor = sim.actor(a.origin.unwrap()).unwrap();
            let l = actor.life().unwrap();
            let got = [
                trace.birth(a.origin),
                i64::from(actor.angle()),
                i64::from(actor.position()[0].bits()),
                i64::from(actor.position()[1].bits()),
                i64::from(actor.momentum()[0].bits()),
                i64::from(actor.momentum()[1].bits()),
                l.state.map_or(-1, |s| i64::from(s.0)),
                i64::from(l.tics),
                i64::from(actor.lastlook()),
                i64::from(l.reaction_time),
                i64::from(l.threshold),
                i64::from(l.movedir),
                i64::from(l.movecount),
                trace.birth(l.target),
                i64::from(l.health),
                i64::from(l.flags),
            ];
            assert_eq!(got.as_slice(), c);
            counts[1] += 16;
        }
    }
    counts
}

pub fn verify() -> u64 {
    let mut counts = [0; 5];
    let mut motion = [0; 2];
    let mut roster = [0; 2];
    let mut captures = 0;
    let mut payload = 0;
    let mut digest = 0;
    for (which, c) in cases().iter().enumerate() {
        let (mut sim, a) = setup(c);
        let mut trace = SourceTrace::seeded(c.seed);
        for op in 0..=c.tics {
            let pending = sim.pending_events().len();
            if op != 0 {
                operations(&mut sim, a, c, op);
                sim.advance(commands(c, op))
                    .unwrap_or_else(|e| panic!("{} op{op}: {e}", c.label));
            }
            let events = if op == 0 {
                sim.pending_events()
            } else {
                &sim.events()[pending..]
            };
            let rows = rows(which, op);
            trace.check(&sim, which, op, events, rows);
            let snapshot = sim.snapshot();
            for (sum, v) in motion
                .iter_mut()
                .zip(check_commands_and_motion(&sim, a, &snapshot, rows))
            {
                *sum += v;
            }
            for (sum, v) in roster
                .iter_mut()
                .zip(check_roster_and_demon(&sim, &trace, a, rows))
            {
                *sum += v;
            }
            captures += usize::from(evidence().assert_snapshot(&snapshot, which, op));
            payload += evidence().assert_events(&snapshot, which, op);
        }
        digest ^= sim.snapshot().digest();
        for (sum, v) in counts.iter_mut().zip(trace.counts) {
            *sum += v;
        }
    }
    assert_eq!(counts, [19032, 2238, 732, 563, 1492]);
    assert_eq!(motion, [14937, 4596]);
    assert_eq!(roster, [5968, 17712]);
    assert_eq!((captures, payload), (373, 12886));

    digest
}
