#!/usr/bin/env python3
"""Check qualified execution against an unchanged independent C-derived digest."""
import csv
import subprocess
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
with (ROOT / 'tests/reference/m3f8_cases.csv').open() as stream:
    case = next(csv.DictReader(stream))
with (ROOT / 'tests/reference/m3f8_snapshots.csv').open() as stream:
    row = next(r for r in csv.DictReader(stream) if r['case'] == '0' and r['operation'] == case['tics'])
expected = f'game_instance: reused C replace_clear captures={int(case["tics"])+1} {row["fnv1a64"]}\n'
for flags in [[], ['--release']]:
    actual = subprocess.check_output(['cargo', 'run', '-q', '-p', 'doom-sim', '--example', 'game_instance', '--locked', *flags], cwd=ROOT, text=True)
    assert actual == expected, actual
print('game_instance: qualified scopes, cross-world/restart rejection; reused C digest passed debug/release')
