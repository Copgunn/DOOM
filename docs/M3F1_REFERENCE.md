# M3f1 source investigation: bounded map scale

This investigation precedes runtime implementation. Original Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0` is behavioral authority. The
[compatibility inventory](COMPATIBILITY.md#map-size-and-processing-limits) is the
starting inventory; it does not imply that parsed maps can execute. Final support
belongs in COMPATIBILITY, measurements in `docs/validation/M3F1.md`, and exact
review/merge/CI history in GitHub. No new comparison result is claimed here.

## Source findings

Inspected original [p_map.c](../linuxdoom-1.10/p_map.c),
[p_maputl.c](../linuxdoom-1.10/p_maputl.c),
[p_setup.c](../linuxdoom-1.10/p_setup.c),
[p_sight.c](../linuxdoom-1.10/p_sight.c),
[p_local.h](../linuxdoom-1.10/p_local.h),
[r_main.c](../linuxdoom-1.10/r_main.c),
[r_bsp.c](../linuxdoom-1.10/r_bsp.c),
[r_defs.h](../linuxdoom-1.10/r_defs.h) and
[doomdata.h](../linuxdoom-1.10/doomdata.h).

- `P_LoadVertexes`, `P_LoadSectors` and `P_LoadNodes` derive table counts from lump
  byte lengths and allocate arrays. They do not impose a universal 128-record cap.
  Their short coordinates/heights become fixed-point integers. References retain
  their source interpretation; increasing allocation cannot widen the format.
- `P_LoadBlockMap` converts short words, preserves the supplied grid/list storage,
  derives origin/dimensions from its header and clears newly allocated block links.
  `P_BlockLinesIterator` follows the selected cell's word order until -1. Its
  conventional leading zero is processed as line zero, not skipped. `validcount`
  suppresses subsequent occurrences across the entire operation. The first
  failing callback ends traversal. Shared lists and suffixes must preserve this.
- `P_CheckPosition` determines the destination subsector's initial floor/ceiling,
  checks things first, then lines. Both cell loops have X outside Y. Thing queries
  extend the box by source `MAXRADIUS` (32 units); line queries do not.
- `PIT_CheckLine` rejects crossed one-sided/blocking lines, then accumulates
  `P_LineOpening` minima/maxima in candidate order. `P_TryMove` checks fit,
  headroom, step greater than 24, then prohibited dropoff greater than 24. These
  inequalities and the distinction between player identity and MF_DROPOFF remain.
- Successful `P_TryMove` unlinks and relinks even within the same cell/sector.
  `P_SetThingPosition` inserts at both sector and BLOCKMAP heads. Thinker order is
  independent. `P_UnsetThingPosition` preserves the remaining linked-list order.
- `R_PointInSubsector` starts at the last node, chooses children using
  `R_PointOnSide`, and strips `NF_SUBSECTOR` (0x8000) for leaf indexes. With no
  nodes it selects leaf zero. `P_GroupLines` assigns each leaf's sector from its
  first seg's sidedef. Rendering's `R_RenderBSPNode` shares the node encoding but
  its visibility/drawing traversal is not a simulation scheduler.
- `P_CheckSight` first indexes REJECT as `observer_sector * numsectors + target_sector`.
  Rejection returns before BSP work. Otherwise `P_CrossBSPNode` traverses the
  observer side first; `P_CrossSubsector` marks linedefs once, retains source seg
  order, tests openings and clips slopes. Sector-count expansion must preserve
  indexing, leaf ownership, duplicate handling and integer arithmetic together.
- `P_PathTraverse` has an original 64-cell loop. `MAXINTERCEPTS=128` is also an
  original intercept-storage bound, distinct from a line-table count. This
  milestone must not silently extend source traversal semantics or admit
  overflow-dependent behavior. Combat/hitscan remains outside the new profile.

## Rust bounds rechecked

Inspected [parser](../crates/doom-map/src/parse.rs),
[map types](../crates/doom-map/src/types.rs),
[geometry](../crates/doom-sim/src/geometry.rs),
[movement](../crates/doom-sim/src/movement.rs),
[topology](../crates/doom-sim/src/multisector.rs),
[sight](../crates/doom-sim/src/sight.rs),
[combat tracing](../crates/doom-sim/src/combat_trace.rs),
[store](../crates/doom-sim/src/store.rs),
[profile selection](../crates/doom-sim/src/lib.rs) and
[snapshot encoding](../crates/doom-sim/src/snapshot.rs).

| Category | Baseline finding and dependency |
| --- | --- |
| Classic encoding | Signed short coordinates/heights; ordinary references limited to nonnegative signed-short range; node-child high bit and absent-sidedef sentinel retain distinct meanings |
| Parser budgets | 32,768 records/table hard cap as well as configurable cap; 64 MiB/lump, 262,144 BLOCKMAP cells, 4 Mi words, 128 MiB cumulative decoded/scratch requests; checked arithmetic and fallible reservation |
| Uniform record cap | THING count derives from byte length, so this is not an intrinsic count field; no parser expansion is needed for this milestone |
| Collision preparation | 128 lines, 4,096 cells, 65,536 raw words; fixed `next[128]`, per-cell u128 membership; first-occurrence compilation sorts source word positions, not line IDs |
| Multi-sector movement | u128 cell membership and fixed `seen[128]`; per-destination global deduplication must retain first encounters |
| Topology | 128 sectors, 4,096 leaves/nodes; sector state and leaf-sector storage are already vectors with validated u16 references |
| Geometry base | Flat legacy queries already size their seen vector from line count; static map/lines/spatial index are shared services, not another physics engine |
| Sight | u128 seen set; coordinates/node deltas/heights at most 512 units; actors/queries at most 1,024; expanded BSP/seg work at most 8,192 including repeated children |
| Sliding and hitscan | Separate seen[128] buffers; source 64-cell traversal; hitscan candidate budget `64*(128+1024)` and 128 intercept cap; not admitted by the new unbound profile |
| Actor capacity | Underlying store at most 65,536; existing static-sector constructor at most 1,024; new geometry work need not expand actors |
| Canonical state | Numeric schema comparisons currently imply typed lifecycle data for versions >=4; the new untyped geometry profile must explicitly select its field layout |

## Implementation decision and safety boundary

Introduce a separate opt-in geometry profile rather than changing historical
admission. Initial target: **512 lines and 256 static sectors**, retaining existing
cell/raw-word/node/leaf and arithmetic ceilings. This covers required 128/129/256
boundaries while allowing a 256-sector strip with more than 256 boundary lines.
It is a bounded implementation decision, not an original-format ceiling. Require
source comparisons at the new ceiling and rejected ceiling-plus-one cases before
claiming support. A prepared-reference budget of 262,144 bounds shared-list
expansion independently of raw BLOCKMAP words; smaller caller budgets can test
rejection without changing successful state.

Use bounded source-ordered vectors for compiled lists, dynamically sized
multiword membership sets for geometry preparation, and a bounded multiword
scratch set for query deduplication. Reserve new storage fallibly before installing
it. No HashMap/HashSet order, global-line sorting or BLOCKMAP rebuilding is allowed.
Retain the legacy 128 admission gates and all historical helper/fixture bytes.

Reuse static-sector XY/Z, openings, spatial lists and the existing actor store.
Admit controlled **unbound synthetic bodies**, not player initialization, input
binding, actor registries, Demon AI, damage, hitscan, map spawning or dynamic
sectors. Read-only original sight may compose the existing query after its map,
actor, identity and work preconditions are checked. It must not enable perception
rosters, sound propagation or monster actions. Unsupported APIs must still reject.

Setup must validate total references, capacities, topology, specials, arithmetic
and sight expansion before returning an instance. Runtime controls/queries and
stepping must preflight the new bounded arithmetic policy before authoritative
mutation. Stale/pending handles and failures retain complete canonical state. No
full-simulation clone per tic; small copied bodies and existing preflight services
remain the execution model. Pause and presentation reads must not consume RNG.

## Evidence plan

A new generator will compile genuine original bodies with a declared synthetic
geometry/ABI adapter, preserving earlier generators. Exercise small and expanded
line/sector maps, dense/sparse/shared/duplicate lists, source order, openings,
24-unit boundaries, BSP/REJECT, sector-head relinking and multi-tic XY/Z. Record
actual source hashes, compiler flags, inputs, fields and visited sequences. Abort
unsupported C callbacks rather than silently simulating them.

DSIM-v18 will compose the existing untyped static-sector fields, with an explicit
new version/profile tag. It adds no redundant actor capacity/corpse state. A
separate encoder must consume C traces and public setup/schema values, never Rust
output; compare full bytes, per-tic digests and first differing fields. Existing
DSIM-v1 through v17 remain frozen. Rust-only bounds, rejected setup/control,
stale references, work exhaustion and atomicity are a separate evidence family.

Measure preparation, collision, sight, repeated stepping and geometry storage on
small and expanded dense/sparse maps. Run the full historical suite and relevant
bounded fuzz/ASan campaign; report actual executions and leak settings. These
observations do not establish complete real-map or vanilla-demo compatibility.
