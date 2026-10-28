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

Status: **M3a/M3b implemented, validated and merged; M3c implemented, validated and merged; M3d1 merged; M3d2 merged; M3d2b merged; M3d3a merged; M3d3b merged; M3d3c merged; M3d3d merged; M3d3e merged; M3d3f Demon melee implemented for separate review**.
M3a [PR #4](https://github.com/Copgunn/DOOM/pull/4) merged by owner-authorized
regular commit `764dd9605fa3af3befc133287e9949930981f469`; fresh and post-merge CI
passed before branching M3b. Exact evidence is in [validation](VALIDATION.md).
M3 as a whole remains incomplete.

| Phase | Scope | Status |
| --- | --- | --- |
| M3a | Grounded player XY, wall sliding, solid actor collision | Implemented, validated, merged |
| M3b | Static multi-sector openings, BSP, XY/Z and gravity | Implemented, validated and merged |
| M3c | Actor states, type-driven spawning and death | Implemented, validated and merged |
| M3d | Combat subphases below | M3d1/M3d2/M3d2b merged; M3d3a merged; M3d3b merged; M3d3c merged; M3d3d merged; M3d3e merged; M3d3f implemented for review; later work planned |
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
- [x] Owner authorization, correction-head checks and verified regular merge;
  post-merge CI passed on `764dd96`.

See [M3a scope and safe policies](M3A_REFERENCE.md), [state schema](STATE_FORMAT.md)
and [validation](VALIDATION.md). A flat static movement subset is not full vanilla
player thinking, gameplay synchronization, demo playback or DOS compatibility.

### M3b acceptance and review boundary

Branch: `feat/m3b-multisector-vertical-physics`, [PR #5](https://github.com/Copgunn/DOOM/pull/5).
Merged by owner-authorized regular commit `c791ef81d9cdb5d42638e4806762b8fe32793f3f`
after corrected-head CI. Post-merge CI passed before the M3c branch was created.
M3c requires its own owner-review gate.

- [x] Source findings before runtime changes; immutable original C and separate provenance.
- [x] Iterative original BSP side/child/leaf semantics, bounded topology validation.
- [x] Two-sided openings, blocking/player flags, exact clearance/step/dropoff order.
- [x] Per-actor XY then Z, source gravity impulse, floor/ceiling contact, airborne rules.
- [x] Explicit gravity/dropoff setup, sector membership and head ordering, safe reuse.
- [x] Investigated reference-list BLOCKMAP policy, sparse/missing-data C comparisons.
- [x] Preserve old profiles and DSIM-v1/v2 expectations; add explicit DSIM-v3.
- [x] 696 C tics, 261 probes, 1280 BSP lookups, 512 side predicates, 20 openings.
- [x] Independent 96-state v3 digests plus five complete transition/landing snapshots.
- [x] Six-scenario headless demo with 144 C assertions and debug/release equality.
- [x] Deep/invalid BSP, height/flag/coverage bounds, extremes, lifecycle/atomicity tests.
- [x] Long multi-sector/gravity/pause/RNG/read-schedule traces; capacity/history checks.
- [x] Required local workspace/release/Rust 1.85 checks; parser fuzz targets compile.
- [x] Initial Linux/Windows/macOS debug/release and Rust 1.85 CI passed on
  `975e2b9`; final exact-head checks are verified on PR #5 before review readiness.
- [x] Owner-authorized regular merge; correction-head and post-merge CI passed.

See [M3B_REFERENCE.md](M3B_REFERENCE.md) and [STATE_FORMAT.md](STATE_FORMAT.md).
M3b is static-sector physical motion, not dynamic doors/lifts, actor state machines,
complete player view/thinking, combat, enhanced portals, demos or rendering.

### M3c acceptance and review boundary

Branch: `feat/m3c-actor-states-spawning`, [PR #6](https://github.com/Copgunn/DOOM/pull/6),
based on verified M3b merge `c791ef8`.
Implemented scope is recorded below; final delivery SHA/CI and PR are in
[validation evidence](VALIDATION.md). The owner reviewed `fdeb900` and conditionally
authorized a regular merge after final documentation corrections and fresh CI.
M3d1 requires its own owner-review and merge authorization.

- [x] Original source findings recorded before runtime implementation.
- [x] All 137 types/967 states/74 action identities; validated executable subset.
- [x] Direct spawn, countdown, zero/infinite/null entry, recursive actions and ordering.
- [x] Five supported source actions; unsupported actions reject before mutation.
- [x] Typed source spawning; 67 admitted source types at three heights compared to C.
- [x] Single-player THINGS filtering, RNG, angle, ambush; deferred starts explicit.
- [x] Controlled death, terminal persistence/null removal and safe retained targets.
- [x] Append-sensitive callbacks, same-pass child update, deferred retirement.
- [x] Bounded metadata/closure/preflight work and recoverable failure atomicity.
- [x] DSIM-v4, 99 independent digests and 24 full byte states; older formats preserved.
- [x] Asserted asset-free headless example; native probes clearly separated from vanilla.
- [x] Local, sanitizer, MSRV and cross-platform CI evidence; final-head CI recorded on PR #6.
- [x] Owner engineering review and conditional regular-merge authorization.
- [x] Fresh correction-head CI, regular merge `4dbb0a5` and post-merge CI verified.

M3d is weapons/projectiles/damage/monster combat. M3e is moving sectors, doors,
lifts, switches and map specials. Full player thinking, scripts, demos and rendering
remain outside M3c. Future DECORATE-like/ACS/ZScript interfaces require independent
implementation and licensing review; no GPLv3 GZDoom code enters this GPL-2.0-only
repository. Custom wgpu rendering with Vulkan preferred remains later work.

### M3d combat subphases

| Phase | Scope | Status |
| --- | --- | --- |
| M3d1 | Damage, targeting, hitscan tracing and kill/death foundations | Merged as c924f18 after fresh and post-merge CI |
| M3d2 | Player weapons, psprites and ammo | Merged as 955f690 after fresh/post-merge CI |
| M3d2b | Bounded original projectile foundations | Implemented, validated and merged |
| M3d3 | Monster targeting/perception, movement, attacks and AI | M3d3a merged; M3d3b merged; M3d3c merged; M3d3d merged; M3d3e merged; M3d3f Demon melee implemented for separate review |
| M3d4 | Integrated combat, explosions, advanced attacks and regressions | Planned |

M3d1 branch: `feat/m3d1-damage-hitscan`, based on verified M3c merge `4dbb0a5`.
Source findings precede runtime work in [M3D1_REFERENCE.md](M3D1_REFERENCE.md).
Each subphase needs independent evidence and owner merge authorization. M3d1
was regularly merged as `c924f18` after correction and fresh/post-merge CI; no
weapon/AI/full-combat acceptance is implied by this slice.

## M4 — Demo compatibility

Status: planned.

Acceptance: pinned reference demos with legal provenance and version-specific
header handling; complete deterministic playback; compatibility regression suite
and desynchronization reports. Publish guarantees per reference version. Treat
DOS 1.9 evidence separately from Linux 1.10/demo version 110.

## M5 — Rendering and platform integration

Status: planned.

Acceptance: renderer abstraction, presentation loop, per-tic input assignment,
audio events/playback, software/headless diagnostic backend, and custom wgpu hardware renderer, with Vulkan preferred where supported. Verify Linux/Windows/macOS presentation where supported by the selected
Vulkan platform path; document prerequisites and gaps. Replay tests must prove
presentation frame rate and renderer choice do not alter simulation results.

### M3d1 acceptance evidence

- [x] M3c correction-head regular merge and post-merge CI verified before branching.
- [x] Source investigation precedes runtime; original source preserved.
- [x] Bounded unbound damage, shootability, impulse, pain/reaction/retarget,
  strict extreme-death threshold, kill flags/height/actions/timer and corpse physics.
- [x] Source BLOCKMAP DDA/candidates/intercepts/ties, aim window, wall/opening and
  first-hit rules; original impact thinkers and RNG order.
- [x] Existing state host, safe generations, conservative spawn preflight,
  atomic error regressions and bounded combat fuzzing.
- [x] 48 original-C scenarios, 431 captures, 38,052 live actor-field comparisons;
  complete supported death tics and RNG wraparound.
- [x] DSIM-v1-v4 retained; v5 has 63 independent captures and 17 byte fixtures.
- [x] Asserted combat arena: hit, wall, pain, lethal hit, 80 tics, corpse and digest.
- [x] Required local stable/release/MSRV/reference/docs checks; exact results in validation.
- [x] Linux/Windows/macOS and Rust 1.85 CI passed on implemented head `10de49e`
  ([run 40617772778](https://github.com/Copgunn/DOOM/actions/runs/40617772778));
  reviewed final `f17e793` also passed
  [CI 40618036910](https://github.com/Copgunn/DOOM/actions/runs/40618036910).
- [x] Owner engineering review and conditional regular-merge authorization.
- [x] Final documentation correction, fresh exact-head CI, regular merge and
  post-merge verification before M3d2 implementation.

M3d2 should implement source weapon/psprite/ammo/button timing and spread/damage
RNG, followed by independently tested projectile foundations. Actual player damage
requires armor/inventory/immunity/death/scoring integration before admission.
M3d3 remains perception/AI, M3d4 integrated combat/explosions. No renderer or
scripting implementation is part of M3d1.


### M3d2 acceptance evidence

Branch `feat/m3d2-player-weapons-psprites` starts from verified master `c924f18`.
Source investigation was recorded as `41d89e5` before runtime implementation.

- [x] M3d1 authorized regular merge and exact post-merge CI verified.
- [x] Explicit LIVE player setup, ownership/ammo, ready/pending weapons and slots.
- [x] Source two-slot psprites, immediate raise, zero/null/infinite semantics,
  typed actions, weapon-before-flash updates, body state and bob timing.
- [x] Command-driven pistol and seven-pellet shotgun; source spread/damage/impact RNG.
- [x] Optional chaingun independently verified, including second flash/last round.
- [x] Unpowered fist preserves exhausted-ammo fallback; deferred weapons reject.
- [x] M3d1 combat reused through private shooter eligibility; bound targets protected.
- [x] Source sector noise propagation, including one/two sound-blocking lines.
- [x] Small-record/RNG/target-overlay preflight and atomic capacity/error regressions.
- [x] 38 C scenarios, 4,890 captures, 422,943 player/actor field comparisons and
  combined source weapon/lifecycle action order; historical regressions preserved.
- [x] DSIM-v6: 69 independent captures, 24 full byte fixtures; v1-v5 unchanged.
- [x] Asserted `player_weapons` example and six-example debug/release equality.
- [x] Local workspace/release/MSRV/docs/reference checks; evidence in validation.
- [x] Linux/Windows/macOS and Rust 1.85 CI passed on implemented head `75f2c72`
  ([CI 40623977578](https://github.com/Copgunn/DOOM/actions/runs/40623977578));
  reviewed final `0e94091` also passed
  [CI 40624283411](https://github.com/Copgunn/DOOM/actions/runs/40624283411).
- [x] Owner engineering review and conditional regular-merge authorization.
- [x] Final correction, fresh CI, regular merge and post-merge verification before M3d2b.

Projectiles remain deferred in profile 6; profile 7 below adds the bounded
projectile foundation separately. Player damage remains future work, followed
by M3d3 perception/monster AI and M3d4 integrated combat. Current weapon support does not establish complete
players, pickups, armor/powers, demos or full vanilla/DOS synchronization.

## M3d2b — Projectile foundations

[PR #9](https://github.com/Copgunn/DOOM/pull/9), branch
`feat/m3d2b-projectile-foundations`, starts from verified master `955f690`.
The source investigation was committed before runtime work. Original MT_PLASMA
(type 34) has action-free flight/explosion states and needs no splash or AI.

- [x] Authorized M3d2 correction, regular merge and exact post-merge CI.
- [x] Source launch/autoaim, velocity, shadow RNG and initial half-step/check.
- [x] Missile XY/Z, candidate/opening order, source/species immunity and boundaries.
- [x] Existing direct damage, pain/death, impact sound/state/timer and full removal.
- [x] Generation-aware owners, spatial/thinker order and synthetic same-pass spawn.
- [x] Explicit unsupported-type/action/player/sky/special/arithmetic/work rejection.
- [x] Atomic failed setup/launch/step, capacity, stale/reused handles and read independence.
- [x] All v1-v6 historical bytes/tools preserved; independent v7 bytes and diagnostics.
- [x] 41 C scenarios, 129 independent snapshots, asserted seventh headless example.
- [x] Local build/fmt/Clippy/docs, 131 workspace/MSRV checks, 107 release tests,
  all seven C generators/encoders and seven-example debug/release equality.
- [x] All six fuzz targets build; bounded projectile ASan campaign recorded in validation.
- [x] Future player-damage prerequisites documented without enabling damage.
- [x] All 165 upstream files, public GPL licensing and private-asset safeguards preserved.
- [x] Linux/Windows/macOS/Rust 1.85 CI on implemented head `5108034`
  ([CI 40629886585](https://github.com/Copgunn/DOOM/actions/runs/40629886585));
  final review-head SHA/run are verified in PR #9 before readiness.
- [x] Owner engineering review and conditional regular-merge authorization in M3d3a mission.
- [x] Final correction-head CI, regular merge `e109ee1` and exact post-merge CI.

No full plasma rifle, rocket/BFG/splash, monster attacks, armor/player damage,
renderer, scripting or engine-limit expansion is included. Recommended next
bounded work is M3d3 source perception/target acquisition before attack callbacks;
actual player damage requires its separately specified acceptance gate. M3d4
remains integrated/advanced combat, and M3e remains moving sectors/map specials.

## M3d3a — Monster perception and target acquisition

Bounded subphase of M3d3, on `feat/m3d3a-monster-perception` from verified merge
`e109ee1`. [Source investigation](M3D3A_REFERENCE.md) was committed as `fec0d24`
before runtime changes; [validation](VALIDATION.md) records the completed PR #9
merge and post-merge CI.

- [x] Pure original REJECT/BSP sight: three-way sides, source traversal, seg order,
  duplicate suppression, one/two-sided openings, intercepts and slope narrowing.
- [x] Source four-slot scanning, player-record health, lastlook, candidate limit,
  unsigned angle/FOV and approximate close-range exception.
- [x] Shared source sector noise with distinct target/emitter; sound awareness,
  ordinary/ambush assignment and fallback, wake sound/RNG and state-entry order.
- [x] Genuine A_Look isolated with labelled native lookers and action-free see;
  original monster A_Chase handoffs reject before mutation.
- [x] Generation-safe roster/targets, bounded empty-roster policy, arithmetic/work
  admission and full canonical-state atomicity on recoverable failure.
- [x] 68 C scenarios, 1536 side/131 fraction vectors, 365 queries/3955 sight visits,
  460 slot observations, 47,520 actor fields and 625 raw RNG draws compared.
- [x] Independent v8 encoder: 420 captures, 45 complete byte fixtures; all v1-v7
  bytes and historical reference code/fixtures preserved.
- [x] 149 workspace/MSRV checks, 125 release tests, eight reference pairs and
  eight example equality checks; seven fuzz builds and 2000 ASan executions.
- [x] [PR #10](https://github.com/Copgunn/DOOM/pull/10); [CI 40637618358](https://github.com/Copgunn/DOOM/actions/runs/40637618358)
  passed Linux/Windows/macOS/MSRV on exact head `4e7720f0ba28781e72e3abe08c269389f582ad31`.
- [x] Owner engineering review and conditional regular-merge authorization in M3d3b mission.
- [x] Final correction, fresh local/remote checks, regular merge as `839fbab` and
  [post-merge CI 40640215475](https://github.com/Copgunn/DOOM/actions/runs/40640215475).

M3d3b remains source-compatible monster pursuit and movement decisions. Later
M3d3 work adds attack selection/actions; M3d4 integrates combat/advanced attacks,
M3e moving sectors/map specials, M4 demos and M5 renderer/platform integration.
These boundaries confer no authorization to begin them. Full original monster
execution, vanilla/DOS demo synchronization and private-IWAD validation remain
unverified.

## M3d3b — Monster pursuit and movement decisions

Bounded subphase of M3d3 on `feat/m3d3b-monster-pursuit`, from verified `839fbab`.
[Source investigation](M3D3B_REFERENCE.md) commit `685f2ed` precedes runtime work.
[Validation](VALIDATION.md) records PR #10 correction/merge/post-merge gates.

- [x] Correct source EAST/zero spawn fields and eight exact fixed direction vectors.
- [x] P_Move/TryWalk using existing ordered collision/relinking and floor placement.
- [x] NewChaseDir diagonal/axis/old/reverse-search/turnaround priorities and RNG.
- [x] Native non-attacking A_Chase in thinker/state order, reaction/threshold,
  quantized turning, JUSTATTACKED, direct movement and active sounds.
- [x] Source target retention, allaround reacquisition and immediate spawn/Look return.
- [x] Attack/door/float rejection, stable generations, bounded work/arithmetic and
  canonical-state atomicity on recoverable errors, without a full-state tic clone.
- [x] 96 original-C scenarios/17 maps; 108,992 actor fields, 2401 move attempts,
  11,458 candidates, 2103 direction assignments and 1380 RNG draws compared.
- [x] Independent DSIM-v9:835 captures/49 full streams; all v1-v8 bytes preserved.
- [x] Asserted ninth headless example;162 workspace/MSRV and 138 release tests,
  nine source/encoder pairs, nine example equality checks, eight fuzz builds,
  2000 ASan executions without findings (leak detection disabled).
- [x] [PR #11](https://github.com/Copgunn/DOOM/pull/11); [CI 40642802778](https://github.com/Copgunn/DOOM/actions/runs/40642802778)
  passed Linux/Windows/macOS/MSRV on exact implementation head
  `14efb10ff02f27d5e15f21ae7c5695b417c0352e`. Final delivery head/CI is pinned in the PR.
- [x] Owner engineering review and conditional regular-merge authorization in M3d3c mission.
- [x] Final correction 373da824, fresh CI 40644890571, regular merge 0007af1 and
  post-merge CI 40645513244. Documentation PR #12 separately merged 876662d after
  integrated-head CI 40646239134 and combined-master CI 40646610326.

Next focused work is later M3d3 attack selection/action admission, subject to
separate review (a bounded M3d3c). M3d4 remains integrated/advanced combat, M3e
moving sectors/map specials, M4 demos and M5 rendering/platform work. This does
not authorize beginning those tasks or claim full original monster gameplay.

## M3d3c — Monster attack selection and bounded actions

Branch `feat/m3d3c-monster-attack-selection` starts at verified combined master
`876662d`. [Investigation](M3D3C_REFERENCE.md) commit `0d57c07` precedes runtime.
[PR #13](https://github.com/Copgunn/DOOM/pull/13) completed its authorized regular
merge as `9b8fde8` after correction/fresh checks and exact post-merge CI.

- [x] Separately finalize PRs #11/#12, verify both regular merges and post-merge CI.
- [x] Source melee/missile decisions, radius/strict bounds, sight/flag/reaction/type/
  cap order and exact RNG; Chase priority, skills/movecount and state/sound timing.
- [x] Source FaceTarget and complete bounded native type 136 PosAttack animation,
  including both facing actions, shared aim/impact/damage and real effect lifecycle.
- [x] Explicit selection-only markers; unsupported closures and actual player
  damage reject. Recoverable failures are canonical-state atomic without tic clones.
- [x] 64 compiled original C bodies,158 scenarios on 4 exercised maps (18 supplied),
  178178 actor-field comparisons,
  2430 RNG draws and 20,785 ordered observations. Full v1-v9 preservation.
- [x] Independent v10:1290 captures/72 complete byte fixtures; asserted tenth example.
- [x]170 workspace/MSRV checks,146 release tests,ten example equality checks,
  nine fuzz builds and 10,000 ASan executions (leak detection disabled).
- [x] Implementation head `bf4bb89` passes Linux/Windows/macOS/MSRV
  [CI 40649024287](https://github.com/Copgunn/DOOM/actions/runs/40649024287).
  Final documentation-head SHA and fresh CI are pinned in PR #13 after verification.
- [x] Conditional owner regular-merge authorization in the M3d3d mission.
- [x] Correction `bc67dce`, fresh CI 40652266967, regular merge 9b8fde8 and
  exact post-merge CI 40653006957 verified before M3d3d.

Acceptance means this declared source/fixture slice, not complete original monster
combat or a playable engine. Next focused work requires source investigation and
separate authorization for another bounded attack/monster closure. M3d4 advanced
combat, M3e dynamic sectors, M4 demos and M5 rendering remain deferred.

## M3d3d — Shotgunner multi-pellet foundations

Branch `feat/m3d3d-shotgunner-attacks` starts at verified master `9b8fde8`.
[Investigation](M3D3D_REFERENCE.md) commit `51a2636` precedes runtime.
[PR #14](https://github.com/Copgunn/DOOM/pull/14) completed its authorized regular
merge as `31b4830` after correction/fresh checks and exact post-merge CI.

- [x] PR #13 correction, fresh local/remote checks, regular merge, parents/tree,
  master and exact post-merge CI verified before runtime work.
- [x] Exact SPOS 10/10/10, fullbright 32773, both faces, sound-before-internal-face,
  one shared aim and three source-order spread/damage/impact/health sequences.
- [x] Updated touched-body/effect projection using shared combat; conservative
  three-slot preflight and complete failed-operation canonical atomicity.
- [x] Source early pain/death, later targets after pellet 1/2 kill, blood/puff,
  fall-forward/RNG wrap, same-pass effects, retirement and generation reuse.
- [x] 65 compiled original bodies, 105 scenarios exercising all 18 supplied maps;
  147940 actor fields, 34,350 ordered observations and 3,894 raw RNG draws compared.
- [x] Independent v11:949 captures/92 full byte fixtures; eleventh asserted example.
- [x] Nine policy tests;180 workspace/MSRV checks,156 release tests; ten fuzz builds
  and 10,000 ASan executions without findings (leak detection disabled).
- [x] Implementation `1d4ffc8ddcc35b1ed081b763c642b0ef69719323` passed all four
  jobs of [CI 40656926875](https://github.com/Copgunn/DOOM/actions/runs/40656926875).
- [x] Reviewed final `dd1450ba3525ec75db0d7c740d387ca5e8d0cf98` passed
  [CI 40659174203](https://github.com/Copgunn/DOOM/actions/runs/40659174203).
  Correction/merge evidence is recorded in VALIDATION and on PR #14.
- [x] Conditional owner authorization for a regular merge in the M3d3e mission.
- [x] Correction `64cf9a9`, fresh CI 40661503550, regular merge `31b4830` and
  exact post-merge CI 40662401302 verified before M3d3e.

This accepts the declared action/fixture closure only. Full original Shotgun Guy,
drops, other attacks, actual player damage, dynamic sectors and demo compatibility
remain deferred. The next bounded attack/actor closure requires its own source
investigation and owner scope; no further gameplay is authorized by this milestone.

## M3d3e — Chaingunner attack and refire foundations

Branch `feat/m3d3e-chaingunner-refire` starts at verified master `31b4830`.
[Investigation](M3D3E_REFERENCE.md) commit `b9855f4` precedes runtime.
[PR #15](https://github.com/Copgunn/DOOM/pull/15) completed its authorized regular
merge as `5b9182472123b8ec86b48cf6a4076426a7856f70` after the documented macOS
recovery and fresh exact-head/post-merge gates.

- [x] PR #14 corrected, fresh local/exact-head checks passed, regular merge with
  verified parents/tree/master and successful exact post-merge CI before runtime.
- [x] Exact CPOS 10/4/4/1, sprite 38/fullbright frames, independent shots, shotgun
  sound before internal facing, source spread/damage/impact/RNG and live re-aiming.
- [x] Genuine refire: facing before RNG, below-40 continuation, lazy null/mobj-
  health/sight checks and immediate see/Chase/Look movement/state actions.
- [x] Strict owner/state/reference/coordinate/work/counter preflight; canonical
  error atomicity; current-shot planning without persistent cache or world clone.
- [x] 67 compiled original bodies, 153 scenarios exercising all 20 supplied maps;
  268,606 actor fields, 63,308 ordered observations and 10,697 raw RNG draws.
- [x] Separate 2,048-case controlled-dependency C boundary probe, including 40,
  which is absent from the authentic gameplay LUT; no production RNG override.
- [x] Independent v12: 1,544 captures/110 full byte fixtures; asserted twelfth
  example with digest `54ac5f9c158c8e29`; all historical v1-v11 suites preserved.
- [x] Ten policy tests; 192 workspace/MSRV checks each, 168 release tests; all
  eleven fuzz builds and 10,000 ASan executions without findings, leaks disabled.
- [x] Implementation `e160f89979f6f8eebcf14869ebf1aaf0beedcc7e` passed all four
  jobs of [CI 40667321344](https://github.com/Copgunn/DOOM/actions/runs/40667321344).
- [x] Reviewed final `ae78d76cae69b4f36811110009454df479a33cc6` passed
  [CI 40668964558](https://github.com/Copgunn/DOOM/actions/runs/40668964558).
  Documentation correction and CI-recovery evidence are recorded below and on PR #15.
- [x] Conditional owner authorization for a regular merge in the M3d3f mission.
- [x] Documentation correction `d1150956`, justified CI-only recovery `08e27f72`,
  fresh local checks and all-four [CI 40733786524](https://github.com/Copgunn/DOOM/actions/runs/40733786524).
- [x] Regular merge `5b918247`, ordered parents/tree/master verified; all-four
  [post-merge CI 40735017337](https://github.com/Copgunn/DOOM/actions/runs/40735017337)
  passed before the M3d3f branch, investigation commit or runtime.

Acceptance covers this bounded fixture/action closure. Full Chaingunner/type 10,
drops/pain/death/raise gameplay, SpidRefire, other attacks, actual player damage,
dynamic sectors, demos and rendering remain deferred. The next focused milestone
requires a separate source investigation and authorization.

## M3d3f — Demon melee attack foundations

Branch `feat/m3d3f-demon-melee` starts at verified merged master `5b918247`.
[Investigation](M3D3F_REFERENCE.md) commit
`3fac94715e8e22a55bf3f3448155c4b6129dfd4d` precedes runtime `e140b3bd`.
The separate owner review completed; the M3d3g mission conditionally authorizes
PR #16 regular merge through the correction/fresh-check/post-merge gates below.

- [x] PR #15 recovery, regular merge, parents/tree/master/integrity and exact
  successful Linux/Windows/macOS/MSRV post-merge gates completed first.
- [x] Explicit profile 13: labelled unshootable first native 136 owner and unbound
  target 135; original Demon/Spectre metadata recognized, full types excluded.
- [x] Source SARG sprite 39/frames 4,5,6 without fullbright; exact 8/8/8;
  three distinct faces; Chase selection sound 52 before entry; return to Chase.
- [x] Separate selection/strike checks, strict metadata-radius boundary and real
  REJECT/BSP sight, source shadow draws and `(R%10+1)*4` direct damage.
- [x] Existing damage/lifecycle services preserve thrust, pain/death, reaction,
  retargeting/threshold and original RNG; no invented hitscan impact or slot demand.
- [x] Bounded copied-RNG preflight plus actual post-Face range query; complete
  canonical rejection, including late damage dependency failures and expiring tics.
- [x] 68 compiled original bodies; 104 scenarios actually exercise all 20 supplied
  synthetic maps. 170,118 actor fields, 5,060 raw RNG draws including setup and
  27,039 ordered instrumentation rows; compiled inventory is not branch coverage.
- [x] Separate 36 original-C radius-divergence component vectors; no public mutable
  collision-radius override and no inclusion in gameplay/canonical capture totals.
- [x] Independent DSIM-v13: 1,309 captures/169 complete byte fixtures; all v1-v12
  expected bytes, generators, encoders and historical Rust helpers preserved.
- [x] Nine public Rust policy tests and three private component/error tests;
  nine-scenario asserted example with independent digest `b6c312bf5fbcc7dc`.
- [x] All twelve ASan fuzz targets build; Demon fuzz uses public synthetic maps,
  at most eight operations/24 individual tics and per-error canonical equality.
- [x] Final capacity-aware Demon ASan: 10,000 executions in 921.9 seconds without findings;
  six public seeds, max_len 65, RSS 2048 MiB, timeout 20s; leak detection disabled.
- [x] Fresh workspace/MSRV 205 each, release 181, thirteen C/encoder pairs and
  thirteen asserted debug/release examples; build/format/Clippy/rustdoc/integrity.
- [x] Implementation b3d5aa555f73b69633cf31d6a3f509295dc692f6 passed all four
  jobs in [CI 40741692813](https://github.com/Copgunn/DOOM/actions/runs/40741692813).
- [x] [PR #16](https://github.com/Copgunn/DOOM/pull/16) opened draft on the verified base.
- [x] Capacity-aware fuzz setup reaches all 64 slots before mutational operations;
  final ASan result is recorded in [VALIDATION](VALIDATION.md).

Reviewed head `16c3ef91df81e99fc56339e6b722816fbbe2bb5d` passed all four jobs in
[CI 40745375088](https://github.com/Copgunn/DOOM/actions/runs/40745375088). The
M3d3g mission conditionally authorizes PR #16 regular merge after documentation
corrections and fresh mandatory local/exact-head CI, followed by verified merge
and four-job post-merge CI. See [VALIDATION](VALIDATION.md); no auto-merge.

Normal timing only: source global fast/Nightmare SARG timing is not admitted.
Full Demon/Spectre pain/death/raise closure, real player damage/scoring, other
attacks, dynamic sectors, demos and rendering remain deferred. A subsequent
bounded Demon pain/death-state investigation is assigned by M3d3g, after the
PR #16 correction/merge/post-merge gates; it is not implemented by M3d3f.

## M3d3g — Demon pain, death and corpse-state foundations

Starts from verified master`75b2a2bc` after PR #16 correction `63efb2a5`, successful
CI 40761281692 and exact post-merge CI 40762344038. Source investigation `2500650d`
precedes runtime. See [M3D3G_REFERENCE](M3D3G_REFERENCE.md).

- [x] Explicit profile 14, named shootable native 136, copied source Demon pain/death
  metadata and declared source-body/effect roles; original full types 12/13 blocked.
- [x] Pain180 with real LUT, 2/2 and delayed sound 26, reaction/JUSTHIT/retarget/Chase.
- [x] Ordinary8/8/4/4/4/-1, first timer 5..8, delayed scream64, immediate corpse/
  dropoff/quarter height, later Fall, stable terminal storage/physics/spatial lists.
- [x] Shared direct/hitscan damage, knockback, null/self/distinct relationships,
  current post-physics melee, thinker/spatial ordering and generation-safe targets.
- [x] Atomic failures, capacity, unsupported players/scoring/raise/other attacks;
  no new damage engine or per-tic full-state clone.
- [x] Genuine original-C331 scenarios/21 exercised maps/68 compiled bodies;
  517,192 actor fields/36,316 raw draws/103,942 ordered rows; separate3 boundary probes.
- [x] Independent v14: 3,972 captures/727 full bytes, field/byte/digest diagnostics;
  v1-v13 expected data and twelve historical Rust helpers frozen.
- [x] Eleven public safety tests plus 3 private fault-injection tests; separate
  private scalar-C comparison. Nine-scenario asserted debug/release example,
  independently derived pain/death digest eecc4b4dcc4fbca5.
- [x] All 13 ASan fuzz targets build; bounded new lifecycle target/seed recipe.
- [x] Fresh local workspace/MSRV 221 each, release 197, all fourteen reference pairs/
  example comparisons, build/format/Clippy/rustdoc and source preservation.
- [x] Bounded lifecycle ASan: 10,000 executions in 1,002.9 seconds without findings;
  twelve fresh public 65-byte seeds, capacity setup, max_len 65/RSS 2048 MiB/timeout 20s;
  leak detection disabled.
- [ ] Verify exact implementation/final-head Linux/Windows/native macOS/MSRV CI.
- [ ] New M3d3g PR ready for separate owner review; OPEN/UNMERGED/no auto-merge.

COUNTKILL/global statistics, full original stand/run actor execution, Spectre,
other autonomous monsters/cross-profile projectiles, raise/resurrection, real
players, dynamic sectors/specials, multiplayer and demos remain excluded. These
are explicit admission boundaries, not silently successful callbacks. A separate
next investigation could cover original SARG stand/run animation closure; it is
not authorized for implementation in this milestone.
