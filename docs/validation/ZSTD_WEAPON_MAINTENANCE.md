# Zstd evidence and weapon formula maintenance

This maintenance has two independent acceptance gates. The immutable baseline
inventory is [zstd-baseline.json](zstd-baseline.json), captured from actual merged
master before transformation. Original bytes were compared to Git objects.

## Design before implementation

Package A selects 19 raw CSV traces of at least 1 MiB (88,469,558 bytes).
Original source, encoders, scenario inputs, complete DSIM fixtures, archives and
existing DREFLZ1 evidence remain unchanged. A pinned standard-zstd encoder and
bounded test-only decoder will verify size, checksum, SHA-256, exact frame
consumption and actual byte equality before removal. A separate Git-baseline
proof will preserve independent verification after migration. Historical Python
observers run unchanged in an isolated workspace with decoded original paths.

The decoder dependency is isolated from production simulation. zstd 0.13.3 with
bundled libzstd 1.5.7 compiled successfully on Rust 1.85; the evaluated ruzstd
0.8.2 failed its Rust 1.85 build (unsigned_is_multiple_of). This does not imply
that every historical ruzstd release is incompatible. Dependency licenses and
transitive MSRVs will be recorded with final validation.

Package B will route historical preview/commit and genuine-player execution
through shared source-derived bullet, unpowered-punch and spread helpers.
Damage precedes ordered left/right spread draws. Accurate bullets omit spread.
The original negative signed shift is qualified by the existing explicit
32-bit wrapping interpretation; portable ISO-C equivalence is not claimed.
No gameplay policy, diagnostic version, actor capacity or supported branch is
expanded. Each package must pass independently before combined acceptance.

## Baseline and merge gate

PR #39 was reviewed and merged with two ordered parents. The exact-master Rust
workflow passed Linux, Windows, native Intel macOS and Rust 1.85. The first macOS
attempt was cancelled; a normal macOS-only retry succeeded on the unchanged
commit. Exact commits, checkout proofs and CI live in GitHub. The baseline
inventory identifies the immutable object used for migration verification.

Quiet baseline measurements are retained separately and will be published with
matched after samples. No maintenance completion is asserted by this design.
