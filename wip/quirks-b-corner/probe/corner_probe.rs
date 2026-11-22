//! CORNER PROBE (scratch copy only): observation-only trace logs; never read by the simulation.
use std::io::Write;
pub(crate) fn log(kind: &str, start: [doom_types::Fixed; 2], end: [doom_types::Fixed; 2], cells: &[[i32; 2]], reached: bool, n: usize) {
    let Ok(path) = std::env::var("CORNER_RUST_LOG") else { return };
    let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) else { return };
    let mut s = format!("{kind} start {} {} end {} {} iters {} reached {} intercepts {} cells", start[0].bits(), start[1].bits(), end[0].bits(), end[1].bits(), cells.len(), u8::from(reached), n);
    for c in cells { s.push_str(&format!(" {},{}", c[0], c[1])); }
    let _ = writeln!(f, "{s}");
}

/// CORNER PROBE direct calls (scratch copy only): `movement::path` (the slide and use walk) on the
/// world of a wire-24 map, once per `[x1, y1, x2, y2]` in fixed-point units; returns every hit
/// (fraction bits, line index). No limit, unchecked arithmetic, as the pure walk.
pub fn corner_direct_paths(map: doom_map::Map, calls: &[[i32; 4]]) -> Vec<Vec<(i32, usize)>> {
    let sim = crate::Simulation::original_player_episode1_v2_map(map, 1, crate::Skill::Medium, crate::MapContextOptions::default()).expect("wire 24 map");
    calls
        .iter()
        .map(|c| {
            let start = [doom_types::Fixed::from_bits(c[0]), doom_types::Fixed::from_bits(c[1])];
            let end = [doom_types::Fixed::from_bits(c[2]), doom_types::Fixed::from_bits(c[3])];
            crate::movement::path(&sim.world, start, end, None, false)
                .expect("unlimited unchecked path")
                .into_iter()
                .map(|(f, i)| (f.bits(), i))
                .collect()
        })
        .collect()
}
