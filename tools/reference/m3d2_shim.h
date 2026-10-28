/* GPL-2.0-only. Original-body combat adapter, not an engine port.
 * Real original structs, metadata, BSP, movement, states, damage and tracing.
 * Unsupported player/AI/special/projectile/drop services abort if reached.
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
#define rcsid weapon_rcsid
#include "d_items.c"
#undef rcsid
#include "d_event.h"
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

void AM_Stop(void){abort();}
void P_SlideMove(mobj_t* p){(void)p;abort();}
void P_ExplodeMissile(mobj_t* p){(void)p;abort();}
void P_NightmareRespawn(mobj_t* p){(void)p;abort();}
void P_TouchSpecialThing(mobj_t* a,mobj_t* b){(void)a;(void)b;abort();}
void P_CrossSpecialLine(int i,int s,mobj_t* p){(void)i;(void)s;(void)p;abort();}
void P_ShootSpecialLine(mobj_t* p,line_t* l){(void)p;(void)l;abort();}
#define STOPSPEED 0x1000
#define FRICTION 0xe800
void P_XYMovement(mobj_t*);
void P_ZMovement(mobj_t*);
void P_SetPsprite(player_t*,int,statenum_t);
void P_BringUpWeapon(player_t*);
boolean playeringame[MAXPLAYERS];
boolean onground;
fixed_t bulletslope;
int maxammo[NUMAMMO]={200,50,300,50};
static side_t side_storage[256];
#define sides side_storage
static line_t* sector_lines[128][128];
mobj_t* soundtarget;
#define MAXBOB 0x100000
#define LOWERSPEED (FRACUNIT*6)
#define RAISESPEED (FRACUNIT*6)
#define WEAPONBOTTOM (128*FRACUNIT)
#define WEAPONTOP (32*FRACUNIT)
#define BFGCELLS 40
#define INVERSECOLORMAP 32
void P_PlayerInSpecialSector(player_t* p){(void)p;abort();}
void P_UseLines(player_t* p){(void)p;abort();}
void P_DeathThink(player_t* p){(void)p;abort();}
static void w_entry(player_t* p,int pos,int state){printf("W,%d,%d,0,%ld,%d,%d\n",scenario,operation,(long)(p-players),pos,state);}
static void w_action(player_t* p,pspdef_t* ps,int action){printf("W,%d,%d,1,%ld,%ld,%d\n",scenario,operation,(long)(p-players),(long)(ps-p->psprites),action);}
static int w_random(int kind){int value=P_Random();printf("W,%d,%d,8,%d,%d\n",scenario,operation,kind,value);return value;}
static int spread(void){int a=w_random(1);int b=w_random(2);return (a-b)*262144;}
static void bullet(mobj_t* mo,unsigned angle,int slope,int damage){printf("W,%d,%d,5,%d,%u,%d,%d\n",scenario,operation,ident(mo),angle,slope,damage);}
/* SOURCE_BODIES */
