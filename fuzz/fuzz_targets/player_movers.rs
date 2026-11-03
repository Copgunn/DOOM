#![no_main]
use doom_sim::{identity::OriginalActorType, *};
use doom_types::{DoomRng, Fixed};
use libfuzzer_sys::fuzz_target;
// Committed synthetic M3h8 rooms (`tests/reference/m3h8_maps.rs`): 0-3 `doorway` (DR
// doors with a monster behind each, a zombieman and a bonus in the door, a monster
// overlapping the closed door), 4-5 `hall`, 6 `switchyard` (SR door switch, S1 exit
// switch, a plain wall), 7-12 `lift` (WR lift, three ceilings, monster riders), 13-16
// `floors` (W1 turbo and lowest floors, W1 blazeOpen door) and 17-18 `dense` (eight
// WR lift lines in one window, an imp).
const MAPS: [&[u8]; 19] = [
    include_bytes!("../../tests/reference/m3h8_geometry_0.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_1.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_2.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_3.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_4.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_5.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_6.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_7.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_8.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_9.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_10.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_11.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_12.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_13.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_14.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_15.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_16.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_17.bin"),
    include_bytes!("../../tests/reference/m3h8_geometry_18.bin"),
];
/// Map and player spawn of each distinct committed scenario start (`m3h8_cases.csv`).
const SPAWNS: [(usize, [i16; 2]); 27] = [
    (0, [-64, 0]),
    (1, [-200, 192]),
    (2, [-48, -192]),
    (3, [100, 0]),
    (4, [-40, 0]),
    (5, [-200, 0]),
    (6, [64, 330]),
    (6, [40, 320]),
    (6, [40, 330]),
    (6, [450, 64]),
    (6, [450, 192]),
    (6, [256, -32]),
    (6, [256, 192]),
    (7, [256, 64]),
    (8, [256, 64]),
    (9, [256, 64]),
    (10, [256, 200]),
    (11, [256, 64]),
    (12, [256, 64]),
    (13, [200, 64]),
    (14, [450, 64]),
    (15, [720, 64]),
    (16, [850, 64]),
    (16, [880, 100]),
    (16, [220, 64]),
    (17, [200, 64]),
    (18, [100, 64]),
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
fn apply(
    s: &mut Simulation,
    body: ActorId,
    monsters: &[ActorId],
    b: &[u8],
    capture: bool,
) -> Result<(), SimError> {
    let candidate = ActorId::from_parts(u32::from(b[6]), u64::from(b[7]));
    let monster = |i: u8| monsters[usize::from(i) % monsters.len()];
    match b[0] % 9 {
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
                target: match b[1] % 3 {
                    0 => body,
                    1 => monster(b[2]),
                    _ => candidate,
                },
                source: (b[3] & 1 == 0).then(|| monster(b[4])),
                inflictor: (b[3] & 2 == 0).then(|| monster(b[4])),
                amount: i32::from(i16::from_le_bytes([b[5], b[8]])),
            })
            .map(|_| ()),
        5 => s.set_actor_target(
            monster(b[1]),
            match b[2] % 3 {
                0 => Some(body),
                1 => Some(monster(b[3])),
                _ => Some(candidate),
            },
        ),
        6 => s.mark_remove(if b[1] & 1 == 0 {
            candidate
        } else {
            monster(b[2])
        }),
        7 => {
            s.set_paused(b[1] & 1 != 0);
            Ok(())
        }
        // The controlled push of the diagonal policies: the player body (or a
        // monster, or a candidate handle) gets an explicit momentum in map units.
        _ => s.set_momentum(
            match b[1] % 3 {
                0 => body,
                1 => monster(b[2]),
                _ => candidate,
            },
            [
                Fixed::from_map_units(i16::from_le_bytes([b[3], b[4]])),
                Fixed::from_map_units(i16::from_le_bytes([b[5], b[8]])),
            ],
        ),
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
    // Independent DSIM35 fixtures are read-only decoder inputs, never executable
    // initial states or Rust-produced expectations.
    for original in [
        include_bytes!("../../tests/reference/m3h8_state_0_82.bin").as_slice(),
        include_bytes!("../../tests/reference/m3h8_state_8_300.bin").as_slice(),
    ] {
        let mut bytes = original.to_vec();
        for c in data.as_chunks::<3>().0.iter().take(32) {
            let at = usize::from(u16::from_le_bytes([c[0], c[1]])) % bytes.len();
            bytes[at] ^= c[2];
        }
        if let Ok(v) = Diagnostic::decode_movers(&bytes, DiagnosticLimits::default()) {
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
    let bounds = mover_bounds(&map).unwrap();
    let mut s = Simulation::original_player_movers_map(
        map,
        MapContextOptions {
            initial_rng: DoomRng::from_indices(data[1], 0),
            respawn_monsters: false,
        },
    )
    .unwrap();
    let mut monsters = s.spawn_map_things().unwrap().spawned;
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
    invariants(&s, &boundary, room, bounds);
    movers_invariants(&s, bounds);
    let mut captured = s.clone();
    let mut ended = None;
    for b in data[2..data.len().min(290)].as_chunks::<9>().0 {
        let before = s.snapshot();
        let (tic, paused) = (s.tic(), s.paused());
        let live = s.original_player(PlayerSlot::One).unwrap().state == PlayerBodyState::Live;
        let result = apply(&mut s, body, &monsters, b, false);
        assert_eq!(result, apply(&mut captured, body, &monsters, b, true));
        // The transaction: a refused operation keeps every byte.
        if result.is_err() {
            assert_eq!(s.snapshot(), before);
        }
        if b[0] % 9 <= 3 {
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
                if s.original_player(PlayerSlot::One).unwrap().state != PlayerBodyState::Live
                    && command.buttons & BT_USE != 0
                    && !live
                {
                    ended = Some(SimError::PlayerReborn);
                }
            }
        }
        invariants(&s, &boundary, room, bounds);
        movers_invariants(&s, bounds);
        let snap = s.snapshot();
        assert_eq!(snap, captured.snapshot());
        let mut bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
        let view = Diagnostic::decode_movers(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
        let at = usize::from(u16::from_le_bytes([b[6], b[7]])) % bytes.len();
        bytes[at] ^= b[8];
        let _ = Diagnostic::decode_movers(&bytes, DiagnosticLimits::default());
    }
});
