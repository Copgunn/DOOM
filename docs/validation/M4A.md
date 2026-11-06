# M4a evidence record

Observed results of the M4a milestone (`PlayerMonstersEpisode1V1`, wire 23, DSIM39): Episode 1,
part 1, proven against the genuine linuxdoom-1.10 engine. Only observations appear here. The
[M4a investigation](../M4A_REFERENCE_INVESTIGATION.md) owns the source facts, the decisions
(§10) and the plan (§12); [format](../STATE_FORMAT.md) owns DSIM39,
[support](../COMPATIBILITY.md#original-episode-1-part-1-every-door-floor-plat-stair-donut-light-and-exit-special-keys-items-powers-chainsaw-rocket-launcher-and-spectre-wire-23)
the admitted behavior and [testing](../TESTING.md#m4a-synthetic-mechanism-wads-and-the-episode-1-comparison)
the commands. Exact review state and CI belong to GitHub.

**Record in progress (Phase 2 steps 1–5 done on 2026-11-05).** The real-IWAD runs are local
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

## Rust replay against the goldens (so far)

After step 5 (keys, items, powers, sector special 11): 20 of the 28 scripts load and match tic for
tic (FLOORS1 933, FLOORS2 285, FLOORS3 181, PLATS1 574, STAIRS1 639, DONUT1 333, DOORS1 922,
DOORS2 559, DOORS3 1,336, EXIT1 154, EXIT2 57, SECT11 182, EXDIE 129, ITEMS1 236, TIMERS 2,253,
SUIT1 303, SUITDIE 183, INVIS1 291 snapshots; PLATS2 matches captures 0–91 and refuses 92 with
`PlatBlocked`; STAIRS2 matches 0–153 and refuses 154 with `StairBlocked`). The weapon, Spectre and
scale scripts need step 6. No divergence was traced to the encoder or the goldens.

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
