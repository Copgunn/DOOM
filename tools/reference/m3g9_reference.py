#!/usr/bin/env python3
"""Independent original source pickup observations, no Rust expectations."""
from pathlib import Path
import sys,re,csv,io,json
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3g8_reference as pack
core=pack.base
from m3g9_cases import populate
LAYOUTS=populate(pack)
def configure():
    core.main.MAPS,core.main.THINGS,core.main.KEYS,core.main.CASES=[],[],[],[]
    for c in core.CASES:
        m=core.previous.geometry.fixture(n=1,count=5 if c['wall'] else 4,blockers=bool(c['wall']))
        if c['wall']==1:m['vertices'][4:6]=[(16,y)for x,y in m['vertices'][4:6]]
        m['vertices']=[(6*x+c['offset_x'],12*y)for x,y in m['vertices']]
        m['lumps']['VERTEXES']=b''.join(core.previous.geometry.old.words(v)for v in m['vertices'])
        m['range']=(-768+c['offset_x'],768+c['offset_x']);m['center']=(c['offset_x'],0)
        m=core.previous.extent.grid(m,(-1024,-1024),16,16)
        demon=[]if not c['thing']else[(c['dx'],c['dy'],c['thing_angle'],3002,15 if c['thing']==2 else 7)]
        things=LAYOUTS[c['layout']]+demon if c['demon_last'] else demon+LAYOUTS[c['layout']]
        core.main.case(c['label'],n=1,custom=m,things=things,tics=c['tics'],rejected=bool(c['rejected']),**{'pc_'+k:v for k,v in c.items()if k not in ['label','tics','map']})
        c['map']=core.main.CASES[-1]['map']
core.configure=configure
core.ADDED.extend([('p_inter.c',name,result)for name,result in [('P_GiveAmmo','boolean'),('P_GiveBody','boolean'),('P_GiveWeapon','boolean'),('P_TouchSpecialThing','void')]])
core.DRIVER=core.DRIVER.replace('if(owned_count)origin=owned[0];','for(wi=0;wi<owned_count;wi++)if(owned[wi]->type==MT_SERGEANT)origin=owned[wi];')
core.CAPTURE+='\nstatic void pickup_capture(void){player_t*p=&players[0];printf("PX,%d,%d,%d,%d,%d,%d,%d,%d,%d\\n",scenario,operation,p->health,p->ammo[0],p->ammo[1],p->readyweapon,p->pendingweapon,p->bonuscount,p->itemcount);}\n'
core.DRIVER=core.DRIVER.replace('weapon_capture();','weapon_capture();pickup_capture();')
core.DRIVER=core.DRIVER.replace('reset(c);onground=0;','reset(c);iquehead=iquetail=0;onground=0;')
core.DRIVER=core.DRIVER.replace('body=P_SpawnMobj(v[V_PC_PX]', 'if(v[V_PC_DROP]){mobj_t*drop=P_SpawnMobj(v[V_PC_DROP_X]*FRACUNIT,v[V_PC_DROP_Y]*FRACUNIT,ONFLOORZ,v[V_PC_DROP]);drop->flags|=MF_DROPPED;}\nbody=P_SpawnMobj(v[V_PC_PX]')
core.DRIVER=core.DRIVER.replace('P_DamageMobj(body,NULL,origin,v[V_PC_PRE_DAMAGE])','P_DamageMobj(body,v[V_PC_PHYSICAL]?origin:NULL,origin,v[V_PC_PRE_DAMAGE])')
core.DRIVER=core.DRIVER.replace('t<=v[V_PC_MOVE_HOLD]?', 't>=v[V_PC_MOVE_START]&&t<=v[V_PC_MOVE_HOLD]?')
core.DRIVER=core.DRIVER.replace('if(p->playerstate==PST_DEAD)', 'if(v[V_PC_SCRIPT]==20){p->cmd.buttons=((t>=15&&t<=50)||t>=255)?BT_ATTACK:0;p->cmd.forwardmove=t>=40&&t<=48?1:0;}\nif(v[V_PC_SCRIPT]==21){p->cmd.forwardmove=t<=80?4:0;}\nif(p->playerstate==PST_DEAD)')
core.DRIVER=core.DRIVER.replace('P_MovePsprites(p);','P_MovePsprites(p);if(p->playerstate==PST_LIVE&&p->bonuscount)p->bonuscount--;')
inter=(ROOT/'linuxdoom-1.10/p_inter.c').read_text()
extra='\n#include "dstrings.h"\n'+re.search(r'^#define\s+BONUSADD[^\n]+',inter,re.M).group(0)+'\n'+re.search(r'int\s+clipammo\[NUMAMMO\][^;]+;',inter).group(0)+r'''
boolean P_GiveArmor(player_t*p,int a){(void)p;(void)a;abort();}
void P_GiveCard(player_t*p,card_t c){(void)p;(void)c;abort();}
boolean P_GivePower(player_t*p,int a){(void)p;(void)a;abort();}
'''
from m3g9_observers import instrument,EXTRA
instrument(core)
extra+=EXTRA
core.CAPTURE=core.CAPTURE.replace('p->health,p->ammo[0],p->ammo[1],p->readyweapon,p->pendingweapon,p->bonuscount,p->itemcount);', 'p->bonuscount,p->itemcount,pickup_message(p->message));').replace('%d,%d,%d,%d,%d,%d,%d,%d,%d\\n','%d,%d,%d,%d,%d\\n')
from m3g9_components import SOURCE as COMPONENT_SOURCE
core.CAPTURE+=COMPONENT_SOURCE
core.commands.header=core.commands.header.replace('if(argc<2)return 2;', 'if(argc<2)return 2;\nif(!strcmp(argv[1],\"pickup-components\"))return pickup_components();')
import subprocess
real_run=subprocess.run
component_output=[]
def compile_components(args,*a,**kw):
    result=real_run(args,*a,**kw)
    if args[0]=='cc' and any(str(x).endswith('reference.c')for x in args) and result.returncode==0:
        binary=args[args.index('-o')+1]
        component_output.append(subprocess.check_output([str(binary),'pickup-components']))
    return result
subprocess.run=compile_components
core.DRIVER=core.DRIVER.replace('if(p->playerstate==PST_DEAD)', 'if(v[V_PC_SCRIPT]==22){p->cmd.buttons=t>=15&&t<=50?BT_ATTACK:0;p->cmd.forwardmove=t>=40&&t<=48?1:0;}\nif(p->playerstate==PST_DEAD)')
old_write=Path.write_text
def write(path,text,*a,**kw):
    if path.name=='m3d3j_shim.h':
        needle='void P_TouchSpecialThing(mobj_t* a,mobj_t* b){(void)a;(void)b;abort();}'
        assert needle in text;text=text.replace(needle,extra)
    return old_write(path,text,*a,**kw)
Path.write_text=write
def capture(flags):
    component_output.clear()
    Path.write_text=write;subprocess.run=compile_components
    try:out=pack.capture(flags)
    finally:Path.write_text=old_write;subprocess.run=real_run
    assert len(component_output)==1
    return out,component_output[0]
Path.write_text=old_write;subprocess.run=real_run

def run(check,overflow,address):
    import hashlib,collections
    from evidence_pack import pack as compress
    sha=lambda data:hashlib.sha256(data).hexdigest()
    flags=list(pack.legacy.base.FLAGS)
    out,components=capture(flags)
    for enabled,sanitizer in [(overflow,'signed-integer-overflow,integer-divide-by-zero,float-cast-overflow'),(address,'address,undefined')]:
        if enabled:
            audited,comp=capture([f for f in flags if f!='-fwrapv']+['-fno-wrapv','-fsanitize='+sanitizer,'-fno-sanitize-recover=all'])
            assert audited['m3d3j_trace.csv']==out['m3d3j_trace.csv']
            assert comp==components
    rows=list(csv.reader(io.StringIO(out['m3d3j_trace.csv'].decode())))
    functions=collections.Counter(r[3] for r in rows if r[0]=='FN')
    selected={0,1,2,8,16,19,20,29,30,31,44,46,50,54,64,96,128,160,192,254,255,256,280,384,512}
    outcomes={(int(r[1]),int(r[2])) for r in rows if (r[0]=='PN' and r[3]=='6' and r[5]=='3') or(r[0]=='W' and r[3]=='5')or(r[0]=='G' and r[3]=='2')}
    keys={(int(r[1]),int(r[2])) for r in rows if r[0]=='P' and(int(r[2])in selected or(int(r[1]),int(r[2]))in outcomes or int(r[2])==core.CASES[int(r[1])]['tics'])}
    kept=[r for r in rows if r[0]!='FN' and(r[0]not in {'A','P','AP','R','S','LS'}or tuple(map(int,r[1:3]))in keys)]
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(kept)
    expanded=('DOOM-PLAYER-WORLD-PICKUPS-v1\n'+text.getvalue()).encode()
    outputs={'m3g9_trace.dref':compress(expanded)}
    text=io.StringIO();w=csv.writer(text,lineterminator='\n');w.writerow(['case',*core.CASES[0]]);w.writerows([i,*c.values()]for i,c in enumerate(core.CASES))
    outputs['m3g9_cases.csv']=text.getvalue().encode()
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(r for r in csv.reader(io.StringIO(components.decode()))if r[0]=='PCOMP')
    outputs['m3g9_components.csv']=text.getvalue().encode()
    known=sorted(p for p in(ROOT/'tests/reference').glob('*map_*.bin')if not p.name.startswith('m3g9_'))
    maps=[]
    for i,m in enumerate(core.main.MAPS):
        match=next((p for p in known if p.read_bytes()==m['bytes']),None)
        if match is None:
            match=ROOT/f'tests/reference/m3g9_map_{i}.bin';outputs[match.name]=m['bytes']
        maps.append('tests/reference/'+match.name)
    outputs['m3g9_maps.rs']=('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("{Path(p).name}"),\n'for p in maps)+'];\n').encode()
    manifest=json.loads(out['m3d3j_provenance.json']);bodies=manifest['functions']+[['p_pspr.c',n,'void']for n in pack.WEAPON_ACTIONS]
    assert len(bodies)==len({tuple(b)for b in bodies})
    known_bodies=set()
    for p in sorted((ROOT/'tests/reference').glob('*provenance.json')):
        if p.name.startswith('m3g9_'):continue
        for b in json.loads(p.read_text()).get('functions',[]):
            if isinstance(b,list):known_bodies.add(':'.join(b[:2]))
            elif isinstance(b,str):known_bodies.add(b)
            elif isinstance(b,dict)and'source'in b and'name'in b:known_bodies.add(b['source']+':'+b['name'])
    counter=lambda tag,code=None:sum(r[0]==tag and(code is None or r[3]==str(code))for r in kept)
    inventory=dict(scenarios=len(core.CASES),scripted_tics=sum(c['tics']for c in core.CASES),captures=len(keys),unticked_components=8,
      selected_original_bodies=len(bodies),compiled_original_bodies=len(bodies)+3,globally_new_bodies=sorted({f'{f}:{n}'for f,n,_ in bodies}-known_bodies),
      exercised_unique_functions=len(functions),function_entries=sum(functions.values()),actor_fields=26*sum(r[0]=='A'and r[4]=='1'for r in kept),player_fields=6*counter('P'),weapon_fields=17*counter('WX'),pickup_fields=3*counter('PX'),
      touches=counter('PN',0),ammo_grants=counter('PN',1),health_grants=counter('PN',2),weapon_grants=counter('PN',3),collected=sum(r[0]=='PN'and r[3]=='6'and r[5]=='3'for r in kept),
      source_only_respawn_queue_observations=counter('PQ'),shots=counter('W',5),gameplay_draws=counter('E',8),weapon_draws=counter('W',8),damage_requests=counter('G',2),impact_spawns=counter('G',11),movement_attempts=counter('M'),collision_candidates=counter('C'),geometry_trace_roles=counter('TR'),
      expanded_trace_bytes=len(expanded),packed_trace_bytes=len(outputs['m3g9_trace.dref']))
    inputs=['m3g9_reference.py','m3g9_cases.py','m3g9_observers.py','m3g9_components.py','m3g8_reference.py','m3g7_reference.py','m3g6_reference.py','m3g5_reference.py','m3g4_reference.py','m3g3_reference.py','m3g2_reference.py','m3f3_shim.h','evidence_pack.py']
    source_names=set(manifest['source_sha256'])|{'d_items.c','dstrings.h'}
    prov=dict(schema='DOOM-PLAYER-WORLD-PICKUPS-v1',revision=manifest['revision'],flags=manifest['flags'],inventory=inventory,maps=maps,functions=bodies,
      source_sha256={n:sha((ROOT/'linuxdoom-1.10'/n).read_bytes())for n in sorted(source_names)},function_sha256={f'{f}:{n}':sha(core.raw_body(f,n,t).encode())for f,n,t in bodies},executed_function_entries=dict(sorted(functions.items())),linked_math_bodies=['FixedMul','FixedDiv','FixedDiv2'],
      inputs_sha256={'tools/reference/'+n:sha((ROOT/'tools/reference'/n).read_bytes())for n in inputs},outputs_sha256={n:sha(d)for n,d in outputs.items()},expanded_trace_sha256=sha(expanded),
      assumptions=[
       'Genuine player0, at most one MT_SERGEANT, actual source items/THING order and sprite-based Touch dispatch. Eight static definitions, at most16 placed+dropped items.',
       'Four new extracted bodies GiveAmmo/GiveBody/GiveWeapon/TouchSpecialThing. Actual P_CheckPosition/PIT_CheckThing execute things before lines. No Rust expected observations.',
       'Controlled loadouts/health setup are isolated case inputs. Sustained cases46/49 use normal50 bullet initialization, actual commands, autonomous melee and actual world healing/ammo; no injected combat damage.',
       'Bounded source command/weapon stage, live bonuscount decrement fragment, then original thinkers. Not complete P_Ticker/PlayerThink/DeathThink, HUD or respawn.',
       'Actual P_RemoveMobj writes inactive itemrespawnque. PQ retains these C-only observations; queue/respawn storage is outside the Rust policy and canonical projection. Passive sound services are observed requests.',
       'Eight unticked defined components establish vertical and square-overlap edges, above100 refusal and dead-body touch; airborne runtime remains excluded.',
       'TR records actual trace callback identity. Geometry-only slide traversal rows remain source evidence, while M/C/SL decisions and supported gameplay events are compared. No fabricated aim events.',
       'Inherited explicit left-before-right spread/jitter arithmetic domain and GNU ABI adapters apply. No freed pointer observations, all-inactive source scans, invalid ammo indexing or commercial assets.',
       'Birth/generation mappings and DSIM25 event projection are independent schema policy, not source pointer-lifetime equivalence.'])
    outputs['m3g9_provenance.json']=(json.dumps(prov,indent=2,sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        p=ROOT/'tests/reference'/name
        if check:assert p.read_bytes()==data,p
        else:p.write_bytes(data)
    print(('Checked'if check else'Generated')+' original world pickups: '+str(inventory)+f'; strict={overflow}; ASan/UBSan={address}')

if __name__=='__main__':
    import argparse
    ap=argparse.ArgumentParser();ap.add_argument('--check',action='store_true');ap.add_argument('--overflow-check',action='store_true');ap.add_argument('--address-check',action='store_true');a=ap.parse_args();run(a.check,a.overflow_check,a.address_check)
