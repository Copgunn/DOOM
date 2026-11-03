#!/usr/bin/env python3
"""Independent DSIM35 fields/events/bytes from compiled original-C doors, lifts, floors, switches and exit rows."""
import argparse,collections,csv,hashlib,json,struct
from pathlib import Path
import movers_snapshot
import specials_snapshot
from evidence_pack import pack,unpack
ROOT=Path(__file__).resolve().parents[2]
def first(rows,test,captures):
    return next((r[1],r[2])for r in rows if test(r)and(r[1],r[2])in captures)
def fixtures(rows,handles):
    """The full-byte captures, chosen from the trace: each names what it pins."""
    captures={(r[1],r[2])for r in rows if r[0]=='P'}
    use_ops={(r[1],r[2])for r in rows if r[0]=='MA'}
    def mv(r,plane,result):return r[0]=='MV'and r[4]==plane and r[10]==result
    pins=[
      (first(rows,lambda r:r[0]=='MM'and(r[1],r[2])in use_ops and r[4]==5,captures),'door created by a use in the player phase and stepped in the same tic'),
      (first(rows,lambda r:r[0]=='TK'and r[4]==5 and r[8]==0 and r[10]>0,captures),'door waiting at the top: direction 0, topcountdown counting'),
      (first(rows,lambda r:mv(r,1,1),captures),'closing door crushed by a thing and sent back up'),
      (first(rows,lambda r:r[0]=='MT'and r[4]==1,captures),'corpse gibbed by a closing door (clip outcome 1)'),
      (first(rows,lambda r:r[0]=='MT'and r[4]==2,captures),'dropped item removed by a closing door (clip outcome 2)'),
      (first(rows,lambda r:r[0]=='MT'and r[4]==4,captures),'stuck shootable: nofit (clip outcome 4)'),
      (first(rows,lambda r:r[0]=='SD',captures),'mover removed, kept in the shared order until unlinked (store kind 8)'),
      (first(rows,lambda r:r[0]=='SF',captures),'mover unlinked: free slot, generation 2'),
      (first(rows,lambda r:r[0]=='MM'and handles[r[1],r[3]][1]>1,captures),'slot reused by a new mover at generation 2'),
      (first(rows,lambda r:r[0]=='TK'and r[4]==6 and r[8]==2,captures),'lift waiting at the bottom (plat status waiting)'),
      (first(rows,lambda r:r[0]=='MV'and r[4]==0 and r[5]==-1 and r[9]>r[8],captures),'turbo floor jump-up in one step'),
      (first(rows,lambda r:r[0]=='MY'and r[5]==0,captures),'W1 line special cleared by a crossing'),
      (first(rows,lambda r:r[0]=='MP',captures),'monster blocked with special lines in spechit: DI_NODIR and a door use'),
      (first(rows,lambda r:r[0]=='MG'and r[3]==0,captures),'S1 exit switch: line cleared, level.exit set'),
      (first(rows,lambda r:r[0]=='MG'and r[3]==2,captures),'BT_USE while dead: reborn request (playerstate 2)')]
    chosen={}
    for key,why in pins:chosen[key]=chosen[key]+'; '+why if key in chosen else why
    assert len(pins)==15 and len(chosen)==13,chosen
    return chosen
def run(check):
    inputs=['tools/reference/m3h8_snapshot_goldens.py','tools/reference/movers_snapshot.py','tools/reference/specials_snapshot.py','tools/reference/real_map_snapshot.py','tools/reference/diagonal_snapshot.py','tools/reference/heights_snapshot.py','tools/reference/map_scale_snapshot.py','tools/reference/imps_snapshot.py','tools/reference/monsters_snapshot.py','tools/reference/bonus_snapshot.py',
      'tools/reference/armor_snapshot.py','tools/reference/pickup_snapshot.py','tools/reference/weapon_snapshot.py',
      'tools/reference/pistol_snapshot.py','tools/reference/player_execution_snapshot.py',
      'tools/reference/player_snapshot.py','tools/reference/m3g2_snapshot_goldens.py',
      'tools/reference/m3f3_snapshot_goldens.py','tools/reference/evidence_pack.py',
      'tests/reference/m3h8_trace.dref','tests/reference/m3h8_cases.csv','tests/reference/m3h8_contract.json']
    lines=unpack((ROOT/inputs[-3]).read_bytes()).decode().splitlines()
    assert lines.pop(0)=='DOOM-PLAYER-MOVERS-v1'
    rows=movers_snapshot.strip([[r[0],*map(int,r[1:])]for r in csv.reader(lines)if r[0]!='TI'])
    cases=list(csv.DictReader((ROOT/inputs[-2]).open()))
    contract=json.loads((ROOT/inputs[-1]).read_text());inputs+=contract['maps'];maps=[ROOT/p for p in contract['maps']]
    handles,_,_=movers_snapshot.stores(rows)
    chosen=fixtures(rows,handles)
    outputs,inventory=movers_snapshot.build(cases,rows,maps,'m3h8',set(chosen))
    events=outputs.pop('tests/reference/m3h8_events.bin')
    # Consistency: without its tag-26 records the stream is the DSIM34 projection of the
    # same rows with every Stage B row removed (the frozen chain sees the same rows).
    ended={r[1]:r[2]for r in rows if r[0]=='MG'and r[3]in(1,2)}
    short=[dict(c,tics=str(ended[i]))if i in ended else c for i,c in enumerate(cases)]
    plain,_=specials_snapshot.build(short,movers_snapshot.plain(rows),maps,'m3h8',set())
    def frames(stream,keep):
        assert stream[:4]==b'PBE1';at=4;out=[];tags=collections.Counter()
        while at<len(stream):
            head=struct.unpack_from('<4I',stream,at);at+=16;records=[]
            for _ in range(head[2]+head[3]):
                size,=struct.unpack_from('<I',stream,at);at+=4;record=stream[at:at+size];at+=size
                if record[0]==movers_snapshot.MOVERS:tags[record[1]]+=1
                if keep or record[0]!=movers_snapshot.MOVERS:records.append(record)
            out.append((head[:2],records))
        return out,tags
    stripped,tags=frames(events,False)
    assert stripped==frames(plain['tests/reference/m3h8_events.bin'],True)[0]
    # Every DSIM35 frame header and policy field names the new identity.
    for path,data in outputs.items():
        if path.endswith('.bin'):
            version,fields=specials_snapshot.decode(data);value=dict(fields)
            assert version==35 and value['profile']==struct.pack('<I',19) and value['context.schema']==movers_snapshot.CONTEXT
    outputs['tests/reference/m3h8_events.dref']=pack(events)
    assert unpack(outputs['tests/reference/m3h8_events.dref'])==events
    names={v:k for k,v in movers_snapshot.SUBTAGS.items()}
    inventory.update(expanded_event_bytes=len(events),packed_event_bytes=len(outputs['tests/reference/m3h8_events.dref']),
      mover_events={names[k]:tags[k]for k in sorted(names)},
      fixtures={f'm3h8_state_{c}_{o}.bin':why for (c,o),why in sorted(chosen.items())})
    sha=lambda data:hashlib.sha256(data).hexdigest()
    outputs['tests/reference/m3h8_snapshot_contract.json']=(json.dumps(dict(
      schema='DSIM-v35',execution_profile=movers_snapshot.POLICY,context=movers_snapshot.CONTEXT.decode(),store_slots=movers_snapshot.MAX_SLOTS,
      live_projectile_bound=movers_snapshot.MAX_LIVE_PROJECTILES,**inventory,
      scope='Original C fields and source event order, independent schema/birth-generation projection. Bounded command/weapon stage then original thinkers, not full player/ticker execution. DSIM34 layout plus level.exit, the moving sector floor and ceiling (MH rows, sector specialdata checked against the store), the runtime line specials (ML rows), the player use_down (MZ rows), door/plat/floor records and removed movers in the sector-thinker store and the shared order (TK, MM, SD, SF rows; lowest free slot, generation + 1 at unlink) and tag-26 mover events (MA, MI, MU, MX/MY, MP, MM, ME, MV, MC exit, HC/MT, SD, SF, SO, MG exit, the reborn request). Use-trace traversal rows, the position checks of a height clip, MK, MB, MW, MQ, MN and mover LE rows are not events. A case that requested an exit or a reborn ends at that op. Without tag 26 the event stream equals the DSIM34 projection of the same rows. No Rust observations.',
      inputs_sha256={p:sha((ROOT/p).read_bytes())for p in inputs},
      outputs_sha256={p:sha(data)for p,data in outputs.items()}),indent=2,sort_keys=True)+'\n').encode()
    for path,data in outputs.items():
        if check:assert (ROOT/path).read_bytes()==data,path
        else:(ROOT/path).write_bytes(data)
    print(('Checked'if check else'Generated')+' DSIM35 doors, lifts, floors, switches and the exit: '+str({k:v for k,v in inventory.items()if k!='fixtures'}))
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');run(p.parse_args().check)
