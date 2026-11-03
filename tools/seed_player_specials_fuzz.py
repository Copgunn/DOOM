#!/usr/bin/env python3
"""Bounded public sky and static-specials operation inputs; no Rust-generated expectations."""
import argparse
from pathlib import Path

# The 17 committed scenario spawns of fuzz_targets/player_specials.rs: 0-6 pools
# (hall, basins, ledge, below the secret, the fight), 7-16 sky (courts, tower, windows).
SPAWNS = 17


def main():
    parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);args=parser.parse_args()
    args.directory.mkdir(parents=True,exist_ok=True)
    # b0 op, b1 slot, b2 forward, b3 side, b4-5 angle_turn, b6-7 handle, b8 buttons.
    idle=bytes([0,0,0,0,0,0,0,1,0])
    fire=bytes([0,0,0,0,0,0,0,1,1])
    run=bytes([0,0,50,0,0,0,0,1,1])
    back=bytes([0,0,206,0,0,0,0,1,0])
    strafe=bytes([0,0,0,206,0,0,0,1,0])
    turn_run=bytes([0,0,50,0,0,4,0,1,0])
    diagonal_run=bytes([0,0,50,50,0,0,0,1,0])
    turn_fire=bytes([0,0,0,0,0,64,0,1,1])
    north=bytes([0,0,0,0,0,0x40,0,1,0])
    south=bytes([0,0,0,0,0,0xc0,0,1,0])
    walk=bytes([0,0,10,0,0,0,0,1,0])
    seeds=[]
    for room in range(SPAWNS):
        for rng in [0,6,8,255]:
            seeds.append(bytes([room,rng]))
            # Long horizons: idling in the damage basins until the 32-tic gates and
            # death, running and firing under the sky at hack walls and windows.
            for op in [idle,fire,run,back,strafe,turn_run,diagonal_run,turn_fire]:
                seeds.append(bytes([room,rng])+op*32)
            # Turning north or south, then walking or running into the basins (over the
            # ledge, onto the secret) or toward the sky walls, firing on the way.
            for turn in [north,south]:
                for op in [walk,run]:
                    seeds.append(bytes([room,rng])+turn+op*31)
            for damage in [0,1,3,24,60,61,130,1000,1001,-1]:
                for kind in [0,1,2]:
                    low,high=damage.to_bytes(2,'little',signed=True)
                    # b0 op, b1 target kind, b2 monster, b3 source/inflictor, b4 monster, b5 low, b6-7 handle, b8 high.
                    seeds.append(bytes([room,rng,4,kind,1,0,2,low,0,1,high])+run*8)
            # Infighting retarget, stale/foreign handles, owner removal, pause.
            for operation in [bytes([5,0,1,1,0,0,0,1,0]),bytes([5,1,1,0,0,0,0,1,0]),bytes([5,0,2,0,0,0,255,9,0]),
                              bytes([6,0,0,0,0,0,200,1,0]),bytes([6,1,1,0,0,0,0,1,0]),bytes([7,1,0,0,0,0,0,1,0])]:
                seeds.append(bytes([room,rng])+run*16+operation+fire*14)
            # Controlled pushes: within the clamp, the corner-jump magnitude, beyond
            # the momentum admission, a monster and a stale handle, then running on.
            for push in [(24,24),(30,30),(-30,30),(4096,0),(5000,0),(-32768,32767)]:
                lx,hx=(push[0]).to_bytes(2,'little',signed=True);ly,hy=(push[1]).to_bytes(2,'little',signed=True)
                for target in [0,1,2]:
                    seeds.append(bytes([room,rng,8,target,1,lx,hx,ly,0,1,hy])+run*8)
    assert all(len(s)<=290 for s in seeds)
    for i,data in enumerate(seeds):(args.directory/f'specials-{i:04d}').write_bytes(data)
    print(f'{len(seeds)} sky and static-specials operation seeds')
if __name__=='__main__':main()
