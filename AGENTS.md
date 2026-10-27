# Standing engineering instructions

These rules apply to this repository and future Codex development sessions.

## Scope, safety, and compatibility

- This is the public `Copgunn/DOOM` independent Rust rewrite. Never access,
  copy from, modify, publish, or otherwise involve private `Copgunn/UZDoom`.
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
  implemented functionality explicit; M0 is not a playable engine or WAD parser.
