#!/usr/bin/env python3
"""Assert deterministic canonical identity example output in debug and release."""
import subprocess
expected = 'Original identity metadata: 137 actors, 967 states, 138 sprites; MT_SERGEANT=12; native aliases remain explicit.\n'
for flags in [[], ['--release']]:
    result = subprocess.check_output(['cargo', 'run', '-q', '-p', 'doom-sim', '--example', 'original_identity', '--locked', *flags], text=True)
    assert result == expected, result
print('original_identity: debug/release assertions and exact output passed')
