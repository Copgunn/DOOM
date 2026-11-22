# M6a evidence record: the level chain, up to the owner's hand test

**Status (2026-11-22, America/Los_Angeles): this is the record up to the hand-test gate, not the finished milestone.** The
playable level chain (exits, intermission, carried start, reborn, secret route) is built and compared; the PR stops next for the
owner's hand test. Not built yet, each a typed refusal or a declared gap: the finale after E1M8, the melt pictures (the wipe's
draws are made and compared, the picture changes at once), the easy skills 0 and 1, and E1M7 on skill 3. Steps 10 to 17 follow
the hand test and will extend this record. **CI is not observed green on the final head** (see [Not run](#not-run-and-limits)).

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
the next map's walk from its load). Wall time 04:09 to 10:25 PST (the first singles batch started before); the logs do not
record the head: by the report times the tool and tests were those after `1229b0ce` (the stale-attacker threshold rule) and before
the ATTACK item (`369d6c97`), which is inferred, not logged.

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
session stops there with `NotYet(Victory)`. **CH2 crosses no level boundary:** the tool reported "no load after tic 3863; the
chain stops there" in both modes; its three loads are reborn reloads (the reason the next map's load was not reached was not
investigated).

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
No session has yet been played on the owner's IWAD with the real binary; his hand test and its results are not part of this record.

## Not run, and limits

- **Local only.** Every real-map figure is from the owner's IWAD on his WSL2 machine on 2026-11-22; it cannot run in CI and nobody
  else can repeat it without the same file. Nothing here was measured for time.
- **Not rerun at the final head** (`f4f9bce0`): the real-map sets above, nor after the ATTACK item was appended to the indexes.
- **Not run** (per the step reports): the workspace-wide tests, workspace Clippy and rustdoc on the final head, the release
  simulation and type tests, MSRV 1.85.0, any fuzz target or sanitizer campaign for the new formats (`DSES`, `DDSP` v3,
  `DSND` v2, DSIM42), the Windows build, macOS and Windows CI. **CI was not observed green on the final head:** at 11:00 on
  2026-11-22 the Actions API showed no run for three of the heads (the PR then conflicted with master; step 9 later reported it
  mergeable), and no run on the final head has been observed.
- **Not covered by any run:** the finale, the melt pictures, the easy skills and E1M7 on skill 3 (not built); a quirk-6 witness
  (no golden for percentages above 100 or "sucks"); a genuine run of the reborn-beats-exit case on the owner's IWAD.
- **Sanitizers.** The genuine runs used the strict build; the address-sanitizer variants and the `address_bounds` build belong
  to the synthetic goldens ([TESTING](../TESTING.md)); leak detection is disabled there (`detect_leaks=0`), so it is not
  LeakSanitizer coverage.
