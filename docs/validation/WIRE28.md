# Wire 28 evidence record (the easy skills and E1M7 on skill 3)

Observed results of wire 28 (`PlayerMonstersEpisode1V6`, captures `DSIM` 44, session state `DSES` 3), recorded 2026-11-25
(America/Los_Angeles; sections 5.3, 5.4, 9 and 11 added 2026-11-26) on branch `feat/wire28-easy-skills`, [PR #86](https://github.com/Copgunn/DOOM/pull/86). The rule and what
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

No recorded E1M7 run on skill 3 exists, so these are **our constructions**: the five E1M7 runs of the 61 (recorded on skill 2) with
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
skill 0 or 1 is expected, and an equal run that never reaches it still counts as compared tics. **No route was constructed and nothing here is derived from the
owner's own recordings.** What each of the twelve E1M8 demos of section 5.3 does on each easy skill:

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
genuine engine in CI: their cost was 149 lines plus 21 (the orchestrator's count when they were built, not recounted here).
**E1M8 to the finale on skills 0 and 1, real map: not yet covered** (as of 2026-11-26). It is to be supplied by the owner's hand-test sessions on the Windows
build and verified against the genuine engine afterward; their verification is running and is not recorded here.

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
prefix), so the runs above are not reproducible from the repository's seeds alone. **Observed while fuzzing (not a library fault):**
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
a skill 0 recording round-trips its header and replays); `crates/doom-game/tests/wire28_mutants.rs` (3, for three doom-game mutation survivors, section 9); `crates/doom-sim/src/wire28_mutant_tests.rs` (1, `dsim44_counts_a_skill_2_capture_with_thing_bit_2`, test-only, wired in by one `#[cfg(test)] mod` line in `lib.rs`). Fuzz targets: section 7.

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
| `diagnostic.rs:580:35`, `581:35` (`&&` to `\|\|`) in `v40_skill` | **killed** by `wire28_mutant_tests::dsim44_counts_a_skill_2_capture_with_thing_bit_2` (`fff2a40d`: a DSIM44 capture at skill 2 on a map whose bit-1 and bit-2 thing counts differ). Observed by applying each mutation by hand and seeing the test fail; the expected behaviour is read from the C (p_mobj.c:743-751, 789-790; info.c:1416), not observed against the genuine engine. **cargo-mutants was not rerun over these two** |
| `demon.rs:154:35` (`&&` to `\|\|`), `154:31` (`>` to `>=`), `154:52` (`&` to `\|`, and `&` to `^`) in `strike_player` | **argued equivalent, no test.** The line is `live = health > 0 && flags & MF_SHOOTABLE != 0`. Only two body states reach it: (A) health 1 or more with `MF_SHOOTABLE` set, (B) health 0 or less with it clear (a kill clears the flag). In A and B each of the four mutants gives the original value (the `^` one because a live player body also carries `MF_SOLID`, an invariant checked at `original_player.rs:404`, so the XOR stays nonzero). The independent reader (section 11) agreed |
| `doom-game` `recorder.rs:83:13` (field `v3` deleted) | **killed** by `a_wire_28_recording_writes_the_easy_skills` (`wire28_mutants.rs`, `4f78d30a`); hand-applied mutation, and caught in the cargo-mutants rerun of the fourth row |
| `app/mod.rs:541:50` (`==` to `!=`) | **killed** by `the_wire_28_app_records_the_easy_skills`; the same two observations |
| `session/state_v3.rs:45:9` (`check` to `Ok(())`) | **killed** by `check_refuses_a_state_out_of_its_domain`; the same two observations |
| `app/mod.rs:541:35` (`&&` to `\|\|`) in `GameData::recorder` | **argued equivalent, no test** (missed in the rerun). Older configurations refuse skills 0 and 1 at load, and on skills 2 and 3 the version 2 and version 3 demo writers give equal bytes, so which writer the `\|\|` picks cannot be seen in any recording. The independent reader agreed |
| `doom-sim` `spawning.rs:201:38` (`&& self.margined(ty)` to `\|\|`) in `spawn_type_checks` | **OPEN: not proven equivalent; no test.** Finding: the margin clause this pull request added on that line does not change what a direct `spawn_type` admits. For a policy monster under wire 28, `preflight_monster_spawn` (`monsters.rs:644`, not changed here) still applies the margin, so the spawn is refused either way and only the error differs (`Monster(Arithmetic)` against `Perception(Arithmetic)`). Not a defect against the C (`P_SpawnMobj` has no position check, and the C has no direct-spawn call; map THINGs under S'' take a different path, covered by `s_double_prime_admits_an_embedded_monster_under_wire_28_alone`). **Owner decision needed:** either `preflight_monster_spawn` follows S'' too (a behaviour change) or line 201's change is dead and is reverted; left as is, no engine change in this pull request |

**Not rerun:** cargo-mutants was not rerun over the two `v40_skill` kills (`580:35`, `581:35`), and the 150 `doom-sim` mutants caught by `doom-sim`'s own tests were not
rerun with `doom-game`'s tests. Each hand-applied kill is an observation of one mutation, not of the whole selection. Reading a mutant as equivalent is an argument,
not a run.

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
- **S1, open decision for the owner:** `DSES` 3 accepts a state whose skill byte disagrees with its embedded level's `actors.options[0]`; no session writes one.
  **S2, open decision for the owner:** the public `damage_actor` on the player halves at skill 0 but never applies the sector-11 cap (since wire 23); API only, not
  reachable from `doom-play`. Neither is changed in this pull request.
- **Open mutation survivor, open decision for the owner:** `spawning.rs:201:38` is not proven equivalent and has no test (section 9): `preflight_monster_spawn` follows
  S'' too, or line 201's change is reverted.
- **Real-map E1M8 to the finale on skills 0 and 1: not yet covered** as of 2026-11-26 (section 5.4); the 63 demos per skill never reach the victory, only the synthetic
  FIN0 and FIN1 do. The skill 0 and 1 runs are the same recordings as on other skills, not new play.
- The DSES version 3 restore test compares a session with itself, and its melt save points are not labelled by melt kind.
- Fuzz seeds are not committed (section 7). `cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings` fails on an existing
  file, `game_input_events.rs` (`manual_range_patterns`), not on a wire 28 file.
- `Snapshot::diagnostic_bytes` handles versions 20 to 40 only and refuses 44; DSIM44 bytes come from `snapshot().canonical_bytes()`.
- `StartError::Domain`'s text still says "skill 2 or 3" (shared with frozen configurations; API only).
- Not checked here: nightmare (refused), skills 0 and 1 beyond the items, FIN0 and FIN1 and the replays of section 5.3, and any other episode. Section 9: cargo-mutants
  not rerun over two kills; `doom-sim`'s 150 own-test kills not rerun with `doom-game`'s tests.
