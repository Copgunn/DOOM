//! Explicit canonical-player inputs plus independently observed command evidence.
#![allow(dead_code)]
#[path = "player_execution_evidence.rs"]
mod evidence;
#[path = "player_observations.rs"]
mod observations;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
#[allow(unused_imports)]
pub use observations::SourceTrace;
use std::{collections::BTreeMap, sync::OnceLock};
include!("../reference/m3g4_maps.rs");

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
}
pub fn cases() -> Vec<Case> {
    let mut lines = include_str!("../reference/m3g4_cases.csv").lines();
    let header: Vec<_> = lines.next().unwrap().split(',').collect();
    lines
        .map(|line| {
            let values: BTreeMap<_, _> = header.iter().copied().zip(line.split(',')).collect();
            Case {
                label: values["label"].into(),
                tics: values["tics"].parse().unwrap(),
                forward: values["forward"].parse().unwrap(),
                side: values["side"].parse().unwrap(),
                turn: values["turn"].parse().unwrap(),
                move_hold: values["move_hold"].parse().unwrap(),
                turn_hold: values["turn_hold"].parse().unwrap(),
                position: [values["px"].parse().unwrap(), values["py"].parse().unwrap()],
                reaction: values["reaction"].parse().unwrap(),
                thing: values["thing"].parse().unwrap(),
                pre_damage: values["pre_damage"].parse().unwrap(),
                hit_at: values["hit_at"].parse().unwrap(),
                hit_amount: values["hit_amount"].parse().unwrap(),
                physical: values["physical"] == "1",
                map: values["map"].parse().unwrap(),
            }
        })
        .collect()
}
pub fn case_map(which: usize) -> doom_map::Map {
    let wad = doom_wad::Wad::parse(MAP_BYTES[which], doom_wad::Limits::default()).unwrap();
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
pub fn setup(c: &Case) -> (Simulation, Actors) {
    let mut sim =
        Simulation::original_player_command_map(case_map(c.map), 64, MapContextOptions::default())
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
    if let Some(target) = origin {
        sim.damage_actor(DamageRequest {
            target,
            source: None,
            inflictor: None,
            amount: 150,
        })
        .unwrap();
    }
    if c.pre_damage != 0 {
        sim.damage_actor(DamageRequest {
            target: body,
            source: None,
            inflictor: None,
            amount: c.pre_damage,
        })
        .unwrap();
    }
    let a = Actors { body, origin };
    if c.hit_at == 0 {
        hit(&mut sim, a, c).unwrap();
    }
    sim.set_initial_player_reaction_time(PlayerSlot::One, c.reaction)
        .unwrap();
    (sim, a)
}
pub fn commands(c: &Case, op: usize) -> [TicCommand; 4] {
    let mut commands = [TicCommand::default(); 4];
    commands[usize::from(PlayerSlot::One.ordinal())] = TicCommand {
        forward: if op <= c.move_hold { c.forward } else { 0 },
        side: if op <= c.move_hold { c.side } else { 0 },
        angle_turn: if op <= c.turn_hold { c.turn } else { 0 },
        buttons: 0,
    };
    commands
}
pub fn operations(sim: &mut Simulation, a: Actors, c: &Case, op: usize) {
    if i32::try_from(op).unwrap() == c.hit_at {
        hit(sim, a, c).unwrap();
    }
}
pub fn evidence() -> &'static evidence::PlayerEvidence {
    static DATA: OnceLock<evidence::PlayerEvidence> = OnceLock::new();
    DATA.get_or_init(|| {
        evidence::PlayerEvidence::new(
            include_str!("../reference/m3g4_trace.csv"),
            "DOOM-PLAYER-COMMAND-v1",
            include_bytes!("../reference/m3g4_events.bin"),
            include_str!("../reference/m3g4_snapshots.csv"),
            &[
                (
                    "m3g4_state_1_1.bin",
                    include_bytes!("../reference/m3g4_state_1_1.bin"),
                ),
                (
                    "m3g4_state_6_8.bin",
                    include_bytes!("../reference/m3g4_state_6_8.bin"),
                ),
                (
                    "m3g4_state_12_4.bin",
                    include_bytes!("../reference/m3g4_state_12_4.bin"),
                ),
                (
                    "m3g4_state_13_4.bin",
                    include_bytes!("../reference/m3g4_state_13_4.bin"),
                ),
                (
                    "m3g4_state_15_16.bin",
                    include_bytes!("../reference/m3g4_state_15_16.bin"),
                ),
                (
                    "m3g4_state_18_72.bin",
                    include_bytes!("../reference/m3g4_state_18_72.bin"),
                ),
            ],
        )
    })
}
pub fn rows(which: usize, op: usize) -> &'static [(String, Vec<i64>)] {
    &evidence().rows[&(which, op)]
}
pub fn check_commands_and_motion(
    sim: &Simulation,
    a: Actors,
    snapshot: &Snapshot,
    rows: &[(String, Vec<i64>)],
) -> [usize; 2] {
    let mut counts = [0, 0];
    for (tag, expected) in rows {
        if tag == "PM" {
            let actor = sim.actor(a.body).unwrap();
            let life = actor.life().unwrap();
            let p = sim.original_player(PlayerSlot::One).unwrap();
            let got = [
                i64::from(actor.angle()),
                i64::from(actor.position()[0].bits()),
                i64::from(actor.position()[1].bits()),
                i64::from(actor.momentum()[0].bits()),
                i64::from(actor.momentum()[1].bits()),
                i64::from(actor.z().bits()),
                i64::from(actor.vertical_momentum().bits()),
                i64::from(life.reaction_time),
                life.state.map_or(-1, |s| i64::from(s.0)),
                i64::from(life.tics),
                i64::from(life.health),
                i64::from(p.health),
                i64::from(p.state == PlayerBodyState::Dead),
            ];
            assert_eq!(got.as_slice(), expected);
            counts[0] += got.len();
        } else if tag == "UC" {
            let field = |path| {
                snapshot
                    .fields()
                    .find(|f| f.path() == path)
                    .unwrap()
                    .value()
            };
            let got = [
                i64::from(i8::from_le_bytes(
                    field("player.0.forward").try_into().unwrap(),
                )),
                i64::from(i8::from_le_bytes(
                    field("player.0.side").try_into().unwrap(),
                )),
                i64::from(i16::from_le_bytes(
                    field("player.0.angle_turn").try_into().unwrap(),
                )),
                i64::from(u8::from_le_bytes(
                    field("player.0.buttons").try_into().unwrap(),
                )),
            ];
            assert_eq!(got.as_slice(), expected);
            counts[1] += got.len();
        }
    }
    counts
}
