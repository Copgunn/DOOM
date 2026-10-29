# Standing engineering instructions

## Scope and source authority

- Work only within this repository and explicitly authorized resources. Do not
  access unrelated private repositories, proprietary assets, private cloud
  storage, credentials or confidential files. Public tests use synthetic or
  legally redistributable fixtures; commercial IWADs never enter Git, CI or logs.
- Preserve all 165 original upstream files, original history, attribution,
  `README.TXT` and `LICENSE.TXT`. Keep the repository public and GPL-2.0-only.
  Never import GPLv3-only implementation code.
- Original id Software Linux DOOM 1.10 at
  `a77dfb96cb91780ca334d0d4cfd86957558007e0` is behavioral authority. Read the
  relevant source investigation before changing an admitted behavior. Metadata
  recognition, executable admission and full compatibility are distinct claims.

## Architecture and determinism

- Prefer idiomatic safe Rust and explicit ownership. Workspace lints forbid
  unsafe code; exceptions require a separately reviewed policy and safety proof.
- Preserve 35 logical tics/second, fixed-point/integer semantics, original RNG
  ordering and deterministic actor/thinker/spatial order. No gameplay-critical
  unordered iteration, floating-point substitution or wall-clock dependency.
- Keep simulation separate from parsing, I/O, audio and future rendering.
  Presentation reads must not consume gameplay RNG or change outcomes.
- Validate offsets, sizes, counts, arithmetic, actor roles and generational
  references. Never sort handles to select blockers or rebuild source BLOCKMAP.
- Recoverable simulation errors preserve complete canonical state. Preflight
  bounded work before mutation; no full-simulation clone per tic. General user
  thinker budgets retain their separately documented nontransactional contract.
- Reuse actor storage, geometry, state entry, shared damage and lifecycle services.
  Unsupported actions must reject explicitly; callbacks must never silently no-op.
- Keep every historical profile and DSIM-v1–v16 expectation unchanged. New behavior
  selects profile17/DSIM-v17 explicitly. Preserve strict native/target roles and
  existing admission/resource bounds; see [compatibility](docs/COMPATIBILITY.md).
  Source-compatible original state entry is immediate; raw spawn has no action.

## Validation

- Run build, format (including fuzz workspace), Clippy with warnings denied,
  workspace tests, release simulation/type tests and warnings-denied rustdoc.
  Verify Rust 1.85.0 and exact-head Linux, Windows, native macOS and MSRV CI.
  Preserve `macos-26-intel`, required check names and existing validation steps.
- Run every original-C generator and independent DSIM encoder with `--check`,
  all historical/new asserted examples in debug/release, all fuzz builds and a
  bounded relevant sanitizer campaign. Commands are in [testing](docs/TESTING.md).
- Expected C traces come from genuine source bodies and declared adapters;
  canonical expectations come from independent encoders, never Rust output.
  Preserve historical fixtures, helper bytes, provenance and qualified evidence.
- Distinguish source equivalence, explicit fixture projection, independent DSIM
  consistency and Rust-only safety tests. Report actual counts, sanitizer options,
  findings and limitations. Disabled leak detection is not LeakSanitizer coverage.

## Branches, review and documentation

- Inspect status, master and open PRs first. Implement on focused feature branches,
  never directly on master; preserve unrelated changes and shared history.
- Do not force-push, squash/rebase-rewrite shared history, delete active/unmerged
  branches or enable auto-merge. Merges require explicit owner authorization and
  every applicable fresh local/exact-head gate; verify post-merge CI when required.
- Open/update a focused PR with behavior, evidence, restrictions and unresolved
  issues. Pending or blocked delivery stays draft. Do not claim unobserved passes.
- Maintain human-readable README, architecture, compatibility, roadmap, testing,
  source investigation and schema documentation. Use the
  [documentation review checklist](docs/DOC_REVIEW_CHECKLIST.md). Archive historical
  evidence with digests and working links instead of deleting or rewriting it.
- Keep the prominent machine-maintained/human-directed disclosure accurate. Do not
  imply universal manual review, absent human oversight or guaranteed correctness.
- Stop at the assigned milestone's owner-review gate; no unrelated systems.
