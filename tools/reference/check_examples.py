#!/usr/bin/env python3
"""Keep historical asserted equality helpers intact and add profile17."""
from pathlib import Path
import subprocess, sys
root=Path(__file__).resolve().parents[2]
for name in ['headless','projectiles','monster_perception','monster_pursuit','monster_attacks','shotgunner_attacks','chaingunner_attacks','demon_melee','demon_lifecycle','demon_stand_run','demon_map_spawn','demon_map_context']:
    subprocess.run([sys.executable,str(root/'tools/reference'/f'check_{name}.py')],cwd=root,check=True)
print('Checked all 17 asserted simulation examples in debug/release.')
