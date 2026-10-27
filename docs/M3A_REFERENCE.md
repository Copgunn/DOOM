# M3a investigation: grounded player XY and solid collision

Authority: preserved Linux DOOM 1.10, revision
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. Findings below were recorded before
M3a runtime implementation. This is an isolated static-sector subset, not a
complete P_PlayerThink, playable game, DOS guarantee or demo compatibility claim.

## Source behavior

- `p_tick.c`, `g_game.c`: apply active players in slot order before append-ordered
  thinkers, then specials/respawn and leveltime. No elapsed time enters a tic.
- `p_user.c` P_PlayerThink/P_MovePlayer: live, grounded, zero reaction time,
  no cheats/just-attacked/buttons is the supported movement subset. Turning wraps
  binary angle; forward then sideways thrust uses command times 2048 and original
  fine tables. View height, bob, weapon/actor animation and other player state
  belong to later phases; none is represented as implemented gameplay.
- `p_mobj.c` P_XYMovement: preserve M2 clamping and positive-only splitting. A
  blocked player calls P_SlideMove using the entire current momentum, even during
  a split update. Non-players zero momentum. Strict stop thresholds apply to a
  player only if forward AND side commands are zero. Angle-only input permits
  stopping. Ground friction is FixedMul by 0xe800.
- `p_map.c` P_SlideMove: trace three leading box corners in fixed order. Each
  trace visits intercepts by increasing fraction, preserving collection order on
  ties. Ignore one-sided lines approached from their back. Select only a strictly
  nearer blocker across traces. Move to fraction minus 0x800, project remaining
  momentum with P_HitSlideLine, retry at most twice, then Y-first/X-second stair
  stepping. Actor-only blocking also uses stair stepping; do not invent actor
  normals or modern circular/capsule projection.
- `p_maputl.c`: P_PathTraverse nudges a trace origin on a block boundary by one
  map unit, runs the original fixed DDA for at most 64 iterations, deduplicates
  lines per trace, then orders fractions stably. P_InterceptVector uses shifted
  FixedMul and guarded FixedDiv. P_HitSlideLine uses exact R_PointToAngle2,
  SlopeDiv/tantoangle, P_AproxDistance and fine sine/cosine tables for angled walls.
- `p_map.c` P_CheckPosition/PIT_CheckThing: things precede lines. Scan cells X
  outer/Y inner, expanding destination bounds by MAXRADIUS=32. Each thing is in
  the cell containing its center. With flags restricted to zero/MF_SOLID, block
  iff both absolute axis differences are strictly less than summed radii and
  the candidate is solid and not self. A non-solid mover is still blocked by a
  solid candidate. There is no Z test in this branch. Missiles/skulls/pickups,
  shootable behavior and all other flags are outside the API.
- P_SetThingPosition/P_UnsetThingPosition: spawn and EVERY successful move insert
  at cell head, including moves within a cell. P_RemoveMobj unlinks spatially
  immediately, then marks deferred thinker retirement. Spawn itself does not
  perform an occupancy check: explicit setup may overlap actors, as in the source.
- `p_mobj.h`, `d_player.h`, `r_defs.h`, `p_local.h`: pointers, allocator order and
  struct padding cannot be portable state. Separate generation identity, thinker
  order and spatial order; never sort generational handles to choose a blocker.

## Safe implementation policies

M2 constructors/profile and DSIM-v1 vectors remain frozen. The new explicit
player-collision profile uses DSIM-v2. Bound players get player XY behavior;
unbound actors retain non-player response. Solidity is independent of binding.
All movers are grounded in one static sector with one-sided, non-special walls.

The M3a profile caps lines at 128, BLOCKMAP cells at 4096 and logical actors at
1024. It conservatively requires every cell overlapped by a line's bounding box
to list that line and requires vertices within BLOCKMAP coverage. Missing spatial
coverage is a setup error, not a normal blocked movement. This may reject usable
vanilla diagonal blockmaps; it is a declared profile restriction, not a parser
change. Duplicate references retain first-encounter behavior. Degenerate lines
and overflowing deltas are rejected before stepping.

A small-profile spatial index maintains global most-recent-link order and filters
by center cell. Restricting that order to each queried cell exactly reproduces
cell-head insertion, without pointers, timestamp overflow or handle sorting.
Off-map actors remain in logical order but are excluded from cell queries.
Removal unlinks immediately; successful relinks do not allocate. Spawn reserves
index capacity before store mutation/RNG. Memory capacity is not canonical state.

All fixed operations have explicit wrapping/narrowing/shift semantics. Unsigned
SlopeDiv shifts wrap as C unsigned operations. Actor overlap uses widened absolute
differences to define abs(INT_MIN)/subtraction-overflow cases safely. P_AproxDistance
uses unsigned magnitudes with wrapping narrowing. Undefined C extremes are tested
as Rust policies separately, not compared as portable C behavior. Fixed DDA and
slide retries are bounded. Dynamic intercept storage replaces C's unchecked
128-entry array; stable fraction order preserves ordinary comparisons.

## Error atomicity audit

M2's apparent late movement errors are unreachable through its safe public API:
actor dimensions are validated on spawn, fields/maps are immutable externally,
M1 validates all indexes, cell loops clip to dimensions, and built-in thinkers
cannot spawn or exceed the store's visit bound. M3a retains these invariants and
makes the internal validated line path infallible. Recoverable step errors are
unsupported buttons and tic exhaustion, checked before commands/events/actors/RNG
change. Pause freezes before validation. Allocation exhaustion follows Rust's
process allocation-failure policy, not a recoverable SimError.

Setup errors validate before mutation. Failed spawn must preserve logical state,
RNG and index order. Generic ThinkerStore user-callback VisitLimit remains
nontransactional; Simulation does not expose those callbacks. No full-state clone
or rollback is needed per tic. Internal expect assertions denote implementation
invariants, never user-selectable geometry errors. Tests must exercise both
recoverable step errors and public setup failures against full canonical state.

## Evidence boundary

The separate M3a C adapter extracts original collision, actor links, DDA/intercept,
slide, XY and input bodies without changing historical files. It uses one static
sector and aborts unsupported effect/state paths. Source and adapter hashes,
compiler flags, inputs and results accompany vectors. Existing M2 provenance and
fixtures remain unchanged. Independent Python encodes versioned project-state
goldens without reading Rust output. Successful isolated comparisons do not
establish complete vanilla gameplay synchronization.
