#!/usr/bin/env python3
"""Assert new ownership example in both modes against independently C-encoded digests."""
import csv
import subprocess
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
cases = list(csv.DictReader((ROOT / 'tests/reference/m3f8_cases.csv').open()))
rows = list(csv.DictReader((ROOT / 'tests/reference/m3f8_snapshots.csv').open()))
expected = ''
for which, c in enumerate(cases):
    row = next(r for r in rows if int(r['case']) == which and r['operation'] == c['tics'])
    expected += f'context_ownership: {c["label"]} captures={int(c["tics"])+1} {row["fnv1a64"]}\n'
for flags in [[], ['--release']]:
    actual = subprocess.check_output(['cargo', 'run', '-q', '-p', 'doom-sim', '--example', 'context_ownership', '--locked', *flags], cwd=ROOT, text=True)
    assert actual == expected, actual
print('context_ownership:8 new C cases,94 captures; debug/release assertions and independent digests passed')
