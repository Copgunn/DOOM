//! Genuine four-weapon inputs and independently committed source/schema evidence.
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
#[allow(clippy::duplicate_mod)]
#[path = "evidence_pack.rs"]
mod packed;
include!("../reference/m3g10_maps.rs");
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
    pub shells: i32,
    pub ready: Weapon,
    pub owned: u16,
    pub part: u8,
    pub thing: u8,
    pub drop: u16,
    pub drop_position: [i16; 2],
    pub physical: bool,
    pub map: usize,
    pub commands: Vec<[TicCommand; 4]>,
}
pub fn cases() -> Vec<Case> {
    let mut lines = include_str!("../reference/m3g10_cases.csv").lines();
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
                shells: v["shells"].parse().unwrap(),
                owned: v["owned"].parse().unwrap(),
                part: v["part"].parse().unwrap(),
                ready: match v["ready"] {
                    "0" => Weapon::Fist,
                    "1" => Weapon::Pistol,
                    "2" => Weapon::Shotgun,
                    "3" => Weapon::Chaingun,
                    _ => panic!("committed ready"),
                },
                thing: v["thing"].parse().unwrap(),
                drop: v["drop"].parse().unwrap(),
                drop_position: [v["drop_x"].parse().unwrap(), v["drop_y"].parse().unwrap()],
                physical: v["physical"] == "1",
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
    let mut s = Simulation::original_player_armor_map(
        case_map(c.map),
        64,
        MapContextOptions {
            initial_rng: DoomRng::from_indices(c.seed, 0),
            respawn_monsters: false,
        },
    )
    .unwrap();
    let spawned = s.spawn_map_things().unwrap().spawned;
    let origin = spawned.iter().copied().find(|id| {
        s.actor(*id).unwrap().life().unwrap().actor_type.0
            == OriginalActorType::MT_SERGEANT.ordinal()
    });
    assert_eq!(origin.is_some(), c.thing != 0);
    if c.drop != 0 {
        let k = match c.drop {
            63 => PickupKind::Clip,
            73 => PickupKind::Chaingun,
            77 => PickupKind::Shotgun,
            _ => panic!("committed drop"),
        };
        s.spawn_dropped_pickup(k, c.drop_position.map(Fixed::from_map_units))
            .unwrap();
    }
    let body = s
        .spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
        .unwrap();
    s.initialize_player_weapons(
        PlayerSlot::One,
        WeaponInventory {
            ready: c.ready,
            owned: c.owned,
            ammo: [i32::from(c.clips), c.shells, 0, 0],
        },
    )
    .unwrap();
    if c.pre_damage != 0 {
        s.damage_actor(DamageRequest {
            target: body,
            inflictor: if c.physical { origin } else { None },
            source: origin,
            amount: c.pre_damage,
        })
        .unwrap();
    }
    if s.original_player(PlayerSlot::One).unwrap().state == PlayerBodyState::Live {
        s.set_initial_player_reaction_time(PlayerSlot::One, c.reaction)
            .unwrap();
    }
    (s, Actors { body, origin })
}
pub fn commands(c: &Case, op: usize) -> [TicCommand; 4] {
    c.commands[op]
}
pub fn evidence() -> &'static evidence_reader::PlayerEvidence {
    static DATA: OnceLock<evidence_reader::PlayerEvidence> = OnceLock::new();
    DATA.get_or_init(|| {
        evidence_reader::PlayerEvidence::new(
            std::str::from_utf8(
                &packed::unpack(include_bytes!("../reference/m3g10_trace.dref")).unwrap(),
            )
            .unwrap(),
            "DOOM-PLAYER-ARMOR-v1",
            &packed::unpack(include_bytes!("../reference/m3g10_events.dref")).unwrap(),
            "case,operation,tic,fnv1a64,bytes_fixture,bytes,fields,event_fields\n",
            &[],
        )
    })
}
pub fn rows(which: usize, op: usize) -> &'static [(String, Vec<i64>)] {
    &evidence().rows[&(which, op)]
}
pub fn assert_snapshot(snapshot: &Snapshot, which: usize, op: usize) -> bool {
    let Some(line) = include_str!("../reference/m3g10_snapshots.csv")
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
                "m3g10_state_0_0.bin",
                include_bytes!("../reference/m3g10_state_0_0.bin"),
            ),
            (
                "m3g10_state_13_1.bin",
                include_bytes!("../reference/m3g10_state_13_1.bin"),
            ),
            (
                "m3g10_state_17_1.bin",
                include_bytes!("../reference/m3g10_state_17_1.bin"),
            ),
            (
                "m3g10_state_1_6.bin",
                include_bytes!("../reference/m3g10_state_1_6.bin"),
            ),
            (
                "m3g10_state_22_64.bin",
                include_bytes!("../reference/m3g10_state_22_64.bin"),
            ),
            (
                "m3g10_state_23_128.bin",
                include_bytes!("../reference/m3g10_state_23_128.bin"),
            ),
            (
                "m3g10_state_23_786.bin",
                include_bytes!("../reference/m3g10_state_23_786.bin"),
            ),
            (
                "m3g10_state_25_1216.bin",
                include_bytes!("../reference/m3g10_state_25_1216.bin"),
            ),
            (
                "m3g10_state_25_512.bin",
                include_bytes!("../reference/m3g10_state_25_512.bin"),
            ),
            (
                "m3g10_state_25_706.bin",
                include_bytes!("../reference/m3g10_state_25_706.bin"),
            ),
            (
                "m3g10_state_26_1024.bin",
                include_bytes!("../reference/m3g10_state_26_1024.bin"),
            ),
            (
                "m3g10_state_26_1216.bin",
                include_bytes!("../reference/m3g10_state_26_1216.bin"),
            ),
            (
                "m3g10_state_26_896.bin",
                include_bytes!("../reference/m3g10_state_26_896.bin"),
            ),
            (
                "m3g10_state_27_128.bin",
                include_bytes!("../reference/m3g10_state_27_128.bin"),
            ),
            (
                "m3g10_state_28_24.bin",
                include_bytes!("../reference/m3g10_state_28_24.bin"),
            ),
            (
                "m3g10_state_2_6.bin",
                include_bytes!("../reference/m3g10_state_2_6.bin"),
            ),
            (
                "m3g10_state_8_1.bin",
                include_bytes!("../reference/m3g10_state_8_1.bin"),
            ),
            (
                "m3g10_state_9_1.bin",
                include_bytes!("../reference/m3g10_state_9_1.bin"),
            ),
        ];
        let full = full.iter().find(|(name, _)| *name == v[4]).unwrap().1;
        let view = Diagnostic::decode_armor(full, DiagnosticLimits::default()).unwrap();
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
    let view = Diagnostic::decode_armor(&bytes, DiagnosticLimits::default())
        .unwrap_or_else(|e| panic!("decode case{which} op{op}: {e:?}"));
    assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
    true
}
pub fn verify() -> u64 {
    verify_range(0..cases().len())
}
pub fn verify_range(selected: std::ops::Range<usize>) -> u64 {
    let mut captures = 0;
    let mut payload = 0;
    let mut digest = 0;
    let mut counts = [0; 5];
    let mut weapon_fields = 0;
    for (which, c) in cases().iter().enumerate() {
        if !selected.contains(&which) {
            continue;
        }
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
            let p = s.original_player_weapons(PlayerSlot::One).unwrap().unwrap();
            if let Some((_, v)) = rows.iter().find(|(tag, _)| tag == "WX") {
                let mut got = vec![
                    p.ready as i64,
                    p.pending.map_or(10, |w| w as i64),
                    i64::from(p.owned),
                    i64::from(p.bullets),
                    i64::from(p.shells),
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
            let armor = s.original_player_armor(PlayerSlot::One).unwrap();
            if let Some((_, v)) = rows.iter().find(|(tag, _)| tag == "AX") {
                assert_eq!(
                    [i64::from(armor.kind.wire_id()), i64::from(armor.points)].as_slice(),
                    v,
                    "armor {} op{op}",
                    c.label
                );
            }
            let pickup = s.original_player_pickups(PlayerSlot::One).unwrap();
            if let Some((_, v)) = rows.iter().find(|(tag, _)| tag == "PX") {
                assert_eq!(
                    [
                        i64::from(pickup.bonus_count),
                        i64::from(pickup.item_count),
                        pickup.message as i64
                    ]
                    .as_slice(),
                    v,
                    "pickup {} op{op}",
                    c.label
                );
            }
            payload += evidence().assert_events(&snap, which, op);
            captures += usize::from(assert_snapshot(&snap, which, op));
        }
        digest ^= s.snapshot().digest();
        for (sum, v) in counts.iter_mut().zip(trace.counts) {
            *sum += v;
        }
    }
    println!(
        "armor source: captures={captures} payload={payload} weapon_fields={weapon_fields} source_checks={counts:?}"
    );
    digest
}
