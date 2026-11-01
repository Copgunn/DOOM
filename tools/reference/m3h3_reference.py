#!/usr/bin/env python3
"""Genuine original-C monsters, fireballs and the player at real-map scale.

Reuses the compiled M3h2 bodies and observers unchanged over walled rectangles
up to the 2,048-unit vertex envelope. Expectations come from compiled original
bodies over synthetic inputs; they never consume Rust output."""
import argparse,collections,csv,hashlib,io,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3h2_reference as inherited
import m3h3_geometry as geometry
from m3h3_cases import ROOMS,populate
from evidence_pack import pack,unpack
previous=inherited.previous;core=inherited.core
LAYOUTS=populate(previous.pack)
def configure():
    core.main.MAPS,core.main.THINGS,core.main.KEYS,core.main.CASES=[],[],[],[]
    for c in core.CASES:
        rect,pillars=ROOMS[c['room']]
        m=geometry.room(rect,pillars)
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
    # Every damage, kill, retarget, launch, explosion, slide, drop and pickup tic is captured,
    # so near-wall impulses and far coordinates are observed as actor fields, not inferred.
    interesting={(int(r[1]),int(r[2]))for r in rows if (r[0]=='G'and r[3]in{'2','7','10'})or r[0]in{'AR','ST','PN','SL'}or(r[0]=='MS'and r[3]in{'0','5'})or(r[0]=='E'and r[3]=='1'and r[2]!='0')}
    keys={(int(r[1]),int(r[2]))for r in rows if r[0]=='P'and(int(r[2])in selected or tuple(map(int,r[1:3]))in interesting or int(r[2])==core.CASES[int(r[1])]['tics'])}
    kept=[r for r in rows if r[0]!='FN'and(r[0]not in{'A','P','AP','R','S','LS'}or tuple(map(int,r[1:3]))in keys)]
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(kept)
    expanded=('DOOM-PLAYER-MAP-SCALE-v1\n'+text.getvalue()).encode()
    outputs={'m3h3_trace.dref':pack(expanded)}
    assert unpack(outputs['m3h3_trace.dref'])==expanded
    text=io.StringIO();writer=csv.writer(text,lineterminator='\n');writer.writerow(['case',*core.CASES[0]]);writer.writerows([i,*c.values()]for i,c in enumerate(core.CASES))
    outputs['m3h3_cases.csv']=text.getvalue().encode()
    maps=[]
    for i,m in enumerate(core.main.MAPS):
        name=f'm3h3_geometry_{i}.bin';outputs[name]=m['bytes'];maps.append('tests/reference/'+name)
    outputs['m3h3_maps.rs']=('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("{Path(p).name}"),\n'for p in maps)+'];\n').encode()
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
    actors=collections.defaultdict(dict)
    for r in kept:
        if r[0]=='A'and r[4]=='1':actors[(r[1],r[2])][r[3]]=(int(r[6])//65536,int(r[7])//65536,int(r[5]))
    def near_wall(r):
        rect=ROOMS[core.CASES[int(r[1])]['room']][0];at=actors.get((r[1],r[2]),{}).get(r[4])
        if at is None:return False
        x,y,_=at;return min(x-rect[0],rect[2]-x,y-rect[1],rect[3]-y)<=36
    coordinates=[abs(v)for group in actors.values()for x,y,_ in group.values()for v in(x,y)]
    live=collections.Counter((case,op)for(case,op),group in actors.items()for _,_,t in group.values()if t==31)
    per_case=[]
    for which,c in enumerate(core.CASES):
        mine=[r for r in kept if r[1]==str(which)]
        rect=ROOMS[c['room']][0]
        per_case.append(dict(label=c['label'],room=list(rect),launches=sum(r[0]=='MS'and r[3]=='5'for r in mine),explosions=sum(r[0]=='MS'and r[3]=='0'for r in mine),
          damage=sum(r[0]=='G'and r[3]=='2'for r in mine),near_wall_damage=sum(r[0]=='G'and r[3]=='2'and near_wall(r)for r in mine),
          slides=sum(r[0]=='SL'for r in mine),blocked_moves=sum(r[0]=='M'and r[4]=='0'for r in mine),kills=sum(r[0]=='G'and r[3]=='7'for r in mine),
          peak_live_fireballs=max([live[(str(which),r[2])]for r in mine if r[0]=='P']or[0]),
          max_abs_coordinate=max([abs(v)for(case,_),group in actors.items()if case==str(which)for x,y,_ in group.values()for v in(x,y)]or[0])))
    inventory=dict(scenarios=len(core.CASES),scripted_tics=sum(c['tics']for c in core.CASES),captures=len(keys),
      selected_original_bodies=len(bodies),exercised_unique_functions=len(functions),function_entries=sum(functions.values()),
      imp_attacks=functions['p_enemy.c:A_TroopAttack'],spawn_missile=functions['p_mobj.c:P_SpawnMissile'],explode_missile=functions['p_mobj.c:P_ExplodeMissile'],
      slide_entries=functions['p_map.c:P_SlideMove'],hit_slide_line_entries=functions['p_map.c:P_HitSlideLine'],
      fireball_launches=counter('MS',5),fireball_explosions=counter('MS',0),fireball_damage_requests=len(missile_damage),
      fireball_player_hits=sum(kind(r,4)==0 for r in missile_damage),fireball_monster_hits=sum(kind(r,4)in monster_types for r in missile_damage),
      monster_kills=sum(kind(r,4)in monster_types for r in kills),imp_kills=sum(kind(r,4)==11 for r in kills),player_deaths=sum(kind(r,4)==0 for r in kills),
      infighting_retargets=sum(kind(r,4)in monster_types and kind(r,5)in monster_types for r in retargets),
      kill_credits=counter('ST',1),drops=sum(r[0]=='E'and r[3]=='1'and r[2]!='0'and kind(r,4)in{63,73,77}for r in kept),drop_touches=sum(r[0]=='PN'and r[3]=='0'and kind(r,4)in{63,73,77}for r in kept),
      damage_requests=len(damages),near_wall_damage_requests=sum(near_wall(r)for r in damages),slide_observations=counter('SL'),blocked_moves=sum(r[0]=='M'and r[4]=='0'for r in kept),
      peak_live_fireballs=max(live.values(),default=0),max_abs_coordinate=max(coordinates,default=0),captures_beyond_994=sum(any(abs(v)>994 for x,y,_ in g.values()for v in(x,y))for g in actors.values()),
      actor_fields=26*sum(r[0]=='A'and r[4]=='1'for r in kept),gameplay_draws=counter('E',8),
      expanded_trace_bytes=len(expanded),packed_trace_bytes=len(outputs['m3h3_trace.dref']))
    inputs=sorted({Path(m.__file__).name for m in list(sys.modules.values()) if getattr(m,'__file__',None) and Path(m.__file__).resolve().parent==ROOT/'tools/reference'})
    contract=dict(schema='DOOM-PLAYER-MAP-SCALE-v1',revision=manifest['revision'],flags=manifest['flags'],inventory=inventory,cases=per_case,maps=maps,functions=bodies,
      source_sha256={n:sha((ROOT/'linuxdoom-1.10'/n).read_bytes())for n in sorted(set(manifest['source_sha256'])|{'d_items.c','dstrings.h'})},
      function_sha256={f'{f}:{n}':sha(core.raw_body(f,n,t).encode())for f,n,t in bodies},
      executed_function_entries=dict(sorted(functions.items())),
      inputs_sha256={'tools/reference/'+n:sha((ROOT/'tools/reference'/n).read_bytes())for n in inputs},
      outputs_sha256={n:sha(d)for n,d in outputs.items()},expanded_trace_sha256=sha(expanded),assumptions=[
       'Genuine player0 plus at most eight map-THING MT_TROOP/MT_POSSESSED/MT_SHOTGUY/MT_CHAINGUY/MT_SERGEANT in walled rectangles whose vertices reach 2,048 units, including an offset room; no Rust expected observations.',
       'Every compiled body, observer and controlled input of M3h2 is reused unchanged; the geometry is a new synthetic room builder (one flat sector, axis walls split at 1,024 units, cell-exact BLOCKMAP with the rectangle minimum as origin), not an original level loader.',
       'The original has no actor coordinate limit: the three stall scenarios continue where the historical Rust envelope rejected. Blocked moves, slides and wall-adjacent impulses are source observations; their non-rejection in Rust is the new policy, not a source claim.',
       'Bounded original command/weapon stage precedes original thinkers; not complete P_Ticker/PlayerThink/DeathThink or respawn.',
       'All M3h1/M3h2 assumptions and adapters apply, including the strict-build retyping of attack-angle int locals; MS/SL rows only observe genuine statements.'])
    outputs['m3h3_contract.json']=(json.dumps(contract,indent=2,sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check:assert path.read_bytes()==data,path
        else:path.write_bytes(data)
    print(('Checked'if check else'Generated')+' original map scale: '+json.dumps(inventory)+f'; strict={overflow}; ASan/UBSan={address}')
    return per_case
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--overflow-check',action='store_true');parser.add_argument('--address-check',action='store_true');parser.add_argument('--cases',action='store_true');args=parser.parse_args()
    table=run(args.check,args.overflow_check,args.address_check)
    if args.cases:
        for row in table:print(row)
