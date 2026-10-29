#!/usr/bin/env python3
"""Fresh public canonical Demon requests, native capacity and stale-handle seeds."""
import sys
from pathlib import Path

destination = Path(sys.argv[1])
destination.mkdir(parents=True, exist_ok=True)
for i in range(18):
    # Two prefix bytes, then eight four-byte operations. Canonical ordinals are
    # seed inputs, not an alternative implementation of the runtime mapping.
    requests = [
        bytes([1, 0, 485 & 255, 485 >> 8]),
        bytes([3, 0, 495 & 255, 495 >> 8]),
        bytes([1, 0, 496 & 255, 496 >> 8]),
        bytes([2, 0, 13, 0]),
        bytes([129, 0, 477 & 255, 477 >> 8]),
        bytes([4, 0, 150, 0]),
        bytes([0, 0, 0, 0]),
        bytes([6, 0, 255, 255]),
    ] if i >= 9 else [bytes([0, 0, 0, 0])] * 8
    (destination / f"public_seed_{i}").write_bytes(bytes([i % 9, 64]) + b"".join(requests))
for limit in [0, 1, 3, 7]:
    (destination / f"limit_{limit}").write_bytes(bytes([2, limit]))
