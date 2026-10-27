# Roadmap

Milestones are accepted using recorded evidence, not the presence of placeholder
APIs or empty tests. “Implemented” describes branch content; owner approval and
merge remain separate gates. See [validation evidence](VALIDATION.md).

## M0 — Foundation

Status: **foundation implemented and validated**.
Historical implementation and CI evidence: [PR #1](https://github.com/Copgunn/DOOM/pull/1).
Merged by owner-authorized regular merge commit
`50a8f64ccc71cdd5e52a7e491aff985dd56d2902` after fresh correction-head CI.

Acceptance:

- [x] Three-crate Rust workspace alongside original source; minimal dependencies.
- [x] Safe foundational vocabulary with explicit unimplemented-feature boundaries.
- [x] Least-privilege Linux/Windows/macOS CI and a minimum compiler job configured.
- [x] README, architecture, compatibility, testing, contribution, and agent guidance.
- [x] Unit/integration structure and generated/invalid-signature fixtures.
- [x] Record successful compilation, formatting, Clippy, tests, and documentation.
- [x] Verify Linux/Windows/macOS stable and Rust 1.85.0 CI; evidence in validation notes.
- [x] Deliver focused commits through PR #1 and the authorized regular merge.
- [x] Record preservation of upstream source, attribution, license, and history.

## M1 — WAD and map parsing

Status: **implemented, validated and merged**.
Historical delivery: [PR #2](https://github.com/Copgunn/DOOM/pull/2), regular merge
`de78b72ee0a005ee737fb126abbf656db7639812`. Corrected-head and post-merge CI passed.
No simulation, rendering, audio, or gameplay is included. The delivery PR and
exact remote checks are recorded in [validation evidence](VALIDATION.md).

Implementation checklist:

- [x] Bounded IWAD/PWAD reader, typed errors, configurable resource limits.
- [x] Raw names/order, ASCII-insensitive last-match lookup and archive overrides.
- [x] Empty/shared/overlapping lump policies and safe indexed byte retrieval.
- [x] All ten classic map lumps, markers, immutable records and validated references.
- [x] Bounded BSP/subsector/BLOCKMAP checks, REJECT coverage, unsupported formats.
- [x] Generated complete map, malformed-input and adversarial-work regressions.
- [x] Runnable WAD/map fuzz targets, original seed provenance and smoke runs.
- [x] Stable and Rust 1.85 local checks, API example and documentation updates.
- [x] Verify remote Linux/Windows/macOS and Rust 1.85 checks; evidence in validation notes.

The acceptance requirements below describe scope; evidence and documented
limitations are in validation, compatibility, and testing notes. Public CI is
independent of proprietary assets and nightly. Real IWAD tests remain opt-in and
unrun in this cloud environment.

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

Status: **foundations implemented, validated and merged**.
[PR #3](https://github.com/Copgunn/DOOM/pull/3) merged by owner-authorized regular
commit `10a1106cecfaec85026f0724dee13a3bf6174560`. Fresh correction-head and
post-merge CI passed; exact evidence is in [validation notes](VALIDATION.md).

- [x] Source investigation before implementation; defined policies for C ambiguities.
- [x] i32 16.16 arithmetic, original division guard/double helper, extreme-value policy.
- [x] Exact owned RNG streams with full-cycle/reset/separation reference tests.
- [x] Safe store-scoped generational actors, deterministic reuse and separate order.
- [x] Append-sensitive scheduling, deferred retirement, pause and phase-order tests.
- [x] One-sector grounded non-player XY subset, original line/BLOCKMAP predicates.
- [x] Source-table ground thrust/angle foundations; unsupported buttons fail.
- [x] Canonical DSIM-v1 fields, hashes and first-divergence diagnostics.
- [x] Headless runner, C vectors, independent snapshot goldens and 768-call regressions.
- [x] Local debug/release/Rust 1.85 checks, parser regressions and fuzz smoke reruns.
- [x] README, architecture, compatibility, format, test and validation documentation.
- [x] Verify Linux/Windows/macOS debug/release CI and Rust 1.85; evidence in validation notes.

M2 excludes full players, sliding, actor collisions, vertical motion, portals/sector
changes, state machines, monsters/weapons/specials and gameplay respawn. M3a
adds the explicit player-collision profile below while preserving M2 regressions.
This scoped foundation does not establish complete vanilla or DOS synchronization.
Evidence boundaries and remaining mechanics are explicit in compatibility notes.

## M3 — Vanilla gameplay

Status: **M3a implemented and validated; owner-review gate pending**.
Delivery: [PR #4](https://github.com/Copgunn/DOOM/pull/4).
Branch: `feat/m3a-player-movement-collision`. Never merge without separate approval.
M3 as a whole remains incomplete.

| Phase | Scope | Status |
| --- | --- | --- |
| M3a | Grounded player XY, wall sliding, solid actor collision | Implemented/validated; owner review pending |
| M3b | Multi-sector movement, portals, Z and gravity | Next planned phase |
| M3c | Actor states, type-driven spawning and death | Planned |
| M3d | Weapons, projectiles, damage, monsters and combat | Planned |
| M3e | Doors, lifts, triggers, sector specials and integration | Planned |

M3a acceptance evidence:

- [x] Source investigation before runtime changes; isolated C adapter/provenance.
- [x] Explicit player/non-player response and zero/MF_SOLID participants.
- [x] Original three-corner traces, fixed DDA/intercepts, angled projection,
  source tie ordering, retry bounds and Y-before-X fallback.
- [x] Things-before-lines collision, strict square overlap, cell-head relinking,
  immediate spatial removal and generation-safe reuse.
- [x] Validate invariant setup; recoverable failed steps preserve canonical state.
- [x] Bounded geometry, coverage/degenerate rejection, shared-list stress, raw
  extremes, duplicate references, overlap and lifecycle regressions.
- [x] Preserve M1/M2 acceptance tests and all original M2 reference/golden fixtures.
- [x] DSIM-v2 alongside frozen v1, independent encoding goldens, 768-call traces.
- [x] 252 original C-reference tics compare results and ordered collision evidence.
- [x] Asserted headless bevel/solid arena; debug/release stdout equality.
- [x] Local workspace, release, Rust 1.85 and parser fuzz checks passed.
- [x] Initial cross-platform debug/release and Rust 1.85 CI passed on `c6b63f5`;
  final exact-head CI is verified on PR #4 before review readiness.
- [ ] Owner review; merge is not authorized by the M2 approval.

See [M3a scope and safe policies](M3A_REFERENCE.md), [state schema](STATE_FORMAT.md)
and [validation](VALIDATION.md). A flat static movement subset is not full vanilla
player thinking, gameplay synchronization, demo playback or DOS compatibility.

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
