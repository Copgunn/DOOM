#!/usr/bin/env python3
"""Compare synthetic example stdout between debug/release on every CI OS."""
import subprocess

for example,args in [('headless',['--','12']),('player_arena',[]),('multisector_arena',[])]:
    base=['cargo','+stable','run','-p','doom-sim','--example',example,'--locked']
    debug=subprocess.check_output(base+args)
    release=subprocess.check_output(base+['--release']+args)
    if debug!=release:
        raise SystemExit('Debug/release headless output differs: '+example)
    print('Identical debug/release synthetic output: '+example)
