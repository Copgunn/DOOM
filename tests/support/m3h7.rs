//! M3h7 genuine sky and static-specials inputs and committed source rows.
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
include!("../reference/m3h7_maps.rs");
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
    /// Harness-only between-tic push: the player body's momentum in map units.
    pub push_at: usize,
    pub push: [i16; 2],
    /// Index into the observer's room table; the map itself carries the geometry.
    pub room: usize,
    pub map: usize,
    pub commands: Vec<[TicCommand; 4]>,
}
pub fn cases() -> Vec<Case> {
    let mut lines = include_str!("../reference/m3h7_cases.csv").lines();
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
                push_at: v["push_at"].parse().unwrap(),
                push: [v["push_x"].parse().unwrap(), v["push_y"].parse().unwrap()],
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
    let mut s = Simulation::original_player_specials_map(
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
    // P_SetupPsprites, then P_SpawnSpecials (the C driver's P_SetupLevel order).
    s.spawn_level_specials().unwrap();
    s.set_initial_player_reaction_time(PlayerSlot::One, c.reaction)
        .unwrap();
    (s, Actors { body, origin }, spawned)
}
pub fn commands(c: &Case, op: usize) -> [TicCommand; 4] {
    c.commands[op]
}
/// The declared controlled between-tic inputs, in the C driver's order: the push
/// (the C adapter sets `momx`/`momy` before the command stage; here
/// `set_momentum`, admitted for the player body under this policy only), then the
/// damage and the attack entry.
pub fn controlled(
    s: &mut Simulation,
    c: &Case,
    a: Actors,
    things: &[ActorId],
    op: usize,
) -> Result<(), SimError> {
    if op == c.push_at {
        s.set_momentum(a.body, c.push.map(Fixed::from_map_units))?;
    }
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
                &packed::unpack(include_bytes!("../reference/m3h7_trace.dref")).unwrap(),
            )
            .unwrap(),
            "DOOM-PLAYER-SPECIALS-v1",
            &packed::unpack(include_bytes!("../reference/m3h7_events.dref")).unwrap(),
            "case,operation,tic,fnv1a64,bytes_fixture,bytes,fields,event_fields\n",
            &[],
        )
    })
}
pub fn rows(which: usize, op: usize) -> &'static [(String, Vec<i64>)] {
    &evidence().rows[&(which, op)]
}
/// The committed capture count (`m3h7_snapshot_contract.json`) and compared tics
/// (every scripted tic plus the spawn capture of each scenario) that `verify`
/// requires, so empty or truncated evidence can never pass.
pub const EXPECTED_CAPTURES: usize = 427;
pub const EXPECTED_COMPARED_TICS: usize = 5160 + 17;
/// The committed full-byte fixtures (`m3h7_snapshot_contract.json`).
pub const FIXTURES: &[(&str, &[u8])] = &[
    (
        "m3h7_state_0_0.bin",
        include_bytes!("../reference/m3h7_state_0_0.bin"),
    ),
    (
        "m3h7_state_0_1200.bin",
        include_bytes!("../reference/m3h7_state_0_1200.bin"),
    ),
    (
        "m3h7_state_11_19.bin",
        include_bytes!("../reference/m3h7_state_11_19.bin"),
    ),
    (
        "m3h7_state_12_19.bin",
        include_bytes!("../reference/m3h7_state_12_19.bin"),
    ),
    (
        "m3h7_state_1_97.bin",
        include_bytes!("../reference/m3h7_state_1_97.bin"),
    ),
    (
        "m3h7_state_2_129.bin",
        include_bytes!("../reference/m3h7_state_2_129.bin"),
    ),
    (
        "m3h7_state_4_30.bin",
        include_bytes!("../reference/m3h7_state_4_30.bin"),
    ),
    (
        "m3h7_state_6_110.bin",
        include_bytes!("../reference/m3h7_state_6_110.bin"),
    ),
    (
        "m3h7_state_7_127.bin",
        include_bytes!("../reference/m3h7_state_7_127.bin"),
    ),
];
/// Independent DSIM34 digest/size/field count, full bytes where committed,
/// and decode/re-encode round trip through the explicit v34 reader.
pub fn assert_snapshot(snapshot: &Snapshot, which: usize, op: usize) -> bool {
    let Some(line) = include_str!("../reference/m3h7_snapshots.csv")
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
        let full = FIXTURES;
        let full = full.iter().find(|(name, _)| *name == v[4]).unwrap().1;
        let view = Diagnostic::decode_specials(full, DiagnosticLimits::default()).unwrap();
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
    let view = Diagnostic::decode_specials(&bytes, DiagnosticLimits::default())
        .unwrap_or_else(|e| panic!("decode case{which} op{op}: {e:?}"));
    assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
    true
}
/// The sector rows of one op: `SX` light/special/secret counts and `TK` shared
/// thinker order with every sector thinker's fields. Returns compared values.
pub fn check_specials(
    s: &Simulation,
    trace: &SourceTrace,
    which: usize,
    op: usize,
    rows: &[(String, Vec<i64>)],
) -> usize {
    let mut compared = 0;
    let sx = rows.iter().find(|(tag, _)| tag == "SX").map(|(_, v)| v);
    let sx = sx.unwrap_or_else(|| panic!("SX row case{which} op{op}"));
    let mut got = vec![
        i64::try_from(s.tic()).unwrap(),
        i64::from(s.original_player_secret_count().unwrap()),
        i64::from(s.level_statistics().unwrap().total_secrets),
        i64::try_from(s.sectors().len()).unwrap(),
    ];
    for (light, special) in s.sector_lights_and_specials().unwrap() {
        got.extend([i64::from(light), i64::from(special)]);
    }
    assert_eq!(&got, sx, "SX case{which} op{op}");
    compared += got.len();
    let expected: Vec<_> = rows.iter().filter(|(tag, _)| tag == "TK").collect();
    let order: Vec<_> = s.shared_thinker_order().collect();
    assert_eq!(order.len(), expected.len(), "TK count case{which} op{op}");
    for (index, (entry, (_, c))) in order.iter().zip(expected).enumerate() {
        let got = match *entry {
            ThinkerId::Actor(id) => {
                let a = s.actor(id).unwrap();
                vec![
                    index as i64,
                    if s.actor_availability(id) == ActorAvailability::PendingRemoval {
                        9
                    } else {
                        0
                    },
                    trace.birth(Some(id)),
                    i64::from(a.life().unwrap().actor_type.0),
                ]
            }
            ThinkerId::Sector(id) => {
                let t = s.sector_thinker(id).unwrap();
                let (kind, f) = match *t {
                    SectorThinker::Flash(t) => (
                        1,
                        [
                            t.sector.into(),
                            t.count,
                            t.minlight,
                            t.maxlight,
                            t.maxtime,
                            t.mintime,
                        ],
                    ),
                    SectorThinker::Strobe(t) => (
                        2,
                        [
                            t.sector.into(),
                            t.count,
                            t.minlight,
                            t.maxlight,
                            t.darktime,
                            t.brighttime,
                        ],
                    ),
                    SectorThinker::Glow(t) => (
                        3,
                        [t.sector.into(), -1, t.minlight, t.maxlight, t.direction, 0],
                    ),
                    SectorThinker::Flicker(t) => {
                        (4, [t.sector.into(), t.count, t.minlight, t.maxlight, 0, 0])
                    }
                };
                assert_eq!(id.generation(), 1, "load-only sector thinkers");
                let mut v = vec![index as i64, kind, i64::from(id.slot())];
                v.extend(f.map(i64::from));
                v
            }
        };
        assert_eq!(&got, c, "TK case{which} op{op} entry{index}");
        compared += got.len();
    }
    compared
}
/// Source actor/player/weapon/armor/pickup/statistics, sector and thinker-order
/// comparison for one case; no committed scenario is refused. Returns the trace
/// counts, captures, event payload, final digest and compared tics.
pub fn verify_case(which: usize) -> ([usize; 5], usize, usize, u64, usize, usize) {
    let c = &cases()[which];
    let (mut s, a, things) = setup(c);
    let mut stepped = s.clone();
    let mut trace = SourceTrace::seeded(c.seed);
    let (mut captures, mut payload, mut compared, mut sector) = (0, 0, 0, 0);
    for op in 0..=c.tics {
        let pending = s.pending_events().len();
        if op != 0 {
            controlled(&mut s, c, a, &things, op).unwrap();
            controlled(&mut stepped, c, a, &things, op).unwrap();
            // During the step completing tic `op` the source leveltime is op - 1:
            // every P_PlayerInSpecialSector entry of this op prints it.
            assert_eq!(s.tic(), op as u64 - 1);
            for (_, v) in rows(which, op).iter().filter(|(tag, _)| tag == "SP") {
                assert_eq!(v[5], op as i64 - 1, "SP leveltime {} op{op}", c.label);
            }
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
        sector += check_specials(&s, &trace, which, op, rows);
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
        compared += 1;
    }
    (
        trace.counts,
        captures,
        payload,
        s.snapshot().digest(),
        compared,
        sector,
    )
}
/// Every committed scenario: source fields, ordered DSIM34 events, digests,
/// full bytes and decode round trips. Returns the combined final digest.
pub fn verify() -> u64 {
    let mut counts = [0; 5];
    let (mut captures, mut payload, mut digest, mut compared, mut sector) = (0, 0, 0, 0, 0);
    for which in 0..cases().len() {
        let (c, k, p, d, n, x) = verify_case(which);
        for (sum, v) in counts.iter_mut().zip(c) {
            *sum += v;
        }
        captures += k;
        payload += p;
        compared += n;
        sector += x;
        digest ^= d.rotate_left(which as u32);
    }
    // Every value of every committed SX and TK row was compared.
    let rows: usize = evidence()
        .rows
        .values()
        .flatten()
        .filter(|(tag, _)| tag == "SX" || tag == "TK")
        .map(|(_, v)| v.len())
        .sum();
    println!(
        "specials source: captures={captures} payload={payload} compared_tics={compared} sector_values={sector} source_checks={counts:?}"
    );
    assert_eq!(
        captures, EXPECTED_CAPTURES,
        "independent DSIM34 captures compared"
    );
    assert_eq!(compared, EXPECTED_COMPARED_TICS, "scripted tics compared");
    assert_eq!(sector, rows, "every SX/TK value compared");
    assert!(payload > 0 && counts.iter().all(|c| *c > 0), "{counts:?}");
    digest
}
