#!/usr/bin/env python3
"""Independent DSIM34 fields/events/bytes from compiled original-C sky and static-specials rows."""
import argparse,collections,csv,hashlib,json,struct
from pathlib import Path
import specials_snapshot
import real_map_snapshot
from evidence_pack import pack,unpack
ROOT=Path(__file__).resolve().parents[2]
def first(rows,test):
    return next((r[1],r[2])for r in rows if test(r))
def fixtures(rows,cases):
    """The full-byte captures, chosen from the trace: each names what it pins."""
    labels={c['label']:i for i,c in enumerate(cases)}
    player={r[1]:r[5]for r in rows if r[0]=='W'and r[2]==0 and r[3]==7}
    env={(r[1],r[2])for r in rows if r[0]=='G'and r[3]==2 and r[5]==-1 and r[6]==-1}
    absorbed={(r[1],r[2])for r in rows if r[0]=='AR'and r[5]>0}
    lights={(r[1],r[2])for r in rows if r[0]=='LC'}
    soak=labels['pools_lights_soak']
    chosen={
      first(rows,lambda r:r[0]=='TK'and r[1]==soak and r[2]==0):'load: all four sector-thinker kinds, cleared light specials, total_secrets',
      (soak,int(cases[soak]['tics'])):'final soak tic: every light thinker after 1,200 tics',
      first(rows,lambda r:r[0]=='G'and r[3]==2 and(r[1],r[2])in env&absorbed):'environmental damage with no inflictor or source, armor absorbing',
      first(rows,lambda r:r[0]=='G'and r[3]==7 and r[4]==player[r[1]]and(r[1],r[2])in env):'environmental death and own frag',
      first(rows,lambda r:r[0]=='SE'):'secret entry: special cleared, secret_count 1',
      first(rows,lambda r:r[0]=='MS'and r[3]==0 and(r[1],r[2])in lights):'a light change and a fireball explosion in one tic',
      first(rows,lambda r:r[0]=='KR'):'fireball removed at a sky ceiling line',
      first(rows,lambda r:r[0]=='KS'and r[3]==0):'hitscan ended above a sky front ceiling',
      first(rows,lambda r:r[0]=='KS'and r[3]==1):'hitscan ended at a sky-hack wall'}
    captures={(r[1],r[2])for r in rows if r[0]=='P'}
    assert len(chosen)==9 and set(chosen)<=captures,chosen
    return chosen
def run(check):
    inputs=['tools/reference/m3h7_snapshot_goldens.py','tools/reference/specials_snapshot.py','tools/reference/real_map_snapshot.py','tools/reference/diagonal_snapshot.py','tools/reference/heights_snapshot.py','tools/reference/map_scale_snapshot.py','tools/reference/imps_snapshot.py','tools/reference/monsters_snapshot.py','tools/reference/bonus_snapshot.py',
      'tools/reference/armor_snapshot.py','tools/reference/pickup_snapshot.py','tools/reference/weapon_snapshot.py',
      'tools/reference/pistol_snapshot.py','tools/reference/player_execution_snapshot.py',
      'tools/reference/player_snapshot.py','tools/reference/m3g2_snapshot_goldens.py',
      'tools/reference/m3f3_snapshot_goldens.py','tools/reference/evidence_pack.py',
      'tests/reference/m3h7_trace.dref','tests/reference/m3h7_cases.csv','tests/reference/m3h7_contract.json']
    lines=unpack((ROOT/inputs[-3]).read_bytes()).decode().splitlines()
    assert lines.pop(0)=='DOOM-PLAYER-SPECIALS-v1'
    rows=[[r[0],*map(int,r[1:])]for r in csv.reader(lines)]
    cases=list(csv.DictReader((ROOT/inputs[-2]).open()))
    contract=json.loads((ROOT/inputs[-1]).read_text());inputs+=contract['maps'];maps=[ROOT/p for p in contract['maps']]
    chosen=fixtures(rows,cases)
    outputs,inventory=specials_snapshot.build(cases,rows,maps,'m3h7',set(chosen))
    events=outputs.pop('tests/reference/m3h7_events.bin')
    # Consistency: without its tag-25 records the stream is the DSIM33 projection of the
    # same rows with every new observer row removed (the frozen chain sees the same rows).
    plain,_=real_map_snapshot.build(cases,[r for r in rows if r[0]not in specials_snapshot.DROPPED|{'SE','KR','KS'}],maps,'m3h7',set())
    def frames(stream,keep):
        assert stream[:4]==b'PBE1';at=4;out=[];tags=collections.Counter()
        while at<len(stream):
            head=struct.unpack_from('<4I',stream,at);at+=16;records=[]
            for _ in range(head[2]+head[3]):
                size,=struct.unpack_from('<I',stream,at);at+=4;record=stream[at:at+size];at+=size
                if record[0]==25:tags[record[1]]+=1
                if keep or record[0]!=25:records.append(record)
            out.append((head[:2],records))
        return out,tags
    stripped,tags=frames(events,False)
    assert stripped==frames(plain['tests/reference/m3h7_events.bin'],True)[0]
    # Every DSIM34 frame header and policy field names the new identity.
    for path,data in outputs.items():
        if path.endswith('.bin'):
            version,fields=specials_snapshot.decode(data);value=dict(fields)
            assert version==34 and value['profile']==struct.pack('<I',18) and value['context.schema']==specials_snapshot.CONTEXT
    outputs['tests/reference/m3h7_events.dref']=pack(events)
    assert unpack(outputs['tests/reference/m3h7_events.dref'])==events
    inventory.update(expanded_event_bytes=len(events),packed_event_bytes=len(outputs['tests/reference/m3h7_events.dref']),
      special_events={'secret_entry':tags[0],'sky_removal':tags[1],'sky_shot':tags[2]},
      fixtures={f'm3h7_state_{c}_{o}.bin':why for (c,o),why in sorted(chosen.items())})
    sha=lambda data:hashlib.sha256(data).hexdigest()
    outputs['tests/reference/m3h7_snapshot_contract.json']=(json.dumps(dict(
      schema='DSIM-v34',execution_profile=specials_snapshot.POLICY,context=specials_snapshot.CONTEXT.decode(),store_slots=specials_snapshot.MAX_SLOTS,
      live_projectile_bound=specials_snapshot.MAX_LIVE_PROJECTILES,**inventory,
      scope='Original C fields and source event order, independent schema/birth-generation projection. Bounded command/weapon stage then original thinkers, not full player/ticker execution. DSIM33 layout plus runtime sector light and special, the sector-thinker store and the shared thinker order (TK rows), the player secret count (SX rows) and tag-25 secret, sky-removal and sky-shot events; light draws and changes are no events; TI, TK, SX, LW, LE, LR, LC, SP, KY, KL and KZ rows are not events. Without tag 25 the event stream equals the DSIM33 projection of the same rows. No Rust observations.',
      inputs_sha256={p:sha((ROOT/p).read_bytes())for p in inputs},
      outputs_sha256={p:sha(data)for p,data in outputs.items()}),indent=2,sort_keys=True)+'\n').encode()
    for path,data in outputs.items():
        if check:assert (ROOT/path).read_bytes()==data,path
        else:(ROOT/path).write_bytes(data)
    print(('Checked'if check else'Generated')+' DSIM34 sky and static specials: '+str({k:v for k,v in inventory.items()if k!='fixtures'}))
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');run(p.parse_args().check)
