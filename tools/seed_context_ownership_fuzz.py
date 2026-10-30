#!/usr/bin/env python3
"""Bounded local ownership seeds; generated corpus remains ignored."""
import sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
directory=Path(sys.argv[1]) if len(sys.argv)>1 else ROOT/'fuzz/corpus/context_ownership'
directory.mkdir(parents=True,exist_ok=True)
def op(kind,slot=0,target=1,generation=1,flags=0):
    data=bytearray(16);data[0]=flags;data[1]=slot;data[3]=generation-1;data[14]=target;data[15]=kind
    return bytes(data)
seeds=[op(0),op(1),op(0,target=2)+op(1),op(2,slot=4)+op(3)+op(6)+op(7,slot=4),op(2,slot=1)+op(3)+op(1)+op(8)+op(3),op(4),bytes([255])*16,op(5),op(7),op(8,target=5)]
for i,data in enumerate(seeds): (directory/f'seed-{i:02d}').write_bytes(data)
print(f'Wrote {len(seeds)} bounded ownership seeds')
