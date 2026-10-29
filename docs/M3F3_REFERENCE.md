# M3f3 source investigation: Demon gameplay on larger static maps

This investigation was committed before runtime implementation. Behavioral authority is
preserved Linux DOOM 1.10 at `a77dfb96cb91780ca334d0d4cfd86957558007e0`.
This document owns source findings and design assumptions. COMPATIBILITY will
own executable limits, STATE_FORMAT the schema, TESTING reproduction commands,
and validation/M3F3 measured evidence. DSIM is a diagnostic format.

## Inspected dependencies

Original p_setup.c, p_mobj.c/h, p_enemy.c, p_inter.c, p_tick.c, p_map.c,
p_maputl.c, p_sight.c, p_local.h, info.c/h, doomstat.h, r_main.c,
m_random.c and m_fixed.c establish the combined closure. Existing
M3D3J_REFERENCE and M3F2_REFERENCE document the two admitted foundations.
Rust lib/profile helpers, demon_map_context, demon_map_spawn, demon_lifecycle,
demon_stand_run, demon, monsters, perception, pursuit, spawning, combat,
geometry, movement, multisector, sight, map_scale, map_extent, snapshot and
store establish the present boundary.

Source functions relevant to composition are P_LoadThings, P_SpawnMapThing,
P_SpawnMobj, P_SetThingPosition/P_UnsetThingPosition, P_SetMobjState,
P_MobjThinker, P_RunThinkers, A_Look/P_LookForPlayers,
P_NoiseAlert/P_RecursiveSound, P_CheckSight and its BSP/subsector helpers,
A_Chase/P_Move/P_TryWalk/P_NewChaseDir, P_TryMove/P_CheckPosition,
A_FaceTarget/A_SargAttack, P_DamageMobj/P_KillMobj and A_Pain/A_Scream/A_Fall.

## Composition dependency matrix

| Dependency | Original behavior | Profile 17 | Profile 19 | M3f3 design, bounds and independent evidence |
| --- | --- | --- | --- | --- |
| Geometry | Source fixed coordinates and supplied BLOCKMAP | 128 lines/sectors, narrower arithmetic | 1024 lines/256 sectors, checked extent preparation | Reuse extent preparation/topology; admit geometry components within 2048 under the existing extent proof; retain sector heights within 512 and separately check line norm. Execute actual native gameplay beyond both old counts. No parser or world duplication. |
| Spawn | THING loop order; filter before allocation/RNG | Demon-only batch, native136 adaptation | No typed spawning | Preserve batch preflight, original lookup before adaptation, two draws, floor/BSP, angle, spawnpoint and totals. Compare genuine loader/spawner and separate unadapted type12 observations. |
| Raw state | P_SpawnMobj installs state without action | Correct raw state/countdown | Untyped terminal state | Preserve raw stand initialization; first Look at later entry. Compare state/timer/draw chronology. |
| State entry | P_SetMobjState actions immediate, nested transition allowed | SARG stand10/10, run eight times2, attack8/8/8 | No typed action closure | Reuse exact registry and state-entry service, aliases unchanged; no new scheduler or modern AI. Compare repeated-frame and nested action order. |
| Identity/order | Append thinkers, prepend sector/BLOCKMAP links | Four owner prefix then stable unbound suffix | Existing unbound arena and list ordering | Same store and scheduler; at most64 slots, four native owners. Preserve retirement/generation and source candidate order. Source identities are explicitly projected; Rust stale-handle policies separate. |
| Sight | REJECT then ordered BSP/seg traversal, whole-unit side tests | Infallible historical path, actor coordinates1024 | Fallible allocation/traversal with INT_MIN clipping rejection | All new-profile AI and direct queries must use checked sight. Propagate errors through bounded operation/tic plans before commit; no false visibility result for an unsupported continuation. Compare complete sight visit order. |
| Look/hearing | Sound target, ambush sight, four-slot scan, immediate see entry | Existing Look/scanning/noise services | Read-only sight only | Reuse roster/lastlook/targets and source sound-line order. All references and eventual sectors validated before operation. Compare sight and sound wake-up with immediate first Chase. Empty active roster remains explicit Rust termination policy. |
| Pursuit | Target delta, angle turn, ordered direction attempts, speed 10 and P_Random | Existing source movement/Chase | XY/Z only | Retain historical owner 978/pre-tic and target994 margins, read-only 1,024. Source clamp of 30 plus possible Chase of 10 stays within 1,024. Compare moves across many real sector boundaries and 24/25-unit steps. |
| Melee | Fresh source range and sight, Face, one damage draw and immediate damage | Existing shared current-body plans for sequential owners | Unsupported | Same late per-owner shared damage service; earlier owner health/RNG changes observed. No stale cached strike plan. Compare shared-target lethal/ignored strike order. |
| Pain/death | Damage before pain draw, pain180, delayed sounds, corpse initialization before Fall | Complete declared lifecycle | Unsupported | Preserve exact closure and native adaptations, ordinary death only. Direct damage may immediately enter Chase after failed pain and retargeting; it requires the same checked planning as a tic. |
| Statistics | Map COUNTKILL increments available kills; kill fallback credits player0 for unbound source | Distinct counters and source chronology | Absent | Reuse LevelStatistics; preflight signed addition. Target135 has no COUNTKILL, so native strikes cannot increment credit in the admitted suffix. Compare updates and per-capture counters separately. |
| Terminal corpse | Physics before state; disabled respawn returns before RNG/movecount | Persistent corpse and no terminal respawn RNG | Untyped physics | Retain corpse identity/spatial membership, delayed solidity clear and quarter height; no resurrection/Nightmare. Compare long post-death runs. |
| Restart | Level setup resets statistics but retains game RNG | Fresh instance constructor/restart | No context | Reuse setup/options; new instance keeps RNG, resets counters and explicitly reinstalls targets. Handles remain instance-scoped. |
| Canonical state | No DSIM equivalent | Explicit typed/statistics layout 17 | Explicit untyped layout 3 under tag 19 | DSIM-v20 explicitly composes layout 17 fields and larger map tables; no numeric-version inheritance, duplicate geometry or serialized preparation cache. Independent C-derived bytes. |

## Arithmetic intersection and restrictions

The first composition should increase structural capacity, not assume the entire
2048-unit unbound domain is safe for every monster action. Proposed map vertices,
node origins/deltas and BLOCKMAP origins use the proven extent envelope of 2,048 units;
sector heights stay within 512.
Extent's line L1 norm remains at most1536. Actor callbacks retain existing1024
coordinate/Z admission, owner 978 and unbound 994 pre-tic XY margins, momentum 64,
and source XY clamp of 30. Owner speed 10, radius 30 and height 56 come from the exact
source-derived registry; target 135 radius16/height 56/mass100 remain explicit
adaptations. Supported effects must retain their existing source closures.

The independent source prototype also executes pursuit, melee and a complete
pain/death/corpse sequence in a 256-sector map spanning -2048 to 2,048 while actor
positions remain inside the conservative gameplay bounds. This establishes a
coherent research input, not a completed Rust implementation.

Within this intersection, actor pairwise XY differences are at most 2,048 units;
approximate distance is at most 3,072 units and fits signed 16.16. Point-angle
integer setup, BSP/line sides, bounding-box expansions and source speed products
fit their recorded source representations. The stronger extent intercept proof
still applies: actor-to-line-start differences are at most 3,072 units and
line norm 1,536, below the existing proof for 4,096-unit query differences. Height/eye/opening differences remain far below signed overflow.
Knockback uses the existing checked damage limits/metadata and source ordering.
No floating-point distance/pathfinding or invented wide-integer clipping result
is admitted. Setup and action bounds are separate: larger tables do not increase
allowed actor counts or admit arbitrary sources, targets or player bindings.

Original FixedDiv's magnitude guard can yield a defined INT_MIN intercept even
at small coordinates, followed by undefined abs(INT_MIN) when used as a clipping
divisor. Narrow coordinates alone cannot remove this case. Preserve the exact
M3f2 guard in AI and readonly paths. Undefined continuations have no C boolean or
snapshot expectation; rejection and complete-state purity are Rust policies.

The existing source 64-cell path and 128-intercept restrictions remain unchanged.
The initial profile can reject direct hitscan/unsupported attacks rather than
implicitly exposing unproved expanded-map impact allocation and nested actions.
Unsupported modes, players, native infighting outside the accepted lethal/ignored
slice, respawn, raise, dynamic sectors, specials and full actor admission reject.

## Original signed spawn-angle boundary

A strict signed-overflow probe of the genuine P_SpawnMapThing body exposed a
separate spawn arithmetic dependency. ANG45 is the signed int literal 0x20000000;
ANG45*(mthing->angle/45) overflows at quotient 4 (positive 180 degrees), even though
the final angle_t destination is unsigned. The legacy GNU wrap contract defines
that behavior for older profiles; it is not well-defined portable signed C.

The new profile should admit only selected Demon THING angles whose truncated
quotient is in [-4, 3], equivalently encoded degrees [-224, 179]. The quotient -4
product is representable INT_MIN; negative 180 has the same intended BAM bits as
positive 180 without an overflowing product. Reject outside this interval before
any batch RNG, statistics or allocation. Filtered THINGS retain source filtering
precedence. Historical profiles and their full recorded angle cases stay frozen.

Source comparisons for the new profile must remain inside this interval. Record
the failing positive 180 source probe separately, alongside Rust-only rejection
tests. Do not cast the original multiplication to unsigned in the harness and
claim the original signed body passed. A future explicit angle-ABI admission may
be separately investigated; this milestone need not widen that dependency.

## Bounded operation and whole-tic planning

The existing global role/state/identity/roster checks remain prerequisites.
New fallible sight cannot be added after authoritative movement or RNG draws.
Plan only the bounded mutable projection: up to 64 actor/store records and list
links, up to 64 spatial handles, copied RNG and four counters, and new observations.
Borrow the immutable world, static sectors, registry, roster and sound references;
do not clone a Simulation, geometry, registry, canonical map cache or old events.

Run the existing ThinkerStore traversal and shared movement/LifeHost services
against that bounded plan in original thinker order. A single successful plan is
committed, avoiding replay with stale assumptions. A failed sight allocation,
clipping operation or later actor discards the plan and returns its structured
error. Commands, events/pending events, counters, actor identities and RNG remain
unchanged. Direct nested entry, Look/scanning, Chase, melee and damage need the
same boundary; direct damage can execute Chase immediately after retargeting.

An internal error latch, if used to unwind void source-style callbacks, must
always be checked before commit and must not manufacture a committed false sight
result. Stop later callbacks after a fault. Bound query/action/event work and
reserve scratch fallibly; document which allocation failures remain the inherited
process-level Rust policy. This is not a second AI engine or scheduler.

## Independent evidence design

Reuse immutable source extraction/instrumentation from the accepted map-context
harness, with a separately hashed adapter and enlarged synthetic table storage.
Compile genuine bodies and original structs/metadata; keep full level loading and
player initialization outside the claim. Explicitly record native136/state aliases,
unbound135 player-roster projection, setup statistics fragments, unsupported
aborting callbacks, compiler/ABI and signed-shift/angle assumptions. An inventory
of compiled bodies is not executed function/branch coverage.

Use public classic strips with 129/256 sectors and at least 129/769 lines, plus
1024-line inputs, sparse/reversed/duplicate lists, REJECT, floor and step variants.
Include long spawn/Look/Chase/movement/melee/pain/death/corpse scenarios, multiple
owners, skill/nomonsters/ambush, source identities, kill statistics and continuing
RNG. Compare per-tic actor fields, ordered candidates, sight visits, actions,
sounds, spatial/thinker lists, raw instrumented draws and RNG indexes. Report
actually exercised maps, not fixture inventory.

Generate DSIM-v20 independently from these C rows, public input bytes and the
explicit typed/statistics schema. Keep full bytes, per-tic digests and first-field
diagnostics. Generation-reuse, capacity, overflow, bounds, unsupported actions and
failure atomicity are separately labeled Rust safety policies. Historical
profiles, schemas, encoders, fixtures and helpers remain byte-identical.

## Implemented composition and additional source probes

The implementation follows the bounded actor-plan design above. Current exact
limits belong to [COMPATIBILITY](COMPATIBILITY.md#expanded-map-demon-profile);
actual comparisons and their exclusions belong to [measured evidence](validation/M3F3.md).
The source adapter retains the established explicit unsigned A_Chase direction
shift, representable negative spawn multiplication and left-before-right implicit
draw ordering under the recorded GNU ABI. Its strict audit is not a claim of
portable unmodified ISO C or exhaustive shift-undefined-behavior coverage.

Additional independent source inputs use non-cardinal AI rays so the original
horizontal-side quirk cannot skip the changed opening. They execute real slope
clipping and a closed opening in larger geometry. Another sparse shared-list
input places a native Demon in a real BLOCKMAP cell above the old 4,096-cell
ceiling. These extend the combined evidence without altering historical helpers
or the bounded fuzz catalog. Counts and per-case observations stay in the evidence
record, not this investigation.
