#!/usr/bin/env python3
"""Original-ID example checked against independently C-encoded DSIM-v21."""
import csv
import subprocess
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
cases = list(csv.DictReader((ROOT / 'tests/reference/m3f5_cases.csv').open()))
rows = list(csv.DictReader((ROOT / 'tests/reference/m3f7_snapshots.csv').open()))
expected = ''
for which in [1, 3]:
    c = cases[which]
    row = next(r for r in rows if int(r['case']) == which and r['operation'] == c['tics'])
    expected += f'original_registry: {c["label"]} actual MT_SERGEANT12; DSIM21/profile1 {row["fnv1a64"]}\n'
for flags in [[], ['--release']]:
    actual = subprocess.check_output(['cargo', 'run', '-q', '-p', 'doom-sim', '--example', 'original_registry', '--locked', *flags], cwd=ROOT, text=True)
    assert actual == expected, actual
print('original_registry: debug/release assertions and independent reused-C digests passed')
