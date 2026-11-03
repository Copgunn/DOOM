#!/usr/bin/env python3
"""Bounded public real-map-scale operation inputs; no Rust-generated expectations."""
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
    diagonal_run=bytes([0,0,50,50,0,0,0,1,0])
    turn_fire=bytes([0,0,0,0,0,64,0,1,1])
    seeds=[]
    for room in range(6):
        for rng in [0,6,8,255]:
            seeds.append(bytes([room,rng]))
            # Long horizons reach pursuit across the town, shots along the gallery
            # and the witness hall (the refused 129th intercept), long sight across
            # the extent room and down the wide corridor, slides into the comb.
            for op in [idle,fire,run,back,strafe,turn_run,diagonal_run,turn_fire]:
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
            # Controlled pushes: within the clamp, the corner-jump magnitude, beyond
            # the momentum admission, a monster and a stale handle, then running on.
            for push in [(24,24),(30,30),(-30,30),(4096,0),(5000,0),(-32768,32767)]:
                lx,hx=(push[0]).to_bytes(2,'little',signed=True);ly,hy=(push[1]).to_bytes(2,'little',signed=True)
                for target in [0,1,2]:
                    seeds.append(bytes([room,rng,8,target,1,lx,hx,ly,0,1,hy])+run*8)
    for i,data in enumerate(seeds):(args.directory/f'real-map-{i:03d}').write_bytes(data)
    print(f'{len(seeds)} real-map-scale operation seeds')
if __name__=='__main__':main()
