#![no_main]
#[path = "../../tests/support/m3f2.rs"]
mod extent;
#[path = "../../tests/support/m3f3.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
fn patch(bytes: &mut [u8], name: &[u8], relative: usize, value: i16) {
    let directory = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    for i in 0..count {
        let entry = directory + 16 * i;
        if bytes[entry + 8..entry + 16].starts_with(name) {
            let offset = u32::from_le_bytes(bytes[entry..entry + 4].try_into().unwrap()) as usize;
            let len = u32::from_le_bytes(bytes[entry + 4..entry + 8].try_into().unwrap()) as usize;
            if relative + 2 <= len {
                bytes[offset + relative..offset + relative + 2]
                    .copy_from_slice(&value.to_le_bytes());
            }
            return;
        }
    }
}
fn operation(s: &mut Simulation, ids: &mut Vec<ActorId>, c: &[u8]) {
    if ids.is_empty() {
        return;
    }
    let mut id = ids[usize::from(c[1]) % ids.len()];
    if c[0] & 128 != 0 {
        id = ActorId::from_parts(id.slot(), id.generation() + 1);
    }
    let other = ids[usize::from(c[3]) % ids.len()];
    let before = s.snapshot();
    let result = match c[0] % 12 {
        0 => {
            let mut result = Ok(());
            for _ in 0..=c[2] % 8 {
                let old = s.snapshot();
                let mut commands = [TicCommand::default(); 4];
                commands[0].buttons = if c[3] & 128 != 0 { c[3] } else { 0 };
                match s.step(commands) {
                    Ok(_) => (),
                    Err(error) => {
                        assert_eq!(s.snapshot(), old);
                        result = Err(error);
                        break;
                    }
                }
            }
            // A multi-call script is not one operation: successful earlier tics
            // are retained. Every individual failed tic is checked above.
            if result.is_err() {
                return;
            }
            result
        }
        1 => s
            .damage_actor(DamageRequest {
                target: id,
                source: if c[2] & 64 != 0 { None } else { Some(other) },
                inflictor: if c[2] & 32 != 0 { Some(other) } else { None },
                amount: i32::from(c[2]),
            })
            .map(|_| ()),
        2 => s.enter_actor_state(
            id,
            StateId(if c[3] & 1 == 0 {
                u16::from(c[2] % 32)
            } else {
                149 + u16::from(c[2] % 25)
            }),
        ),
        3 => s.demon_melee_attack(id),
        4 => s.look(id).map(|_| ()),
        5 => s.mark_remove(id),
        6 => s.set_momentum(
            id,
            [
                Fixed::from_map_units(i16::from(c[2] as i8)),
                Fixed::from_map_units(i16::from(c[3] as i8)),
            ],
        ),
        7 => s
            .spawn_type(
                ActorTypeId(135),
                [
                    Fixed::from_bits(i32::from(c[2] as i8) * 17 * 65536),
                    Fixed::from_bits(i32::from(c[3] as i8) * 17 * 65536),
                ],
                SpawnHeight::Floor,
            )
            .map(|new| ids.push(new)),
        8 => s.noise_alert(id, other),
        9 => s.check_sight(id, other).map(|_| ()),
        10 => s.restart_demon_map_context().map(|new| {
            *s = new;
            *ids = s.actor_order().collect();
        }),
        _ => {
            s.set_paused(c[2] & 1 != 0);
            Ok(())
        }
    };
    if result.is_err() {
        assert_eq!(before, s.snapshot());
    }
}
fuzz_target!(|data: &[u8]| {
    if data.len() < 3 || data.len() > 35 {
        return;
    }
    let cases = fixtures::cases();
    let c = &cases[usize::from(data[0]) % cases.len()];
    let boundary = data[0] == 255;
    let mut bytes = fixtures::map_bytes(c.v("map") as usize).to_vec();
    match data[1] % 10 {
        1 => patch(&mut bytes, b"THINGS", 4, 180),
        2 => patch(&mut bytes, b"LINEDEFS", 0, 32767),
        3 => patch(&mut bytes, b"BLOCKMAP", 0, 2200),
        4 => patch(&mut bytes, b"VERTEXES", 0, 2049),
        8 => patch(&mut bytes, b"SECTORS", 0, 513),
        9 => patch(&mut bytes, b"THINGS", 8, 16),
        _ => (),
    }
    let Ok(wad) = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()) else {
        return;
    };
    let Ok(mut map) = doom_map::Map::from_wad(&wad, "MAP01", doom_map::MapLimits::default()) else {
        return;
    };
    if boundary {
        map = extent::source_clip_boundary_map();
    }
    let mut options = fixtures::options(c);
    if data[1] % 10 == 5 {
        options.skill = Skill::Nightmare;
    }
    if data[1] % 10 == 6 {
        options.mode = ActorGameMode::Cooperative;
    }
    if data[1] % 10 == 7 {
        options.no_monsters = !options.no_monsters;
    }
    let limit = u32::from(data[2] % 65);
    let build = |map| {
        Simulation::with_demon_expanded_map_registry(
            map,
            limit,
            ActorRegistry::demon_map_context_fixture(),
            options,
            MapContextOptions::default(),
        )
    };
    let (mut a, mut b) = match (build(map.clone()), build(map)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(a), Err(b)) => {
            assert_eq!(a, b);
            return;
        }
        _ => panic!("instance admission mismatch"),
    };
    let before_a = a.snapshot();
    let before_b = b.snapshot();
    let ra = a.spawn_map_things();
    let rb = b.spawn_map_things();
    assert_eq!(ra, rb);
    if ra.is_err() {
        assert_eq!(a.snapshot(), before_a);
        assert_eq!(b.snapshot(), before_b);
        return;
    }
    for i in 0..if boundary { 2 } else { 4 } {
        let position = if boundary {
            extent::source_clip_boundary_positions()[i]
        } else {
            [
                Fixed::from_bits(c.v(&format!("p{i}x"))),
                Fixed::from_bits(c.v(&format!("p{i}y"))),
            ]
        };
        let old_a = a.snapshot();
        let old_b = b.snapshot();
        let ia = a.spawn_type(ActorTypeId(135), position, SpawnHeight::Floor);
        let ib = b.spawn_type(ActorTypeId(135), position, SpawnHeight::Floor);
        assert_eq!(ia, ib);
        let id = match ia {
            Ok(id) => id,
            Err(_) => {
                assert_eq!(old_a, a.snapshot());
                assert_eq!(old_b, b.snapshot());
                break;
            }
        };
        for sim in [&mut a, &mut b] {
            sim.set_perception_player(
                i,
                PerceptionPlayer {
                    active: i == 0,
                    health: 100,
                    actor: Some(id),
                },
            )
            .unwrap();
        }
    }
    let mut ia: Vec<_> = a.actor_order().collect();
    let mut ib = ia.clone();
    assert_eq!(a.snapshot(), b.snapshot());
    if boundary && ia.len() >= 2 {
        let old = a.snapshot();
        assert_eq!(
            a.check_sight(ia[0], ia[1]),
            Err(PerceptionError::Arithmetic.into())
        );
        assert_eq!(a.snapshot(), old);
    }
    for command in data[3..].chunks_exact(4).take(8) {
        operation(&mut a, &mut ia, command);
        operation(&mut b, &mut ib, command);
        assert_eq!(ia, ib);
        assert_eq!(a.snapshot(), b.snapshot());
    }
});
