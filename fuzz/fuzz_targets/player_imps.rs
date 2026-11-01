#![no_main]
use doom_sim::*;
use doom_types::{DoomRng, Fixed};
use libfuzzer_sys::fuzz_target;
// Committed synthetic M3h2 geometries: same-species line, mixed roster, eight Imps.
const MAPS: [&[u8]; 3] = [
    include_bytes!("../../tests/reference/m3h2_geometry_10.bin"),
    include_bytes!("../../tests/reference/m3h2_geometry_16.bin"),
    include_bytes!("../../tests/reference/m3h2_geometry_17.bin"),
];
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
    // Independent DSIM29 fixtures are read-only decoder inputs, never
    // executable initial states or Rust-produced expectations.
    for original in [
        include_bytes!("../../tests/reference/m3h2_state_18_720.bin").as_slice(),
        include_bytes!("../../tests/reference/m3h2_state_19_480.bin").as_slice(),
    ] {
        let mut bytes = original.to_vec();
        for c in data.chunks_exact(3).take(32) {
            let at = usize::from(u16::from_le_bytes([c[0], c[1]])) % bytes.len();
            bytes[at] ^= c[2];
        }
        if let Ok(v) = Diagnostic::decode_imps(&bytes, DiagnosticLimits::default()) {
            assert_eq!(v.encode(DiagnosticLimits::default()).unwrap(), bytes);
        }
    }
    let wad =
        doom_wad::Wad::parse(MAPS[usize::from(data[0] % 3)], doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut s = Simulation::original_player_imps_map(
        map,
        64,
        MapContextOptions {
            initial_rng: DoomRng::from_indices(data[1], 0),
            respawn_monsters: false,
        },
    )
    .unwrap();
    let monsters = s.spawn_map_things().unwrap().spawned;
    let body = s
        .spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap();
    s.initialize_player_weapons(
        PlayerSlot::One,
        WeaponInventory {
            ready: Weapon::Chaingun,
            owned: 11,
            ammo: [200, 0, 0, 0],
        },
    )
    .unwrap();
    let mut captured = s.clone();
    for b in data[2..data.len().min(290)].chunks_exact(9) {
        let before = s.snapshot();
        let (tic, paused) = (s.tic(), s.paused());
        let result = apply(&mut s, body, &monsters, b, false);
        assert_eq!(result, apply(&mut captured, body, &monsters, b, true));
        if result.is_err() {
            assert_eq!(s.snapshot(), before);
        } else if b[0] % 8 <= 3 {
            // An accepted tic advances the counter by one unless paused.
            assert_eq!(s.tic(), if paused { tic } else { tic + 1 });
        }
        let snap = s.snapshot();
        assert_eq!(snap, captured.snapshot());
        let mut bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
        let view = Diagnostic::decode_imps(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
        let at = usize::from(u16::from_le_bytes([b[6], b[7]])) % bytes.len();
        bytes[at] ^= b[8];
        let _ = Diagnostic::decode_imps(&bytes, DiagnosticLimits::default());
    }
});
