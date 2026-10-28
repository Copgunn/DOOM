//! Synthetic setups matching m3c_driver.c; expected results come only from C.
#![allow(dead_code)]
#[path = "mod.rs"]
mod base;
use doom_map::{Map, MapLimits};
use doom_sim::*;
use doom_types::Fixed;
use doom_wad::{Limits, Wad};
pub const THINGS: [[i16; 5]; 9] = [
    [0, 0, 90, 1, 7],
    [0, 0, 0, 11, 0],
    [0, 0, 89, 44, 15],
    [32, 0, -91, 49, 7],
    [0, 0, 0, 3004, 7],
    [0, 0, 0, 9999, 16],
    [0, 0, 0, 9999, 0],
    [64, 0, 359, 44, 1],
    [64, 0, 359, 44, 4],
];
pub fn map(things: &[[i16; 5]]) -> Map {
    let bytes = include_bytes!("../reference/m3b_map_19.bin");
    let wad = Wad::parse(bytes, Limits::default()).unwrap();
    let offset = i32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let count = i32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let mut lumps = Vec::new();
    for i in 0..count {
        let at = offset + i * 16;
        let start = i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        let size = i32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let name: [u8; 8] = bytes[at + 8..at + 16].try_into().unwrap();
        let data = if i == 1 {
            things.iter().flat_map(|t| base::words(t)).collect()
        } else {
            bytes[start..start + size].to_vec()
        };
        lumps.push((name, data));
    }
    drop(wad);
    let bytes = base::wad(*b"PWAD", &lumps);
    let wad = Wad::parse(&bytes, Limits::default()).unwrap();
    Map::from_wad(&wad, "MAP01", MapLimits::default()).unwrap()
}
pub fn native_registry(kind: u8) -> ActorRegistry {
    let original = ActorRegistry::vanilla();
    let mut states = original.states().to_vec();
    let mut types = original.types().to_vec();
    for (i, tics, next, action) in [
        (1, 1, 2, ActorAction::None),
        (2, 0, 3, ActorAction::Probe(ProbeAction::Random)),
        (
            3,
            0,
            4,
            ActorAction::Probe(ProbeAction::SpawnTarget(ActorTypeId(132))),
        ),
        (4, 0, 8, ActorAction::Probe(ProbeAction::Enter(StateId(5)))),
        (5, 0, 6, ActorAction::Probe(ProbeAction::Random)),
        (6, 2, 0, ActorAction::Probe(ProbeAction::Random)),
        (8, -1, 0, ActorAction::None),
        (9, 1, 10, ActorAction::None),
        (10, 0, 11, ActorAction::Probe(ProbeAction::RemoveTarget)),
        (11, 1, 0, ActorAction::Probe(ProbeAction::RemoveSelf)),
    ] {
        states[i] = ActorStateDef {
            name: format!("PROBE_{i}"),
            sprite: 0,
            frame: i as u32,
            tics,
            action,
            next: StateId(next),
            misc: [0; 2],
        };
    }
    for (i, ty) in types.iter_mut().enumerate().take(133).skip(131) {
        ty.name = format!("NATIVE_{i}");
        ty.properties = [0; 23];
        ty.properties[0] = -1;
        ty.properties[1] = if i == 131 { 1 } else { 9 };
        ty.properties[2] = 100;
        ty.properties[16] = 8 * 65536;
        ty.properties[17] = 16 * 65536;
        ty.properties[21] = 512;
    }
    if kind >= 7 {
        states[1].tics = -1;
        states[1].next = StateId(0);
        states[1].action = ActorAction::None;
        types[131].properties[9] = 26;
        types[131].properties[14] = 59;
        for (i, name) in [
            (2, "A_Pain"),
            (3, "A_Scream"),
            (4, "A_XScream"),
            (5, "A_PlayerScream"),
            (6, "A_Fall"),
        ] {
            let action = original
                .states()
                .iter()
                .find_map(|s| match s.action {
                    ActorAction::Original(id) if original.action_name(id) == Some(name) => {
                        Some(s.action)
                    }
                    _ => None,
                })
                .unwrap();
            states[i].tics = -1;
            states[i].next = StateId(0);
            states[i].action = action;
        }
    }
    if kind == 8 {
        states[1].tics = 0;
        states[1].next = StateId(2);
        states[1].action = ActorAction::Probe(ProbeAction::Random);
    }
    ActorRegistry::new(states, types).unwrap()
}
pub fn setup(case: u8) -> Simulation {
    let options = ActorOptions {
        skill: match case {
            1 => Skill::Baby,
            2 => Skill::Easy,
            4 => Skill::Hard,
            5 => Skill::Nightmare,
            _ => Skill::Medium,
        },
        no_monsters: (1..=5).contains(&case),
        ..Default::default()
    };
    let mut sim = Simulation::with_actor_registry(
        map(if (1..=5).contains(&case) {
            &THINGS
        } else {
            &[]
        }),
        1024,
        if case >= 6 {
            native_registry(case)
        } else {
            ActorRegistry::vanilla()
        },
        options,
    )
    .unwrap();
    match case {
        0 => {
            let a = sim
                .spawn_type(ActorTypeId(0), [Fixed::ZERO; 2], SpawnHeight::Floor)
                .unwrap();
            sim.spawn_type(
                ActorTypeId(39),
                [Fixed::from_map_units(64), Fixed::ZERO],
                SpawnHeight::Floor,
            )
            .unwrap();
            sim.request_actor_death(a).unwrap();
        }
        1..=5 => {
            let report = sim.spawn_map_things().unwrap();
            assert_eq!(report.player_starts.len(), 1);
            assert_eq!(report.deathmatch_starts.len(), 1);
        }
        _ => {
            sim.spawn_type(ActorTypeId(131), [Fixed::ZERO; 2], SpawnHeight::Floor)
                .unwrap();
        }
    }
    sim
}
