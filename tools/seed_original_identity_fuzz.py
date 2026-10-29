#!/usr/bin/env python3
"""Synthetic identity boundaries; no game assets."""
import argparse
import struct
from pathlib import Path
parser = argparse.ArgumentParser()
parser.add_argument('directory', type=Path)
args = parser.parse_args()
args.directory.mkdir(parents=True, exist_ok=True)
seeds = {f'ordinal-{n}': struct.pack('<I', n) for n in [0, 1, 12, 13, 136, 137, 138, 966, 967, 65535, 65536, 2147483647, 2147483648, 4294967295]}
for name in ['MT_SERGEANT', 'MT_SHADOWS', 'MT_DEMON', 'S_NULL', 'S_SARG_RUN1', 'SPR_SARG', 'NUMSTATES']:
    seeds[name] = b'\0' * 4 + name.encode()
seeds['invalid-utf8'] = b'\0' * 4 + b'\xff'
for name, data in seeds.items():
    (args.directory / name).write_bytes(data)
print(f'{len(seeds)} synthetic seeds; identity target reads at most 68 bytes.')
