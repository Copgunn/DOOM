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

The public synthetic corpus has68 cases/18 maps,1536 side vectors and 131 fraction
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
