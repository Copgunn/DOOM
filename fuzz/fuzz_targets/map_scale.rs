#![no_main]
#[path = "../../tests/support/m3f1.rs"]
mod fixtures;
use doom_sim::{ActorSpec, MapScaleLimits, Simulation, TicCommand};
use doom_types::Fixed;
use libfuzzer_sys::fuzz_target;
fn raw(n: i32) -> Fixed {
    Fixed::from_bits(n)
}
fn operation(sim: &mut Simulation, ids: &mut Vec<doom_sim::ActorId>, c: &[u8]) {
    let id = ids[usize::from(c[1]) % ids.len()];
    let before = sim.snapshot();
    let signed = |i: usize| i32::from(c[i] as i8);
    let result = match c[0] % 8 {
        0 => sim.set_momentum(id, [raw(signed(2) * 65536), raw(signed(3) * 65536)]),
        1 => sim.set_vertical_momentum(id, raw(signed(2) * 65536)),
        2 => sim
            .check_multisector_position(
                id,
                [raw(signed(2) * 8 * 65536), raw(signed(3) * 8 * 65536)],
            )
            .map(|_| ()),
        3 => sim.check_sight(id, ids[0]).map(|_| ()),
        4 => sim.step([TicCommand::default(); 4]).map(|_| ()),
        5 => sim.mark_remove(id),
        6 => sim
            .spawn(ActorSpec {
                position: [raw(signed(2) * 8 * 65536), raw(signed(3) * 4 * 65536)],
                ..Default::default()
            })
            .map(|id| ids.push(id)),
        _ => {
            let mut commands = [TicCommand::default(); 4];
            commands[0].buttons = c[2];
            sim.step(commands).map(|_| ())
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
    let mut lumps = fixtures::lumps(usize::from(data[0]) % fixtures::map_count());
    match data[1] % 8 {
        1 => fixtures::set_word(&mut lumps[2].1, 0, 32767),
        2 => {
            let sector = lumps[8].1[..26].to_vec();
            while lumps[8].1.len() < 257 * 26 {
                lumps[8].1.extend(&sector);
            }
            lumps[9].1.resize((257_usize * 257).div_ceil(8), 0);
        }
        3 => {
            let offset = u16::from_le_bytes(lumps[10].1[8..10].try_into().unwrap()) as usize;
            fixtures::set_word(&mut lumps[10].1, 2 * (offset + 1), 32767);
        }
        5 => {
            let offset = u16::from_le_bytes(lumps[10].1[8..10].try_into().unwrap()) as usize;
            let lines = lumps[2].1.len() / 14;
            fixtures::set_word(
                &mut lumps[10].1,
                2 * (offset + 1),
                (usize::from(data[2]) % lines) as i16,
            );
        }
        6 => fixtures::set_word(&mut lumps[4].1, 0, if data[2] & 1 == 0 { 512 } else { 513 }),
        7 if !lumps[7].1.is_empty() => {
            let node = lumps[7].1[..28].to_vec();
            lumps[7].1.truncate(28);
            for i in 1..15 {
                let mut n = node.clone();
                fixtures::set_word(&mut n, 24, i - 1);
                fixtures::set_word(&mut n, 26, i - 1);
                lumps[7].1.extend(n);
            }
        }
        _ => (),
    }
    let Ok(map) = fixtures::decode(&lumps) else {
        return;
    };
    let budget = if data[1] % 8 == 4 {
        usize::from(data[2]) * 32
    } else {
        262144
    };
    let limits = MapScaleLimits {
        max_actors: 4,
        max_prepared_references: budget,
    };
    let a = Simulation::with_map_scale_limits(map.clone(), limits);
    let b = Simulation::with_map_scale_limits(map, limits);
    let (Ok(mut a), Ok(mut b)) = (a, b) else {
        return;
    };
    let mut ids = Vec::new();
    let mut other_ids = Vec::new();
    for x in [-24, 24] {
        let spec = ActorSpec {
            position: [Fixed::from_map_units(x), Fixed::ZERO],
            height: Fixed::from_map_units(16),
            ..Default::default()
        };
        ids.push(a.spawn(spec).unwrap());
        other_ids.push(b.spawn(spec).unwrap());
    }
    for c in data[3..].as_chunks::<4>().0.iter().take(8) {
        operation(&mut a, &mut ids, c);
        operation(&mut b, &mut other_ids, c);
        assert_eq!(ids, other_ids);
        assert_eq!(a.snapshot(), b.snapshot());
    }
});
