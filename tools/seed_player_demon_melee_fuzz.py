#!/usr/bin/env python3
"""Bounded public combat-timeline and invalid-operation seeds; no private assets."""
import sys
from pathlib import Path

dest=Path(sys.argv[1]);dest.mkdir(parents=True,exist_ok=True)
zero=bytes(10)
SARG_ATTACK_STATE=487  # Original info.h S_SARG_ATK3, not a native fixture state.
for case in range(17):
    for warm in [0,3,5,12,13]:
        (dest/f'case{case}-warm{warm}').write_bytes(bytes([case,warm])+zero)
(dest/'full-input').write_bytes(bytes([1,3])+zero*16)
for name,request in {
    'command':bytes([0,0,8,0,0,0,0,0,1,0]),
    'wrong-slot':bytes([0,3,1,0,0,0,0,0,1,0]),
    'buttons':bytes([0,0,0,0,0,0,1,0,1,0]),
    'stale-target':bytes([8,2,0,0,0,0,0,1,2,0]),
    'retire-source':bytes([7,1,0,0,0,0,0,0,1,0])+zero,
    'remove-player':bytes([7,0,0,0,0,0,0,0,1,0]),
    'damage':bytes([6,3,0,0,40,0,0,0,1,0]),
    'attack-state':bytes([14,0,0,0,*SARG_ATTACK_STATE.to_bytes(2,'little'),0,0,1,0]),
    'missing-target':bytes([8,0,0,0,0,0,0,0,1,0])+zero,
}.items():
    (dest/name).write_bytes(bytes([1,3])+request)
print(f'{len(list(dest.iterdir()))} initial melee seeds')
