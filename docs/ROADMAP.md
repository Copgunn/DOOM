# Roadmap

The project builds original-game behavior in independently validated headless
slices. This describes capabilities and dependencies; immutable CI and counts
belong in [validation records](VALIDATION.md).

## Implemented foundations

- Safe WAD/classic-map parsing and fixed-point/original RNG types.
- Deterministic actors, thinker/spatial order, static-sector movement and collision.
- Explicit state/action lifecycle, shared damage and bounded combat profiles.
- Sight, sound, ordered target acquisition and original pursuit decisions.
- Selected hitscan monster fixtures and Demon melee/pain/death/corpse behavior.
- Original Demon stand/run state closure and bounded THING spawning.
- Independent original-C traces and versioned DSIM snapshots through v17.

## M3d3j: map context and public documentation

The current feature adds available map kills, original COUNTKILL flags, the
source single-player fallback death-credit counter and fresh level reset with RNG
continuity. It retains labeled actor/state adaptations and explicit respawn/player
restrictions. The same PR reorganizes public documentation, preserves detailed
historical evidence and discloses AI maintenance under human direction.

Delivery requires separate owner review; pending gates and the repository About
metadata limitation are recorded in the [M3d3j evidence](validation/M3D3J.md).
No subsequent gameplay milestone is authorized by this work.

## Next engineering priorities

1. Review the map-context slice and remaining original Demon admission matrix.
2. Investigate source actor/state identity and game-context dependencies without
   assuming animations alone establish full actor admission.
3. Design the next separately authorized gameplay slice around independent source
   evidence, deterministic composition and preserved historical tests.
4. Later, extend map/player/special/demo capabilities before whole-game claims.

A custom Vulkan-first renderer is a future independent presentation subsystem.
It must not control authoritative state or RNG; no rendering runtime exists now.

## Dependencies still excluded

Complete original Demon/Spectre admission, original player initialization/damage,
scoring and intermission, multiplayer, Fast/Nightmare timing/respawn, resurrection,
dynamic sectors, doors/specials, general infighting/projectiles, complete map
loading, demo synchronization and rendering remain deferred. See
[compatibility](COMPATIBILITY.md) for exact limits and the
[historical roadmap](validation/archive/pre-m3d3j-roadmap.md) for earlier records.
