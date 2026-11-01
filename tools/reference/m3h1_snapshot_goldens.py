#!/usr/bin/env python3
"""Independent DSIM28 fields/events/bytes from compiled original-C monster rows."""
import argparse,csv,hashlib,json
from pathlib import Path
import monsters_snapshot
from evidence_pack import pack,unpack
ROOT=Path(__file__).resolve().parents[2]
def run(check):
    inputs=['tools/reference/m3h1_snapshot_goldens.py','tools/reference/monsters_snapshot.py','tools/reference/bonus_snapshot.py',
      'tools/reference/armor_snapshot.py','tools/reference/pickup_snapshot.py','tools/reference/weapon_snapshot.py',
      'tools/reference/pistol_snapshot.py','tools/reference/player_execution_snapshot.py',
      'tools/reference/player_snapshot.py','tools/reference/m3g2_snapshot_goldens.py',
      'tools/reference/m3f3_snapshot_goldens.py','tools/reference/evidence_pack.py',
      'tests/reference/m3h1_trace.dref','tests/reference/m3h1_cases.csv','tests/reference/m3h1_contract.json']
    lines=unpack((ROOT/inputs[-3]).read_bytes()).decode().splitlines()
    assert lines.pop(0)=='DOOM-PLAYER-MONSTERS-v1'
    rows=[[r[0],*map(int,r[1:])]for r in csv.reader(lines)]
    cases=list(csv.DictReader((ROOT/inputs[-2]).open()))
    contract=json.loads((ROOT/inputs[-1]).read_text());inputs+=contract['maps']
    full={(0,0),(1,480),(6,720),(7,240),(11,480),(13,512)}
    outputs,inventory=monsters_snapshot.build(cases,rows,[ROOT/p for p in contract['maps']],'m3h1',full)
    events=outputs.pop('tests/reference/m3h1_events.bin')
    outputs['tests/reference/m3h1_events.dref']=pack(events)
    assert unpack(outputs['tests/reference/m3h1_events.dref'])==events
    inventory.update(expanded_event_bytes=len(events),packed_event_bytes=len(outputs['tests/reference/m3h1_events.dref']))
    sha=lambda data:hashlib.sha256(data).hexdigest()
    outputs['tests/reference/m3h1_snapshot_contract.json']=(json.dumps(dict(
      schema='DSIM-v28',execution_profile=12,context=monsters_snapshot.CONTEXT.decode(),**inventory,
      scope='Original C fields and source event order, independent schema/birth-generation projection. Bounded command/weapon stage then original thinkers, not full player/ticker execution. No Rust observations.',
      inputs_sha256={p:sha((ROOT/p).read_bytes())for p in inputs},
      outputs_sha256={p:sha(data)for p,data in outputs.items()}),indent=2,sort_keys=True)+'\n').encode()
    for path,data in outputs.items():
        if check:assert (ROOT/path).read_bytes()==data,path
        else:(ROOT/path).write_bytes(data)
    print(('Checked'if check else'Generated')+' DSIM28 former humans: '+str(inventory))
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');run(p.parse_args().check)
