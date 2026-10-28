#!/usr/bin/env python3
"""Isolated original damage, hitscan, impact and multi-tic combat evidence."""
import argparse,csv,hashlib,io,json,platform,re,subprocess,tempfile
from pathlib import Path
import generate as base
import generate_m3b as maps
import generate_m3c as actors
ROOT,SOURCE,FLAGS=base.ROOT,base.SOURCE,base.FLAGS
U=65536
MAPS=[maps.fixture([(0,128)]),maps.fixture([(0,128),(0,128)]),maps.fixture([(0,128),(100,228)]),maps.fixture([(0,128),(128,256)]),maps.fixture([(0,128),(0,128)],diagonal=True),maps.fixture([(0,128),(0,128)],portal_flags=5),maps.fixture([(0,128),(24,128)]),maps.fixture([(0,128),(-40,128)])]
# Reversed supplied lists retain the source line-zero prefix and duplicate.
permuted=maps.fixture([(0,128),(0,128)],diagonal=True)
for start in permuted['blocks'][4:12]:
    listing=[0,*reversed(range(len(permuted['lines']))),-1]
    permuted['blocks'][start:start+len(listing)]=listing
raw=permuted['bytes'];count,offset=__import__('struct').unpack_from('<ii',raw,4);lumps=[]
for i in range(count):
    start,size,name=__import__('struct').unpack_from('<ii8s',raw,offset+16*i)
    lumps.append((name.rstrip(b'\0').decode(),maps.words(permuted['blocks']) if name.rstrip(b'\0')==b'BLOCKMAP' else raw[start:start+size]))
permuted['bytes']=maps.wad(lumps);MAPS.append(permuted)
# Inputs use explicit raw fixed values; fixture actor 136 is a modified source
# player-state pawn without a player_t; NOBLOOD is an explicit flag variation.
def case(label,mode=0,map=0,x=-80,y=0,z=0,tx=64,ty=0,tz=0,angle=0,range=160*U,slope=0,damage=20,pain=128,health=100,mass=100,flags=6,other=0,inflictor=1,source=1,tics=0,height=56*U):
    return dict(label=label,mode=mode,map=map,x=x*U,y=y*U,z=z*U,tx=tx*U,ty=ty*U,tz=tz*U,angle=angle,range=range,slope=slope,damage=damage,pain=pain,health=health,mass=mass,flags=flags,other=other,inflictor=inflictor,source=source,tics=tics,height=height)
CASES=[case('damage_nonlethal',mode=1),case('damage_zero',mode=1,damage=0),case('damage_pain_zero',mode=1,pain=0),case('damage_pain_always',mode=1,pain=256),case('damage_environment',mode=1,inflictor=0,source=0),case('lethal_exact',mode=1,damage=100,tics=75),case('extreme_equal',mode=1,damage=200),case('extreme_over',mode=1,damage=201,tics=75),case('fall_forward',mode=1,health=20,damage=30,tz=80,flags=518),case('self_source',mode=1,source=2),case('nonshootable',mode=1,flags=2),case('mass_low',mode=1,mass=10),case('negative_direction',mode=1,x=80,tx=-64),
case('unobstructed'),case('wall',tx=160,range=256*U),case('portal',map=1),case('low_opening',map=2,tz=100),case('closed_opening',map=3,tz=128),case('diagonal',map=4),case('blocking_flag_ignored',map=5),case('aligned_nearer',other=1),case('equal_actor_fractions',other=2),case('noblood',flags=6|524288),case('blood_small',damage=8),case('blood_medium',damage=10),case('zero_damage_impact',damage=0),case('range_before',range=127*U),case('range_edge',range=128*U),case('range_after',range=129*U),case('fractional_range',range=128*U+32768),case('target_above',tz=64),case('aim_above',mode=2,tz=64),case('aim_portal',mode=2,map=1),case('aim_low',mode=2,map=2,tz=100),case('aim_closed',mode=2,map=3,tz=128),case('hitbox_edge',ty=16),case('negative_diagonal',x=-64,y=-32,tx=32,ty=32,angle=0x18000000),case('melee_puff',x=100,tx=160,range=64*U),case('shoot_kill_progression',damage=120,tics=80),case('aim_far',mode=2,range=2048*U),case('corpse_step_edge',mode=1,map=6,tx=-40,mass=10,damage=100,tics=30),case('corpse_drop_edge',mode=1,map=7,tx=-40,mass=10,damage=100,tics=40),case('fall_forward_even',mode=1,health=20,damage=30,tz=80,flags=518,other=1),case('short_target_aim',mode=2,height=24*U),case('threshold_locks_target',mode=3,other=1),case('impact_rng_wrap',mode=4,damage=0,tics=35),case('permuted_candidate_lists',map=8),case('near_corner_fractions',x=-64,y=-64,tx=160,ty=96,angle=0x18000000,range=512*U)]
FUNCTIONS=[('r_main.c','R_PointOnSide','int'),('r_main.c','R_PointInSubsector','subsector_t*'),('r_main.c','R_PointToAngle','angle_t'),('r_main.c','R_PointToAngle2','angle_t')]
FUNCTIONS += [('p_maputl.c',n,r) for n,r in [('P_AproxDistance','fixed_t'),('P_PointOnLineSide','int'),('P_BoxOnLineSide','int'),('P_PointOnDivlineSide','int'),('P_MakeDivline','void'),('P_InterceptVector','fixed_t'),('P_LineOpening','void'),('P_SetThingPosition','void'),('P_UnsetThingPosition','void'),('P_BlockLinesIterator','boolean'),('P_BlockThingsIterator','boolean'),('PIT_AddLineIntercepts','boolean'),('PIT_AddThingIntercepts','boolean'),('P_TraverseIntercepts','boolean'),('P_PathTraverse','boolean')]]
FUNCTIONS += [('p_map.c',n,r) for n,r in [('PIT_CheckLine','boolean'),('PIT_CheckThing','boolean'),('P_CheckPosition','boolean'),('P_TryMove','boolean'),('PTR_AimTraverse','boolean'),('PTR_ShootTraverse','boolean'),('P_AimLineAttack','fixed_t'),('P_LineAttack','void')]]
FUNCTIONS += [('p_tick.c',n,'void') for n in ['P_InitThinkers','P_AddThinker','P_RemoveThinker','P_RunThinkers']]
FUNCTIONS += [('p_mobj.c',n,r) for n,r in [('P_SetMobjState','boolean'),('P_MobjThinker','void'),('P_SpawnMobj','mobj_t*'),('P_RemoveMobj','void'),('P_SpawnPuff','void'),('P_SpawnBlood','void'),('P_XYMovement','void'),('P_ZMovement','void')]]
FUNCTIONS += [('p_inter.c',n,'void') for n in ['P_KillMobj','P_DamageMobj']]
def c_maps():
    code=[base.MAIN[base.MAIN.index('void init_line'):base.MAIN.index('void geometry_vectors')], 'void load_world(int which){memset(lines,0,sizeof(lines));memset(sectors,0,sizeof(sectors));memset(block_heads,0,sizeof(block_heads));switch(which){']
    for i,m in enumerate(MAPS):
        code.append(f'case {i}:{{static short blocks[]={{'+','.join(map(str,m['blocks']))+'};blockmaplump=blocks;blockmap=blocks+4;bmapwidth=4;bmapheight=2;bmaporgx=-256*FRACUNIT;bmaporgy=-128*FRACUNIT;')
        code.append(f'numlines={len(m["lines"])};numsectors={len(m["heights"])};numnodes={len(m["nodes"])};')
        for j,(f,c) in enumerate(m['heights']):code.append(f'sectors[{j}].floorheight={f}*FRACUNIT;sectors[{j}].ceilingheight={c}*FRACUNIT;subsectors[{j}].sector=&sectors[{j}];')
        for j,(x,y) in enumerate(m['vertices']):code.append(f'vertices[{j}].x={x}*FRACUNIT;vertices[{j}].y={y}*FRACUNIT;')
        for j,l in enumerate(m['lines']):code.append(f'init_line(&lines[{j}],&vertices[{l["a"]}],&vertices[{l["b"]}]);lines[{j}].frontsector=&sectors[{l["front"]}];lines[{j}].backsector='+('NULL' if l['back'] is None else f'&sectors[{l["back"]}]')+f';lines[{j}].flags={l["flags"]};lines[{j}].sidenum[0]={l["sides"][0]};lines[{j}].sidenum[1]={l["sides"][1]};')
        for j,q in enumerate(m['nodes']):code.append(f'nodes[{j}].x={q["x"]}*FRACUNIT;nodes[{j}].y=0;nodes[{j}].dx={q["dx"]}*FRACUNIT;nodes[{j}].dy={q["dy"]}*FRACUNIT;nodes[{j}].children[0]={q["children"][0]};nodes[{j}].children[1]={q["children"][1]};')
        code.append('break;}')
    code.append('default:abort();}}');return '\n'.join(code)
MAIN=r'''
static void capture(void){int i;for(i=0;i<owned_count;i++){mobj_t* p=owned[i];printf("A,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,i,alive[i],p->type,p->x,p->y,p->z,p->momx,p->momy,p->momz,p->health,p->state?(long)(p->state-states):-1L,p->tics,p->flags,p->height,p->radius,p->reactiontime,p->threshold,p->target?ident(p->target):-1,p->lastlook,p->floorz,p->ceilingz,(int)(p->subsector-subsectors),prndindex);}}
int main(void){int c,i,t; mobj_t *attacker,*target;for(c=0;c<CASE_COUNT;c++){int* v=input[c];for(i=0;i<owned_count;i++)free(owned[i]);owned_count=0;memset(alive,0,sizeof(alive));scenario=c;operation=0;prndindex=0;P_InitThinkers();load_world(v[1]);
 mobjinfo[136]=mobjinfo[MT_PLAYER];mobjinfo[136].doomednum=-1;mobjinfo[136].spawnhealth=v[14];mobjinfo[136].painchance=v[13];mobjinfo[136].mass=v[15];mobjinfo[136].flags=v[16];mobjinfo[136].reactiontime=8;mobjinfo[136].height=v[21];
 attacker=P_SpawnMobj(v[2],v[3],v[4],MT_PLAYER);target=P_SpawnMobj(v[5],v[6],v[7],136);
 if(v[17])P_SpawnMobj(v[17]==1?16*FRACUNIT:v[5],v[6],v[7],136);capture();operation=1;
 last_kind=last_object=-1;impact[0]=impact[1]=impact[2]=0;
 if(v[0]==3){P_DamageMobj(target,attacker,attacker,v[12]);P_DamageMobj(target,owned[2],owned[2],v[12]);}
 else if(v[0]==4){trace_stage=1;for(i=0;i<100;i++)P_LineAttack(attacker,(unsigned)v[8],v[9],v[10],0);}
 else if(v[0]==1)P_DamageMobj(target,v[18]?attacker:NULL,v[19]==2?target:v[19]?attacker:NULL,v[12]);
 else if(v[0]==2){trace_stage=0;P_AimLineAttack(attacker,(unsigned)v[8],v[9]);printf("Q,%d,%d,%d,%d,%d,%d\n",scenario,operation,linetarget?ident(linetarget):-1,linetarget?aimslope:0,bottomslope,topslope);}
 else{trace_stage=1;P_LineAttack(attacker,(unsigned)v[8],v[9],v[10],v[12]);printf("H,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,last_kind,last_object,impact[0],impact[1],impact[2]);}
 capture();for(t=0;t<v[20];t++){operation=t+2;trace_stage=-1;P_RunThinkers();capture();}
 }for(i=0;i<owned_count;i++)free(owned[i]);return 0;}
'''
def run(check):
    shim=(ROOT/'tools/reference/m3d1_shim.h').read_text().replace('RNG_TABLE',','.join(map(str,base.table('m_random.c',r'unsigned char\s+rndtable\[256\]'))))
    parts=[]
    for file,name,result in FUNCTIONS:
        b=base.body(file,name,result)
        if name=='P_PathTraverse':
            b=b.replace('    trace.dy = y2 - y1;', '    trace.dy = y2 - y1;\n    printf("D,%d,%d,%d,%d,%d,%d,%d\\n",scenario,operation,trace_stage,trace.x,trace.y,trace.dx,trace.dy);')
            b=b.replace('for (count = 0 ; count < 64 ; count++)\n    {', 'for (count = 0 ; count < 64 ; count++)\n    {\n        printf("B,%d,%d,%d,%d,%d\\n",scenario,operation,trace_stage,mapx,mapy);')
        if name=='P_SetThingPosition':b=b.rstrip()[:-1]+'printf("L,%d,%d,%d\\n",scenario,operation,ident(thing));}\n'
        if name=='P_SetMobjState':b=re.sub(r'(do\s*\{)',r'\1\n observe(4,ident(mobj),state);',b,count=1)
        if name=='P_SpawnMobj':b=b.replace('return mobj;','observe(1,ident(mobj),type);return mobj;')
        if name=='P_RemoveThinker':b=b.replace('// FIXME: NOP.','observe(2,ident(thinker),0);')
        if name=='P_RunThinkers':b=b.replace('currentthinker->function.acp1 (currentthinker);','{observe(0,ident(currentthinker),0);currentthinker->function.acp1(currentthinker);}')
        if name in ['PIT_AddLineIntercepts','PIT_AddThingIntercepts']:
            b=b.replace('    // avoid precision', '    candidate(1,(int)(ld-lines));\n    // avoid precision') if name=='PIT_AddLineIntercepts' else b.replace('    tracepositive =','    candidate(0,ident(thing));\n    tracepositive =')
            b=b.replace('    intercept_p++;','    printf("I,%d,%d,%d,%d,%d,%d\\n",scenario,operation,trace_stage,intercept_p->isaline,intercept_id(intercept_p),intercept_p->frac);intercept_p++;')
        if name in ['PTR_AimTraverse','PTR_ShootTraverse']:
            b=re.sub(r'(\{)',r'\1\n printf("T,%d,%d,%d,%d,%d,%d\\n",scenario,operation,trace_stage,in->isaline,intercept_id(in),in->frac);',b,count=1)
        if name=='PTR_ShootTraverse':b=b.replace('// Spawn bullet puffs or blod spots,','last_kind=0;last_object=ident(th);\n    // Spawn bullet puffs or blod spots,').replace('// Spawn bullet puffs.','last_kind=1;last_object=(int)(li-lines);\n    // Spawn bullet puffs.')
        if name in ['P_SpawnPuff','P_SpawnBlood']:
            b=b.replace('    z += ((P_Random()-P_Random())<<10);','    impact[0]=x;impact[1]=y;impact[2]=z;\n    z += jitter();')
        if name in ['PIT_AddLineIntercepts','PIT_AddThingIntercepts']:b=b.replace('    intercept_p->frac = frac;','    if(intercept_p-intercepts>=128)abort();\n    intercept_p->frac = frac;')
        parts.append(b)
    parts.insert(0,base.body('m_random.c','P_Random','int','Raw_Random')+'int P_Random(void){int value=Raw_Random();observe(8,-1,value);return value;}\nstatic int jitter(void){int a=P_Random();int b=P_Random();return (a-b)*1024;}')
    for name in actors.ACTIONS[1:]:
        if name in ['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']:
            b=base.body('p_enemy.c',name,'void');arg='mo' if name=='A_PlayerScream' else 'actor';b=re.sub(r'(\{)',r'\1\n observe(5,ident('+arg+'),'+str(actors.ACTIONS.index(name))+');',b,count=1)
        else:b=f'void {name}(mobj_t* actor){{(void)actor;abort();}}'
        parts.append(b)
    text=shim.replace('/* SOURCE_BODIES */','\n'.join(parts))+'\n'+c_maps()
    fields=list(CASES[0].keys())[1:];fields.insert(11,'reserved')
    text+='\n#define CASE_COUNT '+str(len(CASES))+'\nint input[][22]={' + ','.join('{'+','.join(str(c.get(f,0) if f!='angle' or c[f]<2147483648 else c[f]-4294967296) for f in fields)+'}' for c in CASES)+'};\n'+MAIN
    outputs={f'tests/reference/m3d1_map_{i}.bin':m['bytes'] for i,m in enumerate(MAPS)}
    table=io.StringIO();writer=csv.writer(table,lineterminator='\n');writer.writerow(['case',*CASES[0].keys()]);writer.writerows([[i,*c.values()] for i,c in enumerate(CASES)]);outputs['tests/reference/m3d1_cases.csv']=table.getvalue().encode()
    with tempfile.TemporaryDirectory(prefix='doom-m3d1-reference-') as tmp:
        src=Path(tmp)/'reference.c';src.write_text(text);binary=Path(tmp)/'reference';subprocess.run(['cc',*FLAGS,'-I'+str(SOURCE),str(src),str(SOURCE/'m_fixed.c'),str(SOURCE/'tables.c'),'-o',str(binary)],check=True);outputs['tests/reference/m3d1_trace.csv']=subprocess.check_output([str(binary)])
    sources=sorted({f for f,_,_ in FUNCTIONS}|{'info.c','m_random.c','m_fixed.c','tables.c'}|{p.name for p in SOURCE.glob('*.h')})
    manifest={'functions':[{'source':f,'name':name,'return':result} for f,name,result in FUNCTIONS]+[{'source':'m_random.c','name':'P_Random','return':'int'}]+[{'source':'p_enemy.c','name':name,'return':'void'} for name in ['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']], 'abi':{'int_bits':32,'short_bits':16,'pointer_bits':64,'fixed_bits':32,'angle_bits':32},'revision':'a77dfb96cb91780ca334d0d4cfd86957558007e0','flags':FLAGS,'source_sha256':{p:hashlib.sha256((SOURCE/p).read_bytes()).hexdigest() for p in sources},'adapter_sha256':{p:hashlib.sha256((ROOT/'tools/reference'/p).read_bytes()).hexdigest() for p in ['generate.py','generate_m3b.py','generate_m3c.py','generate_m3d1.py','m3d1_shim.h']},'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()}}
    path=ROOT/'tests/reference/m3d1_provenance.json'
    if check:
        old=json.loads(path.read_text())
        for k,v in manifest.items():assert old[k]==v,k
    else:
        manifest.update(compiler=subprocess.check_output(['cc','--version'],text=True).splitlines()[0],platform=platform.platform(),assumptions=['original structs/info/body functions, native player-state fixture type136 without player_t','source intercept array bounded at128; scenarios avoid overflow/abs(INT_MIN)','jitter uses explicit ordered draws and representable multiplication instead of negative signed shift','retirement retains storage; unsupported services abort only on unreachable paths'])
        outputs['tests/reference/m3d1_provenance.json']=(json.dumps(manifest,indent=2)+'\n').encode()
    for p,b in outputs.items():
        if check:assert (ROOT/p).read_bytes()==b,p
        else:(ROOT/p).write_bytes(b)
    print(('Checked' if check else 'Generated')+f' M3d1: {len(CASES)} original-body combat scenarios.')
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
