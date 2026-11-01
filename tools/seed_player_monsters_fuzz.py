#!/usr/bin/env python3
"""Bounded public multi-monster operation inputs; no Rust-generated expectations."""
import argparse
from pathlib import Path

def main():
    parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);args=parser.parse_args()
    args.directory.mkdir(parents=True,exist_ok=True)
    idle=bytes([0,0,0,0,0,0,0,1,0])
    fire=bytes([0,0,0,0,0,0,0,1,1])
    west=bytes([0,0,0,0,0,128,0,1,1])
    seeds=[]
    for color in range(3):
        for rng in [0,6,8,255]:
            seeds.append(bytes([color,rng]))
            # Long horizons reach wake-up, attacks, infighting, kills and drops.
            for op in [idle,fire,west]:
                seeds.append(bytes([color,rng])+op*32)
            for damage in [0,1,3,20,30,70,150,1000,1001,-1]:
                for kind in [0,1,2]:
                    seeds.append(bytes([color,rng,4,kind,1,0,2])+damage.to_bytes(2,'little',signed=True)[:1]+bytes([1])+damage.to_bytes(2,'little',signed=True)[1:]+idle*8)
            # Infighting retarget, stale/foreign handles, removal, pause.
            for operation in [bytes([5,0,1,1,0,0,0,1,0]),bytes([5,1,1,0,0,0,0,1,0]),bytes([5,0,2,0,0,0,63,9,0]),
                              bytes([6,0,0,0,0,0,40,1,0]),bytes([6,1,1,0,0,0,0,1,0]),bytes([7,1,0,0,0,0,0,1,0])]:
                seeds.append(bytes([color,rng])+operation+idle*16)
    for i,data in enumerate(seeds):(args.directory/f'monsters-{i:03d}').write_bytes(data)
    print(f'{len(seeds)} monster operation seeds')
if __name__=='__main__':main()
