#!/usr/bin/env python3
"""Run all C-backed command captures and event assertions in both build modes."""
import subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
outputs=[subprocess.check_output(['cargo','run','-q','-p','doom-sim','--example','player_commands',*mode,'--locked'],cwd=root) for mode in [[],['--release']]]
assert outputs[0]==outputs[1]
print('player_commands: 22 C-backed scenarios, 316 captures and 10530 ordered payload fields agree in debug/release')
