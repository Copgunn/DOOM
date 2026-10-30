#!/usr/bin/env python3
"""Deterministic public original-ID requests; no Rust-generated expectations."""
import struct
import sys
from pathlib import Path
root=Path(__file__).resolve().parents[1]
destination=Path(sys.argv[1] if len(sys.argv)>1 else root/'fuzz/corpus/original_registry')
destination.mkdir(parents=True,exist_ok=True)
for i,raw in enumerate([0,1,12,13,135,136,137,138,475,476,477,484,485,486,487,488,489,490,495,496,501,967,65535,200]):
    ops=[bytes([kind,(i//6)%5])+struct.pack('<H',raw) for kind in range(6)]
    (destination/f'public_{i:02}').write_bytes(bytes([i%9,64 if i%7 else i%8])+b''.join(ops))
print('24 public original-ID seeds, at most six requests per seed')
