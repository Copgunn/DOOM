#!/usr/bin/env python3
"""Seventh asserted example debug/release equality; historical helper unchanged."""
import subprocess
base=['cargo','+stable','run','-p','doom-sim','--example','projectiles','--locked']
assert subprocess.check_output(base)==subprocess.check_output(base+['--release']), 'Projectile debug/release stdout differs'
print('Identical debug/release synthetic output: projectiles')
