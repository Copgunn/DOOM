#!/usr/bin/env python3
"""Public C-derived framing, schemas and malformed bounded diagnostic seeds."""
import sys
from pathlib import Path
root=Path(__file__).resolve().parents[1]
destination=Path(sys.argv[1] if len(sys.argv)>1 else root/'fuzz/corpus/diagnostic_format')
destination.mkdir(parents=True,exist_ok=True)
seeds={'original_spawn':(root/'tests/reference/m3f7_state_0_0.bin').read_bytes(),
       'original_final':(root/'tests/reference/m3f7_state_0_80.bin').read_bytes(),
       'legacy_spawn':(root/'tests/reference/m3f5_state_0_0.bin').read_bytes(),
       'monsters_spawn':(root/'tests/reference/m3h1_state_0_0.bin').read_bytes(),
       'monsters_infighting':(root/'tests/reference/m3h1_state_7_240.bin').read_bytes(),
       'monsters_version':b'DSIM'+(28).to_bytes(4,'little')+bytes(8),
       'empty':b'', 'magic':b'DSIM', 'length':b'DSIM'+(21).to_bytes(4,'little')+b'\xff'*8,
       'version':b'DSIM'+(22).to_bytes(4,'little')+bytes(8),
       'mutation_count':b'\x08\0\xff', 'mutation_utf8':b'\x14\0\xff',
       'mutation_profile':b'\x50\0\x14', 'mutation_state':b'\x80\x01\xff',
       'truncated':b'DSIM'+(21).to_bytes(4,'little')}
for name,data in seeds.items(): (destination/name).write_bytes(data)
print(f'{len(seeds)} public diagnostic seeds; valid independent v20/v21/v28 plus malformed framing')
