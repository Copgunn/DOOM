//! `movement::path`, the one walk of the slide and use traces (p_maputl.c:772-876),
//! on the trace the genuine observer recorded at SLIDE's tic 38 (`O` row of
//! `tests/reference/overshoot_state.txt`): it must collect the genuine intercepts
//! (`L` rows: the dip line 10, 63 cells past the end cell) for both callers and both
//! arithmetic modes, so it walks the original's cells, not a reach-bounded set.
use crate::intercept_vector::InterceptCaller;
use crate::*;

fn golden() -> ([i32; 4], Vec<(i32, usize)>) {
    let text = include_str!("../../../tests/reference/overshoot_state.txt");
    let rows = |tag: &str| -> Vec<Vec<i64>> {
        (text.lines())
            .filter(|l| l.starts_with(&format!("{tag},SLIDE,")))
            .map(|l| {
                l.split(',')
                    .skip(2)
                    .map(|v| v.trim().parse().unwrap())
                    .collect()
            })
            .collect()
    };
    let o = &rows("O")[0];
    assert_eq!(o[0], 38, "the overshoot tic");
    let trace = [o[1], o[2], o[3], o[4]].map(|v| v as i32);
    let lines = rows("L")
        .iter()
        .map(|l| (l[1] as i32, l[0] as usize))
        .collect();
    (trace, lines)
}

#[test]
fn the_slide_and_use_walk_collects_the_genuine_far_intercept() {
    let (trace, expected) = golden();
    assert_eq!(expected, [(28619, 10)]);
    let bytes = include_bytes!("../../../tests/reference/overshoot_wads/overshoot.wad");
    let wad = doom_wad::Wad::parse(bytes, doom_wad::Limits::default()).unwrap();
    let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).unwrap();
    let options = MapContextOptions::default();
    let sim = Simulation::original_player_episode1_v5_level(
        map,
        1,
        Skill::Medium,
        options,
        LevelStart::NewGame,
    )
    .unwrap();
    let start = [Fixed::from_bits(trace[0]), Fixed::from_bits(trace[1])];
    let end = [
        Fixed::from_bits(trace[0] + trace[2]),
        Fixed::from_bits(trace[1] + trace[3]),
    ];
    for caller in [InterceptCaller::Slide, InterceptCaller::Use] {
        for checked in [false, true] {
            let hits = movement::path(&sim.world, start, end, Some(128), checked, caller).unwrap();
            let hits: Vec<_> = hits.into_iter().map(|(f, i)| (f.bits(), i)).collect();
            assert_eq!(hits, expected, "{caller:?} checked {checked}");
        }
    }
}
