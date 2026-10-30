#!/usr/bin/env python3
"""Bounded scripted input seeds; no expected Rust gameplay outputs."""
import argparse,csv
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('destination',type=Path);a=p.parse_args();a.destination.mkdir(parents=True,exist_ok=True)
cases=list(csv.DictReader((Path(__file__).resolve().parents[1]/'tests/reference/m3g4_cases.csv').open()))
def command(f=0,s=0,t=0,slot=0,buttons=0):return bytes([0,slot,f&255,s&255])+t.to_bytes(2,'little',signed=True)+bytes([buttons,0])
for i,c in enumerate(cases):
 (a.destination/f'case-{i:02}').write_bytes(bytes([i])+command(int(c['forward']),int(c['side']),int(c['turn']))*12)
for i,(f,s,t,slot,buttons) in enumerate([(51,0,0,0,0),(-128,0,0,0,0),(0,0,0,3,1),(50,-50,-32768,0,0),(0,0,32767,0,255)]):
 (a.destination/f'boundary-{i}').write_bytes(bytes([0])+command(f,s,t,slot,buttons)*4)
(a.destination/'all-operations').write_bytes(bytes([0])+b''.join(bytes([k,0,1,1,1,0,0,0]) for k in range(8)))
print('player_commands:28 seeds;129-byte input budget; includes source inputs and policy boundaries')
