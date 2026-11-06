# M4a evidence record

Observed results of the M4a milestone (`PlayerMonstersEpisode1V1`, wire 23, DSIM39): Episode 1,
part 1, proven against the genuine linuxdoom-1.10 engine. Only observations appear here. The
[M4a investigation](../M4A_REFERENCE_INVESTIGATION.md) owns the source facts, the decisions
(§10) and the plan (§12); [format](../STATE_FORMAT.md) owns DSIM39,
[support](../COMPATIBILITY.md#original-episode-1-part-1-every-door-floor-plat-stair-donut-light-and-exit-special-keys-items-powers-chainsaw-rocket-launcher-and-spectre-wire-23)
the admitted behavior and [testing](../TESTING.md#m4a-synthetic-mechanism-wads-and-the-episode-1-comparison)
the commands. Exact review state and CI belong to GitHub.

**Record in progress (Phase 2 steps 1–6 done on 2026-11-05; the fuzz target and its campaigns were recorded against the step-5 crates).** The real-IWAD runs are local
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

## Synthetic mechanism WADs (CI evidence)

Nine generated WADs (`tools/reference/m4a_wad.py`, slot E1M1 each, 45–64 KB) and 28 scripts
(13,934 tics) under `tests/reference/m4a_scripts/`; the manifest `tests/reference/m4a_manifest.json`
records each script's end and its observed event tics. Every script gave byte-identical dumps in the
strict, overflow-checked and address-checked builds and in `strict32`; no sanitizer build aborted;
no new sanitizer adapter was needed. The self-test (`--selftest 300` on DOORS1) reports the planted
difference at tic 300. Observed engine facts and the H7 refusal tics (PLATS2 blocked plat at tic 91,
STAIRS2 blocked step at tic 153) are listed in the note's §14.

## Independent projection

`level_snapshot.py --profile episode1` projects the dumps to DSIM39 exactly from the observer rows;
its load-capture field paths equal the Rust writer's 3,841 paths in order (the Rust list was compared
after the fact; the encoder was written from the dump columns and the schema). Goldens:
`tests/reference/m4a_synthetic_goldens.json` (28 scripts, 13,962 snapshots) and 18 fixtures
`m4a_episode1_<SCRIPT>_<tic>.bin`, produced from C dumps only.

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
--example-args "--policy episode1"` on the 28 synthetic scripts, as reported at the end of each step.
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

No blocked stair step or blocked raise-and-change plat occurs in any of these runs. The Rust side of
these runs is recorded below when step 8 lands.
