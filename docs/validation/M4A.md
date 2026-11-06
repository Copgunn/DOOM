# M4a evidence record

Observed results of the M4a milestone (`PlayerMonstersEpisode1V1`, wire 23, DSIM39): Episode 1,
part 1, proven against the genuine linuxdoom-1.10 engine. Only observations appear here. The
[M4a investigation](../M4A_REFERENCE_INVESTIGATION.md) owns the source facts, the decisions
(§10) and the plan (§12); [format](../STATE_FORMAT.md) owns DSIM39,
[support](../COMPATIBILITY.md#original-episode-1-part-1-every-door-floor-plat-stair-donut-light-and-exit-special-keys-items-powers-chainsaw-rocket-launcher-and-spectre-wire-23)
the admitted behavior and [testing](../TESTING.md#m4a-synthetic-mechanism-wads-and-the-episode-1-comparison)
the commands. Exact review state and CI belong to GitHub.

**Record in progress (Phase 2 steps 1–8 done on 2026-11-05; the fuzz target and its campaigns were recorded against the step-5 crates and are to be re-run on the final crates).** The real-IWAD runs are local
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
(`tools/seed_player_episode1_fuzz.py`) wrote **446 seeds** (max length 2,404) from the 28 scripts
(six variants each, one more for scripts longer than 600 tics), 14 synthetic patterns on every WAD with
two RNG settings, and header-only and odd-length inputs; it reads no Rust output.

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
