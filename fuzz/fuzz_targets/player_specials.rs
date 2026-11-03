#![no_main]
use doom_sim::*;
use doom_types::{DoomRng, Fixed};
use libfuzzer_sys::fuzz_target;
// Committed synthetic M3h7 rooms: `pools` (maps 0-2: twelve basins, one per admitted
// sector special, eight light thinkers, three one-sided scrollers; map 2 with five
// imps, a zombie and a shotgunner) and `sky` (maps 3-10: sky courts, hack blocks,
// a sky tower and windows under sky sills, imps firing at the sky walls).
const MAPS: [&[u8]; 11] = [
    include_bytes!("../../tests/reference/m3h7_geometry_0.bin"),
    include_bytes!("../../tests/reference/m3h7_geometry_1.bin"),
    include_bytes!("../../tests/reference/m3h7_geometry_2.bin"),
    include_bytes!("../../tests/reference/m3h7_geometry_3.bin"),
    include_bytes!("../../tests/reference/m3h7_geometry_4.bin"),
    include_bytes!("../../tests/reference/m3h7_geometry_5.bin"),
    include_bytes!("../../tests/reference/m3h7_geometry_6.bin"),
    include_bytes!("../../tests/reference/m3h7_geometry_7.bin"),
    include_bytes!("../../tests/reference/m3h7_geometry_8.bin"),
    include_bytes!("../../tests/reference/m3h7_geometry_9.bin"),
    include_bytes!("../../tests/reference/m3h7_geometry_10.bin"),
];
/// Map and player spawn of each of the 17 committed scenarios (`m3h7_cases.csv`):
/// the hall, inside the basins (5%, 20%, strobe-hurt), at the ledge, below the
/// secret, under the sky, on the tower and before the windows.
const SPAWNS: [(usize, [i16; 2]); 17] = [
    (0, [768, -200]),
    (1, [40, 64]),
    (0, [320, 64]),
    (0, [64, -12]),
    (0, [576, -40]),
    (0, [448, 64]),
    (2, [768, -60]),
    (3, [80, 704]),
    (4, [-400, 960]),
    (5, [697, 860]),
    (6, [432, -668]),
    (7, [432, -668]),
    (7, [-64, 384]),
    (7, [-200, 32]),
    (8, [264, -96]),
    (9, [264, 32]),
    (10, [264, 160]),
];
/// A command the admitted domain must execute: slot One, source movement range,
/// buttons the four-weapon policy admits. Anything else is an invalid input.
fn valid_command(c: &TicCommand) -> bool {
    let weapon = (c.buttons & BT_WEAPONMASK) >> 3;
    (-50..=50).contains(&c.forward)
        && (-50..=50).contains(&c.side)
        && c.buttons & !(BT_ATTACK | BT_CHANGE | BT_WEAPONMASK) == 0
        && if c.buttons & BT_CHANGE != 0 {
            weapon <= 3
        } else {
            weapon == 0
        }
}
/// Invariants V (M3h4) and Ω (M3h5) at a tic boundary, unchanged by the
/// specials policy (no height changes in Stage A): every actor that is not a bullet puff or blood sits within
/// `[floor, ceiling - height]`; every XY mover (not an item) has its center inside
/// the one-sided boundary with its open box meeting no wall, and every live
/// actor's center is in the vertex rectangle.
fn invariants(s: &Simulation, boundary: &Boundary, room: RoomBounds) {
    for id in s.actor_order() {
        let a = s.actor(id).unwrap();
        let Some(l) = a.life() else { continue };
        if matches!(l.actor_type.0, 37..=38) {
            continue;
        }
        let z = a.z();
        assert!(
            a.floor() <= z && z + a.height() <= a.ceiling(),
            "type {} z {} floor {} ceiling {}",
            l.actor_type.0,
            z.bits(),
            a.floor().bits(),
            a.ceiling().bits()
        );
        assert!(
            room.contains(a.position()),
            "type {} outside the rectangle",
            l.actor_type.0
        );
        if l.flags & 1 != 0 {
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
/// The specials properties at a tic boundary: every light thinker keeps its
/// sector's level within `[minlight, maxlight]` (ordered: a fire flicker's
/// `minlight + 16` may exceed `maxlight`); the shared thinker order is the load
/// actors still present, then the light thinkers, then actors created later (A1);
/// `secretcount <= totalsecret`.
fn specials_invariants(s: &Simulation, load: &[ActorId]) {
    let lights: Vec<_> = s.sector_lights_and_specials().unwrap().collect();
    let mut phase = 0;
    for entry in s.shared_thinker_order() {
        match entry {
            ThinkerId::Actor(id) => {
                let at_load = load.contains(&id);
                if phase == 1 || !at_load {
                    phase = 2;
                }
                assert!(phase == 0 || !at_load, "load actor after a light");
            }
            ThinkerId::Sector(id) => {
                assert!(phase < 2, "light after a later actor");
                phase = 1;
                let t = s.sector_thinker(id).unwrap();
                let (lo, hi) = match *t {
                    SectorThinker::Flash(t) => (t.minlight, t.maxlight),
                    SectorThinker::Strobe(t) => (t.minlight, t.maxlight),
                    SectorThinker::Glow(t) => (t.minlight, t.maxlight),
                    SectorThinker::Flicker(t) => (t.minlight, t.maxlight),
                };
                let light = i32::from(lights[usize::from(t.sector())].0);
                assert!(
                    lo.min(hi) <= light && light <= lo.max(hi),
                    "sector {} light {light} outside [{lo}, {hi}]",
                    t.sector()
                );
            }
        }
    }
    let secrets = s.original_player_secret_count().unwrap();
    let total = s.level_statistics().unwrap().total_secrets;
    assert!(0 <= secrets && secrets <= total);
}
fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    // Independent DSIM34 fixtures are read-only decoder inputs, never
    // executable initial states or Rust-produced expectations.
    for original in [
        include_bytes!("../../tests/reference/m3h7_state_4_30.bin").as_slice(),
        include_bytes!("../../tests/reference/m3h7_state_7_127.bin").as_slice(),
        include_bytes!("../../tests/reference/m3h7_state_12_19.bin").as_slice(),
    ] {
        let mut bytes = original.to_vec();
        for c in data.as_chunks::<3>().0.iter().take(32) {
            let at = usize::from(u16::from_le_bytes([c[0], c[1]])) % bytes.len();
            bytes[at] ^= c[2];
        }
        if let Ok(v) = Diagnostic::decode_specials(&bytes, DiagnosticLimits::default()) {
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
    let mut s = Simulation::original_player_specials_map(
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
    let mut load = monsters.clone();
    load.push(body);
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
    specials_invariants(&s, &load);
    let mut captured = s.clone();
    for b in data[2..data.len().min(290)].as_chunks::<9>().0 {
        let before = s.snapshot();
        let (tic, paused) = (s.tic(), s.paused());
        let live = s.original_player(PlayerSlot::One).unwrap().state == PlayerBodyState::Live;
        let result = apply(&mut s, body, &monsters, b, false);
        assert_eq!(result, apply(&mut captured, body, &monsters, b, true));
        if result.is_err() {
            assert_eq!(s.snapshot(), before);
        }
        // Progress property: a valid slot-One command to a live player is never
        // rejected except by the two refused legal source behaviors (the corner
        // jump after a push, the 129th intercept of a trace), and every accepted
        // tic advances the counter by exactly one unless the world is paused. A
        // dead player admits only zero commands, as the body contract documents.
        if b[0] % 9 <= 3 {
            let command = TicCommand {
                forward: i8::from_le_bytes([b[2]]),
                side: i8::from_le_bytes([b[3]]),
                angle_turn: i16::from_le_bytes([b[4], b[5]]),
                buttons: b[8],
            };
            if b[1] % 4 == 0
                && (live || command == TicCommand::default())
                && valid_command(&command)
            {
                assert!(
                    matches!(
                        result,
                        Ok(())
                            | Err(SimError::MapScale(
                                MapScaleError::Escape | MapScaleError::Intercepts
                            ))
                    ),
                    "valid command rejected: {command:?} {result:?}"
                );
            }
            if result.is_ok() {
                assert_eq!(s.tic(), if paused { tic } else { tic + 1 });
            }
        }
        invariants(&s, &boundary, room);
        specials_invariants(&s, &load);
        let snap = s.snapshot();
        assert_eq!(snap, captured.snapshot());
        let mut bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
        let view = Diagnostic::decode_specials(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
        let at = usize::from(u16::from_le_bytes([b[6], b[7]])) % bytes.len();
        bytes[at] ^= b[8];
        let _ = Diagnostic::decode_specials(&bytes, DiagnosticLimits::default());
    }
});
