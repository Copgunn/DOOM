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

Simulation advances one logical tic per successful unpaused call at 35 tics/second.
There is no wall-clock pacing in authoritative gameplay. Fixed-point operations
and binary angle measurements (BAM) use explicit integer semantics. Source-defined
wrapping and checked Rust rejection are documented separately. Gameplay and
miscellaneous RNG streams have independent owned indexes. Presentation cannot consume gameplay draws.

## Stepping and observation

`Simulation::advance` owns the single stepping implementation and returns unit.
`step` delegates to it, then captures one Snapshot. `snapshot` captures explicitly;
encoding and digesting are further explicit work. Paused advance calls do no tic
work or capture; paused step calls retain their historical capture. Events remain
recorded and observable on both paths. `run_commands` still collects snapshots.
`ExecutionContext::advance` delegates to that core, also through GameInstance.

Dynamic snapshot fields allocate names/value buffers and serialize events; map
fields and historical registry fields are cached and shared. Checked-Demon staging
separately copies bounded actor slots/spatial order and small owned state while
borrowing geometry/definitions. Recoverable late failures still discard that
projection in release. There is no unchecked fast mode or allocation-free claim.
The [cost inventory](M3G1_REFERENCE.md) separates these operations; measured costs
and limitations belong in [the evidence record](validation/M3G1.md).

## Actors, lifecycle and ordering

A generational actor handle identifies a slot within one simulation. Reusing a slot
cannot redirect an old target/source to its new occupant. Thinker insertion order,
slot identity and spatial head-insertion order are separate contracts. Source
BLOCKMAP ordering is retained; sorting handles or rebuilding BLOCKMAP would alter
collision decisions.

The original-ID profile also exposes an exclusive `ExecutionContext` callback.
Invariant lifetime-branded `ContextActor` values cannot escape or mix between
callbacks. They carry the existing generational handle, not another definition or
owner ID. Every access still validates current storage; tokens are not liveness
leases. Raw handle admission explicitly interprets a candidate locally and cannot
infer foreign provenance from identical numbers. This preserves legacy APIs and
diagnostics while providing a checked typed boundary for new callers.

`ActorAvailability` derives current, pending, retired, stale and invalid states
from existing slots. Pure target observation keeps stored identity distinct from
availability, health and role. Target mutation, damage, perception repair, spawn,
state entry and stepping delegate to the existing services and target-role guard.
No actor store, scheduler, reference sweep or persistent identity field is added.
Each operation retains its atomicity; the callback is not an aggregate transaction.
See the [contract](COMPATIBILITY.md#context-and-target-lifetime-contract) and
[source investigation](M3F8_REFERENCE.md).

### Game-instance provenance

`GameInstance` owns one unchanged Simulation and one opaque `InstanceId`. Its
key compares private Arc allocation identity; retained tokens prevent aliasing
with a newly allocated incarnation. Moves preserve identity. Wrapping cloned or
extracted simulation state creates a fresh identity; the owner itself is not
Clone and exposes no mutable Simulation escape. Successful restart installs the
existing fresh level state and a new key together. Setup failure retains both.

`InstanceActorRef` carries optional opaque provenance and the existing local
handle, without retaining actor/world storage. Explicit checked export/import use
the same `ExecutionContext` and invariant `ContextActor` brand. Bound scopes borrow
the owner's key; raw Simulation scopes remain unbound. Import rejects missing or
foreign provenance before existing generation/state/expected-definition checks.
Actors, relationships, collision, scheduling and ordinary tics still use local
handles; no per-actor identity mirror or per-tic provenance lookup is introduced.

This is in-process uniqueness and consistency, not authentication or persistent
identity. Explicitly attaching a trusted local key to an erased candidate is local
readmission, not recovery of its origin. There is no byte/numeric InstanceId
constructor, savegame loader or transport resolver. Keys/refcounts are excluded
from snapshots, events and RNG. See [support/trust rules](COMPATIBILITY.md#game-instance-provenance)
and [the complete audit](M3F9_REFERENCE.md).

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
order remain unchanged. A per-query check additionally rejects a saturated
INT_MIN intercept before the original clipping division could evaluate abs(INT_MIN).
The source-side quantization trigger is recorded as a defined C component; its
undefined continuation has no fabricated equivalence result. Historical numeric
policies remain unchanged.

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

## Expanded-map Demon composition

Profile 20 selects the existing geometry preparation and the native map-context
Demon registry together. Capability helpers express map statistics, expanded
geometry and the established perception/monster/lifecycle closures. Numeric
profile order never selects these services implicitly. The constructor validates
their arithmetic intersection and complete registry before returning an instance;
[COMPATIBILITY](COMPATIBILITY.md#expanded-map-demon-profile) owns the exact bounds.

There is one authoritative actor store and one scheduler. The original built-in
thinker pass is shared in `thinkers.rs`; both historical execution and the new
profile call the same movement and LifeHost services in the same linked order.
A new operation stages at most 64 slot records (including generations and links),
64 spatial handles, RNG, statistics, bindings and new observations. It borrows the
immutable world, sectors, registry, roster, sound references and canonical cache.
It does not copy a Simulation, large geometry or previous event queues.

This small actor-state plan runs once and commits only after success. The same
ThinkerStore type supplies traversal and retirement within the temporary plan;
there is no second persistent arena or AI scheduler. Later native strikes plan
against the current staged target health/RNG, including earlier owners' damage.
Public direct Look, Chase, state entry, facing, melee and damage share the boundary.
Damage needs it because source retargeting can enter Chase immediately after a
failed pain decision. Pure player scanning gathers checked results before mutation.

Every new AI sight path uses fallible expanded sight, including the INT_MIN guard.
A transient error latch unwinds void source-style callbacks and stops later work;
it is always checked before commit. No fabricated false visibility result is
committed. A later actor's failure discards staged movement, RNG, retirement,
damage, counters and observations. Original commands and both event queues remain
untouched. Layout 20 serializes actual typed state/statistics, not the plan, latch,
query budget or derived preparation data.

Batch spawning still resolves original DoomEd metadata before the native identity
adaptation and appends actors in THING order. Spatial heads remain most-recent-link
order; generational identity never selects a blocker. Restart remains a setup-time
new-instance operation, retaining RNG and rebuilding through the validated constructor.
[The investigation](M3F3_REFERENCE.md) records arithmetic/closure assumptions and
[the evidence](validation/M3F3.md) records their independently exercised limits.

## Rendering direction

A custom Vulkan-first presentation architecture is planned. No renderer or
Vulkan/wgpu runtime is implemented in this milestone. Future presentation must
consume simulation results without changing time, ordering or RNG.

For detailed historical design constraints see the complete
[pre-M3d3j architecture](validation/archive/pre-m3d3j-architecture.md).

## Canonical metadata and registry identity

The [identity API](ACTOR_IDENTITIES.md) borrows existing immutable source tables.
Private bounded canonical IDs are distinct from registry-local candidates and
from generational live actor handles. Explicit conversions compare a complete
record at its original slot. They certify that record only; downstream registry
records and executable game context still require their existing admission.
No alias is silently promoted, no second registry/store is introduced, and queries
allocate nothing and do not mutate simulation state. Profiles 1–20 retain their
existing identity encodings, state chronology and RNG contracts.

## Canonical Demon execution binding

`DemonExecutionBinding` is a borrowed proof connecting canonical MT_SERGEANT and
its normal SARG closure to Profile20's existing native definition. Admission
checks full canonical records through M3f4 resolution, then checks every bound
native descriptor and translated next edge. An explicit bounded bijection keeps
canonical definition identity distinct from registry slot identity. No second
actor store, persistent identity field or scheduler is added.

The map-context fixture now derives its native records directly from canonical
metadata, preserving its previous bytes. Profile20 construction certifies the
binding. `canonical_demon_map`, `spawn_original_demons` and
`enter_original_demon_state` delegate to existing constructor, batch and state-entry
services; existing role/health/arithmetic checks still apply. Definition queries
are immutable, allocate nothing and consume no RNG. Their bounded validation cost
is paid only by explicit requests, outside ordinary per-tic execution.

Native actor/state aliases remain serialized. Canonical observation explicitly
projects a current owner record through the verified binding; M3f4's same-record
resolver continues rejecting native136 as pristine source metadata. Source raise
metadata remains intact while native raise is suppressed. See the
[mapping](ACTOR_IDENTITIES.md#canonical-demon-execution-binding) and
[investigation](M3F5_REFERENCE.md) for the narrower integration boundary.

## Runtime identity representation boundary

M3f6 keeps ActorLife's registry-local type/state fields authoritative. The
[complete audit](M3F6_REFERENCE.md) distinguishes definition IDs, execution slots,
DoomEd numbers and generational object handles. Persisting both canonical and
native IDs would introduce synchronization invariants without solving historical
observation compatibility; no such fields are added.

Sealed native Demon views make the existing representation explicit. The borrowed
binding still proves metadata; scalar views do not certify another registry, a
live handle or state-entry permission. Existing canonical observations delegate
through the native view. One mapping supplies projection and a derived bounded
reverse lookup; queries allocate nothing. Profile20's entry/death/Fall guards use
named states through constant-time reverse lookup, without full registry validation
per tic. Earlier fixture profiles retain their policies. Thinker, spatial, RNG,
transactional planning and serialization services remain shared and unchanged.

## Original-ID registry and shared execution

The [original-ID registry](ACTOR_IDENTITIES.md#original-id-execution-registry)
certifies an immutable source metadata package and a separate context package.
Source ordinals are direct indexes; a typed definition key prevents the context
slot from becoming a fake original identity. ActorLife remains the sole persistent
identity storage. Only explicitly admitted roles execute, through existing
LifeHost, DamagePlan, ThinkerStore, movement/perception and checked Demon plans.
No second scheduler, actor store or per-monster native translation table is added.

Original-root state sets and action policies are derived once with bounded work.
They preserve damage-controlled entry and exclude raise without modifying its
metadata. Construction certifies all records; state reads do not repeat full table
validation. Historical profiles retain native definitions and guards through a
small explicit profile/role boundary. The [pre-implementation investigation](M3F7_REFERENCE.md)
records alternatives, consumer responsibilities and adapter removal criteria.

Immutable source/context schema IDs replace redundant registry rows in new
DSIM-v21 diagnostics. Complete runtime/map/event order remains observable.
The bounded decoder borrows values, validates identity namespaces and generations,
and never restores a Simulation. It explicitly dispatches v20/v21 and preserves
legacy interpretation. [STATE_FORMAT](STATE_FORMAT.md#dsim-v21--original-id-demon-diagnostics)
defines the wire contract and the limits of validation.

## Original player body ownership and damage

The [player-body policy](COMPATIBILITY.md#original-player-body-lifecycle) extends
the original-ID service family explicitly. OriginalExecutionProfile distinguishes
DemonNormalV1 from PlayerBodyLifecycleV1; the historical public Profile enum is
unchanged for exhaustive callers. The registry carries the selected capability
until construction. Simulation's optional OriginalPlayer record then identifies
the policy without another persistent actor-type or profile mirror.

PlayerSlot identifies the sole source player, independently of ActorId slots.
OriginalPlayer.mo references the existing generational actor owner; reverse
association derives from exact handle equality. A checked initial canonical0 body
binds once. ExecutionContext checks the current branded token and GameInstance
checks incarnation provenance before import. Existing command bindings, synthetic
perception records and historical WeaponPlayer remain separate contracts.

Source-required player health clamps0 while mobj health stays signed. Shared
DamagePlan/LifeHost updates player fields before actor health; death immediately
clears solidity, sets PST_DEAD and records the source weapon-lowering request
before body death-state entry. The small player record is staged and committed
with actors/RNG/statistics/events, so late failure rolls everything back. No actor
store, scheduler, damage engine, player-specific tic loop or per-tic diagnostic
capture is added. Existing contexts hold no player heap allocation; measured
scalar/branch and allocation effects belong in [the record](validation/M3G2.md).

Death leaves the actor allocated and bound; removal is explicit and deferred.
Stored mo/attacker retain their original handles after retirement, and availability
comes from the existing store. Generations prevent rebinding to a new occupant.
C quarantine/opaque-pointer observations and Rust generation safety are distinct.
Unsupported physical inflictors, player inputs and live Demon AI reject before
execution. The fixed unlisted context avoids pretending that PlayerThink, a roster,
weapon lowering animation or a death camera is implemented.
