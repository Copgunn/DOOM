#!/usr/bin/env python3
"""Twelfth asserted example equality; all historical helpers stay unchanged."""
import subprocess
base=['cargo','+stable','run','-p','doom-sim','--example','chaingunner_attacks','--locked']
assert subprocess.check_output(base)==subprocess.check_output(base+['--release']), 'Chaingunner debug/release stdout differs'
print('Identical debug/release synthetic output: chaingunner_attacks')
