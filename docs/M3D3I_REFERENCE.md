# M3d3i source investigation: bounded original Demon map spawning

This investigation precedes runtime implementation. Behavioral authority is the
preserved Linux DOOM 1.10 tree at `a77dfb96cb91780ca334d0d4cfd86957558007e0`.
Inspected `p_mobj.c/h`, `p_setup.c`, `p_tick.c`, `p_enemy.c`, `p_inter.c`,
`p_map.c`, `p_maputl.c`, `p_local.h`, `info.c/h`, `doomstat.h`, `doomdef.h`,
`doomdata.h`, `r_main.c` and `m_random.c`, alongside the accepted M3d3a–h
source investigations and existing parser, spawn, combat and lifecycle services.

Historical delivery accounts are retained in the [complete original investigation](validation/archive/pre-m3d3j-m3d3i_reference.md); the source findings below retain their technical scope.

## Original input and filter chronology

Classic THINGS are ten little-endian bytes: five signed 16-bit fields x, y,
angle, type and options. `doom-map` already decodes them; its options is `u16`
to preserve every option bit. Reuse that validated parser and its record order.
`P_LoadThings` obtains the lump, divides its byte length by `sizeof(mapthing_t)`,
iterates without sorting, converts each field using SHORT, calls P_SpawnMapThing
and releases the lump. This profile requires an already validated whole-record
lump rather than accepting a truncated source remainder.

The existing single-player ActorOptions explicitly denotes DOOM II/commercial.
In other source game modes P_LoadThings encounters one of 68/64/88/89/69/67/71/
65/66/84 and **breaks the entire loop**, even before ordinary placement filters.
Noncommercial loading is excluded; do not silently implement it as continue.

P_SpawnMapThing first handles deathmatch start11 and player types <=4, before
ordinary filters. Those starts are excluded here, not converted to generic
actors. Type0/negative enters unsafe C player-start indexing and must reject as
a Rust input policy, never an original-C equivalence scenario.

Ordinary source order is: single-player option 16 skip; skill-bit skip; first
matching mobjinfo doomednum lookup (unknown is fatal I_Error); deathmatch
MF_NOTDMATCH skip; nomonsters skip for MT_SKULL or MF_COUNTKILL; then spawn.
Easy flag1 covers Baby and Easy, normal2 covers Medium, hard4 covers Hard and
Nightmare. Runtime admits the first four source skill values. Nightmare filter
components may be checked independently, but fast/Nightmare execution rejects.
Cooperative/deathmatch modes reject; no player/netgame implementation is implied.
Filtered ordinary unknown/unsupported types may skip exactly where source does;
enabled unknown types return a structured recoverable error, enabled recognized
non-Demon types reject admission. Original source fatality versus atomic Rust
rejection must remain distinct evidence families.

## Type resolution and explicit executable adaptation

DoomEd3002 resolves to MT_SERGEANT/type 12, not the Shotgun Guy. Source metadata:
health150, radius30, height56, mass400, speed10, reaction 8, flags SOLID|SHOOTABLE|
COUNTKILL; spawn475, see477, melee485, pain488/chance 180, ordinary death490,
xdeath0, missile0, raise496; see/attack/pain/death/active sounds 41/52/26/64/77.

Reuse the single immutable `ActorRegistry::demon_stand_run_fixture` and all
accepted H state aliases. Source lookup/no_monsters uses original type 12 metadata
**before** mapping to labelled native 136. Native136 retains the admitted source
properties and exact state closure, but DoomEd-1, aliases1/3,2/15–21,4–14,
flags SOLID|SHOOTABLE and no raise/count-kill. Full original type 12/13 execution,
scoring/global totals and resurrection remain excluded. A map-origin fixture is
not proof of complete original actor admission. Do not duplicate actor metadata.

## Spawn initialization and observable order

Map x/y become fixed point by exact signed multiplication by65536. Preserve
original BSP/subsector selection, sector floor and ceiling; ONFLOORZ places Z on
the selected sector floor, including nonzero floors. No nearest-sector heuristic
or unconditional Z0. Source P_SetThingPosition inserts each actor at sector and
BLOCKMAP cell heads; P_AddThinker appends at thinker tail. Thus later THINGS may
be newer spatial heads but later thinkers. Keep identities independent of both.

P_SpawnMobj zero-initializes the actor, copies metadata/flags/health/dimensions,
sets reaction 8 except Nightmare0, draws P_Random()%4 for lastlook, directly
installs spawn state/sprite/frame/tics, links spatially, initializes floor/ceiling/
Z and appends its thinker. **It does not call P_SetMobjState or A_Look.** Source
zero movedir is EAST and movecount0; target/momentum are empty/zero.

P_SpawnMapThing then copies spawnpoint, randomizes positive timer to
1+(P_Random()%tics), updates source totals, converts angle, then adds AMBUSH.
Each admitted normal Demon consumes exactly two spawn draws; filtered records
consume none. P_Random increments before reading the original256-entry table and
wraps. Retain order between different THINGS and any later setup actors.

Angle is ANG45*(signed short angle/45), with signed integer division truncating
toward zero and unsigned32 wrapping multiplication. Thus -1→0, -45→0xe0000000,
359→0xe0000000 and 360→0. No floating point. Option8 adds MF_AMBUSH32 after spawn.
The first Look occurs on a later state entry after the randomized1–10-tic timer;
subsequent source stand/run, nested wake, pursuit, melee, pain/death and corpse
behavior reuse accepted services. P_MobjThinker does XY then Z then states;
P_RunThinkers traverses insertion order, including same-pass appends.

## Proposed profile and atomic admission

Add separate profile 16 and an empty constructor with existing ownership/options,
then explicit `spawn_map_things`. Retain all v1–v15 behavior and bytes unchanged.
Bound to256 input THINGS, four admitted Demons,64 actors, existing map±512/query
±1024/sight8192 and 978-tic/1008-callback/994-other-actor coordinate margins.
Batch placement requires tic 0 and a never-allocated store. Validate every record,
filter/type, metadata/state closure, BSP/floor geometry, coordinates and total
capacity before reservation or authoritative RNG/spatial/state mutation. Execute
the bounded plan through existing LifeHost spawn and storage only. No level clone,
second parser/store/spawn scheduler or rebuilt BLOCKMAP.

All native map actors precede unbound 135 bodies/effects in thinker order. Direct
native spawn outside the batch rejects. Live native targets, active perception
roster and sector sound targets must reference admitted unbound 135 bodies, never
another Demon or removable effect. Incoming native damage accepts supported
unbound bodies/environment/self; cross-native infighting is excluded. Corpse old
generations may remain unresolved as in G/H without redirecting reused slots.

Multiple owners expose sequential RNG/health changes. A global bounded preflight
must prove roles, generations/bindings, closures, coordinate/counter/arithmetic
bounds, zero melee allocation and stable target lifetime **before the tic**.
At actual A_SargAttack entry prepare only a small current owner/target/RNG damage
plan through shared DamagePlan, then perform original Face/range/draw/damage.
Do not reuse a before-tic plan after an earlier owner has consumed RNG or hurt
the shared target. Bodies run after native owners, persist through their admitted
non-removing lifecycle, and cannot introduce player/drop/scoring branches. Bound
initial body XY momentum to64 units; at most four40-damage strikes add at most
80 units even with source fall-forward before existing physical clamping. No
per-tic world clone, duplicate damage engine or new AI scheduler is permitted.

Every recoverable setup/direct/tic failure must preserve full canonical state.
Test malformed later target state/metadata/binding/reference and capacity so an
earlier owner cannot partly move, draw RNG or emit events before rejection.
These conservative admission/lifetime/atomicity rules are Rust safety policies,
not original limits or complete monster infighting equivalence.

## Independent evidence plan

Compile genuine P_LoadThings/P_SpawnMapThing/P_SpawnMobj/P_SetThingPosition,
BSP/thinker/RNG and existing action dependencies with original structs, state and
actor metadata. Record original/body/adapter hashes, compiler flags, ABI and exact
inputs. Keep source bodies unchanged except explicit instrumentation/wrapping.

Run an unadapted original spawn pass (type 12/state 475/COUNTKILL/source totals)
and compare initial fields/order/RNG under the explicitly declared type/state/
COUNTKILL projection. Separately run the admitted native 136 gameplay pass through
an explicit P_SpawnMobj type-translation boundary after original type lookup and
filtering. Do not claim that adapted source totals establish complete original
kill accounting. Unsupported callback services abort rather than silently no-op.
Source-fatal unknown types, unsupported modes and undefined negative player-index
inputs remain distinct from executable source scenarios and Rust-only policies.

Exercise one/multiple/filtered THINGS, all admitted skill groups, no_monsters,
ambush, negative coordinates, nonzero/multiple floors/BSP, signed unusual angles,
state/timer/lastlook RNG wrap, thinker versus spatial ordering, delayed Look,
pursuit/melee/pain/death, simultaneous shared-target strikes and generation reuse.
Compare fields, ordered actions/observations, actual RNG values and spatial/
thinker order. Count actual exercised maps separately from fixture inventory.

Independently encode DSIM-v16 from C traces and documented schema, never Rust
output. Existing map THINGS, actor spawnpoint, state/type/flags/health/position/
RNG/order already represent placement; avoid redundant persistent spawn fields.
Provide full bytes, per-tic digests and first-field diagnostics. DSIM is a project
diagnostic format, not an original DOOM savegame or demo. Keep original-C behavior,
independent canonical evidence and Rust safety-policy tests separate.

Unsupported: full map loading/all THING types/player starts, noncommercial map
loader behavior, multiplayer/deathmatch, full Demon/Spectre/monster interactions,
actual player damage, global scoring/counts, fast/Nightmare execution, resurrection,
dynamic sectors/specials, demos, rendering, scripting and new asset formats.

## Implemented slice and observed focused evidence

Investigation `ab755838` precedes batch admission `8cc0192` and runtime/preflight
integration `8084cf1`. The accepted H registry is reused exactly. Batch execution
uses existing validated parser, immutable geometry, storage/spatial linking and
LifeHost initialization; no original upstream file changed. The profile fixes
commercial single-player context independently of the map marker.

Global preflight and late shared planning have focused private fault tests for
later unsupported death callbacks, bound actors, invalid states/counters/targets,
unsafe body momentum, owner suffix and removable/pending targets. They reject
before the earlier owner's action/RNG/movement. All earlier profiles retain their
own planning path. The maximum four body-directed native attacks may share a
body; the C shared-target case proves earlier kill then later draw/ignored damage.

Genuine C70-body inventory:334 scripts, each with unadapted original spawn and
separate admitted native gameplay;53 actually exercised synthetic maps spanning
all 21 geometry bases. Admitted field comparisons 585,936, raw draws 38,944 including
setup/discards, ordered rows 131,946. Original spawn fields 10,868 under explicit
12→136/475→1/COUNTKILL projection, draws 37,437 including discards. Source raw
COUNTKILL totals are observed but excluded from Rust gameplay compatibility.
Isolated source filters123 are separate:73 admitted source comparisons and 50
Rust exclusions/fatal-error policies, including Nightmare and unsupported modes.

Independent v16 evidence is3,440 captures/784 complete bytes with per-tic digests/
first-field diagnostics. No expected values come from Rust. The eleven-scenario
headless example matches source and repeated instances; its135-tic lifecycle digest
is `e327c3fbf88cb132`. Source pain decision45 sounds 46, returns Chase 48; lethal 75
has delayed Scream 81/Fall 93/terminal 101 and remains a corpse through 135.

The shared-target example initially exposed a coverage gap: its first synthetic
target position made both attacks miss. Before committing evidence, the public
input was corrected to the origin and C/independent fixtures regenerated. The
asserted example now requires actual ignored damage after the earlier kill.
Runtime behavior did not need a workaround. Mandatory/regression/sanitizer/CI
acceptance is recorded in [VALIDATION](VALIDATION.md) after actual observation.

## Observed acceptance evidence

Fresh mandatory validation passed 261 workspace/261 Rust 1.85/237 release tests,
all nineteen core commands, all sixteen C/independent-encoder pairs and all
sixteen asserted debug/release examples. All fifteen ASan targets built; the
map-spawn campaign completed 4,096 executions in 983.6 seconds without findings,
with leak detection disabled. Documentation links and original 165/historical 3126
path/ancestry/license/lock preservation passed. Full details and exact limits are
in [VALIDATION](VALIDATION.md).

Implementation `ca6162fcdebfca367599b7e115023ef38eb1cf69` passed Linux, Windows,
native macos-26-intel and Rust 1.85 in
[CI 40795637820](https://github.com/Copgunn/DOOM/actions/runs/40795637820). The verified
PR testmerge has the same feature tree. Final correction-head readiness is gated
on fresh local/four-job checks and recorded in
[PR19](https://github.com/Copgunn/DOOM/pull/19); it remains unmerged without
auto-merge. This is owner review of a coherent bounded fixture foundation.
Unprojected COUNTKILL/statistics, complete original actor admission and the other
listed source dependencies remain explicitly unsupported.
