#!/usr/bin/env python3
"""Genuine original-C movement, sight, hitscan, fireballs and noise across static heights.

Reuses the compiled M3h2 bodies, observers and controlled inputs unchanged over rooms
of axis-aligned sectors with two-sided lines and differing floor/ceiling heights.
Expectations come from compiled original bodies over synthetic inputs; they never
consume Rust output."""
import argparse,collections,csv,hashlib,io,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3h2_reference as inherited
import m3h4_geometry as geometry
from m3h4_cases import ROOMS,populate
from evidence_pack import pack,unpack
previous=inherited.previous;core=inherited.core
LAYOUTS=populate(previous.pack)
OOF=34 # sfx_oof in sounds.h
def configure():
    core.main.MAPS,core.main.THINGS,core.main.KEYS,core.main.CASES=[],[],[],[]
    for c in core.CASES:
        spans,heights,portals,reject=ROOMS[c['room']]
        m=geometry.room(spans,heights,portals=portals,reject=reject)
        core.main.case(c['label'],n=1,custom=m,things=LAYOUTS[c['layout']],tics=c['tics'],rejected=bool(c['rejected']),
                       **{'pc_'+k:v for k,v in c.items()if k not in['label','tics','map']})
        c['map']=core.main.CASES[-1]['map']
core.configure=configure
sha=lambda b:hashlib.sha256(b).hexdigest()
def run(check,overflow,address):
    flags=list(previous.pack.legacy.base.FLAGS)
    observed_trace,_=previous.capture(flags)
    for enabled,sanitizer in [(overflow,'signed-integer-overflow,integer-divide-by-zero,float-cast-overflow'),(address,'address,undefined')]:
        if enabled:
            audited,_=previous.capture([f for f in flags if f!='-fwrapv']+['-fno-wrapv','-fsanitize='+sanitizer,'-fno-sanitize-recover=all'])
            assert audited['m3d3j_trace.csv']==observed_trace['m3d3j_trace.csv']
    rows=list(csv.reader(io.StringIO(observed_trace['m3d3j_trace.csv'].decode())))
    functions=collections.Counter(r[3]for r in rows if r[0]=='FN')
    selected={0,1,2,4,8,16,32,48,64,96,128,192,256,320,384,448,512,576,640,704,768,832,896,960}
    # Every damage, kill, retarget, launch, explosion, slide, drop, pickup, hitscan, noise flood,
    # landing sound, height-reason refusal and floor-contact change is captured, so steps,
    # falls, landings and openings are observed as actor fields, not inferred.
    landed=lambda r:r[0]=='E'and r[3]=='7'and r[5]==str(OOF)
    height_refusal=lambda r:r[0]=='M'and r[7]=='0'and r[12]in{'3','4','5','6'}
    contact={};changes=set()
    for r in rows:
        if r[0]=='Z':
            key=(r[1],r[3])
            if contact.get(key)is not None and contact[key]!=r[10]:changes.add((int(r[1]),int(r[2])))
            contact[key]=r[10]
    interesting={(int(r[1]),int(r[2]))for r in rows if (r[0]=='G'and r[3]in{'2','7','10'})or r[0]in{'AR','ST','PN','SL','X','N'}or(r[0]=='MS'and r[3]in{'0','5'})or(r[0]=='E'and r[3]=='1'and r[2]!='0')or landed(r)or height_refusal(r)}|changes
    keys={(int(r[1]),int(r[2]))for r in rows if r[0]=='P'and(int(r[2])in selected or tuple(map(int,r[1:3]))in interesting or int(r[2])==core.CASES[int(r[1])]['tics'])}
    kept=[r for r in rows if r[0]!='FN'and(r[0]not in{'A','P','AP','R','S','LS'}or tuple(map(int,r[1:3]))in keys)]
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(kept)
    expanded=('DOOM-PLAYER-HEIGHTS-v1\n'+text.getvalue()).encode()
    outputs={'m3h4_trace.dref':pack(expanded)}
    assert unpack(outputs['m3h4_trace.dref'])==expanded
    text=io.StringIO();writer=csv.writer(text,lineterminator='\n');writer.writerow(['case',*core.CASES[0]]);writer.writerows([i,*c.values()]for i,c in enumerate(core.CASES))
    outputs['m3h4_cases.csv']=text.getvalue().encode()
    maps=[]
    for i,m in enumerate(core.main.MAPS):
        name=f'm3h4_geometry_{i}.bin';outputs[name]=m['bytes'];maps.append('tests/reference/'+name)
    outputs['m3h4_maps.rs']=('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("{Path(p).name}"),\n'for p in maps)+'];\n').encode()
    manifest=json.loads(observed_trace['m3d3j_provenance.json'])
    bodies=manifest['functions']+[['p_pspr.c',n,'void']for n in previous.pack.WEAPON_ACTIONS]
    assert len(bodies)==len({tuple(b)for b in bodies})
    types={}
    for r in rows:
        if r[0]=='A'and r[4]=='1':types.setdefault((r[1],r[3]),int(r[5]))
    births={(r[1],r[4]):int(r[5])for r in rows if r[0]=='E'and r[3]=='1'}
    kind=lambda r,i:births.get((r[1],r[i]),types.get((r[1],r[i])))
    monster_types={1,2,10,11,12}
    kills=[r for r in kept if r[0]=='G'and r[3]=='7']
    retargets=[r for r in kept if r[0]=='G'and r[3]=='10']
    damages=[r for r in kept if r[0]=='G'and r[3]=='2']
    counter=lambda tag,code=None:sum(r[0]==tag and(code is None or r[3]==str(code))for r in kept)
    missile_damage=[r for r in damages if kind(r,5)==31]
    reasons=collections.Counter(r[12]for r in kept if r[0]=='M'and r[7]=='0')
    zrows=[r for r in kept if r[0]=='Z']
    explosions={(r[1],r[2],r[4])for r in kept if r[0]=='MS'and r[3]=='0'}
    floor_explosions=sum(1 for r in zrows if r[10]=='1'and(r[1],r[2],r[3])in explosions)
    ceiling_explosions=sum(1 for r in zrows if r[11]=='1'and(r[1],r[2],r[3])in explosions)
    live=collections.Counter()
    for r in kept:
        if r[0]=='A'and r[4]=='1'and int(r[5])==31:live[(r[1],r[2])]+=1
    per_case=[]
    for which,c in enumerate(core.CASES):
        mine=[r for r in kept if r[1]==str(which)]
        spans,heights,portals,reject=ROOMS[c['room']]
        mine_z=[r for r in mine if r[0]=='Z']
        per_case.append(dict(label=c['label'],spans=spans,heights=heights,portals={str(k):v for k,v in portals.items()},reject=[list(p)for p in reject],
          launches=sum(r[0]=='MS'and r[3]=='5'for r in mine),explosions=sum(r[0]=='MS'and r[3]=='0'for r in mine),
          damage=sum(r[0]=='G'and r[3]=='2'for r in mine),kills=sum(r[0]=='G'and r[3]=='7'for r in mine),slides=sum(r[0]=='SL'for r in mine),
          blocked_moves=sum(r[0]=='M'and r[7]=='0'for r in mine),refusals={k:sum(r[0]=='M'and r[7]=='0'and r[12]==k for r in mine)for k in['1','2','3','4','5','6']},
          vertical_steps=len(mine_z),landings=sum(landed(r)for r in mine),falls=sum(r[12]=='1'for r in mine_z),ceiling_contacts=sum(r[11]=='1'for r in mine_z),
          floor_contacts=sum(r[10]=='1'for r in mine_z),min_momz=min([int(r[7])for r in mine_z]or[0]),
          sight_queries=sum(r[0]=='Q'for r in mine),sight_visible=sum(r[0]=='Q'and r[4]=='1'for r in mine),
          hitscans=sum(r[0]=='X'and r[3]=='0'for r in mine),noise_floods=sum(r[0]=='N'for r in mine),
          max_abs_z=max([abs(int(r[8]))//65536 for r in mine if r[0]=='A'and r[4]=='1']or[0])))
    inventory=dict(scenarios=len(core.CASES),scripted_tics=sum(c['tics']for c in core.CASES),captures=len(keys),rooms=len(core.main.MAPS),
      sectors=sum(len(m['heights'])for m in core.main.MAPS),two_sided_lines=sum(l['back']is not None for m in core.main.MAPS for l in m['lines']),
      selected_original_bodies=len(bodies),exercised_unique_functions=len(functions),function_entries=sum(functions.values()),
      point_in_subsector=functions['r_main.c:R_PointInSubsector'],line_opening=functions['p_maputl.c:P_LineOpening'],z_movement=functions['p_mobj.c:P_ZMovement'],
      cross_subsector=functions['p_sight.c:P_CrossSubsector'],recursive_sound=functions['p_enemy.c:P_RecursiveSound'],slide_entries=functions['p_map.c:P_SlideMove'],
      vertical_steps=len(zrows),floor_contacts=sum(r[10]=='1'for r in zrows),ceiling_contacts=sum(r[11]=='1'for r in zrows),falls=sum(r[12]=='1'for r in zrows),
      landing_sounds=sum(landed(r)for r in kept),blocked_moves=sum(reasons.values()),refusal_reasons={k:reasons[k]for k in['1','2','3','4','5','6']},
      floor_explosions=floor_explosions,ceiling_explosions=ceiling_explosions,fireball_launches=counter('MS',5),fireball_explosions=counter('MS',0),
      fireball_damage_requests=len(missile_damage),fireball_player_hits=sum(kind(r,4)==0 for r in missile_damage),
      sight_queries=counter('Q'),sight_visible=sum(r[0]=='Q'and r[4]=='1'for r in kept),sight_rejected=sum(r[0]=='V'and r[4]=='0'and r[8]=='1'for r in kept),
      hitscans=sum(r[0]=='X'and r[3]=='0'for r in kept),hitscan_line_impacts=sum(r[0]=='X'and r[3]=='1'and r[4]=='1'for r in kept),
      noise_floods=counter('N'),noise_through_soundblock=sum(r[0]=='N'and r[4]=='1'for r in kept),
      monster_kills=sum(kind(r,4)in monster_types for r in kills),player_deaths=sum(kind(r,4)==0 for r in kills),
      infighting_retargets=sum(kind(r,4)in monster_types and kind(r,5)in monster_types for r in retargets),kill_credits=counter('ST',1),
      drops=sum(r[0]=='E'and r[3]=='1'and r[2]!='0'and kind(r,4)in{63,73,77}for r in kept),damage_requests=len(damages),slide_observations=counter('SL'),
      peak_live_fireballs=max(live.values(),default=0),actor_fields=26*sum(r[0]=='A'and r[4]=='1'for r in kept),gameplay_draws=counter('E',8),
      expanded_trace_bytes=len(expanded),packed_trace_bytes=len(outputs['m3h4_trace.dref']))
    inputs=sorted({Path(m.__file__).name for m in list(sys.modules.values()) if getattr(m,'__file__',None) and Path(m.__file__).resolve().parent==ROOT/'tools/reference'})
    contract=dict(schema='DOOM-PLAYER-HEIGHTS-v1',revision=manifest['revision'],flags=manifest['flags'],inventory=inventory,cases=per_case,maps=maps,functions=bodies,
      source_sha256={n:sha((ROOT/'linuxdoom-1.10'/n).read_bytes())for n in sorted(set(manifest['source_sha256'])|{'d_items.c','dstrings.h'})},
      function_sha256={f'{f}:{n}':sha(core.raw_body(f,n,t).encode())for f,n,t in bodies},
      executed_function_entries=dict(sorted(functions.items())),
      inputs_sha256={'tools/reference/'+n:sha((ROOT/'tools/reference'/n).read_bytes())for n in inputs},
      outputs_sha256={n:sha(d)for n,d in outputs.items()},expanded_trace_sha256=sha(expanded),assumptions=[
       'Genuine player0 plus at most eight map-THING MT_TROOP/MT_POSSESSED/MT_SHOTGUY/MT_CHAINGUY/MT_SERGEANT in walled rooms of axis-aligned sectors with two-sided lines and static floor/ceiling heights; no Rust expected observations.',
       'Every compiled body, observer and controlled input of M3h2 is reused unchanged; the geometry is a new synthetic room builder (sector strips, one subsector per sector, guillotine BSP, cell-exact BLOCKMAP), not an original level loader. The adapter installs sector line lists, sidedef sectors, segs, subsectors, node partitions and REJECT from the generated lumps.',
       'Steps, falls, landings, height refusals (P_TryMove reasons 3-6), slides against two-sided lines, sight and noise through openings, hitscan impacts on upper and lower walls and fireball floor/ceiling explosions are source observations of the compiled bodies; their admission in Rust is the new policy, not a source claim. No line or sector special, moving sector or sky ceiling exists in any room.',
       'The landing sound is observed as the S_StartSound stub call (E,7 rows); view height and bobbing are source side effects without physical outputs.',
       'Bounded original command/weapon stage precedes original thinkers; not complete P_Ticker/PlayerThink/DeathThink or respawn.',
       'All M3h1/M3h2 assumptions and adapters apply, including the strict-build retyping of attack-angle int locals; MS/SL rows only observe genuine statements.'])
    outputs['m3h4_contract.json']=(json.dumps(contract,indent=2,sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check:assert path.read_bytes()==data,path
        else:path.write_bytes(data)
    print(('Checked'if check else'Generated')+' original heights: '+json.dumps(inventory)+f'; strict={overflow}; ASan/UBSan={address}')
    return per_case
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--overflow-check',action='store_true');parser.add_argument('--address-check',action='store_true');parser.add_argument('--cases',action='store_true');args=parser.parse_args()
    table=run(args.check,args.overflow_check,args.address_check)
    if args.cases:
        for row in table:print(json.dumps(row))
