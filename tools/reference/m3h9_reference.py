#!/usr/bin/env python3
"""Genuine original-C two-sided scrollers, static decorations and the spawn filter
(M3h9a).

Reuses every compiled M3h8 body, observer, declared adapter and controlled input
unchanged (importing m3h8_reference applies the M3h2..M3h8 harness patches) over the
rooms of m3h9_cases. No genuine body is added: the two-sided type-48 effects (M3h9 note
§2.4), the decoration spawn (P_SpawnMapThing over the pristine info.c rows of MT_MISC31,
MT_MISC48, MT_MISC50, MT_MISC71, MT_MISC62, MT_MISC68 and MT_MISC69), their collision
(PIT_CheckThing, PTR_AimTraverse, PTR_ShootTraverse) and their clipping under movers
(PIT_ChangeSector, P_ThingHeightClip) are bodies compiled since M3d3/M3h2/M3h8 reaching
branches these rooms exercise for the first time. Three observer rows are added (DT, XT,
NS; see the contract's `row_formats`) and one per-case harness input, `skill`, selects
the genuine `gameskill` the spawn filter reads. Expectations come from compiled original
bodies over synthetic inputs; they never consume Rust output."""
import argparse,collections,csv,hashlib,io,json,re,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools/reference'))
import m3h8_reference as inherited
import m3h9_geometry as geometry
from m3h9_cases import ROOMS,ROOM_NAMES,DENSE9,populate,MAX_PICKUPS,MAX_MONSTERS,MAX_DECORATIONS,spawns,SPAWN_TICS_DRAW,MEDIUM,ROSTER,PICKUPS,LAMP,PILLAR,CANDELABRA,GIBS,DEADPLAYER,POSSESSED,IMP
from m3h2_observers import replace_once,replace_last
from evidence_pack import pack,unpack
previous=inherited.previous;core=inherited.core;pickup_stage=inherited.pickup_stage
LAYOUTS=populate(previous.pack)
SCHEMA='DOOM-PLAYER-E1M1-v1'
ADDED=[]  # every body this stage needs was compiled by an earlier stage
U=65536
MT=geometry.enum_members('mobjtype_t');ST=geometry.enum_members('statenum_t')
TROOPSHOT,PUFF,PLAYER=MT.index('MT_TROOPSHOT'),MT.index('MT_PUFF'),MT.index('MT_PLAYER')
NOWAY=geometry.enum_index('sfxenum_t','sfx_noway','sounds.h')
MF_SOLID,MF_SHOOTABLE,MF_AMBUSH,MF_COUNTKILL,MF_COUNTITEM=2,4,32,0x400000,0x800000
DI_NODIR=8
# --- observers (genuine statements bracketed; source order, arithmetic and RNG untouched) ---
DT='printf("DT,%d,%d,%d,%d,{code},%d,%d,%d,%d,%d\\n",scenario,operation,ident(tmthing),ident(thing),thing->flags,{result},tmthing->z,thing->z,thing->height);'
inherited_observed=core.observed_body
def observed(file,name,result,rename=None):
    text=inherited_observed(file,name,result,rename)
    if name=='PIT_CheckThing':
        text=replace_once(text,'    if (!(thing->flags & (MF_SOLID|MF_SPECIAL|MF_SHOOTABLE) ))\n\treturn true;',
          '    if (!(thing->flags & (MF_SOLID|MF_SPECIAL|MF_SHOOTABLE) ))\n\t{if(abs(thing->x - tmx) < thing->radius + tmthing->radius && abs(thing->y - tmy) < thing->radius + tmthing->radius)'+DT.format(code=0,result=1)+'return true;}')
        text=replace_once(text,"\t    // didn't do any damage\n\t    return !(thing->flags & MF_SOLID);","\t    // didn't do any damage\n\t    "+DT.format(code=1,result='!(thing->flags & MF_SOLID)')+"return !(thing->flags & MF_SOLID);")
        text=replace_last(text,'    return !(thing->flags & MF_SOLID);\n}','    if(!(thing->flags & MF_SHOOTABLE))'+DT.format(code=2,result='!(thing->flags & MF_SOLID)')+'\n    return !(thing->flags & MF_SOLID);\n}')
    if name in('PTR_AimTraverse','PTR_ShootTraverse'):
        code=0 if name=='PTR_AimTraverse' else 1
        text,count=re.subn(r'(if \(!\(th->flags&MF_SHOOTABLE\)\)\n\t)return true;',lambda m:m.group(1)+'{printf("XT,%d,%d,'+str(code)+',%d,%d,%d\\n",scenario,operation,ident(shootthing),ident(th),th->flags);return true;}',text,count=1)
        assert count==1,name
    return text
core.observed_body=observed
inherited_write=pickup_stage.write
def write(path,text,*a,**kw):
    """record_move prints every P_TryMove outcome (M rows); the NS row before it carries
    numspechit, the count P_Move and the crossing loop read next."""
    if path.name=='m3d3j_shim.h':
        text=replace_once(text,'static void record_move(mobj_t* p,int x,int y,int ok){','static void record_move(mobj_t* p,int x,int y,int ok){printf("NS,%d,%d,%d,%d,%d\\n",scenario,operation,attempt,ident(p),numspechit);')
    return inherited_write(path,text,*a,**kw)
pickup_stage.write=write
def configure():
    core.main.MAPS,core.main.THINGS,core.main.KEYS,core.main.CASES=[],[],[],[]
    for c in core.CASES:
        sectors,portals,reject,tree,segment,lines=ROOMS[c['room']]
        m=geometry.room(sectors,portals=portals,reject=reject,tree=tree,segment=segment,lines=lines)
        core.main.case(c['label'],n=1,custom=m,things=LAYOUTS[c['layout']],tics=c['tics'],rejected=bool(c['rejected']),skill=c['skill'],
                       **{'pc_'+k:v for k,v in c.items()if k not in['label','tics','map','skill']})
        c['map']=core.main.CASES[-1]['map']
    specials=inherited.inherited
    core.CAPTURE=specials.BASE_CAPTURE+specials.world_source()+specials.CAPTURE+inherited.switch_source()+inherited.CAPTURE
core.configure=configure
sha=lambda b:hashlib.sha256(b).hexdigest()
ROW_FORMATS=dict(inherited.ROW_FORMATS,**{
 'DT':'DT,case,op,tmthing,thing,code,flags,result,tmthing z,thing z,thing height: PIT_CheckThing on a thing (flags = thing->flags). code 0: the first flag test passed a thing with none of MF_SOLID|MF_SPECIAL|MF_SHOOTABLE whose box the position check overlaps (printed only when it overlaps; result 1; a flags-0 thing clipping its own position inside P_ThingHeightClip passes itself here, before the self test); code 1: the missile branch met a non-shootable thing and returns !(flags & MF_SOLID) (0 explodes); code 2: the last line of the body for a non-shootable (hence non-special) solid thing returns !(flags & MF_SOLID) = 0 with no height test',
 'XT':'XT,case,op,code,shooter,thing,flags: PTR_AimTraverse (code 0) or PTR_ShootTraverse (code 1) skipped a non-shootable thing intercept ("corpse or something") and the trace continues',
 'NS':'NS,case,op,attempt,thing,numspechit: printed before every M row (P_TryMove outcome). For a refused move it is the count P_CheckPosition left (what the P_Move failure branch reads next); for an accepted move the crossing loop has already run `while (numspechit--)`, so it reads -1 (the source leaves numspechit at -1 until the next P_CheckPosition)',
 'F':'F,case,0,index,x,y,angle,type,options,owned before,owned after,decision,prndindex: one row per THING record from the P_SpawnMapThing wrapper (decision 0 spawned, 16 options bit 16, 2 skill bit, 1 player start, 11 deathmatch start, 32 MF_NOTDMATCH, 64 nomonsters; prndindex after the record); the index -1 row sums the lump (owned, prndindex, totalkills, totalitems)',
 'M':'as M3h6: M,case,op,attempt,actor,x,y,ok,tmfloorz,tmceilingz,tmdropoffz,floatok,move_failure,subsector,sector,block_line,block_actor; move_failure 1 names the blocking thing (a decoration here), 2 the blocking line (the ML_BLOCKING scroller here)'})
def run(check,overflow,address):
    flags=list(previous.pack.legacy.base.FLAGS)
    observed_trace=inherited.capture(flags)
    for enabled,sanitizer in [(overflow,'signed-integer-overflow,integer-divide-by-zero,float-cast-overflow'),(address,'address,undefined')]:
        if enabled:
            audited=inherited.capture([f for f in flags if f!='-fwrapv']+['-fno-wrapv','-fsanitize='+sanitizer,'-fno-sanitize-recover=all'])
            assert audited['m3d3j_trace.csv']==observed_trace['m3d3j_trace.csv']
    rows=list(csv.reader(io.StringIO(observed_trace['m3d3j_trace.csv'].decode())))
    functions=collections.Counter(r[3]for r in rows if r[0]=='FN')
    full=[r for r in rows if r[0]!='FN']
    case_rows=collections.defaultdict(list)
    for r in rows:case_rows[int(r[1])].append(r)
    last_op={w:max(int(r[2])for r in mine if r[0]!='FN')for w,mine in case_rows.items()}
    ended={int(r[1]):int(r[3])for r in full if r[0]=='MG'and r[3]in('1','2')}
    for which,c in enumerate(core.CASES):assert last_op[which]==c['tics']or which in ended,(c['label'],last_op[which])
    assert not ended
    selected={0,1,2,4,8,16,32,48,64,96,128,160}
    pushes={(i,c['push_at'])for i,c in enumerate(core.CASES)if c['push_at']}
    mover={'ME','MU','MX','MG','SO','MP','SD','XT','MI'}
    interesting={(int(r[1]),int(r[2]))for r in full if (r[0]=='G'and r[3]in{'2','7','10'})or r[0]in{'AR','ST','PN','SL','X','N','SE','KY','KR','KS','KZ'}|mover
                 or(r[0]=='MS'and r[3]in{'0','5'})or(r[0]=='E'and r[3]=='1'and r[2]!='0')or(r[0]=='M'and r[7]=='0')or(r[0]=='MT'and r[4]!='0')or(r[0]=='MV'and r[10]!='0')
                 or(r[0]=='DT'and r[5]!='0'and r[7]=='0')}|pushes
    keys={(int(r[1]),int(r[2]))for r in rows if r[0]=='P'and(int(r[2])in selected or tuple(map(int,r[1:3]))in interesting or int(r[2])==last_op[int(r[1])])}
    kept=[r for r in rows if r[0]!='FN'and(r[0]not in{'A','P','AP','R','S','LS'}or tuple(map(int,r[1:3]))in keys)]
    text=io.StringIO();csv.writer(text,lineterminator='\n').writerows(kept)
    expanded=(SCHEMA+'\n'+text.getvalue()).encode()
    outputs={'m3h9_trace.dref':pack(expanded)}
    assert unpack(outputs['m3h9_trace.dref'])==expanded
    text=io.StringIO();writer=csv.writer(text,lineterminator='\n');writer.writerow(['case',*core.CASES[0]]);writer.writerows([i,*c.values()]for i,c in enumerate(core.CASES))
    outputs['m3h9_cases.csv']=text.getvalue().encode()
    maps=[]
    for i,m in enumerate(core.main.MAPS):
        name=f'm3h9_geometry_{i}.bin';outputs[name]=m['bytes'];maps.append('tests/reference/'+name)
    outputs['m3h9_maps.rs']=('// Synthetic source inputs; GPL-2.0-only.\npub const MAP_BYTES: &[&[u8]] = &[\n'+''.join(f'    include_bytes!("{Path(p).name}"),\n'for p in maps)+'];\n').encode()
    # Rust-only: the nine-scroller lane of the SpecialCross witness, built by the same
    # builder but never run in C, where its ninth spechit write would overflow the array.
    dense9=geometry.room(DENSE9[0],portals=DENSE9[1],reject=DENSE9[2],tree=DENSE9[3],segment=DENSE9[4],lines=DENSE9[5])
    outputs['m3h9_dense9.bin']=dense9['bytes']
    manifest=json.loads(observed_trace['m3d3j_provenance.json'])
    bodies=manifest['functions']+[['p_pspr.c',n,'void']for n in previous.pack.WEAPON_ACTIONS]
    assert len(bodies)==len({tuple(b)for b in bodies})
    # ---- observations, all computed from the full trace -------------------------------
    player_ident={w:next(r[5]for r in mine if r[0]=='W'and r[2]=='0'and r[3]=='7')for w,mine in case_rows.items()}
    types={}
    for r in rows:
        if r[0]=='E'and r[3]=='1':types.setdefault((r[1],r[4]),int(r[5]))
    records=collections.defaultdict(list)   # case -> F rows of the THING records in lump order
    doomed={}                               # (case, ident) -> DoomEd number of a spawned record
    options={}
    for r in full:
        if r[0]=='F'and r[3]!='-1':
            records[r[1]].append(r)
            if int(r[10])>int(r[9]):doomed[(r[1],r[9])]=int(r[7]);options[(r[1],r[9])]=int(r[8])
    def kind(w,i):
        if i==player_ident[int(w)]:return'player'
        t=types.get((w,i))
        if t==TROOPSHOT:return'missile'
        d=doomed.get((w,i))
        if d in geometry.DECORATIONS:return f'decoration:{d}'
        if d in ROSTER:return'monster'
        if d in PICKUPS:return'pickup'
        return'other'
    def ident_of(w,doomed_kind,x=None,y=None):
        found=[r for r in records[w]if int(r[10])>int(r[9])and int(r[7])==doomed_kind and(x is None or(int(r[4]),int(r[5]))==(x,y))]
        assert len(found)==1,(w,doomed_kind,x,y,len(found))
        return found[0][9]
    mk=[r for r in full if r[0]=='MK'];assert all(0<=int(r[5])<8 for r in mk)
    spechit_max=1+max(int(r[5])for r in mk)
    # Every spawned decoration carries the pristine info.c row (M3h9 note §2.5): type,
    # spawn state, tics -1, health 1000, radius, height and flags (MF_SOLID, plus
    # MF_AMBUSH from the record's MTF_AMBUSH), read from the op-0 actor capture.
    decoration_rows=0
    for (w,i),d in doomed.items():
        if d not in geometry.DECORATIONS:continue
        _,member,state,radius,height,solid=geometry.DECORATIONS[d]
        a=next(r for r in case_rows[int(w)]if r[0]=='A'and r[2]=='0'and r[3]==i)
        expected_flags=(MF_SOLID if solid else 0)|(MF_AMBUSH if options[(w,i)]&8 else 0)
        assert int(a[5])==MT.index(member)and int(a[13])==ST.index(state)and int(a[14])==-1 and int(a[12])==1000,(w,i,d,a)
        assert int(a[17])==radius*U and int(a[16])==height*U and int(a[15])==expected_flags,(w,i,d,a)
        assert int(a[8])==int(a[22]),(w,i,d,'spawned on its floor')
        decoration_rows+=1
    # ---- per-scenario checks: each scenario must exhibit what it claims ----------------
    def rows_of(mine,tag,**where):
        out=[]
        for r in mine:
            if r[0]!=tag:continue
            if all(r[int(k)]==str(v)for k,v in where.items()):out.append(r)
        return out
    def same_op_after(mine,anchor,tag,**where):
        """Rows `tag` matching `where` printed after `anchor` within anchor's op."""
        at=mine.index(anchor);out=[]
        for r in mine[at+1:]:
            if r[2]!=anchor[2]:break
            if r[0]==tag and all(r[int(k)]==str(v)for k,v in where.items()):out.append(r)
        return out
    def trace_intercepts(mine,op,kind):
        """The I rows of the P_PathTraverse calls of `kind` (TR row: 0 aim, 1 shoot, 2
        slide, 3 use) in op, bracketed by their TR and TI rows."""
        out=[];inside=False
        for r in mine:
            if r[2]!=op:continue
            if r[0]=='TR':inside=r[4]==str(kind)
            elif r[0]=='TI':inside=False
            elif r[0]=='I'and inside:out.append(r)
        return out
    def heights(mine,op):
        mh=next(r for r in mine if r[0]=='MH'and r[2]==str(op));n=int(mh[3])
        return [(int(mh[4+3*k]),int(mh[5+3*k]))for k in range(n)]
    def final_actor(mine,i,w):
        return next(r for r in mine if r[0]=='A'and r[2]==str(last_op[int(w)])and r[3]==i)
    def check_scroll_cross_noop(w,mine,m,p):
        scroll={str(s['line'])for s in m['scrollers']}
        mx=rows_of(mine,'MX',**{'3':p,'6':48});assert mx and all(r[4]in scroll for r in mx)
        my=rows_of(mine,'MY',**{'3':p});assert my and all(r[5]=='48'for r in my)
        stores=rows_of(mine,'MK',**{'3':p});assert stores and all(r[4]in scroll for r in stores)
        assert not rows_of(mine,'ME')and not rows_of(mine,'MU')and not rows_of(mine,'SO')
        assert not[r for r in mine if r[0]=='E'and r[3]=='8'and r[2]!='0'],'only the op-0 lastlook draw of the player body'
        assert heights(mine,0)==heights(mine,last_op[int(w)])
        assert int(final_actor(mine,p,w)[6])>0
        return dict(crossing_ops=sorted({int(r[2])for r in mx}),spechit_stores=len(stores),gameplay_draws_after_load=0,sector_heights_unchanged=True)
    def check_scroll_spechit_nodir(w,mine,m,p):
        z=ident_of(w,POSSESSED);scroll={str(s['line'])for s in m['scrollers']}
        mp=rows_of(mine,'MP',**{'3':z});assert mp and all(int(r[5])>=1 for r in mp)
        mu=rows_of(mine,'MU',**{'3':z});assert mu and all(r[6]=='48'and r[7]=='0'and r[4]in scroll for r in mu)
        draws=[same_op_after(mine,r,'H',**{'3':3,'4':z})for r in mp];assert all(draws)
        stores=rows_of(mine,'MK',**{'3':z});assert stores and all(r[4]in scroll for r in stores)
        assert not rows_of(mine,'ME')
        return dict(nodir_resets=len(mp),reset_ops=sorted({int(r[2])for r in mp}),monster_uses_false=len(mu),draws_after_reset=sum(len(d)for d in draws),movedir_before_reset=sorted({int(r[4])for r in mp}))
    def check_use_scroll_wall_stops_trace(w,mine,m,p):
        scroll={str(s['line'])for s in m['scrollers']}
        mi=rows_of(mine,'MI');assert len(mi)==1 and mi[0][3]in scroll and mi[0][5]=='48'and mi[0][7]=='0',mi
        # P_UseSpecialLine for a player: no case 48, the trailing `return true` (its value
        # is ignored by PTR_UseTraverse); for a monster the non-player filter returns false.
        mu=rows_of(mine,'MU',**{'3':p});assert len(mu)==1 and mu[0][6]=='48'and mu[0][7]=='1'and mu[0][8]=='48'
        assert len(rows_of(mine,'MA'))==1 and not rows_of(mine,'ME')and not rows_of(mine,'SO')
        assert not rows_of(mine,'E',**{'3':7,'5':NOWAY})
        door=[k for k,a in enumerate(m['sector_attrs'])if m['heights'][k]==(0,0)];assert len(door)==1
        assert heights(mine,0)==heights(mine,last_op[int(w)])and heights(mine,0)[door[0]]==(0,0)
        return dict(use_op=int(mi[0][2]),trace_stopped_at_line=int(mi[0][3]),player_use_result=int(mu[0][7]),special_after=int(mu[0][8]),door_sector=door[0],door_stayed_closed=True,noway=0)
    def check_shoot_through_scroller(w,mine,m,p):
        scroll={str(s['line'])for s in m['scrollers']}
        shots=rows_of(mine,'X',**{'3':0,'4':p});assert len(shots)==1
        op=shots[0][2]
        crossed=[r for r in trace_intercepts(mine,op,1)if r[4]=='1'and r[5]in scroll];assert crossed
        impact=rows_of(mine,'X',**{'2':op,'3':1});assert len(impact)==1 and impact[0][4]=='1'and impact[0][5]not in scroll
        assert sum(r[0]=='FN'and r[2]==op and r[3]=='p_spec.c:P_ShootSpecialLine'for r in mine)>=1
        puffs=[r for r in mine if r[0]=='E'and r[2]==op and r[3]=='1'and int(r[5])==PUFF];assert len(puffs)==1
        return dict(shot_op=int(op),scroller_intercepts=len(crossed),impact_line=int(impact[0][5]),puffs=1)
    def check_fireball_through_scroller(w,mine,m,p):
        scroll={str(s['line'])for s in m['scrollers']}
        launches=rows_of(mine,'MS',**{'3':5});assert len(launches)==1
        mx=[r for r in mine if r[0]=='MX'and kind(w,r[3])=='missile'];assert mx and all(r[6]=='48'and r[4]in scroll for r in mx)
        missile=mx[0][3]
        beyond=[r for r in mine if r[0]=='M'and r[4]==missile and r[7]=='1'and int(r[5])<-64*U];assert beyond
        assert not rows_of(mine,'ME')
        return dict(launch_op=int(launches[0][2]),crossing_ops=sorted({int(r[2])for r in mx}),missile_steps_west_of_line=len(beyond),explosions=len(rows_of(mine,'MS',**{'3':0})))
    def check_blocking_scroller_wall(w,mine,m,p):
        s=str(m['scrollers'][0]['line']);z=ident_of(w,POSSESSED)
        blocked={}
        for i in(p,z):
            refused=rows_of(mine,'M',**{'4':i,'7':0,'12':2,'15':s});assert refused,i
            attempts={(x[2],x[3])for x in refused}
            ns=[r for r in mine if r[0]=='NS'and r[4]==i and(r[2],r[3])in attempts]
            assert len(ns)==len(refused)and all(r[5]=='0'for r in ns)
            blocked[kind(w,i)]=len(refused)
        assert not rows_of(mine,'MK')and not rows_of(mine,'MP')and not rows_of(mine,'MX')
        return dict(blocked_moves=blocked,spechit_entries=0,numspechit_at_refusals=0)
    def check_dense_spechit_8(w,mine,m,p):
        scroll={str(s['line'])for s in m['scrollers']};assert len(scroll)==8
        stores=rows_of(mine,'MK',**{'3':p});assert stores and max(int(r[5])for r in stores)==7
        per_op=collections.Counter(r[2]for r in stores);full8=[op for op,n in per_op.items()if n==8];assert full8 and max(per_op.values())==8
        assert all(sorted(int(r[5])for r in stores if r[2]==op)==list(range(8))for op in full8)
        mx=rows_of(mine,'MX',**{'3':p});assert {r[4]for r in mx}==scroll and all(r[6]=='48'for r in mx)
        # After a successful P_TryMove the crossing loop (`while (numspechit--)`) leaves
        # numspechit at -1; the next P_CheckPosition resets it.
        ns=[r for r in mine if r[0]=='NS'and r[4]==p and r[2]in full8];assert ns and all(r[5]=='-1'for r in ns)
        assert not rows_of(mine,'ME')
        return dict(spechit_max=8,position_checks_with_eight=len(full8),crossings=len(mx),numspechit_after_crossing_loop=-1)
    def check_walk_into_lamp(w,mine,m,p):
        lamp=ident_of(w,LAMP,0,128)
        refused=rows_of(mine,'M',**{'4':p,'7':0,'12':1,'16':lamp});assert refused
        dt=rows_of(mine,'DT',**{'3':p,'4':lamp,'5':2,'7':0});assert dt
        assert int(final_actor(mine,p,w)[6])<=-32*U
        return dict(blocked_moves=len(refused),first_block_op=int(refused[0][2]),solid_returns=len(dt))
    def check_walk_into_lamp_from_ledge(w,mine,m,p):
        lamp=ident_of(w,LAMP,-256,226)
        refused=rows_of(mine,'M',**{'4':p,'7':0,'12':1,'16':lamp});assert refused
        dt=rows_of(mine,'DT',**{'3':p,'4':lamp,'5':2,'7':0});assert dt
        assert all(int(r[8])==32*U and int(r[8])>int(r[9])+int(r[10])for r in dt),'walker above the lamp top'
        return dict(blocked_moves=len(refused),walker_z=32,lamp_top=16,solid_returns=len(dt))
    def check_walk_over_gore(w,mine,m,p):
        gore=[ident_of(w,k,x,-300)for k,x in[(GIBS,-120),(DEADPLAYER,-40),(10,40),(12,120)]]
        passed={g:rows_of(mine,'DT',**{'3':p,'4':g,'5':0,'7':1})for g in gore};assert all(passed.values())
        assert not rows_of(mine,'M',**{'4':p,'7':0})
        assert int(final_actor(mine,p,w)[6])>150*U
        return dict(pass_throughs={kind(w,g):len(v)for g,v in passed.items()},blocked_moves=0)
    def check_monster_path_around_pillar(w,mine,m,p):
        z=ident_of(w,POSSESSED);pillar=ident_of(w,PILLAR,-160,-64)
        refused=rows_of(mine,'M',**{'4':z,'7':0,'12':1,'16':pillar});assert refused
        dt=rows_of(mine,'DT',**{'3':z,'4':pillar,'5':2,'7':0});assert dt
        draws=[same_op_after(mine,r,'H',**{'3':3,'4':z})for r in refused];assert any(draws)
        return dict(blocked_moves=len(refused),first_block_op=int(refused[0][2]),solid_returns=len(dt),draws_after_blocks=sum(len(d)for d in draws),player_final_health=int(next(r for r in mine if r[0]=='P'and r[2]==str(last_op[int(w)]))[4]))
    def check_monster_hitscan_through_pillar(w,mine,m,p):
        z=ident_of(w,POSSESSED);pillar=ident_of(w,PILLAR,-480,-64)
        shots=rows_of(mine,'X',**{'3':0,'4':z});assert shots
        skips=rows_of(mine,'XT',**{'3':1,'4':z,'5':pillar});assert skips
        through=[r for r in shots if any(s[2]==r[2]for s in skips)];assert through
        return dict(monster_shots=len(shots),shots_through_pillar=len(through),pillar_skips=len(skips),player_hits=len([r for r in mine if r[0]=='G'and r[3]=='2'and r[4]==p]))
    def check_fireball_hits_candelabra(w,mine,m,p):
        cand=ident_of(w,CANDELABRA)
        dt=[r for r in mine if r[0]=='DT'and r[4]==cand and r[5]=='1'and r[7]=='0'and kind(w,r[3])=='missile'];assert dt
        missile=dt[0][3];assert rows_of(mine,'MS',**{'2':dt[0][2],'3':0,'4':missile})
        assert not rows_of(mine,'G',**{'3':2})
        return dict(explosion_op=int(dt[0][2]),missile_z=int(dt[0][8])//U,candelabra_z=int(dt[0][9])//U,candelabra_height=int(dt[0][10])//U,damage_requests=0)
    def check_fireball_over_corpse(w,mine,m,p):
        pool=ident_of(w,GIBS,0,-192)
        dt=[r for r in mine if r[0]=='DT'and r[4]==pool and r[5]=='0'and kind(w,r[3])=='missile'];assert dt
        missile=dt[0][3];assert not rows_of(mine,'DT',**{'5':1})
        beyond=[r for r in mine if r[0]=='M'and r[4]==missile and r[7]=='1'and int(r[5])<-32*U];assert beyond
        return dict(pass_op=int(dt[0][2]),missile_z=int(dt[0][8])//U,pool_z=int(dt[0][9])//U,missile_steps_beyond=len(beyond),player_hits=len([r for r in mine if r[0]=='G'and r[3]=='2'and r[4]==p]))
    def check_pistol_through_lamp(w,mine,m,p):
        lamp=ident_of(w,LAMP,0,128)
        aim=rows_of(mine,'XT',**{'3':0,'4':p,'5':lamp});shoot=rows_of(mine,'XT',**{'3':1,'4':p,'5':lamp});assert aim and shoot
        op=shoot[0][2];impact=rows_of(mine,'X',**{'2':op,'3':1});assert len(impact)==1 and impact[0][4]=='1'
        puffs=[r for r in mine if r[0]=='E'and r[2]==op and r[3]=='1'and int(r[5])==PUFF];assert len(puffs)==1
        return dict(shot_op=int(op),aim_skips=len(aim),shoot_skips=len(shoot),impact_line=int(impact[0][5]),puffs=1)
    def check_decorations_ride_lift(w,mine,m,p):
        lamp=ident_of(w,LAMP);corpse=ident_of(w,DEADPLAYER)
        lift=next(k for k,a in enumerate(m['sector_attrs'])if a['tag']==1)
        steps=rows_of(mine,'MV',**{'3':lift,'4':0});assert steps and any(r[10]=='2'for r in steps)
        out={}
        for i in(lamp,corpse):
            hc=rows_of(mine,'HC',**{'3':i});assert len(hc)==len(steps)
            assert all(r[4]=='1'and r[6]==r[7]for r in hc),'rides on its floor'
            assert all(r[4]=='0'for r in rows_of(mine,'MT',**{'3':i}))
            assert int(final_actor(mine,i,w)[8])==64*U
            out[kind(w,i)]=dict(clips=len(hc),lowest_z=min(int(r[6])for r in hc)//U)
        return dict(lift_steps=len(steps),riders=out,gibs=0,nofit=0)
    def check_door_closes_on_pillar(w,mine,m,p):
        pillar=ident_of(w,PILLAR)
        door=next(k for k,(f,c)in enumerate(m['heights'])if(f,c)==(2,124))
        uses=rows_of(mine,'ME',**{'3':2});assert len(uses)==2
        down=rows_of(mine,'MV',**{'3':door,'4':1,'5':-1});assert down and down[-1][10]=='2'
        stuck=rows_of(mine,'MT',**{'3':pillar,'4':3});assert stuck and not rows_of(mine,'MT',**{'3':pillar,'4':4})
        hc=rows_of(mine,'HC',**{'3':pillar});assert hc and all(r[5]==r[6]==str(2*U)for r in hc)
        assert heights(mine,last_op[int(w)])[door]==(2*U,2*U)
        return dict(use_ops=[int(r[2])for r in uses],closing_steps=len(down),left_as_is=len(stuck),nofit=0,final_door_ceiling=2,pillar_z_unchanged=True)
    def check_spawn_filter(w,mine,m,p,skill):
        things=LAYOUTS[core.CASES[int(w)]['layout']];recs=records[w];assert len(recs)==len(things)==40
        spawned=[];draws_per_record=[];before=int(core.CASES[int(w)]['seed'])
        for r,t in zip(recs,things):
            assert (int(r[4]),int(r[5]),int(r[7]),int(r[8]))==(t[0],t[1],t[3],t[4]&0xffff)
            expect=spawns(t,skill);got=int(r[10])>int(r[9]);assert got==expect,(r,t)
            assert r[11]==('0'if got else'16'if t[4]&16 else'2')
            delta=int(r[12])-before;before=int(r[12])
            assert delta==((2 if t[3]in SPAWN_TICS_DRAW else 1)if got else 0),(r,t)
            draws_per_record.append(delta)
            if got:spawned.append((r[9],t))
        n=len(spawned)
        a0={r[3]:r for r in mine if r[0]=='A'and r[2]=='0'}
        counted_kills=sum(int(a0[i][15])&MF_COUNTKILL!=0 for i,_ in spawned);counted_items=sum(int(a0[i][15])&MF_COUNTITEM!=0 for i,_ in spawned)
        ls=next(r for r in mine if r[0]=='LS'and r[2]=='0');assert (int(ls[3]),int(ls[4]))==(counted_kills,counted_items)
        assert sum(t[3]in ROSTER for _,t in spawned)==counted_kills<=MAX_MONSTERS and sum(t[3]in PICKUPS for _,t in spawned)<=MAX_PICKUPS and sum(t[3]in geometry.DECORATIONS for _,t in spawned)<=MAX_DECORATIONS
        tk=[r for r in mine if r[0]=='TK'and r[2]=='0'];assert [r[5]for r in tk[:n]]==[i for i,_ in spawned]and tk[n][5]==p and tk[n+1][4]=='1'and len(tk)==n+2
        flash=rows_of(mine,'LR',**{'2':0,'3':1,'6':0});assert len(flash)==1
        total=int(a0[p][25]);assert total==int(core.CASES[int(w)]['seed'])+sum(draws_per_record)+1+1==sum(r[0]=='E'and r[2]=='0'and r[3]=='8'for r in mine)
        ambush=[i for i,t in spawned if t[4]&8];assert ambush and all(int(a0[i][15])&MF_AMBUSH for i in ambush)
        assert all(not int(a0[i][15])&MF_AMBUSH for i,t in spawned if not t[4]&8)
        return dict(skill=skill,records=len(recs),spawned=n,filtered_bit16=sum(r[11]=='16'for r in recs),filtered_skill=sum(r[11]=='2'for r in recs),
                    spawned_by_kind={'monsters':counted_kills,'pickups':sum(t[3]in PICKUPS for _,t in spawned),'decorations':sum(t[3]in geometry.DECORATIONS for _,t in spawned)},
                    totalkills=counted_kills,totalitems=counted_items,record_draws=sum(draws_per_record),two_draw_records=sum(d==2 for d in draws_per_record),
                    prndindex_after_specials=total,flash_spawn_draw=1,thinker_order='lump order, player body, light flash',ambush_decorations=[kind(w,i)for i in ambush if kind(w,i).startswith('decoration')])
    CHECKS={'scroll_cross_noop':check_scroll_cross_noop,'scroll_spechit_nodir':check_scroll_spechit_nodir,'use_scroll_wall_stops_trace':check_use_scroll_wall_stops_trace,
            'shoot_through_scroller':check_shoot_through_scroller,'fireball_through_scroller':check_fireball_through_scroller,'blocking_scroller_wall':check_blocking_scroller_wall,
            'dense_spechit_8':check_dense_spechit_8,'walk_into_lamp':check_walk_into_lamp,'walk_into_lamp_from_ledge':check_walk_into_lamp_from_ledge,'walk_over_gore':check_walk_over_gore,
            'monster_path_around_pillar':check_monster_path_around_pillar,'monster_hitscan_through_pillar':check_monster_hitscan_through_pillar,
            'fireball_hits_candelabra':check_fireball_hits_candelabra,'fireball_over_corpse':check_fireball_over_corpse,'pistol_through_lamp':check_pistol_through_lamp,
            'decorations_ride_lift':check_decorations_ride_lift,'door_closes_on_pillar':check_door_closes_on_pillar,
            'spawn_filter_medium':lambda w,mine,m,p:check_spawn_filter(w,mine,m,p,2),'spawn_filter_easy':lambda w,mine,m,p:check_spawn_filter(w,mine,m,p,1),'spawn_filter_hard':lambda w,mine,m,p:check_spawn_filter(w,mine,m,p,3)}
    assert set(CHECKS)=={c['label']for c in core.CASES}
    per_case=[]
    for which,c in enumerate(core.CASES):
        mine=case_rows[which];m=core.main.MAPS[c['map']];w=str(which);p=player_ident[which]
        observed_facts=CHECKS[c['label']](w,mine,m,p)
        player=[r for r in mine if r[0]=='P']
        spawned=[(i,d)for(cw,i),d in doomed.items()if cw==w]
        per_case.append(dict(label=c['label'],room=ROOM_NAMES[c['room']],tics=c['tics'],last_op=last_op[which],skill=c['skill'],rust_replayed=c['skill']==MEDIUM,
          map=dict(linedefs=len(m['lines']),sectors=len(m['heights']),subsectors=len(m['leaves']),nodes=len(m['nodes']),cells=m['cells'],
                   line_specials=sorted(sp for sp,_ in m['line_specials']if sp),scrollers=m['scrollers'],sector_specials=sorted(a['special']for a in m['sector_attrs']if a['special']),special_window=inherited.special_window(m)),
          things=dict(records=len(records[w]),spawned=len(spawned),decorations=dict(collections.Counter(str(d)for _,d in spawned if d in geometry.DECORATIONS)),monsters=sum(d in ROSTER for _,d in spawned),pickups=sum(d in PICKUPS for _,d in spawned)),
          observed=observed_facts,
          spechit_max=max([1+int(r[5])for r in mine if r[0]=='MK'],default=0),nodir_resets=sum(r[0]=='MP'for r in mine),
          uses=[[int(r[2]),int(r[4]),int(r[5]),int(r[6]),int(r[7])]for r in mine if r[0]=='MU'and r[3]==p],monster_uses=sum(r[0]=='MU'and r[3]!=p for r in mine),
          crossings=[[int(r[2]),kind(w,r[3]),int(r[4]),int(r[6])]for r in mine if r[0]=='MX'],use_trace_stops=sum(r[0]=='MI'and r[7]=='0'for r in mine),noway=sum(r[0]=='MI'and r[5]=='0'and r[7]=='0'for r in mine),
          movers_created=dict(collections.Counter(('EV_DoDoor','EV_VerticalDoor','EV_DoPlat','EV_DoFloor')[int(r[3])-1]for r in mine if r[0]=='ME'and int(r[9])>int(r[8]))),
          clip_outcomes=dict(collections.Counter(('fits','gibs','dropped_removed','left_stuck','nofit')[int(r[4])]for r in mine if r[0]=='MT')),
          blocked_by_thing=dict(collections.Counter(f'{kind(w,r[4])}:{kind(w,r[16])}'for r in mine if r[0]=='M'and r[7]=='0'and r[12]=='1')),
          blocked_by_line=dict(collections.Counter(f'{kind(w,r[4])}:{r[15]}'for r in mine if r[0]=='M'and r[7]=='0'and r[12]=='2')),
          pass_throughs=dict(collections.Counter(f'{kind(w,r[3])}:{kind(w,r[4])}'for r in mine if r[0]=='DT'and r[5]=='0')),
          missile_decoration_hits=sum(r[0]=='DT'and r[5]=='1'and r[7]=='0'for r in mine),hitscan_skips=dict(collections.Counter(f"{('aim','shoot')[int(r[3])]}:{kind(w,r[5])}"for r in mine if r[0]=='XT')),
          launches=sum(r[0]=='MS'and r[3]=='5'for r in mine),explosions=sum(r[0]=='MS'and r[3]=='0'for r in mine),hitscans=sum(r[0]=='X'and r[3]=='0'for r in mine),
          gameplay_draws=sum(r[0]=='E'and r[3]=='8'for r in mine),kills=sum(r[0]=='G'and r[3]=='7'for r in mine),
          player_final=dict(health=int(player[-1][4]),playerstate=int(player[-1][5]))if player else None))
    inventory=dict(scenarios=len(core.CASES),rust_replayed_scenarios=sum(c['skill']==MEDIUM for c in core.CASES),c_only_scenarios=[c['label']for c in core.CASES if c['skill']!=MEDIUM],
      scripted_tics=sum(c['tics']for c in core.CASES),executed_tics=sum(last_op.values()),captures=len(keys),rooms=len(ROOMS),maps=len(core.main.MAPS),
      selected_original_bodies=len(bodies),new_genuine_bodies=len(ADDED),exercised_unique_functions=len(functions),function_entries=sum(functions.values()),
      scrollers={ROOM_NAMES[c['room']]:dict(two_sided=sum(s['two_sided']for s in core.main.MAPS[c['map']]['scrollers']),blocking=sum(s['blocking']for s in core.main.MAPS[c['map']]['scrollers']))for c in core.CASES if core.main.MAPS[c['map']]['scrollers']},
      largest_special_window=max(inherited.special_window(m)for m in core.main.MAPS),
      decoration_spawns=len([1 for d in doomed.values()if d in geometry.DECORATIONS]),decoration_rows_checked_against_info=decoration_rows,
      decoration_spawns_by_type=dict(sorted(collections.Counter(str(d)for d in doomed.values()if d in geometry.DECORATIONS).items())),
      thing_records=sum(len(v)for v in records.values()),thing_spawns=len(doomed),filtered_records=sum(int(r[10])==int(r[9])for v in records.values()for r in v),
      spechit_entries=len(mk),spechit_max=spechit_max,nodir_resets=sum(r[0]=='MP'for r in full),
      uses=dict(sorted(collections.Counter(f"{kind(r[1],r[3])}:{r[6]}:{'true'if r[7]=='1'else'false'}"for r in full if r[0]=='MU').items())),use_traces=sum(r[0]=='MA'for r in full),use_trace_stops_at_scroller=sum(r[0]=='MI'and r[5]=='48'and r[7]=='0'for r in full),noway_stops=sum(r[0]=='MI'and r[5]=='0'and r[7]=='0'for r in full),
      crossings=dict(sorted(collections.Counter(f'{kind(r[1],r[3])}:{r[6]}'for r in full if r[0]=='MX').items())),
      blocked_by_thing=dict(sorted(collections.Counter(f'{kind(r[1],r[4])}:{kind(r[1],r[16])}'for r in full if r[0]=='M'and r[7]=='0'and r[12]=='1').items())),
      blocked_by_blocking_scroller=sum(v for c in per_case if c['room']=='scrollblock'for v in c['blocked_by_line'].values()),
      pass_throughs=dict(sorted(collections.Counter(f'{kind(r[1],r[3])}:{kind(r[1],r[4])}'for r in full if r[0]=='DT'and r[5]=='0').items())),
      missile_decoration_hits=sum(r[0]=='DT'and r[5]=='1'and r[7]=='0'for r in full),solid_returns_without_height_test=sum(r[0]=='DT'and r[5]=='2'for r in full),
      hitscan_skips=dict(sorted(collections.Counter(f"{('aim','shoot')[int(r[3])]}:{kind(r[1],r[5])}"for r in full if r[0]=='XT').items())),
      movers_created=dict(sorted(collections.Counter(('EV_DoDoor','EV_VerticalDoor','EV_DoPlat','EV_DoFloor')[int(r[3])-1]for r in full if r[0]=='ME'and int(r[9])>int(r[8])).items())),
      plane_steps=dict(sorted(collections.Counter(f"{('floor','ceiling')[int(r[4])]}:{'down'if r[5]=='-1'else'up'}:{('ok','crushed','pastdest')[int(r[10])]}"for r in full if r[0]=='MV').items())),
      change_sector_calls=sum(r[0]=='MC'and r[3]=='0'for r in full),height_clips=sum(r[0]=='HC'for r in full),
      clip_outcomes=dict(sorted(collections.Counter(('fits','gibs','dropped_removed','left_stuck','nofit')[int(r[4])]for r in full if r[0]=='MT').items())),
      decoration_clips_pushing_z=sum(r[0]=='HC'and r[6]!=r[7]and kind(r[1],r[3]).startswith('decoration')and r[4]=='0'for r in full),
      fireball_launches=sum(r[0]=='MS'and r[3]=='5'for r in full),fireball_explosions=sum(r[0]=='MS'and r[3]=='0'for r in full),hitscans=sum(r[0]=='X'and r[3]=='0'for r in full),
      gameplay_draws=sum(r[0]=='E'and r[3]=='8'for r in full),kills=sum(r[0]=='G'and r[3]=='7'for r in full),
      rust_only_maps=dict(dense9=dict(path='tests/reference/m3h9_dense9.bin',linedefs=len(dense9['lines']),scrollers=len(dense9['scrollers']),special_window=inherited.special_window(dense9))),
      expanded_trace_bytes=len(expanded),packed_trace_bytes=len(outputs['m3h9_trace.dref']))
    inputs=sorted({Path(m.__file__).name for m in list(sys.modules.values()) if getattr(m,'__file__',None) and Path(m.__file__).resolve().parent==ROOT/'tools/reference'})
    sanitizers=dict(raw_trace_sha256=sha(observed_trace['m3d3j_trace.csv']),rule='--overflow-check rebuilds without -fwrapv under -fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow and --address-check under -fsanitize=address,undefined (both -fno-sanitize-recover=all); each run asserts its raw trace equals the strict trace byte for byte, so raw_trace_sha256 is the digest of all three')
    decorations={str(d):dict(description=desc,mobjinfo=member,mobjtype_index=MT.index(member),spawnstate=state,state_index=ST.index(state),radius=radius,height=height,flags=MF_SOLID if solid else 0,health=1000,tics=-1)
                 for d,(desc,member,state,radius,height,solid)in sorted(geometry.DECORATIONS.items())}
    contract=dict(schema=SCHEMA,revision=manifest['revision'],flags=manifest['flags'],inventory=inventory,cases=per_case,maps=maps,functions=bodies,
      row_formats=ROW_FORMATS,flats=geometry.FLATS,skyflatnum=geometry.SKYFLATNUM,switch_textures=list(geometry.SWITCH_TEXTURES),decorations=decorations,
      source_sha256={n:sha((ROOT/'linuxdoom-1.10'/n).read_bytes())for n in sorted(set(manifest['source_sha256'])|{'d_items.c','dstrings.h','p_spec.c','p_lights.c','p_spec.h','p_user.c','p_setup.c','p_doors.c','p_plats.c','p_floor.c','p_switch.c','p_map.c','g_game.c','m_bbox.c','d_event.h','info.c','info.h','sounds.h','p_mobj.c','p_mobj.h'})},
      function_sha256={f'{f}:{n}':sha(inherited.definition_body(f,n,t).encode())for f,n,t in bodies},
      executed_function_entries=dict(sorted(functions.items())),
      inputs_sha256={'tools/reference/'+n:sha((ROOT/'tools/reference'/n).read_bytes())for n in inputs},
      outputs_sha256={n:sha(d)for n,d in outputs.items()},expanded_trace_sha256=sha(expanded),assumptions=[
       f'Genuine player0 plus at most {MAX_MONSTERS} map-THING MT_TROOP/MT_POSSESSED, at most {MAX_PICKUPS} map-THING pickups and at most {MAX_DECORATIONS} map-THING decorations of the seven E1M1 types (DoomEd 2028, 48, 35, 24, 15, 10, 12) in six polygon rooms (scrollhall, scrollblock, dense8, gallery, liftroom, census) built by the M3h8 builder with declared sector heights, tags, specials and line specials 48 (on two-sided lines, blocking and non-blocking: decision E1), 1 and 88; no Rust expected observations.',
       'Every compiled body, observer, declared adapter and controlled input of M3h2/M3h5/M3h6/M3h7/M3h8 is reused unchanged; no genuine body is added. The decoration types and their spawn states (S_COLU, S_TECHPILLAR, S_CANDELABRA, S_GIBS, S_PLAY_DIE7, S_PLAY_XDIE9) come from the pristine info.c tables the harness has compiled whole since M3f3 (definitions() has been empty since M3g2); the contract\'s `decorations` table is asserted against the op-0 actor capture of every spawned decoration (type, state, tics, health, radius, height, flags).',
       'Harness inputs: `skill` is the genuine gameskill of the case (2 medium for every scenario Rust replays; the census is also run at 1 easy and 3 hard as C-only rows, listed in inventory.c_only_scenarios); the census cases start the RNG at seed 0 as after M_ClearRandom, every other case keeps the M3h8 seed 6. The player body is still spawned by the harness at (px, py), so no record of type 1..4 or 11 is in any lump.',
       'Observers added (no source statement changed): DT in PIT_CheckThing (the first flag test when the box overlaps, the missile branch non-shootable return, the final solid return for non-shootable things), XT in PTR_AimTraverse and PTR_ShootTraverse (the non-shootable skip), NS before every M row (numspechit after P_CheckPosition). Every other row keeps its M3h8 meaning.',
       'spechit keeps its source size line_t* spechit[8] and PIT_CheckLine its unchecked append: the sanitizer build would report a ninth write. No room exceeds rule 6 (largest special window 8, the dense8 lane); the nine-scroller lane is written as m3h9_dense9.bin for the Rust-only SpecialCross witness and is never run in C.',
       'A decoration standing on its floor is always onfloor in P_ThingHeightClip (ONFLOORZ spawn, the clip follows the floor), so the z push of a thing that does not fit (p_map.c:556-557) is unreachable for decorations in these rooms: the pillar under the closing door keeps z == floor and PIT_ChangeSector returns at the MF_SHOOTABLE test (MT code 3) with nofit untouched.',
       'Source facts the scenarios pin that the M3h9 note states loosely: P_UseSpecialLine on a type-48 line returns false for a monster (the non-player filter, p_switch.c) but true with no effect for a player (no case 48, the trailing return true; PTR_UseTraverse ignores the value and stops the trace either way); after an accepted P_TryMove the crossing loop leaves numspechit at -1; a monster\'s first P_NewChaseDir never picks the opposite of its zero-initialised spawn movedir (DI_EAST), so the chasers that must walk toward a target are placed west of it.',
       'Bounded original command/weapon stage precedes original thinkers; not complete P_Ticker/PlayerThink/DeathThink or respawn. P_RespawnSpecials (deathmatch 2 only) is not called.',
       'All M3h1/M3h2/M3h4/M3h5/M3h6/M3h7/M3h8 assumptions and adapters apply, including the strict-build retyping of attack-angle int locals; every new row only observes genuine statements.'],
      sanitizer_comparison=sanitizers)
    outputs['m3h9_contract.json']=(json.dumps(contract,indent=2,sort_keys=True)+'\n').encode()
    for name,data in outputs.items():
        path=ROOT/'tests/reference'/name
        if check:assert path.read_bytes()==data,path
        else:path.write_bytes(data)
    print(('Checked'if check else'Generated')+' original two-sided scrollers, decorations and the spawn filter: '+json.dumps(inventory)+f'; strict={overflow}; ASan/UBSan={address}')
    return per_case
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');parser.add_argument('--overflow-check',action='store_true');parser.add_argument('--address-check',action='store_true');parser.add_argument('--cases',action='store_true');args=parser.parse_args()
    table=run(args.check,args.overflow_check,args.address_check)
    if args.cases:
        for row in table:print(json.dumps(row))
