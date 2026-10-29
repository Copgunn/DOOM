#!/usr/bin/env python3
"""Assert both builds against the independent example expectation."""
import subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
expected='map_extent: -1920 to +1920 through 16 sectors; cell 6939; sight work 16382; DSIM-v19 2c13f52d732a12c4'
for release in [False,True]:
    command=['cargo','run','-p','doom-sim','--example','map_extent','--locked']+(['--release'] if release else [])
    result=subprocess.check_output(command,cwd=root,text=True).strip()
    assert result==expected,(release,result)
print(expected)
