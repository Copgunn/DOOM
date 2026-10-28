#!/usr/bin/env python3
"""Original MT_PLASMA spawn/flight/impact/explosion evidence; no Rust expected data."""
import argparse,csv,hashlib,io,json,platform,re,struct,subprocess,tempfile
from pathlib import Path
import generate as base
import generate_m3c as actors
import generate_m3d1 as combat
ROOT,SOURCE,FLAGS=base.ROOT,base.SOURCE,base.FLAGS
U=65536
MAPS=combat.MAPS[:]+[combat.maps.fixture([(0,64),(56,128)]),combat.maps.fixture([(0,64),(57,128)]),combat.maps.fixture([(-128,128)]),combat.maps.fixture([(0,128),(0,128)],portal_flags=6)]
missing=combat.maps.fixture([(0,128)]);raw=missing['bytes'];count,offset=struct.unpack_from('<ii',raw,4);lumps=[]
for i in range(count):
 start,size,name=struct.unpack_from('<ii8s',raw,offset+i*16);data=raw[start:start+size]
 if name.rstrip(b'\0')==b'BLOCKMAP':
  blocks=missing['blocks'][:4]+[12]*8+[0,-1];missing['blocks']=blocks;data=combat.maps.words(blocks)
 lumps.append((name.rstrip(b'\0').decode(),data))
missing['bytes']=combat.maps.wad(lumps);MAPS.append(missing)
def case(label,mode=0,map=0,x=-80,y=0,z=0,tx=64,ty=0,tz=0,source=0,target=136,health=100,pain=128,mass=100,flags=6,extra=0,count=1,tics=40):
 return dict(label=label,mode=mode,map=map,x=x*U,y=y*U,z=z*U,tx=tx*U,ty=ty*U,tz=tz*U,source=source,target=target,health=health,pain=pain,mass=mass,flags=flags,extra=extra,count=count,tics=tics)
CASES=[case('actor_aim_hit'),case('player_autoaim_hit',mode=1),case('clear_to_wall',flags=0),case('spawn_wall',x=108,tx=160),case('spawn_actor',tx=-56),case('portal',map=1,tx=96),case('blocked_opening',map=2,tz=100),case('closed_opening',map=3,tz=128),case('ignore_blocking',map=5,tx=96),case('ignore_blockmonsters',map=12,tx=96),case('floor_impact',map=11,z=64,tz=0,flags=0),case('ceiling_impact',tz=64,flags=0),case('solid_prop',flags=2),case('nonsolid_target',flags=0),case('vertical_pass_above',tz=-64,map=11,mode=1),case('source_immunity',tx=-80,source=136,target=136),case('same_species',source=136,target=136),case('player_species_exception',target=0,pain=255,flags=33557510),case('baron_knight',source=15,target=17),case('knight_baron',source=17,target=15),case('negative_flight',x=80,tx=-64),case('diagonal',x=-64,y=-64,tx=64,ty=64),case('negative_diagonal',x=64,y=64,tx=-64,ty=-64),case('shadow_target',flags=6|262144),case('lethal',health=10,tics=90),case('pain_always',pain=256),case('nonsolid_shootable_residual',flags=4,tx=-40),case('actor_before_wall',tx=144),case('equal_contacts',extra=1),case('two_projectiles',count=2),case('rng_wrap_batches',mode=3,x=108,tx=160,flags=0,count=50,tics=30),case('exact_opening',map=9,z=24,tz=24,tx=160),case('narrow_opening',map=10,z=24,tz=24,tx=160),case('missing_blockmap',map=13,flags=0,tics=50),case('player_autoaim_miss',mode=1,flags=0),case('same_pass_spawn',mode=2),case('dropoff_flight',map=7,tx=160,tz=-40,flags=0),case('floor_zero_skimming',z=-32,tz=-32,flags=0),case('ceiling_equality',z=88,tz=88,flags=0),case('autoaim_right',mode=1,ty=24),case('autoaim_left',mode=1,ty=-24)]
FUNCTIONS=combat.FUNCTIONS+[('p_mobj.c',n,r) for n,r in [('P_SpawnMissile','mobj_t*'),('P_SpawnPlayerMissile','void'),('P_CheckMissileSpawn','void'),('P_ExplodeMissile','void')]]
MAIN=r'''
static mobj_t *caster,*victim;static int launch_mode,did_launch;
static void launch(void){printf("P,%d,%d,5,%d,%d,%d,%d\n",scenario,operation,ident(caster),launch_mode==1?1:0,launch_mode==1?-1:ident(victim),MT_PLASMA);if(launch_mode==1)P_SpawnPlayerMissile(caster,MT_PLASMA);else P_SpawnMissile(caster,victim,MT_PLASMA);}
static void launching_thinker(mobj_t* p){if(!did_launch){did_launch=1;launch();}P_MobjThinker(p);}
static void capture(void){int i;for(i=0;i<owned_count;i++){mobj_t* p=owned[i];printf("A,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%u,%d,%d\n",scenario,operation,i,alive[i],p->type,p->x,p->y,p->z,p->momx,p->momy,p->momz,p->health,p->state?(long)(p->state-states):-1L,p->tics,p->flags,p->height,p->radius,p->reactiontime,p->threshold,p->target?ident(p->target):-1,p->lastlook,p->floorz,p->ceilingz,(int)(p->subsector-subsectors),prndindex,p->angle,p->sprite,p->frame);}}
int main(void){int c,i,t,batch;for(c=0;c<CASE_COUNT;c++){int* v=input[c];for(i=0;i<owned_count;i++)free(owned[i]);owned_count=0;memset(alive,0,sizeof(alive));scenario=c;operation=0;attempt=-1;prndindex=0;P_InitThinkers();load_world(v[1]);
 for(i=0;i<3;i++){int type=i==0?15:i==1?17:136;mobjinfo[type]=mobjinfo[MT_PLAYER];mobjinfo[type].doomednum=-1;mobjinfo[type].spawnhealth=v[10];mobjinfo[type].painchance=v[11];mobjinfo[type].mass=v[12];mobjinfo[type].flags=type==v[9]?v[13]:6;mobjinfo[type].reactiontime=8;}
 caster=P_SpawnMobj(v[2],v[3],v[4],v[8]);victim=P_SpawnMobj(v[5],v[6],v[7],v[9]);if(v[14])P_SpawnMobj(v[5],v[6],v[7],v[9]);launch_mode=v[0];did_launch=0;if(v[0]==2)caster->thinker.function.acp1=(actionf_p1)launching_thinker;capture();operation=1;
 if(v[0]!=2)for(i=0;i<v[15];i++)launch();capture();for(batch=0;batch<(v[0]==3?2:1);batch++){if(batch){operation++;attempt=-1;for(i=0;i<v[15];i++)launch();capture();}for(t=0;t<v[16];t++){operation++;attempt=-1;P_RunThinkers();capture();}}
 }for(i=0;i<owned_count;i++)free(owned[i]);return 0;}
'''
def run(check):
 combat.MAPS=MAPS
 shim=(ROOT/'tools/reference/m3d2b_shim.h').read_text().replace('RNG_TABLE',','.join(map(str,base.table('m_random.c',r'unsigned char\s+rndtable\[256\]'))))
 parts=[]
 for file,name,result in FUNCTIONS:
  b=base.body(file,name,result,'Raw_ZMovement' if name=='P_ZMovement' else None)
  if name=='P_SetThingPosition':b=b.rstrip()[:-1]+'printf("L,%d,%d,%d\\n",scenario,operation,ident(thing));}\n'
  if name=='P_SetMobjState':b=re.sub(r'(do\s*\{)',r'\1\n observe(4,ident(mobj),state);',b,count=1)
  if name=='P_SpawnMobj':b=b.replace('return mobj;','observe(1,ident(mobj),type);return mobj;')
  if name=='P_RemoveThinker':b=b.replace('// FIXME: NOP.','observe(2,ident(thinker),0);')
  if name=='P_RunThinkers':b=b.replace('currentthinker->function.acp1 (currentthinker);','{observe(0,ident(currentthinker),0);currentthinker->function.acp1(currentthinker);}')
  if name in ['PIT_AddLineIntercepts','PIT_AddThingIntercepts']:b=b.replace('    intercept_p->frac = frac;','    if(intercept_p-intercepts>=128)abort();\n    intercept_p->frac = frac;')
  if name in ['P_SpawnPuff','P_SpawnBlood']:b=b.replace('    z += ((P_Random()-P_Random())<<10);','    z += jitter();')
  if name=='P_DamageMobj':
   b=re.sub(r'(\{)',r'\1\n printf("D,%d,%d,%d,%d,%d,%d\\n",scenario,operation,ident(target),inflictor?ident(inflictor):-1,source?ident(source):-1,damage);',b,count=1).replace('target->target = source;','target->target = source;observe(6,ident(target),ident(source));')
  if name=='PIT_CheckThing':
   b=re.sub(r'(\{)',r'\1\n pcand(0,ident(thing));',b,count=1).replace('return false;','{block_actor=ident(thing);move_failure=1;return false;}').replace('return !(thing->flags & MF_SOLID);','return block_result(!(thing->flags & MF_SOLID),thing);').replace('P_Random()%8','proj_random(3)%8')
  if name=='PIT_CheckLine':
   b=re.sub(r'(\{)',r'\1\n pcand(1,(int)(ld-lines));',b,count=1).replace('return false;','{block_line=(int)(ld-lines);move_failure=2;return false;}').replace('    P_LineOpening (ld);','    P_LineOpening (ld);\n printf("O,%d,%d,%d,%ld,%d,%d,%d,%d\\n",scenario,operation,attempt,(long)(ld-lines),opentop,openbottom,openrange,lowfloor);')
  if name=='P_TryMove':
   b=re.sub(r'(\{)',r'\1\n attempt++;block_actor=block_line=-1;move_failure=0;',b,count=1)
   for text,reason in [("return false;\t// doesn't fit",3),("return false;\t// mobj must lower itself to fit",4),("return false;\t// too big a step up",5),("return false;\t// don't stand over a dropoff",6)]:b=b.replace(text,'{move_failure='+str(reason)+';return false;}')
   b=b.replace('return false;','{record_move(thing,x,y,0);return false;}').replace('    return true;','    record_move(thing,x,y,1);return true;')
  if name in ['P_SpawnMissile','P_SpawnPlayerMissile']:
   b=b.replace('th->target = source;','th->target = source;observe(6,ident(th),ident(source));').replace('(P_Random()-P_Random())<<20','shadow_spread()').replace('    P_CheckMissileSpawn (th);','    velocity(th);P_CheckMissileSpawn (th);')
  if name=='P_CheckMissileSpawn':b=b.replace('P_Random()','proj_random(0)').replace('    if (!P_TryMove','    printf("P,%d,%d,4,%d,%d,%d,%d,%d\\n",scenario,operation,ident(th),th->x,th->y,th->z,th->tics);\n    if (!P_TryMove')
  if name=='P_ExplodeMissile':b=re.sub(r'(\{)',r'\1\n printf("P,%d,%d,0,%d\\n",scenario,operation,ident(mo));',b,count=1).replace('P_Random()','proj_random(4)')
  if name=='P_ZMovement':b+='\nvoid P_ZMovement(mobj_t* p){int z=p->z,m=p->momz,f=p->floorz,c=p->ceilingz,flags=p->flags;int floor=z+m<=f;int ceil=!(floor&&(flags&MF_MISSILE))&&z+m+p->height>c;int gravity=!floor&&!(flags&MF_NOGRAVITY);Raw_ZMovement(p);printf("Z,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\\n",scenario,operation,ident(p),z,m,p->z,p->momz,f,c,floor,ceil,gravity);}\n'
  parts.append(b)
 parts.insert(0,base.body('m_random.c','P_Random','int','Raw_Random')+'int P_Random(void){int value=Raw_Random();observe(8,-1,value);return value;}\nstatic int jitter(void){int a=P_Random();int b=P_Random();return (a-b)*1024;}')
 for name in actors.ACTIONS[1:]:
  if name in ['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']:
   b=base.body('p_enemy.c',name,'void');arg='mo' if name=='A_PlayerScream' else 'actor';b=re.sub(r'(\{)',r'\1\n observe(5,ident('+arg+'),'+str(actors.ACTIONS.index(name))+');',b,count=1)
  else:b=f'void {name}(mobj_t* actor){{(void)actor;abort();}}'
  parts.append(b)
 fields=list(CASES[0].keys())[1:]
 text=shim.replace('/* SOURCE_BODIES */','\n'.join(parts))+'\n'+combat.c_maps()+'\n#define CASE_COUNT '+str(len(CASES))+'\nint input[][17]={' + ','.join('{'+','.join(str(c[f]) for f in fields)+'}' for c in CASES)+'};\n'+MAIN
 outputs={f'tests/reference/m3d2b_map_{i}.bin':m['bytes'] for i,m in enumerate(MAPS)}
 with tempfile.TemporaryDirectory(prefix='doom-m3d2b-reference-') as tmp:
  src=Path(tmp)/'reference.c';src.write_text(text);binary=Path(tmp)/'reference';subprocess.run(['cc',*FLAGS,'-I'+str(SOURCE),str(src),str(SOURCE/'m_fixed.c'),str(SOURCE/'tables.c'),'-o',str(binary)],check=True);outputs['tests/reference/m3d2b_trace.csv']=subprocess.check_output([str(binary)],timeout=30)
 table=io.StringIO();w=csv.writer(table,lineterminator='\n');w.writerow(['case',*CASES[0].keys()]);w.writerows([[i,*c.values()] for i,c in enumerate(CASES)]);outputs['tests/reference/m3d2b_cases.csv']=table.getvalue().encode()
 files=sorted({f for f,_,_ in FUNCTIONS}|{'info.c','m_random.c','m_fixed.c','tables.c','p_enemy.c'}|{p.name for p in SOURCE.glob('*.h')})
 manifest={'revision':'a77dfb96cb91780ca334d0d4cfd86957558007e0','functions':FUNCTIONS+[('m_random.c','P_Random','int')]+[('p_enemy.c',name,'void') for name in ['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']],'flags':FLAGS,'source_sha256':{p:hashlib.sha256((SOURCE/p).read_bytes()).hexdigest() for p in files},'adapter_sha256':{p:hashlib.sha256((ROOT/'tools/reference'/p).read_bytes()).hexdigest() for p in ['generate.py','generate_m3b.py','generate_m3c.py','generate_m3d1.py','generate_m3d2b.py','m3d2b_shim.h']},'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()},'assumptions':['real source structs/metadata, GNU 32-bit int/fixed and 64-bit pointer/function-union ABI','shadow/jitter ordered draws and multiplication replace negative signed shifts; source XY division/shift retained','unbound pawns and explicit player-state source-ID fixtures 15/17/136; no actual player damage/AI/splash/sky/specials','original retired C memory retained only for observations; generational Rust mapping is separate','same-pass scenario uses a synthetic wrapper invoking original P_SpawnMissile during an original thinker traversal']}
 path=ROOT/'tests/reference/m3d2b_provenance.json'
 if check:
  old=json.loads(path.read_text())
  for k,v in manifest.items():assert old[k]==json.loads(json.dumps(v)),k
 else:
  manifest.update(compiler=subprocess.check_output(['cc','--version'],text=True).splitlines()[0],platform=platform.platform());outputs['tests/reference/m3d2b_provenance.json']=(json.dumps(manifest,indent=2)+'\n').encode()
 for p,b in outputs.items():
  if check:assert (ROOT/p).read_bytes()==b,p
  else:(ROOT/p).write_bytes(b)
 print(('Checked' if check else 'Generated')+f' M3d2b: {len(CASES)} original-body projectile scenarios.')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
