#!/usr/bin/env python3
"""Genuine original-C doors, lifts, floors, switches and the exit (M3h8 Stage B).

Reuses every compiled M3h7 body, observer and controlled input unchanged (importing
m3h7_reference applies the M3h2..M3h7 harness patches) over the rooms of
m3h8_cases, and adds the genuine bodies of p_doors.c (T_VerticalDoor, EV_DoDoor,
EV_VerticalDoor), p_plats.c (T_PlatRaise, EV_DoPlat, P_AddActivePlat,
P_RemoveActivePlat), p_floor.c (T_MovePlane, T_MoveFloor, EV_DoFloor), p_switch.c
(P_StartButton, P_ChangeSwitchTexture, P_UseSpecialLine), p_spec.c (P_CrossSpecialLine
and the P_Find*/getSide/getSector/twoSided helpers), p_map.c (P_ThingHeightClip,
PTR_UseTraverse, P_UseLines, PIT_ChangeSector, P_ChangeSector) and g_game.c
(G_ExitLevel). The use fragment of P_PlayerThink (p_user.c:322-331) runs after the
weapon change, the BT_USE statement of P_DeathThink (p_user.c:227-228) after
P_MovePsprites for a dead player, and the sector soundorg/blockbox fragment of
P_GroupLines (p_setup.c:555-574) at load. The genuine P_TryMove crossing loop and the
P_Move failure branch, compiled since M3h2, reach real bodies for the first time.
`spechit[8]` keeps its source size (an overflow would be reported by the sanitizer
build). New rows are observers only (see the contract's `row_formats`). Expectations
come from compiled original bodies over synthetic inputs; they never consume Rust
output."""
import argparse,collections,csv,hashlib,io,json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3h7_reference as inherited
import m3h8_geometry as geometry
from m3h8_cases import ROOMS,ROOM_NAMES,DENSE9,populate,MAX_PICKUPS
from m3h2_observers import replace_once
from evidence_pack import pack,unpack
import m3g2_reference as body_stage
previous=inherited.previous;core=inherited.core;pickup_stage=inherited.pickup_stage
LAYOUTS=populate(previous.pack)
SCHEMA='DOOM-PLAYER-MOVERS-v1'
# Genuine bodies added to the inherited set, in definition order.
ADDED=[('m_bbox.c','M_ClearBox','void'),('m_bbox.c','M_AddToBox','void'),('p_spec.c','twoSided','int'),('p_spec.c','getSide','side_t*'),('p_spec.c','getSector','sector_t*'),
       ('p_spec.c','P_FindLowestFloorSurrounding','fixed_t'),('p_spec.c','P_FindHighestFloorSurrounding','fixed_t'),
       ('p_spec.c','P_FindNextHighestFloor','fixed_t'),('p_spec.c','P_FindLowestCeilingSurrounding','fixed_t'),
       ('p_spec.c','P_FindHighestCeilingSurrounding','fixed_t'),('p_spec.c','P_FindSectorFromLineTag','int'),
       ('p_map.c','P_ThingHeightClip','boolean'),('p_map.c','PIT_ChangeSector','boolean'),('p_map.c','P_ChangeSector','boolean'),
       ('p_floor.c','T_MovePlane','result_e'),('p_floor.c','T_MoveFloor','void'),('p_floor.c','EV_DoFloor','int'),
       ('p_doors.c','T_VerticalDoor','void'),('p_doors.c','EV_DoDoor','int'),('p_doors.c','EV_VerticalDoor','void'),
       ('p_plats.c','T_PlatRaise','void'),('p_plats.c','EV_DoPlat','int'),('p_plats.c','P_AddActivePlat','void'),('p_plats.c','P_RemoveActivePlat','void'),
       ('g_game.c','G_ExitLevel','void'),
       ('p_switch.c','P_StartButton','void'),('p_switch.c','P_ChangeSwitchTexture','void'),('p_switch.c','P_UseSpecialLine','boolean'),
       ('p_spec.c','P_CrossSpecialLine','void'),
       ('p_map.c','PTR_UseTraverse','boolean'),('p_map.c','P_UseLines','void')]
core.ADDED.extend(ADDED)
# Bodies wrapped by an observer (the genuine body is compiled as Raw_<name>).
WRAPPED={'P_ThingHeightClip','PIT_ChangeSector','P_ChangeSector','T_MovePlane','EV_DoFloor','EV_DoDoor','EV_VerticalDoor','EV_DoPlat',
         'G_ExitLevel','P_UseSpecialLine','P_CrossSpecialLine','PTR_UseTraverse','P_UseLines'}
inherited_body=body_stage._original_body
def definition_body(file,name,result,rename=None):
    """The generic extractor matches the first `name (...) {` text, which for several
    p_spec.c helpers is their `// name()` comment header. These bodies are extracted
    from their definition instead: the first `name (` outside a `//` comment whose
    balanced parameter list is followed by `{`. Every other body is unchanged."""
    if name not in {n for _,n,_ in ADDED}:return inherited_body(file,name,result,rename)
    text=(ROOT/'linuxdoom-1.10'/file).read_text()
    for match in re.finditer(r'\b'+name+r'\s*\(',text):
        line=text[text.rfind('\n',0,match.start())+1:match.start()]
        if '//' in line:continue
        depth,i=1,match.end()
        while depth:
            depth+=(text[i]=='(')-(text[i]==')');i+=1
        rest=text[i:].lstrip()
        if not rest.startswith('{'):continue
        start=match.start();end=text.index('{',i)+1;level=1
        while level:
            level+=(text[end]=='{')-(text[end]=='}');end+=1
        return result+' '+text[start:end].replace(name,rename or name,1)+'\n'
    raise ValueError(name)
body_stage._original_body=definition_body
# Declared adapters and observer helpers, placed after the last header the shim includes.
HELPERS=re.search(r'^#define\s+MAX_ADJOINING_SECTORS[^\n]+',(ROOT/'linuxdoom-1.10/p_spec.c').read_text(),re.M).group(0)+r'''
gameaction_t gameaction;boolean secretexit;
int switchlist[MAXSWITCHES*2];int numswitches;
mobj_t* usething;boolean crushchange,nofit;
fixed_t* textureheight;
int EV_DoCeiling(line_t* l,ceiling_e t){(void)l;(void)t;abort();return 0;}
int EV_CeilingCrushStop(line_t* l){(void)l;abort();return 0;}
int EV_Teleport(line_t* l,int s,mobj_t* t){(void)l;(void)s;(void)t;abort();return 0;}
void EV_LightTurnOn(line_t* l,int b){(void)l;(void)b;abort();}
void EV_StartLightStrobing(line_t* l){(void)l;abort();}
void EV_TurnTagLightsOff(line_t* l){(void)l;abort();}
void EV_StopPlat(line_t* l){(void)l;abort();}
int EV_BuildStairs(line_t* l,stair_e t){(void)l;(void)t;abort();return 0;}
int EV_DoDonut(line_t* l){(void)l;abort();return 0;}
int EV_DoLockedDoor(line_t* l,vldoor_e t,mobj_t* m){(void)l;(void)t;(void)m;abort();return 0;}
void G_SecretExitLevel(void){abort();}
void P_ActivateInStasis(int tag){(void)tag;abort();}
void G_ExitLevel(void);boolean P_ThingHeightClip(mobj_t*);boolean PIT_ChangeSector(mobj_t*);boolean PTR_UseTraverse(intercept_t*);
result_e Raw_T_MovePlane(sector_t*,fixed_t,fixed_t,boolean,int,int);
static int mover_kind(thinker_t* t){
 if(t->function.acp1==(actionf_p1)T_VerticalDoor)return 5;if(t->function.acp1==(actionf_p1)T_PlatRaise)return 6;
 if(t->function.acp1==(actionf_p1)T_MoveFloor)return 7;return 0;}
static int specialdata_id(sector_t* s){return s->specialdata?special_id(s->specialdata):-1;}
static int clip_depth;
static void mover_created(int before){int k;thinker_t* t;
 for(k=before;k<special_count;k++){int kind=mover_kind((thinker_t*)special_store[k]),entries=0;sector_t* s;
  if(kind==5)s=((vldoor_t*)special_store[k])->sector;else if(kind==6)s=((plat_t*)special_store[k])->sector;else if(kind==7)s=((floormove_t*)special_store[k])->sector;else abort();
  special_sector[k]=(int)(s-sectors);
  for(t=thinkercap.next;t!=&thinkercap;t=t->next){int id=special_find(t);if(id>=0&&special_sector[id]==special_sector[k])entries++;}
  printf("MM,%d,%d,%d,%d,%d,%d\n",scenario,operation,k,kind,special_sector[k],entries);}}
'''
# The soundorg/blockbox fragment of P_GroupLines, over the bbox the genuine M_ClearBox and
# M_AddToBox build from every line with the sector on either side, in line order (the
# membership and order of the genuine linebuffer loop).
group=(ROOT/'linuxdoom-1.10/p_setup.c').read_text()
GROUP=group[group.index('\t// set the degenmobj_t to the middle of the bounding box'):group.index('\tsector->blockbox[BOXLEFT]=block;')+len('\tsector->blockbox[BOXLEFT]=block;')]
user=(ROOT/'linuxdoom-1.10/p_user.c').read_text()
USE=user[user.index('    // check for use'):user.index('    // cycle psprites')]
DEATH_USE=user[user.index('    if (player->cmd.buttons & BT_USE)\n\tplayer->playerstate = PST_REBORN;'):]
DEATH_USE=DEATH_USE[:DEATH_USE.index('PST_REBORN;')+len('PST_REBORN;')]
assert 'P_UseLines' in USE and USE.count('\n')<14
WORLD=r'''
static void mover_world(void){int i,j;sector_t* sector;line_t* li;fixed_t bbox[4];int block;
 for(i=0,sector=sectors;i<numsectors;i++,sector++){M_ClearBox(bbox);
  for(j=0,li=lines;j<numlines;j++,li++)if(li->frontsector==sector||li->backsector==sector){M_AddToBox(bbox,li->v1->x,li->v1->y);M_AddToBox(bbox,li->v2->x,li->v2->y);}
'''+GROUP+r'''
 }
 switchlist[0]=SWITCH_OFF;switchlist[1]=SWITCH_ON;switchlist[2]=-1;numswitches=1;
 gameaction=ga_nothing;secretexit=false;usething=NULL;
}
static void source_use(player_t* player){ticcmd_t* cmd=&player->cmd;
'''+USE+r'''}
static void source_death_use(player_t* player){
'''+DEATH_USE+r'''
}
'''
CAPTURE=r'''
static void movers_capture(void){int i;
 printf("MH,%d,%d,%d",scenario,operation,numsectors);for(i=0;i<numsectors;i++)printf(",%d,%d,%d",sectors[i].floorheight,sectors[i].ceilingheight,specialdata_id(&sectors[i]));printf("\n");
 printf("ML,%d,%d",scenario,operation);for(i=0;i<numlines;i++)if(map_special(i))printf(",%d,%d",i,lines[i].special);printf("\n");
 printf("MZ,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,players[0].usedown,players[0].playerstate,gameaction,secretexit,players[0].cmd.buttons);
 for(i=0;i<MAXPLATS;i++)if(activeplats[i])printf("MQ,%d,%d,%d,%d\n",scenario,operation,i,special_id(activeplats[i]));
 for(i=0;i<MAXBUTTONS;i++)if(buttonlist[i].btimer||buttonlist[i].line||buttonlist[i].soundorg)printf("MB,%d,%d,%d,%ld,%d,%d,%d,%d\n",scenario,operation,i,buttonlist[i].line?(long)(buttonlist[i].line-lines):-1L,buttonlist[i].where,buttonlist[i].btexture,buttonlist[i].btimer,sound_origin_sector(buttonlist[i].soundorg));
 for(i=0;i<numlines;i++)if(map_special(i)&&(sides[lines[i].sidenum[0]].toptexture||sides[lines[i].sidenum[0]].midtexture||sides[lines[i].sidenum[0]].bottomtexture))printf("MW,%d,%d,%d,%d,%d,%d\n",scenario,operation,i,sides[lines[i].sidenum[0]].toptexture,sides[lines[i].sidenum[0]].midtexture,sides[lines[i].sidenum[0]].bottomtexture);}
'''
# The inherited thinker capture prints an actor ident for every non-light entry; mover
# entries print their fields instead, and a removed sector thinker prints kind 8.
TK_OLD=' if(k==0||k==9)printf("TK,%d,%d,%d,%d,%d,%d\\n",scenario,operation,i,k,ident(t),((mobj_t*)t)->type);'
TK_NEW=(' if(k==9&&special_find(t)>=0)printf("TK,%d,%d,%d,8,%d\\n",scenario,operation,i,special_id(t));\n'
 ' else if(k==0&&mover_kind(t)==5){vldoor_t* d=(vldoor_t*)t;printf("TK,%d,%d,%d,5,%d,%ld,%d,%d,%d,%d,%d,%d\\n",scenario,operation,i,special_id(t),(long)(d->sector-sectors),d->type,d->direction,d->topheight,d->topcountdown,d->speed,d->topwait);}\n'
 ' else if(k==0&&mover_kind(t)==6){plat_t* p=(plat_t*)t;printf("TK,%d,%d,%d,6,%d,%ld,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\\n",scenario,operation,i,special_id(t),(long)(p->sector-sectors),p->type,p->status,p->count,p->low,p->high,p->speed,p->wait,p->crush,p->tag,p->oldstatus);}\n'
 ' else if(k==0&&mover_kind(t)==7){floormove_t* f=(floormove_t*)t;printf("TK,%d,%d,%d,7,%d,%ld,%d,%d,%d,%d,%d,%d,%d\\n",scenario,operation,i,special_id(t),(long)(f->sector-sectors),f->type,f->direction,f->floordestheight,f->speed,f->crush,f->newspecial,f->texture);}\n'
 +' else'+TK_OLD)
OBSERVERS=r'''
static int sound_origin_sector(void* p){int i;for(i=0;i<numsectors;i++)if(p==(void*)&sectors[i].soundorg)return i;return -1;}
static void source_sound(void* p,int sound){int i;
 if(!p){observe(7,-1,sound);return;}
 for(i=0;i<numsectors;i++)if(p==(void*)&sectors[i].soundorg){printf("SO,%d,%d,1,%d,%d\n",scenario,operation,i,sound);return;}
 for(i=0;i<MAXBUTTONS;i++)if(p==(void*)&buttonlist[i].soundorg){printf("SO,%d,%d,2,%d,%d\n",scenario,operation,i,sound);return;}
 observe(7,ident(p),sound);}
'''
WRAPPERS=r'''
boolean P_ThingHeightClip(mobj_t* t){fixed_t z=t->z;boolean r;clip_depth++;r=Raw_P_ThingHeightClip(t);clip_depth--;
 printf("HC,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,ident(t),r,z,t->z,t->floorz,t->ceilingz,t->height,numspechit);return r;}
boolean PIT_ChangeSector(mobj_t* t){int id=ident(t),health=t->health,flags=t->flags;boolean r;int code;
 r=Raw_PIT_ChangeSector(t);
 code=last_clip_ok?0:health<=0?1:(flags&MF_DROPPED)?2:!(flags&MF_SHOOTABLE)?3:4;
 printf("MT,%d,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,id,code,t->z,t->height,t->radius,nofit);return r;}
boolean P_ChangeSector(sector_t* s,boolean crunch){boolean r;printf("MC,%d,%d,0,%ld,%d,%d,%d\n",scenario,operation,(long)(s-sectors),crunch,s->floorheight,s->ceilingheight);
 r=Raw_P_ChangeSector(s,crunch);printf("MC,%d,%d,1,%ld,%d,%d,%d\n",scenario,operation,(long)(s-sectors),r,s->floorheight,s->ceilingheight);return r;}
result_e T_MovePlane(sector_t* s,fixed_t speed,fixed_t dest,boolean crush,int plane,int direction){fixed_t before=plane?s->ceilingheight:s->floorheight;result_e r;
 r=Raw_T_MovePlane(s,speed,dest,crush,plane,direction);
 printf("MV,%d,%d,%ld,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,(long)(s-sectors),plane,direction,speed,dest,before,plane?s->ceilingheight:s->floorheight,r);return r;}
int EV_DoFloor(line_t* l,floor_e type){int before=special_count,r=Raw_EV_DoFloor(l,type);mover_created(before);printf("ME,%d,%d,4,%ld,%d,-1,%d,%d,%d\n",scenario,operation,(long)(l-lines),type,r,before,special_count);return r;}
int EV_DoDoor(line_t* l,vldoor_e type){int before=special_count,r=Raw_EV_DoDoor(l,type);mover_created(before);printf("ME,%d,%d,1,%ld,%d,-1,%d,%d,%d\n",scenario,operation,(long)(l-lines),type,r,before,special_count);return r;}
void EV_VerticalDoor(line_t* l,mobj_t* thing){int before=special_count;Raw_EV_VerticalDoor(l,thing);mover_created(before);printf("ME,%d,%d,2,%ld,%d,%d,-1,%d,%d\n",scenario,operation,(long)(l-lines),l->special,ident(thing),before,special_count);}
int EV_DoPlat(line_t* l,plattype_e type,int amount){int before=special_count,r=Raw_EV_DoPlat(l,type,amount);mover_created(before);printf("ME,%d,%d,3,%ld,%d,-1,%d,%d,%d\n",scenario,operation,(long)(l-lines),type,r,before,special_count);return r;}
void G_ExitLevel(void){Raw_G_ExitLevel();printf("MG,%d,%d,0,%d,%d\n",scenario,operation,gameaction,secretexit);}
boolean P_UseSpecialLine(mobj_t* thing,line_t* l,int side){int special=l->special;boolean r=Raw_P_UseSpecialLine(thing,l,side);
 printf("MU,%d,%d,%d,%ld,%d,%d,%d,%d\n",scenario,operation,ident(thing),(long)(l-lines),side,special,r,l->special);return r;}
void P_CrossSpecialLine(int linenum,int side,mobj_t* thing){int special=lines[linenum].special;
 printf("MX,%d,%d,%d,%d,%d,%d\n",scenario,operation,ident(thing),linenum,side,special);Raw_P_CrossSpecialLine(linenum,side,thing);
 printf("MY,%d,%d,%d,%d,%d\n",scenario,operation,ident(thing),linenum,lines[linenum].special);}
boolean PTR_UseTraverse(intercept_t* in){int special=in->d.line->special;boolean r=Raw_PTR_UseTraverse(in);
 printf("MI,%d,%d,%ld,%d,%d,%d,%d\n",scenario,operation,(long)(in->d.line-lines),in->frac,special,special?-1:openrange,r);return r;}
void P_UseLines(player_t* player){int saved=trace_stage;printf("MA,%d,%d,%d,%u,%d,%d\n",scenario,operation,ident(player->mo),player->mo->angle,player->mo->x,player->mo->y);
 trace_stage=9;Raw_P_UseLines(player);trace_stage=saved;printf("MN,%d,%d\n",scenario,operation);}
'''
inherited_observed=core.observed_body
def observed(file,name,result,rename=None):
    if name in WRAPPED:
        assert rename is None
        text=inherited_observed(file,name,result,'Raw_'+name)
    else:
        text=inherited_observed(file,name,result,rename)
    if name=='PIT_CheckLine':
        text=replace_once(text,'    if (ld->special)\n    {\n\tspechit[numspechit] = ld;','    if (ld->special)\n    {\n\tprintf("MK,%d,%d,%d,%ld,%d,%d\\n",scenario,operation,ident(tmthing),(long)(ld-lines),numspechit,clip_depth);\n\tspechit[numspechit] = ld;')
    if name=='P_Move':
        text=replace_once(text,'\tactor->movedir = DI_NODIR;\n\tgood = false;','\tprintf("MP,%d,%d,%d,%d,%d\\n",scenario,operation,ident(actor),actor->movedir,numspechit);\n\tactor->movedir = DI_NODIR;\n\tgood = false;')
    if name=='P_ThingHeightClip':
        text=replace_once(text,'    if (thing->ceilingz - thing->floorz < thing->height)\n\treturn false;','    if (thing->ceilingz - thing->floorz < thing->height)\n\t{last_clip_ok=0;return false;}\n    last_clip_ok=1;')
    if name in('T_MovePlane','T_VerticalDoor','T_PlatRaise','T_MoveFloor','EV_DoDoor','EV_VerticalDoor','EV_DoFloor','P_ChangeSector','P_ThingHeightClip','P_UseSpecialLine','P_CrossSpecialLine','P_UseLines','PTR_UseTraverse','P_ChangeSwitchTexture','P_StartButton'):
        assert 'P_Random' not in text,name
    return text
core.observed_body=observed
# Driver: the declared mover world after the inherited specials world (P_SpawnSpecials,
# compiled since M3h7, clears activeplats and buttonlist), BT_USE windows, a second turn
# and forward window, the use fragment after the weapon change, the dead player's
# BT_USE statement after P_MovePsprites, and the end of the case at a requested
# G_DoCompleted or G_DoReborn (the next G_Ticker; not executed).
core.DRIVER=replace_once(core.DRIVER,'specials_world(v[V_MAP]);','specials_world(v[V_MAP]);mover_world();switch_world(v[V_MAP]);')
dead='if(p->playerstate==PST_DEAD){p->cmd.forwardmove=p->cmd.sidemove=p->cmd.angleturn=p->cmd.buttons=0;}'
core.DRIVER=replace_once(core.DRIVER,dead,
  'if(t==v[V_PC_TURN2_AT])p->cmd.angleturn=v[V_PC_TURN2];\n'
  'if(t>=v[V_PC_MOVE2_START]&&t<=v[V_PC_MOVE2_HOLD])p->cmd.forwardmove=v[V_PC_FORWARD2];\n'
  'if(t>=v[V_PC_MOVE3_START]&&t<=v[V_PC_MOVE3_HOLD])p->cmd.forwardmove=v[V_PC_FORWARD3];\n'
  'if((t>=v[V_PC_USE_START]&&t<=v[V_PC_USE_END])||(t>=v[V_PC_USE2_START]&&t<=v[V_PC_USE2_END])||(t>=v[V_PC_USE3_START]&&t<=v[V_PC_USE3_END]))p->cmd.buttons|=BT_USE;\n'
  'if(p->playerstate==PST_DEAD){p->cmd.forwardmove=p->cmd.sidemove=p->cmd.angleturn=0;p->cmd.buttons&=BT_USE;}')
core.DRIVER=replace_once(core.DRIVER,'source_weapon_selection(p);}','source_weapon_selection(p);source_use(p);}')
core.DRIVER=replace_once(core.DRIVER,'P_MovePsprites(p);if(p->playerstate==PST_LIVE&&p->bonuscount)p->bonuscount--;','P_MovePsprites(p);if(p->playerstate==PST_DEAD)source_death_use(p);if(p->playerstate==PST_LIVE&&p->bonuscount)p->bonuscount--;')
assert core.DRIVER.count('specials_capture();')==2
core.DRIVER=core.DRIVER.replace('specials_capture();','specials_capture();movers_capture();')
tail='armor_capture();specials_capture();movers_capture();\n}}'
core.DRIVER=replace_once(core.DRIVER,tail,'armor_capture();specials_capture();movers_capture();\nif(gameaction==ga_completed||p->playerstate==PST_REBORN){printf("MG,%d,%d,%d,%d\\n",scenario,operation,gameaction==ga_completed?1:2,p->playerstate);break;}\n}}')
def switch_source():
    """switch_world(which): every side's top, bottom and middle texture numbers resolved
    from the names in the map's SIDEDEFS bytes by the declared synthetic table (the
    R_TextureNumForName stand-in: `-` is 0, an unknown name aborts), and the per-map list
    of lines with a nonzero map special (ML rows)."""
    out=['#include <strings.h>','static int map_special_table[4096];static int map_special(int i){return map_special_table[i];}',
         'static const char synthetic_textures[%d][9]={%s};'%(len(geometry.TEXTURES),','.join(f'"{n}"'for n in geometry.TEXTURES)),
         'static int synthetic_texture_num(const char* name){int i;if(name[0]==\'-\')return 0;for(i=0;i<%d;i++)if(!strncasecmp(synthetic_textures[i],name,8))return i;abort();return -1;}'%len(geometry.TEXTURES)]
    for which,m in enumerate(core.main.MAPS):
        names=geometry.side_names(m)
        out.append(f'static const char side_names_{which}[{len(names)}][3][9]={{'+','.join('{'+','.join(f'"{n}"'for n in s)+'}'for s in names)+'};')
    out.append('static void switch_world(int which){int i;const char (*names)[3][9];int count;for(i=0;i<numlines;i++)map_special_table[i]=lines[i].special!=0;switch(which){')
    for which,m in enumerate(core.main.MAPS):
        out.append(f'case {which}:names=side_names_{which};count={len(geometry.side_names(m))};break;')
    out.append('default:abort();}')
    out.append(' for(i=0;i<count;i++){sides[i].toptexture=synthetic_texture_num(names[i][0]);sides[i].bottomtexture=synthetic_texture_num(names[i][1]);sides[i].midtexture=synthetic_texture_num(names[i][2]);}}')
    return '\n'.join(out)+'\n'
def configure():
    core.main.MAPS,core.main.THINGS,core.main.KEYS,core.main.CASES=[],[],[],[]
    for c in core.CASES:
        sectors,portals,reject,tree,segment,lines=ROOMS[c['room']]
        m=geometry.room(sectors,portals=portals,reject=reject,tree=tree,segment=segment,lines=lines)
        core.main.case(c['label'],n=1,custom=m,things=LAYOUTS[c['layout']],tics=c['tics'],rejected=bool(c['rejected']),
                       **{'pc_'+k:v for k,v in c.items()if k not in['label','tics','map']})
        c['map']=core.main.CASES[-1]['map']
    core.CAPTURE=inherited.BASE_CAPTURE+inherited.world_source()+inherited.CAPTURE+switch_source()+CAPTURE
core.configure=configure
inherited_run=pickup_stage.real_run
STUBS=['void G_ExitLevel(void){abort();}','int EV_DoFloor(line_t* l,floor_e f){(void)l;(void)f;abort();return 0;}',
       'int EV_DoDoor(line_t* l,vldoor_e d){(void)l;(void)d;abort();return 0;}','int EV_DoPlat(line_t* l,plattype_e p,int a){(void)l;(void)p;(void)a;abort();return 0;}',
       'void P_ChangeSwitchTexture(line_t* l,int a){(void)l;(void)a;abort();}','void P_CrossSpecialLine(int i,int s,mobj_t* p){(void)i;(void)s;(void)p;abort();}',
       'boolean P_UseSpecialLine(mobj_t* a,line_t* l,int side){(void)a;(void)l;(void)side;abort();return false;}']
def compile_movers(args,*a,**kw):
    """Source-level harness edits on the generated reference.c: the aborting stubs of the
    bodies now compiled are removed; declared adapters, observers and wrappers are
    inserted; Z_Malloc's PU_LEVSPEC store grows and Z_Free quarantines sector thinkers
    (as it quarantines mobjs); S_StartSound records a sector or button origin."""
    if args and args[0]=='cc':
        for x in args:
            if str(x).endswith('reference.c'):
                path=Path(x);text=path.read_text()
                for stub in STUBS:text=replace_once(text,stub,'')
                text=replace_once(text,'#include "d_event.h"\n','#include "d_event.h"\n'+f'#define SWITCH_OFF {geometry.SWITCH_TEXTURES[0]}\n#define SWITCH_ON {geometry.SWITCH_TEXTURES[1]}\nstatic int last_clip_ok;\n'+HELPERS)
                text=replace_once(text,'static void* special_store[64];','static void* special_store[1024];static int special_freed[1024],special_sector[1024];')
                text=replace_once(text,'if(special_count==64)abort();p=calloc(1,size);special_store[special_count++]=p;','if(special_count==1024)abort();p=calloc(1,size);special_freed[special_count]=0;special_sector[special_count]=-1;special_store[special_count++]=p;')
                text=replace_once(text,'static int special_id(void* p){','static int special_find(void* p){int i;for(i=0;i<special_count;i++)if(special_store[i]==p)return i;return -1;}\nstatic int special_id(void* p){')
                text=replace_once(text,'void Z_Free(void* p){if(p==thing_buffer.bytes)return;','void Z_Free(void* p){if(p==thing_buffer.bytes)return;if(special_find(p)>=0){if(special_freed[special_find(p)])abort();special_freed[special_find(p)]=1;printf("SF,%d,%d,%d\\n",scenario,operation,special_find(p));return;}')
                text=replace_once(text,'void S_StartSound(void* p,int sound){observe(7,p?ident(p):-1,sound);}','static void source_sound(void*,int);void S_StartSound(void* p,int sound){source_sound(p,sound);}')
                text=replace_once(text,'static void run_observe(thinker_t* t){int k=light_kind(t);if(k)','static int mover_kind(thinker_t*);static void run_observe(thinker_t* t){int k=light_kind(t);if(!k)k=mover_kind(t);if(k)')
                text=replace_once(text,TK_OLD,TK_NEW)
                text=replace_once(text,'  observe(2,ident(thinker),0);','  if(special_find(thinker)>=0)printf("SD,%d,%d,%d\\n",scenario,operation,special_find(thinker));else observe(2,ident(thinker),0);')
                anchor='static void capture(void){'
                text=replace_once(text,anchor,OBSERVERS+WORLD+WRAPPERS+anchor)
                path.write_text(text)
    return inherited_run(args,*a,**kw)
pickup_stage.real_run=compile_movers
sha=lambda b:hashlib.sha256(b).hexdigest()
def capture(flags):
    observed_trace,_=previous.capture(flags)
    return observed_trace
def special_window(m):
    """Rule 6 of the M3h8 note (§6): the largest number of distinct lines with a back
    side and a nonzero special listed in one 2 x 2 window of BLOCKMAP cells (a cell's
    leading 0 lists line 0, as P_BlockLinesIterator reads it)."""
    blocks=m['blocks'];width,height=blocks[2],blocks[3]
    special={i for i,(l,(sp,_))in enumerate(zip(m['lines'],m['line_specials']))if sp and l['back']is not None}
    def listed(cell):
        at=blocks[4+cell];out=set()
        while blocks[at]!=-1:out.add(blocks[at]);at+=1
        return out&special
    cells=[listed(c)for c in range(width*height)];best=0
    for cy in range(max(height-1,1)):
        for cx in range(max(width-1,1)):
            distinct=set()
            for wy in(cy,cy+1):
                for wx in(cx,cx+1):
                    if wx<width and wy<height:distinct|=cells[wy*width+wx]
            best=max(best,len(distinct))
    return best
ROW_FORMATS={
 'MH':'MH,case,op,numsectors,(floorheight,ceilingheight,specialdata) for every sector in index order after each op; heights raw 16.16, specialdata = the sector-thinker store index of sector->specialdata or -1',
 'ML':'ML,case,op,(line,special)... for every line whose map special is nonzero, after each op (W1/S1 lines read 0 once cleared)',
 'MZ':'MZ,case,op,usedown,playerstate,gameaction,secretexit,cmd.buttons of players[0] after each op (gameaction 6 = ga_completed)',
 'MQ':'MQ,case,op,slot,id: a nonnull activeplats slot after the op (id = sector-thinker store index)',
 'MB':'MB,case,op,slot,line,where,btexture,btimer,soundorg sector (-1 none): a buttonlist slot with a timer, line or sound origin after the op; presentation',
 'MW':'MW,case,op,line,toptexture,midtexture,bottomtexture: the front side textures of a special line when one is nonzero (declared switch texture numbers); presentation',
 'TK':'as M3h7, plus kind 5 door: id,sector,type,direction,topheight,topcountdown,speed,topwait; kind 6 plat: id,sector,type,status,count,low,high,speed,wait,crush,tag,oldstatus; kind 7 floor: id,sector,type,direction,floordestheight,speed,crush,newspecial,texture; kind 8: id, a sector thinker removed by P_RemoveThinker and not yet unlinked. Mover allocations are zero-filled (declared adapter), so never-written fields print 0',
 'LE':'as M3h7: LE,case,op,kind,id when P_RunThinkers runs a sector thinker, now also kinds 5 door, 6 plat, 7 floor',
 'SD':'SD,case,op,id: P_RemoveThinker marks sector thinker id (the E code 2 row stays for actors)',
 'SF':'SF,case,op,id: P_RunThinkers unlinks sector thinker id and calls Z_Free (the harness quarantines it; ids are never reused)',
 'SO':'SO,case,op,kind,index,sound: S_StartSound with a sector origin (kind 1: &sectors[index].soundorg) or with the address of buttonlist[index].soundorg (kind 2: P_UpdateSpecials passes the address of the field); a NULL origin (global) or a mobj origin keeps the inherited E,case,op,7,actor|-1,sound row',
 'MK':'MK,case,op,thing,line,index,inclip: PIT_CheckLine is about to store line at spechit[index] for tmthing; inclip 1 inside P_ThingHeightClip. spechit keeps its source size (8)',
 'MP':'MP,case,op,actor,movedir,numspechit: the P_Move failure branch with numspechit > 0, before movedir = DI_NODIR and the reverse P_UseSpecialLine loop',
 'MU':'MU,case,op,thing,line,side,special before,result,special after: P_UseSpecialLine (from PTR_UseTraverse or the P_Move failure branch)',
 'MX':'MX,case,op,thing,line,side,special: P_CrossSpecialLine entry from the P_TryMove crossing loop (side = the old side)',
 'MY':'MY,case,op,thing,line,special: P_CrossSpecialLine exit (0 once a W1 line is cleared)',
 'MI':'MI,case,op,line,frac,special,openrange (-1 when special),result: PTR_UseTraverse visited a line intercept (result 0 stops the trace)',
 'MA':'MA,case,op,player,angle,x,y: P_UseLines entry; its P_PathTraverse rows (TR traverser 3, D/B/U/I/TI) carry trace stage 9',
 'MN':'MN,case,op: P_UseLines exit',
 'ME':'ME,case,op,fn,line,arg,thing,result,store count before,store count after: fn 1 EV_DoDoor(arg = vldoor_e), 2 EV_VerticalDoor(arg = line special; thing = user; result -1), 3 EV_DoPlat(arg = plattype_e), 4 EV_DoFloor(arg = floor_e); a mover was created when the store count grew',
 'MV':'MV,case,op,sector,plane,direction,speed,dest,before,after,result: T_MovePlane (plane 0 floor, 1 ceiling; result 0 ok, 1 crushed, 2 pastdest; heights raw)',
 'MM':'MM,case,op,id,kind,sector,entries: a mover just created by an EV_* body (kind 5 door, 6 plat, 7 floor) and the number of thinker-list entries of that sector at that moment, including a removed entry not yet unlinked (M3h8 note §2.1, §8)',
 'MC':'MC,case,op,0,sector,crunch,floorheight,ceilingheight at P_ChangeSector entry; MC,case,op,1,sector,nofit,floorheight,ceilingheight at its exit',
 'HC':'HC,case,op,thing,result,z before,z,floorz,ceilingz,height,numspechit: P_ThingHeightClip (its P_CheckPosition result is ignored by the source)',
 'MT':'MT,case,op,thing,code,z,height,radius,nofit: PIT_ChangeSector outcome after the clip: 0 fits, 1 health <= 0 -> S_GIBS (height and radius 0), 2 MF_DROPPED removed, 3 not shootable left as is, 4 nofit = true',
 'MG':'MG,case,op,0,gameaction,secretexit at G_ExitLevel; MG,case,op,code,playerstate at the end of the op when G_DoCompleted (code 1) or G_DoReborn (code 2) would run at the next G_Ticker: the case ends there',
 'E':'existing; E,case,op,7,actor|-1,sound now also records sfx_noway at the player and the global switch sound of P_ChangeSwitchTexture when buttonlist[0] has no origin'}
def run(check,overflow,address):
    flags=list(previous.pack.legacy.base.FLAGS)
    observed_trace=capture(flags)
    audits={}
    for enabled,sanitizer in [(overflow,'signed-integer-overflow,integer-divide-by-zero,float-cast-overflow'),(address,'address,undefined')]:
        if enabled:
            audited=capture([f for f in flags if f!='-fwrapv']+['-fno-wrapv','-fsanitize='+sanitizer,'-fno-sanitize-recover=all'])
            assert audited['m3d3j_trace.csv']==observed_trace['m3d3j_trace.csv']
            audits[sanitizer]=sha(audited['m3d3j_trace.csv'])
    rows=list(csv.reader(io.StringIO(observed_trace['m3d3j_trace.csv'].decode())))
    functions=collections.Counter(r[3]for r in rows if r[0]=='FN')
    full=[r for r in rows if r[0]!='FN']
    case_rows=collections.defaultdict(list)
    for r in full:case_rows[int(r[1])].append(r)
    last_op={w:max(int(r[2])for r in mine)for w,mine in case_rows.items()}
    ended={int(r[1]):int(r[3])for r in full if r[0]=='MG'and r[3]in('1','2')}
    for which,c in enumerate(core.CASES):assert last_op[which]==c['tics']or which in ended,(c['label'],last_op[which])
    selected={0,1,2,4,8,16,32,48,64,96,128,192,256,320,384,448,512,576}
    pushes={(i,c['push_at'])for i,c in enumerate(core.CASES)if c['push_at']}
    mover={'ME','MU','MX','MG','SO','MP','SD'}
    interesting={(int(r[1]),int(r[2]))for r in full if (r[0]=='G'and r[3]in{'2','7','10'})or r[0]in{'AR','ST','PN','SL','X','N','SE','KY','KR','KS','KZ'}|mover
                 or(r[0]=='MS'and r[3]in{'0','5'})or(r[0]=='E'and r[3]=='1'and r[2]!='0')or(r[0]=='M'and r[7]=='0')or(r[0]=='MT'and r[4]!='0')or(r[0]=='MV'and r[10]!='0')}|pushes
    keys={(int(r[1]),int(r[2]))for r in rows if r[0]=='P'and(int(r[2])in selected or tuple(map(int,r[1:3]))in interesting or int(r[2])==last_op[int(r[1])])}
    kept=[r for r in rows if r[0]!='FN'and(r[0]not in{'A','P','AP','R','S','LS'}or tuple(map(int,r[1:3]))in keys)]
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(kept)
    expanded=(SCHEMA+'\n'+text.getvalue()).encode()
    outputs={'m3h8_trace.dref':pack(expanded)}
    assert unpack(outputs['m3h8_trace.dref'])==expanded
    text=io.StringIO();writer=csv.writer(text,lineterminator='\n');writer.writerow(['case',*core.CASES[0]]);writer.writerows([i,*c.values()]for i,c in enumerate(core.CASES))
    outputs['m3h8_cases.csv']=text.getvalue().encode()
    maps=[]
    for i,m in enumerate(core.main.MAPS):
        name=f'm3h8_geometry_{i}.bin';outputs[name]=m['bytes'];maps.append('tests/reference/'+name)
    outputs['m3h8_maps.rs']=('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("{Path(p).name}"),\n'for p in maps)+'];\n').encode()
    # Rust-only: the nine-line lane of the SpecialCross witness, built by the same builder
    # but never run in C, where its ninth spechit write would overflow the array.
    dense9=geometry.room(DENSE9[0],portals=DENSE9[1],reject=DENSE9[2],tree=DENSE9[3],segment=DENSE9[4],lines=DENSE9[5])
    outputs['m3h8_dense9.bin']=dense9['bytes']
    manifest=json.loads(observed_trace['m3d3j_provenance.json'])
    bodies=manifest['functions']+[['p_pspr.c',n,'void']for n in previous.pack.WEAPON_ACTIONS]
    assert len(bodies)==len({tuple(b)for b in bodies})
    # ---- observations, all computed from the full trace -------------------------------
    player_ident={w:next(r[5]for r in mine if r[0]=='W'and r[2]=='0'and r[3]=='7')for w,mine in case_rows.items()}
    mk=[r for r in full if r[0]=='MK'];assert all(0<=int(r[5])<8 for r in mk)
    spechit_max=1+max(int(r[5])for r in mk)
    births={(r[1],r[4]):int(r[5])for r in rows if r[0]=='E'and r[3]=='1'}
    types={}
    for r in rows:
        if r[0]=='A':types.setdefault((r[1],r[3]),int(r[5]))
    kind=lambda w,i:births.get((w,i),types.get((w,i)))
    def actor_kind(r,i):
        if r[i]==player_ident[int(r[1])]:return 'player'
        return 'missile'if kind(r[1],r[i])==31 else'monster'
    mv=[r for r in full if r[0]=='MV'];results=('ok','crushed','pastdest')
    steps=collections.Counter(f"{('floor','ceiling')[int(r[4])]}:{'down'if r[5]=='-1'else'up'}:{results[int(r[10])]}"for r in mv)
    stopped_short=[r for r in mv if r[10]=='2'and r[9]!=r[7]]
    jump_up=[r for r in mv if r[4]=='0'and r[5]=='-1'and int(r[9])>int(r[8])]
    me=[r for r in full if r[0]=='ME'];created=collections.Counter(('EV_DoDoor','EV_VerticalDoor','EV_DoPlat','EV_DoFloor')[int(r[3])-1]for r in me if int(r[9])>int(r[8]))
    mt=collections.Counter(('fits','gibs','dropped_removed','left_stuck','nofit')[int(r[4])]for r in full if r[0]=='MT')
    mu=[r for r in full if r[0]=='MU']
    uses=collections.Counter(f"{actor_kind(r,3)}:{'back'if r[5]=='1'else'front'}:{r[6]}:{'true'if r[7]=='1'else'false'}"for r in mu)
    mx=[r for r in full if r[0]=='MX']
    crossings=collections.Counter(f'{actor_kind(r,3)}:{r[6]}'for r in mx)
    cleared=sorted({(int(r[1]),int(r[4]))for r in full if r[0]=='MY'}&{(int(r[1]),int(r[4]))for r in full if r[0]=='MY'and r[5]=='0'})
    noway=[r for r in full if r[0]=='MI'and r[5]=='0'and r[7]=='0']
    sounds=collections.Counter(f'{("","sector","button")[int(r[3])]}:{r[5]}'for r in full if r[0]=='SO')
    nodir=[r for r in full if r[0]=='MP']
    # Pickups inside a clip: a PN collect (code 6, result 3) between an MC entry and exit.
    depth=collections.defaultdict(int);clip_pickups=[]
    for r in full:
        if r[0]=='MC':depth[r[1]]+=1 if r[3]=='0'else -1
        elif r[0]=='PN'and r[3]=='6'and r[5]=='3'and depth[r[1]]:clip_pickups.append(r)
    # Mover entries per sector in the thinker list (old + replacement, M3h8 note §2.1).
    per_sector=collections.Counter()
    for r in full:
        if r[0]=='TK'and r[4]in('5','6','7'):per_sector[(r[1],r[2],r[6])]+=1
    mm=[r for r in full if r[0]=='MM']
    sector_slots=collections.Counter()
    for r in full:
        if r[0]=='TK'and r[4]in('1','2','3','4','5','6','7','8'):sector_slots[(r[1],r[2])]+=1
    # Things risen onto a step at their first box-wide clip: an item HC row whose floorz
    # grew from its spawn value.
    rises=[r for r in full if r[0]=='HC'and int(r[6])>int(r[5])]
    stuck=[r for r in full if r[0]=='HC'and r[4]=='0']
    exits=sum(r[0]=='MG'and r[3]=='1'for r in full);reborns=sum(r[0]=='MG'and r[3]=='2'for r in full)
    per_case=[]
    for which,c in enumerate(core.CASES):
        mine=case_rows[which];m=core.main.MAPS[c['map']];w=str(which)
        player=[r for r in mine if r[0]=='P']
        per_case.append(dict(label=c['label'],room=ROOM_NAMES[c['room']],tics=c['tics'],last_op=last_op[which],ended={1:'exit',2:'reborn'}.get(ended.get(which)),
          map=dict(linedefs=len(m['lines']),sectors=len(m['heights']),subsectors=len(m['leaves']),nodes=len(m['nodes']),cells=m['cells'],
                   line_specials=sorted(sp for sp,_ in m['line_specials']if sp),tagged_sectors=sum(1 for a in m['sector_attrs']if a['tag']),special_window=special_window(m)),
          movers_created=dict(collections.Counter(('EV_DoDoor','EV_VerticalDoor','EV_DoPlat','EV_DoFloor')[int(r[3])-1]for r in mine if r[0]=='ME'and int(r[9])>int(r[8]))),
          steps=dict(collections.Counter(f"{('floor','ceiling')[int(r[4])]}:{results[int(r[10])]}"for r in mine if r[0]=='MV')),
          clip_outcomes=dict(collections.Counter(('fits','gibs','dropped_removed','left_stuck','nofit')[int(r[4])]for r in mine if r[0]=='MT')),
          uses=[[int(r[2]),int(r[4]),int(r[5]),int(r[6]),int(r[7])]for r in mine if r[0]=='MU'and r[3]==player_ident[which]],
          monster_uses=sum(r[0]=='MU'and r[3]!=player_ident[which]for r in mine),nodir_resets=sum(r[0]=='MP'for r in mine),
          crossings=[[int(r[2]),actor_kind(r,3),int(r[4]),int(r[6])]for r in mine if r[0]=='MX'],
          noway=sum(r in noway for r in mine),spechit_max=max([1+int(r[5])for r in mine if r[0]=='MK'],default=0),
          sector_sounds=sum(r[0]=='SO'for r in mine),clip_pickups=sum(r[1]==w for r in clip_pickups),
          gameplay_draws=sum(r[0]=='E'and r[3]=='8'for r in mine),kills=sum(r[0]=='G'and r[3]=='7'for r in mine),
          player_final=dict(health=int(player[-1][4]),playerstate=int(player[-1][5]))if player else None))
    inventory=dict(scenarios=len(core.CASES),scripted_tics=sum(c['tics']for c in core.CASES),executed_tics=sum(last_op.values()),captures=len(keys),rooms=len(ROOMS),maps=len(core.main.MAPS),
      selected_original_bodies=len(bodies),exercised_unique_functions=len(functions),function_entries=sum(functions.values()),
      new_body_entries={f'{f}:{n}':functions[f'{f}:{n}']for f,n,_ in ADDED},
      movers_created=dict(sorted(created.items())),plane_steps=dict(sorted(steps.items())),steps_stopped_short=len(stopped_short),turbo_jump_ups=len(jump_up),
      change_sector_calls=sum(r[0]=='MC'and r[3]=='0'for r in full),height_clips=sum(r[0]=='HC'for r in full),clip_outcomes=dict(sorted(mt.items())),
      clips_raising_z=len(rises),clips_not_fitting=len(stuck),pickups_inside_a_clip=len(clip_pickups),
      uses=dict(sorted(uses.items())),use_traces=sum(r[0]=='MA'for r in full),noway_stops=len(noway),crossings=dict(sorted(crossings.items())),
      lines_cleared=len(cleared),nodir_resets=len(nodir),spechit_entries=len(mk),spechit_max=spechit_max,
      max_mover_entries_per_sector_at_tic_end=max(per_sector.values()),max_mover_entries_per_sector_at_creation=max(int(r[6])for r in mm),creations_beside_a_removed_entry=sum(r[6]=='2'for r in mm),max_sector_thinkers_at_tic_end=max(sector_slots.values()),
      sector_sounds=dict(sorted(sounds.items())),exits=exits,reborns=reborns,
      gameplay_draws=sum(r[0]=='E'and r[3]=='8'for r in full),kills=sum(r[0]=='G'and r[3]=='7'for r in full),
      rust_only_maps=dict(dense9=dict(path='tests/reference/m3h8_dense9.bin',linedefs=len(dense9['lines']),special_window=special_window(dense9))),
      expanded_trace_bytes=len(expanded),packed_trace_bytes=len(outputs['m3h8_trace.dref']))
    inputs=sorted({Path(m.__file__).name for m in list(sys.modules.values()) if getattr(m,'__file__',None) and Path(m.__file__).resolve().parent==ROOT/'tools/reference'})
    sanitizers=dict(raw_trace_sha256=sha(observed_trace['m3d3j_trace.csv']),rule='--overflow-check rebuilds without -fwrapv under -fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow and --address-check under -fsanitize=address,undefined (both -fno-sanitize-recover=all); each run asserts its raw trace equals the strict trace byte for byte, so raw_trace_sha256 is the digest of all three')
    contract=dict(schema=SCHEMA,revision=manifest['revision'],flags=manifest['flags'],inventory=inventory,cases=per_case,maps=maps,functions=bodies,
      row_formats=ROW_FORMATS,flats=geometry.FLATS,skyflatnum=geometry.SKYFLATNUM,switch_textures=list(geometry.SWITCH_TEXTURES),
      source_sha256={n:sha((ROOT/'linuxdoom-1.10'/n).read_bytes())for n in sorted(set(manifest['source_sha256'])|{'d_items.c','dstrings.h','p_spec.c','p_lights.c','p_spec.h','p_user.c','p_setup.c','p_doors.c','p_plats.c','p_floor.c','p_switch.c','p_map.c','g_game.c','m_bbox.c','d_event.h'})},
      function_sha256={f'{f}:{n}':sha(definition_body(f,n,t).encode())for f,n,t in bodies},
      executed_function_entries=dict(sorted(functions.items())),
      inputs_sha256={'tools/reference/'+n:sha((ROOT/'tools/reference'/n).read_bytes())for n in inputs},
      outputs_sha256={n:sha(d)for n,d in outputs.items()},expanded_trace_sha256=sha(expanded),assumptions=[
       f'Genuine player0 plus at most eight map-THING MT_TROOP/MT_POSSESSED/MT_SHOTGUY/MT_CHAINGUY/MT_SERGEANT and at most {MAX_PICKUPS} map-THING pickups in five polygon rooms (doorway with its D1-only variant hall, switchyard, lift at three lift ceilings, floors, dense) built by the M3h6 node builder with declared sector heights and tags and line specials 1, 63, 109, 88, 36, 38 and 11 (two-sided 48 is not built: decision N1 keeps it rejected); no Rust expected observations.',
       'Every compiled body, observer and controlled input of M3h2/M3h5/M3h6/M3h7 is reused unchanged. Added genuine bodies: M_ClearBox, M_AddToBox (m_bbox.c); twoSided, getSide, getSector, P_FindLowestFloorSurrounding, P_FindHighestFloorSurrounding, P_FindNextHighestFloor, P_FindLowestCeilingSurrounding, P_FindHighestCeilingSurrounding, P_FindSectorFromLineTag, P_CrossSpecialLine (p_spec.c); P_ThingHeightClip, PIT_ChangeSector, P_ChangeSector, PTR_UseTraverse, P_UseLines (p_map.c); T_MovePlane, T_MoveFloor, EV_DoFloor (p_floor.c); T_VerticalDoor, EV_DoDoor, EV_VerticalDoor (p_doors.c); T_PlatRaise, EV_DoPlat, P_AddActivePlat, P_RemoveActivePlat (p_plats.c); G_ExitLevel (g_game.c); P_StartButton, P_ChangeSwitchTexture, P_UseSpecialLine (p_switch.c). Bodies whose generic extraction would start at a `// name()` comment header are extracted from their definition; function_sha256 hashes that extraction. Thirteen of them are compiled as Raw_<name> under an observer wrapper with the same signature (see row_formats).',
       'Genuine fragments: the use block of P_PlayerThink (p_user.c:322-331) runs after the weapon change and before P_MovePsprites; the BT_USE statement of P_DeathThink (p_user.c:227-228) runs for a dead player after P_MovePsprites; the soundorg and blockbox statements of P_GroupLines (p_setup.c:555-574) run at load over the bbox the genuine M_ClearBox/M_AddToBox build from every line with the sector on either side, in line order. G_PlayerReborn sets usedown = true.',
       'Order (P_Ticker, p_tick.c:148-157; P_PlayerThink): a door opened by use is created in the player stage and steps in the same op; movers created by crossings are created inside P_RunThinkers by the crossing mobj and step later in the same pass; every new mover joins the thinker list at its tail (P_AddThinker). A removed mover keeps its list entry (TK kind 8) until the next pass unlinks it (SF).',
       'Declared adapters: S_StartSound records its origin (SO rows for &sector->soundorg and for the address of a buttonlist soundorg field; E code 7 rows for NULL and mobj origins); every side texture is resolved at load from the names in the committed SIDEDEFS bytes through the declared synthetic texture table (TESTWALL 0, SW1COMM 1, SW2COMM 2; `-` is 0; the R_TextureNumForName stand-in), so the SW1COMM middle texture the builder writes on the front side of every 11/63 line is 1 and every other texture 0; switchlist is the declared pair {SW1COMM, SW2COMM} = {1, 2} with numswitches 1 (P_InitSwitchList stand-in); G_ExitLevel is genuine and its gameaction/secretexit are harness globals; G_DoCompleted and G_DoReborn are not executed: the case ends at the end of the op that requested them (MG rows); mover allocations (Z_Malloc PU_LEVSPEC) are zero-filled in the harness store and Z_Free quarantines a sector thinker as it quarantines a mobj (ids never reused); textureheight is a null pointer (raiseToTexture is unreachable); EV_DoCeiling, EV_CeilingCrushStop, EV_Teleport, EV_LightTurnOn, EV_StartLightStrobing, EV_TurnTagLightsOff, EV_StopPlat, EV_BuildStairs, EV_DoDonut, EV_DoLockedDoor, G_SecretExitLevel and P_ActivateInStasis abort if reached (no admitted line reaches them).',
       'The dead player\'s commands keep only BT_USE (the inherited harness zeroes them); P_DeathThink\'s view-height, angle and damagecount statements are still not run (bounded death stage, as since M3g). Controlled inputs: BT_USE is held on every op of each use window, one extra angleturn (turn2 at turn2_at) and two extra forwardmove windows.',
       'spechit keeps its source size line_t* spechit[8] and PIT_CheckLine its unchecked append: the sanitizer build would report a ninth write. No room exceeds rule 6 (largest special window 8, the dense lane); the nine-line lane is written as m3h8_dense9.bin for the Rust-only SpecialCross witness and is never run in C. T_MovePlane is always called with crush false, so the PIT_ChangeSector damage and blood branch (its only P_Random draws) is unreachable.',
       'Bounded original command/weapon stage precedes original thinkers; not complete P_Ticker/PlayerThink/DeathThink or respawn. P_RespawnSpecials (deathmatch 2 only) is not called.',
       'All M3h1/M3h2/M3h4/M3h5/M3h6/M3h7 assumptions and adapters apply, including the strict-build retyping of attack-angle int locals; every new row only observes genuine statements.'],
      sanitizer_comparison=sanitizers)
    outputs['m3h8_contract.json']=(json.dumps(contract,indent=2,sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check:assert path.read_bytes()==data,path
        else:path.write_bytes(data)
    print(('Checked'if check else'Generated')+' original doors, lifts, floors, switches and the exit: '+json.dumps(inventory)+f'; strict={overflow}; ASan/UBSan={address}')
    return per_case
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--overflow-check',action='store_true');parser.add_argument('--address-check',action='store_true');parser.add_argument('--cases',action='store_true');args=parser.parse_args()
    table=run(args.check,args.overflow_check,args.address_check)
    if args.cases:
        for row in table:print(json.dumps(row))
