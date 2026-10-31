# Testing and reproduction

Run commands from the repository root. Public checks use synthetic or legally
redistributable fixtures; no commercial IWAD, renderer, audio device or network
service is required after toolchain/dependency installation.

## Required Rust checks

```sh
cargo build --workspace --locked
cargo fmt --all -- --check
cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p doom-sim -p doom-types --release --locked
cargo +1.85.0 test --workspace --locked
```

Set `RUSTDOCFLAGS="-D warnings"` in your shell, then run:

```sh
cargo doc --workspace --no-deps --locked
```

Stable toolchain configuration is in [rust-toolchain.toml](../rust-toolchain.toml).
MSRV is Rust 1.85.0. Warnings-denied Clippy and rustdoc are CI requirements.

## Original C and independent canonical bytes

On a supported GNU C host (Linux, 32-bit int/fixed/angle, 16-bit short,
64-bit pointers, little-endian):

```sh
python3 tools/reference/check_references.py
python3 tools/reference/m3f1_reference.py --check
python3 tools/reference/m3f1_snapshot_goldens.py --check
```

The frozen historical helper covers the original reference pairs. Run the
separate milestone commands below as well; it does not discover newer tools.
It does not regenerate expectations to match Rust. For the expanded geometry slice:

```sh
python3 tools/reference/m3f1_reference.py --check
python3 tools/reference/m3f1_snapshot_goldens.py --check
cargo test -p doom-sim --test m3f1_reference --test m3f1_policies --locked
```

All platforms compare committed C observations and independent full bytes/digests
in Rust tests. Source-body reproduction is intentionally Linux-only in CI.
[Reference tooling](../tools/reference/README.md) documents provenance and adapters;
[STATE_FORMAT](STATE_FORMAT.md) specifies DSIM. DSIM is diagnostic state, not a
vanilla demo or savegame.

## Asserted headless examples

```sh
cargo run -p doom-map --example parse_map --locked
cargo run -p doom-sim --example demon_map_context --locked
python3 tools/reference/check_examples.py
python3 tools/reference/check_map_scale.py
python3 tools/reference/check_map_extent.py
```

The frozen historical helper and separate milestone helpers below compare
debug/release output for the committed simulation examples. They span
movement, collision, sectors, lifecycle, combat, weapons/projectiles, perception,
pursuit, selected attacks and Demon gameplay/map context. Assertions use committed
original-C evidence and independent snapshots where applicable. Earlier examples
and equality helpers retain their historical bytes.

## Fuzzing and sanitizers

Install cargo-fuzz and a supported nightly Rust toolchain, then:

```sh
cargo +nightly fuzz build --sanitizer address
cargo +nightly fuzz run map_scale --sanitizer address -- -runs=4096 -max_len=35 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The build covers all committed fuzz targets. The map-scale target mutates public
classic geometry, source BLOCKMAP references, malformed indexes and setup budgets,
then exercises bounded controls/queries/stepping, stale identities and complete-state
rejection in two independent instances. Actual corpus/model/execution results are
owned by [M3f1 evidence](validation/M3F1.md). Historical campaigns remain in their
original records, including [M3d3j](validation/M3D3J.md).
If your environment requires `ASAN_OPTIONS=detect_leaks=0`, disclose it; that
campaign has no LeakSanitizer coverage. Bounded campaigns are not exhaustive.

## Documentation and preservation

```sh
python3 tools/check_docs.py
```

Verify preserved upstream files against baseline
`a77dfb96cb91780ca334d0d4cfd86957558007e0`, historical reference/helper bytes against
the feature base, Git ancestry and license notices. The
[archive manifest](validation/archive/manifest.json) records exact historical
ledger bytes and rebased navigation copies. Private opt-in asset tests stay ignored;
see [ASSETS](ASSETS.md) without accessing such assets for public validation.

For earlier per-profile procedures and qualified historical sanitizer observations,
read the [complete testing archive](validation/archive/pre-m3d3j-testing.md).

## Stacked documentation PRs

The existing [workflow](../.github/workflows/rust.yml) automatically validates PRs
targeting master; its branch filter does not run on a PR targeting a feature
branch. Keep such a documentation stack draft and report its exact-head CI as
unavailable. A passing prerequisite run is evidence for that base commit, not the
new docs head. After the prerequisite merges, retarget to master, recheck the
resulting diff/base and run fresh required local and exact-head platform checks.
Do not change workflow filters or create a duplicate PR to work around this.

## Map-scale measurements

On Linux with GNU C and a release Rust toolchain:

```sh
cargo run -p doom-sim --example map_scale --locked
python3 tools/reference/summarize_m3f1.py
python3 tools/measure_map_scale.py
```

The last command prints three-sample host measurements as JSON; it is not a CI
performance threshold or a golden file. It builds the release example and a small
native `wait4` launcher, then measures preparation excluding parsing, collision,
sight, snapshot-inclusive stepping, requested prepared capacities and process peak
RSS. Recorded results and method limitations live in [M3f1 evidence](validation/M3F1.md).

## Expanded map extents and spatial work

Run the new pair alongside every historical source/encoder pair above:

```sh
python3 tools/reference/m3f2_reference.py --check --overflow-check
python3 tools/reference/m3f2_snapshot_goldens.py --check
python3 tools/reference/summarize_m3f2.py
cargo run -p doom-sim --example map_extent --locked
python3 tools/reference/check_map_extent.py
```

The overflow check recompiles genuine C with signed-overflow, integer-division and
float-cast sanitization and requires identical outputs. It does not claim portable
ISO-C negative shifts or exhaustive undefined-behavior coverage; the GNU source
setup-shift contract is declared in [source findings](M3F2_REFERENCE.md).
To reproduce the separately excluded original C failure on GNU/Linux:

```sh
python3 tools/reference/m3f2_reference.py --check --probe-undefined-clip
```

The tool expects the original abs(INT_MIN) sanitizer failure and prints it. This
is evidence for profile 19's recoverable rejection, not a passing original-C full
sight comparison. Standard reference modes never enter the undefined continuation.

The source/encoder names deliberately stay outside historical `generate*.py`
discovery; frozen aggregators/helpers remain unchanged.

For the bounded synthetic campaign, after installing cargo-fuzz/nightly:

```sh
python3 tools/seed_map_extent_fuzz.py fuzz/corpus/map_extent
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run map_extent fuzz/corpus/map_extent --sanitizer address -- -runs=4096 -max_len=35 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The environment setting explicitly disables leak detection. Do not report that
campaign as LeakSanitizer coverage. Corpus and artifacts remain ignored development
outputs. Inputs exercise expanded geometry/origins, shared lists, repeated BSP
children, budgets, stale identities, controls, pause/RNG and complete-state errors
in two instances; no proprietary data is needed.

On Linux with GNU C, collect separate release-process samples:

```sh
python3 tools/measure_map_extent.py
```

This prints JSON, not a performance golden or acceptance threshold. The same small
fixture is measured under profiles18/19; larger fixtures have different operation
volumes and cannot establish like-for-like regressions. Parsing and full preparation
are timed separately; preparation includes geometry/sight validation, BLOCKMAP and
topology work and canonical-cache construction, not isolated subsystem timers.
[Measured observations](validation/M3F2.md) own results, memory scope and limitations.

## Expanded-map Demon gameplay composition

Run both M3f3 source/encoder adapters alongside all historical pairs:

```sh
python3 tools/reference/m3f3_reference.py --check --overflow-check --probe-undefined-angle
python3 tools/reference/m3f3_snapshot_goldens.py --check
python3 tools/reference/m3f3_clipping_reference.py --check --overflow-check
python3 tools/reference/m3f3_clipping_snapshot_goldens.py --check
cargo test -p doom-sim --test m3f3_reference --test m3f3_clipping_reference --locked
cargo run -p doom-sim --example demon_expanded_map --locked
python3 tools/reference/check_demon_expanded_map.py
python3 tools/measure_demon_expanded_map.py
```

Every capture compares complete ordered event-field bytes, in addition to full
snapshot fixtures and per-tic digests. Event projections come from the independent
C encoder and include named-field/first-byte divergence diagnostics.

The extra adapter explicitly executes non-cardinal AI rays through differing
openings and source slope clipping on larger maps. Both use the same genuine
source bodies. The strict audit excludes the original signed ANG45 overflow; the
optional expected-failure component extracts the exact original expression and
requires its positive-180-degree sanitizer diagnostic. That finding proves the
new angle rejection policy, not a successful full spawn after undefined behavior.
Signed-shift/GNU ABI assumptions remain declared. Rust ASan and C arithmetic
sanitization are separate evidence families.

```sh
python3 tools/seed_demon_expanded_map_fuzz.py fuzz/corpus/demon_expanded_map
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run demon_expanded_map fuzz/corpus/demon_expanded_map --sanitizer address -- -runs=4096 -max_len=35 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The bounded model uses two instances, at most eight four-byte operations and
64 tic calls per instance. It mutates public geometry/THING admission, roles,
identities, actions, damage, sight and setup/operation atomicity. Its exact MIN
seed exercises read-only rejection; nested AI rejection is separately injected in
Rust safety tests. The evolving corpus stays ignored. Disabled leak detection is
not LeakSanitizer coverage. Actual executions and timings belong in
[M3f3 observations](validation/M3F3.md).

Measurements use separate release processes and source-backed inputs; only the
same small input under profiles 17/20 is a direct gameplay comparison. Larger-map
unbound and typed workloads have different behavior and observation volumes.
The tool records construction/spawn/query/Chase/step/snapshot timings and native
peak RSS. It defines no performance threshold; raw samples and caveats belong in
the evidence record.

## Original actor/state identity metadata

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3f4_reference.py --check --sanitize
cargo test -p doom-sim --test m3f4_reference --locked
python3 tools/reference/check_original_identity.py
python3 tools/seed_original_identity_fuzz.py fuzz/corpus/original_identity
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run original_identity fuzz/corpus/original_identity --sanitizer address -- -runs=4096 -max_len=68 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The C observer compiles untouched original tables and evaluates actual enum
symbols. Aborting action stubs supply pointer-to-symbol metadata only. It never
reads Rust output. Its separate schema preserves every historical DSIM fixture.
Rust policy tests exhaust u16 candidates, signed extremes and altered registry
records; existing independent captures verify chronology and RNG retention.
The fuzz target bounds name input, checks descriptors/round trips and mutates
registry properties. It does not broaden gameplay. Both sanitizer commands disable
leak detection explicitly; they provide no LeakSanitizer coverage.
Run all historical commands above as well. Reuse the unmodified
`tools/measure_demon_expanded_map.py` on the base and feature with the same
release toolchain/workloads, three samples each; compare identical cases/profiles.
[M3f4 observations](validation/M3F4.md) own results and measurement limitations.

## Canonical Demon execution binding

These checks retain the original canonical C identities during gameplay, then
compare an explicitly projected native Profile20 representation. They add no
DSIM schema or gameplay profile. Run them alongside every historical check above:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3f5_reference.py --check --overflow-check --address-check
python3 tools/reference/m3f5_snapshot_goldens.py --check
cargo test -p doom-sim --test m3f5_reference --test m3f5_policies --locked -- --nocapture
python3 tools/reference/check_canonical_demon.py
cargo run -p doom-sim --example canonical_demon --locked
cargo run -p doom-sim --example canonical_demon --release --locked
```

The observer retains raw original type/state fields in DOOM-DEMON-EXEC-v1 files.
The independent encoder declares identity-only translation before reusing the
historical DSIM-v20 encoder. Rust compares C physical/statistics/RNG observations,
canonical definition queries, ordered event fields and independent snapshots.
Separate policy tests cover metadata mutation, wrong type/owner, raise, corpse,
stale-handle and atomicity boundaries. See [investigation](M3F5_REFERENCE.md) and
[observations](validation/M3F5.md) for exact coverage and limitations.

```sh
python3 tools/seed_canonical_demon_fuzz.py fuzz/corpus/canonical_demon
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run canonical_demon fuzz/corpus/canonical_demon --sanitizer address -- -runs=4096 -max_len=35 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_demon_expanded_map.py > /tmp/m3f5-legacy-performance.json
python3 tools/measure_canonical_demon.py > /tmp/m3f5-api-performance.json
```

Build-all and actual campaign execution are different results. The seed recipe
creates fresh synthetic requests; evolving data stays ignored. Leak detection is
disabled in the documented managed-host campaign; no leak coverage is claimed.
The legacy benchmark supplies matching before/after workloads. The new canonical
API workload includes explicit validation/query costs and is reported separately;
it has no pre-M3f5 API baseline. Timing output is observational, never an assertion.

To exercise cross-filesystem provenance reproducibility without changing source
observations, reverse file discovery in both new publishers. This checks whole
publisher output bytes, separately from gameplay comparisons:

```sh
python3 - <<'PY'
import sys
from pathlib import Path
sys.path.insert(0, 'tools/reference')
import m3f5_reference as source
import m3f5_snapshot_goldens as encoder
raw = source.capture(list(source.main.legacy.base.FLAGS))
source.capture = lambda flags: dict(reversed(list(raw.items())))
source.run(True, False)
original = Path.glob
try:
    Path.glob = lambda path, pattern: iter(reversed(list(original(path, pattern))))
    encoder.run(True)
finally:
    Path.glob = original
PY
```

## Native runtime identity boundary

Run the complete historical suite and M3f4/M3f5 C/encoder checks above, plus:

```sh
cargo test -p doom-sim --test m3f6_reference --test m3f6_policies --locked -- --nocapture
python3 tools/reference/check_runtime_identity.py
cargo run -p doom-sim --example runtime_identity --locked
python3 tools/seed_runtime_identity_fuzz.py fuzz/corpus/runtime_identity
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run runtime_identity fuzz/corpus/runtime_identity --sanitizer address -- -runs=4096 -max_len=35 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_demon_expanded_map.py
python3 tools/measure_canonical_demon.py
```

The new source comparison reuses immutable M3f5 raw C observations and independent
snapshots/event fields; it does not generate another capture or claim new C input
coverage. Policy tests exhaust all u16 native candidates, canonical/native
collisions, direct-entry restrictions, purity, replay and stale/reused ownership;
private corruption tests ensure canonical ordinals in native storage reject
atomically. The new example runs in debug/release against existing C-derived output.

The fuzz model uses two instances, bounded native/canonical requests and at most
eight operations; corpus seeds include valid native slots, source ordinals and
sentinels. Build-all and campaign counts are separate. Leak detection is disabled
by the documented setting; no leak coverage or exhaustive proof is implied.
Retain matched before/after three-sample legacy and canonical-API workloads and
all outliers. Those M3f6 measurements contain no canonical-storage workload;
the separate original-ID workload below belongs to M3f7.
[M3f6 observations](validation/M3F6.md) retain the historical results.

## Original-ID registry and diagnostic contract

```sh
cargo test -p doom-sim --test m3f7_reference --test m3f7_policies --test m3f7_diagnostics --locked
python3 tools/reference/m3f7_snapshot_goldens.py --check
python3 tools/reference/check_original_registry.py
python3 tools/check_docs.py
```

The C comparison reads immutable M3f5 raw source observations against actual
ActorLife source IDs; it checks every ordered event payload by reusing the old
EV20 bytes with identity fields supplied by C rows. New compact diagnostics retain
provenance and two complete fixtures; no new C-input/gameplay coverage is claimed.
Keep all existing generators/encoders, metadata/source sanitizer checks and all
historical debug/release examples in the full validation run. New decoder policy
checks are structural/identity safety, not physical-state restoration proof.

```sh
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_original_registry_fuzz.py
python3 tools/seed_diagnostic_fuzz.py
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run original_registry fuzz/corpus/original_registry --sanitizer address -- -runs=4096 -max_len=34 -seed=1 -timeout=20 -rss_limit_mb=2048
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run diagnostic_format fuzz/corpus/diagnostic_format --sanitizer address -- -runs=4096 -max_len=131072 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_demon_expanded_map.py
python3 tools/measure_canonical_demon.py
python3 tools/measure_original_registry.py
```

## Context and target-lifetime validation

Run the new pair alongside every historical pair above:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3f8_reference.py --check --overflow-check --address-check
python3 tools/reference/m3f8_snapshot_goldens.py --check
cargo test -p doom-sim --test m3f8_reference --test m3f8_policies --locked -- --nocapture
cargo test -p doom-sim --doc --locked -- --show-output
python3 tools/reference/check_context_ownership.py
```

The observer supplies new controlled C inputs, not another implementation of the
engine. Original source/body hashes are checked against inherited provenance.
Logical retirement uses the declared storage quarantine; no freed-pointer or
allocator-reuse gameplay equivalence is asserted. The independent encoder reads
C observations and declared v21 schema policy, not Rust output. Rust policy tests
separately cover lifetime brands, local admission, rejected mutations, retained
references and generation reuse. Rustdoc output confirms the expected lifetime
errors rather than an unrelated syntax error. Whole workspace, release and MSRV
checks also include the focused tests and generation-exhaustion unit case.

```sh
python3 tools/seed_context_ownership_fuzz.py
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run context_ownership fuzz/corpus/context_ownership --sanitizer address -- -runs=4096 -max_len=128 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The new target decodes bounded operation chunks and malicious handle/definition/
state/roster candidates, compares two executions and checks complete-state
rejection. Continue relevant original-registry/diagnostic campaigns above; building
all targets does not mean all targets were executed. Leak detection disabled by
the environment is not leak coverage. Corpus files remain ignored local inputs.

For matched measurements, retain all before/after JSON from the unchanged
`measure_demon_expanded_map.py`, `measure_canonical_demon.py` and
`measure_original_registry.py` workloads. Also run:

```sh
python3 tools/measure_context_ownership.py
```

This new microbenchmark compares raw and scoped access paths on the same inputs,
mutations and retirement cycles, with identical final digests. Scoped observations
perform additional validation; permissive raw query latency is not equivalent
work. It is not a historical before baseline or a whole-simulation benchmark.
The [record](validation/M3F8.md) owns actual counts, samples, RSS and limitations.

The first two measurement tools retain unchanged workloads for matched baseline/
feature comparisons. The third is separate: source-ID diagnostics omit immutable
metadata and have different volume, so its timings are not a like-for-like legacy
speedup. Keep all three process samples/outliers and RSS. Build-all and executed
campaign counts are separate; disabled leak detection supplies no leak coverage.
[Design](M3F7_REFERENCE.md) and [observations](validation/M3F7.md) define the scope.

## Game-instance provenance validation

```sh
cargo test -p doom-sim --test m3f9_policies --test m3f9_reference --locked -- --nocapture
cargo test -p doom-sim --doc --locked -- --show-output
python3 tools/reference/check_game_instance.py
python3 tools/check_docs.py
```

The new reference test and example drive the existing M3f8 replacement/clear input
through qualified imports. Expected fields, RNG and complete event payloads still
come from frozen C observations and the independent encoder. No new C scenarios,
raw traces or diagnostic version are introduced. Continue every historical/source/
metadata/sanitizer command above. Rust policy tests separately cover identical raw
IDs in different worlds, clone/move/extraction, successful/failed restart, current/
pending/corpse/retired/stale/vacant/invalid actors, missing/foreign provenance, wrong
definitions, pure rejection and exact replay/snapshot/RNG preservation. Inspect
compile-fail diagnostics for invariant token escape and a non-Clone owner.

```sh
python3 tools/seed_game_instance_fuzz.py
cargo +nightly fuzz build --sanitizer address
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run game_instance fuzz/corpus/game_instance --sanitizer address -- -runs=4096 -max_len=128 -seed=1 -timeout=20 -rss_limit_mb=2048
```

The bounded harness selects local opaque keys; input bytes never become InstanceId.
It compares two instances, malformed candidates, missing/foreign/old routing,
generation/state/definition guards, restart/state-clone and rejected-operation
atomicity. Continue the original-registry and diagnostic campaigns above. Building
all targets is separate from executing them. Record seeds, actual executions,
limits, RSS, findings and leak configuration; leak0 is not leak coverage.

Retain all four existing measurement tools unchanged for matched before/after
three-process samples; then measure the separate boundary workload:

```sh
python3 tools/measure_game_instance.py
```

This separately times checked import/export and compares identical gameplay with
an already scoped actor versus explicit reimport on each update. Validation work
differs; import is never inserted automatically into ordinary tic execution.
Retain every sample/outlier/RSS and avoid universal or causal performance claims.
The [evidence record](validation/M3F9.md) owns actual results and limitations.

## Snapshot-free stepping validation

```sh
cargo test -p doom-sim --lib advance_tests --locked
cargo test -p doom-sim --test m3g1_reference --test m3g1_policies --locked
python3 tools/reference/check_diagnostic_free.py
```

The unit capture-boundary hook checks success, pause and rejection without timing
assumptions. Complete snapshots/bytes compare across every supported profile;
reference tests reuse frozen C observations on both APIs, not Rust-generated
expectations. Historical callers, encoders and all example checks remain required.
The new example requests its final diagnostic explicitly.

On Linux/glibc with a C compiler, run advisory optimized measurements:

```sh
python3 tools/measure_stepping.py --check
python3 tools/measure_stepping.py > stepping-observations.json
```

Full measurement uses three separate processes per mode/workload, separate
allocation-counter processes with mandatory zero/positive controls, and private
preflight/staging instrumentation in the release test binary. Host-only
[interposition](../tools/measurement/alloc_probe.c) adds no engine dependency or
unsafe Rust. Counts cover the marked thread's intercepted allocator requests;
requested bytes are cumulative, not peak live heap or leaks. Timing/RSS samples
run without the counter. Private frozen-state component timers are not additive
to evolving update timers, and test-harness RSS differs from example RSS.
Process RSS includes setup and final verification outside the measured interval.
No shared-host timing threshold, CPU isolation or universal speed claim applies.
CI executes a small advisory smoke plus deterministic structural tests.
[The record](validation/M3G1.md) retains all samples and remaining costs.

The bounded refinement fuzz model uses the existing profile setups, pause,
commands, RNG, removals and invalid handles:

```sh
python3 tools/seed_stepping_fuzz.py /tmp/doom-stepping-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run stepping_equivalence /tmp/doom-stepping-seeds --sanitizer address -- -runs=4096 -max_len=97 -seed=1 -timeout=20 -rss_limit_mb=2048
```

Build all targets as above, and report executed targets separately. This setting
provides no leak coverage; bounded executions are not exhaustive. Frozen helper
reuse needs a narrow duplicate-module Clippy allowance in the new host input
modules; no runtime lint, assertion or historical helper is relaxed.

## Original player body validation

Run the standard workspace/MSRV/release/format/Clippy/rustdoc and historical
commands above unchanged, then the new source/encoder/example checks:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3g2_reference.py --check --overflow-check --address-check
python3 tools/reference/m3g2_snapshot_goldens.py --check
cargo test -p doom-sim --test m3g2_reference --test m3g2_policy --locked
cargo test -p doom-sim --lib original_player --locked
python3 tools/reference/check_original_player.py
cargo run -p doom-sim --example original_player --locked
cargo +nightly fuzz build --fuzz-dir fuzz
python3 tools/seed_player_fuzz.py /tmp/doom-player-corpus
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run original_player /tmp/doom-player-corpus -- -runs=4096 -seed=1 -max_len=97 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_player_body.py > /tmp/player-measurements.json
```

The source observer uses actual MT_PLAYER0 and canonical Demon12, with source
association fragment and explicit presentation-service boundaries. The independent
encoder uses C rows/schema policy, never Rust output. C field/event comparisons,
full-byte/digest encoding, scoped/generation/provenance safety, rollback and API
refinement are separate tests. The new reader is explicitly decode_player; the
historical decoder must still reject22. Old fixtures/expected bytes are immutable.

For matched production measurements run unchanged `tools/measure_stepping.py`
before/after on the same toolchain and inputs; use advance rows for gameplay.
Retain old five-tool measurements as separate step/setup/sight/collision controls.
New player measurements have no historical before-player equivalent. Their full
world construction+spawn timer is separate from100-update timers. Keep all three
process samples/RSS and separate allocation-process samples, controls and outliers.
No hard shared-host time gates, zero-allocation inference or leak-coverage claim.
See [scope](COMPATIBILITY.md#original-player-body-lifecycle),
[source investigation](M3G2_REFERENCE.md) and [results](validation/M3G2.md).

## Grounded original-player knockback

Run the new observers alongside every historical pair and M3g2 command:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3g3_reference.py --check --overflow-check --address-check
python3 tools/reference/m3g3_snapshot_goldens.py --check
cargo test -p doom-sim --test m3g3_reference --test m3g3_policy --locked
cargo run -p doom-sim --example player_knockback --locked
python3 tools/reference/check_player_knockback.py
python3 tools/measure_player_knockback.py
```

The genuine source observer retains map-THING spawning for Demon origins,
source association for one player body, eligible thrust and actual subsequent
XY/friction/axis-slide execution. Its manifest separates compiled inventory from
actual entry counts and new inputs from inherited bodies. The independent encoder
compares selected complete states and every frame's ordered event payloads without
Rust-generated expectations. Rust-only policy tests cover handles/provenance,
geometry/arithmetic rejection, rollback and both stepping APIs. See
[source findings](M3G3_REFERENCE.md), [support](COMPATIBILITY.md#original-player-grounded-knockback)
and [observed results](validation/M3G3.md).

After building all fuzz targets, use a fresh development corpus:

```sh
python3 tools/seed_knockback_fuzz.py fuzz/corpus/player_knockback
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_knockback fuzz/corpus/player_knockback --sanitizer address -- -runs=4096 -max_len=129 -seed=1 -timeout=20 -rss_limit_mb=2048
```

Also execute relevant unchanged player/stepping/provenance targets; building them
is not an executed campaign. Leak detection is disabled in the command above and
provides no LeakSanitizer evidence. Keep corpus/artifacts outside tracked fixtures.
The host measurement tool uses three independent timing/RSS processes and separate
allocation probes per workload/mode. Construction is distinct from100 updates;
new physical workloads have no executable baseline equivalent. Retain unchanged
historical tools and matched old-player workloads for before/after comparisons.


## Original player command movement

Read [source assumptions](M3G4_REFERENCE.md) and the
[bounded input contract](COMPATIBILITY.md#original-player-command-movement).
Keep every historical check above; the new observer/encoder are additive:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3g4_reference.py --check --overflow-check --address-check
python3 tools/reference/m3g4_snapshot_goldens.py --check
cargo test -p doom-sim --test m3g4_reference --test m3g4_policy --locked
python3 tools/reference/check_player_commands.py
cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_player_commands_fuzz.py /tmp/doom-command-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_commands /tmp/doom-command-seeds --sanitizer address -- -runs=4096 -max_len=129 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_player_commands.py > /tmp/doom-command-measurements.json
```

The source check intentionally verifies a negative-shift UB diagnostic separately;
its normalized gameplay observer must pass strict/ASan/UBSan equality. Defined
literal source cases are checked independently of negative-turn bit policy. This
is a command-gate harness, not full PlayerThink or a player roster. Rust tests
separately exercise ownership/provenance, inactive/stale input, bounds, late rollback,
full step/advance refinement and zero implicit advance captures. The new fuzz
model compares outcomes and complete rejected state, decodes bounded diagnostics,
and mutates external inputs; it is not an original-C oracle or exhaustive proof.

Retain matched existing measure_stepping, measure_player_body and
measure_player_knockback controls; new command workload timing is separate. Tools
report three independent timing/RSS and three allocation processes, raw samples
and validated allocator controls. Source fields/capture counts, actually executed
campaigns and host limitations belong in [M3g4 evidence](validation/M3G4.md).

## Active original player and Demon acquisition

The [capability contract](COMPATIBILITY.md#active-original-player-and-demon-acquisition)
and [source matrix](M3G5_REFERENCE.md) define fixed active player0, genuine Demon
acquisition, nonmelee pursuit and explicit rollback at the melee boundary.
Historical helpers and expected bytes remain unchanged.

```sh
cargo test -p doom-sim --test m3g5_reference --test m3g5_policy --locked
cargo test -p doom-sim --lib player_roster --locked
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3g5_reference.py --check --overflow-check --address-check
python3 tools/reference/m3g5_snapshot_goldens.py --check
python3 tools/reference/check_player_demon.py
cargo run -p doom-sim --example player_demon --locked
cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_player_demon_fuzz.py /tmp/doom-player-demon-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_demon /tmp/doom-player-demon-seeds --sanitizer address -- -runs=4096 -max_len=161 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_player_demon.py
```

The new source command harness is a declared bounded movement gate before
P_RunThinkers, not full active-player P_Ticker/PlayerThink execution. Never execute
all-inactive original P_LookForPlayers as a terminating test. Source death/corpse
retention is distinct from Rust fixed-body removal and stale/foreign-handle
rejection. Leak detection remains disabled under the tracing environment;
bounded fuzzing is neither exhaustive proof nor leak coverage.

Run every existing workspace/MSRV/release, example, historical reference/encoder,
metadata, parser/generator, documentation, preservation and cross-platform gate.
The new helper supplements all existing checks; the two asset-gated tests stay
ignored. Measure new workloads separately from matched frozen historical tools.
`advance` measures production stepping; diagnostic `step` is a separate mode.
Raw three-process timing/RSS/allocation samples and limitations belong in
[the engineering evidence](validation/M3G5.md).

## Autonomous Demon-player melee

Run the new observer/encoder alongside every historical pair and all existing
checks. Expected source and diagnostic files are independent of Rust output:

```sh
cargo test -p doom-sim --test m3g6_policy --test m3g6_reference --locked
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3g6_reference.py --check --overflow-check --address-check
python3 tools/reference/m3g6_snapshot_goldens.py --check
python3 tools/reference/check_player_demon_melee.py
cargo run -p doom-sim --example player_demon_melee --locked
cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_player_demon_melee_fuzz.py /tmp/doom-player-demon-melee-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_demon_melee /tmp/doom-player-demon-melee-seeds --sanitizer address -- -runs=4096 -max_len=162 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_player_demon_melee.py
python3 tools/measure_player_demon_melee.py --smoke
```

The default example checks original-C fields, RNG, complete ordered events and
independent diagnostics, including pure autonomous acquisition through lethal
damage and a terminal corpse. It compares every scripted tic through step and
advance followed by explicit snapshot. Separate Rust policy tests cover invalid
roles/generations/provenance, pending/retired storage, conservative impulse limits,
late failure after real staged damage and zero implicit advance captures.

The source tool reuses the frozen bounded command gate before original thinkers;
it does not run full P_Ticker/PlayerThink/DeathThink. Current range is rechecked at
attack time. Range misses, absent targets and ignored dead-body damage have
different RNG consequences. New turn inputs are nonnegative literal defined cases;
historical negative-turn qualification remains reused evidence. Never execute an
unbounded all-inactive source scan or dereference freed pointers.

The new fuzz model warms only bounded committed timelines, then exercises individual
checked operations in two instances, complete rejection/equivalence and diagnostic
corruption. Built targets and actually executed campaigns are separate observations.
Leak detection is disabled above; bounded runs are not exhaustive or leak coverage.

Measure old controls with their unchanged tools. New melee intervals each contain
100 shared-core updates, with setup and final explicit diagnostics excluded.
Repeated melee warms64 tics; corpse continuation warms300 tics before its measured
interval. Other labels start at tic0. Near-lethal health reduction, prior impulse
and reaction delay are declared setup; no timed interval injects an attack or
damage. Advance and diagnostic step are separate modes. Retain all three timing/
RSS and separate allocation-process samples, outliers and workload fingerprints.
The smoke mode is advisory and has no timing threshold. See
[source boundaries](M3G6_REFERENCE.md), [support](COMPATIBILITY.md#autonomous-original-demon-player-melee)
and [measured observations](validation/M3G6.md).


## Genuine player pistol

Use synthetic inputs only. Run the complete standard suite and every historical
source/encoder family, then the new genuine-player weapon checks:

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3g7_reference.py --check --overflow-check --address-check
python3 tools/reference/m3g7_snapshot_goldens.py --check
cargo test -p doom-sim --test m3g7_reference --test m3g7_policy --locked
cargo test -p doom-sim --lib player_pistol --locked
python3 tools/reference/check_player_pistol.py
cargo run -p doom-sim --example player_pistol --locked
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_player_pistol_fuzz.py /tmp/doom-player-pistol-seeds
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_pistol /tmp/doom-player-pistol-seeds --sanitizer address -- -runs=4096 -max_len=162 -seed=1 -timeout=20 -rss_limit_mb=2048
python3 tools/measure_player_pistol.py
python3 tools/measure_player_pistol.py --smoke
python3 tools/measure_legacy_weapons.py
```

The compiled-C observer executes genuine reborn/setup, weapon/psprite, aim/trace,
impact, damage and Demon functions over the declared command/weapon stage before
original thinkers. It does not execute complete PlayerThink/Ticker/DeathThink.
The independent encoder reads only source rows and explicit DSIM23 policy. Full
byte fixtures precede matching Rust integration. Separate actual aim-query tests
compare cells, candidates, intercepts, windows, targets and source stopping.
Compiled bodies, linked fixed math, instrumented function entries, source fields,
new composition evidence and reused historical coverage are counted separately.

Rust policy tests cover unsupported inputs, missing/pending/stale/reused/foreign
ownership, invalid weapon state, capacity and late rollback, empty-ammo fallback,
schema corruption, deterministic replay and full step/advance equivalence. These
are safety claims, not fabricated C lifetime behavior. The source driver defaults
commands after death and never scans an all-inactive roster or freed C pointer.

Fuzz targets built are distinct from campaigns actually run. Record seeds,
execution and byte budgets, timeouts, memory limits and findings. Leak detection
is disabled by the recorded ASAN_OPTIONS because of the environment's tracing
limits; this is not leak coverage or exhaustive testing. Historical campaign
parameters and fixtures remain unchanged.

Pistol benchmarks use100 production advances or separately100 diagnostic steps,
with source inputs, warmup/setup and final diagnostics outside the measured
interval. Three separate timing/RSS processes and three allocation-instrumented
processes retain all samples/outliers. New labels are continuous source scenarios,
not isolated shot/pain/retaliation costs; some labels intentionally share an
interval. Compare old controls only with identical frozen workloads/tools and
compiler settings. Whole-process RSS includes setup; requested allocation bytes
are cumulative and do not establish peak live heap or leaks. No shared-runner
hard timing gate is used. [Observations](validation/M3G7.md) hold raw records.

CI preserves all required jobs and the native Intel macOS runner. Linux adds the
new source/encoder checks and all stable platforms run the asserted pistol
example. Its timeout has additional headroom because the unchanged M3g6
exact-master Linux job approached28 minutes; no validation is removed or bypassed.
See [support](COMPATIBILITY.md#genuine-player-pistol-combat) and
[source arithmetic/harness limits](M3G7_REFERENCE.md).

## Original player weapon expansion

```sh
python3 tools/reference/evidence_pack.py
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3g8_reference.py --check --overflow-check --address-check
python3 tools/reference/m3g8_snapshot_goldens.py --check
cargo test -p doom-sim --test player_weapon_pack --locked
python3 tools/reference/check_player_weapon_pack.py
cargo run -p doom-sim --example player_weapon_pack --locked
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_player_weapon_pack_fuzz.py /tmp/player-weapon-pack-seeds
# From fuzz/; bounded example, not exhaustive coverage or leak detection:
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_weapon_pack /tmp/player-weapon-pack-seeds -- -seed=381 -runs=4096 -max_len=162 -timeout=15 -rss_limit_mb=1024
python3 tools/measure_player_weapons.py
```

The asserted example executes source command streams, not injected damage.
New DREFLZ1 test storage is reproducible with standard-library Python and a bounded
safe Rust test decoder. For inspection:

```sh
python3 tools/reference/evidence_pack.py --unpack tests/reference/m3g8_trace.dref --output /tmp/m3g8-trace.csv
```

Expected source fields/events and independent DSIM24 fixtures precede runtime
integration. Historical tools and expectations remain unchanged. All historical
checks above still apply. See [source qualification](M3G8_REFERENCE_INVESTIGATION.md)
and [observed results](validation/M3G8.md); source observation totals and actual
Rust-admitted comparison totals are deliberately reported separately.


## Original world pickups

```sh
ASAN_OPTIONS=detect_leaks=0 python3 tools/reference/m3g9_reference.py --check --overflow-check --address-check
python3 tools/reference/m3g9_snapshot_goldens.py --check
cargo test -p doom-sim --lib world_pickup_tests --locked
python3 tools/reference/check_player_world_pickups.py
cargo run -p doom-sim --example player_world_pickups --locked
cargo +nightly fuzz build --sanitizer address
python3 tools/seed_player_world_pickups_fuzz.py /tmp/player-world-pickup-seeds
# From fuzz/; bounded execution, leak detection disabled:
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run player_world_pickups /tmp/player-world-pickup-seeds -- -seed=391 -runs=4096 -max_len=162 -timeout=15 -rss_limit_mb=1024
python3 tools/measure_player_world_pickups.py
```

All historical commands remain required. The new observer compiles actual source
GiveAmmo/GiveBody/GiveWeapon/TouchSpecialThing and reuses existing original map,
movement, player, weapon, damage and AI bodies. Source observation rows and
independent DSIM25 bytes precede matching runtime integration. The asserted
example uses fixed source command streams, actual world contacts and autonomous
combat; it never injects a bullet/melee hit to stand in for gameplay. Controlled
health/loadout/dropped setup cases remain explicitly separate. Direct vertical
components do not admit airborne physics. Source-only item-respawn queue and
geometry trace rows remain qualified observations.

The focused fuzz target exercises commands, contacts, resource/lifetime/provenance
boundaries, invalid operations and DSIM25 with exact advance/step equality and
full-state rejection checks. Record built executables separately from campaigns
run. ASAN_OPTIONS disables leak detection; bounded campaigns do not prove
exhaustive coverage. Benchmarks retain three timing/RSS and separate allocation
processes, frozen control workloads and independently labeled pickup intervals.
See [source matrix](M3G9_REFERENCE_INVESTIGATION.md) and
[observed validation](validation/M3G9.md).
