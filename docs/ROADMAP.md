# Roadmap

This is the authoritative development summary and plan. The [documentation index](README.md#documentation-authority)
identifies the other information owners. Exact review state, commits, CI and merge
history belong to GitHub; supported behavior and limits belong to [COMPATIBILITY](COMPATIBILITY.md).

## Done

The merged baseline provides:

- WAD and classic-map parsing with checked references and resource budgets.
- Deterministic simulation, fixed-point arithmetic and original random-number streams.
- Ordered actors, state changes, static-sector movement and collision.
- Selected weapon/monster combat, sight, hearing and pursuit.
- Demon stand/run, melee, pain/death and persistent corpses.
- Bounded classic THING spawning (M3d3i, merged), independently checked against
  original C and versioned simulation snapshots.
- Demon map context and statistics (M3d3j): original kill-count flags, separate
  map and credited-kill totals, and fresh level reset with RNG continuity.
  See the [source investigation](M3D3J_REFERENCE.md) and
  [observed tests](validation/M3D3J.md) for technical evidence.
- M3f1 bounded line/sector scale, regularly merged: reusable source-ordered static
  collision and read-only sight preparation. See [source findings](M3F1_REFERENCE.md)
  and [observations](validation/M3F1.md).

- M3f2 expanded coordinates and spatial work, regularly merged: checked arithmetic,
  bounded collision preparation and fallible sight. See [source findings](M3F2_REFERENCE.md)
  and [observations](validation/M3F2.md).

These foundations support controlled headless tests, not a complete playable game.
Earlier findings and milestones are retained in the [historical records](validation/archive/README.md).

## Next

### M3f3: Demon gameplay on larger static maps

M3f3 is the current implementation and review milestone. Its opt-in profile
composes map-spawned Demon perception, pursuit, melee and lifecycle
with larger geometry while retaining explicit actor, arithmetic and action limits.
See the [support boundary](COMPATIBILITY.md#expanded-map-demon-profile),
[source investigation](M3F3_REFERENCE.md) and [measured evidence](validation/M3F3.md).
This is a step toward real-map readiness, not complete original actor or game loading.

### Remaining Demon admission

Next, investigate original Demon actor/state identity and remaining game-context
dependencies. The [support matrix](COMPATIBILITY.md#demon-support-remaining-dependencies)
explains why the current test adaptation does not establish full Demon/Spectre support.

### Map Scale, Limits, and Real-Map Readiness

**Broader readiness remains planned.** M3f1 supplies the merged line/sector
foundation; M3f2 supplies bounded coordinates/work and M3f3 composes a controlled
Demon gameplay subset. Further stages
require separate owner direction. The longer-term
purpose is practical support for increasingly complex classic maps,
not unlimited geometry or arbitrarily larger constants.

Start from the [code-backed limits inventory](COMPATIBILITY.md#map-size-and-processing-limits).
It distinguishes original encoding/source constraints, parser allocation budgets,
temporary simulation restrictions, and CPU/traversal work bounds. Parsing a map
successfully does not mean a simulation configuration can execute it.

Acceptance for that future milestone:

- Inventory line, sidedef, sector, vertex, seg, node, subsector, BLOCKMAP and THING
  limits, including interactions between individual table and aggregate budgets.
- Identify unnecessarily restrictive bounds for valid classic maps; document
  encoding/source constraints that must remain and where wider internal indexes,
  dynamic work buffers or other representation changes are required.
- Keep classic compatibility distinct from extended map formats. Specify the
  intended supported slice and compatibility implications before changing limits.
- Preserve checked indexing/arithmetic/allocation, bounded memory and recoverable
  error atomicity; choose justified capacities and work budgets.
- Benchmark larger synthetic or legally redistributable public maps. Verify BSP,
  collision, sight, thinker/spatial ordering and deterministic runtime capacities.
- Add boundary, rejection and regression tests; preserve original-C evidence and
  historical canonical fixtures. Publish measurements and remaining restrictions.

This plan does not authorize commercial-asset access, new map formats or broader
monster/player behavior. Further stages require separate owner direction.

## Later

- Complete player/gameplay integration and broader monster/THING support.
- Moving sectors and map specials (the established M3e direction): doors, lifts,
  triggers and dynamic-sector interactions.
- **M4 — Demo compatibility:** version-specific playback and divergence diagnostics;
  keep DOS and Linux behavior claims separate.
- **M5 — Rendering and platform integration:** presentation/input/audio and a
  custom Vulkan-first rendering direction across supported platforms. Presentation
  must not change authoritative simulation or RNG; no renderer exists yet.
- Multiplayer and later experiments, after their source/game-context dependencies
  can be independently validated.

The established historical labels are retained; this order expresses dependencies,
not old document append order. Current exclusions remain in [COMPATIBILITY](COMPATIBILITY.md#deferred-systems).
