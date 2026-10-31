#!/usr/bin/env python3
"""Codec-policy seeds, not independently derived gameplay expectations."""
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
ROOT=Path(__file__).resolve().parents[1]
out=Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
subprocess.run(['cargo','build','-q','-p','doom-test-evidence','--bin','evidence','--locked'],cwd=ROOT,check=True)
with tempfile.TemporaryDirectory(prefix='doom-evidence-seeds-')as directory:
    raw=Path(directory)/'sample.csv';packed=Path(directory)/'sample.zst'
    raw.write_bytes(b'kind,tic,value\nR,1,42\n'*128)
    subprocess.run([str(ROOT/'target/debug/evidence'),'encode',str(raw),str(packed)],check=True)
    frame=struct.pack('<I',raw.stat().st_size)+packed.read_bytes()
    seeds={'valid_frame':frame,'empty':b'','short_header':frame[:8],'truncated':frame[:-1],
           'trailing':frame+b'junk','two_frames':frame+packed.read_bytes(),
           'oversized':struct.pack('<I',0xffffffff)+frame[4:],
           'manifest':(ROOT/'tests/reference/zstd_manifest.json').read_bytes(),
           'invalid_manifest':b'{"entries":[{"original_path":"../escape.csv"}]}'}
    for name,data in seeds.items():(out/name).write_bytes(data)
print(f'Wrote {len(seeds)} codec policy seeds')
