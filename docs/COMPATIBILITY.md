# Compatibility boundaries

The behavioral authority is original id Software Linux DOOM 1.10. This is an early
headless rewrite supporting specific, limited behavior, not a complete source port.
This describes the code in this checkout, not its merge/review status. This
document owns support restrictions and limits; [ROADMAP](ROADMAP.md) owns
development status and plans. The [glossary](README.md#short-glossary) explains
profile, admission and fixture terminology.
Linux 1.10 comparisons do not prove DOS 1.9 demo synchronization.

## What a compatibility claim means

| Level | Meaning |
| --- | --- |
| Recognition | An original type, state or resource is represented as metadata |
| Validated parsing | Input records and references pass structural/safety checks |
| Supported execution | An explicitly selected configuration accepts specified actors and reachable actions |
| Original-C evidence | Specific inputs match genuine source bodies under declared adapters |
| Full-game compatibility | All enabled dependencies compose in the original game context; not established |

Loading a WAD, recognizing MT_SERGEANT or matching an isolated hash does not
establish the last level. [Reference notes](README.md) describe each admitted
slice and the [validation guide](VALIDATION.md) distinguishes evidence families.

## Stepping and diagnostics

`Simulation::advance([TicCommand; 4]) -> Result<(), SimError>` completes one tic
when unpaused. It constructs no Snapshot. `step` runs that same core and returns
one Snapshot on success; explicit `snapshot()` remains available in release.
`Snapshot::canonical_bytes` and `digest` additionally encode bytes. Opening an
execution scope or using GameInstance does not select a second stepping engine.

Both APIs preserve commands, ordered events, pending observations, thinker/slot
ordering, RNG, actor/state admission and per-operation recoverable-error atomicity.
Paused calls are no-ops before command/overflow checks: advance returns unit;
step captures unchanged state. Errors capture nothing and preserve canonical state,
including queues and RNG. This is per call, not rollback of a whole command stream
or context callback. Allocation failure outside fallible reservations remains the
existing process-level Rust policy, not a promise to recover from every OOM.

Events and gameplay/preflight/staging allocations remain enabled. This API makes
full diagnostic capture optional; it adds no gameplay profile, identity storage,
DSIM version, actor or geometry capacity. `run_commands` retains its diagnostic
collection contract. Use an ordinary loop over advance when snapshots are unwanted.
See [architecture](ARCHITECTURE.md#stepping-and-observation),
[source/cost audit](M3G1_REFERENCE.md) and [measurements](validation/M3G1.md).

## Versioned gameplay profiles

Historical profile behavior and DSIM bytes remain frozen. New behavior is opt-in.

| Configuration (profile) | Supported foundation |
| --- | --- |
| 1 | Non-solid fixed-point headless movement and command foundations |
| 2 | Grounded player XY sliding and solid collision |
| 3 | Static multi-sector BSP/openings, XY/Z and gravity/dropoff |
| 4 | Explicit type/state lifecycle and bounded synthetic probes |
| 5 | Unbound damage, aim and hitscan |
| 6 | Bounded player weapons and psprites; complete player damage excluded |
| 7 | Bounded original plasma projectile foundations |
| 8 | Sight, ordered player scanning, sound propagation and Look |
| 9 | Non-attacking source pursuit |
| 10–12 | Separately admitted attack-selection, shotgunner and chaingunner fixtures |
| 13 | Demon melee with simplified fixture stand/run loops |
| 14 | Demon pain, ordinary death and persistent corpse lifecycle |
| 15 | Original two-state stand and eight-state run closure |
| 16 | Bounded classic Demon THING spawning, retaining the prior fixture flags |
| 17 | Map kill totals, original COUNTKILL flags and bounded death-credit context |
| 18 | Expanded bounded static geometry, unbound XY/Z and read-only original sight |
| 19 | Checked larger coordinate envelope and spatial work for unbound static actors |
| 20 | Explicit intersection of larger static geometry and adapted Demon gameplay |
| OriginalDemon | Same bounded Demon closure with actual original IDs and explicit context; DSIM21/profile wire ID 1 |

These are separate contracts, not modes that automatically admit all original
actors or map features. Detailed older limits remain available in the complete
[historical compatibility document](validation/archive/pre-m3d3j-compatibility.md).

## Demon support: remaining dependencies

| Dependency | Map-context configuration (profile 17) support |
| --- | --- |
| Source metadata | Original `MT_SERGEANT` type 12 / editor type 3002 is recognized unchanged |
| Map THING admission | Commercial single-player, normal timing, bounded Demon-only execution |
| Spawn compatibility | Signed fixed coordinates, BSP floor, quantized angle, raw state, original RNG/order |
| Original flags | SOLID/SHOOTABLE/COUNTKILL = 4194310 restored; ambush applied from THING |
| Level statistics | Kills/items/secrets reset to 0; admitted map COUNTKILL placements add available kills |
| Player statistics | Only genuine players[0].killcount fallback for absent/unbound sources; no player-pointer scoring |
| Damage/death | Existing shared services, pain chance 180, delayed sounds, ordinary death/Fall/quarter height |
| Terminal corpse | COUNTKILL retained; respawn disabled, no extra terminal RNG/movecount work |
| Respawn/raise | Explicitly unsupported; no Nightmare respawn or arch-vile resurrection |
| Complete original actor | Deferred: native type 136 identity, state aliases and disabled raise remain adaptations |

Only the old COUNTKILL flag projection is removed. Profile 17 uses
`ActorRegistry::demon_map_context_fixture`, not complete original actor admission.
Available `total_kills` never decreases on death/removal. The separate
`single_player_kill_count` represents source's unbound single-player fallback,
including absent sources, self damage and admitted monster-caused lethal damage.
It is not points, frags or a complete player/intermission subsystem. Other level
totals remain zero because items and sector specials are excluded.

Fresh restart resets the bounded level/actors/statistics while continuing RNG.
It returns a new simulation and does not perform complete game/input/player reload.
No API promises globally unique handles across separate simulation instances.

## Map size and processing limits

A valid parsed map may still be rejected by a simulation configuration. The bounds
below come from the linked implementation, not a promise of real-map gameplay or
unlimited geometry. Four different kinds of limits must be considered separately.

### 1. Original encoding and source semantics

The classic records have fixed-width fields. [Map types](../crates/doom-map/src/types.rs)
preserve signed 16-bit coordinates/heights, unsigned flag bit patterns and
16-bit references. The [parser](../crates/doom-map/src/parse.rs) additionally
validates references according to the selected Linux 1.10 interpretation:

- Ordinary vertex, line, sidedef and sector references must address existing
  records and fit the nonnegative signed 16-bit range. A Rust `u16` storage type
  does not automatically authorize larger source indexes.
- Node children use the high bit to mark a subsector and the lower bits for its
  index. Sidedef `0xffff` means absent; subsector first-seg/count fields have
  signed-range and contiguous-range checks.
- BLOCKMAP dimensions must be positive signed 16-bit values; referenced list
  offsets and line indexes follow signed source constraints. `0xffff` terminates
  a list. These rules can reject data well below allocation budgets.

Changing a budget cannot widen an on-disk field or remove a reserved marker.
Supporting another source interpretation or an extended map format requires an
explicit compatibility decision and, where needed, different representations.

### 2. Parser safety budgets

[MapLimits and Budget](../crates/doom-map/src/parse.rs) provide these defaults:

| Setting | Default | What it bounds |
| --- | --- | --- |
| `max_records` | 32,768 | Decoded records per table |
| `max_lump_bytes` | 64 MiB | Bytes in one map lump |
| `max_blockmap_cells` | 262,144 | Decoded collision-grid cells |
| `max_blockmap_words` | 4 Mi words | Raw BLOCKMAP header/table/list storage |
| `max_decoded_bytes` | 128 MiB | Cumulative requested decoded and scratch allocation |

These are resource ceilings, not guaranteed supported map sizes. Checked
multiplication/addition and fallible reservations enforce the allocation budget.
The parser also hard-caps every record table at 32,768 independently of the
configurable setting. Its uniform application needs table-by-table investigation:
for example, the THINGS record count comes from lump length, not a 16-bit count
field. Neither that policy nor every classic reference is interchangeable with a
universal format limit. Increasing `max_records` alone does not remove the cap.

### 3. Temporary simulation support restrictions

These narrower bounds support the existing arithmetic, ownership and preflight
proofs. They are not classic-map format limits; changing one requires checking
all dependent representations and services.

| Area | Current bound | Implementation |
| --- | --- | --- |
| Historical collision profiles 2–17 | 128 lines; 4,096 BLOCKMAP cells; 65,536 BLOCKMAP words | [prepare_cells](../crates/doom-sim/src/movement.rs) |
| Historical topology profiles 3–17 | 128 sectors; 4,096 subsectors; 4,096 nodes | [Topology::new](../crates/doom-sim/src/multisector.rs) |
| Historical sight-supported geometry through profile 18 | Absolute vertex coordinates, node origins/deltas and sector heights at most 512 map units | [validate_map](../crates/doom-sim/src/sight.rs) |
| Map Demon setup | 256 THINGS, four native Demon test actors, 64 total actor slots | [map spawning](../crates/doom-sim/src/demon_map_spawn.rs), [constructor](../crates/doom-sim/src/demon_lifecycle.rs) |
| Earlier general actor-state setup | 4,096 THINGS and 1,024 actor slots | [actor-state constructor](../crates/doom-sim/src/spawning.rs) |
| Underlying actor store | Configurable, at most 65,536 slots; individual configurations are stricter | [ThinkerStore::new](../crates/doom-sim/src/store.rs) |
| Map Demon arithmetic margins | Absolute spawn coordinates 978, owner query margin 1,008, other actor coordinates 994, general actor/query range 1,024 map units; body initial XY momentum and admitted Z momentum at most 64 units | [map preflight](../crates/doom-sim/src/demon_map_spawn.rs), [spawn checks](../crates/doom-sim/src/spawning.rs), [sight](../crates/doom-sim/src/sight.rs) |

For example, collision and hitscan services contain fixed line-work buffers tied
to the line bound; raising only the rejection constant would not be sufficient.
The [future map-scale milestone](ROADMAP.md#map-scale-limits-and-real-map-readiness)
describes the broader direction. The separate geometry configurations below
expand their declared slices while retaining historical admission.

### 4. Performance and traversal work limits

| Service | Current work bound | Meaning |
| --- | --- | --- |
| Historical sight through profile 18 | 8,192 units of fully expanded BSP traversal work, including repeated children/segs | [sight work validation](../crates/doom-sim/src/sight.rs) |
| Hitscan | 64 grid-cell iterations; candidate count bounded by `64 * (128 + 1024)` | [combat tracing](../crates/doom-sim/src/combat_trace.rs) |
| State actions | 128 immediate entry work units, 8,192 planned step work units, and bounded child spawning | [lifecycle planning](../crates/doom-sim/src/lifecycle.rs) |

Some work bounds also preserve source semantics: the hitscan cell loop follows
[original P_PathTraverse](../linuxdoom-1.10/p_maputl.c), so expanding it may change
vanilla behavior. Other bounds prevent pathological CPU/memory growth. The future
inventory must distinguish these reasons rather than replace every value with a
large constant. Keep deterministic traversal and checked resource use throughout.

## Map-scale profile

`Simulation::map_scale` / `with_map_scale_limits` select profile 18, not a Demon or
player profile. Parsing capacities/formats are unchanged. The new configuration
supports at most **1,024 linedefs, 256 static sectors, 64 unbound actor slots,
4,096 BLOCKMAP cells, 65,536 raw BLOCKMAP words, and 262,144 compiled first-occurrence
references summed over cells**. Shared lists count once per cell in the last budget.
Nodes and subsectors remain at most 4,096 each. Smaller actor/reference budgets
are available through `MapScaleLimits`; higher budgets reject.

Vertex coordinates, node origins/deltas, sector heights and BLOCKMAP origins stay within
512 absolute map units. Original sight expansion remains bounded at 8,192 work
units, including repeated node children and segs. Spawn positions/absolute Z and
current positions/Z before a step stay within 994 units; XY/Z momentum is at most
64. Source XY movement clamps to 30. Read-only destination/sight actor coordinates
are bounded at 1,024 units. A source step can leave the 994-unit step-admission
margin; the next step then rejects atomically. This is an explicit conservative
Rust boundary, not unrestricted off-map movement compatibility.

Use parameterized unbound bodies and existing XY/Z, gravity, openings, collision,
sector/BLOCKMAP head insertion and thinker storage. THINGS remain parsed data.
Read-only `check_sight` preserves REJECT indexing and source BSP/seg ordering without
admitting perception rosters or AI. Unsupported player binding, typed spawning,
map spawning, damage/hitscan, noise/Look/Chase, map specials and dynamic sectors
reject; no unsupported callback becomes a no-op. Real player, Demon/Spectre and
whole-map/game admission are not implied.

Source BLOCKMAP words, cell traversal and first encounters determine candidates.
Lists are never rebuilt or globally sorted by line ID. Recoverable rejected setup,
controls, identity queries and stepping preserve canonical state. Allocation for
new geometry preparation is fallible and bounded; existing runtime diagnostic
allocation exhaustion retains the process-level Rust policy described by `step`.
The source 64-cell slide/hitscan traversal and 128-intercept restriction are not
expanded or admitted by this configuration. Historical constructors retain their
old capacities and DSIM expectations.

[Source findings](M3F1_REFERENCE.md) explain the representation and adapter;
[measured evidence](validation/M3F1.md) separates original-C equivalence,
independent DSIM consistency, safety tests and host measurements. This supports
larger controlled classic fixtures, not all real DOOM maps or vanilla demos.

## Map-extent profile

`Simulation::map_extent` / `with_map_extent_limits` select **profile 19**, a separate
unbound static-geometry configuration. Historical profiles, including profile 18,
retain their existing coordinate/work rejection thresholds and canonical bytes.
Classic formats and parser budgets are unchanged.

| Domain or resource | Profile19 admission |
| --- | --- |
| Vertex components, node origins and node deltas | Absolute 2,048 map units each |
| Linedef delta | `abs(dx) + abs(dy) <= 1,536` units, checked from endpoints |
| Sector floor/ceiling | Absolute 1,024 units; noninverted |
| BLOCKMAP origin components | Absolute 2,048 units |
| Actor spawn/current XY before a tic | Absolute 2,018 units |
| Destination and sight XY | Absolute 2,048 units |
| Absolute spawn/current Z before a tic | Absolute 1,984 units; floor/ceiling placement uses the sector |
| Sight Z | Absolute 2,048 units |
| Actor radius / height | Radius 0–32; positive height fitting the spawn sector, at most 2,048 |
| XY/Z momentum | Absolute 64; original XY clamp remains 30 |
| Linedefs / sectors / unbound actor slots | 1,024 / 256 / 64 |
| Nodes / subsectors | 4,096 each |
| BLOCKMAP cells / raw words | 8,192 / 65,536 |
| Prepared first-occurrence references | 262,144, counting each shared list once per cell |
| Conservative preparation allocation requests | 4 MiB; independently configurable downward |
| Collision work proof | At most 16,384 units; setup checks `nodes + 4*lines + 4*actor_limit` |
| Expanded sight work | 16,384, counting both repeated children, leaves and segs |

`MapExtentLimits` allows smaller actor, reference, preparation-memory and collision-
work budgets. Larger budgets reject. Preparation charges reserved reference limits
and native-size cell/line/membership/topology/work scratch conservatively, including
scratch whose lifetimes do not overlap. Parsed Map storage, canonical caches,
actors, snapshots and allocator overhead are outside that request budget and retain
their separately bounded/process-level allocation policies. Geometry preparation
and new sight-query scratch reservations are fallible. Existing tic diagnostics
and canonical allocation exhaustion remain process-level Rust failures, not `SimError`.

World-space bounds and pairwise differences are distinct: admitted query actors
can differ by 4,096 units. The line-delta norm additionally protects the original
shifted intercept sum/difference; blindly enlarging all coordinates would not.
This is not a strict superset of profile 18: some long old diagonals require the
old narrower interaction domain. A successful tic can leave the next-tic XY/Z or
momentum margin; a subsequent tic rejects before any earlier actor moves.

A radius-bounded destination visits at most four line cells and four thing cells:
at most 4,096 compiled references, 1,024 unique linedef callbacks and 64 unique
thing candidates. BSP lookup adds at most the admitted node count. Source supplied
list/cell order, duplicate suppression, leading-zero behavior, things-before-lines,
head insertion and thinker order remain authoritative. Conventional empty cells
still contain the required zero word and therefore visit line0. Unused grid padding
need not fit world coordinates; runtime queries cannot leave their proven domain.

Coordinate bounds are necessary, but not sufficient for every source sight input.
Quantized side tests can produce an intercept saturated to `INT_MIN`; original
FixedDiv would then evaluate undefined `abs(INT_MIN)` during clipping. Profile19
returns a recoverable arithmetic error before that division, preserving canonical
state. Historical profiles retain their existing explicit Rust numeric policy.

Sight caches a proved full-root expansion cost; query execution uses that same
bound and fallibly reserves bounded tasks/observations. REJECT returns before BSP
work or scratch reservation. No query consumes RNG or changes canonical state.

Use existing unbound XY/Z, gravity, static openings, spatial storage and read-only
sight only. Player binding, typed actors, map spawning, AI/hearing, combat/hitscan,
sliding, dynamic sectors, specials and complete game/map admission remain excluded.
Original 64-cell `P_PathTraverse` and 128-intercept behavior is unchanged.
[Source analysis](M3F2_REFERENCE.md) owns the arithmetic derivation;
[observations](validation/M3F2.md) distinguish genuine C comparisons, independent
canonical encoding, Rust safety tests and host measurements.

## Expanded-map Demon profile

`Simulation::with_demon_expanded_map_registry` selects **Profile 20**. Supply the
existing `ActorRegistry::demon_map_context_fixture`, explicit actor options and
map context; then call `spawn_map_things` before installing unbound targets.
This composes the admitted Demon closure with expanded static preparation. It
retains native type 136/state aliases and does not admit complete MT_SERGEANT.

| Area | Profile 20 limit or behavior |
| --- | --- |
| Tables | 1,024 lines, 256 static sectors; 4,096 nodes and subsectors each |
| Geometry | Vertex, node origin/delta and BLOCKMAP-origin components within ±2,048 units |
| Heights and line deltas | Sector heights within ±512, noninverted; linedef `abs(dx)+abs(dy) <= 1536` |
| Collision preparation | 8,192 cells, 65,536 raw words, 262,144 prepared references; separate conservative 4 MiB preparation-request ceiling |
| Collision work | Existing 16,384-unit setup proof; source list order, leading zero, duplicates and suffix semantics retained |
| Sight | 16,384 fully expanded node/leaf/seg work; checked scratch allocation and original INT_MIN clipping exclusion |
| Per-operation sight budget | At most 64 action queries and 1,048,576 conservatively charged diagnostic work units, including reserved extra per-line observations |
| Actor count | At most 64 slots; at most four native Demon owners before the stable unbound suffix |
| Actor coordinates | Read-only actor XY/Z within ±1,024; pre-tic owner XY within ±978 and suffix XY within ±994; callback/spawn XY within ±1,008, map Demons within ±978 |
| Vertical setup | Explicit absolute spawn Z within ±1,018; pre-tic vertical momentum within ±64 and Z plus momentum within ±1,024 |
| Momentum and dimensions | XY momentum within ±64 in the new profile; original XY clamp 30 and Demon Chase speed 10. Demon radius/height 30/56; body target radius/height 16/56; exact registry/effect dimensions must fit static sectors |
| THINGS | At most 256 records; original lookup/filter/nomonsters order, DoomEd 3002 only after filtering; source raw state, floor/BSP, spawnpoint, RNG and insertion order |
| Selected THING angles | Truncated `angle/45` in `[-4,3]`, encoded degrees `[-224,179]`. Outside values reject atomically because the original signed ANG45 product overflows. All eight BAM octants can use signed equivalent encodings |
| Gameplay | Original normal stand/run, Look/hearing/roster, Chase/movement, melee, shared direct damage, pain/death/Fall and persistent corpses |
| Targets and sources | Stable unbound body 135 for live AI/sound/roster targets; existing effect lifecycles 37/38 remain distinct. No real player pointers or native infighting beyond the inherited synthetic lethal/ignored slice |
| Statistics/restart | Separate available map kills and single-player fallback credit; retained COUNTKILL, disabled respawn; fresh restart resets level state and continues RNG |

Geometry admission is broader than actor admission. This does not allow Demon
execution at ±2,048 actor coordinates. The line-norm and signed-angle restrictions
also mean this is not an unconditional superset of older profiles. Historical
Profiles 17 and 19 retain their contracts and byte fixtures.

Direct hitscan/autoaim, other attacks, player bindings/damage, multiplayer,
Fast/Nightmare, resurrection, dynamic sectors and specials reject. The source's
64-cell path-traversal and 128-intercept limits are unchanged. Unsupported types
never become generic actors; unknown enabled types remain recoverable batch errors.

Whole-tic and nested direct actions use a bounded actor-state plan. Any recoverable
error preserves actors, identities, RNG, lists, statistics, commands and both event
queues. Checked reservations/work limits are distinct from inherited process-level
Rust allocation failure for ordinary observation/snapshot vectors. Preparation
accounting excludes parsed Map storage, canonical caches, the small actor plan,
observations and allocator overhead; it is not a process RSS limit.

[Source findings](M3F3_REFERENCE.md) explain the arithmetic and original signed-angle
failure. [Architecture](ARCHITECTURE.md#expanded-map-demon-composition) explains
planning; [evidence](validation/M3F3.md) separates genuine C, independent canonical
encoding, safety policies and host measurements.

## Demon runtime restrictions

Map-spawned Demon thinkers precede stable unbound type 135 test targets/effects.
Live Demon, perception-roster and sound targets must use that target role. Real
player bindings reject. Demon actors can enter only through the atomic map setup
batch, not ordinary in-tic spawning. Source identifiers/state aliases remain the
documented adaptation in the support matrix above. The [source investigation](M3D3I_REFERENCE.md)
explains its ownership proof. Capacity/work/checked-statistics overflow and stale
identities reject before changing canonical state.

Synthetic cross-native lethal or already-ignored damage is admitted because source
returns before retargeting. Nonlethal cross-native damage rejects; autonomous AI
still cannot acquire native targets. No generalized infighting or projectile
interaction is implied. Source/inflictor identity is validated even for ignored
requests as a Rust safety policy. Unsupported actions never silently no-op.

## Deferred systems

Full player damage/inventory/armor/powers/scoring, multiplayer, Fast/Nightmare
execution, respawning, resurrection, dynamic sectors, doors, specials, complete
map/actor spawning, generalized monster interactions, demos and presentation remain
unsupported. Spectre metadata recognition is not Spectre gameplay compatibility.
Source undefined behavior is not a compatibility target; checked Rust rejection
and adapter-defined evaluation order are recorded separately.

Read [M3D3J_REFERENCE](M3D3J_REFERENCE.md) for exact statistics chronology,
[STATE_FORMAT](STATE_FORMAT.md) for schemas, and [TESTING](TESTING.md) to reproduce.

## Original identity metadata

M3f4 represents all 137 original actor, 967 state and 138 sprite identities and
all original actor/state descriptor records. The [mapping guide](ACTOR_IDENTITIES.md)
defines checked conversions and named properties. This is complete metadata
recognition, not complete actor/state execution. M3f4 admitted no new gameplay
profile or behavior. Existing original decorative/effect/lifecycle
slices retain their existing checks; the focused integration uses MT_TFOG through
the same typed owner, raw spawn, state entry and thinker services.

Canonical-to-registry resolution compares only the requested complete record at
its original ordinal. It does not certify every referenced state or live actor
context. Returned slots must still pass existing action/flag/role/map checks.
Altered records, reordered names and fixture replacements reject. Native Demon
136 and state aliases remain explicit fixture adaptations. The separate binding
below connects MT_SERGEANT to its already admitted subset. MT_SHADOWS, players,
raise states and other unimplemented dependencies remain metadata without new
gameplay permission. Spectre gameplay, real
players, infighting, resurrection and Nightmare/respawn remain deferred.

## Canonical Demon execution requests

The M3f5 binding connects original MT_SERGEANT (Demon/Pinky, not Shotgun Guy) to
the existing [Profile20 closure](#expanded-map-demon-profile). It derives native
metadata from canonical records and admits typed canonical batch/state requests
through existing services. All original ordinals are preserved in the source
registry. All runtime limits, native identity bytes, event order and game-context
restrictions remain the Profile20 contract; there is no Profile21.

The binding covers normal stand, run, melee, pain and death definitions. Only
stand/run/attack have direct entry permission; pain/death remain damage-controlled.
Raise references are represented but have no binding. Invalid ordinal conversion,
unsupported identity/state requests, wrong owners and stale handles reject safely.
MT_SHADOWS (Spectre) and MT_SHOTGUY (Shotgun Guy) are explicitly excluded.

Canonical definition queries are projections through a validated correspondence,
not raw original IDs in ActorLife/DSIM. Full actor compatibility, real players,
generalized infighting, Nightmare/respawn, resurrection, dynamic sectors, new
hitscan/projectiles and rendering remain deferred. See the
[mapping](ACTOR_IDENTITIES.md#canonical-demon-execution-binding),
[source investigation](M3F5_REFERENCE.md) and [observations](validation/M3F5.md).

## Native runtime identity boundary

M3f6 retains all existing stored IDs and admission bounds. The typed native Demon
view covers only the validated Profile20 type 136/21-state execution correspondence.
It is separate from original identities and live handles; it grants no additional
state entry, actor, raise, Spectre or player behavior. The complete original registry
remains metadata, not executable support. Earlier profiles retain their distinct
native fixtures, including unrelated uses of136.

Raw canonical type 12 or SARG475–501 requests are not silently interpreted as native
owner/state requests. Use the explicit canonical APIs or checked native projection.
Pain/death observations are representable native values but direct entry still
rejects. All historical public raw observations, event payloads and DSIM-v20 remain
unchanged. See [mapping](ACTOR_IDENTITIES.md#native-runtime-identity-boundary),
[audit](M3F6_REFERENCE.md) and [evidence](validation/M3F6.md).

## Original-ID Demon profile

`Profile::OriginalDemon` is opt-in. `OriginalExecutionProfile::DemonNormalV1`
identifies its role contract independently of diagnostic version. The
[registry](ACTOR_IDENTITIES.md#original-id-execution-registry) keeps complete
original metadata pristine and stores admitted Demon type 12/states 475–495 directly.
Normal gameplay is the existing Profile20 closure, not broader Demon support.
Direct stand/run/melee entry is guarded; pain/death remain damage-controlled and
raise 496–501 remains metadata only. MT_SHADOWS, MT_SHOTGUY, MT_PLAYER and original
MT_MISC85/86 have no executable actor role. Existing puff/blood effects and an
explicit synthetic unbound body context retain their bounded roles.

All [expanded-map restrictions](#expanded-map-demon-profile), four-owner/64-slot
limits, geometry/work bounds, THING filter order, normal timing, signed-angle
proof, checked operation atomicity and unbound statistics policy apply. Source
DoomEd lookup precedes admission; source decorations at 135/136 now retain their
true metadata and reject if selected. This is distinct from their overwritten
fixture metadata in old profiles. Source-ID metadata does not authorize new map
placements, callbacks, players, infighting, hitscan, Fast, respawn or resurrection.

This profile reuses the immutable M3f5 independent C scenarios. New comparisons
exercise actual source-ID storage and diagnostic interpretation; they are not new
C-input or whole-game coverage. Profiles 1–20, native aliases and DSIM-v20 bytes
remain unchanged. V21 is a read-only diagnostic contract, not a savegame; see
[format and validation limits](STATE_FORMAT.md#dsim-v21--original-id-demon-diagnostics).

## Context and target-lifetime contract

The original-ID profile admits an additive `with_execution_context` interface.
Its lifetime-branded instance tokens belong to one exclusive callback; they cannot
escape or mix with another scope. Explicit local raw-handle admission checks
generation, actual definition and the caller's expected definition. This is an
access contract over the existing Simulation, not a second lifetime manager.
Metadata recognition and token possession alone do not grant gameplay permission.
Historical profiles reject this scope before invoking its callback; their raw APIs
and diagnostics retain their prior meanings.

| Reference condition | Observation and mutation policy |
| --- | --- |
| Current nonpending storage | `Live`, including corpses; health and shootability remain separate |
| Pending removal | Readable until traversal; spatial membership already unlinked; acquisition rejects |
| Latest retired occupant, slot still vacant | `Retired`; actor access rejects; stored references remain inert |
| Slot reused / older generation | `StaleGeneration`; never resolves to the new occupant |
| Unissued current generation in vacant slot | `Vacant`, not a live actor |
| Generation zero / unallocated slot | `InvalidGeneration` / `OutOfRange`; admission rejects |
| No stored target | `TargetReference::Absent`; no implicit mutation |
| Current target with incompatible role | `Incompatible` observation; acquisition remains rejected |

Generation exhaustion seals a slot instead of wrapping. `Retired` above describes
the most recent occupant; the existing DSIM slot `.retired` flag means permanent
generation exhaustion. Availability is derived without adding a history table.

Target observation reuses the shared definition-role guard, consumes no
RNG and never clears/rebinds a reference. An available body corpse is not proof of
combat permission. The Demon cannot target itself because its target must be a
context body; the existing synthetic-body self policy is retained. Shared damage
can retain a source on an inert body even when that source is not an admissible
Demon target. Observation does not authorize a new autonomous relationship.

Assignment/replacement validate before mutation; clearing does not resolve the
old target. Removal is spatially immediate and thinker-deferred; duplicate marks
are idempotent. Death preserves instance identity and does not universally clear
targets. There is no incoming-reference sweep. A living Demon with an unavailable
target retains the established checked-step rejection until explicit caller repair;
roster/sound references have their own guards and are not automatically repaired.
Dead-owner references can remain inert and observable. Each existing operation
retains its atomicity; successful earlier operations inside a callback remain
applied if a later operation fails.

Raw handles remain store-scoped. Their equal values across worlds/restarts do not
establish provenance; the origin of an erased/re-admitted ID cannot be recovered
from its raw value. No persistent global context ID or cross-world transport is admitted.
No actor/geometry limit, execution profile, registry schema or DSIM version changes.
Real player mo/attacker ownership, general species relationships, tracer/projectile
ownership and resurrection require separate source/capability investigations.
The new boundary grants none of those behaviors. See [mapping](ACTOR_IDENTITIES.md#context-scoped-actor-instances),
[investigation](M3F8_REFERENCE.md) and [qualified evidence](validation/M3F8.md).

## Game-instance provenance

`GameInstance` adds an owning in-process boundary over one Simulation. Fresh
wrapping always creates a distinct opaque InstanceId, even for equal map/registry/
state or cloned Simulation data. Moving the owner preserves its identity. The
wrapper is not Clone; its read-only Simulation view cannot replace state behind
the key. Consuming extraction preserves raw state, while rewrapping creates a new
incarnation. Existing raw APIs remain explicitly local, without inferred provenance.

`GameInstance::with_execution_context` reuses the current original-ID scope and
all role/state/action guards. Wrapping a historical profile does not admit that
scope. An exported InstanceActorRef holds only an opaque key and local handle,
not a world/actor lease. Import checks these conditions in order:

| Condition | Result |
| --- | --- |
| Raw Simulation scope, without an owner key | UnboundContext |
| Candidate without provenance, including decoded local IDs | MissingProvenance |
| Different instance or earlier incarnation | InstanceMismatch, before actor lookup |
| Matching key, invalid/stale/vacant/retired local identity | Existing ContextError::Unavailable classification |
| Matching current actor, wrong expected definition/state/role | Existing identity/definition error |
| Matching current actor, including pending storage or corpse | Scoped observation; existing gameplay guards still apply |

Import/export are pure: no game-state mutation, event, RNG draw or actor allocation.
Generation exhaustion, lowest-slot reuse and scheduling remain unchanged. A pending
actor can be observed but cannot be newly acquired through a rejected target role.
Successful earlier operations remain applied when a later callback import fails;
the scope is not an aggregate transaction.

`restart()` delegates to existing fresh level setup/spawning with continuing RNG.
Only success replaces both state and key. References to the previous incarnation
reject even if new actors have equal slots/generations/definitions. Failure leaves
the prior identity and complete simulation state intact. There is no global live-
instance directory or history index; foreign and pre-restart keys share the explicit
InstanceMismatch classification.

### Trust and unsupported boundaries

Opaque keys compare runtime allocation identity, not numeric contents, map hashes
or snapshot equality. A retained key prevents reuse aliasing but retains neither
the world nor actor. Keys are process-local, with no persistent/serialized meaning,
byte constructor, numeric conversion, remote resolver or authentication guarantee.
There is no savegame restoration or networking implementation. DSIM decoding
remains diagnostic observation and cannot manufacture provenance.

`InstanceActorRef::candidate` is deliberately explicit local routing/readmission.
A caller possessing a trusted local key can assert that a raw candidate should be
interpreted there; an erased origin cannot be recovered or authenticated. Preserved
checked exports reject foreign worlds automatically, whereas deliberately replacing
their provenance changes the caller's assertion. Untrusted remote data would need
a separately trusted resolver/transport policy, not a numeric tag alone.

One small Arc allocation is made per new wrapper/successful restart, under the
existing process-level OOM policy. Import borrows the key; export/reference cloning
shares it without a new allocation. Allocation counts were not instrumented.
No per-actor or Simulation identity field, global counter, wall clock, OS randomness,
RNG consumption, capacity expansion, gameplay profile or DSIM schema is added.
Real-player mo/attacker, new species relationships, persistence and transport remain
separate work. See [architecture](ARCHITECTURE.md#game-instance-provenance),
[audit/design](M3F9_REFERENCE.md) and [observations](validation/M3F9.md).

## Original player body lifecycle

Select `Simulation::original_player_map` or an OriginalExecutionRegistry with
`OriginalExecutionProfile::PlayerBodyLifecycleV1` explicitly. The legacy
`Profile::OriginalDemon` names the shared original-ID service family;
`original_execution_profile()` identifies the selected policy. Historical
DemonNormalV1, ContextRole::UnboundBody137 and all old profiles keep their behavior.
No synthetic body is reclassified as a player.

This policy admits one genuine MT_PLAYER=0, with original S_PLAY/RUN/pain/normal
and extreme-death states149–153/156–173. Fixed settings are Medium, commercial,
single-player, no armor/powers/cheats, pistol presentation request, automap false,
no respawn, static one-sector floor 0 / ceiling 128, existing64 actor-slot maximum and
existing geometry/work budgets. Input movement/buttons, physical inflictors,
manual player state/health/pursuit/momentum changes and unsupported settings reject.
The body stays grounded and stationary; XY must already fit the shared pre-tic
±994-unit proof when spawned or bound. This is a Rust support bound, not an
original-C coordinate limit. Real player starts, spawn-angle loading,
HUD, weapons, inventory and view initialization are not claimed.

`spawn_player_body(PlayerSlot::One, XY)` atomically spawns at the floor with
angle0 and health100, then establishes source mo ownership. A separately raw-spawned
initial canonical body can be bound through checked local/scoped APIs. Bind validates
current generation, nonpending storage, pristine state/health/flags/physics and
initial counters/orientation. Double binding, wrong definition, stale/pending or
invalid handles reject atomically. Player index0 can own a different actor slot;
raw slot equality never establishes ownership. GameInstance import still rejects
foreign or old incarnation references. Raw admission remains explicitly store-local.

Supported direct damage is `P_DamageMobj(target, NULL, source, amount)` for bounded
amount0–1000 and admitted current player/Demon sources or NULL. Source and inflictor
are distinct; physical thrust is explicitly excluded. Player health subtracts and
clamps0; mobj health stays signed. Attacker assigns source including NULL, damagecount
caps100 and the console tactile request is observed before mobj health subtraction.
Nonlethal pain/reaction/retargeting and lethal state selection/jitter use the shared
source-tested damage service and RNG. Self damage may store self as attacker;
NULL clears player.attacker without clearing the distinct mobj.target.

Death preserves the mo binding, sets PST_DEAD, clears MF_SOLID immediately, keeps
source corpse/dropoff flags, quarters height and retains the allocated corpse.
Normal versus extreme death uses the strict health < -100 boundary. Commercial
player scream uses the source -50 threshold. P_DropWeapon's requested pistol-down
state is observed; psprite lowering/weapon execution is excluded. Source environment/
self frags and sole-player monster kill credit are admitted. No score/intermission,
respawn, automatic retirement or PlayerThink/DeathThink damagecount/camera update
is implied. Repeated damage to a dead body is source-ignored after Rust reference
validation; malformed sources still reject without changes.

The player body is unlisted (`playeringame=false`), not an active AI roster or
complete game-loop player. Original all-inactive LookForPlayers does not terminate.
Live Demon AI stepping/manual AI callbacks therefore reject in this policy. A live
canonical Demon can be a controlled direct damage source between tics, then must
be explicitly killed or marked for removal before stepping. Dead-but-allocated
sources and Demon death/corpse actions remain supported. Player-to-Demon damage is
restricted to lethal input, avoiding unsupported nonlethal retarget/infighting.
Automatic targeting and physical Demon melee against the player remain future work.

mo/attacker retain their original opaque handles after pending removal, retirement
and reuse. New damage/binding cannot use pending or unavailable actors. Death alone
keeps a current actor, including a possible attacker corpse. No incoming-reference
sweep clears identities. Generational availability queries never resolve an old
reference to a new occupant. These guarantees are Rust policy, separate from safe
C logical-retirement observations. Restart/respawn reject rather than silently
losing or transferring the player binding.

DSIM-v22 supplies the genuinely new player observations; v20/v21 and their readers
retain their old interpretations. `advance` still captures no Snapshot, `step`
captures after the same shared core, and every supported/rejected operation keeps
its existing transaction boundary. See [schema](STATE_FORMAT.md#dsim-v22--original-player-body),
[source investigation](M3G2_REFERENCE.md), [tests](TESTING.md#original-player-body-validation)
and [measured evidence](validation/M3G2.md). No complete player gameplay claim follows.

## Original player grounded knockback

Select `Simulation::original_player_knockback_map` or explicit
`OriginalExecutionProfile::PlayerBodyKnockbackV1`. This is a separate capability;
PlayerBodyLifecycleV1 stays stationary and continues rejecting physical inflictors.
The constructor and new capability-enum variant are explicit API additions;
exhaustive matches on OriginalExecutionProfile need the new arm. The historical
Profile enum and existing signatures/variant meanings stay unchanged.
Both use one genuine original MT_PLAYER=0 with the same checked mo/player ownership,
source health, pain/death states and allocated corpse semantics. Canonical identity
representation still does not imply general executable support.

Settings remain Medium/commercial/single-player/pistol, no armor/powers/cheats,
automap/respawn false, unlisted player, zero commands, no PlayerThink/DeathThink.
Geometry is one static sector floor 0 / ceiling 128 with axis-aligned, one-sided walls
whose flags are ML_BLOCKING. Existing actor/geometry/work limits are unchanged.
Angled/two-sided walls, steps/dropoffs, airborne player motion, dynamic sectors and
unsupported settings reject during construction. Player/body spawn/binding keeps
the existing ±994-unit initial XY boundary and initial zero momentum.

A controlled DamageRequest may supply a distinct optional attacker source and
physical inflictor. Both references must be current and nonpending original player
or Demon instances; a dead-but-allocated source remains valid. The physical origin
and player target must be at floor 0 with zero vertical momentum. New momentum may
only enter through shared damage; direct XY momentum control in this policy rejects.
Physical damage to another actor is outside this new admission. No new weapon,
projectile, infighting or automatic melee/acquisition system is implied.

Eligible hits require an already executable thinker closure: all unlisted Demon
owners must be dying/corpses or pending removal before the hit. Their creation
retains the existing map-THING admission; raw Demon spawning remains rejected.
Old no-inflictor controlled source interactions keep their lifecycle boundaries.
A valid definition or available handle alone never grants permission to execute AI.

Damage 0..1000 is accepted only when its original ordered thrust arithmetic,
checked momentum addition and forward travel proof succeed. Let M=max(abs XY
momentum) in raw 16.16 bits after impulse: M<=30U and each
abs(position)+ceil(160M/3)<=994U. This conservative support bound proves later
axis-wall/friction tics remain executable; it is not an original-C coordinate
limit or a gameplay clamp. Additional hits must establish it again. Even a small
hit near the old coordinate boundary may reject atomically instead of committing
an unsupported future position. See the [derivation](M3G3_REFERENCE.md#forward-coordinate-proof).

Actual source thrust uses point-to-angle, mass 100, integer damage*(FRACUNIT>>3)*100/
mass and fine cosine/sine FixedMul additions before player-specific health,
attacker/damagecount/tactile and signed mobj health. The reverse-thrust branch is
unreachable because both Z values are 0; no thrust RNG draw or reverse-thrust
coverage is claimed. Zero damage still follows source pain processing. Existing
pain/death RNG, flags, statistics and ordered event semantics remain shared.

Subsequent tics perform real shared XY movement, source positive-only subdivision,
things-before-lines collision, source BLOCKMAP candidate order, spatial relinking,
axis-wall sliding/stairstep, grounded friction 0xe800 and strict stop speed 0x1000.
A stopped RUN frame enters S_PLAY before state countdown. Lethal hits preserve
momentum: the allocated player corpse can move, decay and stop while mo/attacker
stay bound. Death, explicit removal and generational reuse remain distinct. There
is no automatic respawn or reference rebinding; stale handles never select a reused
occupant. Rejected operations preserve complete actor/player/RNG/statistics/spatial/
event state, including late checked failures.

[DSIM-v22](STATE_FORMAT.md#dsim-v22-grounded-player-execution-policy) already
represents these observations. Wire policy 3 and a distinct grounded context schema
select the new behavior with unchanged field/event layout. Old v20/v21/v22 policy 2
fixtures/readers retain their meanings. advance remains diagnostic-free, and step
captures only after the same core. [Evidence](validation/M3G3.md) distinguishes new
C observations, reused history and Rust-only safety policies. Full player controls,
PlayerThink, roster/automatic Demon attacks, view/camera, weapons, inventory,
armor/powers, air movement, respawn and multiplayer remain unsupported.


## Original player command movement

Select `Simulation::original_player_command_map` or
`OriginalExecutionProfile::PlayerCommandMovementV1`. This is a new explicit
capability, not a change to PlayerBodyLifecycleV1 or PlayerBodyKnockbackV1. Their
old command rejection, diagnostics and inputs remain unchanged. All three use
one genuine MT_PLAYER=0 and the existing player/mo ownership, damage, pain/death,
corpse retention, scoped handles and game-instance provenance. Context UnboundBody
remains synthetic. An exhaustive match on OriginalExecutionProfile needs the new
variant; historical Profile variants and existing API signatures are unchanged.

Only source player slot One executes. TicCommand forward/side each range -50..50
(g_game MAXPLMOVE), angle_turn accepts all signed i16 values, and buttons must be
zero. Other command slots must be default. Medium/commercial/pistol, no armor,
powers, cheats or active roster, and the grounded flat axis-wall geometry/limits
of the knockback capability remain required. MF_JUSTATTACKED rewriting is excluded.
The bound live body's existing reaction_time must be in 0..255. Controlled
`set_initial_player_reaction_time` accepts u8 on a current live bound body only
before the first successful tic; default spawn is unchanged. This setup operation
is not teleport gameplay or a second timer.

The single player stage runs before every actor thinker. If reaction_time is
nonzero it decrements once and suppresses all P_MovePlayer work for that tic,
including turning; movement resumes on the next tic that begins with zero.
Commands still reach XY/friction's stop condition. Source damage can reset the
counter before the stage. Otherwise angle updates first, forward thrust uses that
new angle, then sideways thrust uses angle minus ANG90. Integer fine tables and
FixedMul accumulate existing momentum. Standing directional input immediately
enters S_PLAY_RUN1; ordinary thinker countdown continues its source graph. When
existing XY movement stops a RUN frame with zero directional input, S_PLAY entry
precedes countdown. An already zero-momentum RUN state is not forcibly reset.
Turning alone does not enter RUN. Pain does not disable an otherwise live command.

Negative original signed left shifts are undefined ISO C. Supported turns use the
explicit unsigned two's-complement bit-pattern interpretation of the existing
historical observer adapter. Defined literal C cases are compared separately;
there is no portable ISO-C equivalence claim for negative signed shifts. See
[arithmetic investigation](M3G4_REFERENCE.md).

Shared physical damage/knockback composes without replacing momentum. New command
attempts require M=max(abs XY momentum)<=30U and
abs(position)+4M+8 raw bits<=994U for nonzero M; zero M requires only the coordinate
bound. After actual shared movement, the staged body must satisfy the existing
abs(position)+ceil(160M/3)<=994U travel envelope. These conservative Rust support
checks preserve a safe zero-command continuation, not original coordinate limits
or clamps. A source-valid command can reject near this bounded domain's edges.
Any early or late recoverable error preserves complete authoritative state,
including angle, reaction, commands, RNG, both event queues and spatial order.

Unbound, pending-removal, retired/stale or dead bodies accept only default inactive
input through the ordinary thinker/removal pass; nonzero input rejects. Pending
and stale handles never redirect to a reused slot. Dead allocated corpses keep
physical motion and ownership; no command revives them. Malformed current body
identity/health/state rejects. Pause retains its historical precedence. `advance`
remains diagnostic-free; `step` uses the same core and captures once on success.

This is a declared P_PlayerThink movement conditional plus P_MovePlayer, not full
PlayerThink, P_Ticker, P_DeathThink or player roster execution. View/camera/bob,
weapons/buttons, inventory/powers/cheats, special sectors, airborne controls,
respawn/multiplayer, arbitrary angled/two-sided walls and automatic Demon
perception/acquisition remain unsupported. Dying/corpse Demon origins retain
existing controlled interaction semantics; no live AI/roster is silently enabled.

## Active original player and Demon acquisition

Select `Simulation::original_player_demon_map` or the separate
`OriginalExecutionProfile::PlayerDemonAcquisitionV1`. It composes the existing
command/knockback/player lifecycle with canonical Demon visual acquisition and
bounded nonmelee pursuit. Historical DemonNormalV1 and player policies are unchanged.
Metadata still represents every original actor/state; only admitted roles execute.

The fixed source roster is `[true,false,false,false]`: only PlayerSlot::One
(source index0) is active, regardless of its actor storage slot or health.
`spawn_player_body` or checked initial binding associates the genuine MT_PLAYER=0
with the existing OriginalPlayer. `original_player_roster()` derives current
health/mo from that record; it is an observation, not a separate mutable roster.
`perception_players()` remains the historical fixture-array observation and does
not describe this capability's active player. Its fixture registration setter
rejects here, including attempts to deactivate or fabricate players.

Before any unpaused tic or search, the player must have a current, nonpending,
correct-generation owned body with consistent definition/health/state. Incomplete
setup may be observed but cannot execute. Player activity persists when health
becomes zero; the allocated corpse remains owned but source search ignores its
player health. Removing this fixed active body rejects, including after death;
roster deactivation, body replacement and respawn require a future capability.
Other actor retirement/reuse retains existing generation safety. Foreign instance
references reject at existing provenance boundaries; raw ActorId remains local.
Damage can also retain a source in the player's mobj target, separately from
OriginalPlayer.attacker. Neither reference registers a player or enables player AI.
The scoped target view classifies compatibility with the perception-target role:
a retained Demon source can be Incompatible with that role while remaining a valid
stored identity. Actor fields/availability and source lifetime are separate observations.

At most one MT_SERGEANT=12 may enter through the original Demon THING batch,
before the player/effect suffix. Zero Demons is supported. This narrows the new
capability without changing historical four-owner or 64-total-slot bounds. All
existing Medium/commercial, grounded one-flat-sector, static axis-wall, geometry,
coordinate/travel and command bounds still apply. Player commands remain slot0,
forward/side -50..50, signed i16 turns under the declared bit interpretation,
buttons0 and default other slots. Physical damage composes through existing shared
services; it is not proof of autonomous melee. No sector soundtarget/noise
propagation is admitted; AMBUSH can use the source visual fallback.

The existing scanner preserves source lastlook, stopping slot and active encounter
count, then player-health/sight/facing checks. It does not sort by distance. Rear
angle strictly between ANG90/ANG270 permits acquisition only within approximate
64-unit distance. Initial lastlook1 with only player0 active stops at that slot
before eligibility on its first scan. Other initial lastlook values and repeated
scans keep original progression. New execution never feeds an empty active roster
to source scanning: original all-inactive search may not terminate. The historical
Rust EmptyRoster safety policy remains isolated in its historical configuration.

A_Look resets threshold, scans, assigns the real player's current ActorId, emits
the single Demon wake sound without sound-selection RNG, and immediately enters
SARG see/Chase chronology. Shared A_Chase supports target/reaction/threshold/facing,
movement direction, original collision/sight and RNG. Commands update player
angle/momentum before thinkers; XY moves at the player's own thinker position.
A Demon inserted first therefore initially sees pre-player-XY coordinates.

SARG attack states and autonomous melee are explicitly unexecutable here. If the
actual source melee check succeeds, the complete operation rejects before attack
sound/state/damage commit; chase does not silently skip the branch. Controlled
initial direction/state setup is distinct from automatic gameplay. Death or loss
of target shootability invokes source allaround fallback; failed scanning and
spawn entry retain an allocated dead target handle. Pending, retired, stale or
otherwise incompatible target/body references reject before execution and never
resolve to a different slot occupant. Nonshootable living player corruption rejects
as Rust invariant policy, not a claim about a separately admitted source branch.

Both stepping APIs share the same transaction; late rejection preserves player,
actor, targets, RNG, statistics, spatial/thinker order, commands, tic and both event
queues. Paused input retains historical no-op precedence. `advance` captures no
Snapshot; `step` captures once after successful advancement. DSIM22 policy5/context
select the fixed active contract with unchanged layout; [STATE_FORMAT](STATE_FORMAT.md#dsim-v22-active-player-demon-policy)
owns its precise identifiers.

Full P_PlayerThink/P_Ticker/P_DeathThink, view/camera, weapons, inventory, powers,
armor/cheats, roster changes, respawn/multiplayer, generalized targeting/infighting,
other monster species, sound propagation, arbitrary geometry and autonomous combat
remain unsupported. [Source findings](M3G5_REFERENCE.md) and
[observed evidence](validation/M3G5.md) qualify this bounded gameplay composition.

## Autonomous original Demon-player melee

Select `Simulation::original_player_demon_melee_map` or the separate
`OriginalExecutionProfile::PlayerDemonMeleeV1`. This adds a public capability
variant and constructor; callers with exhaustive policy matches need a new arm.
It composes the existing fixed active player0, genuine MT_PLAYER=0 ownership,
map-THING MT_SERGEANT=12, commands, acquisition and pursuit. Historical policies,
including acquisition-only PlayerDemonAcquisitionV1, keep their original inputs,
state permissions and rejection behavior. No actor or geometry bound expands.

The admitted domain retains Medium/commercial single-player settings, at most one
Demon, one active owned player, floor0/ceiling128, static one-sector axis-aligned
one-sided walls, source BLOCKMAP/sight order and existing arithmetic/work limits.
Zero Demons is allowed. The same player slot0 commands execute before thinkers:
forward/side -50..50, signed i16 turns under the declared bit interpretation,
buttons0 and default other slots. No armor, powers, cheats, soundtarget propagation,
Fast/Nightmare, inventory, weapon or respawn configuration is implied.

Original A_Chase chooses melee using current approximate distance and sight, emits
the attack sound and immediately enters S_SARG_ATK1. ATK1/2/3 retain original
ordinals485/486/487, durations8/8/8 and Face/Face/SargAttack actions. The damaging
action occurs16 tics after selection, and returns to run chronology8 tics later.
The player can move during windup. SargAttack returns on absent target; otherwise
it faces and checks range again. A pristine player requires distance strictly
below60 units plus successful sight. A range miss consumes no damage draw.

An accepted range check draws exactly `((P_Random()%10)+1)*4`, giving4–40 damage
in increments of4. The current Demon is explicitly both source and inflictor in
DamageRequest. The existing shared damage service applies mass-scaled fixed-point
thrust before player health/attacker/damagecount and signed mobj health. Incoming
momentum includes the command stage and prior impulses; the player's later thinker
then performs existing XY/friction/sliding. The live inflictor remains executable
afterward and need not be killed or marked for removal.

Current ownership, generation, definition, state, health and grounded assumptions
are checked. The shared impulse/travel proof must accept the actual staged result:
component magnitude<=30 units and the existing conservative `160/3` travel envelope
inside±994. These are Rust admission limits, not original world limits or a clamp.
A well-defined C impulse outside that proof is an atomic unsupported operation.
Player SHADOW/power flags are outside the accepted source settings. Both z values
are0, so reverse-thrust height>64 and its conditional random draw are unreachable.

Nonlethal damage preserves the source pain draw, reaction reset and conditional
mobj retargeting. Player chance255 does not trigger pain on draw255. Player health
clamps0 while mobj health can become negative. Lethal damage enters the existing
player death path, clears solidity/shootability, retains momentum, sets PST_DEAD
and leaves an allocated corpse. Kill returns before the nonlethal reaction reset.
Death does not deactivate the roster, clear mo/attacker or erase the Demon's target.
Subsequent Chase handles nonshootability through the existing source scan/spawn
fallback; no actor-slot reuse can redirect retained generations.

Melee from positive health cannot produce mobj health below-39. Autonomous extreme
death(<-100) and the commercial below-50 high scream are therefore unreachable.
Existing controlled-damage extreme-death behavior remains separate coverage. If a
target dies during windup but remains in range, SargAttack can still draw damage;
P_DamageMobj then ignores the dead/nonshootable body without thrust, health or new
pain/death effects. This differs from a range miss and from invalid storage.

Existing live-owner controlled original state-entry restrictions still apply;
player pain/death remain damage-controlled and raise is excluded. The legacy
synthetic `demon_melee_attack` API remains unavailable with a genuine player.
Normal autonomous execution needs only advance/step, not that helper or injected
damage. Fixed player removal/rebinding, all-inactive roster transitions and
foreign/stale/pending relationship admission retain their checked boundaries.

All late failures discard the bounded existing transaction, including command,
actor/player fields, impulse, references, RNG, statistics, spatial/thinker order,
tic and pending/last events. No release exception or full-world clone is added.
`advance` constructs no implicit Snapshot; `step` uses the same core and captures
once after success. [DSIM22 policy6](STATE_FORMAT.md#dsim-v22-autonomous-demon-player-melee-policy)
uses the existing layout with a distinct context; policy5 remains no-melee.

This is a bounded command gate plus original actor thinkers, not complete
P_Ticker/P_PlayerThink/P_DeathThink. View/camera, damagecount decay, weapons,
pickups, armor/powers/cheats, roster changes, respawn/multiplayer, other monsters,
infighting, dynamic sectors, airborne/general geometry and rendering remain
excluded. [Source findings](M3G6_REFERENCE.md) and [observed evidence](validation/M3G6.md)
separate source behavior, reusable historical evidence and Rust safety policy.


## Genuine player pistol combat

Select `Simulation::original_player_demon_pistol_map` or the explicit
`OriginalExecutionProfile::PlayerDemonPistolV1` (wire7). Exhaustive public policy
matches need a new arm. It composes the existing genuine MT_PLAYER0, at most one
map-spawned MT_SERGEANT12, fixed active player0, commands, acquisition, pursuit and
autonomous melee. Older policies, including the no-weapon melee policy6, retain
all prior inputs, diagnostics and rejection behavior. No synthetic body is
reclassified as a player.

After spawning/binding the fresh live health100 body, call
`initialize_player_pistol(PlayerSlot::One, INITIAL_PISTOL_BULLETS)` before any tic.
G_PlayerReborn uses50; explicit initial values0..200 are controlled inventory
setup, not pickups. Double setup, wrong capability, absent/stale/pending body and
noninitial setup reject. PistolState observations are copies of weapon-only
fields; mo/health/state/attacker remain authoritative in the existing player.
Fixed player retirement/rebinding and foreign-instance admission remain checked.

Settings are Medium/commercial/single-player, floor0/ceiling128, one static flat
sector, axis-aligned one-sided walls, at most128 trace lines and existing64 actor,
coordinate/travel, BLOCKMAP, sight and work bounds. Turning and forward/side
-50..50 retain the prior arithmetic qualification. Only BT_ATTACK is newly
admitted; other player slots must remain default. Dead-player commands must be
default, while existing corpse physics and source weapon lowering continue.
There is no full PlayerThink/DeathThink, view/camera or damagecount decay.

Original pistol overlays use10..17 plus immediate LightDone1/Null0, distinct from
mobj states. Body attack states154/155 execute under this capability and the separately selected
weapon expansion capability. Setup
immediately raises128 to122; continuous attack can select at tic15 and first
fires at19. Pistol attack durations4/6/4/5 and immediate ReFire yield a14-tic held
shot interval. Ammo decrements at FirePistol, after sound/body-state and before
flash/aim/shot. Release does not cancel a selected windup. Refire resets when the
source ReFire action observes release, not at arbitrary button-change time.

P_GunShot draws5/10/15 damage first. An accurate refire0 shot has no spread draws;
inaccurate shots draw two ordered spread values. Autoaim tries the current angle,
+2^26 and-2^26 at1024 units; it changes slope, not the actual bullet angle. Bullet
range is2048. Both use the same source BLOCKMAP/intercept service. Original
P_PathTraverse processes collected intercepts after its bounded64-cell loop even
when the endpoint was not reached; this newly observed pistol domain admits that
finite result. Historical trace APIs retain their stricter endpoint rejection.
Puff37 and Blood38 are real allocated thinkers with source RNG and same-tic
thinker insertion, not presentation placeholders. Walls and misses cannot
fabricate Demon damage.

One-sector P_NoiseAlert sets the genuine player's sector soundtarget at
FireWeapon selection, before the later bullet action. Nonambush Look may wake
from sound despite rear facing or a wall; ambush retains its sight check. This is
not generalized sound propagation. Direct damage retaliation is separately
performed by the shared damage service. Old acquisition/melee policies still
reject sector soundtargets as before.

A real hit uses the current owned player as both source and inflictor, applies
checked mass400 Demon thrust, then original health/pain/reaction/retarget/death
processing. The existing single-player kill counter is the source player kill
count; no duplicate statistic is stored. Surviving AI and autonomous Demon melee
continue through the same core. Player death immediately lowers the weapon,
retains mo/attacker and active roster membership, and leaves an allocated moving
corpse. A dead player never fires or automatically respawns.

Empty ammo may idle, but source CheckAmmo fallback requires an unadmitted weapon.
That continuation rejects atomically, including a late ReFire check after
release. There is no infinite ammo or silently executed fist. Trace, capacity,
impulse, ownership, unsupported-action and later AI failures preserve ammo,
psprites, health, references, RNG, statistics, soundtarget, ordering, queues and
tic. `advance` constructs no implicit Snapshot; `step` captures once from the
same execution. [DSIM23](STATE_FORMAT.md#dsim-v23-genuine-player-pistol) adds only
the required dynamic weapon fields; historical20/21/22 remain intact.

Other weapons, switching, pickups, armor/powers/cheats, respawn, multiplayer,
other monsters, infighting, full player thinkers, airborne/general geometry,
dynamic sectors, generalized sound propagation and rendering remain unsupported.
New source scenarios use defined map angles0/-180 and nonnegative turns; inherited
negative-turn and ordered spread/jitter qualifications are not new portable
ISO-C evidence. [Investigation](M3G7_REFERENCE.md),
[reproduction](TESTING.md#genuine-player-pistol) and [observations](validation/M3G7.md)
state exactly what source comparisons and Rust safety tests establish.

## Original player weapon expansion

Opt in with `original_player_demon_weapons_map` / `PlayerDemonWeaponsV1`.
The fixed active slot0 owns genuine MT_PLAYER0; at most one map-spawned
MT_SERGEANT12 shares the original command, acquisition, pursuit, melee and damage
core. Initialize once with `initialize_player_weapons(PlayerSlot::One,
WeaponInventory::default())` after fresh body binding, before any tic. Default
means fist+pistol, pistol ready,50 bullets. Alternative ready/owned/loadouts are
explicit controlled source-compatible setup: only fist/pistol/shotgun/chaingun,
fist+pistol always owned, bullets0..200, shells0..50, cells/rockets0. This is not
pickup gameplay. Wrong ready ownership, foreign/stale/pending body, double or
noninitial setup rejects. `original_player_weapons` returns a copy of weapon-only
observations; OriginalPlayer still owns body, health, attacker and lifetime.

Medium/commercial settings, grounded movement, one static floor0/ceiling128 sector,
axis-aligned one-sided walls,128 trace lines,64 actors and all earlier coordinate,
travel, sight and work proofs remain. Forward/side commands are -50..50; turning
retains the original arithmetic qualification. Player0 accepts BT_ATTACK and
BT_CHANGE with only the four supported weapon selections. Unsupported button bits,
weapon-mask bits without CHANGE and commands in other slots reject. A request for
an unowned supported weapon follows source behavior and does nothing; it never
grants ownership or ammunition. Dead-player commands must be default.

Empty ammunition selects an owned usable chaingun, shotgun, pistol or fist in
source priority. Pending selection enters the actual ready weapon's downstate;
ready changes at bottom, followed by raise and ready actions. Requests are not
instantaneous. A pending manual selection can be superseded by the source
CheckAmmo branch. Attackdown/refire, held attack, release and selected windup
retain source chronology. Empty chaingun still makes the source sound request
before returning without bullet/flash/ammo/damage draws. No infinite ammo exists.

Unpowered fists draw damage2..20 then two spread draws, aim/attack at64 units,
and use real current intercepts. An actual actor hit produces the source punch
sound and faces the target's **post-damage** position. Shotgun consumes one shell,
aims once, then resolves seven independent inaccurate bullets sequentially through
effects, damage, pain/death and retaliation. Chaingun uses pistol bullet damage but
its own cadence, two initial accurate shots, refire spread and state-dependent
flashes. No shotgun aggregation or precomputed future hits/health is used.
Pistol behavior retains the earlier source algorithm within this new capability.

Immediate attack actions can execute before P_FireWeapon's NoiseAlert: chaingun
can damage/retarget the Demon first. The same admitted one-sector soundtarget then
feeds existing AI. Fist/pistol/shotgun windups place noise before their later attack
action. All effects are real Puff/Blood thinkers; the same Demon can hurt or kill
the genuine player while weapons execute. Death lowers the actual ready weapon,
retains pending/ownership/roster/body/attacker, and continues supported corpse
physics. No death camera, respawn or dead-player firing is implemented.

Every recoverable failure discards the existing bounded transaction, including
partial pellet effects/damage, ammo/psprites, commands, soundtarget, references,
RNG, statistics, ordering, events and tic. `advance` constructs no implicit
Snapshot; `step` runs that core and captures once. Historical pistol-only and
synthetic weapon policies retain all prior rejection and diagnostic contracts.
[DSIM24](STATE_FORMAT.md#dsim-v24-original-player-weapon-expansion) records the
additional dynamic ownership, ready/pending and shell fields explicitly.

No Berserk, other weapons, weapon pickups, armor/powers/cheats, inventory expansion,
full PlayerThink/DeathThink/view, respawn, multiplayer, additional monsters,
infighting, general sound propagation, arbitrary geometry, dynamic sectors or
rendering is admitted. Source scenarios are a qualified bounded command/weapon
stage before original thinkers, not full ticker equivalence. The source-only
out-of-enclosing-wall scenario eventually exceeds the unchanged Rust978-unit
Demon admission proof; Rust compares its supported prefix and rejects atomically
rather than following that later C path. See [source qualifications](M3G8_REFERENCE_INVESTIGATION.md),
[reproduction](TESTING.md#original-player-weapon-expansion) and
[observations](validation/M3G8.md).


## Original world pickups

`PlayerDemonPickupsV1` (wire policy9) composes the four-weapon/player/Demon
capability with one actual original player0, fixed active roster, at most one
map Demon and at most16 placed plus controlled dropped items within64 actor slots.
Medium/commercial settings and existing grounded one-sector, flat0/128,
axis-aligned one-sided wall, coordinate/travel, sight and trace budgets remain.
There is one player, actor store, inventory, movement service and thinker pass.

| Original item | DoomEd | Executable grant |
| --- | ---: | --- |
| MT_CLIP |2007|10 bullets; controlled dropped Clip gives5 |
| MT_MISC17 |2048|50 bullets |
| MT_MISC22 |2008|4 shells |
| MT_MISC23 |2049|20 shells |
| MT_MISC10 |2011|10 health |
| MT_MISC11 |2012|25 health |
| MT_SHOTGUN |2001|Shotgun ownership and8 shells; dropped4 |
| MT_CHAINGUN |2002|Chaingun ownership and20 bullets; dropped10 |

`spawn_map_things` admits this subset in the existing atomic THING pass, retaining
single-player/skill filtering, angle admission and input order. Demon count and
item count are separate. Map kill totals follow MF_COUNTKILL; none of these items
has MF_COUNTITEM, so level.total_items and source player itemcount remain0.
Items may surround the map Demon in placement order; all precede the bound body
and effect suffix. Original monster identities remain distinct from weapon items.
Raw `spawn_type` does not grant item execution. `spawn_dropped_pickup` admits only
Clip/Shotgun/Chaingun before body binding and tic0, through actual raw spawn plus
MF_DROPPED. This is controlled setup, not monster loot or runtime respawn.

Collection occurs only through the existing moving player's thing checks. Strict
square overlap precedes line checks; source vertical bounds are inclusive body
height/−8, and nonpositive toucher health refuses collection. The grounded policy
does not grant airborne item access. A successful pickup remains collected if a
later wall/solid actor normally blocks movement. It is immediately unlinked from
spatial queries, marked pending and retired by existing thinker traversal. Its old
handle never resolves to a later effect reusing that slot. A recoverable error
instead discards the complete transaction, including earlier grants/removals.

Ammo caps at200 bullets/50 shells. Full resources refuse without removing the
item. Zero-to-nonzero ammo selection uses current readyweapon: bullets select
owned chaingun or pistol from fist; shells select owned shotgun from fist/pistol.
GiveWeapon grants ammo before new ownership/pending selection. Already-owned
weapons can grant ammo; new ownership still collects with full ammo. Pending
selection uses existing source lowering/raising, never instantaneous replacement.
Health caps at100, synchronizes the same player/mo and preserves pain, attacker
and damagecount. Dead bodies cannot heal or collect. Medikit message selection
uses post-heal health; GOTMEDINEED is unreachable for positive admitted health.

Accepted grants assign a typed persistent source message, mark removal, add6 to
bonuscount, then request source item/weapon pickup sound without an actor origin.
Only the live bonuscount decrement fragment after the weapon stage is admitted;
no HUD message clearing or full PlayerThink/DeathThink is implied. The16-item
one-time budget bounds bonuscount at96; this is Rust admission policy, not a
claimed original source cap. Original P_RemoveMobj also writes an inactive item
respawn queue; those C-only PQ observations are retained, while queue storage and
respawning are outside this executable/diagnostic contract. Audio requests do not
implement playback or active-sound ownership.

Historical no-pickup policies and DSIM20–24 remain unchanged. DSIM25 separately
records pickup fields/events; decoding never restores a game. Armor, powers,
keys, backpacks, other items/weapons, player rebirth, multiplayer, generalized
specials and dynamic geometry remain excluded. Source evidence, explicit harness
adapters, generation/provenance safety and rollback are separate claims; see
[investigation](M3G9_REFERENCE_INVESTIGATION.md), [schema](STATE_FORMAT.md#dsim-v25-original-world-pickups)
and [observations](validation/M3G9.md).

## Maintenance compatibility boundary

Reference compression and shared weapon formulas change no supported gameplay
profile, actor admission, capacities, error contract or diagnostic schema. The
historical raw bytes remain in Git and are supplied exactly to existing test
parsers. Source arithmetic retains its qualified ordered-draw/BAM interpretation.
See [storage limits](EVIDENCE_STORAGE.md) and [formula scope](WEAPON_FORMULAS.md).
