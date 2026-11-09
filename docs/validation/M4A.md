# M4a evidence record

Observed results of the M4a milestone (`PlayerMonstersEpisode1V1`, wire 23, DSIM39): Episode 1,
part 1, proven against the genuine linuxdoom-1.10 engine. Only observations appear here. The
[M4a investigation](../M4A_REFERENCE_INVESTIGATION.md) owns the source facts, the decisions
(§10) and the plan (§12); [format](../STATE_FORMAT.md) owns DSIM39,
[support](../COMPATIBILITY.md#original-episode-1-part-1-every-door-floor-plat-stair-donut-light-and-exit-special-keys-items-powers-chainsaw-rocket-launcher-and-spectre-wire-23)
the admitted behavior and [testing](../TESTING.md#m4a-synthetic-mechanism-wads-and-the-episode-1-comparison)
the commands. Exact review state and CI belong to GitHub.

**Record in progress (Phase 2 steps 1–8 done on 2026-11-05; the fuzz campaigns were re-run on the final crates).** The real-IWAD runs are local
only (`DOOM_IWAD_DIR`, read in place); only counts, tic numbers and digests are recorded and no
map-derived value appears. Address-checked builds ran with `ASAN_OPTIONS=detect_leaks=0`, so
LeakSanitizer was disabled, not passed. Timings quoted before the matched-performance section
were taken on a host that was not idle and are not performance measurements.

## The 32-bit build (G3 closed)

The opt-in `strict32` variant (`m3h9c_build.py --variant strict32`: the strict flags plus `-m32`,
the genuine text with none of the eight LP64 ABI adapter lines and none of the sanitizer
adapters; gcc 15.2.0; `file` reports an i386 32-bit binary; the 34 whole files compared byte for
byte against `linuxdoom-1.10/`) gave dumps byte-identical to the LP64 strict build on all five
real E1M1 scripts (idle 2,100 tics, WALK 766, FIGHT 1,281, BARRELS 882, CHAOS 5,000; `prndindex`
144 at load, 253/127/89/187/227 at the end) and on all six M3h9c synthetic scripts, every result
record equal to the committed `m3h9c_synthetic_results.json`. The only warnings were the two
`-Wformat-overflow` warnings in p_setup.c that every variant has.

Re-run on the final drivers on 2026-11-05 with `tools/local/episode1_reference.py --strict32 --jobs 2`:
all 30 real-script dump sets (the 25 scripts of E1M2, E1M3, E1M4, E1M6 and E1M7 on the M4a driver and
the five E1M1 scripts on the M3h9c driver) are byte-identical between the strict LP64 build and the
`strict32` build (no adapters), and each strict dump digest equals the one from the step 8 runs below.

## Synthetic mechanism WADs (CI evidence)

Nine generated WADs (`tools/reference/m4a_wad.py`, slot E1M1 each, 45–64 KB) and 30 scripts
(15,086 tics) under `tests/reference/m4a_scripts/`; the manifest `tests/reference/m4a_manifest.json`
records each script's end and its observed event tics. Every script gave byte-identical dumps in the
strict, overflow-checked and address-checked builds and in `strict32`; no sanitizer build aborted;
no new sanitizer adapter was needed. The self-test (`--selftest 300` on DOORS1) reports the planted
difference at tic 300. Observed engine facts and the H7 refusal tics (PLATS2 blocked plat at tic 91,
STAIRS2 blocked step at tic 153) are listed in the note's §14.

## Independent projection

`level_snapshot.py --profile episode1` projects the dumps to DSIM39 exactly from the observer rows;
its load-capture field paths equal the Rust writer's 3,841 paths in order (the Rust list was compared
after the fact; the encoder was written from the dump columns and the schema). Goldens:
`tests/reference/m4a_synthetic_goldens.json` (30 scripts, 15,116 snapshots) and 18 fixtures
`m4a_episode1_<SCRIPT>_<tic>.bin`, produced from C dumps only.

Two further synthetic scripts on the ITEMS WAD were added at step 8, each identical in the three C
variants, with goldens added and every tic passing in Rust (the old entries and all 18 fixtures are
byte-identical):

- `TURBO1` (673 snapshots) reproduces the turbo message of the Rust divergence below: the message
  appears on gametics 0, 128, 256 and 640 and not for forward 100 on 64, forward 50 on 384 or forward
  -100 on 512.
- `TARGET1` (481 snapshots, authored closed loop against the strict runner) reproduces the encoder
  deficiency below: a pending imp fireball whose imp moves one place earlier in the thinker list when
  the computer map, listed before the imp, is taken and freed on the next tic.

The encoder deficiency itself was found on the real E1M3 walk. The driver prints `target` and `tracer`
as thinker-list positions of its own dump. A barrel marked for removal whose first damager is listed
after it leaves the list before that damager, so the damager's position drops by one between two
consecutive dumps, and the projection stopped at DSIM tic 5216 ("the RR record differs from the next
RM row", column `target`). `level_snapshot.py` now compares those two columns as the serials they name;
Rust matched all 5,794 snapshots of that script unchanged, so this was an encoder deficiency, not a
Rust divergence. A barrel version of the reproduction could not be authored: every synthetic WAD lists
the player first, and about 40 closed-loop attempts on WEAPONS never got an imp to hit a barrel; the
real E1M3 run is the barrel evidence.

## The driver fork

The first M4a commits changed the merged M3h9c driver (`H.secretexit`, the locked-door message codes
38..40, the `--wrap=P_RemoveThinker` observer and the R-row `fn,sector` columns) and re-hashed two
merged result files. After the owner's review of 2026-11-05 the additions moved into the fork
`tools/reference/m4a_driver.c`, built by `tools/reference/m4a_build.py` (schema `DOOM-M4A-RUNNER-v1`,
contract `tests/reference/m4a_runner_contract.json`, whose hash covers the fork). Observed after the
split:

- `m3h9c_driver.c`, `m3h9c_runner_contract.json` and `m3h9c_synthetic_results.json` equal master's
  bytes (`git diff master` empty for each); `m3h9c_build.py --check` passes against master's contract
  (34 whole files, 8 ABI and 6 sanitizer adapters, 6 extracted bodies, 43 function and 17 variable
  stubs). `m3h9c_build.py` gained only a `Runner` parameter whose defaults are the M3h9c values.
- The M4a contract differs from the re-hashed M3h9c one in the schema name, the runner hash, the order
  of two behaviour entries and one corrected citation (`P_RemoveThinker` is p_tick.c:80-84).
  `m4a_synthetic_results.json` changed only in `runner_contract_sha256`; no dump changed.
- `tools/local/episode1_reference.py` builds E1M2-E1M7 with the fork and E1M1 with the M3h9c driver
  (frozen wire-22 evidence); the `episode1` encoder projects a missing `H.secretexit` as 0, exact for a
  map with no special 51 or 124 line.

## Rust replay against the goldens

`tools/reference/check_player_episode1.py --binary target/release/examples/player_level_run
--example-args "--policy episode1"` on the 30 synthetic scripts, as reported at the end of each step.
Each count is the number of captures that matched the golden digests, out of the number compared. No
divergence from a golden was found on any script that loads, and no divergence was traced to the encoder
or the goldens. Each step also ran the wire-22 checker (`check_player_level.py`, six scripts, ten
fixtures), which passed unchanged. The step reports give the workspace test counts 1,095 (step 3),
1,111 (step 4), 1,121 (step 5) and 1,129 passed (step 6), 0 failed and 4 ignored at each; those were
the step engineers' runs, not repeated for this record.

| Step | Scripts matched (captures matched / compared) | Refusal |
| --- | --- | --- |
| 3 (plumbing, DSIM39) | none (Rust-only: the synthetic E1M1 load equals wire 22 plus the DSIM39 fields; six wire-22 demos run in lockstep under both wires) | |
| 4 (movers) | FLOORS1 933/933, FLOORS2 285/285, FLOORS3 181/181, PLATS1 574/574, STAIRS1 639/639, DONUT1 333/333; PLATS2 92/92; STAIRS2 154/154 | PLATS2 `PlatBlocked` at capture 92 (dump tic 91); STAIRS2 `StairBlocked` at capture 154 (dump tic 153) |
| 5 (keys, items, powers, sector 11) | DOORS1 922/922, DOORS2 559/559, DOORS3 1,336/1,336, EXIT1 154/154, EXIT2 57/57, SECT11 182/182, EXDIE 129/129, ITEMS1 236/236, TIMERS 2,253/2,253, SUIT1 303/303, SUITDIE 183/183, INVIS1 291/291 | EXIT1 `LevelExited` at 154, EXIT2 at 57, SECT11 at 182; EXDIE `PlayerReborn` at 129, SUITDIE at 183 |
| 6 (chainsaw, launcher, missiles, Spectre) | SAW1 239/239, ROCKET1 930/930, ROCKDIE 124/124, SPECT1 609/609, SPECT2 643/643, SPECTD 523/523 | ROCKDIE `PlayerReborn` at 124, SPECTD at 523 |

The step-6 run reported "all checks passed" for the 26 scripts that load, with the 18 fixtures. The
two scale scripts are the exception. SCALE.WAD originally had a tower ceiling of 1,100, above the
+-1,024 sector-height rule that wire 23 keeps, so Rust refused the load (`Geometry(CoordinateOverflow)`).
The committed WAD now has the ceiling at 1,000 and its goldens were regenerated from the genuine runs.
**SCALE1 329/329 and SCALE2 601/601 were observed only in a scratch copy of the old WAD with the one
ceiling lowered to 1,000 and the captured ceiling patched back before hashing; to be confirmed by the
final run on the committed WAD.** In the same step all 930 scale captures, and 12,864 captures of the
whole set, decoded with the DSIM39 reader (the latter in an uncommitted scratch test).

Facts observed on the way, in Rust-only tests rather than against C: pickups are counted down from the
next tic (a pickup on tic N leaves 2,100/4,200 of a power at the end of tic N); overlapping items are
touched newest first; the radiation suit's draws interleave with the pain draws. The shadow aim fuzz
orders its draws as `A_FaceTarget` (first minus second, times 2^21) then `P_SpawnMissile` (times
2^20); INVIS1 matches 291/291 with an invisible player hit by a fireball at tic 277.

At step 8 every manifest entry that said `rust: "tbd"` (24 of them) became `rust: "match"`, so
`check_player_episode1.py --require` is a hard gate for all 30 scripts and 18 fixtures; it passed on the
main tree on 2026-11-05 (30 scripts, 18 fixtures, including TURBO1 673/673 and TARGET1 481/481). On the
same tree `m4a_reference.py --check --keep-dumps --selftest 300` passed (the self-test's extra draw and
ticcmd perturbations were each detected on DOORS1 at tic 300) and `m4a_snapshot_goldens.py --dumps ...
--check` passed (19 files).

## Fuzz target `player_episode1`

The 50th target. The nine committed M4a WADs come in by `include_bytes!`; an input is a four-byte
header (WAD, RNG, turn low byte, check stride) and four-byte version-110 ticcmd records, at most 600
tics (`-max_len=2404`). Properties: no panic; a refused tic leaves the canonical bytes, RNG and tic
unchanged; a command inside the domain the step admits runs or hits a declared witness; the DSIM39
`diagnostic_bytes` round trip at load, at each stride and at the end; the frozen `decode_level`
refuses a DSIM39 capture; a second fresh simulation fed the same bytes agrees tic for tic. The seeder
(`tools/seed_player_episode1_fuzz.py`) wrote **446 seeds** at step 5 (max length 2,404) from the 28 scripts
then committed (six variants each, one more for scripts longer than 600 tics), 14 synthetic patterns on
every WAD with two RNG settings, and header-only and odd-length inputs; it reads no Rust output. With
`TURBO1` and `TARGET1` it writes **459 seeds**.

| Run | Observed |
| --- | --- |
| Target compiled, plain (`--sanitizer none`) and AddressSanitizer (`--sanitizer address`) | yes; cargo-fuzz 0.13.2, nightly cargo 1.101.0 (2026-10-27) |
| Seed replay, plain | 447 runs in 24 s, cov 7,504, ft 27,693, 278 inputs kept, no crash |
| Seed replay, AddressSanitizer (`detect_leaks=0`) | 448 runs in 140 s, cov 14,073, ft 51,955, peak RSS 679 MB, no crash |
| Coverage-guided campaign, plain, `-seed=801`, 300 s | 1,804 runs in total (447 initial included), about 5-6 exec/s after the seed pass (18 exec/s during it); cov 7,504 to 7,601, ft 27,693 to 30,956, corpus 282 to 674 inputs (446 to 839 files), 393 new units; 0 crashes, timeouts or out-of-memory |
| Sanitizer campaign, AddressSanitizer (`detect_leaks=0`), `-seed=802`, 300 s | 791 runs in total (448 initial included), 2-3 exec/s; cov 14,073 to 14,144, ft 51,914 to 54,584, corpus 284 to 401 inputs (446 to 563 files), 117 new units; 0 crashes, timeouts or out-of-memory; one slow unit (933 bytes, 12 s inside the campaign, 5.4 s alone under ASan, 0.55 s plain; the stairs WAD with many preflight-refused commands, each refusal costing three snapshots), not a failure |

LeakSanitizer was disabled (`ASAN_OPTIONS=detect_leaks=0`), not passed. A seed replay is not a
campaign, five-minute campaigns are short and bounded, and the host was not idle (1-minute load average
3 to 4.2), so the execution rates are indicative only.

**The WADs refused at that step.** The campaigns ran against the step-5 crates, where three of the nine
WADs did not load: the weapons WAD (`MapSpawn(UnsupportedType(2005))`), the Spectre WAD
(`UnsupportedType(58)`) and the scale WAD (`Geometry(CoordinateOverflow)`). About a third of the
executions (header byte 5, 7 or 8 modulo 9) therefore only exercised the load refusal. Of the seeds
that loaded, the endings were `PlatBlocked` (7), `StairBlocked` (8), `LevelExited` (11), `PlayerReborn`
(3 on exits, 5 on items) and the 600-tic budget; no seed reached `SecondDoor`, `DonutBusy`,
`MoverFamily`, `Arithmetic` or `StoreError::Capacity`. **The campaigns are to be re-run against the final
crates, after step 7, with all nine WADs admitted**; the numbers above must not be read as coverage of
the chainsaw, launcher, Spectre or scale paths.

**Re-run on the final crates** (2026-11-05, branch at 1f91d0e for the 446 seeds and at the seeder
commit for the 13 new ones; the host was not idle, 1-minute load 1.5 to 2.5, so rates are indicative):

| Run | Observed |
| --- | --- |
| Target compiled, plain and AddressSanitizer | yes, both |
| Seed replay, plain, `-timeout=15` | 447 runs in 92 s, cov 8,492, ft 32,810, 395 inputs kept, no failure |
| Coverage-guided campaign, plain, `-seed=811`, 600 s | 2,926 runs (447 initial included), about 4 exec/s; cov 8,492 to 8,743, ft 32,810 to 37,621, corpus 395 to 1,108 inputs (717 new units); 0 crashes, timeouts or out-of-memory; one slow unit (153 bytes on the scale WAD, 11 s) |
| Seed replay, AddressSanitizer (`detect_leaks=0`), `-timeout=15` as then documented | **did not complete**: libFuzzer's timeout fired on a 2,404-byte scale-WAD seed after 8 min; replayed alone it runs clean, only slowly |
| Sanitizer campaign, AddressSanitizer, `-seed=812`, `-timeout=15`, 600 s | **did not reach the end of the seed pass**: timeout on an 804-byte scale-WAD seed after 6 min; clean when replayed with a longer timeout |
| Seed replay, AddressSanitizer (`detect_leaks=0`), `-timeout=120` | 448 runs in 517 s, cov 15,815, ft 61,290, 394 inputs kept, no failure |
| Sanitizer campaign, AddressSanitizer (`detect_leaks=0`), `-seed=812`, `-timeout=120`, 600 s | 551 runs (448 initial included, so only 103 mutated inputs), cov 15,815 to 15,830, ft 61,290 to 62,249, 438 inputs (39 new units); 0 crashes or out-of-memory. Mostly the seed pass: thin as a campaign |
| The 13 new `TURBO1`/`TARGET1` seeds, plain replay `-timeout=15` and AddressSanitizer replay `-timeout=120` | 14 and 15 runs, no failure (2 s plain, 12 s ASan) |

No panic, assertion failure, sanitizer report or out-of-memory in any run. **Finding:** every input on the
scale WAD (160 actors, the largest map) costs 2.2 to 3.8 s plain and 26 to 33 s under AddressSanitizer
even for a 52-byte input, because a record's repeat count reaches 600 tics and the harness runs two
simulations and clones the state before every tic for the refusal-preservation check; the map load
itself is 10 to 30 ms (release example). 42 of the 446 seeds use that WAD. The documented ASan replay
therefore uses `-timeout=120` ([testing](../TESTING.md)); the plain `-timeout=15` was never hit. No code
changed for this. The campaign numbers are what a 10-minute run on a loaded host produced and are not a
coverage claim for the Spectre, launcher or scale paths beyond the seeds that reach them.

## Real Episode 1 maps: genuine-engine runs (local only)

Scripts for E1M2, E1M3, E1M4, E1M6 and E1M7 (idle 2,100 tics; chaos 5,000 commands from seed
20261105 + map number; fighting; death ending `end=reborn`; walk_to_exit ending on the exit tic),
all byte-identical across the four genuine builds, no sanitizer abort:

| Map | Thinkers / mobjs at load | `prndindex` at load | walk_to_exit tics (exit) | fighting tics | chaos death tic | death script `end=reborn` |
| --- | --- | --- | --- | --- | --- | --- |
| E1M2 | 226 / 200 | 64 | 2,863 | 1,581 | 1,549 | yes |
| E1M3 | 322 / 310 | 9 | 5,792 (normal exit) | 1,495 | 3,361 | yes |
| E1M4 | 213 / 201 | 85 | 3,443 | 1,771 | 1,668 | yes |
| E1M6 | 382 / 370 | 111 | 7,839 | 1,435 | 1,017 | yes |
| E1M7 | 283 / 267 | 207 | 7,736 | 1,221 | 1,514 | yes |

No blocked stair step or blocked raise-and-change plat occurs in any of these runs. Rust matches the
genuine engine tic for tic on all 25 of these scripts, and on the five real E1M1 scripts re-run under
wire 23 (30 runs). The comparison is DSIM39 without the `map.*` and `events.*` groups; the ten map
lump groups are equal; the per-tic sequence SHA-256 and the final stripped SHA-256 are equal on both
sides. No refuse-the-tic witness fired on any run. The only Rust refusals are the expected
`PlayerReborn` on the tic after a reborn request (the chaos scripts), where the demo continues past it.
The Rust runs are `player_level_run --policy episode1 --wad DOOM.WAD --map E1Mn --demo <lmp> --captures`,
compared with `level_compare.py --profile episode1`. Snapshots are the tics compared (the load plus
every tic); the comparator's DSIM tic numbers are one higher than the C end tics shown.

| Map | Script | Snapshots | Result | C end | Rust end | Sequence SHA-256 (both sides) | Final stripped SHA-256 (both sides) | Map group | Witness |
| --- | --- | ---: | --- | --- | --- | --- | --- | --- | --- |
| E1M2 | M2IDLE | 2101 | match | marker at 2100 | demo end (last capture 2100) | `718c9386457ee56692be644450616fb5709dd3bfce6a6dfe81c83ad07eb8220c` | `2a5c9e6eb31416c88b4a2d88071ac5753b73988d61f2c9c5c5d3ac31b15d09bf` | equal | none |
| E1M2 | M2WALK | 2865 | match | exit at 2863 | demo end (last capture 2864) | `9c35ef89fca258cdf98b594150354345d963c514adc9e6dd1a187cbab5819b3d` | `4a5fa9307ddc67b182a57b4f9b042a32b0313a52f8268fa84c98b84b918ed2c6` | equal | none |
| E1M2 | M2FIGHT | 1582 | match | marker at 1581 | demo end (last capture 1581) | `f6a02e6f1196c550ec4fa096a1643c0536507cfac79929226a3cbb04c1338a53` | `1ae10253fa760fdbe102b75bba95a399fc74a5b8cef6e1541806825c2c5b9f57` | equal | none |
| E1M2 | M2CHAOS | 1551 | match | reborn at 1549 | refused at 1551 (PlayerReborn) | `b9eaa9ff2c10e27c832dba99ca7aa179403a8965d9d302b8eafe2e89d078f787` | `1644054a445116a854aee38ac5cb046c618e275954657e1dd739db4a9daa4bb0` | equal | none |
| E1M2 | M2DIE | 1000 | match | reborn at 998 | demo end (last capture 999) | `f04facc4935d75d8cd50d14154b4bdf6e4d743d16d25854371538050cc59c8fa` | `bcec1c147c7f3429897d8a8351ccf3347725b8b312ad5ceedf67c0a4ac0ded11` | equal | none |
| E1M3 | M3IDLE | 2101 | match | marker at 2100 | demo end (last capture 2100) | `8f1e2e1769d1d1b03e4397d437b1b127e863cd30136d4e3c91472d33cc5fda0e` | `b1b406788d6cf9d1ec7f34600f45ee27a6f7c8cbb17f15e4ae890fa454b5584d` | equal | none |
| E1M3 | M3WALK | 5794 | match | exit at 5792 | demo end (last capture 5793) | `fe5949ed83af5f5d8663ab6f02bbc2ec049a97f4bf19f06e07f215d49e74821b` | `0897116828278db783d8dc2f5815c230460a2bbcd26820e868391873f7b11649` | equal | none |
| E1M3 | M3FIGHT | 1496 | match | marker at 1495 | demo end (last capture 1495) | `657f2805f3be0d6549f36300c0c3f39bffac0e20ddf948867dd5347f5f9a6914` | `f5f0f3d1b64c83391219c3ebc16868d3ddba24f6aecb0c8fff3d7d93bb7eafc1` | equal | none |
| E1M3 | M3CHAOS | 3363 | match | reborn at 3361 | refused at 3363 (PlayerReborn) | `b29043d84117ff3eed299d35d9e672fb81a5cb5aeb2d065686ab704ece23fa1a` | `4f9f51220dcbae75af51adc975ea68b001ac37dd472d6cfb3a64d7b20ba6620d` | equal | none |
| E1M3 | M3DIE | 1001 | match | reborn at 999 | demo end (last capture 1000) | `e972e23666c42eecdf7b8fb69d32dd59fc032f0c63e360c4409702b5427e0958` | `e12b02042a0dd803615b0a3a06b956396196eaa3e32e91f19ec6229d88859802` | equal | none |
| E1M4 | M4IDLE | 2101 | match | marker at 2100 | demo end (last capture 2100) | `2419afabbb3aca7bbbe212e90979f97cc88a7cd7532219676b0d7430e82b7821` | `ca1580c55ef61d8b9372c4f39467e3a55652b6dd7f81929e613236308bf90665` | equal | none |
| E1M4 | M4WALK | 3445 | match | exit at 3443 | demo end (last capture 3444) | `9744aa6f8bc90d4d79db1fe2c5e45169a9de8846a1757d69200f26b9035b9942` | `f6593f28bb93733395898b0eda288a97080208ab51fa694e6fb006f5cfbffa4b` | equal | none |
| E1M4 | M4FIGHT | 1772 | match | marker at 1771 | demo end (last capture 1771) | `b2d2f3c128ee53eb090d223c99d197d2aa16ddd60e290097f70eb12ba43fc188` | `b30e8119d430242cec144a3750663ad3fa4f212e1d56e64ffa3ab7b04fbeb6bd` | equal | none |
| E1M4 | M4CHAOS | 1670 | match | reborn at 1668 | refused at 1670 (PlayerReborn) | `9f9e6cf46f1ad61f872758bed372d2a4f17b8354a4a9804022854db44b56a933` | `1c061248ce0244c3591be404111a2c8988aeeb44def4627382926dac04baa86d` | equal | none |
| E1M4 | M4DIE | 403 | match | reborn at 401 | demo end (last capture 402) | `5cd59307a01c44440d2fd5210cd523c7a079a39c724695ed2f333af03a02cf60` | `6efd176effd1acfb57c928575fa22a69133fc419bc5107cc5ff374def79bcbd3` | equal | none |
| E1M6 | M6IDLE | 2101 | match | marker at 2100 | demo end (last capture 2100) | `0f0a310d44c6e337450495a776e435c032e9bfe251e20dc1b6e545b61313cb2f` | `9bfd4a13a5b4f2c204a5332ec56fe7b2362d4af8ae50ccc2a4502a9fdfcc0146` | equal | none |
| E1M6 | M6WALK | 7840 | match | exit at 7838 | demo end (last capture 7839) | `47999074112eddb0433cccb9d89eef3ebf92e15f9942932f1796c878efe5f49f` | `0b63116c3491d8488b6ac996d32bc34cab3fa21f4634ad113ac726ce3fd6d3ee` | equal | none |
| E1M6 | M6FIGHT | 1436 | match | marker at 1435 | demo end (last capture 1435) | `17e2e0cd9e72f8652deeb239d103b85eb23b6f03f747c5790957b2087a39edc9` | `8f0cd2271bc6c21278589f1df531a89cc4cd86c54efd174a4e6962129f3cc98c` | equal | none |
| E1M6 | M6CHAOS | 1019 | match | reborn at 1017 | refused at 1019 (PlayerReborn) | `d10fbf422ca889a0657555289a163dc94836ce6a5a92183b9ccb3a3bfcb34d92` | `53b5a6802234ef19175e9f519b11590dbb536311ea35254ef2896c7a98f0698d` | equal | none |
| E1M6 | M6DIE | 351 | match | reborn at 349 | demo end (last capture 350) | `b99f15f54f3360949ac93a7aecdbaa60842c99353bea549522489eaafdd3c721` | `d93a6cc7a1b80163a0e62728192ce7a07bc7748422d3b64cca826fca91044d89` | equal | none |
| E1M7 | M7IDLE | 2101 | match | marker at 2100 | demo end (last capture 2100) | `e0387e7e8aaa964e5ca760a7984bb5f5ee7f735fe6f7915e1a3c944d0bb7b4cb` | `59efb10ef647c40b22c3cc68a50bdab1cd3ad1bed82427f3e2626bd87db9e2c6` | equal | none |
| E1M7 | M7WALK | 7737 | match | exit at 7735 | demo end (last capture 7736) | `e822e0eeedae2561a20f551f431a23a55913ea7ac8453326b6df8031740b6ecd` | `bbe5eb8378308b5fc3259e3b311ff5a0fed5ccf342f675d86ef36627ec477a57` | equal | none |
| E1M7 | M7FIGHT | 1222 | match | marker at 1221 | demo end (last capture 1221) | `f819d63ff38efed179784f833e70f94147d84eb229b875256052901099ed49af` | `b19c864becde9e86b600a49b623ebea1389ff7f8dbdf5bb84b8fb4526f811d05` | equal | none |
| E1M7 | M7CHAOS | 1516 | match | reborn at 1514 | refused at 1516 (PlayerReborn) | `796951af82867250980e4cf2976e38da50ff91bbe47f9e8d076f37231d2ebde2` | `2669931402d97cb1707242f01ce29bf2a62a4a744f2cd4ba2e2748e6ea60b04d` | equal | none |
| E1M7 | M7DIE | 368 | match | reborn at 366 | demo end (last capture 367) | `f62ec6f75beedf51cf946a08e34ad2bc90ae96597510e651c46de5ab90adffb0` | `b8d7c069574f5beae57212fe8a1a40440819f1c651672cb9542f428786cd3195` | equal | none |
| E1M1 | idle | 2101 | match | marker at 2100 | demo end (last capture 2100) | `12ae5824a70518806d5b34a2060e284f1128475472097021e7c310331d4c48fa` | `6005a700e49a8309d9ab073f5aec14183ff151909c275d8625792d614b1f2885` | equal | none |
| E1M1 | WALK | 767 | match | exit at 765 | demo end (last capture 766) | `53f76f42efee684625c1dbf27411d1998eaf6186142bac7b6a15d59a42771cd0` | `4d18ef238bf4b18e9101bcc04df3c4b0078316ffd29320f618acab6eeb12bef3` | equal | none |
| E1M1 | FIGHT | 1282 | match | marker at 1281 | demo end (last capture 1281) | `7245812641b846c35c1a4e0b7cb474166b3088afe449b92d201f02eb854a82bf` | `e340f2cff6111e451e77a6fcd0b5e09aee530759f999499746c9292514b98b43` | equal | none |
| E1M1 | CHAOS | 5001 | match | marker at 5000 | demo end (last capture 5000) | `79fa736865bbc34fdb55f696d31ecad28814ecbd7e4655abb5dec0bcc36de281` | `ec422dfaf0005975db203d559bd752c6b555e40b873ae1cc7f5cee2792b15aaf` | equal | none |
| E1M1 | BARRELS | 883 | match | marker at 882 | demo end (last capture 882) | `34a5e76effab3733d885c1c85bbdad5e0f14f8cd2a11e444dced7b43e6828d58` | `ee59beaba4f128830bc1dbba284121d7e4d92a944cf3c55127234507ea621765` | equal | none |

Rust work maxima for the same runs (final binary, from `stats.txt`; counts only):

| Map | Script | Max intercepts | Max spechit | Max ChangeSector calls/tic | Max ChangeSector thing clips/tic | Peak actors | Peak thinkers | Store high-water | exit_tic |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| E1M2 | M2IDLE | 0 | 0 | 0 | 0 | 200 | 226 | 200 | none |
| E1M2 | M2WALK | 27 | 2 | 2 | 7 | 205 | 232 | 205 | 2864 |
| E1M2 | M2FIGHT | 27 | 2 | 1 | 3 | 209 | 236 | 209 | none |
| E1M2 | M2CHAOS | 26 | 0 | 0 | 0 | 201 | 227 | 201 | none |
| E1M2 | M2DIE | 22 | 0 | 0 | 0 | 203 | 229 | 203 | none |
| E1M3 | M3IDLE | 0 | 0 | 0 | 0 | 310 | 322 | 310 | none |
| E1M3 | M3WALK | 29 | 4 | 10 | 34 | 321 | 334 | 322 | 5793 |
| E1M3 | M3FIGHT | 32 | 2 | 1 | 6 | 316 | 328 | 317 | none |
| E1M3 | M3CHAOS | 26 | 1 | 1 | 3 | 313 | 325 | 313 | none |
| E1M3 | M3DIE | 27 | 1 | 1 | 3 | 313 | 325 | 313 | none |
| E1M4 | M4IDLE | 0 | 0 | 0 | 0 | 201 | 213 | 201 | none |
| E1M4 | M4WALK | 26 | 2 | 6 | 12 | 212 | 229 | 212 | 3444 |
| E1M4 | M4FIGHT | 26 | 2 | 2 | 3 | 208 | 220 | 208 | none |
| E1M4 | M4CHAOS | 21 | 1 | 2 | 8 | 210 | 222 | 210 | none |
| E1M4 | M4DIE | 20 | 1 | 1 | 3 | 209 | 221 | 209 | none |
| E1M6 | M6IDLE | 0 | 0 | 0 | 0 | 370 | 382 | 370 | none |
| E1M6 | M6WALK | 37 | 2 | 5 | 37 | 382 | 395 | 382 | 7839 |
| E1M6 | M6FIGHT | 36 | 2 | 1 | 8 | 381 | 393 | 381 | none |
| E1M6 | M6CHAOS | 21 | 1 | 0 | 0 | 374 | 386 | 374 | none |
| E1M6 | M6DIE | 24 | 0 | 0 | 0 | 376 | 388 | 376 | none |
| E1M7 | M7IDLE | 0 | 0 | 0 | 0 | 267 | 283 | 267 | none |
| E1M7 | M7WALK | 36 | 3 | 3 | 11 | 272 | 288 | 272 | 7736 |
| E1M7 | M7FIGHT | 32 | 2 | 1 | 8 | 275 | 291 | 275 | none |
| E1M7 | M7CHAOS | 15 | 2 | 1 | 8 | 267 | 283 | 267 | none |
| E1M7 | M7DIE | 14 | 1 | 1 | 8 | 268 | 284 | 268 | none |
| E1M1 | idle | 0 | 0 | 0 | 0 | 92 | 96 | 92 | none |
| E1M1 | WALK | 18 | 2 | 2 | 5 | 93 | 99 | 93 | 766 |
| E1M1 | FIGHT | 24 | 2 | 2 | 2 | 99 | 103 | 99 | none |
| E1M1 | CHAOS | 15 | 0 | 0 | 0 | 93 | 97 | 93 | none |
| E1M1 | BARRELS | 25 | 2 | 2 | 2 | 94 | 99 | 94 | none |

All 30 C runs gave identical dumps in the strict, overflow-checked and address-checked builds and report
`unknown_messages 0`; the address builds ran with `ASAN_OPTIONS=detect_leaks=0`, so leak detection was
disabled and this is not LeakSanitizer coverage. The 32-bit result is under
[the 32-bit build](#the-32-bit-build-g3-closed).

One Rust divergence was found and fixed, the turbo message. `G_Ticker` (g_game.c:671-679) sets
`players[i].message = "<name> is turbo!"` when `forwardmove` exceeds `TURBOTHRESHOLD` (50), `!(gametic&31)`
and `((gametic>>5)&3) == i`, that is on every 128th gametic for player 0, and it also runs during demo
playback. It first showed in DSIM tic 1 of M4FIGHT and M4WALK (also M6FIGHT, M6WALK and M7WALK) as a
`pickups.message` difference. The Rust model now assigns the new message code 30 (wire 23 only), and the
driver reports it as message index 41, so the earlier counts of 20 unknown messages for M6WALK and 23
for M7WALK are gone. The synthetic reproduction is `TURBO1`, described under
[Independent projection](#independent-projection). The corrections to the investigation note are in its
[section 14](../M4A_REFERENCE_INVESTIGATION.md#14-corrections-and-findings-from-phase-2).

Gates observed by the step 8 engineer on the worktree: 1,138 tests passed, 0 failed, 4 ignored (debug);
clippy with warnings denied, `cargo fmt --check` (including the fuzz workspace) and rustdoc with
warnings denied were clean. Not run at step 8, and so not claimed here: release or MSRV test runs, fuzz
or sanitizer campaigns, timings and exact-head CI.

None of DOOM.WAD's original demos can run under wire 23: all are skill 3 (M4b), and DEMO1 is E1M5 with
teleporters, also deferred to M4b. None was attempted.

## Matched performance

Recorded **2026-11-09 PST** (America/Los_Angeles), on the idle local host, one workload at a time,
uncapped. The earlier run was discarded because of an orphan test process; [its observations are
archived](archive/M4A_PERFORMANCE_2026-11-06.md).

**Host and preparation.** Intel Core i7-8750H @ 2.20 GHz, 6 cores / 12 logical CPUs, 7.7 GiB, Linux
6.18.58.1 WSL2, rustc 1.99.1, default release profile; no CPU, frequency or core isolation. After WSL
startup settled, `uptime` was 0.58 / 0.88 / 0.41 and no cargo, rustc, nextest, cargo-mutants, test or
timing process was running. Both trees were built release from fresh scratch targets before any
measurements, with no build-job cap. After builds and the caller check finished, the harness waited for
1-minute load below 1.0. No concurrent build, test or fuzz job was observed in the per-round process
records; the owner had closed other apps. `/mnt/c` had 81 GB free before builds, 80 GB during
measurements and 80 GB after cleanup (at least 20 GB throughout the observed checks).

**Method.** Fresh detached master worktree at `0ecfe4b58da2bab67f56eb62ac442bb35d4060c7`, confirmed
against origin/master, versus branch `6d10b5b5838eaafca178f2ce179b0758a030115d`. The timing caller is
the one committed at `9aa7d6c2a81382250abd56e5edcc7e256cc0c416`; `git diff --exit-code 9aa7d6c -- crates
tests/support Cargo.toml Cargo.lock` was clean. Both trees built with `cargo build -p doom-sim
--examples --release --locked`, separate `CARGO_TARGET_DIR`s, and one legacy caller compiled per tree
with the measure script’s rustc command. All 36 independent C-derived caller checks passed in release
before timing. Scratch root: `~/scratch/m4a/perf-idle-nov09`, scripts `prepare_builds.py`, `harness.py`,
`cg.py`, `pertic.py`; build provenance in `builds.json`.

All **246 workloads** are retained: 241 from the existing measure scripts/examples and five default
wire-22 real-E1M1 level runs. Eleven paired rounds, alternating master/branch order by workload and
round parity, yield **5,412 samples**. Every non-timing stdout line is compared after removing `ns`,
`_ns` and `_us` fields; the level runs additionally compare SHA-256 of `digests.txt` and `stats.txt`.
All signatures were stable and identical across all 22 samples per workload. Example timers are their
existing fields: `step100_with_query_us` for canonical_demon, `retirement100_us` for context_ownership,
`step100_us` for demon_expanded_map/game_instance/map_extent/map_scale/original_registry, and `ns` for
the other examples; the level runner uses monotonic whole-process wall time, including DSIM and hashing.
No warmup or sample was discarded.

Pre-run and per-round load averages (1 / 5 / 15 minutes):

| Observation | Load averages |
| --- | --- |
| before | 0.21 / 1.35 / 1.04 |
| round0 start | 0.21 / 1.35 / 1.04 |
| round1 start | 0.85 / 1.28 / 1.04 |
| round2 start | 0.94 / 1.19 / 1.03 |
| round3 start | 1.01 / 1.16 / 1.02 |
| round4 start | 1.12 / 1.16 / 1.03 |
| round5 start | 1.19 / 1.17 / 1.04 |
| round6 start | 1.13 / 1.15 / 1.05 |
| round7 start | 1.03 / 1.11 / 1.04 |
| round8 start | 1.04 / 1.10 / 1.04 |
| round9 start | 1.13 / 1.10 / 1.04 |
| round10 start | 1.03 / 1.07 / 1.03 |
| after | 1.06 / 1.08 / 1.04 |

| Group | n | Median delta of medians | Range | Median delta of minima |
| --- | ---: | ---: | ---: | ---: |
| canonical_demon | 1 | -1.59% | -1.59% to -1.59% | -2.68% |
| context_ownership | 2 | -0.47% | -0.64% to -0.31% | -0.31% |
| demon_expanded_map | 9 | +1.09% | -0.52% to +1.59% | +1.14% |
| game_instance | 2 | -2.69% | -3.28% to -2.11% | -3.99% |
| map_extent | 11 | -0.25% | -2.78% to +6.55% | -0.51% |
| map_scale | 9 | +1.87% | -1.32% to +4.83% | +2.25% |
| original_registry | 6 | +0.71% | -2.90% to +2.17% | +0.94% |
| diagnostic_free | 10 | +2.55% | +1.25% to +8.76% | +2.34% |
| original_player | 4 | +0.19% | -1.43% to +1.87% | +2.44% |
| player_commands | 9 | +2.93% | -0.13% to +4.48% | +2.27% |
| player_knockback | 6 | +1.50% | -2.56% to +3.06% | +1.56% |
| player_demon | 11 | +0.33% | -2.08% to +4.99% | +2.14% |
| player_demon_melee | 9 | +2.92% | +1.84% to +12.54% | +4.16% |
| player_pistol | 15 | +2.02% | +0.04% to +4.39% | +2.36% |
| player_weapon_pack | 25 | -0.22% | -6.55% to +5.90% | +0.05% |
| player_world_pickups | 17 | +3.03% | -2.20% to +5.96% | +2.79% |
| player_armor | 19 | +3.12% | -0.05% to +4.40% | +2.73% |
| player_bonuses | 13 | +1.74% | -3.45% to +4.27% | +1.38% |
| player_monsters | 7 | +2.25% | +0.06% to +5.80% | +1.86% |
| player_imps | 7 | +3.65% | +1.93% to +5.98% | +3.72% |
| player_map_scale | 8 | +4.05% | +1.41% to +5.42% | +3.47% |
| player_heights | 6 | +2.12% | -1.06% to +3.97% | +1.97% |
| player_diagonal | 3 | +2.13% | -1.57% to +5.04% | +0.80% |
| player_real_map | 7 | +1.68% | +0.58% to +5.92% | +1.21% |
| player_specials | 6 | +3.37% | +0.10% to +5.74% | +3.04% |
| player_movers | 9 | +5.04% | +2.11% to +8.29% | +3.61% |
| player_e1m1 | 4 | +5.30% | +3.32% to +11.83% | +5.99% |
| player_barrels | 5 | +5.21% | +1.47% to +8.57% | +4.85% |
| legacy_pistol | 1 | -2.24% | -2.24% to -2.24% | -1.82% |
| level_run_e1m1 | 5 | -0.06% | -4.10% to +0.69% | -0.09% |

Over all 246 workloads, the median of median deltas is **+2.02%**, and the median of minimum deltas
**+2.03%**. The five E1M1 whole-run workloads have a median delta of **−0.06%**. Short timers remain
sensitive to scheduling and clock resolution even on an idle host; instruction counts below qualify the
elapsed-time changes.

**Callgrind, only for new >3% median slowdowns.** All 78 selected workloads ran once per tree under
`valgrind --tool=callgrind`, sequentially after the matched rounds. In 74, whole-process Ir differs by
at most 1%; this does not establish an instruction-count cause for their >3% elapsed slowdown. Small
real simulation costs can still be masked by whole-process setup. Four exceed 1% total Ir:

| Workload | Time delta | Ir master | Ir branch | Ir delta | Advance Ir master → branch | Advance delta |
| --- | ---: | ---: | ---: | ---: | --- | ---: |
| player_e1m1 gallery_patrol | +11.83% | 452,507,984 | 477,401,020 | +5.50% | 222,745,889 → 248,021,949 | +11.35% |
| player_barrels dense64_fight | +5.94% | 2,130,983,008 | 2,239,098,972 | +5.07% | 1,704,811,498 → 1,812,887,124 | +6.34% |
| player_barrels dense64_lift | +5.21% | 2,622,550,340 | 2,700,324,178 | +2.97% | 2,365,775,605 → 2,443,649,619 | +3.29% |
| player_demon_melee corpse_continuation | +12.54% | 8,694,788 | 8,812,899 | +1.36% | 6,462,515 → 6,584,145 | +1.88% |

**Cause (Callgrind self costs plus source reading, no code experiment).** Wire 23 extends the validation
shared with older policies. `require_role_for` adds role/type checks (+24.27M self Ir in dense64_fight,
+18.85M in dense64_lift, +83,450 in corpse_continuation); `OriginalActorType::descriptor` adds +18.08M,
+14.29M and +6.70M in dense64_fight, dense64_lift and gallery_patrol respectively. `decoration_states`
now builds a `Vec` of valid states during decoration validation: +11.38M, +8.99M and +4.21M self Ir in
those three workloads, with additional malloc/free costs. `PlayerWeaponState::validate_for` adds +3.21M
in gallery_patrol (partly offset by the older validate symbol disappearing). The new guarded cases and
animated-decoration state membership run for older policies too, preserving their answers at a per-tic
cost. Dense64_fight also has +13.71M self Ir in memcpy; this contributes to its measured total, without
an experiment isolating its allocation/layout cause. The compiler shifts exactly 49,058,396 Ir between
two iterator symbols in dense64_fight, 35,736,484 in dense64_lift and 7,873,514 in gallery_patrol; these
offset and are not extra work. No simulation code was changed. Reordering/caching these validations
remains an owner decision.

### Simulation-only real-map timings

Five repetitions of each of the **30** real-map cases, plus five wire-22 E1M1 control cases, were run
sequentially with rotating full/advance/C order. The timing caller sums only successful
`Simulation::advance` intervals: parsing, setup, spawning, final capture/hash and I/O are outside the
sum; refused-call time is separate. There is no per-tic DSIM capture or hashing. No clock overhead is
subtracted; all reported clock-floor/median probes were 0 ns at this host’s observed resolution, which
does not mean the clock is free. Each observation is a run’s summed successful-call time divided by its
successful tic count. “Median” is the median of the five script medians (each script has five
repetitions); “worst” is the largest run-average across all 25 observations for that map, not the
slowest individual tic.

All 150 advance-only final stripped SHA-256 values match the 30 existing independent genuine-C final
projections in `~/scratch/m4a/real/keep/cmp_*.json`; full/advance counts, refusal boundaries and final
FNV match in every repetition. Fresh C counts, RNG and end conditions match the historical C counts.
This is final-state validation, not a new tic-for-tic comparison. The first per-tic attempt stopped
after six cases because the scratch driver incorrectly required a refused call for a reborn-ending demo
that exhausts its commands on the request tic (E1M2 death). Counts and RNG already matched. The
scratch-only check was corrected to accept that exhausted-command boundary with `end_tic + 1 == tics`;
the incomplete attempt is excluded, preserved separately in scratch, and all five rounds were restarted.
No test, reference file or simulation code changed.

| Map | Scripts | Successful tics | Advance median µs/tic | Advance worst µs/tic | Historical C strict32 range µs/tic | Fresh C median / worst µs/tic |
| --- | ---: | ---: | ---: | ---: | --- | ---: |
| E1M1 | 5 | 10,029 | 55.494 | 59.970 | 10.0–15.5 | 3.700 / 3.900 |
| E1M2 | 5 | 9,094 | 134.216 | 142.774 | 14.8–34.9 | 5.800 / 6.900 |
| E1M3 | 5 | 13,750 | 296.371 | 304.578 | 41.9–48.6 | 12.100 / 13.300 |
| E1M4 | 5 | 9,386 | 214.708 | 226.845 | 22.4–32.8 | 6.800 / 7.800 |
| E1M6 | 5 | 12,742 | 659.597 | 745.566 | 31.6–54.2 | 13.400 / 17.100 |
| E1M7 | 5 | 12,939 | 305.888 | 352.794 | 40.3–68.2 | 11.500 / 17.300 |

Total: **67,940 successful tics** per repetition; the older 67,970 count included 30 initial snapshots.
Historical C numbers above are retained from `real/keep/strict32/*.json`, not reinterpreted as current
timings. Fresh C runs use genuine `G_Ticker`, strict32, diagnostic `--no-dump --no-checkheap`:
dumping/heap walking and the marker-termination call are excluded, but the driver’s
setjmp/arming/gametic increment remains inside its timer. C and Rust boundaries differ; these are
descriptive costs, not an equivalent-work speed ratio. The historical AddressSanitizer range (28–153
µs/tic) is not a fresh sanitizer measurement. IWADs were read in place at `/mnt/d/DoomIWADs`,
local-only. No IWAD contents or geometry are committed.

Full-run wall time, retained to show the DSIM/hash cost separately (median of script medians; µs per
successful tic):

| Map / policy | Full wall | Advance only |
| --- | ---: | ---: |
| E1M1 / episode1 | 1945.227 | 55.494 |
| E1M2 / episode1 | 4369.735 | 134.216 |
| E1M3 / episode1 | 5950.207 | 296.371 |
| E1M4 / episode1 | 4078.606 | 214.708 |
| E1M6 / episode1 | 7564.425 | 659.597 |
| E1M7 / episode1 | 5356.233 | 305.888 |
| E1M1 / level | 1799.719 | 54.301 |

Per-tic campaign load averages (1 / 5 / 15 minutes):

| Observation | Load averages |
| --- | --- |
| before | 0.27 / 0.11 / 0.25 |
| round-0-start | 0.27 / 0.11 / 0.25 |
| round-1-start | 1.06 / 0.78 / 0.52 |
| round-2-start | 1.27 / 1.07 / 0.75 |
| round-3-start | 1.02 / 1.07 / 0.88 |
| round-4-start | 1.01 / 1.06 / 0.97 |
| after | 1.26 / 1.11 / 1.02 |

**Cleanup and provenance.** The fresh `wt-master-idle-nov09` worktree, its `target-master` and legacy
caller were removed after measurements. Historical scratch/evidence and the new raw counts, state
digests, observations and Callgrind reports remain local for review. No CI-budget or full validation
gate was rerun here; those are Claude’s next step. Original archive SHA-256 is recorded in the archive
above. Local result digests:

| Scratch result | SHA-256 |
| --- | --- |
| `raw.json` | `0d275d0efe13f9ea460503ea5a41ea01074086a0dbb567c811f30d158511a019` |
| `analysis.json` | `9c4363875c626c1d000662f858532ef8e1b34fc2c73ee7b6a9dedcd0e2fc11d2` |
| `pertic.py` | `2b6098995f7c36b1c4a08d19696ea2cbc02073651c26b18bbc3716af36d8cf75` |
| `pertic.json` | `1fe6a996ba9ed803abe9ae278174d487fb76b8f797606b9955b533ec1d8a5423` |
| `pertic-analysis.json` | `9f8a8a3969631c193e78cba83e97d61b2d67af89d52ec8a0df84439134b4a1db` |
| `cg/results.json` | `e4b720f7fb628fb91621a09dff818f11118cc42af07193387deb2b6b82a03960` |
| `cg/self-deltas.json` | `902e7d963bdd5bd8d90c14ac2b3bcd05fef647f79c5e4e4819ff2d6acdbaa2e3` |

### Simulation-only caller validation (2026-11-08)

The new `player_level_advance` companion leaves simulation code unchanged and uses the public setup and
commands of the full runner. In debug, all **36** independent C-derived final-state and
successful-call/refusal-boundary checks passed: six DSIM38 scripts and thirty DSIM39 scripts. The
checker reads frozen C projections, using the independent pre-refusal fixtures for the two H7 cases; it
does not use another Rust run as its oracle. This validates final states, not a newly observed complete
C trace. Release validation passed all 36 checks on 2026-11-09 before the completed idle-machine
campaign recorded above. Measurement boundaries and commands are in
[TESTING](../TESTING.md#whole-level-simulation-only-timing).

## README fresh-reader check

Run on 2026-11-05 after the owner's README and ROADMAP edits, by a reviewer with no prior view of
the project reading README.md cold, then applying [the documentation review checklist](../DOC_REVIEW_CHECKLIST.md).
Asked which item made the project most interesting, the reader named the live editor with rewind
("Change anything while the game runs, then rewind and replay from any moment"), as the concrete
payoff of the strictness that the "Why this exists" section leaves abstract; the runner-up was
"Why did that happen?". The reader could tell what exists today from what is a plan. Five
sentences were found misleading or hard to read (demo playback presented as a present use, a
rocket-jump sentence beside the then-unverified rocket launcher, an ambiguous "It", "small test
maps" against the real E1M1 bullet, and a fragment in the in-progress paragraph) and were
reworded; the forward references to Track 1 and Track 3 now say "below", "genuine engine" is
defined where first used, and the history clause in the wire-21 policy row was cut. The
owner-approved "Where it's (maybe) going" items and the long-term and mutation-testing sentences
were not changed. The README is 197 lines; `tools/check_docs.py` is clean.

## Mutation testing

Recorded 2026-11-08 (America/Los_Angeles). These campaigns test the Rust
implementation and refusal witnesses; they are neither independent C traces nor
independent DSIM expectations. Test expectations cite the original C source or
the written Rust admission rule. Existing reference fixtures were unchanged.

The campaigns used cargo-mutants 27.1.0 and cargo-nextest 0.9.151,
`CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG=0`. Run 1 selects the
milestone diff against master, split into behaviour and reader/encoder shards;
run 2 selects the rejection, rollback and refusal functions. Its function-name
filters exclude lifecycle; cargo-mutants also executes the struct-field deletion
controls shown in the per-file counts.
The original selections and full outcomes remain in the owner's authorized
`~/scratch/m4a/mutants/` directory; each targeted rerun uses one exact, anchored
`--re` per mutation. The working copy was synced before each run. Mutation locations below refer to the campaign source revision.

### Campaign counts by file

Counts below are the original campaign outcomes, before survivor tests and
timeout reruns. A timeout is counted as caught only where the separate rerun
and the code's loop condition justify it.

#### Run 1, shard 1

| File | Caught | Missed | Unviable | Timeout | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| `barrels.rs` | 13 | 6 | 0 | 0 | 19 |
| `combat.rs` | 13 | 4 | 21 | 0 | 38 |
| `combat_trace.rs` | 46 | 1 | 11 | 0 | 58 |
| `demo.rs` | 18 | 0 | 1 | 0 | 19 |
| `demon.rs` | 4 | 1 | 0 | 0 | 5 |
| `demon_expanded_map.rs` | 44 | 5 | 8 | 0 | 57 |
| `demon_map_spawn.rs` | 9 | 2 | 0 | 0 | 11 |
| `e1m1_rules.rs` | 52 | 22 | 2 | 0 | 76 |
| `episode1.rs` | 101 | 13 | 0 | 0 | 114 |
| `geometry.rs` | 0 | 0 | 1 | 0 | 1 |
| `lib.rs` | 1 | 0 | 2 | 0 | 3 |
| `lifecycle.rs` | 13 | 8 | 1 | 0 | 22 |
| `line_triggers.rs` | 6 | 0 | 0 | 0 | 6 |
| `map_extent.rs` | 0 | 0 | 1 | 0 | 1 |
| `monsters.rs` | 1 | 0 | 14 | 0 | 15 |
| `movement.rs` | 9 | 3 | 1 | 0 | 13 |
| `mover_rules.rs` | 78 | 30 | 5 | 3 | 116 |
| `movers.rs` | 102 | 7 | 6 | 1 | 116 |
| `original_player.rs` | 9 | 2 | 0 | 0 | 11 |
| `original_registry.rs` | 73 | 4 | 7 | 0 | 84 |
| `perception.rs` | 4 | 0 | 0 | 0 | 4 |
| `planes.rs` | 1 | 0 | 15 | 0 | 16 |
| `player_commands.rs` | 23 | 0 | 1 | 0 | 24 |
| `player_map_scale.rs` | 6 | 1 | 1 | 0 | 8 |
| `player_monsters.rs` | 0 | 0 | 2 | 0 | 2 |
| `player_pistol.rs` | 36 | 10 | 3 | 0 | 49 |
| `player_roster.rs` | 6 | 0 | 0 | 0 | 6 |
| `player_weapons.rs` | 31 | 2 | 4 | 0 | 37 |
| `projectiles.rs` | 20 | 5 | 1 | 0 | 26 |
| `pursuit.rs` | 5 | 0 | 0 | 0 | 5 |
| `sector_thinkers.rs` | 20 | 4 | 0 | 0 | 24 |
| `shotgunner.rs` | 2 | 0 | 1 | 0 | 3 |
| `sight.rs` | 24 | 0 | 4 | 0 | 28 |
| `spawning.rs` | 8 | 1 | 1 | 0 | 10 |
| `thinkers.rs` | 1 | 0 | 0 | 0 | 1 |
| `use_lines.rs` | 40 | 4 | 0 | 0 | 44 |
| `weapons.rs` | 82 | 17 | 3 | 0 | 102 |
| `world_pickups.rs` | 125 | 30 | 7 | 0 | 162 |
| **Total** | **1026** | **182** | **124** | **4** | **1336** |

#### Run 1, shard 2

| File | Caught | Missed | Unviable | Timeout | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| `diagnostic.rs` | 311 | 109 | 4 | 0 | 424 |
| `snapshot.rs` | 19 | 0 | 1 | 0 | 20 |
| **Total** | **330** | **109** | **5** | **0** | **444** |

#### Run 2 (executed records)

| File | Caught | Missed | Unviable | Timeout | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| `demon_expanded_map.rs` | 56 | 1 | 0 | 0 | 57 |
| `episode1.rs` | 108 | 6 | 0 | 0 | 114 |
| `lib.rs` | 40 | 2 | 0 | 0 | 42 |
| `lifecycle.rs` | 6 | 0 | 0 | 0 | 6 |
| `mover_rules.rs` | 142 | 12 | 2 | 6 | 162 |
| `movers.rs` | 88 | 4 | 6 | 1 | 99 |
| `original_player.rs` | 80 | 9 | 0 | 0 | 89 |
| `original_registry.rs` | 38 | 2 | 1 | 0 | 41 |
| `player_commands.rs` | 33 | 6 | 1 | 0 | 40 |
| `player_map_scale.rs` | 16 | 1 | 1 | 0 | 18 |
| `player_monsters.rs` | 0 | 0 | 1 | 0 | 1 |
| `spawning.rs` | 6 | 2 | 0 | 0 | 8 |
| `store.rs` | 14 | 1 | 0 | 0 | 15 |
| `world_pickups.rs` | 10 | 3 | 0 | 0 | 13 |
| **Total** | **637** | **49** | **12** | **7** | **705** |

Run 2 comprises 705 executed records and 693 unique mutations: twelve field
deletion records repeat across the partial run and its continuation. Eleven
repeated records were caught; the deleted `side` field was missed twice. Thus
its original unique counts are 626 caught, 48 missed, 12 unviable and 7
timeouts. The 48 unique misses comprise 19 carried-over run-1 equivalents,
10 new equivalents and 19 mutation-test targets.

### Historical run settings

The initial run-1 attempt was stopped after 22 of 1,780 mutants because its
measured projection exceeded the owner's 12-hour guardrail; it is not a
completed campaign. Shard 1 subsequently used 3 mutant jobs, with no owner
build/thread caps then; shard 2 and its survivor rerun used 2 jobs. Run 2's
first 387 records ran before the caps (08:51–12:06 PST on November 7); its
318-record continuation used 2 mutant jobs, `CARGO_BUILD_JOBS=3` and
`NEXTEST_TEST_THREADS=2`. Its four earlier timeout reruns used one mutant
job, 1,800-second test timeouts and the same build/thread caps. The owner
lifted those job/thread caps on November 8. No simultaneous campaign, build,
test or fuzz job ran during this resumption's mutation campaigns. These are
validation durations, not idle-machine performance measurements.

### Refusal witnesses tested below the public API

The 23 shard 1 survivor mutations reclassified "test" from "harmless" are killed by 10 crate-internal tests (one more, `a_negative_bonus_count_is_refused`, pins a witness without a survivor). Each test's doc comment now names its refusal path and why no constructible map reaches it through the public API (commit `c3d2192`).

| file | test | refusal path | survivors |
|---|---|---|---|
| `crates/doom-sim/src/player_episode1_items_tests.rs` | `just_attacked_at_a_tic_boundary_needs_the_chainsaw_ready` | `Simulation::preflight_player_body`, the `MF_JUSTATTACKED` exception (original_player.rs:413) | original_player.rs:413:33 |
| `crates/doom-sim/src/player_episode1_items_tests.rs` | `a_moving_item_at_a_tic_boundary_is_refused` | `Simulation::preflight_player_body`, the airborne-mover rule (original_player.rs:422) | original_player.rs:422:40 |
| `crates/doom-sim/src/player_episode1_weapons_tests.rs` | `a_weapon_adapter_with_another_actor_slot_or_maxima_is_refused` | `PlayerWeaponState::after_adapter`, the adapter identity rule (player_weapons.rs:87–88) | player_weapons.rs:87:13, 88:13 |
| `crates/doom-sim/src/player_episode1_items_tests.rs` | `a_cycling_item_accepts_only_its_own_loop_states` | item witness `world_pickups::state_allowed` (world_pickups.rs:318) in the tic preflight | world_pickups.rs:318:40 |
| `crates/doom-sim/src/player_episode1_items_tests.rs` | `a_stored_medikit_needed_message_is_refused` | pickup-state witness `PlayerPickupState::validate` (world_pickups.rs:420) | world_pickups.rs:420:13 |
| `crates/doom-sim/src/player_episode1_items_tests.rs` | `item_statistics_outside_the_written_domain_are_refused` | `Simulation::preflight_world_pickups`, the item-statistics witness (world_pickups.rs:1189, 1229–1230) | world_pickups.rs:1189:9, 1229:17, 1229:28, 1229:37, 1230:17, 1230:27 (`==`, `<`) |
| `crates/doom-sim/src/player_episode1_items_tests.rs` | `a_zero_item_total_before_the_thing_batch_is_accepted` | `Simulation::preflight_world_pickups`, the zero-total case (world_pickups.rs:1230) | world_pickups.rs:1230:27 (`>=`) |
| `crates/doom-sim/src/player_episode1_weapons_tests.rs` | `a_flying_rocket_without_an_owner_is_refused` | `Simulation::preflight_live_missiles`, the live-missile owner witness (projectiles.rs:855) | projectiles.rs:855:9 |
| `crates/doom-sim/src/player_episode1_items_tests.rs` | `a_decoration_whose_sprite_or_frame_disagrees_with_its_state_is_refused` | decoration witness `e1m1_rules::validate_decoration` (e1m1_rules.rs:291–292) | e1m1_rules.rs:291:9, 292:9 |
| `crates/doom-sim/src/player_episode1_items_tests.rs` | `power_counters_outside_their_domains_are_refused_one_at_a_time` | `episode1::validate_powers` (episode1.rs:254–260) in the tic preflight | episode1.rs:254:5, 256:9–260:9 |

### Reader boundary and older-reader gap

The prior owner-approved reader change requires DSIM39 `store.max_slots` to
be exactly 1,024 (`fb2df32`), as decision D1 and STATE_FORMAT require. Tests
refuse 1,023 and 1,025, accept 1,024, and decode every committed v39 fixture.
Historical wire ≤22 survivor tests pin the frozen current reader behaviour;
they do not upgrade it to C equivalence. The v38 and older readers remain frozen: they check only their upper bound,
so a v38 capture can carry a value below the format's fixed 384. This is an
older-reader gap, not a claim that the older format permits those values.

### Final dispositions and targeted checks

| Campaign | Caught, including proven nontermination | Equivalent | Observation-only harmless | Unviable | Unresolved |
| --- | ---: | ---: | ---: | ---: | ---: |
| Run 1 shard 1 | 1120 | 90 | 2 | 124 | 0 |
| Run 1 shard 2 | 435 | 4 | 0 | 5 | 0 |
| Run 2, execution records | 664 | 29 | 0 | 12 | 0 |
| Run 2, unique mutations | 652 | 29 | 0 | 12 | 0 |

Run 1 shard 1 added 90 survivor kills (89 earlier, one dense-barrel witness)
and four nontermination catches. Shard 2 added 105 survivor kills. Run 2
added 19 unique survivor kills (20 execution records) and seven
nontermination catches. Unviable mutations are excluded from viable totals,
not counted as killed. Run 1's two observation-only changes remain explicitly
qualified below; they do not alter simulation state or refusal decisions.

The final work-branch Rust tree passed all 1,218 doom-sim nextest tests (two
skipped), workspace/all-targets Clippy with warnings denied, workspace and fuzz
format checks, and the dense-barrel test in release. The four work commits were
cherry-picked individually; Clippy passed at each resulting commit. The initial
WIP's thirty-plat test needed a neutral tic before use: G_PlayerReborn initializes
usedown (g_game.c:822). An initial validation build exhausted the 3.9 GB `/tmp`
tmpfs before tests; rerunning with persistent scratch build directories passed.
No simulation implementation was changed during this survivor follow-up.

`run2_survivors_codex` caught all 19 selected unique survivor targets; the
executed selection caught 30/30 including eleven automatic field-deletion
controls, with no misses, timeouts or unviable results (1,347 seconds;
2026-11-08 00:25:24–00:47:51 PST; mutant/build/test concurrency 3/4/4).
`run1_barrel_codex` selected exactly the remaining barrel target and caught
1/1, with no misses, timeouts or unviable results (179 seconds;
15:22:41–15:25:40 PST; concurrency 1/6/6).

The remaining `barrels.rs:362:50` mutant (`*` to `+` in `radius_attack`)
is killed by `player_barrel_combat_tests::a_dense_barrel_walk_preserves_nonshootable_items`.
The barrel witness uses valid public operations with 16 monsters and either
zero or 256 nonshootable clips. Its expected states, attribution and RNG draw
counts derive from p_maputl.c:428–442,529–535, p_map.c:1170–1194,
p_inter.c:888–915, info.c:946–947,1006,2746–2770,
p_mobj.c:506,787–788 and m_random.c:57–60. The clips must survive unchanged;
the explosion must advance and damage actors with the player's attribution.
This is a Rust admission regression, not an independently executed C trace.
Center-based BLOCKMAP membership invalidated the earlier nine-actors-per-cell
packing argument; collision boxes need not fit inside their cells (§14 of the
investigation records the correction).

### Closed dispositions by file

Caught includes the individually proven nontermination cases. Run 2 below counts
execution records; the twelve duplicate records are reconciled in the unique
totals above. These are final dispositions of the original campaign selections.

#### Run 1 shard 1

| File | Caught | Equivalent | Harmless | Unviable |
| --- | ---: | ---: | ---: | ---: |
| `barrels.rs` | 14 | 5 | 0 | 0 |
| `combat.rs` | 15 | 2 | 0 | 21 |
| `combat_trace.rs` | 47 | 0 | 0 | 11 |
| `demo.rs` | 18 | 0 | 0 | 1 |
| `demon.rs` | 4 | 1 | 0 | 0 |
| `demon_expanded_map.rs` | 44 | 5 | 0 | 8 |
| `demon_map_spawn.rs` | 9 | 2 | 0 | 0 |
| `e1m1_rules.rs` | 54 | 20 | 0 | 2 |
| `episode1.rs` | 108 | 6 | 0 | 0 |
| `geometry.rs` | 0 | 0 | 0 | 1 |
| `lib.rs` | 1 | 0 | 0 | 2 |
| `lifecycle.rs` | 18 | 3 | 0 | 1 |
| `line_triggers.rs` | 6 | 0 | 0 | 0 |
| `map_extent.rs` | 0 | 0 | 0 | 1 |
| `monsters.rs` | 1 | 0 | 0 | 14 |
| `movement.rs` | 9 | 1 | 2 | 1 |
| `mover_rules.rs` | 105 | 6 | 0 | 5 |
| `movers.rs` | 108 | 2 | 0 | 6 |
| `original_player.rs` | 11 | 0 | 0 | 0 |
| `original_registry.rs` | 73 | 4 | 0 | 7 |
| `perception.rs` | 4 | 0 | 0 | 0 |
| `planes.rs` | 1 | 0 | 0 | 15 |
| `player_commands.rs` | 23 | 0 | 0 | 1 |
| `player_map_scale.rs` | 6 | 1 | 0 | 1 |
| `player_monsters.rs` | 0 | 0 | 0 | 2 |
| `player_pistol.rs` | 41 | 5 | 0 | 3 |
| `player_roster.rs` | 6 | 0 | 0 | 0 |
| `player_weapons.rs` | 33 | 0 | 0 | 4 |
| `projectiles.rs` | 24 | 1 | 0 | 1 |
| `pursuit.rs` | 5 | 0 | 0 | 0 |
| `sector_thinkers.rs` | 21 | 3 | 0 | 0 |
| `shotgunner.rs` | 2 | 0 | 0 | 1 |
| `sight.rs` | 24 | 0 | 0 | 4 |
| `spawning.rs` | 8 | 1 | 0 | 1 |
| `thinkers.rs` | 1 | 0 | 0 | 0 |
| `use_lines.rs` | 42 | 2 | 0 | 0 |
| `weapons.rs` | 95 | 4 | 0 | 3 |
| `world_pickups.rs` | 139 | 16 | 0 | 7 |
| **Total** | 1120 | 90 | 2 | 124 |

#### Run 1 shard 2

| File | Caught | Equivalent | Harmless | Unviable |
| --- | ---: | ---: | ---: | ---: |
| `diagnostic.rs` | 416 | 4 | 0 | 4 |
| `snapshot.rs` | 19 | 0 | 0 | 1 |
| **Total** | 435 | 4 | 0 | 5 |

#### Run 2

| File | Caught | Equivalent | Harmless | Unviable |
| --- | ---: | ---: | ---: | ---: |
| `demon_expanded_map.rs` | 57 | 0 | 0 | 0 |
| `episode1.rs` | 108 | 6 | 0 | 0 |
| `lib.rs` | 42 | 0 | 0 | 0 |
| `lifecycle.rs` | 6 | 0 | 0 | 0 |
| `mover_rules.rs` | 154 | 6 | 0 | 2 |
| `movers.rs` | 91 | 2 | 0 | 6 |
| `original_player.rs` | 85 | 4 | 0 | 0 |
| `original_registry.rs` | 40 | 0 | 0 | 1 |
| `player_commands.rs` | 34 | 5 | 0 | 1 |
| `player_map_scale.rs` | 16 | 1 | 0 | 1 |
| `player_monsters.rs` | 0 | 0 | 0 | 1 |
| `spawning.rs` | 7 | 1 | 0 | 0 |
| `store.rs` | 14 | 1 | 0 | 0 |
| `world_pickups.rs` | 10 | 3 | 0 | 0 |
| **Total** | 664 | 29 | 0 | 12 |

### Run 2 survivor tests now killing their targets

The seven tests marked internal below are owner-approved **refusal witnesses
tested below the public API**. They inject otherwise unconstructible invalid
state and call private preflights; they do not claim valid public-input coverage.
Together with the earlier 23 mutations covered by ten functions above, this
qualification is part of the evidence, not an equivalence claim. Other new
survivor tests exercise public operations. Expectations and source/rule citations
are in the test comments; none use captured Rust output as their oracle.

| Mutation | Killing test / qualification |
| --- | --- |
| `crates/doom-sim/src/lib.rs:532:13: delete field side from struct TicCommand expression in apply_player_motion` | player_episode1_commands_tests::an_airborne_body_ignores_its_side_move; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/original_player.rs:384:21: replace \|\| with && in Simulation::preflight_player_body` | Owner-approved internal refusal witness below the public API (2026-11-08): player_episode1_items_tests::a_live_body_one_raw_unit_short_is_refused; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/original_player.rs:381:21: replace \|\| with && in Simulation::preflight_player_body` | Owner-approved internal refusal witness below the public API (2026-11-08): player_episode1_items_tests::a_corpse_flag_on_a_live_body_is_refused; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/original_player.rs:380:21: replace \|\| with && in Simulation::preflight_player_body` | Owner-approved internal refusal witness below the public API (2026-11-08): player_episode1_items_tests::a_dead_player_state_on_a_live_body_is_refused; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/original_player.rs:406:47: replace \|\| with && in Simulation::preflight_player_body` | Owner-approved internal refusal witness below the public API (2026-11-08): player_weapons::tests::a_pre_heights_body_off_its_floor_or_with_vertical_momentum_is_refused; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/original_player.rs:432:68: replace \|\| with && in Simulation::preflight_player_body` | Owner-approved internal refusal witness below the public API (2026-11-08): player_weapons::tests::a_pre_heights_moving_monster_off_its_floor_or_with_vertical_momentum_is_refused; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/original_registry.rs:1259:37: replace > with == in Simulation::with_original_registry` | player_weapons::tests::historical_weapon_registry_admits_128_lines_and_refuses_more; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/original_registry.rs:1259:37: replace > with >= in Simulation::with_original_registry` | player_weapons::tests::historical_weapon_registry_admits_128_lines_and_refuses_more; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/player_commands.rs:150:72: replace \|\| with && in Simulation::preflight_player_commands` | Owner-approved internal refusal witness below the public API (2026-11-08): player_commands::tests::a_bound_body_without_a_state_refuses_the_command_preflight; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/demon_expanded_map.rs:528:38: replace \|\| with && in Simulation::preflight_expanded_demons_for` | Owner-approved internal refusal witness below the public API (2026-11-08): demon_expanded_map::tests::missing_statistics_or_a_short_sound_target_table_is_refused; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/spawning.rs:230:36: replace == with != in Simulation::validate_entry` | m3c::a_noblood_spawn_child_is_refused_at_state_entry_outside_combat; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/mover_rules.rs:698:54: replace - with / in reachable` | player_movers_tests::a_door_below_zero_rises_to_its_neighbour_ceiling_minus_4; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/mover_rules.rs:718:21: replace \|\| with && in reachable` | player_movers_tests::adjacent_normal_doors_have_unbounded_heights; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/mover_rules.rs:722:25: replace \|= with &= in reachable` | player_movers_tests::adjacent_normal_doors_have_unbounded_heights; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/mover_rules.rs:746:23: replace \|\| with && in reachable` | player_movers_tests::lift_widenings_propagate_into_a_turbo_destination; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/mover_rules.rs:750:25: replace \|= with &= in reachable` | player_movers_tests::lift_widenings_propagate_into_a_turbo_destination; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/mover_rules.rs:760:29: replace \|= with &= in reachable` | player_movers_tests::adjacent_turbo_floors_with_different_heights_are_unbounded; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/movers.rs:658:55: replace > with == in Starter<'_, M>::do_plat` | player_episode1_movers_tests::thirty_plats_started_by_one_switch_are_all_active; killed in run2_survivors_codex (all 30 executed mutants caught) |
| `crates/doom-sim/src/movers.rs:658:55: replace > with >= in Starter<'_, M>::do_plat` | player_episode1_movers_tests::thirty_plats_started_by_one_switch_are_all_active; killed in run2_survivors_codex (all 30 executed mutants caught) |

### Timeout proofs and individual reruns

Each of the seven mutations below was rerun alone with a 1,800-second test
timeout and a successful build. Loop reasons were read and recorded before
counting them caught. The final three used `run2_timeouts_remaining_codex_retry1`
(5,737 seconds; 01:57:28–03:33:05 PST; concurrency 1/4/2). An earlier attempt
was deliberately interrupted during its baseline build after 22 seconds for
the CI repair and produced no mutant result. The other four individual reruns
are in `run2_timeouts`. These proofs also resolve the four run-1 timeout targets.

| Mutation | Why it does not terminate | Test timeout observed (seconds) |
| --- | --- | ---: |
| `crates/doom-sim/src/mover_rules.rs:607:5: replace widen -> bool with true` | Returning true from widen means every settled normal-door/lift/turbo interval reports a change forever under nonsaturating older policies. The reachable fixpoint cannot break; the unchanged bounded heights cannot trigger its out-of-bounds refusal. | 1800.01 |
| `crates/doom-sim/src/mover_rules.rs:610:15: replace != with == in widen` | Reversing != to == makes widen report a change on an unchanged interval. On an older-policy map whose widening is already settled, changed stays true each round; the bounded intervals remain unchanged and the fixpoint loops forever. | 1800.06 |
| `crates/doom-sim/src/mover_rules.rs:636:19: replace += with *= in stair_bounds` | k *= 1 keeps the stair component-walk index at zero, so component.get(k) repeatedly returns its start sector. | 1800.03 |
| `crates/doom-sim/src/mover_rules.rs:782:25: replace != with == in reachable` | changed = f == pf \|\| c != pc is true when the saturated floor and ceiling intervals have both settled; the fixpoint therefore never breaks. | 1800.04 |
| `crates/doom-sim/src/mover_rules.rs:782:36: replace != with == in reachable` | changed = f != pf \|\| c == pc is true when the saturated floor and ceiling intervals have both settled; the fixpoint therefore never breaks. | 1800.01 |
| `crates/doom-sim/src/mover_rules.rs:786:12: delete ! in reachable` | Deleting ! changes the exit to if changed. A map with no mover-induced changes has changed=false on every pass, so it never breaks; this includes the zero-mover constructor cases. On changing maps it may instead terminate prematurely. | 1800.05 |
| `crates/doom-sim/src/movers.rs:755:28: replace + with * in Starter<'_, M>::build_stairs` | secnum = found * 1 restarts the inclusive tag search at the same sector. After spawning its stair thinker that sector is busy; continue finds it again indefinitely. | 1800.01 |

### Every surviving equivalence and observation-only change

The following tables enumerate each campaign's equivalent survivors, including
the 19 run-1 proofs carried into run 2. Repeated records are identified above.
Reasons concern all reachable inputs, rather than a missing boundary test.

#### Run 1 shard 1: equivalences

| Mutation | Reason |
| --- | --- |
| `crates/doom-sim/src/original_registry.rs:761:56: replace && with \|\| in rocket_policy` | compile-time check of the info.c rocket closure: S_ROCKET, S_EXPLODE2/3 have no action and S_EXPLODE1 has A_Explode, so the widened predicate gives the same Ok. |
| `crates/doom-sim/src/original_registry.rs:784:17: replace \|\| with && in spectre_policy` | as 782:17. |
| `crates/doom-sim/src/original_registry.rs:783:17: replace \|\| with && in spectre_policy` | as 782:17. |
| `crates/doom-sim/src/original_registry.rs:782:17: replace \|\| with && in spectre_policy` | compile-time check: the Spectre's four closures equal the Demon's (info.c:1420–1470), so the widened predicate gives the same Ok. |
| `crates/doom-sim/src/player_map_scale.rs:944:17: replace && with \|\| in Simulation::spawn_admitted` | unreachable (reclassified from harmless): every wire-23 spawn position is inside the room rectangle (a THING outside it is refused earlier at spawn, observed `Monster(Arithmetic)`; missiles, puffs, blood and drops spawn at an actor or a trace point inside the lines), so `room.contains` is always true and `&&`/`\|\|` agree on every reachable input. |
| `crates/doom-sim/src/player_pistol.rs:240:9: replace <impl weapons::Host for Host<'_, '_, A>>::admit_weapon -> Result<(), SimError> with Ok(())` | the only caller (P_CheckAmmo's fallback) can name only owned weapons, and the fallback set (chaingun, shotgun, pistol, chainsaw, launcher, fist) is exactly what each policy owns; every call returns Ok. |
| `crates/doom-sim/src/player_pistol.rs:245:17: replace && with \|\| in <impl weapons::Host for Host<'_, '_, A>>::admit_weapon` | as 240:9: Rocket/Chainsaw are never owned outside wire 23, and every fallback is admitted under wire 23. |
| `crates/doom-sim/src/player_pistol.rs:253:9: replace <impl weapons::Host for Host<'_, '_, A>>::episode1 -> bool with true` | wire 22's command preflight refuses a selection above 3 before the machine runs (weapon_selections_under_wire_23 asserts it), and for selections 0–3 the two branches coincide without a chainsaw. |
| `crates/doom-sim/src/player_pistol.rs:259:9: replace <impl weapons::Host for Host<'_, '_, A>>::strength -> bool with false` | powers[0] (pw_strength) is 0 under every admitted policy (validate_powers refuses anything else; no berserk is admitted), so strength() is always false. |
| `crates/doom-sim/src/player_pistol.rs:737:54: replace > with >= in saw` | delta exactly ANG180 is impossible: A_Saw's linetarget lies on its aim line, within the ±(255<<18) spread plus the target's width of the facing, never opposite it. |
| `crates/doom-sim/src/world_pickups.rs:44:20: replace match guard policy.has_episode1() with true in pickup_bound` | world.episode1 is Some only under wire 23, so `Some(b) if has_episode1()` and `Some(b)` match the same inputs. |
| `crates/doom-sim/src/world_pickups.rs:55:20: replace match guard policy.has_episode1() with true in bonus_bound` | as 44:20. |
| `crates/doom-sim/src/world_pickups.rs:67:20: replace match guard policy.has_episode1() with true in world_item_bound` | as 44:20. |
| `crates/doom-sim/src/world_pickups.rs:240:13: replace \| with ^ in PickupKind::definition_flags` | no match arm contains MF_SPECIAL (MF_COUNTITEM, MF_NOTDMATCH or 0), so `\|` and `^` give the same flags. |
| `crates/doom-sim/src/world_pickups.rs:303:15: replace += with -= in spawn_loop` | every cycling pickup loop (info.c:964–1005) closes on its spawn state within 6 states, so the step counter never ends the loop. |
| `crates/doom-sim/src/world_pickups.rs:303:15: replace += with *= in spawn_loop` | every cycling pickup loop (info.c:964–1005) closes on its spawn state within 6 states, so the step counter never ends the loop. |
| `crates/doom-sim/src/world_pickups.rs:305:40: replace < with <= in spawn_loop` | as 303:15. |
| `crates/doom-sim/src/world_pickups.rs:537:13: replace && with \|\| in validate_definition_uncached` | as 533:22. |
| `crates/doom-sim/src/world_pickups.rs:534:13: replace && with \|\| in validate_definition_uncached` | as 533:22. |
| `crates/doom-sim/src/world_pickups.rs:513:5: replace validate_definition_uncached -> Result<(), SimError> with Ok(())` | compile-time table: every pickup kind satisfies its definition predicate (info.c), so the result is Ok either way. |
| `crates/doom-sim/src/world_pickups.rs:533:22: replace < with <= in validate_definition_uncached` | compile-time table: widening only; every cycling kind passes the original predicate. |
| `crates/doom-sim/src/world_pickups.rs:539:70: replace && with \|\| in validate_definition_uncached` | as 533:22. |
| `crates/doom-sim/src/world_pickups.rs:539:37: replace && with \|\| in validate_definition_uncached` | as 533:22. |
| `crates/doom-sim/src/world_pickups.rs:539:33: replace > with >= in validate_definition_uncached` | as 533:22. |
| `crates/doom-sim/src/world_pickups.rs:749:22: replace match guard matches!(p.ready, Weapon::Fist \| Weapon::Pistol) && owns(Weapon::Plasma) with false in ammo` | the plasma rifle is never owned under any admitted policy (inventory bits 0x9f, no plasma pickup), so the guard is always false. |
| `crates/doom-sim/src/world_pickups.rs:1046:40: replace \|= with &= in <impl movement::MotionAccess for Contacts<'_, A>>::contacts` | the player body is also flagged through the working copy `body` (line 1040), which the move writes back over the stored actor after the contact pass, so the stored write is overwritten either way. |
| `crates/doom-sim/src/demon.rs:81:9: replace && with \|\| in strike_player` | the Spectre is admitted only under wire 23; under wire 23 the widened test passes any type, but the next clause requires S_SARG_ATK3, a state only the Demon and the Spectre can hold. |
| `crates/doom-sim/src/demon_expanded_map.rs:156:36: replace * with + in Simulation::with_demon_expanded_profile` | an overflow guard whose value is discarded: (sight_work + 5·lines)·queries fits usize for every admitted map, and `5 + lines`/`5 / lines` are smaller, so both forms pass on every input. |
| `crates/doom-sim/src/demon_expanded_map.rs:156:36: replace * with / in Simulation::with_demon_expanded_profile` | an overflow guard whose value is discarded: (sight_work + 5·lines)·queries fits usize for every admitted map, and `5 + lines`/`5 / lines` are smaller, so both forms pass on every input. |
| `crates/doom-sim/src/demon_expanded_map.rs:374:18: replace > with == in action_sight` | the count never reaches the limit: per tic a monster runs its own action plus at most one damage-entered seestate A_Chase, each making at most 2 sight queries (A_Look: P_LookForPlayers stops after 2 players, p_enemy.c:520; A_Chase: either the re-look (≤ 2) or P_CheckMeleeRange + P_CheckMissileRange (≤ 2), p_enemy.c:174–201, 672–760), so ≤ 4K < max(64, 8(K+1)) under wire 23 and ≤ 32 < 64 under the older wires (K ≤ 8); barrel/rocket explosions charge their own control. `>`, `>=` and `==` are all false on every input. |
| `crates/doom-sim/src/demon_expanded_map.rs:374:18: replace > with >= in action_sight` | the count never reaches the limit: per tic a monster runs its own action plus at most one damage-entered seestate A_Chase, each making at most 2 sight queries (A_Look: P_LookForPlayers stops after 2 players, p_enemy.c:520; A_Chase: either the re-look (≤ 2) or P_CheckMeleeRange + P_CheckMissileRange (≤ 2), p_enemy.c:174–201, 672–760), so ≤ 4K < max(64, 8(K+1)) under wire 23 and ≤ 32 < 64 under the older wires (K ≤ 8); barrel/rocket explosions charge their own control. `>`, `>=` and `==` are all false on every input. |
| `crates/doom-sim/src/demon_expanded_map.rs:374:52: replace > with >= in action_sight` | work is the per-query charge times the count and the budget is the same charge times the limit (map_extent.rs:197–203), so `work >= budget` needs count ≥ limit, which never happens (see 374:18). |
| `crates/doom-sim/src/demon_map_spawn.rs:106:29: replace += with *= in Simulation::preflight_map_demons` | redundant re-count: spawning.rs refuses the same DecorationLimit before any spawn (player_e1m1_tests.rs:917), and under wire 23 the limit is the THINGS lump's own count D, which the spawned count can never exceed. |
| `crates/doom-sim/src/demon_map_spawn.rs:115:25: replace += with *= in Simulation::preflight_map_demons` | redundant re-count: spawning.rs refuses BarrelLimit before any spawn (player_barrel_combat_tests.rs:282); under wire 23 the limit is the lump's own B. |
| `crates/doom-sim/src/projectiles.rs:610:9: replace \|\| with && in launch_player_missile` | launch_player_missile's only caller is A_FireMissile, which exists only in the wire-23 launcher and passes MT_ROCKET, so both forms pass. |
| `crates/doom-sim/src/weapons.rs:308:9: replace Host::episode1 -> bool with true` | default used only by the PlayerWeapons fixture's Preview/Commit hosts, whose step preflight refuses selections above 3 (weapons.rs:1143); for 0–3 both branches coincide without a chainsaw. |
| `crates/doom-sim/src/weapons.rs:318:9: replace Host::take_drop -> bool with true` | read only after A_FireMissile (after_fire), which the fixture hosts using the default never run. |
| `crates/doom-sim/src/weapons.rs:312:9: replace Host::strength -> bool with true` | read only in the wire-23 branch (line 634), which the fixture hosts using the default never enter. |
| `crates/doom-sim/src/weapons.rs:634:40: replace == with != in Machine<'_, H>::weapon_change` | strength() is always false under every admitted policy (see player_pistol.rs:259 false), so `!(x && false)` is true for either comparison. |
| `crates/doom-sim/src/combat.rs:487:49: replace && with \|\| in DamagePlan::current_original_in` | both forms give the same result (reclassified from harmless): the condition gates only the `impulse()` bounds preflight, which applies nothing; a damaged player body is always live (P_DamageMobj callers skip non-shootable targets) and `thrust` is false only when the player saws himself, which A_Saw cannot do, so `live && thrust` = `live \|\| thrust` on every reachable call. |
| `crates/doom-sim/src/combat.rs:528:45: replace && with \|\| in DamagePlan::current_original_in` | the extra check never refuses (reclassified from harmless): it gates only the `impulse()` bounds preflight (nothing applied); the extra preflight on a chainsaw hit (at most 20 damage, at most 2.5 units/tic) is far inside the travel bound, so it never refuses; the thrust itself is not applied (existing saw_hit_lunges_and_does_not_thrust). |
| `crates/doom-sim/src/lifecycle.rs:881:9: replace ActorRegistry::validate_state_rocket -> Result<(), ActorError> with Ok(())` | the rocket's states come from the built-in info.c table, which original_registry's rocket_policy proves satisfy the rocket closure at first use; no other registry reaches validate_state_rocket. |
| `crates/doom-sim/src/lifecycle.rs:936:9: replace ActorRegistry::validate_type_original -> Result<(), ActorError> with Ok(())` | validate_type_original runs only on the built-in original registry, whose types the original_registry policies prove admissible; every call returns Ok either way. |
| `crates/doom-sim/src/lifecycle.rs:969:42: replace \| with ^ in ActorRegistry::validate_type_original` | ALLOWED_FLAGS lacks MF_MISSILE (admitted only here, per the comment), so `\|` and `^` give the same mask. |
| `crates/doom-sim/src/spawning.rs:195:17: replace && with \|\| in Simulation::validate_entry` | unreachable (reclassified from harmless): validate_entry is not reached under wire 23 (monster entry routes to enter_monster_state, request_actor_death refuses the demon lifecycle; under the mutant every wire-23 entry would be checked against the rocket closure and refused, and the whole suite passed), and under the older imps policies no actor holds a rocket state. |
| `crates/doom-sim/src/use_lines.rs:320:25: replace \|\| with && in shoot_lines` | shoot_special_line acts only on 46 (it returns for every other special), so continuing or not on a non-46 line is the same, and 46 proceeds under both forms. |
| `crates/doom-sim/src/use_lines.rs:320:29: delete ! in shoot_lines` | as 320:25: with `!` removed only non-46 lines change routing, and those are no-ops. |
| `crates/doom-sim/src/barrels.rs:280:32: replace \|\| with && in ExplosionSight::charge` | `one` is fixed per map, so `work` is always `queries·one` and `budget` is `allowance·one`: `queries > allowance` holds exactly when `work > budget`. |
| `crates/doom-sim/src/barrels.rs:309:9: replace && with \|\| in radius_attack` | MT_ROCKET is admitted only under wire 23, so `is_rocket` implies `has_episode1`; under wire 23 the widened test admits any type, but `A_Explode` is admitted only in S_BEXP4 and S_EXPLODE1, so only barrels and rockets reach the check. |
| `crates/doom-sim/src/barrels.rs:362:19: replace * with + in radius_attack` | the refusal cannot fire under either form (reclassified from harmless): `9 + order·(K+2)` ≥ `order·(K+2)` > `order·(1+K)`, the most visits one explosion can make (one visit per thing plus one revisit pass per relinking monster). |
| `crates/doom-sim/src/barrels.rs:362:60: replace + with - in radius_attack` | the refusal cannot fire under any form (reclassified from harmless): roster term K ≥ 8 (`visit_roster` floor), so `9·order·2K` and `9·order·(K−2)` (8K ≥ 19) both stay ≥ `order·(1+K)`, the most visits one explosion can make. |
| `crates/doom-sim/src/barrels.rs:362:60: replace + with * in radius_attack` | the refusal cannot fire under any form (reclassified from harmless): roster term K ≥ 8 (`visit_roster` floor), so `9·order·2K` and `9·order·(K−2)` (8K ≥ 19) both stay ≥ `order·(1+K)`, the most visits one explosion can make. |
| `crates/doom-sim/src/e1m1_rules.rs:157:75: replace < with <= in decoration_states` | compile-time table: every listed decoration's spawn loop closes on its spawn state within 4 states (info.c), so the `< 8`/`<= 8` cap never decides. |
| `crates/doom-sim/src/e1m1_rules.rs:208:5: replace animated_decoration -> bool with true` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:223:9: replace && with \|\| in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:222:9: replace && with \|\| in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:221:9: replace && with \|\| in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:220:9: replace && with \|\| in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:216:9: replace && with \|\| in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:215:9: replace && with \|\| in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:214:9: replace && with \|\| in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:213:9: replace && with \|\| in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:212:21: replace > with >= in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:213:25: replace > with >= in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:214:25: replace < with <= in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:218:46: replace && with \|\| in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:218:24: replace && with \|\| in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:218:20: replace > with >= in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:222:48: replace == with != in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:222:54: replace * with + in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:222:54: replace * with / in animated_decoration` | animated_decoration over the compile-time table: the firestick (MT_MISC43, info.c:3526–3550: 4 timed states, no action, MF_SOLID, health 1000, radius 16, height 16) is the only input reaching it and satisfies it; every mutant only widens acceptance. |
| `crates/doom-sim/src/e1m1_rules.rs:245:5: replace validate_listed_definition -> Result<(), ActorError> with Ok(())` | every listed decoration passes (the E1M1 seven and the candle the static predicate, the firestick the animated one, info.c:3526–3550, 3682–3706); the result is fixed by the compile-time table. |
| `crates/doom-sim/src/episode1.rs:85:40: replace \|\| with && in Episode1Bounds::from_things` | kinds 1–4 and 11 are in none of the counted lists (monsters, pickups, barrel, statics), so skipping them or not gives the same counts. |
| `crates/doom-sim/src/episode1.rs:118:20: replace > with >= in Episode1Bounds::sight_queries` | at equality (64) both branches return 64. |
| `crates/doom-sim/src/episode1.rs:126:20: replace > with >= in Episode1Bounds::explosion_sight_queries` | at equality (216) both branches return 216. |
| `crates/doom-sim/src/episode1.rs:135:26: replace > with >= in Episode1Bounds::visit_roster` | at equality (8) both branches return 8. |
| `crates/doom-sim/src/episode1.rs:235:26: replace += with -= in powers_tick` | dead branch: validate_powers (line 221, before it) refuses powers[1] ≠ 0, so `powers[1] != 0` is never true here. |
| `crates/doom-sim/src/episode1.rs:235:26: replace += with *= in powers_tick` | dead branch: validate_powers (line 221, before it) refuses powers[1] ≠ 0, so `powers[1] != 0` is never true here. |
| `crates/doom-sim/src/movement.rs:718:61: replace && with \|\| in Mover<'_, F>::slide` | the checked and unchecked intercept compute the same P_InterceptVector value and differ only by refusing on overflow, which bounded admitted coordinates never reach. |
| `crates/doom-sim/src/mover_rules.rs:732:25: replace \|= with &= in reachable` | close30 is set only under wire 23, where the fixpoint is saturated and line 782 overwrites `changed` with `f != pf \|\| c != pc` after every round: the per-line `changed` updates are discarded. |
| `crates/doom-sim/src/mover_rules.rs:738:25: replace \|= with &= in reachable` | raise is wire-23 only; `changed` is overwritten at line 782 under saturation, as 732:25. |
| `crates/doom-sim/src/mover_rules.rs:744:25: replace \|= with &= in reachable` | nearest/change are wire-23 only; `changed` is overwritten at line 782, as 732:25. |
| `crates/doom-sim/src/mover_rules.rs:767:25: replace \|= with &= in reachable` | stairs exist only under wire 23; `changed` is overwritten at line 782, as 732:25. |
| `crates/doom-sim/src/mover_rules.rs:774:21: replace \|= with &= in reachable` | donuts exist only under wire 23; `changed` is overwritten at line 782, as 732:25. |
| `crates/doom-sim/src/mover_rules.rs:776:21: replace \|= with &= in reachable` | as 774:21. |
| `crates/doom-sim/src/movers.rs:378:9: replace Starter<'_, M>::episode1 -> Result<(), SimError> with Ok(())` | unreachable: an earlier validator already rejects this input (reclassified from harmless): rule 1 admits the Episode 1 starters' specials only under wire 23 (`PursuitError::Specials` at load otherwise), where world.episode1 is always Some. |
| `crates/doom-sim/src/movers.rs:808:31: replace + with * in Starter<'_, M>::build_stairs` | `secnum = tsec` instead of `tsec + 1`: the next inclusive search can only re-find tsec if it is tagged, and tsec is now busy (its step was just spawned), so it is skipped with secnum = tsec + 1, the original state. |
| `crates/doom-sim/src/sector_thinkers.rs:442:23: replace match guard self.player_policy().has_episode1() with true in Simulation::spawn_level_specials` | the load rules refuse sector special 11 outside wire 23 (validate_sector_specials_for), so the guard never sees 11 under another policy. |
| `crates/doom-sim/src/sector_thinkers.rs:548:15: replace match guard episode1 with true in player_in_special_sector` | as 442:23: special 11 never loads outside wire 23. |
| `crates/doom-sim/src/sector_thinkers.rs:539:37: replace < with <= in player_in_special_sector` | rndtable (m_random.c) has no entry 5, so `< 5` and `<= 5` agree on every draw. |

#### Run 1 shard 2: equivalences

| Mutation | Reason |
| --- | --- |
| `crates/doom-sim/src/diagnostic.rs:1105:55: replace == with != in Diagnostic<'a>::validate_specials` | equivalent (C, dead clause): validate_specials runs only for version 34 (diagnostic.rs:908), so `self.version == 39 && _` is false in both forms; no input distinguishes them |
| `crates/doom-sim/src/diagnostic.rs:1493:17: replace \|\| with && in Diagnostic<'a>::validate_decorations` | equivalent (C, earlier refusal): the identity check of every occupied actor (validate loop, diagnostic.rs:646, actor -> identity -> state_role_for -> e1m1_rules::spawn_state, original_registry.rs:1104-1107) already refuses a v39 decoration state outside decoration_states, so the skipped clause never fires |
| `crates/doom-sim/src/diagnostic.rs:1795:51: replace \|\| with && in Diagnostic<'a>::validate_episode1` | equivalent (C, earlier refusal): validate_weapons (called at diagnostic.rs:892, before validate_episode1 at 924) already refuses cells or rockets above AMMO_CAPACITIES << backpack (PlayerWeaponState::validate_for, weapons.rs:196) with the same backpack, so neither `\|\|` nor `&&` fires when reached |
| `crates/doom-sim/src/diagnostic.rs:1816:28: replace && with \|\| in Diagnostic<'a>::validate_episode1` | equivalent (C, earlier refusal): validate_level_body (diagnostic.rs:1852, called at 921 before validate_episode1 at 924) refuses an `original_player.0.mo` that is not 13 bytes with presence 1, so `len == 13 && raw[0] == 1` and `\|\|` agree on every input that reaches line 1816 |

#### Run 2: equivalences

| Mutation | Reason |
| --- | --- |
| `crates/doom-sim/src/original_player.rs:398:84: replace \| with ^ in Simulation::preflight_player_body` | The source bit flags are pairwise disjoint, so XOR and OR form the same mask for every input (p_mobj.h:142-189 for actor flags; d_event.h:75-87 for button flags). |
| `crates/doom-sim/src/original_player.rs:398:73: replace \| with ^ in Simulation::preflight_player_body` | The source bit flags are pairwise disjoint, so XOR and OR form the same mask for every input (p_mobj.h:142-189 for actor flags; d_event.h:75-87 for button flags). |
| `crates/doom-sim/src/original_player.rs:398:59: replace \| with ^ in Simulation::preflight_player_body` | The source bit flags are pairwise disjoint, so XOR and OR form the same mask for every input (p_mobj.h:142-189 for actor flags; d_event.h:75-87 for button flags). |
| `crates/doom-sim/src/original_player.rs:398:46: replace \| with ^ in Simulation::preflight_player_body` | The source bit flags are pairwise disjoint, so XOR and OR form the same mask for every input (p_mobj.h:142-189 for actor flags; d_event.h:75-87 for button flags). |
| `crates/doom-sim/src/player_map_scale.rs:944:17: replace && with \|\| in Simulation::spawn_admitted` | Run 1 carry-over: unreachable (reclassified from harmless): every wire-23 spawn position is inside the room rectangle (a THING outside it is refused earlier at spawn, observed `Monster(Arithmetic)`; missiles, puffs, blood and drops spawn at an actor or a trace point inside the lines), so `room.contains` is always true and `&&`/`\|\|` agree on every reachable input. |
| `crates/doom-sim/src/player_commands.rs:112:48: replace \| with ^ in Simulation::preflight_player_commands` | The source bit flags are pairwise disjoint, so XOR and OR form the same mask for every input (p_mobj.h:142-189 for actor flags; d_event.h:75-87 for button flags). |
| `crates/doom-sim/src/player_commands.rs:112:36: replace \| with ^ in Simulation::preflight_player_commands` | The source bit flags are pairwise disjoint, so XOR and OR form the same mask for every input (p_mobj.h:142-189 for actor flags; d_event.h:75-87 for button flags). |
| `crates/doom-sim/src/player_commands.rs:112:27: replace \| with ^ in Simulation::preflight_player_commands` | The source bit flags are pairwise disjoint, so XOR and OR form the same mask for every input (p_mobj.h:142-189 for actor flags; d_event.h:75-87 for button flags). |
| `crates/doom-sim/src/player_commands.rs:114:39: replace \| with ^ in Simulation::preflight_player_commands` | The source bit flags are pairwise disjoint, so XOR and OR form the same mask for every input (p_mobj.h:142-189 for actor flags; d_event.h:75-87 for button flags). |
| `crates/doom-sim/src/player_commands.rs:114:27: replace \| with ^ in Simulation::preflight_player_commands` | The source bit flags are pairwise disjoint, so XOR and OR form the same mask for every input (p_mobj.h:142-189 for actor flags; d_event.h:75-87 for button flags). |
| `crates/doom-sim/src/world_pickups.rs:44:20: replace match guard policy.has_episode1() with true in pickup_bound` | Run 1 carry-over: world.episode1 is Some only under wire 23, so `Some(b) if has_episode1()` and `Some(b)` match the same inputs. |
| `crates/doom-sim/src/world_pickups.rs:55:20: replace match guard policy.has_episode1() with true in bonus_bound` | Run 1 carry-over: as 44:20. |
| `crates/doom-sim/src/world_pickups.rs:67:20: replace match guard policy.has_episode1() with true in world_item_bound` | Run 1 carry-over: as 44:20. |
| `crates/doom-sim/src/spawning.rs:195:17: replace && with \|\| in Simulation::validate_entry` | Run 1 carry-over: unreachable (reclassified from harmless): validate_entry is not reached under wire 23 (monster entry routes to enter_monster_state, request_actor_death refuses the demon lifecycle; under the mutant every wire-23 entry would be checked against the rocket closure and refused, and the whole suite passed), and under the older imps policies no actor holds a rocket state. |
| `crates/doom-sim/src/episode1.rs:85:40: replace \|\| with && in Episode1Bounds::from_things` | Run 1 carry-over: kinds 1–4 and 11 are in none of the counted lists (monsters, pickups, barrel, statics), so skipping them or not gives the same counts. |
| `crates/doom-sim/src/episode1.rs:118:20: replace > with >= in Episode1Bounds::sight_queries` | Run 1 carry-over: at equality (64) both branches return 64. |
| `crates/doom-sim/src/episode1.rs:126:20: replace > with >= in Episode1Bounds::explosion_sight_queries` | Run 1 carry-over: at equality (216) both branches return 216. |
| `crates/doom-sim/src/episode1.rs:135:26: replace > with >= in Episode1Bounds::visit_roster` | Run 1 carry-over: at equality (8) both branches return 8. |
| `crates/doom-sim/src/episode1.rs:235:26: replace += with -= in powers_tick` | Run 1 carry-over: dead branch: validate_powers (line 221, before it) refuses powers[1] ≠ 0, so `powers[1] != 0` is never true here. |
| `crates/doom-sim/src/episode1.rs:235:26: replace += with *= in powers_tick` | Run 1 carry-over: dead branch: validate_powers (line 221, before it) refuses powers[1] ≠ 0, so `powers[1] != 0` is never true here. |
| `crates/doom-sim/src/mover_rules.rs:732:25: replace \|= with &= in reachable` | Run 1 carry-over: close30 is set only under wire 23, where the fixpoint is saturated and line 782 overwrites `changed` with `f != pf \|\| c != pc` after every round: the per-line `changed` updates are discarded. |
| `crates/doom-sim/src/mover_rules.rs:738:25: replace \|= with &= in reachable` | Run 1 carry-over: raise is wire-23 only; `changed` is overwritten at line 782 under saturation, as 732:25. |
| `crates/doom-sim/src/mover_rules.rs:744:25: replace \|= with &= in reachable` | Run 1 carry-over: nearest/change are wire-23 only; `changed` is overwritten at line 782, as 732:25. |
| `crates/doom-sim/src/mover_rules.rs:767:25: replace \|= with &= in reachable` | Run 1 carry-over: stairs exist only under wire 23; `changed` is overwritten at line 782, as 732:25. |
| `crates/doom-sim/src/mover_rules.rs:774:21: replace \|= with &= in reachable` | Run 1 carry-over: donuts exist only under wire 23; `changed` is overwritten at line 782, as 732:25. |
| `crates/doom-sim/src/mover_rules.rs:776:21: replace \|= with &= in reachable` | Run 1 carry-over: as 774:21. |
| `crates/doom-sim/src/movers.rs:378:9: replace Starter<'_, M>::episode1 -> Result<(), SimError> with Ok(())` | Run 1 carry-over: unreachable: an earlier validator already rejects this input (reclassified from harmless): rule 1 admits the Episode 1 starters' specials only under wire 23 (`PursuitError::Specials` at load otherwise), where world.episode1 is always Some. |
| `crates/doom-sim/src/movers.rs:808:31: replace + with * in Starter<'_, M>::build_stairs` | Run 1 carry-over: `secnum = tsec` instead of `tsec + 1`: the next inclusive search can only re-find tsec if it is tagged, and tsec is now busy (its step was just spawned), so it is skipped with secnum = tsec + 1, the original state. |
| `crates/doom-sim/src/store.rs:217:45: replace - with + in reserve` | Owner-approved qualified equivalence (2026-11-08): only the private Vec allocation request changes; the free_slots guard, claimed slots, logical maximum, actor order and canonical state are unchanged, and Vec capacity is not exposed by the public API. This equivalence is for deterministic simulation inputs; allocation failure under host memory pressure can differ. |

#### Harmless (2)

| file:line:col | mutation | observable change | why it does not matter |
|---|---|---|---|
| `movement.rs:423:79` | replace + with * in path | `level_stats::observe_intercepts` records `hits.len()` (×1) instead of `hits.len() + 1` when a slide trace overflows its intercept limit. That changes the maximum `take_level_run_stats` reports for that refused trace. | `level_stats` is an observation side channel (module doc: never read by the simulation). It is not in the snapshot, any digest or any refusal decision: the `PathError::Overflow` refusal is returned either way. No admitted map reaches the overflow. |
| `movement.rs:423:79` | replace + with - in path | the same, recording `hits.len() − 1` | as above |
