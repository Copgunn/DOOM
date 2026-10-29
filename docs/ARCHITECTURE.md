# Architecture

Rust DOOM separates validated input, owned authoritative simulation and eventual
presentation. Compatibility comes from original-source investigation and independent
comparisons within explicitly selected simulation configurations, rather than
mechanically copying C globals. The [glossary](README.md#short-glossary) explains
terms such as profile, fixture and admission.

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
and binary angle measurements (BAM) use explicit integer semantics. Source-defined
wrapping and checked Rust rejection are documented separately. Gameplay and
miscellaneous RNG streams have independent owned indexes. Presentation cannot consume gameplay draws.

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

Each simulation owns its level statistics alongside actors and RNG. Map placement
and shared damage/lifecycle services update those counters; there is no separate
statistics world, actor store or scheduler. Available map kills and credited deaths
have distinct ownership and meaning.

Fresh restart builds a new simulation through the existing constructor and THING
batch, keeping the old instance intact on failure. Handles remain scoped to their
own simulation. [COMPATIBILITY](COMPATIBILITY.md) defines the supported reset and
statistics behavior; the [source investigation](M3D3J_REFERENCE.md) explains exact
chronology, and [STATE_FORMAT](STATE_FORMAT.md) owns the encoding.

## Validation and failure atomicity

Immutable map/registry data and small, bounded plans validate roles, references,
capacity, reachable actions, arithmetic and source dependencies before changing state.
Damage uses one shared plan/execution service. Multi-Demon melee evaluates the
current body health and RNG after earlier thinkers; it does not reuse stale plans.
There is no full-simulation clone per tic. Setup may construct a bounded fresh
instance, and the generic user thinker budget has a separate nontransactional
contract from built-in simulation operations.

[DSIM](STATE_FORMAT.md) serializes canonical fields and ordered diagnostics in
versioned schemas. Historical profiles remain frozen. Source-C equivalence,
independent encoding and Rust-specific safety tests serve different purposes;
see [validation](VALIDATION.md).

## Bounded larger geometry

The opt-in map-scale configuration reuses `StaticWorld`, sector state, the actor
store and existing movement/sight services. Its capacities and excluded gameplay
are owned by [COMPATIBILITY](COMPATIBILITY.md#map-scale-profile).

Preparation compiles each supplied cell into a vector of linedefs in original
first-occurrence order. Reverse word-position lookup handles shared lists/suffixes
without repeated raw scans. Sorting preparation metadata by original word position
does not sort gameplay candidates by line ID. Checked total-reference accounting
and fallible reservations precede installation; source BLOCKMAP is never rebuilt.

Preparation membership is a multiword bit matrix sized from actual cells and
lines. Query-local deduplication uses a bounded multiword stack set; membership
bits never choose traversal order. Topology retains source first-seg sector
ownership and indexed sector vectors. The existing spatial index preserves
head insertion independently from thinker order.

Preparation has separate storage and CPU bounds. Let L be linedefs, C cells, W raw
words, K distinct list starts, T terminators and E compiled references. Reverse
lookup, line-marker resets, per-start ordering and coverage diagnostics take
`O(W log C + T*L + K*L log L + E + L*C)` work in the conservative worst case.
Temporary membership/storage is `O(C + L + E + C*ceil(L/64))`. The historical
algorithm already reset/scanned its fixed line table; increasing the admitted line
count raises that work ceiling. All dimensions are separately capped in
[COMPATIBILITY](COMPATIBILITY.md#map-scale-profile). The reference budget bounds
list expansion/copying, while raw-word and line caps also bound unused-terminator
work. Avoid treating one budget as a bound on every stage. A query's membership
check is constant-time and never chooses candidate order. These bounds describe
collision/topology preparation; parsing and canonical-map cache construction add
their separately bounded decoded-table work.

Arithmetic admission runs before authoritative movement. The new untyped profile
explicitly selects the earlier static-sector canonical field layout under a new
version tag; numeric version order cannot implicitly enable typed actors. Immutable
preparation budgets and derived storage metrics are not redundant persistent actor
state. Snapshot fields remain defined only in [STATE_FORMAT](STATE_FORMAT.md).

## Expanded arithmetic and spatial work

The separate map-extent profile adds immutable admission analysis to the same
world, movement, spatial and sight services. [COMPATIBILITY](COMPATIBILITY.md#map-extent-profile)
owns its exact domains and resource ceilings. [M3F2_REFERENCE](M3F2_REFERENCE.md)
derives them from original source expressions, including intercept sums; representable
classic coordinates alone do not establish arithmetic safety.

World coordinates, pairwise differences, line-delta norm, node deltas, heights,
actor margins and momentum have separate checks. Those checks make overflow in
reachable original arithmetic unnecessary; wider temporaries are not used to
invent results for overflowing C. Existing wrapping numeric primitives retain
historical contracts. Original FixedDiv guard/rounding, BSP ties and XY/Z action
order remain unchanged.

Setup conservatively charges native-size preparation requests and a collision
work proof before constructing a simulation. Supplied BLOCKMAP cells still compile
into source first-occurrence lists; membership bits never choose traversal order.
Cells, raw words, expanded references and memory remain independent dimensions.
The preparation complexity above still applies to the expanded cell domain.

Sight memoizes subtree costs but adds both child references, so a shared DAG counts
its full expansion. A derived root cost is cached in the immutable world and used
by the query work assertions and fallible scratch reservations together. Its
observation bound is root cost plus five records per unique linedef; the task
stack is bounded by node count. REJECT needs neither scratch nor traversal. These
caches and limits are not canonical actor state.

Spawn/control/query validation and an all-live-actor tic preflight precede RNG,
event consumption and movement. Public world BSP queries enforce the new domain
too. No full simulation copy or second physics/actor/spatial service is introduced.
Historical profile selection keeps its old limits, and the new untyped canonical
tag cannot implicitly enable gameplay fields. Allocation exhaustion for existing
tic diagnostics/snapshots retains its documented process-level policy.

## Rendering direction

A custom Vulkan-first presentation architecture is planned. No renderer or
Vulkan/wgpu runtime is implemented in this milestone. Future presentation must
consume simulation results without changing time, ordering or RNG.

For detailed historical design constraints see the complete
[pre-M3d3j architecture](validation/archive/pre-m3d3j-architecture.md).
