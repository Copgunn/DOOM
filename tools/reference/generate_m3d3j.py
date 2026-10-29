#!/usr/bin/env python3
"""Genuine original THING loader/spawner plus declared Demon fixture gameplay.
Unadapted source spawn evidence is separate from native-adapted gameplay.
No Rust output or proprietary data is read.
"""
import argparse,copy,csv,hashlib,io,json,platform,re,struct,subprocess,tempfile
from pathlib import Path
import generate_m3d3h as previous
base,maps,actors,combat,sight=previous.base,previous.maps,previous.actors,previous.combat,previous.sight
ROOT,SOURCE,FLAGS=base.ROOT,base.SOURCE,base.FLAGS
U=65536
LOOK=previous.LOOK
instrument,attack_action,once=previous.instrument,previous.attack_action,previous.once
WRAPPERS,ATTACK_WRAPPERS=previous.WRAPPERS,previous.ATTACK_WRAPPERS
MAPS=[];THING_INPUTS=[];GEOMETRIES=[];CASES=[]
def fixture(geometry,things):
 key=(geometry,tuple(tuple(t) for t in things))
 if key in GEOMETRIES:return GEOMETRIES.index(key)
 m=copy.deepcopy(previous.MAPS[geometry]);data=b''.join(struct.pack('<5h',*t) for t in things)
 sight.replace_lumps(m,{'THINGS':data});MAPS.append(m);THING_INPUTS.append(data);GEOMETRIES.append(key);return len(MAPS)-1
POSITIONS=[(-80,-80),(80,-80),(-80,80),(80,80)]
def demon(x=-64,y=0,angle=0,options=7):return (x,y,angle,3002,options)
def case(label,geometry=0,things=None,skill=2,no_monsters=0,skip=0,tics=40,enter_all=0,body_health=1000,roster_health=100,damage_tick=-1,damage=1,kill_tick=-1,kill=150,sound_tick=-1,retire_tick=-1):
 c=dict(label=label,map=fixture(geometry,things if things is not None else [demon()]),skill=skill,no_monsters=no_monsters,skip=skip,tics=tics,mode=4,enter_all=enter_all,body_health=body_health,damage_tick=damage_tick,damage=damage,kill_tick=kill_tick,kill=kill,sound_tick=sound_tick,retire_tick=retire_tick)
 for i,(x,y,z,active,health) in enumerate([(96,0,0,1,roster_health),(-200,-96,0,0,100),(-200,-96,0,0,100),(-200,-96,0,0,100)]):
  for field,value in zip(['x','y','z','active','health'],[x*U,y*U,z*U,active,health]):c[f'p{i}{field}']=value
 CASES.append(c);return c
case('one_idle',roster_health=0,tics=45)
case('one_wake',tics=90)
for n in [2,3,4]:case(f'multiple_{n}',things=[demon(x,y) for x,y in POSITIONS[:n]],tics=100)
for skill in range(4):case(f'skill_group_{skill}',skill=skill,things=[demon(-96,-96,options=1),demon(96,-96,options=2),demon(-96,96,options=4),demon(96,96,options=7)],tics=35)
case('single_player_skip',things=[demon(options=23),demon(options=0),demon(-96,-64), (0,0,0,32000,0),(0,0,0,2011,16)],tics=30)
case('nomonsters_original_flags',things=[demon(x,y) for x,y in POSITIONS],no_monsters=1,tics=3)
case('all_filtered',things=[demon(options=0),(0,0,0,32000,16)],tics=3)
case('ambush',things=[demon(options=15)],sound_tick=2,tics=70)
for a in [-32768,-359,-91,-90,-46,-45,-44,-1,0,44,45,46,89,90,179,180,269,270,359,360,32767]:
 case(f'angle_{a}',things=[demon(-64,-32,a)],tics=0)
case('nonzero_floor',geometry=20,things=[demon(-64,-64),demon(64,64)],tics=45)
for i in range(21):case(f'geometry_{i}',geometry=i,things=[demon(-64,-64),demon(64,64)],tics=45)
for i in range(256):case(f'spawn_lut_{i}',skip=i,tics=0)
for i in range(16):case(f'multi_rng_{i}',skip=240+i,things=[demon(x,y) for x,y in POSITIONS],tics=65)
case('shared_target_death',things=[demon(-32,-32),demon(32,32,180)],enter_all=5,body_health=1,tics=35)['p0x']=0
case('spawn_lifecycle',skip=1,damage_tick=45,kill_tick=75,tics=135)
case('multiple_lifecycle',things=[demon(x,y) for x,y in POSITIONS],damage_tick=20,kill_tick=55,tics=110)
case('corpse_source_reuse',enter_all=4,damage_tick=1,damage=150,retire_tick=40,tics=70)
case('signed_coordinates',things=[demon(-128,-64),demon(128,64)],tics=45)
case('spawn_boundaries',things=[demon(-978,-978),demon(978,978)],tics=0,roster_health=0)
# All earlier public inputs are replayed with original COUNTKILL restored.
for c in CASES:
 c.update(seed=0,continue_rng=0,source_kind=1,inflictor_kind=0,kill_all=0,remove_corpse=-1)
def context_case(label,**kw):
 special={k:kw.pop(k) for k in list(kw) if k in ['continue_rng','source_kind','inflictor_kind','kill_all','remove_corpse']}
 c=case(label,**kw);c.update(seed=0,continue_rng=0,source_kind=1,inflictor_kind=0,kill_all=0,remove_corpse=-1);c.update(special);return c
context_case('empty_level',things=[],tics=5)
context_case('maximum_kills',things=[demon(x,y) for x,y in POSITIONS],kill_tick=1,kill_all=1,tics=100)
for source_kind in range(4):
 context_case(f'credit_source_{source_kind}',things=[demon(-64,-64),demon(64,64)],source_kind=source_kind,kill_tick=2,tics=90)
for inflictor_kind in [1,2,3]:
 context_case(f'credit_inflictor_{inflictor_kind}',things=[demon(-64,-64),demon(64,64)],inflictor_kind=inflictor_kind,kill_tick=2,tics=75)
context_case('repeated_dead_damage',damage_tick=1,damage=150,kill_tick=2,tics=110)
context_case('ordinary_overkill',kill_tick=1,kill=1000,tics=100)
context_case('terminal_no_respawn',kill_tick=1,tics=1000)
context_case('corpse_removal_total',kill_tick=1,remove_corpse=40,tics=70)
context_case('restart_before',things=[demon(-64,-64),demon(64,64)],kill_tick=2,tics=90,skip=247)
context_case('restart_after',things=[demon(-64,-64),demon(64,64)],kill_tick=20,tics=90,continue_rng=1)
context_case('reset_independent',things=[demon(-64,-64),demon(64,64)],kill_tick=20,tics=90)
hit=context_case('hitscan_credit',kill_tick=2,tics=100);hit.update(hitscan=1,p0x=0)
for c in CASES:c.setdefault('hitscan',0)
FUNCTIONS=previous.FUNCTIONS+[('p_setup.c','P_LoadThings','void'),('p_mobj.c','P_SpawnMapThing','void')]
MAIN=r"""
static void capture(void){int i;printf("LS,%d,%d,%d,%d,%d,%d\n",scenario,operation,totalkills,totalitems,totalsecret,players[0].killcount);for(i=0;i<owned_count;i++){mobj_t* p=owned[i];printf("A,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%u,%d,%d,%d,%d\n",scenario,operation,i,alive[i],p->type,p->x,p->y,p->z,p->momx,p->momy,p->momz,p->health,p->state?(long)(p->state-states):-1L,p->tics,p->flags,p->height,p->radius,p->reactiontime,p->threshold,p->target?ident(p->target):-1,p->lastlook,p->floorz,p->ceilingz,(int)(p->subsector-subsectors),prndindex,p->angle,p->sprite,p->frame,p->movedir,p->movecount);}
for(i=0;i<4;i++)printf("R,%d,%d,%d,%d,%d,%d\n",scenario,operation,i,playeringame[i],players[i].health,players[i].mo?ident(players[i].mo):-1);for(i=0;i<numsectors;i++)printf("S,%d,%d,%d,%d\n",scenario,operation,i,sectors[i].soundtarget?ident(sectors[i].soundtarget):-1);}


static void definitions(void){int i;{int originals[]={S_SARG_STND,S_SARG_STND2,S_SARG_RUN1,S_SARG_RUN2,S_SARG_RUN3,S_SARG_RUN4,S_SARG_RUN5,S_SARG_RUN6,S_SARG_RUN7,S_SARG_RUN8};int ids[]={1,3,2,15,16,17,18,19,20,21};int next[]={3,1,15,16,17,18,19,20,21,2};for(i=0;i<10;i++){states[ids[i]]=states[originals[i]];states[ids[i]].nextstate=next[i];}}
{int originals[]={S_SARG_ATK1,S_SARG_ATK2,S_SARG_ATK3,S_SARG_PAIN,S_SARG_PAIN2,S_SARG_DIE1,S_SARG_DIE2,S_SARG_DIE3,S_SARG_DIE4,S_SARG_DIE5,S_SARG_DIE6};int next[]={5,6,2,8,2,10,11,12,13,14,0};for(i=0;i<11;i++){states[4+i]=states[originals[i]];states[4+i].nextstate=next[i];}}
mobjinfo[136]=mobjinfo[MT_SERGEANT];mobjinfo[136].doomednum=-1;mobjinfo[136].spawnstate=1;mobjinfo[136].seestate=2;mobjinfo[136].painstate=7;mobjinfo[136].meleestate=4;mobjinfo[136].deathstate=9;mobjinfo[136].xdeathstate=0;mobjinfo[136].raisestate=0;mobjinfo[136].flags=4194310;
mobjinfo[135]=mobjinfo[MT_PLAYER];mobjinfo[135].doomednum=-1;mobjinfo[135].spawnhealth=1000;mobjinfo[135].painchance=0;mobjinfo[135].radius=16*FRACUNIT;mobjinfo[135].flags=6;
}

static void reset(int c){int i;for(i=0;i<owned_count;i++)free(owned[i]);owned_count=0;memset(alive,0,sizeof(alive));memset(players,0,sizeof(players));memset(playeringame,0,sizeof(playeringame));scenario=c;operation=0;prndindex=0;query_index=-1;attempt=-1;totalkills=91;totalitems=92;totalsecret=93;wminfo.maxfrags=94;wminfo.partime=95;for(i=0;i<4;i++){players[i].killcount=96;players[i].itemcount=97;players[i].secretcount=98;}setup_statistics();leveltime=99;respawnmonsters=false;deathmatch_p=deathmatchstarts;netgame=deathmatch=false;nomonsters=input[c][V_NO_MONSTERS];gameskill=input[c][V_SKILL];P_InitThinkers();load_perception_world(input[c][V_MAP]);definitions();setup_leveltime();load_things_input(input[c][V_MAP]);}
static int components(void){int skill,flag,j;int options[]={1,2,4,7,16,23};int types[]={3002,5,2011,32000};for(skill=0;skill<5;skill++)for(flag=0;flag<6;flag++)for(j=0;j<4;j++){mapthing_t thing;int fatal=0;reset(0);gameskill=skill;admit_native=0;thing.x=-32;thing.y=0;thing.angle=-91;thing.type=types[j];thing.options=options[flag];catching_error=1;if(setjmp(component_failure))fatal=1;else P_SpawnMapThing(&thing);catching_error=0;printf("COMP,%d,%d,%d,0,0,%d,%d,%d,%d,%d,%ld,%d\n",skill,options[flag],types[j],fatal,owned_count,prndindex,totalkills,owned_count?owned[0]->reactiontime:-1,owned_count?(long)(owned[0]->state-states):-1L,owned_count?owned[0]->flags:0);}
for(j=0;j<3;j++){mapthing_t thing;reset(0);admit_native=0;netgame=j!=0;deathmatch=j==2;thing.x=0;thing.y=0;thing.angle=0;thing.type=j==2?5:3002;thing.options=23;P_SpawnMapThing(&thing);printf("COMP,2,23,%d,%d,0,0,%d,%d,%d,%d,%ld,%d\n",thing.type,j,owned_count,prndindex,totalkills,owned_count?owned[0]->reactiontime:-1,owned_count?(long)(owned[0]->state-states):-1L,owned_count?owned[0]->flags:0);}
return 0;}
static void credit_components(void){int network,kind;for(network=0;network<2;network++)for(kind=0;kind<5;kind++){mobj_t *target,*source=NULL;reset(0);netgame=network;admit_native=0;target=P_SpawnMobj(0,0,ONFLOORZ,MT_SERGEANT);if(kind==1||kind==2){source=P_SpawnMobj(128*FRACUNIT,0,ONFLOORZ,135);if(kind==2)source->player=&players[2];}if(kind==3)source=target;if(kind==4)source=P_SpawnMobj(128*FRACUNIT,0,ONFLOORZ,MT_SERGEANT);P_DamageMobj(target,NULL,source,150);printf("CREDIT,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n",network,kind,totalkills,players[0].killcount,players[2].killcount,target->flags,target->height,(int)(target->state-states),target->tics,prndindex,respawnmonsters);}}
static mobj_t* damage_source(int kind,int demons){if(!kind)return NULL;if(kind==1)return players[0].mo;if(kind==2)return owned[0];if(kind==3&&demons>1)return owned[1];abort();return NULL;}
int main(int argc,char** argv){int c,i,t,original;int seeds[CASE_COUNT];if(argc<2)return 2;if(!strcmp(argv[1],"setup")){reset(0);prndindex=211;rndindex=87;totalkills=99;totalitems=98;totalsecret=97;players[0].killcount=96;leveltime=95;setup_statistics();setup_leveltime();printf("SETUP,%d,%d,%d,%d,%d,%d,%d,%d",totalkills,totalitems,totalsecret,wminfo.maxfrags,wminfo.partime,leveltime,prndindex,rndindex);for(i=0;i<4;i++)printf(",%d,%d,%d",players[i].killcount,players[i].itemcount,players[i].secretcount);printf("\n");return 0;}if(!strcmp(argv[1],"components")){int result=components();credit_components();return result;}original=!strcmp(argv[1],"spawn");if(!original&&strcmp(argv[1],"trace"))return 2;if(original){FILE* f;if(argc!=3)return 3;f=fopen(argv[2],"r");if(!f)return 4;for(i=0;i<CASE_COUNT;i++)if(fscanf(f,"%d",&seeds[i])!=1)return 5;fclose(f);}
for(c=0;c<CASE_COUNT;c++){int* v=input[c];int demons;int seed=original?seeds[c]:(v[V_CONTINUE_RNG]?prndindex:0);reset(c);prndindex=seed;printf("SEED,%d,0,%d\n",c,seed);admit_native=!original;for(i=0;i<v[V_SKIP];i++){int draw=P_Random();observe(9,-1,draw);}P_LoadThings(0);demons=owned_count;
printf("F,%d,0,-1,0,0,0,0,0,%d,%d,%d,%d\n",scenario,demons,prndindex,totalkills,totalitems);
if(original){capture();continue;}
for(i=0;i<4;i++){players[i].mo=P_SpawnMobj(v[V_P0X+5*i],v[V_P0Y+5*i],v[V_P0Z+5*i],135);players[i].mo->health=v[V_BODY_HEALTH];playeringame[i]=v[V_P0ACTIVE+5*i];players[i].health=v[V_P0HEALTH+5*i];}
if(v[V_ENTER_ALL])for(i=0;i<demons;i++){owned[i]->target=players[0].mo;observe(6,i,ident(players[0].mo));P_SetMobjState(owned[i],v[V_ENTER_ALL]);}capture();
for(t=1;t<=v[V_TICS];t++){operation=t;query_index=-1;attempt=-1;
if(t==v[V_DAMAGE_TICK])P_DamageMobj(owned[0],damage_source(v[V_INFLICTOR_KIND],demons),damage_source(v[V_SOURCE_KIND],demons),v[V_DAMAGE]);
if(t==v[V_KILL_TICK]){if(v[V_HITSCAN])P_LineAttack(players[0].mo,ANG180,512*FRACUNIT,0,v[V_KILL]);else for(i=0;i<(v[V_KILL_ALL]?demons:1);i++)P_DamageMobj(owned[i],damage_source(v[V_INFLICTOR_KIND],demons),damage_source(v[V_SOURCE_KIND],demons),v[V_KILL]);}
if(t==v[V_REMOVE_CORPSE])P_RemoveMobj(owned[0]);
if(t==v[V_SOUND_TICK])P_NoiseAlert(players[0].mo,players[0].mo);
if(t==v[V_RETIRE_TICK]){playeringame[0]=0;players[0].mo=NULL;P_RemoveMobj(owned[demons]);}
if(v[V_RETIRE_TICK]>=0&&t==v[V_RETIRE_TICK]+2){players[0].mo=P_SpawnMobj(v[V_P0X],v[V_P0Y],v[V_P0Z],135);playeringame[0]=1;players[0].health=100;}
P_RunThinkers();leveltime++;capture();}}
return 0;}

"""

SPAWN_WRAPPERS=r"""
mobj_t* P_SpawnMobj(fixed_t x,fixed_t y,fixed_t z,mobjtype_t type){if(admit_native&&inside_map&&type==MT_SERGEANT)type=136;return Raw_SpawnMobj(x,y,z,type);}
void P_SpawnMapThing(mapthing_t* thing){int before=owned_count;inside_map=1;map_decision=0;Raw_SpawnMapThing(thing);inside_map=0;printf("F,%d,%d,%d,%d,%d,%d,%d,%u,%d,%d,%d,%d\n",scenario,operation,thing_index++,thing->x,thing->y,thing->angle,thing->type,(unsigned short)thing->options,before,owned_count,map_decision,prndindex);}
"""
def run(check):
 previous.MAPS=MAPS;combat.MAPS=MAPS
 shim=(ROOT/'tools/reference/m3d3j_shim.h').read_text().replace('RNG_TABLE',','.join(map(str,base.table('m_random.c',r'unsigned char\s+rndtable\[256\]'))))
 shim=shim.replace('query_index,stage,','query_index,stage,').replace('query_index,(long)(l-lines),stage','query_index,(long)(l-lines),stage').replace(',4,%ld,%d,%d,%d',',5,%ld,%d,%d,%d').replace(',7,%ld,%d',',8,%ld,%d')
 enemy=(SOURCE/'p_enemy.c').read_text()
 direction_tables=enemy[enemy.index('typedef enum'):enemy.index('void A_Fall')]
 direction_tables+='\n'+re.search(r'fixed_t\s+xspeed\[8\][^;]+;',enemy).group(0)+'\n'+re.search(r'fixed_t\s+yspeed\[8\][^;]+;',enemy).group(0)
 shim=shim.replace('/* PURSUIT_DIRECTION_TABLES */',direction_tables)
 setup_source=(SOURCE/'p_setup.c').read_text()
 setup_start=setup_source.index('    totalkills = totalitems = totalsecret = wminfo.maxfrags = 0;')
 setup_end=setup_source.index('\n    // Initial height',setup_start)
 setup_fragment=setup_source[setup_start:setup_end]
 time_fragment=re.search(r'^    leveltime = 0;.*$',setup_source,re.M).group(0)
 parts=['static void setup_statistics(void){int i;\n'+setup_fragment+'\n}', 'static void setup_leveltime(void){'+time_fragment+'}']
 for file,name,result in FUNCTIONS:
  rename={'P_CheckSight':'Raw_CheckSight','P_LookForPlayers':'Raw_LookForPlayers','P_Move':'Raw_Move','P_TryWalk':'Raw_TryWalk','P_NewChaseDir':'Raw_NewChaseDir','A_Chase':'Raw_Chase','P_CheckMeleeRange':'Raw_Melee','P_CheckMissileRange':'Raw_Missile','P_AimLineAttack':'Raw_Aim','P_LineAttack':'Raw_LineAttack','P_ZMovement':'Raw_ZMovement','P_SpawnMobj':'Raw_SpawnMobj','P_SpawnMapThing':'Raw_SpawnMapThing'}.get(name)
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
  if name=='P_SpawnMapThing':
   b=b.replace('mthing->x << FRACBITS','(fixed_t)mthing->x * FRACUNIT').replace('mthing->y << FRACBITS','(fixed_t)mthing->y * FRACUNIT')
   decisions=iter([11,1,16,2,32,64])
   b=b.replace('return;', 'RETURN_PLACEHOLDER')
   b=re.sub('RETURN_PLACEHOLDER',lambda m:'{map_decision='+str(next(decisions))+';return;}',b)
  if name=='P_SpawnMapThing':
   b=b.replace('totalkills++;','{totalkills++;printf("ST,%d,%d,0,%d,%d\\n",scenario,operation,ident(mobj),totalkills);}')
  if name=='P_KillMobj':
   b=b.replace('players[0].killcount++;','players[0].killcount++;printf("ST,%d,%d,1,%d,%d\\n",scenario,operation,ident(target),players[0].killcount);')
  b=instrument(name,b)
  parts.append(b)
 parts.insert(0,'int controlled_random=-1;\n'+base.body('m_random.c','P_Random','int','Raw_Random')+'int P_Random(void){int value=controlled_random<0?Raw_Random():controlled_random;observe(8,-1,value);return value;}\nstatic int jitter(void){int a=combat_random(3);int b=combat_random(4);return (a-b)*1024;}')
 for name in actors.ACTIONS[1:]:
  if name in ['A_Look','A_Chase']:continue
  if name in ['A_FaceTarget','A_PosAttack','A_SPosAttack','A_CPosAttack','A_CPosRefire','A_SargAttack']:
   parts.append(attack_action(name));continue
  if name in ['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']:
   b=base.body('p_enemy.c',name,'void');arg='mo' if name=='A_PlayerScream' else 'actor';b=re.sub(r'(\{)',r'\1\n observe(5,ident('+arg+'),'+str(actors.ACTIONS.index(name))+');',b,count=1)
  else:b=f'void {name}(mobj_t* actor){{(void)actor;abort();}}'
  parts.append(b)
 parts.append(SPAWN_WRAPPERS)
 fields=list(CASES[0])[1:]
 code=shim.replace('/* SOURCE_BODIES */','\n'.join(parts)+'\n'+WRAPPERS+ATTACK_WRAPPERS)
 code+='\n'+combat.c_maps()+'\n'+previous.post_loader()
 code+='\nenum {'+','.join('V_'+f.upper() for f in fields)+'};\n#define CASE_COUNT '+str(len(CASES))+'\nint input[]['+str(len(fields))+']={'+','.join('{'+','.join(str(c[f]) for f in fields)+'}' for c in CASES)+'};\n'
 code+='void load_things_input(int which){thing_index=0;switch(which){'
 for i,data in enumerate(THING_INPUTS):code+='case '+str(i)+':{static const unsigned char bytes[]={'+','.join(map(str,data or b'\0'))+'};thing_size='+str(len(data))+';memcpy(thing_buffer.bytes,bytes,thing_size);break;}'
 code+='default:abort();}}\n'+MAIN
 outputs={f'tests/reference/m3d3j_map_{i}.bin':m['bytes'] for i,m in enumerate(MAPS)}
 with tempfile.TemporaryDirectory(prefix='doom-m3d3j-reference-') as tmp:
  src=Path(tmp)/'reference.c';src.write_text(code);binary=Path(tmp)/'reference'
  subprocess.run(['cc',*FLAGS,'-I'+str(SOURCE),str(src),str(SOURCE/'m_fixed.c'),str(SOURCE/'tables.c'),'-o',str(binary)],check=True)
  for mode in ['trace','spawn','components','setup']:
   arguments=[str(binary),mode]
   if mode=='spawn':
    seed_rows=[r for r in csv.reader(io.StringIO(outputs['tests/reference/m3d3j_trace.csv'].decode())) if r[0]=='SEED']
    assert len(seed_rows)==len(CASES)
    for row in seed_rows:CASES[int(row[1])]['seed']=int(row[3])
    seed_path=Path(tmp)/'initial-seeds.txt';seed_path.write_text('\n'.join(str(c['seed']) for c in CASES));arguments.append(str(seed_path))
   result=subprocess.run(arguments,stdout=subprocess.PIPE,timeout=90)
   if result.returncode:
    Path('/tmp/m3d3j-failed-output').write_bytes(result.stdout);Path('/tmp/m3d3j-failed-reference.c').write_text(code)
    import shutil;shutil.copy(binary,'/tmp/m3d3j-failed-reference');raise RuntimeError('Original C aborted; diagnostics /tmp/m3d3j-failed-output')
   if mode=='setup':
    values=next(line[6:] for line in result.stdout.decode().splitlines() if line.startswith('SETUP,'))
    result.stdout=('totalkills,totalitems,totalsecret,maxfrags,partime,leveltime,gameplay_rng,misc_rng,'+','.join(f'p{i}_{field}' for i in range(4) for field in ['kills','items','secrets'])+'\n'+values+'\n').encode()
   if mode=='components':
    credit=[line[7:] for line in result.stdout.decode().splitlines() if line.startswith('CREDIT,')]
    assert len(credit)==10
    outputs['tests/reference/m3d3j_credit_components.csv']=('netgame,source_class,totalkills,player0_kills,player2_kills,flags,height,state,tics,rng,respawn\n'+'\n'.join(credit)+'\n').encode()
    result.stdout=('skill,options,type,mode,nomonsters,fatal,spawned,rng,totalkills,reaction,state,flags\n'+'\n'.join(line[5:] for line in result.stdout.decode().splitlines() if line.startswith('COMP,'))+'\n').encode()
   outputs[f'tests/reference/m3d3j_{mode}.csv']=result.stdout
 table=io.StringIO();w=csv.writer(table,lineterminator='\n');w.writerow(['case',*CASES[0]]);w.writerows([[i,*c.values()] for i,c in enumerate(CASES)]);outputs['tests/reference/m3d3j_cases.csv']=table.getvalue().encode()
 extracted=FUNCTIONS+[('p_enemy.c',n,'void') for n in ['A_FaceTarget','A_PosAttack','A_SPosAttack','A_CPosAttack','A_CPosRefire','A_SargAttack','A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']]+[('m_random.c','P_Random','int')]
 trace=list(csv.reader(io.StringIO(outputs['tests/reference/m3d3j_trace.csv'].decode())))
 raw=list(csv.reader(io.StringIO(outputs['tests/reference/m3d3j_spawn.csv'].decode())))
 tags={'E','J','G','X','H','V','K','D','B','U','I','T','M','C','O','Z','Y'}
 inventory={'scenarios':len(CASES),'compiled_original_bodies':len(extracted),'supplied_maps':len(MAPS),'exercised_maps':sorted({c['map'] for c in CASES}),'geometry_bases':sorted({g for g,t in GEOMETRIES}),'actor_fields':sum(26 for r in trace if r[0]=='A' and r[4]!='0'),'raw_gameplay_draws_including_setup':sum(r[0]=='E' and r[3]=='8' for r in trace),'ordered_instrumentation_rows':sum(r[0] in tags|{'ST'} for r in trace),'level_statistics_fields':4*sum(r[0]=='LS' for r in trace),'statistics_updates':sum(r[0]=='ST' for r in trace),'isolated_filter_components':123,'isolated_credit_components':10,'setup_fragments':2,'isolated_setup_components':1,'unadapted_spawn_actor_fields':sum(26 for r in raw if r[0]=='A'),'unadapted_spawn_rng_draws_including_discards':sum(r[0]=='E' and r[3]=='8' for r in raw)}
 files=sorted({'p_setup.c'}|{f for f,n,r in extracted}|{'info.c','m_fixed.c','tables.c'}|{p.name for p in SOURCE.glob('*.h')})
 manifest={'setup_fragments':{'statistics':{'sha256':hashlib.sha256(setup_fragment.encode()).hexdigest(),'source':setup_fragment},'leveltime':{'sha256':hashlib.sha256(time_fragment.encode()).hexdigest(),'source':time_fragment}},'revision':'a77dfb96cb91780ca334d0d4cfd86957558007e0','evidence_inventory':inventory,'functions':extracted,'function_sha256':{f+':'+n:hashlib.sha256(base.body(f,n,r).encode()).hexdigest() for f,n,r in extracted},'source_sha256':{f:hashlib.sha256((SOURCE/f).read_bytes()).hexdigest() for f in files},'flags':FLAGS,'adapter_sha256':{f:hashlib.sha256((ROOT/'tools/reference'/f).read_bytes()).hexdigest() for f in sorted([p.name for p in (ROOT/'tools/reference').glob('generate*.py')]+['m3d3j_shim.h'])},'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()},'assumptions':['real original structs/info/state tables; int/fixed/angle32, short16, pointer64; mapthing_t10 bytes, little-endian host SHORT','genuine P_LoadThings and P_SpawnMapThing use original commercial single-player filters/type lookup; unadapted spawn pass retains original type12/state475/COUNTKILL/totals','separate native gameplay pass translates resolved type12 to explicit native136 only at P_SpawnMobj boundary after source lookup/no_monsters; copies exact H fixture aliases and disables raise; original COUNTKILL/available totals/fallback kill credit are retained; player-pointer/network branches remain outside gameplay admission','source spawnpoint/timer/angle/ambush and two real RNG draws are preserved; no spawn action','negative-coordinate signed shifts replaced with representable multiplication; signed angle overflow defined by recorded wrap flags; inherited explicit left-before-right shadow/spread/jitter draw ordering','unsupported player/drop/special/resurrection services abort; no undefined negative player-index inputs or all-inactive C scans','P_SetupLevel statistics/leveltime statements extracted verbatim and inventoried separately, not the whole loader; dirty counters verify reset; level restart retains original RNG stream using trace-derived seeds in separate raw-spawn observations','source actor bytes retained until scenario reset solely to observe retired identity; no Rust output or proprietary assets read']}
 path=ROOT/'tests/reference/m3d3j_provenance.json'
 if check:
  old=json.loads(path.read_text())
  for k,v in manifest.items():assert old[k]==json.loads(json.dumps(v)),k
 else:
  manifest.update(compiler=subprocess.check_output(['cc','--version'],text=True).splitlines()[0],platform=platform.platform());outputs['tests/reference/m3d3j_provenance.json']=(json.dumps(manifest,indent=2)+'\n').encode()
 for p,b in outputs.items():
  if check:assert (ROOT/p).read_bytes()==b,p
  else:(ROOT/p).write_bytes(b)
 print(('Checked' if check else 'Generated')+f' M3d3j {len(CASES)} scenarios, {len(MAPS)} exercised synthetic maps: {inventory}')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
