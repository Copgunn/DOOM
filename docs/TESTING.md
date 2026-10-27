# Testing and fixture guide

Run the commands in [README](../README.md#build-and-test) before opening a PR.
CI runs them on stable Linux, Windows, and macOS, and runs tests on Rust 1.85.0.
Documentation builds also deny rustdoc warnings in CI. No proprietary data or
original C build is required.

## Current coverage

- `crates/doom-types/src/lib.rs`: unit regressions for lossless eight-byte lump
  names, non-text bytes, and distinction between raw equality and lookup policy.
- `crates/doom-wad/tests/signature.rs`: public-API integration tests for exact
  IWAD/PWAD magic and invalid signatures using generated empty-WAD bytes.
- `crates/doom-wad/tests/common/mod.rs`: fixture helper, deliberately below
  `tests/common/` so Cargo does not create a separate empty test binary for it.
- `doom-map`: value vocabulary only; no parser behavior to test yet.
- `crates/doom-wad/tests/private_iwad.rs`: ignored-by-default, read-only external
  IWAD signature probe using `DOOM_IWAD_DIR`. It is excluded from public CI and
  does not establish archive validity, authenticity, or compatibility.

The fixture helper emits a complete 12-byte empty archive layout, but current
tests inspect only its magic. Directory bounds, truncated headers, malformed
offsets, map records, simulation, and replay are **not tested or implemented**.
Do not describe these tests as a WAD parser suite. See M1 for that acceptance gate.

## Extending the suite

Put narrow invariant/edge tests next to implementation in `#[cfg(test)]` modules.
Place public API and cross-crate workflows under the consuming crate's `tests/`.
Generate small byte fixtures with explicit little-endian writes; extend the WAD
helper with directory/lump builders when the parser exists. Pair each malformed
case with the relevant error class and ensure the public API returns an error
without panic. Keep regressions independent of local file paths and OS ordering.

Prefer generated fixtures over checked-in binaries. Any static fixture needs a
nearby provenance record with creator/source, license, purpose, expected behavior,
and hash where appropriate. Never copy commercial IWADs or extracted assets.
There are no placeholder fixture directories, fuzz targets, replay tests, or
benchmark results in M0.

For separately authorized private testing, set `DOOM_IWAD_DIR` in a private shell
to an external directory and run `cargo test -p doom-wad --test private_iwad --
--ignored`. Provision assets only into an explicitly authorized environment;
connected storage access alone does not authorize cloud copying. Keep originals
unchanged. Do not print private filenames, paths, links, contents, or credentials
in public output, and do not upload private fixtures or results as CI artifacts.
Future parser/replay suites should keep private tests opt-in and public fixtures
synthetic/redistributable. The ignored probe must not be reported as a successful
real-asset test unless it was actually run with authorized assets.

M1 adds parser fuzz targets in a dedicated `fuzz/` package kept out of the stable
workspace when its tooling requires nightly. Bound allocations and execution,
record reproducible invocations and seeds, and promote minimized failures to
ordinary unit/integration tests. Stable CI must remain useful without fuzz tools.

M2 adds ordered command streams and canonical per-tic state records. Compare
repeat runs, debug/release, and platforms; perturb rendering schedules without
changing commands. M3/M4 add pinned reference builds and version-specific demo
replays, recording data hashes and the first differing tic/field. Comparison
fixtures and reference setup must be legally distributable and reproducible.
Avoid timing-sensitive assertions; later performance tests must state environment,
build profile, workload, and correctness baseline.
