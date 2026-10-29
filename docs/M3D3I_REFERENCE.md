# M3d3i source investigation: bounded original Demon map spawning

This investigation precedes runtime implementation. Behavioral authority is the
preserved Linux DOOM 1.10 tree at `a77dfb96cb91780ca334d0d4cfd86957558007e0`.
Inspected `p_mobj.c/h`, `p_setup.c`, `p_tick.c`, `p_enemy.c`, `p_inter.c`,
`p_map.c`, `p_maputl.c`, `p_local.h`, `info.c/h`, `doomstat.h`, `doomdef.h`,
`doomdata.h`, `r_main.c` and `m_random.c`, alongside the accepted M3d3a–h
source investigations and existing parser, spawn, combat and lifecycle services.

Verified starting master: `cae36bedc9de4f0ec14b571e1730fbc041ea5ba3`.
PR18 correction `f5d1bc830ed04fac1ab75b1bf52a6df26336e610` changed only the two
required documentation files and passed fresh local 242 workspace/242 MSRV/218
release tests, fifteen C/encoder pairs and fifteen debug/release examples.
[Correction CI40791592122](https://github.com/Copgunn/DOOM/actions/runs/40791592122)
passed all four jobs. The authorized regular merge has ordered parents
`f44828d5ad3169fe77c116bdebb30c74d1b5feb2` and the correction, tree
`588735801e1f72dbffc26e86f2c7be346c4edf1a`, equal to the correction tree.
PR18 is closed/merged; local and remote master agree. Exact merge
[push CI40792730511](https://github.com/Copgunn/DOOM/actions/runs/40792730511)
passed Linux, Windows, native macos-26-intel and Rust1.85, completed
2026-10-29 00:24:09 UTC before this branch or runtime work. Native Mac checkout
confirmed that merge SHA. All165 original files and3126 historical reference/
tool/support paths, history, attribution and GPL-2.0-only licensing are preserved.
The repository remains public. No private repository or commercial IWAD was
accessed. This investigation records a plan, not completed runtime evidence.

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

Ordinary source order is: single-player option16 skip; skill-bit skip; first
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

DoomEd3002 resolves to MT_SERGEANT/type12, not the Shotgun Guy. Source metadata:
health150, radius30, height56, mass400, speed10, reaction8, flags SOLID|SHOOTABLE|
COUNTKILL; spawn475, see477, melee485, pain488/chance180, ordinary death490,
xdeath0, missile0, raise496; see/attack/pain/death/active sounds41/52/26/64/77.

Reuse the single immutable `ActorRegistry::demon_stand_run_fixture` and all
accepted H state aliases. Source lookup/no_monsters uses original type12 metadata
**before** mapping to labelled native136. Native136 retains the admitted source
properties and exact state closure, but DoomEd-1, aliases1/3,2/15–21,4–14,
flags SOLID|SHOOTABLE and no raise/count-kill. Full original type12/13 execution,
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
sets reaction8 except Nightmare0, draws P_Random()%4 for lastlook, directly
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
359→0xe0000000 and360→0. No floating point. Option8 adds MF_AMBUSH32 after spawn.
The first Look occurs on a later state entry after the randomized1–10-tic timer;
subsequent source stand/run, nested wake, pursuit, melee, pain/death and corpse
behavior reuse accepted services. P_MobjThinker does XY then Z then states;
P_RunThinkers traverses insertion order, including same-pass appends.

## Proposed profile and atomic admission

Add separate profile16 and an empty constructor with existing ownership/options,
then explicit `spawn_map_things`. Retain all v1–v15 behavior and bytes unchanged.
Bound to256 input THINGS, four admitted Demons,64 actors, existing map±512/query
±1024/sight8192 and978-tic/1008-callback/994-other-actor coordinate margins.
Batch placement requires tic0 and a never-allocated store. Validate every record,
filter/type, metadata/state closure, BSP/floor geometry, coordinates and total
capacity before reservation or authoritative RNG/spatial/state mutation. Execute
the bounded plan through existing LifeHost spawn and storage only. No level clone,
second parser/store/spawn scheduler or rebuilt BLOCKMAP.

All native map actors precede unbound135 bodies/effects in thinker order. Direct
native spawn outside the batch rejects. Live native targets, active perception
roster and sector sound targets must reference admitted unbound135 bodies, never
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

Run an unadapted original spawn pass (type12/state475/COUNTKILL/source totals)
and compare initial fields/order/RNG under the explicitly declared type/state/
COUNTKILL projection. Separately run the admitted native136 gameplay pass through
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
