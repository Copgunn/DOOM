#!/usr/bin/env python3
"""Debug/release example output checked against independent C-derived DSIM."""
import csv,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
cases=list(csv.DictReader((root/'tests/reference/m3f3_cases.csv').open()))
which=next(i for i,c in enumerate(cases) if c['label']=='long_lifecycle')
last=cases[which]['tics']
row=next(r for r in csv.DictReader((root/'tests/reference/m3f3_snapshots.csv').open()) if r['case']==str(which) and r['operation']==last)
expected='demon_expanded_map: 769 lines / 256 sectors; pursuit, melee, pain, corpse; DSIM-v20 '+row['fnv1a64']
for release in [False,True]:
    command=['cargo','run','-p','doom-sim','--example','demon_expanded_map','--locked']+(['--release'] if release else [])
    actual=subprocess.check_output(command,cwd=root,text=True).strip()
    assert actual==expected,(release,actual,expected)
print(expected)
