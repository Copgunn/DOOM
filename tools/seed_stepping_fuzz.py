#!/usr/bin/env python3
"""Bounded profile/command/error seeds; destination must be supplied explicitly."""
import argparse
from pathlib import Path
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('destination', type=Path)
args = parser.parse_args()
args.destination.mkdir(parents=True, exist_ok=True)
for profile in range(21):  # Index in the test-only PROFILES table, never an actor ID.
    commands = [bytes([0, 0, 0, 0, 0, 0, 0, 0]),
                bytes([1, 1, 0, 0, 0, 0, 2, 0]),
                bytes([0, 0, 0, 0, 0, 255, 0, 0]),
                bytes([0, 0, 0, 0, 0, 255, 1, 0]),
                bytes([0, 0, 0, 0, 0, 0, 4, 1])]
    (args.destination / f'profile-{profile:02}').write_bytes(bytes([profile]) + b''.join(commands))
print('stepping_equivalence: 21 bounded profile seeds')
