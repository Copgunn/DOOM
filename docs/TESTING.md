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
```

This runs all 17 historical/new generator and independent-encoder pairs with
`--check`. It does not regenerate expectations to match Rust. For the new slice:

```sh
python3 tools/reference/generate_m3d3j.py --check
python3 tools/reference/m3d3j_snapshot_goldens.py --check
cargo test -p doom-sim --test m3d3j_reference --test m3d3j_policies --locked
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
```

The helper compares debug/release output for all 17 simulation examples. They span
movement, collision, sectors, lifecycle, combat, weapons/projectiles, perception,
pursuit, selected attacks and Demon gameplay/map context. Assertions use committed
original-C evidence and independent snapshots where applicable. Earlier examples
and equality helpers retain their historical bytes.

## Fuzzing and sanitizers

Install cargo-fuzz and a supported nightly Rust toolchain, then:

```sh
cargo +nightly fuzz build --sanitizer address
cargo +nightly fuzz run demon_map_context --sanitizer address -- -runs=4096 -max_len=65 -timeout=20 -rss_limit_mb=2048
```

The build retains all 15 previous targets and adds the bounded map-context target.
It mutates public synthetic maps and limits operations/tics, testing roles,
identity, damage, reset, capacity and complete-state rejection. Local sanitizer
results and exact model/seed/options are recorded in [M3d3j evidence](validation/M3D3J.md).
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
