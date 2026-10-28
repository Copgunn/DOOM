#!/usr/bin/env python3
"""Tenth asserted example equality; all historical helpers stay unchanged."""
import subprocess
base=['cargo','+stable','run','-p','doom-sim','--example','monster_attacks','--locked']
assert subprocess.check_output(base)==subprocess.check_output(base+['--release']), 'Monster attacks debug/release stdout differs'
print('Identical debug/release synthetic output: monster_attacks')
