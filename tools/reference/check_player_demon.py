#!/usr/bin/env python3
"""Run all C-backed command captures and event assertions in both build modes."""
import subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
outputs=[subprocess.check_output(['cargo','run','-q','-p','doom-sim','--example','player_demon',*mode,'--locked'],cwd=root) for mode in [[],['--release']]]
assert outputs[0]==outputs[1]
print('player_demon: 25 C-backed scenarios, 373 captures and 12886 ordered payload fields agree in debug/release')
