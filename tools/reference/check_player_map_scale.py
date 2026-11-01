#!/usr/bin/env python3
"""Assert the genuine real-map-scale example and debug/release output equality."""
import subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
def run(release):
    return subprocess.check_output(['cargo','run','-q','-p','doom-sim','--example','player_map_scale','--locked',*(['--release']if release else[])],cwd=root,text=True)
assert run(False)==run(True)
print('player_map_scale debug/release assertions and equality passed')
