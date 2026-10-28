/* GPL-2.0-only. Original-body perception adapter, not an engine port.
 * Real original structs, metadata, BSP, movement, states, damage and tracing.
 * Unsupported player/attack/special/projectile/drop services abort if reached.
 * Retirement retains bytes until reset to define original freed-next traversal. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <limits.h>
#include "z_zone.h"
#include "p_local.h"
#include "doomstat.h"
#include "r_state.h"
#include "sounds.h"
#include "s_sound.h"
#include "i_system.h"
#include "m_random.h"
#include "m_bbox.h"
#include "info.c"
typedef char require_reference_abi[(sizeof(int)==4 && sizeof(short)==2 && sizeof(void*)==8 && sizeof(fixed_t)==4 && sizeof(angle_t)==4) ? 1 : -1];
static line_t line_storage[128];
static vertex_t vertices[256];
static sector_t sector_storage[128];
static subsector_t leaf_storage[128];
static node_t node_storage[128];
line_t* lines=line_storage;
#define sectors sector_storage
#define subsectors leaf_storage
#define nodes node_storage
static sector_t sector;
int numsectors,numlines,numnodes,validcount,skyflatnum=-1;
fixed_t viewx,viewy;
fixed_t* finecosine=finesine+FINEANGLES/4;
fixed_t tmbbox[4],tmx,tmy,tmfloorz,tmceilingz,tmdropoffz;
mobj_t* tmthing;
int tmflags,numspechit;
boolean floatok;
line_t* ceilingline;
line_t* spechit[8];
fixed_t opentop,openbottom,openrange,lowfloor;
int bmapwidth,bmapheight;
fixed_t bmaporgx,bmaporgy;
short *blockmaplump,*blockmap;
static mobj_t* block_heads[4096];
mobj_t** blocklinks=block_heads;
intercept_t intercepts[128],*intercept_p;
divline_t trace;
boolean earlyout;
int ptflags;
mobj_t *linetarget,*shootthing;
fixed_t shootz,attackrange,aimslope,topslope,bottomslope;
int la_damage;
int leveltime,consoleplayer;
boolean respawnmonsters,deathmatch,nomonsters,netgame,automapactive;
skill_t gameskill=sk_medium;
GameMode_t gamemode=commercial;
player_t players[MAXPLAYERS];
thinker_t thinkercap;
mapthing_t itemrespawnque[ITEMQUESIZE];
int itemrespawntime[ITEMQUESIZE],iquehead,iquetail;
int prndindex,rndindex;
unsigned char rndtable[256]={ RNG_TABLE };
static mobj_t* owned[1024];
static int alive[1024],owned_count,scenario,operation,trace_stage,last_object,last_kind;
static fixed_t impact[3];
static int ident(void* p){int i;for(i=0;i<owned_count;i++)if(owned[i]==p)return i;abort();return -1;}
static void observe(int code,int actor,int value){printf("E,%d,%d,%d,%d,%d\n",scenario,operation,code,actor,value);}
static void candidate(int line,int actor){printf("C,%d,%d,%d,%d,%d\n",scenario,operation,trace_stage,line,actor);}
static int intercept_id(intercept_t* in){return in->isaline?(int)(in->d.line-lines):ident(in->d.thing);}
void* Z_Malloc(int size,int tag,void* user){mobj_t* p;(void)tag;(void)user;if(owned_count==1024)abort();p=calloc(1,size);owned[owned_count]=p;alive[owned_count++]=1;return p;}
void Z_Free(void* p){alive[ident(p)]=0;observe(3,ident(p),0);}
void I_Error(char* fmt,...){(void)fmt;abort();}
void I_Tactile(int a,int b,int c){(void)a;(void)b;(void)c;abort();}
void S_StartSound(void* p,int sound){observe(7,p?ident(p):-1,sound);}
void S_StopSound(void* p){(void)p;}
void P_DropWeapon(player_t* p){(void)p;abort();}
void AM_Stop(void){abort();}
void P_SlideMove(mobj_t* p){(void)p;abort();}

void P_NightmareRespawn(mobj_t* p){(void)p;abort();}
void P_TouchSpecialThing(mobj_t* a,mobj_t* b){(void)a;(void)b;abort();}
void P_CrossSpecialLine(int i,int s,mobj_t* p){(void)i;(void)s;(void)p;abort();}
void P_ShootSpecialLine(mobj_t* p,line_t* l){(void)p;(void)l;abort();}
#define STOPSPEED 0x1000
#define FRICTION 0xe800
void P_CheckMissileSpawn(mobj_t*);
void P_ExplodeMissile(mobj_t*);
void P_XYMovement(mobj_t*);
void P_ZMovement(mobj_t*);

void P_ExplodeMissile(mobj_t* p){(void)p;abort();}
static side_t side_storage[256];side_t* sides=side_storage;
static line_t* sector_lines[128][128];
static seg_t seg_storage[8192];seg_t* segs=seg_storage;
int numsubsectors;
static byte reject_storage[2048];byte* rejectmatrix=reject_storage;
fixed_t sightzstart,t2x,t2y;divline_t strace;int sightcounts[2];
mobj_t* soundtarget;boolean playeringame[MAXPLAYERS];
static int query_index=-1;
int P_DivlineSide(fixed_t,fixed_t,divline_t*);
boolean P_LookForPlayers(mobj_t*,boolean);
static int node_side(fixed_t x,fixed_t y,divline_t* d,int stage){int result=P_DivlineSide(x,y,d);printf("V,%d,%d,%d,%d,%ld,%d\n",scenario,operation,query_index,stage,(long)((node_t*)d-nodes),result);return result;}
static void line_sides(line_t* l,int stage,int a,int b){printf("V,%d,%d,%d,4,%ld,%d,%d,%d\n",scenario,operation,query_index,(long)(l-lines),stage,a,b);}
static void sight_block(line_t* l,int reason){printf("V,%d,%d,%d,7,%ld,%d\n",scenario,operation,query_index,(long)(l-lines),reason);}
static int look_random(mobj_t* a){int v=P_Random();printf("K,%d,%d,9,%d,%d\n",scenario,operation,ident(a),v);return v;}
static int attempt=-1,block_actor=-1,block_line=-1,move_failure=0;
static void pcand(int line,int index){printf("C,%d,%d,%d,%d,%d\n",scenario,operation,attempt,line,index);}
static int block_result(int result,mobj_t* p){if(!result){block_actor=ident(p);move_failure=1;}return result;}
static void record_move(mobj_t* p,int x,int y,int ok){subsector_t* s=R_PointInSubsector(x,y);printf("M,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%ld,%ld,%d,%d\n",scenario,operation,attempt,ident(p),x,y,ok,tmfloorz,tmceilingz,tmdropoffz,floatok,move_failure,(long)(s-subsectors),(long)(s->sector-sectors),block_line,block_actor);}
typedef enum {DI_EAST,DI_NORTHEAST,DI_NORTH,DI_NORTHWEST,DI_WEST,DI_SOUTHWEST,DI_SOUTH,DI_SOUTHEAST,DI_NODIR,NUMDIRS} dirtype_t;
dirtype_t opposite[]={4,5,6,7,0,1,2,3,8},diags[]={3,1,5,7};
fixed_t xspeed[]={65536,47000,0,-47000,-65536,-47000,0,47000};
fixed_t yspeed[]={0,47000,65536,47000,0,-47000,-65536,-47000};
boolean fastparm=false;
boolean P_UseSpecialLine(mobj_t* a,line_t* l,int side){(void)a;(void)l;(void)side;abort();return false;}
boolean P_Move(mobj_t*);boolean P_TryWalk(mobj_t*);void P_NewChaseDir(mobj_t*);
boolean P_CheckMeleeRange(mobj_t* a){(void)a;abort();return false;}
boolean P_CheckMissileRange(mobj_t* a){(void)a;abort();return false;}
static void pursuit_start(mobj_t* a,int op){printf("H,%d,%d,0,%d,%d\n",scenario,operation,ident(a),op);}
static void pursuit_return(mobj_t* a,int op,int result){printf("H,%d,%d,1,%d,%d,%d\n",scenario,operation,ident(a),op,result);}
static void pursuit_direction(mobj_t* a){printf("H,%d,%d,2,%d,%d\n",scenario,operation,ident(a),a->movedir);}
static int pursuit_random(mobj_t* a,int kind){int v=P_Random();printf("H,%d,%d,3,%d,%d,%d\n",scenario,operation,ident(a),kind,v);return v;}
#define CHASE_ACTION 15
/* SOURCE_BODIES */

