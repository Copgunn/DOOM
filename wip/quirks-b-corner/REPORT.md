# WIP: the block-corner trace (Q-p_maputl-866 and Q-p_maputl-871), follow-up B

Work in progress, stopped on 2026-11-22 about 00:30 PST because usage ran out, not at a decision. This directory is a
copy of scratch sources (`~/scratch/quirks/followup-b/corner/` on the owner's machine) kept on a side branch so the
work is not only on one disk. It is not for merging: follow-up B's pull request is docs-only. Nothing here was
reviewed. Synthetic maps only; no IWAD was used.

The owner's instruction (2026-11-22): build a synthetic scene that sends a trace exactly through a BLOCKMAP cell
corner, run it in the genuine engine, compare with Rust on every wire that has traces, and report what the original
does, what each wire does and which documents are wrong. If Rust matches: a docs correction. If any wire differs:
stop and report; frozen wires are not patched.

Every statement is labelled read, observed or inferred. The figures are the investigating agent's, relayed by the
orchestrator from its final message; the orchestrator did not re-run them.

## Established

- (read, p_maputl.c:866-875) The step is an `if / else if` with no final else. When neither test is true nothing
  changes, so the walk stays on that cell until the 64-step limit (line 846) and then uses what it collected
  (line 879).
- (observed, genuine engine, 46 direct `P_PathTraverse` calls through exact corners, slopes 1 and 2) Up-right
  (+x,+y) stops in the current cell. Down-left (-x,-y) steps into the side cell and stops there. Down-right (+x,-y)
  and up-left (-x,+y) visit one side cell, then the diagonal, and carry on. "Visits only two of the four cells" is
  therefore wrong in every direction. Of 32 near misses (one fixed-point unit off) some still stop, at a later
  corner: an exact hit is not needed.
- (observed, genuine engine in play) The shot stops in all four diagonal directions, from a start nudged off a
  block line, on autoaim (a barrel never targeted), on the use trace (a door not opened) and near the shooter. Each
  control one map unit away does not stop.
- (observed) A repeated cell adds its things again on every repeat; one scene collects 63 intercepts. (read)
  Nothing checks the 128 limit.
- (observed) The probe build's dumps are byte-identical to the unmodified strict build in all 22 scenes.

## Rust against the genuine engine: no difference on any wire that ran

- (observed) Wires 22, 23, 24: the repository comparators report a tic-for-tic match on all 21 scenes on each wire.
- (observed) Wire 25: `level_compare_m5c` matches on all 21 scenes, with 0 sound disagreements.
- (observed) Instrumented copy, wires 22 to 25: 76 runs and 296 traces; each has the same nudged start, cell at
  every step, step count, end reached and intercept count as the genuine run.
- (observed) Wires 7 to 21: every trace that ran matches. The 768-unit scenes match to the end on all of wires 7 to
  21. Wires 7 to 16 refuse the larger maps at load. Wires 7 to 13 refuse tic 36 of two scenes because the stopped
  shot's puff lands outside those wires' position limit: an explicit refusal, and the traces still match.
- (observed) Direct calls to Rust's slide/use walk (`movement::path`): 46 of 46 equal the genuine calls.
- (observed) Historical profile 5 (not a wire) refuses a stopped trace with `TraversalLimit` and leaves the state
  unchanged. (read only) Profiles 6, 7 and 10 to 12 use the same code.
- (read) The Rust code mirrors the missing else at `crates/doom-sim/src/combat_trace.rs:330-337` and
  `movement.rs:520-527`.

## NOT established

- The mutant self-test, the proof that these comparators can fail on these scenes, was never run: its build was
  stopped. **Until it runs there is no final verdict.**
- Not run: the intercept-overflow scene (D2), the overflow and address genuine builds, an in-game slide scene,
  wires 2 to 6, and profiles 6, 7 and 10 to 12.
- The four documents were not re-read at their exact lines.

## Provisional verdict

Rust matches the original on every wire tested (7 to 25), which would make this a docs correction. Provisional
until the mutant self-test has run and the not-run items above are either run or stated as limits.

## Exact next action

Apply `probe/mutant.diff` to a copy of the tree (`repo-mutant`), build
`cargo build -p doom-sim --release --locked --offline --example player_level_run` into its own target directory,
run `tools/run_rust.py --bin <mutant player_level_run> mutant H1PP U1 S3`, and expect a first difference at the
shot or use tic. Then run the not-run items, then write the docs correction.

## Documents that look wrong (proposals; nothing was edited)

- `docs/COMPATIBILITY.md` near line 1549 and the M3h5 note near line 152 ("visits only two of the four cells").
  Proposed: "At a block corner P_PathTraverse steps one axis at a time with no third branch (p_maputl.c:866-875):
  moving up-right it stops in the current cell, moving down-left it enters the side cell and stops; the other two
  directions visit one side cell, then the diagonal, and go on; a stopped walk repeats its cell to the 64-step limit
  and uses what it collected. Rust reproduces this on every wire." (The last sentence only once the verdict is
  final, and worded for the wires actually run.)
- Rows Q-p_maputl-866 and Q-p_maputl-871 and `docs/quirks/disagreements.md` entry 3: the stop depends on
  direction, happens on near misses too, and Rust matches.
- The comment at `crates/doom-sim/src/combat_trace.rs:162-165`: wires 7 to 25 all use the source-budget collector,
  not only the pistol. (A source comment: not a docs-only change.)

## Files

- `tools/corner_wad.py` writes the 22 scene WADs, their demos and `scenes.json`; `tools/build_probe.py` builds the
  probe variant of the genuine runner; `tools/run_scenes.py` runs the genuine side and checks dump identity;
  `tools/run_rust.py` (wires 22 to 24), `tools/run_w25.py` (wire 25), `tools/run_old_wires.py` (wires 7 to 21);
  `tools/compare_cells.py` compares trace by trace; `tools/direct_list.py` and `tools/compare_direct.py` are the
  exact-corner direct calls; `tools/pt.py` is a Python copy of the C cell stepping used only to search for scenes;
  the `find_*`, `search_*`, `run_find*`, `detail.py` and `probe_origin.py` scripts are those searches.
- `harness/` and `harness-probe/` are small Rust drivers (`cert`, `wires`, `hist`, `hist22`; `direct`, `wires`).
- `probe/corner_probe.rs` and `probe/hooks.diff` are the observation-only Rust trace log and its hooks;
  `probe/mutant.diff` is the unbuilt self-test mutant.
- Genuine builds used: `m4b_build.py --variant strict` and `m5c_build.py --variant strict`, into scratch.
- Paths inside the scripts point at the scratch directory named above.
