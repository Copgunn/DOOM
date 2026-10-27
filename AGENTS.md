# Standing engineering instructions

These rules apply to this repository and future Codex development sessions.

## Scope, safety, and compatibility

- This is the public `Copgunn/DOOM` independent Rust rewrite. Never access,
  copy from, modify, publish, or otherwise involve private `Copgunn/UZDoom` or `Copgunn/Gallowmark`.
- Preserve upstream history, original source/attribution, `README.TXT`, and
  `LICENSE.TXT`. Use Linux DOOM 1.10 at the documented upstream commit as the
  authoritative behavioral reference; do not mechanically translate C architecture.
- Prefer correctness over optimization and idiomatic safe Rust with explicit
  ownership. Current workspace lints forbid unsafe code; any future exception
  requires a reviewed policy change and documented safety justification/invariants.
- Deterministic simulation is non-negotiable: exactly 35 logical tics/second,
  explicit fixed-point/integer semantics, original RNG sequencing, deterministic
  actor/thinker order, and no gameplay-critical unordered iteration. Separate
  simulation, input, I/O, audio, and rendering. Rendering cannot change outcomes.
- Validate all input offsets, sizes, counts, indexes, and references. Future actor
  relationships use validated handles/stable identifiers rather than raw pointers.
- Linux 1.10 and DOS 1.9 compatibility are distinct objectives. Demo version 110
  is not proof of DOS compatibility. Record findings in `docs/COMPATIBILITY.md`.
- Never commit commercial assets, credentials, private content, or build outputs.
- Keep the repository public and GPL-2.0 licensed. Private IWADs stay external
  via `DOOM_IWAD_DIR`; never alter source originals or upload assets to GitHub
  branches, PRs, releases, or CI artifacts. Connected storage access does not
  authorize copying into a cloud workspace. Obtain explicit destination consent.
  Keep asset contents, private filenames/locations, storage links, credentials,
  and private test output out of public logs, screenshots, snapshots, and docs.
  Public CI uses synthetic/redistributable fixtures; private tests stay opt-in.

## Branch management

- Inspect repository status, `master`, existing branches, and open PRs before
  implementation. Reconcile new work without overwriting unrelated changes.
- Never implement directly on `master`. Create focused feature/fix branches with
  descriptive names (`feat/`, `fix/`, `docs/`, `chore/`).
- Keep PRs reasonably small and independently reviewable, with logical commits.
- Avoid force-pushing shared branches. Do not delete unmerged or active branches.
- Identify stale/completed branches using evidence and recommend cleanup when
  appropriate; do not assume inactivity establishes safe deletion.

## Pull request management

- Create a PR for each meaningful milestone. Review existing PRs before creating
  duplicates; update the existing PR as requirements evolve.
- Include purpose, implementation, tests, compatibility implications,
  documentation changes, and unresolved risks. Link roadmap acceptance evidence.
- Run and verify CI before requesting review. Keep pending/blocked delivery draft;
  report exact external toolchain, permission, or configuration limitations.
- Do not merge without explicit owner authorization. Do not enable auto-merge.

## Checks and tests

Use the root stable toolchain file. Required checks:

```sh
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked
```

CI covers Linux, Windows, and macOS, denies rustdoc warnings, and tests the Rust
1.85.0 compiler floor. Preserve that floor or deliberately document and test a
change. Keep dependencies minimal and justified. Add meaningful tests for new
behavior and regressions using `docs/TESTING.md`; do not add empty scaffolding or
assertions that merely create the appearance of coverage. CI requires no IWAD.
Never claim unverified tests, benchmarks, commits, branches, or features.

## Documentation maintenance

- Update README when user-facing behavior or project status changes.
- Update `docs/ARCHITECTURE.md` when boundaries/design decisions change.
- Update `docs/ROADMAP.md` with milestone progress and verifiable acceptance evidence.
- Document important compatibility findings in `docs/COMPATIBILITY.md`.
- Include relevant documentation updates in feature PRs. Keep planned and
  implemented functionality explicit. M1 parses resources/maps; M2 provides a
  narrow headless simulation profile; M3a adds player/solid XY and M3b adds
  static-sector BSP/openings/XY/Z. Complete player/view/actor states, dynamic
  sectors, combat, specials, demos and rendering remain incomplete. Preserve
  all M2/v1, M3a/v2 and M3b/v3 suites; read their source investigations before
  changing movement.
  Preserve tested policies in `docs/COMPATIBILITY.md` and the versioned canonical
  format in `docs/STATE_FORMAT.md`. Read source findings in `docs/M2_REFERENCE.md`.
- Keep C-derived vectors separate from project regression consistency. Never
  claim complete vanilla/DOS synchronization from isolated comparisons or hashes.
  Run deterministic arithmetic/simulation tests in debug and release; preserve
  cross-platform golden checks. Intentional reference/schema changes require
  updated provenance, documented decisions, and reviewed fixture regeneration.

- Simulation recoverable step errors must preserve full canonical state. Validate
  setup invariants before mutation; no automatic full-state clone per tic. Generic
  user thinker budgets have a separate documented nontransactional contract.
- Actor identity, thinker order and spatial order are separate. Never sort handles
  to choose blockers; successful same-cell moves reinsert at spatial cell head.
- Run all three reference generators and independent golden encoders on Linux;
  cross-platform CI compares recorded vectors/goldens and debug/release examples.
- M3b stops at owner review. M3a merge authorization does not authorize M3b merge.

- M3b consumes supplied BLOCKMAP lists without rebuilding them; conservative
  omissions are diagnostic, not proof of spatial correctness. Keep older profiles
  unchanged. Sector-head and cell-head order derive from explicit relinking.
- Keep future custom wgpu rendering separate, preferring Vulkan where supported.
  No renderer, Bevy, audio or general physics dependency enters authoritative tics.
