#!/usr/bin/env python3
"""Sixteenth asserted example; historical helpers stay unchanged."""
import subprocess
base=['cargo','+stable','run','-p','doom-sim','--example','demon_map_spawn','--locked']
assert subprocess.check_output(base)==subprocess.check_output(base+['--release']), 'Demon map-spawn debug/release stdout differs'
print('Identical debug/release synthetic output: demon_map_spawn')
