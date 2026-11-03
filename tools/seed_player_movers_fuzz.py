#!/usr/bin/env python3
"""Bounded public doors, lifts, floors, switches and exit operation inputs; no Rust-generated expectations."""
import argparse
from pathlib import Path

# The 27 distinct committed scenario starts of fuzz_targets/player_movers.rs: 0-5
# doorway and hall, 6-12 switchyard (switches, exit, plain wall, back side), 13-18
# lift, 19-24 floors (turbo, lowest, blazeOpen), 25-26 dense.
SPAWNS = 27


def main():
    parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);args=parser.parse_args()
    args.directory.mkdir(parents=True,exist_ok=True)
    # b0 op, b1 slot, b2 forward, b3 side, b4-5 angle_turn, b6-7 handle, b8 buttons.
    idle=bytes([0,0,0,0,0,0,0,1,0])
    use=bytes([0,0,0,0,0,0,0,1,2])
    fire=bytes([0,0,0,0,0,0,0,1,1])
    run=bytes([0,0,50,0,0,0,0,1,0])
    run_use=bytes([0,0,50,0,0,0,0,1,2])
    walk=bytes([0,0,25,0,0,0,0,1,0])
    back=bytes([0,0,206,0,0,0,0,1,0])
    strafe=bytes([0,0,0,206,0,0,0,1,0])
    turn_use=bytes([0,0,0,0,0,4,0,1,2])
    turns=[bytes([0,0,0,0,0,a,0,1,0]) for a in (0x40,0x80,0xc0)]
    seeds=[]
    for room in range(SPAWNS):
        for rng in [0,6,8,255]:
            seeds.append(bytes([room,rng]))
            # Long horizons: waiting doors closing, lifts returning, monsters
            # pressing against doors, walking over W1/WR lines, pressing use.
            for op in [idle,use,fire,run,run_use,walk,back,strafe,turn_use]:
                seeds.append(bytes([room,rng])+op*32)
            # Use, release, use again (latch, reversal, buttons), then wait.
            seeds.append(bytes([room,rng])+use+idle+use+idle*8+use+idle*20)
            # Turn to each direction, then walk and use (switches, doors, lifts).
            for turn in turns:
                for op in [walk,run_use]:
                    seeds.append(bytes([room,rng])+turn+op*16+use+idle*14)
            # Controlled damage (the player, a monster, a stale handle), then use
            # while possibly dead (the reborn request) and keep commanding.
            for damage in [1,60,100,200,1000,-1]:
                low,high=damage.to_bytes(2,'little',signed=True)
                for kind in [0,1,2]:
                    seeds.append(bytes([room,rng,4,kind,1,0,2,low,0,1,high])+use+idle+use+run*8)
            # Infighting retarget, removal, pause, pushes into door sectors and lifts.
            for operation in [bytes([5,0,1,1,0,0,0,1,0]),bytes([6,1,1,0,0,0,0,1,0]),bytes([7,1,0,0,0,0,0,1,0]),
                              bytes([8,0,1,24,0,0,0,1,0]),bytes([8,0,1,0,0,24,0,1,0]),bytes([8,1,1,30,0,30,0,1,0])]:
                seeds.append(bytes([room,rng])+walk*8+operation+use+walk*12)
    assert all(len(s)<=290 for s in seeds)
    for i,data in enumerate(seeds):(args.directory/f'movers-{i:04d}').write_bytes(data)
    print(f'{len(seeds)} doors, lifts, floors, switches and exit operation seeds')
if __name__=='__main__':main()
