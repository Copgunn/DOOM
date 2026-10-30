#!/usr/bin/env python3
"""Bounded source inputs and roster/target policy requests, never gameplay outputs."""
import argparse,csv
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('destination',type=Path);a=p.parse_args();a.destination.mkdir(parents=True,exist_ok=True)
root=Path(__file__).resolve().parents[1]
cases=list(csv.DictReader((root/'tests/reference/m3g5_cases.csv').open()))
def command(f=0,s=0,t=0,slot=0,buttons=0):return bytes([0,slot,f&255,s&255])+t.to_bytes(2,'little',signed=True)+bytes([buttons,0,1,0])
for i,c in enumerate(cases):(a.destination/f'case-{i:02}').write_bytes(bytes([i])+command(int(c['forward']),int(c['side']),int(c['turn']))*12)
for op in range(12):(a.destination/f'operation-{op:02}').write_bytes(bytes([1])+bytes([op,1,0,0,0,0,0,0,1,0])*12)
for i,(f,s,t,slot,buttons)in enumerate([(51,0,0,0,0),(-128,0,0,0,0),(0,0,0,3,1),(50,-50,-32768,0,0),(0,0,32767,0,255)]):(a.destination/f'boundary-{i}').write_bytes(bytes([1])+command(f,s,t,slot,buttons)*4)
(a.destination/'retire-reuse').write_bytes(bytes([1])+bytes([5,1,0,0,0,0,0,0,1,0])+command()+bytes([11,0,0,0,0,0,0,0,1,0]))
print('player_demon:43 seeds;161-byte budget;source commands plus roster/lifetime/diagnostic policy')
