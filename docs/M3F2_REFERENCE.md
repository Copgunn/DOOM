# M3f2 source investigation: map extents and bounded spatial work

This investigation was committed before runtime changes. Behavioral authority is the preserved
Linux DOOM 1.10 source. This records the arithmetic/admission design; [observed validation](validation/M3F2.md)
separately records the implementation evidence. Current supported
behavior belongs in COMPATIBILITY; reproduction commands in TESTING; subsequent
observations in the milestone evidence record. DSIM remains a project diagnostic.

## Source and Rust inventory

Inspected original `p_map.c`, `p_maputl.c`, `p_setup.c`, `p_sight.c`, `p_mobj.c`,
`p_tick.c`, `p_local.h`, `r_main.c`, `r_bsp.c`, `r_defs.h`, `doomdata.h`,
`m_fixed.c`, `m_fixed.h` and the angle/sine tables. The existing
[M3f1 investigation](M3F1_REFERENCE.md) supplies the line/sector
capacity and adapter history; it does not establish expanded-coordinate safety.

Inspected Rust parser/types, Fixed, geometry, movement, multisector, sight,
line_work, map_scale, combat_trace, store, profile construction/spawn/control/query
paths and snapshot encoding. Relevant facts:

| Dependency | Source behavior and baseline Rust constraint |
| --- | --- |
| Classic tables | Signed short coordinates become signed 16.16; table representability alone is not gameplay safety. Ordinary references retain signed-short restrictions, node child high bit and absent sidedef sentinel. Parser record/allocation budgets remain unchanged. |
| Fixed | Source FixedMul uses signed long-long multiplication then arithmetic shift and conversion. FixedDiv has its original magnitude guard and active binary64 FixedDiv2. Rust Fixed deliberately wraps add/sub/negate/narrowing generally; the new profile must prove those wraps unreachable in admitted operations. |
| Collision | P_CheckPosition builds radius bbox, then source MAXRADIUS32 things box; X-cell loop outside Y, things before lines. Source P_PointOnLineSide uses shifted FixedMul. P_BoxOnLineSide retains slope-specific ties. P_TryMove checks fit/headroom/step/dropoff in source order. |
| Spatial | P_UnsetThingPosition/P_SetThingPosition unlink and insert at sector/BLOCKMAP heads, including same-cell moves. Source thinker order is independent. Origin subtraction precedes arithmetic shift23; integer row indexing uses original width. |
| Movement | P_XYMovement clamps to30, splits only when a positive component exceeds15, and preserves source negative split asymmetry. Friction is skipped above floor; XY precedes guarded Z. P_ZMovement adds momz, clips floors/ceilings and applies source gravity. No player, missile, float or skull branch is admitted. |
| BSP | R_PointInSubsector starts at last node, uses R_PointOnSide and first-seg sidedef sector ownership. Rendering R_RenderBSPNode is not the simulation scheduler. Nodes/subsectors remain4096. |
| Sight | P_CheckSight indexes REJECT as observer_sector*N+target_sector before geometric work. P_CrossBSPNode visits observer child, checks target side, then other child if required. P_DivlineSide retains three-way ties and the original horizontal x==node.y quirk. P_CrossSubsector marks lines globally per query, preserves seg order and clips source opening slopes. |
| Sight arithmetic | P_InterceptVector2 uses shifted FixedMul results whose *sum/difference* can overflow despite valid operands. Actor bounds alone do not prove safety. P_InterceptVector in p_maputl has the same dependency but its sliding/hitscan callers stay excluded. |
| Work | Baseline preparation4096cells/65536words, references262144. Baseline sight8192 counts full repeated-child expansion, not unique nodes; query asserts that same bound. New validation and execution must share a proven ceiling. |
| Profile | Baseline18 explicitly composes untyped static-sector schema3 under tag18. Version19 must make the same explicit choice, never infer typed admission from numeric ordering. |

## Derived arithmetic domain

Start with the source intercept expression, not an arbitrary replacement constant.
Let G bound absolute world vertices/partition origins, A bound actors/queries,
L bound the sum of absolute components of a linedef delta, and U=65536. Each shifted FixedMul
term in the numerator is bounded approximately by `(G+A)*L*256` raw units; each
denominator term by `(2*A)*L*256`. Both summed terms must fit signed32, as must
each individual FixedMul conversion. Translation does not reduce world-space
subtraction requirements; rays can join opposite signs.

For a research domain G=A=2048, both differences are at most4096. Bounding
`|line.dx| + |line.dy|` directly bounds the sum in both intercept expressions.
The simple choice L=2048 reaches `4096*2048*256 = 2147483648`, outside positive
i32. Choose L=1536, yielding1610612736 raw. Allowing negative-shift/product
rounding adds at most1538 raw, still below1610614274, with over536million raw
units of headroom. This admits an axis-aligned1536-unit line or a diagonal with
components768+768; a per-component768 ceiling would reject safe axis cases.
The norm is a setup proof, not a substitute gameplay distance calculation.
No query-specific exception silently admits longer lines.

Derived admission envelope:

- Vertex and node-origin components: absolute2048 units. Node deltas:2048.
- Linedef deltas: sum of absolute XY components at most1536, checked from endpoints.
- BLOCKMAP origin components: absolute2048. These do not constrain unused grid
  padding to lie inside the actor/world coordinate envelope. Geometry admission is not a strict superset
  of profile18: some old long diagonals need a narrower interaction domain and
  remain available only through their historical constructor.
- Sector floor/ceiling: absolute1024; noninverted, so actor height at most2048.
- Actor XY spawn/current before a tic:2018; destination/sight XY:2048. Source
  clamp30 leaves a fully representable next position and bbox.
- Absolute spawn/current Z before a tic:1984; sight Z:2048; XY/Z momentum:64.
  Floor/ceiling placement comes from the admitted sector, not unconditional zero.
  A successful tic may leave the next-tic admission margin; its next tic rejects
  atomically unless an admitted operation restores the margin.
- Radius remains0..32; positive height must fit spawn-sector span. No new role,
  flags, controls, collision semantics or actor allocation rules are introduced.

Actor-to-actor and actor-to-map differences are consequently at most4096 units.
Bbox+MAXRADIUS adds64; origin subtraction is at most4160 units, only272629760
raw. P_DivlineSide whole-unit products are at most4096 squared, below2^24;
R_PointOnSide and P_PointOnLineSide shifted products are smaller. No i32 add/sub,
absolute-value INT_MIN, line-delta conversion or long-long product overflows.
Largest FixedMul operand product is far below signed64. Source divisions retain
their documented saturation guard; this is not a new clamp. Intermediate eye,
height, opening, clipping-numerator and gravity arithmetic stays within i32.
REJECT has at most256 squared bits; row/cell indexes have at most8192 entries.

Source angular helpers/tables were inspected. This unbound profile does not bind
players, call facing/AI or enter sliding, so trig/angle multiplication is not newly
reachable; existing unsigned BAM fields remain unchanged. Float/missile/player
and gameplay actions reject, rather than expanding their arithmetic closure.

The reference ABI is GNU89, 32-bit signed int/fixed_t, signed16 short,
two's-complement arithmetic right shifts, little-endian fixture words, IEEE
binary64 and the documented GNU negative fixed-shift interpretation. Original
negative-coordinate setup shifts and signed right shifts are pinned adapter/ABI
assumptions, not portable ISO-C claims. New signed arithmetic overflow is not
accepted as equivalence. Original files remain byte-identical; no wider Rust
temporary is used to invent the result of overflowing original arithmetic.

## Work and allocation design

Retain line1024, sector256, actor64, rawword65536 and compiled-reference262144
ceilings. Increase only the explicit new profile to8192 grid cells and16384 fully
expanded sight units. A256x32 grid permits a real actor/query cell index above4096
within the derived coordinate envelope, rather than merely padding an unused grid.
List offsets after an8192-cell table still fit positive signed16. Shared/suffix
lists count references once per cell; conventional leading zero remains line0.

Every referenced classic list/suffix retains the existing parser requirement to
start with zero. A conventional empty list is `[0,-1]`, so source iteration still
visits line0. This parser policy is stricter than arbitrary original list starts;
it is not widened here. Shared suffix fixtures start at another genuine zero.

Reuse existing first-occurrence preparation, bounded SeenLines and cell membership.
No source list is regenerated or sorted by line ID. Checked native-size arithmetic
must budget cell vectors, starts, membership, compiled references, line/topology
and work scratch against a separate4MiB preparation ceiling before allocation.
This conservative allocation-request budget is distinct from parsed Map storage,
canonical caches and allocator overhead. Fallible reservations reject setup before
returning a simulation. Raw terminator scans and unique starts retain separately
bounded CPU dimensions described by the existing preparation complexity formula.

A radius32 line query touches at most four source cells, hence at most4096
prepared references and1024 unique line callbacks. The things margin is at most64;
it touches at most four cells, scanning at most64 live bodies per cell. A sight
query reserves at most `work + 5*1024` diagnostic visits and `2*nodes+1` tasks before
execution, using fallible allocation in the new profile. These are bounded
temporary observations, excluded from DSIM.

Memoize subtree cost but count both child references: leaf cost1+seg_count;
node cost2+child0+child1, checked against16384. Repeated children multiply cost.
Twelve duplicate-child levels over a one-seg leaf cost16382, exceeding the old
limit and allowing actual complete traversal when the ray crosses every partition.
Setup and query must use the same selected ceiling; acyclicity alone is insufficient.
Bound every reachable root expansion, while coordinate validation covers every
table entry. Preserve REJECT early-out and all source child/seg/tie ordering.

P_PathTraverse's original64-cell loop and MAXINTERCEPTS128 are behavioral source
restrictions, not these safety budgets. They remain unchanged and outside the new
profile. This work does not authorize larger weapon traces or player sliding.

## Atomicity, independent evidence and exclusions

Construct the new simulation only after immutable geometry/work validation.
Preflight all live actors before clearing events, consuming commands/RNG or moving
the first thinker. Validate generational identities, destination/current positions,
momentum and vertical margins before mutation. Read-only operations consume no RNG.
Use existing actor/spatial/sector storage, XY/Z and topology; never clone the full
simulation per tic. A derived world-domain marker can guard public BSP queries;
it is not additional authoritative state. Historical constructors keep old limits.

New C tooling must avoid the historical `generate*.py` discovery inventory:
use `m3f2_reference.py`, hash the declared adapters and retain genuine source bodies.
Compile/run expanded positions, negative origins, long but bounded lines,
positive/negative/origin-crossing sectors, height24/25 boundaries, gravity/momentum,
shared/duplicate/suffix lists, actual high cell indexes, REJECT and repeated-child
sight. Compare actor fields, line/thing/thinker/spatial order, source RNG indexes,
opening/clipping and ordered BSP/seg observations. Compiled inventory is not coverage.
An independent DSIM-v19 encoder must read C traces and synthetic map/setup inputs,
never Rust results. Bounds, invalid handles, pending retirement/reuse, capacities,
pause and failure atomicity are separately labeled Rust safety evidence.

This slice remains untyped/static/unbound. No full classic-map loader, player or
monster admission, dynamic sectors, specials, hitscan, rendering, extended format,
commercial asset or vanilla demo compatibility is implied. Source arithmetic or
work outside the proved domain must reject explicitly.

The implementation caches the proved root cost and sizes new query scratch from
that cost and the actual linedef count. It separately checks the configurable
collision proof budget (`nodes + 4*lines + 4*actor_limit`) and conservatively
charges preparation memory before allocation. The supported values and exclusions
are maintained only in [COMPATIBILITY](COMPATIBILITY.md#map-extent-profile).
