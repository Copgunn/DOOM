# Contributing

Read [AGENTS.md](AGENTS.md), [architecture](docs/ARCHITECTURE.md),
[compatibility](docs/COMPATIBILITY.md), and [roadmap](docs/ROADMAP.md) first.
This is an independent Rust rewrite using the preserved Linux DOOM 1.10 source
as the behavioral reference. Use idiomatic Rust rather than mechanical C translation.

## Setup and checks

Install rustup, clone this repository, and work from its root. The checked-in
toolchain file installs stable with rustfmt/Clippy; Rust 1.85 is the minimum
supported compiler. No commercial data, C compiler, Vulkan SDK, or additional
libraries are required for the stable Rust workspace. Optional fuzzing requires
nightly, cargo-fuzz, and a host C++ compiler; see `docs/TESTING.md`.

```sh
git switch master
git pull --ff-only
git switch -c feat/short-purpose
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p doom-sim -p doom-types --release --locked
cargo doc --workspace --no-deps --locked
```

Use `cargo fmt --all` to apply formatting. To check the compiler floor, install
`rustup toolchain install 1.85.0 --profile minimal` and run
`cargo +1.85.0 test --workspace --locked`. CI additionally denies rustdoc warnings.
Commit intentional lockfile changes. See [TESTING.md](docs/TESTING.md) for fixture,
regression, fuzzing, simulation, and reference-comparison conventions.

## Branches and pull requests

Never implement directly on `master`. Use descriptive `feat/`, `fix/`, `docs/`, or
`chore/` branches, and check existing branches/PRs before duplicating work. Keep
changes focused, reasonably small, independently reviewable, and split into
logical descriptive commits. Avoid force-pushing shared branches or deleting
unmerged/active work. Recommend cleanup of completed/stale branches with evidence.

Open a PR against `master` for each meaningful implementation milestone. Include
purpose, implementation, tests actually run, CI links, compatibility implications,
documentation changes, and unresolved risks. Use the repository PR template.
Update the existing PR when its requirements evolve. Use draft state while checks
are pending; verify CI before requesting review. If CI is blocked by configuration
or permissions, state the exact limitation and leave the PR draft until resolved.
Do not merge without explicit owner authorization.

## Review standards

Correctness and determinism take priority over optimization. Add meaningful tests
for behavior changes and regressions, using generated or legally distributable
fixtures. Prefer safe Rust, explicit ownership, validated handles, checked input
bounds, and deterministic iteration. New dependencies need a concrete rationale.
Any proposed unsafe code needs documented invariants and an explicit policy review.

Update README for user-facing functionality/status, architecture for boundary or
design changes, roadmap for milestone evidence/progress, and compatibility notes
for behavioral findings. Relevant documentation updates belong in the same PR.
Never claim unrun tests, unverified benchmarks, or planned features as implemented.

## Licensing and repository boundaries

Contributions to the rewrite are GPL-2.0-only. Preserve upstream history,
attribution, source notices, `README.TXT`, and `LICENSE.TXT`. Do not commit
proprietary IWADs/assets, credentials, generated build output, or private repository
content. This project is separate from `Copgunn/UZDoom`: never access, copy from,
modify, publish, or otherwise involve that repository.

## Simulation and reference changes

Run arithmetic/simulation tests in debug and release. Compare C-derived vectors
separately from project consistency checks. `tools/reference/generate.py --check`
requires a supported GNU C host; all platforms can run the committed-vector Rust
tests. `snapshot_goldens.py --check` independently verifies DSIM-v1 encoding.
Regenerate fixtures deliberately, retaining compiler/source/adapter provenance.
Explain changed policies or schema versions; never overwrite expectations merely
to make a regression pass. See `docs/M2_REFERENCE.md` and `docs/STATE_FORMAT.md`.
