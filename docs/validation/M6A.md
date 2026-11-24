# M6a evidence record: the level chain, up to the owner's hand test

**Status (2026-11-22, America/Los_Angeles): this is the record up to the hand-test gate, not the finished milestone.** The
playable level chain (exits, intermission, carried start, reborn, secret route) is built and compared; the PR stops next for the
owner's hand test. Not built yet, each a typed refusal or a declared gap: the finale after E1M8, the melt pictures (the wipe's
draws are made and compared, the picture changes at once), the easy skills 0 and 1, and E1M7 on skill 3. Steps 10 to 17 follow
the hand test and will extend this record. The gates run at the final code head are in [§4](#4-gates-at-the-final-code-head);
what was not run is in [Not run](#not-run-and-limits). **After the gate (2026-11-22 evening):** the owner's first hand test, the
rebuilt chain CH2 and the wire-26 constructor fix are in [§5](#5-after-the-hand-test-gate-2026-11-22-evening).

Observations only. [TESTING](../TESTING.md#the-playable-level-chain-steps-4-to-9) owns the commands,
[COMPATIBILITY](../COMPATIBILITY.md#episode-1-as-one-session-m6a) the support statement, the
[investigation note](../M6A_REFERENCE_INVESTIGATION.md) the C facts and rulings, and [ROADMAP](../ROADMAP.md) the status. Exact
review state and CI belong to GitHub. Counts, pass or fail facts and result digests only: nothing from the IWAD's lumps, names,
coordinates or geometry is recorded.

## 1. The real-map comparison (step 8), local only

**What ran.** `tools/local/m6a_session_real.py` ran the genuine M6a runner on a scratch copy of the owner's IWAD named for the
mode (his file only read), `--jobs 2`, then the Rust gates `real_maps_session_local` and `real_maps_state_local` read its
goldens (`M6A_REFERENCE_DIR`, release build). Per run the gates compare the session rows, every frame at screenblocks 10 and 11,
sound rows, the genuine mixer's buffers and the DSIM42 state digest of every level tic. All runs used the committed 61 real runs
(**singles**: each started at its map, a new game) or the nine **chains** (CH1 to CH9: a walk to the exit, the intermission,
the next map's walk from its load). Wall time 03:55 to 10:25 PST. The logs do not
record the head; by the commit times it was `1229b0ce` (the stale-attacker threshold rule, committed 03:45): the next commit on
the branch is the ATTACK item (`369d6c97`, 10:36). The Rust gates were run again at the final head ([§4](#4-gates-at-the-final-code-head)).

| Set | Runs | Session tics | State: levels / captures / points | Session gate | State gate | Stale-attacker rule fired on | Result digest (size) |
| --- | ---: | ---: | --- | --- | --- | --- | --- |
| retail singles, batch 1 | 15 | 37,961 | 19 / 37,961 / 1,111 | pass | pass | 0 loads | `9b8c20252a68a193789d48260b6f02de1670f846cc873cb3b5673a5eee89f6d8` (25 MB) |
| retail singles, batch 2 | 46 | 158,872 | 84 / 158,872 / 4,660 | pass | pass | 0 loads | `559075664c275e5396d9548d1c0d118206da835ae9ee6e4f6c78d10954a4e26f` (108 MB) |
| registered singles | 61 | 196,833 | 103 / 196,833 / 5,771 | pass | pass | 0 loads | `b79c5f684cf7e11668e0491e17d39136a75b07736394cb5bf96e7ef2f2ce080e` (132 MB) |
| retail chains | 9 | 90,816 | 37 / 90,683 / 2,647 | pass | pass | 6 loads | `39faf49a421d50faf6387fad96820ed7937fa3740001977ccaafdea6a0f831cb` (62 MB) |
| registered chains | 9 | 90,816 | 37 / 90,683 / 2,647 | pass | pass | 6 loads | `764887ea2e9310062cddf892006436028ec2f50a9d117fdc1c3fa74d9ff03cfd` (62 MB) |

All 5 sets: 0 failures reported by the tool, no first difference in any gate. The digest is SHA-256 of the sorted list of the
SHA-256 of every file in the set's output directory (IWAD-derived scratch, not committed; the work directories were deleted after
each set). The two chain digests differ though every count agrees (the files carry the mode's own lines).

**The retail singles were run in two batches (15 and 46 runs) for disk reasons.** Their sums equal the registered singles in
every compared count: session tics 196,833 (37,961 + 158,872), the gate's loads (84), wipes (206), level frames (393,666), sound
rows (276,047), buffers (121,825), mixer rows (117,580), masked frames (1,138), uncomparable frames (0), and the state levels, captures and points above (sums
computed from the two batch logs). This is the owner's condition of ruling U4: all 61 compared in retail, state dumps and frame
digests included, against the registered results. The chains' gate counts are identical in the two modes as well.

**Stale attacker.** The rule fired on 0 loads in all 61 singles in both modes and on 6 loads in each chain set: one each in
CH3, CH4, CH5, CH6, CH7 and CH9 (0 in CH1, CH2 and CH8). Before the rule the independent encoder refused CH4's first carried load: it
had no rule for the C's dangling pointer (the reasoning and the four conditions are in
[note §5](../M6A_REFERENCE_INVESTIGATION.md#owner-rulings-of-2026-11-22-the-stale-attacker)).

**Planted faults** (run on both chain sets, not on the singles): the session gate with a carried start one health short
(`CarriedHealth`) and the state gate with a changed carried `bob` (`CarriedBob`). Each was **caught in 7 runs and missed in none**,
in retail and in registered: the runs with a carried start (CH1, CH3, CH4, CH5, CH6, CH7, CH9), each at its first carried load
(session tics 785, 5,812, 3,463, 9,475, 7,858, 7,755 and 5,982).

**What the chains reached.** The forward-100 walks after a load (the E1M4, E1M6 and E1M7 walks use a forward move of 100,
spliced as committed; the turbo message follows the session's gametic, ruling 1 of 2026-11-22) ran in CH3, CH5, CH6 and CH9 and
passed in both modes. CH8 ended at the E1M8 victory cut: compared up to the tic before the finale (5,813 session tics), as the
session stops there with `NotYet(Victory)`. **CH2 as built for these sets crossed no level boundary:** the tool reported "no load
after tic 3863; the chain stops there" in both modes; its three loads were the start and two reborn reloads. So in the two chain
sets of the table **eight of the nine chains cross a boundary**, not nine. The cause was found and CH2 rebuilt afterwards
([§5.2](#52-chain-ch2-rebuilt)); the table's sets were not regenerated.

## 2. Synthetic checks that belong with the gate

| Check | Observed |
| --- | --- |
| The ATTACK item (a synthetic monster hurts the player, who exits; our own WAD) | the genuine state through the independent encoder: the stale-attacker rule fired on load 1 at session tic 235 only (`tests/reference/m6a_stale_attacker.txt`: `A,ATTACK,1,235`, `T,1`); the Rust session equals the genuine goldens at screenblocks 10 and 11 and the level state on 2 levels |
| `m6a_state.py --selftest` | 14 of 14 planted cases pass, among them the genuine-side fault (attacker set to the player's own body from the carried load): the rule fires on nothing and the load capture differs from the committed digest |
| The played session (`m6a_play`, step 9) | REBHELD's demo recorded byte for byte, CHAIN's first 346 tics by prefix, every picture and `G` column equal the genuine goldens, replay reaches the same `DSES` bytes; planted faults caught |
| The played session through the genuine engine (synthetic WADs, local) | a 519-tic session through the secret exit: genuine end marker, 4 loads, 0 failures; the Rust gates passed; planted faults caught at the carried load (session tic 297) |
| `m6a_melt_steps.py --check` | 125 of 125 melts of 29 items: predicted step count equals the observed one (2026-11-22) |

## 3. The hand-test gate

The Windows artifact is built only from the `windows-artifact` label, from the PR head. Its README.txt (generated in
`.github/workflows/windows-artifact.yml`) tells the owner how to run the session, the options, where the demos go and which
stops are expected: E1M8's exit (finale not built) and, on skill 3, the load of E1M7. The owner sends the `.lmp` files back; each
is checked against the genuine engine with `tools/local/m6a_play_check.py` ([TESTING](../TESTING.md#the-playable-level-chain-steps-4-to-9)).
When this section was written no session had been played on the owner's IWAD with the real binary; his first hand test is in
[§5.1](#51-the-owners-first-hand-test).

## 4. Gates at the final code head

Local, 2026-11-22, one command at a time. The code head is `f4f9bce0`; `9ab8c3b3` after it changes documents and the artifact's
README text only (no file under `crates`, `tools` or the Cargo files differs).

| Gate | Head | Observed |
| --- | --- | --- |
| Build, `cargo fmt --check` (workspace and fuzz workspace), Clippy with warnings denied, rustdoc with warnings denied | `f4f9bce0` | clean |
| Workspace tests (debug) | `f4f9bce0` | 2,087 passed, 0 failed, 10 ignored (149 targets) |
| Release simulation and type tests | `f4f9bce0` | 1,412 passed, 0 failed, 2 ignored (87 targets) |
| Rust 1.85.0 workspace tests | `f4f9bce0` | 2,087 passed, 0 failed, 10 ignored |
| The Rust real-map gates again, against the kept genuine result sets of §1 (11:53 to 14:50 PST) | `9ab8c3b3` | session and state gates pass on all 5 sets; the two planted-fault gates pass on both chain sets (14 gate runs, 0 failures) |
| CI | `9ab8c3b3` | all nine jobs green (the full suite, not the docs fast path) |
| Windows artifact (the `windows-artifact` label) | `9ab8c3b3` | built: `doom-play-windows-x86_64`, 1,636,887 bytes |

The re-run did **not** regenerate the genuine side: the genuine engine's result files are those of §1, made with the tools at
`1229b0ce`. Since then `m6a_state.py`, `m6a_wad.py` and `m6a_session_real.py` changed for the ATTACK item and the fired-count
record; the real sets were not regenerated with the changed tools. A later push does not rebuild the artifact (it is built on
the label event only), and CI on any later head is GitHub's to show, not this record's.

## 5. After the hand-test gate (2026-11-22 evening)

### 5.1 The owner's first hand test

The owner played four sessions on the Windows artifact built from `9ab8c3b3` (retail mode, skill 2), each ended by his quit,
and reported nothing wrong by eye and 0 audio underruns and 0 failed writes in every log (his observation, not measured here).
His four demo files were copied to scratch (never committed) and checked with `tools/local/m6a_play_check.py` and the Rust
`real_maps` gates, 18:24 to 18:42 PST (the genuine runs with the tools of `f0dc8a02`, the Rust gates at code head `0101b9e1`). The genuine engine replayed each file to its end marker.

| Session (start map; route) | Session tics compared | Loads (start included) | Level tics / other tics | Session rows | Frames, screenblocks 10 and 11 | Sound and mixer | Whole state (levels / captures) | First difference |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- |
| E1M1; to E1M2 | 1,405 | 2 | 980 / 425 | equal | equal (1 and 1 masked) | equal (1,667 rows, 885 buffers) | equal (2 / 980) | none |
| E1M3; a death and reborn (reload at session tic 938), then to E1M9 by the secret exit | 9,025 | 3 | 8,808 / 217 | equal | equal (32 and 34 masked) | equal (11,080 rows, 5,576 buffers) | equal (3 / 8,808) | none |
| E1M9; to E1M4 | 14,352 | 2 | 14,256 / 96 | equal | equal (30 and 36 masked) | equal (17,408 rows, 8,840 buffers) | equal (2 / 14,256) | none |
| E1M1; to E1M2, a death and reborn (reload at session tic 1,791), then E1M3 | 3,734 | 4 | 3,576 / 158 | equal | equal (32 and 33 masked) | equal (4,599 rows, 2,298 buffers) | equal (4 / 3,576) | none |

Sums: 28,516 session tics, 11 loads (4 starts, 5 carried, 2 reborn), 27,620 state captures (804 digest points), 199 masked
frames over both sizes, 0 uncomparable frames; the rows and buffers are the gate's counts. A masked frame is one where the original reads undefined memory in some columns; it is compared with
those columns blanked (decision R4), every other frame by its whole digest. Sound and mixer rows are compared at screenblocks
10; the genuine screenblocks-11 sound stream must equal the screenblocks-10 one. The stale-attacker rule fired on 3 loads (one
each in the first, second and fourth sessions). Planted faults: carried health caught in 4 of 4 sessions and carried bob in 4
of 4, each at the session's first carried load. Result digest (as in §1): `f6f3f0165859edf64234905cd22bfef580ab47c73a1c0d74c2f397791554baed` (19 MB,
scratch). The fourth session covers by hand the route CH2 was meant to cover (a reborn, then a level change); the second holds a reborn
as well.

The demo file names carry UTC times ([COMPATIBILITY](../COMPATIBILITY.md#episode-1-as-one-session-m6a)), so these four read
`20261123-02…` for sessions played on the evening of 2026-11-22 in America/Los_Angeles.

### 5.2 Chain CH2 rebuilt

**Cause.** CH2 is E1M2's death script (999 tics), the reborn reload, E1M2's walk, then E1M3's walk. The reload at tic 999 was
reached; the "3863" of the message is 999 plus the walk's 2,864 tics. The committed walk is a new game's: after the reborn the
`P_Random` index is not reset (207 after that load, where a new game has 64), the monsters act differently, and the player
died at session tics 1,838 and 4,008 (reborn once more at 2,183 by one of the walk's own use presses) without reaching the
exit. Idle tics before the walk did not help (0 to 59 tried, none reached another map).

**Fix** (`tools/local/m6a_session_real.py`, `build_chain`): the script's reborn press is moved later, one idle tic at a time, to
the first reload that begins with `P_Random` index 0 and from which the walk reaches another map, each candidate a genuine
run. Observed in both modes: press moved 886 tics, reload at session tic 1,885, E1M3 loaded at 4,768 (and a reborn on E1M3 at
5,737, as other chains have after their boundary); the built demo is the same bytes in both modes.

| Set (CH2 alone) | Session tics | Loads | State: levels / captures / points | Session gate | State gate | Planted faults | Stale-attacker rule | Result digest (size) |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- |
| retail | 10,765 | 4 | 4 / 10,746 / 312 | pass | pass | caught, 1 of 1 each | 1 load | `af75e1b9a342bce2b6604cd5d872ad8f6282b42fe90bc45c1138c94d63d2c29e` (6.9 MB) |
| registered | 10,765 | 4 | 4 / 10,746 / 312 | pass | pass | caught, 1 of 1 each | 1 load | `0f06dd9d7c78431f810cdad8dcc04cae47cf84da5ab0c594c44fa729d20e6ac4` (6.9 MB) |

With it all nine chains cross a boundary, but that is a statement about two result sets: CH1 and CH3 to CH9 in the sets of §1
(tools at `1229b0ce`) and CH2 here (tools at `c69c8783`). **The nine were not run again as one set.** The chain totals of §1
(90,816 tics, 6 stale-attacker loads, planted faults caught in 7 runs) are the first build's and count the old CH2 (4,067 tics).

### 5.3 Wire 26 has one constructor

An independent review found, and a reading of the code confirmed, that the general constructor
(`Simulation::with_original_registry`) accepted the wire-26 profile and returned a simulation with no level start, which the
snapshot writer labelled DSIM42 and wrote without the start group its reader requires (the group was written under an
`if let`). No test or session path built one: `carried_start.rs` was the only caller for wire 26. Fixed at `0101b9e1`: the
general constructor refuses wire 26 before anything is built, and the writer takes the start group without a skip, so a
wire-26 simulation with no start is not written at all. Two tests in `crates/doom-sim/src/wire26_tests.rs`
(`the_general_constructor_refuses_wire_26`, `a_wire_26_simulation_without_its_start_is_not_written`). Wires 1 to 25 take the
same construction path as before; their fixtures and DSIM41 expectations are unchanged and pass (the gates below).

### 5.4 Gates at `c69c8783`

Local, 2026-11-22 19:05 to 19:29 PST, one command at a time, no uncommitted code. Docs commits follow this head.

| Gate | Observed |
| --- | --- |
| Build, `cargo fmt --check` (workspace and fuzz workspace), Clippy with warnings denied, rustdoc with warnings denied | clean |
| Workspace tests (debug) | 2,089 passed, 0 failed, 10 ignored (149 targets): the 2,087 of §4 and the two new tests |
| Release simulation and type tests | 1,414 passed, 0 failed, 2 ignored (87 targets) |
| Rust 1.85.0 workspace tests | 2,089 passed, 0 failed, 10 ignored |
| The Rust real-map gates | on the four hand-test sessions (at `0101b9e1`) and on CH2 in both modes (the crates of `0101b9e1`, the tool change of `c69c8783` not yet committed): pass, with the planted-fault gates. **Not run again** on the five sets of §1 |

No existing test, fixture or expectation was changed by either commit: every DSIM1-41 expectation passes as committed. CI on
this head and later ones is GitHub's to show.

### 5.5 The reader review (2026-11-22, late evening)

An independent read-only review of the four new format readers at `af64d1aa` listed eleven items. Each was checked against the
code first. **All eleven had something real; three were narrower than stated**: of item 9's four states, three (a cleared wipe
at the entry, a level scene's tic, carried taint with no wipe due) were already refused by the display's `check` and by every
restore, and were accepted only by decoding alone; of item 11's second state, the load capture (level tic 0) was already
refused through the cross-check with `events.pending`, and the hole was at level tic 1; and item 10's generation-0 origin was
already refused at restore (it resolves to no actor) and accepted only by decoding. The rules as they now stand are in
[STATE_FORMAT](../STATE_FORMAT.md#session-state-dses-version-1-m6a).

| Item | Found | Change |
| --- | --- | --- |
| 1. `DSES`: `wminfo.last + 1` on the file's value | real: overflow panic in a debug build, wrap in a release build | checked addition; refused as `wminfo map` |
| 2. Intermission: `bcnt + ...` on restored values; wrapping tally arithmetic | real for the two sums; the wrapping operations were already equal in both builds, but reachable with any value | the two sums wrap like the rest; the reader now refuses what made them matter: `bcnt` is the intermission's tic count, `nexttic` 1 to 11 ahead, counts at most `i32::MAX / 100`, every tally counter on its stage's steps |
| 3. `DDSP` v3: run count added before the bound | real on a 32-bit `usize` only | compared with the room left; the same sum in the older `DRST` v2 reader is documented as a limit and not patched |
| 4. `DSES` `secretexit` against `Session::secretexit()` | real disagreement on an exit tic | **documents changed, code and bytes kept**: the byte is the last completed exit's flag, the accessor the C's variable; the pending exit is in the level capture |
| 5. `DDSP` v3 melt tag "never written" | real: `encode` writes it for a hand-built state | **document changed** (and the type's comment): no display produces it; the format carries it; `check` refuses it |
| 6. `DSND` v2: two channel counts and two texture counts in the in-memory state | real: a disagreeing state encoded to another, valid state | **code changed**: a disagreeing state is written with the field's largest value, which the reader refuses; bytes of consistent states unchanged |
| 7. `DSES`: `gameaction` 8 in the tally, any view heights, any counts, a carried player unlike the finished one | real | 8 exactly when the last count is 0; counts, maxima, time and the carried player equal the finished level's at restore, the carried health equals `st_oldhealth` at decode; **view heights cannot be tied** (no embedded state holds them outside a level): `viewheight` bounded above, the rest a declared limit |
| 8. `DSES`: embedded `DSIM` map, tic, start tic; `DDSP` `entered` against `DSND` level end | real at decode (tic and start tic were tied at restore; the map and `entered` were not) | the capture's map, tic and start `gametic` and the display's `entered` are checked when decoding |
| 9. `DDSP` v3: four states | one real (`st_oldhealth` above 200 outside a level); three accepted by decoding alone | `st_oldhealth` 0 to 200 outside a level; `DSES` runs the display's `check` when decoding |
| 10. `DSND` v2: four states | real at decode | refused by version 2's reader (`Unreachable`); version 1's reader untouched |
| 11. `DSIM` 42 start group against the level | real at level tic 0 (health) and tic 1 (the chainsaw record) | checked by the version 42 reader at those tics |

**No saved byte and no golden changed.** No file under `tests/reference` is touched; every state a session saves encodes as
before (only hand-built `DSND` states with disagreeing counts encode differently). One existing M6a test was edited, none on
master: `refusals_leave_the_session_unchanged` in `crates/doom-sound/tests/m6a_chain.rs` forged a state at the last countable
tic with channels still occupied, which the new handle rule refuses; the forgery now clears its channels (one line).

**That the rules refuse nothing a session saves** (Rust against Rust): `every_saved_state_decodes_and_restores` saves, decodes
and restores the state before every tic and after the last of every committed item the session starts: 26 items, 11,367
tics, 11,399 states, none refused (NOE1M2AT, SKILL0 and SKILL1 do not start). Locally on the real-map sets, decoding every
state and restoring every 35th: the four hand-test sessions (28,516 tics,
28,520 states) and CH2 retail (10,765 tics, 10,766 states), none refused. The other real-map sets were not run through it.

**Tests added** (12 that CI runs and 1 local): `doom-game` `m6a_dses_reader` (5, and the local real-map one) and 1 in
`session/intermission.rs`; `doom-sound` `dsnd2_reader` (2); `doom-render` `d_display/session_reader_tests.rs` (3); `doom-sim`
`wire26_tests.rs` (1). For items 1, 2, 7, 8 and 11 the forged state was accepted (or, for 1 and 2, overflowed) before the
change: read from the code for all, and run for item 11 with the new check switched off.

**Gates at `7490f5e2`** (local, 2026-11-22 20:22 to 20:47 PST, one command at a time, no uncommitted code; `7490f5e2`
differs from `572ed88f` in one doc comment):

| Gate | Head | Observed |
| --- | --- | --- |
| Build, `cargo fmt --check` (workspace and fuzz workspace), Clippy with warnings denied | `572ed88f` | clean |
| Workspace tests (debug) | `572ed88f` | 2,101 passed, 0 failed, 11 ignored (151 targets): the 2,089 of §5.4 and the 12 new tests |
| Release simulation and type tests | `572ed88f` | 1,415 passed, 0 failed, 2 ignored (87 targets) |
| Rustdoc with warnings denied | `572ed88f`: **failed** (a link from a public comment to a private constant); `7490f5e2`: clean | fixed in the comment |
| Rust 1.85.0 workspace tests | `7490f5e2` | 2,101 passed, 0 failed, 11 ignored |
| The local real-map saved-state check | the code of `572ed88f` before its commit | the counts above |

**Not run again after the reader changes:** the Rust real-map session and state gates (§1, §5.1, §5.2). The readers changed
are not on those gates' path (they compare rows, frames, sound and capture digests and do not decode a saved state), but that
is an argument from the code, not an observation.

### 5.6 Before the owner's review: part 1 (2026-11-22, night)

By the owner's ruling PR #65 is M6a part 1 (the level chain); the finale, the melt pictures, the easy skills and E1M7 on
skill 3 are part 2. No feature was added after the ruling. Code head `6689bb47` (fuzz targets, a seed writer in a test, the
CI job; no file under `crates/*/src` differs from `7490f5e2`).

**The owner's second hand test** (three more sessions on the artifact of `9ab8c3b3`; genuine engine and Rust gates as §5.1,
at `6689bb47`'s crates): all equal, no first difference.

| Session | Mode | Session tics | Loads (start included) | State: levels / captures | Masked frames, sizes 10 and 11 |
| --- | --- | ---: | ---: | --- | --- |
| E1M1 to E1M2, ended by a new game | registered | 3,145 | 2 | 2 / 2,767 | 8 and 9 |
| E1M1, the new game after it, ended by quit | registered | 198 | 1 | 1 / 198 | 0 and 0 |
| E1M4 to E1M6, a death and reborn there, ended by quit while dead | retail | 15,286 | 4 | 4 / 14,842 | 47 and 50 |

Sums: 18,629 session tics, 7 loads; planted faults caught in the 2 sessions that have a carried start; every saved state
decoded (18,632) and every 35th restored; result digest `b1cb290601e6972a638057b5fa0d7f38f4973d6f2bbecbec0a82c48bd14d2f02`
(13 MB, scratch). These are the first registered-mode sessions played by hand.

**Fuzzing the four readers** (2026-11-22, cargo-fuzz 0.13.2, nightly, AddressSanitizer build, **leak detection on**: the
default, `detect_leaks` not set; decoders only, restore is not driven; seeds and corpora in scratch, not committed):

| Target (reader) | Seed replay under the sanitizer | Coverage-guided campaign under the sanitizer, 240 s, `-seed=1` | Findings |
| --- | --- | --- | --- |
| `m6a_dses` (`DSES` v1) | 3 seeds, 4 runs | 614,099 runs, `-max_len=120000` | none |
| `m6a_ddsp3` (`DDSP` v3) | 3 seeds, 4 runs | 319,783 runs, `-max_len=100000` | none |
| `m6a_dsnd2` (`DSND` v2) | 3 seeds, 4 runs | 25,980,575 runs, `-max_len=2048` | none |
| `m6a_dsim42` (`DSIM` 42 with its start group) | 3 seeds, 4 runs | 2,812,003 runs, `-max_len=131072` | none |

One sanitizer only (address, with its leak check); no memory or thread sanitizer run, and no campaign longer than four
minutes. The targets were built one at a time; the whole fuzz workspace was not rebuilt.

**The CI job `M6a genuine session`** (ruling U15) fits under the cap and is in. Its seven commands passed locally with
`--jobs 2` before the push (exit 0; the state step: 21 items, 63 levels, 6,205 captures, both golden files matching).

**The Rust real-map gates again on the final code head** (`6689bb47`, release build, 2026-11-22 21:50 to 2026-11-23 01:20
PST, no uncommitted code; the genuine result sets are the kept ones of §1, §5.1 and §5.2, not regenerated): **26 gate runs,
0 failures.**

| Set | Session gate | State gate | Planted-fault gates |
| --- | --- | --- | --- |
| retail singles, batches 1 and 2 (15 and 46 runs) | pass | pass (19 and 84 levels; 37,961 and 158,872 captures) | not run on singles |
| registered singles (61 runs) | pass | pass (103 levels, 196,833 captures) | not run on singles |
| retail chains and registered chains (9 runs each, CH2 as first built) | pass | pass (37 levels, 90,683 captures each) | caught in 7 of 7 runs, both faults, both modes |
| CH2 rebuilt, retail and registered | pass | pass (4 levels, 10,746 captures each) | caught in 1 of 1, both faults, both modes |
| the four sessions of hand test 1 | pass | pass (11 levels, 27,620 captures) | caught in 4 of 4, both faults |

The three sessions of hand test 2 were gated at the same crates earlier that evening (above). CI on `6689bb47`: all ten
jobs green, the new `M6a genuine session` among them.

### 5.7 The owner's third hand test (2026-11-23)

Two more sessions the owner played on the artifact of `9ab8c3b3` (retail mode, skill 2, about 17:41 to 17:52 PST; the file
names read `20261124-01…` in UTC). Checked as in §5.1, 17:55 to 18:10 PST, with the tools and crates of `43157152` (no
uncommitted change; no file under `crates`, `fuzz` or `tools` differs from `6689bb47`). The genuine engine replayed each file
to its end marker with 0 failures: both equal, no first difference.

| Session | Session tics compared | Loads (start included) | Level tics / other tics | Session rows | Frames, screenblocks 10 and 11 | Sound and mixer | Whole state (levels / captures) | First difference |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- |
| E1M7 to E1M8, ended by quit | 15,522 | 2 | 14,906 / 616 | equal | equal (29 and 30 masked) | equal (19,225 rows, 9,589 buffers) | equal (2 / 14,906) | none |
| E1M8, to the end of the file (by the owner's account it ends at the finale refusal at E1M8's exit) | 4,988 | 1 | 4,988 / 0 | equal | equal (16 and 17 masked) | equal (7,746 rows, 3,100 buffers) | equal (1 / 4,988) | none |

Sums: 20,510 session tics, 3 loads (2 starts, 1 carried), 19,894 state captures (573 digest points), 92 masked frames over
both sizes, 0 uncomparable frames. The stale-attacker rule fired on 1 load (the first session's). Planted faults: carried
health and carried bob each caught in the 1 session that has a carried start, at session tic 15,234. Every saved state
decoded (20,512) and every 35th restored. Result digest (as in §1): `1601c966e7477e4ad00564f62a558dd33cd0cc392377cbc2c813f90fb69e2a32`
(14 MB, scratch). Nothing after the last recorded tic of the second file is compared: the finale is not built.

## Not run, and limits

- **Local only.** Every real-map figure is from the owner's IWAD on his WSL2 machine on 2026-11-22 and 2026-11-23; it cannot run in CI and nobody
  else can repeat it without the same file. Nothing here was measured for time.
- **Not regenerated after `1229b0ce`:** the genuine side of the real-map sets (see [§4](#4-gates-at-the-final-code-head)), except
  CH2 and the hand-test sessions of [§5](#5-after-the-hand-test-gate-2026-11-22-evening).
- **Not run:** the original-C generators' `--check` locally other than the M6a ones (CI runs them);
  `tools/local/m5d_lost_display.py` after step 9 changed `doom-play`; a fuzz campaign that drives restore. (When this list was
  first written no fuzz target existed for the new formats and no CI job ran the genuine M6a engine: see §5.6.)
- **Not covered by any run:** the finale, the melt pictures, the easy skills and E1M7 on skill 3 (not built); a quirk-6 witness
  (no golden for percentages above 100 or "sucks"); a genuine run of the reborn-beats-exit case on the owner's IWAD.
- **Sanitizers.** The genuine runs used the strict build; the address-sanitizer variants and the `address_bounds` build belong
  to the synthetic goldens ([TESTING](../TESTING.md)); leak detection is disabled there (`detect_leaks=0`), so it is not
  LeakSanitizer coverage.
