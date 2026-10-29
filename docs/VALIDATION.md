# Validation guide

Compatibility claims are limited to independently demonstrated input slices.
A passing test suite or matching digest does not prove a complete game, original
savegame compatibility or vanilla demo synchronization.

## Evidence families

| Evidence | What it establishes | Main records |
| --- | --- | --- |
| Genuine original C | Source decisions, fields, RNG and ordering under documented adapters | Milestone REFERENCE notes and `tests/reference/*provenance.json` |
| Fixture projection | Explicit native identity/state adaptations compared with original C | Source investigations and adapter assumptions |
| Independent DSIM | Complete bytes/per-tic digests from C observations plus documented schema; no Rust expected-output generation | [State format](STATE_FORMAT.md), independent Python encoders |
| Rust safety policies | Checked arithmetic, bounds, atomic rejection, stale handles and independent instances | Policy/unit tests, [compatibility](COMPATIBILITY.md) |
| Platform/toolchain | Recorded checks execute on supported hosts and Rust versions | Exact-head GitHub Actions, [testing](TESTING.md) |

Compiled source-function inventory is not branch coverage. Setup fragments are
reported separately from whole functions. Unadapted C, declared fixture projection,
canonical consistency and Rust-specific rejection are never interchangeable claims.

## Reproduce

[TESTING](TESTING.md) owns build, format, Clippy, workspace/release, rustdoc/MSRV,
reference/encoder, example, documentation and fuzz reproduction commands.
Cross-platform CI requires Linux, Windows, native macOS and
Rust 1.85.0. Original-C regeneration runs on Linux; other hosts compare recorded
observations and independent canonical fixtures.

Sanitizer reports name the actual target, corpus/input model, execution count,
limits, findings and leak setting. Disabled leak detection does not provide
LeakSanitizer coverage; no bounded campaign is exhaustive.

## Detailed records

- [M3f3 combined Demon/expanded-map evidence](validation/M3F3.md).

- [M3f2 expanded-coordinate/spatial-work evidence](validation/M3F2.md).
- [M3f1 map-scale source comparisons and host measurements](validation/M3F1.md).

- [M3d3j map-context implementation and delivery evidence](validation/M3D3J.md).
- [M3d3j original-source investigation](M3D3J_REFERENCE.md).
- [Reference-tool reproduction and provenance](../tools/reference/README.md).
- [All historical engineering records](validation/archive/README.md), including
  the complete [pre-M3d3j ledger](validation/archive/pre-m3d3j-ledger.md).

## Historical evidence preservation

The old validation ledger is retained byte-for-byte in the
[archive](validation/archive/README.md). Its readable Markdown copy rebases links. The
[manifest](validation/archive/manifest.json) records source commit, hashes, sizes,
archive destinations and every original heading. Other former guides are also
archived intact, preserving detailed restrictions and evidence.

Historical SHA/CI references, source hashes, qualified counts and sanitizer limits
remain historical observations. Moving them does not assert they were rerun on a
new commit. Actual generators, encoders, fixtures and historical helper bytes remain
at their existing paths. The [documentation checklist](DOC_REVIEW_CHECKLIST.md)
governs evidence and link review.

## Keep observations separate from current status

Detailed measured results belong in milestone records, not this guide or other
entry points. ROADMAP owns the development summary; GitHub PRs/Actions own exact
review commits, statuses and CI history. Source investigations and frozen archives
describe their original observation context. Follow the
[authority map](README.md#documentation-authority) rather than copying those facts.
