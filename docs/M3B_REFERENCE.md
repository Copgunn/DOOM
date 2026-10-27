# M3b investigation: static sectors, BSP and vertical movement

Recorded before M3b runtime changes. Authority: preserved Linux DOOM 1.10,
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. M2/DSIM-v1 and M3a/DSIM-v2 remain
separate reproducible profiles. M3b uses an explicit multi-sector profile/DSIM-v3.

## Confirmed source behavior

- `r_main.c` R_PointInSubsector starts at the last node, uses R_PointOnSide and
  children in stored 0/1 order, and iterates to a high-bit leaf; no nodes means
  subsector zero. R_PointOnSide has axis-specific equality rules, a sign-bit fast
  path and shifted FixedMul products. It is not polygon containment and differs
  from the line-side helper for overflowing/sign-boundary cases. M1 validates
  references/cycles; M3b still bounds iterative runtime traversal by node count.
- `p_setup.c` P_GroupLines sets each subsector's sector from its first seg's
  sidedef. M1 already requires consistent sector references across that leaf's
  segs. Linedef front/back sectors come from sidedefs; back existence and the
  ML_TWOSIDED flag must be consistent in the supported profile.
- `p_maputl.c` P_LineOpening: top=min(ceilings), bottom=max(floors),
  range=top-bottom, lowfloor=min(floors). A one-sided line has no opening.
- `p_map.c` P_CheckPosition starts constraints from the destination BSP leaf,
  checks things before lines, and preserves X-outer/Y-inner BLOCKMAP ordering.
  PIT_CheckLine first tests strict bbox/infinite-line crossing. One-sided walls
  block. On two-sided lines ML_BLOCKING blocks all ordinary movers;
  ML_BLOCKMONSTERS blocks non-players. Otherwise openings lower ceiling, raise
  floor and lower dropoff. The sector containing the center can differ from the
  sector contributing the highest floor/lowest ceiling while a box straddles.
- P_TryMove initializes floatok=false. After CheckPosition, reject if available
  height < actor height; then floatok=true. Next reject head clearance < height,
  floor-z > 24*FRACUNIT, and (without MF_DROPOFF or MF_FLOAT)
  floor-dropoff > 24*FRACUNIT, in that order. Equality at 24 and exact clearance
  are allowed. ML_BLOCKMONSTERS and dropoff permission are independent of player
  identity; binding does not silently set actor flags.
- On success P_TryMove updates floor/ceiling and XY, then unlinks/relinks spatial
  AND sector lists, even within one sector/cell. Z is unchanged until its later
  update. P_SetThingPosition resolves BSP again for sector membership. Removal
  unlinks both immediately and defers thinker retirement. P_SpawnMobj initializes
  floor/ceiling from the spawn leaf, and resolves floor/ceiling spawn Z modes.
- PTR_SlideTraverse checks two-sided opening range, ceiling-z and bottom-z step;
  it does not apply the ordinary line-blocking flags or dropoff restriction.
  Preserve this asymmetry; the later TryMove still checks all movement conditions.
  Existing three-corner traces, fixed DDA, stable ties, retries and fallback remain.
- `p_user.c` P_MovePlayer always applies angle turning, but forward/side thrust
  only when z<=floorz. `p_mobj.c` P_XYMovement skips ground friction when z>floorz.
  Its clamp/split/rounding/blocking/player response remains as in M3a.
- P_MobjThinker calls XY when momentum exists, then Z if z!=floorz or momz!=0,
  then state transitions. M3b implements only these ordered motion portions.
- P_ZMovement adds momz before collision/gravity. At/below floor, negative momz
  resets to zero and Z clamps to floor (positive momz is retained). Above floor,
  ordinary gravity is one unit/tic: zero momz first becomes -2 units, otherwise
  subtract one unit. MF_NOGRAVITY suppresses that branch. Afterwards, if z+height
  exceeds ceiling, positive momz resets and Z clamps to ceiling-height. Thus a
  zero-momentum airborne actor does not fall on the first gravity tic. There is
  no generic terminal-velocity clamp in this branch.
- Player step/landing viewheight/deltaviewheight and oof sound are side effects
  of P_ZMovement. They do not feed this physical subset. Rust records physical
  floor/ceiling/landing observations but does not pretend to implement player
  view, bob, sound, damage, state machines or complete P_PlayerThink.
- `p_local.h`: GRAVITY=FRACUNIT. MF_DROPOFF and MF_NOGRAVITY are supported explicit
  exceptions. MF_FLOAT, missiles/skulls/corpses, teleport/no-clip, pickups and
  shootable/combat behavior remain absent and have no arbitrary-bit API.

## BLOCKMAP investigation and explicit policy

M3a's bbox-cell completeness test treats every cell in a diagonal line's bounding
rectangle as intersected; cells away from the actual diagonal need not list it.
M1 structural validity neither establishes this conservative property nor real
spatial completeness. Original P_BlockLinesIterator consumes supplied lists,
including dummy linedef zero, without checking coverage or rebuilding data.

M3b explicitly selects reference-list semantics. It compiles the same first-
occurrence candidate order from structurally validated lists, retains work bounds,
and reports conservative bbox-cell omissions diagnostically rather than rejecting
them. A sparse diagonal fixture and a deliberately omitted solid boundary are
compared with unchanged C bodies: the latter can permit movement through missing
collision data exactly as the source does. Acceptance of a structural map is not
proof of correct spatial coverage. No automatic rebuilding or substitute geometry
query is allowed. M3a's strict profile remains unchanged. This is a deliberate,
reference-tested policy change in a new profile, not an unexplained relaxed check.

## Ownership, limits and safe policies

Map definitions/topology stay immutable. A separate runtime sector vector holds
floor/ceiling heights; no public height-mutation API or moving-sector gameplay is
part of M3b. Logical spatial-link order reconstructs source cell-head and sector-
head order by filtering current membership. Relinking changes both, immediate
removal excludes both, stale generation handles cannot re-enter either index.

Retain 128 lines, 4096 cells, 65536 raw BLOCKMAP words and 1024 actor slots. Add
128 sectors and 4096 nodes/leaves, bounded iterative lookup, nonzero lines and
consistent two-sided flags. Each sector may have zero clearance (closed opening),
but not inverted heights. The global vertical extent must fit signed i32 fixed
subtraction; unsupported extreme map spans reject before mutation. Structurally
valid BSPs need not be geometrically correct; this profile trusts their partition
and leaf definitions just as lookup in the source does, without claiming a node-
builder proof. Missing/invalid references/cycles remain M1 errors.

Fixed math retains explicit wrapping add/sub/negation, widened multiply and signed
shifts. Runtime raw-coordinate/Z extremes outside defined C arithmetic have safe
Rust policies and separate tests; exclude C signed overflow and abs(INT_MIN) from
reference comparisons. No allocation/address/hash iteration determines gameplay.
Setup validation precedes store/RNG/index mutation. Recoverable step errors remain
pre-mutation; movement invariants make per-tic motion infallible without full-state
cloning. Allocation exhaustion follows the documented process-level Rust policy.

## Reference and schema plan

Use a separate synthetic C harness and provenance, preserving all old vectors,
adapters and independent encoders. Extract original BSP/opening/collision/slide/
XY/Z/input functions. The adapter calls XY then Z with the source guard conditions;
it omits actor states and suppresses audio output while retaining C player-view
fields for source code execution. Record exact maps, flags, commands, attempted
moves/constraints, membership and multi-tic physical outputs. Reference generators
must not read Rust output or adjust expectations after failures without rationale.

DSIM-v3 adds momz, gravity/dropoff flags, leaf/sector membership, runtime sector
heights/list ordering, explicit BLOCKMAP policy and ordered opening/vertical
observations. Keep v1/v2 fields/fixtures unchanged in their original profiles.
Independent Python goldens and field-level first divergence supplement C traces;
neither establishes full vanilla engine, DOS demo or enhanced visual-portal support.
