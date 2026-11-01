//! M3h3 genuine real-map-scale inputs and committed source rows.
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
include!("../reference/m3h3_maps.rs");
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
    /// Controlled between-tic P_DamageMobj(thing, NULL, NULL, amount).
    pub hit_at: usize,
    pub hit_thing: usize,
    pub hit_amount: i32,
    /// Controlled between-tic player target plus S_TROO_ATK1 entry.
    pub attack_at: usize,
    pub attack_thing: usize,
    /// Index into the observer's room table; the map itself carries the geometry.
    pub room: usize,
    pub map: usize,
    pub commands: Vec<[TicCommand; 4]>,
}
pub fn cases() -> Vec<Case> {
    let mut lines = include_str!("../reference/m3h3_cases.csv").lines();
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
                ready: match v["ready"] {
                    "0" => Weapon::Fist,
                    "1" => Weapon::Pistol,
                    "2" => Weapon::Shotgun,
                    "3" => Weapon::Chaingun,
                    _ => panic!("committed ready"),
                },
                hit_at: v["hit_at"].parse().unwrap(),
                hit_thing: v["hit_thing"].parse().unwrap(),
                hit_amount: v["hit_amount"].parse().unwrap(),
                attack_at: v["attack_at"].parse().unwrap(),
                attack_thing: v["attack_thing"].parse().unwrap(),
                room: v["room"].parse().unwrap(),
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
pub fn setup(c: &Case) -> (Simulation, Actors, Vec<ActorId>) {
    let mut s = Simulation::original_player_map_scale_map(
        case_map(c.map),
        MapContextOptions {
            initial_rng: DoomRng::from_indices(c.seed, 0),
            respawn_monsters: false,
        },
    )
    .unwrap();
    let spawned = s.spawn_map_things().unwrap().spawned;
    // The C driver's demon_capture origin is the last spawned MT_SERGEANT.
    let origin = spawned.iter().rev().copied().find(|id| {
        s.actor(*id).unwrap().life().unwrap().actor_type.0
            == OriginalActorType::MT_SERGEANT.ordinal()
    });
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
    s.set_initial_player_reaction_time(PlayerSlot::One, c.reaction)
        .unwrap();
    (s, Actors { body, origin }, spawned)
}
pub fn commands(c: &Case, op: usize) -> [TicCommand; 4] {
    c.commands[op]
}
/// The declared controlled between-tic inputs, in the C driver's order.
pub fn controlled(
    s: &mut Simulation,
    c: &Case,
    a: Actors,
    things: &[ActorId],
    op: usize,
) -> Result<(), SimError> {
    if op == c.hit_at {
        s.damage_actor(DamageRequest {
            target: things[c.hit_thing],
            inflictor: None,
            source: None,
            amount: c.hit_amount,
        })?;
    }
    if op == c.attack_at {
        let imp = things[c.attack_thing];
        s.set_actor_target(imp, Some(a.body))?;
        s.enter_original_state(imp, OriginalState::S_TROO_ATK1)
            .expect("declared controlled attack entry");
    }
    Ok(())
}
pub fn evidence() -> &'static evidence_reader::PlayerEvidence {
    static DATA: OnceLock<evidence_reader::PlayerEvidence> = OnceLock::new();
    DATA.get_or_init(|| {
        evidence_reader::PlayerEvidence::new(
            std::str::from_utf8(
                &packed::unpack(include_bytes!("../reference/m3h3_trace.dref")).unwrap(),
            )
            .unwrap(),
            "DOOM-PLAYER-MAP-SCALE-v1",
            &packed::unpack(include_bytes!("../reference/m3h3_events.dref")).unwrap(),
            "case,operation,tic,fnv1a64,bytes_fixture,bytes,fields,event_fields\n",
            &[],
        )
    })
}
pub fn rows(which: usize, op: usize) -> &'static [(String, Vec<i64>)] {
    evidence().rows.get(&(which, op)).map_or(&[], Vec::as_slice)
}
/// Independent DSIM30 digest/size/field count, full bytes where committed,
/// and decode/re-encode round trip through the explicit v30 reader.
pub fn assert_snapshot(snapshot: &Snapshot, which: usize, op: usize) -> bool {
    let Some(line) = include_str!("../reference/m3h3_snapshots.csv")
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
                "m3h3_state_0_0.bin",
                include_bytes!("../reference/m3h3_state_0_0.bin"),
            ),
            (
                "m3h3_state_2_320.bin",
                include_bytes!("../reference/m3h3_state_2_320.bin"),
            ),
            (
                "m3h3_state_8_480.bin",
                include_bytes!("../reference/m3h3_state_8_480.bin"),
            ),
            (
                "m3h3_state_9_480.bin",
                include_bytes!("../reference/m3h3_state_9_480.bin"),
            ),
            (
                "m3h3_state_10_960.bin",
                include_bytes!("../reference/m3h3_state_10_960.bin"),
            ),
            (
                "m3h3_state_11_720.bin",
                include_bytes!("../reference/m3h3_state_11_720.bin"),
            ),
            (
                "m3h3_state_12_512.bin",
                include_bytes!("../reference/m3h3_state_12_512.bin"),
            ),
        ];
        let full = full.iter().find(|(name, _)| *name == v[4]).unwrap().1;
        let view = Diagnostic::decode_map_scale(full, DiagnosticLimits::default()).unwrap();
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
    let view = Diagnostic::decode_map_scale(&bytes, DiagnosticLimits::default())
        .unwrap_or_else(|e| panic!("decode case{which} op{op}: {e:?}"));
    assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
    true
}
/// Source actor/player/weapon/armor/pickup/statistics comparison for one case.
pub fn verify_case(which: usize) -> ([usize; 5], usize, usize, u64) {
    let c = &cases()[which];
    let (mut s, a, things) = setup(c);
    let mut stepped = s.clone();
    let mut trace = SourceTrace::seeded(c.seed);
    let (mut captures, mut payload) = (0, 0);
    for op in 0..=c.tics {
        let pending = s.pending_events().len();
        if op != 0 {
            controlled(&mut s, c, a, &things, op).unwrap();
            controlled(&mut stepped, c, a, &things, op).unwrap();
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
    (trace.counts, captures, payload, s.snapshot().digest())
}
/// Every committed scenario: source fields, ordered DSIM30 events, digests,
/// full bytes and decode round trips. Returns the combined final digest.
pub fn verify() -> u64 {
    let mut counts = [0; 5];
    let (mut captures, mut payload, mut digest) = (0, 0, 0);
    for which in 0..cases().len() {
        let (c, k, p, d) = verify_case(which);
        for (sum, v) in counts.iter_mut().zip(c) {
            *sum += v;
        }
        captures += k;
        payload += p;
        digest ^= d.rotate_left(which as u32);
    }
    println!("map-scale source: captures={captures} payload={payload} source_checks={counts:?}");
    digest
}
