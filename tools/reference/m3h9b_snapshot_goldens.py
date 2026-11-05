#!/usr/bin/env python3
"""Independent DSIM37 fields/events/bytes from compiled original-C barrel, radius-attack, pickup-scale and angle rows."""
import argparse,collections,csv,hashlib,json,struct,sys,traceback
from pathlib import Path
import barrels_snapshot
import e1m1_snapshot
import specials_snapshot
from evidence_pack import pack,unpack
ROOT=Path(__file__).resolve().parents[2]
MEDIUM=2
EXPLODE=19                                  # A_Explode in the original action table (lexicographic)
A_TYPE,A_HEALTH,A_STATE,A_THRESHOLD=5,12,13,19   # A row columns: A,case,op,ident,alive,type,..,health,state,..,threshold
OLD_ITEMS,OLD_BONUS=16,96                   # the DSIM30..36 ranges asserted by the frozen chain
def first(rows,test,captures):
    return next((r[1],r[2])for r in rows if test(r)and(r[1],r[2])in captures)
def fixtures(rows,cases):
    """The full-byte captures, chosen from the C rows (A rows of MT_BARREL, E action rows):
    each names what it pins."""
    captures={(r[1],r[2])for r in rows if r[0]=='P'}
    case={c['label']:i for i,c in enumerate(cases)}
    barrel=lambda r:r[0]=='A'and r[A_TYPE]==barrels_snapshot.MT_BARREL
    def last(label):return (case[label],max(op for c,op in captures if c==case[label]))
    def of(label,test):return first(rows,lambda r:r[1]==case[label]and test(r),captures)
    state=lambda s:(lambda r:barrel(r)and r[A_STATE]==s)
    S=lambda name:barrels_snapshot.enum_index('statenum_t',name)
    pins=[
      (of('barrel_wounded',lambda r:barrel(r)and r[A_STATE]in barrels_snapshot.ALIVE and r[A_HEALTH]==20),'range at load: a live barrel in S_BAR1 (health 20, reaction_time 8, no target, threshold 0) after its lastlook and tics draws'),
      (of('barrel_wounded',lambda r:barrel(r)and r[A_THRESHOLD]==100),'barrel wounded by the pistol: health 5, target the player, threshold 100, reaction_time 0, thrust momentum, puff not blood'),
      (of('barrel_killed',state(S('S_BEXP'))),'barrel killed: S_BEXP, health <= 0, SOLID|NOBLOOD|CORPSE|DROPOFF, height 42 >> 2'),
      (of('chain_reaction',state(S('S_BEXP2'))),'S_BEXP2 (A_Scream at entry, no draw; barrel_killed has no capture in S_BEXP2): the cluster with three second-generation barrels in S_BEXP2'),
      (of('barrel_killed',state(S('S_BEXP3'))),'S_BEXP3'),
      (of('barrel_killed',lambda r:r[0]=='E'and r[3]==5 and r[5]==EXPLODE),'A_Explode op of barrel_killed: the S_BEXP4 entry and Action 19 records, no candidate but the barrel itself (not shootable: no record)'),
      (of('barrel_killed',state(S('S_BEXP5'))),'S_BEXP5'),
      (of('barrel_killed',state(-1)),'S_NULL entered: state 65535, tics 0, pending removal, still in the thinker order'),
      (of('fist_kills_barrel',lambda r:r[0]=='E'and r[3]==5 and r[5]==EXPLODE),'explosion of the barrel the fist killed in one blow: no target, threshold 0, bomb source absent, the 200-health player damaged with no source'),
      (of('chain_reaction',lambda r:r[0]=='E'and r[3]==5 and r[5]==EXPLODE),'first explosion of the cluster: four barrels killed in walk order, the wounded barrel W retargeted to the player, Q out of the wrapped 3x3 window'),
      (of('monster_wakes_mid_walk',lambda r:r[0]=='E'and r[3]==5 and r[5]==EXPLODE),'explosion whose victim Imp relinks mid-walk: Imp 4 damaged twice and killed'),
      (of('door_gibs_dying_barrel',state(barrels_snapshot.S_GIBS)),'dying barrel crunched by the closing door: S_GIBS, radius and height 0, SOLID clear'),
      ((case['dense_pickups'],0),'dense64 at load: 64 pickups, 16 barrels, 6 monsters, 27 decorations, 114 actors: the 113 records in lump order then the player body'),
      (last('intercept_heavy_trace'),'final tic of the 92-intercept shoot trace through the dense64 room'),
      ((case['all_octants'],0),'angles room at load: 17 lamps whose angles are ANG45 * (angle / 45) mod 2^32'),
      ((case['embedded_spawns'],0),'embedded room at load: Imp, lamp and bonus spawned overlapping or touching a one-sided wall (rule S-prime)')]
    chosen={}
    for key,why in pins:chosen[key]=chosen[key]+'; '+why if key in chosen else why
    assert len(chosen)==len(pins),chosen
    return chosen
def frames(stream):
    """PBE1 frames as (header, records)."""
    assert stream[:4]==b'PBE1';at=4;out=[]
    while at<len(stream):
        head=list(struct.unpack_from('<4I',stream,at));at+=16;records=[]
        for _ in range(head[2]+head[3]):
            size,=struct.unpack_from('<I',stream,at);at+=4;records.append(stream[at:at+size]);at+=size
        out.append((head,records))
    return out
def explosions(stream):
    """Per explosion (tag-12 Action record with action 19): the tag-16 sub-tag 10 sight
    records whose target is that barrel (P_CheckSight (thing, bombspot)) and the tag-13
    Damage records whose inflictor is that barrel, up to the next explosion of the frame
    or the frame's end, counted in the frame's last family."""
    count=sights=damage=0
    for head,records in frames(stream):
        spot=None
        for record in records[:head[2]]:
            if record[:2]==bytes([12,2])and record[14:17]==b'\1'+struct.pack('<H',EXPLODE):count+=1;spot=record[2:14];continue
            if spot is None:continue
            if record[:2]==bytes([16,10])and record[14:26]==spot:sights+=1
            if record[:2]==bytes([13,2])and record[14:27]==b'\1'+spot:damage+=1
    return count,sights,damage
def run(check):
    lines=unpack((ROOT/'tests/reference/m3h9b_trace.dref').read_bytes()).decode().splitlines()
    assert lines.pop(0)=='DOOM-PLAYER-BARRELS-v1'
    raw=[[r[0],*map(int,r[1:])]for r in csv.reader(lines)if r[0]!='TI']
    cases=list(csv.DictReader((ROOT/'tests/reference/m3h9b_cases.csv').open()))
    contract=json.loads((ROOT/'tests/reference/m3h9b_contract.json').read_text());maps=[ROOT/p for p in contract['maps']]
    # Every scenario is medium and every player owns the fist and the pistol (the DSIM
    # weapon layout requires both; STATE_FORMAT, DSIM v23), so every scenario is projected.
    assert all(int(c['skill'])==MEDIUM for c in cases)and len(cases)==contract['inventory']['scenarios']
    assert all(r[5]&3==3 for r in raw if r[0]=='WX')
    rows=barrels_snapshot.strip(raw)
    chosen=fixtures(rows,cases)
    outputs,inventory=barrels_snapshot.build(cases,rows,maps,'m3h9b',set(chosen))
    events=outputs['tests/reference/m3h9b_events.bin']
    captures={(r[1],r[2])for r in rows if r[0]=='P'}
    # Decision F9: every capture is byte for byte the DSIM36 projection of the same rows
    # except the version, the profile value, the context bytes and store.max_slots; the
    # event stream is identical. Within the DSIM36 ranges (at most 16 items, bonus 96)
    # the baseline is the frozen e1m1_snapshot.build itself; outside them (the dense64
    # room) it is the same build with only the item and bonus ranges lifted, and the
    # frozen build is shown to refuse those rows.
    def within(case):
        return all(r[4]<=OLD_ITEMS for r in rows if r[1]==case and r[0]in('LS','IT'))and \
               all(r[3]<=OLD_BONUS and r[4]<=OLD_ITEMS for r in rows if r[1]==case and r[0]=='PX')
    inside=[c for c in range(len(cases))if within(c)];outside=[cases[c]['label']for c in range(len(cases))if not within(c)]
    assert outside==['dense_pickups','intercept_heavy_trace'],outside
    every37,_=barrels_snapshot.build(cases,rows,maps,'m3h9b',captures)
    scaled36,_=barrels_snapshot.dsim36_scaled(cases,rows,maps,'m3h9b',captures)
    sub={o:i for i,o in enumerate(inside)}
    pure36,_=e1m1_snapshot.build([cases[c]for c in inside],[[r[0],sub[r[1]],*r[2:]]for r in rows if r[1]in sub],maps,'m3h9b',{(sub[c],op)for c,op in captures if c in sub})
    for c in range(len(cases)):
        if c in sub:continue
        try:e1m1_snapshot.build([cases[c]],[[r[0],0,*r[2:]]for r in rows if r[1]==c],maps,'m3h9b',set())
        except AssertionError as error:
            where=traceback.extract_tb(error.__traceback__)[-1]
            assert Path(where.filename).name in('bonus_snapshot.py','map_scale_snapshot.py')and('16'in where.line or'96'in where.line),where
        else:raise AssertionError(('the frozen DSIM36 chain admitted rows outside its ranges',cases[c]['label']))
    assert every37.pop('tests/reference/m3h9b_events.bin')==events==scaled36.pop('tests/reference/m3h9b_events.bin')
    pure_events=frames(pure36.pop('tests/reference/m3h9b_events.bin'))
    assert [(h,r)for h,r in frames(events)if h[0]in sub]==[([inside[h[0]],*h[1:]],r)for h,r in pure_events]
    extra=len(barrels_snapshot.CONTEXT)-len(e1m1_snapshot.CONTEXT);largest=0
    identity=('profile','context.schema','store.max_slots')
    for which,op in sorted(captures):
        name=f'tests/reference/m3h9b_state_{which}_{op}.bin'
        v37,f37=specials_snapshot.decode(every37[name]);v36,f36=specials_snapshot.decode(scaled36[name])
        assert v37==37 and v36==36 and [p for p,_ in f37]==[p for p,_ in f36]
        assert all(a==b or p in identity for(p,a),(_,b)in zip(f37,f36))
        d37,d36=dict(f37),dict(f36)
        assert d37['profile']==struct.pack('<I',21)and d36['profile']==struct.pack('<I',20)
        assert d37['context.schema']==barrels_snapshot.CONTEXT and d36['context.schema']==e1m1_snapshot.CONTEXT
        assert d37['store.max_slots']==struct.pack('<I',384)and d36['store.max_slots']==struct.pack('<I',320)
        assert len(every37[name])==len(scaled36[name])+extra
        if which in sub:assert pure36[f'tests/reference/m3h9b_state_{sub[which]}_{op}.bin']==scaled36[name]
        occupied=sum(1 for p,v in f37 if p.startswith('actor.')and p.endswith('.occupied')and v==b'\1');largest=max(largest,occupied)
        assert occupied<=384
        if (which,op)in chosen:assert outputs[name]==every37[name]
    index_lines=outputs['tests/reference/m3h9b_snapshots.csv'].decode().splitlines()
    for a,b,c in zip(index_lines[1:],every37['tests/reference/m3h9b_snapshots.csv'].decode().splitlines()[1:],scaled36['tests/reference/m3h9b_snapshots.csv'].decode().splitlines()[1:],strict=True):
        a,b,c=a.split(','),b.split(','),c.split(',')
        assert a[:4]==b[:4]and a[5:]==b[5:]and a[:3]==c[:3]and a[6:]==c[6:]and int(a[5])==int(c[5])+extra
        assert (a[4]!='-')==((int(a[0]),int(a[1]))in chosen)
    for path,data in outputs.items():
        if path.endswith('.bin')and'_state_'in path:
            version,fields=specials_snapshot.decode(data);value=dict(fields)
            assert version==37 and value['profile']==struct.pack('<I',21)and value['context.schema']==barrels_snapshot.CONTEXT
    # The explosion's event mapping: one Action 19 record per A_Explode row, one sight
    # record per candidate that reached P_CheckSight (RC codes 3 and 4) and one Damage
    # record per damaged candidate (RC code 4), each G row of PIT_RadiusAttack's call.
    projected=raw
    rc=collections.Counter(r[6]for r in projected if r[0]=='RC')
    explode_rows=sum(1 for r in projected if r[0]=='E'and r[3]==5 and r[5]==EXPLODE)
    assert explode_rows==sum(1 for r in projected if r[0]=='RA')
    count,sights,damage=explosions(events)
    assert (count,sights,damage)==(explode_rows,rc[3]+rc[4],rc[4]),(count,sights,damage,explode_rows,rc)
    events=outputs.pop('tests/reference/m3h9b_events.bin')
    outputs['tests/reference/m3h9b_events.dref']=pack(events)
    assert unpack(outputs['tests/reference/m3h9b_events.dref'])==events
    fixture_names={f'm3h9b_state_{c}_{o}.bin':why for (c,o),why in sorted(chosen.items())}
    inventory.update(expanded_event_bytes=len(events),packed_event_bytes=len(outputs['tests/reference/m3h9b_events.dref']),
      new_event_tags=0,new_fields=0,largest_occupied_slots=largest,
      explosions=count,explosion_sight_records=sights,explosion_damage_records=damage,
      radius_candidates={('not_shootable','boss','out_of_range','sight_blocked','damaged')[k]:rc[k]for k in range(5)},
      projected_scenarios=[c['label']for c in cases],
      scenarios_beyond_dsim36_ranges=outside,
      fixtures=fixture_names)
    barrel=dict(doomednum=barrels_snapshot.BARREL,mobjtype_index=barrels_snapshot.MT_BARREL,
                alive_states=sorted(barrels_snapshot.ALIVE),dying_states=sorted(barrels_snapshot.DYING),gibs_state=barrels_snapshot.S_GIBS,null_state=barrels_snapshot.S_NULL,
                radius=10,height=42,dead_height_fixed=(42*65536)>>2,health_max=20,reaction_time=barrels_snapshot.REACTION_TIME,threshold=barrels_snapshot.BASETHRESHOLD,
                flags_alive=barrels_snapshot.MF_SOLID|barrels_snapshot.MF_SHOOTABLE|barrels_snapshot.MF_NOBLOOD,
                flags_dying=barrels_snapshot.MF_SOLID|barrels_snapshot.MF_NOBLOOD|barrels_snapshot.MF_CORPSE|barrels_snapshot.MF_DROPOFF,
                flags_gibbed=barrels_snapshot.MF_NOBLOOD|barrels_snapshot.MF_CORPSE|barrels_snapshot.MF_DROPOFF,ambush_flag=barrels_snapshot.MF_AMBUSH)
    sha=lambda data:hashlib.sha256(data).hexdigest()
    inputs=sorted({'tools/reference/'+Path(m.__file__).name for m in list(sys.modules.values())if getattr(m,'__file__',None)and Path(m.__file__).resolve().parent==ROOT/'tools/reference'})
    inputs+=['tools/reference/m3h9b_snapshot_goldens.py','linuxdoom-1.10/info.h','tests/reference/m3h9b_trace.dref','tests/reference/m3h9b_cases.csv','tests/reference/m3h9b_contract.json']+contract['maps']
    inputs=sorted(set(inputs))
    outputs['tests/reference/m3h9b_snapshot_contract.json']=(json.dumps(dict(
      schema='DSIM-v37',execution_profile=barrels_snapshot.POLICY,context=barrels_snapshot.CONTEXT.decode(),store_slots=barrels_snapshot.MAX_SLOTS,
      pickup_limit=barrels_snapshot.MAX_PICKUPS,bonus_limit=barrels_snapshot.MAX_BONUS,barrel_limit=barrels_snapshot.MAX_BARRELS,
      decoration_limit=barrels_snapshot.MAX_DECORATIONS,monster_limit=barrels_snapshot.MAX_MONSTERS,live_projectile_bound=barrels_snapshot.MAX_LIVE_PROJECTILES,
      barrel=barrel,**inventory,
      fixtures_sha256={name:sha(outputs['tests/reference/'+name])for name in fixture_names},
      field_count_rule='Each snapshots.csv row gives the capture digest (FNV-1a 64 of the full DSIM bytes), its fixture name or -, its byte size, its field count and its ordered event-field count. The field count and event-field count equal the DSIM36 projection of the same rows; the byte size is the DSIM36 size plus 3 (the longer context.schema). No field is added or removed.',
      comparison='Every capture equals the DSIM36 projection (e1m1_snapshot.build) of the same stripped rows field for field and byte for byte except the version (37), the profile value (21), the context.schema bytes and store.max_slots (384 for 320); the PBE1 event stream is identical. For the scenarios inside the DSIM36 ranges the baseline is the frozen build unchanged; for scenarios_beyond_dsim36_ranges it is the frozen build with only the 16-item and 96-bonus range assertions of map_scale_snapshot.build and bonus_snapshot.events lifted (the frozen build refuses those rows, asserted). No other field differs. Decision F9: no new field, no new event.',
      event_mapping='An explosion is the existing tag-12 Entered record of S_BEXP4 and the tag-12 Action record (original action 19, A_Explode), then for each P_RadiusAttack candidate in walk order: nothing for a candidate that is not shootable or out of range, the existing tag-16 sub-tag 10 sight record for a candidate that reaches P_CheckSight (RC codes 3 and 4), and for a damaged candidate (code 4) the existing tag-13 Damage record (inflictor the barrel, source its target or absent) followed by the Knockback/Health/Pain/Kill/DeathTimer/Retarget/Random records of P_DamageMobj, P_KillMobj and the victims state entries, exactly as the inherited chain encodes the C G/H/K/E rows. The RA and RC observer rows are comparison rows, not state and not events, and are removed before the chain.',
      dropped_rows=dict(RA='one per P_RadiusAttack: comparison only',RC='one per PIT_RadiusAttack candidate: comparison only',DT='M3h9 observer',XT='M3h9 observer',NS='M3h9 observer',TI='trace index rows'),
      scope='Original C fields and source event order, independent schema/birth-generation projection. Bounded command/weapon stage then original thinkers, not full player/ticker execution. DSIM36 layout and event vocabulary under version 37, policy 21 and the barrels context, store.max_slots 384, items and total items at most 64 and bonus count at most 384, with exploding barrels (DoomEd 2035, MT_BARREL) as ordinary actor records checked against the C rows and the documented chain: S_BAR1/S_BAR2 alive with health 1..20, radius 10, height 42 and SOLID|SHOOTABLE|NOBLOOD; S_BEXP..S_BEXP5 dying (and S_NULL pending removal) with health <= 0, height 42 >> 2 and SOLID|NOBLOOD|CORPSE|DROPOFF; S_GIBS crunched with radius and height 0 and SOLID clear; MF_AMBUSH from the THING record; threshold 0 or 100 with target present exactly at 100; reaction_time 8 or 0 (P_DamageMobj on a survivor); spawnpoint from the THING record; at most 16 per capture; every MT_BARREL actor a spawned 2035 record. Every one of the 26 scenarios is encoded. No Rust observations.',
      inputs_sha256={p:sha((ROOT/p).read_bytes())for p in inputs},
      outputs_sha256={p:sha(data)for p,data in outputs.items()}),indent=2,sort_keys=True)+'\n').encode()
    for path,data in outputs.items():
        if check:assert (ROOT/path).read_bytes()==data,path
        else:(ROOT/path).write_bytes(data)
    print(('Checked'if check else'Generated')+' DSIM37 barrels, the radius attack, pickup scale and angles: '+str({k:v for k,v in inventory.items()if k not in('fixtures','projected_scenarios')}))
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');run(p.parse_args().check)
