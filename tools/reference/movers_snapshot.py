"""Independent DSIM35 projection from compiled original-C doors, lifts, floors, switches and exit rows.

Extends the frozen DSIM34 projection without editing it: the DSIM34 field layout and
event vocabulary under version 35, policy 19 and the movers context, plus exactly the
Stage B state of the M3h8 note §10 (docs/STATE_FORMAT.md, "DSIM v35"):

- `level.exit` (u8, G_ExitLevel's `gameaction == ga_completed`; MZ rows) after
  `level.single_player_kill_count`;
- `thinker.shared_order` rebuilt from every TK row: movers (TK kinds 5 door, 6 plat,
  7 floor) and a removed mover not yet unlinked (kind 8) are sector-thinker entries;
- the sector-thinker store with mover records (TK rows) and free slots: lights keep the
  load slots 0..L-1; a mover takes the lowest free slot at its creation (MM row) and its
  slot is freed, generation + 1, when P_RunThinkers unlinks it (SF row);
- `sector.{i}.floor`/`ceiling` the moving heights (MH rows), with `sector->specialdata`
  checked against the store (the live mover naming the sector) but not encoded;
- `line.{i}.special` (i16, ML rows) for every line whose map special is nonzero, after
  the last sector group;
- `original_player.0.use_down` (u8, MZ rows) after `original_player.0.secret_count`;
  the reborn request is the source `playerstate` PST_REBORN (2) in the existing
  `original_player.0.state` field;
- event tag 26 (movers) with sub-tags 0..14 (see SUBTAGS).

Rows that are not state or events: the use trace's own traversal rows (TR traverser 3,
D/B/U/I/T/TI at trace stage 9), the P_CheckPosition candidates of a height clip (C/O
rows and MK rows between an MC entry and exit) and every MK row, MA's coordinates, MN,
MC entries, MB, MW, MQ, MG end-of-op markers, and LE runs of movers (no sector thinker
emits a think event). Pickups, removals and state changes inside a clip stay inherited
events in place."""
import collections,struct
import specials_snapshot as inherited
base=inherited.base;n=base.n;physical=inherited.physical;map_scale=inherited.map_scale
CONTEXT=b'PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1'
VERSION=35;POLICY=19;MAX_SLOTS=inherited.MAX_SLOTS;MAX_LIVE_PROJECTILES=inherited.MAX_LIVE_PROJECTILES
MOVERS=26
SUBTAGS=dict(use=0,use_intercept=1,special_use=2,crossing=3,spechit_blocked=4,mover_start=5,mover_call=6,
             plane_step=7,change_sector=8,clip=9,mover_removed=10,mover_freed=11,sector_sound=12,exit=13,reborn=14)
DOOR,PLAT,FLOOR,REMOVED=5,6,7,8
NORMAL,BLAZE_OPEN=0,6                     # vldoor_e
DOWN_WAIT_UP_STAY=1                       # plattype_e
LOWER_TO_LOWEST,TURBO_LOWER=1,2           # floor_e
FRAC=65536
DOOR_SPEED={NORMAL:2*FRAC,BLAZE_OPEN:8*FRAC};DOOR_WAIT=150
PLAT_SPEED=4*FRAC;PLAT_WAIT=105
FLOOR_SPEED={LOWER_TO_LOWEST:FRAC,TURBO_LOWER:4*FRAC}
GA_COMPLETED=6;PST_REBORN=2
# Row tags that are state (persistent fields), presentation or harness markers.
STATE={'MH','ML','MZ','MQ','MB','MW','MN','MK','LE'}
NEW={'MA','MI','MU','MX','MY','MP','MM','ME','MV','MC','HC','MT','SD','SF','SO','MG','MR'}|STATE
def handle(h):
    slot,generation=h;return struct.pack('<IQ',slot,generation)
def strip(rows):
    """Rows of no DSIM vocabulary, removed before every chain (they are neither state nor
    events): the use trace's traversal rows and the position-check candidates of a
    height clip. A synthetic MR row marks the reborn request at the end of the players
    phase of the op whose MZ row first reads PST_REBORN (P_DeathThink's BT_USE statement
    is the last statement of a dead player's P_PlayerThink)."""
    out=[];clip=collections.defaultdict(bool);use=collections.defaultdict(bool)
    reborn={(r[1],r[2])for r in rows if r[0]=='MZ'and r[4]==PST_REBORN}
    for r in rows:
        key=r[1]
        if r[0]=='MA':assert not use[key];use[key]=True
        elif r[0]=='MN':assert use[key];use[key]=False
        if r[0]=='TR'and r[4]==3:assert use[key];continue
        if r[0]in{'D','B','U','I','T','TI'}and r[3]==9:assert use[key];continue
        if r[0]=='MC':assert clip[key]==(r[3]==1);clip[key]=r[3]==0
        if clip[key]and r[0]in{'C','O'}:continue
        if r[0]=='PG'and r[3]==2 and(r[1],r[2])in reborn:out.append(['MR',r[1],r[2]])
        out.append(r)
    assert not any(clip.values())and not any(use.values())
    return out
def plain(rows):
    """The same rows with every Stage B row removed (the DSIM34-vocabulary baseline)."""
    return [r for r in plain_thinkers(rows)if r[0]not in NEW or(r[0]=='LE'and r[3]<DOOR)]
class Store:
    """Sector-thinker slots of one case: lights at load in slots 0..L-1 (generation 1,
    never freed); a mover takes the lowest free slot when its EV_* body creates it (MM)
    and frees it, generation + 1, when P_RunThinkers unlinks it (SF)."""
    def __init__(self,lights):
        self.generations=[1]*lights;self.occupied=set(range(lights));self.handles={i:(i,1)for i in range(lights)}
    def create(self,ident):
        assert ident not in self.handles
        slot=next((s for s in range(len(self.generations))if s not in self.occupied),len(self.generations))
        if slot==len(self.generations):self.generations.append(1)
        self.occupied.add(slot);self.handles[ident]=(slot,self.generations[slot])
    def free(self,ident):
        slot,generation=self.handles[ident];assert slot in self.occupied and self.generations[slot]==generation
        self.occupied.remove(slot);self.generations[slot]+=1
def stores(rows):
    """Every mover's handle per case and every op's slot generations (after the op)."""
    handles={};tables={};store={};sectors={};removed=set()
    lights=collections.Counter(r[1]for r in rows if r[0]=='TK'and r[2]==0 and 1<=r[4]<=4)
    for r in rows:
        key=r[1]
        if r[0]=='TK'and r[2]==0 and key not in store:store[key]=Store(lights[key])
        if r[0]=='MM':
            ident,kind,sector,entries=r[3:];assert kind in(DOOR,PLAT,FLOOR)and 1<=entries<=2
            store[key].create(ident);handles[key,ident]=store[key].handles[ident];sectors[key,ident]=sector
        elif r[0]=='SD':assert (key,r[3])in handles and(key,r[3])not in removed;removed.add((key,r[3]))
        elif r[0]=='SF':assert (key,r[3])in removed;store[key].free(r[3])
        if key in store:tables[key,r[2]]=list(store[key].generations)
    for key,s in store.items():
        for ident,h in s.handles.items():handles.setdefault((key,ident),h)
    return handles,tables,sectors
def record_fields(slot,entry,sector_of,tags,lights):
    """One occupied slot: the DSIM34 light record, or a mover record (M3h8 note §10, N6)."""
    kind=entry[0];p=f'sector_thinker.{slot}'
    if kind<DOOR:return [(path,v)for path,v in lights if path.startswith(p+'.')]
    out=[(p+'.generation',None),(p+'.occupied',b'\1'),(p+'.kind',n('B',kind))]
    if kind==REMOVED:
        assert len(entry)==2
        return out+[(p+'.sector',n('H',sector_of))]
    sector=entry[2];assert sector==sector_of
    out.append((p+'.sector',n('H',sector)))
    if kind==DOOR:
        typ,direction,top,countdown,speed,wait=entry[3:]
        assert typ in DOOR_SPEED and speed==DOOR_SPEED[typ]and wait==DOOR_WAIT and direction in(-1,0,1)
        out+=[(p+'.type',n('B',typ)),(p+'.direction',n('i',direction)),(p+'.topheight',n('i',top)),(p+'.topcountdown',n('i',countdown))]
    elif kind==PLAT:
        typ,status,count,low,high,speed,wait,crush,tag,oldstatus=entry[3:]
        assert typ==DOWN_WAIT_UP_STAY and speed==PLAT_SPEED and wait==PLAT_WAIT and crush==0 and oldstatus==0 and status in(0,1,2)
        assert tag==tags[sector]and low<=high
        out+=[(p+'.status',n('B',status)),(p+'.count',n('i',count)),(p+'.low',n('i',low)),(p+'.high',n('i',high))]
    else:
        typ,direction,dest,speed,crush,newspecial,texture=entry[3:]
        assert typ in FLOOR_SPEED and speed==FLOOR_SPEED[typ]and direction==-1 and crush==0 and newspecial==0 and texture==0
        out+=[(p+'.type',n('B',typ)),(p+'.floordestheight',n('i',dest))]
    return out
def store_fields(entries,generations,handles,case,sector_of,tags):
    by_slot={}
    for entry in entries:
        kind,ident=entry[:2]
        slot,generation=(ident,1)if kind<DOOR else handles[case,ident]
        assert generations[slot]==generation and slot not in by_slot
        by_slot[slot]=entry
    lights=inherited.sector_thinker_fields([e for e in entries if e[0]<DOOR])
    out=[('sector_thinkers.slot_count',n('Q',len(generations)))]
    for slot,generation in enumerate(generations):
        p=f'sector_thinker.{slot}'
        if slot not in by_slot:out+=[(p+'.generation',n('Q',generation)),(p+'.occupied',b'\0')];continue
        entry=by_slot[slot]
        fields=record_fields(slot,entry,sector_of.get((case,entry[1]))if entry[0]>=DOOR else None,tags,lights)
        out+=[(path,n('Q',generation)if path.endswith('.generation')else v)for path,v in fields]
    return out
def events(rows):
    """Tag-26 records in source position. Every inherited row goes through the frozen
    DSIM34 chain; a chunk is flushed only at a Stage B row. A crossing (MX..MY, with the
    mover rows its line action nests) happens inside P_TryMove before the C harness
    prints the movement result, so its records wait for that M row and follow the
    movement attempt's records."""
    out=[];chunk=[];deferred=None;crossing=None;clip=None
    def flush():
        nonlocal chunk
        out.extend(INHERITED_EVENTS(chunk));chunk=[]
    def emit(record):
        (deferred if deferred is not None else out).append(record)
    for r in rows:
        tag=r[0]
        if tag not in NEW:
            assert deferred is None or tag in{'C','O','L','M'},('a crossing without its movement result',r)
            chunk.append(r)
            if tag=='M'and deferred is not None:
                assert crossing is None
                flush();out.extend(deferred);deferred=None
            continue
        if tag in STATE:continue
        if tag=='MX':
            assert crossing is None
            if deferred is None:deferred=[]
            crossing=r[3:];continue
        if deferred is None:flush()
        sub=lambda name:bytes([MOVERS,SUBTAGS[name]])
        if tag=='MY':
            thing,line,side,before=crossing;assert r[3:5]==[thing,line]
            emit(sub('crossing')+base.ident(thing)+n('H',line)+n('B',side)+n('h',before)+n('h',r[5]));crossing=None;continue
        if tag=='MA':emit(sub('use')+base.ident(r[3]));continue
        if tag=='MI':
            line,frac,special,openrange,result=r[3:];assert result in(0,1)and(special==0)==(openrange!=-1)
            emit(sub('use_intercept')+n('H',line)+n('i',frac)+n('B',result));continue
        if tag=='MU':
            thing,line,side,before,result,after=r[3:];assert side in(0,1)and result in(0,1)
            emit(sub('special_use')+base.ident(thing)+n('H',line)+n('B',side)+n('h',before)+n('B',result)+n('h',after));continue
        if tag=='MP':
            actor,_movedir,count=r[3:];assert 1<=count<=8
            emit(sub('spechit_blocked')+base.ident(actor)+n('B',count));continue
        if tag=='MM':
            ident,kind,sector,_=r[3:]
            emit(sub('mover_start')+handle(HANDLES[r[1],ident])+n('B',kind)+n('H',sector));continue
        if tag=='ME':
            fn,line,arg,_thing,result,before,after=r[3:];assert fn in(1,2,3,4)and after>=before
            emit(sub('mover_call')+n('B',fn)+n('H',line)+n('i',arg)+n('i',result)+n('B',after-before));continue
        if tag=='MV':
            sector,plane,direction,speed,dest,before,after,result=r[3:];assert plane in(0,1)and direction in(-1,1)and result in(0,1,2)
            emit(sub('plane_step')+n('H',sector)+n('B',plane)+n('i',direction)+n('i',speed)+n('i',dest)+n('i',before)+n('i',after)+n('B',result));continue
        if tag=='MC':
            if r[3]==0:assert r[5]==0;continue
            emit(sub('change_sector')+n('H',r[4])+n('B',r[5]));continue
        if tag=='HC':assert clip is None;clip=r[3:];continue
        if tag=='MT':
            thing,_result,_zbefore,z,floor,ceiling,_height,_count=clip;clip=None
            assert r[3]==thing and r[4]in(0,1,2,3,4)
            emit(sub('clip')+base.ident(thing)+n('B',r[4])+n('i',z)+n('i',floor)+n('i',ceiling));continue
        if tag=='SD':emit(sub('mover_removed')+handle(HANDLES[r[1],r[3]]));continue
        if tag=='SF':emit(sub('mover_freed')+handle(HANDLES[r[1],r[3]]));continue
        if tag=='SO':
            kind,index,sound=r[3:];assert kind in(1,2)
            emit(sub('sector_sound')+n('B',kind)+n('H',index)+n('H',sound));continue
        if tag=='MG':
            if r[3]==0:assert r[4:]==[GA_COMPLETED,0];emit(sub('exit')+b'\0')
            continue
        if tag=='MR':emit(sub('reborn')+b'\0');continue
        raise ValueError(r)
    assert deferred is None and crossing is None and clip is None
    flush()
    return out
INHERITED_EVENTS=inherited.events
HANDLES={}
def movers(fields,case,op,thinkers,generations,mh,ml,mz,state,tags):
    """Insert the DSIM35 fields into one finished DSIM34-layout capture."""
    value=dict(fields)
    usedown,playerstate,gameaction,secretexit,_buttons=mz
    assert usedown in(0,1)and gameaction in(0,GA_COMPLETED)and secretexit==0 and playerstate==state[0]
    count,*heights=mh;assert len(heights)==3*count and struct.unpack('<I',value['sectors.count'])[0]==count
    order=b'';entries=[];live={}
    for kind,ident,*rest in thinkers:
        if kind in(0,9):order+=b'\0'+base.ident(ident);continue
        h=(ident,1)if kind<DOOR else HANDLES[case,ident];order+=b'\1'+handle(h);entries.append((kind,ident,*rest))
        if kind in(DOOR,PLAT,FLOOR):
            assert rest[0]not in live,'one live mover per sector';live[rest[0]]=ident
    # sector->specialdata is derived: the live mover naming the sector (lights never set it).
    for i in range(count):assert heights[3*i+2]==live.get(i,-1),(case,op,i)
    lump=value['map.linedefs'];special={i:struct.unpack_from('<h',lump,14*i+6)[0]for i in range(len(lump)//14)}
    pairs=dict(zip(ml[::2],ml[1::2]));assert sorted(pairs)==[i for i,s in special.items()if s]
    for i,s in pairs.items():assert s in(0,special[i])
    stored=[];inside=False
    for path,v in fields:
        if path=='sector_thinkers.slot_count':inside=True;stored+=store_fields(entries,generations,HANDLES,case,SECTORS,tags)
        if path=='spatial.link_order':inside=False
        if inside:continue
        if path=='thinker.shared_order':v=order
        if path.startswith('sector.')and path.split('.')[2]in('floor','ceiling'):
            i=int(path.split('.')[1]);v=n('i',heights[3*i+(path.endswith('.ceiling'))])
        if path=='player.0.actor':stored+=[(f'line.{i}.special',n('h',s))for i,s in sorted(pairs.items())]
        stored.append((path,v))
        if path=='level.single_player_kill_count':stored.append(('level.exit',n('B',gameaction==GA_COMPLETED)))
        elif path=='original_player.0.secret_count':stored.append(('original_player.0.use_down',n('B',usedown)))
    assert not inside
    return stored
SECTORS={}
def build(cases,rows,maps,prefix,full):
    """DSIM35 outputs of the stripped rows (strip() first); a case that requested an
    exit or a reborn ends at that op (its MG end-of-op row), so its frames end there."""
    ended={r[1]:r[2]for r in rows if r[0]=='MG'and r[3]in(1,2)}
    cases=[dict(c,tics=str(ended[i]))if i in ended else c for i,c in enumerate(cases)]
    handles,tables,sectors=stores(rows)
    HANDLES.clear();HANDLES.update(handles);SECTORS.clear();SECTORS.update(sectors)
    thinkers=collections.defaultdict(list);mh={};ml={};mz={};state={}
    for r in rows:
        if r[0]=='TK':
            assert r[3]==len(thinkers[r[1],r[2]])
            thinkers[r[1],r[2]].append((r[4],r[5],*r[6:]))
        elif r[0]=='MH':mh[r[1],r[2]]=r[3:]
        elif r[0]=='ML':ml[r[1],r[2]]=r[3:]
        elif r[0]=='MZ':mz[r[1],r[2]]=r[3:]
        elif r[0]=='P':state[r[1],r[2]]=r[5:6]
    selected=iter((r[1],r[2])for r in rows if r[0]=='P');added=[]
    real=map_scale.execution.build
    def hooked(*a,**k):
        inner=physical.frame
        def frame(fields):
            data=inner(fields);case,op=next(selected)
            version,old=inherited.decode(data);assert version==VERSION
            lump=dict(old)['map.sectors'];tags=[struct.unpack_from('<h',lump,26*i+24)[0]for i in range(len(lump)//26)]
            new=movers(old,case,op,thinkers[case,op],tables[case,op],mh[case,op],ml[case,op],mz[case,op],state[case,op],tags)
            added.append(len(new)-len(old));return inherited.encode(new)
        physical.frame=frame
        try:outputs,inventory=real(*a,**k)
        finally:physical.frame=inner
        path='tests/reference/'+a[5]+'_snapshots.csv';lines=outputs[path].decode().splitlines()
        assert len(lines)-1==len(added)
        for i in range(1,len(lines)):
            v=lines[i].split(',');v[6]=str(int(v[6])+added[i-1]);lines[i]=','.join(v)
        outputs[path]=('\n'.join(lines)+'\n').encode()
        return outputs,inventory
    old=(inherited.VERSION,inherited.POLICY,inherited.CONTEXT,inherited.events,map_scale.execution.build)
    inherited.VERSION,inherited.POLICY,inherited.CONTEXT,inherited.events,map_scale.execution.build=VERSION,POLICY,CONTEXT,events,hooked
    try:outputs,inventory=inherited.build(cases,plain_thinkers(rows),maps,prefix,full)
    finally:inherited.VERSION,inherited.POLICY,inherited.CONTEXT,inherited.events,map_scale.execution.build=old
    assert next(selected,None)is None
    return outputs,inventory
def plain_thinkers(rows):
    """The rows the DSIM34 layer sees: mover TK entries removed and the light/actor
    entries renumbered (that layer rebuilds only the light store and the actor order;
    the shared order and the store are replaced here)."""
    out=[];index=collections.Counter()
    for r in rows:
        if r[0]=='TK':
            if r[4]in(DOOR,PLAT,FLOOR,REMOVED):continue
            r=[*r[:3],index[r[1],r[2]],*r[4:]];index[r[1],r[2]]+=1
        out.append(r)
    return out
