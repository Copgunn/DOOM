#!/usr/bin/env python3
"""Assert the full M3h8 movers replay in debug and release and the equality of its output.

The replay (crates/doom-sim/src/movers_replay.rs, module `full`) drives every committed
C scenario through the Rust movers policy and compares, at every op, the C rows (sector
heights and specialdata, line specials, use latch, exit and player state, crossings,
uses, blocked moves, the thinker list with every mover record, the buttonlist) and the
independent DSIM35 goldens (the PBE1 event frame, and at every capture the digest, size,
field count, full bytes of the 13 fixtures and the v35 reader's round trip). It needs
crate-internal declared inputs (the controlled damage transaction), so it runs as a
library test rather than an example."""
import subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
TEST='movers_replay::full::full_replay_matches_the_c_rows_and_the_independent_dsim35_goldens'
def run(release):
    out=subprocess.run(['cargo','test','-q','-p','doom-sim','--lib','--locked',*(['--release']if release else[]),'--',TEST,'--exact','--nocapture'],
                       cwd=root,text=True,capture_output=True,check=True).stdout
    lines=[l for l in out.splitlines()if l.startswith('player_movers full replay:')]
    assert len(lines)==1 and '1 passed' in out,out
    return lines[0]
debug=run(False);release=run(True)
assert debug==release,(debug,release)
print(debug)
print('player_movers debug/release assertions and equality passed')
