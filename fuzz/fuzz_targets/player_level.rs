#![no_main]
//! Bounded public operations under `PlayerMonstersLevelV1` (wire 22, DSIM38): the
//! body spawned at its type-1 THINGS record in lump order, the `damagecount` decay
//! of `P_PlayerThink`, `P_DeathThink` (the turn toward the attacker, any admitted
//! command, `BT_USE` reborn) and the version-110 demo reader and writer, in the 26
//! committed synthetic M3h9b scenario rooms (not the real E1M1).
//!
//! Input: a six-byte header, then at most 40 nine-byte operation records, then a
//! demo tail. Byte 0 picks one of the 26 scenario starts (room, player position);
//! byte 1 the start (0: the case's own seed, inventory and reaction time; any other
//! value: RNG index `data[1]` and a full chaingun inventory); byte 2 the lump index
//! of the inserted type-1 record (`data[2] % (records + 1)`); bytes 3-4 its angle
//! (little-endian `i16`); byte 5 the record count (`data[5] % 41`). Records: b0 the
//! operation (`b0 % 16`), b1 a slot or selector, b2-b5 and b8 operands, b6-b7 a
//! candidate handle (index, generation). The bytes after the records are a demo
//! (`parse_demo`), whose commands op 8 plays. `-max_len=1404`: 6 + 9 × 40 plus a
//! 1,038-byte demo (13 header bytes, 256 ticcmds, the marker). At most 1,024 tics
//! run per input.
#[path = "../../tests/support/m3h9b.rs"]
mod source;
use doom_sim::{
    demo::{DEMO_HEADER, DEMO_MARKER, DemoError, parse_demo, write_demo},
    identity::{OriginalActorType, OriginalState},
    *,
};
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
use std::{cell::RefCell, collections::BTreeMap, sync::OnceLock};

// `tantoangle` of the preserved source (tables.c), as `R_PointToAngle2` reads it.
include!("../../crates/doom-sim/src/tan_to_angle.rs");

const MF_SPECIAL: u32 = 0x1;
const MF_SOLID: u32 = 0x2;
const MF_SHOOTABLE: u32 = 0x4;
const MF_AMBUSH: u32 = 0x20;
const MF_DROPOFF: u32 = 0x400;
const MF_MISSILE: u32 = 0x10000;
const MF_NOBLOOD: u32 = 0x80000;
const MF_CORPSE: u32 = 0x100000;
/// `BASETHRESHOLD` (p_local.h): a wounded barrel's threshold with its target.
const BASETHRESHOLD: i32 = 100;
/// V′ admits a corpse down to `R_min − 56` (the tallest admitted type).
const V_PRIME_MAX_HEIGHT_UNITS: i32 = 56;
/// p_user.c:180 `ANG5 (ANG90/18)`; tables.h `ANG180`.
const ANG5: u32 = 0x4000_0000 / 18;
const ANG180: u32 = 0x8000_0000;
/// Header bytes, records per input and tics per input.
const HEADER: usize = 6;
const RECORDS: usize = 40;
const TIC_BUDGET: usize = 1024;
/// Inside a long run the invariants are checked every 8th admitted tic and the
/// DSIM38 round trip every 32nd, besides both after every operation.
const RUN_CHECK_EVERY: usize = 8;
const RUN_CAPTURE_EVERY: usize = 32;

fn cases() -> &'static [source::Case] {
    static CASES: OnceLock<Vec<source::Case>> = OnceLock::new();
    CASES.get_or_init(|| {
        let cases = source::cases();
        assert_eq!(cases.len(), 26);
        cases
    })
}

/// A PWAD as an ordered list of named lumps (the committed room bytes, with one
/// type-1 record spliced into THINGS).
fn lumps(wad: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
    let word = |at: usize| u32::from_le_bytes(wad[at..at + 4].try_into().unwrap()) as usize;
    let (count, directory) = (word(4), word(8));
    (0..count)
        .map(|i| {
            let entry = directory + 16 * i;
            let (at, size) = (word(entry), word(entry + 4));
            (
                wad[entry + 8..entry + 16].to_vec(),
                wad[at..at + size].to_vec(),
            )
        })
        .collect()
}
fn wad(lumps: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut data = b"PWAD".to_vec();
    data.extend((lumps.len() as u32).to_le_bytes());
    data.extend([0; 4]);
    let mut directory = Vec::new();
    for (name, bytes) in lumps {
        directory.extend((data.len() as u32).to_le_bytes());
        directory.extend((bytes.len() as u32).to_le_bytes());
        directory.extend(name);
        data.extend(bytes);
    }
    let at = (data.len() as u32).to_le_bytes();
    data[8..12].copy_from_slice(&at);
    data.extend(directory);
    data
}
/// Room `index` with a type-1 record (options 7) at `position` and `angle`
/// inserted at lump index `at % (records + 1)`; returns the map and that index.
fn level_map(index: usize, at: u8, position: [i16; 2], angle: i16) -> (doom_map::Map, usize) {
    let mut l = lumps(source::MAP_BYTES[index]);
    let things = &mut l
        .iter_mut()
        .find(|(n, _)| n.starts_with(b"THINGS"))
        .unwrap()
        .1;
    let at = usize::from(at) % (things.len() / 10 + 1);
    let record: Vec<u8> = [position[0], position[1], angle, 1, 7]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    things.splice(at * 10..at * 10, record);
    let bytes = wad(&l);
    let w = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&w, "MAP01", doom_map::MapLimits::default()).unwrap();
    (map, at)
}

/// `R_PointToAngle2 (x1, y1, x2, y2)` (r_main.c:250-330 through `R_PointToAngle`),
/// with the source's wrapping `x - viewx` and `SlopeDiv`.
fn point_angle(from: [Fixed; 2], to: [Fixed; 2]) -> u32 {
    let x = to[0].bits().wrapping_sub(from[0].bits());
    let y = to[1].bits().wrapping_sub(from[1].bits());
    if x == 0 && y == 0 {
        return 0;
    }
    let slope = |num: u32, den: u32| -> usize {
        if den < 512 {
            2048
        } else {
            ((num.wrapping_shl(3) / (den >> 8)).min(2048)) as usize
        }
    };
    let (ax, ay) = (x.unsigned_abs(), y.unsigned_abs());
    let value = if ax > ay {
        TAN_TO_ANGLE[slope(ay, ax)]
    } else {
        TAN_TO_ANGLE[slope(ax, ay)]
    };
    match (x >= 0, y >= 0, ax > ay) {
        (true, true, true) => value,
        (true, true, false) => 0x3fff_ffff_u32.wrapping_sub(value),
        (true, false, true) => value.wrapping_neg(),
        (true, false, false) => 0xc000_0000_u32.wrapping_add(value),
        (false, true, true) => 0x7fff_ffff_u32.wrapping_sub(value),
        (false, true, false) => 0x4000_0000_u32.wrapping_add(value),
        (false, false, true) => 0x8000_0000_u32.wrapping_add(value),
        (false, false, false) => 0xbfff_ffff_u32.wrapping_sub(value),
    }
}

fn barrel_type() -> u16 {
    OriginalActorType::MT_BARREL.ordinal()
}
fn player_type() -> u16 {
    OriginalActorType::MT_PLAYER.ordinal()
}
fn ty(s: &Simulation, id: ActorId) -> Option<u16> {
    s.actor(id).and_then(Actor::life).map(|l| l.actor_type.0)
}
fn is_barrel(s: &Simulation, id: ActorId) -> bool {
    ty(s, id) == Some(barrel_type())
}
fn is_decoration(s: &Simulation, id: ActorId) -> bool {
    ty(s, id).is_some_and(|t| E1M1_DECORATIONS.iter().any(|d| d.ordinal() == t))
}
/// A live, non-pending handle: an actor that entered `S_NULL` this tic (state
/// `None`, its removal marked) stays in the order until its storage retires.
fn current(s: &Simulation, id: ActorId) -> bool {
    let marked = Event::RemovalMarked(id);
    s.actor_order().any(|a| a == id)
        && s.actor(id)
            .and_then(Actor::life)
            .is_some_and(|l| l.state.is_some())
        && !s.pending_events().contains(&marked)
        && !s.events().contains(&marked)
}
fn player(s: &Simulation) -> OriginalPlayer {
    s.original_player(PlayerSlot::One).unwrap()
}
/// A command the admitted domain must execute: slot One, source movement range,
/// buttons the policy admits (`BT_USE` included). Under wire 22 this holds for a
/// dead player too (`P_DeathThink` reads only `BT_USE`).
fn valid_command(c: &TicCommand) -> bool {
    let weapon = (c.buttons & BT_WEAPONMASK) >> 3;
    (-50..=50).contains(&c.forward)
        && (-50..=50).contains(&c.side)
        && c.buttons & !(BT_ATTACK | BT_USE | BT_CHANGE | BT_WEAPONMASK) == 0
        && if c.buttons & BT_CHANGE != 0 {
            weapon <= 3
        } else {
            weapon == 0
        }
}
/// The vertical invariant V′ (M3h8 note §4.1): one of the cases (a)-(e).
fn v_prime_case(a: &Actor, bounds: MoverBounds) -> Option<char> {
    let l = a.life()?;
    let (z, floor, ceiling) = (a.z(), a.floor(), a.ceiling());
    let top = z + a.height();
    let shootable = l.flags & MF_SHOOTABLE != 0;
    let units = |u: i32| Fixed::from_bits(u * 65536);
    if floor <= z && top <= ceiling {
        Some('a')
    } else if !shootable && z == floor && ceiling < top {
        Some('b')
    } else if floor - units(bounds.delta) <= z && z < floor && top <= ceiling {
        Some('c')
    } else if shootable && z == floor && ceiling < top {
        Some('d')
    } else if l.health <= 0
        && !shootable
        && units(bounds.reach[0] - V_PRIME_MAX_HEIGHT_UNITS) <= z
        && z < floor
        && top <= ceiling
    {
        Some('e')
    } else {
        None
    }
}

/// Everything a check needs about the loaded room.
struct Room {
    boundary: Boundary,
    room: RoomBounds,
    bounds: MoverBounds,
    body: ActorId,
    decorations: Vec<(ActorId, Static)>,
    /// Every actor present after setup with its spawn position, and whether it has
    /// left that position since (spawn rule S′, M3h9b decision F3).
    spawns: RefCell<BTreeMap<ActorId, ([Fixed; 2], bool)>>,
}
/// The static fields a decoration keeps for its whole life (M3h9 note §5.1 item 2).
type Static = (ActorTypeId, Option<StateId>, i32, i32, u32, Fixed, Fixed);
fn decoration_state(s: &Simulation, id: ActorId) -> Option<Static> {
    let a = s.actor(id)?;
    let l = a.life()?;
    Some((
        l.actor_type,
        l.state,
        l.tics,
        l.health,
        l.flags,
        a.radius(),
        a.height(),
    ))
}

/// The public barrel witness of the barrels target, unchanged under wire 22.
fn barrel_invariants(s: &Simulation, room: RoomBounds) {
    let mut count = 0;
    for id in s.actor_order() {
        if !is_barrel(s, id) {
            continue;
        }
        count += 1;
        if !current(s, id) {
            continue;
        }
        let a = s.actor(id).unwrap();
        let l = a.life().unwrap();
        let state = OriginalState::try_from(l.state.expect("a current barrel has a state").0)
            .expect("an original state");
        let alive = matches!(state, OriginalState::S_BAR1 | OriginalState::S_BAR2);
        let gibbed = state == OriginalState::S_GIBS;
        let dying = matches!(
            state,
            OriginalState::S_BEXP
                | OriginalState::S_BEXP2
                | OriginalState::S_BEXP3
                | OriginalState::S_BEXP4
                | OriginalState::S_BEXP5
        );
        assert!(alive || dying || gibbed, "barrel in {state:?}");
        let d = state.descriptor();
        assert_eq!(l.sprite, d.sprite.ordinal());
        assert_eq!(l.frame, d.frame);
        if !gibbed {
            assert!((1..=d.tics).contains(&l.tics), "barrel tics {}", l.tics);
        }
        assert!(l.health <= 20);
        assert_eq!(l.health > 0, alive, "barrel health {}", l.health);
        let flags = l.flags & !MF_AMBUSH;
        let dead = MF_SOLID | MF_NOBLOOD | MF_CORPSE | MF_DROPOFF;
        if alive {
            assert_eq!(flags, MF_SOLID | MF_SHOOTABLE | MF_NOBLOOD);
            assert_eq!(a.radius(), Fixed::from_map_units(10));
            assert_eq!(a.height(), Fixed::from_map_units(42));
            assert!(!a.allows_dropoff());
        } else if dying {
            assert_eq!(flags, dead);
            assert_eq!(a.radius(), Fixed::from_map_units(10));
            assert_eq!(a.height(), Fixed::from_bits(42 << 14));
            assert!(a.allows_dropoff());
        } else {
            assert_eq!(flags & MF_SOLID, 0);
            assert_eq!(a.radius(), Fixed::ZERO);
            assert_eq!(a.height(), Fixed::ZERO);
        }
        assert_eq!(a.solidity() == Solidity::Solid, l.flags & MF_SOLID != 0);
        assert!(l.reaction_time == 8 || l.reaction_time == 0);
        assert!(l.threshold == 0 || l.threshold == BASETHRESHOLD);
        assert_eq!(l.target.is_some(), l.threshold == BASETHRESHOLD);
        assert!(l.movedir == 0 && l.movecount == 0 && !l.death_requested);
        assert!(a.lastlook() <= 3);
        assert_eq!(l.spawnpoint.map(|p| p.kind), Some(2035));
        assert!(room.contains(a.position()), "barrel outside the rectangle");
    }
    assert!(count <= MAX_BARRELS, "{count} barrels");
}
/// The barrels target's invariants (V′, the room rectangle, items on their floor,
/// Ω once moved, the decorations' static fields and `z == floorz`, sector heights,
/// mover counts, secrets, the barrel witness) plus the level policy's: the binding
/// is the body spawned from the record, the only `MT_PLAYER` actor, without a
/// spawnpoint, and `damagecount` stays in `0..=100`.
fn invariants(s: &Simulation, r: &Room) {
    for id in s.actor_order() {
        let a = s.actor(id).unwrap();
        let Some(l) = a.life() else { continue };
        let blast = l.actor_type.0 == OriginalActorType::MT_TROOPSHOT.ordinal()
            && l.flags & MF_MISSILE == 0;
        if matches!(l.actor_type.0, 37..=38) || blast {
            continue;
        }
        assert!(
            v_prime_case(a, r.bounds).is_some(),
            "type {} z {} floor {} ceiling {} height {} health {} outside V'",
            l.actor_type.0,
            a.z().bits(),
            a.floor().bits(),
            a.ceiling().bits(),
            a.height().bits(),
            l.health
        );
        assert!(
            r.room.contains(a.position()),
            "type {} outside the rectangle",
            l.actor_type.0
        );
        if l.flags & MF_SPECIAL != 0 {
            assert_eq!(
                a.z(),
                a.floor(),
                "item type {} off its floor",
                l.actor_type.0
            );
            continue;
        }
        if a.radius() <= Fixed::ZERO {
            continue;
        }
        assert!(
            r.boundary.inside(a.position()),
            "type {} at {:?} outside the boundary",
            l.actor_type.0,
            a.position().map(Fixed::bits)
        );
        let mut spawns = r.spawns.borrow_mut();
        let unmoved = spawns.get_mut(&id).is_some_and(|(at, moved)| {
            *moved |= *at != a.position();
            !*moved
        });
        assert!(
            unmoved
                || !r
                    .boundary
                    .box_meets_wall(a.position(), a.radius() - Fixed::from_bits(1)),
            "type {} at {:?} meets a wall",
            l.actor_type.0,
            a.position().map(Fixed::bits)
        );
    }
    for (id, fixed) in &r.decorations {
        assert!(current(s, *id), "decoration removed");
        assert_eq!(decoration_state(s, *id).as_ref(), Some(fixed));
        let a = s.actor(*id).unwrap();
        assert_eq!(a.z(), a.floor(), "decoration off its floor");
    }
    assert_eq!(
        s.actor_order().filter(|id| is_decoration(s, *id)).count(),
        r.decorations.len()
    );
    let units = |u: i32| Fixed::from_bits(u * 65536);
    for sector in s.sectors() {
        for h in [sector.floor, sector.ceiling] {
            assert!(units(r.bounds.reach[0]) <= h && h <= units(r.bounds.reach[1]));
        }
    }
    let mut movers = 0;
    let mut plats = 0;
    for entry in s.shared_thinker_order() {
        if let ThinkerId::Sector(id) = entry {
            let t = s.sector_thinker(id).unwrap();
            if matches!(
                t,
                SectorThinker::Door(_) | SectorThinker::Plat(_) | SectorThinker::Floor(_)
            ) {
                movers += 1;
                plats += usize::from(matches!(t, SectorThinker::Plat(_)));
            }
        }
    }
    assert!(movers <= r.bounds.movers * 2 && plats <= 30);
    let secrets = s.original_player_secret_count().unwrap();
    let total = s.level_statistics().unwrap().total_secrets;
    assert!(0 <= secrets && secrets <= total);
    barrel_invariants(s, r.room);
    // The level policy: one body, bound since the THINGS batch, never rebound.
    let p = player(s);
    assert_eq!(p.mo, Some(r.body));
    assert!(
        (0..=100).contains(&p.damage_count),
        "damagecount {}",
        p.damage_count
    );
    let players: Vec<_> = s
        .actor_order()
        .filter(|id| ty(s, *id) == Some(player_type()))
        .collect();
    assert_eq!(players, [r.body]);
    let life = s.actor(r.body).unwrap().life().unwrap();
    assert!(life.spawnpoint.is_none());
    assert_eq!(p.state == PlayerBodyState::Live, p.health > 0);
}
/// The DSIM38 capture of an admitted state: `decode_level` round-trips it, the
/// DSIM37 reader rejects it with `Version(38)`, and a mutated copy never panics.
fn capture(s: &Simulation, flip: [u8; 3]) {
    let limits = DiagnosticLimits::default();
    let mut bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    let view = Diagnostic::decode_level(&bytes, limits).unwrap();
    assert_eq!(view.encode(limits).unwrap(), bytes);
    assert_eq!(
        Diagnostic::decode_barrels(&bytes, limits).err(),
        Some(DiagnosticError::Version(38))
    );
    let at = usize::from(u16::from_le_bytes([flip[0], flip[1]])) % bytes.len();
    bytes[at] ^= flip[2];
    if let Ok(v) = Diagnostic::decode_level(&bytes, limits) {
        assert_eq!(v.encode(limits).unwrap(), bytes);
    }
}

/// `parse_demo` on arbitrary bytes: it never panics, its error names the first
/// defect in reading order, and a parsed demo is written back byte for byte by
/// `write_demo` unless a command carries buttons 0x40/0x80, which the writer
/// refuses at the first such tic (the reader keeps any byte).
fn demo_check(bytes: &[u8]) -> Option<Vec<TicCommand>> {
    match parse_demo(bytes) {
        Ok(demo) => {
            let n = demo.commands.len();
            assert_eq!(bytes.len(), DEMO_HEADER.len() + 4 * n + 1);
            assert_eq!(&bytes[..DEMO_HEADER.len()], &DEMO_HEADER);
            assert_eq!(bytes.last(), Some(&DEMO_MARKER));
            for (i, c) in demo.commands.iter().enumerate() {
                assert_ne!(c.forward, i8::MIN);
                assert_eq!(c.angle_turn & 0xff, 0);
                assert_eq!(bytes[DEMO_HEADER.len() + 4 * i + 3], c.buttons);
            }
            match demo.commands.iter().position(|c| c.buttons & 0xc0 != 0) {
                None => assert_eq!(write_demo(&demo.commands).as_deref(), Ok(bytes)),
                Some(tic) => {
                    assert_eq!(write_demo(&demo.commands), Err(DemoError::Buttons { tic }))
                }
            }
            Some(demo.commands)
        }
        Err(DemoError::Truncated) => {
            assert!(bytes.len() < DEMO_HEADER.len());
            None
        }
        Err(DemoError::Header { index, value }) => {
            assert_eq!(bytes[index], value);
            assert_ne!(value, DEMO_HEADER[index]);
            assert_eq!(bytes[..index], DEMO_HEADER[..index]);
            None
        }
        Err(DemoError::UnterminatedCommands | DemoError::TrailingBytes) => {
            assert_eq!(bytes[..DEMO_HEADER.len()], DEMO_HEADER);
            None
        }
        Err(e) => panic!("parse_demo returned a writer error {e:?}"),
    }
}
/// `write_demo` on commands built from arbitrary bytes: a written demo parses back
/// to the same commands; a refusal names the first unrepresentable tic.
fn demo_write_check(raw: &[u8]) {
    let commands: Vec<TicCommand> = raw
        .as_chunks::<4>()
        .0
        .iter()
        .take(64)
        .map(|c| TicCommand {
            forward: i8::from_le_bytes([c[0]]),
            side: i8::from_le_bytes([c[1]]),
            angle_turn: i16::from_le_bytes([u8::from(c[3] == 0xff), c[2]]),
            buttons: c[3],
        })
        .collect();
    let first_bad = commands
        .iter()
        .position(|c| c.forward == i8::MIN || c.buttons & 0xc0 != 0 || c.angle_turn & 0xff != 0);
    match write_demo(&commands) {
        Ok(bytes) => {
            assert_eq!(first_bad, None);
            assert_eq!(parse_demo(&bytes).map(|d| d.commands), Ok(commands));
        }
        Err(
            DemoError::MarkerForward { tic }
            | DemoError::Buttons { tic }
            | DemoError::AngleTurn { tic },
        ) => assert_eq!(first_bad, Some(tic)),
        Err(e) => panic!("write_demo returned a reader error {e:?}"),
    }
}

/// The handles one input addresses.
struct Cast {
    body: ActorId,
    monsters: Vec<ActorId>,
    decorations: Vec<ActorId>,
    pickups: Vec<ActorId>,
    barrels: Vec<ActorId>,
}
impl Cast {
    fn pick(list: &[ActorId], i: u8) -> Option<ActorId> {
        (!list.is_empty()).then(|| list[usize::from(i) % list.len()])
    }
    fn monster(&self, i: u8) -> ActorId {
        Self::pick(&self.monsters, i).unwrap_or(self.body)
    }
    fn decoration(&self, i: u8) -> Option<ActorId> {
        Self::pick(&self.decorations, i)
    }
    fn pickup(&self, i: u8) -> Option<ActorId> {
        Self::pick(&self.pickups, i)
    }
    fn barrel(&self, i: u8) -> Option<ActorId> {
        Self::pick(&self.barrels, i)
    }
}

/// Whether `b` is a tic operation, and how many tics it asks for.
fn tic_count(b: &[u8; 9]) -> Option<usize> {
    match b[0] % 16 {
        0..=3 | 15 => Some(1),
        // A long run (4, 14) or a demo playback (8): up to 256 tics.
        4 | 8 | 14 => Some(1 + usize::from(b[1])),
        _ => None,
    }
}
/// The command of tic `t` of a tic operation. A single tic is the raw record (any
/// slot, any buttons). A long run holds a valid slot-One command (movement clamped
/// to the source range, `BT_ATTACK`/`BT_USE` only, the turn applied every 32nd
/// tic); once the player is dead it keeps sending that command (any admitted
/// command, decision G4) without `BT_USE` unless b8 bit 0x40 asks for the reborn.
/// A demo playback sends the demo's next commands as they are (none past its end).
fn command(
    s: &Simulation,
    b: &[u8; 9],
    t: usize,
    demo: &[TicCommand],
    cursor: usize,
) -> [TicCommand; 4] {
    let mut commands = [TicCommand::default(); 4];
    match b[0] % 16 {
        8 => {
            if let Some(c) = demo.get(cursor + t) {
                commands[0] = *c;
            }
        }
        4 | 14 => {
            let dead = player(s).state != PlayerBodyState::Live;
            let mut buttons = b[8] & (BT_ATTACK | BT_USE);
            if dead && b[8] & 0x40 == 0 {
                buttons &= !BT_USE;
            }
            commands[0] = TicCommand {
                forward: i8::from_le_bytes([b[2]]).clamp(-50, 50),
                side: i8::from_le_bytes([b[3]]).clamp(-50, 50),
                angle_turn: if t % 32 == 0 {
                    i16::from_le_bytes([b[4], b[5]])
                } else {
                    0
                },
                buttons,
            };
        }
        _ => {
            commands[usize::from(b[1] % 4)] = TicCommand {
                forward: i8::from_le_bytes([b[2]]),
                side: i8::from_le_bytes([b[3]]),
                angle_turn: i16::from_le_bytes([b[4], b[5]]),
                buttons: b[8],
            };
        }
    }
    commands
}
/// The witnesses a valid tic may be refused with, state preserved: those of the
/// barrels policy (the 129th intercept, the corner jump, `SpecialCross`, the
/// vertical witness, the per-explosion sight budget, the exit and reborn tics, the
/// stale bomb source) plus the level policy's unresolved dead-player attacker
/// (`Invariant`, `level::death_turn`).
fn declared_witness(e: &SimError) -> bool {
    matches!(
        e,
        SimError::MapScale(
            MapScaleError::Escape
                | MapScaleError::Intercepts
                | MapScaleError::SpecialCross
                | MapScaleError::Vertical
        ) | SimError::Perception(PerceptionError::WorkLimit)
            | SimError::PlayerBody(PlayerBodyError::Invariant)
            | SimError::LevelExited
            | SimError::PlayerReborn
    )
}

/// The player state an admitted tic is checked against.
struct Before {
    paused: bool,
    /// The between-tic events the tic retains at the front of its events.
    pending: usize,
    player: OriginalPlayer,
    angle: u32,
    body: [Fixed; 2],
    /// The attacker's position at the start of the tic, when it resolves.
    attacker: Option<[Fixed; 2]>,
}
impl Before {
    fn of(s: &Simulation, body: ActorId) -> Self {
        let player = player(s);
        let a = s.actor(body).unwrap();
        Self {
            paused: s.paused(),
            pending: s.pending_events().len(),
            player,
            angle: a.angle(),
            body: a.position(),
            attacker: player
                .attacker
                .filter(|id| *id != body)
                .and_then(|id| s.actor(id))
                .map(Actor::position),
        }
    }
}
/// The level policy's per-tic player properties of an admitted tic.
///
/// A paused tic changes neither `damagecount` nor the angle. A tic that starts
/// live decays `damagecount` by one (p_user.c:355-356) unless the player was
/// damaged in it: then it ends at the last damage's count, or one below it when
/// that damage came from `P_PlayerInSpecialSector` before the decrement
/// (p_user.c:274). A tic that starts dead runs `P_DeathThink` (p_user.c:200-225)
/// with the positions at the start of the tic (the player thinks before every
/// thinker): with an attacker other than the body, the angle lands exactly on
/// `R_PointToAngle2 (body, attacker)` when within `ANG5` (and `damagecount`
/// decays), else steps `ANG5` toward it (no decay); without one, the angle is kept
/// and `damagecount` decays. A dead player takes no damage.
fn player_tic(s: &Simulation, body: ActorId, b: &Before) {
    let p = player(s);
    let angle = s.actor(body).unwrap().angle();
    if b.paused {
        assert_eq!(p.damage_count, b.player.damage_count);
        assert_eq!(angle, b.angle);
        return;
    }
    let damaged: Vec<i32> = s.events()[b.pending..]
        .iter()
        .filter_map(|e| match e {
            Event::PlayerBody(PlayerBodyEvent::Damaged { damage_count, .. }) => Some(*damage_count),
            _ => None,
        })
        .collect();
    let decayed = |n: i32| (n - 1).max(0);
    if b.player.state == PlayerBodyState::Live {
        match damaged.last() {
            None => assert_eq!(
                p.damage_count,
                decayed(b.player.damage_count),
                "damagecount did not decay by one"
            ),
            Some(&last) => assert!(
                p.damage_count == last || p.damage_count == decayed(last),
                "damagecount {} after damage to {last}",
                p.damage_count
            ),
        }
        return;
    }
    assert!(damaged.is_empty(), "a dead player was damaged");
    let attacker = b.player.attacker.filter(|id| *id != body);
    let (expected, decay) = match attacker {
        None => (b.angle, true),
        Some(_) => {
            let target = point_angle(
                b.body,
                b.attacker
                    .expect("an admitted death turn with an unresolved attacker"),
            );
            let delta = target.wrapping_sub(b.angle);
            if delta < ANG5 || delta > ANG5.wrapping_neg() {
                (target, true)
            } else if delta < ANG180 {
                (b.angle.wrapping_add(ANG5), false)
            } else {
                (b.angle.wrapping_sub(ANG5), false)
            }
        }
    };
    assert_eq!(angle, expected, "death turn");
    let step = angle.wrapping_sub(b.angle);
    assert!(
        step <= ANG5 || step >= ANG5.wrapping_neg(),
        "turned more than ANG5"
    );
    let count = if decay {
        decayed(b.player.damage_count)
    } else {
        b.player.damage_count
    };
    assert_eq!(p.damage_count, count, "dead damagecount");
}

/// Runs `tics` tics of a tic operation through `advance` or `step`, checking after
/// each one: after an exit or a reborn request every tic is refused with that
/// error and keeps every byte and the RNG; a valid slot-One command (live or dead,
/// decision G4) is admitted or refused only with a declared witness; an admitted
/// tic advances the counter unless paused and keeps the player properties of
/// [`player_tic`]. With `check`, the invariants every 8th admitted tic and the
/// DSIM38 capture every 32nd. Stops at the first refused tic and returns the
/// admitted count and that tic's result.
#[allow(clippy::too_many_arguments)]
fn drive(
    s: &mut Simulation,
    b: &[u8; 9],
    tics: usize,
    step: bool,
    ended: &mut Option<SimError>,
    check: Option<&Room>,
    demo: &[TicCommand],
    cursor: usize,
    body: ActorId,
) -> (usize, Result<(), SimError>) {
    for t in 0..tics {
        let commands = command(s, b, t, demo, cursor);
        let tic = s.tic();
        let before = Before::of(s, body);
        let kept = ended.map(|_| (s.snapshot(), s.rng()));
        let live = before.player.state == PlayerBodyState::Live;
        let result = if step {
            s.step(commands).map(|_| ())
        } else {
            s.advance(commands)
        };
        if let Some(error) = *ended {
            assert_eq!(result, Err(error));
            let (snapshot, rng) = kept.unwrap();
            assert_eq!(s.snapshot(), snapshot);
            assert_eq!(s.rng(), rng);
        }
        let c = commands[0];
        let slot_one = commands[1..].iter().all(|o| *o == TicCommand::default());
        if slot_one && valid_command(&c) {
            assert!(
                result.as_ref().map_or_else(declared_witness, |_| true),
                "valid command refused (live {live}): {c:?} {result:?}"
            );
        }
        if let Err(e) = result {
            return (t, Err(e));
        }
        assert_eq!(s.tic(), if before.paused { tic } else { tic + 1 });
        player_tic(s, body, &before);
        if s.events()
            .iter()
            .any(|e| matches!(e, Event::Mover(MoverEvent::Exit { .. })))
        {
            *ended = Some(SimError::LevelExited);
        }
        // P_DeathThink reads BT_USE only on an unpaused tic that starts dead.
        if !live && c.buttons & BT_USE != 0 && !before.paused {
            *ended = Some(SimError::PlayerReborn);
        }
        if let Some(r) = check {
            if (t + 1) % RUN_CHECK_EVERY == 0 {
                invariants(s, r);
            }
            if (t + 1) % RUN_CAPTURE_EVERY == 0 {
                capture(s, [b[6], b[7], b[8] | 1]);
            }
        }
    }
    (tics, Ok(()))
}

/// One bounded non-tic operation. The refusals the level policy owes are asserted
/// here; the caller asserts that every refused operation keeps the snapshot and
/// the RNG.
fn apply(s: &mut Simulation, cast: &Cast, b: &[u8; 9]) -> Result<(), SimError> {
    let candidate = ActorId::from_parts(u32::from(b[6]), u64::from(b[7]));
    let unsupported = Err(SimError::PlayerBody(PlayerBodyError::Unsupported));
    let push = [
        Fixed::from_map_units(i16::from_le_bytes([b[3], b[4]])),
        Fixed::from_map_units(i16::from_le_bytes([b[5], b[8]])),
    ];
    let momz = Fixed::from_bits(i32::from_le_bytes([b[3], b[4], b[5], b[8]]));
    let who = |k: u8| match k & 3 {
        0 => None,
        1 => Some(cast.monster(b[4])),
        2 => Some(cast.barrel(b[4]).unwrap_or(candidate)),
        _ => Some(cast.body),
    };
    match b[0] % 16 {
        // `damage_actor` aimed at the body, a monster, a decoration, a pickup, a
        // barrel or a candidate handle, with no source or inflictor, a monster, a
        // barrel or the body.
        5 => {
            let target = match b[1] % 6 {
                0 => cast.body,
                1 => cast.monster(b[2]),
                2 => cast.decoration(b[2]).unwrap_or(candidate),
                3 => cast.pickup(b[2]).unwrap_or(candidate),
                4 => cast.barrel(b[2]).unwrap_or(candidate),
                _ => candidate,
            };
            let before = player(s);
            let result = s
                .damage_actor(DamageRequest {
                    target,
                    source: who(b[3]),
                    inflictor: who(b[3] >> 2),
                    amount: i32::from(i16::from_le_bytes([b[5], b[8]])),
                })
                .map(|_| ());
            let after = player(s);
            if target == cast.body && result.is_ok() && before.state == PlayerBodyState::Live {
                // P_DamageMobj never lowers the count (p_inter.c:840-843).
                assert!(after.damage_count >= before.damage_count.min(100));
            }
            result
        }
        // Kill the player: at least 1,000 damage to the body from no source, a
        // monster or the body itself (never a barrel source), with no inflictor or
        // a monster. A live player dies with `damagecount` 100 and the source as
        // its attacker; the following tics run `P_DeathThink`.
        6 => {
            let before = player(s);
            let source = match b[3] % 3 {
                0 => None,
                1 => Some(cast.monster(b[4])),
                _ => Some(cast.body),
            };
            let inflictor = (b[3] & 4 != 0).then(|| cast.monster(b[4]));
            let result = s
                .damage_actor(DamageRequest {
                    target: cast.body,
                    source,
                    inflictor,
                    amount: 1000 + i32::from(b[5]),
                })
                .map(|_| ());
            if result.is_ok() && before.state == PlayerBodyState::Live {
                let p = player(s);
                assert_eq!(p.state, PlayerBodyState::Dead);
                assert_eq!(p.health, 0);
                assert_eq!(p.damage_count, 100);
                assert_eq!(p.attacker, source);
            }
            result
        }
        7 => {
            s.set_paused(b[1] & 1 != 0);
            Ok(())
        }
        // Vertical momentum on the body, a monster or a candidate handle.
        9 => match b[1] % 3 {
            0 => s.set_vertical_momentum(cast.body, momz),
            1 => s.set_vertical_momentum(cast.monster(b[2]), momz),
            _ => s.set_vertical_momentum(candidate, momz),
        },
        // Retarget a monster at the body, a monster, a barrel or a candidate.
        10 => {
            let target = match b[2] % 4 {
                0 => Some(cast.body),
                1 => Some(cast.monster(b[3])),
                2 => cast.barrel(b[3]).or(Some(candidate)),
                _ => Some(candidate),
            };
            s.set_actor_target(cast.monster(b[1]), target)
        }
        // The spawns the level policy refuses: a caller-placed body, a second
        // THINGS batch and a direct `MT_PLAYER` spawn.
        11 => match b[1] % 3 {
            0 => {
                let position = [
                    Fixed::from_map_units(i16::from_le_bytes([b[2], b[3]])),
                    Fixed::from_map_units(i16::from_le_bytes([b[4], b[5]])),
                ];
                let result = s.spawn_player_body(PlayerSlot::One, position).map(|_| ());
                assert_eq!(result, unsupported);
                result
            }
            1 => {
                let result = s.spawn_map_things().map(|_| ());
                assert!(result.is_err(), "a second THINGS batch was admitted");
                result
            }
            _ => {
                let position = [
                    Fixed::from_map_units(i16::from_le_bytes([b[2], b[3]])),
                    Fixed::from_map_units(i16::from_le_bytes([b[4], b[5]])),
                ];
                let result = s
                    .spawn_type(ActorTypeId(player_type()), position, SpawnHeight::Floor)
                    .map(|_| ());
                assert!(result.is_err(), "a direct MT_PLAYER spawn was admitted");
                result
            }
        },
        // Removal: a candidate, a monster (a dead player's attacker then no longer
        // resolves), the current attacker, or the body (refused).
        12 => match b[1] % 4 {
            0 => s.mark_remove(candidate),
            1 => s.mark_remove(cast.monster(b[2])),
            2 => s.mark_remove(player(s).attacker.unwrap_or(candidate)),
            _ => {
                let result = s.mark_remove(cast.body);
                assert!(result.is_err(), "the player body was removed");
                result
            }
        },
        // The controlled push: the body, a monster, a pickup or a candidate.
        _ => match b[1] % 4 {
            0 => s.set_momentum(cast.body, push),
            1 => s.set_momentum(cast.monster(b[2]), push),
            2 => s.set_momentum(cast.pickup(b[2]).unwrap_or(candidate), push),
            _ => s.set_momentum(candidate, push),
        },
    }
}

/// A DSIM38 capture of the default setup of case 0, a read-only decoder input.
fn reference_capture() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let c = &cases()[0];
        let (map, _) = level_map(c.map, 0, c.position, 0);
        let mut s = Simulation::original_player_level_map(map, source::options(c.seed)).unwrap();
        s.spawn_map_things().unwrap();
        s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
            .unwrap();
        s.spawn_level_specials().unwrap();
        s.snapshot()
            .diagnostic_bytes(DiagnosticLimits::default())
            .unwrap()
    })
}

fuzz_target!(|data: &[u8]| {
    // The demo reader on the whole input and the writer on commands built from it.
    demo_check(data);
    demo_write_check(data);
    if data.len() < HEADER {
        return;
    }
    let mut bytes = reference_capture().to_vec();
    for c in data.as_chunks::<3>().0.iter().take(32) {
        let at = usize::from(u16::from_le_bytes([c[0], c[1]])) % bytes.len();
        bytes[at] ^= c[2];
    }
    if let Ok(v) = Diagnostic::decode_level(&bytes, DiagnosticLimits::default()) {
        assert_eq!(v.encode(DiagnosticLimits::default()).unwrap(), bytes);
    }
    let count = usize::from(data[5]) % (RECORDS + 1);
    let records: Vec<[u8; 9]> = data[HEADER..]
        .as_chunks::<9>()
        .0
        .iter()
        .take(count)
        .copied()
        .collect();
    let tail = &data[(HEADER + 9 * records.len()).min(data.len())..];
    let demo = demo_check(tail).unwrap_or_default();

    let c = &cases()[usize::from(data[0]) % 26];
    let angle = i16::from_le_bytes([data[3], data[4]]);
    let (map, at) = level_map(c.map, data[2], c.position, angle);
    let boundary = Boundary::derive(&map).unwrap();
    let room = RoomBounds::derive_real_map(&map).unwrap();
    let bounds = e1m1_mover_bounds(&map).unwrap();
    let seed = if data[1] == 0 { c.seed } else { data[1] };
    let mut s = Simulation::original_player_level_map(map, source::options(seed)).unwrap();
    let report = s.spawn_map_things().unwrap();
    let body = report.player_body.expect("the body spawned at its record");
    let things = report.spawned;
    // The body sits at its record's lump position with `ANG45 * (angle/45)`.
    let order: Vec<_> = s.actor_order().collect();
    let position = order.iter().position(|id| *id == body).unwrap();
    assert!(position <= at && order.len() == things.len() + 1);
    assert_eq!(
        s.actor(body).unwrap().angle(),
        ((i32::from(angle) / 45) as u32).wrapping_mul(0x2000_0000)
    );
    assert_eq!(
        s.actor(body).unwrap().position(),
        c.position.map(Fixed::from_map_units)
    );
    let inventory = if data[1] == 0 {
        WeaponInventory {
            ready: c.ready,
            owned: c.owned,
            ammo: [c.clips, 0, 0, 0],
        }
    } else {
        WeaponInventory {
            ready: Weapon::Chaingun,
            owned: 15,
            ammo: [200, 50, 0, 0],
        }
    };
    s.initialize_player_weapons(PlayerSlot::One, inventory)
        .unwrap();
    s.spawn_level_specials().unwrap();
    if data[1] == 0 {
        s.set_initial_player_reaction_time(PlayerSlot::One, c.reaction)
            .unwrap();
    }
    let of = |f: &dyn Fn(&Simulation, ActorId) -> bool| -> Vec<ActorId> {
        things.iter().copied().filter(|id| f(&s, *id)).collect()
    };
    let barrels = of(&|s, id| is_barrel(s, id));
    let decorations = of(&|s, id| is_decoration(s, id));
    let pickups = of(&|s, id| {
        s.actor(id)
            .and_then(Actor::life)
            .is_some_and(|l| l.flags & MF_SPECIAL != 0)
    });
    let monsters = of(&|s, id| {
        !is_barrel(s, id)
            && s.actor(id)
                .and_then(Actor::life)
                .is_some_and(|l| l.flags & MF_SHOOTABLE != 0)
    });
    assert!(barrels.len() <= MAX_BARRELS && decorations.len() <= MAX_DECORATIONS);
    assert!(monsters.len() <= MAX_PLAYER_MONSTERS && pickups.len() <= BARRELS_MAX_PICKUPS);
    let fixed = decorations
        .iter()
        .map(|id| (*id, decoration_state(&s, *id).unwrap()))
        .collect();
    let r = Room {
        boundary,
        room,
        bounds,
        body,
        decorations: fixed,
        spawns: RefCell::new(
            s.actor_order()
                .map(|id| (id, (s.actor(id).unwrap().position(), false)))
                .collect(),
        ),
    };
    let cast = Cast {
        body,
        monsters,
        decorations,
        pickups,
        barrels,
    };
    invariants(&s, &r);
    capture(&s, [0, 0, 1]);
    let mut ended = None;
    let mut budget = TIC_BUDGET;
    let mut cursor = 0;
    for (index, b) in records.iter().enumerate() {
        let (before, rng) = (s.snapshot(), s.rng());
        if let Some(asked) = tic_count(b) {
            let tics = asked.min(budget);
            budget -= tics;
            let start = s.clone();
            let ended_before = ended;
            let (admitted, result) = drive(
                &mut s,
                b,
                tics,
                false,
                &mut ended,
                Some(&r),
                &demo,
                cursor,
                body,
            );
            // `step` equals `advance` on a fresh clone, every fourth operation.
            if index % 4 == 0 {
                let mut fresh = start.clone();
                let mut e = ended_before;
                assert_eq!(
                    drive(&mut fresh, b, tics, true, &mut e, None, &demo, cursor, body),
                    (admitted, result)
                );
                assert_eq!(fresh.snapshot(), s.snapshot());
                assert_eq!(e, ended);
            }
            // The transaction: a refused tic keeps every byte and the RNG of the
            // state before it (replayed from the operation's start).
            if result.is_err() {
                let mut replay = start;
                let mut e = ended_before;
                assert_eq!(
                    drive(
                        &mut replay,
                        b,
                        admitted,
                        false,
                        &mut e,
                        None,
                        &demo,
                        cursor,
                        body
                    ),
                    (admitted, Ok(()))
                );
                assert_eq!(s.snapshot(), replay.snapshot());
                assert_eq!(s.rng(), replay.rng());
            }
            if b[0] % 16 == 8 {
                cursor += tics;
            }
            invariants(&s, &r);
            if admitted > 0 {
                capture(&s, [b[6], b[7], b[8]]);
            }
        } else {
            let result = apply(&mut s, &cast, b);
            if result.is_err() {
                assert_eq!(s.snapshot(), before);
                assert_eq!(s.rng(), rng);
            }
            invariants(&s, &r);
        }
    }
    capture(&s, [0, 0, 0x80]);
});
