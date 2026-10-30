#!/usr/bin/env python3
"""Assert the C-backed grounded player example agrees in debug and release."""
import subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
outputs=[]
for mode in [[],['--release']]:
    outputs.append(subprocess.check_output(['cargo','run','-q','-p','doom-sim','--example','player_knockback',*mode,'--locked'],cwd=root))
assert outputs[0]==outputs[1]
print('player_knockback: debug/release C-backed observations agree')
