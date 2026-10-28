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
content. This project is separate from private `Copgunn/UZDoom` and `Copgunn/Gallowmark`:
never access, copy from, modify, publish, or otherwise involve those repositories.

## Simulation and reference changes

Run arithmetic/simulation tests in debug and release. Compare C-derived vectors
separately from project consistency checks. `tools/reference/generate.py --check`
requires a supported GNU C host; all platforms can run the committed-vector Rust
tests. `snapshot_goldens.py --check` independently verifies DSIM-v1 encoding.
Regenerate fixtures deliberately, retaining compiler/source/adapter provenance.
Explain changed policies or schema versions; never overwrite expectations merely
to make a regression pass. See `docs/M2_REFERENCE.md` and `docs/STATE_FORMAT.md`.


For M3a, also run `generate_m3a.py --check`,
`m3a_snapshot_goldens.py --check` and `check_headless.py` under `tools/reference/`.
Read `docs/M3A_REFERENCE.md` before changing collision or slide semantics. Preserve
M2 profile/v1 expectations, explicit M3a/v2 selection, independent spatial order,
and failed-step atomicity. Reproduction requires no commercial assets. M3 phases
are independently reviewed; stop at the current feature PR's owner-review gate.


For M3b, run `generate_m3b.py --check` and `m3b_snapshot_goldens.py --check` in
addition to earlier checks. Read `docs/M3B_REFERENCE.md`: profile-specific BSP,
opening, gravity/dropoff, BLOCKMAP and source-undefined policies are contracts.
Preserve v1/v2 fixtures and test all three schemas. Include per-field/per-tic
reference diagnostics, immutable map versus runtime-sector separation, and
atomic setup/step failures. Static two-sided gameplay openings do not establish
enhanced geometry or full player/actor-state behavior. M3b merge requires its own
owner approval. Renderer work remains a separate custom wgpu subsystem.

## M3c actor changes

Run `generate_m3c.py --check` and `m3c_snapshot_goldens.py --check` in addition to
all older generators/encoders. Run `actor_states`, all four debug/release examples,
and the actor-state fuzz target when changing admission/transitions. Preserve all
v1/v2/v3 fixtures. Record recognized metadata, admitted types, source actions and
synthetic probe capabilities separately. State entry is not damage processing;
map player starts do not imply complete player initialization. See
[the investigation](docs/M3C_REFERENCE.md) and [schema](docs/STATE_FORMAT.md).
M3c was regularly merged as `4dbb0a5` after final corrections and fresh CI. Each M3d subphase still needs separate owner
review and merge authorization.

## M3d1 combat changes

Read [M3D1_REFERENCE.md](docs/M3D1_REFERENCE.md). Run the M3d1 C generator and
independent snapshot encoder with `--check` alongside all earlier tools, all five
examples in debug/release, and the combat fuzz target. No C compiler is required
for recorded Rust comparisons. Preserve v1-v4 bytes and unsupported-branch
rejection; new native definitions must not bypass action or effect admission.
`request_actor_death` keeps its M3c contract; actual damage uses `damage_actor`.
M3d1 was regularly merged as `c924f18` after final correction and fresh/post-merge CI. M3d2 was regularly merged as `955f690` after correction-head and post-merge CI. M3d2b requires separate owner review.

## M3d2 weapon changes

Read [M3D2_REFERENCE.md](docs/M3D2_REFERENCE.md). Reproduce both M3d2 tools with
`--check`, preserve all earlier references/goldens, run `player_weapons` and
six-example debug/release equality, and build all five fuzz targets. Keep the
public combat API's bound-player protection, immutable source-state admission,
small preview/target-overlay preflight, explicit setup inventory and typed action
boundaries. Changes to timings/RNG require independent source evidence, not new
expectations copied from Rust. M3d2 is merged; M3d2b remains unmerged for owner review.

## M3d2b projectile changes

Read [M3D2B_REFERENCE.md](docs/M3D2B_REFERENCE.md). Reproduce all seven C
generators/independent encoders, both example-equality scripts and all six fuzz
builds. Preserve historical v1-v6 bytes/helpers. New comparisons must derive from
original C and explicit synthetic inputs, never Rust-generated expected traces.
Keep plasma, owner lifetime and arithmetic admission explicit, preflight failures
before mutation and reuse existing actor/damage services. Player damage is a
dependency investigation only. Record actual ASan runs and whether leak detection
was disabled. Leave the new PR open for owner review without auto-merge.
