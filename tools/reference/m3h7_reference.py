#!/usr/bin/env python3
"""Genuine original-C sky ceilings and static sector specials (M3h7 Stage A).

Reuses every compiled M3h6 body, observer and controlled input unchanged (importing
m3h6_reference applies the M3h2..M3h6 harness patches) over the two rooms of
m3h7_cases, and adds the genuine bodies of P_SpawnSpecials, P_PlayerInSpecialSector,
P_UpdateSpecials, getNextSector, P_FindMinSurroundingLight and the p_lights.c light
spawners and thinkers. P_SpawnSpecials runs once after every THING spawn and after
the player body (the P_SetupLevel order), P_PlayerInSpecialSector at its P_PlayerThink
position (after P_CalcHeight, before the weapon change and P_MovePsprites) and
P_UpdateSpecials after P_RunThinkers, before `leveltime++`. The sky branches of the
already compiled P_XYMovement and PTR_ShootTraverse are reached for the first time
through a declared `skyflatnum`. New rows are observers only (see the contract's
`row_formats`). Expectations come from compiled original bodies over synthetic
inputs; they never consume Rust output."""
import argparse,collections,csv,hashlib,io,json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3h6_reference as inherited
import m3h7_geometry as geometry
from m3h7_cases import ROOMS,ROOM_NAMES,populate,MAX_PICKUPS
from m3h2_observers import replace_once,replace_last
from evidence_pack import pack,unpack
import m3g2_reference as body_stage
previous=inherited.previous;core=inherited.core;pickup_stage=inherited.pickup_stage
LAYOUTS=populate(previous.pack)
SCHEMA='DOOM-PLAYER-SPECIALS-v1'
LIGHT_KINDS={1:'flash',2:'strobe',3:'glow',4:'flicker'}
# Genuine bodies added to the inherited set, in definition order.
ADDED=[('p_spec.c','getNextSector','sector_t*'),('p_spec.c','P_FindMinSurroundingLight','int'),
       ('p_lights.c','T_FireFlicker','void'),('p_lights.c','P_SpawnFireFlicker','void'),
       ('p_lights.c','T_LightFlash','void'),('p_lights.c','P_SpawnLightFlash','void'),
       ('p_lights.c','T_StrobeFlash','void'),('p_lights.c','P_SpawnStrobeFlash','void'),
       ('p_lights.c','T_Glow','void'),('p_lights.c','P_SpawnGlowingLight','void'),
       ('p_spec.c','P_PlayerInSpecialSector','void'),('p_spec.c','P_ShootSpecialLine','void'),('p_spec.c','P_UpdateSpecials','void'),('p_spec.c','P_SpawnSpecials','void')]
core.ADDED.extend(ADDED)
LIGHT_THINKERS={'T_LightFlash':(1,'lightflash_t'),'T_StrobeFlash':(2,'strobe_t'),'T_Glow':(3,'glow_t'),'T_FireFlicker':(4,'fireflicker_t')}
inherited_original=body_stage._original_body
def raw_body(file,name,result,rename=None):
    """The generic extractor matches the first `name (...) {` text; getNextSector's
    comment header (`// getNextSector()`) precedes its definition, so it is
    extracted from its definition line instead. Every other body is unchanged."""
    if name!='getNextSector':return inherited_original(file,name,result,rename)
    text=(ROOT/'linuxdoom-1.10'/file).read_text()
    match=re.search(r'^sector_t\*\s*\ngetNextSector\s*\(',text,re.M)
    start=text.index('getNextSector',match.start());end=text.index('{',start)+1;level=1
    while level:
        level+=(text[end]=='{')-(text[end]=='}');end+=1
    return result+' '+text[start:end].replace(name,rename or name,1)+'\n'
body_stage._original_body=raw_body
HELPERS=r'''
#define SKYFLATNUM %d
#define MAXANIMS 32
#define MAXLINEANIMS 64
typedef struct{boolean istexture;int picnum;int basepic;int numpics;int speed;}anim_t;
anim_t anims[MAXANIMS],*lastanim;
int *flattranslation,*texturetranslation;
short numlinespecials;line_t* linespeciallist[MAXLINEANIMS];
boolean levelTimer;int levelTimeCount;
ceiling_t* activeceilings[MAXCEILINGS];plat_t* activeplats[MAXPLATS];button_t buttonlist[MAXBUTTONS];
int myargc;char** myargv;
int M_CheckParm(char* check){(void)check;return 0;}
int W_CheckNumForName(char* name){(void)name;return -1;}
void G_ExitLevel(void){abort();}
int EV_DoFloor(line_t* l,floor_e f){(void)l;(void)f;abort();return 0;}
int EV_DoDoor(line_t* l,vldoor_e d){(void)l;(void)d;abort();return 0;}
int EV_DoPlat(line_t* l,plattype_e p,int a){(void)l;(void)p;(void)a;abort();return 0;}
void P_ChangeSwitchTexture(line_t* l,int a){(void)l;(void)a;abort();}
void P_SpawnDoorCloseIn30(sector_t* sec){(void)sec;abort();}
void P_SpawnDoorRaiseIn5Mins(sector_t* sec,int secnum){(void)sec;(void)secnum;abort();}
void T_FireFlicker(fireflicker_t*);
static void* special_store[64];static int special_count;
static void* special_alloc(int size){void* p;if(special_count==64)abort();p=calloc(1,size);special_store[special_count++]=p;return p;}
static void special_reset(void){int i;for(i=0;i<special_count;i++)free(special_store[i]);special_count=0;}
static int special_id(void* p){int i;for(i=0;i<special_count;i++)if(special_store[i]==p)return i;abort();return -1;}
static int light_kind(thinker_t* t){
 if(t->function.acp1==(actionf_p1)T_LightFlash)return 1;if(t->function.acp1==(actionf_p1)T_StrobeFlash)return 2;
 if(t->function.acp1==(actionf_p1)T_Glow)return 3;if(t->function.acp1==(actionf_p1)T_FireFlicker)return 4;return 0;}
static int light_random(int kind,int code,void* thinker,sector_t* s){int v=P_Random();printf("LR,%%d,%%d,%%d,%%d,%%ld,%%d,%%d\n",scenario,operation,kind,special_id(thinker),(long)(s-sectors),code,v);return v;}
static void run_observe(thinker_t* t){int k=light_kind(t);if(k)printf("LE,%%d,%%d,%%d,%%d\n",scenario,operation,k,special_id(t));else observe(0,ident(t),0);}
static void sky_refusal(mobj_t* mo){printf("KY,%%d,%%d,%%d,%%ld,%%ld,%%d\n",scenario,operation,ident(mo),ceilingline?(long)(ceilingline-lines):-1L,ceilingline&&ceilingline->backsector?(long)(ceilingline->backsector-sectors):-1L,ceilingline&&ceilingline->backsector&&ceilingline->backsector->ceilingpic==skyflatnum);}
static void sky_shot(int code,line_t* li,fixed_t z){printf("KS,%%d,%%d,%%d,%%ld,%%ld,%%ld,%%d\n",scenario,operation,code,(long)(li-lines),(long)(li->frontsector-sectors),li->backsector?(long)(li->backsector-sectors):-1L,z);}
'''%geometry.SKYFLATNUM
ZNEEDLE='void* Z_Malloc(int size,int tag,void* user){mobj_t* p;(void)tag;(void)user;'
inherited_write=pickup_stage.write
def write(path,text,*a,**kw):
    if path.name=='m3d3j_shim.h':
        text=replace_once(text,ZNEEDLE,HELPERS+ZNEEDLE+'if(tag==PU_LEVSPEC)return special_alloc(size);')
        text=replace_once(text,'void P_ShootSpecialLine(mobj_t* p,line_t* l){(void)p;(void)l;abort();}','')
    return inherited_write(path,text,*a,**kw)
pickup_stage.write=write
inherited_observed=core.observed_body
def observed(file,name,result,rename=None):
    if name in LIGHT_THINKERS:
        assert rename is None
        kind,struct=LIGHT_THINKERS[name]
        text=inherited_observed(file,name,result,'Observed_'+name)
        if name=='T_LightFlash':
            text=replace_once(text,'(P_Random()&flash->mintime)',f'(light_random({kind},1,flash,flash->sector)&flash->mintime)')
            text=replace_once(text,'(P_Random()&flash->maxtime)',f'(light_random({kind},2,flash,flash->sector)&flash->maxtime)')
        if name=='T_FireFlicker':text=replace_once(text,'(P_Random()&3)',f'(light_random({kind},3,flick,flick->sector)&3)')
        assert 'P_Random' not in text
        return text+f'''void {name}({struct}* t){{int before=t->sector->lightlevel;Observed_{name}(t);if(t->sector->lightlevel!=before)printf("LC,%d,%d,{kind},%d,%ld,%d,%d\\n",scenario,operation,special_id(t),(long)(t->sector-sectors),before,t->sector->lightlevel);}}
'''
    text=inherited_observed(file,name,result,rename)
    if name=='P_SpawnLightFlash':text=replace_once(text,'(P_Random()&flash->maxtime)','(light_random(1,0,flash,sector)&flash->maxtime)')
    if name=='P_SpawnStrobeFlash':text=replace_once(text,'(P_Random()&7)','(light_random(2,0,flash,sector)&7)')
    if name in('P_SpawnFireFlicker','P_SpawnGlowingLight','P_SpawnSpecials','P_UpdateSpecials'):assert 'P_Random' not in text
    if name=='P_PlayerInSpecialSector':
        text=replace_once(text,'sector = player->mo->subsector->sector;','sector = player->mo->subsector->sector;printf("SP,%d,%d,%ld,%d,%d,%d,%d,%d\\n",scenario,operation,(long)(sector-sectors),sector->special,player->mo->z,sector->floorheight,player->mo->floorz,leveltime);')
        text=replace_once(text,'sector->special = 0;','sector->special = 0;printf("SE,%d,%d,%ld,%d\\n",scenario,operation,(long)(sector-sectors),player->secretcount);')
    if name=='P_XYMovement':
        text=replace_once(text,'if (ceilingline &&','sky_refusal(mo);\n\t\tif (ceilingline &&')
        text=replace_once(text,'P_RemoveMobj (mo);\n\t\t    return;','printf("KR,%d,%d,%d\\n",scenario,operation,ident(mo));P_RemoveMobj (mo);\n\t\t    return;')
    if name=='PIT_CheckLine':
        text=replace_once(text,'if (opentop < tmceilingz)','if(tmthing->flags&MF_MISSILE)printf("KL,%d,%d,%d,%ld,%d,%d\\n",scenario,operation,ident(tmthing),(long)(ld-lines),opentop,opentop<tmceilingz);\n    if (opentop < tmceilingz)')
    if name=='PTR_ShootTraverse':
        text=replace_once(text,'if (z > li->frontsector->ceilingheight)\n\t\treturn false;','if (z > li->frontsector->ceilingheight)\n\t\t{sky_shot(0,li,z);return false;}')
        text=replace_once(text,'if\t(li->backsector && li->backsector->ceilingpic == skyflatnum)\n\t\treturn false;','if\t(li->backsector && li->backsector->ceilingpic == skyflatnum)\n\t\t{sky_shot(1,li,z);return false;}')
    if name=='P_ZMovement':
        at=text.index('// hit the ceiling');explode=text.index('P_ExplodeMissile (mo);',at)
        text=text[:explode]+'printf("KZ,%d,%d,%d,%ld,%d\\n",scenario,operation,ident(mo),(long)(mo->subsector->sector-sectors),mo->subsector->sector->ceilingpic==skyflatnum);'+text[explode:]
    return text
core.observed_body=observed
# Driver: the declared map fields and skyflatnum after reset, P_SpawnSpecials after the
# THING spawns and the player body (with its psprites, as P_SpawnPlayer does),
# P_PlayerInSpecialSector at its P_PlayerThink position, P_UpdateSpecials after the
# thinker pass and before leveltime++.
core.DRIVER=replace_once(core.DRIVER,'admit_native=0;','admit_native=0;special_reset();specials_world(v[V_MAP]);skyflatnum=SKYFLATNUM;lastanim=anims;')
core.DRIVER=replace_once(core.DRIVER,'P_SetupPsprites(p);','P_SetupPsprites(p);P_SpawnSpecials();')
core.DRIVER=replace_once(core.DRIVER,'P_CalcHeight(p);source_weapon_selection(p);}','P_CalcHeight(p);if(p->mo->subsector->sector->special)P_PlayerInSpecialSector(p);source_weapon_selection(p);}')
core.DRIVER=replace_once(core.DRIVER,'P_RunThinkers();leveltime++;','P_RunThinkers();P_UpdateSpecials();leveltime++;')
assert core.DRIVER.count('armor_capture();')==2
core.DRIVER=core.DRIVER.replace('armor_capture();','armor_capture();specials_capture();')
CAPTURE=r'''
static void specials_capture(void){thinker_t* t;int i=0;
for(t=thinkercap.next;t!=&thinkercap;t=t->next,i++){int k=t->function.acv==(actionf_v)(-1)?9:light_kind(t);
 if(k==0||k==9)printf("TK,%d,%d,%d,%d,%d,%d\n",scenario,operation,i,k,ident(t),((mobj_t*)t)->type);
 else if(k==1){lightflash_t* f=(lightflash_t*)t;printf("TK,%d,%d,%d,1,%d,%ld,%d,%d,%d,%d,%d\n",scenario,operation,i,special_id(t),(long)(f->sector-sectors),f->count,f->minlight,f->maxlight,f->maxtime,f->mintime);}
 else if(k==2){strobe_t* f=(strobe_t*)t;printf("TK,%d,%d,%d,2,%d,%ld,%d,%d,%d,%d,%d\n",scenario,operation,i,special_id(t),(long)(f->sector-sectors),f->count,f->minlight,f->maxlight,f->darktime,f->brighttime);}
 else if(k==3){glow_t* g=(glow_t*)t;printf("TK,%d,%d,%d,3,%d,%ld,-1,%d,%d,%d,0\n",scenario,operation,i,special_id(t),(long)(g->sector-sectors),g->minlight,g->maxlight,g->direction);}
 else{fireflicker_t* f=(fireflicker_t*)t;printf("TK,%d,%d,%d,4,%d,%ld,%d,%d,%d,0,0\n",scenario,operation,i,special_id(t),(long)(f->sector-sectors),f->count,f->minlight,f->maxlight);}}
printf("SX,%d,%d,%d,%d,%d,%d",scenario,operation,leveltime,players[0].secretcount,totalsecret,numsectors);
for(i=0;i<numsectors;i++)printf(",%d,%d",sectors[i].lightlevel,sectors[i].special);printf("\n");
for(i=0;i<numlinespecials;i++)printf("LW,%d,%d,%d,%ld,%d\n",scenario,operation,i,(long)(linespeciallist[i]-lines),sides[linespeciallist[i]->sidenum[0]].textureoffset);}
'''
BASE_CAPTURE=core.CAPTURE
def flat(name):return geometry.FLATS.index(name)
def world_source():
    """specials_world(which): every sector's light, special, tag and flat numbers and every
    line's special and tag from the generated lumps (the inherited loader sets only heights
    and geometry, and does not clear line fields between maps)."""
    out=['static void specials_world(int which){int i;for(i=0;i<numlines;i++){lines[i].special=0;lines[i].tag=0;}switch(which){']
    for which,m in enumerate(core.main.MAPS):
        out.append(f'case {which}:')
        for i,a in enumerate(m['sector_attrs']):
            out.append(f'sectors[{i}].lightlevel={a["light"]};sectors[{i}].special={a["special"]};sectors[{i}].tag={a["tag"]};sectors[{i}].floorpic={flat(a["floor"])};sectors[{i}].ceilingpic={flat(a["ceiling"])};')
        for i,(special,tag) in enumerate(m['line_specials']):
            if special or tag:out.append(f'lines[{i}].special={special};lines[{i}].tag={tag};')
        out.append('break;')
    out.append('default:abort();}}')
    return '\n'.join(out)+'\n'
def configure():
    core.main.MAPS,core.main.THINGS,core.main.KEYS,core.main.CASES=[],[],[],[]
    for c in core.CASES:
        sectors,portals,reject,tree,segment,lines=ROOMS[c['room']]
        m=geometry.room(sectors,portals=portals,reject=reject,tree=tree,segment=segment,lines=lines)
        core.main.case(c['label'],n=1,custom=m,things=LAYOUTS[c['layout']],tics=c['tics'],rejected=bool(c['rejected']),
                       **{'pc_'+k:v for k,v in c.items()if k not in['label','tics','map']})
        c['map']=core.main.CASES[-1]['map']
    core.CAPTURE=BASE_CAPTURE+world_source()+CAPTURE
core.configure=configure
inherited_run=pickup_stage.real_run
def compile_specials(args,*a,**kw):
    """The generator wraps every thinker call with an actor observation (E code 0,
    ident of the thinker); a sector thinker is not an actor, so that one observer
    call becomes run_observe (E code 0 for actors, LE for sector thinkers)."""
    if args and args[0]=='cc':
        for x in args:
            if str(x).endswith('reference.c'):
                path=Path(x);path.write_text(replace_once(path.read_text(),'observe(0,ident(currentthinker),0);','run_observe(currentthinker);'))
    return inherited_run(args,*a,**kw)
pickup_stage.real_run=compile_specials
sha=lambda b:hashlib.sha256(b).hexdigest()
def capture(flags):
    observed_trace,_=previous.capture(flags)
    return observed_trace
ROW_FORMATS={
 'TK':'TK,case,op,index,kind,id,... one row per thinker-list entry in list order after the op (op 0: after P_SpawnSpecials). kind 0 mobj / 9 mobj removed but not yet unlinked: id = actor ident, then mobj type. kind 1 flash, 2 strobe, 3 glow, 4 flicker: id = sector-thinker index (creation order of PU_LEVSPEC allocations), then sector, count, minlight, maxlight, a, b with a,b = maxtime,mintime (flash), darktime,brighttime (strobe), direction,0 (glow; count printed -1, glow_t has none), 0,0 (flicker)',
 'SX':'SX,case,op,leveltime,players[0].secretcount,totalsecret,numsectors,(lightlevel,special) for every sector in index order; after each op (leveltime already incremented, so leveltime == op)',
 'LW':'LW,case,op,index,line,sides[sidenum[0]].textureoffset for every linespeciallist entry (line special 48) after each op; presentation only',
 'LE':'LE,case,op,kind,id: P_RunThinkers runs a sector thinker (the actor runs keep their E code 0 rows, in the same interleaved order)',
 'LR':'LR,case,op,kind,id,sector,code,value: the light body P_Random draw just made (the E,...,8 row printed by P_Random precedes it); code 0 spawn count (flash: (v&64)+1, strobe unsynced: (v&7)+1), 1 flash going dark ((v&mintime)+1, mintime 7), 2 flash going bright ((v&maxtime)+1, maxtime 64), 3 fire flicker cycle ((v&3)*16)',
 'LC':'LC,case,op,kind,id,sector,before,after: a light thinker call changed its sector light level',
 'SP':'SP,case,op,sector,special,mo->z,sector->floorheight,mo->floorz,leveltime: entry of P_PlayerInSpecialSector (called only when the center sector special is nonzero); the body returns unless z == floorheight',
 'SE':'SE,case,op,sector,secretcount: secret entry after player->secretcount++ and sector->special = 0',
 'KY':'KY,case,op,missile,ceilingline (-1 none),ceilingline backsector (-1 none),backsector ceilingpic == skyflatnum: a missile P_TryMove refusal in P_XYMovement, before the sky test',
 'KR':'KR,case,op,missile: the sky branch removes the missile (P_RemoveMobj, no P_ExplodeMissile and no draw)',
 'KL':'KL,case,op,missile,line,opentop,opentop < tmceilingz: PIT_CheckLine for a missile tmthing reached the ceilingline test of a two-sided line (1 = this line becomes ceilingline)',
 'KS':'KS,case,op,code,line,frontsector,backsector,z: PTR_ShootTraverse ended at a front-sector sky ceiling without a puff; code 0 z > front ceiling height, 1 back sector also sky',
 'KZ':'KZ,case,op,missile,sector,ceilingpic == skyflatnum: P_ZMovement ceiling hit about to call P_ExplodeMissile (no sky test)',
 'G':'existing damage rows; environmental damage is G,case,op,2,target,-1,-1,damage preceded in the same op by the SP row of the call',
 'E':'existing; E,case,op,8,-1,value is every gameplay P_Random draw, including the P_SpawnSpecials spawn draws at op 0 and the light thinker draws'}
def run(check,overflow,address):
    flags=list(previous.pack.legacy.base.FLAGS)
    observed_trace=capture(flags)
    for enabled,sanitizer in [(overflow,'signed-integer-overflow,integer-divide-by-zero,float-cast-overflow'),(address,'address,undefined')]:
        if enabled:
            audited=capture([f for f in flags if f!='-fwrapv']+['-fno-wrapv','-fsanitize='+sanitizer,'-fno-sanitize-recover=all'])
            assert audited['m3d3j_trace.csv']==observed_trace['m3d3j_trace.csv']
    rows=list(csv.reader(io.StringIO(observed_trace['m3d3j_trace.csv'].decode())))
    functions=collections.Counter(r[3]for r in rows if r[0]=='FN')
    selected={0,1,2,4,8,16,32,48,64,96,128,192,256,320,384,448,512,576,640,704,768,832,896,960,1024,1088,1152,1200}
    pushes={(i,c['push_at'])for i,c in enumerate(core.CASES)if c['push_at']}
    interesting={(int(r[1]),int(r[2]))for r in rows if (r[0]=='G'and r[3]in{'2','7','10'})or r[0]in{'AR','ST','PN','SL','X','N','SE','KY','KR','KS','KZ'}or(r[0]=='MS'and r[3]in{'0','5'})or(r[0]=='E'and r[3]=='1'and r[2]!='0')or(r[0]=='M'and r[7]=='0')}|pushes
    keys={(int(r[1]),int(r[2]))for r in rows if r[0]=='P'and(int(r[2])in selected or tuple(map(int,r[1:3]))in interesting or int(r[2])==core.CASES[int(r[1])]['tics'])}
    kept=[r for r in rows if r[0]!='FN'and(r[0]not in{'A','P','AP','R','S','LS'}or tuple(map(int,r[1:3]))in keys)]
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(kept)
    expanded=(SCHEMA+'\n'+text.getvalue()).encode()
    outputs={'m3h7_trace.dref':pack(expanded)}
    assert unpack(outputs['m3h7_trace.dref'])==expanded
    text=io.StringIO();writer=csv.writer(text,lineterminator='\n');writer.writerow(['case',*core.CASES[0]]);writer.writerows([i,*c.values()]for i,c in enumerate(core.CASES))
    outputs['m3h7_cases.csv']=text.getvalue().encode()
    maps=[]
    for i,m in enumerate(core.main.MAPS):
        name=f'm3h7_geometry_{i}.bin';outputs[name]=m['bytes'];maps.append('tests/reference/'+name)
    outputs['m3h7_maps.rs']=('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("{Path(p).name}"),\n'for p in maps)+'];\n').encode()
    manifest=json.loads(observed_trace['m3d3j_provenance.json'])
    bodies=manifest['functions']+[['p_pspr.c',n,'void']for n in previous.pack.WEAPON_ACTIONS]
    assert len(bodies)==len({tuple(b)for b in bodies})
    # ---- observations, all computed from the full trace -------------------------------
    full=[r for r in rows if r[0]!='FN']
    case_rows=collections.defaultdict(list)
    for r in full:case_rows[int(r[1])].append(r)
    player_ident={}
    for which,mine in case_rows.items():
        player_ident[which]=next(r[5]for r in mine if r[0]=='W'and r[2]=='0'and r[3]=='7')
    # leveltime: SP rows are inside op t with leveltime t-1; SX rows after op t with leveltime t.
    assert all(int(r[8])==int(r[2])-1 for r in full if r[0]=='SP')
    assert all(int(r[3])==int(r[2]) for r in full if r[0]=='SX')
    # Load order: every sector thinker follows every actor in the op-0 thinker list, and
    # every P_SpawnSpecials draw follows the last actor spawn of op 0.
    load_lights=collections.Counter();trailing_actors=0;spawn_order_ok=True
    for which,mine in case_rows.items():
        first=[r for r in mine if r[0]=='TK'and r[2]=='0']
        kinds=[int(r[4])for r in first]
        assert kinds==sorted(kinds,key=lambda k:k!=0),(which,kinds)
        if which==0:load_lights=collections.Counter(LIGHT_KINDS[k]for k in kinds if k)
        zero=[i for i,r in enumerate(mine)if r[2]=='0']
        births=[i for i in zero if mine[i][0]=='E'and mine[i][3]=='1']
        spawn_draws=[i for i in zero if mine[i][0]=='LR']
        spawn_order_ok&=all(i>max(births)for i in spawn_draws)
        for t,snap in itertools_groupby([r for r in mine if r[0]=='TK'and r[2]!='0']):
            k=[int(r[4])for r in snap];first_light=next((i for i,x in enumerate(k)if 0<x<9),None)
            if first_light is not None:trailing_actors=max(trailing_actors,sum(1 for x in k[first_light:]if x in(0,9)))
    assert spawn_order_ok
    # Environmental damage: G code 2 with no inflictor and no source, preceded in the op by SP.
    env=[];sp_last={}
    for r in full:
        if r[0]=='SP':sp_last[(r[1],r[2])]=r
        if r[0]=='G'and r[3]=='2'and r[5]=='-1'and r[6]=='-1':
            assert r[4]==player_ident[int(r[1])]and (r[1],r[2])in sp_last,r
            assert all(c['hit_at']==0 for c in core.CASES)
            env.append((r,sp_last[(r[1],r[2])]))
    env_by_special=collections.Counter(f'{s[4]}:{r[7]}'for r,s in env)
    absorbed=[r for r in full if r[0]=='AR'and int(r[5])>0 and any(e[0][1]==r[1]and e[0][2]==r[2]for e in env)]
    env_kills=[r for r in full if r[0]=='G'and r[3]=='7'and r[4]==player_ident[int(r[1])]and any(e[0][1]==r[1]and e[0][2]==r[2]for e in env)]
    own_frags={r[1]:int(r[4])for r in full if r[0]=='PL'and r[3]=='2'}
    ledge=[r for r in full if r[0]=='SP'and r[5]!=r[6]and r[7]!=r[6]]
    # Secrets: totalsecret at load, entries, special cleared in the same op's SX row.
    secrets=[r for r in full if r[0]=='SE']
    sx={(r[1],r[2]):r for r in full if r[0]=='SX'}
    for r in secrets:assert sx[(r[1],r[2])][7+2*int(r[3])+1]=='0'
    total_secret={r[1]:int(r[5])for r in full if r[0]=='LS'and r[2]=='0'}
    # Lights.
    lc=collections.Counter(LIGHT_KINDS[int(r[3])]for r in full if r[0]=='LC')
    lr=collections.Counter(LIGHT_KINDS[int(r[3])]for r in full if r[0]=='LR')
    spawn_lr=collections.Counter(LIGHT_KINDS[int(r[3])]for r in full if r[0]=='LR'and r[6]=='0')
    bright=collections.Counter((int(r[7])&64)+1 for r in full if r[0]=='LR'and r[3]=='1'and r[6]=='2')
    dark=collections.Counter((int(r[7])&7)+1 for r in full if r[0]=='LR'and r[3]=='1'and r[6]=='1')
    flicker=collections.Counter((int(r[7])&3)*16 for r in full if r[0]=='LR'and r[3]=='4')
    draws=collections.Counter(r[1]for r in full if r[0]=='E'and r[3]=='8')
    light_tics={(r[1],r[2])for r in full if r[0]=='LC'};boom_tics={(r[1],r[2])for r in full if r[0]=='MS'and r[3]=='0'}
    # Sky.
    ky=[r for r in full if r[0]=='KY'];kr={(r[1],r[2],r[3])for r in full if r[0]=='KR'}
    explode={(r[1],r[2],r[4])for r in full if r[0]=='MS'and r[3]=='0'}
    for r in ky:assert ((r[1],r[2],r[3])in kr)==(r[6]=='1')and((r[1],r[2],r[3])in explode)==(r[6]=='0'),r
    # The KL rows of one P_CheckPosition precede its M row (M,case,op,attempt,actor,...,ok);
    # a refused missile step's M row precedes its KY row.
    checks=[];pending=collections.defaultdict(list);closed={}
    for r in full:
        if r[0]=='KL':pending[(r[1],r[3])].append(r)
        elif r[0]=='M':
            lines=pending.pop((r[1],r[4]),[])
            if r[7]=='0':closed[(r[1],r[4])]=(lines,r)
        elif r[0]=='KY':checks.append((r,*closed.pop((r[1],r[3]))))
    two_lowering=[(r,k)for r,k,_ in checks if sum(x[6]=='1'for x in k)>=2]
    later_not_lowering=[(r,k)for r,k,_ in checks if any(x[6]=='1'for x in k)and k[-1][6]=='0']
    entered_lower=[(r,k)for r,k,_ in checks if r[4]=='-1'and k and all(x[6]=='0'for x in k)]
    # M move_failure 2: a blocking (one-sided) line refused the step after the ceilingline.
    removed_at_wall=[r for r,_,move in checks if r[6]=='1'and move[12]=='2']
    ks=collections.Counter(r[3]for r in full if r[0]=='KS')
    kz=[r for r in full if r[0]=='KZ']
    lw={}
    for r in full:
        if r[0]=='LW':lw[(r[1],r[3])]=r
    scroll_ok=all(int(r[5])==int(r[2])*65536 for r in lw.values())
    monster_types={1,2,10,11,12}
    per_case=[]
    for which,c in enumerate(core.CASES):
        mine=case_rows[which];m=core.main.MAPS[c['map']];w=str(which)
        player=[r for r in mine if r[0]=='P']
        mine_env=[(r,s)for r,s in env if r[1]==w]
        per_case.append(dict(label=c['label'],room=ROOM_NAMES[c['room']],tics=c['tics'],
          map=dict(linedefs=len(m['lines']),sectors=len(m['heights']),subsectors=len(m['leaves']),nodes=len(m['nodes']),cells=m['cells'],
                   sky_ceilings=sum(a['ceiling']==geometry.SKY for a in m['sector_attrs']),sector_specials=sorted(a['special']for a in m['sector_attrs']if a['special']),
                   scrollers=sum(s==48 for s,_ in m['line_specials'])),
          light_thinkers=dict(collections.Counter(LIGHT_KINDS[int(r[4])]for r in mine if r[0]=='TK'and r[2]=='0'and 0<int(r[4])<9)),
          light_changes=dict(collections.Counter(LIGHT_KINDS[int(r[3])]for r in mine if r[0]=='LC')),
          light_draws=dict(collections.Counter(LIGHT_KINDS[int(r[3])]for r in mine if r[0]=='LR')),
          spawn_draws=sum(r[0]=='LR'and r[6]=='0'for r in mine),gameplay_draws=draws[w],
          special_calls=sum(r[0]=='SP'for r in mine),ledge_returns=sum(r in ledge for r in mine),
          environmental_damage=[[int(r[2]),int(s[4]),int(r[7])]for r,s in mine_env],
          armor_absorptions=sum(r[1]==w for r in absorbed),environmental_deaths=sum(r[1]==w for r in env_kills),
          secrets=[[int(r[2]),int(r[3])]for r in secrets if r[1]==w],total_secret=total_secret[w],
          launches=sum(r[0]=='MS'and r[3]=='5'for r in mine),explosions=sum(r[0]=='MS'and r[3]=='0'for r in mine),
          light_and_explosion_tics=sum(1 for k in light_tics&boom_tics if k[0]==w),
          missile_refusals=sum(r[1]==w for r in ky),sky_removals=sum(k[0]==w for k in kr),
          refusals_with_two_lowering_lines=sum(r[1]==w for r,_ in two_lowering),refusals_with_later_nonlowering_line=sum(r[1]==w for r,_ in later_not_lowering),
          refusals_center_already_lower=sum(r[1]==w for r,_ in entered_lower),sky_removals_at_blocking_line=sum(r[1]==w for r in removed_at_wall),
          sky_shots={'above_front_ceiling':sum(r[0]=='KS'and r[3]=='0'for r in mine),'hack_wall':sum(r[0]=='KS'and r[3]=='1'for r in mine)},
          sky_ceiling_explosions=sum(r[1]==w and r[5]=='1'for r in kz),
          hitscans=sum(r[0]=='X'and r[3]=='0'for r in mine),hitscan_line_impacts=sum(r[0]=='X'and r[3]=='1'and r[4]=='1'for r in mine),
          kills=sum(r[0]=='G'and r[3]=='7'for r in mine),pickups=sum(r[0]=='PN'for r in mine),
          player_final=dict(health=int(player[-1][4]),playerstate=int(player[-1][5]),frags=int(player[-1][8]))if player else None))
    inventory=dict(scenarios=len(core.CASES),scripted_tics=sum(c['tics']for c in core.CASES),captures=len(keys),rooms=len(ROOMS),maps=len(core.main.MAPS),
      selected_original_bodies=len(bodies),exercised_unique_functions=len(functions),function_entries=sum(functions.values()),
      new_body_entries={f'{f}:{n}':functions[f'{f}:{n}']for f,n,_ in ADDED},
      light_thinkers_at_load=dict(load_lights),light_changes=dict(lc),light_draws=dict(lr),spawn_draws=dict(spawn_lr),
      flash_bright_phase_lengths=dict(sorted(bright.items())),flash_dark_phase_lengths=dict(sorted(dark.items())),flicker_amounts=dict(sorted(flicker.items())),
      max_actor_entries_after_first_light=trailing_actors,
      special_calls=sum(r[0]=='SP'for r in full),ledge_returns=len(ledge),
      environmental_damage_by_special_and_amount=dict(sorted(env_by_special.items())),environmental_damage_events=len(env),
      armor_absorptions=len(absorbed),environmental_deaths=len(env_kills),own_frags=sum(own_frags.values()),
      secrets=len(secrets),secret_specials_cleared=len(secrets),
      fireball_launches=sum(r[0]=='MS'and r[3]=='5'for r in full),fireball_explosions=sum(r[0]=='MS'and r[3]=='0'for r in full),
      light_and_explosion_tics=len(light_tics&boom_tics),
      missile_refusals=len(ky),missile_refusals_with_ceilingline=sum(r[4]!='-1'for r in ky),sky_removals=len(kr),
      refusals_with_two_lowering_lines=len(two_lowering),refusals_with_later_nonlowering_line=len(later_not_lowering),refusals_center_already_lower=len(entered_lower),sky_removals_at_blocking_line=len(removed_at_wall),
      sky_shots_above_front_ceiling=ks['0'],sky_shots_hack_wall=ks['1'],puffs_avoided=sum(ks.values()),puff_draws_avoided=2*sum(ks.values()),
      sky_ceiling_explosions=sum(r[5]=='1'for r in kz),
      scroller_lines={ROOM_NAMES[c['room']]:sum(s==48 for s,_ in core.main.MAPS[c['map']]['line_specials'])for c in core.CASES},scroll_offsets_equal_leveltime=scroll_ok,
      hitscans=sum(r[0]=='X'and r[3]=='0'for r in kept),damage_requests=sum(r[0]=='G'and r[3]=='2'for r in kept),
      monster_kills=sum(1 for r in kept if r[0]=='G'and r[3]=='7'and r[4]!=player_ident[int(r[1])]),player_deaths=sum(1 for r in kept if r[0]=='G'and r[3]=='7'and r[4]==player_ident[int(r[1])]),
      gameplay_draws=sum(draws.values()),thinker_rows=sum(r[0]=='TK'for r in kept),sector_rows=sum(r[0]=='SX'for r in kept),
      expanded_trace_bytes=len(expanded),packed_trace_bytes=len(outputs['m3h7_trace.dref']))
    inputs=sorted({Path(m.__file__).name for m in list(sys.modules.values()) if getattr(m,'__file__',None) and Path(m.__file__).resolve().parent==ROOT/'tools/reference'})
    contract=dict(schema=SCHEMA,revision=manifest['revision'],flags=manifest['flags'],inventory=inventory,cases=per_case,maps=maps,functions=bodies,
      row_formats=ROW_FORMATS,flats=geometry.FLATS,skyflatnum=geometry.SKYFLATNUM,
      source_sha256={n:sha((ROOT/'linuxdoom-1.10'/n).read_bytes())for n in sorted(set(manifest['source_sha256'])|{'d_items.c','dstrings.h','p_spec.c','p_lights.c','p_spec.h','p_user.c','p_setup.c'})},
      function_sha256={f'{f}:{n}':sha(raw_body(f,n,t).encode())for f,n,t in bodies},
      executed_function_entries=dict(sorted(functions.items())),
      inputs_sha256={'tools/reference/'+n:sha((ROOT/'tools/reference'/n).read_bytes())for n in inputs},
      outputs_sha256={n:sha(d)for n,d in outputs.items()},expanded_trace_sha256=sha(expanded),assumptions=[
       f'Genuine player0 plus at most eight map-THING MT_TROOP/MT_POSSESSED/MT_SHOTGUY/MT_CHAINGUY/MT_SERGEANT and at most {MAX_PICKUPS} map-THING pickups in two polygon rooms (pools, sky) built by the M3h6 node builder with declared sector light, special, tag and flats and line specials; no Rust expected observations.',
       'Every compiled body, observer and controlled input of M3h2/M3h5/M3h6 is reused unchanged. Added genuine bodies: getNextSector, P_FindMinSurroundingLight, P_PlayerInSpecialSector, P_ShootSpecialLine, P_UpdateSpecials, P_SpawnSpecials (p_spec.c) and T_FireFlicker, P_SpawnFireFlicker, T_LightFlash, P_SpawnLightFlash, T_StrobeFlash, P_SpawnStrobeFlash, T_Glow, P_SpawnGlowingLight (p_lights.c). getNextSector is extracted from its definition line because the generic extractor would start at its comment header; its function_sha256 hashes that extraction.',
       'Order (P_SetupLevel, p_setup.c:664-682; P_Ticker, p_tick.c:148-157; P_PlayerThink, p_user.c:272-275): after reset the declared map fields are installed; G_PlayerReborn, P_LoadThings (every THING), the player body with P_SetupPsprites, then P_SpawnSpecials once, so every light thinker follows every map actor and the player in the thinker list and every spawn draw follows every THING draw. In op t: the bounded command stage, P_CalcHeight, then P_PlayerInSpecialSector when the center sector special is nonzero (live player only), weapon selection, P_MovePsprites, P_RunThinkers (actors and sector thinkers in one list, LE/E rows), P_UpdateSpecials, leveltime++. During op t leveltime == t - 1 (reset sets 0 via the genuine P_SetupLevel fragment); the SP rows print it and the SX rows print t after the increment.',
       'Declared adapters: skyflatnum is the index of F_SKY1 in the declared flat table (R_FlatNumForName stand-in); floorpic/ceilingpic are flat-table indices; W_CheckNumForName returns -1 (no TEXTURE2, episode unused); M_CheckParm returns 0 (no -avg/-timer, and deathmatch is false); P_InitPicAnims is not run, so lastanim == anims and P_UpdateSpecials animates nothing (presentation); buttonlist, activeplats and activeceilings are zeroed arrays of their source sizes; P_SpawnDoorCloseIn30, P_SpawnDoorRaiseIn5Mins, EV_DoFloor, EV_DoDoor, EV_DoPlat, P_ChangeSwitchTexture and G_ExitLevel abort if reached (no room has sector specials 10, 11, 14 or a line special other than 48); P_CrossSpecialLine and P_UseSpecialLine keep their aborting stubs (one-sided scrollers never enter spechit; no BT_USE).',
       'Sector thinkers are allocated by the genuine Z_Malloc(PU_LEVSPEC) calls into a separate harness store (special_alloc), so actor idents keep their M3h6 meaning; TK/LE/LR/LC rows name a sector thinker by its creation index. The generator-added observer on every thinker call (E code 0) is routed through run_observe so a sector thinker prints LE instead of an actor ident.',
       'Line special 48 is compiled and observed (LW rows): textureoffset equals leveltime * FRACUNIT; it is presentation (no game-code reader). Shots at a scroller call the genuine P_ShootSpecialLine, which does nothing for 48.',
       'Environmental damage (P_PlayerInSpecialSector to P_DamageMobj(player->mo, NULL, NULL, n)) and death (P_KillMobj with a null source: own frag, PL code 2 row) are observed only for the player: only players read sector damage specials, so the null-source players[0] kill credit for an MF_COUNTKILL monster is not reachable here. No radiation suit exists in the roster, so the 4/16 P_Random() < 5 draw is short-circuited away (no draw).',
       'A back-sided line without ML_TWOSIDED (the only way two sectors share a line that getNextSector skips) is rejected by the admitted load rules (player_map_scale.rs back side must agree with ML_TWOSIDED); the pools room instead has a dark gallery (light 0) behind the basins one-sided north walls and a void strip, so P_FindMinSurroundingLight skips it, plus one-sided lines in every basin line list.',
       'Bounded original command/weapon stage precedes original thinkers; not complete P_Ticker/PlayerThink/DeathThink or respawn. P_RespawnSpecials (deathmatch 2 only) is not called.',
       'All M3h1/M3h2/M3h4/M3h5/M3h6 assumptions and adapters apply, including the strict-build retyping of attack-angle int locals; every new row only observes genuine statements.'])
    outputs['m3h7_contract.json']=(json.dumps(contract,indent=2,sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check:assert path.read_bytes()==data,path
        else:path.write_bytes(data)
    print(('Checked'if check else'Generated')+' original sky and static specials: '+json.dumps(inventory)+f'; strict={overflow}; ASan/UBSan={address}')
    return per_case
def itertools_groupby(rows):
    """(op, rows) groups of consecutive rows sharing an op."""
    out=[]
    for r in rows:
        if out and out[-1][0]==r[2]:out[-1][1].append(r)
        else:out.append((r[2],[r]))
    return out
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--overflow-check',action='store_true');parser.add_argument('--address-check',action='store_true');parser.add_argument('--cases',action='store_true');args=parser.parse_args()
    table=run(args.check,args.overflow_check,args.address_check)
    if args.cases:
        for row in table:print(json.dumps(row))
