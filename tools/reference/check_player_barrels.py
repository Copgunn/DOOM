#!/usr/bin/env python3
"""Assert the M3h9b barrels-policy replay and example in debug and release and the equality of their output.

The replay (crates/doom-sim/src/barrels_replay.rs) drives the 26 committed
medium-skill C scenarios through the Rust barrels policy and compares, at every op,
every C row kind the M3h9 replay compares (sector heights and specialdata, line
specials, use latch, exit and player state, crossings, uses, use intercepts, blocked
moves, the thinker list), the barrel rows (spawns and the ident map, every state
entry and action with A_Explode, the RA row of each explosion with its 3 x 3
window, every RC candidate in walk order with its block, ordinal, outcome and the
damage row of a code-4 candidate, the contract's death chains) and the independent
DSIM37 goldens (the PBE1 event frame, and at every capture the digest, size, field
count, full bytes of the 16 fixtures and the v37 reader's round trip, with the v36
reader rejecting it). Two Rust-only tests replay the lanes the C harness never
runs: the embedded barrel refused by S' at spawn and the intercept lane whose shot
tic is refused with the intercept witness. The replay needs crate-internal declared
inputs (the controlled damage of hit_* and the 200 health of script 30) and a
test-only walk observer, so it runs as library tests rather than an example.

The asserted example `player_barrels` replays, through the public API alone, the
scenarios whose declared inputs are public (spawns, state entries, actions,
explosion sight and damage, heights, work and every capture digest against the C
rows and the independent index) in the synthetic M3h9b rooms (not the real E1M1)
and asserts the C rows' headline counts; its output must be equal in debug and
release. `--example-only` checks the example alone (the CI step of the hosts that
do not run the replay)."""
import subprocess
import sys
from pathlib import Path
root=Path(__file__).resolve().parents[2]
TESTS=['barrels_replay::full_replay_matches_the_c_rows_and_the_independent_dsim37_goldens',
       'barrels_replay::an_embedded_barrel_is_refused_at_spawn_by_s_prime',
       'barrels_replay::the_intercept_lane_refuses_the_shot_with_the_intercept_witness']
def run(release):
    out=subprocess.run(['cargo','test','-q','-p','doom-sim','--lib','--locked',*(['--release']if release else[]),'--',*TESTS,'--exact','--nocapture'],
                       cwd=root,text=True,capture_output=True,check=True).stdout
    # libtest's quiet progress dots may precede the summary on its line.
    lines=[l[l.index('player_barrels full replay:'):]for l in out.splitlines()if'player_barrels full replay:'in l]
    assert len(lines)==1 and f'{len(TESTS)} passed' in out,out
    return lines[0]
def example(release):
    return subprocess.check_output(['cargo','run','-q','-p','doom-sim','--example','player_barrels','--locked',*(['--release']if release else[])],cwd=root,text=True)
if '--example-only' not in sys.argv[1:]:
    debug=run(False);release=run(True)
    assert debug==release,(debug,release)
    print(debug)
debug=example(False);release=example(True)
assert debug==release,(debug,release)
print(debug.strip())
print('player_barrels debug/release assertions and equality passed')
