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

The frozen historical helper plus the new pair cover every committed version
with `--check`.
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
```

The frozen historical helper and new map-scale helper together compare debug/release
output for every committed simulation example. They span
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
