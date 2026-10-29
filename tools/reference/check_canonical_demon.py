#!/usr/bin/env python3
"""Example expectations come from independent C-derived snapshot indexes."""
import csv
import subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
cases=list(csv.DictReader((root/'tests/reference/m3f5_cases.csv').open()))
rows=list(csv.DictReader((root/'tests/reference/m3f5_snapshots.csv').open()))
expected=''
for which in [1,3]:
    c=cases[which];r=next(r for r in rows if int(r['case'])==which and r['operation']==c['tics'])
    expected+=f'canonical_demon: {c["label"]} MT_SERGEANT=12; 21 bound states; Profile20 {r["fnv1a64"]}\n'
for flags in [[],['--release']]:
    actual=subprocess.check_output(['cargo','run','-q','-p','doom-sim','--example','canonical_demon','--locked',*flags],cwd=root,text=True)
    assert actual==expected,actual
print('canonical_demon: debug/release assertions and independent C-derived output passed')
