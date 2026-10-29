#!/usr/bin/env python3
"""Asserted synthetic example equality; all previous helpers stay frozen."""
import os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
outputs=[]
for release in [False,True]:
    command=['cargo','run','-p','doom-sim','--example','map_scale','--locked']+(['--release'] if release else [])
    outputs.append(subprocess.check_output(command,cwd=root,env=os.environ))
assert outputs[0]==outputs[1],(outputs[0],outputs[1])
print(outputs[0].decode().strip())
