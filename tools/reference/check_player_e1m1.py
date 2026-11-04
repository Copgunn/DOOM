#!/usr/bin/env python3
"""Assert the M3h9 E1M1-policy replay and example in debug and release and the equality of their output.

The replay (crates/doom-sim/src/e1m1_replay.rs) drives the 20 committed medium-skill
C scenarios through the Rust policy and compares, at every op, the C rows (sector
heights and specialdata, line specials, use latch, exit and player state, crossings,
uses, use intercepts, blocked moves, the thinker list) and the independent DSIM36
goldens (the PBE1 event frame, and at every capture the digest, size, field count,
full bytes of the 15 fixtures and the v36 reader's round trip); a second, Rust-only
test asserts that the closed-box spawn rule still refuses the alcove room rebuilt in
memory with the Zombieman's box touching its walls; two more assert the review
scenarios (the fireball's crossing of the blocking scroller, and the pillar whose
floor stays above its ceiling while the door is shut on it). The replay needs crate-internal
declared inputs (the controlled 200 health of script 30), so it runs as library tests
rather than an example.

The asserted example `player_e1m1` replays, through the public API alone, the
scenarios whose declared inputs are public (heights, mover work, line specials and
every capture digest against the C rows and the independent index) in the synthetic
M3h9 rooms (not the real E1M1) and asserts the C rows' headline counts; its output
must be equal in debug and release. `--example-only` checks the example
alone (the CI step of the hosts that do not run the replay)."""
import subprocess
import sys
from pathlib import Path
root=Path(__file__).resolve().parents[2]
TESTS=['e1m1_replay::full_replay_matches_the_c_rows_and_the_independent_dsim36_goldens',
       'e1m1_replay::a_zombieman_box_touching_the_alcove_walls_is_refused_at_spawn',
       'e1m1_replay::a_fireball_crosses_a_blocking_two_sided_scroller',
       'e1m1_replay::a_door_shut_on_a_straddling_pillar_leaves_its_floor_above_its_ceiling']
def run(release):
    out=subprocess.run(['cargo','test','-q','-p','doom-sim','--lib','--locked',*(['--release']if release else[]),'--',*TESTS,'--exact','--nocapture'],
                       cwd=root,text=True,capture_output=True,check=True).stdout
    # libtest's quiet progress dots may precede the summary on its line.
    lines=[l[l.index('player_e1m1 full replay:'):]for l in out.splitlines()if'player_e1m1 full replay:'in l]
    assert len(lines)==1 and '4 passed' in out,out
    return lines[0]
def example(release):
    return subprocess.check_output(['cargo','run','-q','-p','doom-sim','--example','player_e1m1','--locked',*(['--release']if release else[])],cwd=root,text=True)
if '--example-only' not in sys.argv[1:]:
    debug=run(False);release=run(True)
    assert debug==release,(debug,release)
    print(debug)
debug=example(False);release=example(True)
assert debug==release,(debug,release)
print(debug.strip())
print('player_e1m1 debug/release assertions and equality passed')
