> Historical document at verified pre-M3d3j master. Capabilities, pending review statements and rendering plans describe that observation. Use the current guides for present status. Exact original bytes are in [pre-m3d3j-architecture.txt](pre-m3d3j-architecture.txt).

# Architecture

## Scope and decisions

M0 establishes the workspace alongside untouched original source. M1 implements
bounded archive/map parsing. M2 introduces the fourth crate, `doom-sim`, with a
functioning headless deterministic foundation. M3a adds an explicit player/solid
collision profile, documented in [M3A_REFERENCE.md](../../M3A_REFERENCE.md). M3b adds
static-sector topology and XY/Z physics; see [M3B_REFERENCE.md](../../M3B_REFERENCE.md).
M3c adds admitted typed actor lifecycle; see [M3C_REFERENCE.md](../../M3C_REFERENCE.md).
M3d1 adds bounded explicit damage/hitscan; see [M3D1_REFERENCE.md](../../M3D1_REFERENCE.md).
M3d2 adds player weapons/psprites; M3d2b composes explicit plasma launches,
missile collision and shared damage in profile 7. A playable client and renderer
remain future work. Source findings were recorded before implementation in
[M2_REFERENCE.md](../../M2_REFERENCE.md).
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
once in linear space. See [STATE_FORMAT.md](../../STATE_FORMAT.md).

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
Defaults and intentional strictness are listed in [compatibility](../../COMPATIBILITY.md).

Validation is linear in input records/words/cells. Iterative three-color BSP
traversal checks every node, including unreachable cycles, without recursion.
Subsector sector-consistency checks use a prefix transition count, so overlapping
ranges cannot multiply work. BLOCKMAP suffix validation is memoized once per
word, so shared offsets neither multiply validation work nor duplicate lists.
No geometry traversal occurs until all indexes and ranges are validated.

Compatibility is an observable behavior contract, not permission to reproduce C
undefined behavior. Specify ambiguous behavior with pinned reference evidence;
document safe deviations. See [compatibility](../../COMPATIBILITY.md) and
[test extension rules](../../TESTING.md).


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
explicit profile and licensing review. See [M3C_REFERENCE.md](../../M3C_REFERENCE.md).

## M3d1 combat services

Explicit profile 5 composes the accepted typed lifecycle and static-sector world.
`combat_trace` performs pure original fixed-point path collection, source cell/
line/thing ordering, stable fraction sorting, aim-window clipping and shot hit
selection. `combat` validates identities/admission, plans damage/state demand,
reserves effect/callback slots, then applies source-ordered mutations through the
existing `LifeHost`. Source and inflictor are independent optional handles.
Neither service invents a weapon damage generator, persistent attack queue or
player weapon state. `step` still takes only tic commands; attack buttons remain
M3d2. Callers issue explicit APIs between logical tics in deterministic order.

Damage planning conservatively sums possible pain and see callbacks without
advancing RNG. Kill planning chooses the source threshold/state in advance.
Tracing and all recoverable validation complete before events, RNG or physical
mutation. Real blood/puff types and reachable states must retain source metadata;
the constructor validates their map-wide dimensional admission. LifeHost then
spawns the actual thinker before damage. `pending_events()` exposes observations
read-only. Failed supported API calls preserve the entire canonical state.
No full Simulation/Map clone is used per attack or tic; bounded result/trace
copies preserve independent API/event observations.

Bounds compose prior limits: 128 lines/sectors, 1024 actors, 4096 map cells,
65536 BLOCKMAP words, 64 DDA visits, 128 intercepts, positive range <=2048 fixed
units, damage 0..=1000, 128 entries per closure and 8192 entries per built-in
step. Per trace, at most 64*(128+1024) candidate observations and 64*1024 ordered
actor scans are permitted. Handles resolve by slot/generation in O(1); stable
sorting is O(128 log 128). Pending removal does not supply preflight capacity.
Conservative preflight may reject a branch that would not actually draw pain;
this is an explicit safety policy. There is no optimization superiority claim.

DSIM-v5 adds source threshold and structured combat observations. Existing
health/flags/momentum/state/targets, storage, spatial order and RNG already cover
persistent consequences. Presentation can observe these records at any frequency
without consuming RNG or altering gameplay. Custom wgpu/Vulkan, native resources
and future scripting adapters remain separate planned systems; no graphics,
physics, scripting or third-party runtime dependency was introduced.

## M3d2 player weapons

Explicit profile 6 owns one `WeaponPlayer` record bound to any of the four ordered
input slots, two source psprites and persistent generation-safe sector sound targets.
Psprites share validated source state metadata with actors but use a separate typed
weapon action interpreter; they are never thinkers. Source states/types remain
immutable except the declared type-136 player-state fixture properties. Actor action
admission remains separate and unchanged in older profiles.

The interpreter runs after reaction-time/motion/bob and command selection, before
actor thinkers. It updates weapon then flash, copying overlay positions last.
Body PLAY/RUN/ATK transitions use the existing lifecycle host. Friction restores
PLAY through that same host. Source weapon sounds reuse inert actor sound events.
Noise propagation visits source-ordered sector lines, allows at most one blocking
line and persists only soundtarget handles; per-call traversal caches are derived.

Recoverable weapon failures are preflighted on only a small copied player record,
preview pawn, two RNG indexes and an overlay of at most seven touched targets.
The same pure combat trace functions accept a read-only actor lookup for predicted
kill flags/height. Exact source pawn first pain/death frames have no immediate
callbacks; preflight accounts for their pain/kill/forward-fall draws and four real
impact draws per hit. No full simulation, map or actor-store clone is used. Runtime
replays the admitted interpreter and existing combat pipeline; a discrepancy is
an invariant bug, not a safe partly committed tic error.

Capacity is at most 64 actors, with conservative reservation of one impact per
bullet (seven per shotgun group), no pending-retirement credit, 128 psprite entries
per operation, and the inherited 8192-entry thinker budget. At most seven shots
and three autoaim probes run per firing group; each trace retains the 64-cell/
128-intercept limits and source ordering. Original actor states cannot spawn/remove
through native probes in profile 6. Sector noise visits each sector at most twice
with at most 128 ordered line checks per visit. There is no optimization superiority
claim. Source spread RNG is sequential within each pellet and its effects/damage.

The private weapon combat route admits only the initialized shooter as bound
source/inflictor. Public direct APIs and damage targets retain bound-player
protection. Setup ownership/ammo is explicit and validated; full inventory/pickup,
player damage/powers/camera/respawn, AI and projectile systems remain separate work.
Renderer/audio/scripting cannot advance or alter player weapon state.

## M3d2b projectile composition

`projectiles.rs` owns source-specific launch, missile contact predicates and impact
sequencing. Actors remain ordinary typed ThinkerStore entries. A shared internal
cursor implements the same append/retirement semantics as public `run`, allowing
an actor thinker to call existing damage services without aliasing a callback's
mutable store borrow. The synthetic same-pass test invokes the launch API during
a source thinker; no monster/weapon firing action is admitted.

M3b's cell/head iterator and line/opening/fit kernel are shared. Profile 7 supplies
the original missile thing predicate and ignores BLOCKING/BLOCKMONSTERS only
while MISSILE remains set. It applies the resulting direct hit through M3d1 before
recording the move decision and invoking explosion. LifeHost handles spawn, state
entry/countdown, immediate spatial unlink and deferred retirement. Source
NOBLOCKMAP missiles retain sector membership but never enter thing candidates.

Exact original states/types plus bounded native player-state fixtures establish
that impact actions cannot spawn/remove actors, call unsupported combat, award
scores or drop items. Launch validates all prerequisites and reserves one slot
before authoritative effects. Step validates handles, current arithmetic bounds
and existing state demand before mutation. There is no automatic Simulation, Map
or arena clone per tic. ActorLife.target is the source owner; the intended victim
is not persistent. DSIM-v7 composes v5 persistent fields and event tag 15, without
v6-only player/noise fields or redundant missile state. See the reference and
schema for precise admission and ownership policies.

## M3d3a perception composition

Profile 8 composes the v5 typed lifecycle and M3b static-sector XY/Z services with
pure sight, a minimal perception roster and the existing v6 noise service. It
admits no weapon player, projectile API or player damage. Schema versions are
explicit profiles rather than successive feature supersets.

`sight.rs` consumes immutable validated geometry, current sectors and copied actor
records. A local stack reproduces P_CrossBSPNode continuations and seg order;
a local linedef mask replaces source stamps. REJECT returns before eye/slope
initialization. No renderer, BLOCKMAP hitscan, geometry rebuild or persistent
sight cache participates. Expanded traversal cost is proved at construction,
including repeated DAG leaves/segs; actor arithmetic is preflighted before use.

`perception.rs` owns the four active/health/optional-ID records and source scan
planning. Successful target writes use LifeHost; read-only sight stays independent
of roster/sound validity. A read-only `PerceptionInputs` view lets the existing
thinker/state host execute the same A_Look as explicit `look`. There is no second
scheduler or allocator. Noise accepts distinct target/emitter inputs through the
shared source propagation implementation; old weapon calls pass the shooter twice.

Registry validation proves see-state actions and whole spawn/death closures before
admission. Labelled native lookers isolate A_Look from unsupported immediate
A_Chase. Stable referenced actor states cannot remove themselves mid-tic. Step
preflight checks active references, future position bounds and every retained
sound identity when a looker could run, covering sector changes before dispatch.
Failure precedes movement, target/lastlook/threshold changes, events and RNG.
Only bounded records, traversal stacks and plans are used; no full-state tic clone.
See [the final admission contract](../../M3D3A_REFERENCE.md) and [v8 encoding](../../STATE_FORMAT.md).

## M3d3b pursuit composition

Profile 9 composes profile 8 perception/noise with a constrained non-attacking
walking actor profile. `direction.rs` represents source dirtype_t and exact fixed
components. `pursuit.rs` implements P_Move/TryWalk, labelled registry admission,
small setup/preflight services and ordered observations; `pursuit/directions.rs`
and `pursuit/chase.rs` contain finite source direction search and A_Chase.

LifeHost uses a read-only perception context derived from the selected profile
to permit native Chase callbacks. A_Look can enter the Chase see state immediately;
Chase target loss can enter Look through the same recursive state-entry host.
Native positive-duration states and stable body references bound that callback
chain. No alternate allocator, scheduler or collision engine is introduced.

A direct walk calls the existing M3b check_move with current ActorAccess/spatial
order and immutable geometry/sectors. Success changes XY/floor/ceiling/location,
relinks even in the same cell, clears INFLOAT and snaps Z to the contacted floor.
The shared movement/vertical events retain their layouts. Ordinary momentum XY/Z
still precedes state countdown/actions; direct walking is an immediate positional
effect inside the action. Sector membership derives from the existing relink order.

Constructor validation rejects specials, float/attack metadata and unbounded work.
Only exact source player bodies or labelled native types 19/21/136 execute in this
profile. Native copies explicitly clear the source player's missile-animation
reference. Runtime state admission confines bodies to non-removing player/native
states, so earlier thinkers cannot retire a referenced actor during a tic. Preflight
proves counters and margins for 30 units physical+16 direct movement and validates
active roster, retained sounds and current targets before side effects. Missing/
nonshootable targets take the source scan/spawn path; stale generations reject.
The registry comparison clone happens only at construction; tics use bounded local
records/queries, never a full Simulation/Map clone. v6 weapons, v7 missiles,
player damage, doors, floating AI and attacks remain separate.

## M3d3c attack composition

Profile 10 adds no attack scheduler or persistent decision cache. `monsters.rs`
provides bounded range/selection/facing actions; Chase calls its selection at the
original point, then uses the existing LifeHost for immediate state entry. The
same positive 10/8/8 animation progresses in normal thinker order after XY/Z.
Melee metadata uses a labelled inert marker, not an unimplemented damage callback.

Combat preparation is separated from execution. M3d1's original trace/damage
planning stays read-only on Simulation. Its authoritative impact/damage code now
executes generically through LifeHost/ActorAccess, shared by older explicit combat
APIs and the native attack callback. Existing effects, pain/death, target, spatial
and thinker ownership are reused, with source events/RNG preserved.

One unshootable floor-standing native owner is always thinker head and has no
physical momentum. This proves that its pose and RNG are unchanged before a
planned delayed PosAttack. Preflight copies only owner/RNG, computes exact facing,
aim, spread and damage, prepares the existing trace/damage plan and reserves real
impact capacity. Positive attack durations prevent selection from immediately
firing. The action performs source RNG/sounds/effects; damaged targets and appended
impacts then run later in the same source thinker pass. Direct entry/actions use
the same preparation. No whole Simulation/Map/store clone occurs per tic.

Only native type 136 executes PosAttack; isolated native source type identities exercise
probability branches without enabling those monsters. Dummy type 135 source body states
cannot recursively chase/attack. Active/sound/current attack references use stable
non-removing roles; effect identities may retire/reuse. Bound-player protection,
separate roster health and earlier profile semantics remain intact. Complete
monster closures, advanced attacks and dynamic-world services remain deferred.

## M3d3d sequential preflight and SPOS execution

Profile 11 reuses reserved native136 in a distinct exact registry adaptation, with
SPOS 10/10/10 states 4/5/6, fullbright 32773 and native Look/Chase handoffs. It adds
no AI scheduler, actor registry expansion or persistent shot cache. Existing
positive state timing and first stationary floor-owner proof determine exactly
which delayed burst must be prepared before a tic or direct state entry.

Shared combat planning now accepts an optional actor lookup. Existing callers use
base actors with unchanged semantics. Shotgunner preparation instead reads an
overlay holding only touched targets (at most three) and projected impacts (at
most three), plus copied RNG and scratch events/links. Each prepared pellet runs
through the EXISTING LifeHost combat executor against that private overlay before
planning the next. Thus health, flags, height, momentum, pain/death state, timers,
reaction and retargeting are projected by the same implementation as the commit.
No independent damage/RNG model or full Simulation/map/store clone is introduced.

The strict body closure cannot move/relink, change radius/index participation,
remove a body immediately or recursively attack during the burst. Exact impacts
are NOBLOCKMAP. Existing base collection geometry therefore remains valid; hit
eligibility and damage read UPDATED projected bodies. Actual execution draws each
pellet's spread/damage, commits its effects/damage and only then advances to the
next pellet, checking planned RNG boundaries. Aim is computed once. Sound occurs
before internal facing; the separate earlier animation face still executes.

Three real slots are reserved conservatively for every non-null burst; predicted
lowest-free identities honor current generations/exhausted slots. Scratch spawn
order matches appended real impacts, without processing them early or crediting
pending retirement. Real thinker traversal later processes bodies/effects in its
existing same-pass order. Null returns need no capacity. All projection/trace/
coordinate/admission errors precede authoritative sound, state, health or RNG.

## M3d3e cross-tic CPOS and immediate refire handoffs

Profile 12 adds no scheduler or persistent firing counter. Exact positive CPOS
states 4/5/6/7 encode the 10/4/4/1 loop. Every CPosAttack uses the shared single-shot
plan and executor; an explicit sound-order parameter preserves source PosAttack's
old order and CPosAttack's shotgun-before-face order. Only the current callback
is prepared. State 4/5/7 expiry prepares a shot from current authoritative bodies,
so moving, shadowed, damaged or dead targets are observed anew on the next shot.

Refire uses a pure lazy decision helper after source facing and RNG. Health and
sight closures execute only when the earlier conditions reach them. Its event
precedes immediate LifeHost see-state entry, which executes source Chase/Look,
reacquisition, JUSTATTACKED, movement, relinking and any new ATK1 in the same call.
Read-only owner/reference/map/state/counter preflight establishes that the strict
closure's existing sight/movement services cannot recoverably fail. Positive tics
prevent immediate nested shooting; refire needs no allocation, copied world or
speculative RNG plan. The accepted two-Chase/one-successful-walk bound and reaction
guard are exercised by source traces and separate arithmetic policy tests.

One hit reserves its real impact slot before sound/draws. Null/miss/refire can
succeed at capacity; pending retirement supplies no credit for a hit. Actual
player binding/damage stays protected. New tag 20 records only the refire branch;
existing actor/state/RNG/sight/combat events carry the remaining chronology.

## M3d3f direct melee and bounded damage planning

Profile 13 composes existing awareness/pursuit, source state dispatch and direct
damage. It adds no physics, scheduler, hitscan path, impact actor or persistent
attack counter. Exact positive SARG entries 4/5/6 have 8/8/8 tics; their source
actor fields already represent animation progress. Source Look/Chase state 1/2
remain positive bounded fixture loops. Original type 12 is metadata, while the
labelled native 136 owner and source-body target 135 declare executable roles.

The only damage refactor is internal visibility of DamagePlan/plan_damage and
LifeHost apply_damage plus an actual spawn-demand accessor. The algorithm and
public combat/player protection boundaries are unchanged. The original damage
call uses owner as both inflictor and source, retaining thrust, pain/death and
retargeting through the same host as earlier combat profiles.

Before a direct call, state entry 6 or state 5 expiration, validate owner/roster/
retained references/state closure and copy the small owner/RNG record. Preview
internal shadow draws, current pure range/sight, the damage draw only if accepted,
then the existing damage plan. Reserve actual state demand, zero for these exact
body closures. The ephemeral plan stores owner, initial RNG, accepted boolean and
optional amount/DamagePlan; it never clones the store, world or simulation.

Authoritative execution returns on null, otherwise faces, repeats the existing
melee query against the current host, asserts preview agreement, records source
sight/range order, draws once and applies shared damage immediately. Sight/range
ignore facing angle/AMBUSH. First-owner/zero-physical-momentum and positive state
closure mean no earlier thinker/input can invalidate the preview. Each sight
query retains its existing 8192 work bound. Target thrust applies before that
body's later thinker; existing XY clamping to 30 preserves the 994 margin.

Chase's earlier melee selection emits metadata sound 52 before entry 4 and returns
before movement/active sound. A later miss retains that selection sound. Return
from state 6 executes real Chase/Look and may select another positive attack state;
no cached range or skipped callback substitutes for original behavior. Source
fast/Nightmare global timing transformation stays outside profile 13. DSIM-v13
uses existing event layouts and fields; expected bytes come from original-C rows
and independent encoding, as described in [the investigation](../../M3D3F_REFERENCE.md).

## M3d3g damageable Demon fixture and corpse lifecycle

Profile14 composes existing LifeHost, DamagePlan, M3b physics, spatial lists and
ThinkerStore. Native136 copies original MT_SERGEANT metadata but remaps spawn/
see/attack/pain/death into named aliases, removes COUNTKILL and disables raise.
Look/Chase are explicit positive stand/run loops. Source-body135 supplies unbound
sources/targets; original blood/puff definitions serve explicit synthetic hitscan.
Full original types 12/13 and second autonomous owners are not admitted.

Incoming damage runs the shared pipeline with perception/Chase host context.
Strict complete state closure, reference/roster/sound/binding checks and existing
work/movement bounds make immediate non-pain spawn-to-Chase and later pain return
infallible after preflight. Pain/death/Fall allocate no actors; hitscan preserves
impact-before-damage RNG and real reservation. No new damage/state scheduler.

Unlike profile 13, a living lifecycle owner may have knockback and source XY/Z
motion. It remains first, with the existing978 tic/1008 callback bounds. The small
melee plan validates potential damage even if its current pure preview misses;
after physics and real facing, the authoritative query decides whether to draw
and apply damage. Ordinary unbound XY/Z consumes no RNG, and no earlier thinker
can change the target's health. No stale geometry acceptance or full-store clone
is carried into execution. Profile13 retains its exact earlier behavior.

Kill changes flags/height/dropoff/gravity through the existing executor. Solidity
persists until A_Fall. Corpse physics, friction and relinking use the same movement
path, including ledge and reduced-height portal behavior. Terminal -1 stops state
advancement only. Dead owners' closed action-free/sound/Fall graph does not read
retained targets; their unresolved generations remain stored without aliasing.
Living callbacks still reject stale dependencies. Public state/setup cannot
resurrect the owner. Canonical fields/events already represent every transition.

## M3d3h original stand/run composition

`demon_stand_run` defines immutable source-state aliases and the explicit profile 15
constructor. It copies complete source state records into the fixture registry;
only identity/name/next-state aliases change. Both stand states and every run
state retain their own timers and actions. Existing LifeHost entry executes
nested Look→Chase→melee, and attack/pain returns execute the first Chase
immediately. No AI scheduler or movement implementation is added.

The shared lifecycle constructor selects the exact registry for profile 14 or15.
The profile helper grants existing damageable-owner physics/combat/preflight only
to those two explicit profiles;13 and all older contracts stay separate. Native
state roles extend to1–21 only in15. Public live entry permits stand/run/melee;
pain/death enter through shared damage. Public death-request shortcuts and corpse
revival remain rejected. Strict positive/infinite closure keeps existing nested
counter,978-tic/1008-callback movement and64-actor proofs. Unsupported future
callbacks are validated before movement/RNG/sound, including the second stand
state and future run states. There is no whole-world clone per tic.

Canonical state15 uses existing actor/state/registry/identity/spatial/roster/RNG
fields and observations; no redundant persistent state or new observation tag is
needed. Profile 14 retains its simplified loops byte-for-byte. C adapters and
independent encoders remain tooling, separate from native Rust simulation.

## M3d3i bounded map-spawn composition

Profile 16 reuses doom-map THINGS, the exact profile 15 fixture registry, LifeHost
spawn, actor storage, source spatial head insertion and thinker append. Original
metadata resolves/filter DoomEd3002 before the declared12→136 executable mapping.
The bounded setup plan validates every selected record and reserves once; only
then does existing spawn install raw state/link/lastlook, followed by spawnpoint,
timer randomization, angle and ambush. No persistent "level spawned" flag is
needed: tic 0/allocated-slot state defines admission. All-filtered setup may repeat
without canonical changes; any allocated/retired store or later tic rejects.

Up to four native thinkers form a prefix before unbound 135 bodies and effects.
Live native targets and active perception/sound references are stable135 bodies;
unsupported infighting and real players reject. This ensures an earlier native
cannot damage another native or retire/move its target before that target's later
thinker. Native corpses keep their existing closed lifecycle and identity.

A bounded global preflight checks all roles/future closures/references/bindings/
coordinates/counters before any thinker mutates. Four maximum40-damage strikes
can add under80 fixed units of body thrust even with fall-forward; initial body
XY momentum is bounded64 and existing physical clamping remains source-derived.
Target health positive subtraction fits; already-dead/nonshootable bodies follow
shared ignored damage. Pain/death states allocate zero and do not remove bodies.

At actual A_SargAttack, a small current owner/body/copied-RNG plan uses the shared
DamagePlan completion logic. Actual Face/range/RNG/apply_damage services remain
unique. Later owners therefore see earlier owners' real health/RNG effects.
The transient host flag selects this admitted proof; no canonical duplicate state,
attack queue, new scheduler, damage engine or per-tic full-state clone is added.
Profile13–15 retain their earlier plan and exact bytes. Source C adapters and the
independent canonical encoder remain external validation tools.
