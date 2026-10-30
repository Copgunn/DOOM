#!/usr/bin/env python3
"""Bounded ownership/physical damage seeds; no expected gameplay output."""
import argparse
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('destination',type=Path);a=p.parse_args();a.destination.mkdir(parents=True,exist_ok=True)
tick=bytes([7,0,0,1,0,0,0,0,0,0])
def hit(amount):return bytes([2,0,0,1,2,0,1,2])+amount.to_bytes(2,'little',signed=True)
for which in range(17):
    (a.destination/f'case-{which:02}').write_bytes(bytes([which])+hit([0,1,20,100,144,1000][which%6])+tick*4)
spawn=bytes([0,0,0,1,0,0,0,0,0,0])
(a.destination/'binding').write_bytes(bytes([128])+spawn+bytes([1,0,0,1,0,0,0,0,0,0])+tick)
(a.destination/'raw-bind').write_bytes(bytes([128])+bytes([4,0,0,1,1,0,0,0,0,0])+bytes([1,2,0,1,0,0,0,0,0,0])+tick)
(a.destination/'live-origin').write_bytes(bytes([192])+spawn+hit(20)+bytes([10,1,0,1,0,0,0,0,150,0])+hit(20)+tick)
(a.destination/'all-operations').write_bytes(bytes([0])+b''.join(bytes([kind,0,0,1,0,0,0,0,0,0]) for kind in range(12)))
print('player_knockback:21 bounded seeds,129-byte campaign budget')
