#!/usr/bin/env python3
"""Independent DSIM29 fields/events/bytes from compiled original-C Imp rows."""
import argparse,csv,hashlib,json
from pathlib import Path
import imps_snapshot
from evidence_pack import pack,unpack
ROOT=Path(__file__).resolve().parents[2]
def run(check):
    inputs=['tools/reference/m3h2_snapshot_goldens.py','tools/reference/imps_snapshot.py','tools/reference/monsters_snapshot.py','tools/reference/bonus_snapshot.py',
      'tools/reference/armor_snapshot.py','tools/reference/pickup_snapshot.py','tools/reference/weapon_snapshot.py',
      'tools/reference/pistol_snapshot.py','tools/reference/player_execution_snapshot.py',
      'tools/reference/player_snapshot.py','tools/reference/m3g2_snapshot_goldens.py',
      'tools/reference/m3f3_snapshot_goldens.py','tools/reference/evidence_pack.py',
      'tests/reference/m3h2_trace.dref','tests/reference/m3h2_cases.csv','tests/reference/m3h2_contract.json']
    lines=unpack((ROOT/inputs[-3]).read_bytes()).decode().splitlines()
    assert lines.pop(0)=='DOOM-PLAYER-IMPS-v1'
    rows=[[r[0],*map(int,r[1:])]for r in csv.reader(lines)]
    cases=list(csv.DictReader((ROOT/inputs[-2]).open()))
    contract=json.loads((ROOT/inputs[-1]).read_text());inputs+=contract['maps']
    labels={c['label']:(i,int(c['tics']))for i,c in enumerate(cases)}
    full={(0,0),labels['imp_missile_far'],labels['fireball_hits_imp'],labels['mixed_roster'],labels['roster_eight_imps'],labels['player_death_fireball']}
    outputs,inventory=imps_snapshot.build(cases,rows,[ROOT/p for p in contract['maps']],'m3h2',full)
    events=outputs.pop('tests/reference/m3h2_events.bin')
    outputs['tests/reference/m3h2_events.dref']=pack(events)
    assert unpack(outputs['tests/reference/m3h2_events.dref'])==events
    inventory.update(expanded_event_bytes=len(events),packed_event_bytes=len(outputs['tests/reference/m3h2_events.dref']))
    sha=lambda data:hashlib.sha256(data).hexdigest()
    outputs['tests/reference/m3h2_snapshot_contract.json']=(json.dumps(dict(
      schema='DSIM-v29',execution_profile=13,context=imps_snapshot.CONTEXT.decode(),**inventory,
      scope='Original C fields and source event order, independent schema/birth-generation projection. Bounded command/weapon stage then original thinkers, not full player/ticker execution. No Rust observations.',
      inputs_sha256={p:sha((ROOT/p).read_bytes())for p in inputs},
      outputs_sha256={p:sha(data)for p,data in outputs.items()}),indent=2,sort_keys=True)+'\n').encode()
    for path,data in outputs.items():
        if check:assert (ROOT/path).read_bytes()==data,path
        else:(ROOT/path).write_bytes(data)
    print(('Checked'if check else'Generated')+' DSIM29 Imps: '+str(inventory))
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');run(p.parse_args().check)
