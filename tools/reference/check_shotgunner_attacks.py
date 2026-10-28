#!/usr/bin/env python3
"""Eleventh asserted example equality; all historical helpers stay unchanged."""
import subprocess
base=['cargo','+stable','run','-p','doom-sim','--example','shotgunner_attacks','--locked']
assert subprocess.check_output(base)==subprocess.check_output(base+['--release']), 'Shotgunner debug/release stdout differs'
print('Identical debug/release synthetic output: shotgunner_attacks')
