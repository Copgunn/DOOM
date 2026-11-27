# Wire 28 evidence record (the easy skills and E1M7 on skill 3)

Observed results of wire 28 (`PlayerMonstersEpisode1V6`, captures `DSIM` 44, session state `DSES` 3), recorded 2026-11-25
(America/Los_Angeles; sections 5.3 to 5.5, 9, 10 and 11 added 2026-11-26; recording file-name stamps are UTC) on branch `feat/wire28-easy-skills`, [PR #86](https://github.com/Copgunn/DOOM/pull/86). The rule and what
is admitted are in [COMPATIBILITY](../COMPATIBILITY.md#wire-28-the-easy-skills-and-e1m7-on-skill-3), the format in
[STATE_FORMAT](../STATE_FORMAT.md#dsim-v44--wire-28-the-easy-skills-and-e1m7-on-skill-3), the C facts in the
[M6a note §8](../M6A_REFERENCE_INVESTIGATION.md#8-wire-28-the-easy-skills-and-e1m7-on-skill-3-source-investigation-2026-11-25), the
commands in [TESTING](../TESTING.md#wire-28-the-easy-skills-and-e1m7-on-skill-3) and the quirk rows in the [register](../QUIRKS.md)
(Q-p_inter-799, Q-p_inter-838, Q-p_inter-894, Q-p_mobj-152, Q-p_mobj-777, Q-p_map-189, Q-g_game-828, Q-p_enemy-1909,
Q-p_pspr-255). Exact review commits and CI belong to GitHub. Only counts, pass/fail facts and map names from the commercial
IWAD appear here; nothing is said of any map's things or geometry.

**Tic convention (as in [WIRE27](WIRE27.md)).** A session tic is the 0-based index of a tic; a refusal at session tic N keeps
the state after N tics. **Everything labelled Rust-only is Rust-only:** no genuine engine ran it.

## 1. What wire 28 is, in one paragraph

Wire 27's simulation plus: skills 0 (`sk_baby`) and 1 (`sk_easy`) beside 2 and 3 (THING bit 1; at skill 0 every `P_GiveAmmo`
amount doubled before the cap and the player's damage halved before everything else); the Demon bite taking the sector-11
cap; spawn rule S'' (S' without its margin clause for monsters) with the **monster escape witness**, which refuses the tic when
an accepted move of a non-player, non-missile mover leaves the map's boundary. `doom-play` plays it for every skill. Nightmare
is refused at launch.

## 2. Decisions

The owner was not watching; each decision was taken by the orchestrating session on 2026-11-25 and is **changeable before
merge**. Options not taken are named.

| | Decision | Why |
| --- | --- | --- |
| D1 | DSIM44 is DSIM43 field for field; only the version (44), the profile (28) and `context.schema` (`PLAYER-MONSTERS-EPISODE1-6-v1`) differ, at byte offsets 4, 82 and 181. Not taken: new fields. | Nothing in the state is new: the skill is already in `actors.options`, the baby flag follows from it, and the witness keeps no state. |
| D2 | `DSES` 3 is version 2's layout with an embedded DSIM44 (`Diagnostic::decode_episode1_v6`); versions 1 and 2 are frozen. No new `DDSP` or `DSND` version: version 3 embeds `DDSP` 4 and `DSND` 3 unchanged. Not taken: redefine version 2. | Frozen formats are not redefined; wire 28 changes nothing the display or sound hold. |
| D3 | A third session configuration, `Configuration::FinaleAndMeltWire28`; `doom-play` uses it for every skill (`GameData::load_wire28_session`). Not taken: wire 28 only for skills 0 and 1 and E1M7 on skill 3, wire 27 elsewhere. | One configuration for the player; wire 28 equals wire 27 on skills 2 and 3 in the tests of section 6. |
| D4 | The Demon and Spectre bite on the player takes the sector-11 cap under wire 28 (gated on `has_easy_skills`). Under wires 23 to 27 it skipped it (p_inter.c:836-842), a silent divergence that stays frozen. Evidence: BITE2 equals the genuine engine under wire 28; wire 27 first differs at capture 97; wires 23 to 26 by the independent reader's Rust-only scratch probe (section 11), not run against the genuine engine. Not taken: leave wire 28 diverging; refuse the tic. | A divergence the item showed should not be carried into the new wire; refusing would stop a legal tic. |
| D5 | The stop message for `MapScale(Escape)` is a plain sentence; it also replaces the debug text for a barrel's escape under older configurations (text only, the old text kept as its prefix; no test pins it). That touches the frozen app's text although `app/mod.rs`'s module doc says `load_session` keeps the frozen app exactly; the doc comment is not edited here. | A player-facing stop needs a sentence. |
| D6 | **Superseded.** The first note said to time the witness once the owner said the machine was idle. The owner's ruling of 2026-11-25 replaces it: the timing is taken only with nothing else running and said so, otherwise listed as not run. | Owner's ruling. |

Owner's rulings of 2026-11-25 for this pull request that bear on the record: existing-test changes only for expectations that
pinned skills 0 and 1 or E1M7 on skill 3 as refused, each named (section 8); the proof of wire 28 stays in this pull request.

## 3. The twelve synthetic genuine items (CI)

One WAD, [wire28.wad](../../tests/reference/wire28_wads) (nine map slots built with the M6a room builder), twelve demo scripts
under [wire28_scripts](../../tests/reference/wire28_scripts), digests in [wire28_state.txt](../../tests/reference/wire28_state.txt)
(DSIM44, through the independent encoder `tools/reference/level_snapshot_wire28.py`, never Rust output) and data in
[wire28_results.json](../../tests/reference/wire28_results.json). **Three genuine builds** (strict, overflow, address; the
address build runs with `detect_leaks=0`: leak detection disabled, not LeakSanitizer coverage) gave identical streams. Wire 28
equals the strict build at every capture of every item: **2,985 captures** (ESCAPE counts the 78 before the refusal).

| Item | Skill | Captures | Ends | What it shows |
| --- | ---: | ---: | --- | --- |
| AMMO0 / AMMO2 | 0 / 2 | 311 / 311 | complete | every `P_GiveAmmo` path of Episode 1; doubled at skill 0 only |
| DROP0 / DROP2 | 0 / 2 | 411 / 411 | complete | a dropped clip (half a clip, then doubled) and a dropped shotgun (one load, doubled) |
| HURT0 / HURT2 | 0 / 2 | 301 / 301 | complete | fireball, claw, hitscan, bite and floor damage halved before thrust and armour |
| SPLASH0 / SPLASH2 | 0 / 2 | 145 / 145 | complete | a barrel blast of 1 halved to 0 (pain roll kept, session tic 57) and a blast of 54 halved |
| BITE2 | 2 | 99 | level exits after session tic 97 | a lethal bite on a sector of special 11 capped to health 1 (tics 96 and 97) |
| BITE0 | 0 | 194 | level exits after tic 192 | the same room, every blow halved before the cap |
| ESCAPE | 2 | 171 genuine, 78 compared | Rust refuses session tic 77, `MapScale(Escape)` | two barrels (49, then a lethal 128) on an imp embedded at distance 1: momentum 22.125, one step to x = -21.125 through the wall at x = 0; the corpse slides to about x = -234 and the game goes on |
| SMALL | 2 | 278 | complete | three embedded imps (a boundary value, well inside, a diagonal) stand still while chasing; pistol and shotgun hits move none |

With the witness removed Rust equals the genuine engine at all 171 ESCAPE captures, so the refusal is the only difference. Wire 27
refuses the embedded-imp rooms (`Setup(Monster(Arithmetic))`) and wires 27 and 28 both refuse an embedded barrel
(`Setup(Monster(Arithmetic))`) and an embedded player start (`Setup(PlayerBody(Binding))`) at load.
Wire 27 against BITE2 first differs at capture 97 (the bite kills the player under wire 27; the genuine engine exits).

**Planted faults, each detected** (the port, then altered genuine builds with the p_inter.c rule removed or moved in memory):
`baby = false` in the port is caught at capture 10 (AMMO0), 321 (DROP0), 1 (HURT0), 58 (SPLASH0) and 1 (BITE0); no doubling
AMMO0 at 10 and DROP0 at 321; no halving HURT0 at 1, SPLASH0 at 58 and BITE0 at 1; halving after the thrust HURT0 at 62 and BITE0
at 97; halving after the cap BITE0 at 193; no bite cap is wire 27 on BITE2 at 97; the margin kept is wire 27's load refusal.
**Self-tests:** an extra `P_Random` draw before tic 40 of HURT0 is caught at capture 41; a SKILL1 run against SKILL0 goldens fails
(section 4). The results file was generated with the local `cc`; the CI compiler differs.

## 4. SKILL0 and SKILL1 against every genuine stream

The M6a session items SKILL0 and SKILL1 (the same scripts at skills 0 and 1) run through the wire 28 session
(`skills_0_and_1_against_every_genuine_stream`) against the genuine runner's streams at screenblocks 10 and 11:
364 tics, 685 sound rows, 210 mixer rows, 3 melts (119 steps) and the DSIM44 state over 2 levels (348 captures, screenblocks 10),
all equal. Self-test: SKILL1's run against SKILL0's goldens fails at tic 32 (health 95 against 98).

FIN0 and FIN1 (2026-11-25; `tools/reference/wire28_fin.py`, goldens under `tests/reference/wire28_fin/`) are FINRET's demo at skills 0 and 1, FIN0 with 128 empty tics inserted because the halved floor damage exits 128 tics later. The genuine victory falls on session tic 449 (FIN0, 2,079 tics) and 321 (FIN1, 1,951 tics), followed by 1,630 finale tics through the stage-1 melt. The same test (`victory_on_skills_0_and_1_against_every_genuine_stream`) compares every stream above plus the F row at screenblocks 10 and 11, with 3 melts per run and the DSIM44 state over E1M8 (449 and 321 captures), and all are equal. Self-test: FIN1 against FIN0's goldens fails at tic 192 (health 80 against 90).

![The same recorded commands on a synthetic map at skill 0 and skill 1, with their per-pixel difference.](../media/wire28-skills-side-by-side.gif)

*SKILL0 and SKILL1 side by side, every 4th tic, Rust frames only; regenerate with `tools/media/wire28_readme_visual.py`.* The runs part at tic 32.
Every level frame of both runs (348 each, screenblocks 10) is the one `skills_0_and_1_against_every_genuine_stream` compares with the genuine
engine's frame digest at every tic, and the script re-checks that digest; the picture itself is Rust-drawn and synthetic.

## 5. Real maps: E1M7 on skill 3 and the owner's five sessions

Local only (`tools/local/wire28_real.py`, never in CI: it reads the IWAD in place through a symlink). Every demo runs through the
**whole session path** (`Session::new_finale_and_melt_wire28`, the session `doom-play` runs), at screenblocks 10 and 11, against
the genuine strict build (`-fwrapv`), comparing on every tic the session rows, every frame, every sound row and every melt step,
and afterwards the mixer buffers and the whole DSIM44 state after every level tic. Self-test on one run: a flipped ticcmd at tic 100
and an extra genuine `P_Random` draw before tic 200 were each reported at their tic, at both sizes.

### 5.1 E1M7 on skill 3

When these were built no recorded E1M7 run on skill 3 existed, so they are **our constructions** (the owner's hand test of 2026-11-26 later supplied one, section 5.4): the five E1M7 runs of the 61 (recorded on skill 2) with
the header's skill byte set to 3, plus 35 idle tics from the load. All six pass at both sizes: **16,459 session tics**, 0 refusals,
the whole DSIM44 state equal at every one (21 levels).

| Run | Tics |
| --- | ---: |
| M7CHAOS (4 loads) | 5,000 |
| M7DIE | 367 |
| M7FIGHT | 1,221 |
| M7IDLE (all idle commands) | 2,100 |
| M7WALK (1 load) | 7,736 |
| 35 idle tics from the load, M7IDLE's header | 35 |

**The escape witness was reached 0 times** on E1M7 at skill 3: these runs show that wire 28 plays the map equal to the genuine engine,
not that the witness is right there (that rests on ESCAPE, section 3). The imp's distance to the wall is **not recorded**.

### 5.2 The owner's five sessions of 2026-11-25

Recorded by `doom-play` of master `9da7505f` (wire 27's configuration) and replayed under **both** wire 28 (all five pass at both
sizes, every stream) and wire 27 (the unchanged ignored test `real_maps_finale_and_melt_local`: 10 runs, 34,600 session tics, 17,300
at each size). Recordings are named by their file time.

| Recording | Tics | Where it ends | Reborn loads | Melts (steps) | Result |
| --- | ---: | --- | ---: | --- | --- |
| 201221 (E1M1) | 767 | the recording's end, mid-level | 0 | 1 (42) | pass |
| 201252 (E1M1) | 623 | the recording's end, mid-level | 0 | 1 (42) | pass |
| 201426 (E1M8) | 6,503 | victory at session tic 4671, then 1,832 finale tics | 1 | 4 (155) | pass |
| 202907 (E1M8) | 5,467 | victory at 3722, then 1,745 finale tics | 1 | 4 (158) | pass |
| 203216 (E1M8) | 3,940 | victory at 3171, then 769 finale tics | 0 | 2 (85) | pass |

Level-state captures: 767, 623, 4,671, 3,722 and 3,171. No tic was refused and none differed. **Limit of the gate:** on a difference
or refusal it stops at the first one, and the whole-state and mixer comparisons come after the run, so a refusal case would need a
separate state-only check.

### 5.3 Real maps on skills 0 and 1

Local only, 2026-11-25 and 2026-11-26 (`wire28_real.py --set skill0` and `--set skill1`), the same whole-session gate as above. **These are the
same 61 committed command recordings (the E1M1 to E1M9 scripts and `DEMO1`, made on skills 2 and 3), plus two constructions of ours (M8WALK_FIN and
M8WALK3_FIN: a recording followed by 2,400 idle tics): 63 demos, replayed with the header's skill byte set to 0 and then to 1.** On another skill
different things spawn and the game plays out differently, so these are different simulation states from the same commands. They are not new
recordings and no one played them; they show that the port and the genuine engine agree on these inputs.

| Map | Demos | Session tics | Skill 0: pass, refused | Skill 1: pass, refused |
| --- | ---: | ---: | --- | --- |
| E1M1 | 5 | 10,029 | 5, 0 | 5, 0 |
| E1M2 | 5 | 12,544 | 5, 0 | 5, 0 |
| E1M3 | 5 | 15,388 | 5, 0 | 5, 0 |
| E1M4 | 5 | 12,717 | 5, 0 | 5, 0 |
| E1M5 | 11 | 47,146 | 11, 0 | 11, 0 |
| E1M6 | 5 | 16,724 | 5, 0 | 5, 0 |
| E1M7 | 5 | 16,424 | 5, 0 | 5, 0 |
| E1M8 | 12 | 52,095 | 12, 0 | 12, 0 |
| E1M9 | 10 | 29,884 | 10, 0 | 10, 0 |
| **Total** | **63** | **212,951** | **63, 0** | **63, 0** |

**63 of 63 pass at screenblocks 10 and 11 on each skill (126 runs each), 212,951 session tics per skill, 0 refusals, every stream compared**
(session rows, frames, sound rows, melt steps; at screenblocks 10 also the mixer buffers and the whole DSIM44 state after every level tic). The per-map
counts are the same on both skills because the commands are. **Distinct tics per skill:** 212,951 less 5,813 (M8WALK, repeated inside M8WALK_FIN) and 5,505
(M8WALK3, inside M8WALK3_FIN) is **201,633**, of which 4,800 are our appended idle tics.

**One failure and its resume.** The first run of skill 0 stopped with exit 1 on the genuine side (`OSError: [Errno 12] Cannot allocate memory`, reading the
WAD, at demo 55 of 63) because too many jobs ran at once on the machine; the tool skips demos it has finished, the run was resumed and compared all 63.
**One helper difference.** The first comparison pass of each skill used a copy of the test helper (`m6a2_run/mod.rs`) in which `read_sound` was `pub`, a one-word
change later reverted. Confirming reruns of both skills on the restored helper (tree `fff2a40d`, 2026-11-26 about 00:55): 63 of 63 pass at both sizes, 212,951 tics,
0 refusals, exit 0 on each skill, and every one of the 126 result lines of each rerun equals the first pass's (a diff of the result lines is empty).

### 5.4 E1M8 on the easy skills

The owner's ruling V5 (2026-11-25): valid inputs even where they stop reaching their exits; a recording made on another skill that does not reach the victory on
skill 0 or 1 is expected, and an equal run that never reaches it still counts as compared tics. **No route was constructed, and the table below is derived from no
recording of the owner's.** What each of the twelve E1M8 demos of section 5.3 does on each easy skill:

| Demo | Tics compared | Equal | Ends (genuine and Rust alike) |
| --- | ---: | --- | --- |
| M8CHAOS, M8CHAOS3 | 5,000 each | both skills | the demo's end marker |
| M8DIE / M8DIE3 | 553 / 344 | both skills | the demo's end marker |
| M8FIGHT / M8FIGHT3 | 5,740 / 3,822 | both skills | the demo's end marker |
| M8IDLE, M8IDLE3 | 2,100 each | both skills | the demo's end marker |
| M8WALK / M8WALK3 | 5,813 / 5,505 | both skills | the demo's end marker |
| M8WALK_FIN / M8WALK3_FIN (our constructions: M8WALK and M8WALK3 plus 2,400 idle tics each) | 8,213 / 7,905 | both skills | the demo's end marker |

All twelve are equal on skill 0 and on skill 1 (52,095 tics each), **none reaches the victory**, and the tic counts are the same on both skills. The victory
is reached by the synthetic items FIN0 and FIN1 instead ([section 4](#4-skill0-and-skill1-against-every-genuine-stream); not repeated here), compared with the
genuine engine in CI: their cost was 168 added lines (149 as first built; `git diff --numstat d3d142bc cbe88eba -- . ':!tests/reference'`, recounted from the tree).

**E1M8 to the finale on skills 0 and 1, real map: covered** by the owner's hand test of 2026-11-26 (America/Los_Angeles) on the Windows build of `cbe88eba`, verified
afterwards against the genuine engine through the full session path. The owner's report: passed; E1M7 loaded on skill 3 with no refusal; on skill 0 ammunition
pickups were doubled and damage halved, on skill 1 both were normal; both easy runs reached the finale. The three recordings (read-only on the owner's machine, never
copied into the repository; file-name stamps are UTC) were compared on 2026-11-26 about 01:00 with `tools/local/wire28_real.py --set sessions` (wire 28 session,
genuine strict build, every stream at screenblocks 10 and 11, mixer buffers, the whole DSIM44 state at screenblocks 10 after every level tic), exit 0:

| Recording (UTC stamp) | Tics compared | Where it ends | Loads after the first | Melts (steps) | Masked frames sb10 / sb11 | Result |
| --- | ---: | --- | ---: | --- | --- | --- |
| 082510, E1M7 skill 3 | 18,263 of 18,263 | the recording's end: level 17,778, intermission 485, no victory; played on into E1M8 | 2 | 4 (161) | 39 / 48 | pass |
| 083445, E1M8 skill 0 | 3,929 of 3,929 | victory at session tic 3521, then 408 finale tics | 0 | 2 (77) | 25 / 29 | pass |
| 083716, E1M8 skill 1 | 3,643 of 3,643 | victory at session tic 3516, then 127 finale tics | 0 | 2 (77) | 26 / 29 | pass |

**Total 25,835 session tics, 3 of 3 pass, 0 refusals, 0 differences**; state captures 17,778 (3 levels), 3,521 and 3,516. The genuine victory tics equal the owner's reported
3521 and 3516. The escape witness was not reached in the E1M7 run. No existing construction reaches the victory on either easy skill, and none was forced (the owner's
ruling V5); the hand test is what supplies it. These tics are counted in figure 1 of [M6A §14.2](M6A.md#142-the-readmes-evidence-figures-summed-from-the-records).

**What the hand test stands for.** It ran on `cbe88eba`. Since then the only change under a crate's `src` outside test files is the line `#[cfg(test)] mod wire28_mutant_tests;`
in `crates/doom-sim/src/lib.rs` plus the new test-only file it names (`fff2a40d`). Checked on 2026-11-26 with `git diff --stat cbe88eba HEAD -- 'crates/*/src/*'`, which at `58049822` prints exactly two files, `crates/doom-sim/src/lib.rs` (2 added lines) and
`crates/doom-sim/src/wire28_mutant_tests.rs` (60 added lines), 62 insertions (the pathspec `'crates/*/src'` without the final `/*` matches nothing). Under `crates/` outside `src` the same range also adds the test file
`doom-game/tests/wire28_mutants.rs` and the example `doom-sim/examples/wire28_witness_timing.rs`. A later change under a crate's `src` would void this statement.
**It is voided for two places (2026-11-26, [section 12](#12-the-owners-rulings-of-2026-11-26)):** `7aa0d05d` and `95b61473` change engine lines under `src` after the hand test, which therefore ran without them. Read, not run: neither is on a path `doom-play` takes (the only non-test callers of `spawn_type` spawn the player body, and `doom-play` neither saves nor loads a `DSES` state).

### 5.5 The 61 unchanged runs under wire 28, and wire 27's gate

Local only, 2026-11-26: the 61 committed recordings (made on skill 2, the E1M1 to E1M9 scripts and `DEMO1`), unchanged, through the wire 28 session (`wire28_real.py --set own
--wire27`). **61 of 61 pass at screenblocks 10 and 11, 196,833 session tics, 0 refusals.** Per map (runs, tics): E1M1 5, 10,029; E1M2 5, 12,544; E1M3 5, 15,388; E1M4 5, 12,717;
E1M5 11, 47,146; E1M6 5, 16,724; E1M7 5, 16,424; E1M8 10, 35,977; E1M9 10, 29,884. The unchanged wire 27 test `real_maps_finale_and_melt_local` on the same demos: exit 0, 122
runs, 393,666 session tics over both sizes. These add nothing to the evidence sums (the 61 are already counted in figure 1 of M6A §14.2).

## 6. `DSES` version 3: restore and continue

Rust-only (a session compared with itself, not with the genuine engine). Test
`dses_version_3_restore_and_continue_at_every_kind_of_point` (`crates/doom-game/tests/wire28_session.rs`): a wire 28 session is saved
(`save_v3`), encoded, decoded, restored (`restore_v3`), re-saved (bytes equal), then continued beside the uninterrupted session to the
demo's end; every call (tic or melt step) and the final saved bytes are equal. Runs: SKILL0 and SKILL1 at their own skills, and INTFAST,
CHAIN and FINRET each replayed at skill 0 and at skill 1 (their demos were recorded on skill 2, so only restored-equals-uninterrupted
is asserted there, not agreement with a genuine golden). **68 save points**: level 8, last level tic 20, intermission 12, finale 4,
melt 24. The file's four tests pass in debug and in release. **Limit:** melt save points are the first, middle and last of all melt boundaries, not
labelled per melt kind, so the finale's text-to-art melt is not specifically asserted. The version-3 round trip, the refusals of
versions 1 and 2 and the older configurations' refusal of skills 0 and 1 are the other tests of that file (section 8).
Equality with wire 27 on skills 2 and 3: seven demos (SKILL2, SKILL3, INTFAST, REBHELD, SECRET, NOVICTOR, FINRET), 3,956 tics, 134 saved
states equal under the projection of the three differing DSIM values.

## 7. Fuzzing, by type

Three targets, Rust-only safety, cargo-fuzz 0.13.2 on nightly, options `-max_total_time=240 -seed=1 -timeout=20 -rss_limit_mb=2048`
and `-max_len` 131072 (`wire28_dsim44`), 300000 (`wire28_dses3`), 4096 (`wire28_demo3`).

- `wire28_dsim44`: `Diagnostic::decode_episode1_v6`; an accepted blob has version 44 and the DSIM43 reader refuses it.
- `wire28_dses3`: `SessionStateV3::decode`; an accepted state passes its check, re-encodes to itself, and the version 2 reader refuses it.
- `wire28_demo3`: both demo version-3 readers (`parse_demo_episode1_v3` with the map from byte 0; `SessionDemo::parse_v3`); an accepted
  demo the writer will write re-parses equal; the version 2 reader agrees on skills 2 and 3 and refuses 0 and 1.

| Target | Seed replay (address sanitizer) | Coverage-guided, **no sanitizer** | Address-sanitizer campaign, **leak detection on** (default options) |
| --- | --- | --- | --- |
| `wire28_dsim44` | 6 seeds, exit 0 | 2,553,835 runs, 241 s, exit 0 | 866,934 runs, 241 s, exit 0 |
| `wire28_dses3` | 5 seeds, exit 0 | 867,561 runs, 241 s, exit 0 | 187,630 runs, 241 s, exit 0 |
| `wire28_demo3` | 6 seeds, exit 0 | 23,045,199 runs, 241 s, exit 0 | 4,675,609 runs, 241 s, exit 0 |

No crash, no leak. **Leak detection was shown to work**: a scratch copy of `wire28_dsim44` with a planted leak
(`forget(black_box(data.to_vec()))` on an accepted input) reported "LeakSanitizer: detected memory leaks" (15,712 bytes in one
allocation, exit 77); the same binary with `detect_leaks=0` exited 0; the unmodified target leaks nothing. A first plant
(`forget(Box::new([7u8; 4096]))`) was optimised away and showed nothing, which is why it is not the one reported.
**Seeds:** made by a scratch helper that is **not committed** (6 DSIM44 captures of SKILL0 and SKILL1 at tics 0, 35 and 70; 5 `DSES`
3 states from INTFAST and from SKILL0 and SKILL1 in a level and in a melt; 6 demos, SKILL0, SKILL1 and SKILL3 bare and with a map
prefix), so the runs above are not reproducible from the repository's seeds alone (one `DSES` 3 seed was committed later, [section 12](#12-the-owners-rulings-of-2026-11-26)). **Observed while fuzzing (not a library fault):**
the demo version-3 parsers accept a command whose button byte has its top two bits set; the writers refuse to write it and the session
refuses that command later. The four new readers are covered; the demo version-2 readers were never fuzzed (no model existed).

## 8. Tests: added, and existing ones touched

**Added** (counts are the tests below): `crates/doom-sim/src/wire28_tests.rs` (21, Rust-only: identity and skills, THING bit, demo
header, the DSIM44 differences at offsets 4, 82 and 181, the doubling and its cap, the halving and its order, S'' and the margin kept
for barrels and the player, the witness on both sides of the wall, refusal with the state kept, every non-player move carrying the
witness, the bite and its cap); `wire28_genuine_tests.rs` (3: `wire_28_equals_the_genuine_engine_on_every_item`,
`wire_27_diverges_on_the_bite_and_refuses_the_embedded_rooms`, `planted_faults_in_the_port_are_caught`);
`crates/doom-game/tests/wire28_genuine.rs` (2: `skills_0_and_1_against_every_genuine_stream`, section 4, and `victory_on_skills_0_and_1_against_every_genuine_stream`, FIN0 and FIN1, section 4); `crates/doom-game/tests/wire28_session.rs` (4: the
session starts on skills 0 to 3 and the older ones refuse 0 and 1; wire 28 equals wire 27 on skills 2 and 3; `DSES` 3 round trip,
restore and refusals; section 6); `crates/doom-play/src/wire28_tests.rs` (3: skills 0 to 3 accepted and nightmare refused; the help;
a skill 0 recording round-trips its header and replays); `crates/doom-game/tests/wire28_mutants.rs` (3, for three doom-game mutation survivors, section 9); `crates/doom-sim/src/wire28_mutant_tests.rs` (1, `dsim44_counts_a_skill_2_capture_with_thing_bit_2`, test-only, wired in by one `#[cfg(test)] mod` line in `lib.rs`). Two more on 2026-11-26 ([section 12](#12-the-owners-rulings-of-2026-11-26)): `a_direct_monster_spawn_follows_s_double_prime` in `wire28_mutant_tests.rs` and `dses_version_3_refuses_a_skill_byte_its_level_does_not_have` in `wire28_mutants.rs`. Fuzz targets: section 7.

**Existing tests touched** (the two whose expectations pinned the easy skills as refused; the owner's ruling of 2026-11-25), both
in `crates/doom-play/src/main.rs`, each with a dated comment: `tests::arguments` (`"--iwad a --skill 1"` left the list of refused
arguments; `4` and `nightmare` stay) and `tests::the_game_mode_flag_and_the_session_in_the_help` (`--skill 0` and `--skill 1`
now parse and give skills 0 and 1 where the test expected "easy skills (0, 1) are not built yet"). **No other existing test, fixture
or golden is changed**: everything under `tests/reference` that this branch touches is a new file, and no frozen format's
documentation was edited beyond cross-links.

## 9. Mutation testing

cargo-mutants 27.1.0 with cargo-nextest, `-j 1`, a scratch worktree on the Linux disk, 2026-11-25 and 2026-11-26. The mutants are the lines this pull request changed in
`crates/` (`--in-diff` of `git diff origin/master...cd1f7b05 -- crates/`; the crate sources did not change after `cd1f7b05` except for test files and one
`#[cfg(test)]` line, so the survivors are the same at `fff2a40d`). The command forms are in [TESTING](../TESTING.md#wire-28-the-easy-skills-and-e1m7-on-skill-3).
Exit code 2 is cargo-mutants' "a mutant was missed"; the runs that exited 2 are marked.

| Mutants run against | Head | Mutants | Caught | Missed | Timeout | Unviable | Exit |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `doom-sim`, its own tests | diff at `cd1f7b05` | 178 | 150 | 11 | 0 | 17 | 2 |
| `doom-sim`'s 11 survivors, with `doom-sim`'s and `doom-game`'s tests | `cbe88eba` | 11 | 4 | 7 | 0 | 0 | 2 |
| `doom-game`, its own tests | diff at `cd1f7b05` | 75 | 49 | 4 | 0 | 22 | 2 |
| `doom-game`'s 4 survivors, `doom-game`'s tests (now with `wire28_mutants.rs`) | `4f78d30a` | 4 | 3 | 1 | 0 | 0 | 2 |
| `doom-play`, its own tests (local) | diff at `cd1f7b05` | 3 | 2 | 0 | 0 | 1 | 0 |
| `doom-render`, `doom-sound` | | 0 | | | | | this pull request changes no line in either |

**Every survivor and its answer** (15: the 11 of `doom-sim` and the 4 of `doom-game`):

| Survivor | Answer |
| --- | --- |
| `doom-sim` `diagnostic.rs:580:25`, `581:25` (match guard to `false`), `580:40`, `581:40` (`==` to `!=`) in `v40_skill` | **killed once `doom-game`'s tests run** (4 caught in the second row; the run does not name the test); no new test |
| `diagnostic.rs:580:35`, `581:35` (`&&` to `\|\|`) in `v40_skill` | **killed** by `wire28_mutant_tests::dsim44_counts_a_skill_2_capture_with_thing_bit_2` (`fff2a40d`: a DSIM44 capture at skill 2 on a map whose bit-1 and bit-2 thing counts differ). Observed by applying each mutation by hand and seeing the test fail; the expected behaviour is read from the C (p_mobj.c:743-751, 789-790; info.c:1416), not observed against the genuine engine. cargo-mutants was rerun over these two on 2026-11-26: both caught ([section 12](#12-the-owners-rulings-of-2026-11-26)) |
| `demon.rs:154:35` (`&&` to `\|\|`), `154:31` (`>` to `>=`), `154:52` (`&` to `\|`, and `&` to `^`) in `strike_player` | **argued equivalent, no test.** The line is `live = health > 0 && flags & MF_SHOOTABLE != 0`. Only two body states reach it: (A) health 1 or more with `MF_SHOOTABLE` set, (B) health 0 or less with it clear (a kill clears the flag). In A and B each of the four mutants gives the original value (the `^` one because a live player body also carries `MF_SOLID`, an invariant checked at `original_player.rs:404`, so the XOR stays nonzero). The independent reader (section 11) agreed |
| `doom-game` `recorder.rs:83:13` (field `v3` deleted) | **killed** by `a_wire_28_recording_writes_the_easy_skills` (`wire28_mutants.rs`, `4f78d30a`); hand-applied mutation, and caught in the cargo-mutants rerun of the fourth row |
| `app/mod.rs:541:50` (`==` to `!=`) | **killed** by `the_wire_28_app_records_the_easy_skills`; the same two observations |
| `session/state_v3.rs:45:9` (`check` to `Ok(())`) | **killed** by `check_refuses_a_state_out_of_its_domain`; the same two observations |
| `app/mod.rs:541:35` (`&&` to `\|\|`) in `GameData::recorder` | **argued equivalent, no test** (missed in the rerun). Older configurations refuse skills 0 and 1 at load, and on skills 2 and 3 the version 2 and version 3 demo writers give equal bytes, so which writer the `\|\|` picks cannot be seen in any recording. The independent reader agreed |
| `doom-sim` `spawning.rs:201:38` (`&& self.margined(ty)` to `\|\|`) in `spawn_type_checks` | **Closed on 2026-11-26 by the owner's ruling 1: `preflight_monster_spawn` follows S'' and the mutant is caught ([section 12](#12-the-owners-rulings-of-2026-11-26)).** As found before the ruling: the margin clause this pull request added on that line does not change what a direct `spawn_type` admits. For a policy monster under wire 28, `preflight_monster_spawn` (`monsters.rs:644`, not changed here) still applies the margin, so the spawn is refused either way and only the error differs (`Monster(Arithmetic)` against `Perception(Arithmetic)`). Not a defect against the C (`P_SpawnMobj` has no position check, and the C has no direct-spawn call; map THINGs under S'' take a different path, covered by `s_double_prime_admits_an_embedded_monster_under_wire_28_alone`). **Owner decision needed:** either `preflight_monster_spawn` follows S'' too (a behaviour change) or line 201's change is dead and is reverted; left as is, no engine change in this pull request |

**Not rerun:** the 150 `doom-sim` mutants caught by `doom-sim`'s own tests were not
rerun with `doom-game`'s tests. Each hand-applied kill is an observation of one mutation, not of the whole selection. Reading a mutant as equivalent is an argument,
not a run.

**2026-11-26 (certificate audit 1, item 9):** the `v40_skill` guard mutant (`diagnostic.rs:580:25`) is now caught by `doom-sim`'s own tests (exit 0 with or without `doom-game`'s; the test added at `fff2a40d` kills it), so the first row of the survivor table, "killed once `doom-game`'s tests run", is out of date for it at this head. The table above is left as written. The mutation workflow now judges `doom-sim` mutants with `doom-game`'s tests too ([AUDIT1 section 9](AUDIT1.md#9-cross-crate-mutation-testing-in-ci)).

## 10. Witness timing

Reported, not gated. Taken 2026-11-26, 02:30 to 02:33 (America/Los_Angeles), **run alone**: nothing else was running (no cargo, no real-map job, no agent; `ps` showed only the
session and an idle `rust-analyzer` at 0.2 % CPU; `free -g` 11 total, 9 available). The load average read 0.8 to 1.0 throughout with no visible process using CPU; that is not explained.
Binary built from `~/scratch/wire28/target-tests` (ext4, release, `--locked`), rustc 1.100.0, i7-8750H, 12 logical CPUs, WSL2. Harness: `crates/doom-sim/examples/wire28_witness_timing.rs` at
`58049822`; it alternates wire 27 and wire 28 runs of one skill 2 demo, takes the wall-clock median in ns per run, and checks equality (tics, RNG, state fields except `profile` and
`context.schema`) outside the timed region. Two passes each:

| Demo | Map | Tics | Runs per wire | Wire 27 ns/tic | Wire 28 ns/tic | Difference |
| --- | --- | ---: | ---: | --- | --- | --- |
| MTELE1 (synthetic `montele.wad`) | E1M1 | 160 | 200 | 2902.5 / 2835.6 | 2930.0 / 2875.0 | +0.95 % / +1.39 % |
| CHAOS (real E1M1) | E1M1 | 5,000 | 40 | 33385.8 / 32790.0 | 33319.5 / 32605.7 | -0.20 % / -0.56 % |
| M7FIGHT (real E1M7) | E1M7 | 1,221 | 60 | 118764.3 / 119131.0 | 124530.1 / 124606.1 | +4.85 % / +4.60 % |
| M7IDLE (real E1M7) | E1M7 | 2,100 | 60 | 107662.5 / 108693.7 | 107437.8 / 108582.7 | -0.21 % / -0.10 % |

Every equality line: tics equal, RNG equal, differing fields `profile` and `context.schema`; every run exited 0.

**What it measures.** The cost of all wire 28 code on a skill 2 run, not of the witness alone: the library has no counter, so the harness cannot count witness checks, and the checks are not
counted here. **M7FIGHT's +4.6 to +4.9 % is as measured:** it repeats in both passes and lies outside the spread of the other three demos (within about 1.4 % of zero). Its cause was not
investigated. It is a wall-clock median on a laptop CPU, not an instruction count. CI's "Simulation instruction counts" job is green on `58049822`.
**Harness limit.** M7CHAOS (4 loads) was run twice and exited 1 both times with `Error: PlayerReborn`: the harness runs one level and does not handle a reborn. That is not a timing result.

## 11. The independent read

A fresh reviewer that did not write the code, read-only, at `fff2a40d`, 2026-11-26 about 00:45 (America/Los_Angeles). **It found no place where wire 28 differs from
the C without refusing** in the damage order, `P_GiveAmmo`, the bite, the thing-spawn bits or the escape witness (both move paths); that nothing frozen changed (wires 27
and below, DSIM 43 and below, `DSES` 2 and below, the older configurations; `tests/reference` and `crates/*/tests` hold only new files; `doom-play` changes only inside the two
allowed tests); that every golden is from the genuine engine; that FIN0's 128 inserted empty tics are honestly described; and that no IWAD coordinate is committed. It ran
scratch probes (a Demon bite on a special-11 sector on wires 23 to 28; `damage_actor` at skill 2 on wire 28), uncommitted and Rust-only.

| Finding | What it said | What was done |
| --- | --- | --- |
| M1 | the bite divergence is on wires 23 to 27, not 24 to 27 (player health 4, every sector special 11: wires 23 to 27 end dead, wire 28 leaves health 1) | fixed in COMPATIBILITY (the heading and its anchor, now linked from ROADMAP and the quirk page), D4, Limits, the M6a note §8.3, Q-p_inter-838 and QUIRKS; wire 27 stays observed by BITE2, wires 23 to 26 by the probe |
| M2 | the restore test has 68 save points, not 60 | fixed in section 6, STATE_FORMAT |
| M3 | `wire28_genuine.rs` has two tests; section 8 omitted `wire28_mutants.rs` and `wire28_mutant_tests.rs` | fixed in section 8; TESTING now links instead of counting (N8) |
| M4 | the M6a note says a corpse's lack of thrust was "observed" | relabelled "(read)" in §8.5 |
| S1 | `DSES` 3 accepts a skill byte that disagrees with the embedded level's `actors.options[0]` (`check_as` checks only the skill domain); inherited from version 2, widened to skills that change rules; no session writes such a state | **not changed** (engine): named limit in Limits and STATE_FORMAT; open decision for the owner |
| S2 | the public `damage_actor` on the player halves at `sk_baby` but never applies the sector-11 cap (all sectors special 11, health 100, skill 2, amount 150: killed; the C leaves 1); API only, not reachable from `doom-play` | **not changed** (engine, since wire 23): named at M6a §8.2 and in Limits; open decision for the owner |
| S3 | "embedded walkers never move" is too broad: a box overlapping a wall by less than the step can walk away | reworded in §8.4 and Q-p_mobj-777: only what S'' alone admits (overlap above 2r - 31, at least 9 for r = 20, 29 for the Demon, against speeds 8 to 10) is stuck, inferred from the arithmetic, not run |
| Notes | N2 the demo3 fuzz doc said the two readers agree (never compared on one input); in `wire28_dses3` `state.check()` after a decode cannot fail. N3 "cannot be stored across tics" holds for a blocked or embedded body only. N4 the pain state is entered for the player unless `P_Random` returns 255. N5 Q-p_map-189's effect column was stale. N6 D5 touches the frozen app's text. N7 §8.5 omitted the chaingunner (health 70: barrels alone 24.6, an escape for d under 4.6, inferred) | N2 doc comment fixed in `fuzz/fuzz_targets/wire28_demo3.rs`; the `state.check()` remark is recorded here only, the `wire28_dses3` target is unchanged; N3, N4, N5, N7 fixed in the note and the quirk rows; N6 a clause at D5, no source edit |

**Its three verdicts.** `app/mod.rs:541:35` is equivalent (agreed). The four `demon.rs:154` mutants are equivalent (agreed). For `spawning.rs:201:38` it agreed the admitted set is
unchanged but the error differs (wire 28 `Monster(Arithmetic)`; wire 27 or the mutant `Perception(Arithmetic)`), so the mutant is killable only on the error variant, and
it is not reachable in play (the only non-test `spawn_type` callers spawn the player body); the map-THING path does drop the margin (`spawning.rs` about 656 and 680), covered
by `s_double_prime_admits_an_embedded_monster_under_wire_28_alone`. Section 9 keeps it open.

**What the reader did not read or check:** no genuine build, no generator `--check`, `tools/local/wire28_real.py` beyond its paths, no fuzzing, mutation, clippy, full test suite or
release build, the unwritten record sections, every `A_Chase` path after a `P_Move` witness refusal for panics, and `Boundary::derive` edge cases on real maps.

## 12. The owner's rulings of 2026-11-26

Seven rulings on the three open decisions and on this record, applied the same day (America/Los_Angeles). Two change engine lines (1 and 2); the rest change tests, runs and text.
**Correction:** the CI run on `6c09beeb` was the full suite (workflow "Rust": Linux, Windows, macOS, Rust 1.85.0, the four genuine jobs and the instruction counts, all green), **not** the documentation fast path, as an earlier report of this work said.

1. **A direct monster spawn follows S'' (`7aa0d05d`).** `preflight_monster_spawn` (`monsters.rs`) passes `margined(ty)` where it passed `true`, so under wire 28 a direct `spawn_type` of a policy monster has no margin clause, as in the THINGS batch (`P_SpawnMobj` checks no position, p_mobj.c:479-534). Every other wire and type is unchanged (`margined` is true there). `a_direct_monster_spawn_follows_s_double_prime` pins what it returns in the hall of section 3's layout: wire 28 admits an Imp at 11, 10, 5 and 1 units from the wall and refuses a centre behind it with `Perception(Arithmetic)`; wire 27 admits 11 and refuses the rest with `Perception(Arithmetic)`; `Monster(Arithmetic)` is returned for none (line 201 reads the same clauses first). Rust-only; not reached in play (the only non-test `spawn_type` callers spawn the player body).
2. **`DSES` 3 refuses a skill byte its level does not have (`95b61473`).** Version 3 alone: `check_as` refuses a state whose skill byte differs from `actors.options[0]` of its DSIM44 capture, with the new `DsesError::SkillAndLevel { skill, level }`. Version 2 is untouched and the test pins that it still accepts a 2/3 disagreement. Test `dses_version_3_refuses_a_skill_byte_its_level_does_not_have`; committed seed `fuzz/corpus/wire28_dses3/skill_and_level` (84,022 bytes, a synthetic SKILL0 state with the skill byte set to 1; written by that test when `WIRE28_DSES3_SEED` is set); the `wire28_dses3` target now also asserts the rule on every accepted state. Fuzz, the changed target only, options as in section 7: seed replay of the committed seed and of a scratch corpus (that seed and the same state with its skill byte restored), address sanitizer, exit 0 both; 100,379 runs in 241 s under the address sanitizer with leak detection on, exit 0; 535,710 runs in 241 s with no sanitizer, exit 0; no crash, no leak, no artifact. The campaigns ran while a `-j 1` cargo-mutants run was using the machine.
3. **`damage_actor` on the player without the sector-11 cap: not changed.** A known divergence from p_inter.c:836-842, API only; on the audit list below with the census.
4. **The Demon and Spectre bite on wires 23 to 27: frozen, documented** ([COMPATIBILITY](../COMPATIBILITY.md#silent-divergence-in-wires-23-to-27-the-bite-on-a-sector-of-special-11)), and on the audit list.
5. **Three more audit items, not changed** (list below).
6. **cargo-mutants over `diagnostic.rs` 580:35 and 581:35**, and over the new lines of 1 and 2 (cargo-mutants 27.1.0, nextest, `-j 1`, a scratch clone of `95b61473` on the Linux disk; the spawn test was restyled afterwards for a Clippy lint, `clone_on_copy`, with no assertion changed):

| Run | Mutants | Caught | Missed | Unviable | Exit |
| --- | ---: | ---: | ---: | ---: | ---: |
| `doom-sim`, its own tests: `diagnostic.rs:580:35`, `581:35`, `spawning.rs:201:38` (named, `--file` and `--re`) | 3 | 3 | 0 | 0 | 0 |
| `doom-sim`, its own tests: the lines changed since `6c09beeb` (`--in-diff`; `monsters.rs:622:9`, `647:12`) | 2 | 2 | 0 | 0 | 0 |
| `doom-game`, its own tests: the same diff (`state_v2.rs:385:9`, `419:19`, `419:33`, `419:48`) | 4 | 4 | 0 | 0 | 0 |

7. **The CI statement** is corrected above and in the pull request's text.

**Rerun on `5d238a17` (2026-11-26, America/Los_Angeles), before merge, at the owner's request:** from a clean clone of that head with new work directories (the genuine side regenerated), `wire28_real.py --set e1m7 --selftest` (the six E1M7 skill 3 constructions: 6 of 6 pass, 16,459 session tics, both self-tests reported at their tics, exit 0) and `--set sessions` on the owner's three hand-test recordings (082510: 18,263; 083445: 3,929; 083716: 3,643; 3 of 3 pass, 25,835 session tics, exit 0), every stream at screenblocks 10 and 11, 0 refusals, 0 differences.

**The audit list** (for the later pull request that audits the old certificate argument, [ROADMAP](../ROADMAP.md); nothing here is changed now):

- the public `damage_actor` on the player never applies the sector-11 cap (since wire 23; section 11, S2);
- the Demon and Spectre bite on the player skips that cap on wires 23 to 27 (wire 28 applies it);
- the demo version-3 parsers accept a command whose button byte has its top two bits set, which the writers refuse to write (section 7);
- `Snapshot::diagnostic_bytes` handles versions 20 to 40 and refuses 44 (Limits);
- M7FIGHT runs 4.6 to 4.9 % slower under wire 28 than under wire 27, cause not investigated (section 10).

**Census: every path that damages the player, against the sector-11 cap.** Read from the code at `6c09beeb` by a read-only agent on 2026-11-26 (**read, not run**; no probe, no genuine comparison). In the C every one goes through `P_DamageMobj`, so the cap (p_inter.c:836-842) applies to all. In Rust the cap is `hell_cap` (`combat.rs`), called from two places only: `DamagePlan::current_original_in` (`combat.rs:528`, when the policy has Episode 1) and the bite (`demon.rs:160`, wire 28 only). Sector special 11 is admitted from wire 23 on (`validate_sector_specials_for`), so the cap cannot matter on wires 1 to 22.

| Path to the player | Rust site | Cap on wires 23 to 27 | Cap on wire 28 | Reached in play |
| --- | --- | --- | --- | --- |
| Former-human hitscan | `player_monsters.rs`, `bullet` | yes | yes | yes |
| Imp and Baron fireball, contact | `projectiles.rs`, `strike` | yes | yes | yes |
| Barrel and rocket splash | `barrels.rs`, `pit_radius_attack` | yes | yes | yes |
| Imp claw; Baron claw (wires 24 on) | `player_monsters.rs`, `troop_attack_checked`, `bruis_attack_checked` | yes | yes | yes |
| Floor damage (specials 4, 5, 7, 11, 16) | `sector_thinkers.rs:569` | yes | yes | yes |
| **Demon and Spectre bite** | `demon.rs`, `strike_player` | **no** | yes | yes |
| **Public `Simulation::damage_actor`** (also through `ExecutionContext::damage` and the older `Simulation` missile `strike`) | `combat.rs:921` | **no** | **no** | no: API only |
| Telefrag | `DamagePlan::telefrag` | refuses a player target | the same | not for the player |
| Crushers, lost soul | none found | not admitted | not admitted | no |

**What the census did not settle:** whether any profile configuration reaches `damage_actor` or the older missile `strike` on wires 23 to 28 (inferred: none in play); whether the player's own rocket can ever touch the player directly (the owner skip was read in part; its splash is the splash row); that the older fixture attacks (`execute_attack` with a plan prepared earlier, `current_map_melee`), which have no cap, never run on wires 23 to 28 (inferred from the dispatch). The skill 0 halving sits before the cap on all three request builders.

**2026-11-26 (certificate audit 1):** the audit list and the census above are answered in [AUDIT1](AUDIT1.md); no engine line changed. The census was run: the three open points are settled (no configuration reaches `damage_actor`, `ExecutionContext::damage` or the older missile `strike` with the player as target on wires 23 to 28 except a direct API call; the player's own rocket cannot touch the player directly; the older fixture attacks never run on wires 23 to 28), and the wires 23 to 26 bite is confirmed against the genuine engine, not only by the Rust-only probe. The tables above are left as written.

## Limits

- The genuine comparison is synthetic (section 3) for everything the real maps did not reach: real E1M7 at skill 3 never brought a
  monster to the wall, so **the witness has no real-map observation of firing**. It is a refuse-the-tic witness: where a monster or a
  corpse would leave the map the session stops with a TIC REFUSED screen and keeps the demo.
- E1M7's imp distance is not recorded; the M6a note's arithmetic bounds what a blow can do, and east and north escapes are inferred,
  only west was observed.
- ESCAPE, SMALL and the rooms behind them are one room layout built to the witness; they show equality, refusal and standing still,
  not every monster, wall angle or blow.
- Wires 23 to 27 keep the bite divergence (D4); only wire 27 was observed to differ against the genuine engine (BITE2), wires 23 to 26 by the independent reader's
  Rust-only probe (section 11).
- **The three decisions that were open here were ruled on by the owner on 2026-11-26** ([section 12](#12-the-owners-rulings-of-2026-11-26)): `preflight_monster_spawn` follows S''; `DSES` 3 refuses a
  skill byte its embedded level does not have; the public `damage_actor` stays without the sector-11 cap (since wire 23; API only, not reachable from `doom-play`), **a known divergence** on the audit list.
- The real E1M8 finale on skills 0 and 1 is covered by the hand test of section 5.4 (three recordings); no existing construction reaches the victory there, and none was forced. The skill 0 and 1
  replays of section 5.3 are the same recordings as on other skills, not new play. The hand test ran on `cbe88eba`; two engine changes came after it (section 5.4, last paragraph).
- Witness timing (section 10) is the cost of all wire 28 code, one unexplained +4.6 to +4.9 % on M7FIGHT, no per-witness count.
- The DSES version 3 restore test compares a session with itself, and its melt save points are not labelled by melt kind.
- Fuzz seeds are not committed (section 7). `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings` fails on an existing
  file, `game_input_events.rs` (`manual_range_patterns`), not on a wire 28 file.
- `Snapshot::diagnostic_bytes` handles versions 20 to 40 only and refuses 44; DSIM44 bytes come from `snapshot().canonical_bytes()`.
- `StartError::Domain`'s text still says "skill 2 or 3" (shared with frozen configurations; API only).
- Not checked here: nightmare (refused), skills 0 and 1 beyond the items, FIN0 and FIN1 and the replays of section 5.3, and any other episode. Section 9: `doom-sim`'s 150 own-test kills not rerun with `doom-game`'s tests.
