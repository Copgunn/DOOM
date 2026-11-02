#![no_main]
#[path = "../../tests/support/m3f2.rs"]
mod fixtures;
use doom_sim::{ActorSpec, MapExtentLimits, Simulation, TicCommand};
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
fn raw(n: i32) -> Fixed {
    Fixed::from_bits(n)
}
fn operation(sim: &mut Simulation, ids: &mut Vec<doom_sim::ActorId>, c: &[u8]) {
    let id = ids[usize::from(c[1]) % ids.len()];
    let before = sim.snapshot();
    let signed = |i: usize| i32::from(c[i] as i8);
    let coordinate = |i: usize| {
        if c[0] & 128 != 0 {
            if c[i] & 1 == 0 { i32::MIN } else { i32::MAX }
        } else {
            signed(i) * 17 * 65536 + i32::from(c[0] & 1)
        }
    };
    let result = match c[0] % 10 {
        0 => sim.set_momentum(id, [raw(signed(2) * 65536), raw(signed(3) * 65536)]),
        1 => sim.set_vertical_momentum(id, raw(signed(2) * 65536)),
        2 => sim
            .check_multisector_position(id, [raw(coordinate(2)), raw(coordinate(3))])
            .map(|_| ()),
        3 => sim
            .check_sight(id, ids[usize::from(c[2]) % ids.len()])
            .map(|_| ()),
        4 => sim.step([TicCommand::default(); 4]).map(|_| ()),
        5 => sim.mark_remove(id),
        6 => sim
            .spawn(ActorSpec {
                position: [raw(coordinate(2)), raw(coordinate(3))],
                height: Fixed::from_map_units(16),
                ..Default::default()
            })
            .map(|id| ids.push(id)),
        7 => {
            let mut commands = [TicCommand::default(); 4];
            commands[0].buttons = c[2];
            sim.step(commands).map(|_| ())
        }
        8 => {
            for _ in 0..=c[2] % 32 {
                sim.gameplay_random();
            }
            Ok(())
        }
        _ => {
            sim.set_paused(c[2] & 1 != 0);
            Ok(())
        }
    };
    if result.is_err() {
        assert_eq!(before, sim.snapshot());
    }
}
fuzz_target!(|data: &[u8]| {
    if data.len() < 3 || data.len() > 35 {
        return;
    }
    let boundary = data[0] == 255;
    let mut lumps = fixtures::lumps(usize::from(data[0]) % fixtures::map_count());
    match data[1] % 10 {
        1 => fixtures::set_word(&mut lumps[2].1, 0, 32767),
        2 => fixtures::set_word(&mut lumps[10].1, 0, i16::from(data[2] as i8) * 17),
        3 => {
            let start = u16::from_le_bytes(lumps[10].1[8..10].try_into().unwrap()) as usize;
            fixtures::set_word(&mut lumps[10].1, 2 * (start + 1), 32767);
        }
        5 => {
            let start = u16::from_le_bytes(lumps[10].1[8..10].try_into().unwrap()) as usize;
            let lines = lumps[2].1.len() / 14;
            fixtures::set_word(
                &mut lumps[10].1,
                2 * (start + 1),
                (usize::from(data[2]) % lines) as i16,
            );
        }
        6 => fixtures::set_word(
            &mut lumps[4].1,
            0,
            if data[2] & 1 == 0 { 2048 } else { 2049 },
        ),
        7 if !lumps[7].1.is_empty() => {
            let node = lumps[7].1[..28].to_vec();
            lumps[7].1.truncate(28);
            for i in 1..=13 {
                let mut next = node.clone();
                fixtures::set_word(&mut next, 24, i - 1);
                fixtures::set_word(&mut next, 26, i - 1);
                lumps[7].1.extend(next);
            }
        }
        8 => lumps[10]
            .1
            .resize(2 * (65536 + usize::from(data[2] & 1)), 255),
        _ => (),
    }
    let Ok(mut map) = fixtures::decode(&lumps) else {
        return;
    };
    if boundary {
        map = fixtures::source_clip_boundary_map();
    }
    let limits = MapExtentLimits {
        max_actors: 4,
        max_prepared_references: if data[1] % 10 == 4 {
            usize::from(data[2]) * 1024
        } else {
            262144
        },
        max_collision_work: if data[1] % 10 == 9 {
            usize::from(data[2]) * 16
        } else {
            16384
        },
        ..Default::default()
    };
    let center = map
        .vertices()
        .iter()
        .fold([i32::MAX, i32::MIN, i32::MAX, i32::MIN], |v, p| {
            [
                v[0].min(i32::from(p.x)),
                v[1].max(i32::from(p.x)),
                v[2].min(i32::from(p.y)),
                v[3].max(i32::from(p.y)),
            ]
        });
    let x = (center[0] + center[1]) / 2;
    let y = (center[2] + center[3]) / 2;
    let a = Simulation::with_map_extent_limits(map.clone(), limits);
    let b = Simulation::with_map_extent_limits(map, limits);
    let (mut a, mut b) = match (a, b) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(a), Err(b)) => {
            assert_eq!(a, b);
            return;
        }
        _ => panic!("independent constructor admission differs"),
    };
    let mut ids = Vec::new();
    let mut other_ids = Vec::new();
    for (i, delta) in [-24, 24].into_iter().enumerate() {
        let spec = ActorSpec {
            position: if boundary {
                fixtures::source_clip_boundary_positions()[i]
            } else {
                [raw((x + delta) * 65536), raw(y * 65536)]
            },
            height: Fixed::from_map_units(16),
            ..Default::default()
        };
        let before = a.snapshot();
        let other_before = b.snapshot();
        match (a.spawn(spec), b.spawn(spec)) {
            (Ok(id), Ok(other)) => {
                ids.push(id);
                other_ids.push(other);
            }
            (Err(_), Err(_)) => {
                assert_eq!(before, a.snapshot());
                assert_eq!(other_before, b.snapshot());
                assert_eq!(a.snapshot(), b.snapshot());
                return;
            }
            _ => panic!("independent setup admission differs"),
        }
    }
    for c in data[3..].as_chunks::<4>().0.iter().take(8) {
        operation(&mut a, &mut ids, c);
        operation(&mut b, &mut other_ids, c);
        assert_eq!(ids, other_ids);
        assert_eq!(a.snapshot(), b.snapshot());
    }
});
