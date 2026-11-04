#!/usr/bin/env python3
"""Bounded public two-sided scroller, decoration and spawn-filter operation inputs; no Rust-generated expectations."""
import argparse
from pathlib import Path

# The 18 distinct replayed M3h9 scenario starts of fuzz_targets/player_e1m1.rs: 0-4
# scrollhall (crossing, the ledge Zombieman, the use through the scroller, the shot,
# the Imp in the strip), 5 scrollblock, 6 dense8, 7-14 gallery (the lamps, the gore,
# the pillar Zombieman, the alcove Zombieman, the two Imps, the pistol lane), 15-16
# liftroom (riders on the lift, the pillar under the door), 17 census.
SPAWNS = 18


def main():
    parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);args=parser.parse_args()
    args.directory.mkdir(parents=True,exist_ok=True)
    # b0 op, b1 slot or selector, b2 forward, b3 side, b4-5 angle_turn, b6-7 handle,
    # b8 buttons.
    idle=bytes([0,0,0,0,0,0,0,1,0])
    use=bytes([0,0,0,0,0,0,0,1,2])
    fire=bytes([0,0,0,0,0,0,0,1,1])
    run=bytes([0,0,50,0,0,0,0,1,0])
    run_fire=bytes([0,0,50,0,0,0,0,1,1])
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
            # Long horizons: walking into lamps and over gore, crossing scrollers,
            # the lift carrying its riders, the door closing on the pillar, shots
            # through decorations.
            for op in [idle,use,fire,run,run_fire,run_use,walk,back,strafe,turn_use]:
                seeds.append(bytes([room,rng])+op*32)
            seeds.append(bytes([room,rng])+use+idle+use+idle*8+use+idle*20)
            for turn in turns:
                for op in [walk,run_use,run_fire]:
                    seeds.append(bytes([room,rng])+turn+op*16+use+idle*14)
            # Controlled damage of the player, a monster, a decoration (refused) and a
            # stale handle, then use while possibly dead and keep commanding.
            for damage in [1,60,200,1000,-1]:
                low,high=damage.to_bytes(2,'little',signed=True)
                for kind in [0,1,2,3]:
                    seeds.append(bytes([room,rng,4,kind,1,0,2,low,0,1,high])+use+idle+use+run*8)
            # Retarget (a monster at the player, at a decoration, a decoration at the
            # player: refused), removal (a monster, a decoration: refused, a stale
            # handle), pause, pushes (the body, a monster, a decoration: refused, a
            # zero push on a decoration: accepted).
            for operation in [bytes([5,0,0,0,0,0,0,1,0]),bytes([5,1,2,0,0,0,0,1,0]),bytes([5,0x80,0,0,0,0,0,1,0]),bytes([5,0x81,1,1,0,0,0,1,0]),
                              bytes([6,1,0,0,0,0,0,1,0]),bytes([6,2,0,0,0,0,0,1,0]),bytes([6,2,3,0,0,0,0,1,0]),bytes([6,0,0,0,0,0,9,9,0]),
                              bytes([7,1,0,0,0,0,0,1,0]),
                              bytes([8,0,0,24,0,0,0,1,0]),bytes([8,1,1,0,0,24,0,1,0]),bytes([8,2,0,30,0,30,0,1,0]),
                              bytes([8,2,1,1,0,0,0,1,0]),bytes([8,2,0,0,0,0,0,1,0]),bytes([8,3,0,24,0,0,0,1,0])]:
                seeds.append(bytes([room,rng])+walk*8+operation+use+walk*12)
    assert all(len(s)<=290 for s in seeds)
    for i,data in enumerate(seeds):(args.directory/f'e1m1-{i:04d}').write_bytes(data)
    print(f'{len(seeds)} two-sided scroller, decoration and spawn-filter operation seeds')
if __name__=='__main__':main()
