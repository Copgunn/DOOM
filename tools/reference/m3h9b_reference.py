#!/usr/bin/env python3
"""Genuine original-C exploding barrels, the radius attack, pickup scale and every THING
angle (M3h9b).

Reuses every compiled M3h9 body, observer, declared adapter and controlled input
unchanged (importing m3h9_reference applies the M3h2..M3h9 harness patches) over the
rooms of m3h9b_cases, and adds three genuine bodies: PIT_RadiusAttack and
P_RadiusAttack (p_map.c:1164-1198, 1206-1233, with their globals bombsource, bombspot
and bombdamage, p_map.c:1154-1156) and A_Explode (p_enemy.c:1598-1601), which replaces
the aborting stub every unlisted action has had since M3d3j. The barrel's spawn, its
S_BAR1/S_BAR2 cycle, its damage (P_DamageMobj with thrust, the pain draw and the
retarget; P_KillMobj), the puff on a MF_NOBLOOD thing, A_Scream, P_RemoveMobj,
P_CheckSight, P_BlockThingsIterator and the movement bodies are compiled since
M3d3/M3g/M3h2 and reach the barrel's branches for the first time. Two observer rows
are added (RA, RC; see the contract's `row_formats`). Two declared adapters rewrite one
statement each in the two sanitizer builds only (the strict -fwrapv build compiles the
genuine text): see the contract's `sanitizer_adapters`. Expectations come from compiled
original bodies over synthetic inputs; they never consume Rust output."""
import argparse,collections,csv,hashlib,io,json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3h9_reference as inherited
import m3h9b_geometry as geometry
from m3h9b_cases import ROOMS,ROOM_NAMES,populate,MAX_PICKUPS,MAX_MONSTERS,MAX_DECORATIONS,MAX_BARRELS,BARREL,SPAWN_TICS_DRAW,ROSTER,PICKUPS,POSSESSED,IMP
from m3h2_observers import replace_once
from evidence_pack import pack,unpack
import m3g2_reference as body_stage
previous=inherited.previous;core=inherited.core;pickup_stage=inherited.pickup_stage
LAYOUTS=populate(previous.pack)
SCHEMA='DOOM-PLAYER-BARRELS-v1'
# Genuine bodies added to the inherited set, in definition order.
ADDED=[('p_map.c','PIT_RadiusAttack','boolean'),('p_map.c','P_RadiusAttack','void'),('p_enemy.c','A_Explode','void')]
core.ADDED.extend(ADDED)
EXPLODE_ACTION=core.legacy.actors.ACTIONS.index('A_Explode')
U=65536
MT=geometry.enum_members('mobjtype_t');ST=geometry.enum_members('statenum_t')
inherited_extract=body_stage._original_body
def definition_body(file,name,result,rename=None):
    """The added bodies are extracted from their definition: the first `name (` outside a
    `//` comment whose balanced parameter list is followed by `{` (the M3h8 rule; the
    P_RadiusAttack definition puts its name and parameter list on separate lines)."""
    if name not in {n for _,n,_ in ADDED}:return inherited_extract(file,name,result,rename)
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
# The globals of the radius attack, verbatim from p_map.c (they follow the RADIUS ATTACK
# comment header), and the observer helpers. ra_row prints one RC row per PIT_RadiusAttack
# call, in walk order; ra_seen brackets the genuine P_CheckSight call and returns its value.
_map=(ROOT/'linuxdoom-1.10/p_map.c').read_text()
BOMB_GLOBALS=re.search(r'mobj_t\*\s+bombsource;\s*\nmobj_t\*\s+bombspot;\s*\nint\s+bombdamage;',_map).group(0)
HELPERS=BOMB_GLOBALS+r'''
static int ra_block=-1,ra_pos=0;
static void ra_row(mobj_t* t,int code,int dist,int damage){printf("RC,%d,%d,%d,%d,%d,%d,%d,%d\n",scenario,operation,ident(t),ra_block,ra_pos++,code,dist,damage);}
static boolean ra_seen(mobj_t* t,boolean seen,int dist){ra_row(t,seen?4:3,dist,seen?bombdamage-dist:0);return seen;}
'''
inherited_observed=core.observed_body
def observed(file,name,result,rename=None):
    text=inherited_observed(file,name,result,rename)
    if name=='PIT_RadiusAttack':
        text=replace_once(text,'    if (!(thing->flags & MF_SHOOTABLE) )\n\treturn true;','    if (!(thing->flags & MF_SHOOTABLE) )\n\t{ra_row(thing,0,-1,0);return true;}')
        text=replace_once(text,'\t|| thing->type == MT_SPIDER)\n\treturn true;','\t|| thing->type == MT_SPIDER)\n\t{ra_row(thing,1,-1,0);return true;}')
        text=replace_once(text,'    if (dist >= bombdamage)\n\treturn true;','    if (dist >= bombdamage)\n\t{ra_row(thing,2,dist,0);return true;}')
        text=replace_once(text,'    if ( P_CheckSight (thing, bombspot) )','    if ( ra_seen(thing,P_CheckSight (thing, bombspot),dist) )')
    if name=='P_RadiusAttack':
        text=replace_once(text,'    bombdamage = damage;','    bombdamage = damage;printf("RA,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d\\n",scenario,operation,ident(spot),source?ident(source):-1,damage,xl,xh,yl,yh,spot->x,spot->y,spot->z,dist);')
        text=replace_once(text,'\t    P_BlockThingsIterator (x, y, PIT_RadiusAttack );','\t    {ra_block=y*bmapwidth+x;ra_pos=0;P_BlockThingsIterator (x, y, PIT_RadiusAttack );}')
    if name=='A_Explode':
        text=text.replace('{','{\n observe(5,ident(thingy),'+str(EXPLODE_ACTION)+');',1)
    if name in('PIT_RadiusAttack','P_RadiusAttack','A_Explode'):
        assert 'P_Random' not in text,name
    return text
core.observed_body=observed
# Declared adapters of the two sanitizer builds (decisions F5 and F8 of the M3h9b note).
# The strict build compiles -fwrapv over the genuine text; the sanitizer builds report the
# int product of p_mobj.c:794 for octants outside -4..3 (the left shift of p_map.c:1221
# is undefined only from C99 on, and the harness compiles -std=gnu89: see its reason).
# Each rewrite computes the same 32-bit value in unsigned arithmetic; the byte-identical
# raw traces of the three builds prove it reproduces the wrap on every input these rooms
# reach.
SANITIZER_ADAPTERS=[
 dict(file='p_mobj.c',line=794,function='P_SpawnMapThing',original='    mobj->angle = ANG45 * (mthing->angle/45);',
      adapted='    mobj->angle = (angle_t)((unsigned)ANG45 * (unsigned)(mthing->angle/45));',
      reason='ANG45 * q overflows int for quotients outside -4..3 (every angle of 180..359 or below -224, decision F8); the original targets wrap it to q * 0x20000000 mod 2^32, which the unsigned product computes. Needed: without it both sanitizer builds stop at this statement (observed: runtime error: signed integer overflow: 4 * 536870912 cannot be represented in type int).'),
 dict(file='p_map.c',line=1221,function='P_RadiusAttack',original='    dist = (damage+MAXRADIUS)<<FRACBITS;',
      adapted='    dist = (fixed_t)(((unsigned)damage + (unsigned)MAXRADIUS) << FRACBITS);',
      reason='MAXRADIUS is 32*FRACUNIT without parentheses (p_local.h:51), so (128 + 32*65536) << 16 does not fit in int (undefined behaviour in ISO C99 and later); the original targets keep the low 32 bits, 128 << 16 (decision F5: the window is exactly 3 x 3 blocks), which the unsigned rewrite computes without depending on the language mode. Declared for robustness, not needed by these builds: the harness compiles -std=gnu89 and with the generating compiler neither sanitizer build reports the original statement (GCC checks shift-base only for C99 and later), observed by building both without this adapter.')]
for adapter in SANITIZER_ADAPTERS:
    source_line=(ROOT/'linuxdoom-1.10'/adapter['file']).read_text().split('\n')[adapter['line']-1]
    assert source_line==adapter['original'],adapter
inherited_run=pickup_stage.real_run
def compile_barrels(args,*a,**kw):
    """Source-level harness edits on the generated reference.c: the aborting A_Explode
    stub is removed (the genuine body is compiled), the radius-attack globals and the
    observer helpers are inserted, and in a sanitizer build only the declared adapters
    replace their one statement each."""
    if args and args[0]=='cc':
        sanitized=any(str(x).startswith('-fsanitize=')for x in args)
        for x in args:
            if str(x).endswith('reference.c'):
                path=Path(x);text=path.read_text()
                text=replace_once(text,'void A_Explode(mobj_t* actor){(void)actor;abort();}','')
                text=replace_once(text,'#include "d_event.h"\n','#include "d_event.h"\n'+HELPERS)
                if sanitized:
                    for adapter in SANITIZER_ADAPTERS:text=replace_once(text,adapter['original']+'\n',adapter['adapted']+'\n')
                path.write_text(text)
    return inherited_run(args,*a,**kw)
pickup_stage.real_run=compile_barrels
def configure():
    core.main.MAPS,core.main.THINGS,core.main.KEYS,core.main.CASES=[],[],[],[]
    for c in core.CASES:
        sectors,portals,reject,tree,segment,lines=ROOMS[c['room']]
        m=geometry.room(sectors,portals=portals,reject=reject,tree=tree,segment=segment,lines=lines)
        core.main.case(c['label'],n=1,custom=m,things=LAYOUTS[c['layout']],tics=c['tics'],rejected=bool(c['rejected']),skill=c['skill'],
                       **{'pc_'+k:v for k,v in c.items()if k not in['label','tics','map','skill']})
        c['map']=core.main.CASES[-1]['map']
    specials=inherited.inherited.inherited
    core.CAPTURE=specials.BASE_CAPTURE+specials.world_source()+specials.CAPTURE+inherited.inherited.switch_source()+inherited.inherited.CAPTURE
core.configure=configure
sha=lambda b:hashlib.sha256(b).hexdigest()
FLAGS=list(previous.pack.legacy.base.FLAGS)
SANITIZERS=[('signed-integer-overflow,integer-divide-by-zero,float-cast-overflow'),('address,undefined')]
def sanitizer_flags(sanitizer):
    return [f for f in FLAGS if f!='-fwrapv']+['-fno-wrapv','-fsanitize='+sanitizer,'-fno-sanitize-recover=all']
def capture(flags):
    return inherited.inherited.capture(flags)
ROW_FORMATS=dict(inherited.ROW_FORMATS,**{
 'RA':'RA,case,op,spot,source,damage,xl,xh,yl,yh,spot x,spot y,spot z,dist: one row per P_RadiusAttack call, printed after bombdamage is assigned (p_map.c:1228): the bomb spot (the exploding thing, A_Explode passes the barrel), the bomb source (thingy->target, -1 for NULL), the damage (128 for A_Explode), the BLOCKMAP window (block columns xl..xh, rows yl..yh, p_map.c:1222-1225), the spot position and the computed `dist` (p_map.c:1221: 8388608 = 128 << 16 on the original targets, the MAXRADIUS term wrapped away). The window is walked y outer, x inner (p_map.c:1230-1232)',
 'RC':'RC,case,op,thing,block,position,code,dist,damage: one row per PIT_RadiusAttack call, in walk order: the candidate, its BLOCKMAP list (block = y * bmapwidth + x, the blocklinks index the iterator walks), its ordinal among the callbacks of that block (0 = the list head, newest linked; a thing relinked mid-walk changes what follows, p_maputl.c:529-535), and the outcome: 0 not MF_SHOOTABLE (p_map.c:1170), 1 MT_CYBORG/MT_SPIDER (1175), 2 out of range (dist >= bombdamage, 1188), 3 P_CheckSight (thing, bombspot) false (1191), 4 damaged (P_DamageMobj (thing, bombspot, bombsource, bombdamage - dist), 1194; the G rows of that call follow). dist is the floored Chebyshev distance minus the thing radius (p_map.c:1179-1186; -1 for codes 0 and 1), damage is bombdamage - dist for code 4 and 0 otherwise. Printed before the P_DamageMobj call for code 4 and after the P_CheckSight call for codes 3 and 4'})
def run(check,overflow,address):
    observed_trace=capture(FLAGS)
    for enabled,sanitizer in zip((overflow,address),SANITIZERS):
        if enabled:
            audited=capture(sanitizer_flags(sanitizer))
            assert audited['m3d3j_trace.csv']==observed_trace['m3d3j_trace.csv'],sanitizer
    rows=list(csv.reader(io.StringIO(observed_trace['m3d3j_trace.csv'].decode())))
    functions=collections.Counter(r[3]for r in rows if r[0]=='FN')
    full=[r for r in rows if r[0]!='FN']
    case_rows=collections.defaultdict(list)
    for r in full:
        if r[0]!='SEED':case_rows[int(r[1])].append(r)
    last_op={w:max(int(r[2])for r in mine)for w,mine in case_rows.items()}
    ended={int(r[1]):int(r[3])for r in full if r[0]=='MG'and r[3]in('1','2')}
    for which,c in enumerate(core.CASES):assert last_op[which]==c['tics']or which in ended,(c['label'],last_op[which])
    assert {core.CASES[w]['label']for w in ended}=={'player_killed_by_barrel'}and set(ended.values())=={2},ended
    selected={0,1,2,4,8,16,32,48,64,96,128,160,192,256,320,384,448,512,576,640}
    pushes={(i,c['push_at'])for i,c in enumerate(core.CASES)if c['push_at']}
    mover={'ME','MU','MX','MG','SO','MP','SD','XT','MI'}
    interesting={(int(r[1]),int(r[2]))for r in full if (r[0]=='G'and r[3]in{'2','7','10'})or r[0]in{'AR','ST','PN','SL','X','N','SE','KY','KR','KS','KZ','RA'}|mover
                 or(r[0]=='MS'and r[3]in{'0','5'})or(r[0]=='E'and r[3]in('1','2')and r[2]!='0')or(r[0]=='M'and r[7]=='0')or(r[0]=='MT'and r[4]!='0')or(r[0]=='MV'and r[10]!='0')
                 or(r[0]=='DT'and r[5]!='0'and r[7]=='0')}|pushes
    keys={(int(r[1]),int(r[2]))for r in rows if r[0]=='P'and(int(r[2])in selected or tuple(map(int,r[1:3]))in interesting or int(r[2])==last_op[int(r[1])])}
    kept=[r for r in rows if r[0]!='FN'and(r[0]not in{'A','P','AP','R','S','LS'}or tuple(map(int,r[1:3]))in keys)]
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(kept)
    expanded=(SCHEMA+'\n'+text.getvalue()).encode()
    outputs={'m3h9b_trace.dref':pack(expanded)}
    assert unpack(outputs['m3h9b_trace.dref'])==expanded
    text=io.StringIO();writer=csv.writer(text,lineterminator='\n');writer.writerow(['case',*core.CASES[0]]);writer.writerows([i,*c.values()]for i,c in enumerate(core.CASES))
    outputs['m3h9b_cases.csv']=text.getvalue().encode()
    maps=[]
    for i,m in enumerate(core.main.MAPS):
        name=f'm3h9b_geometry_{i}.bin';outputs[name]=m['bytes'];maps.append('tests/reference/'+name)
    outputs['m3h9b_maps.rs']=('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("{Path(p).name}"),\n'for p in maps)+'];\n').encode()
    manifest=json.loads(observed_trace['m3d3j_provenance.json'])
    bodies=manifest['functions']+[['p_pspr.c',n,'void']for n in previous.pack.WEAPON_ACTIONS]
    assert len(bodies)==len({tuple(b)for b in bodies})
    assert all([f,n,t]in bodies for f,n,t in ADDED),'every added body is compiled'
    # ---- observations, all computed from the full trace -------------------------------
    player_ident={w:next(r[5]for r in mine if r[0]=='W'and r[2]=='0'and r[3]=='7')for w,mine in case_rows.items()}
    types={}
    for r in full:
        if r[0]=='E'and r[3]=='1':types.setdefault((r[1],r[4]),int(r[5]))
    records=collections.defaultdict(list);doomed={};options={}
    for r in full:
        if r[0]=='F'and r[3]!='-1':
            records[r[1]].append(r)
            if int(r[10])>int(r[9]):doomed[(r[1],r[9])]=int(r[7]);options[(r[1],r[9])]=int(r[8])
    MTB=MT.index('MT_BARREL');PUFF=MT.index('MT_PUFF');CLIPT=MT.index('MT_CLIP');SHOT=MT.index('MT_TROOPSHOT')
    S=lambda name:ST.index(name)
    def kind(w,i):
        if i=='-1':return'none'
        if i==player_ident[int(w)]:return'player'
        t=types.get((w,i))
        if t==MTB:return'barrel'
        if t==SHOT:return'missile'
        if t==PUFF:return'puff'
        d=doomed.get((w,i))
        if d in geometry.DECORATIONS:return f'decoration:{d}'
        if d in ROSTER:return'monster'
        if d in PICKUPS:return'pickup'
        if t==CLIPT:return'drop'
        return'other'
    def ident_of(w,doomed_kind,x=None,y=None):
        found=[r for r in records[w]if int(r[10])>int(r[9])and int(r[7])==doomed_kind and(x is None or(int(r[4]),int(r[5]))==(x,y))]
        assert len(found)==1,(w,doomed_kind,x,y,len(found))
        return found[0][9]
    def rows_of(mine,tag,**where):
        return [r for r in mine if r[0]==tag and all(r[int(k)]==str(v)for k,v in where.items())]
    def actor(mine,i,op):
        return next(r for r in mine if r[0]=='A'and r[2]==str(op)and r[3]==i)
    def final_actor(mine,i,w):return actor(mine,i,last_op[int(w)])
    def heights(mine,op):
        mh=next(r for r in mine if r[0]=='MH'and r[2]==str(op));n=int(mh[3])
        return [(int(mh[4+3*k]),int(mh[5+3*k]))for k in range(n)]
    def player_row(mine,op):return next(r for r in mine if r[0]=='P'and r[2]==str(op))
    U2=lambda v:int(v)/U
    # ---- every spawned barrel carries the pristine info.c row (M3h9b note §2.1) --------
    barrel_rows=0
    for (w,i),d in doomed.items():
        if d!=BARREL:continue
        a=actor(case_rows[int(w)],i,0);b=geometry.BARREL
        flags=b['flags']|(geometry.MF_AMBUSH if options[(w,i)]&8 else 0)
        assert int(a[5])==MTB and int(a[13])==S(b['spawnstate'])and 1<=int(a[14])<=6 and int(a[12])==b['health'],(w,i,a)
        assert int(a[17])==b['radius']*U and int(a[16])==b['height']*U and int(a[15])==flags and a[20]=='-1'and int(a[18])==b['reactiontime'],(w,i,a)
        assert int(a[8])==int(a[22]),(w,i,'spawned on its floor')
        barrel_rows+=1
    # ---- every explosion: the wrapped dist, the exact 3 x 3 window, y-outer/x-inner walk,
    # and every damaged candidate's P_DamageMobj call -----------------------------------
    explosions=[];chain_timings=[]
    for w,mine in case_rows.items():
        m=core.main.MAPS[core.CASES[w]['map']];ox,oy,bw,bh=geometry.origin(m)
        for k,r in enumerate(mine):
            if r[0]!='RA':continue
            spot,source,damage=r[3],r[4],int(r[5]);xl,xh,yl,yh=map(int,r[6:10]);sx,sy,sz,dist=map(int,r[10:14])
            assert kind(str(w),spot)=='barrel'and damage==geometry.EXPLOSION_DAMAGE and dist==geometry.wrapped_dist(damage)==damage<<16,r
            assert (xl,xh,yl,yh)==geometry.window(sx,sy,m,dist)and xh-xl==2 and yh-yl==2,r
            action=mine[k-1];assert action[0]=='E'and action[3]=='5'and action[4]==spot and action[5]==str(EXPLODE_ACTION),action
            entered=[x for x in mine[:k]if x[0]=='E'and x[3]=='4'and x[4]==spot];assert entered[-1][5]==str(S('S_BEXP4')),entered[-1]
            targets=[x for x in mine[:k]if x[0]=='E'and x[3]=='6'and x[4]==spot];assert (targets[-1][5]if targets else'-1')==source,(r,'bomb source is the barrel target')
            # The explosion's span: every row until the next thinker runs (E row 0) or the
            # next explosion starts.
            span=[];j=k+1
            while j<len(mine)and mine[j][0]!='RA'and not(mine[j][0]=='E'and mine[j][3]=='0'):
                span.append(mine[j]);j+=1
            rc=[x for x in span if x[0]=='RC']
            blocks=[int(x[4])for x in rc];assert blocks==sorted(blocks),(r,'y outer, x inner')
            window={y*bw+x for y in range(yl,yh+1)for x in range(xl,xh+1)if 0<=x<bw and 0<=y<bh};assert set(blocks)<=window
            for b in set(blocks):assert [int(x[5])for x in rc if int(x[4])==b]==list(range(blocks.count(b)))
            for x in rc:
                code=int(x[6])
                if code==4:
                    g=span[span.index(x)+1];assert g[0]=='G'and g[3]=='2'and g[4:8]==[x[3],spot,source,x[8]]and int(x[8])==damage-int(x[7])and 0<=int(x[7])<damage,(x,g)
                if code==3:
                    q=[y for y in span[:span.index(x)]if y[0]=='Q'];assert q and q[-1][4]=='0'and 0<=int(x[7])<damage,(x,q)
                if code==2:assert int(x[7])>=damage
                if code in(0,1):assert x[7]=='-1'and x[8]=='0'
            # Draw accounting: P_RadiusAttack, PIT_RadiusAttack, P_CheckSight and
            # P_BlockThingsIterator draw nothing; every draw of the span is explained by a
            # combat, pursuit, attack, look or missile row, or is a spawn's lastlook.
            draws=sum(x[0]=='E'and x[3]=='8'for x in span)
            explained=sum((x[0]=='G'and x[3]=='3')or(x[0]=='H'and x[3]=='3')or(x[0]=='J'and x[3]=='1')or(x[0]=='MS'and x[3]=='2')or(x[0]=='K'and x[3]=='9')for x in span)
            spawns=sum(x[0]=='E'and x[3]=='1'for x in span);assert draws==explained+spawns,(r,draws,explained,spawns)
            explosions.append(dict(case=w,op=int(r[2]),spot=spot,source=source,window=[xl,xh,yl,yh],spot_xyz=[sx,sy,sz],candidates=[[x[3],kind(str(w),x[3]),int(x[4]),int(x[5]),int(x[6]),int(x[7]),int(x[8])]for x in rc],draws=draws))
        # Death-chain timing (M3h9b note §2.5) for every barrel that reached S_BEXP4.
        for k,r in enumerate(mine):
            if not(r[0]=='G'and r[3]=='7'and kind(str(w),r[4])=='barrel'):continue
            b=r[4];T=int(r[2]);t0=int(next(x for x in mine[k:]if x[0]=='G'and x[3]=='8'and x[4]==b)[5]);assert 2<=t0<=5
            killer=[x for x in mine[:k]if x[0]=='RA'and x[2]==r[2]]
            # d = 1 when the killing code runs after the victim's thinker in tic T: the
            # victim precedes the exploding barrel in thinker (lump) order, or the kill is
            # a missile's (spawned after every map thing).
            hit=[x for x in mine[:k]if x[0]=='G'and x[3]=='2'and x[4]==b][-1]
            d=1 if (killer and int(b)<int(killer[-1][3]))or kind(str(w),hit[5])=='missile'else 0
            states={ST[int(x[5])]:int(x[2])for x in mine[k:]if x[0]=='E'and x[3]=='4'and x[4]==b}
            gibbed='S_GIBS'in states
            expect={'S_BEXP2':T+d+t0-1,'S_BEXP3':T+d+t0+4,'S_BEXP4':T+d+t0+9,'S_BEXP5':T+d+t0+19,'S_NULL':T+d+t0+29}
            for name,at in expect.items():
                if at<=last_op[w]and not gibbed:assert states.get(name)==at,(core.CASES[w]['label'],b,name,states,expect)
            if 'S_BEXP2'in states:
                scream=[x for x in mine if x[0]=='E'and x[3]=='5'and x[4]==b and x[5]==str(core.legacy.actors.ACTIONS.index('A_Scream'))];assert scream and scream[0][2]==str(states['S_BEXP2'])
            if 'S_NULL'in states:assert rows_of(mine,'E',**{'2':states['S_NULL'],'3':2,'4':b})
            for op in range(T,min(states.get('S_NULL',last_op[w]+1),last_op[w]+1)):
                if gibbed and op>=states['S_GIBS']:break
                a=actor(mine,b,op)if any(x[0]=='A'and x[2]==str(op)and x[3]==b for x in mine)else None
                if a and op>T:
                    f=int(a[15]);assert f&geometry.MF_SOLID and not f&geometry.MF_SHOOTABLE and f&geometry.MF_CORPSE and f&geometry.MF_DROPOFF and int(a[16])==(42*U)>>2,(b,op,a)
            chain_timings.append(dict(case=w,barrel=b,killed=T,t0=t0,d=d,scream=states.get('S_BEXP2'),explode=states.get('S_BEXP4'),removed=states.get('S_NULL'),gibbed=states.get('S_GIBS')))
    # ---- per-scenario checks ------------------------------------------------------------
    def draws_in(mine,op):return sum(r[0]=='E'and r[3]=='8'and r[2]==str(op)for r in mine)
    def damage_rows(mine,target,**where):return [r for r in rows_of(mine,'G',**{'3':2,'4':target})if all(r[int(k)]==str(v)for k,v in where.items())]
    def first_ra(mine,spot=None):return [r for r in mine if r[0]=='RA'and(spot is None or r[3]==spot)]
    def rc_of(mine,ra):
        k=mine.index(ra);out=[]
        for x in mine[k+1:]:
            if x[0]=='RA'or(x[0]=='E'and x[3]=='0')or x[2]!=ra[2]:break
            if x[0]=='RC':out.append(x)
        return out
    def check_barrel_wounded(w,mine,m,p):
        b=ident_of(w,BARREL);hit=damage_rows(mine,b);assert len(hit)==1 and hit[0][5]==hit[0][6]==p
        op=hit[0][2];wk=rows_of(mine,'WK',**{'2':op});assert wk and wk[0][4]=='1','accurate: refire 0'
        assert draws_in(mine,op)==6,'damage, puff jitter 2, puff lastlook, puff tics, pain'
        assert rows_of(mine,'G',**{'2':op,'3':3,'4':1})and not rows_of(mine,'G',**{'3':6,'4':b}),'pain draw, painchance 0: no pain state'
        assert rows_of(mine,'G',**{'2':op,'3':10,'4':b,'5':p})and rows_of(mine,'G',**{'2':op,'3':4,'4':b})
        puffs=[r for r in mine if r[0]=='E'and r[3]=='1'and r[2]==op and int(r[5])==PUFF];assert len(puffs)==1 and not[r for r in mine if r[0]=='E'and r[3]=='1'and int(r[5])==MT.index('MT_BLOOD')]
        a=final_actor(mine,b,w);assert a[20]==p and int(a[19])==100 and int(a[12])==20-int(hit[0][7])and ST[int(a[13])]in('S_BAR1','S_BAR2')
        return dict(shot_op=int(op),damage=int(hit[0][7]),draws=6,target='player',threshold=100,momx=int(rows_of(mine,'G',**{'3':4,'4':b})[0][7]),puff=1,blood=0)
    def check_barrel_killed(w,mine,m,p):
        b=ident_of(w,BARREL);hits=damage_rows(mine,b);assert len(hits)==2 and all(h[5]==h[6]==p for h in hits)
        kill=rows_of(mine,'G',**{'3':7,'4':b});assert len(kill)==1 and kill[0][2]==hits[1][2]and int(kill[0][5])==S('S_BEXP')
        op=kill[0][2];assert draws_in(mine,op)==6,'damage, puff 4, death tics; no spread (accurate), no fall-forward, no pain'
        assert not rows_of(mine,'G',**{'2':op,'3':3,'4':1})and not rows_of(mine,'G',**{'2':op,'3':3,'4':0})
        t=next(x for x in chain_timings if x['case']==int(w)and x['barrel']==b);assert t['removed']is not None
        assert not rows_of(mine,'E',**{'2':t['scream'],'3':8}),'A_Scream for sfx_barexp draws nothing (no other draw in that tic)'
        ra=first_ra(mine);assert len(ra)==1 and ra[0][4]==p;rc=rc_of(mine,ra[0]);assert [(x[3],x[6])for x in rc]==[(b,'0')],rc
        skips=rows_of(mine,'XT',**{'3':0,'5':b});assert skips and int(skips[0][2])>int(op)
        return dict(hits=[[int(h[2]),int(h[7])]for h in hits],killed=int(op),t0=t['t0'],scream=t['scream'],explode=t['explode'],removed=t['removed'],candidates=len(rc),kill_draws=6,aim_skips_after_death=len(skips))
    def check_fist(w,mine,m,p,survives):
        b=ident_of(w,BARREL);hits=damage_rows(mine,b);assert len(hits)==1 and hits[0][7]=='20'and hits[0][6]==p
        op=hits[0][2];assert rows_of(mine,'WK',**{'2':op,'3':0}),'A_Punch'
        assert not rows_of(mine,'G',**{'3':10,'4':b}),'killed in one blow: no retarget'
        puff=[r for r in mine if r[0]=='E'and r[3]=='1'and r[2]==op and int(r[5])==PUFF][0][4]
        assert rows_of(mine,'E',**{'2':op,'3':4,'4':puff,'5':S('S_PUFF3')}),'S_PUFF3 at MELEERANGE'
        ra=first_ra(mine);assert len(ra)==1 and ra[0][4]=='-1'
        hurt=damage_rows(mine,p,**{'5':b,'6':-1});assert len(hurt)==1
        after=player_row(mine,last_op[int(w)]);assert after[6]=='-1','attacker NULL'
        if survives:assert int(after[4])>0
        else:
            assert rows_of(mine,'G',**{'3':7,'4':p})and after[5]=='2'and after[8]=='1','killed by a NULL source: frags[0]++, then the reborn request'
            death=int(rows_of(mine,'G',**{'3':7,'4':p})[0][2]);angles={actor(mine,p,op)[26]for op in range(death,last_op[int(w)]+1)if any(x[0]=='A'and x[2]==str(op)and x[3]==p for x in mine)};assert len(angles)==1,'P_DeathThink: no attacker, no turn'
        return dict(punch_op=int(op),damage=20,barrel_target='NULL',bomb_source='NULL',player_damage=int(hurt[0][7]),player_health=int(after[4]),playerstate=int(after[5]),frags=int(after[8]),attacker='NULL')
    def check_autoaim(w,mine,m,p):
        b0=ident_of(w,BARREL,0,0);b1=ident_of(w,BARREL,240,0)
        kill=int(rows_of(mine,'G',**{'3':7,'4':b0})[0][2])
        aims=[r for r in mine if r[0]=='J'and r[3]=='5'and r[4]==p and r[5]!='-1']
        before=[r for r in aims if int(r[2])<=kill];after=[r for r in aims if int(r[2])>kill]
        assert before and all(r[5]==b0 for r in before)and after and all(r[5]==b1 for r in after)
        skips=[r for r in rows_of(mine,'XT',**{'3':0,'5':b0})if int(r[2])>kill];assert skips
        return dict(aim_targets_alive=len(before),killed=kill,aim_targets_after=[int(r[2])for r in after],aim_skips_of_dying_barrel=len(skips),shoot_skips=len([r for r in rows_of(mine,'XT',**{'3':1,'5':b0})]))
    def check_chain(w,mine,m,p):
        ids={k:ident_of(w,BARREL,x,y)for k,(x,y)in dict(B0=(0,-4),B1=(28,-4),B2=(14,22),B3=(14,-30),B4=(40,20),W=(0,-124),Q=(0,132)).items()}
        ra=first_ra(mine);first=ra[0];assert first[3]==ids['B0']and first[4]==p
        rc=rc_of(mine,first);visited={x[3]for x in rc}
        assert ids['Q']not in visited
        sx,sy=int(first[10]),int(first[11]);qa=actor(mine,ids['Q'],int(first[2])-1);qx,qy=int(qa[6]),int(qa[7])
        cheb=max(abs(qx-sx),abs(qy-sy));qdist=(cheb-10*U)>>16;assert qdist<128,'in range had it been visited'
        ox,oy,_,_=geometry.origin(m);qrow=geometry.block_of(qy,oy);assert qrow>int(first[9]),'outside the wrapped window'
        intended=geometry.window(sx,sy,m,(128+32)<<16);assert intended[2]<=qrow<=intended[3],'inside the intended 160-unit window'
        killed=[x[3]for x in rc if x[6]=='4'and rows_of(mine,'G',**{'2':first[2],'3':7,'4':x[3]})]
        assert set(killed)>={ids[k]for k in('B1','B2','B3','B4')}
        wr=[x for x in rc if x[3]==ids['W']];assert len(wr)==1 and wr[0][6]=='4'and int(wr[0][8])<20
        second=[r for r in ra[1:]if r[2]==ra[1][2]]
        same_tic=collections.defaultdict(list)
        for r in ra:same_tic[r[2]].append(int(r[3]))
        assert all(v==sorted(v)for v in same_tic.values()),'explosions of one tic in thinker (lump) order'
        return dict(first_explosion=int(first[2]),first_window=list(map(int,first[6:10])),killed_by_first=sorted(killed,key=int),survivor_W=dict(damage=int(wr[0][8]),dist=int(wr[0][7])),
                    Q=dict(chebyshev=cheb/U,dist=qdist,row=qrow,window_rows=[int(first[8]),int(first[9])],intended_rows=[intended[2],intended[3]],visited_by_first=False,
                           later=[[int(x[2]),int(x[6]),int(x[8])]for r in ra[1:]for x in rc_of(mine,r)if x[3]==ids['Q']]),
                    explosions=[[int(r[2]),r[3],r[4]]for r in ra])
    def check_player_hurt(w,mine,m,p):
        armor=[r for r in mine if r[0]=='AR'];ra=first_ra(mine);assert len(ra)==2
        a,b=ra;assert a[4]==p and b[4]=='-1'
        hurt=[x for x in rc_of(mine,b)if x[3]==p];assert len(hurt)==1 and hurt[0][6]=='4'
        ar=[r for r in armor if r[2]==b[2]];assert ar and int(ar[0][5])>0,'armor absorbed part'
        after=player_row(mine,int(b[2]));assert after[6]=='-1','attacker NULL'
        pain=[r for r in mine if r[2]==b[2]and r[0]=='G'and r[3]=='3'and r[4]=='1'];assert pain
        return dict(armor_pickup=bool(rows_of(mine,'E',**{'2':1,'3':2})),first=dict(op=int(a[2]),source='player',player_damage=sum(int(x[8])for x in rc_of(mine,a)if x[3]==p)),
                    second=dict(op=int(b[2]),source='NULL',damage=int(hurt[0][8]),saved=int(ar[0][5]),health=int(after[4]),attacker='NULL',pain_draw=int(pain[-1][5])))
    def check_own(w,mine,m,p):
        ra=first_ra(mine);assert len(ra)==1 and ra[0][4]==p
        hurt=[x for x in rc_of(mine,ra[0])if x[3]==p];assert len(hurt)==1 and hurt[0][6]=='4'
        after=player_row(mine,int(ra[0][2]));assert after[6]==p,'attacker = the player'
        return dict(explosion=int(ra[0][2]),damage=int(hurt[0][8]),attacker='player',health=int(after[4]))
    def check_fall(w,mine,m,p):
        z=ident_of(w,POSSESSED);ra=first_ra(mine);assert len(ra)==1 and ra[0][4]==p
        hit=damage_rows(mine,z);assert len(hit)==1;dmg=int(hit[0][7]);assert 20<dmg<40
        k=mine.index(hit[0]);fall=[x for x in mine[k:k+4]if x[0]=='G'and x[3]=='3'and x[4]=='0'];assert len(fall)==1
        za=actor(mine,z,int(ra[0][2])-1);assert int(za[8])-int(ra[0][12])>64*U
        thrust=[x for x in mine[k:k+6]if x[0]=='G'and x[3]=='4'][0];base=dmg*(U>>3)*100//100
        assert int(thrust[6])==(4*base if int(fall[0][5])&1 else base),'odd draw: ang += ANG180, thrust *= 4 (p_inter.c:820-827)'
        assert rows_of(mine,'G',**{'3':7,'4':z})and rows_of(mine,'ST',**{'3':1,'4':z})
        drop=[r for r in mine if r[0]=='E'and r[3]=='1'and r[2]==ra[0][2]and int(r[5])==CLIPT];assert len(drop)==1
        return dict(damage=dmg,height_above=(int(za[8])-int(ra[0][12]))//U,fall_draw=int(fall[0][5]),flipped=int(fall[0][5])&1,thrust=int(thrust[6]),killcount_credit='player',clip_drop=1)
    def check_kills_monsters(w,mine,m,p):
        ra=first_ra(mine);assert len(ra)==2 and ra[0][4]==p and ra[1][4]=='-1'
        rc=rc_of(mine,ra[1]);dead=[x[3]for x in rc if x[6]=='4'and rows_of(mine,'G',**{'2':ra[1][2],'3':7,'4':x[3]})]
        imp=ident_of(w,IMP);assert imp not in dead and [x for x in rc if x[3]==imp and x[6]=='4']
        assert not rows_of(mine,'G',**{'3':10,'4':imp}),'NULL source: no retarget'
        assert rows_of(mine,'G',**{'2':ra[1][2],'3':3,'4':1})
        credits=[r for r in rows_of(mine,'ST',**{'2':ra[1][2],'3':1})];assert len(credits)==len(dead)==2
        drops=[r for r in mine if r[0]=='E'and r[3]=='1'and r[2]==ra[1][2]and int(r[5])==CLIPT];assert len(drops)==2
        return dict(first=int(ra[0][2]),second=int(ra[1][2]),second_source='NULL',killed=[kind(w,d)for d in dead],players0_credits=len(credits),clip_drops=2,imp_damage=int(next(x for x in rc if x[3]==imp)[8]),imp_retargets=0)
    def check_credit(w,mine,m,p):
        ra=first_ra(mine);assert len(ra)==1 and ra[0][4]==p;z=ident_of(w,POSSESSED)
        assert rows_of(mine,'G',**{'2':ra[0][2],'3':7,'4':z})and rows_of(mine,'ST',**{'2':ra[0][2],'3':1,'4':z})
        ls=next(r for r in mine if r[0]=='LS'and r[2]==ra[0][2]);assert ls[6]=='1'
        return dict(explosion=int(ra[0][2]),source='player',killcount=1)
    def check_wake(w,mine,m,p):
        ra=first_ra(mine);assert len(ra)==1 and ra[0][4]==p
        k=mine.index(ra[0]);rc=rc_of(mine,ra[0]);seen=[x[3]for x in rc]
        twice=[t for t in set(seen)if seen.count(t)>1];assert twice
        relinked=[];span=mine[k:mine.index(rc[-1])]
        for x in span:
            if x[0]=='L':relinked.append(x[3])
        assert relinked
        mover=relinked[0];entry=[x for x in span if x[0]=='E'and x[3]=='4'and x[4]==mover];assert entry
        pain=[x for x in span if x[0]=='G'and x[3]=='3'and x[4]=='1'];assert any(int(x[5])>=200 for x in pain)
        return dict(explosion=int(ra[0][2]),walk=[[x[3],kind(w,x[3]),int(x[4]),int(x[5]),int(x[6]),int(x[8])]for x in rc],relinked_mid_walk=relinked,visited_twice=twice,
                    damage_to_twice_visited=[int(x[8])for x in rc if x[3]==twice[0]],killed=[x[4]for x in mine if x[0]=='G'and x[3]=='7'and x[2]==ra[0][2]])
    def check_window(w,mine,m,p):
        ra=first_ra(mine);assert len(ra)==1;hurt=[x for x in rc_of(mine,ra[0])if x[3]==p];assert hurt and hurt[0][6]=='4'
        return dict(explosion=int(ra[0][2]),player_damage=int(hurt[0][8]),through='window gap x in [0, 16], y in [-16, 16]')
    def check_step(w,mine,m,p):
        ra=first_ra(mine);assert len(ra)==1 and ra[0][4]=='-1';hurt=[x for x in rc_of(mine,ra[0])if x[3]==p];assert len(hurt)==1 and hurt[0][6]=='3'
        curb=[i for i,l in enumerate(m['lines'])if {m['vertices'][l['a']],m['vertices'][l['b']]}=={(-320,96),(-320,224)}];assert len(curb)==1
        blocks=[x for x in mine if x[0]=='V'and x[2]==ra[0][2]and x[4]=='7'and x[5]==str(curb[0])];assert blocks
        return dict(explosion=int(ra[0][2]),player_dist=int(hurt[0][7]),outcome='sight blocked',blocking_line=curb[0],curb_height=12,window='bottom 10.5 units of the dead barrel')
    def check_pushed(w,mine,m,p):
        b=ident_of(w,BARREL);refused=rows_of(mine,'M',**{'4':b,'7':0});assert refused and refused[0][12]=='2'
        op=int(refused[0][2]);a=actor(mine,b,op);assert a[9]=='0'and a[10]=='0'
        return dict(blocked_op=op,move_failure='line',momentum_after=[0,0])
    def check_lift(w,mine,m,p):
        b=ident_of(w,BARREL);mx=rows_of(mine,'MX',**{'3':b,'6':88});assert len(mx)==1
        me=[r for r in rows_of(mine,'ME',**{'3':3})if r[2]==mx[0][2]];assert me
        lift=next(k for k,a in enumerate(m['sector_attrs'])if a['tag']==1)
        down=rows_of(mine,'MV',**{'3':lift,'4':0,'5':-1});assert down
        return dict(crossing_op=int(mx[0][2]),line=int(mx[0][4]),plat_created=True,lift_steps_down=len(down),lift_floor_final=heights(mine,last_op[int(w)])[lift][0]//U)
    def check_dropoff(w,mine,m,p):
        pb=ident_of(w,BARREL,300,190);tb=ident_of(w,BARREL,330,150);ra=first_ra(mine);assert ra[0][3]==pb
        assert rows_of(mine,'G',**{'2':ra[0][2],'3':7,'4':tb})
        second=[r for r in ra if r[3]==tb];assert len(second)==1 and int(second[0][12])<32*U
        moves=[r for r in rows_of(mine,'M',**{'4':tb,'7':1})if int(r[2])>int(ra[0][2])];assert any(int(r[9])-int(r[10])>24*U or int(r[8])<32*U for r in moves)
        return dict(pusher_explosion=int(ra[0][2]),fallen_explosion=int(second[0][2]),fallen_z=int(second[0][12])/U,fallen_xy=[U2(second[0][10]),U2(second[0][11])])
    def check_door_gibs(w,mine,m,p):
        pb=ident_of(w,BARREL,-90,-55);tb=ident_of(w,BARREL,-12,-40)
        ra=first_ra(mine);assert len(ra)==1 and ra[0][3]==pb
        assert rows_of(mine,'G',**{'2':ra[0][2],'3':7,'4':tb})
        gib=rows_of(mine,'MT',**{'3':tb,'4':1});assert gib
        assert rows_of(mine,'E',**{'3':4,'4':tb,'5':S('S_GIBS')})and not rows_of(mine,'E',**{'3':4,'4':tb,'5':S('S_BEXP4')})
        assert not rows_of(mine,'E',**{'3':2,'4':tb})and final_actor(mine,tb,w)[4]=='1'
        door=next(k for k,(f,c)in enumerate(m['heights'])if(f,c)==(0,124));assert heights(mine,last_op[int(w)])[door][1]==0
        return dict(pusher_explosion=int(ra[0][2]),gibbed_op=int(gib[0][2]),barrel_killed=int(ra[0][2]),never_exploded=True,never_removed=True,door_closed=True)
    def check_door_reverse(w,mine,m,p):
        b=ident_of(w,BARREL);nofit=rows_of(mine,'MT',**{'3':b,'4':4});assert nofit and nofit[0][8]=='1'
        door=next(k for k,(f,c)in enumerate(m['heights'])if(f,c)==(0,124))
        up=[r for r in rows_of(mine,'MV',**{'3':door,'4':1,'5':1})if int(r[2])>int(nofit[0][2])];assert up
        return dict(nofit_op=int(nofit[0][2]),reversed=True)
    def check_fireball_on_dying(w,mine,m,p):
        b=ident_of(w,BARREL);dt=[r for r in mine if r[0]=='DT'and r[4]==b and r[5]=='1'and kind(w,r[3])=='missile'];assert dt and dt[0][7]=='0'
        assert not[r for r in damage_rows(mine,b)if kind(w,r[5])=='missile']
        assert rows_of(mine,'MS',**{'2':dt[0][2],'3':0,'4':dt[0][3]})
        return dict(contact_op=int(dt[0][2]),explodes=True,damage=0)
    def check_dense(w,mine,m,p,trace):
        things=LAYOUTS[core.CASES[int(w)]['layout']];recs=records[w];assert len(recs)==len(things)
        spawned=[t for r,t in zip(recs,things)if int(r[10])>int(r[9])];assert len(spawned)==len(things)
        by=collections.Counter('pickup'if t[3]in PICKUPS else'barrel'if t[3]==BARREL else'monster'if t[3]in ROSTER else'decoration'for t in spawned)
        assert by=={'pickup':MAX_PICKUPS,'barrel':MAX_BARRELS,'monster':6,'decoration':27},by
        expected=int(core.CASES[int(w)]['seed'])+sum(2 if t[3]in SPAWN_TICS_DRAW else 1 for t in spawned)+1
        a0=actor(mine,p,0);assert int(a0[25])==expected
        out=dict(spawned=dict(by),prndindex_after_load=expected)
        if trace:
            shots=[r for r in mine if r[0]=='TI']
            counts=[int(r[4])for r in shots];assert counts and max(counts)<128
            shoot=[r for r in mine if r[0]=='TR'and r[4]=='1'];assert len(shoot)==1
            k=mine.index(shoot[0]);ti=next(x for x in mine[k:]if x[0]=='TI');i_rows=[x for x in mine[k:mine.index(ti)]if x[0]=='I']
            assert int(ti[4])==len(i_rows)
            # Every pickup and decoration of the row is an intercept (no flag filter,
            # p_maputl.c:616-674), and so is the shooter's own box at frac 0 (skipped later
            # by PTR_ShootTraverse, p_map.c:975-977).
            assert sum(x[4]=='0'for x in i_rows)==MAX_PICKUPS+27+1 and i_rows[0][4:7]==['0',p,'0'],i_rows[:2]
            out.update(shoot_intercepts=int(ti[4]),shoot_lines=sum(x[4]=='1'for x in i_rows),shoot_things=sum(x[4]=='0'for x in i_rows),aim_intercepts=[int(r[4])for r in shots[:-1]],intercept_limit=128)
        return out
    def check_angles(w,mine,m,p):
        recs=records[w];out=[]
        for r in recs:
            a=actor(mine,r[9],0);want=geometry.angle_of(int(r[6]));assert int(a[26])==want,(r,a[26],want)
            out.append([int(r[6]),int(a[26])])
        assert sorted(x[0]for x in out)==sorted(__import__('m3h9b_cases').ANGLES)
        return dict(records=len(out),angles=out)
    def check_embedded(w,mine,m,p):
        imp=ident_of(w,IMP);lamp=ident_of(w,2028);bonus=ident_of(w,2014)
        moves=[r for r in rows_of(mine,'M',**{'4':imp,'7':1})];assert moves and all(int(r[5])-20*U>=-512*U for r in moves)
        assert int(final_actor(mine,imp,w)[6])>-476*U
        assert rows_of(mine,'E',**{'3':2,'4':bonus})
        return dict(spawned=3,imp_first_accepted_move=int(moves[0][2]),imp_final_x=U2(final_actor(mine,imp,w)[6]),bonus_collected=int(rows_of(mine,'E',**{'3':2,'4':bonus})[0][2]))
    def check_infight(w,mine,m,p):
        z=ident_of(w,POSSESSED,300,0);b=ident_of(w,BARREL);imp=ident_of(w,IMP);z2=ident_of(w,POSSESSED,40,-80)
        hits=damage_rows(mine,b);assert hits and hits[0][5]==hits[0][6]==z,'first wound: the Zombieman hitscan (inflictor and source)'
        assert rows_of(mine,'G',**{'2':hits[0][2],'3':10,'4':b,'5':z}),'barrel target = the Zombieman, threshold 100'
        ra=first_ra(mine);assert len(ra)==1 and ra[0][4]==z
        rc=rc_of(mine,ra[0]);assert rows_of(mine,'G',**{'2':ra[0][2],'3':10,'4':imp,'5':z}),'the Imp retargets the shooter'
        assert rows_of(mine,'G',**{'2':ra[0][2],'3':7,'4':z2})and rows_of(mine,'ST',**{'2':ra[0][2],'3':1,'4':z2}),'players[0] credit: source->player is NULL'
        own=[x for x in rc if x[3]==z];assert own and own[0][6]=='4'and int(actor(mine,z,int(ra[0][2]))[12])>0,'no self-exclusion for the bomb source'
        return dict(first_wound=int(hits[0][2]),wounds=[[int(h[2]),int(h[7])]for h in hits],explosion=int(ra[0][2]),bomb_source='Zombieman',
                    candidates=[[x[3],kind(w,x[3]),int(x[6]),int(x[8])]for x in rc],imp_retarget='Zombieman',shooter_self_damage=int(own[0][8]),credit='players[0] (monster source)')
    def check_fireball_kills(w,mine,m,p):
        imp=ident_of(w,IMP);b=ident_of(w,BARREL);z=ident_of(w,POSSESSED)
        hits=damage_rows(mine,b);assert len(hits)>=2 and all(kind(w,h[5])=='missile'and h[6]==imp for h in hits)
        for h in hits:
            k=mine.index(h);assert any(x[0]=='MS'and x[3]=='2'and x[4]=='3'for x in mine[k-3:k]),'the missile damage draw precedes P_DamageMobj'
        assert rows_of(mine,'G',**{'2':hits[0][2],'3':10,'4':b,'5':imp})
        ra=first_ra(mine);assert len(ra)==1 and ra[0][4]==imp
        assert rows_of(mine,'G',**{'2':ra[0][2],'3':7,'4':z})and rows_of(mine,'ST',**{'2':ra[0][2],'3':1,'4':z})
        return dict(wounds=[[int(h[2]),int(h[7])]for h in hits],barrel_target='Imp',explosion=int(ra[0][2]),bomb_source='Imp',zombieman_killed=True,credit='players[0] (monster source)')
    CHECKS={'barrel_wounded':check_barrel_wounded,'barrel_killed':check_barrel_killed,'fist_kills_barrel':lambda w,mine,m,p:check_fist(w,mine,m,p,True),
            'player_killed_by_barrel':lambda w,mine,m,p:check_fist(w,mine,m,p,False),'autoaim_barrel':check_autoaim,'chain_reaction':check_chain,
            'player_hurt_by_barrel':check_player_hurt,'player_hurt_by_own_barrel':check_own,'fall_forward_draw':check_fall,
            'barrel_kills_monsters':check_kills_monsters,'barrel_infight':check_infight,'player_credit':check_credit,'monster_wakes_mid_walk':check_wake,
            'radius_through_window':check_window,'radius_blocked_by_step':check_step,'barrel_pushed_into_wall':check_pushed,'barrel_slides_over_lift_line':check_lift,
            'dying_barrel_dropoff':check_dropoff,'door_gibs_dying_barrel':check_door_gibs,'door_reverses_on_live_barrel':check_door_reverse,
            'fireball_kills_barrel':check_fireball_kills,'fireball_on_dying_barrel':check_fireball_on_dying,
            'dense_pickups':lambda w,mine,m,p:check_dense(w,mine,m,p,False),'intercept_heavy_trace':lambda w,mine,m,p:check_dense(w,mine,m,p,True),
            'all_octants':check_angles,'embedded_spawns':check_embedded}
    assert set(CHECKS)=={c['label']for c in core.CASES}
    per_case=[]
    for which,c in enumerate(core.CASES):
        mine=case_rows[which];m=core.main.MAPS[c['map']];w=str(which);p=player_ident[which]
        observed_facts=CHECKS[c['label']](w,mine,m,p)
        player=[r for r in mine if r[0]=='P'];spawned=[(i,d)for(cw,i),d in doomed.items()if cw==w]
        per_case.append(dict(label=c['label'],room=ROOM_NAMES[c['room']],tics=c['tics'],last_op=last_op[which],seed=c['seed'],
          map=dict(linedefs=len(m['lines']),sectors=len(m['heights']),subsectors=len(m['leaves']),nodes=len(m['nodes']),cells=m['cells'],blockmap_origin=list(geometry.origin(m)[:2]),
                   blockmap_size=list(geometry.origin(m)[2:]),line_specials=sorted(sp for sp,_ in m['line_specials']if sp),sector_specials=sorted(a['special']for a in m['sector_attrs']if a['special'])),
          things=dict(records=len(records[w]),spawned=len(spawned),barrels=sum(d==BARREL for _,d in spawned),monsters=sum(d in ROSTER for _,d in spawned),pickups=sum(d in PICKUPS for _,d in spawned),
                      decorations=sum(d in geometry.DECORATIONS for _,d in spawned)),
          observed=observed_facts,
          explosions=[dict(op=e['op'],spot=e['spot'],source=e['source'],window=e['window'],candidates=e['candidates'],draws=e['draws'])for e in explosions if e['case']==which],
          barrel_deaths=[dict(barrel=t['barrel'],killed=t['killed'],t0=t['t0'],d=t['d'],scream=t['scream'],explode=t['explode'],removed=t['removed'],gibbed=t['gibbed'])for t in chain_timings if t['case']==which],
          gameplay_draws=sum(r[0]=='E'and r[3]=='8'and r[2]!='0'for r in mine),kills=sum(r[0]=='G'and r[3]=='7'for r in mine),
          player_final=dict(health=int(player[-1][4]),playerstate=int(player[-1][5]),attacker=player[-1][6],frags=int(player[-1][8]))if player else None))
    # Rust-only lanes, built by the same builder and never run in C: an embedded barrel
    # (its box overlapping a one-sided wall: refused by the S' spawn rule, decision F3) and
    # a trace lane whose 2,048-unit pistol trace would collect more than 128 intercepts
    # (the C harness aborts before the 129th entry, so the lane cannot run in C).
    from m3h9b_cases import embedded_barrel_lane,intercept_lane
    lanes={}
    for name,(room,things,note) in dict(embedded_barrel=embedded_barrel_lane(),intercepts129=intercept_lane()).items():
        sectors,portals,reject,tree,segment,lines=room
        lm=geometry.room(sectors,portals=portals,reject=reject,tree=tree,segment=segment,lines=lines)
        lm['lumps']['THINGS']=b''.join(__import__('struct').pack('<5h',*t)for t in things)
        data=__import__('generate_m3b').wad(list(lm['lumps'].items()))
        outputs[f'm3h9b_{name}.bin']=data
        lanes[name]=dict(path=f'tests/reference/m3h9b_{name}.bin',linedefs=len(lm['lines']),things=len(things),note=note)
    inventory=dict(scenarios=len(core.CASES),scripted_tics=sum(c['tics']for c in core.CASES),executed_tics=sum(last_op.values()),captures=len(keys),rooms=len(ROOMS),maps=len(core.main.MAPS),
      selected_original_bodies=len(bodies),new_genuine_bodies=len(ADDED),exercised_unique_functions=len(functions),function_entries=sum(functions.values()),
      new_body_entries={f'{f}:{n}':functions[f'{f}:{n}']for f,n,_ in ADDED},
      barrel_spawns=barrel_rows,explosions=len(explosions),radius_candidates=sum(len(e['candidates'])for e in explosions),
      candidate_outcomes=dict(sorted(collections.Counter(('not_shootable','boss','out_of_range','sight_blocked','damaged')[x[4]]for e in explosions for x in e['candidates']).items())),
      barrel_deaths=len(chain_timings),barrel_gibs=sum(t['gibbed']is not None for t in chain_timings),
      thing_records=sum(len(v)for v in records.values()),thing_spawns=len(doomed),gameplay_draws=sum(r[0]=='E'and r[3]=='8'and r[2]!='0'for r in full),
      kills=sum(r[0]=='G'and r[3]=='7'for r in full),rust_only_lanes=lanes,
      expanded_trace_bytes=len(expanded),packed_trace_bytes=len(outputs['m3h9b_trace.dref']))
    inputs=sorted({Path(m.__file__).name for m in list(sys.modules.values()) if getattr(m,'__file__',None) and Path(m.__file__).resolve().parent==ROOT/'tools/reference'})
    sanitizers=dict(raw_trace_sha256=sha(observed_trace['m3d3j_trace.csv']),rule='--overflow-check rebuilds without -fwrapv under -fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow and --address-check under -fsanitize=address,undefined (both -fno-sanitize-recover=all), each with the declared sanitizer_adapters applied; each run asserts its raw trace equals the strict -fwrapv trace of the genuine text byte for byte, so raw_trace_sha256 is the digest of all three')
    adapters=[dict(a,builds=['--overflow-check','--address-check'],strict_build='genuine text under -fwrapv',original_sha256=sha(a['original'].encode()),adapted_sha256=sha(a['adapted'].encode()))for a in SANITIZER_ADAPTERS]
    barrel_table=dict(geometry.BARREL,mobjtype_index=MTB,spawnstate_index=S('S_BAR1'),deathstate_index=S('S_BEXP'),states={n:S(n)for n in geometry.BARREL_STATES})
    contract=dict(schema=SCHEMA,revision=manifest['revision'],flags=manifest['flags'],inventory=inventory,cases=per_case,maps=maps,functions=bodies,
      row_formats=ROW_FORMATS,barrel=barrel_table,explode_action=EXPLODE_ACTION,flats=geometry.FLATS,skyflatnum=geometry.SKYFLATNUM,switch_textures=list(geometry.SWITCH_TEXTURES),
      source_sha256={n:sha((ROOT/'linuxdoom-1.10'/n).read_bytes())for n in sorted(set(manifest['source_sha256'])|{'d_items.c','dstrings.h','p_spec.c','p_lights.c','p_spec.h','p_user.c','p_setup.c','p_doors.c','p_plats.c','p_floor.c','p_switch.c','p_map.c','g_game.c','m_bbox.c','d_event.h','info.c','info.h','sounds.h','p_mobj.c','p_mobj.h','p_enemy.c','p_local.h','tables.h'})},
      function_sha256={f'{f}:{n}':sha((definition_body(f,n,t)if(f,n,t)in ADDED else inherited.inherited.definition_body(f,n,t)).encode())for f,n,t in bodies},
      executed_function_entries=dict(sorted(functions.items())),
      inputs_sha256={'tools/reference/'+n:sha((ROOT/'tools/reference'/n).read_bytes())for n in inputs},
      outputs_sha256={n:sha(d)for n,d in outputs.items()},expanded_trace_sha256=sha(expanded),sanitizer_adapters=adapters,assumptions=[
       f'Genuine player0 plus at most {MAX_MONSTERS} map-THING MT_TROOP/MT_POSSESSED, at most {MAX_PICKUPS} map-THING pickups, at most {MAX_DECORATIONS} map-THING decorations of the seven E1M1 types and at most {MAX_BARRELS} map-THING MT_BARREL (DoomEd 2035), every short THING angle, in ten polygon rooms ({", ".join(ROOM_NAMES)}) built by the M3h9 builder; no Rust expected observations.',
       'Every compiled body, observer, declared adapter and controlled input of M3h2..M3h9 is reused unchanged. Added genuine bodies: PIT_RadiusAttack and P_RadiusAttack (p_map.c, with the globals bombsource, bombspot and bombdamage copied verbatim from p_map.c:1154-1156) and A_Explode (p_enemy.c), which replaces the aborting stub every unlisted action has had since M3d3j. The barrel table of the contract is asserted against the op-0 actor capture of every spawned barrel (type, spawn state, tics 1..6 after the spawn draw, health, radius, height, flags, reaction time, target).',
       'Observers added (no source statement changed): RA after bombdamage is assigned in P_RadiusAttack and a block marker around the unchanged P_BlockThingsIterator call; RC at every return of PIT_RadiusAttack and around the genuine P_CheckSight call (ra_seen returns the value it is given); E action row 5 at A_Explode entry (as A_Scream).',
       'Declared adapters, sanitizer builds only (sanitizer_adapters): p_mobj.c:794 and p_map.c:1221 are rewritten in unsigned arithmetic computing the same 32-bit values; the first is needed (the sanitizer builds report the int product of an octant outside -4..3), the second is declared for robustness (under -std=gnu89 the shift is not reported). The strict build compiles the genuine text, and the three raw traces are byte-identical (sanitizer_comparison).',
       'Controlled inputs: no new input. hit_* (P_DamageMobj(thing, NULL, NULL, amount) between tics) is used only in radius_blocked_by_step (a real shooter thrusts the barrel away from the curb) and fireball_on_dying_barrel (a pistol or fist kill alerts the Imp, whose own fireballs would then reach the live barrel first); attack_at (an Imp S_TROO_ATK1 entry with the player as target) only in fireball_on_dying_barrel; chase_* (target assignment plus see-state entry) for the Zombieman of barrel_infight, the Imp of fireball_kills_barrel and the Imp of embedded_spawns. Scenarios whose outcome depends on the gameplay RNG (fist_kills_barrel and player_killed_by_barrel need a 20-point blow, monster_wakes_mid_walk a failed pain draw, barrel_infight and fireball_kills_barrel a monster that wounds the barrel first) start at a declared seed found by trying every rndtable index; the seed only chooses the starting prndindex, as M_ClearRandom does at 0.',
       'intercept_t intercepts[128] keeps its source size; the harness has aborted before the 129th entry since M3d3 (an observer guard, not a source check: PIT_AddLineIntercepts and PIT_AddThingIntercepts write with no bounds check). The densest trace observed is intercept_heavy_trace (see its case); the Rust-only intercepts129 lane is never run in C.',
       'Bounded original command/weapon stage precedes original thinkers; not complete P_Ticker/PlayerThink/DeathThink or respawn. The reborn request ends player_killed_by_barrel (MG row 2); G_DoReborn is not run.',
       'All M3h1..M3h9 assumptions and adapters apply, including the strict-build retyping of attack-angle int locals; every new row only observes genuine statements.'],
      sanitizer_comparison=sanitizers)
    outputs['m3h9b_contract.json']=(json.dumps(contract,indent=2,sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check:assert path.read_bytes()==data,path
        else:path.write_bytes(data)
    print(('Checked'if check else'Generated')+' original barrels, the radius attack, pickup scale and angles: '+json.dumps({k:v for k,v in inventory.items()if k!='rust_only_lanes'})+f'; overflow={overflow}; ASan/UBSan={address}')
    return per_case
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--overflow-check',action='store_true');parser.add_argument('--address-check',action='store_true');parser.add_argument('--cases',action='store_true');args=parser.parse_args()
    table=run(args.check,args.overflow_check,args.address_check)
    if args.cases:
        for row in table:print(json.dumps(row))
