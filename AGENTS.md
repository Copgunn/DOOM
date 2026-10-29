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
- Run all fifteen reference generators and independent golden encoders on Linux;
  cross-platform CI compares recorded vectors/goldens and debug/release examples.
- M3c PR #6 was owner-authorized and regularly merged as 4dbb0a5 after fresh
  correction-head and post-merge CI. M3d1 was regularly merged as c924f18 after fresh correction-head and
  post-merge CI in the M3d2 mission. M3d2 was regularly merged as 955f690 after final corrections and fresh/post-merge CI. M3d2b was regularly merged as e109ee1 after final correction and fresh/post-merge
  CI in the M3d3a mission. M3d3a completed its regular merge as 839fbab after fresh/post-merge CI;
  M3d3b regularly merged as 0007af1 and documentation PR #12 separately as
  876662d after exact-head and post-merge CI. M3d3c regularly merged as 9b8fde8
  after its correction/fresh/post-merge gates; M3d3d regularly merged as 31b4830
  after its correction/fresh/post-merge gates; later merges require separate approval.

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
  Pursuit and bounded attacks have separate profile 9/10 admission; M3d4 and M3e remain later work.

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
  M3d3b pursuit is merged; profile 8 excludes attacks and auto-merge stays disabled.

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
  schema bytes and Rust-only safety policies. PRs #11/#12 completed their separate
  regular merges and post-merge gates before M3d3c; auto-merge remains disabled.

- Read `docs/M3D3C_REFERENCE.md` before attack changes. Profile 10 composes v9
  perception/pursuit with strict named native roles and shared M3d1 combat.
  Preserve every v1-v9 byte, fixture, generator, encoder and historical helper.
- Keep source melee radius/strict threshold/sight order, missile sight/JUSTHIT/
  reaction/type adjustments/caps/RNG, Chase melee priority/skill/movecount and
  JUSTATTACKED-after-entry order. fastparm=false and netgame=false stay explicit.
- Only native type 136 admits the copied positive 10/8/8 POSS Face/Pos/return closure.
  Preserve both facing calls, ordered shadow/spread draws, ((R%5)+1)*3 damage,
  aim-before-sound and real impacts-before-damage. Markers are selection-only.
- One unshootable floor-standing owner must be first and stationary in physical
  momentum. Preflight its exact delayed shot with only copied owner/RNG and
  existing trace/damage plans; reserve effects before mutation. No full-state tic
  clone, parallel scheduler, duplicate hitscan/damage or unsupported action no-op.
- Retained references exclude removable effects. Dummy type 135 targets are unbound
  source-body fixtures; actual player damage/binding and public direct combat in
  profile 10 remain rejected. Do not expand these ownership/geometry policies
  without new proof. Preserve canonical state on every recoverable error.
- Reproduce all ten C/encoder pairs, ten asserted examples, nine fuzz builds and
  bounded attack ASan; report actual counts and disabled leak detection. Use
  `docs/DOC_REVIEW_CHECKLIST.md`; PR #13 completed its regular merge and
  exact post-merge CI before M3d3d. PR #14 subsequently completed its authorized
  regular merge and post-merge gates before M3d3e; no auto-merge.

- Read `docs/M3D3D_REFERENCE.md` before shotgunner/shared planning changes. Profile 11
  separately admits labelled native136 SPOS Look/Chase/Face/SPos with exact source
  10/10/10 timing, sprite 30 and fullbright frame 32773. Do not grant SPos to v10,
  Pos to v11, full original type 2, drops or actual player damage.
- Preserve shotgun sound BEFORE internal facing, both facing calls, one shared aim,
  saved base angle, three independent spread/damage draws per pellet and complete
  effect/damage RNG BEFORE the next pellet. No aggregate damage or nine-draw batch.
- Sequential preflight reads a small overlay of at most three touched bodies and
  three projected impacts, using the existing generic LifeHost combat executor.
  Update hit/damage decisions after each projection. Position/radius/index flags
  stay fixed inside this strict closure; real impacts are NOBLOCKMAP. Document
  that proof rather than freezing health/shootability or copying the full store.
- Reserve three real slots for any non-null blast before mutation, even a miss;
  no pending-retirement credit. Null needs no reservation. Predict lowest-free
  IDs/current generations without storage cloning. Retain all existing coordinate,
  work and one-first-floor-owner bounds; every recoverable failure stays canonical.
- Reproduce all eleven source/encoder pairs, eleven asserted examples, ten fuzz
  builds and bounded shotgunner ASan. Record actual executions/leak settings,
  source-exercised maps separately from map inventory, and compiled bodies
  separately from branch coverage. Preserve every v1-v10 reference byte/helper.
  Use the docs checklist; PR #14 completed its regular merge and exact post-merge
  CI before M3d3e. The M3d3f mission conditionally authorizes PR #15’s regular
  merge after its documentation correction and fresh checks; no auto-merge.

- Read `docs/M3D3E_REFERENCE.md` before CPOS/refire/shared combat changes. Profile 12
  separately admits exact labelled native 136, CPOS states 4/5/6/7 with sprite 38,
  frames 4/32773/32772/5 and 10/4/4/1. Preserve all historical v1-v11 bytes/helpers.
- Keep shotgun sound before internal facing, one fresh aim per CPosAttack and
  spread-left/right, damage, real impact and immediate damage RNG in source order.
  A null shot returns without sound/RNG. Never batch shots across tics or cache aim.
- Refire faces (including shadow draws) BEFORE its own draw. Below 40 preserves
  the natural loop; higher draws short-circuit null, mobj health, then original
  sight. Roster health is separate. Preserve immediate see/Chase/Look actions,
  source movement and genuinely reachable new ATK1 selection; never insert a no-op.
- The strict positive-duration closure and existing map/owner/reference/counter
  invariants make refire's sight/movement/state services infallible after read-only
  preflight. It allocates no actors. Keep first floor-owner/zero momentum, 1008/978
  owner margins, 994/1018 impact bounds, pending/stale checks and nested reaction
  guard. Single shots use the shared prepared combat path and reserve real impacts.
- The original LUT has 39/41 and no 40. Never manufacture gameplay-RNG coverage;
  label the isolated original-C injected-dependency boundary probe separately.
  Report body inventories, actually exercised maps and C/DSIM/policy families precisely.
- Reproduce all twelve source/encoder pairs and twelve asserted examples, build
  all eleven fuzz targets and run bounded chaingunner ASan with actual counts and
  leak settings. Keep full original type 10/drops, SpidRefire, other attacks and
  player damage/binding excluded. PR #15 completed its authorized regular merge
  as 5b918247 after macOS recovery and fresh exact-head/post-merge checks.
  M3d3f started only after those gates; PR #16 finalization follows the separately
  authorized M3d3g correction/fresh-check/regular-merge/post-merge gates.

## M3d3f Demon melee changes

- Read [M3D3F_REFERENCE](docs/M3D3F_REFERENCE.md) before SARG/shared damage changes.
  PR #15 regularly merged as `5b918247` after documented hosted arm64 acquisition
  failures, supported Intel macOS recovery and successful exact-head/post-merge CI.
  Keep the existing required macOS check name and all steps; do not infer arm64
  coverage from the explicitly selected `macos-26-intel` execution.
- Profile 13 admits only exact labelled native 136 SARG Look/Chase/4/5/6 and
  unbound target 135 source-body states. Keep source Demon health 150/mass 400
  adaptation explicit; original types 12/13 remain recognized/non-executable.
- Preserve raw normal 8/8/8, frames 4/5/6, all three faces and shadow draws.
  Nightmare's global SARG timer rewrite is rejected. Melee-selection sound 52 is
  before ATK1; SargAttack adds no duplicate attack sound or hitscan effect.
- Null returns before Face/range/RNG; misses still face but draw no damage.
  Use target info radius, strict approximate-distance boundary and original sight;
  selection and strike queries are separate. Dead/nonshootable current targets
  still reach the damage draw and shared ignored-damage branch when in range.
- Preflight copied owner/RNG, current range and shared DamagePlan before real
  facing/flags/RNG/events. Actual execution repeats range/sight after real Face,
  then one `(R%10+1)*4` draw and shared direct damage with source=inflictor=owner.
  Preserve actual demand (zero), capacity success and complete failed-call/tic
  canonical equality. No full store/world clone or second damage implementation.
- Retain first floor owner/zero physical momentum,64 actors,1008/978 owner margins,
 994 other XY,+/-512 maps,+/-1024 queries and 8192 work per sight query. New public
  combat/binding bypasses, mutable radius overrides, other attacks and full monsters
  remain excluded. Private invalid-bound-target tests are Rust safety evidence.
- Reproduce all thirteen C/independent-encoder pairs and thirteen asserted example
  comparisons. Build all twelve fuzz targets; report actual Demon ASan executions,
  corpus/limits and leak settings. Preserve every v1-v12 fixture/generator/encoder
  and eleven historical Rust helpers; reference README prose may change.
- The M3d3g mission conditionally authorizes PR #16 regular merge after its prose
  corrections, fresh mandatory local/four-job CI and live blocker review. Verify
  ordered merge parents/tree/master and post-merge CI before M3d3g runtime. No
  auto-merge; M3d3g needs separate owner review. No full vanilla sync is established.

## M3d3g Demon lifecycle changes

- Read [M3D3G_REFERENCE](docs/M3D3G_REFERENCE.md) before damage/state/corpse changes.
  PR #16 regularly merged as `75b2a2bc` after documentation correction `63efb2a5`,
  fresh four-job CI 40761281692 and exact post-merge CI 40762344038. Source
  investigation `2500650d` precedes metadata `da5becb8` and runtime `8739e753`.
- Profile14 copies source Demon metadata into named native 136 with state aliases,
  one first owner, flags SOLID|SHOOTABLE, no COUNTKILL/raise/xdeath/missile. Full
  original types 12/13, player/scoring, resurrection and cross-profile projectiles
  stay rejected. Synthetic hitscan shooters are type 135; never grant Pos/SPos/CPos.
- Preserve pain chance 180 and actual LUT, 2/2, delayed A_Pain sound 26, ordinary
  8/8/4/4/4/-1, randomized first timer 5..8, delayed A_Scream sound 64, immediate
  corpse/dropoff/quarter height and later A_Fall solidity clearing. Terminal -1
  retains identity/physics/spatial lists. Do not add a redundant corpse field.
- Use shared DamagePlan/LifeHost, existing physics/spatial/store. Incoming damage
  may immediately enter Chase. XY/Z before strike consumes no RNG in this strict
  unbound closure; prepare potential damage with small copied RNG, then execute
  the actual range query after physics/facing. Never cache pre-physics acceptance.
- Keep existing64 actor/map/query/work/counter bounds, owner 978/1008 and other XY 994
  margins. Preflight identities/roles/bindings/all closures/capacity before mutation.
  Direct damage needs zero slots; hitscan still reserves its real impact. Corpse
  targets retain generations; no stale reference may alias a reused slot. Public
  setup/state APIs must not restore a corpse to live native states.
- Preserve all v1-v13 bytes/generators/encoders/evidence and twelve older Rust
  helpers. Reproduce fourteen C/encoder pairs and fourteen asserted debug/release
  examples; build all thirteen fuzz targets and report actual ASan/leak settings.
  Distinguish331 C gameplay scenarios on 21 exercised maps, 68 compiled bodies,
  three separately injected boundary components, independent canonical bytes and
  Rust-only policies. Body inventory is not branch coverage.
- The real LUT lacks179/180. Never count injected boundary values as gameplay
  draws or captures. Preserve the scalar component's explicit provenance.
- Keep verified native macos-26-intel and all four CI jobs/steps. M3d3g PR17
  completed its authorized regular merge as f44828d5 after correction and exact
  post-merge gates. M3d3h has a separate owner-review gate without auto-merge.

## M3d3h original Demon stand/run changes

- Read [M3D3H_REFERENCE](docs/M3D3H_REFERENCE.md) before stand/run admission or
  state scheduling changes. PR17 regularly merged as `f44828d5` after correction
  `9d48f6d4`, fresh CI40780721974 and exact post-merge CI40781481835. Investigation
  `4aec917b` precedes metadata `b073f648` and integration `d35a67af`.
- Profile 15 uses exact original STND/STND2 and all eight RUN states, remapped to
  1/3 and2/15–21. Keep sprite39, frames,10/10 and2-tic timings, every Look/Chase
  entry and immediate nested handoffs. Do not merge repeated frames or run AI
  every presentation frame. Direct spawn installs state without its action.
- Reuse shared Look/Chase, movement, DamagePlan/LifeHost and lifecycle preflight.
  Normal source melee/pain/death/corpses remain composed. Preserve the exact
  spawn-state-only damage-to-see check; STND2 is not spawnstate. No new scheduler,
  pathfinder, damage engine or per-tic world clone. Keep existing 64-actor,
  coordinate/work/counter bounds and complete failed-operation atomicity.
- Preserve generations and source thinker/spatial order. Public death requests,
  corpse revival, raise, scoring/player branches, other actions and full original
  Demon/Spectre remain rejected. Profile 14 and its loops stay unchanged.
- Reproduce all fifteen original-C/independent-encoder pairs and fifteen asserted
  debug/release examples; build fourteen ASan fuzz targets and record bounded
  stand/run sanitizer executions and leak configuration. Freeze v1–v14 fixtures,
  encoders, provenance and thirteen historical Rust helpers; only reference README
  prose may change. Never derive expected data from Rust output.
- Report source comparisons (390 scenarios, all 21 maps), compiled-body inventory,
  independent DSIM-v15 bytes and Rust-only ownership/atomicity tests separately.
  The three injected C pain components are not gameplay draws or captures.
- Native macos-26-intel and all four CI jobs/steps remain required. Keep the new
  M3d3h PR draft while validation is pending, then ready/open/unmerged without
  auto-merge for owner review. No later gameplay milestone is authorized.

## M3d3i original Demon map-spawn changes

- PR18 completed its owner-authorized regular merge as `cae36bed` after correction
  `f5d1bc83`, fresh CI40791592122 and exact post-merge CI40792730511. All four jobs
  passed before investigation `ab755838` or runtime. The M3d3i mission supersedes
  M3d3h's earlier owner-review-only status; the new M3d3i PR must stay open/unmerged
  without auto-merge. Do not begin another gameplay milestone.
- Read [M3D3I_REFERENCE](docs/M3D3I_REFERENCE.md) before spawn/admission changes.
  Profile16 reuses the existing classic THING parser and profile15 registry.
  Source3002 resolves to original MT_SERGEANT12 before filtering/adaptation to
  labelled native136. Keep COUNTKILL-based nomonsters filtering before mapping;
  scoring/raise/full original actor admission remain excluded.
- Preserve source skill groups, single-player bit16, record order, signed fixed
  coordinates, BSP/floor/ceiling, truncating/wrapping ANG45 conversion, raw spawn
  without action, lastlook then positive timer RNG and post-spawn ambush. No second
  parser/store/scheduler, sorted actors, rebuilt BLOCKMAP or per-tic world clone.
- Batch requires tic0/never-allocated store, at most256 THINGS/four Demons/64 slots;
  preflight the entire batch before RNG/allocation/link changes. Native thinkers
  precede stable unbound135 targets/effects. Live native/roster/sound targets must
  be135; cross-native retargeting/infighting rejects. Corpses retain old generations.
- Global bounded preflight proves roles, closures, references, bindings, geometry,
  counters and zero melee allocation before a pass. Late small shared DamagePlan
  reads actual current target health/RNG at A_SargAttack after earlier owners;
  never reuse an earlier owner's cached plan. Keep body initial XY momentum64,
  native978/1008 and other994/map512/query1024/sight8192 bounds. Unsupported future
  callbacks must reject before any earlier actor mutates.
- Reproduce all sixteen C/independent-encoder pairs and sixteen asserted examples;
  build all fifteen ASan targets and report actual bounded map-spawn executions
  with leak settings. Freeze v1–v15 bytes/fixtures/encoders and all fourteen older
  Rust helpers; reference README prose may change. Distinguish334 scripts with
  separate unadapted/adapted source passes,123 isolated filter components,
  canonical evidence and Rust-only policy tests. Inventory70 is not branch coverage.
- Keep native macos-26-intel, required check names and every prior CI step. Pending
  delivery stays draft until observed mandatory checks and exact final-head Linux/
  Windows/native macOS/Rust1.85 CI pass. Never access IWADs or other repositories.
