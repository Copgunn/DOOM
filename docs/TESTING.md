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

## Fuzzing

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

M2 adds canonical per-tic state records and ordered command streams. Compare
repeat runs, debug/release, and platforms; perturb presentation schedules without
changing commands. M3/M4 add pinned reference builds and version-specific demo
replays, recording legal data hashes and first divergent tic/field. Performance
checks must state environment, profile, workload, and a correctness baseline.
