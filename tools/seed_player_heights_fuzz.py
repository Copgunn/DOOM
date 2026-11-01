#!/usr/bin/env python3
"""Bounded public static-heights operation inputs; no Rust-generated expectations."""
import argparse
from pathlib import Path

def main():
    parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);args=parser.parse_args()
    args.directory.mkdir(parents=True,exist_ok=True)
    idle=bytes([0,0,0,0,0,0,0,1,0])
    fire=bytes([0,0,0,0,0,0,0,1,1])
    run=bytes([0,0,50,0,0,0,0,1,1])
    back=bytes([0,0,206,0,0,0,0,1,0])
    strafe=bytes([0,0,0,206,0,0,0,1,0])
    turn_run=bytes([0,0,50,0,0,4,0,1,0])
    seeds=[]
    for room in range(4):
        for rng in [0,6,8,255]:
            seeds.append(bytes([room,rng]))
            # Long horizons reach steps, falls, landings, ledge refusals, window
            # sight, fence slides, launches and kills.
            for op in [idle,fire,run,back,strafe,turn_run]:
                seeds.append(bytes([room,rng])+op*32)
            for damage in [0,1,3,24,60,61,130,1000,1001,-1]:
                for kind in [0,1,2]:
                    low,high=damage.to_bytes(2,'little',signed=True)
                    # b0 op, b1 target kind, b2 monster, b3 source/inflictor, b4 monster, b5 low, b6-7 handle, b8 high.
                    seeds.append(bytes([room,rng,4,kind,1,0,2,low,0,1,high])+run*8)
            # Infighting retarget, stale/foreign handles, owner removal, pause.
            for operation in [bytes([5,0,1,1,0,0,0,1,0]),bytes([5,1,1,0,0,0,0,1,0]),bytes([5,0,2,0,0,0,255,9,0]),
                              bytes([6,0,0,0,0,0,200,1,0]),bytes([6,1,1,0,0,0,0,1,0]),bytes([7,1,0,0,0,0,0,1,0])]:
                seeds.append(bytes([room,rng])+run*16+operation+fire*16)
    for i,data in enumerate(seeds):(args.directory/f'heights-{i:03d}').write_bytes(data)
    print(f'{len(seeds)} static-heights operation seeds')
if __name__=='__main__':main()
