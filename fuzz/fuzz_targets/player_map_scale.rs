#![no_main]
use doom_sim::*;
use doom_types::{DoomRng, Fixed};
use libfuzzer_sys::fuzz_target;
// Committed synthetic M3h3 rooms: the stall room, the full 4096-unit room with
// eight far Imps, the mixed-roster corner fight and the offset room with drops.
const MAPS: [&[u8]; 4] = [
    include_bytes!("../../tests/reference/m3h3_geometry_2.bin"),
    include_bytes!("../../tests/reference/m3h3_geometry_10.bin"),
    include_bytes!("../../tests/reference/m3h3_geometry_11.bin"),
    include_bytes!("../../tests/reference/m3h3_geometry_12.bin"),
];
/// Player spawn inside each room, far from the origin where the rooms allow it.
const SPAWNS: [[i16; 2]; 4] = [[-600, 0], [1800, 1800], [1900, 0], [1800, -1800]];
/// A command the admitted domain must execute: slot One, source movement range,
/// buttons the four-weapon policy admits. Anything else is an invalid input.
fn valid_command(c: &TicCommand) -> bool {
    let weapon = (c.buttons & 0x38) >> 3;
    (-50..=50).contains(&c.forward)
        && (-50..=50).contains(&c.side)
        && c.buttons & !0x3b == 0
        && if c.buttons & 2 != 0 {
            weapon <= 3
        } else {
            weapon == 0
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
    match b[0] % 8 {
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
        _ => {
            s.set_paused(b[1] & 1 != 0);
            Ok(())
        }
    }
}
fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    // Independent DSIM30 fixtures are read-only decoder inputs, never
    // executable initial states or Rust-produced expectations.
    for original in [
        include_bytes!("../../tests/reference/m3h3_state_10_960.bin").as_slice(),
        include_bytes!("../../tests/reference/m3h3_state_11_720.bin").as_slice(),
    ] {
        let mut bytes = original.to_vec();
        for c in data.chunks_exact(3).take(32) {
            let at = usize::from(u16::from_le_bytes([c[0], c[1]])) % bytes.len();
            bytes[at] ^= c[2];
        }
        if let Ok(v) = Diagnostic::decode_map_scale(&bytes, DiagnosticLimits::default()) {
            assert_eq!(v.encode(DiagnosticLimits::default()).unwrap(), bytes);
        }
    }
    let which = usize::from(data[0] % 4);
    let wad = doom_wad::Wad::parse(MAPS[which], doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut s = Simulation::original_player_map_scale_map(
        map,
        MapContextOptions {
            initial_rng: DoomRng::from_indices(data[1], 0),
            respawn_monsters: false,
        },
    )
    .unwrap();
    let monsters = s.spawn_map_things().unwrap().spawned;
    let body = s
        .spawn_player_body(PlayerSlot::One, SPAWNS[which].map(Fixed::from_map_units))
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
    let mut captured = s.clone();
    for b in data[2..data.len().min(290)].chunks_exact(9) {
        let before = s.snapshot();
        let (tic, paused) = (s.tic(), s.paused());
        let live = s.original_player(PlayerSlot::One).unwrap().state == PlayerBodyState::Live;
        let result = apply(&mut s, body, &monsters, b, false);
        assert_eq!(result, apply(&mut captured, body, &monsters, b, true));
        if result.is_err() {
            assert_eq!(s.snapshot(), before);
        }
        // Progress property: a valid slot-One command to a live player is never
        // rejected, and every accepted tic advances the counter by exactly one
        // unless the world is paused. A dead player admits only zero commands,
        // as the body contract documents.
        if b[0] % 8 <= 3 {
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
                assert_eq!(result, Ok(()), "valid command rejected: {command:?}");
            }
            if result.is_ok() {
                assert_eq!(s.tic(), if paused { tic } else { tic + 1 });
            }
        }
        let snap = s.snapshot();
        assert_eq!(snap, captured.snapshot());
        let mut bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
        let view = Diagnostic::decode_map_scale(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
        let at = usize::from(u16::from_le_bytes([b[6], b[7]])) % bytes.len();
        bytes[at] ^= b[8];
        let _ = Diagnostic::decode_map_scale(&bytes, DiagnosticLimits::default());
    }
});
