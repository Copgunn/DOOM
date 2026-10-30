#!/usr/bin/env python3
"""Public bounded requests; no serialized instance-key format is implied."""
import sys
from pathlib import Path
destination = Path(sys.argv[1]) if len(sys.argv) > 1 else Path('fuzz/corpus/game_instance')
destination.mkdir(parents=True, exist_ok=True)
def request(op, slot=0, key=0, flags=0):
    value = bytearray(16)
    value[0], value[1], value[14], value[15] = flags, slot, key, op
    return bytes(value)
seeds = {f'op_{op}': request(op) for op in range(10)}
seeds.update(foreign=request(0, key=1), missing=request(0, key=2), invalid_generation=request(0, flags=64), out_of_range=request(0, flags=128), wrong_definition=request(0, flags=32), clone_then_stale=request(4, flags=16)+request(0, key=3), restart_then_stale=request(4)+request(0, key=3), retire_reuse=request(2)+request(9)+request(1, slot=1)+request(3)+request(5)+request(0, slot=1))
for name, value in sorted(seeds.items()):
    (destination / name).write_bytes(value)
print(f'Wrote {len(seeds)} public game-instance seeds to {destination}')
