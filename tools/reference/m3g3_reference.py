#!/usr/bin/env python3
"""Compiled original player thrust/grounded movement; no Rust expected outputs."""
import argparse
import collections
import contextlib
import csv
import hashlib
import io
import json
import shutil
import tempfile
from pathlib import Path
import m3g2_reference as player

ROOT = Path(__file__).resolve().parents[2]
main, legacy = player.main, player.legacy
SCHEMA = 'DOOM-PLAYER-KNOCKBACK-v1'
CASES = []
def case(label, tics=48, **values):
    c = dict(label=label,tics=tics,skip=0,ix=-96,iy=0,px=0,py=0,
             extra=0,source=-1,physical=1,amount=20,repeat=0,pre=0,retire=0)
    c.update(values);CASES.append(c)
case('no_inflictor',physical=0)
case('inflictor_without_source')
case('same_source_west',ix=96,source=0)
case('distinct_source',extra=1,source=1)
case('north',ix=0,iy=-48,py=16)
case('south',ix=0,iy=48,py=-16)
case('diagonal',ix=-64,iy=-48,py=16)
case('accumulation',tics=56,source=0,repeat=1)
case('wall_slide',ix=0,iy=-32,px=96,amount=32)
case('solid_dying_blocker',tics=64,extra=2,source=0,amount=100,repeat=2)
case('positive_subdivision',tics=72,amount=144,repeat=2)
case('negative_subdivision',tics=72,ix=96,amount=144,repeat=2)
case('extreme_moving_corpse',tics=72,amount=120,pre=99,source=0,repeat=2)
case('zero_physical',tics=12,amount=0)
case('nonpain_run_stop',tics=24,amount=1,source=0,
     skip=(player.rng.index(255)-4)%256)
case('self_inflictor',physical=2,source=2)
case('moving_body_retirement',tics=8,retire=1)

main.MAPS,main.THINGS,main.KEYS,main.CASES=[],[],[],[]
for c in CASES:
    main.case(c['label'],n=1,tics=c['tics'],skip=c['skip'],
              **{'kb_'+k:v for k,v in c.items() if k not in ['label','tics','skip']})
assert main.MAPS[0]['bytes']==(ROOT/'tests/reference/m3f5_map_0.bin').read_bytes()
# Keep the source association fragment and original setup/filter/credit probes.
# New gameplay inputs use raw original spawns, not P_LoadThings/map-count claims.
legacy.MAIN=player.MAIN[:player.MAIN.index('for(c=0;c<CASE_COUNT;c++)')]+r'''
for(c=0;c<CASE_COUNT;c++){int* v=input[c];mobj_t *origin,*other=NULL,*body,*inflictor,*source;
reset(c);printf("SEED,%d,0,0\n",c);admit_native=0;
for(i=0;i<v[V_SKIP];i++){int draw=P_Random();observe(9,-1,draw);}
origin=P_SpawnMobj(v[V_KB_IX]*FRACUNIT,v[V_KB_IY]*FRACUNIT,ONFLOORZ,MT_SERGEANT);
if(v[V_KB_EXTRA])other=P_SpawnMobj((v[V_KB_EXTRA]==1?-96:64)*FRACUNIT,(v[V_KB_EXTRA]==1?40:0)*FRACUNIT,ONFLOORZ,MT_SERGEANT);
players[0].health=100;players[0].readyweapon=wp_pistol;
body=P_SpawnMobj(v[V_KB_PX]*FRACUNIT,v[V_KB_PY]*FRACUNIT,ONFLOORZ,MT_PLAYER);associate(body);
P_DamageMobj(origin,NULL,NULL,150);
if(other)P_DamageMobj(other,NULL,NULL,150);
if(v[V_KB_PRE])P_DamageMobj(body,NULL,NULL,v[V_KB_PRE]);
inflictor=v[V_KB_PHYSICAL]==0?NULL:v[V_KB_PHYSICAL]==2?body:origin;
source=v[V_KB_SOURCE]<0?NULL:v[V_KB_SOURCE]==0?origin:v[V_KB_SOURCE]==1?other:body;
capture();
for(t=1;t<=v[V_TICS];t++){operation=t;query_index=-1;attempt=-1;
if(t==1)P_DamageMobj(body,inflictor,source,v[V_KB_AMOUNT]);
if(v[V_KB_REPEAT]==1&&t==2)P_DamageMobj(body,inflictor,source,20);
if(v[V_KB_REPEAT]==1&&t==3)P_DamageMobj(body,inflictor,source,10);
if(v[V_KB_REPEAT]==2&&t==2)P_DamageMobj(body,inflictor,source,1);
if(v[V_KB_RETIRE]&&t==3)P_RemoveMobj(body);
P_RunThinkers();leveltime++;capture();
}}
return 0;}
'''
ADDED=[('p_map.c','P_HitSlideLine','void'),('p_map.c','PTR_SlideTraverse','boolean'),('p_map.c','P_SlideMove','void')]
legacy.FUNCTIONS=[*legacy.FUNCTIONS,*ADDED]

def capture(flags):
    # Reuse the frozen player extraction runner, with an explicit service shim
    # extension; original source and every historical adapter remain unchanged.
    with tempfile.TemporaryDirectory(prefix='doom-grounded-services-') as directory:
        temp=Path(directory);(temp/'tools/reference').mkdir(parents=True)
        for p in (ROOT/'tools/reference').glob('generate*.py'):
            shutil.copyfile(p,temp/'tools/reference'/p.name)
        shim=(ROOT/'tools/reference/m3f3_shim.h').read_text()
        shim=shim.replace('void P_SlideMove(mobj_t* p){(void)p;abort();}',
            'fixed_t bestslidefrac,secondslidefrac,tmxmove,tmymove;\nline_t *bestslideline,*secondslideline;\nmobj_t* slidemo;')
        shim=shim.replace('static void record_move(', 'static int observer_query;\nstatic void record_move(')
        shim=shim.replace('subsector_t* s=R_PointInSubsector(x,y);',
            'subsector_t* s;observer_query=1;s=R_PointInSubsector(x,y);observer_query=0;')
        (temp/'tools/reference/m3f3_shim.h').write_text(shim)
        old_root,old_body=player.ROOT,player.observed_body
        player.ROOT=temp;player.observed_body=observed_body
        try:
            with contextlib.redirect_stdout(io.StringIO()):return player.capture(flags)
        finally:
            player.ROOT=old_root;player.observed_body=old_body

# Keep a stable reference because capture temporarily installs our entry observer.
_player_body=player.observed_body
# Resolve the saved frozen entry observer rather than its temporarily patched name.
def observed_body(file,name,result,rename=None):
    text=_player_body(file,name,result,rename)
    text=text.replace('printf("FN,','if(!observer_query)printf("FN,')
    if name=='P_HitSlideLine':
        text=text.replace('{','{\n printf("SL,%d,%d,%d,%ld\\n",scenario,operation,ident(slidemo),(long)(ld-lines));',1)
    return text

def run(check,overflow,address):
    flags=list(legacy.base.FLAGS);output=capture(flags);raw=output['m3d3j_trace.csv']
    for enabled,extra in [(overflow,['-fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow']),
                          (address,['-fsanitize=address,undefined'])]:
        if enabled:
            strict=capture([f for f in flags if f!='-fwrapv']+['-fno-wrapv',*extra,'-fno-sanitize-recover=all'])
            assert strict['m3d3j_trace.csv']==raw
    rows=list(csv.reader(io.StringIO(raw.decode())))
    functions=collections.Counter(r[3] for r in rows if r[0]=='FN')
    selected={0,1,2,3,4,6,7,8,12,16,24,32,40,48,56,64,72}
    keep=[r for r in rows if r[0] not in {'FN','D','B','U','I','T'} and
          (r[0] not in {'A','P','AP','R','S','LS'} or int(r[2]) in selected or int(r[2])==CASES[int(r[1])]['tics'])]
    trace=io.StringIO();csv.writer(trace,lineterminator='\n').writerows(keep)
    cases=io.StringIO();w=csv.writer(cases,lineterminator='\n');w.writerow(['case',*CASES[0]])
    w.writerows([i,*c.values()] for i,c in enumerate(CASES))
    outputs={'tests/reference/m3g3_trace.csv':(SCHEMA+'\n'+trace.getvalue()).encode(),
             'tests/reference/m3g3_cases.csv':cases.getvalue().encode()}
    manifest=json.loads(output['m3d3j_provenance.json'])
    hashes={f+':'+n:hashlib.sha256(player._original_body(f,n,r).encode()).hexdigest() for f,n,r in manifest['functions']}
    inherited=json.loads((ROOT/'tests/reference/m3g2_provenance.json').read_text())
    for k,v in inherited['function_sha256'].items():assert hashes[k]==v,k
    inventory=dict(new_cases=len(CASES),tics=sum(c['tics'] for c in CASES),captures=sum(r[0]=='P' for r in keep),
        actor_fields=26*sum(r[0]=='A' and r[4]=='1' for r in keep),player_fields=6*sum(r[0]=='P' for r in keep),
        association_fields=sum(r[0]=='AP' for r in keep),statistics_fields=4*sum(r[0]=='LS' for r in keep),
        gameplay_draws=sum(r[0]=='E' and r[3]=='8' for r in keep),setup_discards=sum(c['skip'] for c in CASES),
        thrust_rng_draws=sum(r[0]=='G' and r[3:5]==['3','0'] for r in keep),
        movement_attempts=sum(r[0]=='M' for r in keep),collision_candidates=sum(r[0]=='C' for r in keep),
        slide_line_observations=sum(r[0]=='SL' for r in keep),compiled_original_bodies=len(manifest['functions']),
        added_observer_bodies=len(ADDED),new_engine_source_bodies=0,
        exercised_unique_functions=len(functions),function_entries=sum(functions.values()))
    assert inventory['thrust_rng_draws']==0 and inventory['slide_line_observations']>0
    assert sum(r[0]=='M' and r[1:3]==['10','1'] for r in keep)==2
    assert sum(r[0]=='M' and r[1:3]==['11','1'] for r in keep)==1
    provenance=dict(schema=SCHEMA,revision=manifest['revision'],flags=flags,inventory=inventory,
        source_sha256={**manifest['source_sha256'],'d_items.c':hashlib.sha256((legacy.SOURCE/'d_items.c').read_bytes()).hexdigest()},
        functions=manifest['functions'],function_sha256=hashes,executed_function_entries=dict(sorted(functions.items())),
        association_fragment_sha256=hashlib.sha256(player.ASSOCIATION.encode()).hexdigest(),
        input_sha256={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest() for p in ['tools/reference/m3g3_reference.py','tools/reference/m3g2_reference.py','tools/reference/m3f3_shim.h','tests/reference/m3f5_map_0.bin']},
        assumptions=[
            'Pristine original player0/Demon12, raw P_SpawnMobj plus verbatim P_SpawnPlayer association fragment; no alias, map-THING admission or full player spawn claim.',
            'Medium/commercial/single-player/pistol; no cheats/armor/powers/PlayerThink/active roster; all Demon origins killed before traversal. Physical target/origin floor0/momz0: reverse-thrust vertical predicate false, no thrust RNG draw.',
            'Source and inflictor are separate declared inputs. Source health/pain/death/corpse/statistics and actual XY/friction/axis-slide are observed, including source negative subdivision asymmetry.',
            'Three genuine slide bodies added to this observer already occur in historical M3a evidence; not globally new compiled function coverage. FN counts source setup/gameplay entries, excluding observer-only BSP lookup; inventory is not branch coverage.',
            'Inherited GNU ABI/arithmetic/instrumentation, quarantined retirement, tactile and psprite-request service boundaries remain; no unsafe freed-pointer dereference or C allocator reuse claim.',
            'Selected physical captures retain every compared collision/damage/state/RNG/thinker/slide row. Uncompared auxiliary slide-path traces are omitted; chosen lines and TryMove candidate order remain. No Rust expected output.'],
        outputs_sha256={p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()})
    outputs['tests/reference/m3g3_provenance.json']=(json.dumps(provenance,indent=2,sort_keys=True)+'\n').encode()
    for p,b in outputs.items():
        if check:assert (ROOT/p).read_bytes()==b,p
        else:(ROOT/p).write_bytes(b)
    print(f'{"Checked" if check else "Generated"} {SCHEMA}: {inventory}; strict={overflow}; ASan/UBSan={address}; bytes={sum(map(len,outputs.values()))}')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');p.add_argument('--overflow-check',action='store_true');p.add_argument('--address-check',action='store_true');a=p.parse_args();run(a.check,a.overflow_check,a.address_check)
