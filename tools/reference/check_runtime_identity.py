#!/usr/bin/env python3
"""Native boundary example against existing independent C-derived digests."""
import csv
import subprocess
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
cases = list(csv.DictReader((ROOT / 'tests/reference/m3f5_cases.csv').open()))
rows = list(csv.DictReader((ROOT / 'tests/reference/m3f5_snapshots.csv').open()))
expected = ''
for which in [1, 3]:
    case = cases[which]
    row = next(r for r in rows if int(r['case']) == which and r['operation'] == case['tics'])
    expected += f'runtime_identity: {case["label"]} native136 -> MT_SERGEANT12; DSIM20 {row["fnv1a64"]}\n'
for flags in [[], ['--release']]:
    actual = subprocess.check_output(['cargo', 'run', '-q', '-p', 'doom-sim', '--example', 'runtime_identity', '--locked', *flags], cwd=ROOT, text=True)
    assert actual == expected, actual
print('runtime_identity: debug/release assertions and existing C-derived output passed')
