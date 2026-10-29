# Isolated Linux DOOM reference vectors

Run from the repository root on a supported GNU C host:

```sh
python3 tools/reference/generate.py
python3 tools/reference/generate.py --check
```

The first command intentionally regenerates vectors/tables and provenance; review
all changes. `--check` compares recorded results and generated Rust tables without
replacing host-specific provenance. Stable tests read committed vectors and need
no C compiler. Linux CI also reproduces them; Windows/macOS run Rust comparisons.

The script links the untouched `m_fixed.c` and `m_random.c`, extracts the original
point/box predicates, BLOCKMAP iterator, position/try-move, XY movement, thrust,
player-move, and thinker bodies, and builds a temporary standalone executable.
It never reads WAD assets or builds the full engine. Inputs/results are in
`tests/reference/`; the manifest records source hashes, compiler, flags, platform,
and adapter hashes. Original id Software source/table attribution and GPL-2.0-only
licensing apply to the derived fixtures and generated lookup tables.

The adapter retains thinker storage instead of freeing it before a link read.
Negative angleturn shifts use an explicit unsigned shift, defining a C-undefined
case according to the documented Rust wrapping policy. Pointer sentinel casts,
negative right shifts and out-of-range signed narrowing are GNU compiler-profile
observations, not portable C guarantees. FixedDiv inputs involving abs(INT_MIN)
are excluded; the direct floating helper excludes zero divisors and traps its
fatal range path through setjmp/longjmp. Rust extreme-value policies are tested
separately. No NaN-to-int conversion is invoked.

Movement uses the synthetic square's one static sector, no other actors, no
specials/portals and grounded non-player flags. Unsupported branches abort.
Thrust is compared separately from complete player thinking/sliding/state changes.
Candidate order and attempted-move counts are recorded, including the original
positive-only substep split and strict contact/stop thresholds. Thinker traces
record update order rather than allocator addresses or removed storage.

These are isolated reference comparisons, not full-engine synchronization,
complete gameplay verification, or DOS 1.9 compatibility. Project snapshot goldens
have separate provenance and must not be represented as C reference traces.


## M3a extension (separate provenance)

```sh
python3 tools/reference/generate_m3a.py --check
python3 tools/reference/m3a_snapshot_goldens.py --check
python tools/reference/check_headless.py
```

The new adapter imports the original extraction helper without modifying the M2
script, shim, provenance, vectors or tables. It extracts original actor link and
thing iterator/predicate, fixed DDA/intercept traversal, slide projection, point
angles, XY and input bodies. A static flat sector, zero/MF_SOLID flags, live
players with zero reaction time/no cheats, state 7, and aborting unsupported effect
stubs isolate the supported mechanics. ABI/shift/binary64 assertions and the same
GNU flags make assumptions explicit. Historical C sources remain unchanged.

The 252 rows cover 21 explicit setups and twelve tics each. Inputs, outputs,
ordered attempts, thing/line candidates and slide walls are recorded. Case 17
moves a blocker at tic 2, unlinks it at tic 3 and spawns a replacement at tic 4.
Case 20 reorders complete cell lists to test equal-fraction corner ties. Radius-sum
boundaries and widened Rust extreme policies have additional Rust tests; undefined
C position differences/abs(INT_MIN) are excluded from comparison. Enlarged adapter
intercept storage avoids unchecked C writes; cases stay within the original bound.

The v2 golden encoder independently applies explicit schema additions and clockwise
synthetic map records to Python encoding rules and C coasting values. Its separate
manifest hashes inputs, encoders and outputs. Neither golden encoder reads Rust
output. The final script compares actual headless stdout across debug/release on
all CI operating systems. No complete engine, DOS, commercial data or demo run is
claimed by these isolated fixtures.


## M3b extension (independent physical traces and DSIM-v3)

```sh
python3 tools/reference/generate_m3b.py --check
python3 tools/reference/m3b_snapshot_goldens.py --check
```

The separate generator creates twenty original synthetic classic maps, exact input
CSV and standalone C setup. It extracts source R_PointOnSide/R_PointInSubsector,
P_LineOpening, collision/slide/XY/Z/input bodies without modifying original files.
Twenty-nine setups run 24 tics each; 696 motion rows include constraints, all try
results, per-attempt candidate counts, chronological slide/attempt order and sector
heads. Additional outputs contain 261 probes, 1280 leaf lookups, 512 side predicates
and 20 openings. Input bytes, source/adapter/compiler/flags and output hashes are
recorded in `m3b_provenance.json`. `--check` reproduces without replacing host data.

The adapter calls XY then Z with P_MobjThinker's original guards, omitting state
machines. It executes source player-view fields internally but records physical
state; sound output is suppressed. Supported flags are zero, MF_SOLID, MF_DROPOFF
and MF_NOGRAVITY. Setup fixed coordinates use representable multiplication rather
than reproducing negative signed loader shifts. Undefined overflowing positions,
abs(INT_MIN), unsupported effects and full-engine/demo behavior are excluded.
Reference-list policy includes valid sparse diagonal and intentionally omitted-wall
fixtures; no spatial rebuilding occurs. C outputs are never generated from Rust.

The v3 encoder consumes exact C physical/attempt/order data, explicit schema/setup
and synthetic map bytes. It independently emits 96 digests and five full snapshots
for steps, transitions, falling and landing; source contact predicates determine
contact bits from recorded pre-Z values. A separate manifest hashes its inputs,
encoder and outputs. Both older encoders/vectors/provenance remain unchanged.
These are independent physical comparisons and project-state encodings, not
complete engine savegames or vanilla synchronization proof.

## M3c actor metadata and lifecycle

```sh
python3 tools/reference/generate_m3c.py --check
python3 tools/reference/m3c_snapshot_goldens.py --check
```

The first tool compiles preserved `info.c` and extracted state/spawn/map/thinker
and five action bodies using `m3c_driver.c`. It emits source tables plus 393 actor
and 444 event observations, including the independent 67-type/201-height spawn
sweep. Source/headers/adapters/outputs, compiler and flags are pinned in
`m3c_provenance.json`. All unsupported source actions abort; motion/player setup
and retained-free adapters are documented in `docs/M3C_REFERENCE.md`. Synthetic
probe actions are visibly distinct from source actions and imply no monster AI.

The second tool reads C rows and explicit synthetic setup/schema rules, never
Rust output. It produces 99 DSIM-v4 digests and 24 full byte streams, including
immutable original/native registries and ordered action events. Separate snapshot
provenance pins all inputs/outputs. Older generators and authoritative fixtures
remain unchanged. Rust tests/example consume recorded outputs without Python or C.

## M3d1

Run `generate_m3d1.py --check` and `m3d1_snapshot_goldens.py --check` in addition
to all earlier generators/encoders. `m3d1_shim.h` uses original structs/info and
extracted source bodies; unsupported services abort. 48 cases, nine public
synthetic maps and 431 captures cover damage/kill/hitscan/effects/death progressions.
The v5 encoder reads C rows and explicit source/fixture metadata, never Rust.
See [M3D1_REFERENCE.md](../../docs/M3D1_REFERENCE.md) for scope, exact comparison
counts, undefined-C exclusions and provenance. The old generators/fixtures stay
unchanged. `check_headless.py` now compares all five examples on every stable CI OS.

## M3d2

Run `generate_m3d2.py --check` and `m3d2_snapshot_goldens.py --check` alongside
all earlier tools. The separate shim uses original player/psprite/weapon, combat,
effect, noise and thinker bodies. 38 explicit command scenarios produce 4,890
captures and 422,943 player/actor field comparisons, with combined event order.
The v6 encoder reads only C/setup/schema data: 69 captures and 24 byte fixtures.
See [M3D2_REFERENCE.md](../../docs/M3D2_REFERENCE.md) for exact functions, adapters,
ABI/undefined-C exclusions, supported source paths and generation mapping.
`check_headless.py` compares all six asserted examples in debug/release on each OS.

## M3d2b

```sh
python3 tools/reference/generate_m3d2b.py --check
python3 tools/reference/m3d2b_snapshot_goldens.py --check
python3 tools/reference/check_projectiles.py
```

The new shim compiles original plasma spawn/check/explode, XY/Z, collision,
damage, state/thinker and RNG bodies plus real source metadata/headers. Its 41
synthetic scenarios compare 157,872 actor fields, 13,189 ordered observations,
1,194 moves, 6,229 candidates, 16 openings and 549 raw RNG draws. One scenario
uses a visibly synthetic wrapper to launch during an original thinker pass;
it does not establish monster AI. The omitted-coverage map retains the required
BLOCKMAP zero word and omits other walls. All unsupported paths abort rather
than silently returning a successful no-op.

The trace distinguishes A actor captures, E source lifecycle/raw RNG, P projectile
observations, D damage requests, C candidates, O openings, M move results, Z
vertical results and L source relinks. Original allocation ordinals are mapped
to generation-aware Rust handles; two 50-missile batches exercise reuse and wrap.
`m3d2b_provenance.json` pins source including p_enemy actions, every original
header, adapters, compiler/ABI, inputs and outputs. See
[M3D2B_REFERENCE.md](../../docs/M3D2B_REFERENCE.md) for complete source/admission/
undefined-C policies and actual-player-damage dependencies.

The independent v7 encoder reads C rows/public inputs/schema only, producing 129
captures and 35 full streams. It covers all setups, full wall/dropoff lifecycles,
settled lethal state and both batches' generations. It does not encode transient
damage event bytes; those semantic results have separate C comparisons. No Rust
output or private asset is read. The new seventh-example equality helper leaves
all historical generators/encoders/helpers unchanged.

## M3d3a

```sh
python3 tools/reference/generate_m3d3a.py --check
python3 tools/reference/m3d3a_snapshot_goldens.py --check
python3 tools/reference/check_monster_perception.py
```

The new isolated generator/shim extracts56 genuine original function bodies,
including all five sight functions, P_LookForPlayers, A_Look and both shared noise
functions. It does not substitute handwritten sight/scan decisions. Observational
hooks record exact source-order REJECT/BSP/seg, slopes, slot/health/FOV, sound,
target/state/RNG and thinker outcomes. Original structs, info, tables and fixed
helpers remain authoritative. Clearly labelled native states/types isolate real
A_Look from an action-free see handoff; unreachable chase/attack callbacks abort.

The public synthetic corpus has68 cases/18 maps, 1536 side vectors and 131 fraction
vectors. Hashed provenance records upstream source/header/function, adapters,
inputs/outputs, GCC flags and ABI, including the ordered spawn-jitter negative
shift adapter. Retained C allocation bytes permit observation; Rust generation
and empty-roster/unsafe-input policies are separately tested, never mislabeled
as C equivalence. See [M3D3A_REFERENCE.md](../../docs/M3D3A_REFERENCE.md) for exact
admission and [VALIDATION.md](../../docs/VALIDATION.md) for asserted counts.

The new independent v8 encoder reads C rows/public setup/schema only. It produces
420 captures and 45 full byte streams, including transient sight/scan/noise/wake
events and RNG wrap. No Rust output or proprietary asset is read. The eighth
example helper preserves both historical equality helpers; all seven older
reference generators, encoders and code fixtures remain byte-identical. Linux
CI reproduces all eight pairs; every stable OS compares recorded evidence.

## M3d3b

```sh
python3 tools/reference/generate_m3d3b.py --check
python3 tools/reference/m3d3b_snapshot_goldens.py --check
python3 tools/reference/check_monster_pursuit.py
```

The isolated generator/shim compiles 60 tracked genuine original bodies, including
P_Move/P_TryWalk/P_NewChaseDir/A_Chase and original perception, movement, structs,
metadata, states/thinkers and RNG. Original direction definitions are extracted.
Source bodies/functions, files/headers, tables, adapters and synthetic inputs/
outputs are hashed; compiler flags/ABI and the unsigned BAM shift adapter are
explicit. Native fixtures remove both attack states (including the source player
missile animation); attack/range/door services abort only when unreachable.

96 scenarios/17 legal synthetic maps compare exact direction/movement/candidate/
counter/target/state/sound/RNG and whole thinker order. No Rust expected output
is read. See [M3D3B_REFERENCE.md](../../docs/M3D3B_REFERENCE.md) and
[validation counts](../../docs/VALIDATION.md). Unsafe/nonterminating C cases are
separately labelled Rust safety policies. The independent v9 encoder produces 835
captures and 49 complete streams, including transient pursuit/perception/geometry
events. Named-field diagnostics precede bytes/digests. The ninth asserted example
and eighth fuzz target preserve every historical helper. All old v1-v8 reference
code/fixtures/encoders remain byte-identical; this README update is prose only.

## M3d3c

```sh
python3 tools/reference/generate_m3d3c.py --check
python3 tools/reference/m3d3c_snapshot_goldens.py --check
python3 tools/reference/check_monster_attacks.py
```

The new generator/shim compiles 64 tracked original bodies, adding genuine melee/
missile checks, FaceTarget and PosAttack to the earlier sight/Chase/trace/damage/
state/thinker closure. Original structs, info, tables and fixed helpers are used;
source/header/function/adapter/input/output hashes and GNU C flags/ABI are recorded.
Ordered multiplication defines historical signed shadow/spread/jitter shifts;
unsupported services abort only outside the admitted scenarios.158 cases use
four maps from 18 supplied public synthetic fixtures and isolate named source-player-body fixture roles and source
type probability branches without admitting complete original monsters.

The independent v10 encoder reads C rows and public schema/inputs only. It covers
1290 captures/72 full streams, transient decision/face/aim/combat events, raw source
RNG, real effects and generation reuse. Rust first-field/byte/digest checks and
178178 source actor-field comparisons are distinct from seven safety-policy tests.
The tenth asserted example and ninth bounded fuzz target preserve all older
helpers. Historical v1-v9 reference code and fixtures remain byte-identical;
this README is the sole intentionally updated historical reference path.
See [M3D3C_REFERENCE](../../docs/M3D3C_REFERENCE.md) and
[VALIDATION](../../docs/VALIDATION.md) for exact scope and observed gates.

## M3d3d

```sh
python3 tools/reference/generate_m3d3d.py --check
python3 tools/reference/m3d3d_snapshot_goldens.py --check
python3 tools/reference/check_shotgunner_attacks.py
```

The new generator/shim compiles 65 genuine original bodies including SPosAttack
with original structs/info/tables/fixed helpers. It preserves actual three-pellet
source control flow, interleaved impact/damage and both facing calls; explicit
ordered multiplication defines historical spread/shadow/jitter intent. Compiler,
GNU C flags/ABI and source/header/function/adapter/input/output hashes are recorded.
105 executed scenarios exercise all 18 supplied maps. Body inventory is distinct
from branch coverage; unsupported services abort only outside admitted paths.

The independent v11 encoder consumes C rows/public schema/inputs only, with
separate trace scopes per pellet, exact RNG boundaries and real generation reuse.
It produces 949 captures/92 full streams; 147,940 actor fields and 34,350 ordered C
observations are checked. The eleventh asserted example and tenth fuzz target
preserve every earlier helper. All v1-v10 reference code/fixtures/encoders remain
byte-identical; only this historical README's prose changes. C equivalence,
independent project bytes and nine Rust policy tests are separate evidence.
See [M3D3D_REFERENCE](../../docs/M3D3D_REFERENCE.md) and
[VALIDATION](../../docs/VALIDATION.md) for the ownership proof and observed gates.

## M3d3e

```sh
python3 tools/reference/generate_m3d3e.py --check
python3 tools/reference/m3d3e_snapshot_goldens.py --check
python3 tools/reference/check_chaingunner_attacks.py
```

The new generator/shim compiles genuine original CPosAttack/CPosRefire alongside
the 65 earlier bodies, retaining original structs/info/tables/fixed helpers and
actual short-circuit/nested Chase control flow. Instrumentation records state/
function actions, shotgun sound before face, independent aim, original spread/
damage/effects, refire RNG, reached sight and immediate movement/reacquisition.
Ordered multiplication defines historical shadow/spread/jitter intent. Source/
header/function/adapter/input/output hashes, GNU C flags, compiler and ABI are
recorded. The inventory of 67 bodies is not a branch-coverage claim.

153 gameplay scenarios exercise all 20 supplied maps, including two new REJECT
fixtures. Rust compares 268,606 actor fields, 63,308 ordered observations and
10,697 raw draws including setup. All refire branches and source effect retirement
occur. Original LUT values 39/41 bound the threshold; value 40 is absent. A separate
2,048-row controlled-dependency probe compiles the original refire body with injected
RNG, recording FaceTarget/state-entry doubles and scripted sight. It records call
order and is not original-LUT gameplay evidence; the main harness uses real services.

The independent v12 encoder consumes C rows/public inputs/schema only, with cached
immutable metadata and grouped source operations. It checks 1,544 captures/110 full
byte streams, real actor generations and event chronology. The twelfth asserted
example and eleventh bounded fuzz target retain all earlier helpers. Historical
v1-v11 reference code/fixtures/encoders remain byte-identical; only this historical
README's prose is updated. C behavior, independent DSIM and Rust policy tests remain
separate claims. See [M3D3E_REFERENCE](../../docs/M3D3E_REFERENCE.md) and
[VALIDATION](../../docs/VALIDATION.md).

## M3d3f

```sh
python3 tools/reference/generate_m3d3f.py --check
python3 tools/reference/m3d3f_snapshot_goldens.py --check
python3 tools/reference/check_demon_melee.py
```

The new harness compiles 68 genuine original bodies, including actual SargAttack,
FaceTarget, melee range, sight, Chase, damage/kill/state/thinker/fixed/RNG services.
Compiled inventory is not branch coverage; inherited hitscan bodies may be linked
but direct melee does not execute LineAttack or spawn its blood/puff. Provenance
records function/header/source/adapter/input/output hashes, GNU flags, compiler,
ABI assumptions and a C-derived evidence inventory. No expected result reads Rust.

The 104 main scenarios actually exercise all 20 supplied synthetic maps; generation
and Rust comparison assert that coverage. Raw fixed threshold edges, all ten
actual-LUT damage values, three shadow faces, selection sound, current moving/
occluded/dead/null target decisions, direct thrust/pain/death/retargeting and
repeated source pursuit/state/RNG are included. Explicit driver inputs supply
an extra strike during animation and remove/retire/body-spawn reuse; they do not
claim autonomous resurrection or direct-melee effects. Source fixed arithmetic
and left-before-right shadow draws retain the documented defined adapter intent.

The separate 36 radius vectors call original melee/sight on original structs/info,
with metadata radii 0/4/16/32, current collision radii 0/4/32 and raw threshold
neighbors-1/0/+1. Private Rust actor-copy tests compare those results without a
production radius setter. They are component evidence, excluded from the 104
main scenarios and independent canonical capture totals.

Rust explicitly checks 170,118 actor fields and 5,060 raw gameplay draws including
setup/discards;27,039 chronological C instrumentation rows feed the independent
ordered canonical observations/visits (not a one-row/one-event mapping). The
independent v13 encoder checks 1,309 captures/169 full streams, complete source
state/events and real generation reuse. The asserted thirteenth example ends at
`b6c312bf5fbcc7dc`; twelve prior examples and eleven historical Rust helpers are
unchanged. Source-equivalence, canonical-schema checks and Rust safety policies
remain distinct. Source findings/limits are in [M3D3F_REFERENCE](../../docs/M3D3F_REFERENCE.md).

## M3d3g Demon lifecycle tools

```sh
python3 tools/reference/generate_m3d3g.py --check
python3 tools/reference/m3d3g_snapshot_goldens.py --check
python3 tools/reference/check_demon_lifecycle.py
```

The new generator/shim compile68 genuine original bodies, including Damage/Kill,
Pain/Scream/Fall/SetMobjState/MobjThinker/RunThinkers and relevant collision/RNG,
with original structures/info/state tables. Explicit alias/native metadata setup
and inert sound observations are adapters; unsupported services abort. Provenance
records source/header/body/adapter hashes, exact outputs/inputs, compiler/ABI/flags
and established signed arithmetic/ordered-shadow assumptions. No Rust output is
used as expected source data. Original files and historical builders stay frozen.

331 source scenarios actually exercise all 21 maps and compare 517,192 actor fields,
36,316 actual raw draws (including setup/discards) and 103,942 ordered instrumentation
rows. These are not branch-coverage counts or one raw row per event. Three separate
controlled-dependency pain probes at179/180/181 (missing179/180 in the actual LUT)
are recorded in m3d3g_pain_boundary.csv, not gameplay/scenario/capture totals.

The independent encoder composes existing schema/event families from C and public
setup: 3,972 v14 captures/727 full bytes, with per-tic digests and first-field
comparison. The example checks nine C-backed/repeated-instance trajectories and
70-tic pain/death digest eecc4b4dcc4fbca5. Full Demon/Spectre/statistics/player/
raise/projectile gameplay remains outside the declared fixture. Canonical-state
consistency and Rust safety tests are distinct from original-C equivalence.

## M3d3h original Demon stand/run tools

```sh
python3 tools/reference/generate_m3d3h.py --check
python3 tools/reference/m3d3h_snapshot_goldens.py --check
python3 tools/reference/check_demon_stand_run.py
```

The generator compiles 68 genuine Linux 1.10 bodies, original structures/info/state
tables, collision/RNG helpers and instrumented action/state/thinker order. It
copies the original two stand/eight run states into declared aliases, rather
than approximating actions. Its390 scenarios actually exercise all 21 synthetic
maps (IDs0–20), including active run on each. Compare 935,012 live actor fields,
38,578 raw RNG draws including setup/discards and151,474 ordered instrumentation
rows. Inventory is not branch coverage; rows are not one row per event. Three
separate pain boundary injections179/180/181 retain explicit provenance and are
not counted as gameplay captures. C idle scenarios retain an active dead roster
slot; all-inactive/stale/unsupported extremes are labelled Rust policies.

`m3d3h_shim.h`, cases/maps/traces and provenance record exact inputs/outputs,
source/body/header/adapter hashes, compiler/ABI/flags and arithmetic assumptions.
Scripted loss/wake/noise/damage operations are declared fixture inputs; source
callbacks remain genuine. Original retired memory is retained for observation,
while Rust demonstrates real generation reuse. Unsupported C services abort.
No proprietary data or Rust-generated expected outputs are read.

The independent Python encoder reads C/public inputs/schema, copies original
metadata into the same documented aliases, and checks7,186 DSIM-v15 captures/
1,395 complete byte streams with per-tic digests/first-field diagnostics. Full
fixtures include idle/run loops, nested wake/lifecycle, target loss/reacquisition
and observed RNG-wrap boundaries. The asserted ten-scenario example checks the
same C fields/bytes and 130-tic digest `51463158173610c5` across repeated instances.
Keep all fourteen older pairs/examples, v1–v14 definitions and thirteen historical
Rust helpers byte-identical; this README's prose may evolve. See
[M3D3H_REFERENCE](../../docs/M3D3H_REFERENCE.md) and
[validation](../../docs/VALIDATION.md) for acceptance evidence and limits.

## M3d3i original THING spawn and actor admission

```sh
python3 tools/reference/generate_m3d3i.py --check
python3 tools/reference/m3d3i_snapshot_goldens.py --check
python3 tools/reference/check_demon_map_spawn.py
```

The70 compiled genuine bodies add P_LoadThings/P_SpawnMapThing to the accepted
spawn/link/BSP/thinker/RNG/action/combat closure. Original structs/info/state tables
are used. Commercial single-player THINGS bytes are supplied through bounded WAD
lump ownership adapters. Original algorithm branches are preserved; unsupported
player/special/drop/raise services abort. Negative signed shifts use representable
fixed multiplication; wrap flags and inherited ordered RNG assumptions are recorded.

Each of334 scripts first records an unadapted type12/state475/COUNTKILL spawn pass,
then independently runs adapted136 fixture gameplay. The explicit P_SpawnMobj
boundary translates12→136 after genuine lookup/nomonsters filtering. Raw totals
remain evidence only; no full scoring/actor claim. All53 supplied map fixtures,
covering21 geometry bases, are actually exercised. Inputs/cases, raw spawn and
native traces,123 isolated filter components and provenance record exact source/
body/header/adapter/output hashes, ABI and compiler flags.

Gameplay compares585,936 live actor fields,38,944 actual draws including setup/
discards and131,946 reconciled instrumentation rows. Original raw spawn separately
compares10,868 projected actor fields/37,437 draws and spawnpoints. Of123 isolated
filter components,73 compare admitted source behavior;50 demonstrate explicit
Rust rejection boundaries. Compiled inventory is not branch coverage, rows are
not one per event, and components are not gameplay/canonical captures.

Independent Python v16 encoding reads only original-C traces, public inputs and
schema:3,440 captures/784 full bytes, per-tic digests and first-field diagnostics.
It composes the unchanged H registry and existing spawnpoint representation.
The asserted eleven-scenario example ends at135 tics with `e327c3fbf88cb132`.
Preserve all fifteen older pairs/examples and fourteen historical Rust helpers.
See [source investigation](../../docs/M3D3I_REFERENCE.md),
[state schema](../../docs/STATE_FORMAT.md) and [validation](../../docs/VALIDATION.md).
