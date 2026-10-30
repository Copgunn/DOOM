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

- M3f3 expanded-map Demon composition is merged. Its opt-in profile reuses
  larger static geometry with bounded map-spawned Demon perception, pursuit,
  melee and lifecycle. See [support](COMPATIBILITY.md#expanded-map-demon-profile),
  [source findings](M3F3_REFERENCE.md) and [observations](validation/M3F3.md).

- M3f4 original actor/state identity admission is merged. Complete original
  identity metadata and checked registry resolution preserve existing execution.
  See [mapping](ACTOR_IDENTITIES.md), [source findings](M3F4_REFERENCE.md) and
  [observations](validation/M3F4.md).

- M3f5 canonical MT_SERGEANT execution binding is merged. Typed source requests
  use the existing native closure, with independent canonical C observations and
  explicit DSIM projection. See [investigation](M3F5_REFERENCE.md) and
  [observations](validation/M3F5.md).

- M3f6 original-ID runtime storage audit is merged. Its native representation
  guards preserve all historical aliases and diagnostics; see
  [audit](M3F6_REFERENCE.md) and [observations](validation/M3F6.md).

- M3f7 original-ID registry/profile and diagnostic contract is merged. Actual
  source IDs, explicit context definitions and root-derived state permissions use
  shared execution services; bounded v20/v21 readers preserve historical meaning.
  See [investigation](M3F7_REFERENCE.md) and [observations](validation/M3F7.md).

- M3f8 context and target-lifetime ownership is merged. Scoped access, derived
  reference availability and compact independent lifetime inputs preserve existing
  gameplay. See [design](M3F8_REFERENCE.md) and [observations](validation/M3F8.md).

M3f9 game-instance provenance is merged. Opaque incarnation keys qualify boundary
references while the same actor store and execution context remain authoritative;
see [design](M3F9_REFERENCE.md) and [observations](validation/M3F9.md).

These foundations support controlled headless tests, not a complete playable game.
Earlier findings and milestones are retained in the [historical records](validation/archive/README.md).

## Next

### M3g3: Bounded original-player knockback and grounded movement

The implementation adds opt-in physical damage, deterministic XY momentum,
friction, bounded axis-wall sliding and moving corpses through shared services.
Its source observations and independent diagnostics preserve the older stationary
body policy. It stops at owner review; exact review/CI state belongs to GitHub.
See [investigation](M3G3_REFERENCE.md), [supported inputs](COMPATIBILITY.md#original-player-grounded-knockback)
and [observations](validation/M3G3.md).

M3g2 original player ownership/damage/death is merged; its
[investigation](M3G2_REFERENCE.md) and [observations](validation/M3G2.md) remain
historical references. M3g1 diagnostic-free stepping is merged; its
[audit](M3G1_REFERENCE.md) and [observations](validation/M3G1.md) remain unchanged.
Both stepping APIs still use one transactional gameplay core.

### Next narrow gameplay milestone

Investigate bounded canonical player command movement through original P_MovePlayer
and shared grounded services, with independently observed command/reaction-time/
state-entry ordering. Resolve the relevant PlayerThink dependencies explicitly;
do not infer full PlayerThink, view/camera, weapons or respawn compatibility from
movement alone. Active roster and automatic Demon acquisition require their own
source-validated player-thinker/roster boundary: the all-inactive source scan does
not terminate. Preserve both existing player policies and diagnostic schemas.

### Focused maintenance acceptance criteria

- Inventory production panic paths reachable from public inputs; replace recoverable
  failures with checked errors and preserve complete-state rejection evidence.
  Name remaining gameplay flags/constants with source references, without broad
  unrelated refactoring or changing historical profile semantics.
- Measure reference bytes against independently useful input/event coverage before
  adding large traces. Prefer compact uniquely useful evidence while retaining
  immutable historical fixtures/tools/archives. Any storage migration needs its own
  preservation proof and reviewed scope.

- Inventory repeated preflight walks and transaction staging against the measured
  component/allocation baseline. Any removal must preserve late-error rollback,
  bounded resources and complete source/event/RNG observations, with matched
  three-process timings and actual allocation counters. Do not add an unchecked
  release mode or fragile timing gate. Prioritize a separately reviewed bounded
  scratch reuse or one proven redundant validation over broad redesign.

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
