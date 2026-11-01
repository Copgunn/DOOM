#!/usr/bin/env python3
"""Independent DSIM30 fields/events/bytes from compiled original-C real-map-scale rows."""
import argparse,csv,hashlib,json
from pathlib import Path
import map_scale_snapshot
from evidence_pack import pack,unpack
ROOT=Path(__file__).resolve().parents[2]
def run(check):
    inputs=['tools/reference/m3h3_snapshot_goldens.py','tools/reference/map_scale_snapshot.py','tools/reference/imps_snapshot.py','tools/reference/monsters_snapshot.py','tools/reference/bonus_snapshot.py',
      'tools/reference/armor_snapshot.py','tools/reference/pickup_snapshot.py','tools/reference/weapon_snapshot.py',
      'tools/reference/pistol_snapshot.py','tools/reference/player_execution_snapshot.py',
      'tools/reference/player_snapshot.py','tools/reference/m3g2_snapshot_goldens.py',
      'tools/reference/m3f3_snapshot_goldens.py','tools/reference/evidence_pack.py',
      'tests/reference/m3h3_trace.dref','tests/reference/m3h3_cases.csv','tests/reference/m3h3_contract.json']
    lines=unpack((ROOT/inputs[-3]).read_bytes()).decode().splitlines()
    assert lines.pop(0)=='DOOM-PLAYER-MAP-SCALE-v1'
    rows=[[r[0],*map(int,r[1:])]for r in csv.reader(lines)]
    cases=list(csv.DictReader((ROOT/inputs[-2]).open()))
    contract=json.loads((ROOT/inputs[-1]).read_text());inputs+=contract['maps']
    labels={c['label']:(i,int(c['tics']))for i,c in enumerate(cases)}
    full={(0,0),labels['stall_fireball_past_994'],labels['long_fireball_flight'],labels['full_room_traversal'],labels['eight_imps_far_room'],labels['mixed_roster_corner_fight'],labels['drops_far_from_origin']}
    outputs,inventory=map_scale_snapshot.build(cases,rows,[ROOT/p for p in contract['maps']],'m3h3',full)
    events=outputs.pop('tests/reference/m3h3_events.bin')
    outputs['tests/reference/m3h3_events.dref']=pack(events)
    assert unpack(outputs['tests/reference/m3h3_events.dref'])==events
    inventory.update(expanded_event_bytes=len(events),packed_event_bytes=len(outputs['tests/reference/m3h3_events.dref']))
    sha=lambda data:hashlib.sha256(data).hexdigest()
    outputs['tests/reference/m3h3_snapshot_contract.json']=(json.dumps(dict(
      schema='DSIM-v30',execution_profile=map_scale_snapshot.POLICY,context=map_scale_snapshot.CONTEXT.decode(),store_slots=map_scale_snapshot.MAX_SLOTS,
      live_projectile_bound=map_scale_snapshot.MAX_LIVE_PROJECTILES,**inventory,
      scope='Original C fields and source event order, independent schema/birth-generation projection. Bounded command/weapon stage then original thinkers, not full player/ticker execution. No Rust observations.',
      inputs_sha256={p:sha((ROOT/p).read_bytes())for p in inputs},
      outputs_sha256={p:sha(data)for p,data in outputs.items()}),indent=2,sort_keys=True)+'\n').encode()
    for path,data in outputs.items():
        if check:assert (ROOT/path).read_bytes()==data,path
        else:(ROOT/path).write_bytes(data)
    print(('Checked'if check else'Generated')+' DSIM30 map scale: '+str(inventory))
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');run(p.parse_args().check)
