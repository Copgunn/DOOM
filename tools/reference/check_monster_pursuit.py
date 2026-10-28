#!/usr/bin/env python3
"""Ninth asserted example equality; every historical helper stays unchanged."""
import subprocess
base=['cargo','+stable','run','-p','doom-sim','--example','monster_pursuit','--locked']
assert subprocess.check_output(base)==subprocess.check_output(base+['--release']), 'Pursuit debug/release stdout differs'
print('Identical debug/release synthetic output: monster_pursuit')
