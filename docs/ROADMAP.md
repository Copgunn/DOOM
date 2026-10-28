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

Status: **M3a/M3b implemented, validated and merged; M3c implemented, validated and merged; M3d1 merged; M3d2 merged; M3d2b implemented and validated, awaiting owner review**.
M3a [PR #4](https://github.com/Copgunn/DOOM/pull/4) merged by owner-authorized
regular commit `764dd9605fa3af3befc133287e9949930981f469`; fresh and post-merge CI
passed before branching M3b. Exact evidence is in [validation](VALIDATION.md).
M3 as a whole remains incomplete.

| Phase | Scope | Status |
| --- | --- | --- |
| M3a | Grounded player XY, wall sliding, solid actor collision | Implemented, validated, merged |
| M3b | Static multi-sector openings, BSP, XY/Z and gravity | Implemented, validated and merged |
| M3c | Actor states, type-driven spawning and death | Implemented, validated and merged |
| M3d | Combat subphases below | M3d1/M3d2 merged; M3d2b implemented for review; later work planned |
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
| M3d2b | Bounded original projectile foundations | Implemented and validated; owner review pending |
| M3d3 | Monster targeting/perception, movement, attacks and AI | Planned |
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
- [ ] Owner review/acceptance; PR must remain open and unmerged.

No full plasma rifle, rocket/BFG/splash, monster attacks, armor/player damage,
renderer, scripting or engine-limit expansion is included. Recommended next
bounded work is M3d3 source perception/target acquisition before attack callbacks;
actual player damage requires its separately specified acceptance gate. M3d4
remains integrated/advanced combat, and M3e remains moving sectors/map specials.
