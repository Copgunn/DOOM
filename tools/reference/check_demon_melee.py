#!/usr/bin/env python3
"""Thirteenth asserted example equality; every historical helper is unchanged."""
import subprocess
base=['cargo','+stable','run','-p','doom-sim','--example','demon_melee','--locked']
assert subprocess.check_output(base)==subprocess.check_output(base+['--release']), 'Demon debug/release stdout differs'
print('Identical debug/release synthetic output: demon_melee')
