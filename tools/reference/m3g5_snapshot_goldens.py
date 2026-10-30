#!/usr/bin/env python3
"""Independent fixed-active-roster DSIM22 and source event evidence."""
import argparse,collections,csv,hashlib,json
from pathlib import Path
import player_execution_snapshot as player
ROOT=Path(__file__).resolve().parents[2]
def run(check):
    inputs=['tools/reference/m3g5_snapshot_goldens.py','tools/reference/player_execution_snapshot.py',
            'tools/reference/player_snapshot.py','tools/reference/m3g2_snapshot_goldens.py',
            'tools/reference/m3f3_snapshot_goldens.py','tools/reference/m3c_snapshot_goldens.py',
            'tests/reference/m3g5_trace.csv','tests/reference/m3g5_cases.csv','tests/reference/m3g5_provenance.json']
    text=(ROOT/inputs[6]).read_text().splitlines();assert text.pop(0)=='DOOM-PLAYER-DEMON-v1'
    rows=[[r[0],*map(int,r[1:])]for r in csv.reader(text)]
    grouped=collections.defaultdict(list)
    for r in rows:grouped[tuple(r[1:3])].append(r)
    roster=0
    for key,frame in grouped.items():
        p=next((r[3:]for r in frame if r[0]=='P'),None)
        if p is None:continue
        observed=[r[3:]for r in frame if r[0]=='R']
        assert observed==[[0,1,p[1],p[0]],[1,0,0,-1],[2,0,0,-1],[3,0,0,-1]],key
        associations=[r[3:]for r in frame if r[0]=='AP']
        assert [p[0],0]in associations,key
        roster+=4*len(observed)
    cases=list(csv.DictReader((ROOT/inputs[7]).open()))
    maps=json.loads((ROOT/inputs[8]).read_text())['maps'];inputs+=maps
    outputs,inventory=player.build(cases,[r for r in rows if r[0]!='DM'],[ROOT/p for p in maps],5,
        b'PLAYER-DEMON-1-MEDIUM-GROUNDED-NOMELEE-v1','m3g5',
        {(1,10),(2,10),(2,11),(7,10),(10,32),(17,128),(19,32),(23,32)})
    provenance=dict(schema='DSIM-v22',execution_profile=5,roster_fields=roster,**inventory,
        scope='Actual original player0 active and other slots inactive, verified against source R/P/AP rows; activity fixed by context, not duplicated mutable health. Shared frozen player field/event primitives with actual command-stage markers and original Look/Chase/scan evidence. Same DSIM22 layout; distinct policy5/context, no autonomous melee. This is bounded command-gate plus thinker execution, not full P_Ticker or P_PlayerThink. Negative turns and birth/generation projection retain explicitly qualified arithmetic/schema policies. No Rust expected outputs.',
        inputs_sha256={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()for p in inputs},
        outputs_sha256={p:hashlib.sha256(b).hexdigest()for p,b in outputs.items()})
    outputs['tests/reference/m3g5_snapshot_provenance.json']=(json.dumps(provenance,indent=2,sort_keys=True)+'\n').encode()
    for p,data in outputs.items():
        if check:assert(ROOT/p).read_bytes()==data,p
        else:(ROOT/p).write_bytes(data)
    print(f'{"Checked"if check else "Generated"} DSIM22 active player/Demon: {inventory}; roster_fields={roster}; bytes={sum(map(len,outputs.values()))}')
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
