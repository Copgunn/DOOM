#!/usr/bin/env python3
"""Reproduce every source/independent-encoder pair without changing fixtures."""
from pathlib import Path
import subprocess, sys
root=Path(__file__).resolve().parents[2]
versions=['','m3a','m3b','m3c','m3d1','m3d2','m3d2b','m3d3a','m3d3b','m3d3c','m3d3d','m3d3e','m3d3f','m3d3g','m3d3h','m3d3i','m3d3j']
for version in versions:
    generator='generate'+('_'+version if version else '')
    encoder=(version+'_' if version else '')+'snapshot_goldens'
    for tool in [generator,encoder]:
        subprocess.run([sys.executable,str(root/'tools/reference'/f'{tool}.py'),'--check'],cwd=root,check=True)
print(f'Checked all {len(versions)} original-C / independent-encoder pairs.')
