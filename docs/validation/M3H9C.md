# M3h9c evidence record

Observed results of the M3h9c milestone (`PlayerMonstersLevelV1`, wire 22, DSIM38): the real
E1M1 run in step with the genuine linuxdoom-1.10 engine, built whole. Only observations
appear here. The [M3h9c investigation](../M3H9C_REFERENCE_INVESTIGATION.md) owns the source
facts, the decisions (§6) and the plan (§9); [format](../STATE_FORMAT.md) owns DSIM38,
[support](../COMPATIBILITY.md#original-level-runs-the-player-start-in-lump-order-and-the-full-player-think-wire-22)
the admitted behavior and [testing](../TESTING.md#m3h9c-whole-engine-runner-and-the-real-e1m1-comparison)
the commands. Exact review state and CI belong to GitHub.

All timings are rough: the local host was not idle for any of them. Nothing here is a
performance measurement. The real-E1M1 runs are **local only** (commercial IWAD read in
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
6 extracted `r_main.c` bodies, 7 declared LP64 ABI adapters in every build plus 6
sanitizer-only adapters (`p_mobj.c` lines 794 and 671 angle products, `p_map.c:1221`,
`p_enemy.c` lines 816, 839 and 860 int angle spreads; the `p_enemy.c` ones were found by the
UBSan build on the synthetic chaos run), and 43 function plus 17 variable stubs (31 I/O-only
or unreachable, 6 read-only, 6 writers whose writes `P_Ticker` cannot observe; the contract
lists them). `--wrap` observers cover `P_AddThinker`, `Z_Free`, `Z_Malloc` (zero-filling),
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
to 15 s) ran the three variants: all five scripts byte-identical across variants; 98
thinkers, 92 mobjs and `prndindex` 146 after load.

| Script | Tics | End | Events (tic numbers) |
| --- | --- | --- | --- |
| SYNIDLE | 2,100 | marker | player killed at tic 884/885, `P_DeathThink` afterwards |
| SYNWALK | 1,468 | exit at tic 1467 | 6 kills, 2 doors, nukage damage, secret at 1034, lift ridden |
| SYNFIGHT | 273 | marker | 4 kills including one shotgun |
| SYNBARRL | 581 | marker | chain explosion at 464 and 477, a zombieman killed, an imp hurt |
| SYNCHAOS | 5,000 | marker | `nouse`, death at tic 816 |

## Independent projection

`tools/reference/level_snapshot.py` projects the C dumps to DSIM38 exactly from the observer
rows (its `--infer` fallback is never used by the comparator);
`tools/reference/level_compare.py` compares the bytes minus the `map.*` and `events.*`
groups (the map groups are compared separately and are equal; no tolerance).
`tests/reference/m3h9c_synthetic_goldens.json` and nine fixtures `m3h9c_level_*.bin`
(SYNIDLE load, tic 885 death and tic 920 death-think; SYNCHAOS tics 816, 851 and 2500;
SYNFIGHT 273; SYNWALK 1468, the exit tic; SYNBARRL 465, `A_Explode`) were produced from C
dumps only. `m3h9c_snapshot_goldens.py --check` passes (10 files).

| Script | Snapshots | Sequence SHA-256 | Final SHA-256 |
| --- | --- | --- | --- |
| SYNIDLE | 2,101 | `85c8069dd63cb160d4ba038c579710df697add317eb82f1057193638a6b071df` | `c7e0743cddbf6cbf972b9efc73d1515ec928ebd9537f26e9666c5ead18726822` |
| SYNWALK | 1,469 | `17a9680ac99e9c5d15a927d38285a75f5a72717a1220fdec87b4b35ff4aa47b6` | `5e12a6655e13e45ccd627b6eaf5a5bb4fec07733f0d1c4ca3563113a4f4cd67a` |
| SYNFIGHT | 274 | `a9e005f3d5af7cde72284c169d31f5c57cc1f85a13d563d55b5d83a798e05fcf` | `fffa46e0f996884d51ea217a56c573f1753606dab01f00b4929855e84e41fc2d` |
| SYNBARRL | 582 | `b519f64f91e0f4bf3c8620276cd6f321c43607971659007c3103d8d23a9d5fab` | `3dfd5a6f16a83bf134eabc2caceb5e58e30190c13be38e00248939b683e99466` |
| SYNCHAOS | 5,001 | `9d4078400d715297aa993ecb8f0b15a8e530cc395eb4f24f425dc6fd4e13038b` | `0eba38f408a4c56ab2c744f49f14abc443067616ba612ed616c75a6772c15dea` |

Encoder-side corrections found while matching (all in the encoder, none in Rust): a new
door's `topcountdown` and a new plat's `count` read zone garbage in C before first use (the
DSIM35 schema says 0, decision N6), so the encoder projects 0 until the first wait; the link
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

**Synthetic Rust against the goldens** (`tools/reference/check_player_level.py`): all five
scripts and all 9 fixtures pass. The Rust test `crates/doom-sim/tests/level_fixtures.rs`
decodes the fixtures, checks that the load capture equals the SYNIDLE load fixture field for
field minus `events.*`, and that the SYNIDLE replay reproduces the golden hashes.

**Self-tests** (committed tools; each plants a difference and requires the first reported
divergence): real idle with ticcmd 500 `forwardmove` 0 to 25 on the C side only gives tic
501, field `actor.0.x`; real idle with `--selftest-extra-draw 100` gives tic 101, field
`rng.gameplay`. The same two on SYNIDLE give tic 501 `actor.52.x` and tic 101
`rng.gameplay`.

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

To be recorded (step 12).

## Independent review

To be recorded (step 13).

## Local gates

To be recorded (step 14).

## CI

To be recorded (step 15). The workflow steps are listed in
[testing](../TESTING.md#m3h9c-whole-engine-runner-and-the-real-e1m1-comparison); exact runs
belong to GitHub.
