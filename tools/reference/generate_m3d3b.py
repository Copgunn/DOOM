#!/usr/bin/env python3
"""Genuine original pursuit bodies and complete synthetic thinker traces; no Rust output."""
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
def roster(x=96,y=0,active=1,health=100):return [(x,y,0,active,health),(-200,-96,0,0,100),(-200,-96,0,0,100),(-200,-96,0,0,100)]
def case(label,mode=2,map=0,ox=-64,oy=0,oz=0,players=None,direction=8,count=0,reaction=8,threshold=100,target=1,speed=8,radius=16,active_sound=0,see=1,stand=1,skill=2,enter=0,mx=0,my=0,mz=0,target_health=100,target_flags=6,flip_tick=2,flip_flags=-1,flip_actorhealth=-999,flip_target=-2,other=0,**kw):
 c=sight.case(label,otype=136,mode=mode,map=map,ox=ox,oy=oy,oz=oz,players=players or roster(),sound_id=0,stand=stand,threshold=threshold,**kw)
 c.update(direction=direction,count=count,reaction=reaction,target=target,speed=speed,radius=radius*U,active_sound=active_sound,see=see,skill=skill,enter=enter,mx=mx*U,my=my*U,mz=mz*U,target_health=target_health,target_flags=target_flags,flip_tick=flip_tick,flip_flags=flip_flags,flip_actorhealth=flip_actorhealth,flip_target=flip_target,other=other)
 return c
CASES=[case('spawn_defaults',mode=6,direction=0,threshold=0),case('spawn_defaults_nightmare',mode=6,skill=4,threshold=0)]
for mode in [0,1]:
 for d in range(9):CASES.append(case(('move' if mode==0 else 'walk')+'_'+str(d),mode=mode,ox=0,direction=d,count=7))
for x,y,name in [(96,0,'east'),(-96,0,'west'),(0,48,'north'),(0,-48,'south'),(96,48,'ne'),(-96,48,'nw'),(-96,-48,'sw'),(96,-48,'se')]:CASES.append(case('direction_'+name,ox=0,players=roster(x,y)))
for x,y in [(10,0),(-10,0),(0,10),(0,-10),(11,0),(-11,0),(0,11),(0,-11)]:CASES.append(case(f'threshold_{x}_{y}',ox=0,players=roster(x,y),target_flags=0))
CASES += [case('blocked_direct',map=9,ox=-16,players=roster(96,32)),case('diagonal_blocked_axis',map=11,ox=16,oy=0,players=roster(-64,32)),case('old_direction_retry',ox=0,direction=6,players=roster(5,5),target_flags=0),case('turnaround_avoided',direction=0,players=roster(-96,32)),case('turnaround_last_resort',map=12,ox=110,direction=0,players=roster(-96)),case('fully_trapped',map=14,ox=0,players=roster(96),direction=0),case('fully_trapped_reverse_search',map=14,ox=0,players=roster(96),direction=8,skip=4),case('priority_rng_swap',ox=0,players=roster(96,32),skip=2),case('rng_wrap_decisions',map=14,ox=0,players=roster(96),direction=0,tics=180)]
for m,name in [(1,'portal'),(2,'step24'),(3,'step25'),(4,'drop24'),(5,'drop25'),(6,'opening56'),(7,'opening55'),(8,'closed'),(9,'blocking'),(10,'blockmonsters')]:CASES.append(case('move_'+name,mode=0,map=m,ox=-16,direction=0))
CASES += [case('move_drop_allowed',mode=0,map=5,ox=-16,direction=0,flags=1030,tics=4),case('move_actor_blocks',mode=0,ox=-40,direction=0,players=roster(-8)),case('move_actor_z_independent',mode=0,ox=-40,direction=0,players=[(-8,0,64,1,100),*roster()[1:]]),case('move_shootable_nonsolid',mode=0,ox=-40,direction=0,players=roster(-8),target_flags=4),case('move_infloat_clear',mode=0,direction=0,flags=6|2097152),case('move_narrow_corridor',mode=0,map=12,direction=0,tics=8),case('move_narrow_blocked',mode=0,map=13,direction=0),case('move_omitted_wall',mode=0,map=15,ox=112,direction=0,tics=4),case('move_three_sectors',mode=0,map=16,ox=-32,direction=0,tics=16,players=roster(200),target_flags=0)]
for d in range(9):CASES.append(case('chase_turn_'+str(d),mode=3,ox=0,players=roster(96,32),direction=d,count=3,angle=0xf1234567))
CASES += [case('chase_count_zero',mode=3,direction=0,count=0),case('chase_count_negative',mode=3,direction=0,count=-1),case('chase_count_one',mode=3,direction=0,count=1),case('chase_missing_reacquires',mode=3,target=-1),case('chase_nonshootable_reacquires',mode=3,target_flags=2,players=[(96,0,0,1,0),(-32,32,0,1,100),*roster()[2:]]),case('chase_dead_shootable_retained',mode=3,target_health=0),case('chase_failed_reacquisition',mode=3,target=-1,players=roster(96,active=1,health=0)),case('chase_justattacked',mode=3,flags=6|128),case('chase_justattacked_nightmare',mode=3,flags=6|128,skill=4),case('chase_active_sound',mode=3,active_sound=1,tics=300,players=roster(200),target_flags=4),case('thinker_acquire_and_pursue',mode=4,target=-1,tics=20,players=roster(96,32)),case('thinker_wall_alternates',mode=4,map=9,target=-1,tics=40,players=roster(96,32)),case('thinker_target_loss_spawn',mode=4,target=-1,tics=12,players=roster(96),flip_slot=0,flip_health=0,flip_flags=2,flip_tick=6),case('thinker_reacquisition',mode=4,target=-1,tics=15,players=[(96,0,0,1,100),(-32,32,0,1,100),*roster()[2:]],flip_slot=0,flip_health=0,flip_flags=2,flip_tick=6),case('thinker_health_only',mode=4,target=-1,tics=12,flip_actorhealth=0,flip_tick=6),case('thinker_momentum_before_action',mode=4,target=-1,tics=12,mx=4,my=1,mz=2,oz=16),case('thinker_append_order',mode=4,target=-1,tics=16,other=1,players=roster(96,32))]
CASES += [case('move_headroom',mode=0,map=6,ox=-16,oz=16,direction=0),case('move_speed_one',mode=0,ox=0,direction=1,speed=1),case('move_speed_sixteen',mode=0,ox=0,direction=7,speed=16),case('move_zero_radius',mode=0,ox=0,direction=0,radius=0),case('move_radius32',mode=0,ox=0,direction=2,radius=32),case('thinker_state_countdown',mode=4,target=-1,stand=3,see=3,tics=18,players=roster(96,32))]
FUNCTIONS=sight.FUNCTIONS+[('p_enemy.c',n,r) for n,r in [('P_Move','boolean'),('P_TryWalk','boolean'),('P_NewChaseDir','void'),('A_Chase','void')]]

MAIN=r"""
static void capture(void){int i;for(i=0;i<owned_count;i++){mobj_t* p=owned[i];printf("A,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%u,%d,%d,%d,%d\n",scenario,operation,i,alive[i],p->type,p->x,p->y,p->z,p->momx,p->momy,p->momz,p->health,p->state?(long)(p->state-states):-1L,p->tics,p->flags,p->height,p->radius,p->reactiontime,p->threshold,p->target?ident(p->target):-1,p->lastlook,p->floorz,p->ceilingz,(int)(p->subsector-subsectors),prndindex,p->angle,p->sprite,p->frame,p->movedir,p->movecount);}
for(i=0;i<4;i++)printf("R,%d,%d,%d,%d,%d,%d\n",scenario,operation,i,playeringame[i],players[i].health,players[i].mo?ident(players[i].mo):-1);for(i=0;i<numsectors;i++)printf("S,%d,%d,%d,%d\n",scenario,operation,i,sectors[i].soundtarget?ident(sectors[i].soundtarget):-1);}
int main(int argc,char** argv){int c,i,t;int ids[3]={19,21,136};mobj_t* observer;
if(argc!=2||strcmp(argv[1],"trace"))return 2;
for(c=0;c<CASE_COUNT;c++){int* v=input[c];for(i=0;i<owned_count;i++)free(owned[i]);owned_count=0;memset(alive,0,sizeof(alive));memset(players,0,sizeof(players));memset(playeringame,0,sizeof(playeringame));scenario=c;operation=0;prndindex=0;query_index=-1;attempt=-1;gameskill=v[V_SKILL];P_InitThinkers();load_perception_world(v[V_MAP]);
memset(&states[1],0,sizeof(state_t));memset(&states[2],0,sizeof(state_t));states[1].sprite=SPR_POSS;states[1].tics=v[V_STAND];states[1].action.acp1=(actionf_p1)A_Look;states[1].nextstate=1;states[2].sprite=SPR_POSS;states[2].frame=1;states[2].tics=v[V_SEE];states[2].nextstate=2;states[2].action.acp1=(actionf_p1)A_Chase;
for(i=0;i<3;i++){mobjinfo[ids[i]]=mobjinfo[MT_PLAYER];mobjinfo[ids[i]].doomednum=-1;mobjinfo[ids[i]].spawnstate=1;mobjinfo[ids[i]].seestate=2;mobjinfo[ids[i]].seesound=v[V_SOUND_ID];mobjinfo[ids[i]].reactiontime=8;mobjinfo[ids[i]].flags=v[V_FLAGS];mobjinfo[ids[i]].height=v[V_OBSHEIGHT];mobjinfo[ids[i]].speed=v[V_SPEED];mobjinfo[ids[i]].radius=v[V_RADIUS];mobjinfo[ids[i]].activesound=v[V_ACTIVE_SOUND];mobjinfo[ids[i]].meleestate=0;mobjinfo[ids[i]].missilestate=0;}
for(i=0;i<v[V_SKIP];i++){int draw=P_Random();observe(9,-1,draw);}
observer=P_SpawnMobj(v[V_OX],v[V_OY],v[V_OZ],v[V_OTYPE]);
for(i=0;i<4;i++){players[i].mo=P_SpawnMobj(v[V_P0X+5*i],v[V_P0Y+5*i],v[V_P0Z+5*i],MT_PLAYER);playeringame[i]=v[V_P0ACTIVE+5*i];players[i].health=v[V_P0HEALTH+5*i];}
if(v[V_MODE]!=6){observer->angle=(angle_t)v[V_ANGLE];observer->threshold=v[V_THRESHOLD];observer->reactiontime=v[V_REACTION];observer->movedir=v[V_DIRECTION];observer->movecount=v[V_COUNT];observer->momx=v[V_MX];observer->momy=v[V_MY];observer->momz=v[V_MZ];players[0].mo->health=v[V_TARGET_HEALTH];players[0].mo->flags=v[V_TARGET_FLAGS];if(v[V_TARGET]>=0){observer->target=owned[v[V_TARGET]];observe(6,ident(observer),v[V_TARGET]);}}
if(v[V_OTHER])P_SpawnMobj(-64*FRACUNIT,32*FRACUNIT,0,136);
if(v[V_ENTER])P_SetMobjState(observer,v[V_ENTER]);capture();
for(t=1;t<=v[V_TICS];t++){operation=t;query_index=-1;attempt=-1;
if(t==v[V_FLIP_TICK]){if(v[V_FLIP_SLOT]>=0)players[v[V_FLIP_SLOT]].health=v[V_FLIP_HEALTH];if(v[V_FLIP_FLAGS]>=0)players[0].mo->flags=v[V_FLIP_FLAGS];if(v[V_FLIP_ACTORHEALTH]!=-999)players[0].mo->health=v[V_FLIP_ACTORHEALTH];if(v[V_FLIP_TARGET]>=-1){observer->target=v[V_FLIP_TARGET]<0?NULL:owned[v[V_FLIP_TARGET]];observe(6,0,v[V_FLIP_TARGET]);}}
if(t==1&&v[V_SOUND]>=0)P_NoiseAlert(owned[v[V_SOUND]],owned[v[V_EMITTER]]);
if(v[V_MODE]==0)P_Move(observer);else if(v[V_MODE]==1)P_TryWalk(observer);else if(v[V_MODE]==2)P_NewChaseDir(observer);else if(v[V_MODE]==3)A_Chase(observer);else if(v[V_MODE]==4)P_RunThinkers();else if(v[V_MODE]!=6)abort();capture();}
}for(i=0;i<owned_count;i++)free(owned[i]);return 0;}
"""
WRAPPERS=r'''
boolean P_CheckSight(mobj_t* a,mobj_t* b){int pair=(int)(a->subsector->sector-sectors)*numsectors+(int)(b->subsector->sector-sectors);int reject=(rejectmatrix[pair>>3]&(1<<(pair&7)))!=0;boolean visible;query_index++;printf("V,%d,%d,%d,0,%d,%d,%d,%d\n",scenario,operation,query_index,ident(a),ident(b),pair,reject);visible=Raw_CheckSight(a,b);printf("Q,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,query_index,visible,reject?0:sightzstart,reject?0:bottomslope,reject?0:topslope);return visible;}
boolean P_LookForPlayers(mobj_t* a,boolean allaround){boolean result=Raw_LookForPlayers(a,allaround);printf("K,%d,%d,7,%d,%d,%d\n",scenario,operation,ident(a),result,a->lastlook);return result;}
'''

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
 combat.MAPS=MAPS
 shim=(ROOT/'tools/reference/m3d3b_shim.h').read_text().replace('RNG_TABLE',','.join(map(str,base.table('m_random.c',r'unsigned char\s+rndtable\[256\]'))))
 shim=shim.replace('query_index,stage,','query_index,stage,').replace('query_index,(long)(l-lines),stage','query_index,(long)(l-lines),stage').replace(',4,%ld,%d,%d,%d',',5,%ld,%d,%d,%d').replace(',7,%ld,%d',',8,%ld,%d')
 enemy=(SOURCE/'p_enemy.c').read_text()
 direction_tables=enemy[enemy.index('typedef enum'):enemy.index('void A_Fall')]
 direction_tables+='\n'+re.search(r'fixed_t\s+xspeed\[8\][^;]+;',enemy).group(0)+'\n'+re.search(r'fixed_t\s+yspeed\[8\][^;]+;',enemy).group(0)
 shim=shim.replace('/* PURSUIT_DIRECTION_TABLES */',direction_tables)
 parts=[]
 for file,name,result in FUNCTIONS:
  rename={'P_CheckSight':'Raw_CheckSight','P_LookForPlayers':'Raw_LookForPlayers','P_Move':'Raw_Move','P_TryWalk':'Raw_TryWalk','P_NewChaseDir':'Raw_NewChaseDir','A_Chase':'Raw_Chase','P_ZMovement':'Raw_ZMovement'}.get(name)
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
  parts.append(b)
 parts.insert(0,base.body('m_random.c','P_Random','int','Raw_Random')+'int P_Random(void){int value=Raw_Random();observe(8,-1,value);return value;}\nstatic int jitter(void){int a=P_Random();int b=P_Random();return (a-b)*1024;}')
 for name in actors.ACTIONS[1:]:
  if name in ['A_Look','A_Chase']:continue
  if name in ['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']:
   b=base.body('p_enemy.c',name,'void');arg='mo' if name=='A_PlayerScream' else 'actor';b=re.sub(r'(\{)',r'\1\n observe(5,ident('+arg+'),'+str(actors.ACTIONS.index(name))+');',b,count=1)
  else:b=f'void {name}(mobj_t* actor){{(void)actor;abort();}}'
  parts.append(b)
 def c_int(v):return str(v if v<2147483648 else v-4294967296)
 fields=list(CASES[0])[1:]
 text=shim.replace('/* SOURCE_BODIES */','\n'.join(parts)+'\n'+WRAPPERS)+'\n'+combat.c_maps()+'\n'+post_loader()+'\nenum {'+','.join('V_'+f.upper() for f in fields)+'};\n#define CASE_COUNT '+str(len(CASES))+'\nint input[]['+str(len(fields))+']={'+','.join('{'+','.join(c_int(c[f]) for f in fields)+'}' for c in CASES)+'};\n'+MAIN
 outputs={f'tests/reference/m3d3b_map_{i}.bin':m['bytes'] for i,m in enumerate(MAPS)}
 with tempfile.TemporaryDirectory(prefix='doom-m3d3b-reference-') as tmp:
  src=Path(tmp)/'reference.c';src.write_text(text);binary=Path(tmp)/'reference';subprocess.run(['cc',*FLAGS,'-I'+str(SOURCE),str(src),str(SOURCE/'m_fixed.c'),str(SOURCE/'tables.c'),'-o',str(binary)],check=True)
  for mode in ['trace']:
   result=subprocess.run([str(binary),mode],stdout=subprocess.PIPE,timeout=30)
   if result.returncode:
    Path('/tmp/m3d3b-failed-output').write_bytes(result.stdout);Path('/tmp/m3d3b-failed-reference.c').write_text(text)
    import shutil;shutil.copy(binary,'/tmp/m3d3b-failed-reference');raise RuntimeError('Original C aborted; bounded diagnostics in /tmp/m3d3b-failed-output')
   outputs[f'tests/reference/m3d3b_{mode}.csv']=result.stdout
 table=io.StringIO();w=csv.writer(table,lineterminator='\n');w.writerow(['case',*CASES[0]]);w.writerows([[i,*c.values()] for i,c in enumerate(CASES)]);outputs['tests/reference/m3d3b_cases.csv']=table.getvalue().encode()
 files=sorted({f for f,_,_ in FUNCTIONS}|{'info.c','m_random.c','m_fixed.c','tables.c','p_enemy.c'}|{p.name for p in SOURCE.glob('*.h')})
 extracted=FUNCTIONS+[('m_random.c','P_Random','int')]+[('p_enemy.c',n,'void') for n in ['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']]
 manifest={'revision':'a77dfb96cb91780ca334d0d4cfd86957558007e0','functions':extracted,'function_sha256':{f+':'+n:hashlib.sha256(base.body(f,n,r).encode()).hexdigest() for f,n,r in extracted},'direction_tables_sha256':hashlib.sha256(direction_tables.encode()).hexdigest(),'flags':FLAGS,'source_sha256':{f:hashlib.sha256((SOURCE/f).read_bytes()).hexdigest() for f in files},'adapter_sha256':{f:hashlib.sha256((ROOT/'tools/reference'/f).read_bytes()).hexdigest() for f in ['generate.py','generate_m3b.py','generate_m3c.py','generate_m3d1.py','generate_m3d3a.py','generate_m3d3b.py','m3d3b_shim.h']},'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()},'assumptions':['real original structs/info and 32-bit fixed/int, 64-bit pointer/function-union ABI','exact sight horizontal x==node.y quirk, shifts/division and BSP order retained','native states1/2 and types19/21/136 isolate genuine Look/Chase with absent melee/missile states; no attack/float/door/player damage','empty fallback roster, stale identities and unsafe extremes excluded from original-C execution and tested as declared Rust safety policies','source retired memory retained solely for observation; no Rust expected values or proprietary data read','unsigned BAM movedir shift adapter defines original signed-shift intent, preserving 32-bit two-complement delta; countdowns avoid INT_MIN and abs overflow']}
 path=ROOT/'tests/reference/m3d3b_provenance.json'
 if check:
  old=json.loads(path.read_text())
  for k,v in manifest.items():assert old[k]==json.loads(json.dumps(v)),k
 else:
  manifest.update(compiler=subprocess.check_output(['cc','--version'],text=True).splitlines()[0],platform=platform.platform());outputs['tests/reference/m3d3b_provenance.json']=(json.dumps(manifest,indent=2)+'\n').encode()
 for p,b in outputs.items():
  if check:assert (ROOT/p).read_bytes()==b,p
  else:(ROOT/p).write_bytes(b)
 print(('Checked' if check else 'Generated')+f' M3d3b {len(CASES)} original-body pursuit scenarios.')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
