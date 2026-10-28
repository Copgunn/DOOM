# M3c investigation: metadata, state timing and lifecycle

Recorded before M3c runtime implementation. Authority: untouched Linux DOOM 1.10,
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. Tables are evidence of metadata;
execution compatibility requires the separate function/trace comparisons below.
No GZDoom/GPLv3, private project, IWAD or scripting implementation is involved.

## Original source findings

- `info.c/h`: indexed states carry sprite/frame/tics/action/next/misc fields;
  mobjinfo carries DoomEd number, spawn health/reaction time, dimensions, mass,
  speed, sounds and spawn/see/pain/melee/missile/death/xdeath/raise references.
  Recognizing a table entry does not implement its action functions or AI.
- `p_mobj.c` P_SpawnMobj zeroes storage, copies type dimensions/flags/health,
  assigns reactiontime except on nightmare, consumes exactly one P_Random for
  lastlook modulo four, and assigns spawn state/tics/sprite/frame DIRECTLY. It
  does not call the spawn action or chase zero-tic next states. Then it links
  spatial/sector state, resolves floor/ceiling/absolute Z and appends the thinker.
- P_SetMobjState checks null first: set state pointer null, remove, return false;
  tics/sprite/frame are not reassigned in that branch. Otherwise assign the state,
  duration and render identifiers BEFORE calling its action, then use that
  entered state's nextstate and repeat iff the actor's CURRENT tics are zero.
  If an action recursively changes state, the outer frame retains its own next
  reference while inspecting the changed actor's timer. Reproduce that distinction.
- P_MobjThinker runs XY, checks removal, runs Z, checks removal, then decrements
  tics unless exactly -1. If the decrement becomes zero, enter nextstate. Thus an
  initial spawn duration of zero becomes -1 without executing an action. Infinite
  states do not transition just because nextstate is null. Nightmare monster
  respawning is a separate COUNTKILL path and remains unsupported.
- `p_tick.c`, `d_think.h`: append-sensitive traversal permits a newly appended
  child to think in the same pass. Marking current/visited storage defers
  retirement until a later pass; marking unvisited storage skips its update and
  retires at its visit. Physical unlink is immediate in P_RemoveMobj.
- P_RemoveMobj handles item respawn bookkeeping, unlinks sector/block lists,
  stops sounds and marks thinker removal. It does not clear other actors' target
  or tracer pointers. Raw dangling pointers after free are unsafe; Rust retains
  stored generation identities but resolves stale targets as unavailable, never
  silently pointing to a reused slot. Target is the supported relationship;
  tracer/owner/combat reference semantics are reserved for later milestones.
- P_SpawnMapThing handles deathmatch starts (bounded ten), player starts, single-
  player option-16 exclusion, skill bits (baby/easy=1, medium=2, hard/nightmare=4),
  DoomEd lookup, deathmatch/nomonsters exclusions, type spawn, positive initial
  tic randomization with an ADDITIONAL P_Random, angle=ANG45*(signed angle/45),
  and ambush flag. Preserve source order and toward-zero division for angles.
- P_SpawnPlayer additionally initializes player inventory/view/UI/weapon state.
  M3c does not implement that function. Map player starts are explicitly reported
  as deferred, not silently instantiated as complete players. Supported map mode
  is single-player DOOM II; other modes reject before mutation. Skill/nomonsters
  filtering precedes unsupported selected-type checks, matching source order.
- `p_enemy.c`: A_Fall clears SOLID without removing/unlinking the actor. A_Pain
  conditionally emits type painsound; A_XScream emits slop; A_PlayerScream chooses
  pldeth/pdiehi using commercial mode and health; A_Scream selects the original
  death sound variants/RNG, including boss-global origin. These can be represented
  as presentation-neutral observations without audio playback. Other actions are
  unsupported unless separately implemented and verified; never no-op them.
- `p_inter.c` P_KillMobj performs combat flags, height, scores/drops, player death
  state and RNG-adjusted timers. M3c controlled death is explicitly entry into the
  type's death STATE only, not P_KillMobj/damage. Do not fabricate health loss,
  gibbing, pickups, drops, resurrection, corpse flags or randomized kill timers.
  A terminal death object may persist; null entry is a separate removal operation.

## Execution profile and safe policies

M3c uses explicit profile 4 / DSIM-v4, preserving all older profiles and fixtures.
Recognized source metadata is complete; admitted executable types require valid
spawn/death state closures, supported actions/flags/dimensions and fit in every
runtime sector. This conservative fit rule makes dynamically requested child
spawns infallible geometrically; closed/short sectors can restrict profile-4 type
admission. Source physics and all earlier profiles retain their own contracts.

Supported flags are limited to combinations whose unimplemented collision paths
are unreachable: SOLID/SHOOTABLE, SPAWNCEILING, NOGRAVITY, DROPOFF, PICKUP when no
SPECIAL actors are admitted, AMBUSH/NOTDMATCH and explicit NOSECTOR/NOBLOCKMAP
membership rules. SPECIAL, missiles/skulls/floating monsters/corpses/AI-counting
behaviors are not silently admitted. Metadata keeps other flags recognizable.
No-sector/block flags filter the existing logical relink order rather than
introducing a second allocator. A_Fall synchronizes collision solidity immediately.

Metadata and dispatch are separate. Original action identities remain distinct
from a small explicitly named synthetic PROBE action namespace. Probe callbacks
exercise original state/scheduler semantics (random draw, state entry, child spawn,
self/target removal, target assignment); they are reference-adapter tests, not
invented vanilla monster actions or a scripting language. No source-supported
spawn-producing combat/AI action is claimed in M3c.

Validate all indexes and metadata before use. Bound registry sizes, immediate
entry depth/work and spawn dependency graphs. Positive-duration cycles are valid;
pathological immediate zero/callback cycles reject before mutation. Initial
zero-duration spawn timing remains distinct. Explicit state entry checks its full
executable closure before consuming RNG or mutating storage. No unsupported action
can arise dynamically in an admitted closure.

A bounded state-only preflight computes a conservative upper bound on action
entries/spawns before player/physics/event mutation. It does not clone the full
simulation or maps, and does not credit pending retirements as free capacity.
That conservative capacity policy can reject tight cases that an unbounded C
allocator would accept; it prevents partial-tic failures/RNG drift. Reserve arena
and relink capacity before execution; allocations outside recoverable reservation
use the existing process-level policy. Static action admission and bounded spawn
closure make execution deterministic and infallible after preflight.

Typed target assignment validates identity; self/mutual references are allowed.
Removal does not eagerly clear stored references. Resolution can read pending
storage until retirement; stale generations then resolve None and never alias
new occupants. Public mutation rejects removed/stale actors. Probe callbacks
operate through restricted capabilities, not arbitrary closures/global state.

## Scope boundaries and evidence plan

Use original info.c table compilation plus source P_SpawnMobj, P_SetMobjState,
P_MobjThinker, map-spawn and thinker bodies in a separate C adapter. Unsupported
callbacks abort; supported source sound actions emit inert observations. Retain
retired C storage to avoid the original traversal's read-after-free. GNU function-
pointer union/sentinel ABI assumptions are explicit. Rust fixed-coordinate
conversion is defined; C trace coordinates avoid the negative-shift case. No historical file
is changed and no Rust output produces expected reference results.

Reference traces must distinguish source metadata, original executable cases and
synthetic probe callbacks. Compare state/timers/actions/RNG/thinker/lifecycle/map
filter order. DSIM-v4 includes registry definitions, typed actor state/health/
flags/timers/target/spawnpoint, game options and ordered transition/action/sound/
lifecycle observations. Old formats remain reproducible. Independent encoding,
field/byte comparisons and long traces supplement isolated C evidence, not full
combat/monster/player or DOS compatibility claims.

Future native/DECORATE-like definitions, ACS/ZScript compatibility adapters and
custom actions may build on validated immutable metadata and restricted services.
They are architectural directions only, requiring independent implementation and
licensing review. No scripting, PK3, renderer, wgpu/audio or Bevy is added here.

## Observed isolated C evidence

`generate_m3c.py` compiles the untouched `info.c` initializers and symbolic headers:
967 state rows (seven numeric fields plus name), 137 type rows (23 properties plus
name), and 74 non-null action identities. The generated Rust table carries GPL
attribution. Every recognized type/state reference is validated separately from
execution admission.

`m3c_driver.c` receives extracted original P_SpawnMobj, P_SpawnMapThing,
P_SetMobjState, P_MobjThinker, P_RemoveMobj, thinker init/add/mark/run and the five
supported action bodies. Its 393 actor observations and 444 event observations
cover player-type controlled death through terminal persistence, fog through null
removal/next-pass retirement, all five skill filters/nomonsters, signed angle
quantization, ceiling map props, randomized positive spawn tics, five sound/fall
actions, direct zero-tic spawn, recursive entry and same-pass child removal.
An independently coded C admission predicate selects 67 original types in a flat
128-unit sector and compares 201 floor/ceiling/absolute initializations. These are
isolated supported functions, not complete monster/combat synchronization.

Adapters are explicit: a flat sector supplies geometry; motion callbacks abort
if reached (these state traces use floor actors or capture ceiling setup without
stepping). Player-start creation is deferred by an adapter; real P_SpawnPlayer is
not tested. All other source action bodies abort. Sound calls emit events with
no device. Five synthetic PROBE actions replace selected weapon states/native
type slots only for the probe scenarios; original tables are restored before the
67-type spawn sweep. Logging is inserted at state/thinker/action boundaries, and
A_Scream's RNG calls are wrapped to record the unchanged lookup value. Allocation
uses zeroed owned blocks; Z_Free marks retired but retains bytes until scenario
reset so P_RunThinkers can read its original next pointer safely. Original union
function-pointer dispatch and the `(actionf_v)-1` sentinel depend on the documented
GNU C ABI. THINGS coordinates in these C scenarios are nonnegative, avoiding the
original negative signed-left-shift undefined behavior; Rust conversion is defined.

Provenance records source/header/adapter/output SHA-256, upstream revision,
compiler, flags and host. Reproduction reads no Rust output and changes no
historical file. `m3c_snapshot_goldens.py` independently serializes 99 captures and
24 full DSIM-v4 states from C rows plus explicit setup/schema rules, including
source/probe registry definitions. Recorded-byte tests run without a C compiler.

## Admission matrix and bounds

All 137 source type definitions remain recognized. In the 128-unit flat fixture,
67 are admitted: MT_PLAYER, MT_SMOKE, MT_BOSSTARGET, MT_PUFF, MT_BLOOD, MT_TFOG,
MT_IFOG, MT_TELEPORTMAN, MT_EXTRABFG and MT_MISC29 through MT_MISC86. This is a
spawn/state subset: animation-only blood/puff effects do not imply hit processing,
BFG effects do not imply BFG combat, boss targets do not implement boss actions,
and a player-type pawn does not implement P_SpawnPlayer. Shorter sectors may
reduce this set. Complex AI, pickups, barrels requiring A_Explode, projectiles,
COUNTKILL/COUNTITEM, corpse movement, resurrection and other actions are rejected.
SHADOW is retained as an inert presentation flag; no renderer is implemented.

Native registry bounds: 2048 states, 256 types, 64 UTF-8 bytes/name, source sprite/
sound indexes, durations >= -1, validated state references and canonical inert
S_NULL definition. Source state zero as direct initial assignment is distinguishable
from an entered null pointer. Type admission memoizes dependencies with a 32768
visit bound and rejects spawn cycles; immediate entry work is at most 128. Step
preflight bounds aggregate entry work to 8192 and reserves at most 1024 spawns.
Profile 4 permits at most 1024 actor slots and 4096 map THINGS. Unknown/unsupported
selected placements reject the batch before mutation, unlike original fatal setup.
Registry definitions and options are immutable after construction.

Entry validation includes all next/callback/spawn dependencies, even unreachable
next edges of infinite states. This conservative policy can reject definitions
that the source would never execute; it prevents future requests from exposing
unsupported actions. Pending retirements are not credited as free spawn capacity.
No compatibility promise is made for rejected pathological graphs or C dangling
pointer dereferences. General ThinkerStore callbacks retain their older explicitly
nontransactional visit-budget contract; the built-in profile-4 runtime preflights.
