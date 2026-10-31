#!/usr/bin/env python3
"""Public original world-pickup timelines and invalid-operation seeds; no assets."""
import sys
from pathlib import Path
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
idle=bytes(10)
attack=bytes([0,0,0,0,0,0,1,0,1,0])
for case in range(51):
    for warm in range(20):
        (out/f'case{case}-warm{warm}').write_bytes(bytes([case,warm])+attack)
for name,request in {
    'switch-fist':bytes([0,0,0,0,0,0,4,0,1,0])+attack*15,
    'switch-shotgun':bytes([0,0,0,0,0,0,20,0,1,0])+attack*15,
    'switch-chaingun':bytes([0,0,0,0,0,0,28,0,1,0])+attack*15,
    'full-held':attack*16,'release':idle*16,'unsupported-buttons':bytes([0,0,0,0,0,0,255,0,1,0]),
    'wrong-slot':bytes([0,3,0,0,0,0,1,0,1,0]),
    'stale-target':bytes([8,2,0,0,0,0,0,1,2,0]),
    'retire-demon':bytes([7,1,0,0,0,0,0,0,1,0])+attack,
    'remove-player':bytes([7,0,0,0,0,0,0,0,1,0]),
    'physical-hit':bytes([6,1,1,0,40,0,0,0,1,0]),
    'player-to-demon':bytes([6,2,2,1,15,0,0,0,1,0]),
    'missing-target':bytes([8,0,0,0,0,0,0,0,1,0])+attack,
}.items(): (out/name).write_bytes(bytes([0,2])+request)
print(f'{len(list(out.iterdir()))} initial world-pickup seeds')

for name,request in {'forward':bytes([0,0,4,0,0,0,0,0,1,0])*16,'retire-item':bytes([7,1,0,0,0,0,0,0,1,1]),'drop-rejection':bytes([17,3,0,0,0,0,0,0,1,0]),'item-as-target':bytes([18,0,0,0,0,0,0,0,1,0]),'retry-map':bytes([16,0,0,0,0,0,0,0,1,0])}.items():
    (out/name).write_bytes(bytes([1,0])+request)
print(f'{len(list(out.iterdir()))} total pickup seeds')
