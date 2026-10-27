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
