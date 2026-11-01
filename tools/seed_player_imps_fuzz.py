#!/usr/bin/env python3
"""Bounded public Imp/fireball operation inputs; no Rust-generated expectations."""
import argparse
from pathlib import Path

def main():
    parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);args=parser.parse_args()
    args.directory.mkdir(parents=True,exist_ok=True)
    idle=bytes([0,0,0,0,0,0,0,1,0])
    fire=bytes([0,0,0,0,0,0,0,1,1])
    west=bytes([0,0,0,0,0,128,0,1,1])
    dodge=bytes([0,0,0,40,0,0,0,1,0])
    seeds=[]
    for color in range(3):
        for rng in [0,6,8,255]:
            seeds.append(bytes([color,rng]))
            # Long horizons reach wake-up, launches, contact, explosions and kills.
            for op in [idle,fire,west,dodge]:
                seeds.append(bytes([color,rng])+op*32)
            for damage in [0,1,3,24,60,61,130,1000,1001,-1]:
                for kind in [0,1,2]:
                    low,high=damage.to_bytes(2,'little',signed=True)
                    # b0 op, b1 target kind, b2 monster, b3 source/inflictor, b4 monster, b5 low, b6-7 handle, b8 high.
                    seeds.append(bytes([color,rng,4,kind,1,0,2,low,0,1,high])+idle*8)
            # Infighting retarget, stale/foreign handles, owner removal, pause.
            for operation in [bytes([5,0,1,1,0,0,0,1,0]),bytes([5,1,1,0,0,0,0,1,0]),bytes([5,0,2,0,0,0,63,9,0]),
                              bytes([6,0,0,0,0,0,40,1,0]),bytes([6,1,1,0,0,0,0,1,0]),bytes([7,1,0,0,0,0,0,1,0])]:
                seeds.append(bytes([color,rng])+idle*16+operation+idle*16)
    for i,data in enumerate(seeds):(args.directory/f'imps-{i:03d}').write_bytes(data)
    print(f'{len(seeds)} Imp operation seeds')
if __name__=='__main__':main()
