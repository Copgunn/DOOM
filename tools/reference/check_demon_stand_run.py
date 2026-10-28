#!/usr/bin/env python3
"""Fourteenth asserted example; historical helpers stay unchanged."""
import subprocess
base=['cargo','+stable','run','-p','doom-sim','--example','demon_stand_run','--locked']
assert subprocess.check_output(base)==subprocess.check_output(base+['--release']), 'Demon stand/run debug/release stdout differs'
print('Identical debug/release synthetic output: demon_stand_run')
