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
  static-sector BSP/openings/XY/Z. M3c adds typed actor lifecycle in profile 4; M3d1 adds bounded explicit unbound
  damage/aim/hitscan in profile 5; M3d2 adds bounded player weapons/psprites
  in profile 6; M3d2b adds original plasma foundations in profile 7. Complete player/view, dynamic
  sectors, combat, specials, demos and rendering remain incomplete. Preserve
  all M2/v1, M3a/v2, M3b/v3, M3c/v4, M3d1/v5, M3d2/v6 and M3d2b/v7 suites; read their source investigations before
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
- Run all nine reference generators and independent golden encoders on Linux;
  cross-platform CI compares recorded vectors/goldens and debug/release examples.
- M3c PR #6 was owner-authorized and regularly merged as 4dbb0a5 after fresh
  correction-head and post-merge CI. M3d1 was regularly merged as c924f18 after fresh correction-head and
  post-merge CI in the M3d2 mission. M3d2 was regularly merged as 955f690 after final corrections and fresh/post-merge CI. M3d2b was regularly merged as e109ee1 after final correction and fresh/post-merge
  CI in the M3d3a mission. M3d3a completed its regular merge as 839fbab after fresh/post-merge CI;
  M3d3b or later merges require separate authorization.

- M3b consumes supplied BLOCKMAP lists without rebuilding them; conservative
  omissions are diagnostic, not proof of spatial correctness. Keep older profiles
  unchanged. Sector-head and cell-head order derive from explicit relinking.
- Profile-4 state/action work must preflight before gameplay mutation. Keep original
  actions separate from synthetic probes; unsupported callbacks must never no-op.
  Preserve source direct-spawn versus entered-state timing, target generations,
  same-pass append order and immediate spatial/deferred storage removal.
- Keep future custom wgpu rendering separate, preferring Vulkan where supported.
  No renderer, Bevy, audio or general physics dependency enters authoritative tics.

- Read `docs/M3D1_REFERENCE.md` before combat changes. Preserve original effect
  thinkers and RNG-before-damage order, pure aim queries, conservative capacity
  preflight and complete failed-operation atomicity. Combat cannot recursively
  invoke combat from state probes; future weapon/AI work needs separate evidence.
- Run `generate_m3d1.py --check`, `m3d1_snapshot_goldens.py --check`, all five
  debug/release examples and the bounded combat fuzz target for relevant changes.
  Preserve all historical generators, fixtures and the original shared builder.

- Read `docs/M3D2_REFERENCE.md` before weapon/player changes. Keep actor and
  psprite dispatch separate. Preserve original active-slot/player-before-thinker,
  weapon-before-flash, real impact-before-damage and per-pellet RNG order.
- Profile 6 admits one LIVE weapon player and at most 64 actors. Preserve exact
  source state admission, bounded source pawn fixtures, protected bound targets
  and the private shooter-only combat route. No global player-damage enablement.
- Weapon preflight copies only the small player/RNG/pawn record and overlays
  actually touched targets. Preserve all canonical fields on recoverable errors;
  no full Simulation/Map/store clone per tic. Unsupported actions never no-op.
- Run `generate_m3d2.py --check`, `m3d2_snapshot_goldens.py --check`, all six
  debug/release examples and bounded weapon fuzzing, alongside every older suite.
  All v1-v5 schemas and historical helpers/fixtures/encoders remain unchanged.

- Read `docs/M3D2B_REFERENCE.md` before projectile changes. Profile 7 admits only
  exact MT_PLASMA missiles, original immutable states/types and labelled bounded
  player-state fixtures. Reuse ThinkerStore/cursor, M3b thing/line queries, LifeHost
  and M3d1 damage; no second physics, scheduler or damage implementation.
- Preserve owner generations, source half-step-before-relink, things-before-lines,
  positive-only XY subdivision and residual movement after explosion. Live owners
  must remain active; exploded missiles may retain unresolved old identities.
  Bound players and actual player damage remain rejected. No full-state tic clone.
- Run `generate_m3d2b.py --check`, `m3d2b_snapshot_goldens.py --check`, both
  `check_headless.py` and `check_projectiles.py`, all six fuzz builds and bounded
  projectile ASan smoke tests. Preserve every v1-v6 schema, fixture, generator,
  encoder and historical helper unchanged. New projectile observations use tag 15.
- M3d2b PR #9 completed its owner-authorized regular merge and post-merge gates;
  never enable auto-merge. M3d3a completed its authorized regular merge and post-merge gates.
  Pursuit is admitted only in profile 9; attacks, M3d4 and M3e remain later work.

- Read `docs/M3D3A_REFERENCE.md` before perception changes. Profile 8 composes
  static-sector typed lifecycle, pure REJECT/BSP sight, a distinct four-slot
  active/health/generation roster and shared noise; v6 weapon players, v7 missiles,
  player damage and full monster execution remain separate.
- Preserve source three-way sight arithmetic (including horizontal x==node.y),
  seg/duplicate order, scan stopping/lastlook, sound assignment before ambush
  confirmation/fallback, threshold reset and wake RNG/state-entry order.
- Profile 8 A_Look executes only labelled native fixture lookers with fully admitted
  action-free see handoffs. Reject original A_Chase and unsupported reachable
  actions before allocation/mutation. No chase no-op, pursuit or attack scheduler.
- Active roster/sound references must stay valid and non-removing during a tic.
  Validate every retained sound identity before a tic with a possible looker,
  since movement can change its sector. Empty-roster termination and stale-ID
  rejection are Rust safety policies, never C-equivalence claims. Keep all failed
  operations canonical-state atomic without a full Simulation/Map clone.
- Preserve +/-512 map, +/-1024 actor-query, 8192 expanded sight work, 64 actor and
  documented movement bounds. Run `generate_m3d3a.py --check`, independent
  `m3d3a_snapshot_goldens.py --check`, the eighth example equality helper, all seven
  fuzz builds and bounded perception ASan. Preserve every historical v1-v7 code
  fixture/generator/encoder/helper. M3d3a completed its regular merge as 839fbab after fresh/post-merge checks.
  M3d3b pursuit is separately authorized; attacks and auto-merge remain excluded.

- Read `docs/M3D3B_REFERENCE.md` before pursuit changes. Profile 9 composes the
  perception roster/noise with non-attacking native look/chase fixtures. Keep all
  v1-v8 bytes/fixtures/generators/encoders and historical helpers unchanged.
- Source spawn initializes movedir=EAST and movecount=0. Preserve exact eight
  fixed vectors, immediate P_TryMove, spatial relinking/floor snap, successful-only
  TryWalk count draw, NewChaseDir retry/fallback/turnaround order and RNG.
- A_Chase executes through LifeHost and existing thinker order after XY/Z updates.
  Preserve reaction/threshold countdown, signed BAM turning, target health versus
  shootability, allaround reacquisition, spawn return, JUSTATTACKED and active-sound
  draws. Do not add a scheduler or convert direct walking into momentum.
- Native fixtures explicitly remove source player missile-animation metadata and
  have no melee/missile states. Reject attacks, FLOAT/NOCLIP/TELEPORT, line/sector
  specials, original monster action closures and unsupported flags before mutation.
  No door/float/attack stub may fabricate success. Single-player, fastparm=false.
- At most 64 actors, speed 1..16, +/-512 maps, +/-1024 queries, 8192 expanded sight
  work. Direct callback XY margin is 1008; tic margin 978 proves 30 physical+16 direct
  units. Validate all retained sound/current target/active roster IDs before chase
  effects. Stable actor-state admission prevents mid-tic target removal. Preserve
  full canonical state on recoverable rejection without a full-state tic clone.
- Countdowns reject INT_MIN; reaction also rejects INT_MIN+1 because a loss->Look
  handoff may immediately invoke Chase twice. Missing/nonshootable targets use
  source reacquisition; pending/stale IDs reject and never alias slot reuse.
- Run all nine source/encoder pairs, nine example equality checks, all eight fuzz
  builds and bounded pursuit ASan. Distinguish C equivalence, independent project
  schema bytes and Rust-only safety policies. M3d3b stops at owner review, open
  and unmerged with auto-merge disabled. Attacks require separate authorization.
