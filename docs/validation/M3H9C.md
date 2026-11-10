# M3h9c evidence record

Observed results of the M3h9c milestone (`PlayerMonstersLevelV1`, wire 22, DSIM38): the real
E1M1 run in step with the genuine linuxdoom-1.10 engine, built whole. Only observations
appear here. The [M3h9c investigation](../M3H9C_REFERENCE_INVESTIGATION.md) owns the source
facts, the decisions (§6) and the plan (§9); [format](../STATE_FORMAT.md) owns DSIM38,
[support](../COMPATIBILITY.md#original-level-runs-the-player-start-in-lump-order-and-the-full-player-think-wire-22)
the admitted behavior and [testing](../TESTING.md#m3h9c-whole-engine-runner-and-the-real-e1m1-comparison)
the commands. Exact review state and CI belong to GitHub.

Timings in the earlier sections are rough: the local host was not guaranteed idle for them,
and they are not performance measurements. Only the [matched performance](#matched-performance)
section was measured alone on the idle host. The real-E1M1 runs are **local only** (commercial IWAD read in
place from `DOOM_IWAD_DIR`, never copied); only counts, tic numbers and digests are recorded
and no map-derived value (coordinates, angles, lump bytes, per-object data) appears. The
claim is limited to the five scripted inputs below on medium skill; it is not a claim for
every input, skill or map. The address-checked builds ran with `ASAN_OPTIONS=detect_leaks=0`,
so LeakSanitizer was disabled, not passed.

## IWAD facts

Phase 1 probe (wire 21, the real E1M1, unstripped), 2026-11-05/06:

- 143 THING records, 91 spawned on medium, 43 skill-filtered; 92 actors with the body;
  RNG index 144 after setup.
- Idle for 2,100 and 35,000 tics: no refusal. 32 pseudo-random command streams: no refusal
  (the player died in 9 of the long runs).
- A capture is 4,139 fields, about 203 KB (about 63.5 KB of it `map.*`). About 45 us per
  advanced tic, 0.54 ms per snapshot, 0.74 ms with the digest.

## Original-C evidence

`tools/reference/m3h9c_build.py` builds the headless genuine runner: 34 whole genuine files,
6 extracted `r_main.c` bodies, 8 declared LP64 ABI adapter lines (five sites; the z_zone.c:440 one was added after the first CI run, investigation §12) in every build plus 6
sanitizer-only adapters (`p_mobj.c` lines 794 and 671 angle products, `p_map.c:1221`,
`p_enemy.c` lines 816, 839 and 860 int angle spreads; the `p_enemy.c` ones were found by the
UBSan build on the synthetic chaos run), and 43 function plus 17 variable stubs (the contract's `stub_counts`: 29 I/O-only or unreachable, 6 read-only, 8 writers whose
writes `P_Ticker` cannot observe, among them `I_Quit` and `WI_Ticker`; the contract lists
them). `--wrap` observers cover `P_AddThinker`, `Z_Free`, `Z_Malloc` (zero-filling),
`P_SetThingPosition` and `P_InitThinkers`. The driver calls `G_DoPlayDemo` directly, loops
`G_Ticker` with `gametic++`, stops on `ga_completed`, turns `I_Quit` into a `longjmp`, runs
`Z_CheckHeap` every tic and uses a 6 MB zone that was never raised. `-m32` is not available
on the host. `valgrind` memcheck on an early strict build: 0 errors (300 idle tics and a
pistol run).

**Real E1M1** (local, three variants: strict, `--overflow-check`, `--address-check`). The
dumps were byte-identical across the three variants for every script.

| Script | Tics | End | `prndindex` load to end | Thinkers / mobjs at load | Thinker peak |
| --- | --- | --- | --- | --- | --- |
| idle | 2,100 | marker | 144 to 253 | 96 / 92 | 96 |
| WALK (`walk_to_exit`) | 766 | exit at tic 765 | 144 to 127 | 96 / 92 | 99 |
| FIGHT (`fighting`) | 1,281 | marker | 144 to 89 | 96 / 92 | 103 |
| BARRELS (`barrels`) | 882 | marker | 144 to 187 | 96 / 92 | 99 |
| CHAOS (`chaos`, seed 20261105) | 5,000 | marker | 144 to 227 | 96 / 92 | 97 |

Microseconds per `G_Ticker` call over the five scripts (idle machine not guaranteed): strict
4.8 to 7.3, overflow-checked 5.2 to 7.8, address-checked 13.4 to 38.9. An earlier run gave
idle 5.8 / 5.5 / 14.5, WALK 6.0 / 7.4 / 24.3, FIGHT 5.3 / 8.3 / 20.6, BARRELS 6.2 / 8.7 /
21.2 and CHAOS 7.2 / 8.0 / 23.5 (strict / overflow / address).

Script events (tic counts only). WALK: three DR doors open (door thinkers first appear at
tics 136, 593 and 702), 4 damage tics, itemcount 2, killcount 1 at tic 758 (no ATTACK
pressed), exit USE at tic 765. FIGHT: 6 kills at tics 248, 262, 496, 651, 909 and 1136 (five
pistol, one shotgun), the shotgun picked up and auto-selected at 1017, the player survives
at 75 health. BARRELS: barrels explode at tics 158, 715, 727 (chain from 715) and 821; an
imp is killed by the 727 blast; the player is never hurt by an explosion. CHAOS: no death,
no exit, itemcount 4. The self-test (`--selftest 100`) reports the first difference at tic
100, column `prndindex`, in the C dumps.

**Finding (zone fill).** Before the zone allocator zero-filled, the BARRELS strict and
address dumps differed from tic 517 in `PL count` and `FL newspecial` only: uninitialised
zone fields the genuine code never reads before setting them (`p_plats.c` lines 173, 120,
75, 84, 113; `p_floor.c` lines 282, 231, 242; `p_spec.c` lines 1203, 422, 433). One vanilla
exception exists (`lowerAndChange` with no neighbour, `p_floor.c:242-243`); it is not on
E1M1.

**Synthetic game WAD** (`tools/reference/m3h9c_wad.py` writes
`tests/reference/m3h9c_synthetic.wad`): IWAD ident, 28 lumps, about 60 KB; 67 textures (58
switch names, SKY1-3, 6 walls, door and lift), 1 blank patch, 8 flats including F_SKY1 and
NUKAGE1-3. Map E1M1: 28 sectors, 115 lines (57 two-sided), 172 sides, 110 vertices, 184
segs, 53 subsectors, 52 nodes, BLOCKMAP 19 x 12; 95 THINGs (92 spawn on medium including
the body, 3 filtered): 52 pickups of E1M1's kinds, 27 decorations, 6 barrels, 6 monsters (4
zombiemen, 2 imps); the player start is THING 52 of 95 (42 follow it); angles in 45-degree
steps including 180 and above. 9 movers (8 DR doors, with DR lines on the hall side only,
see the correction in the note's §12), 8 type-48 two-sided scrollers, sector specials 1, 2,
3, 7, 8, 9, 12 and 17, one S1 exit (11) with SW1COMM, minimum sector height 128, at most 2
special lines per 2 x 2 window.

`tools/reference/m3h9c_reference.py` (about 19 to 22 s wall: build about 7 s, runs about 12
to 15 s) ran the three variants: all six scripts byte-identical across variants; 98
thinkers, 92 mobjs and `prndindex` 146 after load.

| Script | Tics | End | Events (tic numbers) |
| --- | --- | --- | --- |
| SYNIDLE | 2,100 | marker | player killed at tic 884/885, `P_DeathThink` afterwards |
| SYNWALK | 1,468 | exit at tic 1467 | 6 kills, 2 doors, nukage damage, secret at 1034, lift ridden |
| SYNFIGHT | 273 | marker | 4 kills including one shotgun |
| SYNBARRL | 581 | marker | chain explosion at 464 and 477, a zombieman killed, an imp hurt |
| SYNCHAOS | 5,000 | marker | `nouse`, death at tic 816 |
| SYNDIE | 1,001 | reborn at tic 1000 | SYNIDLE's start (death at tic 884/885), then USE while dead at tic 1000 (DSIM tic 1001) |

## Independent projection

`tools/reference/level_snapshot.py` projects the C dumps to DSIM38 exactly from the observer
rows (its `--infer` fallback is never used by the comparator);
`tools/reference/level_compare.py` compares the bytes minus the `map.*` and `events.*`
groups; the `map.*` groups (the bytes of the ten lumps each side located, the lump check
of the note's §5) are compared separately, must be equal for a match, and were equal; there
is no tolerance: fields the genuine code has not yet written must read the schema's 0 and
are asserted, not forced. `tests/reference/m3h9c_synthetic_goldens.json` and ten fixtures
`m3h9c_level_*.bin` (SYNIDLE load, tic 885 death and tic 920 death-think; SYNCHAOS tics 816,
851 and 2500; SYNFIGHT 273; SYNWALK 1468, the exit tic; SYNBARRL 465, `A_Explode`; SYNDIE
1001, the reborn request) were produced from C dumps only. `m3h9c_snapshot_goldens.py
--check` passes (11 files).

| Script | Snapshots | Sequence SHA-256 | Final SHA-256 |
| --- | --- | --- | --- |
| SYNIDLE | 2,101 | `85c8069dd63cb160d4ba038c579710df697add317eb82f1057193638a6b071df` | `c7e0743cddbf6cbf972b9efc73d1515ec928ebd9537f26e9666c5ead18726822` |
| SYNWALK | 1,469 | `17a9680ac99e9c5d15a927d38285a75f5a72717a1220fdec87b4b35ff4aa47b6` | `5e12a6655e13e45ccd627b6eaf5a5bb4fec07733f0d1c4ca3563113a4f4cd67a` |
| SYNFIGHT | 274 | `a9e005f3d5af7cde72284c169d31f5c57cc1f85a13d563d55b5d83a798e05fcf` | `fffa46e0f996884d51ea217a56c573f1753606dab01f00b4929855e84e41fc2d` |
| SYNBARRL | 582 | `b519f64f91e0f4bf3c8620276cd6f321c43607971659007c3103d8d23a9d5fab` | `3dfd5a6f16a83bf134eabc2caceb5e58e30190c13be38e00248939b683e99466` |
| SYNCHAOS | 5,001 | `9d4078400d715297aa993ecb8f0b15a8e530cc395eb4f24f425dc6fd4e13038b` | `0eba38f408a4c56ab2c744f49f14abc443067616ba612ed616c75a6772c15dea` |
| SYNDIE | 1,002 | `716e3546a74d755a11b2cdeff0aa22adcbfceee48ac6df862abab963936ef84c` | `13d6d59416a3937349825a05e81f1f25a85448fd9f60b88589df27c303eadeab` |

Order of the commits: the wire-22 Rust writer (4e64d08) preceded the first committed goldens
(cb99f8c), not the other way round as the plan intended. The encoder never reads Rust
output; the corrections made to it while matching are listed here, and each cites C or the
schema. Encoder-side corrections found while matching (all in the encoder, none in Rust): a
new door's `topcountdown` and a new plat's `count` read zone garbage in C before first use
(the DSIM35 schema says 0, decision N6), so the encoder first projected 0 there; with the
driver's ZeroFill C reads 0 and the encoder now asserts it; the link
order was first inferred and then made exact by the observers; the driver gained the add
count at each free (`K.adds`) and `RR` rows to remove the last ambiguity (slot interleaving
of frees and spawns; the pending record at a run's last dump).

## Rust source replay

The Rust example `player_level_run` under wire 22 against the strict C dumps of the **real**
E1M1 (local only). Both sides' per-tic sequence SHA-256 and the final stripped SHA-256 are
equal for every run, and no per-tic witness fired on any real run.

| Script | Snapshots | Result | End | Sequence SHA-256 | Final SHA-256 |
| --- | --- | --- | --- | --- | --- |
| idle | 2,101 | match | marker | `77e2da519ba92053bcae18fc37692822dc0d5a9d3b8b5eaa23666aac47a6628f` | `678e0d293edeb023e509a58a61b373d488fff58f95d9d6f2caecf25385ee5a5b` |
| walk_to_exit | 767 | match | exit at DSIM tic 766 | `ce62cba5dd74953d5a278b9f1c211a75b36f2a0618a160342dc8acca806b4a32` | `3f3384039408abf2d191e715dcaa75f217cf1098c3765f11b8146af9e7644bba` |
| fighting | 1,282 | match | marker | `1636d2f69a4c9178eb63fbda03b44554c6196abce625202804fd0ef268ed8949` | `d899aea2f4328dee7c949e9a53fb2ac10b82696caaa5af674f841dd1eb308122` |
| barrels | 883 | match | marker | `50226bd260f906de01e22563c732ef6920758e9f276509ceef5caee741a50cf5` | `740eac1d000a6950b047cfff984ec0f2996ae1aa355eb27e257b45ad96f7cfcb` |
| chaos | 5,001 | match | marker | `37d7231dba74f2bf98934ec931dc24ce69cef9de016aae44589112f0b5d087ba` | `a35113ea30a812848fb1ef800978f077caaf498941c6ffe3fbd68d4754de523c` |

For `walk_to_exit` the C end is the exit and Rust's `level.exit` is set on the same tic; the
demo has exactly 766 commands, so no refused tic is observed.

Rust-side work maxima (`stats.txt`; the intercept budget is 128, spechit 8):

| Script | Intercepts | Spechit | ChangeSector calls / clips | Peak actors | Thinkers |
| --- | --- | --- | --- | --- | --- |
| idle | 0 | 0 | 0 / 0 | 92 | 96 |
| walk_to_exit | 18 | 2 | 2 / 5 | 93 | 99 |
| fighting | 24 | 2 | 2 / 2 | 99 | 103 |
| barrels | 25 | 2 | 2 / 2 | 94 | 99 |
| chaos | 15 | 0 | 0 / 0 | 93 | 97 |

Earlier matched runs with the prototype's demos: PISTOL 301, WALK 2,101 and WALKUSE 2,101
tics.

**Synthetic Rust against the goldens** (`tools/reference/check_player_level.py`): all six
scripts and all 10 fixtures pass. SYNDIE: the golden ends `reborn` at DSIM tic 1001 (1,002
snapshots); Rust matches all 1,002 captures and refuses tic 1002 with `PlayerReborn`, which
the checker requires; `level_compare.py` on SYNDIE: 1,002 tics, match, map groups equal. The
Rust test `crates/doom-sim/tests/level_fixtures.rs` decodes the fixtures, checks that the load
capture equals the SYNIDLE load fixture field for field minus `events.*`, and that the
SYNIDLE and SYNDIE replays reproduce the golden hashes (SYNDIE also its refusal of tic 1002).

**Self-tests.** Committed C-side tool: `m3h9c_reference.py --selftest 500` plants, on
SYNIDLE in the strict build, an extra `P_Random` draw before `G_Ticker` 500 and separately
`forwardmove` 25 in command 500 of a copy of the demo, and requires both to be first seen
in the dump of tic 500 (passed: `H prndindex`, and `M #52 x`). Comparator-side, by manual
procedure (each plants a difference on the C side only and reads the comparator's first
divergence): real idle with ticcmd 500 `forwardmove` 0 to 25 gives tic 501, field
`actor.0.x`; real idle with `--selftest-extra-draw 100` gives tic 101, field `rng.gameplay`.
The same two on SYNIDLE give tic 501 `actor.52.x` and tic 101 `rng.gameplay`.

## Rust-only policy tests

Wire 22 adds 8 policy tests, 4 demo tests, the ignored `private_level` test (needs
`DOOM_IWAD_DIR`) and `level_fixtures.rs` (3 tests). Workspace tests went from 1,063 to 1,075
passed after the policy commits (ignored 3 to 4); `cargo test -p doom-sim --locked` passed
1,039 after the fixtures test. Clippy with warnings denied, `cargo fmt` (including the fuzz
manifest) and rustdoc with warnings denied were clean; `cargo +1.85.0 test --workspace
--locked` passed 1,075 at the policy commits.

## Fuzzing

Target `player_level` (`fuzz/fuzz_targets/player_level.rs`, the 49th target) with the seeder
`tools/seed_player_level_fuzz.py` (1,508 seeds, `-max_len=1404`; commands in
[testing](../TESTING.md#m3h9c-whole-engine-runner-and-the-real-e1m1-comparison)). It was
compiled plain and with `--sanitizer address` (cargo-fuzz 0.13.2, nightly). A harness bug was
fixed during development (Damaged events double-counted across the operation boundary). 15 of
the 26 rooms have no monster, so the attacker falls back to the body there. Every
AddressSanitizer run set `ASAN_OPTIONS=detect_leaks=0`: LeakSanitizer was disabled, not
passed.

## Fuzz campaigns

All with `-max_len=1404 -timeout=15 -rss_limit_mb=1024`; no run produced a finding.

| Run | Observed |
| --- | --- |
| Seed replay, AddressSanitizer (`detect_leaks=0`) | 1,510 runs in 373 s, cov 14,321, ft 63,976, peak RSS 765 MB |
| Seed replay, plain | 78 s, cov 7,583, ft 33,204 |
| Coverage-guided campaign, plain, `-seed=801` | 1,201 s, 28,084 executions, 2,136 new units, cov 8,439, ft 42,864, corpus 2,515 |
| Sanitizer campaign, AddressSanitizer (`detect_leaks=0`), `-seed=802` | 1,201 s, 4,636 executions, 519 new units, cov 14,877, ft 71,727, corpus 1,147; two slow units (up to 14 s, timeout 15 s) |

libFuzzer's input length reached 1,081. A seed replay is not a campaign and no bounded
campaign is exhaustive.

## Matched performance

Recorded 2026-11-05 (plan step 9), alone on the idle machine: one timing harness process
and no other build, test, fuzzing or job during a round. The 1-minute load average was 0.94
before, 1.00 to 1.11 at the ten round boundaries and 1.07 after (the harness itself is the one
running process). Host: Intel Core i7-8750H, 6 cores / 12 logical CPUs, 7.7 GiB, Linux
6.18.58.1 WSL2, rustc 1.99.1, the default release profile, no CPU, frequency or core
isolation. These are host observations, not budgets.

**Method** (as the M3h9b record): master `59a397e` (a fresh worktree with its own
`CARGO_TARGET_DIR`) against the branch at `e5eafb5`; 239 matched workloads, the M3h9b 235 plus
the four `player_barrels` workloads (now an older policy); 11 rounds, AB/BA alternating,
5,258 samples; the timed field is each example's own timer. Every non-timing output line of
every sample is identical between the trees (30,250 fields); a naive "only `ns`" filter
would misreport the extra `*_ns` fields of the specials and movers examples.

| Example | n | Timed field | Median delta of medians | Range of the delta | Median delta of minima |
| --- | ---: | --- | ---: | --- | ---: |
| canonical_demon | 1 | `step100_with_query_us` | +9.2% | +9.2..+9.2 | +8.7% |
| context_ownership | 2 | `retirement100_us` | -0.2% | -0.3..-0.1 | +1.0% |
| demon_expanded_map | 9 | `step100_us` | +6.2% | -2.2..+11.7 | +7.3% |
| game_instance | 2 | `step100_us` | +0.2% | -0.1..+0.4 | -0.7% |
| map_extent | 11 | `step100_us` | -1.2% | -4.1..+1.5 | -1.5% |
| map_scale | 9 | `step100_us` | -1.6% | -2.5..+3.0 | -1.5% |
| original_registry | 6 | `step100_us` | +0.5% | -2.2..+1.5 | +0.4% |
| diagnostic_free | 10 | `ns` | +11.0% | -4.3..+13.4 | +11.5% |
| original_player | 4 | `ns` | +0.5% | -0.3..+1.1 | -0.9% |
| player_commands | 9 | `ns` | -2.0% | -5.6..-0.5 | +0.1% |
| player_knockback | 6 | `ns` | -0.5% | -2.3..+3.5 | -0.3% |
| player_demon | 11 | `ns` | -2.5% | -6.1..+3.8 | -2.2% |
| player_demon_melee | 9 | `ns` | -2.9% | -3.3..+2.2 | -0.7% |
| player_pistol | 15 | `ns` | +1.9% | -5.1..+9.4 | +1.6% |
| player_weapon_pack | 25 | `ns` | +0.6% | -2.8..+3.7 | +1.2% |
| player_world_pickups | 17 | `ns` | +1.1% | -1.2..+4.8 | +0.2% |
| player_armor | 19 | `ns` | -1.5% | -5.2..+1.3 | -0.8% |
| player_bonuses | 13 | `ns` | -3.4% | -4.5..+6.0 | -2.1% |
| player_monsters | 7 | `ns` | +0.7% | -1.0..+4.3 | +0.1% |
| player_imps | 7 | `ns` | +0.8% | -2.8..+2.4 | +0.4% |
| player_map_scale | 8 | `ns` | -0.1% | -5.5..+1.0 | -0.3% |
| player_heights | 6 | `ns` | +2.3% | -1.0..+3.2 | +1.8% |
| player_diagonal | 3 | `ns` | +0.5% | 0.0..+2.1 | +0.3% |
| player_real_map | 6 | `ns` | -0.7% | -4.1..-0.2 | -0.1% |
| player_specials | 6 | `ns` | +2.3% | +1.6..+5.5 | +1.8% |
| player_movers | 9 | `ns` | -1.3% | -4.5..+2.4 | -0.8% |
| player_e1m1 | 4 | `ns` | +0.4% | -1.7..+2.8 | -0.5% |
| legacy_pistol | 1 | `ns` | +1.0% | +1.0..+1.0 | +7.2% |
| player_barrels | 4 | `ns` | +1.9% | +0.1..+2.1 | +1.6% |

Over all 239 workloads the median of the median deltas is +0.1% and the median of the minimum
deltas +0.0%.

**Slowdowns over 3%.** 31 workloads slowed by more than 3%; callgrind ran once per tree for
each. 16 are a real instruction-count change: `canonical_demon`, 7 of the 9
`demon_expanded_map` and 8 of the 10 `diagnostic_free` workloads. In them
`Simulation::advance` executes 13.1 to 14.5% more instructions (for example
`diagnostic_free small_control 20`: 109,764,777 to 125,653,971; `canonical_demon`: 109,803,420
to 125,692,614), attributed to `BTreeMap<StateId,_>::insert` and `remove` in
`ActorRegistry::immediate_cycle` (`crates/doom-sim/src/lifecycle.rs`, unchanged on the branch)
no longer being inlined. The other 15 are noise: whole-process Ir within 1% (at most +0.89%,
and `advance` +0.5 to +0.7%) against +3 to +9% in time.

**Cause of the 16** (follow-up experiments, no code committed). The release build uses 16
codegen units. On master the btree code and `immediate_cycle` share one unit (`cgu.04`); on the
branch they are in `cgu.06` and `cgu.09`. Bisection by building the library at each branch
commit points to `2130d9c`, which adds `pub mod demo`. With
`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1` the trees differ by +0.012% Ir (master 121,432,498 and
121,472,323; branch 121,446,511 and 121,486,336 for the two workloads), so master's 109.8M was
itself a favourable split. Three source-level attempts did not restore it stably:
`#[inline]` on the new accessors (no effect); `#[inline]` on `parse_demo` and `write_demo`
(halved the gap to +7.0%); moving the demo items into an existing module (worse). The matched
timings were not re-run after these experiments, which changed no committed code. The
regression is recorded as explained (decision
[G13](../M3H9C_REFERENCE_INVESTIGATION.md#6-new-owner-decisions-g1g13-with-recommendations)).

**Real-E1M1 per-tic costs** (branch only, not matched; the IWAD read in place, five-run
medians). The Rust runner `player_level_run` without captures: idle 1,684 us/tic (3.536 s for
2,100 tics, about 60.8 ms of it fixed) and chaos 1,693 us/tic (8.465 s for 5,000 tics). The
runner hashes a full DSIM38 capture every tic, which dominates; the simulation alone was about
45 us/tic in the Phase 1 probe. With `--captures`: 1,929 and 1,944 us/tic, writing 417 MB and
990 MB. The genuine strict build: `G_Ticker` 3.5 us/tic (idle) and 5.2 us/tic (chaos), the dump
taking 0.513 s and 1.236 s.

**CI budget** (alone, branch tree): `m3h9c_reference.py --check` 19.7 s (build 6.7 s, runs
12.9 s); the six-script Rust loop with `--captures` 17.3 s (prebuilt, writes 1.4 GB);
`check_player_level.py` 37.4 s.

## Independent review

Recorded 2026-11-05 (plan step 11). An Opus reviewer subagent without access to the authors'
reasoning read the full diff against `origin/master` and the linuxdoom-1.10 source, with the
specific question whether any stub or `--wrap` observer writes game state that `P_Ticker` or
`G_Ticker` can observe.

**Verdict: no blocker.** Wires up to 21 are unchanged (every new branch is gated on
`has_level`); each runner adapter matches its source line and computes the 32-bit value; the
`ZeroFill` claim holds; wire 22 is correct against p_mobj.c:642-737 and p_user.c:182-229,
355-359; the encoder reads no Rust output and follows the frozen slot rule.

**Stub and observer proof.** The wrappers `P_AddThinker`, `P_SetThingPosition`, `Z_Free`
(copies from the block, never into it) and `P_InitThinkers` write only side tables;
`Z_Malloc` zero-fills before the caller's first write; the driver sets only vanilla
initialisation values (plus one `P_Random` under `--selftest-extra-draw`); 29 I/O stubs write no
game state; 6 read-only stubs are empty; the 8 writers are `I_Quit` (vanilla's exit),
`I_GetTime` (timing demo only), `HU_Ticker` (`message`, never read by gameplay), `AM_Stop`
(unreachable) and `WI_Start`, `WI_Ticker`, `F_StartFinale`, `D_AdvanceDemo` (abort).

The reviewer ran `check_docs`, the three static `--check` tools, `m3h9c_reference.py --check`
(three variants identical), `m3h9c_snapshot_goldens.py --check`, `cargo test -p doom-sim`
(1,039 passed), the six synthetic Rust runs with `check_player_level` (all pass) and a
planted-difference check (first divergence at tic 501, `actor.52.x`).

| ID | Severity | Finding | Outcome |
| --- | --- | --- | --- |
| R1 | major (docs) | ROADMAP and note status contradicted each other | fixed |
| R2 | minor | stub counts 31/6/6 against the contract's 29/6/8 | fixed |
| R3 | minor | the comparator's `match` did not require the map group | fixed (required; `map_group_sha256` reported) |
| R4 | minor | only the extra-draw self-test was a committed tool | fixed (`m3h9c_reference.py --selftest TIC` plants both perturbations on SYNIDLE; the comparator-side half of the real run stays a manual procedure) |
| R5 | minor | the encoder forced schema zeros instead of reading them | fixed (asserted from the dump) |
| R6 | minor | no C evidence for the dead player's `BT_USE` tic | fixed (driver `end=reborn`, script `synth_die_use`/SYNDIE, golden and fixture) |
| R7 | minor | commit order: the Rust writer before the goldens | disclosed |
| R8 | minor | stale dead-player command-domain text | fixed |
| R9 | note | ARCHITECTURE wording | fixed |
| R10 | note | README "cannot play back demos" | fixed |
| R11 | note | driver comment omitted ZeroFill | fixed |
| R12 | note | fuzz `point_angle` differs from C only at an unreachable 2^31 difference | no change |

## Local gates

Recorded 2026-11-05 (plan step 12) at head `e5eafb5`, from clean (`cargo clean` of the
workspace and `fuzz/target`). The diagnostics-only commit `c97f238` (the runner printing its log
tail, and a failure-only artifact upload) landed 2.5 minutes into the 47-minute run: the Rust
steps ran at `e5eafb5` and the whole-engine step at `c97f238`. Host rustc 1.99.1, cargo
+1.85.0, nightly cargo 1.101.0 (2026-10-27), GCC 15.2.0, Python 3.14.4. **66 of 66 steps
passed in 2,825 s wall:**

- format (workspace and fuzz manifest), Clippy with warnings denied, build;
- `cargo test --workspace --locked`: 1,079 passed, 0 failed, 4 ignored over 87 suites (366 s);
- release `doom-sim` and `doom-types`: 1,046 passed, 0 failed, 2 ignored over 75 suites (143 s);
- `run_reference.py --suite --jobs 2`: 65 historical commands passed (724 s);
- every reference and snapshot-golden `--check` from M3g10 through M3h9c (sanitizer runs with
  `ASAN_OPTIONS=detect_leaks=0`: LeakSanitizer disabled, not passed);
- the M3h9c static checks, the three-variant synthetic run, the DSIM38 projection (70 s), the
  six-script Rust loop with `check_player_level` (55 s);
- every `check_player_*` and `check_original_player` script, the `parse_map` and headless
  examples, the 22 headless equality scripts (54 s; `python` was a symlink to `python3`, as
  this host has no `python`), 6 measurement `--check` and 1 `--smoke`;
- `check_docs` (124 files, 2,114 links, 484 anchors, 20 archives) and rustdoc with warnings
  denied;
- the MSRV job, `cargo +1.85.0 test --workspace --locked`: 1,079 passed, 0 failed, 4 ignored (394 s);
- `check_examples.py`: 17 asserted examples, debug and release;
- fuzz build with `--sanitizer address` (49 targets, 100 s) and the plain fuzz build.

The working tree was unchanged afterwards. The verify-git step fetched `138c66b`.

## CI

Draft pull request #56 (`feat/m3h9c-e1m1-run` to `master`), workflow `Rust`. The workflow
steps are listed in
[testing](../TESTING.md#m3h9c-whole-engine-runner-and-the-real-e1m1-comparison); exact runs
belong to GitHub. Heads observed so far:

| Head | Observed |
| --- | --- |
| `e5eafb5` (code) | Detect changes pass; Minimum Rust pass (4m17s); Stable/windows pass (19m48s); Stable/macos (`macos-26-intel`) pass (30m35s); Stable/ubuntu **failed** (35m12s) at "Genuine whole-engine run of the synthetic E1M1" (address variant, SYNBARRL, exit status -6, log not visible) |
| `c97f238` (diagnostics only) | Minimum Rust pass (7m41s); windows pass (15m3s); macos pass (26m59s); ubuntu **failed** (19m31s) with `Error: Z_ChangeTag: an owner is required for purgable blocks`: the genuine z_zone.c:440 test `(unsigned)block->user < 0x100` truncates an LP64 pointer, under the runner's ASan heap layout (`W_CacheLumpNum` re-caching the synthetic WAD's single patch lump) |
| `7141319` (the fifth LP64 adapter, z_zone.c:440 compares `(uintptr_t)`; the contract and the results JSON's contract hash regenerated; synthetic dumps unchanged) | Detect changes pass (9s); Minimum Rust pass (7m38s); Stable/windows pass (16m51s); Stable/macos pass (25m11s); Stable/ubuntu pass (36m58s): **all five checks green**. The documentation-only commits after it take the fast path; their runs belong to GitHub |

Earlier heads `ab79afb` and `0e565b9` were cancelled by the next push.
