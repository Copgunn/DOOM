#!/usr/bin/env python3
"""Assert the new C-backed example has identical debug/release observations."""
import subprocess
outputs=[]
for mode in [[],['--release']]:
    p=subprocess.run(['cargo','run','-p','doom-sim','--example','original_player','--locked',*mode],check=True,capture_output=True,text=True)
    outputs.append(p.stdout)
assert outputs[0]==outputs[1]
assert outputs[0].startswith('original_player: C captures=185 ordered_events=4256 ')
print('original_player: debug/release C-backed observations agree')
