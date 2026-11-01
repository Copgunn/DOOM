//! p_enemy.c P_RecursiveSound as a pure flood over the static sectors, for the
//! staged weapon noise of the heights policy. `Simulation::emit_noise` keeps its
//! own copy for the historical profiles; both follow the same source order.
use crate::*;

/// Every `(sector, soundblocks)` visit of `P_RecursiveSound(start, 0)` in source
/// order: depth-first, pre-order, lines in linedef index order (`P_GroupLines`),
/// through two-sided lines whose opening is positive (`P_LineOpening`), where an
/// `ML_SOUNDBLOCK` line raises `soundblocks` once and a sector is revisited only
/// with fewer blocks (`sec->soundtraversed <= soundblocks+1`). Bounded by the
/// sector count times two.
pub(crate) fn flood(world: &StaticWorld, sectors: &[SectorState], start: u16) -> Vec<(u16, u8)> {
    let map = world.map();
    let mut visits = Vec::new();
    let mut traversed = vec![u8::MAX; sectors.len()];
    let mut stack = vec![(start, 0_u8)];
    while let Some((sector, blocks)) = stack.pop() {
        let i = usize::from(sector);
        if traversed[i] <= blocks {
            continue;
        }
        traversed[i] = blocks;
        visits.push((sector, blocks));
        let mut children = Vec::new();
        for (line, l) in map.linedefs().iter().enumerate() {
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
