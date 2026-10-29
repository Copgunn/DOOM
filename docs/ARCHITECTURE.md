# Architecture

Rust DOOM separates validated input, owned authoritative simulation and eventual
presentation. Compatibility comes from original-source investigation and independent
comparisons within explicit profiles, rather than mechanically copying C globals.

## Crate boundaries

| Crate | Responsibility |
| --- | --- |
| [`doom-types`](../crates/doom-types/src/lib.rs) | Fixed-point arithmetic, original RNG table and owned stream indexes |
| [`doom-wad`](../crates/doom-wad/src/lib.rs) | Bounded WAD directory/resource access without assuming assets are executable |
| [`doom-map`](../crates/doom-map/src/lib.rs) | Validated classic map records, BSP, REJECT and source BLOCKMAP data |
| [`doom-sim`](../crates/doom-sim/src/lib.rs) | Headless deterministic gameplay profiles, actors, collision and canonical diagnostics |

The preserved [original source](../linuxdoom-1.10/) is behavioral reference, not a
runtime dependency. The Rust workspace forbids unsafe code and global mutable game
state. Each simulation owns its RNG, actors, counters and ordered observations.

## Deterministic time and numbers

Simulation advances exactly one logical tic per successful step at 35 tics/second.
There is no wall-clock pacing in authoritative gameplay. Fixed-point operations
and BAM angles use explicit integer semantics; source-defined wrapping and Rust
checked rejection are documented separately. Gameplay and miscellaneous RNG streams
have independent owned indexes. Presentation cannot consume gameplay draws.

## Actors, lifecycle and ordering

A generational actor handle identifies a slot within one simulation. Reusing a slot
cannot redirect an old target/source to its new occupant. Thinker insertion order,
slot identity and spatial head-insertion order are separate contracts. Source
BLOCKMAP ordering is retained; sorting handles or rebuilding BLOCKMAP would alter
collision decisions.

State entry installs frame/timer and executes its action immediately, including
nested Look-to-Chase or Chase-to-melee transitions. Raw spawning initializes state
without executing its action. Damage, death state entry, delayed sounds, Fall and
terminal corpses compose the same lifecycle machinery. A corpse retains its actor
and thinker identity; completion of its animation does not imply retirement.

## Map context

Profile17 composes the profile16 THING path with source `MF_COUNTKILL`, initialized
level totals and the original single-player unbound-source kill-credit branch.
Available map kills and credited deaths are separate signed counters. Spawn totals
change after source timer RNG; death credit follows corpse initialization and
precedes death-state entry. Removal does not decrement available kills.

Fresh restart reuses the same validated constructor and batch, creates a new actor
store, clears level statistics/tic and retains RNG state as source P_SetupLevel
does. Old instances are unchanged; handles remain scoped to their own simulation.
It does not implement complete game reload, player starts or intermission.

## Validation and failure atomicity

Immutable map/registry data and bounded plans validate roles, references, capacity,
action closure, arithmetic and source dependencies before authoritative mutation.
Damage uses one shared plan/execution service. Multi-Demon melee evaluates the
current body health and RNG after earlier thinkers; it does not reuse stale plans.
There is no full-simulation clone per tic. Setup may construct a bounded fresh
instance, and the generic user thinker budget has a separate nontransactional
contract from built-in simulation operations.

[DSIM](STATE_FORMAT.md) serializes canonical fields and ordered diagnostics in
versioned schemas. Historical profiles remain frozen. Source-C equivalence,
independent encoding and Rust-specific safety tests serve different purposes;
see [validation](VALIDATION.md).

## Rendering direction

A custom Vulkan-first presentation architecture is planned. No renderer or
Vulkan/wgpu runtime is implemented in this milestone. Future presentation must
consume simulation results without changing time, ordering or RNG.

For detailed historical design constraints see the complete
[pre-M3d3j architecture](validation/archive/pre-m3d3j-architecture.md).
