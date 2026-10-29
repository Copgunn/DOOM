#!/usr/bin/env python3
"""Fresh bounded native/canonical namespace collisions and lifecycle requests."""
import sys
from pathlib import Path
destination = Path(sys.argv[1])
destination.mkdir(parents=True, exist_ok=True)
values = [0, 1, 3, 6, 7, 9, 12, 14, 15, 21, 22, 135, 136, 475, 487, 495, 496, 501, 967, 65535]
for i, value in enumerate(values):
    requests = [(6, value), (3, value), (1, value), (2, value), (4, 150), (0, 0), (5, 0), (131, value)]
    data = bytes([i % 9, 64]) + b''.join(bytes([op, 0]) + raw.to_bytes(2, 'little') for op, raw in requests)
    (destination / f'namespace_{i}').write_bytes(data)
for limit in [0, 1, 3, 7]:
    (destination / f'limit_{limit}').write_bytes(bytes([2, limit]))
