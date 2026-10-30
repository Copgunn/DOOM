#!/usr/bin/env python3
"""Independent grounded-player DSIM22 and compact ordered event evidence."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import player_snapshot
ROOT=Path(__file__).resolve().parents[2]
def run(check):
    inputs=['tools/reference/m3g3_snapshot_goldens.py','tools/reference/player_snapshot.py',
            'tools/reference/m3g2_snapshot_goldens.py','tools/reference/m3f3_snapshot_goldens.py',
            'tools/reference/m3c_snapshot_goldens.py','tests/reference/m3g3_trace.csv',
            'tests/reference/m3g3_cases.csv','tests/reference/m3f5_map_0.bin']
    lines=(ROOT/inputs[5]).read_text().splitlines();assert lines.pop(0)=='DOOM-PLAYER-KNOCKBACK-v1'
    rows=[[r[0],*map(int,r[1:])] for r in csv.reader(lines)]
    cases=list(csv.DictReader((ROOT/inputs[6]).open()))
    maps=[ROOT/inputs[7]]+[ROOT/f'tests/reference/m3g3_map_{i}.bin' for i in range(1,1+max(int(c['map']) for c in cases))]
    inputs.extend(str(m.relative_to(ROOT)) for m in maps[1:])
    outputs,inventory=player_snapshot.build(cases,rows,maps,3,
        b'PLAYER-BODY-1-MEDIUM-GROUNDED-v1','m3g3',{(8,8),(10,1),(11,72),(12,72)})
    provenance=dict(schema='DSIM-v22',execution_profile=3,**inventory,
        scope='New compiled-C knockback/grounded player fields and complete ordered events. Same DSIM22 layout, distinct policy3/context; old stationary policy2 unchanged. PBE1 compact event fixture omits derivable path strings; every payload/order/count is compared. Source allocation births project through independently specified Rust slot/generation policy. No Rust expected output.',
        inputs_sha256={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in inputs},
        outputs_sha256={p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()})
    outputs['tests/reference/m3g3_snapshot_provenance.json']=(json.dumps(provenance,indent=2,sort_keys=True)+'\n').encode()
    for p,b in outputs.items():
        if check:assert (ROOT/p).read_bytes()==b,p
        else:(ROOT/p).write_bytes(b)
    print(f'{"Checked" if check else "Generated"} DSIM22 grounded player: {inventory}; bytes={sum(map(len,outputs.values()))}')
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
