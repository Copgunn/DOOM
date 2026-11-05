#![no_main]
//! Bounded public operations under `PlayerMonstersBarrelsV1` (wire 21, DSIM37):
//! exploding barrels, `A_Explode`/`P_RadiusAttack`, 64 pickups and every THING
//! angle in the 26 committed synthetic M3h9b scenario rooms (not the real E1M1).
//!
//! Input: byte 0 picks one of the 26 scenario starts (room, player spawn), byte 1
//! the start (0: the C driver's own setup of that case, `source::setup`; any other
//! value: the same room and spawn with RNG index `data[1]` and a full chaingun
//! inventory), then at most 40 nine-byte operation records (`-max_len=362`): b0
//! the operation (`b0 % 16`), b1 a slot or selector, b2-b5 and b8 operands, b6-b7
//! a candidate handle (index, generation). At most 1,024 tics run per input.
#[path = "../../tests/support/m3h9b.rs"]
mod source;
use doom_sim::{
    identity::{OriginalActorType, OriginalState},
    *,
};
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
use std::{cell::RefCell, collections::BTreeMap, sync::OnceLock};

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
/// Records per input and tics per input (the `-max_len` is `2 + 9 × 40 = 362`).
const RECORDS: usize = 40;
const TIC_BUDGET: usize = 1024;
/// Inside a long run the invariants are checked every 8th admitted tic and the
/// DSIM37 round trip every 32nd, besides both after every operation.
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
fn barrel_type() -> u16 {
    OriginalActorType::MT_BARREL.ordinal()
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
/// A live, non-pending handle: a barrel that entered `S_NULL` this tic (state
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
/// A command the admitted domain must execute: slot One, source movement range,
/// buttons the policy admits (`BT_USE` included). Anything else is invalid input.
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
    decorations: Vec<(ActorId, Static)>,
    /// Every actor present after setup with its spawn position, and whether it has
    /// left that position since (the spawn rule S′ admits a box that meets a wall at
    /// spawn; the box clause of Ω holds once the actor has moved, M3h9b decision F3).
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

/// The public equivalent of the crate's `barrels::validate_barrel` witness for every
/// current barrel, and the barrel bound: a role state (`S_BAR1`/`S_BAR2` alive,
/// `S_BEXP..S_BEXP5` dying) or `S_GIBS` after a crunch, with the state's sprite and
/// frame and a countdown inside its tics; health at most 20 and positive exactly
/// while alive; the info flags (plus `MF_AMBUSH`) while alive, `MF_SHOOTABLE`
/// cleared and `MF_CORPSE|MF_DROPOFF` set once dead, `MF_SOLID` kept until a crunch;
/// solidity from `MF_SOLID`; radius 10 and height 42 alive, a dying barrel solid,
/// not shootable and 10.5 units high, a crunched one with radius and height 0;
/// dropoff exactly when dead; reaction time 8 or 0; threshold 0, or 100 exactly
/// with a target; no `movedir`, `movecount` or death request; `lastlook` below 4; a
/// 2035 spawnpoint; a center in the room rectangle. At most `MAX_BARRELS` barrels.
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
            // A dying barrel stays solid and is no longer shootable (p_inter.c:675-681).
            assert_eq!(flags, dead);
            assert_eq!(a.radius(), Fixed::from_map_units(10));
            assert_eq!(a.height(), Fixed::from_bits(42 << 14));
            assert!(a.allows_dropoff());
        } else {
            // The crunch (p_map.c:1270-1276): radius and height 0, not solid.
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
/// V′ and the relaxed item witness for every actor that is not a bullet puff,
/// blood or an exploded fireball, Ω for every actor with a box (its box clause once
/// the actor has left its spawn position, as S′ admits), the decorations'
/// static fields, the movers properties and the barrel witness.
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
}
/// The DSIM37 capture of an admitted state: `decode_barrels` round-trips it, the
/// DSIM36 reader rejects it with `Version(37)`, and a mutated copy never panics.
fn capture(s: &Simulation, flip: [u8; 3]) {
    let limits = DiagnosticLimits::default();
    let mut bytes = s.snapshot().diagnostic_bytes(limits).unwrap();
    let view = Diagnostic::decode_barrels(&bytes, limits).unwrap();
    assert_eq!(view.encode(limits).unwrap(), bytes);
    assert_eq!(
        Diagnostic::decode_e1m1(&bytes, limits).err(),
        Some(DiagnosticError::Version(37))
    );
    let at = usize::from(u16::from_le_bytes([flip[0], flip[1]])) % bytes.len();
    bytes[at] ^= flip[2];
    if let Ok(v) = Diagnostic::decode_barrels(&bytes, limits) {
        assert_eq!(v.encode(limits).unwrap(), bytes);
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
        // A long run: up to 256 tics of one held command, so explosions, chain
        // reactions, falls and the lift run their course.
        4 | 14 => Some(1 + usize::from(b[1])),
        _ => None,
    }
}
/// The command of tic `t` of a tic operation. A single tic is the raw record (any
/// slot, any buttons); a long run holds a valid slot-One command (movement clamped
/// to the source range, `BT_ATTACK`/`BT_USE` only, the turn applied every 32nd tic)
/// and sends the empty command once the player is dead.
fn command(s: &Simulation, b: &[u8; 9], t: usize) -> [TicCommand; 4] {
    let mut commands = [TicCommand::default(); 4];
    if tic_count(b) == Some(1) {
        commands[usize::from(b[1] % 4)] = TicCommand {
            forward: i8::from_le_bytes([b[2]]),
            side: i8::from_le_bytes([b[3]]),
            angle_turn: i16::from_le_bytes([b[4], b[5]]),
            buttons: b[8],
        };
    } else if s
        .original_player(PlayerSlot::One)
        .is_ok_and(|p| p.state == PlayerBodyState::Live)
    {
        commands[0] = TicCommand {
            forward: i8::from_le_bytes([b[2]]).clamp(-50, 50),
            side: i8::from_le_bytes([b[3]]).clamp(-50, 50),
            angle_turn: if t % 32 == 0 {
                i16::from_le_bytes([b[4], b[5]])
            } else {
                0
            },
            buttons: b[8] & (BT_ATTACK | BT_USE),
        };
    }
    commands
}
/// The witnesses a valid tic may be refused with, state preserved (M3h9b note
/// §5.1): the 129th intercept, the corner jump (a player's slide or a barrel's
/// step, `Escape`), `SpecialCross`, the vertical witness, the per-explosion sight
/// budget, the exit and reborn tics, and the stale bomb source (`Invariant`,
/// decision F4(iv)).
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
/// Runs `tics` tics of a tic operation through `advance` or `step`, checking after
/// each one: after an exit or a reborn request every tic is refused with that
/// error; a valid slot-One command to a live player (a dead one admits the empty
/// command and `BT_USE` alone) is admitted or refused only with a declared
/// witness; an admitted tic advances the counter unless paused. With `check`, the
/// invariants every 8th admitted tic and the DSIM37 capture every 32nd. Stops at
/// the first refused tic and returns the admitted count and that tic's result.
fn drive(
    s: &mut Simulation,
    b: &[u8; 9],
    tics: usize,
    step: bool,
    ended: &mut Option<SimError>,
    check: Option<&Room>,
) -> (usize, Result<(), SimError>) {
    for t in 0..tics {
        let commands = command(s, b, t);
        let (tic, paused) = (s.tic(), s.paused());
        let live = s.original_player(PlayerSlot::One).unwrap().state == PlayerBodyState::Live;
        let result = if step {
            s.step(commands).map(|_| ())
        } else {
            s.advance(commands)
        };
        if let Some(error) = *ended {
            assert_eq!(result, Err(error));
        }
        let c = commands[0];
        let slot_one = commands[1..].iter().all(|o| *o == TicCommand::default());
        let dead_command = c == TicCommand::default()
            || (c.buttons == BT_USE && c.forward == 0 && c.side == 0 && c.angle_turn == 0);
        if slot_one && (live || dead_command) && valid_command(&c) {
            assert!(
                result.as_ref().map_or_else(declared_witness, |_| true),
                "valid command refused: {c:?} {result:?}"
            );
        }
        if let Err(e) = result {
            return (t, Err(e));
        }
        assert_eq!(s.tic(), if paused { tic } else { tic + 1 });
        if s.events()
            .iter()
            .any(|e| matches!(e, Event::Mover(MoverEvent::Exit { .. })))
        {
            *ended = Some(SimError::LevelExited);
        }
        // P_DeathThink sees the command only on an unpaused tic, in slot One.
        if s.original_player(PlayerSlot::One).unwrap().state != PlayerBodyState::Live
            && c.buttons & BT_USE != 0
            && !paused
            && !live
        {
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

/// One bounded non-tic operation. The refusals the barrels policy owes a barrel
/// and a decoration are asserted here; the caller asserts that every refused
/// operation keeps the snapshot and the RNG.
fn apply(s: &mut Simulation, cast: &Cast, b: &[u8; 9]) -> Result<(), SimError> {
    let candidate = ActorId::from_parts(u32::from(b[6]), u64::from(b[7]));
    let unsupported = Err(SimError::PlayerBody(PlayerBodyError::Unsupported));
    let push = [
        Fixed::from_map_units(i16::from_le_bytes([b[3], b[4]])),
        Fixed::from_map_units(i16::from_le_bytes([b[5], b[8]])),
    ];
    let out_of_bound = push.iter().any(|v| v.bits().unsigned_abs() > 64 << 16);
    let momz = Fixed::from_bits(i32::from_le_bytes([b[3], b[4], b[5], b[8]]));
    let momz_out = momz.bits().unsigned_abs() > 64 << 16;
    match b[0] % 16 {
        // `damage_actor` aimed at the body, a monster, a decoration, a pickup, a
        // barrel or a candidate handle, with no source or inflictor, a monster, a
        // barrel or the body (never an API inflictor).
        5 => {
            let target = match b[1] % 6 {
                0 => cast.body,
                1 => cast.monster(b[2]),
                2 => cast.decoration(b[2]).unwrap_or(candidate),
                3 => cast.pickup(b[2]).unwrap_or(candidate),
                4 => cast.barrel(b[2]).unwrap_or(candidate),
                _ => candidate,
            };
            let who = |k: u8| match k & 3 {
                0 => None,
                1 => Some(cast.monster(b[4])),
                2 => Some(cast.barrel(b[4]).unwrap_or(candidate)),
                _ => Some(cast.body),
            };
            s.damage_actor(DamageRequest {
                target,
                source: who(b[3]),
                inflictor: who(b[3] >> 2),
                amount: i32::from(i16::from_le_bytes([b[5], b[8]])),
            })
            .map(|_| ())
        }
        // Decision F4(v): a target, a removal, a state entry and a death request on
        // a barrel are refused, with every byte kept.
        6 => {
            let Some(barrel) = cast.barrel(b[2]) else {
                return s.mark_remove(candidate);
            };
            let live = current(s, barrel);
            let result = match b[1] % 4 {
                0 => {
                    let target = if b[3] & 1 == 0 {
                        Some(cast.body)
                    } else {
                        Some(cast.monster(b[4]))
                    };
                    let r = s.set_actor_target(barrel, target);
                    if live {
                        assert_eq!(r, unsupported);
                    }
                    r
                }
                1 => {
                    let r = s.mark_remove(barrel);
                    if live {
                        assert_eq!(r, unsupported);
                    }
                    r
                }
                2 => {
                    let state = [
                        OriginalState::S_BAR1,
                        OriginalState::S_BEXP,
                        OriginalState::S_BEXP4,
                        OriginalState::S_GIBS,
                    ][usize::from(b[3] % 4)];
                    let r = s.enter_actor_state(barrel, StateId(state.ordinal()));
                    if live {
                        assert_eq!(r, unsupported);
                    }
                    r
                }
                _ => s.request_actor_death(barrel),
            };
            assert!(result.is_err(), "barrel operation admitted");
            result
        }
        7 => {
            s.set_paused(b[1] & 1 != 0);
            Ok(())
        }
        // A push on a barrel: admitted within the shared 64-unit input bound (a
        // thrust barrel is ordinary play), that bound's error past it.
        8 => match cast.barrel(b[2]) {
            Some(barrel) => {
                let live = current(s, barrel);
                let result = s.set_momentum(barrel, push);
                if out_of_bound {
                    assert_eq!(result, Err(SimError::Geometry(GeometryError::InvalidActor)));
                } else if live {
                    assert_eq!(result, Ok(()));
                }
                result
            }
            None => s.set_momentum(candidate, push),
        },
        // Vertical momentum on a barrel (admitted within 64 units, the bound's
        // error past it), the body, a monster, a decoration (refused unless zero)
        // or a candidate handle.
        9 => match b[1] % 5 {
            0 => match cast.barrel(b[2]) {
                Some(barrel) => {
                    let live = current(s, barrel);
                    let result = s.set_vertical_momentum(barrel, momz);
                    if momz_out {
                        assert_eq!(
                            result,
                            Err(SimError::Perception(PerceptionError::Arithmetic))
                        );
                    } else if live {
                        assert_eq!(result, Ok(()));
                    }
                    result
                }
                None => s.set_vertical_momentum(candidate, momz),
            },
            1 => s.set_vertical_momentum(cast.body, momz),
            2 => s.set_vertical_momentum(cast.monster(b[2]), momz),
            3 => match cast.decoration(b[2]) {
                Some(d) => {
                    let result = s.set_vertical_momentum(d, momz);
                    if momz_out {
                        assert_eq!(
                            result,
                            Err(SimError::Perception(PerceptionError::Arithmetic))
                        );
                    } else if momz != Fixed::ZERO {
                        assert_eq!(result, unsupported);
                    } else {
                        assert_eq!(result, Ok(()));
                    }
                    result
                }
                None => s.set_vertical_momentum(candidate, momz),
            },
            _ => s.set_vertical_momentum(candidate, momz),
        },
        // A direct spawn of a barrel (2035) or a decoration type: always refused
        // (`DirectSpawn`), both exist only as THINGs.
        10 => {
            let ty = if b[1] % 8 == 7 {
                OriginalActorType::MT_BARREL
            } else {
                E1M1_DECORATIONS[usize::from(b[1] % 8)]
            };
            let position = [
                Fixed::from_map_units(i16::from_le_bytes([b[2], b[3]])),
                Fixed::from_map_units(i16::from_le_bytes([b[4], b[5]])),
            ];
            let height = if b[8] & 1 == 0 {
                SpawnHeight::Floor
            } else {
                SpawnHeight::Ceiling
            };
            let result = s
                .spawn_type(ActorTypeId(ty.ordinal()), position, height)
                .map(|_| ());
            assert_eq!(result, Err(SimError::MapSpawn(MapSpawnError::DirectSpawn)));
            result
        }
        // Retarget: a monster at the body, a monster, a decoration, a barrel or a
        // candidate; a decoration (refused) at anything.
        11 => {
            let target = match b[2] % 5 {
                0 => Some(cast.body),
                1 => Some(cast.monster(b[3])),
                2 => cast.decoration(b[3]).or(Some(candidate)),
                3 => cast.barrel(b[3]).or(Some(candidate)),
                _ => Some(candidate),
            };
            match cast.decoration(b[1]).filter(|_| b[1] & 0x80 != 0) {
                Some(d) => {
                    let result = s.set_actor_target(d, target);
                    assert_eq!(result, unsupported);
                    result
                }
                None => s.set_actor_target(cast.monster(b[1]), target),
            }
        }
        // Removal: a candidate, a monster (whose wounded barrel then holds a stale
        // bomb source), or a decoration (refused).
        12 => match b[1] % 3 {
            0 => s.mark_remove(candidate),
            1 => s.mark_remove(cast.monster(b[2])),
            _ => match cast.decoration(b[2]) {
                Some(d) => {
                    let result = s.mark_remove(d);
                    assert_eq!(result, unsupported);
                    result
                }
                None => s.mark_remove(candidate),
            },
        },
        // The controlled push: the body, a monster, a pickup or a decoration
        // (refused unless zero, after the shared bound), or a candidate handle.
        _ => match b[1] % 5 {
            0 => s.set_momentum(cast.body, push),
            1 => s.set_momentum(cast.monster(b[2]), push),
            2 => s.set_momentum(cast.pickup(b[2]).unwrap_or(candidate), push),
            3 => match cast.decoration(b[2]) {
                Some(d) => {
                    let result = s.set_momentum(d, push);
                    if out_of_bound {
                        assert_eq!(result, Err(SimError::Geometry(GeometryError::InvalidActor)));
                    } else if push != [Fixed::ZERO; 2] {
                        assert_eq!(result, unsupported);
                    }
                    result
                }
                None => s.set_momentum(candidate, push),
            },
            _ => s.set_momentum(candidate, push),
        },
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    // Independent DSIM37 fixtures are read-only decoder inputs, never executable
    // initial states or Rust-produced expectations.
    for original in [
        include_bytes!("../../tests/reference/m3h9b_state_1_85.bin").as_slice(),
        include_bytes!("../../tests/reference/m3h9b_state_18_210.bin").as_slice(),
    ] {
        let mut bytes = original.to_vec();
        for c in data.as_chunks::<3>().0.iter().take(32) {
            let at = usize::from(u16::from_le_bytes([c[0], c[1]])) % bytes.len();
            bytes[at] ^= c[2];
        }
        if let Ok(v) = Diagnostic::decode_barrels(&bytes, DiagnosticLimits::default()) {
            assert_eq!(v.encode(DiagnosticLimits::default()).unwrap(), bytes);
        }
    }
    let c = &cases()[usize::from(data[0]) % 26];
    let map = source::case_map(c.map);
    let boundary = Boundary::derive(&map).unwrap();
    let room = RoomBounds::derive_real_map(&map).unwrap();
    let bounds = e1m1_mover_bounds(&map).unwrap();
    let (mut s, body, things) = if data[1] == 0 {
        source::setup(c).unwrap()
    } else {
        let mut s = Simulation::original_player_barrels_map(map, source::options(data[1])).unwrap();
        let things = s.spawn_map_things().unwrap().spawned;
        let body = s
            .spawn_player_body(PlayerSlot::One, c.position.map(Fixed::from_map_units))
            .unwrap();
        s.initialize_player_weapons(
            PlayerSlot::One,
            WeaponInventory {
                ready: Weapon::Chaingun,
                owned: 15,
                ammo: [200, 50, 0, 0],
            },
        )
        .unwrap();
        s.spawn_level_specials().unwrap();
        (s, body, things)
    };
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
    for (index, b) in data[2..]
        .as_chunks::<9>()
        .0
        .iter()
        .take(RECORDS)
        .enumerate()
    {
        let (before, rng) = (s.snapshot(), s.rng());
        if let Some(asked) = tic_count(b) {
            let tics = asked.min(budget);
            budget -= tics;
            let start = s.clone();
            let ended_before = ended;
            let (admitted, result) = drive(&mut s, b, tics, false, &mut ended, Some(&r));
            // `step` equals `advance` on a fresh clone, every fourth operation.
            if index % 4 == 0 {
                let mut fresh = start.clone();
                let mut e = ended_before;
                assert_eq!(
                    drive(&mut fresh, b, tics, true, &mut e, None),
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
                    drive(&mut replay, b, admitted, false, &mut e, None),
                    (admitted, Ok(()))
                );
                assert_eq!(s.snapshot(), replay.snapshot());
                assert_eq!(s.rng(), replay.rng());
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
