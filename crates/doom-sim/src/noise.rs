//! p_enemy.c P_RecursiveSound as a pure flood over the static sectors, for the
//! staged weapon noise of the heights policy. `Simulation::emit_noise` keeps its
//! own copy for the historical profiles; both follow the same source order.
use crate::*;

/// Every `(sector, soundblocks)` visit of `P_RecursiveSound(start, 0)` in source
/// order: depth-first, pre-order, lines in linedef index order (`P_GroupLines`),
/// through two-sided lines whose opening is positive (`P_LineOpening`), where an
/// `ML_SOUNDBLOCK` line raises `soundblocks` once and a sector is revisited only
/// with fewer blocks (`sec->soundtraversed <= soundblocks+1`). Bounded by the
/// sector count times two. With the world's prepared per-sector line lists (the
/// `sector->lines` of `P_GroupLines`, in linedef order, the real-map policy) each
/// sector reads only its own lines; the results are identical because the scan
/// below only asks which two-sided lines leave the sector, in linedef order.
pub(crate) fn flood(world: &StaticWorld, sectors: &[SectorState], start: u16) -> Vec<(u16, u8)> {
    let map = world.map();
    let mut visits = Vec::new();
    let mut traversed = vec![u8::MAX; sectors.len()];
    let mut stack = vec![(start, 0_u8)];
    let all: Vec<u16> = match &world.sector_lines {
        Some(_) => Vec::new(),
        None => (0..map.linedefs().len()).map(|i| i as u16).collect(),
    };
    while let Some((sector, blocks)) = stack.pop() {
        let i = usize::from(sector);
        if traversed[i] <= blocks {
            continue;
        }
        traversed[i] = blocks;
        visits.push((sector, blocks));
        let mut children = Vec::new();
        let candidates = match &world.sector_lines {
            Some(lists) => lists[i].as_slice(),
            None => all.as_slice(),
        };
        for &line in candidates {
            let line = usize::from(line);
            let l = &map.linedefs()[line];
            if l.flags & 4 == 0 {
                continue;
            }
            let ends = l
                .sides
                .map(|s| map.sidedefs()[usize::from(s.expect("admitted two-sided line"))].sector);
            if !ends.contains(&sector) {
                continue;
            }
            if multisector::opening(world, sectors, line).is_none_or(|o| o.range <= Fixed::ZERO) {
                continue;
            }
            let other = if ends[0] == sector { ends[1] } else { ends[0] };
            let blocked = u8::from(l.flags & 64 != 0);
            if blocks + blocked <= 1 {
                children.push((other, blocks + blocked));
            }
        }
        stack.extend(children.into_iter().rev());
    }
    visits
}
