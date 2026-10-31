#!/usr/bin/env python3
"""Bounded public bonuses-operation inputs; no Rust-generated expected behavior."""
import argparse
from pathlib import Path

def main():
    parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);args=parser.parse_args()
    args.directory.mkdir(parents=True,exist_ok=True)
    seeds=[]
    for color in range(3):
        for rng in [0,6,8,255]:
            seeds.append(bytes([color,rng]))
            for damage in [0,1,2,5,10,40,60,140,200,1000,1001,-1]:
                for physical in [0,1]:
                    seeds.append(bytes([color,rng,3,0,0,physical])+damage.to_bytes(2,'little',signed=True)+bytes([0,1,0]))
            # Command, invalid slot/buttons, removal, pause and unsupported drop.
            for operation in [bytes([0,0,4,0,0,0,0,1,0]),bytes([0,0,0,0,0,0,0,1,1]),bytes([0,1,51,0,0,0,0,1,2]),
                              bytes([4,1,0,0,0,0,255,255,0]),bytes([5,0,0,0,0,0,0,1,0]),bytes([6,1,0,0,0,0,0,1,0]),bytes([7,0,0,0,0,0,0,1,0])]:
                seeds.append(bytes([color,rng])+operation*4)
            # Full32-operation horizon reaches the close Demon's delayed melee.
            seeds.append(bytes([color,rng])+bytes([0,0,0,0,0,0,0,1,0])*32)
    for i,data in enumerate(seeds):(args.directory/f'bonuses-{i:03d}').write_bytes(data)
    print(f'{len(seeds)} bonuses operation seeds')
if __name__=='__main__':main()
