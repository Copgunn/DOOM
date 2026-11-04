#!/usr/bin/env python3
"""Independent DSIM36 fields/events/bytes from compiled original-C two-sided scroller, static decoration and spawn-filter rows."""
import argparse,collections,csv,hashlib,json,struct,sys
from pathlib import Path
import e1m1_snapshot
import movers_snapshot
import specials_snapshot
from evidence_pack import pack,unpack
ROOT=Path(__file__).resolve().parents[2]
MEDIUM=2
def first(rows,test,captures):
    return next((r[1],r[2])for r in rows if test(r)and(r[1],r[2])in captures)
def fixtures(rows,cases):
    """The full-byte captures, chosen from the trace: each names what it pins."""
    captures={(r[1],r[2])for r in rows if r[0]=='P'}
    case={c['label']:i for i,c in enumerate(cases)}
    def last(label):return (case[label],max(op for c,op in captures if c==case[label]))
    def of(label,tag,test=lambda r:True):return first(rows,lambda r:r[0]==tag and r[1]==case[label]and test(r),captures)
    def latest(label,tag,test):return max((r[1],r[2])for r in rows if r[0]==tag and r[1]==case[label]and test(r)and(r[1],r[2])in captures)
    spechit=collections.Counter((r[1],r[2])for r in rows if r[0]=='MK')
    pins=[
      (of('scroll_cross_noop','MX'),'player crossed a non-blocking two-sided 48 line: the line in spechit, the crossing event, no draw, line.special stays 48'),
      (of('scroll_spechit_nodir','MP'),'monster blocked with the 48 line in spechit: DI_NODIR, P_UseSpecialLine false, the chase-direction draw'),
      (of('use_scroll_wall_stops_trace','MI'),'use trace stopped at the two-sided 48 line: the door beyond stays closed, no sfx_noway'),
      (last('fireball_through_scroller'),'final tic of scrollhall: the fireball crossed the 48 line and exploded on the player'),
      (last('blocking_scroller_wall'),'final tic of scrollblock: player and monster refused by the ML_BLOCKING 48 line, spechit empty'),
      (first(rows,lambda r:r[0]=='MK'and r[1]==case['dense_spechit_8']and spechit[r[1],r[2]]==8,captures),'dense8: one position check filled spechit to its eight entries, no overflow'),
      (last('dense_spechit_8'),'final tic of dense8: every scroller crossed, every line special still 48'),
      (of('walk_into_lamp','M',lambda r:r[7]==0),'gallery with all seven decoration types: player blocked by the solid lamp (no height test)'),
      (of('fireball_hits_candelabra','MS',lambda r:r[3]==0),'fireball exploded on the solid candelabra standing on its pedestal (missile branch, no damage)'),
      (last('walk_over_gore'),'final tic of gallery: player walked through the row of flags-0 gore'),
      (latest('decorations_ride_lift','TK',lambda r:r[4]==6 and r[8]==1),'lift descending (plat status down), its last capture before it waits at the bottom: the lamp and the dead player clipped to its floor (mid-lift)'),
      (of('door_closes_on_pillar','MV',lambda r:r[4]==1 and r[5]==-1 and r[10]==2),'door closed onto the techno pillar: PIT_ChangeSector left it as is, no nofit, the mover removed'),
      (last('door_closes_on_pillar'),'final tic of liftroom: door shut on the pillar, mover unlinked'),
      ((case['spawn_filter_medium'],0),'census after spawn (medium): skill- and bit-16-filtered records absent, MF_AMBUSH on a lamp and a gib pool, thinker order = lump order then the player body'),
      (last('spawn_filter_medium'),'final tic of census')]
    chosen={}
    for key,why in pins:chosen[key]=chosen[key]+'; '+why if key in chosen else why
    assert len(pins)==15 and len(chosen)==15,chosen
    return chosen
def frames(stream):
    """PBE1 frames as (header, records) with the tag-26 and tag-25 record counts."""
    assert stream[:4]==b'PBE1';at=4;out=[];tags=collections.Counter()
    while at<len(stream):
        head=struct.unpack_from('<4I',stream,at);at+=16;records=[]
        for _ in range(head[2]+head[3]):
            size,=struct.unpack_from('<I',stream,at);at+=4;record=stream[at:at+size];at+=size
            if record[0]in(specials_snapshot.SPECIAL_EVENTS,movers_snapshot.MOVERS):tags[record[0],record[1]]+=1
            records.append(record)
        out.append((head[:2],records))
    return out,tags
def run(check):
    lines=unpack((ROOT/'tests/reference/m3h9_trace.dref').read_bytes()).decode().splitlines()
    assert lines.pop(0)=='DOOM-PLAYER-E1M1-v1'
    raw=[[r[0],*map(int,r[1:])]for r in csv.reader(lines)if r[0]!='TI']
    cases=list(csv.DictReader((ROOT/'tests/reference/m3h9_cases.csv').open()))
    contract=json.loads((ROOT/'tests/reference/m3h9_contract.json').read_text());maps=[ROOT/p for p in contract['maps']]
    # Only the scenarios Rust replays (skill 2) are projected; the C-only census rows at
    # skill 1 and 3 are excluded (they are the trailing cases).
    replayed=[i for i,c in enumerate(cases)if int(c['skill'])==MEDIUM];excluded=[c['label']for c in cases if int(c['skill'])!=MEDIUM]
    assert replayed==list(range(len(replayed)))and excluded==contract['inventory']['c_only_scenarios']
    cases=cases[:len(replayed)]
    rows=e1m1_snapshot.strip([r for r in raw if r[1]in replayed])
    chosen=fixtures(rows,cases)
    outputs,inventory=e1m1_snapshot.build(cases,rows,maps,'m3h9',set(chosen))
    events=outputs.pop('tests/reference/m3h9_events.bin')
    # Decision E8: every capture is byte for byte the DSIM35 projection of the same rows
    # except the version, the profile value and the context bytes; the event stream is
    # identical. Both projections are built with every capture as a full-byte fixture.
    captures={(r[1],r[2])for r in rows if r[0]=='P'}
    every36,_=e1m1_snapshot.build(cases,rows,maps,'m3h9',captures)
    every35,_=movers_snapshot.build(cases,rows,maps,'m3h9',captures)
    assert every36.pop('tests/reference/m3h9_events.bin')==events==every35.pop('tests/reference/m3h9_events.bin')
    shorter=len(movers_snapshot.CONTEXT)-len(e1m1_snapshot.CONTEXT)
    for which,op in sorted(captures):
        name=f'tests/reference/m3h9_state_{which}_{op}.bin'
        v36,f36=specials_snapshot.decode(every36[name]);v35,f35=specials_snapshot.decode(every35[name])
        assert v36==36 and v35==35 and [p for p,_ in f36]==[p for p,_ in f35]
        assert all(a==b or p in('profile','context.schema')for(p,a),(_,b)in zip(f36,f35))
        assert dict(f36)['profile']==struct.pack('<I',20)and dict(f35)['profile']==struct.pack('<I',19)
        assert dict(f36)['context.schema']==e1m1_snapshot.CONTEXT and dict(f35)['context.schema']==movers_snapshot.CONTEXT
        if (which,op)in chosen:assert outputs[name]==every36[name]
    index=outputs['tests/reference/m3h9_snapshots.csv'].decode().splitlines()
    for a,b,c in zip(index[1:],every36['tests/reference/m3h9_snapshots.csv'].decode().splitlines()[1:],every35['tests/reference/m3h9_snapshots.csv'].decode().splitlines()[1:]):
        a,b,c=a.split(','),b.split(','),c.split(',')
        assert a[:4]==b[:4]and a[5:]==b[5:]and a[:3]==c[:3]and a[6:]==c[6:]and int(a[5])==int(c[5])-shorter
        assert (a[4]!='-')==((int(a[0]),int(a[1]))in chosen)
    for path,data in outputs.items():
        if path.endswith('.bin'):
            version,fields=specials_snapshot.decode(data);value=dict(fields)
            assert version==36 and value['profile']==struct.pack('<I',20)and value['context.schema']==e1m1_snapshot.CONTEXT
    _,tags=frames(events)
    outputs['tests/reference/m3h9_events.dref']=pack(events)
    assert unpack(outputs['tests/reference/m3h9_events.dref'])==events
    names={v:k for k,v in movers_snapshot.SUBTAGS.items()}
    inventory.update(expanded_event_bytes=len(events),packed_event_bytes=len(outputs['tests/reference/m3h9_events.dref']),
      mover_events={names[k]:tags[movers_snapshot.MOVERS,k]for k in sorted(names)},
      specials_events={('secret_entry','sky_fireball','sky_hitscan')[k]:tags[specials_snapshot.SPECIAL_EVENTS,k]for k in range(3)},
      new_event_tags=0,new_fields=0,rust_replayed_scenarios=[c['label']for c in cases],c_only_scenarios_excluded=excluded,
      fixtures={f'm3h9_state_{c}_{o}.bin':why for (c,o),why in sorted(chosen.items())})
    decorations={str(d):dict(description=desc,mobjinfo=member,mobjtype_index=typ,spawnstate=state,state_index=state_index,radius=radius,height=height,flags=flags,health=1000,tics=-1,reaction_time=e1m1_snapshot.REACTION_TIME)
                 for d,(desc,member,state,radius,height,_)in sorted(e1m1_snapshot.DECORATIONS.items())for typ,state_index,_,_,flags in[e1m1_snapshot.ADMITTED[d]]}
    sha=lambda data:hashlib.sha256(data).hexdigest()
    inputs=sorted({'tools/reference/'+Path(m.__file__).name for m in list(sys.modules.values())if getattr(m,'__file__',None)and Path(m.__file__).resolve().parent==ROOT/'tools/reference'})
    inputs+=['tools/reference/m3h9_snapshot_goldens.py','linuxdoom-1.10/info.h','tests/reference/m3h9_trace.dref','tests/reference/m3h9_cases.csv','tests/reference/m3h9_contract.json']+contract['maps']
    inputs=sorted(set(inputs))
    outputs['tests/reference/m3h9_snapshot_contract.json']=(json.dumps(dict(
      schema='DSIM-v36',execution_profile=e1m1_snapshot.POLICY,context=e1m1_snapshot.CONTEXT.decode(),store_slots=e1m1_snapshot.MAX_SLOTS,
      live_projectile_bound=e1m1_snapshot.MAX_LIVE_PROJECTILES,decoration_limit=e1m1_snapshot.MAX_DECORATIONS,decorations=decorations,**inventory,
      dsim35_identity='Every capture equals the DSIM35 projection (movers_snapshot) of the same stripped rows field for field and byte for byte except the version (36), the profile value (20) and the context.schema bytes; the PBE1 event stream is identical. Decision E8: no new field, no new event.',
      scope='Original C fields and source event order, independent schema/birth-generation projection. Bounded command/weapon stage then original thinkers, not full player/ticker execution. DSIM35 layout and event vocabulary under version 36, policy 20 and the E1M1 context, with static decorations (DoomEd 2028, 48, 35, 24, 15, 10, 12) as ordinary actor records checked against the pristine info.c values read from the op-0 captures (type, spawn state, tics -1, health 1000, radius, height, flags with MF_AMBUSH from the THING record, spawnpoint, no target, no momentum, z on the floor) in lump order in the thinker order, at most 32 per map, and two-sided type-48 lines as line.{i}.special = 48 (never cleared; the scroll offset is leveltime * FRACUNIT and is not encoded). The spawn filter encodes nothing: filtered records have no actor. The M3h9 observer rows DT, XT and NS are neither state nor events and are removed before the chain. Only the 18 scenarios Rust replays (skill 2) are projected; the two C-only census rows (skill 1 and 3) are not. No Rust observations.',
      inputs_sha256={p:sha((ROOT/p).read_bytes())for p in inputs},
      outputs_sha256={p:sha(data)for p,data in outputs.items()}),indent=2,sort_keys=True)+'\n').encode()
    for path,data in outputs.items():
        if check:assert (ROOT/path).read_bytes()==data,path
        else:(ROOT/path).write_bytes(data)
    print(('Checked'if check else'Generated')+' DSIM36 two-sided scrollers, static decorations and the spawn filter: '+str({k:v for k,v in inventory.items()if k!='fixtures'}))
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');run(p.parse_args().check)
