#![no_main]
use doom_sim::{identity::OriginalActorType, *};
use doom_types::{DoomRng, Fixed};
use libfuzzer_sys::fuzz_target;
// Committed synthetic M3h9 rooms (`tests/reference/m3h9_maps.rs`): 0-2 `scrollhall`
// (two non-blocking two-sided 48 lines, a DR door; empty, a Zombieman on the ledge,
// an Imp in the strip), 3 `scrollblock` (a blocking two-sided 48), 4 `dense8` (eight
// two-sided 48 lines in one window), 5-9 `gallery` (the ten decorations alone, with a
// Zombieman beside the pillar or in the alcove, with an Imp before the candelabra or
// the pool of blood), 10-11 `liftroom` (a lamp and a dead player on the lift, a pillar
// under the door), 12 `census` (forty records, every skill bit, bit 16, ambush), 13
// `scrollblock` with an Imp whose fireball crosses the blocking 48 and 14 `doorstep`
// (a pillar straddling a door and a higher step).
const MAPS: [&[u8]; 15] = [
    include_bytes!("../../tests/reference/m3h9_geometry_0.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_1.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_2.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_3.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_4.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_5.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_6.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_7.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_8.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_9.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_10.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_11.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_12.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_13.bin"),
    include_bytes!("../../tests/reference/m3h9_geometry_14.bin"),
];
/// Map and player spawn of each of the 20 replayed medium scenarios
/// (`m3h9_cases.csv`, skill 2; every start is distinct).
const SPAWNS: [(usize, [i16; 2]); 20] = [
    (0, [-40, 0]),
    (1, [-200, 72]),
    (0, [-8, 0]),
    (0, [-64, 0]),
    (2, [-300, -160]),
    (3, [48, 64]),
    (4, [200, 64]),
    (5, [-128, 128]),
    (5, [-256, 300]),
    (5, [-220, -300]),
    (6, [200, -64]),
    (7, [200, -64]),
    (8, [-256, 0]),
    (9, [-256, -192]),
    (5, [-256, 128]),
    (10, [256, 64]),
    (11, [470, 64]),
    (12, [-448, -340]),
    (13, [-200, 0]),
    (14, [470, 64]),
];
const MF_SPECIAL: u32 = 0x1;
const MF_SHOOTABLE: u32 = 0x4;
const MF_MISSILE: u32 = 0x10000;
/// V′ admits a corpse down to `R_min − 56` (the tallest admitted type).
const V_PRIME_MAX_HEIGHT_UNITS: i32 = 56;
/// A command the admitted domain must execute: slot One, source movement range,
/// buttons the movers policy admits (`BT_USE` included). Anything else is an
/// invalid input.
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
/// The vertical invariant V′ (M3h8 note §4.1), evaluated here from the public
/// bounds of the map: one of the cases (a)-(e) with the actor's own floor, ceiling
/// and height, tried in order; `None` when the actor is in none.
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
/// V′ and the relaxed item witness (M3h8 note §4.2: an item rests on its own
/// floor) for every actor that is not a bullet puff, blood or an exploded fireball,
/// and the XY invariant Ω (M3h5) for every mover with a box: its center inside the
/// one-sided boundary with its open box meeting no wall; every live actor's center
/// is in the vertex rectangle.
fn invariants(s: &Simulation, boundary: &Boundary, room: RoomBounds, bounds: MoverBounds) {
    for id in s.actor_order() {
        let a = s.actor(id).unwrap();
        let Some(l) = a.life() else { continue };
        let blast = l.actor_type.0 == OriginalActorType::MT_TROOPSHOT.ordinal()
            && l.flags & MF_MISSILE == 0;
        if matches!(l.actor_type.0, 37..=38) || blast {
            continue;
        }
        assert!(
            v_prime_case(a, bounds).is_some(),
            "type {} z {} floor {} ceiling {} height {} health {} outside V'",
            l.actor_type.0,
            a.z().bits(),
            a.floor().bits(),
            a.ceiling().bits(),
            a.height().bits(),
            l.health
        );
        assert!(
            room.contains(a.position()),
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
            boundary.inside(a.position()),
            "type {} at {:?} outside the boundary",
            l.actor_type.0,
            a.position().map(Fixed::bits)
        );
        assert!(
            !boundary.box_meets_wall(a.position(), a.radius() - Fixed::from_bits(1)),
            "type {} at {:?} meets a wall",
            l.actor_type.0,
            a.position().map(Fixed::bits)
        );
    }
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
fn is_decoration(s: &Simulation, id: ActorId) -> bool {
    s.actor(id).and_then(Actor::life).is_some_and(|l| {
        E1M1_DECORATIONS
            .iter()
            .any(|d| d.ordinal() == l.actor_type.0)
    })
}
/// Every decoration is live with its spawn type, state, tics, health, flags, radius
/// and height, rests on its floor (`z == floorz`, riding a mover through
/// `P_ThingHeightClip`), and no decoration appeared or vanished.
fn decoration_invariants(s: &Simulation, decorations: &[(ActorId, Static)]) {
    for (id, fixed) in decorations {
        assert!(
            s.actor_order().any(|a| a == *id)
                && !s.pending_events().contains(&Event::RemovalMarked(*id)),
            "decoration removed"
        );
        assert_eq!(decoration_state(s, *id).as_ref(), Some(fixed));
        let a = s.actor(*id).unwrap();
        assert_eq!(a.z(), a.floor(), "decoration off its floor");
    }
    assert_eq!(
        s.actor_order().filter(|id| is_decoration(s, *id)).count(),
        decorations.len()
    );
}
/// One bounded operation. `decoration` is a spawned decoration (or the body when the
/// room has none); the refusals the E1M1 policy owes a decoration are asserted
/// here: a target, a removal, a nonzero push and nonzero vertical momentum are
/// `PlayerBodyError::Unsupported`, and a direct spawn of a decoration type is
/// `MapSpawnError::DirectSpawn`.
fn apply(
    s: &mut Simulation,
    body: ActorId,
    monsters: &[ActorId],
    decorations: &[ActorId],
    b: &[u8],
    capture: bool,
) -> Result<(), SimError> {
    let candidate = ActorId::from_parts(u32::from(b[6]), u64::from(b[7]));
    let monster = |i: u8| monsters[usize::from(i) % monsters.len()];
    let decoration = |i: u8| {
        decorations
            .get(usize::from(i) % decorations.len().max(1))
            .copied()
    };
    let unsupported = Err(SimError::PlayerBody(PlayerBodyError::Unsupported));
    match b[0] % 11 {
        0..=3 => {
            let mut commands = [TicCommand::default(); 4];
            commands[usize::from(b[1] % 4)] = TicCommand {
                forward: i8::from_le_bytes([b[2]]),
                side: i8::from_le_bytes([b[3]]),
                angle_turn: i16::from_le_bytes([b[4], b[5]]),
                buttons: b[8],
            };
            if capture {
                s.step(commands).map(|_| ())
            } else {
                s.advance(commands)
            }
        }
        4 => s
            .damage_actor(DamageRequest {
                target: match b[1] % 4 {
                    0 => body,
                    1 => monster(b[2]),
                    2 => decoration(b[2]).unwrap_or(candidate),
                    _ => candidate,
                },
                source: (b[3] & 1 == 0).then(|| monster(b[4])),
                inflictor: (b[3] & 2 == 0).then(|| monster(b[4])),
                amount: i32::from(i16::from_le_bytes([b[5], b[8]])),
            })
            .map(|_| ()),
        5 => {
            let target = match b[2] % 4 {
                0 => Some(body),
                1 => Some(monster(b[3])),
                2 => decoration(b[3]).or(Some(candidate)),
                _ => Some(candidate),
            };
            match decoration(b[1]).filter(|_| b[1] & 0x80 != 0) {
                Some(d) => {
                    let result = s.set_actor_target(d, target);
                    assert_eq!(result, unsupported);
                    result
                }
                None => s.set_actor_target(monster(b[1]), target),
            }
        }
        6 => match b[1] % 3 {
            0 => s.mark_remove(candidate),
            1 => s.mark_remove(monster(b[2])),
            _ => match decoration(b[2]) {
                Some(d) => {
                    let result = s.mark_remove(d);
                    assert_eq!(result, unsupported);
                    result
                }
                None => s.mark_remove(candidate),
            },
        },
        7 => {
            s.set_paused(b[1] & 1 != 0);
            Ok(())
        }
        // Vertical momentum: the player body, a monster or pickup (a pickup is
        // refused unless it is zero), a candidate handle or a decoration (refused
        // unless it is zero, after the shared 64-unit bound).
        9 => {
            let momz = Fixed::from_bits(i32::from_le_bytes([b[3], b[4], b[5], b[8]]));
            match b[1] % 4 {
                0 => s.set_vertical_momentum(body, momz),
                1 => s.set_vertical_momentum(monster(b[2]), momz),
                2 => match decoration(b[2]) {
                    Some(d) => {
                        let result = s.set_vertical_momentum(d, momz);
                        if momz.bits().unsigned_abs() > 64 << 16 {
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
            }
        }
        // A direct spawn of one of the seven decoration types: always refused, the
        // first check of `spawn_type` (a decoration exists only as a THING).
        10 => {
            let ty = ActorTypeId(E1M1_DECORATIONS[usize::from(b[1]) % 7].ordinal());
            let position = [
                Fixed::from_map_units(i16::from_le_bytes([b[2], b[3]])),
                Fixed::from_map_units(i16::from_le_bytes([b[4], b[5]])),
            ];
            let height = if b[8] & 1 == 0 {
                SpawnHeight::Floor
            } else {
                SpawnHeight::Ceiling
            };
            let result = s.spawn_type(ty, position, height).map(|_| ());
            assert_eq!(result, Err(SimError::MapSpawn(MapSpawnError::DirectSpawn)));
            result
        }
        // The controlled push: the player body, a monster or pickup, a candidate
        // handle, or a decoration (refused unless the push is zero).
        _ => {
            let push = [
                Fixed::from_map_units(i16::from_le_bytes([b[3], b[4]])),
                Fixed::from_map_units(i16::from_le_bytes([b[5], b[8]])),
            ];
            match b[1] % 4 {
                0 => s.set_momentum(body, push),
                1 => s.set_momentum(monster(b[2]), push),
                2 => match decoration(b[2]) {
                    Some(d) => {
                        let result = s.set_momentum(d, push);
                        // The shared 64-unit input bound is checked before any role
                        // (`map_scale::bounded`, the same precedence as an item push),
                        // so an out-of-bound push to a decoration is that bound's
                        // `Geometry(InvalidActor)`; an in-bound nonzero push is the
                        // decoration refusal. Both keep every byte (asserted by the
                        // caller's transaction check).
                        if push.iter().any(|v| v.bits().unsigned_abs() > 64 << 16) {
                            assert_eq!(
                                result,
                                Err(SimError::Geometry(GeometryError::InvalidActor))
                            );
                        } else if push != [Fixed::ZERO; 2] {
                            assert_eq!(result, unsupported);
                        }
                        result
                    }
                    None => s.set_momentum(candidate, push),
                },
                _ => s.set_momentum(candidate, push),
            }
        }
    }
}
/// The movers properties at a tic boundary: every sector height within the
/// admitted reach; at most one mover per sector (`specialdata`) and at most 30 live
/// plats; the shared order lists every live thinker once; `secretcount <=
/// totalsecret`.
fn movers_invariants(s: &Simulation, bounds: MoverBounds) {
    let units = |u: i32| Fixed::from_bits(u * 65536);
    for sector in s.sectors() {
        for h in [sector.floor, sector.ceiling] {
            assert!(units(bounds.reach[0]) <= h && h <= units(bounds.reach[1]));
        }
    }
    let mut sectors = Vec::new();
    let mut plats = 0;
    for entry in s.shared_thinker_order() {
        if let ThinkerId::Sector(id) = entry {
            let t = s.sector_thinker(id).unwrap();
            if matches!(
                t,
                SectorThinker::Door(_) | SectorThinker::Plat(_) | SectorThinker::Floor(_)
            ) {
                sectors.push(t.sector());
                plats += usize::from(matches!(t, SectorThinker::Plat(_)));
            }
        }
    }
    assert!(sectors.len() <= bounds.movers * 2 && plats <= 30);
    let secrets = s.original_player_secret_count().unwrap();
    let total = s.level_statistics().unwrap().total_secrets;
    assert!(0 <= secrets && secrets <= total);
}
fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    // Independent DSIM36 fixtures are read-only decoder inputs, never executable
    // initial states or Rust-produced expectations.
    for original in [
        include_bytes!("../../tests/reference/m3h9_state_7_20.bin").as_slice(),
        include_bytes!("../../tests/reference/m3h9_state_15_48.bin").as_slice(),
    ] {
        let mut bytes = original.to_vec();
        for c in data.as_chunks::<3>().0.iter().take(32) {
            let at = usize::from(u16::from_le_bytes([c[0], c[1]])) % bytes.len();
            bytes[at] ^= c[2];
        }
        if let Ok(v) = Diagnostic::decode_e1m1(&bytes, DiagnosticLimits::default()) {
            assert_eq!(v.encode(DiagnosticLimits::default()).unwrap(), bytes);
        }
    }
    let (which, spawn) = SPAWNS[usize::from(data[0]) % SPAWNS.len()];
    let wad = doom_wad::Wad::parse(MAPS[which], doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let boundary = Boundary::derive(&map).unwrap();
    let room = RoomBounds::derive_real_map(&map).unwrap();
    ArithmeticCertificate::derive(&map).unwrap();
    assert!(slide_window_lines(&map) <= MAX_INTERCEPTS);
    // Rule 6 holds, so the SpecialCross witness can never refuse a tic here.
    assert!(special_window_lines(&map) <= 8);
    let bounds = e1m1_mover_bounds(&map).unwrap();
    // The movers policy's rules refuse exactly the rooms with a two-sided 48.
    let two_sided_48 = map
        .linedefs()
        .iter()
        .any(|l| l.special == 48 && l.sides[1].is_some());
    assert_eq!(mover_bounds(&map).is_err(), two_sided_48);
    let mut s = Simulation::original_player_e1m1_map(
        map,
        MapContextOptions {
            initial_rng: DoomRng::from_indices(data[1], 0),
            respawn_monsters: false,
        },
    )
    .unwrap();
    let spawned = s.spawn_map_things().unwrap().spawned;
    let decorations: Vec<_> = spawned
        .iter()
        .copied()
        .filter(|id| is_decoration(&s, *id))
        .collect();
    assert!(decorations.len() <= MAX_DECORATIONS);
    let mut monsters: Vec<_> = spawned
        .iter()
        .copied()
        .filter(|id| !is_decoration(&s, *id))
        .collect();
    let body = s
        .spawn_player_body(PlayerSlot::One, spawn.map(Fixed::from_map_units))
        .unwrap();
    if monsters.is_empty() {
        monsters.push(body);
    }
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
    let fixed: Vec<_> = decorations
        .iter()
        .map(|id| (*id, decoration_state(&s, *id).unwrap()))
        .collect();
    invariants(&s, &boundary, room, bounds);
    movers_invariants(&s, bounds);
    decoration_invariants(&s, &fixed);
    let mut captured = s.clone();
    let mut ended = None;
    for b in data[2..data.len().min(290)].as_chunks::<9>().0 {
        let before = s.snapshot();
        let (tic, paused) = (s.tic(), s.paused());
        let live = s.original_player(PlayerSlot::One).unwrap().state == PlayerBodyState::Live;
        let result = apply(&mut s, body, &monsters, &decorations, b, false);
        assert_eq!(
            result,
            apply(&mut captured, body, &monsters, &decorations, b, true)
        );
        // The transaction: a refused operation keeps every byte.
        if result.is_err() {
            assert_eq!(s.snapshot(), before);
        }
        if b[0] % 11 <= 3 {
            let command = TicCommand {
                forward: i8::from_le_bytes([b[2]]),
                side: i8::from_le_bytes([b[3]]),
                angle_turn: i16::from_le_bytes([b[4], b[5]]),
                buttons: b[8],
            };
            // After an exit or a reborn request every tic is refused with that error.
            if let Some(error) = ended {
                assert_eq!(result, Err(error));
            }
            // Progress: a valid slot-One command to a live player (a dead one admits
            // the empty command and `BT_USE` alone) is never rejected except by the
            // refused legal source behaviors (the corner jump, the 129th intercept)
            // and the exit and reborn refusals; every accepted tic advances the
            // counter by one unless the world is paused.
            let dead_command = command == TicCommand::default()
                || (command.buttons == BT_USE
                    && command.forward == 0
                    && command.side == 0
                    && command.angle_turn == 0);
            if b[1] % 4 == 0 && (live || dead_command) && valid_command(&command) {
                assert!(
                    matches!(
                        result,
                        Ok(())
                            | Err(SimError::MapScale(
                                MapScaleError::Escape | MapScaleError::Intercepts
                            ))
                            | Err(SimError::LevelExited | SimError::PlayerReborn)
                    ),
                    "valid command rejected: {command:?} {result:?}"
                );
            }
            if result.is_ok() {
                assert_eq!(s.tic(), if paused { tic } else { tic + 1 });
                if s.events()
                    .iter()
                    .any(|e| matches!(e, Event::Mover(MoverEvent::Exit { .. })))
                {
                    ended = Some(SimError::LevelExited);
                }
                // P_DeathThink sees the command only on an unpaused tic (P_Ticker
                // returns first while paused) and only in slot One, the body's slot.
                if s.original_player(PlayerSlot::One).unwrap().state != PlayerBodyState::Live
                    && command.buttons & BT_USE != 0
                    && b[1] % 4 == 0
                    && !paused
                    && !live
                {
                    ended = Some(SimError::PlayerReborn);
                }
            }
        }
        invariants(&s, &boundary, room, bounds);
        movers_invariants(&s, bounds);
        decoration_invariants(&s, &fixed);
        let snap = s.snapshot();
        assert_eq!(snap, captured.snapshot());
        let mut bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
        let view = Diagnostic::decode_e1m1(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
        assert!(Diagnostic::decode_movers(&bytes, DiagnosticLimits::default()).is_err());
        let at = usize::from(u16::from_le_bytes([b[6], b[7]])) % bytes.len();
        bytes[at] ^= b[8];
        let _ = Diagnostic::decode_e1m1(&bytes, DiagnosticLimits::default());
    }
});
