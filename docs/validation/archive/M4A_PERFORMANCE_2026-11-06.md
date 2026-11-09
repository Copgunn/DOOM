# Archived M4a performance observations, 2026-11-05–06

Discarded as performance evidence because an orphan test process was active.
This preserves the historical observations, including the old CI-budget runs;
they were not repeated by the replacement campaign. Current measurements are
in [the M4a evidence record](../M4A.md#matched-performance).

The original section is retained below, with its relative investigation link adjusted for this archive’s directory.
Original section SHA-256 (before that link adjustment): `ae722bdda76c3e8d75d8466301e5e90d9a6913b2da0800d4730ac11b993a031e`.

## Matched performance

Recorded 2026-11-05 to 2026-11-06 (local time, America/Los_Angeles), on the local host, one measurement at a time (plan step, decision H12). No build, test or fuzz job ran during a timing round, but the host was **not idle in the strict sense**: an orphan test process (pid 222172, `doom_sim-13f971279bca8f16 --exact player_episode1_movers_tests::rule_1_admits_exactly_the_29_part_1_specials`, running since 16:47) was pinned at 100% of one core throughout and could not be killed from the session (the owner kills it). It accounts for about 1.0 of every load average below; the harness is the other 1.0. 12 logical CPUs, so the pinned core did not share a CPU with the harness by construction, but this is not proven. The 1-minute load average was 3.92 before round 0 (still decaying from the two release builds, which had finished about 1.5 minutes earlier), 2.33 / 2.11 / 2.12 / 2.10 / 2.10 / 2.13 / 2.18 / 2.06 / 2.29 / 2.32 at the starts of rounds 1 to 10 and 2.05 after the last round. Host: Intel Core i7-8750H @ 2.20 GHz, 6 cores / 12 logical CPUs, 7.7 GiB, Linux 6.18.58.1 WSL2, rustc 1.99.1, the default release profile (no `[profile.release]` in `Cargo.toml`), no CPU, frequency or core isolation. `df -h /mnt/c` before: 90G available (63% used); after cleanup: 90G available (63% used). These are host observations, not budgets.

**Method** (as the M3h9c record): master `0ecfe4b58da2bab67f56eb62ac442bb35d4060c7` (merge of PR #56; a fresh `git worktree add ~/scratch/m4a/wt-master origin/master` with `CARGO_TARGET_DIR=~/scratch/m4a/perf/target-master`) against the branch `feat/m4a-episode-1` at `08ac31c` (default target dir). Both built with `cargo build -p doom-sim --examples --release --locked` before any timing (clean worktree). The `measure_legacy_weapons.py` program was compiled once per tree with that script's `rustc` command (`--extern` rlibs of that tree's `release/deps`, newest by mtime; both trees print digest `63003ae3ffaa78af`). Harness (not committed): `~/scratch/m4a/perf/harness.py`, analysis `~/scratch/m4a/perf/analyze.py`; raw per-workload, per-sample JSON `~/scratch/m4a/perf/raw.json` (all 5,412 samples, per-sample non-timing signatures and the load averages), analysis output `~/scratch/m4a/perf/analysis.txt`. Run: `python3 harness.py run 11`.

Workloads: **246 matched workloads**. The `tools/measure_*.py` scripts and the examples give 241, not the record's 239: the previous table counts `player_real_map` as 6 (the script lists 4 `--measure` workloads and 3 `--admit` rooms, 7) and `player_barrels` as 4 (the script lists 5, including `dense64_lift`). All 241 are included so nothing is dropped silently (which two the earlier record left out cannot be told from the record; the other 27 groups have exactly the earlier counts), plus 5 new wire-22 real-E1M1 workloads: `player_level_run --wad /mnt/d/DoomIWADs/DOOM.WAD --map E1M1 --demo <WALK|FIGHT|CHAOS|BARRELS|idle>.lmp --out <dir>` (default policy, no `--captures`), timed by a monotonic timer (`time.perf_counter`) around the process. 11 rounds; each round runs every workload once per tree as a process pair, order alternating AB/BA by workload and round (`(workload + round)` parity); 5,412 samples. The examples were called directly with the measure scripts' arguments (no `wait4` launcher, so no RSS). Timed field: each example's own timer; the `diagnostic_free` `--measure <label> <profile> advance` runs, the `--admit` rooms and everything else use `ns` (admit rooms: the `ns` field). "Non-timing" fields are every `key=value` token except `ns` and keys ending `_ns` or `_us`. For the five level runs the comparison is the SHA-256 of `digests.txt` and `stats.txt`.

**Every non-timing output line of every sample is identical between the trees** (30,712 non-timing fields over the 241 example workloads; 5,412 samples; each workload stable across its 11 samples in each tree), and `digests.txt` and `stats.txt` of the five level runs are byte-identical.

| Example | n | Timed field | Median delta of medians | Range of the delta | Median delta of minima |
| --- | ---: | --- | ---: | --- | ---: |
| canonical_demon | 1 | `step100_with_query_us` | -2.1% | -2.1..-2.1 | +4.1% |
| context_ownership | 2 | `retirement100_us` | -1.4% | -1.5..-1.3 | +0.6% |
| demon_expanded_map | 9 | `step100_us` | +1.9% | -14.4..+19.1 | +0.7% |
| game_instance | 2 | `step100_us` | +1.0% | +0.9..+1.2 | +0.7% |
| map_extent | 11 | `step100_us` | -1.7% | -14.4..+10.9 | -0.8% |
| map_scale | 9 | `step100_us` | +3.3% | -17.1..+15.0 | +3.8% |
| original_registry | 6 | `step100_us` | -0.9% | -5.2..+2.9 | +1.3% |
| diagnostic_free | 10 | `ns` | -0.1% | -8.8..+5.0 | +1.0% |
| original_player | 4 | `ns` | -2.4% | -24.8..+1.9 | -2.0% |
| player_commands | 9 | `ns` | +0.8% | -4.3..+8.0 | +1.5% |
| player_knockback | 6 | `ns` | -3.3% | -5.3..+10.0 | +0.5% |
| player_demon | 11 | `ns` | +3.1% | -10.2..+19.3 | +4.6% |
| player_demon_melee | 9 | `ns` | +5.0% | -33.1..+9.5 | +0.5% |
| player_pistol | 15 | `ns` | +0.3% | -7.4..+14.5 | +7.7% |
| player_weapon_pack | 25 | `ns` | +0.0% | -16.1..+42.6 | -0.0% |
| player_world_pickups | 17 | `ns` | +3.6% | -6.2..+19.1 | +3.8% |
| player_armor | 19 | `ns` | -0.4% | -7.0..+13.0 | +1.6% |
| player_bonuses | 13 | `ns` | +2.8% | -6.0..+11.6 | +1.7% |
| player_monsters | 7 | `ns` | +2.4% | -5.4..+7.5 | +0.9% |
| player_imps | 7 | `ns` | +1.5% | -7.4..+14.2 | +4.0% |
| player_map_scale | 8 | `ns` | +2.3% | -7.9..+7.5 | +0.4% |
| player_heights | 6 | `ns` | -1.4% | -5.7..+7.3 | +1.1% |
| player_diagonal | 3 | `ns` | -0.9% | -6.7..+5.9 | -1.2% |
| player_real_map | 7 | `ns` | +1.2% | -8.1..+18.2 | +6.8% |
| player_specials | 6 | `ns` | +0.4% | -3.2..+7.8 | +2.6% |
| player_movers | 9 | `ns` | +10.5% | -5.2..+22.4 | +9.9% |
| player_e1m1 | 4 | `ns` | +16.1% | +1.5..+21.2 | +12.0% |
| player_barrels | 5 | `ns` | +5.5% | -1.1..+15.0 | +3.7% |
| legacy_pistol | 1 | `ns` | -7.0% | -7.0..-7.0 | -0.6% |
| level_run_e1m1 | 5 | `wall time` | -2.9% | -3.7..+5.6 | +2.1% |

Over all 246 workloads the median of the median deltas is +1.5% and the median of the minimum deltas +2.0% (the same over the 241 example workloads). Host noise is much larger than in M3h9c: the load was about 2.1 instead of 1.0 and most timers are 0.1 to 1 ms, so single-example medians swing by tens of percent in both directions (for example `player_weapon_pack` -16.1..+42.6% with a median of +0.0%); the minimum deltas and the callgrind counts below are the better evidence. The `player_e1m1`, `player_movers` and `player_barrels` groups read +16.1%, +10.5% and +5.5% by median.

**Slowdowns over 3%.** 102 workloads (of 246) slowed by more than 3% by median (list in the scratch analysis, deleted with the scratch); callgrind (`valgrind --tool=callgrind`, same binary and arguments, once per tree; `cg.py`, output `~/scratch/m4a/perf/cg/*.out`, `res-*.json` and `cgA.log`/`cgB.log`) ran for each. Whole-process instruction counts differ by 1% or less in 98 of them (at most +0.98%, `player_demon_melee repeated`): noise by the M3h9c criterion. In 46 of those 98 the `Simulation::advance` inclusive count (where callgrind reports one) is 1.5% or more higher on the branch (for example `player_commands idle`: 659,388 to 681,288 of 2.4M total; `player_armor green_deplete` +4.1%), a small real per-tic cost below 1% of the process. Four are above 1% in total Ir and are a real instruction-count change:

| Workload | Total Ir master | Total Ir branch | Delta | `Simulation::advance` Ir master to branch | Delta |
| --- | ---: | ---: | ---: | --- | ---: |
| player_e1m1 gallery_patrol | 452,465,336 | 477,404,376 | +5.51% | 222,702,689 to 248,022,697 | +11.37% |
| player_barrels dense64_fight | 2,130,983,268 | 2,211,769,449 | +3.79% | 1,704,811,498 to 1,785,679,884 | +4.74% |
| player_barrels dense64_lift | 2,622,550,724 | 2,700,323,722 | +2.97% | 2,365,775,605 to 2,443,644,279 | +3.29% |
| player_demon_melee corpse_continuation | 8,694,990 | 8,815,190 | +1.38% | 6,462,515 to 6,585,909 | +1.91% |

(The two `player_level_run` rows are noise: `WALK` 12,866,164,425 to 12,961,615,985, +0.74% with `advance` 371,749,067 to 391,884,724, and `FIGHT` 21,856,383,032 to 21,850,269,719, -0.03% with `advance` 669,302,331 to 690,630,802.) Self-cost differences from `callgrind_annotate` (branch minus master) name the responsible functions: `doom_sim::original_registry::require_role_for` (+24.3M in dense64_fight, +18.8M in dense64_lift, +83k in corpse_continuation), `OriginalActorType::descriptor` (+18.1M, +14.3M, +6.7M in gallery_patrol), `doom_sim::e1m1_rules::decoration_states` (+4.2M in gallery_patrol, +9.0M in dense64_lift) and `PlayerWeaponState::validate_for` (+3.2M in gallery_patrol), plus a shift of 49M (dense64_fight) between two compiler-generated `Chain`/`Map` iterator instantiations (inlining, no net cost) and changes in the `advance_expanded_demons` closures. These are the role/descriptor checks of wire 23 on every actor of the large E1M1-style rooms (decorations and barrels); this is a measured per-tic instruction cost on the branch, not noise. The M3h9c codegen-unit cause (BTreeMap inlining) is not what is seen here: `diagnostic_free` and `demon_expanded_map` are within 0.1% in total Ir. Medians of up to +40% in the table for small workloads (`player_weapon_pack full_encounter` +42.6%) correspond to total Ir changes of -0.09% and are timer noise.

**Cause of the four** (source reading, no experiment): the per-actor validation tables shared by every
wire gained wire-23 arms. `require_role_for` (`original_registry.rs`) checks `world_pickups::kind_for`
and two further guarded arms (the Spectre and rocket, the candle and firestick) before the E1M1
decoration arm; `OriginalActorType::descriptor` covers more types; `decoration_states` runs the same
loop over a larger table; `PlayerWeaponState::validate_for` takes the `episode1` flag and validates
the backpack and ammo bounds. On wires 19 to 21 every added arm evaluates to the same answer as
before, so the output is identical, at a cost of up to 5.5% of the process instructions on the
largest E1M1-style rooms. No code was changed for this after the matched runs; reordering the arms
or caching the per-type answers is a follow-up for the owner to decide, as the M3h9c codegen
finding was ([G13](../../M3H9C_REFERENCE_INVESTIGATION.md)).

**Per-tic costs on real E1M1 to E1M7 runs** (branch only, not matched; the IWAD read in place; `player_level_run` without `--captures`, five runs each, median wall time; us/tic = median wall / tics, tics = the `digests.txt` line count; `~/scratch/m4a/perf/pertic.py`, raw `pertic.json`; load 2.0 to 2.2, the orphan process present). The runner hashes a full DSIM capture every tic, which dominates.

| Policy | Map | Scripts | Tics (total) | us/tic range | us/tic median |
| --- | --- | ---: | ---: | --- | ---: |
| `episode1` | E1M1 | 5 | 10,034 | 2,436 to 3,115 | 2,785 |
| `episode1` | E1M2 | 5 | 9,099 | 6,598 to 7,117 | 6,974 |
| `episode1` | E1M3 | 5 | 13,755 | 9,048 to 10,505 | 9,372 |
| `episode1` | E1M4 | 5 | 9,391 | 6,074 to 6,736 | 6,485 |
| `episode1` | E1M6 | 5 | 12,747 | 11,268 to 11,972 | 11,576 |
| `episode1` | E1M7 | 5 | 12,944 | 7,176 to 8,380 | 8,096 |
| default (wire 22) | E1M1 | 5 | 10,034 | 2,454 to 2,839 | 2,608 |

All 30 episode1 scripts together: 67,970 tics, median 7,146 us/tic. The largest single run was `M6WALK` (7,840 tics, 89.2 s, 11,374 us/tic); the cheapest `E1M1 CHAOS` (5,001 tics, 12.2 s, 2,436 us/tic). For the same five E1M1 scripts the episode1 policy costs 2,785 us/tic against 2,608 us/tic under the wire-22 default (+6.8% by median; individual scripts: BARRELS 2,531 vs 2,672, CHAOS 2,436 vs 2,466, FIGHT 3,012 vs 2,839, WALK 3,115 vs 2,608, idle 2,785 vs 2,454). No example prints a simulation-alone per-tic figure for these runs, so none is reported. The genuine strict C runner (from `~/scratch/m4a/real/keep/strict32/*.json`, field `us_per_tic`, not re-run): the five E1M1 scripts 10.0 to 15.5 us/tic (`strict32`; plain `strict` 10.0 to 13.5), E1M2 14.8 to 34.9, E1M3 41.9 to 48.6, E1M4 22.4 to 32.8, E1M6 31.6 to 54.2, E1M7 40.3 to 68.2 us/tic (`strict32`); the AddressSanitizer variant is 28 to 153 us/tic.

**CI budget** (branch tree, alone, one at a time, after the rounds; load 1.0 to 1.7 with the orphan present):

| Command | Wall time | Result |
| --- | ---: | --- |
| `python3 -I tools/reference/m4a_reference.py --work ~/scratch/m4a/perf/ci --check --keep-dumps --selftest 300` | 33.6 s (build 11.1 s, runs 21.5 s) | exit 0; `m4a_synthetic_results.json` matches (30 scripts); selftest on DOORS1 passes |
| `python3 -I tools/reference/m4a_snapshot_goldens.py --dumps ~/scratch/m4a/perf/ci/runs --check` | 32.7 s | exit 0; 18 fixtures, `check passed: 19 files` |
| `python3 -I tools/reference/check_player_episode1.py --binary target/release/examples/player_level_run --out ~/scratch/m4a/perf/ci-rust --example-args "--policy episode1" --require` | 36.3 s | exit 0; all checks passed |

Cleanup: the master worktree and `~/scratch/m4a/perf/target-master` were removed; the raw JSON, harness, analysis, callgrind outputs (37 MB) and this report are kept. Limits: the orphan process and a load of about 2.1 (against 1.0 in M3h9c) make wall-time deltas of single small workloads unreliable; the first round started while the load average was still decaying from the builds; callgrind totals are exact instruction counts but are not time.

