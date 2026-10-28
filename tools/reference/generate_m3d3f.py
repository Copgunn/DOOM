#!/usr/bin/env python3
"""Genuine original chaingunner attack/refire bodies and complete synthetic thinker traces; no Rust output."""
import argparse,csv,hashlib,io,json,platform,re,struct,subprocess,tempfile
from pathlib import Path
import generate as base
import generate_m3b as maps
import generate_m3c as actors
import generate_m3d1 as combat
import generate_m3d3a as sight
ROOT,SOURCE,FLAGS=base.ROOT,base.SOURCE,base.FLAGS
U=65536
lumps=sight.lumps
LOOK=actors.ACTIONS.index('A_Look');CHASE=actors.ACTIONS.index('A_Chase')
POSIT=actors.SOUND_NAMES.index('sfx_posit1')
MAPS=[maps.fixture(h) for h in [[(0,128)],[(0,128)]*2,[(0,128),(24,128)],[(0,128),(25,128)],[(0,128),(-24,128)],[(0,128),(-25,128)],[(0,64),(8,128)],[(0,64),(9,128)],[(0,128),(128,256)]]]
MAPS += [maps.fixture([(0,128)]*2,portal_flags=5),maps.fixture([(0,128)]*2,portal_flags=6),maps.fixture([(0,128)]*2,diagonal=True,portal_flags=5)]
for x,y in [(128,16),(128,15),(16,16)]:
 m=maps.fixture([(0,128)]);m['vertices']=[((-x if a<0 else x),(-y if b<0 else y)) for a,b in m['vertices']]
 sight.replace_lumps(m,{'VERTEXES':b''.join(maps.words(v) for v in m['vertices'])});MAPS.append(m)
MAPS += [maps.fixture([(0,128)],omit=True),maps.fixture([(0,128)]*3)]
def roster(x=96,y=0,z=0,active=1,health=100):return [(x,y,z,active,health),(-200,-96,0,0,100),(-200,-96,0,0,100),(-200,-96,0,0,100)]
def case(label,mode=2,map=0,ox=-64,oy=0,oz=0,players=None,direction=8,count=0,reaction=8,threshold=100,target=1,speed=8,radius=16,active_sound=0,see=1,stand=1,skill=2,enter=0,mx=0,my=0,mz=0,target_health=100,target_flags=6,flip_tick=2,flip_flags=-1,flip_actorhealth=-999,flip_target=-2,other=0,**kw):
 c=sight.case(label,otype=136,mode=mode,map=map,ox=ox,oy=oy,oz=oz,players=players or roster(),sound_id=kw.pop("sound_id",0),stand=stand,threshold=threshold,**kw)
 c.update(direction=direction,count=count,reaction=reaction,target=target,speed=speed,radius=radius*U,active_sound=active_sound,see=see,skill=skill,enter=enter,mx=mx*U,my=my*U,mz=mz*U,target_health=target_health,target_flags=target_flags,flip_tick=flip_tick,flip_flags=flip_flags,flip_actorhealth=flip_actorhealth,flip_target=flip_target,other=other)
 return c
import generate_m3d3c as previous
MAPS=previous.MAPS+[sight.replace_lumps(maps.fixture([(0,128)]*2),{"REJECT":bytes([2])}),sight.replace_lumps(maps.fixture([(0,128)]),{"REJECT":bytes([1])})]
BASE_CASE=case
def shot_case(label,mode=5,pain=128,spawn_health=100,target_radius=16,target_height=56,secondary_health=100,secondary_flags=-1,target_mx=0,target_my=0,flip_tick=9999,**kw):
 c=BASE_CASE(label,mode=mode,flags=kw.pop('flags',2),reaction=kw.pop('reaction',0),radius=kw.pop('radius',20),flip_tick=flip_tick,**kw)
 c.update(melee=0,missile=4,attacksound=0,pain=pain,spawn_health=spawn_health,target_radius=target_radius*U,target_height=target_height*U,secondary_health=secondary_health,secondary_flags=secondary_flags,target_mx=target_mx*U,target_my=target_my*U)
 return c
# Actual original gameplay LUT, including all ten modulo-10 damage values.
table=base.table('m_random.c',r'unsigned char\s+rndtable\[256\]')
BASE_CASE=case
def melee_case(label,mode=5,pain=0,spawn_health=1000,target_radius=16,target_height=56,secondary_health=1000,secondary_flags=-1,target_mx=0,target_my=0,flip_tick=9999,extra_strike_tick=-1,reuse_tick=-1,**kw):
 kw.setdefault('ox',-32);kw.setdefault('players',roster(16));kw.setdefault('target_health',1000);kw.setdefault('stand',10);kw.setdefault('see',2)
 c=BASE_CASE(label,mode=mode,flags=kw.pop('flags',2),reaction=kw.pop('reaction',0),radius=kw.pop('radius',30),speed=kw.pop('speed',10),flip_tick=flip_tick,**kw)
 c.update(melee=4,missile=0,attacksound=52,pain=pain,spawn_health=spawn_health,target_radius=target_radius*U,target_height=target_height*U,secondary_health=secondary_health,secondary_flags=secondary_flags,target_mx=target_mx*U,target_my=target_my*U,extra_strike_tick=extra_strike_tick,reuse_tick=reuse_tick)
 return c
CASES=[melee_case('null',target=-1),melee_case('ordinary'),melee_case('outside',players=roster(96)),melee_case('wall',ox=112,players=roster(160)),melee_case('shadow',target_flags=262150),melee_case('shadow_miss',target_flags=262150,players=roster(96)),melee_case('pain_always',pain=256),melee_case('pain_never'),melee_case('kill_first',target_health=1),melee_case('extreme',spawn_health=1,target_health=1),melee_case('null_ambush',target=-1,flags=34,angle=0xdeadbeef),melee_case('rng_shadow_wrap',skip=250,target_flags=262150),melee_case('dead_shootable',target_health=0),melee_case('unshootable',target_flags=2),melee_case('self_target',target=0),melee_case('body_alive_roster_dead',players=roster(16,health=0)),melee_case('body_dead_roster_alive',target_health=-1)]
for radius in [0,4,16,30,32]:
 for delta in [-1,0,1]:
  c=melee_case(f'radius{radius}_edge{delta}',target_radius=radius);c['p0x']=c['ox']+(44+radius)*U+delta;CASES.append(c)
for damage in range(4,41,4):
 for shadow in [False,True]:
  index=next(i for i,value in enumerate(table) if (value%10+1)*4==damage)
  CASES.append(melee_case(f'damage{damage}_shadow{int(shadow)}',skip=(index-6-(2 if shadow else 0))&255,target_flags=262150 if shadow else 6))
for name,amount,odd in [('fall_forward36_taken',36,1),('fall_forward36_not_taken',36,0),('fall_forward40_no_draw',40,None)]:
 index=next(i for i,value in enumerate(table) if (value%10+1)*4==amount and (odd is None or table[(i+1)&255]&1==odd))
 CASES.append(melee_case(name,players=roster(16,z=80),target_health=1,skip=(index-6)&255))
for i,m in enumerate(MAPS):
 floor=m['heights'][1 if len(m['heights'])>1 else 0][0]
 CASES.append(melee_case(f'geometry_{i}',map=i,ox=-16,players=roster(32,z=floor)))
for x,y in [(48,0),(32,32),(0,48),(-32,32),(-48,0),(-32,-32),(0,-48),(32,-32)]:
 CASES.append(melee_case(f'bam_{x}_{y}',ox=0,players=roster(x,y)))
for height in [16,32,56,64]:CASES.append(melee_case(f'target_height{height}',target_height=height))
CASES += [
 melee_case('negative_coordinates',ox=-96,oy=-64,players=roster(-64,-32)),
 melee_case('chase_select',mode=3),
 melee_case('thinker_cycles',mode=4,target=-1,ox=-96,tics=150,stand=2,see=2),
 melee_case('thinker_shadow',mode=4,target=-1,tics=100,target_flags=262150,pain=256,stand=2,see=2),
 melee_case('thinker_pursue',mode=4,target=-1,ox=-96,tics=110,reaction=20,count=3,speed=2,stand=2,see=2),
 melee_case('thinker_dead',mode=4,enter=2,tics=70,target_health=1,spawn_health=1,pain=256),
 melee_case('thinker_wrap',mode=4,target=-1,tics=320,target_flags=262150,pain=256,see=1,stand=1,speed=2,skip=240),
 melee_case('moving_miss',mode=4,enter=2,tics=40,target_mx=4),
 melee_case('lost_sight',mode=4,map=18,enter=2,tics=55,players=[(-80,0,0,1,100),(16,0,0,0,100),*roster()[2:]],flip_tick=12,flip_target=2),
 melee_case('target_dies_during_animation',mode=4,enter=2,tics=26,target_health=1,skip=250,extra_strike_tick=12),
 melee_case('target_lost',mode=4,enter=2,tics=35,flip_tick=12,flip_target=-1),
 melee_case('shadow_change',mode=4,enter=2,tics=35,target_flags=262150,flip_tick=12,flip_flags=6),
 melee_case('target_change',mode=4,enter=2,tics=35,players=[(16,0,0,1,100),(-64,16,0,0,100),*roster()[2:]],flip_tick=12,flip_target=2),
 melee_case('reuse',mode=4,enter=4,tics=35,reuse_tick=3),
]
for state in [4,5,6]:CASES.append(melee_case(f'direct_state{state}',mode=4,enter=state,tics=35))

FUNCTIONS=sight.FUNCTIONS+[('p_enemy.c',n,r) for n,r in [('P_Move','boolean'),('P_TryWalk','boolean'),('P_NewChaseDir','void'),('A_Chase','void'),('P_CheckMeleeRange','boolean'),('P_CheckMissileRange','boolean')]]

MAIN=r"""
static void capture(void){int i;for(i=0;i<owned_count;i++){mobj_t* p=owned[i];printf("A,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%u,%d,%d,%d,%d\n",scenario,operation,i,alive[i],p->type,p->x,p->y,p->z,p->momx,p->momy,p->momz,p->health,p->state?(long)(p->state-states):-1L,p->tics,p->flags,p->height,p->radius,p->reactiontime,p->threshold,p->target?ident(p->target):-1,p->lastlook,p->floorz,p->ceilingz,(int)(p->subsector-subsectors),prndindex,p->angle,p->sprite,p->frame,p->movedir,p->movecount);}
for(i=0;i<4;i++)printf("R,%d,%d,%d,%d,%d,%d\n",scenario,operation,i,playeringame[i],players[i].health,players[i].mo?ident(players[i].mo):-1);for(i=0;i<numsectors;i++)printf("S,%d,%d,%d,%d\n",scenario,operation,i,sectors[i].soundtarget?ident(sectors[i].soundtarget):-1);}

static int radius_vectors(void){mobj_t a,b;int ir,cr,d,info[]={0,4,16,32},collision[]={0,4,32};P_InitThinkers();load_perception_world(0);scenario=-1;operation=0;memset(&a,0,sizeof(a));memset(&b,0,sizeof(b));mobjinfo[135]=mobjinfo[MT_PLAYER];a.info=&mobjinfo[MT_PLAYER];b.info=&mobjinfo[135];a.height=b.height=56*FRACUNIT;a.target=&b;owned_count=2;owned[0]=&a;owned[1]=&b;a.subsector=R_PointInSubsector(0,0);
for(ir=0;ir<4;ir++)for(cr=0;cr<3;cr++)for(d=-1;d<=1;d++){int ok;mobjinfo[135].radius=info[ir]*FRACUNIT;b.radius=collision[cr]*FRACUNIT;b.x=(44+info[ir])*FRACUNIT+d;b.subsector=R_PointInSubsector(b.x,0);ok=P_CheckMeleeRange(&a);printf("F,%d,%d,%d,%d,%d\n",mobjinfo[135].radius,b.radius,b.x,d,ok);}return 0;}

int main(int argc,char** argv){int c,i,t;int ids[1]={136};mobj_t* observer;
if(argc==2&&!strcmp(argv[1],"radius"))return radius_vectors();if(argc!=2||strcmp(argv[1],"trace"))return 2;
for(c=0;c<CASE_COUNT;c++){int* v=input[c];for(i=0;i<owned_count;i++)free(owned[i]);owned_count=0;memset(alive,0,sizeof(alive));memset(players,0,sizeof(players));memset(playeringame,0,sizeof(playeringame));scenario=c;operation=0;prndindex=0;query_index=-1;attempt=-1;gameskill=v[V_SKILL];P_InitThinkers();load_perception_world(v[V_MAP]);
memset(&states[1],0,sizeof(state_t));memset(&states[2],0,sizeof(state_t));states[1].sprite=SPR_SARG;states[1].tics=v[V_STAND];states[1].action.acp1=(actionf_p1)A_Look;states[1].nextstate=1;states[2].sprite=SPR_SARG;states[2].frame=0;states[2].tics=v[V_SEE];states[2].nextstate=2;states[2].action.acp1=(actionf_p1)A_Chase;
for(i=0;i<1;i++){mobjinfo[ids[i]]=mobjinfo[MT_PLAYER];mobjinfo[ids[i]].doomednum=-1;mobjinfo[ids[i]].spawnhealth=150;mobjinfo[ids[i]].mass=400;mobjinfo[ids[i]].spawnstate=1;mobjinfo[ids[i]].seestate=2;mobjinfo[ids[i]].seesound=v[V_SOUND_ID];mobjinfo[ids[i]].reactiontime=8;mobjinfo[ids[i]].flags=v[V_FLAGS];mobjinfo[ids[i]].height=v[V_OBSHEIGHT];mobjinfo[ids[i]].speed=v[V_SPEED];mobjinfo[ids[i]].radius=v[V_RADIUS];mobjinfo[ids[i]].activesound=v[V_ACTIVE_SOUND];mobjinfo[ids[i]].meleestate=v[V_MELEE];mobjinfo[ids[i]].missilestate=(ids[i]!=136&&v[V_MISSILE]==4)?7:v[V_MISSILE];mobjinfo[ids[i]].attacksound=v[V_ATTACKSOUND];}
states[4]=states[S_SARG_ATK1];states[4].nextstate=5;states[5]=states[S_SARG_ATK2];states[5].nextstate=6;states[6]=states[S_SARG_ATK3];states[6].nextstate=2;
mobjinfo[135]=mobjinfo[MT_PLAYER];mobjinfo[135].doomednum=-1;mobjinfo[135].spawnhealth=v[V_SPAWN_HEALTH];mobjinfo[135].painchance=v[V_PAIN];mobjinfo[135].flags=v[V_TARGET_FLAGS];mobjinfo[135].radius=v[V_TARGET_RADIUS];mobjinfo[135].height=v[V_TARGET_HEIGHT];
for(i=0;i<v[V_SKIP];i++){int draw=P_Random();observe(9,-1,draw);}
observer=P_SpawnMobj(v[V_OX],v[V_OY],v[V_OZ],v[V_OTYPE]);
for(i=0;i<4;i++){players[i].mo=P_SpawnMobj(v[V_P0X+5*i],v[V_P0Y+5*i],v[V_P0Z+5*i],135);playeringame[i]=v[V_P0ACTIVE+5*i];players[i].health=v[V_P0HEALTH+5*i];}
if(v[V_MODE]!=6){observer->angle=(angle_t)v[V_ANGLE];observer->threshold=v[V_THRESHOLD];observer->reactiontime=v[V_REACTION];observer->movedir=v[V_DIRECTION];observer->movecount=v[V_COUNT];observer->momx=v[V_MX];observer->momy=v[V_MY];observer->momz=v[V_MZ];players[0].mo->health=v[V_TARGET_HEALTH];players[0].mo->flags=v[V_TARGET_FLAGS];if(v[V_TARGET]>=0){observer->target=owned[v[V_TARGET]];observe(6,ident(observer),v[V_TARGET]);}}
for(i=1;i<4;i++){players[i].mo->health=v[V_SECONDARY_HEALTH];if(v[V_SECONDARY_FLAGS]>=0)players[i].mo->flags=v[V_SECONDARY_FLAGS];}players[0].mo->momx=v[V_TARGET_MX];players[0].mo->momy=v[V_TARGET_MY];
if(v[V_OTHER])P_SpawnMobj(-64*FRACUNIT,32*FRACUNIT,0,136);
if(v[V_ENTER])P_SetMobjState(observer,v[V_ENTER]);capture();
for(t=1;t<=v[V_TICS];t++){operation=t;query_index=-1;attempt=-1;
if(t==v[V_FLIP_TICK]){if(v[V_FLIP_SLOT]>=0)players[v[V_FLIP_SLOT]].health=v[V_FLIP_HEALTH];if(v[V_FLIP_FLAGS]>=0)players[0].mo->flags=v[V_FLIP_FLAGS];if(v[V_FLIP_ACTORHEALTH]!=-999)players[0].mo->health=v[V_FLIP_ACTORHEALTH];if(v[V_FLIP_TARGET]>=-1){observer->target=v[V_FLIP_TARGET]<0?NULL:owned[v[V_FLIP_TARGET]];observe(6,0,v[V_FLIP_TARGET]);}}
if(t==v[V_EXTRA_STRIKE_TICK]){observe(5,ident(observer),SARG_ACTION);A_SargAttack(observer);}
if(t==v[V_REUSE_TICK]){observer->target=NULL;observe(6,0,-1);playeringame[0]=0;players[0].mo=NULL;P_RemoveMobj(owned[1]);}
if(v[V_REUSE_TICK]>=0&&t==v[V_REUSE_TICK]+2){players[0].mo=P_SpawnMobj(v[V_P0X],v[V_P0Y],v[V_P0Z],135);playeringame[0]=1;players[0].health=v[V_SPAWN_HEALTH];observer->target=players[0].mo;observe(6,0,ident(players[0].mo));}
if(t==1&&v[V_SOUND]>=0)P_NoiseAlert(owned[v[V_SOUND]],owned[v[V_EMITTER]]);
if(v[V_MODE]==0)P_CheckMeleeRange(observer);else if(v[V_MODE]==1)P_CheckMissileRange(observer);else if(v[V_MODE]==2){observe(5,ident(observer),FACE_ACTION);A_FaceTarget(observer);}else if(v[V_MODE]==3)A_Chase(observer);else if(v[V_MODE]==4)P_RunThinkers();else if(v[V_MODE]==5){observe(5,ident(observer),SARG_ACTION);A_SargAttack(observer);}else if(v[V_MODE]==7){observe(5,ident(observer),CREFIRE_ACTION);A_CPosRefire(observer);}else if(v[V_MODE]!=6)abort();capture();}
}for(i=0;i<owned_count;i++)free(owned[i]);return 0;}
"""
WRAPPERS=r"""
boolean P_CheckSight(mobj_t* a,mobj_t* b){int pair=(int)(a->subsector->sector-sectors)*numsectors+(int)(b->subsector->sector-sectors);int reject=(rejectmatrix[pair>>3]&(1<<(pair&7)))!=0;boolean visible;query_index++;printf("V,%d,%d,%d,0,%d,%d,%d,%d\n",scenario,operation,query_index,ident(a),ident(b),pair,reject);visible=Raw_CheckSight(a,b);printf("Q,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,query_index,visible,reject?0:sightzstart,reject?0:bottomslope,reject?0:topslope);return visible;}
boolean P_LookForPlayers(mobj_t* a,boolean allaround){boolean result=Raw_LookForPlayers(a,allaround);printf("K,%d,%d,7,%d,%d,%d\n",scenario,operation,ident(a),result,a->lastlook);return result;}
"""

WRAPPERS += r"""
boolean P_Move(mobj_t* a){boolean r;pursuit_start(a,0);r=Raw_Move(a);pursuit_return(a,0,r);return r;}
boolean P_TryWalk(mobj_t* a){boolean r;pursuit_start(a,1);r=Raw_TryWalk(a);pursuit_return(a,1,r);return r;}
void P_NewChaseDir(mobj_t* a){pursuit_start(a,2);Raw_NewChaseDir(a);pursuit_return(a,2,0);}
void A_Chase(mobj_t* a){observe(5,ident(a),CHASE_ACTION);pursuit_start(a,3);Raw_Chase(a);pursuit_return(a,3,0);}
"""
def post_loader():
 code=['void load_perception_world(int which){int i,j;load_world(which);query_index=-1;validcount=0;memset(sightcounts,0,sizeof(sightcounts));memset(side_storage,0,sizeof(side_storage));memset(seg_storage,0,sizeof(seg_storage));memset(reject_storage,0,sizeof(reject_storage));for(i=0;i<numsectors;i++){sectors[i].linecount=0;sectors[i].lines=sector_lines[i];for(j=0;j<numlines;j++)if(lines[j].frontsector==&sectors[i]||lines[j].backsector==&sectors[i])sector_lines[i][sectors[i].linecount++]=&lines[j];}for(i=0;i<numlines;i++){sides[lines[i].sidenum[0]].sector=lines[i].frontsector;if(lines[i].sidenum[1]>=0)sides[lines[i].sidenum[1]].sector=lines[i].backsector;}switch(which){']
 for which,m in enumerate(MAPS):
  ls=lumps(m);code.append(f'case {which}:numsubsectors={len(m["heights"])};')
  for i,values in enumerate(struct.iter_unpack('<6H',ls['SEGS'])):
   a,b,angle,line,side,offset=values;l=m['lines'][line];front=l['front'] if side==0 else l['back'];back=l['back'] if side==0 else l['front'];code.append(f'segs[{i}].v1=&vertices[{a}];segs[{i}].v2=&vertices[{b}];segs[{i}].linedef=&lines[{line}];segs[{i}].frontsector=&sectors[{front}];segs[{i}].backsector='+('NULL' if back is None else f'&sectors[{back}]')+';')
  for i,(count,first) in enumerate(struct.iter_unpack('<2H',ls['SSECTORS'])):code.append(f'subsectors[{i}].firstline={first};subsectors[{i}].numlines={count};')
  for i,n in enumerate(m['nodes']):code.append(f'nodes[{i}].y={n["y"]}*FRACUNIT;')
  for i,b in enumerate(ls['REJECT']):code.append(f'rejectmatrix[{i}]={b};')
  code.append('break;')
 code.append('default:abort();}}');return '\n'.join(code)
def once(pattern,replacement,text):
 text,count=re.subn(pattern,replacement,text,count=1);assert count==1,pattern;return text
def run(check):
 assert {c['map'] for c in CASES}==set(range(len(MAPS))), 'Every supplied map must be actually exercised'
 combat.MAPS=MAPS
 shim=(ROOT/'tools/reference/m3d3f_shim.h').read_text().replace('RNG_TABLE',','.join(map(str,base.table('m_random.c',r'unsigned char\s+rndtable\[256\]'))))
 shim=shim.replace('query_index,stage,','query_index,stage,').replace('query_index,(long)(l-lines),stage','query_index,(long)(l-lines),stage').replace(',4,%ld,%d,%d,%d',',5,%ld,%d,%d,%d').replace(',7,%ld,%d',',8,%ld,%d')
 enemy=(SOURCE/'p_enemy.c').read_text()
 direction_tables=enemy[enemy.index('typedef enum'):enemy.index('void A_Fall')]
 direction_tables+='\n'+re.search(r'fixed_t\s+xspeed\[8\][^;]+;',enemy).group(0)+'\n'+re.search(r'fixed_t\s+yspeed\[8\][^;]+;',enemy).group(0)
 shim=shim.replace('/* PURSUIT_DIRECTION_TABLES */',direction_tables)
 parts=[]
 for file,name,result in FUNCTIONS:
  rename={'P_CheckSight':'Raw_CheckSight','P_LookForPlayers':'Raw_LookForPlayers','P_Move':'Raw_Move','P_TryWalk':'Raw_TryWalk','P_NewChaseDir':'Raw_NewChaseDir','A_Chase':'Raw_Chase','P_CheckMeleeRange':'Raw_Melee','P_CheckMissileRange':'Raw_Missile','P_AimLineAttack':'Raw_Aim','P_LineAttack':'Raw_LineAttack','P_ZMovement':'Raw_ZMovement'}.get(name)
  b=base.body(file,name,result,rename)
  if name=='P_SetThingPosition':b=b.rstrip()[:-1]+'printf("L,%d,%d,%d\\n",scenario,operation,ident(thing));}\n'
  if name=='P_SetMobjState':b=re.sub(r'(do\s*\{)',r'\1\n observe(4,ident(mobj),state);',b,count=1)
  if name=='P_SpawnMobj':b=b.replace('return mobj;','observe(1,ident(mobj),type);return mobj;')
  if name=='P_RemoveThinker':b=b.replace('// FIXME: NOP.','observe(2,ident(thinker),0);')
  if name=='P_RunThinkers':b=b.replace('currentthinker->function.acp1 (currentthinker);','{observe(0,ident(currentthinker),0);currentthinker->function.acp1(currentthinker);}')
  if name in ['P_SpawnPuff','P_SpawnBlood']:b=b.replace('    z += ((P_Random()-P_Random())<<10);','    z += jitter();')
  if name=='P_CrossBSPNode':b=b.replace('P_DivlineSide (strace.x, strace.y, (divline_t *)bsp)','node_side(strace.x,strace.y,(divline_t*)bsp,1)').replace('P_DivlineSide (t2x, t2y,(divline_t *)bsp)','node_side(t2x,t2y,(divline_t*)bsp,2)')
  if name=='P_CrossSubsector':
   b=b.replace('sub = &subsectors[num];','sub = &subsectors[num];printf("V,%d,%d,%d,3,%d\\n",scenario,operation,query_index,num);').replace('line = seg->linedef;','line = seg->linedef;printf("V,%d,%d,%d,4,%d,%ld,%ld,%d\\n",scenario,operation,query_index,num,(long)(seg-segs),(long)(line-lines),line->validcount==validcount);')
   b=b.replace('s2 = P_DivlineSide (v2->x, v2->y, &strace);','s2 = P_DivlineSide (v2->x, v2->y, &strace);line_sides(line,0,s1,s2);').replace('s2 = P_DivlineSide (t2x, t2y, &divl);','s2 = P_DivlineSide (t2x, t2y, &divl);line_sides(line,1,s1,s2);')
   b=once(r'if \( !\(line->flags & ML_TWOSIDED\) \)\s*return false;',r'if (!(line->flags & ML_TWOSIDED)){sight_block(line,0);return false;}',b)
   b=once(r'if \(openbottom >= opentop\)\s*return false;',r'printf("V,%d,%d,%d,6,%ld,%d,%d\\n",scenario,operation,query_index,(long)(line-lines),opentop,openbottom);if(openbottom>=opentop){sight_block(line,1);return false;}',b)
   b=once(r'if \(topslope <= bottomslope\)\s*return false;',r'printf("V,%d,%d,%d,7,%ld,%d,%d,%d\\n",scenario,operation,query_index,(long)(line-lines),frac,bottomslope,topslope);if(topslope<=bottomslope){sight_block(line,2);return false;}',b)
  if name=='P_LookForPlayers':
   b=b.replace('stop = (actor->lastlook-1)&3;','stop = (actor->lastlook-1)&3;printf("K,%d,%d,2,%d,%d,%d,%d\\n",scenario,operation,ident(actor),allaround,actor->lastlook,stop);')
   b=b.replace('if (!playeringame[actor->lastlook])','printf("K,%d,%d,3,%d,%d,%d,%d\\n",scenario,operation,ident(actor),actor->lastlook,playeringame[actor->lastlook],c);if (!playeringame[actor->lastlook])').replace('player = &players[actor->lastlook];','player = &players[actor->lastlook];printf("K,%d,%d,4,%d,%d,%d\\n",scenario,operation,ident(actor),actor->lastlook,player->health);')
   b=b.replace('if (an > ANG90 && an < ANG270)','if (!(an>ANG90&&an<ANG270))printf("K,%d,%d,5,%d,%d,%u,0,0,1\\n",scenario,operation,ident(actor),actor->lastlook,an);if (an > ANG90 && an < ANG270)')
   b=b.replace('// if real close, react anyway','printf("K,%d,%d,5,%d,%d,%u,1,%d,%d\\n",scenario,operation,ident(actor),actor->lastlook,an,dist,dist<=MELEERANGE);\n // if real close, react anyway').replace('actor->target = player->mo;','actor->target = player->mo;observe(6,ident(actor),ident(player->mo));')
  if name=='A_Look':
   b=b.replace('{','{\n observe(5,ident(actor),'+str(LOOK)+');printf("K,%d,%d,0,%d,%d\\n",scenario,operation,ident(actor),actor->threshold);',1)
   b=b.replace('targ = actor->subsector->sector->soundtarget;','targ = actor->subsector->sector->soundtarget;printf("K,%d,%d,1,%d,%d,%d,%d\\n",scenario,operation,ident(actor),targ?ident(targ):-1,targ?!!(targ->flags&MF_SHOOTABLE):0,!!(actor->flags&MF_AMBUSH));').replace('actor->target = targ;','actor->target = targ;observe(6,ident(actor),ident(targ));').replace('P_Random()','look_random(actor)').replace('P_SetMobjState (actor, actor->info->seestate);','printf("K,%d,%d,8,%d,%d\\n",scenario,operation,ident(actor),actor->info->seestate);P_SetMobjState (actor, actor->info->seestate);')
  if name=='P_RecursiveSound':b=b.replace('sec->soundtarget = soundtarget;','sec->soundtarget = soundtarget;printf("N,%d,%d,%ld,%d,%d\\n",scenario,operation,(long)(sec-sectors),soundblocks,ident(soundtarget));')

  if name=='PIT_CheckThing':
   b=re.sub(r'(\{)',r'\1\n pcand(0,ident(thing));',b,count=1).replace('return false;','{block_actor=ident(thing);move_failure=1;return false;}').replace('return !(thing->flags & MF_SOLID);','return block_result(!(thing->flags & MF_SOLID),thing);')
  if name=='PIT_CheckLine':
   b=re.sub(r'(\{)',r'\1\n pcand(1,(int)(ld-lines));',b,count=1).replace('return false;','{block_line=(int)(ld-lines);move_failure=2;return false;}').replace('    P_LineOpening (ld);','    P_LineOpening (ld);\n printf("O,%d,%d,%d,%ld,%d,%d,%d,%d\\n",scenario,operation,attempt,(long)(ld-lines),opentop,openbottom,openrange,lowfloor);')
  if name=='P_TryMove':
   b=re.sub(r'(\{)',r'\1\n attempt++;block_actor=block_line=-1;move_failure=0;',b,count=1)
   for text,reason in [("return false;\t// doesn't fit",3),("return false;\t// mobj must lower itself to fit",4),("return false;\t// too big a step up",5),("return false;\t// don't stand over a dropoff",6)]:b=b.replace(text,'{move_failure='+str(reason)+';return false;}')
   b=b.replace('return false;','{record_move(thing,x,y,0);return false;}').replace('    return true;','    record_move(thing,x,y,1);return true;')
  if name=='P_ZMovement':b+='\nvoid P_ZMovement(mobj_t* p){int z=p->z,m=p->momz,f=p->floorz,c=p->ceilingz;int floor=z+m<=f;int ceil=z+m+p->height>c;int gravity=!floor&&!(p->flags&MF_NOGRAVITY);Raw_ZMovement(p);printf("Z,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\\n",scenario,operation,ident(p),z,m,p->z,p->momz,f,c,floor,ceil,gravity);}\n'
  if name=='P_TryWalk':b=re.sub(r'P_Random\s*\(\s*\)',r'pursuit_random(actor,0)',b)
  if name=='P_NewChaseDir':
   kinds=iter([1,2]);b=re.sub(r'P_Random\s*\(\s*\)',lambda m:'pursuit_random(actor,'+str(next(kinds))+')',b)
   b=re.sub(r'(actor->movedir\s*=\s*[^;]+;)',r'\1 pursuit_direction(actor);',b)
  if name=='A_Chase':b=re.sub(r'P_Random\s*\(\s*\)',r'pursuit_random(actor,3)',b).replace('(actor->movedir << 29)','((angle_t)actor->movedir << 29)')
  b=instrument(name,b)
  parts.append(b)
 parts.insert(0,base.body('m_random.c','P_Random','int','Raw_Random')+'int P_Random(void){int value=Raw_Random();observe(8,-1,value);return value;}\nstatic int jitter(void){int a=combat_random(3);int b=combat_random(4);return (a-b)*1024;}')
 for name in actors.ACTIONS[1:]:
  if name in ['A_Look','A_Chase']:continue
  if name in ['A_FaceTarget','A_PosAttack','A_SPosAttack','A_CPosAttack','A_CPosRefire','A_SargAttack']:
   parts.append(attack_action(name));continue
  if name in ['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']:
   b=base.body('p_enemy.c',name,'void');arg='mo' if name=='A_PlayerScream' else 'actor';b=re.sub(r'(\{)',r'\1\n observe(5,ident('+arg+'),'+str(actors.ACTIONS.index(name))+');',b,count=1)
  else:b=f'void {name}(mobj_t* actor){{(void)actor;abort();}}'
  parts.append(b)
 def c_int(v):return str(v if v<2147483648 else v-4294967296)
 fields=list(CASES[0])[1:]
 text=shim.replace('/* SOURCE_BODIES */','\n'.join(parts)+'\n'+WRAPPERS+ATTACK_WRAPPERS)+'\n'+combat.c_maps()+'\n'+post_loader()+'\nenum {'+','.join('V_'+f.upper() for f in fields)+'};\n#define CASE_COUNT '+str(len(CASES))+'\nint input[]['+str(len(fields))+']={'+','.join('{'+','.join(c_int(c[f]) for f in fields)+'}' for c in CASES)+'};\n'+MAIN
 outputs={f'tests/reference/m3d3f_map_{i}.bin':m['bytes'] for i,m in enumerate(MAPS)}
 with tempfile.TemporaryDirectory(prefix='doom-m3d3f-reference-') as tmp:
  src=Path(tmp)/'reference.c';src.write_text(text);binary=Path(tmp)/'reference';subprocess.run(['cc',*FLAGS,'-I'+str(SOURCE),str(src),str(SOURCE/'m_fixed.c'),str(SOURCE/'tables.c'),'-o',str(binary)],check=True)
  component=subprocess.check_output([str(binary),'radius'],timeout=30).decode();outputs['tests/reference/m3d3f_melee_radius.csv']=('info_radius,collision_radius,x,delta,accepted\n'+'\n'.join(line[2:] for line in component.splitlines() if line.startswith('F,'))+'\n').encode()
  for mode in ['trace']:
   result=subprocess.run([str(binary),mode],stdout=subprocess.PIPE,timeout=30)
   if result.returncode:
    Path('/tmp/m3d3f-failed-output').write_bytes(result.stdout);Path('/tmp/m3d3f-failed-reference.c').write_text(text)
    import shutil;shutil.copy(binary,'/tmp/m3d3f-failed-reference');raise RuntimeError('Original C aborted; bounded diagnostics in /tmp/m3d3f-failed-output')
   outputs[f'tests/reference/m3d3f_{mode}.csv']=result.stdout
 table=io.StringIO();w=csv.writer(table,lineterminator='\n');w.writerow(['case',*CASES[0]]);w.writerows([[i,*c.values()] for i,c in enumerate(CASES)]);outputs['tests/reference/m3d3f_cases.csv']=table.getvalue().encode()
 files=sorted({f for f,_,_ in FUNCTIONS}|{'info.c','m_random.c','m_fixed.c','tables.c','p_enemy.c'}|{p.name for p in SOURCE.glob('*.h')})
 extracted=FUNCTIONS+[('p_enemy.c',n,'void') for n in ['A_FaceTarget','A_PosAttack','A_SPosAttack','A_CPosAttack','A_CPosRefire','A_SargAttack']]+[('m_random.c','P_Random','int')]+[('p_enemy.c',n,'void') for n in ['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']]
 trace_rows=list(csv.reader(io.StringIO(outputs['tests/reference/m3d3f_trace.csv'].decode())))
 observed_tags={'E','J','G','X','H','V','K','D','B','U','I','T','M','C','O','Z','Y'}
 inventory={'scenarios':len(CASES),'compiled_original_bodies':len(extracted),'supplied_maps':len(MAPS),'exercised_maps':sorted({c['map'] for c in CASES}),'actor_fields':sum(26 for row in trace_rows if row[0]=='A' and row[4]!='0'),'raw_gameplay_draws_including_setup':sum(1 for row in trace_rows if row[0]=='E' and row[3]=='8'),'ordered_instrumentation_rows':sum(1 for row in trace_rows if row[0] in observed_tags),'radius_component_vectors':36}
 manifest={'evidence_inventory':inventory,'revision':'a77dfb96cb91780ca334d0d4cfd86957558007e0','functions':extracted,'function_sha256':{f+':'+n:hashlib.sha256(base.body(f,n,r).encode()).hexdigest() for f,n,r in extracted},'direction_tables_sha256':hashlib.sha256(direction_tables.encode()).hexdigest(),'flags':FLAGS,'source_sha256':{f:hashlib.sha256((SOURCE/f).read_bytes()).hexdigest() for f in files},'adapter_sha256':{f:hashlib.sha256((ROOT/'tools/reference'/f).read_bytes()).hexdigest() for f in ['generate.py','generate_m3b.py','generate_m3c.py','generate_m3d1.py','generate_m3d3a.py','generate_m3d3b.py','generate_m3d3c.py','generate_m3d3f.py','m3d3f_shim.h']},'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()},'assumptions':['real original structs/info and 32-bit fixed/int, 64-bit pointer/function-union ABI','exact sight horizontal x==node.y quirk, shifts/division and BSP order retained','labelled normal-timing native136 SARG8/8/8 owner adapted from source player body with Demon health150/mass400; genuine SargAttack/Face/range/Chase/direct damage; full original Demon/Spectre/player/scoring services excluded','empty fallback roster, stale identities and unsafe extremes excluded from original-C execution and tested as declared Rust safety policies','source retired memory retained solely for observation; no Rust expected values or proprietary data read','left-before-right shadow/spread/jitter draws and representable multiplication define historical unspecified operand order and signed-shift intent','all modulo-10 damage values come from the actual original LUT; no injected gameplay RNG','unsigned BAM movedir shift adapter defines original signed-shift intent, preserving 32-bit two-complement delta; countdowns avoid INT_MIN and abs overflow']}
 path=ROOT/'tests/reference/m3d3f_provenance.json'
 if check:
  old=json.loads(path.read_text())
  for k,v in manifest.items():assert old[k]==json.loads(json.dumps(v)),k
 else:
  manifest.update(compiler=subprocess.check_output(['cc','--version'],text=True).splitlines()[0],platform=platform.platform());outputs['tests/reference/m3d3f_provenance.json']=(json.dumps(manifest,indent=2)+'\n').encode()
 for p,b in outputs.items():
  if check:assert (ROOT/p).read_bytes()==b,p
  else:(ROOT/p).write_bytes(b)
 print(('Checked' if check else 'Generated')+f' M3d3f {len(CASES)} original-body Demon melee scenarios.')
# Observation adapters retain genuine branch/function bodies.
def instrument(name,b):
 if name=='P_CheckMeleeRange':
  for replacement in ['return range_result(actor,0,0,1,0,0,0,0);','return range_result(actor,0,0,2,1,dist,1,MELEERANGE-20*FRACUNIT+pl->info->radius);','return range_result(actor,0,0,3,1,dist,1,MELEERANGE-20*FRACUNIT+pl->info->radius);']:
   b=b.replace('return false;',replacement,1)
  b=b.replace('return true;','return range_result(actor,0,1,0,1,dist,1,MELEERANGE-20*FRACUNIT+pl->info->radius);')
 if name=='P_CheckMissileRange':
  for replacement in ['return range_result(actor,1,0,3,0,0,0,0);','return range_result(actor,1,0,5,0,0,0,0);','return range_result(actor,1,0,6,1,dist,0,0);','return range_result(actor,1,0,7,1,dist,0,0);','return range_result(actor,1,0,8,1,dist,1,dist);']:
   b=b.replace('return false;',replacement,1)
  b=b.replace('return true;','return range_result(actor,1,1,4,0,0,0,0);',1)
  b=b.replace('return true;','return range_result(actor,1,1,0,1,dist,1,dist);',1)
  b=b.replace('P_Random ()','monster_random(actor,5)')
 if name=='A_Chase':
  b=b.replace('if (actor->info->attacksound)','printf("J,%d,%d,3,%d,0,%d\\n",scenario,operation,ident(actor),actor->info->meleestate);if (actor->info->attacksound)')
  b=b.replace('P_SetMobjState (actor, actor->info->missilestate);','printf("J,%d,%d,3,%d,1,%d\\n",scenario,operation,ident(actor),actor->info->missilestate);P_SetMobjState (actor, actor->info->missilestate);')
  b=b.replace('actor->flags |= MF_JUSTATTACKED;','actor->flags |= MF_JUSTATTACKED;printf("J,%d,%d,4,%d,%u\\n",scenario,operation,ident(actor),(unsigned)actor->flags);')
 if name=='P_SetMobjState':
  b=b.replace('if (st->action.acp1)','if(st->action.acp1==(actionf_p1)A_SargAttack)observe(5,ident(mobj),SARG_ACTION);if(st->action.acp1==(actionf_p1)A_CPosAttack)observe(5,ident(mobj),CPOS_ACTION);if(st->action.acp1==(actionf_p1)A_CPosRefire)observe(5,ident(mobj),CREFIRE_ACTION);if (st->action.acp1)')
  b=b.replace('if (st->action.acp1)','if(st->action.acp1==(actionf_p1)A_SPosAttack)observe(5,ident(mobj),SPOS_ACTION);if (st->action.acp1)')
  b=b.replace('if (st->action.acp1)','if(st->action.acp1==(actionf_p1)A_FaceTarget)observe(5,ident(mobj),FACE_ACTION);if(st->action.acp1==(actionf_p1)A_PosAttack)observe(5,ident(mobj),POS_ACTION);if (st->action.acp1)')
 if name=='P_PathTraverse':
  b=b.replace('    trace.dy = y2 - y1;','    trace.dy = y2 - y1;printf("D,%d,%d,%d,%d,%d,%d,%d\\n",scenario,operation,trace_stage,trace.x,trace.y,trace.dx,trace.dy);')
  b=b.replace('for (count = 0 ; count < 64 ; count++)\n    {','for (count = 0 ; count < 64 ; count++)\n    {printf("B,%d,%d,%d,%d,%d\\n",scenario,operation,trace_stage,mapx,mapy);')
 if name in ['PIT_AddLineIntercepts','PIT_AddThingIntercepts']:
  b=b.replace('    // avoid precision','    candidate(1,(int)(ld-lines));\n    // avoid precision') if name=='PIT_AddLineIntercepts' else b.replace('    tracepositive =','    candidate(0,ident(thing));\n    tracepositive =')
  b=b.replace('    intercept_p->frac = frac;','    if(intercept_p-intercepts>=128)abort();\n    intercept_p->frac = frac;')
  b=b.replace('    intercept_p++;','printf("I,%d,%d,%d,%d,%d,%d\\n",scenario,operation,trace_stage,intercept_p->isaline,intercept_id(intercept_p),intercept_p->frac);intercept_p++;')
 if name in ['PTR_AimTraverse','PTR_ShootTraverse']:
  b=b.replace('{','{\n trace_count++;printf("T,%d,%d,%d,%d,%d,%d\\n",scenario,operation,trace_stage,in->isaline,intercept_id(in),in->frac);',1)
  if name=='PTR_AimTraverse':b=b.replace('return false;','{trace_stop_kind=in->isaline;trace_stop_id=intercept_id(in);return false;}')
  else:b=b.replace('// Spawn bullet puffs or blod spots,','last_kind=0;last_object=ident(th);\n // Spawn bullet puffs or blod spots,').replace('// Spawn bullet puffs.','last_kind=1;last_object=(int)(li-lines);\n // Spawn bullet puffs.')
 if name in ['P_SpawnPuff','P_SpawnBlood']:
  b=b.replace('    z += jitter();','    impact[0]=x;impact[1]=y;impact[2]=z;\n    z += jitter();').replace('th->tics -= P_Random()&3;','th->tics -= combat_random(5)&3;')
  b=b.rstrip()[:-1]+'printf("G,%d,%d,11,%d\\n",scenario,operation,ident(th));last_effect=ident(th);}\n'
 if name=='P_DamageMobj':
  b=b.replace('{','{\n int damage_oldhealth=target->health;printf("G,%d,%d,2,%d,%d,%d,%d\\n",scenario,operation,ident(target),inflictor?ident(inflictor):-1,source?ident(source):-1,damage);',1)
  b=b.replace('return;\t// shouldn\'t happen...','{printf("G,%d,%d,12,%d\\n",scenario,operation,ident(target));return;}')
  b=b.replace('if (target->health <= 0)\n\treturn;','if (target->health <= 0){printf("G,%d,%d,12,%d\\n",scenario,operation,ident(target));return;}')
  b=b.replace('P_Random ()','combat_random(0)',1).replace('P_Random ()','combat_random(1)',1)
  b=b.replace('ang >>= ANGLETOFINESHIFT;','knock_angle=ang;ang >>= ANGLETOFINESHIFT;')
  b=b.replace('target->momy += FixedMul (thrust, finesine[ang]);','target->momy += FixedMul (thrust, finesine[ang]);printf("G,%d,%d,4,%d,%u,%d,%d,%d\\n",scenario,operation,ident(target),knock_angle,thrust,target->momx,target->momy);')
  b=b.replace('target->health -= damage;','target->health -= damage;printf("G,%d,%d,5,%d,%d,%d\\n",scenario,operation,ident(target),damage_oldhealth,target->health);')
  b=b.replace('P_SetMobjState (target, target->info->painstate);','printf("G,%d,%d,6,%d,%d\\n",scenario,operation,ident(target),target->info->painstate);P_SetMobjState (target, target->info->painstate);')
  b=b.replace('target->reactiontime = 0;','target->reactiontime = 0;printf("G,%d,%d,9,%d\\n",scenario,operation,ident(target));')
  b=b.replace('target->target = source;','target->target = source;observe(6,ident(target),ident(source));')
  b=b.replace('target->threshold = BASETHRESHOLD;','target->threshold = BASETHRESHOLD;printf("G,%d,%d,10,%d,%d\\n",scenario,operation,ident(target),ident(source));')
 if name=='P_KillMobj':
  b=b.replace('// Drop stuff.','printf("G,%d,%d,8,%d,%d\\n",scenario,operation,ident(target),target->tics);\n // Drop stuff.')
  b=b.replace('    if (target->health < -target->info->spawnhealth','    printf("G,%d,%d,7,%d,%d,%d\\n",scenario,operation,ident(target),(target->health < -target->info->spawnhealth && target->info->xdeathstate)?target->info->xdeathstate:target->info->deathstate,!!(target->health < -target->info->spawnhealth && target->info->xdeathstate));\n    if (target->health < -target->info->spawnhealth')
  b=b.replace('P_Random()','combat_random(2)')
  b=b.replace('// I_StartSound (&actor->r, actor->info->deathsound);','printf("G,%d,%d,8,%d,%d\\n",scenario,operation,ident(target),target->tics);\n // I_StartSound (&actor->r, actor->info->deathsound);')
 return b

def attack_action(name):
 b=base.body('p_enemy.c',name,'void','Raw_FaceTarget' if name=='A_FaceTarget' else None)
 if name=='A_FaceTarget':
  b=b.replace('(P_Random()-P_Random())<<21','shadow(actor)')
  b+='\nvoid A_FaceTarget(mobj_t* a){unsigned angle=a->angle,flags=a->flags;Raw_FaceTarget(a);printf("J,%d,%d,2,%d,%d,%u,%u,%u,%u\\n",scenario,operation,ident(a),a->target?ident(a->target):-1,angle,a->angle,flags,(unsigned)a->flags);}\n'
 elif name=='A_SargAttack':
  b=b.replace('P_Random()%10','monster_random(actor,4)%10')
 elif name=='A_CPosRefire':
  b=b.replace('P_Random ()','monster_random(actor,6)')
  b=once(r'if \(monster_random\(actor,6\) < 40\)\s*return;',r'if(monster_random(actor,6)<40){refire_result(actor,0);return;}',b)
  b=b.replace('P_SetMobjState (actor, actor->info->seestate);','refire_result(actor,!actor->target?1:(actor->target->health<=0?2:3));P_SetMobjState (actor, actor->info->seestate);')
  b=b.rstrip()[:-1].rstrip()
  assert b.endswith('}')
  b+='else {refire_result(actor,4);}}'
 elif name=='A_CPosAttack':
  b=b.replace('(P_Random()-P_Random())<<20','spread(actor)').replace('P_Random()%5','monster_random(actor,4)%5')
  b=b.replace('P_LineAttack (actor,','pellet_index=1;P_LineAttack (actor,')
 elif name=='A_SPosAttack':
  b=b.replace('S_StartSound (actor, sfx_shotgn);','printf("W,%d,%d,0,%d\\n",scenario,operation,ident(actor));S_StartSound (actor, sfx_shotgn);')
  b=b.replace('for (i=0 ; i<3 ; i++)\n    {','for (i=0 ; i<3 ; i++)\n    {pellet_index=i+1;printf("W,%d,%d,1,%d,%d\\n",scenario,operation,ident(actor),pellet_index);')
  b=b.replace('(P_Random()-P_Random())<<20','spread(actor)').replace('P_Random()%5','monster_random(actor,4)%5')
  b=b.replace('P_LineAttack (actor, angle, MISSILERANGE, slope, damage);','printf("W,%d,%d,2,%d,%d,%u,%d,%d\\n",scenario,operation,ident(actor),pellet_index,(unsigned)angle,slope,damage);P_LineAttack (actor, angle, MISSILERANGE, slope, damage);printf("W,%d,%d,3,%d,%d,%d,%d\\n",scenario,operation,ident(actor),pellet_index,prndindex,last_effect);')
  b=b.rstrip()[:-1]+'printf("W,%d,%d,4,%d,%d\\n",scenario,operation,ident(actor),prndindex);}'
 else:
  b=b.replace('(P_Random()-P_Random())<<20' ,'spread(actor)').replace('P_Random()%5','monster_random(actor,4)%5')
 return b
ATTACK_WRAPPERS=r"""
boolean P_CheckMeleeRange(mobj_t* a){return Raw_Melee(a);}
boolean P_CheckMissileRange(mobj_t* a){return Raw_Missile(a);}
fixed_t P_AimLineAttack(mobj_t* a,angle_t angle,fixed_t range){fixed_t slope;trace_stage=0;trace_count=0;trace_stop_kind=trace_stop_id=-1;slope=Raw_Aim(a,angle,range);printf("J,%d,%d,5,%d,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,ident(a),linetarget?ident(linetarget):-1,slope,bottomslope,topslope,trace_count,trace_stop_kind,trace_stop_id);return slope;}
void P_LineAttack(mobj_t* a,angle_t angle,fixed_t range,fixed_t slope,int damage){printf("X,%d,%d,0,%d,%u,%d,%d,%d\n",scenario,operation,ident(a),angle,range,slope,damage);trace_stage=pellet_index;trace_count=0;last_effect=-1;last_kind=last_object=-1;memset(impact,0,sizeof(impact));Raw_LineAttack(a,angle,range,slope,damage);printf("X,%d,%d,1,%d,%d,%d,%d,%d,%d\n",scenario,operation,last_kind,last_object,impact[0],impact[1],impact[2],trace_count);}
"""

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
