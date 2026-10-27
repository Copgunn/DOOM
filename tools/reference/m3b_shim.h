/* Isolated Linux DOOM reference adapter, GPL-2.0-only. Not an engine port.
 * Bodies are extracted from preserved sources by generate.py. Unsupported
 * gameplay branches abort; static multi-sector XY/Z vectors with explicit source adapters. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdarg.h>
#include <limits.h>
#include <float.h>
#include <string.h>
#include <setjmp.h>
#include "m_fixed.h"
#include "m_random.h"
typedef uint32_t angle_t;
#define NF_SUBSECTOR 0x8000
#define VIEWHEIGHT (41*FRACUNIT)
#define MF_INFLOAT 0x200000
#define GRAVITY FRACUNIT
#define FLOATSPEED (4*FRACUNIT)
#define MF_NOGRAVITY 512
#define sfx_oof 0
#define MAXMOVE (30*FRACUNIT)
#define MAXRADIUS (32*FRACUNIT)
#define MAPBLOCKSHIFT 23
#define ANGLETOFINESHIFT 19
#define MAPBLOCKSIZE (128*FRACUNIT)
#define MAPBTOFRAC 7
#define PT_ADDLINES 1
#define PT_ADDTHINGS 2
#define PT_EARLYOUT 4
#define ANG180 0x80000000u
#define ANG270 0xc0000000u
#define SLOPERANGE 2048
#define ANG90 0x40000000u
#define STOPSPEED 0x1000
#define FRICTION 0xe800
enum { BOXTOP, BOXBOTTOM, BOXLEFT, BOXRIGHT };
enum { ST_HORIZONTAL, ST_VERTICAL, ST_POSITIVE, ST_NEGATIVE };
enum { MF_SPECIAL=1, MF_SOLID=2, MF_SHOOTABLE=4, MF_NOSECTOR=8, MF_NOBLOCKMAP=16, MF_PICKUP=0x800,
       MT_KNIGHT=1, MT_BRUISER=2, MT_PLAYER=0,
       MF_MISSILE=0x10000, MF_SKULLFLY=0x1000000, MF_CORPSE=0x100000,
       MF_NOCLIP=0x1000, MF_TELEPORT=0x8000, MF_DROPOFF=0x400, MF_FLOAT=0x4000,
       ML_TWOSIDED=4, ML_BLOCKING=1, ML_BLOCKMONSTERS=2, CF_NOMOMENTUM=4, S_PLAY=0, S_PLAY_RUN1=1 };
typedef struct { int dummy; } state_t;
typedef struct { fixed_t x,y; } vertex_t;
typedef struct { fixed_t floorheight,ceilingheight; int ceilingpic; struct mobj_s* thinglist; } sector_t;
typedef struct { sector_t* sector; } subsector_t;
typedef struct { vertex_t *v1,*v2; fixed_t dx,dy,bbox[4]; int slopetype,flags,special,validcount; short sidenum[2]; sector_t *frontsector,*backsector; } line_t;
typedef struct { signed char forwardmove,sidemove; short angleturn; unsigned char buttons; } ticcmd_t;
struct mobj_s;
typedef struct { struct mobj_s* mo; ticcmd_t cmd; int cheats; fixed_t viewheight,deltaviewheight; } player_t;
typedef struct { int spawnstate,damage; } mobjinfo_t;
typedef struct mobj_s { fixed_t x,y,z,momx,momy,momz,floorz,ceilingz,radius,height; int flags; angle_t angle; player_t* player; mobjinfo_t* info; subsector_t* subsector; state_t* state; struct mobj_s *bnext,*bprev,*snext,*sprev,*target; int type,id; } mobj_t;
typedef void (*actionf_v)(void);
typedef void (*actionf_p1)(void*);
typedef union { actionf_v acv; actionf_p1 acp1; } actionf_t;
typedef struct thinker_s { struct thinker_s *prev,*next; actionf_t function; } thinker_t;
thinker_t thinkercap;
state_t states[8];
int onground,skyflatnum;
fixed_t tmfloorz,tmdropoffz,tmceilingz,tmx,tmy,opentop,openbottom,lowfloor;
mobj_t* tmthing;
int tmflags,floatok,validcount,numspechit;
fixed_t tmbbox[4];
line_t lines[32],*ceilingline,*spechit[8];
vertex_t vertices[32];
sector_t sector={-16*FRACUNIT,128*FRACUNIT,0};
subsector_t subsectors[8];
sector_t sectors[8];
typedef struct { fixed_t x,y,dx,dy; unsigned short children[2]; } node_t;
node_t nodes[16]; int numnodes;
int bmapwidth=2,bmapheight=2;
fixed_t bmaporgx=-128*FRACUNIT,bmaporgy=-128*FRACUNIT;
short blockstorage[]={-128,-128,2,2,8,8,8,8,0,0,1,2,3,-1};
short *blockmaplump=blockstorage,*blockmap=blockstorage+4;
int attempts,candidate_count,candidates[4096];
void I_Error(char* message,...) { (void)message; abort(); }
/* Retain storage: avoid the original traversal's read-after-free. */
void Z_Free(void* p) { (void)p; }
void P_SetMobjState(mobj_t* m,int state) { (void)m;(void)state; abort(); }
void P_RemoveMobj(mobj_t* m) { (void)m; abort(); }
void P_ExplodeMissile(mobj_t* m) { (void)m; abort(); }
void P_CrossSpecialLine(int n,int side,mobj_t* m) { (void)n;(void)side;(void)m; abort(); }
typedef struct { fixed_t x,y,dx,dy; } divline_t;
typedef struct { fixed_t frac; int isaline; union { line_t* line; mobj_t* thing; } d; } intercept_t;
typedef boolean (*traverser_t)(intercept_t*);
intercept_t intercepts[4096],*intercept_p;
divline_t trace;
boolean earlyout;
int ptflags;
mobj_t* blocklinks[64];
fixed_t bestslidefrac,secondslidefrac,tmxmove,tmymove,openrange;
line_t *bestslideline,*secondslideline;
mobj_t* slidemo;
fixed_t viewx,viewy;
int thing_count,things_seen[4096],slide_count,slides_seen[4096];
int try_count,try_x[128],try_y[128],try_allowed[128];
boolean Ref_TryMove(mobj_t*,fixed_t,fixed_t);
boolean Ref_CheckLine(line_t*);
boolean Ref_CheckThing(mobj_t*);
void Ref_HitSlideLine(line_t*);
void P_SlideMove(mobj_t*);
void P_UnsetThingPosition(mobj_t*);
void P_SetThingPosition(mobj_t*);
boolean P_BlockThingsIterator(int,int,boolean(*)(mobj_t*));
boolean P_BlockLinesIterator(int,int,boolean(*)(line_t*));
void P_DamageMobj(mobj_t* a,mobj_t* b,mobj_t* c,int n) { abort(); }
void P_TouchSpecialThing(mobj_t* a,mobj_t* b) { abort(); }
boolean PIT_AddThingIntercepts(mobj_t* a) { abort(); return false; }
int motion_count,motion_kind[256],motion_value[256];
void record_motion(int kind,int value){int i=motion_count++;if(i>=256)abort();motion_kind[i]=kind;motion_value[i]=value;}
void P_HitSlideLine(line_t* l) { record_motion(1,(int)(l-lines)); if(slide_count>=4096)abort();slides_seen[slide_count++]=(int)(l-lines);Ref_HitSlideLine(l); }
/* Explicit ABI/compiler-profile assumptions, rather than host C defaults. */
typedef char int_is_32[(sizeof(int)==4)?1:-1];
typedef char short_is_16[(sizeof(short)==2)?1:-1];
typedef char byte_is_8[(CHAR_BIT==8)?1:-1];
typedef char long_long_is_64[(sizeof(long long)==8)?1:-1];
typedef char arithmetic_shift[((-2>>1)==-1)?1:-1];
typedef char binary64[(DBL_MANT_DIG==53 && FLT_RADIX==2)?1:-1];

subsector_t* R_PointInSubsector(fixed_t,fixed_t);
void Ref_LineOpening(line_t*);
int try_data[128][15],checking,blocked_line=-1,blocked_actor=-1;
int opening_count,opening_data[4096][5];
int vertical_called,before_z,before_momz,sound_count;
void S_StartSound(mobj_t* m,int n){(void)m;(void)n;sound_count++;}
void P_LineOpening(line_t* l){int i;Ref_LineOpening(l);if(checking){i=opening_count++;if(i>=4096)abort();opening_data[i][0]=(int)(l-lines);opening_data[i][1]=opentop;opening_data[i][2]=openbottom;opening_data[i][3]=openrange;opening_data[i][4]=lowfloor;}}
boolean P_TryMove(mobj_t* m,fixed_t x,fixed_t y){int i=try_count++,ok,reason=0,lc=candidate_count,tc=thing_count,oc=opening_count;subsector_t* ss;if(i>=128)abort();record_motion(0,i);blocked_line=blocked_actor=-1;checking=1;ok=Ref_TryMove(m,x,y);checking=0;ss=R_PointInSubsector(x,y);
if(!ok){if(blocked_actor>=0)reason=1;else if(blocked_line>=0)reason=2;else if(tmceilingz-tmfloorz<m->height)reason=3;else if(tmceilingz-m->z<m->height)reason=4;else if(tmfloorz-m->z>24*FRACUNIT)reason=5;else reason=6;}
try_data[i][0]=x;try_data[i][1]=y;try_data[i][2]=ok;try_data[i][3]=tmfloorz;try_data[i][4]=tmceilingz;try_data[i][5]=tmdropoffz;try_data[i][6]=floatok;try_data[i][7]=reason;try_data[i][8]=(int)(ss-subsectors);try_data[i][9]=(int)(ss->sector-sectors);try_data[i][10]=blocked_line;try_data[i][11]=blocked_actor;try_data[i][12]=candidate_count-lc;try_data[i][13]=thing_count-tc;try_data[i][14]=opening_count-oc;return ok;}
boolean PIT_CheckLine(line_t* l){int ok;if(candidate_count>=4096)abort();candidates[candidate_count++]=(int)(l-lines);ok=Ref_CheckLine(l);if(!ok)blocked_line=(int)(l-lines);return ok;}
boolean PIT_CheckThing(mobj_t* m){int ok;if(thing_count>=4096)abort();things_seen[thing_count++]=m->id;ok=Ref_CheckThing(m);if(!ok)blocked_actor=m->id;return ok;}
