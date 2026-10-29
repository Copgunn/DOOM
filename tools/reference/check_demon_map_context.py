#!/usr/bin/env python3
"""Seventeenth asserted example; historical helpers stay unchanged."""
import subprocess
base=['cargo','+stable','run','-p','doom-sim','--example','demon_map_context','--locked']
assert subprocess.check_output(base)==subprocess.check_output(base+['--release']), 'Demon map-context debug/release stdout differs'
print('Identical debug/release synthetic output: demon_map_context')
