//! Real-map-scale admission for `PlayerMonstersMapScaleV1`: the walled vertex
//! rectangle derived from the loaded map, the budgets derived from fireball flight
//! time and attack cadence, and the policy-aware coordinate checks that replace the
//! historical 978/994/1,008-unit margins and the `160M/3` travel envelope for this
//! policy only. See `docs/M3H3_REFERENCE_INVESTIGATION.md` for the proof.
use crate::*;
use std::{error::Error, fmt};

/// Actor slots of the map-scale policy: `1 + 8 + 16 + 26k + (8-k) + 8(8-k) + 8 <= 241`
/// live actors for `k` Imps, plus at most `39 - 3k` spawns in one pass, is at most
/// 256 visits, so the store never fills and the visit budget keeps a 64-visit margin.
pub const MAP_SCALE_ACTOR_SLOTS: u32 = 320;
/// Actor slots of the barrels policy (`PlayerMonstersBarrelsV1`, M3h9b note §5.3,
/// decision F2): with `K` 8 monsters, `P` 64 pickups, `R` 8 drops, `D` 32
/// decorations and `B` 16 barrels, `1 + K + P + 26k + (K−k) + 8(K−k) + R + D + B`
/// is at most 337 live actors (at `k = 8` Imps; 201 at `k = 0`), and even the crude
/// bound 337 + 39 spawns in one pass is 376, within 384, so the store never fills.
/// Encoded as `store.max_slots` in DSIM37; every older policy keeps 320.
pub const BARRELS_ACTOR_SLOTS: u32 = 384;
/// Actor slots of the Episode 1 policy (`PlayerMonstersEpisode1V1`, M4a decision
/// H4 and plan decision D1): the store's hard cap, a constant so that
/// `store.max_slots` is one value in DSIM39. No live-actor proof backs it: every
/// spawn site reserves its slots before mutating, and a spawn that finds the store
/// full refuses the tic with `StoreError::Capacity` (the store-full witness).
pub const EPISODE1_ACTOR_SLOTS: u32 = 1024;
/// The actor slot count a map-scale policy requires of its constructor and encodes
/// as `store.max_slots`: [`EPISODE1_ACTOR_SLOTS`] under the Episode 1 policy,
/// [`BARRELS_ACTOR_SLOTS`] under the barrels and level policies,
/// [`MAP_SCALE_ACTOR_SLOTS`] under every other map-scale policy.
pub const fn actor_slots(policy: OriginalExecutionProfile) -> u32 {
    if policy.has_episode1() {
        EPISODE1_ACTOR_SLOTS
    } else if policy.has_barrels() {
        BARRELS_ACTOR_SLOTS
    } else {
        MAP_SCALE_ACTOR_SLOTS
    }
}
/// Live fireballs: one Imp launches at most every 23 tics (its attack state cut
/// short by pain, then the JUSTHIT shortcut of `P_CheckMissileRange`) and a fireball
/// holds its slot for at most `ceil(5776/10) + 20 = 598` passes in the largest room,
/// so `ceil(598/23) = 26` per Imp and 208 for eight. Unreachable in play; kept as
/// the schema bound and launch witness.
pub const MAP_SCALE_MAX_LIVE_PROJECTILES: usize = 208;
/// Linedefs plus the at most 33 BLOCKMAP-linked things never exceed the source
/// `MAXINTERCEPTS` (128) in one trace.
pub const MAP_SCALE_WALL_LINES: usize = 95;
/// XY momentum bound per component. In-world impulses add at most about 650 units
/// in one pass and `P_XYMovement` clamps to 30 every tic; `2^28` raw leaves two
/// orders of magnitude before any checked addition could overflow.
pub const MAP_SCALE_MOMENTUM_UNITS: u32 = 4096;
/// Absolute vertex, BLOCKMAP-origin and actor coordinate bound: the M3f2 intercept
/// proof with `G = A = 2048` and line norm 1,536.
pub const MAP_SCALE_EXTENT_UNITS: u32 = map_extent::QUERY_UNITS;
/// Vertical momentum witness of `PlayerMonstersHeightsV1`. Sector heights are within
/// ±512 and actor heights within 56, so a fireball chord `(dest.z - source.z) /
/// time` with `time >= 1` is within 968 and gravity reaches −46 in the longest
/// fall; older policies keep the historical 64. Rust-only, never source behavior.
pub const HEIGHTS_MOMZ_UNITS: u32 = 1024;
/// Absolute z witness of the heights policy: the sight proof's margin.
pub const HEIGHTS_Z_UNITS: u32 = MAP_SCALE_EXTENT_UNITS;
/// Linedef flag bits admitted under the heights policy besides `ML_TWOSIDED`:
/// `ML_BLOCKING`, `ML_BLOCKMONSTERS` and `ML_SOUNDBLOCK` are read by the admitted
/// bodies (p_map.c PIT_CheckLine, p_enemy.c P_RecursiveSound); `ML_DONTPEGTOP`,
/// `ML_DONTPEGBOTTOM`, `ML_SECRET`, `ML_DONTDRAW` and `ML_MAPPED` are renderer and
/// automap data with no reader in this project (doomdata.h).
pub const HEIGHTS_LINE_FLAGS: u16 = 1 | 2 | 8 | 16 | 32 | 64 | 128 | 256;
pub(crate) const ML_TWOSIDED: u16 = 4;
/// sounds.h `sfx_oof`: the landing sound `P_ZMovement` requests for a player whose
/// `momz` is below `-GRAVITY*8` on floor contact. A sound request only.
pub(crate) const SFX_OOF: u16 = 34;
/// p_mobj.c P_ZMovement: `if (mo->player && mo->momz < -GRAVITY*8)`.
pub(crate) const LANDING_SOUND_MOMZ: i16 = -8;
const MAX_LINKED_THINGS: usize = 1 + MAX_PLAYER_MONSTERS + MAX_WORLD_PICKUPS + MAX_PLAYER_MONSTERS;

/// Map geometry or actor state outside the proven real-map-scale domain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapScaleError {
    /// A side of the vertex rectangle is not completely covered by one-sided walls.
    Walls,
    /// The BLOCKMAP does not cover the rectangle or omits a wall from a cell it touches.
    Blockmap,
    /// More linedefs than `MAP_SCALE_WALL_LINES`, or, under the real-map policy,
    /// more than `MAX_INTERCEPTS` distinct lines in one 2 × 2 window of BLOCKMAP
    /// cells (the slide-window certificate, M3h6 note §4.4).
    Lines,
    /// An actor center outside the rectangle before a tic (unreachable by the proof).
    Position,
    /// An XY momentum component beyond `MAP_SCALE_MOMENTUM_UNITS`.
    Momentum,
    /// A linedef that is not axis-aligned, has no front side, carries a flag bit
    /// outside `HEIGHTS_LINE_FLAGS`, or whose back side disagrees with `ML_TWOSIDED`.
    Geometry,
    /// A spawn whose type is taller than its spawn sector (a broken map).
    Fit,
    /// An actor outside `[floor, ceiling - height]` or beyond the z/momz witnesses
    /// before a tic (unreachable by the vertical invariant).
    Vertical,
    /// Under the diagonal policy, the one-sided walls do not form a closed boundary
    /// facing its room: two walls meet elsewhere than at a shared endpoint, a vertex
    /// has odd degree, or a wall's front side faces even parity (M3h5 note §4.1).
    Boundary,
    /// Under the diagonal policy, a player's projected slide move was accepted by
    /// `P_TryMove` with its center outside the room (the corner jump, M3h5 note
    /// §4.4): the one legal source move the policy refuses for the player. Under
    /// the barrels policy also any accepted `P_TryMove` step of a barrel whose
    /// center leaves the room: a barrel's radius is 10, and a negative
    /// `P_XYMovement` step reaches 30 units per axis, more than its box width of
    /// 20, so it can clear a wall corner the destination-only test does not see
    /// (`barrels::BarrelMotion`). The player (box 32) and the admitted monsters
    /// (box 40) cannot, so only barrels pay that check. The tic fails and the
    /// state before it is kept.
    Escape,
    /// Under the real-map policy, the arithmetic certificate of the M3h6 note §4.2
    /// fails for the loaded map: a line norm above 3,853, a vertex rectangle whose
    /// diagonal times the longest line exceeds the sight-intercept bound, spans
    /// whose sum reaches 32,768 or whose maximum reaches 8,192, a BLOCKMAP that
    /// does not cover the rectangle, a node partition the side tests cannot
    /// evaluate exactly, heights beyond ±512, or a rectangle within 2,177 units
    /// of the format's edge (where a trace endpoint or a BLOCKMAP index would
    /// leave the `short` range). `ArithmeticCertificate::evaluate` names the
    /// clause.
    Arithmetic,
    /// Under the specials policy, a sector light level outside the range where
    /// the `p_lights.c` arithmetic stays inside `short lightlevel`
    /// (`i16::MIN + 64 ..= i16::MAX - 64`), refused at load.
    Light,
    /// Under the real-map policy, a trace of this tic would write the source's
    /// 129th intercept (`MAXINTERCEPTS`, p_local.h), which the original writes past
    /// `intercepts[]` (undefined): the tic is refused and the state before it is
    /// kept. Reached only by construction (M3h6 note §4.4); never observed in C.
    Intercepts,
    /// Under the movers policy, a DR door line (special 1) without a back side,
    /// whose `EV_VerticalDoor` would read `sides[-1]` (M3h8 note §7 rule 1).
    DoorLine,
    /// Under the movers policy, a sector in two mover families (door, plat, floor),
    /// whose `specialdata` cast could read a different thinker type (rule 2).
    MoverFamily,
    /// Under the movers policy, a mover sector without a `getNextSector`
    /// neighbour (a `MAXINT − 4` door top or a −500 turbo destination, rule 3).
    MoverNeighbor,
    /// Under the movers policy, a reachable floor or ceiling height beyond ±512
    /// (the §5 fixpoint, rule 4).
    MoverHeights,
    /// Under the movers policy, a door whose smallest possible top lies below its
    /// floor (rule 4b).
    InvertingDoor,
    /// Under the movers policy, a door sector with a two-sided line that has the
    /// sector on both sides (rule 4c).
    SelfReferencingDoor,
    /// Under the movers policy, more than 30 plat sectors (`MAXPLATS`) or more than
    /// 16 reusable switch lines (`MAXBUTTONS`, rule 5).
    MoverCount,
    /// Under the movers policy, more than 8 distinct two-sided special lines in
    /// one 2 × 2 window of BLOCKMAP cells (`MAXSPECIALCROSS`, rule 6).
    SpecialWindow,
    /// Under the movers policy, a mover sector with an `F_SKY1` ceiling (rule 8,
    /// decision N4).
    SkyMover,
    /// Under the movers policy, a `P_CheckPosition` of this tic would store a ninth
    /// `spechit` entry (`MAXSPECIALCROSS`), which the original writes past the
    /// array (undefined): the tic is refused and the state before it is kept.
    /// Unreachable on admitted maps (rule 6).
    SpecialCross,
    /// Under the movers policy, a switch line (special 11 or 63) whose front side
    /// carries no episode-1 switch texture in its upper, lower or middle texture, so
    /// `P_ChangeSwitchTexture` would find nothing in `switchlist` (no texture
    /// change, no sound, no button; M3h8 note §7 rule 9, decision N9a).
    SwitchTexture,
    /// Under the movers policy, a mover sector whose `P_GroupLines` `blockbox`
    /// arithmetic would overflow `fixed_t` (the box is undefined in the source, so
    /// `P_ChangeSector` could visit no defined cells; M3h8 note §7 rule 10).
    MoverBlockBox,
    /// Under the Episode 1 policy, a D1 door line (31–34) used on a sector whose
    /// `specialdata` already holds a live mover: `EV_VerticalDoor` falls through
    /// and spawns a second thinker over the busy sector, overwriting
    /// `specialdata` (p_doors.c:416–461). The tic is refused and the state before
    /// it is kept (M4a decision H3, plan decision D1).
    SecondDoor,
    /// Under the Episode 1 policy, a stair step's `T_MovePlane` found the first
    /// `P_ChangeSector` blocked: the source then reads the uninitialised `crush` of
    /// an `EV_BuildStairs` thinker (p_floor.c:113–119, 516–528), so the tic is
    /// refused and the state before it is kept (decision H7).
    StairBlocked,
    /// Under the Episode 1 policy, a `raiseToNearestAndChange` plat rising into a
    /// thing (decision H7): its zero-projected fields keep their meaning only
    /// while it moves freely, so the tic is refused and the state before it is
    /// kept.
    PlatBlocked,
    /// Under the Episode 1 policy, an `EV_DoDonut` whose outer sector `s2` already
    /// has a live mover (`specialdata`): the source spawns a second floor over it
    /// and overwrites the pointer (p_spec.c:1163–1221). The tic is refused and the
    /// state before it is kept (decision H8).
    DonutBusy,
    /// Under the Episode 1 policy, a sector tagged by a donut line (special 9)
    /// whose topology makes `EV_DoDonut` dereference a null side or sector
    /// (p_spec.c:1184–1190), refused at load (decision H8,
    /// `mover_rules::donut_geometry`).
    DonutGeometry,
}
impl fmt::Display for MapScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "map scale: {self:?}")
    }
}
impl Error for MapScaleError {}
impl From<MapScaleError> for SimError {
    fn from(value: MapScaleError) -> Self {
        Self::MapScale(value)
    }
}

/// The closed vertex rectangle of an admitted walled room, in raw fixed point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoomBounds {
    /// Minimum x and y over all VERTEXES.
    pub min: [Fixed; 2],
    /// Maximum x and y over all VERTEXES.
    pub max: [Fixed; 2],
}
impl RoomBounds {
    /// Closed containment of an actor center.
    pub fn contains(&self, position: [Fixed; 2]) -> bool {
        (0..2).all(|axis| position[axis] >= self.min[axis] && position[axis] <= self.max[axis])
    }
    /// The absolute coordinate envelope of the rectangle in map units (at least the
    /// M3f2 query envelope): the bound every live actor center is checked against
    /// by the sight and perception reads under the real-map policy.
    pub fn envelope_units(&self) -> u32 {
        let raw = self
            .min
            .iter()
            .chain(self.max.iter())
            .map(|v| v.bits().unsigned_abs() >> 16)
            .max()
            .unwrap_or(0);
        raw.max(MAP_SCALE_EXTENT_UNITS)
    }
    /// Closed containment of a whole bounding box: the spawn condition under which
    /// every later `P_TryMove` keeps the open box inside the open rectangle.
    /// The box edges are formed in i64, so a coordinate near `i32::MIN` cannot wrap
    /// into the rectangle.
    pub fn contains_box(&self, position: [Fixed; 2], radius: Fixed) -> bool {
        (0..2).all(|axis| {
            let center = i64::from(position[axis].bits());
            let radius = i64::from(radius.bits());
            center - radius >= i64::from(self.min[axis].bits())
                && center + radius <= i64::from(self.max[axis].bits())
        })
    }
    /// Derive and check the rectangle of a grounded single-sector map: four fully
    /// walled sides, a BLOCKMAP that covers the rectangle and lists every wall in
    /// every cell its bbox touches, and at most `MAP_SCALE_WALL_LINES` linedefs. The
    /// caller has already required one-sided axis-aligned blocking walls and the
    /// 2,048-unit vertex envelope. Admission only; nothing here runs per tic.
    pub fn derive(map: &Map) -> Result<Self, SimError> {
        Self::derive_for(map, false)
    }
    /// `derive`, counting only one-sided linedefs toward the boundary covering when
    /// `one_sided_only` is set: `PIT_CheckLine` blocks a one-sided line regardless
    /// of flags (`if (!ld->backsector) return false`, p_map.c), which is what the
    /// wall invariant of the heights policy relies on. The BLOCKMAP must still list
    /// every linedef, one- or two-sided, in every cell its bbox touches.
    pub fn derive_for(map: &Map, one_sided_only: bool) -> Result<Self, SimError> {
        if map.linedefs().len() > MAP_SCALE_WALL_LINES {
            return Err(MapScaleError::Lines.into());
        }
        let vertices = map.vertices();
        let (Some(first), Some(_)) = (vertices.first(), map.linedefs().first()) else {
            return Err(MapScaleError::Walls.into());
        };
        let mut min = [i32::from(first.x), i32::from(first.y)];
        let mut max = min;
        for v in vertices {
            for (axis, value) in [v.x, v.y].into_iter().enumerate() {
                min[axis] = min[axis].min(i32::from(value));
                max[axis] = max[axis].max(i32::from(value));
            }
        }
        // Each side of the rectangle must be covered by the union of the wall
        // segments lying on it (closed integer intervals along the other axis).
        for axis in 0..2 {
            let other = 1 - axis;
            for edge in [min[axis], max[axis]] {
                let mut intervals: Vec<[i32; 2]> = map
                    .linedefs()
                    .iter()
                    .filter(|line| !one_sided_only || line.sides[1].is_none())
                    .filter_map(|line| {
                        let [a, b] = line.vertices.map(|i| vertices[usize::from(i)]);
                        let (pa, pb) = (
                            [i32::from(a.x), i32::from(a.y)],
                            [i32::from(b.x), i32::from(b.y)],
                        );
                        (pa[axis] == edge && pb[axis] == edge)
                            .then(|| [pa[other].min(pb[other]), pa[other].max(pb[other])])
                    })
                    .collect();
                intervals.sort_unstable();
                let mut covered = min[other];
                for [start, end] in intervals {
                    if start > covered {
                        return Err(MapScaleError::Walls.into());
                    }
                    covered = covered.max(end);
                }
                if covered < max[other] {
                    return Err(MapScaleError::Walls.into());
                }
            }
        }
        let blockmap = map.blockmap();
        let origin = [
            i32::from(blockmap.origin().x),
            i32::from(blockmap.origin().y),
        ];
        let [width, height] = blockmap.dimensions().map(i32::from);
        if origin[0] > min[0]
            || origin[1] > min[1]
            || max[0] >= origin[0] + 128 * width
            || max[1] >= origin[1] + 128 * height
        {
            return Err(MapScaleError::Blockmap.into());
        }
        let words = blockmap.words();
        for (index, line) in map.linedefs().iter().enumerate() {
            let [a, b] = line.vertices.map(|i| vertices[usize::from(i)]);
            let xl = (i32::from(a.x.min(b.x)) - origin[0]) >> 7;
            let xh = (i32::from(a.x.max(b.x)) - origin[0]) >> 7;
            let yl = (i32::from(a.y.min(b.y)) - origin[1]) >> 7;
            let yh = (i32::from(a.y.max(b.y)) - origin[1]) >> 7;
            for x in xl..=xh {
                for y in yl..=yh {
                    let range = blockmap.cell_ranges()[(y * width + x) as usize].clone();
                    if !words[range].contains(&(index as u16)) {
                        return Err(MapScaleError::Blockmap.into());
                    }
                }
            }
        }
        Ok(Self {
            min: min.map(|v| Fixed::from_map_units(v as i16)),
            max: max.map(|v| Fixed::from_map_units(v as i16)),
        })
    }
    /// The vertex rectangle of a diagonal-policy room: at most
    /// `MAP_SCALE_WALL_LINES` linedefs, a BLOCKMAP whose grid covers the rectangle
    /// and lists every linedef, one- or two-sided, in every cell whose closed
    /// 128-unit square meets the closed segment (the rule real node builders
    /// follow, and what the M3h5 proof needs: the cell holding any wall point lists
    /// that wall). The boundary itself is checked by `Boundary::derive`.
    pub fn derive_diagonal(map: &Map) -> Result<Self, SimError> {
        if map.linedefs().len() > MAP_SCALE_WALL_LINES {
            return Err(MapScaleError::Lines.into());
        }
        Self::derive_closed_cell(map)
    }
    /// The vertex rectangle of a real-map-policy room: `derive_diagonal` without the
    /// 95-line bound, whose `MAXINTERCEPTS` role moves to the runtime witness
    /// `MapScaleError::Intercepts` (M3h6 note §4.4). The line count itself is
    /// bounded by the WAD format (`short` references).
    pub fn derive_real_map(map: &Map) -> Result<Self, SimError> {
        Self::derive_closed_cell(map)
    }
    fn derive_closed_cell(map: &Map) -> Result<Self, SimError> {
        let vertices = map.vertices();
        let (Some(first), Some(_)) = (vertices.first(), map.linedefs().first()) else {
            return Err(MapScaleError::Walls.into());
        };
        let mut min = [i32::from(first.x), i32::from(first.y)];
        let mut max = min;
        for v in vertices {
            for (axis, value) in [v.x, v.y].into_iter().enumerate() {
                min[axis] = min[axis].min(i32::from(value));
                max[axis] = max[axis].max(i32::from(value));
            }
        }
        let blockmap = map.blockmap();
        let origin = [
            i32::from(blockmap.origin().x),
            i32::from(blockmap.origin().y),
        ];
        let [width, height] = blockmap.dimensions().map(i32::from);
        if origin[0] > min[0]
            || origin[1] > min[1]
            || max[0] >= origin[0] + 128 * width
            || max[1] >= origin[1] + 128 * height
        {
            return Err(MapScaleError::Blockmap.into());
        }
        let words = blockmap.words();
        for (index, line) in map.linedefs().iter().enumerate() {
            let [a, b] = line.vertices.map(|i| vertices[usize::from(i)]);
            let segment = [
                [i64::from(a.x), i64::from(a.y)],
                [i64::from(b.x), i64::from(b.y)],
            ];
            let xl = (i32::from(a.x.min(b.x)) - origin[0]) >> 7;
            let xh = (i32::from(a.x.max(b.x)) - origin[0]) >> 7;
            let yl = (i32::from(a.y.min(b.y)) - origin[1]) >> 7;
            let yh = (i32::from(a.y.max(b.y)) - origin[1]) >> 7;
            for x in xl..=xh {
                for y in yl..=yh {
                    let cell = [
                        i64::from(origin[0] + 128 * x),
                        i64::from(origin[1] + 128 * y),
                    ];
                    if !segment_meets_box(segment, cell, [cell[0] + 128, cell[1] + 128]) {
                        continue;
                    }
                    let range = blockmap.cell_ranges()[(y * width + x) as usize].clone();
                    if !words[range].contains(&(index as u16)) {
                        return Err(MapScaleError::Blockmap.into());
                    }
                }
            }
        }
        Ok(Self {
            min: min.map(|v| Fixed::from_map_units(v as i16)),
            max: max.map(|v| Fixed::from_map_units(v as i16)),
        })
    }
}

/// Twice the signed area of `(a, b, p)`: negative when `p` is right of the directed
/// line `a → b` (the source front side), zero on it.
fn orient(a: [i64; 2], b: [i64; 2], p: [i64; 2]) -> i64 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}
/// Whether the closed segment meets the closed box `[lo, hi]`: the bounding boxes
/// overlap and the segment's line does not leave all four corners strictly on one
/// side. Exact in integers (the chord of the line inside the box and the segment are
/// intervals of one line whose projections overlap exactly when the boxes do).
fn segment_meets_box(segment: [[i64; 2]; 2], lo: [i64; 2], hi: [i64; 2]) -> bool {
    let [a, b] = segment;
    if a[0].max(b[0]) < lo[0]
        || a[0].min(b[0]) > hi[0]
        || a[1].max(b[1]) < lo[1]
        || a[1].min(b[1]) > hi[1]
    {
        return false;
    }
    let corners = [lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]].map(|p| orient(a, b, p));
    !(corners.iter().all(|c| *c > 0) || corners.iter().all(|c| *c < 0))
}
/// Whether the closed segment meets the open box `(lo, hi)`: the bounding boxes
/// overlap strictly and the segment's line leaves corners strictly on both sides.
/// Exact in integers, by the argument of [`segment_meets_box`] with open intervals
/// (three pairwise-meeting intervals of one line share a point); a segment that
/// only touches the box's boundary, at an edge, a corner or along a side, does not
/// meet it.
fn segment_meets_open_box(segment: [[i64; 2]; 2], lo: [i64; 2], hi: [i64; 2]) -> bool {
    let [a, b] = segment;
    if a[0].max(b[0]) <= lo[0]
        || a[0].min(b[0]) >= hi[0]
        || a[1].max(b[1]) <= lo[1]
        || a[1].min(b[1]) >= hi[1]
    {
        return false;
    }
    let corners = [lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]].map(|p| orient(a, b, p));
    corners.iter().any(|c| *c > 0) && corners.iter().any(|c| *c < 0)
}
/// Whether the closed segments `ab` and `cd` share a point.
fn segments_meet(a: [i64; 2], b: [i64; 2], c: [i64; 2], d: [i64; 2]) -> bool {
    let on = |p: [i64; 2], q: [i64; 2], r: [i64; 2]| {
        orient(p, q, r) == 0
            && r[0] >= p[0].min(q[0])
            && r[0] <= p[0].max(q[0])
            && r[1] >= p[1].min(q[1])
            && r[1] <= p[1].max(q[1])
    };
    let (d1, d2, d3, d4) = (
        orient(c, d, a),
        orient(c, d, b),
        orient(a, b, c),
        orient(a, b, d),
    );
    ((d1 > 0) != (d2 > 0) && (d1 < 0) != (d2 < 0) && (d3 > 0) != (d4 > 0) && (d3 < 0) != (d4 < 0))
        || on(c, d, a)
        || on(c, d, b)
        || on(a, b, c)
        || on(a, b, d)
}
/// Crossings of the horizontal ray `{(x, p.y) : x > p.x}` with the closed segment
/// under the half-open rule, which is exact for a point not on the segment. With
/// `from_below`, a vertex exactly at the ray's height counts as above it (the limit
/// of a ray just below `p`); without it, as below (the limit from above).
fn ray_crosses(p: [i64; 2], a: [i64; 2], b: [i64; 2], from_below: bool) -> bool {
    let above = |v: [i64; 2]| v[1] > p[1] || (from_below && v[1] == p[1]);
    if above(a) == above(b) {
        return false;
    }
    // The crossing abscissa exceeds p.x exactly when num / den > 0.
    let num = (p[1] - a[1]) * (b[0] - a[0]) - (p[0] - a[0]) * (b[1] - a[1]);
    let den = b[1] - a[1];
    (num > 0) == (den > 0)
}

/// The one-sided walls of a diagonal-policy room as closed integer segments, with the
/// exact ray-parity test that defines the admitted region `Ω` (M3h5 note §4.1–4.3).
/// Derived from the immutable map at load; never canonical state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Boundary {
    /// Endpoints of every one-sided linedef in map units, in linedef order.
    segments: Vec<[[i64; 2]; 2]>,
}
impl Boundary {
    /// Collect the one-sided linedefs and check the closed-boundary rules: C1, two
    /// walls meet only at a shared endpoint; C2, every vertex ends an even number of
    /// walls; C3, the front (sidedef 0) side of every wall has odd parity. Admission
    /// only; at most `MAP_SCALE_WALL_LINES²` integer products, nothing per tic.
    pub fn derive(map: &Map) -> Result<Self, SimError> {
        let vertices = map.vertices();
        let mut segments = Vec::new();
        for line in map.linedefs() {
            if line.sides[1].is_some() {
                continue;
            }
            let [a, b] = line.vertices.map(usize::from);
            let from = vertices.get(a).ok_or(MapScaleError::Geometry)?;
            let to = vertices.get(b).ok_or(MapScaleError::Geometry)?;
            segments.push([
                [i64::from(from.x), i64::from(from.y)],
                [i64::from(to.x), i64::from(to.y)],
            ]);
        }
        if segments.is_empty() {
            return Err(MapScaleError::Walls.into());
        }
        for (i, &[a, b]) in segments.iter().enumerate() {
            if a == b {
                return Err(MapScaleError::Geometry.into());
            }
            for &[c, d] in &segments[i + 1..] {
                let shared: Vec<[i64; 2]> =
                    [a, b].into_iter().filter(|p| *p == c || *p == d).collect();
                match shared.as_slice() {
                    [] => {
                        if segments_meet(a, b, c, d) {
                            return Err(MapScaleError::Boundary.into());
                        }
                    }
                    [p] => {
                        // Collinear walls leaving the shared endpoint the same way
                        // overlap; anything else meets only at the endpoint.
                        let q1 = if a == *p { b } else { a };
                        let q2 = if c == *p { d } else { c };
                        let dot = (q1[0] - p[0]) * (q2[0] - p[0]) + (q1[1] - p[1]) * (q2[1] - p[1]);
                        if orient(*p, q1, q2) == 0 && dot > 0 {
                            return Err(MapScaleError::Boundary.into());
                        }
                    }
                    _ => return Err(MapScaleError::Boundary.into()),
                }
            }
        }
        let mut degree: Vec<([i64; 2], u32)> = Vec::new();
        for &[a, b] in &segments {
            for p in [a, b] {
                match degree.iter_mut().find(|(v, _)| *v == p) {
                    Some((_, n)) => *n += 1,
                    None => degree.push((p, 1)),
                }
            }
        }
        if degree.iter().any(|(_, n)| n % 2 != 0) {
            return Err(MapScaleError::Boundary.into());
        }
        let boundary = Self { segments };
        // C3 in doubled coordinates so the midpoint is integral: the parity of a
        // point just in front of the midpoint is the ray count over the other walls
        // taken as the limit toward the front side, plus the wall itself when the
        // ray from the front side crosses it (`dy < 0`).
        for (i, &[a, b]) in boundary.segments.iter().enumerate() {
            let m = [a[0] + b[0], a[1] + b[1]];
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let from_below = dx > 0;
            let mut odd = dy < 0;
            for (j, &[c, d]) in boundary.segments.iter().enumerate() {
                if j != i && ray_crosses(m, [2 * c[0], 2 * c[1]], [2 * d[0], 2 * d[1]], from_below)
                {
                    odd = !odd;
                }
            }
            if !odd {
                return Err(MapScaleError::Boundary.into());
            }
        }
        Ok(boundary)
    }
    /// The walls in map units.
    pub fn segments(&self) -> &[[[i64; 2]; 2]] {
        &self.segments
    }
    /// Ray parity of a raw fixed-point center: true when odd, i.e. inside `Ω`.
    /// Exact for a center not on a wall; a center exactly on a wall gets whatever
    /// the half-open rule yields for that wall, which no caller relies on (a box
    /// containing its center meets the wall and is refused separately).
    pub fn inside(&self, position: [Fixed; 2]) -> bool {
        let p = position.map(|v| i64::from(v.bits()));
        self.segments
            .iter()
            .filter(|[a, b]| ray_crosses(p, raw(*a), raw(*b), false))
            .count()
            % 2
            == 1
    }
    /// Whether the closed box `[x − r, x + r] × [y − r, y + r]` (raw units, formed
    /// in i64) meets any wall.
    pub fn box_meets_wall(&self, position: [Fixed; 2], radius: Fixed) -> bool {
        let p = position.map(|v| i64::from(v.bits()));
        let r = i64::from(radius.bits());
        self.segments.iter().any(|[a, b]| {
            segment_meets_box(
                [raw(*a), raw(*b)],
                [p[0] - r, p[1] - r],
                [p[0] + r, p[1] + r],
            )
        })
    }
    /// The spawn condition of the diagonal policy (M3h5 note §4.1 item 4): center
    /// inside `Ω` and the closed box meeting no wall, so the open box lies in `Ω`.
    pub fn spawn_admitted(&self, position: [Fixed; 2], radius: Fixed) -> bool {
        self.inside(position) && !self.box_meets_wall(position, radius)
    }
    /// The spawn rule S′ of the barrels policy (M3h9b note decision F3): the center
    /// inside `Ω` (odd ray parity); a static (pickup, decoration) needs nothing more,
    /// since the M3h5 invariant asks only its center; a mover (monster, player body)
    /// additionally needs, for every wall `W = (a, b)` its OPEN box
    /// `(x − r, x + r) × (y − r, y + r)` meets, its center in front of `W`'s line by
    /// at least `(31 − r)·w(θ)`, in exact integers
    /// `|dx·(cy − ay) − dy·(cx − ax)| ≥ (31 − r)·(|dx| + |dy|)` with
    /// `(dx, dy) = b − a`. A wall exactly on the closed box's edge imposes nothing.
    /// Both sides are formed on raw fixed-point values in i128 (each is 2³² times its
    /// map-unit value, so the comparison is the map-unit one, exact for fractional
    /// centers too). The vertex-rectangle clause is the caller's.
    pub fn spawn_admitted_relaxed(&self, position: [Fixed; 2], radius: Fixed, mover: bool) -> bool {
        if !self.inside(position) {
            return false;
        }
        if !mover {
            return true;
        }
        let p = position.map(|v| i64::from(v.bits()));
        let r = i64::from(radius.bits());
        let (lo, hi) = ([p[0] - r, p[1] - r], [p[0] + r, p[1] + r]);
        let reach = i128::from((31_i64 << 16) - r);
        self.segments.iter().all(|[a, b]| {
            let (a, b) = (raw(*a), raw(*b));
            if !segment_meets_open_box([a, b], lo, hi) {
                return true;
            }
            let (dx, dy) = (i128::from(b[0] - a[0]), i128::from(b[1] - a[1]));
            let cross = dx * i128::from(p[1] - a[1]) - dy * i128::from(p[0] - a[0]);
            cross.abs() >= reach * (dx.abs() + dy.abs())
        })
    }
}
/// Map units to raw fixed point, in i64.
fn raw(p: [i64; 2]) -> [i64; 2] {
    p.map(|v| v << 16)
}
/// Linedef admission of the diagonal policy: as `validate_heights_geometry` without
/// the axis rule (every `P_HitSlideLine` branch is admitted; the projected move's
/// witness is `MapScaleError::Escape`). Lines may have any direction; degenerate
/// lines reject. The line norm, heights, specials, sky and the vertex envelope are
/// checked by the shared expanded-map admission; the boundary by `Boundary::derive`.
pub(crate) fn validate_diagonal_geometry(map: &Map) -> Result<(), SimError> {
    for line in map.linedefs() {
        let [a, b] = line.vertices.map(usize::from);
        let from = map.vertices().get(a).ok_or(MapScaleError::Geometry)?;
        let to = map.vertices().get(b).ok_or(MapScaleError::Geometry)?;
        let two_sided = line.flags & ML_TWOSIDED != 0;
        if line.sides[0].is_none()
            || line.sides[1].is_some() != two_sided
            || line.flags & !(HEIGHTS_LINE_FLAGS | ML_TWOSIDED) != 0
            || (from.x == to.x && from.y == to.y)
        {
            return Err(MapScaleError::Geometry.into());
        }
    }
    Ok(())
}

/// Linedef admission of the heights policy, checked once before construction:
/// every linedef is axis-aligned and not degenerate (the `P_HitSlideLine` axis
/// cases are the only proven slide projections), has a front side, carries
/// `ML_TWOSIDED` exactly when it has a back side, and no flag bit outside
/// `HEIGHTS_LINE_FLAGS`. Sector heights, specials, sky ceilings and the vertex
/// envelope are checked by the shared expanded-map admission.
pub(crate) fn validate_heights_geometry(map: &Map) -> Result<(), SimError> {
    for line in map.linedefs() {
        let [a, b] = line.vertices.map(usize::from);
        let from = map.vertices().get(a).ok_or(MapScaleError::Geometry)?;
        let to = map.vertices().get(b).ok_or(MapScaleError::Geometry)?;
        let two_sided = line.flags & ML_TWOSIDED != 0;
        if line.sides[0].is_none()
            || line.sides[1].is_some() != two_sided
            || line.flags & !(HEIGHTS_LINE_FLAGS | ML_TWOSIDED) != 0
            || (from.x != to.x && from.y != to.y)
            || (from.x == to.x && from.y == to.y)
        {
            return Err(MapScaleError::Geometry.into());
        }
    }
    Ok(())
}
/// Whether a type of the given height fits the sector under `position`, as
/// `P_SpawnMobj` would place it (`R_PointInSubsector`, then `floorz`/`ceilingz`
/// from that sector). The source never checks this; rejecting a spawn that does
/// not fit keeps the vertical invariant and costs no legal play.
pub(crate) fn spawn_fits(
    world: &StaticWorld,
    sectors: crate::staging::Sectors<'_>,
    position: [Fixed; 2],
    spawn: SpawnHeight,
    height: Fixed,
) -> bool {
    let sector = sectors[usize::from(world.locate_validated(position).sector)];
    let z = match spawn {
        SpawnHeight::Floor => sector.floor,
        SpawnHeight::Ceiling => sector.ceiling - height,
        SpawnHeight::Absolute(z) => z,
    };
    sector.floor <= z && z + height <= sector.ceiling
}
/// Vertical momentum witness of a policy, in map units.
pub(crate) const fn momz_bound(policy: OriginalExecutionProfile) -> u32 {
    if policy.has_heights() {
        HEIGHTS_MOMZ_UNITS
    } else {
        64
    }
}
/// The per-actor vertical witness of the monster preflights: `z` within
/// `HEIGHTS_Z_UNITS` and `momz` within `HEIGHTS_MOMZ_UNITS` under heights (so every
/// `P_ZMovement` sum is representable), or the historical `momz` within 64 and
/// `z + momz` within 1,024.
pub(crate) fn vertical_witness(policy: OriginalExecutionProfile, a: &Actor) -> bool {
    if policy.has_heights() {
        sight::within(a.z, HEIGHTS_Z_UNITS) && sight::within(a.momz, HEIGHTS_MOMZ_UNITS)
    } else {
        sight::within(a.momz, 64) && sight::within(a.z + a.momz, 1024)
    }
}
/// The vertical invariant of the heights policy at a tic boundary, for every
/// actor that is not an impact: `floor <= z <= ceiling - height` with the actor's
/// own `floorz`/`ceilingz`, `z` within `HEIGHTS_Z_UNITS` and `momz` within
/// `HEIGHTS_MOMZ_UNITS`. Both sums are formed from bounded operands.
pub(crate) fn vertical_admitted(actor: &Actor) -> Result<(), SimError> {
    if !sight::within(actor.z, HEIGHTS_Z_UNITS)
        || !sight::within(actor.momz, HEIGHTS_MOMZ_UNITS)
        || actor.z < actor.floor
        || actor.z + actor.spec.height > actor.ceiling
    {
        return Err(MapScaleError::Vertical.into());
    }
    Ok(())
}
/// The largest admitted actor height `H_max` of the corpse envelope (V′ case (e)):
/// the tallest admitted type (the demon, 56) bounds a corpse's pre-gib height.
pub(crate) const V_PRIME_MAX_HEIGHT_UNITS: i32 = 56;
/// Which case of the vertical invariant V′ (M3h8 note §4.1) a non-impact actor is
/// in at a tic boundary under the movers policy, or `None` when it is in none.
/// `reach` is the admission's `[R_min, R_max]` and `delta` its `Δ`, in map units.
/// The cases are tried in order (a)–(e); (b) and (d) differ only in
/// `MF_SHOOTABLE`. Every operand is bounded by the `z` and height witnesses checked
/// first, so no sum overflows.
pub(crate) fn v_prime_case(actor: &Actor, reach: [i32; 2], delta: i32) -> Option<char> {
    let (z, floor, ceiling, height) = (actor.z, actor.floor, actor.ceiling, actor.spec.height);
    let life = actor.life()?;
    let shootable = life.flags & original_player::MF_SHOOTABLE != 0;
    let top = z + height;
    let units = |u: i32| Fixed::from_bits(u.saturating_mul(65536));
    if floor <= z && top <= ceiling {
        Some('a')
    } else if !shootable && z == floor && ceiling < top {
        Some('b')
    } else if floor - units(delta) <= z && z < floor && top <= ceiling {
        Some('c')
    } else if shootable && z == floor && ceiling < top {
        Some('d')
    } else if life.health <= 0
        && !shootable
        && units(reach[0] - V_PRIME_MAX_HEIGHT_UNITS) <= z
        && z < floor
        && top <= ceiling
    {
        Some('e')
    } else {
        None
    }
}
/// The vertical invariant V′ of the movers policy at a tic boundary (M3h8 note
/// §4.1, decision N2), replacing [`vertical_admitted`] for that policy only: the
/// `z` and `momz` witnesses of the heights policy, then one of the cases (a)–(e)
/// of [`v_prime_case`] with the actor's own `floorz`/`ceilingz`.
pub(crate) fn v_prime_admitted(
    actor: &Actor,
    admission: &mover_rules::MoverAdmission,
) -> Result<(), SimError> {
    if !sight::within(actor.z, HEIGHTS_Z_UNITS)
        || !sight::within(actor.momz, HEIGHTS_MOMZ_UNITS)
        || !sight::within(actor.floor, HEIGHTS_Z_UNITS)
        || !sight::within(actor.ceiling, HEIGHTS_Z_UNITS)
        || v_prime_case(actor, admission.reach, admission.delta).is_none()
    {
        return Err(MapScaleError::Vertical.into());
    }
    Ok(())
}
impl Simulation {
    /// Whether `actor` is a gibbed corpse admitted by the movers policy
    /// ([`planes::is_gibs`]); always false under every other policy.
    #[inline]
    pub(crate) fn gibbed(&self, actor: &Actor) -> bool {
        self.player_policy().has_movers() && planes::is_gibs(actor)
    }
    /// The tic-boundary vertical invariant of the selected policy for one
    /// non-impact actor: V′ under the movers policy, the heights invariant under
    /// the other heights policies.
    pub(crate) fn vertical_invariant(&self, actor: &Actor) -> Result<(), SimError> {
        if self.player_policy().has_movers() {
            let admission = self.movers.as_ref().ok_or(MapSpawnError::Context)?;
            v_prime_admitted(actor, admission)
        } else {
            vertical_admitted(actor)
        }
    }
}
/// Position admission under the policy: the derived room, or the historical
/// absolute margin when no room is admitted.
pub(crate) fn position_admitted(
    room: Option<RoomBounds>,
    position: [Fixed; 2],
    margin: u32,
) -> bool {
    match room {
        Some(room) => room.contains(position),
        None => position.iter().all(|v| sight::within(*v, margin)),
    }
}
/// Spawn admission: the whole closed box inside the room, or the historical margin.
pub(crate) fn spawn_admitted(
    room: Option<RoomBounds>,
    position: [Fixed; 2],
    radius: Fixed,
    margin: u32,
) -> bool {
    match room {
        Some(room) => room.contains_box(position, radius),
        None => position.iter().all(|v| sight::within(*v, margin)),
    }
}
/// Travel admission before or after a tic: inside a room, the center stays in
/// the rectangle by the wall invariant and momentum stays checked; otherwise the
/// historical `160M/3` envelope.
pub(crate) fn travel_admitted(
    room: Option<RoomBounds>,
    position: [Fixed; 2],
    momentum: [Fixed; 2],
) -> Result<(), SimError> {
    match room {
        Some(room) => {
            if !room.contains(position) {
                return Err(MapScaleError::Position.into());
            }
            momentum_admitted(momentum)
        }
        None => original_player::travel_bounds(position, momentum).map_err(Into::into),
    }
}
pub(crate) fn momentum_admitted(momentum: [Fixed; 2]) -> Result<(), SimError> {
    if momentum
        .iter()
        .all(|m| sight::within(*m, MAP_SCALE_MOMENTUM_UNITS))
    {
        Ok(())
    } else {
        Err(MapScaleError::Momentum.into())
    }
}
impl Simulation {
    /// The admitted room of the map-scale policy, if selected.
    pub(crate) fn room(&self) -> Option<RoomBounds> {
        self.original_player.and_then(|p| p.room)
    }
    /// What world items' heights are validated against: the item's own stored
    /// heights under the movers policy (the relaxed witness, M3h8 note §4.2), the
    /// sector table under heights, the flat world otherwise.
    pub(crate) fn item_sectors(&self) -> crate::world_pickups::ItemHeights<'_> {
        crate::world_pickups::ItemHeights::for_policy(
            self.player_policy(),
            crate::staging::Sectors::committed(&self.sectors),
        )
    }
    /// Rust-only spawn fit of the heights policy (§4.3 of the M3h4 note): the
    /// spawn's resulting `z` lies within `[floor, ceiling - height]` of the spawn
    /// position's sector (an absolute `z` included). Always true elsewhere.
    pub(crate) fn spawn_fits(
        &self,
        ty: ActorTypeId,
        position: [Fixed; 2],
        spawn: SpawnHeight,
    ) -> Result<(), SimError> {
        if !self.player_policy().has_heights() {
            return Ok(());
        }
        let height = self
            .registry
            .as_deref()
            .and_then(|r| r.actor_type(ty))
            .ok_or(ActorError::InvalidType)?
            .height();
        if spawn_fits(&self.world, self.sector_view(), position, spawn, height) {
            Ok(())
        } else {
            Err(MapScaleError::Fit.into())
        }
    }
    pub(crate) fn position_admitted(&self, position: [Fixed; 2], margin: u32) -> bool {
        position_admitted(self.room(), position, margin)
    }
    /// Spawn admission: the rectangle rule of the map-scale policies, and under the
    /// diagonal policy also the boundary rule (center inside `Ω`, closed box meeting
    /// no wall).
    ///
    /// Under the barrels policy the rule is S′ (M3h9b note decision F3): the center
    /// in the closed vertex rectangle (`RoomBounds::contains`) and
    /// [`Boundary::spawn_admitted_relaxed`] with `mover` (monsters and the player
    /// body: true; pickups and decorations: false). Every older policy ignores
    /// `mover` and keeps the closed-box rule.
    pub(crate) fn spawn_admitted(
        &self,
        position: [Fixed; 2],
        radius: Fixed,
        margin: u32,
        mover: bool,
    ) -> bool {
        if self.player_policy().has_episode1() {
            // Decision D4: without a boundary, room containment alone.
            return self.room().is_some_and(|room| room.contains(position))
                && self
                    .boundary
                    .as_deref()
                    .is_none_or(|b| b.spawn_admitted_relaxed(position, radius, mover));
        }
        if self.player_policy().has_barrels() {
            return self.room().is_some_and(|room| room.contains(position))
                && self
                    .boundary
                    .as_deref()
                    .is_some_and(|b| b.spawn_admitted_relaxed(position, radius, mover));
        }
        spawn_admitted(self.room(), position, radius, margin)
            && self
                .boundary
                .as_deref()
                .is_none_or(|b| b.spawn_admitted(position, radius))
    }
    /// Absolute XY/Z bound of every live actor for sight and perception reads: the
    /// room's own envelope under the real-map policy, the M3f2 envelope under the
    /// other map-scale policies, the historical 1,024 elsewhere.
    pub(crate) fn sight_margin(&self) -> u32 {
        match self.room() {
            Some(room) if self.player_policy().has_real_map() => room.envelope_units(),
            Some(_) => MAP_SCALE_EXTENT_UNITS,
            None => 1024,
        }
    }
    /// Live fireball bound of the selected policy.
    pub(crate) fn live_projectile_bound(&self) -> usize {
        live_projectile_bound(self.player_policy())
    }
}
/// Live fireball bound of `policy`: under the Episode 1 policy the store's own
/// [`EPISODE1_ACTOR_SLOTS`] (decision D10: no per-type cap; the store-full
/// witness covers every missile), [`MAP_SCALE_MAX_LIVE_PROJECTILES`] under the
/// other map-scale policies, [`MAX_LIVE_PROJECTILES`] otherwise.
pub(crate) const fn live_projectile_bound(policy: OriginalExecutionProfile) -> usize {
    if policy.has_episode1() {
        EPISODE1_ACTOR_SLOTS as usize
    } else if policy.has_map_scale() {
        MAP_SCALE_MAX_LIVE_PROJECTILES
    } else {
        MAX_LIVE_PROJECTILES
    }
}
pub(crate) const fn admitted_lines(policy: OriginalExecutionProfile) -> usize {
    if policy.has_real_map() {
        REAL_MAP_FORMAT_RECORDS
    } else if policy.has_map_scale() {
        MAP_SCALE_WALL_LINES
    } else {
        128
    }
}
/// Every linedef, sidedef, vertex, seg, subsector, node and sector table of the WAD
/// format is bounded by its `short` references (doomdata.h; M3h6 note §2.1): a
/// non-negative `short` index, or a 15-bit node child. The parser enforces it.
pub const REAL_MAP_FORMAT_RECORDS: usize = i16::MAX as usize;
/// p_local.h `MAXINTERCEPTS`: the source array one trace may fill; the real-map
/// policy refuses the tic that would write the 129th entry.
pub const MAX_INTERCEPTS: usize = 128;
/// The arithmetic certificate of the real-map policy (M3h6 note §4.2), computed
/// from the loaded map in exact integer arithmetic against the same source
/// expressions the M3f2/M3h3 envelope bounded with constants: (i) the hitscan
/// `P_InterceptVector` numerator and denominator, (ii) the sight
/// `P_InterceptVector2` sums, (iii) `P_AproxDistance`, (iv) `SlopeDiv`, (v) the
/// BLOCKMAP grid, (vi) the side tests, (vii) the heights. Derived at load; never
/// canonical state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArithmeticCertificate {
    /// Width and height of the vertex rectangle in map units.
    pub span: [u32; 2],
    /// Largest `|dx| + |dy|` over the linedefs.
    pub norm1: u32,
    /// Largest `dx² + dy²` over the linedefs.
    pub norm2_squared: u64,
    /// Largest `|dx|` or `|dy|` of any linedef.
    pub component: u32,
    /// Largest `|dx| + |dy|` over the node partitions.
    pub node_norm: u32,
    /// Largest absolute floor or ceiling height.
    pub height: u16,
}
impl ArithmeticCertificate {
    /// Trace reach of a hitscan in map units: `MISSILERANGE` plus one cell plus
    /// the `FRACUNIT` nudge (M3h6 note §2.3).
    pub const HITSCAN_REACH: u64 = 2_048 + 128 + 1;
    /// Largest line norm the hitscan intercept arithmetic admits (`256 · 2,177 · L
    /// + L + 2 ≤ 2³¹ − 1`).
    pub const MAX_LINE_NORM: u32 = 3_853;
    /// `SlopeDiv` reproduces the source quotient for deltas below 8,192 units per
    /// axis (owner decision 10 of the M3h6 note).
    pub const SLOPE_DIV_UNITS: u32 = 8_192;
    /// Roman names of the eight clauses, in the order `evaluate` reports them.
    pub const CLAUSES: [&'static str; 8] = ["i", "ii", "iii", "iv", "v", "vi", "vii", "viii"];
    /// Compute the certificate and check (i)–(viii); `MapScaleError::Arithmetic`
    /// names any failure. Linear in the map; nothing runs per tic.
    pub fn derive(map: &Map) -> Result<Self, SimError> {
        let (certificate, clauses) = Self::evaluate(map)?;
        if clauses.iter().all(|c| *c) {
            Ok(certificate)
        } else {
            Err(MapScaleError::Arithmetic.into())
        }
    }
    /// The Episode 1 policy's subset of [`Self::derive`] (decision H3): clauses
    /// (iii), (iv), (v), (vi) and (viii) stay load rules; (i), (ii) and (vii)
    /// are not required, because the traces of that policy compute the bounded
    /// intercept and height sums checked and refuse the operation where the
    /// source would overflow (`MapScaleError::Arithmetic`).
    pub fn derive_episode1(map: &Map) -> Result<Self, SimError> {
        let (certificate, clauses) = Self::evaluate(map)?;
        if [2, 3, 4, 5, 7].iter().all(|&i| clauses[i]) {
            Ok(certificate)
        } else {
            Err(MapScaleError::Arithmetic.into())
        }
    }
    /// The certificate values and the verdict of each clause (`CLAUSES` order):
    /// (i) hitscan intercepts, (ii) sight intercepts, (iii) `P_AproxDistance`, (iv)
    /// `SlopeDiv`, (v) the BLOCKMAP grid, (vi) the side tests, (vii) heights,
    /// (viii) reach: every trace endpoint, collision box and BLOCKMAP index formed
    /// from a center inside the rectangle stays within the `short` range, and every
    /// node partition origin lies in the rectangle. Only an empty map is an error.
    pub fn evaluate(map: &Map) -> Result<(Self, [bool; 8]), SimError> {
        let vertices = map.vertices();
        let (Some(first), Some(_)) = (vertices.first(), map.linedefs().first()) else {
            return Err(MapScaleError::Walls.into());
        };
        let mut min = [i64::from(first.x), i64::from(first.y)];
        let mut max = min;
        for v in vertices {
            for (axis, value) in [v.x, v.y].into_iter().enumerate() {
                min[axis] = min[axis].min(i64::from(value));
                max[axis] = max[axis].max(i64::from(value));
            }
        }
        let span = [max[0] - min[0], max[1] - min[1]];
        let (mut norm1, mut norm2_squared, mut component) = (0_i64, 0_i64, 0_i64);
        for line in map.linedefs() {
            let [a, b] = line.vertices.map(|i| vertices[usize::from(i)]);
            let dx = i64::from(b.x) - i64::from(a.x);
            let dy = i64::from(b.y) - i64::from(a.y);
            norm1 = norm1.max(dx.abs() + dy.abs());
            norm2_squared = norm2_squared.max(dx * dx + dy * dy);
            component = component.max(dx.abs()).max(dy.abs());
        }
        let node_norm = map
            .nodes()
            .iter()
            .map(|n| i64::from(n.delta.x).abs() + i64::from(n.delta.y).abs())
            .max()
            .unwrap_or(0);
        let height = map
            .sectors()
            .iter()
            .flat_map(|s| s.heights)
            .map(i16::unsigned_abs)
            .max()
            .unwrap_or(0);
        let certificate = Self {
            span: span.map(|v| v as u32),
            norm1: norm1 as u32,
            norm2_squared: norm2_squared as u64,
            component: component as u32,
            node_norm: node_norm as u32,
            height,
        };
        let limit = i64::from(i32::MAX);
        // (i) hitscan intercepts: |num| ≤ 256 · 2,177 · L₁ + L₁ + 2, |den| ≤ 256 · L₁ · 2,049.
        let hitscan = 256 * Self::HITSCAN_REACH as i64 * norm1 + norm1 + 2 <= limit
            && 256 * norm1 * 2_049 <= limit;
        // (ii) sight intercepts: 256 · diag · L₂ + L₁ + 2 ≤ 2³¹ − 1, squared.
        let diag_squared = (span[0] * span[0] + span[1] * span[1]) as u128;
        let sight = 65_536 * diag_squared * norm2_squared as u128
            <= ((limit - norm1 - 2).max(0) as u128).pow(2);
        // (iii) P_AproxDistance of any actor pair.
        let approx = span[0] + span[1] < 32_768;
        // (iv) SlopeDiv for actor pairs and line directions.
        let slope = span[0].max(span[1]) < i64::from(Self::SLOPE_DIV_UNITS)
            && component < i64::from(Self::SLOPE_DIV_UNITS);
        // (v) the grid: origin at or below the minimum, the maximum inside the
        // grid and within 32,768 units of the origin (MAPBLOCKSHIFT indexing).
        let blockmap = map.blockmap();
        let origin = [
            i64::from(blockmap.origin().x),
            i64::from(blockmap.origin().y),
        ];
        let [width, height_cells] = blockmap.dimensions().map(i64::from);
        let grid = origin[0] <= min[0]
            && origin[1] <= min[1]
            && max[0] < origin[0] + 128 * width
            && max[1] < origin[1] + 128 * height_cells
            && max[0] - origin[0] < 32_768
            && max[1] - origin[1] < 32_768;
        // (vi) the side tests: (L₁ + N) · span < 2³¹.
        let sides = (norm1 + node_norm) * span[0].max(span[1]) < 1_i64 << 31;
        // (vii) heights within ±512 (row 11 of the M3h6 note, kept).
        let heights = i64::from(height) <= 512;
        // (viii) reach: a hitscan endpoint (2,048), one cell (128) and the nudge (1)
        // beyond any center in R stay within ±32,767 units and within 32,768 units
        // of the BLOCKMAP origin (the raw fixed-point sums and `>> MAPBLOCKSHIFT`
        // indexes of P_AimLineAttack, P_LineAttack, P_PathTraverse and
        // P_CheckPosition); every node partition origin lies in R, so the side
        // tests of R_PointOnSide/P_DivlineSide difference points of R only.
        let reach = Self::HITSCAN_REACH as i64;
        let far = min
            .iter()
            .chain(max.iter())
            .map(|v| v.abs())
            .max()
            .unwrap_or(0);
        let in_rect = map.nodes().iter().all(|n| {
            let o = [i64::from(n.origin.x), i64::from(n.origin.y)];
            (0..2).all(|axis| o[axis] >= min[axis] && o[axis] <= max[axis])
        });
        let within_reach = far + reach < 32_768
            && max[0] - origin[0] + reach < 32_768
            && max[1] - origin[1] + reach < 32_768
            && in_rect;
        Ok((
            certificate,
            [
                hitscan,
                sight,
                approx,
                slope,
                grid,
                sides,
                heights,
                within_reach,
            ],
        ))
    }
}
/// Distinct linedefs listed in the fullest 2 × 2 window of BLOCKMAP cells (line 0
/// counted through each cell's leading 0): the slide-window certificate of the
/// M3h6 note §4.4. A slide trace adds lines only, from at most three cells inside
/// one such window, so a window of at most `MAX_INTERCEPTS` lines proves that no
/// slide ever reaches the intercept witness.
pub fn slide_window_lines(map: &Map) -> usize {
    let blockmap = map.blockmap();
    let [width, height] = blockmap.dimensions().map(usize::from);
    let words = blockmap.words();
    let ranges = blockmap.cell_ranges();
    let lines = map.linedefs().len();
    let mut best = 0;
    let mut seen = vec![usize::MAX; lines];
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
                        if line < lines && seen[line] != stamp {
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
/// The slide-window admission rule of the real-map policy.
pub(crate) fn validate_slide_window(map: &Map) -> Result<(), SimError> {
    if slide_window_lines(map) > MAX_INTERCEPTS {
        return Err(MapScaleError::Lines.into());
    }
    Ok(())
}
/// `P_GroupLines` (p_setup.c): every sector's lines in linedef order, a two-sided
/// line listed once in each of its sectors.
pub(crate) fn sector_lines(map: &Map) -> Vec<Vec<u16>> {
    let mut lists = vec![Vec::new(); map.sectors().len()];
    for (index, line) in map.linedefs().iter().enumerate() {
        let ends = line
            .sides
            .map(|s| s.map(|s| map.sidedefs()[usize::from(s)].sector));
        if let Some(front) = ends[0] {
            lists[usize::from(front)].push(index as u16);
        }
        if let Some(back) = ends[1] {
            if ends[0] != Some(back) {
                lists[usize::from(back)].push(index as u16);
            }
        }
    }
    lists
}
const _: () = assert!(MAP_SCALE_WALL_LINES + MAX_LINKED_THINGS <= 128);
