/* Isolated Linux DOOM reference adapter, GPL-2.0-only. Not an engine port.
 * Bodies are extracted from preserved sources by generate.py. Unsupported
 * gameplay branches abort; single-sector/grounded/non-player vectors only. */
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
#define MAXMOVE (30*FRACUNIT)
#define MAXRADIUS (32*FRACUNIT)
#define MAPBLOCKSHIFT 23
#define ANGLETOFINESHIFT 19
#define ANG90 0x40000000u
#define STOPSPEED 0x1000
#define FRICTION 0xe800
enum { BOXTOP, BOXBOTTOM, BOXLEFT, BOXRIGHT };
enum { ST_HORIZONTAL, ST_VERTICAL, ST_POSITIVE, ST_NEGATIVE };
enum { MF_MISSILE=0x10000, MF_SKULLFLY=0x1000000, MF_CORPSE=0x100000,
       MF_NOCLIP=0x1000, MF_TELEPORT=0x8000, MF_DROPOFF=0x400, MF_FLOAT=0x4000,
       ML_BLOCKING=1, ML_BLOCKMONSTERS=2, CF_NOMOMENTUM=4, S_PLAY=0, S_PLAY_RUN1=1 };
typedef struct { int dummy; } state_t;
typedef struct { fixed_t x,y; } vertex_t;
typedef struct { fixed_t floorheight,ceilingheight; int ceilingpic; } sector_t;
typedef struct { sector_t* sector; } subsector_t;
typedef struct { vertex_t *v1,*v2; fixed_t dx,dy,bbox[4]; int slopetype,flags,special,validcount; sector_t *frontsector,*backsector; } line_t;
typedef struct { signed char forwardmove,sidemove; short angleturn; unsigned char buttons; } ticcmd_t;
struct mobj_s;
typedef struct { struct mobj_s* mo; ticcmd_t cmd; int cheats; } player_t;
typedef struct { int spawnstate; } mobjinfo_t;
typedef struct mobj_s { fixed_t x,y,z,momx,momy,momz,floorz,ceilingz,radius,height; int flags; angle_t angle; player_t* player; mobjinfo_t* info; subsector_t* subsector; state_t* state; } mobj_t;
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
line_t lines[4],*ceilingline,*spechit[8];
vertex_t vertices[4];
sector_t sector={-16*FRACUNIT,128*FRACUNIT,0};
subsector_t subsector={&sector};
int bmapwidth=2,bmapheight=2;
fixed_t bmaporgx=-128*FRACUNIT,bmaporgy=-128*FRACUNIT;
short blockstorage[]={-128,-128,2,2,8,8,8,8,0,0,1,2,3,-1};
short *blockmaplump=blockstorage,*blockmap=blockstorage+4;
int attempts,candidate_count,candidates[64];
jmp_buf div_failure;
void I_Error(char* message,...) { (void)message; longjmp(div_failure,1); }
/* Retain storage: avoid the original traversal's read-after-free. */
void Z_Free(void* p) { (void)p; }
void P_SetMobjState(mobj_t* m,int state) { (void)m;(void)state; abort(); }
void P_RemoveMobj(mobj_t* m) { (void)m; abort(); }
void P_ExplodeMissile(mobj_t* m) { (void)m; abort(); }
void P_SlideMove(mobj_t* m) { (void)m; abort(); }
void P_LineOpening(line_t* l) { (void)l; abort(); }
void P_UnsetThingPosition(mobj_t* m) { (void)m; }
void P_SetThingPosition(mobj_t* m) { m->subsector=&subsector; }
void P_CrossSpecialLine(int n,int side,mobj_t* m) { (void)n;(void)side;(void)m; abort(); }
subsector_t* R_PointInSubsector(fixed_t x,fixed_t y) { (void)x;(void)y; return &subsector; }
boolean PIT_CheckThing(mobj_t* m) { (void)m; return true; }
boolean P_BlockThingsIterator(int x,int y,boolean(*f)(mobj_t*)) { (void)x;(void)y;(void)f; return true; }
boolean Ref_TryMove(mobj_t*,fixed_t,fixed_t);
boolean Ref_CheckLine(line_t*);
boolean P_TryMove(mobj_t* m,fixed_t x,fixed_t y) { attempts++; return Ref_TryMove(m,x,y); }
boolean PIT_CheckLine(line_t* l) { candidates[candidate_count++]=(int)(l-lines); return Ref_CheckLine(l); }
boolean P_BlockLinesIterator(int,int,boolean(*)(line_t*));
