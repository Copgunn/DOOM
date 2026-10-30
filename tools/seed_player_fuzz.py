#!/usr/bin/env python3
"""Bounded player binding/damage/retirement inputs, never expected outputs."""
import argparse
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('destination',type=Path);a=p.parse_args();a.destination.mkdir(parents=True,exist_ok=True)
spawn=bytes([0,0,0,0,0,0,0,0]);tick=bytes([7,0,0,1,0,0,0,0])
for damage in [0,1,20,99,100,150,200,201,255]:
    hit=bytes([9,0,0,1,damage,0,0,0]);(a.destination/f'damage-{damage:03}').write_bytes(bytes([0])+spawn+hit+tick)
(a.destination/'reuse').write_bytes(bytes([0])+spawn+bytes([3,0,0,1,0,0,0,0])+tick+bytes([4,0,0,0,0,0,0,0])+bytes([1,1,0,1,0,0,0,0]))
(a.destination/'all-operations').write_bytes(bytes([1])+b''.join(bytes([kind,1,0,1,0,0,0,0]) for kind in range(10)))
print('original_player: 11 bounded seeds')
