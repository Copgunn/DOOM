# Wire 27 evidence record (the wrapping `P_InterceptVector`)

Observed results of wire 27 (`PlayerMonstersEpisode1V5`, captures `DSIM` 43), recorded 2026-11-24 (America/Los_Angeles) on
branch `feat/wire27-intercept-vector`, [PR #82](https://github.com/Copgunn/DOOM/pull/82). The rule and what is admitted
are in [COMPATIBILITY](../COMPATIBILITY.md#wire-27-the-wrapping-p_interceptvector), the format in
[STATE_FORMAT](../STATE_FORMAT.md#dsim-v43--wire-27-the-wrapping-p_interceptvector), the commands in
[TESTING](../TESTING.md#wire-27-the-wrapping-p_interceptvector), and the quirk rows in the
[register](../QUIRKS.md) (Q-p_maputl-240a, Q-p_maputl-240b, Q-p_maputl-846c, Q-m_fixed-62-4). Exact review commits and CI
belong to GitHub. Only counts, pass/fail facts and map names from the commercial IWAD appear here.

**Tic convention (stated once, used throughout).** A session tic is the 0-based index of a tic, equal to the number of
tics completed before it: the number `doom-play` prints. A refusal at session tic N keeps the state after N tics.

## 1. Why wire 27 exists

Two real E1M8 sessions at skill 2, recorded by the owner, are refused by wire 26 with `MapScale(Arithmetic)`: at session
tic 3894 and at session tic 3064, the first tic after each recording's end (shown on our copies extended with idle tics,
section 4). The cause is `P_InterceptVector` (p_maputl.c:231-251): its `int` sums overflow there, and the genuine engine,
built with `-fwrapv`, wraps. Wires 23 to 26 refuse such a tic (decision H3). **Wire 27 reproduces the wrap only for the
caller and wrap-class pairs that a genuine-compared item takes (section 3); it does not wrap "whole" or "for every
caller".** Wire 26's refusal on E1M8 is a known gap, documented and not patched.

**The claim names the genuine build:** linuxdoom-1.10 compiled with `-fwrapv` (the "strict" build) compiles the genuine
text and is the claim. Every state digest below is that build's, projected by the independent `DSIM` 43 encoder
(`tools/reference/level_snapshot_wire27.py`), never Rust output.

## 2. The eight genuine items (CI)

One WAD, [wire27.wad](../../tests/reference/wire27_wads): **eight map slots, one room layout** (a long slanted east wall
that makes the traces' intercept leave `int`), one demo script per slot. Item data is in
[wire27_results.json](../../tests/reference/wire27_results.json) and the digests in
[wire27_state.txt](../../tests/reference/wire27_state.txt). Wire 27 equals the genuine strict build at every tic of every
item; wire 26 refuses each item at the tic shown (session tic).

| Item | Slot | Tics | Wrapping calls the genuine engine made | Wire 26 refuses at | Nowrap build stops at |
| --- | --- | ---: | --- | ---: | --- |
| USE | E1M1 | 16 | use, cancel x1 | 5 | p_maputl.c:246-248, tic 5 |
| SLIDE | E1M2 | 35 | slide, cancel x15 | 20 | no stop |
| AIMSHOT | E1M3 | 35 | aim cancel x2; aim and shoot_line "den wrong, num exact" x1 each | 18 | 246-248, tic 18 |
| SHOOTZ | E1M4 | 35 | shoot_line, den wraps to zero x1 | 18 | no stop |
| SATUR | E1M5 | 35 | shoot_line, wrong/wrong, saturation MAXINT x1 | 18 | 246-248, tic 18 |
| NUMWRAP | E1M6 | 35 | shoot_line, wrong/wrong, FixedDiv2 x1 | 18 | no stop |
| SATNEG | E1M7 | 35 | shoot_line, wrong/wrong, saturation MININT x1 | 18 | no stop |
| SHOOTC | E1M8 | 35 | aim cancel x2; shoot_line cancel x1 | 18 | 246-248, tic 18 |

What the wrap does in each: SHOOTZ, intercept 0, a puff behind the shooter, the barrel not hit; SATUR, a wall added with
fraction MAXINT that is never traversed, the shot hits the barrel behind it; SATNEG, a negative fraction is "behind
source" (p_maputl.c:591-592), no wall, no puff; NUMWRAP, a puff in the open room about 757 units out where exact
arithmetic puts it at the wall; AIMSHOT, the wall is not hit, no puff.

How the item data was produced. The **witness build** (strict flags, `-finstrument-functions` on p_maputl.c and
`--wrap` of `FixedMul`, `FixedDiv` and `P_PathTraverse`, [wire27_witness.c](../../tools/reference/wire27_witness.c)) logs
every `P_InterceptVector` call with its caller and operands; the **nowrap build** (the overflow build without the two
adapters) must stop at the first `int` add or subtract overflow the witness shows and otherwise run to the end identical
to strict (the "Nowrap build stops at" column). The **overflow** (UBSan, `-fno-wrapv`) and **address** (ASan + UBSan)
builds compile p_maputl.c with two declared sanitizer adapters, applied by `tools/reference/wire27_goldens.py` to its own
copy: p_maputl.c:240 and p_maputl.c:246-248, unsigned casts around the two sums (original and adapted text and their
SHA-256 are in the results file). Those two builds, the witness build and the strict build gave identical dumps, sound
and session streams. **The address build runs with `ASAN_OPTIONS=detect_leaks=0`: leak detection is disabled, so it is not
LeakSanitizer coverage.** The results file was generated with `cc` (Ubuntu 15.2.0); the CI compiler differs.

**Declared in-process overrides** (`wire27_wad.py`, listed under `in_process_overrides` in the results file and in the
[tools README](../../tools/reference/README.md#wire-27-the-wrapping-p_interceptvector-against-the-genuine-engine)): while
the maps are built, `m3h6_geometry.MAX_NORM` is raised to 16,382 (so the east wall stays one unsplit linedef) and
`m5a_wad.seg_angle` is replaced by a rounded `atan2` (the slanted wall's SEGS angle, read by the renderer alone). No file
is edited and both are restored. The owner accepted this.

Self-tests, each detected: an extra `P_Random` draw, a witness row with a `FixedMul` result one off, a witness row with an
unknown traverser.

## 3. Admitted and refused: caller by wrap class

Source of the numbers: `class_table` in the results file and the `T` rows of the state file. The cell is the item that
takes the pair and the genuine calls it witnessed; "refused" means the tic is refused with `MapScale(Arithmetic)`, state
kept, and **no genuine item reaches any refusal: refusals are shown by unit tests only**
(`wire27_admits_exactly_the_witnessed_classes` checks the table against the `T` rows).

| Wrap class | aim | shoot_line | slide | use |
| --- | --- | --- | --- | --- |
| wraps cancel (den and num exact, FixedDiv2) | AIMSHOT 2, SHOOTC 2 | SHOOTC 1 | SLIDE 15 | USE 1 |
| den wrong, num exact, FixedDiv2 | AIMSHOT 1 | AIMSHOT 1 | refused | refused |
| den wrong, num wrong, FixedDiv2 | refused | NUMWRAP 1 | refused | refused |
| den wrong, num wrong, saturation MAXINT | refused | SATUR 1 | refused | refused |
| den wrong, num wrong, saturation MININT | refused | SATNEG 1 | refused | refused |
| den wraps to zero | refused | SHOOTZ 1 | refused | refused |

- **Each admitted pair rests on one or two witnessed calls, with two exceptions: `slide`'s cancel pair has 15 (all in
  SLIDE) and `aim`'s cancel pair has 4 (two in each of two items).** Every other admitted pair has exactly one.
- **shoot_thing** stays on the checked computation: a thing's intercept line is its diagonal, 2 x radius, so the products
  cannot wrap on an admitted map. There is no item for it, and every wrap class refuses.
- **No-wrap classes** are admitted for every caller and equal wire 26's checked value. Ordinary division has genuine calls
  for every caller except use (aim 18, shoot_line 1, shoot_thing 8, slide 7); no item reaches an unwrapped saturation or
  an exactly-zero unwrapped denominator.
- **Refused for every caller:** a numerator or denominator exactly `INT_MIN`, wrapped or not (`AbsMin`; `abs(INT_MIN)` is
  undefined even under `-fwrapv`, so **refused, not reproduced**); FixedDiv2's `I_Error` range (`Div2Range`); every wrap
  class not in the table, including saturation for aim, slide and use and saturation with an exact numerator.
- **A monster's aim trace is as long as its shot, so the aim row limits monster fire:** aim admits only "cancel" and "den
  wrong, num exact"; a monster shot whose aim trace falls in any other wrap class refuses the tic.
- **Saturation signs.** Genuine items cover two of the four sign combinations: SATUR (numerator and denominator both
  positive, MAXINT) and SATNEG (numerator positive, denominator negative, MININT). The other two (numerator negative with
  denominator positive; both negative) rest on unit tests only
  (`a_wrapped_saturation_follows_the_sign_of_num_xor_den`, m_fixed.c:62-63).
- **Sight** (`P_InterceptVector2`, p_sight.c:109-128) does not wrap under wire 27: it stays checked and additionally
  refuses an exactly-`INT_MIN` numerator or denominator.
- **After wire 27** the code reading is: no `FixedDiv` site is neither refused on an `INT_MIN` operand nor proven unable to
  receive one (19 `Fixed / Fixed` expressions and one direct call examined); it relies on the player alone sliding under
  wire 27.

**The E1M8 finale is expected, not guaranteed.** The refused pairs above are known gaps for a later wire; a real session
can still meet one.

## 4. Real maps (local only; pass/fail and tic counts)

`tools/local/wire27_session_real.py` reads the IWAD in place through a symlink, never a copy. The "+700 idle" rows are
**our constructions** (each recording extended by 700 idle tics so that it passes the point where wire 26 refuses); the
two recordings are **the owner's**. Compared with the genuine strict build after every tic through the independent
encoder. Final code, rerun:

| Session | Wire | Tics compared | Result | Level ended |
| --- | --- | ---: | --- | --- |
| owner's recording 115101 | 26 | 3894 | pass | no |
| owner's recording 115101 | 27 | 3894 | pass | no |
| 115101 + 700 idle tics (our construction) | 26 | 3894 | refused at session tic 3894 | no |
| 115101 + 700 idle tics (our construction) | 27 | 3905 | pass | yes |
| owner's recording 115513 | 26 | 3064 | pass | no |
| owner's recording 115513 | 27 | 3064 | pass | no |
| 115513 + 700 idle tics (our construction) | 26 | 3064 | refused at session tic 3064 | no |
| 115513 + 700 idle tics (our construction) | 27 | 3093 | pass | yes |

Self-test: a flipped ticcmd and an extra `P_Random` draw are each detected at the next capture. No wrap occurred in the
recorded parts; the extensions logged three wrapping calls (one aim, two shoot_line), all in the cancel class;
shoot_thing never wrapped.

**Saturation in E1M8: not observed in these sessions, not ruled out.** Ruled out, by an in-memory check of 43,646,976
line, direction, trace-length and nudge combinations: a wrapped denominator small enough to saturate, or equal to
`INT_MIN`, for any E1M8 line. Open: an unwrapped small denominator with a numerator that wraps at some trace start.

## 5. Wires 1 to 26 unchanged: the proof and its limits

One scratch driver built against each tree, master (25cdb3ba) and the branch (cac2ae37): 3,585 scenario lines identical
(outcome, refusal tic and text, SHA-256 over the canonical state after every tic): 3,312 synthetic (the M4a to M6a demo
families under wires 23 to 26, the eight items under wires 23 to 26, and admission of 92 maps by 38 constructors) and 273
on real maps (nine Episode 1 maps, wires 23 to 26, including the two recordings and the two extended copies).
**Limits:** wires 1 to 22 got the admission comparison and their existing tests only; the driver is not committed (kept in
scratch during the PR). No existing test, fixture, golden or manifest is modified or deleted; the test list grew by 17
(13 unit, 4 integration; one doctest only shifted a line), and every existing generator's `--check` matched.

## 6. The BLOCKMAP census and the bound on the intercept operands

Local only (`tools/local/e1_blockmap_listings.py`, never in CI), results in
[wire27_e1_blockmap.json](wire27_e1_blockmap.json) (IWAD size and SHA-256 there). It supports one claim about wires 23 to
26, stated in [COMPATIBILITY](../COMPATIBILITY.md#silent-divergence-in-wires-23-to-26-an-int_min-operand): **proven
unreachable on seven Episode 1 maps; not proven on E1M2 and E1M8.**

**Census** (does every line listed in a cell meet that cell; two readings of each cell's leading 0 word: (a) a marker,
(b) a listing of linedef 0 as `P_BlockLinesIterator` iterates it). Miss distance: 0, 1 or more than 1 cells.

| Map | Cells | Listings (a) | Missed (a) | Miss (a) | Listings (b) | Missed (b) | Miss (b) |
| --- | ---: | ---: | ---: | --- | ---: | ---: | --- |
| E1M1 | 828 | 986 | 4 | 1 | 1,814 | 831 | more than 1 |
| E1M2 | 1,302 | 2,025 | 3 | 1 | 3,327 | 1,302 | more than 1 |
| E1M3 | 850 | 1,893 | 8 | 1 | 2,743 | 856 | more than 1 |
| E1M4 | 660 | 1,625 | 2 | 1 | 2,285 | 661 | more than 1 |
| E1M5 | 832 | 1,516 | 4 | 1 | 2,348 | 834 | more than 1 |
| E1M6 | 1,748 | 2,654 | 1 | 1 | 4,402 | 1,746 | more than 1 |
| E1M7 | 864 | 1,827 | 5 | 1 | 2,691 | 867 | more than 1 |
| E1M8 | 2,912 | 960 | 7 | 1 | 3,872 | 2,918 | more than 1 |
| E1M9 | 702 | 1,280 | 1 | 1 | 1,982 | 699 | more than 1 |

The converse census fails on all nine maps under both readings (reading (a): 1 to 8 missed listings per map, each by one
cell; reading (b): widely). No map has an invalid line reference or a cell without its leading 0. **Bound verdicts**
(PASS = the operand's bound stays below 2^31, so it cannot be `INT_MIN`): E1M1, E1M3 to E1M7 and E1M9 pass for numerator
and denominator for every trace kind (slide, use/melee, aim, attack, sight), things, and linedef 0. E1M2: the line
numerator is not proven for any kind (the tool says FAIL, and **E1M2's hitscan numerator is not proven**), the sight
denominator is not proven, the rest is proven. E1M8: the line numerator is not proven for any kind, the attack-2,048 and
sight denominators are not proven, the rest is proven. A FAIL means not proven, not that `INT_MIN` is reached. Linedef 0
passes on all nine maps.

> **Marked for the independent read: (a) the derivation below, and (b) its premise 7**, that every actor's centre stays
> inside the map's vertex rectangle. That premise is a Rust admission rule, not a C fact; "proven on seven maps" rests on
> it. Neither has had the independent read yet.

**The derivation** (transcribed from the docstring of `tools/local/e1_blockmap_listings.py`; R is the map's closed vertex
rectangle, in map units):

1. *Roles.* `PIT_AddLineIntercepts` calls `P_InterceptVector(&trace, &dl)` with `dl` the line: start v1, delta (dx, dy) in
   whole units (p_maputl.c:588-589; `P_MakeDivline` 211-218). `PIT_AddThingIntercepts` passes a thing's diagonal, 2 x
   radius per axis through its centre (p_maputl.c:616-663). `P_CrossSubsector` calls `P_InterceptVector2(&strace, &divl)`,
   the same body, with `divl` the line (p_sight.c:176, 185-188, 227).
2. *Exact form.* `FixedMul` is `((long long)a*b)>>16` (m_fixed.c:44-49). With whole-unit L = (Ldx, Ldy), `dl.dy>>8` is
   256 x Ldy and `FixedMul(a, Ldy<<16)` = a x Ldy exactly, so (p_maputl.c:240, 246-248; p_sight.c:117, 123-124)
   den = floor(Ldy x Tx / 256) - floor(Ldx x Ty / 256) and num = floor(X/256) x Ldy + floor(Y/256) x Ldx, with T the
   trace delta, X = v1.x - t.x, Y = t.y - v1.y in raw units, t the trace start. Radii in info.c are whole units, so a
   thing diagonal has the same form.
3. *Denominator.* |den| < (|Ldy| x |Tx| + |Ldx| x |Ty|) / 256 + 1.
4. *Numerator.* X x Ldy + Y x Ldx = Ldy x (p.x - t.x) - Ldx x (p.y - t.y) for every point p of the line, so with d the
   Chebyshev distance in units from t to the closed segment and L1 = |Ldx| + |Ldy|: |num| < 256 x d x L1 + L1 (the two
   floors lose less than L1).
5. *Verdict.* A bound at most 2^31 - 1 makes the value differ from `INT_MIN`, the one operand where `unsigned_abs` and
   `abs` disagree: PASS. Otherwise FAIL, meaning not proven. `checked_intercept` (sight.rs) returns the source's value or
   refuses, so PASS means no silent divergence for that operand.
6. *Deltas.* `trace.dx` = x2 - x1 after the +FRACUNIT nudge (p_maputl.c:777-786), so |T| is at most |delta| + 1 unit;
   |finesine| is at most 65535 (tables.c:584-1866). Aim and attack deltas are (distance>>16) x finecosine (p_map.c:1034-1035,
   1076-1077): use 64 (USERANGE p_local.h:56; p_map.c:1144-1147), punch and saw 64 (MELEERANGE and MELEERANGE+1 after
   >>16, p_local.h:57; p_pspr.c:483-516), 1024 (p_pspr.c:607-616, 795; p_mobj.c:949-959), 2048 (MISSILERANGE p_local.h:58;
   p_pspr.c:639, 721-723; p_enemy.c:813-862). A slide traces `mo->mom` (p_map.c:738-743), clamped to 30 (p_mobj.c:137-145).
   One projection maps a per-axis bound m to m + ceil(m/2) (`FixedMul` by `bestslidefrac` at most FRACUNIT, p_map.c:768-777;
   `P_HitSlideLine` 596-629 with `P_AproxDistance`, p_maputl.c:49-58). `P_SlideMove` traces at entry and after one
   projection and leaves at most two (hitcount, p_map.c:706-710, 781-786); `P_XYMovement` calls it at most twice
   (p_mobj.c:150-191). Slide deltas: 30, 45, 67.5 or 101.25 units. Sight: strace delta = t2 - t1 (p_sight.c:342-343).
7. *Starts (premise from the Rust policy, not from the C).* Every actor centre is in R before each committed tic and after
   it ([M4a note A6](../M4A_REFERENCE_INVESTIGATION.md); teleport landings too, teleport.rs:63); a tic that fails is refused
   with its state preserved, so only committed tics matter. Players think first (p_tick.c:150-152) and use and fire from
   the tic-start centre (p_user.c:326, 334): margin 1 (the nudge). Another actor seen during a thinker is at its tic-start
   or final centre. A monster attacks or looks after its own `P_XYMovement` (at most 30 per axis; a blocked non-player
   stops, p_mobj.c:150-191) and before its `P_Move` (p_enemy.c:726-765; speed at most 16, info.c): margin 64 for sight
   ends, 65 for monster traces. Before a slide trace the player moved at most 30 + 45 + 67.5 = 142.5 units in a first
   `P_SlideMove` (flush 756-763, re-flush, then stairstep 749-751 or projection 784) plus 67.5 (the second call's flush);
   `EV_Teleport` zeroes momentum at a landing in R (p_telept.c:103-125). With radius 16 (info.c:1125; p_map.c:714-733) and
   the nudge the margin is 227.
8. *Lines met.* The block walk visits at most 64 cells, each step moving one coordinate by 1 (p_maputl.c:846-876; the
   comment at 841-842 says round-off can skip the end cell: the walk can then run past it along the extended line), from
   the start cell, which holds t (790-791). Off-grid cells list nothing (481-487). A line listed under reading (a) in a
   visited cell, on a map of miss class (a) at most 1, has a point within one more cell: d < 128 x (63 + 1 + 1) = 8320. In
   every case d is at most Dg, the largest Chebyshev distance from a corner of R grown by the margin to the segment (d is
   convex in t). Each line takes min(Dg, 8320) where that holds and Dg otherwise; linedef 0 (every cell's leading 0) always
   takes Dg, and so does every line for sight, which meets lines through the BSP (p_sight.c:160-227).
9. *Things* (aim, attack, punch, saw: `PT_ADDTHINGS`; slide and use add lines only, p_map.c:738-743, 1147). The diagonal
   passes through the centre c, so d is at most |c - t|, at most span + 2 x margin, with L1 = 4r, r at most 128 (info.c:1619).
10. *Lines tested:* every word of every list (reading (b)) for traces, every linedef for sight.

Step 8 is where the overshoot of section 7 matters: the "64 + 1 + 1" cells assume the walk stays near the trace.

## 7. The block-walk overshoot (a stub harness, not a full-engine run)

`P_PathTraverse`'s block walk (p_maputl.c:841-876) is bounded only by `for (count = 0; count < 64; count++)`
(p_maputl.c:846), not by the trace's length. Round-off can make a short trace skip its end cell and then either run up to
63 cells further or stick on one cell until the count runs out. **Shown in a stub harness only, not in a full-engine
run** (the genuine walk text p_maputl.c:743-886 compiled with the genuine m_fixed.c and stub block iterators, in
scratch; not committed). **There is no genuine-compared item for this path.** The quirk register's rows Q-p_maputl-846a/b
say only that a very long trace is cut short; the overshoot is the new row Q-p_maputl-846c. The premise of the M3h6
certificate that a visited cell lies within 2,048 + 128 + 1 units of the trace start does not hold when the walk
overshoots ([dated note](../M3H6_REFERENCE_INVESTIGATION.md), after the bound on `P_InterceptVector`). Nothing in
code or certificate was changed; the seven maps still pass under the map-extent bound of section 6, **which rests on the
vertex-rectangle premise (step 7), a Rust admission rule marked for the independent read.**

## 8. Fuzzing, by type (the run from the repository's seeds only)

Target `fuzz/fuzz_targets/wire27_dsim43.rs` (the decoder `Diagnostic::decode_episode1_v5` only; Rust-only safety).

- **Compiled:** `cargo +nightly fuzz build --sanitizer address` built all 65 targets.
- **Seeds:** written from the repository by `crates/doom-sim/tests/wire27_fuzz_seeds.rs` (wire-27 captures of the item
  AIMSHOT at session tics 0, 35 and 70; [command](../TESTING.md)). An earlier run from scratch-made seeds is not reported.
- **Seed replay:** the 3 seeds, `-runs=0`: no crash.
- **Coverage-guided campaign:** from those seeds alone into a fresh corpus, `-runs=2000000 -max_len=120000 -seed=1
  -timeout=20 -rss_limit_mb=2048`: 2,000,000 runs, 0 crashes, 0 artifacts, 419 corpus files.
- **Sanitizer:** AddressSanitizer through cargo-fuzz with `ASAN_OPTIONS=detect_leaks=0`: **leak detection disabled; not
  LeakSanitizer coverage.** nightly 1.101.0, cargo-fuzz 0.13.2.

## 9. Mutation testing (cargo-mutants 27.1.0, nextest, `-j 1`)

- **Scope 1**, all of `intercept_vector.rs`, filter `test(wire27) | test(wire_27) | test(intercept)`: 63 generated, 61
  caught, 1 missed, 1 unviable, 0 timeout. **Missed:** `intercept_vector.rs:232:24 replace ^ with | in classified`, a real
  gap (no test had a saturating intercept with numerator and denominator both negative); answered by the test
  `a_wrapped_saturation_follows_the_sign_of_num_xor_den` (four sign pairs); the targeted re-run caught it, **the full 63 were
  not re-run after the new test.** Unviable: line 205 (`classified` replaced by `Ok(Default...)` does not compile).
- **Scope 2**, lines this PR changed elsewhere in `doom-sim` (`--in-diff`, excluding `intercept_vector.rs` and tests), full
  `doom-sim` suite: 101 generated, 79 caught, 1 missed, 21 unviable (`Default` returns that do not compile), 0 timeout.
  **Survivor:** `movement.rs:869:61 replace && with || in Mover::slide` on the pre-existing line
  `let checked = self.world.checked_arithmetic && limit.is_some();`, swept into the scope by neighbouring changed lines. No
  test was written: the line is not changed by this PR, and distinguishing inputs need a wrapping slide with a checked world
  and a no-limit access (or the reverse), a pin of pre-existing behaviour that needs its own C citation. **Not proven
  equivalent.** It is an **open survivor**: the owner accepted the stated reason for this pull request (2026-11-24), and it
  is a named item of the later certificate-audit pull request, to be closed there by a test or by a proof with a C
  citation.

## 10. CI note

On one earlier head the macOS job failed once in an existing `doom-play` wall-clock test
(`a_stalled_disk_keeps_the_queue_bounded_and_loses_no_tic`: 747 ms against 500 ms), unrelated to this PR, and passed on
rerun. The fix is for a later PR ([ROADMAP](../ROADMAP.md)).
