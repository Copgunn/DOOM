# Testing and fixture guide

Run the required commands in [README](../README.md#build-and-test). Stable CI
covers Linux, Windows, macOS, and Rust 1.85.0, denying Clippy/rustdoc warnings.
It needs no proprietary IWAD, C engine build, nightly toolchain, or fuzz package.

## Coverage and fixtures

- `doom-types` unit tests preserve raw eight-byte names, including non-text bytes.
- `doom-wad/tests/archive.rs` covers both archive kinds, empty archives, directory
  bounds/signed fields, exact and mixed-case names, duplicate/patch precedence,
  zero markers, shared/overlapping ranges, resource limits, mutations/truncations.
- `doom-map/tests/parsing.rs` decodes a complete square map and tests every fixed
  record width, required geometry, references/sentinels, BSP cycles and 20,000-node
  depth, subsectors, REJECT, BLOCKMAP, map grouping/overrides, unsupported formats,
  all policy limits, and structured mutations with valid archive containers.
- Adversarial sharing tests cover 20,000 cells sharing a 32,000-word list and
  20,000 overlapping subsectors over 32,767 segs, without timing-sensitive asserts.
- Unit boundary tests exercise allocation/range overflow before any allocation.
- `tests/support/mod.rs` creates original synthetic bytes with explicit little-endian
  fields. `cargo run -p doom-map --example parse_map --locked` exercises the public
  API on a committed synthetic seed. The WAD API also has an executable doctest.

Assertions check actual decoded values and error categories. Mutation/truncation
loops call parser entry points directly: a panic fails the test. Allocation-failure
paths cannot be forced portably without allocator injection; bounds/overflow/limit
checks are tested. Structural validity does not prove a BSP's geometric accuracy,
BLOCKMAP spatial completeness, gameplay compatibility, or exhaustive error coverage.

Put narrow tests next to implementation and public-API regressions under each
crate's `tests/`. Keep fixture builders deterministic, small, and independent of
OS ordering. Static fixtures need provenance, license, purpose, and regeneration
instructions. Never use commercial or extracted proprietary data. Promote
minimized fuzz failures into ordinary stable regressions.

## M2 reference and deterministic checks

```sh
cargo test -p doom-types -p doom-sim --locked
cargo test -p doom-types -p doom-sim --release --locked
cargo run -p doom-sim --example headless --locked -- 12
python3 tools/reference/generate.py --check
python3 tools/reference/snapshot_goldens.py --check
```

The Python commands need a supported C compiler for isolated vector regeneration;
ordinary Rust tests read committed vectors and need no C toolchain. Linux CI
reproduces vectors/goldens; all three stable platforms run recorded comparisons
and debug/release deterministic checks. Rust 1.85 runs the workspace suite.

`doom-types/tests/reference.rs` checks 1,303 arithmetic cases, 512 C RNG lookups,
safe extreme policies and reset/wrap/separation. `doom-sim/tests/reference.rs`
checks 162 geometry predicates, two 96-step movement layouts (including different
cell-list ordering), 112 command-thrust cases and thinker update-order traces.
Reference compiler flags, source/adapter hashes and exclusions are committed.
See [tools/reference/README.md](../tools/reference/README.md) and
[source findings](M2_REFERENCE.md); these comparisons do not run the whole engine.

`snapshot_goldens.py` reads no Rust output: it encodes explicit setup state and
C-derived coasting values into DSIM-v1 bytes. Tests compare the complete first
snapshot and twelve per-tic digests plus decoded position/momentum fields.
`doom-sim/tests/determinism.rs` also checks exact phase/actor order, pause, atomic
input rejection, unsupported profiles, capacity failures without RNG drift, stale
handles/reuse, and 768-call command/lifecycle streams. Fresh worlds and different
presentation read schedules must match all fields/bytes, not merely counts/hashes.
An injected extra RNG draw must report the first divergent tic and named field.
Store tests cover callback append/removal, empty/clear, slot reuse, generation
exhaustion and visit budgets. [STATE_FORMAT.md](STATE_FORMAT.md) defines widths,
field order, pending observations and the noncryptographic digest.

Golden regeneration is intentional: run the generators without `--check`, inspect
the complete diff, retain provenance, and explain changed compatibility/schema
decisions. Never replace expected values with current Rust output to hide a failure.
Long-run project consistency is distinguished from C-derived reference vectors.

## Parser fuzzing

`fuzz/` is an independent Cargo workspace, with its own committed lockfile and
`libfuzzer-sys` dependency. It does not affect the dependency-free stable workspace.
Install a supported host C++ compiler, then from repository root:

```sh
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --version 0.13.2 --locked
cargo run -p doom-map --example generate_fuzz_seeds --locked
cargo +nightly fuzz build
cargo +nightly fuzz run wad -- -runs=100000 -seed=1 -max_len=1048576 -rss_limit_mb=1024 -timeout=10
cargo +nightly fuzz run map -- -runs=100000 -seed=1 -max_len=1048576 -rss_limit_mb=1024 -timeout=10
```

Use a clean checkout/copy of the committed seeds for a reproducible initial corpus;
fuzzing adds ignored generated corpus entries. Seed provenance is in
[fuzz/corpus/README.md](../fuzz/corpus/README.md). Targets limit archive input to
1 MiB/4096 entries; the map target further caps records, cells, words, and allocation.
It tries MAP01 and E1M1. This bounded smoke campaign is not an exhaustive run or a
coverage guarantee; schedule longer campaigns separately when requested.

In managed environments where LeakSanitizer cannot inspect threads because of
ptrace restrictions, prefix the two run commands with `ASAN_OPTIONS=detect_leaks=0`.
AddressSanitizer remains enabled; leak detection is then **not verified**. Do not
change host ptrace/security settings to run tests. Record the exact tool versions,
commands, corpus, run count, and any sanitizer limitations in validation notes.
Public CI runs stable mutation/adversarial smoke tests and the synthetic API
example; it deliberately does not install nightly or run cargo-fuzz.

## Private assets

Private signature/archive/map tests are ignored by default. Set `DOOM_IWAD_DIR`
only in a private shell after separate authorization for the destination, then:

```sh
cargo test -p doom-map --test private_iwad -- --ignored
```

The test opens external assets read-only, bounds file input, validates archives
and recognized maps, and suppresses private names, paths, and contents in failures.
Do not print or upload private results, links, assets, snapshots, or credentials.
Storage access alone does not authorize cloud copying. Preserve source originals.
A successful synthetic exercise of this test is not evidence of real-IWAD testing.

## Future verification

Extend M2 per-tic state/command regressions as gameplay grows, preserving
repeatability across profiles/platforms and presentation schedules. M3/M4 add
pinned full reference builds and version-specific demo
replays, recording legal data hashes and first divergent tic/field. Performance
checks must state environment, profile, workload, and a correctness baseline.


## M3a player/collision checks

```sh
cargo test -p doom-sim --test m3a --test m3a_reference --locked
python3 tools/reference/generate_m3a.py --check
python3 tools/reference/m3a_snapshot_goldens.py --check
cargo run -p doom-sim --example player_arena --locked
python tools/reference/check_headless.py
```

The M3a C adapter compares 21 setups over 12 tics (252 total), including axis and
angled walls, negative momentum, endpoints/corners, equal-fraction ties under
reordered lists, player/non-player response, active-command stop thresholds,
solid/non-solid candidates, same-cell reinsertion, immediate spatial removal and
replacement spawning. Tests compare initial/result XY and momentum, binary angle,
every attempted destination/result, line/actor candidate order and slide lines.
Failures identify the first case/tic/observable field. Compiler/source/adapter
hashes and unsafe exclusions are in `m3a_provenance.json`; existing M2 evidence is
unchanged. C regeneration is Linux-only; Rust recorded comparisons run everywhere.

Independent DSIM-v2 first-tic bytes and twelve coasting goldens have separate
encoder/input/output hashes. M2 v1 expectations remain unchanged. New 768-call
traces (767 advancing plus pause) include wall and actor blocks, slides, lifecycle
reuse, RNG wrapping and presentation reads. An extra RNG draw must identify tic
400 and `rng.gameplay`. Every stable OS runs these in debug/release, both schema
goldens, and byte-identical debug/release stdout checks for both headless examples.

Bounded generated tests use nine extreme-coordinate setups with five extreme
commands and 128 four-actor setups over twelve tics. They test safe raw i32
policies and reproducibility, not undefined-C equivalence. Additional regressions
cover strict radius-sum contact, square versus circular overlap, negative cells,
off-map centers, actor-order ties, removal/reuse/stale handles, duplicate lists,
missing coverage, degenerate/overflow/over-limit geometry, failed setup/step
atomicity, both profiles' tic exhaustion, and shared 60,000-word lists across
4096 cells. The simulation has no side-effecting collision callbacks; generic
thinker removal-during-traversal remains covered by the existing store suite.

The headless bevel arena uses original synthetic data and fixed commands. Its
asserted trace matches C case 19 and records four solid-actor blocks, four wall
blocks and four angled-wall projections. This establishes the scoped mechanics,
not a playable port or complete P_PlayerThink. No private IWAD is required.


## M3b static multi-sector and vertical checks

```sh
cargo test -p doom-sim --test m3b --test m3b_reference --locked
python3 tools/reference/generate_m3b.py --check
python3 tools/reference/m3b_snapshot_goldens.py --check
cargo run -p doom-sim --example multisector_arena --locked
python tools/reference/check_headless.py
```

The independent generator emits twenty synthetic classic maps and 29 actor/input
setups. Its preserved C bodies produce 696 XY/Z tics, 261 destination probes,
1280 BSP lookups, 512 original side predicates and 20 two-sided opening results.
Tests compare complete physical fields and every attempted target/constraint/
allow decision, per-attempt candidate counts, ordered line/thing/opening lists,
slide choices, chronological attempt/slide order and sector-head membership.
First failures identify case/tic/observable field. Fixture/provenance generation
never reads Rust output or changes historical C. Zero-node, axis reversals,
diagonals, boundary ties, sparse lists and deliberately missing walls are explicit.

New independent v3 encoding checks 96 digests and five complete snapshots, with
named-field comparisons across step-up, sector transition, fall, drop and landing.
V1/v2 goldens stay unchanged. All stable OS jobs run debug/release vectors and
all three schemas; Linux additionally regenerates C/independent goldens. All three
headless examples must have byte-identical debug/release stdout on each OS.

Long 768-call streams visit three sectors and exercise landing, step/drop physics,
solid blockers, removal/reuse, pause, RNG wrapping and presentation read schedules.
Injected RNG drift reports tic 400 / `rng.gameplay`. Different reserved capacities
produce identical complete state; differing retired-slot/RNG/tic histories test
normalized physical trajectories rather than falsely equating canonical metadata.

Adversarial checks cover raw i32 XY/Z/momz extremes (7 setups × 12 tics), 128
three-actor generated setups × 12 tics, 4096-node iterative traversal and 4097-node
rejection, cyclic/invalid-sector input, inverted/extreme heights, inconsistent flags,
zero/low openings, strict raw thresholds, sector-head ordering, stale reuse,
read-only queries and full-state error atomicity. Raw-overflow cases test safe Rust
policies, not undefined C equivalence. Parser fuzz targets still compile with
nightly AddressSanitizer instrumentation; no new parser fuzz campaign or leak-
detection result is claimed for M3b. Physics uses these bounded property-style
regressions rather than a new libFuzzer target.

## M3c actor states and spawning

```sh
python3 tools/reference/generate_m3c.py --check
python3 tools/reference/m3c_snapshot_goldens.py --check
cargo test -p doom-sim --test m3c --test m3c_reference --locked
cargo run -p doom-sim --example actor_states --locked
python tools/reference/check_headless.py
cargo +nightly fuzz build
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run actor_states -- -runs=10000 -max_len=24
```

Keep all six older generator/encoder commands. C metadata comparisons cover all
source properties; traces compare countdown/state/actions/health/flags/targets,
RNG and thinker/lifecycle order. The sweep checks 67 types at three spawn heights.
Independent v4 fixtures compare 99 digests and 24 full byte streams; failures name
the first mismatching field. The asserted 80-tic example finishes with digest
`3dcbad5257374b3e`. All four example stdout streams must match debug/release.

Policy regressions cover every type state/sound reference, bad action indexes,
zero/callback/spawn cycles, long entry chains, aggregate tic work, exhausted
capacity, whole-batch placement atomicity, pause, unsupported modes/types, immediate
unlinking, targets retained across retirement/reuse, self/mutual relationships,
XY-before-Z-before-state order, terminal persistence and null timer preservation.
256 bounded generated graphs exercise deterministic accepted/error behavior;
a separate actor_states fuzz target mutates native graphs with no private data.
Sanitizer run outcomes and environment limits are recorded in VALIDATION.md.

## M3d1 combat evidence and regressions

```sh
python3 tools/reference/generate_m3d1.py --check
python3 tools/reference/m3d1_snapshot_goldens.py --check
cargo test -p doom-sim --test m3d1 --test m3d1_reference --test m3d1_snapshots --locked
cargo run -p doom-sim --example combat_arena --locked
python3 tools/reference/check_headless.py
cargo +nightly fuzz build
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run combat -- -runs=10000 -max_len=256
```

Run all earlier generators/encoders too. Linux CI regenerates all five source
profiles/encoders; each stable OS checks recorded comparisons, independent v1-v5
goldens and five-example debug/release equality. Rust 1.85 checks the workspace.
No proprietary data or graphics device is required.

The separate `tests/support/m3d1.rs` uses nine original synthetic maps and explicit
fixture metadata. The historical shared builder and old fixtures are preserved.
`m3d1_reference` checks 48 C scenarios and 431 captures: 38,052 live actor fields,
RNG indexes, source candidate/intercept/fraction/traversal order, hit/aim decisions,
and ordered lifecycle/action observations across full attack/death sequences.
Cases include opening/wall/height/range/tie/negative/diagonal/list-order boundaries,
forward-fall parity, damage zero, pain 0/256, strict gib thresholds, corpse edges,
threshold target lock and 100 impacts wrapping RNG followed by 35 thinker tics.
Retired C storage is retained only for observation; Rust compares unavailability.

`m3d1_snapshots` compares 63 independently encoded captures and 17 full byte states,
with first-field diagnostics. Selected shot API states include the complete trace
and ordered effect/damage events; final states include settled ordinary/extreme
corpses and effect retirement. `combat_arena` asserts successful/wall-blocked shots,
pain, death/actions and final C-derived digest `8906eb60707d089e` after 80 tics.
The 12 separate policy tests cover atomic errors, stale/pending/reused handles,
spatial tie order after reuse, invalid metadata/flags/impact changes, player/drop/
sky/clearance exclusions, capacity, recursive pain removal/spawning, extreme
coordinates, intercept overflow, zero-damage impacts and a repeated 160-tic stream.
Those policy checks are project evidence, not new original-C equivalence claims.

The combat fuzz target accepts at most 32 operations on eight slots, varying
handles, damage, angles/range/slope and step timing; recoverable errors must leave
all snapshot fields unchanged. ASan with `detect_leaks=0` does not verify leaks.
Record the observed run count/toolchain and report any crash honestly. Fuzz
corpora/artifacts/builds stay ignored; minimized failures become stable tests.

## M3d2 player weapons

```sh
python3 tools/reference/generate_m3d2.py --check
python3 tools/reference/m3d2_snapshot_goldens.py --check
cargo test -p doom-sim --test m3d2 --test m3d2_reference --test m3d2_snapshots --locked
cargo run -p doom-sim --example player_weapons --locked
python3 tools/reference/check_headless.py
cargo +nightly fuzz build
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_weapons -- -runs=10000 -max_len=129
```

Run all earlier generators/encoders and required stable/release/MSRV checks too.
Linux CI reproduces six reference profiles and encoders. All stable CI platforms
run recorded comparisons, independent v1-v6 goldens and six-example stdout equality.
Public tests require no proprietary data, device or C compiler; reproduction needs
GNU C. Five fuzz targets compile separately under nightly.

`m3d2_reference` compares 38 original-body scenarios, 4,890 captures and 422,943
player/actor field values, plus angles, noise targets and combined event order.
C allocation ordinals map independently to observed Rust generations so slot reuse
is checked without treating addresses or C allocation order as canonical storage.
Cases cover all four input slots, held/released/repressed attack, raise timing,
switches during raise/ready/fire, ammo zero/last rounds, two-target shotgun order,
source body/bob timing, walls/vertical/side aim, deaths and sound-block propagation.
RNG wrapper counts show 323 draws in held shotgun and 268 in the long pistol case,
including wrap. Older source comparison/golden fixtures remain unchanged.

The separate v6 encoder uses C rows and explicit schema/setup only. Its 69 captures
and 24 complete byte states cover initial/raise/ready/attack-start and settled
outcomes. Full named-field comparisons precede digest comparisons. Transient shot
state is compared per tic by the C reference test; no full transient-shot v6 byte
fixture claim is made. The example asserts each player capture, pellet counts,
ammo, switching, walls and deaths, ending at independent digest `1cc5a89450da5585`.

Nine API policy tests and two internal interpreter tests cover unsupported commands
in all slots, setup/ownership/actions, atomic seven-impact capacity rejection,
protected bound targets, pending/stale/reused identities, pause/read schedules,
512-tic repeated streams, earliest ammo-field divergence, extreme bob momentum,
zero/null/infinite/coordinate psprite semantics and bounded pathological chains.
Only the isolated internal interpreter tests use modified psprite definitions;
public profile admission rejects them before mutation.

The bounded weapon fuzz target uses at most 32 commands after raising, three
admitted loadouts and whole-snapshot checks on every recoverable error. It adds
no stable dependency. Evolving corpora, sanitizer outputs and builds remain ignored;
only a tiny original synthetic seed is tracked. ASan with leak detection disabled
provides no LeakSanitizer evidence. Report observed counts and any failures.

## M3d2b projectile checks

```sh
python3 tools/reference/generate_m3d2b.py --check
python3 tools/reference/m3d2b_snapshot_goldens.py --check
cargo test -p doom-sim --test m3d2b_reference --locked -- --nocapture
cargo test -p doom-sim --lib projectiles --locked -- --nocapture
cargo run -p doom-sim --example projectiles --locked
python3 tools/reference/check_headless.py
python3 tools/reference/check_projectiles.py
cargo +nightly fuzz build --fuzz-dir fuzz
```

Run every older generator/encoder and normal workspace/release/MSRV/docs checks
as well. Recorded Rust comparisons need no C compiler, Python, IWAD or device.
The seventh example helper leaves the six-example historical helper unchanged.
CI reproduces all seven reference pairs on Linux, and recorded comparisons,
release tests, seven-example equality and warning-free docs on all stable OS jobs.

41 C scenarios compare actual actor fields, source/inflictor identities, RNG draws,
collision candidates/openings and ordered lifecycle/movement/impact observations.
The synthetic same-pass scenario lives in the unit suite to access the private
shared traversal cursor without exposing a callback scripting API. V7 checks 129
independent snapshots/35 full streams with named-field diagnostics; all older
versions are preserved. Error-policy, boundary and read-independence tests are
project consistency evidence; full vanilla/DOS behavior remains unverified.

`projectiles` fuzzing executes at most 12 three-byte operations against nine
public fixture setups. It varies launch handles/aims, removals, momentum, bindings,
typed spawn and stepping; every recoverable error compares complete snapshots.
A representative bounded campaign uses a temporary synthetic corpus:

```sh
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run projectiles --fuzz-dir fuzz \
  /tmp/projectile-corpus -- -runs=2000 -max_len=37 -seed=7
```

Populate that external directory with redistributable byte inputs first. Never
point fuzzing at private IWADs. Report actual executions and sanitizer findings;
`detect_leaks=0` means this does not verify LeakSanitizer. See validation for the
observed campaign and compiler versions. No test fixture is generated from Rust
expected results and no commercial asset enters public CI.

## M3d3a perception checks

```sh
python3 tools/reference/generate_m3d3a.py --check
python3 tools/reference/m3d3a_snapshot_goldens.py --check
cargo test -p doom-sim --test m3d3a_reference --locked -- --nocapture
cargo run -p doom-sim --example monster_perception --locked
python3 tools/reference/check_monster_perception.py
cargo +nightly fuzz build --fuzz-dir fuzz
cargo +nightly fuzz run monster_perception --fuzz-dir fuzz -- -runs=2000 -max_len=49 -seed=13
```

Run alongside all seven older reference pairs/helpers and required workspace,
release, MSRV and warning-free rustdoc checks. The C suite compares 24 pure sight,
20 scan and 24 A_Look cases (68 total), 1536 side vectors and 131 intercept vectors.
It checks ordered source observations, full actor fields, all raw gameplay RNG
draws, roster/sector fields and sight decisions. Rejected pure queries are checked
for state/RNG purity. Case/operation/query/visit diagnostics identify the first
sight difference; named canonical fields precede full-byte/digest comparisons.

The independent v8 encoder uses C rows, public setup and explicit schema, never
Rust output: 420 captures and 45 complete streams, including transient scan,
sight, wake/noise events and RNG wrap. Twelve project-only policy tests separately
cover empty-roster termination, invalid/pending/reused IDs, ambush assignment with
failed empty fallback, stale sound before movement, unsupported chase/handoffs,
protected reference states, independent player health, arithmetic/capacity/work
bounds, malformed cycles/shared DAGs, pause, separate instances and query cadence.

The seventh fuzz target generates bounded perception operations over synthetic
fixtures. Recoverable errors compare complete snapshots; it exercises sight,
scan/look/noise, roster edits, stale generations, removal/reuse, state/step/setup
and pause. The recorded campaign used ASan with `ASAN_OPTIONS=detect_leaks=0`;
leak detection is not claimed. All seven fuzz targets build. Public CI uses no
private fixtures: Linux reproduces all eight C/encoder pairs; stable Linux,
Windows and macOS compare recorded fixtures, release tests and eight example
outputs. Rust 1.85 runs the workspace suite.

## M3d3b pursuit checks

```sh
python3 tools/reference/generate_m3d3b.py --check
python3 tools/reference/m3d3b_snapshot_goldens.py --check
cargo test -p doom-sim --test m3d3b_reference --locked -- --nocapture
cargo test -p doom-sim --test m3d3b_policies --locked
cargo run -p doom-sim --example monster_pursuit --locked
python3 tools/reference/check_monster_pursuit.py
cargo +nightly fuzz build --fuzz-dir fuzz
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run monster_pursuit --fuzz-dir fuzz -- -runs=2000 -max_len=65 -seed=17
```

Run alongside every historical pair/helper and mandatory workspace/release/MSRV/
rustdoc checks. The new C harness compiles 60 tracked original bodies plus source
fixed/table helpers and extracts the original direction tables. It covers 96 cases
on 17 synthetic maps: all directions/NODIR, +/-10 ties, diagonal/axis/old/search/
turnaround attempts, strict actor overlap, steps 24/25, dropoffs, height/headroom,
blocked/omitted lines, corridors/traps, skill/counter/turning, reacquisition, active
sound, RNG wrap and whole state/thinker/spatial order. Per-case/op/observation and
named actor-field diagnostics stop at the first difference. Exact aggregate
counts are asserted. All raw gameplay draws are compared, not only final indices.

The independent Python v9 encoder reads C captures/public setup/schema only:
835 captures, 49 full byte streams, including transient movement/vertical/sight/
scan/Look/Chase/noise events. Rust compares named fields, complete bytes and
per-operation/per-tic digests. This is project schema evidence, not a DOOM format.
Twelve separate policy tests cover invalid input, attacks/float/specials, empty
roster, pending/retired/reused IDs, stale sounds, immediate callback bounds, the
two-decrement reaction edge, capacity, pause/independent instances and read-only
presentation. Unsafe/nonterminating C cases are not executed as equivalence tests.

The eighth fuzz target uses bounded synthetic operation sequences, including
walking, direction/chase/look/state calls, roster/health/flag/counter edits,
removal/reuse and momentum. Every recoverable error compares full snapshots.
The recorded ASan campaign completed 2000 executions (seed 17,max input 65,24 initial
synthetic seeds), no finding, leak detection disabled. All eight targets build.
Linux CI reproduces all nine C/encoder pairs; stable Linux/Windows/macOS run
recorded comparisons, release, nine example equality checks and rustdoc, with
Rust 1.85 workspace coverage. The ninth example asserts four complete pursuit/
loss/reacquisition traces and final digest `8c7fd6adfe73112d`.

## M3d3c attack checks

```sh
python3 tools/reference/generate_m3d3c.py --check
python3 tools/reference/m3d3c_snapshot_goldens.py --check
cargo test -p doom-sim --test m3d3c_reference --test m3d3c_policies --locked
cargo run -p doom-sim --example monster_attacks --locked
python3 tools/reference/check_monster_attacks.py
cargo +nightly fuzz build
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run monster_attacks -- -runs=10000 -seed=1 -max_len=65 -rss_limit_mb=2048 -timeout=20
```

Reproduce all ten historical/new source/encoder pairs and all ten asserted
examples; preserve every older helper and fixture. Linux CI regenerates evidence;
Windows/macOS compare checked-in source vectors and canonical bytes. Stable and
Rust 1.85 run the complete suite. No proprietary assets or devices are needed.

The new C harness compiles 64 genuine bodies across 158 scenarios on 4 exercised synthetic
maps, from 18 supplied fixture files. Rust compares 178,178 live actor fields, 2,430 raw gameplay draws and complete
state/event captures through the independent v10 encoder (1290 captures,72 full
streams with first-field diagnostics). Pure melee additionally compares source
sight visits/results without mutating canonical state. Direct aim/shot trace
candidates/intercepts/window/impact and damage events are included in v10 bytes.

Seven safety-policy tests cover capacity before direct/delayed actions, unsupported
roles/states/callbacks, immutable impact definitions, player protection, pending/
stale/reused generations, removable references, arithmetic, null/pure queries,
pause, independent instances and presentation reads. These are Rust policies,
not C behavior for undefined or unsupported inputs. `monster_attacks` checks
acquisition/walking/rejection/selection/face/sound/RNG/effects/return/retirement;
final 110-tic independently encoded digest is 245e71d4a9c4023e.

The recorded ASan campaign used six public synthetic 65-byte seeds, at most eight
operations per input and 10,000 executions. All nine fuzz targets built. No finding
occurred; detect_leaks=0 means LeakSanitizer coverage is not claimed. Corpus,
mutations and sanitizer output remain local ignored/temporary files.

## M3d3d shotgunner checks

```sh
python3 tools/reference/generate_m3d3d.py --check
python3 tools/reference/m3d3d_snapshot_goldens.py --check
cargo test -p doom-sim --test m3d3d_reference --test m3d3d_policies --locked
cargo run -p doom-sim --example shotgunner_attacks --locked
python3 tools/reference/check_shotgunner_attacks.py
cargo +nightly fuzz build
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run shotgunner_attacks -- -runs=10000 -seed=1 -max_len=65 -rss_limit_mb=2048 -timeout=20
```

Run all eleven source/encoder pairs and all eleven example comparisons; preserve
all historical code, fixture bytes and helpers. Linux CI reproduces original C;
all stable platforms compare recorded vectors/canonical bytes. Rust 1.85 exercises
the workspace. No private/commercial assets, devices or network are needed locally.

The C harness compiles 65 tracked source bodies and executes 105 cases across all 18
supplied maps. Rust compares 147,940 actor fields, 34,350 ordered observations and 3,894
raw draws, including each pellet's exact post-effect/damage RNG index. V11's
independent encoder produces 949 captures/92 complete streams with named-field and
full-byte checks. Distinct per-pellet trace scopes prevent mixing three line
attacks in one operation. Spread-extreme inputs derive from adjacent original
RNG LUT values, never Rust output. The example verifies a pellet 2 kill followed
by pellet 3 hitting another body and repeated state cycles; final 130-tic digest
is `eb5b7166a162544e`.

Nine policy tests separately cover zero/one/two free slots, delayed-action and
pending-retirement capacity, conservative misses/null exception, role/player/Pos
rejection, exact timing/fullbright/closure, stale/pending/reused IDs, unsupported
metadata/flags/arithmetic, sequential health/retargeted hit, pause/instances/reads
and first-field diagnostics. Those are Rust safety claims, not C behavior for
undefined inputs. The declared closure uses at most three touched bodies/three
projected effects and never clones full simulation/map/store state per tic.

All ten fuzz targets built with ASan. The new campaign completed 10,000 executions
in 211 seconds, seed 1,max_len 65,timeout 20 seconds,RSS limit 2048 MiB, at most eight
operations per input, from six public synthetic seeds, with no findings.
`ASAN_OPTIONS=detect_leaks=0` disables leak detection; no LeakSanitizer claim is made.
Generated corpus/mutations/logs remain temporary or ignored local outputs.
