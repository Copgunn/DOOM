#![no_main]
use doom_sim::*;
use doom_types::{DoomRng, Fixed};
use libfuzzer_sys::fuzz_target;
// Committed synthetic M3h4 rooms: the stairs with an Imp on a tread, the ledge with
// a shotgunner at its edge, the window room with two former humans and the
// blocking fence with an Imp behind it.
const MAPS: [&[u8]; 4] = [
    include_bytes!("../../tests/reference/m3h4_geometry_1.bin"),
    include_bytes!("../../tests/reference/m3h4_geometry_8.bin"),
    include_bytes!("../../tests/reference/m3h4_geometry_11.bin"),
    include_bytes!("../../tests/reference/m3h4_geometry_17.bin"),
];
/// Player spawn of the committed scenario in each room.
const SPAWNS: [[i16; 2]; 4] = [[900, 0], [-500, 0], [-300, 0], [-300, 0]];
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
/// Invariant V of the M3h4 note at a tic boundary: every actor that is not a
/// bullet puff or blood sits within `[floor, ceiling - height]`.
fn vertical_invariant(s: &Simulation) {
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
    // Independent DSIM31 fixtures are read-only decoder inputs, never
    // executable initial states or Rust-produced expectations.
    for original in [
        include_bytes!("../../tests/reference/m3h4_state_3_480.bin").as_slice(),
        include_bytes!("../../tests/reference/m3h4_state_10_240.bin").as_slice(),
    ] {
        let mut bytes = original.to_vec();
        for c in data.as_chunks::<3>().0.iter().take(32) {
            let at = usize::from(u16::from_le_bytes([c[0], c[1]])) % bytes.len();
            bytes[at] ^= c[2];
        }
        if let Ok(v) = Diagnostic::decode_heights(&bytes, DiagnosticLimits::default()) {
            assert_eq!(v.encode(DiagnosticLimits::default()).unwrap(), bytes);
        }
    }
    let which = usize::from(data[0] % 4);
    let wad = doom_wad::Wad::parse(MAPS[which], doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut s = Simulation::original_player_heights_map(
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
        vertical_invariant(&s);
        let snap = s.snapshot();
        assert_eq!(snap, captured.snapshot());
        let mut bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
        let view = Diagnostic::decode_heights(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
        let at = usize::from(u16::from_le_bytes([b[6], b[7]])) % bytes.len();
        bytes[at] ^= b[8];
        let _ = Diagnostic::decode_heights(&bytes, DiagnosticLimits::default());
    }
});
