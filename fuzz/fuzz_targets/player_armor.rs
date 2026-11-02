#![no_main]
use doom_sim::{identity::*, *};
use doom_types::{DoomRng, Fixed};
use libfuzzer_sys::fuzz_target;
const MAPS: [&[u8]; 2] = [
    include_bytes!("../../tests/reference/m3g10_geometry_17.bin"),
    include_bytes!("../../tests/reference/m3g10_geometry_27.bin"),
];
fn apply(
    s: &mut Simulation,
    body: ActorId,
    demon: ActorId,
    b: &[u8],
    capture: bool,
) -> Result<(), SimError> {
    let candidate = ActorId::from_parts(u32::from(b[6]), u64::from(b[7]));
    match b[0] % 8 {
        0..=2 => {
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
        3 => s
            .damage_actor(DamageRequest {
                target: if b[1] & 1 == 0 { body } else { candidate },
                source: if b[2] & 1 == 0 { Some(demon) } else { None },
                inflictor: if b[3] & 1 == 0 { Some(demon) } else { None },
                amount: i32::from(i16::from_le_bytes([b[4], b[5]])),
            })
            .map(|_| ()),
        4 => s.set_actor_target(
            demon,
            if b[1] & 1 == 0 {
                Some(body)
            } else {
                Some(candidate)
            },
        ),
        5 => s.mark_remove(candidate),
        6 => {
            s.set_paused(b[1] & 1 != 0);
            Ok(())
        }
        _ => s
            .spawn_dropped_pickup(
                if b[1] & 1 == 0 {
                    PickupKind::GreenArmor
                } else {
                    PickupKind::BlueArmor
                },
                [Fixed::ZERO; 2],
            )
            .map(|_| ()),
    }
}
fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let wad =
        doom_wad::Wad::parse(MAPS[usize::from(data[0] & 1)], doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()).unwrap();
    let mut s = Simulation::original_player_armor_map(
        map,
        64,
        MapContextOptions {
            initial_rng: DoomRng::from_indices(data[1], 0),
            respawn_monsters: false,
        },
    )
    .unwrap();
    let spawned = s.spawn_map_things().unwrap();
    let demon = spawned
        .spawned
        .into_iter()
        .find(|id| {
            s.actor(*id).unwrap().life().unwrap().actor_type.0
                == OriginalActorType::MT_SERGEANT.ordinal()
        })
        .unwrap();
    let body = s
        .spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])
        .unwrap();
    s.initialize_player_weapons(
        PlayerSlot::One,
        WeaponInventory {
            ready: Weapon::Pistol,
            owned: 3,
            ammo: [50, 0, 0, 0],
        },
    )
    .unwrap();
    // Source-shaped, bounded world collection. No test-only armor setter.
    for _ in 0..8 {
        let mut c = [TicCommand::default(); 4];
        c[0].forward = 4;
        s.advance(c).unwrap();
    }
    let mut captured = s.clone();
    for b in data[2..data.len().min(290)].as_chunks::<9>().0 {
        let before = s.snapshot();
        let armor = s.original_player_armor(PlayerSlot::One).unwrap();
        let result = apply(&mut s, body, demon, b, false);
        assert_eq!(result, apply(&mut captured, body, demon, b, true));
        if result.is_err() {
            assert_eq!(s.snapshot(), before);
            assert_eq!(s.original_player_armor(PlayerSlot::One).unwrap(), armor);
        }
        let snap = s.snapshot();
        assert_eq!(snap, captured.snapshot());
        let mut bytes = snap.diagnostic_bytes(DiagnosticLimits::default()).unwrap();
        let view = Diagnostic::decode_armor(&bytes, DiagnosticLimits::default()).unwrap();
        assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
        let at = usize::from(u16::from_le_bytes([b[6], b[7]])) % bytes.len();
        bytes[at] ^= b[8];
        let _ = Diagnostic::decode_armor(&bytes, DiagnosticLimits::default());
    }
});
