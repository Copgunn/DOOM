/* GPL-2.0-only. Isolated adapters; original bodies inserted by generate_m3c.py.
 * Single flat sector, zero XY/Z momentum; motion callbacks abort if reached.
 * Z_Free retains bytes so original P_RunThinkers' freed-next read is defined.
 * No graphics, audio, player setup, specials, AI or commercial data. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include "z_zone.h"
#include "p_local.h"
#include "doomstat.h"
#include "sounds.h"
#include "s_sound.h"
#include "i_system.h"
#include "m_random.h"
#include "info.c"
static sector_t sector;
static subsector_t leaf;
static mobj_t* allocated[512];
static int alive[512], count, step, scenario, rngindex;
static int rngtable[256] = { RNG_TABLE };
int leveltime, totalkills, totalitems;
boolean respawnmonsters, deathmatch, nomonsters;
boolean netgame;
skill_t gameskill;
GameMode_t gamemode=commercial;
mapthing_t deathmatchstarts[10], *deathmatch_p=deathmatchstarts, playerstarts[4];
thinker_t thinkercap;
mapthing_t itemrespawnque[ITEMQUESIZE];
int itemrespawntime[ITEMQUESIZE], iquehead, iquetail;
static int ident(void* actor){int i;for(i=0;i<count;i++)if(allocated[i]==actor)return i;abort();}
static void event(int code,int actor,int argument){printf("E,%d,%d,%d,%d,%d\n",scenario,step,code,actor,argument);}
void* Z_Malloc(int size,int tag,void* user){mobj_t* p;(void)tag;(void)user;if(count==512)abort();p=calloc(1,size);allocated[count]=p;alive[count++]=1;return p;}
void Z_Free(void* p){alive[ident(p)]=0;event(3,ident(p),-1);}
int P_Random(void){rngindex=(rngindex+1)&255;return rngtable[rngindex];}
void P_SetThingPosition(mobj_t* p){p->subsector=&leaf;}
void P_UnsetThingPosition(mobj_t* p){(void)p;}
void S_StopSound(void* p){(void)p;}
void S_StartSound(void* p,int sound){event(7,p?ident(p):-1,sound);}
void I_Error(char* fmt,...){(void)fmt;abort();}
void P_SpawnPlayer(mapthing_t* thing){(void)thing;}
void P_XYMovement(mobj_t* p){(void)p;abort();}
void P_ZMovement(mobj_t* p){(void)p;abort();}
void P_NightmareRespawn(mobj_t* p){(void)p;abort();}
static int action_random(mobj_t* p){int value=P_Random();event(8,ident(p),value);return value;}
/* SOURCE_BODIES */
static void probe_random(mobj_t* p){event(5,ident(p),100);event(8,ident(p),P_Random());}
static void probe_enter(mobj_t* p){event(5,ident(p),101);P_SetMobjState(p,5);}
static void probe_spawn(mobj_t* p){mobj_t* child;event(5,ident(p),102);child=P_SpawnMobj(p->x,p->y,p->z,132);p->target=child;child->target=p;event(6,ident(p),ident(child));event(6,ident(child),ident(p));}
static void probe_remove_self(mobj_t* p){event(5,ident(p),103);P_RemoveMobj(p);}
static void probe_remove_target(mobj_t* p){event(5,ident(p),104);P_RemoveMobj(p->target);}
static void set_state(int i,int tics,int next,actionf_p1 action){states[i].sprite=0;states[i].frame=i;states[i].tics=tics;states[i].nextstate=next;states[i].action.acp1=action;states[i].misc1=states[i].misc2=0;}
static void setup_probe(void){int i;
 set_state(1,1,2,NULL);set_state(2,0,3,(actionf_p1)probe_random);
 set_state(3,0,4,(actionf_p1)probe_spawn);set_state(4,0,8,(actionf_p1)probe_enter);
 set_state(5,0,6,(actionf_p1)probe_random);set_state(6,2,0,(actionf_p1)probe_random);
 set_state(8,-1,0,NULL);set_state(9,1,10,NULL);
 set_state(10,0,11,(actionf_p1)probe_remove_target);set_state(11,1,0,(actionf_p1)probe_remove_self);
 for(i=131;i<=132;i++){memset(&mobjinfo[i],0,sizeof(mobjinfo[i]));mobjinfo[i].doomednum=-1;mobjinfo[i].spawnstate=i==131?1:9;mobjinfo[i].radius=8*FRACUNIT;mobjinfo[i].height=16*FRACUNIT;mobjinfo[i].flags=MF_NOGRAVITY;mobjinfo[i].spawnhealth=100;}
}
static void reset(int which){int i;for(i=0;i<count;i++)free(allocated[i]);count=0;memset(alive,0,sizeof(alive));scenario=which;step=0;rngindex=0;sector.floorheight=0;sector.ceilingheight=128*FRACUNIT;leaf.sector=&sector;P_InitThinkers();deathmatch_p=deathmatchstarts;netgame=nomonsters=deathmatch=0;gameskill=sk_medium;}
static void capture(void){int i;for(i=0;i<count;i++){mobj_t* p=allocated[i];printf("A,%d,%d,%d,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%u,%d,%d\n",scenario,step,i,alive[i],p->type,p->state?(long)(p->state-states):-1L,p->tics,p->sprite,p->frame,p->health,p->reactiontime,p->flags,p->lastlook,p->z,p->radius,p->height,p->angle,p->target?ident(p->target):-1,rngindex);}}
static int admitted_state(int root){int seen[NUMSTATES]={0};while(root&&!seen[root]){actionf_p1 a=states[root].action.acp1;seen[root]=1;if(a&&a!=(actionf_p1)A_Fall&&a!=(actionf_p1)A_Pain&&a!=(actionf_p1)A_Scream&&a!=(actionf_p1)A_XScream&&a!=(actionf_p1)A_PlayerScream)return 0;root=states[root].nextstate;}return 1;}
int main(void){mobj_t* p;int i,k;state_t saved_states[NUMSTATES];mobjinfo_t saved_types[NUMMOBJTYPES];memcpy(saved_states,states,sizeof(states));memcpy(saved_types,mobjinfo,sizeof(mobjinfo));
 reset(0);p=P_SpawnMobj(0,0,ONFLOORZ,MT_PLAYER);P_SpawnMobj(64*FRACUNIT,0,ONFLOORZ,MT_TFOG);P_SetMobjState(p,mobjinfo[p->type].deathstate);capture();
 for(step=1;step<=80;step++){P_RunThinkers();capture();}
 for(k=0;k<5;k++){reset(1+k);gameskill=k;nomonsters=1;
  {mapthing_t things[]={{0,0,90,1,7},{0,0,0,11,0},{0,0,89,44,15},{32,0,-91,49,7},{0,0,0,3004,7},{0,0,0,9999,16},{0,0,0,9999,0},{64,0,359,44,1},{64,0,359,44,4}};
  for(i=0;i<9;i++)P_SpawnMapThing(&things[i]);}capture();}
 reset(6);setup_probe();P_SpawnMobj(0,0,ONFLOORZ,131);capture();for(step=1;step<=3;step++){P_RunThinkers();capture();}
 reset(7);set_state(1,-1,0,NULL);mobjinfo[131].spawnstate=1;mobjinfo[131].painsound=sfx_dmpain;mobjinfo[131].deathsound=sfx_podth1;
 set_state(2,-1,0,(actionf_p1)A_Pain);set_state(3,-1,0,(actionf_p1)A_Scream);set_state(4,-1,0,(actionf_p1)A_XScream);set_state(5,-1,0,(actionf_p1)A_PlayerScream);set_state(6,-1,0,(actionf_p1)A_Fall);
 p=P_SpawnMobj(0,0,ONFLOORZ,131);capture();for(step=1;step<=5;step++){P_SetMobjState(p,step+1);capture();}
 reset(8);states[1].tics=0;states[1].nextstate=2;states[1].action.acp1=(actionf_p1)probe_random;p=P_SpawnMobj(0,0,ONFLOORZ,131);capture();step=1;P_RunThinkers();capture();step=2;P_SetMobjState(p,1);capture();
 reset(9);memcpy(states,saved_states,sizeof(states));memcpy(mobjinfo,saved_types,sizeof(mobjinfo));
 for(i=0;i<NUMMOBJTYPES;i++){mobjinfo_t* t=&mobjinfo[i];if((t->flags&~(2|4|8|16|32|256|512|1024|2048|262144|33554432))||t->radius<0||t->radius>32*FRACUNIT||t->height<=0||t->height>128*FRACUNIT||!admitted_state(t->spawnstate)||!admitted_state(t->deathstate))continue;P_SpawnMobj(0,0,ONFLOORZ,i);P_SpawnMobj(0,0,ONCEILINGZ,i);P_SpawnMobj(0,0,16*FRACUNIT,i);}capture();
 reset(10);return 0;
}
