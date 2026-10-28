# Architecture

## Scope and decisions

M0 establishes the workspace alongside untouched original source. M1 implements
bounded archive/map parsing. M2 introduces the fourth crate, `doom-sim`, with a
functioning headless deterministic foundation. M3a adds an explicit player/solid
collision profile, documented in [M3A_REFERENCE.md](M3A_REFERENCE.md). M3b adds
static-sector topology and XY/Z physics; see [M3B_REFERENCE.md](M3B_REFERENCE.md).
M3c adds admitted typed actor lifecycle; see [M3C_REFERENCE.md](M3C_REFERENCE.md).
A playable client and renderer
remain future work. Source findings were recorded before implementation in
[M2_REFERENCE.md](M2_REFERENCE.md).
The C source is an authoritative behavioral reference, not a Rust design template.

Current Cargo dependency edges (arrows mean “depends on”):

```text
doom-wad -> doom-types
doom-map -> doom-wad, doom-types
doom-sim -> doom-map, doom-types (doom-wad is a test/example dependency only)
```

- `doom-types`: 35 Hz constant, lossless `LumpName`, signed 16.16 `Fixed`, and
  owned exact-table `DoomRng`. No I/O or third-party dependencies. Arithmetic
  width, wrap, shift, division guard, binary64 helper and rounding are explicit.
- `doom-wad`: `Wad<'a>` borrows immutable caller-owned bytes and owns validated
  directory metadata. `Lump` ranges are private; `data(index)` checks indexes.
  `WadSet` borrows archives in load order and returns archive/directory locations,
  preserving original entries and last-match precedence without hash ordering.
  The old `LumpInfo` remains explicitly unvalidated vocabulary; it cannot grant
  access to archive bytes. Parsing itself performs no file I/O.
- `doom-map`: `Map::load` selects the overriding marker and decodes its contiguous
  classic lump group within that archive. It owns immutable geometry and exposes
  slices, keeping validated references intact. Signed coordinates, heights,
  offsets and numeric bit patterns are retained without floating-point conversion.
  Every binary field is decoded explicitly; Rust struct layout is not disk layout.
- External byte acquisition belongs to the caller/future client. Ignored tests
  read from `DOOM_IWAD_DIR` without printing private names/paths. Provisioning
  private assets into a cloud environment requires separate destination consent.
- `doom-sim`: owns `StaticWorld`, `ThinkerStore<Actor>`, RNG, four ordered command
  slots, logical tic/pause state, and deterministic observations. `Simulation::step`
  takes commands only. No OS clock, renderer, audio, filesystem, or GPU dependency
  participates. `StaticWorld` rejects unsupported multi-sector/portal/special
  profiles and compiles fixed line predicates from validated M1 records.

Actor identities are store-scoped u32 slot + u64 generation. Lowest reusable
slots are chosen deterministically; generation exhaustion retires slots. Safe
linked handles define append order independently of physical slots. Marked
storage remains readable until its visit; retirement saves the next handle
before unlinking. Callback contexts allow append/mark/value updates but not
reentrant traversal or clear. Retirement observers are read-only. Generic
traversal has an explicit visit budget and retains completed callbacks on budget
failure; built-in motion thinkers never spawn, so the actor bound covers each step.

Players/input foundations precede thinkers, then Specials/Respawn phase markers
and tic increment. Full player/special/respawn logic is not implemented. M2
controlled movers use the non-player ground response. In M3a bindings select
player-specific XY sliding/friction; solidity remains independent. Full player
thinking and state machinery remain absent. Pause freezes the whole step before
commands/RNG/events/time; invalid buttons/tic exhaustion reject before mutation.
Setup spawn/mark/random observations queue for the next unpaused tic. Presentation
reads immutable snapshots/events without draining them.

Canonical state includes every logical slot/generation/link, actor parameters,
processing order, commands/bindings, RNG, tic/pause, last/pending events and static
map tables. Immutable map fields are shared internally, with no addresses in the
encoding. Read-only BLOCKMAP word/range views let snapshots encode shared lists
once in linear space. See [STATE_FORMAT.md](STATE_FORMAT.md).

M3a's small-profile spatial index keeps most-recent-link order, filtered by center
cell in X-outer/Y-inner queries. This reproduces independent cell-head lists even
when actors move within a cell. It uses validated generation handles, not pointers,
handle sorting or timestamps. All live actors, including off-map centers, retain
logical link order; queries exclude out-of-map cells. Spawn reserves index capacity
before actor/RNG mutation, removal unlinks immediately, and relinking does not
allocate. Thinker append order and spatial order are independent state.

The M3a profile caps 128 lines, 4096 cells, 65536 raw BLOCKMAP words and 1024
logical actor slots. Reverse next-occurrence compilation handles shared/overlapping
cell lists without repeated raw-list scans, preserving first occurrence order.
Derived lists and geometry are reproducible from map fields/profile and are not
additional canonical state. Coverage checks conservatively require line bounding
boxes in cells; valid vanilla diagonal lists can fail this declared restriction.
Sliding uses a bounded 64-iteration fixed DDA, stable intercept ties, three corner
traces, at most two projections and a stair-step fallback. No mutable C globals
or graphics dependencies are introduced.

The error audit found no public path to M2's apparent late movement errors.
Private immutable map/actor invariants make validated movement infallible; all
profiles check recoverable step failures before mutation. Setup failures preserve
logical state. Internal assertions identify implementation invariant defects;
allocation exhaustion is process-level, not a recoverable simulation error.
Generic user thinker callback budgets remain explicitly nontransactional.

M3b adds compiled leaf-to-sector topology and an independent runtime sector-height
vector. The public sector view is read-only; there is no moving-sector/height-
mutation API in this milestone. Iterative BSP traversal follows original child
ordering and side predicates, bounded by validated node count. Location is not
polygon containment; M1 reference/cycle validation is distinct from proving a
node builder's geometry. M3b caps sectors at 128 and nodes/leaves at 4096, retains
the M3a line/grid/actor bounds, and rejects inverted or globally overflowing height
spans. Zero-clearance sectors can block openings but cannot host an actor too tall.

Each destination starts constraints at its BSP center, checks things then ordered
lines, accumulates two-sided floor/ceiling/dropoff values and evaluates exact
TryMove restrictions. Success changes XY/constraints and relinks both spatial and
sector lists, without changing Z until the actor's later Z phase. Global logical
link order reconstructs source cell heads and sector heads by filtering current
membership, including sector-only off-map actors. Removal excludes both immediately.
Generational identities and thinker append order remain independent.

Player command turning always applies; airborne thrust is suppressed. Each thinker
runs supported XY first, skips airborne ground friction, then conditionally runs
Z/gravity/clipping. MF_DROPOFF and MF_NOGRAVITY are explicit setup choices. Supported
ordinary collision is the source XY-only predicate, not modern Z-stacked actors.
View/animation/audio side effects and full actor/player state logic remain absent.

M3b intentionally uses supplied BLOCKMAP lists with the same compiled first-
occurrence order. Conservative bbox omissions are reported, not rejected or
rebuilt: legitimate sparse diagonals and deliberately missing walls are compared
with original C. This is explicit profile-3 behavior, not a change to M3a's strict
profile. The diagnostic does not prove spatial correctness. Prepared membership
masks keep omission counting bounded without rescanning shared raw lists.

DSIM-v3 includes momz, flags, sector/leaf references, runtime heights/list order,
BLOCKMAP policy and ordered opening/vertical observations. Old schemas remain
byte-identical. Recoverable failures stay pre-mutation; no full-state clone occurs
per tic. Legacy flat query helpers reject M3b requests lacking sector/actor context.

The stable workspace has no third-party dependencies. The separate `fuzz/`
package uses `libfuzzer-sys` solely for instrumented parser testing; its lockfile
does not affect stable builds. Add dependencies only for a concrete need,
with rationale, license review, platform impact, and determinism implications.
Use stable Rust, edition 2024, minimum 1.85, and Cargo resolver 3. Workspace
lints currently forbid unsafe code and warn on undocumented public APIs.
Do not add empty crates for the future components below.

## Intended boundaries

| Future component | Owns | May consume |
| --- | --- | --- |
| `doom-sim` (M2/M3a subsets exist) | Per-tic state, ordered thinkers; fuller gameplay in M3 | `doom-types`, validated `doom-map` data |
| `doom-demo` | Versioned demo command decoding/encoding and replay | `doom-types`; replay drives `doom-sim` through commands |
| `doom-render` | Presentation snapshots and renderer interface | Immutable simulation output and resource views |
| `doom-render-software` | Minimal diagnostic software/headless path | Renderer interface |
| `doom-render-wgpu` | Custom hardware presentation/GPU resources, Vulkan preferred | Renderer interface; platform surface |
| `doom-platform` | OS events, clocks, windows, input acquisition | Platform libraries, no gameplay authority |
| `doom-audio` | Playback and mixing | Ordered sound events and resources |
| `doom-client` | Composition, startup, resource loading, scheduling | All required boundaries through explicit APIs |
| `doom-compat` | Reference profiles, state comparisons, diagnostics | Simulation/replay public APIs and legal test data |

Keep dependencies acyclic. Resource bytes belong to the
archive/resource layer; validated map structures belong to the map layer;
runtime actors and mutable sector state belong to simulation. Do not embed GPU,
windowing, filesystem, or audio objects in gameplay data.

## Data flow and determinism

```text
archive bytes -> bounded WAD view -> decoded/validated map -> simulation state
OS input or demo -> ordered per-tic commands -> simulation.step -> snapshots/events
snapshots -> software/headless or wgpu presentation
events -> audio; recorded commands/state -> replay diagnostics
```

The future client scheduler accrues time and executes exactly **35 logical tics per
second**. A simulation step accepts explicit commands, never frame duration or a
wall clock. Rendering frame rate, interpolation, audio timing, and I/O completion
must not affect tic outcomes. Catch-up policy and command assignment must be
specified and tested when the scheduler is implemented.

Preserve vanilla fixed-point/integer semantics and the precise RNG table,
indices, reset points, and call sequence. Keep gameplay RNG separate from
cosmetic RNG. Preserve deterministic actor/thinker insertion, update, and removal
ordering; stable identifiers alone do not guarantee stable iteration. Use
explicit ordered storage. Never depend on randomized hash iteration, pointer
addresses, thread scheduling, or unordered parallel reductions in gameplay.
Rendering can interpolate immutable snapshots but cannot mutate simulation or
consume its RNG. A custom wgpu renderer is planned, preferring Vulkan on supported
platforms; a minimal software or headless path must support automated verification.

## Ownership and input safety

Prefer safe Rust and explicit owners over global mutable state. Future actor
relationships use validated handles/stable identifiers with a specified stale
handle policy (for example, generation checks); avoid raw pointers between actors.
Any future unsafe code requires a deliberately reviewed lint-policy change,
documented safety invariants, justification, and tests.

Treat archive and demo data as untrusted. Validate signed fields before unsigned
conversion, checked offset/size arithmetic before slicing, record widths before
decoding, allocation limits before allocating, and every index/cross-reference
before traversal. Reject malformed input with structured errors, without panic,
out-of-bounds access, or attacker-controlled unbounded allocation.

`Limits` bounds archive/lump bytes and directory entries before allocation;
`MapLimits` bounds map lump bytes, record counts, BLOCKMAP words/cells, and total
requested decoded plus scratch bytes. Vectors use fallible reservation. Caller
input acquisition and caller-initiated clones are outside parse allocation policy.
Defaults and intentional strictness are listed in [compatibility](COMPATIBILITY.md).

Validation is linear in input records/words/cells. Iterative three-color BSP
traversal checks every node, including unreachable cycles, without recursion.
Subsector sector-consistency checks use a prefix transition count, so overlapping
ranges cannot multiply work. BLOCKMAP suffix validation is memoized once per
word, so shared offsets neither multiply validation work nor duplicate lists.
No geometry traversal occurs until all indexes and ranges are validated.

Compatibility is an observable behavior contract, not permission to reproduce C
undefined behavior. Specify ambiguous behavior with pinned reference evidence;
document safe deviations. See [compatibility](COMPATIBILITY.md) and
[test extension rules](TESTING.md).


## Future presentation direction

A custom wgpu renderer is planned, with Vulkan preferred on supported platforms. GPU-driven rendering, clustered
Forward+ lighting, colored dynamic lights, selective shadows, emissive materials,
and optional atmospheric/volumetric effects are design candidates, not implemented
features or performance results. Presentation consumes immutable state/events and
must not influence RNG, collision or tic order. No graphics work is part of M3a.

## M3c typed actor lifecycle

Profile 4 composes the static M3b physics with immutable `ActorRegistry` metadata
and per-actor `ActorLife`. Registry definitions, action IDs and service dispatch
are separate. Complete source tables remain recognizable; type admission walks
spawn/death closures and rejects unavailable actions, unsupported flags and
map-wide dimension failures. Native `ProbeAction` capabilities test the same
restricted store context, with no raw pointers, global state or parallel scheduling.

Before players or physics, a bounded state-only pass calculates conservative
entry/spawn demand and reserves storage. It makes no full simulation/map clone.
Recoverable graph/capacity/work failures preserve canonical state and RNG. For
admitted tics, each actor runs XY, Z, countdown and state actions in order. Actions
may recursively enter states or append/mark actors through a restricted host.
Appended children can think in that traversal; removal immediately unlinks and
unbinds input, while storage retirement follows linked thinker scheduling.

Targets use generational IDs. Pending storage remains resolvable; retirement
invalidates resolution without eagerly clearing references or aliasing a reused
slot. Full tracing/owner/projectile relationships remain later work. Definition
and native service boundaries can support future compatibility adapters, but no
DECORATE/ACS/ZScript or PK3 implementation is present. Enhanced behavior needs an
explicit profile and licensing review. See [M3C_REFERENCE.md](M3C_REFERENCE.md).
