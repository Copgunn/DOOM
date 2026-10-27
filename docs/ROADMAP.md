# Roadmap

Milestones are accepted using recorded evidence, not the presence of placeholder
APIs or empty tests. “Implemented” describes branch content; owner approval and
merge remain separate gates. See [validation evidence](VALIDATION.md).

## M0 — Foundation

Status: **foundation implemented and validated**.
Historical implementation and CI evidence: [PR #1](https://github.com/Copgunn/DOOM/pull/1).
No later milestone is complete.

Acceptance:

- [x] Three-crate Rust workspace alongside original source; minimal dependencies.
- [x] Safe foundational vocabulary with explicit unimplemented-feature boundaries.
- [x] Least-privilege Linux/Windows/macOS CI and a minimum compiler job configured.
- [x] README, architecture, compatibility, testing, contribution, and agent guidance.
- [x] Unit/integration structure and generated/invalid-signature fixtures.
- [x] Record successful compilation, formatting, Clippy, tests, and documentation.
- [x] Verify Linux/Windows/macOS stable and Rust 1.85.0 CI; evidence in validation notes.
- [x] Push focused commits and link an open PR against `master` for owner review.
- [x] Record preservation of upstream source, attribution, license, and history.

## M1 — WAD and map parsing

Status: planned. Recommended next assignment: implement bounded archive reading
and classic binary map decoding in the existing crates, without simulation or
rendering scope.

Acceptance:

1. Parse IWAD/PWAD headers and signed little-endian directory fields with checked
   arithmetic, configurable resource limits, and errors for truncation, negative
   fields, out-of-bounds regions, and excessive counts before allocation/access.
   Specify and test zero-size markers and overlapping/shared lump ranges.
2. Provide stable directory enumeration and tested reference-compatible name
   lookup; later matching lumps/archives override earlier ones without losing
   original order or raw names. Cover exact eight-byte names and padding/case.
3. Decode classic THINGS, LINEDEFS, SIDEDEFS, VERTEXES, SEGS, SSECTORS, NODES,
   SECTORS, REJECT, and BLOCKMAP under documented E#M#/MAP## grouping rules.
   Reject unsupported map formats explicitly. Validate record widths, sidedef
   sentinels, vertex/sector/line/seg/subsector references, BSP children/cycles,
   REJECT size, and BLOCKMAP offsets/termination before traversal.
4. Integration fixtures include an empty archive, duplicate names, archive patch
   precedence, a complete minimal synthetic map, boundary values, and a malformed
   case for every error class. Parser entry points return errors rather than
   panic, even on arbitrary bytes. Include structured-error context.
5. Add runnable WAD/map fuzz targets, seed corpus provenance, resource limits,
   and a documented smoke command; minimize failures into ordinary regressions.
6. Run all workspace checks and CI without proprietary data. Update README,
   architecture, compatibility decisions, and test guide with actual behavior.

## M2 — Deterministic simulation core

Status: planned.

Acceptance: explicit vanilla fixed-point semantics and edge tests; reference RNG
table/sequence/reset tests; validated actor handles and deterministic thinker
insertion/removal scheduling; movement and collision with fixtures; canonical
per-tic state comparisons. Identical input must repeat across debug/release,
supported platforms, and differing presentation schedules. Record ambiguous C
semantics and safe deviations.

## M3 — Vanilla gameplay

Status: planned.

Acceptance: player and monster behavior, weapons, interactions, and map specials
with focused regression fixtures; deterministic reference comparisons covering
each behavior; field-level first-divergence diagnostics. Document tested scenarios
and remaining gaps rather than claiming all vanilla behavior from a smoke test.

## M4 — Demo compatibility

Status: planned.

Acceptance: pinned reference demos with legal provenance and version-specific
header handling; complete deterministic playback; compatibility regression suite
and desynchronization reports. Publish guarantees per reference version. Treat
DOS 1.9 evidence separately from Linux 1.10/demo version 110.

## M5 — Rendering and platform integration

Status: planned.

Acceptance: renderer abstraction, presentation loop, per-tic input assignment,
audio events/playback, software/headless diagnostic backend, and Vulkan hardware
renderer. Verify Linux/Windows/macOS presentation where supported by the selected
Vulkan platform path; document prerequisites and gaps. Replay tests must prove
presentation frame rate and renderer choice do not alter simulation results.
