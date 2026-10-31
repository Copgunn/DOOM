# M3g11 implementation handoff — incomplete, owner requested another agent finish

The owner paused this run to conserve usage and requested the branch be started
for another agent. Continue the original M3g11 mission; this is a work-in-progress
branch, not a completed delivery. No final PR has been opened or merged.
Read AGENTS.md and the owner's full mission before proceeding. Preserve historical
source/evidence and do not merge M3g11 without separate authorization.

## Completed merge gate

PR41 was independently reviewed and regularly merged under explicit authorization.
Reviewed feature: `6dcdf08786f06fccf16bc85b3719c0c4c31545f1`.
Actual merge/baseline: `ab3f300fb0a4b9a8504aba2a47c062cd1deeddf1`.
Ordered parents: `ebbf9e78a9f10111f9ba990631a59dc7e3813fc9`, then reviewed feature.
Tree: `f247acd4668338cdc543263249880aeaf3c9cd1c`, identical to reviewed tree.
Exact-master CI: https://github.com/Copgunn/DOOM/actions/runs/41244891650 .
Linux, Windows, native Intel macOS and Rust1.85 all succeeded; completed checkout
logs were inspected for the actual merge SHA. Branch created only after this gate.
Public/GPL-2.0-only/attribution,165 original upstream files,19 zstd identities,
historical source evidence and20 documentation archives verified at baseline.
No force push, rewrite, protection bypass, branch deletion or auto-merge used.

## Implemented checkpoints

- `2902951`: source-first genuine C evidence and independent DSIM27 encoder.
- `1f290a3`: canonical animated bonus actors, isolated policy11.
- `7dfaf5e`: +1 capped resources through existing player/armor/DamagePlan owners.
- `e39c6c1`: existing level/player counted-item statistics, atomic spawn/contact.
- Following checkpoint: integrated source-backed tests/example and CI commands.
- Final WIP checkpoint: unvalidated fuzz additions and this handoff.

Policy `PlayerDemonBonusesV1` wire11/context
`PLAYER-DEMON-1-MEDIUM-GROUNDED-BONUSES-v1`, explicit `decode_bonuses`, DSIM27.
Old wires/policies/DSIM20–26 remain unchanged. One player, at most one Demon,
64 actors and16 total world items retained. Existing scheduler/store/RNG/owners.
Health bonus actor45/DoomEd2014/sprite60/states816..821; armor bonus46/2015/61/
822..827. Exact0x800001 flags,6x6-tic loops, frames0,1,2,3,2,1; map RNG reused.
Health/armor cap200; green type1 may exceed100 only under policy11. Ordinary
GiveBody>=100 refusal and GiveArmor100/200 thresholds unchanged. Green /3,
blue /2, raw knockback through shared DamagePlan. Map total at spawn, player count
before removal; cap pickups still count/remove/message/bonus/sound.
New-policy map spawning reuses bounded DemonPlan staging and shared spawn service.

## Source evidence and observed targeted checks

`tools/reference/m3g11_reference.py`, `m3g11_snapshot_goldens.py`,
`tests/reference/m3g11_contract.json`, `m3g11_snapshot_contract.json`,
`tests/support/m3g11.rs`, `docs/M3G11_REFERENCE_INVESTIGATION.md`.
31 scenarios/8800 scripted tics,29 ordinary worlds,6 sustained encounters,
2 explicitly controlled resource200 timelines;48 direct resource/damage boundary
components plus4 capped-repeat and4 depletion/reacquisition rows.766 captures,
33 independent full-byte fixtures,8831 complete event frames/117473 ordered fields.
102 selected source bodies +3 math bodies; main91 entered functions/216692 entries,
component25/1521. Function entries are NOT branch coverage. Genuine source strict
arithmetic and ASan/UBSan equality passed before runtime implementation.

Main comparisons:128 bonus timer randomizations,590 animation entries,100 counted
grants,128 map-total increments,42 health/58 armor bonus effects,204 shots,
3399 gameplay draws,606 weapon draws,97 damage requests,49 player damage blocks,
31 nonzero absorptions,4 depletions. Queue writes remain source-only.
T40 raw300 physical post-hit exceeds30-unit momentum admission: only pickup is
Rust-matched (identical toT4), followed by atomic rejection. Do not claim its
post-hit result is admitted. Other47 direct T post-action components compare.

At last observed checkpoint,17 `bonus_tests` passed (plus prior3 actor tests),
including full source timelines, controlled cap movement, source repeat/depletion
components, late spawn RNG/statistics rollback, multiple collection rollback,
pending retirement, animation+damage rollback, foreign instance references,
stale generation/reuse, paused step/advance and malformed DSIM27. Historical21
armor tests also passed at prior checkpoints. No old expected bytes changed.
The new example debug/release comparison and all38 fuzz builds were running at
handoff; inspect logs before claiming completion. Final full regression is pending.

## Workspace artifacts and running checks

Same environment: `/workspace/m3g11-work/` has all preparation, baseline reports,
logs and scripts. Rust environment:

```
export PATH=/workspace/toolchains/cargo/bin:$PATH
export CARGO_HOME=/workspace/toolchains/cargo
export RUSTUP_HOME=/workspace/toolchains/rustup
```

Toolchains stable1.99.1,1.85.0,nightly and cargo-fuzz0.13.2 installed.
Shell network commands require the execution tool's additional network permission.
No credentials should be printed or copied. Git push authorization already works.

Ongoing tool sessions: historical source suite52407, package-E tests/example13096,
fuzz build37126. They may complete after handoff; inspect process/log state first.
Logs: `m3g11-historical-source.log/json`, `package-e-tests.log`,
`package-e-example.log`, `final-fuzz-build.log`. Historical65 commands are
independent of the pending Rust-only work; don't rerun needlessly if inputs match.
The frozen inventory has21 historical BASE pairs,31 source/encoder pairs total,
one reuse encoder plus metadata/pack commands,65 commands/32 encoders overall.
M3g10 and new M3g11 source pairs run separately in CI. Do not conflate inventories.

Baseline performance completed for18 unchanged drivers: `before-measure/`.
`baseline-binaries/manifest.json` preserves38 release binaries from actual merge,
with compiler/hash/tree provenance. Repeat18 drivers after final code, then use
`tools/measure_m3g10_controls.py` for interleaved matched controls. Prior M3g10
+4.9% original-ID timing remains a maintenance concern, not proven repaired.
Baseline validation reports include stable620/MSRV620/release587,36 debug/release
examples, all37 old fuzz builds and5 preliminary campaigns/10240 executions;
these are BASELINE results, not final M3g11 results. Leak detection was disabled.

## Required remaining work

1. Inspect/finish new example debug/release and fuzz builds. Fuzz target uses maps
   14/18/17 with real movement, correct player start0/-300; independent200-cap
   fixtures are read-only corruption inputs. New fuzz target/seed script have NOT
   yet run. Build all38 final targets; run bonus and representative ownership,
   pickup, armor, weapon, melee and diagnostic campaigns. Report actual campaigns,
   seeds, budgets, executions, timeout/RSS/ASan options and findings. Never claim
   leak checking with `ASAN_OPTIONS=detect_leaks=0`.
2. Complete matched release performance using existing3-process method. New example
   has ordinary workloads/initialization/frozen snapshot mode. New measurement
   driver NOT written. Cap200 timing needs a PRIVATE controlled unit-component
   harness, not public state setters. Record individual samples, allocations,
   reallocations, requested bytes, RSS, deterministic digests and staging costs.
3. Complete all local build/fmt(including fuzz)/Clippy-D/rustdoc-D, workspace,
   Rust1.85 and release tests; all historical examples plus new one; parser and
   generator checks;65 source checks plus M3g10/new source and encoder checks,
   strict arithmetic, ASan/UBSan and weapon formulas. Keep asset tests ignored.
4. Finish README/ROADMAP/ARCHITECTURE/COMPATIBILITY/ACTOR_IDENTITIES/STATE_FORMAT/
   TESTING/VALIDATION/index updates and a proper engineering/fuzz/performance
   record. Source note still contains prospective wording. Follow documentation
   authority map; do not rewrite historical records. Move/remove this temporary
   handoff once authoritative records contain final results.
5. Independently review full runtime diff and conditional diagnostic invariants,
   map filtering, health/body ownership, ordinary thresholds, exact ordered events,
   no implicit snapshots, rollback and all preservation. Preserve165 originals,
  19 zstd and all historical fixtures/archives; no recompression. Final protection
   inventory must include M3g10 evidence added in baseline.
6. Commit meaningful remaining work, push one branch, open ONE integrated final
   PR when ready, non-draft/open/unmerged/auto-merge off. Verify Linux, Windows,
   native Intel macOS and Rust1.85 exact feature/synthetic merge checkout logs;
   ordered synthetic parents and tree; remote HEAD and clean working tree.
   Do NOT merge final PR. Provide user-required A merge report and B engineering
   report, actual evidence/limits only, with recommended next bounded milestone.

The user has authorized all implementation/validation/push/one final PR, but NOT
M3g11 merge. This run stops because the owner explicitly requested the handoff,
not because completion or all final gates have been achieved.
