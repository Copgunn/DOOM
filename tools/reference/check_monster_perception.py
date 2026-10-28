#!/usr/bin/env python3
"""Eighth asserted example equality; all historical helpers remain unchanged."""
import subprocess
base=['cargo','+stable','run','-p','doom-sim','--example','monster_perception','--locked']
assert subprocess.check_output(base)==subprocess.check_output(base+['--release']), 'Perception debug/release stdout differs'
print('Identical debug/release synthetic output: monster_perception')
