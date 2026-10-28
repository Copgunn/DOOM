#!/usr/bin/env python3
"""Original sight/scan/A_Look bodies; synthetic inputs only, never Rust expected data."""
import argparse,csv,hashlib,io,json,platform,re,struct,subprocess,tempfile
from pathlib import Path
import generate as base
import generate_m3b as maps
import generate_m3c as actors
import generate_m3d1 as combat
ROOT,SOURCE,FLAGS=base.ROOT,base.SOURCE,base.FLAGS
U=65536
POSIT=actors.SOUND_NAMES.index('sfx_posit1');BGSIT=actors.SOUND_NAMES.index('sfx_bgsit1')
LOOK=actors.ACTIONS.index('A_Look')
def replace_lumps(m,updates):
 raw=m['bytes'];count,offset=struct.unpack_from('<ii',raw,4);out=[]
 for i in range(count):
  start,size,name=struct.unpack_from('<ii8s',raw,offset+i*16);name=name.rstrip(b'\0').decode();out.append((name,updates.get(name,raw[start:start+size])))
 m['bytes']=maps.wad(out);return m
def lumps(m):
 raw=m['bytes'];count,off=struct.unpack_from('<ii',raw,4);out={}
 for i in range(count):
  start,size,name=struct.unpack_from('<ii8s',raw,off+i*16);out[name.rstrip(b'\0').decode()]=raw[start:start+size]
 return out
MAPS=[maps.fixture(h) for h in [[(0,128)],[(0,128)]*2,[(0,64),(56,128)],[(0,128),(128,256)],[(0,128),(80,208)],[(0,128),(0,64)],[(0,128),(24,112),(0,128)]]]
MAPS += [maps.fixture([(0,128),(16,128)],diagonal=True)]
MAPS += [replace_lumps(maps.fixture([(0,128)]*2),{'REJECT':bytes([2])}),replace_lumps(maps.fixture([(0,128)]),{'REJECT':bytes([1])})]
MAPS += [maps.fixture([(0,128)]*3,portal_flags=f) for f in [4,68,68]]
m=MAPS[12];portal=[i for i,l in enumerate(m['lines']) if l['back'] is not None][-1];m['lines'][portal]['flags']=4;linebytes=bytearray(lumps(m)['LINEDEFS']);struct.pack_into('<H',linebytes,portal*14+4,4);replace_lumps(m,{'LINEDEFS':bytes(linebytes)})
MAPS += [maps.fixture([(-128,0),(-64,64)])]
m=maps.fixture([(0,128),(16,128)]);m['vertices']=[(-y,x) for x,y in m['vertices']]
for n in m['nodes']:n.update(x=-n['y'],y=n['x'],dx=-n['dy'],dy=n['dx'])
raw=bytearray(lumps(m)['NODES'])
for i,n in enumerate(m['nodes']):
 struct.pack_into('<hhhh',raw,i*28,n['x'],n['y'],n['dx'],n['dy'])
 for j in [8,16]:
  top,bottom,left,right=struct.unpack_from('<hhhh',raw,i*28+j);struct.pack_into('<hhhh',raw,i*28+j,right,left,-top,-bottom)
replace_lumps(m,{'VERTEXES':b''.join(maps.words(v) for v in m['vertices']),'NODES':bytes(raw)});MAPS.append(m)
MAPS += [maps.fixture([(0,128),(0,128)],omit=True),replace_lumps(maps.fixture([(0,128)]*3),{'REJECT':bytes([0,1])})]

def case(label,mode=0,map=0,ox=-64,oy=0,oz=0,angle=0,otype=None,flags=6,sound_id=POSIT,skip=0,allaround=0,sound=-1,emitter=1,tics=1,obsheight=56,stand=3,threshold=100,flip_slot=-1,flip_health=100,players=None):
 if players is None:players=[(64,0,0,1,100),(-32,16,0,0,100),(-32,-16,0,0,100),(-96,32,0,0,100)]
 c=dict(label=label,mode=mode,map=map,ox=ox*U,oy=oy*U,oz=oz*U,angle=angle,otype=(0 if mode<2 else 136) if otype is None else otype,flags=flags,sound_id=sound_id,skip=skip,allaround=allaround,sound=sound,emitter=emitter,tics=tics,obsheight=obsheight*U,stand=stand,threshold=0 if mode==0 else threshold,flip_slot=flip_slot,flip_health=flip_health)
 for i,p in enumerate(players):
  for key,value in zip(['x','y','z','active','health'],p):c[f'p{i}{key}']=value*U if key in ['x','y','z'] else value
 return c
def pawn(x,y=0,z=0,active=1,health=100):return (x,y,z,active,health)
def roster(p0,*others):return [p0,*others,*[pawn(-32,active=0)]*4][:4]
CASES=[case('sight_clear'),case('sight_wall',players=roster(pawn(160))),case('sight_equal_portal',map=1),case('sight_partial_window',map=2,players=roster(pawn(64,z=56))),case('sight_window_blocks_low_target',map=2),case('sight_closed',map=3,players=roster(pawn(64,z=128))),case('sight_floor_clip',map=4,players=roster(pawn(64,z=80))),case('sight_ceiling_clip',map=5),case('sight_multiple_nodes',map=6,players=roster(pawn(96))),case('sight_diagonal',map=7,players=roster(pawn(96,32,z=16))),case('sight_reject',map=8),case('sight_same_sector_reject',map=9),case('sight_negative',ox=-96,oy=-32,players=roster(pawn(-16,32))),case('sight_partition_start',map=1,ox=0),case('sight_partition_target',map=1,players=roster(pawn(0))),case('sight_partition_both',map=1,ox=0,oy=-32,players=roster(pawn(0,32))),case('sight_horizontal_bsp',map=14,ox=0,oy=-64,players=roster(pawn(0,64,z=16))),case('sight_without_blockmap_wall',map=15,players=roster(pawn(160))),case('sight_reject_second_byte',map=16,ox=96,players=roster(pawn(100))),case('sight_negative_floor',map=13,oz=-128,players=roster(pawn(64,z=-64))),case('sight_identical_position',players=roster(pawn(-64))),
case('scan_front',mode=1),case('scan_rear_far',mode=1,ox=32,players=roster(pawn(-64))),case('scan_rear_close',mode=1,ox=0,players=roster(pawn(-64))),case('scan_rear_outside_close',mode=1,ox=0,players=roster(pawn(-65))),case('scan_allaround',mode=1,ox=32,allaround=1,players=roster(pawn(-64))),case('scan_dead_record',mode=1,players=roster(pawn(64,health=0))),case('scan_inactive_first',mode=1,players=roster(pawn(64,active=0),pawn(64,16))),case('scan_candidate_limit',mode=1,players=roster(pawn(160),pawn(160),pawn(64))),case('scan_dead_then_visible',mode=1,players=roster(pawn(64,health=0),pawn(64,16))),case('scan_spawn_lastlook_one',mode=1,skip=1,players=roster(pawn(64),pawn(64,16))),case('scan_spawn_lastlook_two',mode=1,skip=3,players=roster(pawn(64),pawn(64,16),pawn(64,-16))),case('scan_spawn_lastlook_three',mode=1,skip=6,players=roster(pawn(64),pawn(64,16),pawn(64,-16),pawn(32,-32))),case('scan_health_changes',mode=1,tics=3,players=roster(pawn(160),pawn(-32,health=0)),flip_slot=1),case('scan_face_west',mode=1,angle=0x80000000,ox=32,players=roster(pawn(-64))),case('scan_90_boundary',mode=1,angle=0xffffffff,ox=0,players=roster(pawn(0,32))),case('scan_270_boundary',mode=1,ox=0,players=roster(pawn(0,-32))),case('look_visual',mode=2),case('look_sound_through_wall',mode=2,sound=1,players=roster(pawn(160))),case('look_ambush_blocked',mode=2,flags=38,sound=1,players=roster(pawn(160))),case('look_ambush_visible',mode=2,flags=38,sound=1),case('look_ambush_fallback',mode=2,flags=38,sound=1,players=roster(pawn(160),pawn(-32))),case('look_nonshootable_sound',mode=2,flags=2,sound=0),case('look_sound_unregistered',mode=2,sound=1,players=roster(pawn(64,active=0))),case('look_sound_dead_record',mode=2,sound=1,players=roster(pawn(64,health=0))),case('look_bgsit_rng',mode=2,sound_id=BGSIT),case('look_no_sound_rng',mode=2,sound_id=0),case('look_fixed_sound',mode=2,sound_id=1),case('look_spider_global',mode=2,otype=19,sound_id=actors.SOUND_NAMES.index('sfx_spisit')),case('look_cyborg_global',mode=2,otype=21,sound_id=actors.SOUND_NAMES.index('sfx_cybsit')),case('look_rng_wrap',mode=2,tics=260),case('thinker_wait_then_wake',mode=3,tics=8),case('thinker_outside_then_visible',mode=3,tics=8,stand=1,players=roster(pawn(160),pawn(-32,health=0)),flip_slot=1),case('thinker_ambush_stays_asleep',mode=3,tics=10,stand=1,flags=38,sound=1,players=roster(pawn(160))),case('noise_three_sectors',mode=2,map=10,sound=1,players=roster(pawn(96))),case('noise_two_soundblocks',mode=2,map=11,sound=1,players=roster(pawn(96))),case('noise_one_soundblock',mode=2,map=12,sound=1,players=roster(pawn(96))),case('noise_closed',mode=2,map=3,sound=1,players=roster(pawn(64,z=128))),case('noise_distinct_emitter',mode=2,map=1,sound=1,emitter=2,players=roster(pawn(64),pawn(-32,active=0)))]
MAPS.append(maps.fixture([(0,128)]*2,portal_flags=5))
CASES.extend([case('sight_ceiling_partial',map=5,players=roster(pawn(64,z=64))),case('sight_ceiling_slope_closed',map=5,players=roster(pawn(64,z=96))),case('sight_blocking_flag_ignored',map=17),case('look_posit3_group_base',mode=2,sound_id=POSIT+2),case('look_bgsit2_group_base',mode=2,sound_id=BGSIT+1),case('scan_just_rear_of_90',mode=1,angle=0xfffffffe,ox=0,players=roster(pawn(0,32))),case('scan_just_rear_of_270',mode=1,angle=1,ox=0,players=roster(pawn(0,-32))),case('scan_approximate_close_equal',mode=1,ox=0,players=roster(pawn(-48,32))),case('scan_approximate_close_outside',mode=1,ox=0,players=roster(pawn(-48,33)))])
FUNCTIONS=combat.FUNCTIONS+ [('p_sight.c',n,r) for n,r in [('P_DivlineSide','int'),('P_InterceptVector2','fixed_t'),('P_CrossSubsector','boolean'),('P_CrossBSPNode','boolean'),('P_CheckSight','boolean')]]+[('p_enemy.c','P_LookForPlayers','boolean'),('p_enemy.c','A_Look','void'),('p_enemy.c','P_RecursiveSound','void'),('p_enemy.c','P_NoiseAlert','void')]
MAIN=r'''
static void capture(void){int i;for(i=0;i<owned_count;i++){mobj_t* p=owned[i];printf("A,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%u,%d,%d\n",scenario,operation,i,alive[i],p->type,p->x,p->y,p->z,p->momx,p->momy,p->momz,p->health,p->state?(long)(p->state-states):-1L,p->tics,p->flags,p->height,p->radius,p->reactiontime,p->threshold,p->target?ident(p->target):-1,p->lastlook,p->floorz,p->ceilingz,(int)(p->subsector-subsectors),prndindex,p->angle,p->sprite,p->frame);}
for(i=0;i<4;i++)printf("R,%d,%d,%d,%d,%d,%d\n",scenario,operation,i,playeringame[i],players[i].health,players[i].mo?ident(players[i].mo):-1);for(i=0;i<numsectors;i++)printf("S,%d,%d,%d,%d\n",scenario,operation,i,sectors[i].soundtarget?ident(sectors[i].soundtarget):-1);}
static void side_vectors(void){int d[][2]={{0,128},{0,-128},{128,0},{-128,0},{128,128},{128,-128},{-128,128},{-128,-128}},orig[][2]={{0,0},{8,16},{-16,8}};fixed_t p[]={-128*FRACUNIT,-FRACUNIT,-1,0,1,8*FRACUNIT,64*FRACUNIT,128*FRACUNIT};int i,j,x,y;divline_t n;puts("x0,y0,dx,dy,x,y,side");for(i=0;i<8;i++)for(j=0;j<3;j++){n.x=orig[j][0]*FRACUNIT;n.y=orig[j][1]*FRACUNIT;n.dx=d[i][0]*FRACUNIT;n.dy=d[i][1]*FRACUNIT;for(x=0;x<8;x++)for(y=0;y<8;y++)printf("%d,%d,%d,%d,%d,%d,%d\n",n.x,n.y,n.dx,n.dy,p[x],p[y],P_DivlineSide(p[x],p[y],&n));}}
static void fractions(void){int i;divline_t a,b;puts("x2,y2,dx2,dy2,x1,y1,dx1,dy1,fraction");for(i=0;i<FRACTION_COUNT;i++){a.x=fractions_input[i][0];a.y=fractions_input[i][1];a.dx=fractions_input[i][2];a.dy=fractions_input[i][3];b.x=fractions_input[i][4];b.y=fractions_input[i][5];b.dx=fractions_input[i][6];b.dy=fractions_input[i][7];printf("%d,%d,%d,%d,%d,%d,%d,%d,%d\n",a.x,a.y,a.dx,a.dy,b.x,b.y,b.dx,b.dy,P_InterceptVector2(&a,&b));}}
int main(int argc,char** argv){int c,i,t;state_t original_states[2];mobjinfo_t original_types[3];int ids[3]={19,21,136};mobj_t* observer;
if(argc!=2)return 2;if(!strcmp(argv[1],"sides")){side_vectors();return 0;}if(!strcmp(argv[1],"fractions")){fractions();return 0;}if(strcmp(argv[1],"trace"))return 2;
original_states[0]=states[1];original_states[1]=states[2];for(i=0;i<3;i++)original_types[i]=mobjinfo[ids[i]];
for(c=0;c<CASE_COUNT;c++){int* v=input[c];for(i=0;i<owned_count;i++)free(owned[i]);owned_count=0;memset(alive,0,sizeof(alive));memset(players,0,sizeof(players));memset(playeringame,0,sizeof(playeringame));scenario=c;operation=0;prndindex=0;query_index=-1;P_InitThinkers();load_perception_world(v[V_MAP]);
states[1]=original_states[0];states[2]=original_states[1];for(i=0;i<3;i++)mobjinfo[ids[i]]=original_types[i];
if(v[V_MODE]>=2){memset(&states[1],0,sizeof(state_t));memset(&states[2],0,sizeof(state_t));states[1].sprite=SPR_POSS;states[1].tics=v[V_STAND];states[1].action.acp1=(actionf_p1)A_Look;states[1].nextstate=1;states[2].sprite=SPR_POSS;states[2].frame=1;states[2].tics=-1;
for(i=0;i<3;i++){mobjinfo[ids[i]]=mobjinfo[MT_PLAYER];mobjinfo[ids[i]].doomednum=-1;mobjinfo[ids[i]].spawnstate=1;mobjinfo[ids[i]].seestate=2;mobjinfo[ids[i]].seesound=v[V_SOUND_ID];mobjinfo[ids[i]].reactiontime=8;mobjinfo[ids[i]].flags=v[V_FLAGS];mobjinfo[ids[i]].height=v[V_OBSHEIGHT];}}
for(i=0;i<v[V_SKIP];i++){int draw=P_Random();observe(9,-1,draw);}
observer=P_SpawnMobj(v[V_OX],v[V_OY],v[V_OZ],v[V_OTYPE]);observer->angle=(angle_t)v[V_ANGLE];observer->threshold=v[V_THRESHOLD];
for(i=0;i<4;i++){players[i].mo=P_SpawnMobj(v[V_P0X+5*i],v[V_P0Y+5*i],v[V_P0Z+5*i],MT_PLAYER);playeringame[i]=v[V_P0ACTIVE+5*i];players[i].health=v[V_P0HEALTH+5*i];}capture();
for(t=1;t<=v[V_TICS];t++){operation=t;query_index=-1;if(t==2&&v[V_FLIP_SLOT]>=0)players[v[V_FLIP_SLOT]].health=v[V_FLIP_HEALTH];if(t==1&&v[V_SOUND]>=0)P_NoiseAlert(owned[v[V_SOUND]],owned[v[V_EMITTER]]);
if(v[V_MODE]==0)P_CheckSight(observer,owned[1]);else if(v[V_MODE]==1)P_LookForPlayers(observer,v[V_ALLAROUND]);else if(v[V_MODE]==2)A_Look(observer);else if(v[V_MODE]==3)P_RunThinkers();else abort();capture();}
}for(i=0;i<owned_count;i++)free(owned[i]);return 0;}
'''
WRAPPERS=r'''
boolean P_CheckSight(mobj_t* a,mobj_t* b){int pair=(int)(a->subsector->sector-sectors)*numsectors+(int)(b->subsector->sector-sectors);int reject=(rejectmatrix[pair>>3]&(1<<(pair&7)))!=0;boolean visible;query_index++;printf("V,%d,%d,%d,0,%d,%d,%d,%d\n",scenario,operation,query_index,ident(a),ident(b),pair,reject);visible=Raw_CheckSight(a,b);printf("Q,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,query_index,visible,reject?0:sightzstart,reject?0:bottomslope,reject?0:topslope);return visible;}
boolean P_LookForPlayers(mobj_t* a,boolean allaround){boolean result=Raw_LookForPlayers(a,allaround);printf("K,%d,%d,7,%d,%d,%d\n",scenario,operation,ident(a),result,a->lastlook);return result;}
'''
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
 shim=(ROOT/'tools/reference/m3d3a_shim.h').read_text().replace('RNG_TABLE',','.join(map(str,base.table('m_random.c',r'unsigned char\s+rndtable\[256\]'))))
 shim=shim.replace('query_index,stage,','query_index,stage,').replace('query_index,(long)(l-lines),stage','query_index,(long)(l-lines),stage').replace(',4,%ld,%d,%d,%d',',5,%ld,%d,%d,%d').replace(',7,%ld,%d',',8,%ld,%d')
 parts=[]
 for file,name,result in FUNCTIONS:
  rename={'P_CheckSight':'Raw_CheckSight','P_LookForPlayers':'Raw_LookForPlayers'}.get(name)
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
  parts.append(b)
 parts.insert(0,base.body('m_random.c','P_Random','int','Raw_Random')+'int P_Random(void){int value=Raw_Random();observe(8,-1,value);return value;}\nstatic int jitter(void){int a=P_Random();int b=P_Random();return (a-b)*1024;}')
 for name in actors.ACTIONS[1:]:
  if name=='A_Look':continue
  if name in ['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']:
   b=base.body('p_enemy.c',name,'void');arg='mo' if name=='A_PlayerScream' else 'actor';b=re.sub(r'(\{)',r'\1\n observe(5,ident('+arg+'),'+str(actors.ACTIONS.index(name))+');',b,count=1)
  else:b=f'void {name}(mobj_t* actor){{(void)actor;abort();}}'
  parts.append(b)
 fields=list(CASES[0])[1:];fractions=[[0,0,128*U,0,64*U,-64*U,0,128*U],[0,0,0,0,0,0,0,0],[0,0,U,U,0,0,U,U]]
 seed=7
 for i in range(128):
  row=[]
  for j in range(8):seed=(seed*1664525+1013904223)&0xffffffff;row.append((seed%257-128)*U+(seed%65536))
  fractions.append(row)
 def c_int(v):return str(v if v<2147483648 else v-4294967296)
 text=shim.replace('/* SOURCE_BODIES */','\n'.join(parts)+'\n'+WRAPPERS)+'\n'+combat.c_maps()+'\n'+post_loader()+'\nenum {'+','.join('V_'+f.upper() for f in fields)+'};\n#define CASE_COUNT '+str(len(CASES))+'\nint input[]['+str(len(fields))+']={'+','.join('{'+','.join(c_int(c[f]) for f in fields)+'}' for c in CASES)+'};\n#define FRACTION_COUNT '+str(len(fractions))+'\nint fractions_input[][8]={'+','.join('{'+','.join(map(str,r))+'}' for r in fractions)+'};\n'+MAIN
 outputs={f'tests/reference/m3d3a_map_{i}.bin':m['bytes'] for i,m in enumerate(MAPS)}
 with tempfile.TemporaryDirectory(prefix='doom-m3d3a-reference-') as tmp:
  src=Path(tmp)/'reference.c';src.write_text(text);binary=Path(tmp)/'reference';subprocess.run(['cc',*FLAGS,'-I'+str(SOURCE),str(src),str(SOURCE/'m_fixed.c'),str(SOURCE/'tables.c'),'-o',str(binary)],check=True)
  for mode in ['trace','sides','fractions']:outputs[f'tests/reference/m3d3a_{mode}.csv']=subprocess.check_output([str(binary),mode],timeout=30)
 table=io.StringIO();w=csv.writer(table,lineterminator='\n');w.writerow(['case',*CASES[0]]);w.writerows([[i,*c.values()] for i,c in enumerate(CASES)]);outputs['tests/reference/m3d3a_cases.csv']=table.getvalue().encode()
 files=sorted({f for f,_,_ in FUNCTIONS}|{'info.c','m_random.c','m_fixed.c','tables.c','p_enemy.c'}|{p.name for p in SOURCE.glob('*.h')})
 extracted=FUNCTIONS+[('m_random.c','P_Random','int')]+[('p_enemy.c',n,'void') for n in ['A_Fall','A_Pain','A_Scream','A_XScream','A_PlayerScream']]
 manifest={'revision':'a77dfb96cb91780ca334d0d4cfd86957558007e0','functions':extracted,'flags':FLAGS,'source_sha256':{f:hashlib.sha256((SOURCE/f).read_bytes()).hexdigest() for f in files},'adapter_sha256':{f:hashlib.sha256((ROOT/'tools/reference'/f).read_bytes()).hexdigest() for f in ['generate.py','generate_m3b.py','generate_m3c.py','generate_m3d1.py','generate_m3d3a.py','m3d3a_shim.h']},'outputs_sha256':{p:hashlib.sha256(b).hexdigest() for p,b in outputs.items()},'assumptions':['real original structs/info and 32-bit fixed/int, 64-bit pointer/function-union ABI','exact sight horizontal x==node.y quirk, shifts/division and BSP order retained','native states 1/2 and types 19/21/136 explicitly isolate A_Look with action-free see handoff; no chase/attacks/player damage','empty fallback roster, stale identities and unsafe extremes excluded from original-C execution and tested as declared Rust safety policies','source retired memory retained solely for observation; no Rust expected values or proprietary data read']}
 path=ROOT/'tests/reference/m3d3a_provenance.json'
 if check:
  old=json.loads(path.read_text())
  for k,v in manifest.items():assert old[k]==json.loads(json.dumps(v)),k
 else:
  manifest.update(compiler=subprocess.check_output(['cc','--version'],text=True).splitlines()[0],platform=platform.platform());outputs['tests/reference/m3d3a_provenance.json']=(json.dumps(manifest,indent=2)+'\n').encode()
 for p,b in outputs.items():
  if check:assert (ROOT/p).read_bytes()==b,p
  else:(ROOT/p).write_bytes(b)
 print(('Checked' if check else 'Generated')+f' M3d3a {len(CASES)} original-body scenarios, 1536 side and {len(fractions)} fraction vectors.')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');run(p.parse_args().check)
