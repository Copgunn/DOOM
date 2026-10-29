#!/usr/bin/env python3
"""Write deterministic synthetic map_extent corpus inputs; no assets or network."""
import argparse
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);args=parser.parse_args()
args.directory.mkdir(parents=True,exist_ok=True)
root=Path(__file__).resolve().parents[1]
operations=bytes([0,0,30,0, 2,0,112,88, 3,0,0,0, 4,0,0,0, 5,1,0,0, 4,0,0,0, 6,0,88,88, 3,1,0,0])
seeds={f'map-{i:02}':bytes([i,0,0])+operations for i in range(len(list((root/'tests/reference').glob('m3f2_map_*.bin'))))}
for index in [20,27]:
    for mutation in range(10):seeds[f'mutate-{index}-{mutation}']=bytes([index,mutation,128])+operations
for index in [7,20,27]:seeds[f'extreme-{index}']=bytes([index,0,0,132,0,0,1,0,0,64,0,4,0,0,0])
seeds['rng-wrap']=bytes([7,0,0])+bytes([8,0,31,0])*8
seeds['source-undefined-clip']=bytes([255,0,0])+bytes([3,0,1,0])*8
for name,data in seeds.items():(args.directory/name).write_bytes(data)
print(f'{len(seeds)} synthetic seeds; max35 bytes/eight operations/eight tics/four actor slots.')
