//! Load rules of the movers policy (M3h8 note §5–§7): admitted line specials, the
//! mover families, neighbours, the reachable-height fixpoint, counts, the `spechit`
//! window and sky movers, all computed once from the static map in exact integers.
//! Every failure is a field-less error and no state is built.
use crate::{MapScaleError, PursuitError, SimError, player_map_scale::ML_TWOSIDED};
use doom_map::Map;

/// p_switch.c `P_UseSpecialLine` / p_doors.c `EV_VerticalDoor`: DR door (use).
pub(crate) const LINE_DOOR_DR: i16 = 1;
/// SR door, `EV_DoDoor(normal)` on the tagged sectors (use, repeatable switch).
pub(crate) const LINE_DOOR_SR: i16 = 63;
/// W1 door, `EV_DoDoor(blazeOpen)` on the tagged sectors (cross).
pub(crate) const LINE_DOOR_BLAZE: i16 = 109;
/// WR lift, `EV_DoPlat(downWaitUpStay)` on the tagged sectors (cross).
pub(crate) const LINE_LIFT: i16 = 88;
/// W1 floor, `EV_DoFloor(turboLower)` on the tagged sectors (cross).
pub(crate) const LINE_FLOOR_TURBO: i16 = 36;
/// W1 floor, `EV_DoFloor(lowerFloorToLowest)` on the tagged sectors (cross).
pub(crate) const LINE_FLOOR_LOWEST: i16 = 38;
/// S1 exit switch, `G_ExitLevel` (use).
pub(crate) const LINE_EXIT: i16 = 11;
/// p_spec.h `MAXPLATS`: `activeplats` slots (`P_AddActivePlat` `I_Error`s past it).
pub(crate) const MAX_PLATS: usize = 30;
/// p_spec.h `MAXBUTTONS`: `buttonlist` slots (`P_StartButton` `I_Error`s past it).
pub(crate) const MAX_BUTTONS: usize = 16;
/// p_map.c `MAXSPECIALCROSS`: `spechit` entries one `P_CheckPosition` may store.
pub(crate) const MAX_SPECIAL_CROSS: usize = 8;
/// The reachable-height bound of rule 4 (map units), the M3h4 vertical arithmetic's.
pub(crate) const REACHABLE_UNITS: i32 = 512;

/// The mover family of a sector (rule 2): what its `specialdata` can hold.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Family {
    /// Back sector of a DR line or target of 63/109 (`vldoor_t`).
    Door,
    /// Target of 88 (`plat_t`).
    Plat,
    /// Target of 36/38 (`floormove_t`).
    Floor,
}

/// Which admitted kinds act on one sector (a sector may be a door of several kinds,
/// decision N5).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Kinds {
    dr: bool,
    sr: bool,
    blaze: bool,
    lift: bool,
    turbo: bool,
    lowest: bool,
}
impl Kinds {
    fn door(self) -> bool {
        self.dr || self.sr || self.blaze
    }
    fn families(self) -> impl Iterator<Item = Family> {
        [
            self.door().then_some(Family::Door),
            self.lift.then_some(Family::Plat),
            (self.turbo || self.lowest).then_some(Family::Floor),
        ]
        .into_iter()
        .flatten()
    }
}

/// The episode-1 entries of `alphSwitchList` (p_switch.c:51–69), both columns, in
/// table order: the first 38 `switchlist` slots `P_InitSwitchList` builds in every
/// game mode (p_switch.c:107–148 keeps `episode <= 1, 2 or 3` in table order).
pub(crate) const EPISODE1_SWITCHES: [[&[u8]; 2]; 19] = [
    [b"SW1BRCOM", b"SW2BRCOM"],
    [b"SW1BRN1", b"SW2BRN1"],
    [b"SW1BRN2", b"SW2BRN2"],
    [b"SW1BRNGN", b"SW2BRNGN"],
    [b"SW1BROWN", b"SW2BROWN"],
    [b"SW1COMM", b"SW2COMM"],
    [b"SW1COMP", b"SW2COMP"],
    [b"SW1DIRT", b"SW2DIRT"],
    [b"SW1EXIT", b"SW2EXIT"],
    [b"SW1GRAY", b"SW2GRAY"],
    [b"SW1GRAY1", b"SW2GRAY1"],
    [b"SW1METAL", b"SW2METAL"],
    [b"SW1PIPE", b"SW2PIPE"],
    [b"SW1SLAD", b"SW2SLAD"],
    [b"SW1STARG", b"SW2STARG"],
    [b"SW1STON1", b"SW2STON1"],
    [b"SW1STON2", b"SW2STON2"],
    [b"SW1STONE", b"SW2STONE"],
    [b"SW1STRTN", b"SW2STRTN"],
];
/// Whether a sidedef texture name resolves to the same texture as `candidate`:
/// `R_CheckTextureNumForName` (r_data.c:696–710) maps a name starting with `-` to
/// texture 0 and otherwise compares with `strncasecmp(.., 8)` (ASCII case ignored,
/// stopping at the first NUL or after eight characters).
fn same_texture(name: &doom_types::LumpName, candidate: &[u8]) -> bool {
    let name = name.as_bytes();
    if name[0] == b'-' {
        return false;
    }
    for (k, &a) in name.iter().enumerate() {
        let b = candidate.get(k).copied().unwrap_or(0);
        if !a.eq_ignore_ascii_case(&b) {
            return false;
        }
        if a == 0 {
            return true;
        }
    }
    true
}
/// The `switchlist` slot `P_ChangeSwitchTexture` (p_switch.c:224–262) matches for a
/// front side's `[upper, lower, middle]` textures, restricted to the episode-1
/// prefix: slots in index order, each tested against the top, then the middle,
/// then the bottom texture. `Some((slot, part))` with `part` 0 top, 1 middle,
/// 2 bottom (`bwhere_e`). The first match is unique and the same in every game
/// mode, since the episode-1 slots precede every other.
pub(crate) fn switch_match(textures: &[doom_types::LumpName; 3]) -> Option<(usize, u8)> {
    let [upper, lower, middle] = textures;
    EPISODE1_SWITCHES
        .iter()
        .flatten()
        .enumerate()
        .find_map(|(slot, name)| {
            [(upper, 0), (middle, 1), (lower, 2)]
                .into_iter()
                .find(|(texture, _)| same_texture(texture, name))
                .map(|(_, part)| (slot, part))
        })
}

/// What the movers admission derived from the loaded map; immutable, never
/// canonical state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MoverAdmission {
    /// Every sector's family, `None` for a sector no admitted line moves.
    pub family: Vec<Option<Family>>,
    /// The number of mover sectors `M` (sectors with a family).
    pub movers: usize,
    /// The smallest and largest reachable height `R_min`, `R_max` (map units).
    pub reach: [i32; 2],
    /// The largest one-tic sink of a fitting thing, `Δ` (§5, map units).
    pub delta: i32,
}

/// The bounds the movers admission derives from a map (M3h8 note §5), public so
/// that external checks (the fuzz target) can evaluate the vertical invariant V′
/// (note §4.1) on their own: `reach` is `[R_min, R_max]` and `delta` is `Δ`, the
/// largest one-tic sink of a fitting thing, both in map units; `movers` is the
/// number of mover sectors `M`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoverBounds {
    /// The smallest and largest reachable floor or ceiling height.
    pub reach: [i32; 2],
    /// The largest one-tic sink of a fitting thing.
    pub delta: i32,
    /// The number of sectors an admitted line can move.
    pub movers: usize,
}
/// Runs the movers policy's line and sector load rules on `map` (the Stage A sector
/// special and light rules, then M3h8 note §7: the function
/// `Simulation::original_player_movers_map` runs after its geometry rules) and
/// returns the derived bounds, or the field-less error of the first failing rule.
/// Admission only; no simulation is built, and the geometry rules are not run.
pub fn mover_bounds(map: &Map) -> Result<MoverBounds, SimError> {
    let admission = validate_movers(map)?;
    Ok(MoverBounds {
        reach: admission.reach,
        delta: admission.delta,
        movers: admission.movers,
    })
}

/// `getNextSector` (p_spec.c:251–261) of `line` seen from `sector`: `None` without
/// `ML_TWOSIDED`; the back sector when `sector` is the front (itself for a line with
/// the sector on both sides), else the front.
pub(crate) fn next_sector(map: &Map, line: usize, sector: usize) -> Option<usize> {
    let line = &map.linedefs()[line];
    if line.flags & ML_TWOSIDED == 0 {
        return None;
    }
    let side = |i: usize| line.sides[i].map(|s| usize::from(map.sidedefs()[usize::from(s)].sector));
    if side(0) == Some(sector) {
        side(1)
    } else {
        side(0)
    }
}
/// The sectors `P_FindSectorFromLineTag` visits for `tag`, in index order (tag 0
/// selects every untagged sector, faithfully).
pub(crate) fn tagged(map: &Map, tag: i16) -> impl Iterator<Item = usize> + '_ {
    map.sectors()
        .iter()
        .enumerate()
        .filter(move |(_, s)| s.tag == tag)
        .map(|(i, _)| i)
}

/// The movers policy's load rules (M3h8 note §7), after the Stage A sector rules:
///
/// 1. every line special is 0, a one-sided 48 (a two-sided 48 stays rejected,
///    decision N1) or one of 1, 63, 109, 88, 36, 38, 11 — exactly E1M1's tag,
///    use and cross types, M3h8 note §4.0 — (`PursuitError::Specials`),
///    and every DR line has a back side ([`MapScaleError::DoorLine`]);
/// 2. no sector is in two mover families ([`MapScaleError::MoverFamily`]);
/// 3. every mover sector has a `getNextSector` neighbour
///    ([`MapScaleError::MoverNeighbor`]);
/// 4. the §5 reachable-height fixpoint, with the W1 rules run to a global fixpoint
///    like the repeatable ones (decision N7), stays within ±512
///    ([`MapScaleError::MoverHeights`]); no door's smallest possible top is below
///    its floor ([`MapScaleError::InvertingDoor`]); no door sector has a
///    self-referencing two-sided line ([`MapScaleError::SelfReferencingDoor`]);
/// 5. at most 30 plat sectors and 16 reusable switch lines
///    ([`MapScaleError::MoverCount`]), at most 64 scrollers (`PursuitError::Specials`,
///    Stage A);
/// 6. at most 8 distinct two-sided special lines in any 2 × 2 window of the map's
///    own BLOCKMAP cells ([`MapScaleError::SpecialWindow`]);
/// 7. no crushers (none is in rule 1's set);
/// 8. no mover sector has an `F_SKY1` ceiling ([`MapScaleError::SkyMover`],
///    decision N4);
/// 9. every switch line (11, 63) has an episode-1 switch texture on its front side
///    ([`MapScaleError::SwitchTexture`], decision N9a, [`switch_match`]).
pub(crate) fn validate_movers(map: &Map) -> Result<MoverAdmission, SimError> {
    use crate::sector_thinkers::{MAX_LINE_ANIMS, SCROLLING_WALL};
    crate::demon_expanded_map::validate_sector_specials(map)?;
    let sectors = map.sectors().len();
    let mut kinds = vec![Kinds::default(); sectors];
    let (mut scrollers, mut switches) = (0, 0);
    for line in map.linedefs() {
        let back = line.sides[1].map(|s| usize::from(map.sidedefs()[usize::from(s)].sector));
        // Rule 9: every switch line finds its front texture in `switchlist`.
        if matches!(line.special, LINE_EXIT | LINE_DOOR_SR) {
            let front = line.sides[0].ok_or(MapScaleError::SwitchTexture)?;
            let side = map
                .sidedefs()
                .get(usize::from(front))
                .ok_or(MapScaleError::SwitchTexture)?;
            if switch_match(&side.textures).is_none() {
                return Err(MapScaleError::SwitchTexture.into());
            }
        }
        match line.special {
            0 | LINE_EXIT => {}
            SCROLLING_WALL if back.is_none() => scrollers += 1,
            LINE_DOOR_DR => kinds[back.ok_or(MapScaleError::DoorLine)?].dr = true,
            special @ (LINE_DOOR_SR | LINE_DOOR_BLAZE | LINE_LIFT | LINE_FLOOR_TURBO
            | LINE_FLOOR_LOWEST) => {
                if special == LINE_DOOR_SR {
                    switches += 1;
                }
                for i in tagged(map, line.tag) {
                    let k = &mut kinds[i];
                    match special {
                        LINE_DOOR_SR => k.sr = true,
                        LINE_DOOR_BLAZE => k.blaze = true,
                        LINE_LIFT => k.lift = true,
                        LINE_FLOOR_TURBO => k.turbo = true,
                        _ => k.lowest = true,
                    }
                }
            }
            _ => return Err(PursuitError::Specials.into()),
        }
    }
    if scrollers > MAX_LINE_ANIMS {
        return Err(PursuitError::Specials.into());
    }
    crate::demon_expanded_map::validate_lights(map)?;
    let mut family = Vec::with_capacity(sectors);
    for k in &kinds {
        let mut f = k.families();
        let first = f.next();
        if f.next().is_some() {
            return Err(MapScaleError::MoverFamily.into());
        }
        family.push(first);
    }
    let lists = crate::player_map_scale::sector_lines(map);
    let neighbors = |i: usize| {
        lists[i]
            .iter()
            .filter_map(move |&l| next_sector(map, usize::from(l), i))
    };
    for (i, f) in family.iter().enumerate() {
        let Some(f) = f else { continue };
        if crate::demon_expanded_map::is_sky_flat(&map.sectors()[i].flats[1]) {
            return Err(MapScaleError::SkyMover.into());
        }
        if neighbors(i).next().is_none() {
            return Err(MapScaleError::MoverNeighbor.into());
        }
        if *f == Family::Door && neighbors(i).any(|n| n == i) {
            return Err(MapScaleError::SelfReferencingDoor.into());
        }
    }
    let plats = family.iter().filter(|f| **f == Some(Family::Plat)).count();
    if plats > MAX_PLATS || switches > MAX_BUTTONS {
        return Err(MapScaleError::MoverCount.into());
    }
    let (reach, delta) = reachable(map, &kinds, &neighbors)?;
    if special_window_lines(map) > MAX_SPECIAL_CROSS && !window_rule_bypassed() {
        return Err(MapScaleError::SpecialWindow.into());
    }
    Ok(MoverAdmission {
        movers: family.iter().filter(|f| f.is_some()).count(),
        family,
        reach,
        delta,
    })
}

// Set only by `with_window_rule_bypassed`.
#[cfg(test)]
thread_local! {
    static WINDOW_RULE_BYPASSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// Whether a test has bypassed rule 6 (never outside `cfg(test)`).
fn window_rule_bypassed() -> bool {
    #[cfg(test)]
    {
        WINDOW_RULE_BYPASSED.with(std::cell::Cell::get)
    }
    #[cfg(not(test))]
    {
        false
    }
}
/// Runs `f` with rule 6 bypassed on this thread: the only way to load the Rust-only
/// nine-line lane and reach the `SpecialCross` runtime witness (M3h8 note §6).
#[cfg(test)]
pub(crate) fn with_window_rule_bypassed<R>(f: impl FnOnce() -> R) -> R {
    WINDOW_RULE_BYPASSED.with(|b| b.set(true));
    let result = f();
    WINDOW_RULE_BYPASSED.with(|b| b.set(false));
    result
}

/// Widens `interval` to contain `[lo, hi]`; whether it changed.
fn widen(interval: &mut [i32; 2], lo: i32, hi: i32) -> bool {
    let before = *interval;
    interval[0] = interval[0].min(lo);
    interval[1] = interval[1].max(hi);
    *interval != before
}

/// The §5 least fixpoint of the floor and ceiling intervals of every sector under
/// the admitted rules, then rule 4b and `Δ`. Returns `[R_min, R_max]` and `Δ`.
fn reachable<'m, I: Iterator<Item = usize> + 'm>(
    map: &'m Map,
    kinds: &[Kinds],
    neighbors: &impl Fn(usize) -> I,
) -> Result<([i32; 2], i32), SimError> {
    let static_heights = |i: usize, plane: usize| i32::from(map.sectors()[i].heights[plane]);
    let n = map.sectors().len();
    let mut f: Vec<[i32; 2]> = (0..n).map(|i| [static_heights(i, 0); 2]).collect();
    let mut c: Vec<[i32; 2]> = (0..n).map(|i| [static_heights(i, 1); 2]).collect();
    let top = |c: &[[i32; 2]], i: usize| {
        neighbors(i).fold([i32::MAX, i32::MAX], |t, m| {
            [t[0].min(c[m][0] - 4), t[1].min(c[m][1] - 4)]
        })
    };
    let in_bounds = |v: &[[i32; 2]]| {
        v.iter()
            .all(|r| r[0] >= -REACHABLE_UNITS && r[1] <= REACHABLE_UNITS)
    };
    loop {
        let mut changed = false;
        for (i, k) in kinds.iter().enumerate() {
            if k.dr || k.sr {
                // A normal door rises to its top and closes to its floor.
                let t = top(&c, i);
                let lo = f[i][0].min(t[0]);
                changed |= widen(&mut c[i], lo, t[1]);
            }
            if k.blaze {
                let t = top(&c, i);
                changed |= widen(&mut c[i], t[0], t[1]);
            }
            if k.lift || k.lowest {
                // P_FindLowestFloorSurrounding starts at the sector's own floor.
                let lo = neighbors(i).fold(f[i][0], |lo, m| lo.min(f[m][0]));
                let hi = f[i][1];
                changed |= widen(&mut f[i], lo, hi);
            }
            if k.turbo {
                let constant =
                    neighbors(i).all(|m| f[m][0] == f[m][1] && f[m] == f[i]) && f[i][0] == f[i][1];
                if !constant {
                    let hi = neighbors(i).fold([i32::MIN, i32::MIN], |h, m| {
                        [h[0].max(f[m][0]), h[1].max(f[m][1])]
                    });
                    changed |= widen(&mut f[i], hi[0] + 8, hi[1] + 8);
                }
            }
        }
        if !in_bounds(&f) || !in_bounds(&c) {
            return Err(MapScaleError::MoverHeights.into());
        }
        if !changed {
            break;
        }
    }
    let mut delta = 0;
    for (i, k) in kinds.iter().enumerate() {
        if k.dr || k.sr {
            delta = delta.max(2);
        }
        if k.lift {
            delta = delta.max(4);
        }
        if k.door() {
            let t = top(&c, i);
            if t[0] < f[i][1] {
                return Err(MapScaleError::InvertingDoor.into());
            }
            delta = delta.max(c[i][1] - t[0]);
        }
        if k.turbo {
            let hi = neighbors(i).fold(i32::MIN, |h, m| h.max(f[m][1]));
            delta = delta.max(hi + 8 - f[i][0]);
        }
    }
    let lo = f.iter().chain(&c).map(|r| r[0]).min().unwrap_or(0);
    let hi = f.iter().chain(&c).map(|r| r[1]).max().unwrap_or(0);
    Ok(([lo, hi], delta))
}

/// Rule 6: the largest number of distinct lines with a back side and a nonzero
/// special listed in one 2 × 2 window of the map's own BLOCKMAP cells (a window at
/// the last row or column is clipped to the grid).
pub fn special_window_lines(map: &Map) -> usize {
    let blockmap = map.blockmap();
    let [width, height] = blockmap.dimensions().map(usize::from);
    let words = blockmap.words();
    let ranges = blockmap.cell_ranges();
    let lines = map.linedefs();
    let mut best = 0;
    let mut seen = vec![usize::MAX; lines.len()];
    for cy in 0..height.max(1) {
        for cx in 0..width.max(1) {
            let stamp = cy * width + cx;
            let mut distinct = 0;
            for wy in [cy, cy + 1] {
                for wx in [cx, cx + 1] {
                    if wx >= width || wy >= height {
                        continue;
                    }
                    for &word in &words[ranges[wy * width + wx].clone()] {
                        let line = usize::from(word);
                        if line < lines.len()
                            && seen[line] != stamp
                            && lines[line].sides[1].is_some()
                            && lines[line].special != 0
                        {
                            seen[line] = stamp;
                            distinct += 1;
                        }
                    }
                }
            }
            best = best.max(distinct);
        }
    }
    best
}
