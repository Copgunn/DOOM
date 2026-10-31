# Historical evidence storage

Historical gameplay expectations retain their original byte interpretation.
Storage compression changes how selected source traces reach test parsers; it
does not regenerate them, change DSIM, or admit gameplay. Reproduction commands
belong to [TESTING](TESTING.md#historical-zstd-evidence).

## Selected raw traces

The [before inventory](validation/zstd-baseline.json) records the original Git
blob, size and SHA-256 before transformation. The
[migration manifest](../tests/reference/zstd_manifest.json) identifies the matching
standard-zstd frame, both lengths/hashes, source family and decode budget.
Nineteen CSV traces of at least 1 MiB were selected. Smaller readable tables,
source/scenario inputs, independent encoders, full-byte DSIM fixtures, archived
records and existing DREFLZ1 streams remain in their established format.

Each raw file was read completely, checked against the pinned baseline Git object,
encoded and decoded, then compared byte for byte. Removal happened only after
all counterparts passed. The [migration record](validation/zstd-migration-proof.json)
is accompanied by a reproducible Git-object check, not trusted as proof of itself.
This reduces current-checkout payload bytes. Existing commits still contain the
raw data; Git history has not shrunk or been rewritten.

## Bounded offline loading

The isolated `doom-test-evidence` crate is a doom-sim **dev dependency** and a
tooling/fuzz dependency. Production stepping has no codec dependency. Its pinned
encoder uses zstd crate 0.13.3, zstd-safe 7.3.0 and bundled libzstd 1.5.7. Exact
parameters are manifest data. The decoder needs no external zstd executable or
online service. Dependencies are installed by normal Cargo builds; tests then
use only committed evidence. [Dependency metadata](validation/zstd-dependencies.json)
records versions, license alternatives and declared MSRVs. Native C/FFI remains
inside the external library; no unsafe Rust was added to this repository.

The contract accepts one standard, checksummed, size-bearing frame, no dictionary,
no extra frames or trailing bytes. Bounds are 32 MiB decoded per entry, 8 MiB
compressed per entry, 8 MiB decode window, 96 MiB aggregate decode budgets,
32 MiB aggregate compressed lengths, 64 entries and 128 KiB of manifest JSON.
Checked header/window/size validation precedes output allocation and native
decoder creation. Fixed bounded output, exact input consumption, exact output
length and separate compressed/decoded SHA-256 checks precede UTF-8 admission.
Manifest paths are limited to simple `tests/reference/*.csv` names and their
`.zst` counterparts; duplicate/unsorted entries and unknown fields reject.
Filesystem loading also checks canonical root containment.

An immutable, bounded test-process cache preserves the former `&'static str`
consumer interface without leaking allocations. It owns decoded text and checks
manifest/input identity; it owns no game actors, IDs or RNG. Dynamic malformed
input and fuzz tests use checked uncached APIs. Test assertions report a failed
load normally; public codec APIs return errors.

## Frozen observers and preservation

Historical Python observers and independent encoders remain byte-identical.
The wrapper copies their source, inputs and supporting files to an isolated
temporary workspace, restores decoded raw paths there, then executes the exact
65 previous CI source commands with their original flags. It never hardlinks
mutable files or generates expectations from Rust. A complete suite shares one
workspace to avoid repeated fixture copying. This requires temporary disk space
for the historical inputs in addition to the compressed checkout.

The explicit Git-baseline verifier checks every migrated byte and protects all
other historical source/evidence/archive paths. Only the enumerated Rust
include-loading substitutions and the active reference README are exceptions.
The original 165 upstream paths are included. CI explicitly fetches the pinned
baseline object for this separate proof; ordinary tests require no Git history
and never fetch it. Archive hash/link checks need no exception.

[Engineering observations](validation/ZSTD_WEAPON_MAINTENANCE.md) distinguish
migration, corruption/safety checks, independent C comparisons and performance.
