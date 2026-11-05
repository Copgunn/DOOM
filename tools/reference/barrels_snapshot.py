"""Independent DSIM37 projection from compiled original-C barrel, radius-attack, pickup-scale and angle rows.

Extends the frozen DSIM36 projection without editing it (M3h9b note §6, decisions F2
and F9): the DSIM36 field layout and event vocabulary under version 37, policy 21 and
the barrels context, with **no new field and no new event**:

- an exploding barrel (DoomEd 2035, `MT_BARREL`) is an ordinary actor record: type,
  state (`S_BAR1`/`S_BAR2` alive, `S_BEXP`..`S_BEXP5` dying, `S_GIBS` crunched by a
  mover), tics, health, radius, height, flags, `target`/`threshold`, momentum, z and
  `lastlook` exactly as the C A rows give them; the encoder projects C, it does not
  simulate, and `barrel_records` only checks what the layout may now contain;
- `store.max_slots` is 384 (the only value that changes in an existing field's
  meaning); the item and bonus ranges are those of F2 (items and total items at most
  64, bonus count at most 6 x 64 = 384). The frozen chain asserts the DSIM30..36
  ranges (16 items, 96 bonus) in two places, the map-scale frame
  (map_scale_snapshot.build) and the bonus event layer (bonus_snapshot.events, the IT
  row and the PN code 5 and 10 payloads); both are replaced for the duration of a
  DSIM37 build only by `scaled_build` and `bonus_events`, which produce the same bytes
  with the F2 ranges. Every inherited module keeps its own behaviour when it runs on
  its own;
- an explosion is the existing tag-12 Action record of `A_Explode` (the C `E,..,5,ident,19`
  row, action index 19) followed by the existing tag-13 records of each victim in walk
  order (the C G/H/K rows the inherited chain already encodes); the M3h9b observer rows
  RA (one per P_RadiusAttack) and RC (one per PIT_RadiusAttack candidate) are
  comparison rows, neither state nor events, and are removed before every chain, as
  the M3h9 observers are.

A DSIM37 capture is byte for byte the DSIM36 projection of the same rows except the
version (37), the `profile` value (21), the `context.schema` bytes and `store.max_slots`
(384); the generator (m3h9b_snapshot_goldens.py) asserts this for every capture."""
import collections,struct
import e1m1_snapshot as inherited
import bonus_snapshot
from m3h9_geometry import enum_index
movers=inherited.inherited;specials=inherited.specials;base=inherited.base;n=base.n;physical=inherited.physical;map_scale=inherited.map_scale
CONTEXT=b'PLAYER-MONSTERS-BARRELS-1-MEDIUM-v1'
VERSION=37;POLICY=21;MAX_SLOTS=384;MAX_LIVE_PROJECTILES=inherited.MAX_LIVE_PROJECTILES
MAX_PICKUPS=64;MAX_BONUS=6*MAX_PICKUPS;MAX_BARRELS=16;MAX_MONSTERS=8;MAX_DECORATIONS=inherited.MAX_DECORATIONS
BARREL=2035
OBSERVERS={'RA','RC'}
U=65536
# mobjflag_t values (p_mobj.h)
MF_SPECIAL,MF_SOLID,MF_SHOOTABLE,MF_AMBUSH,MF_DROPOFF,MF_DROPPED,MF_NOBLOOD,MF_CORPSE=1,2,4,32,0x400,0x20000,0x80000,0x100000
MTF_AMBUSH=8
REACTION_TIME=8                            # mobjinfo[MT_BARREL].reactiontime; P_DamageMobj sets 0 on a survivor
BASETHRESHOLD=100                          # p_inter.c: the retarget block of P_DamageMobj
MT_BARREL=enum_index('mobjtype_t','MT_BARREL')
ALIVE={enum_index('statenum_t',s)for s in('S_BAR1','S_BAR2')}
DYING={enum_index('statenum_t',s)for s in('S_BEXP','S_BEXP2','S_BEXP3','S_BEXP4','S_BEXP5')}
S_GIBS=enum_index('statenum_t','S_GIBS');S_NULL=65535   # DSIM: 65535 = entered null pointer
FIREBALL=map_scale.FIREBALL
def strip(rows):
    """The M3h9b observer rows removed, then the DSIM36 strip (the M3h9 observers, the
    use-trace traversal rows, clip position checks, the synthetic reborn marker)."""
    return inherited.strip([r for r in rows if r[0]not in OBSERVERS])
ORIGINAL_BONUS_EVENTS=bonus_snapshot.events
def bonus_events(rows):
    """bonus_snapshot.events with the F2 ranges. The frozen body asserts at most 16 items
    in the IT row and the PN code 5 and 10 payloads; those rows are encoded here with the
    same bytes (tag 21 sub-tag 2: actor ID, total i32; tag 23 code 5: bonus count i32,
    item count i32; tag 23 code 10: before i32, after i32) under the ranges of F2, and
    every other row goes through the frozen body. The frozen body flushes its chunk at
    each of these rows, so splitting there changes nothing; its slide-trace filter is
    carried across a split by repeating the last TR row (a TR row emits nothing), and a
    split with a held collision prefix fails the frozen body's own assertion."""
    out=[];segment=[];role=None
    for r in rows:
        if r[0]=='TR':role=r
        if r[0]=='IT':
            actor,total=r[3:];assert 1<=total<=MAX_PICKUPS
            out+=ORIGINAL_BONUS_EVENTS(segment);segment=[role]if role else []
            out.append(bytes([21,2])+base.ident(actor)+n('i',total));continue
        if r[0]=='PN'and r[3]==5:
            count,items=r[4:];assert 0<=count<=MAX_BONUS and 0<=items<=MAX_PICKUPS
            out+=ORIGINAL_BONUS_EVENTS(segment);segment=[role]if role else []
            out.append(bytes([23,5])+struct.pack('<ii',count,items));continue
        if r[0]=='PN'and r[3]==10:
            before,after=r[4:];assert 0<=before<after<=MAX_PICKUPS and after==before+1
            out+=ORIGINAL_BONUS_EVENTS(segment);segment=[role]if role else []
            out.append(bytes([23,10])+struct.pack('<ii',before,after));continue
        segment.append(r)
    return out+ORIGINAL_BONUS_EVENTS(segment)
def scaled(slots):
    """map_scale_snapshot.build with the F2 item and bonus ranges and `slots` in
    `store.max_slots`; everything else (the weapon, pickup and armor fields, the kill and
    live-fireball bounds, the DSIM header of the module's current version, the field-count
    column) is the frozen body's."""
    def build(cases,rows,maps,prefix,full):
        bodies=map_scale.inherited.inherited.BODIES;bodies.clear()
        bodies.update({r[1]:r[4]for r in rows if r[0]=='PL'and r[3]==0})
        selected=iter((r[1],r[2])for r in rows if r[0]=='P')
        weapons={(r[1],r[2]):r[3:]for r in rows if r[0]=='WX'}
        pickups={(r[1],r[2]):r[3:]for r in rows if r[0]=='PX'}
        armor={(r[1],r[2]):r[3:]for r in rows if r[0]=='AX'}
        def frame(fields):
            case,op=next(selected);assert next(v for p,v in fields if p=='tic')==n('Q',op)
            at=next(i for i,(p,_)in enumerate(fields)if p=='events.last.count')
            bonus,items,message=pickups[case,op];kind,points=armor[case,op]
            total=struct.unpack('<i',next(v for p,v in fields if p=='level.total_items'))[0]
            kills,credits=(struct.unpack('<i',next(v for p,v in fields if p==name))[0]for name in['level.total_kills','level.single_player_kill_count'])
            live=sum(p.endswith('.type')and struct.unpack('<H',v)[0]==FIREBALL for p,v in fields)
            assert 0<=bonus<=MAX_BONUS and 0<=items<=total<=MAX_PICKUPS and message in set(range(14))-{7}
            assert 0<=credits<=kills<=MAX_MONSTERS and live<=map_scale.MAX_LIVE_PROJECTILES
            bonus_snapshot.armor_state(kind,points)
            fields=[(p,n('I',slots)if p=='store.max_slots'else v)for p,v in fields]
            fields=fields[:at]+bonus_snapshot.weapon.weapon_fields(weapons[case,op])+[
                ('original_player.0.pickups.present',b'\1'),
                ('original_player.0.pickups.bonus_count',n('i',bonus)),
                ('original_player.0.pickups.item_count',n('i',items)),
                ('original_player.0.pickups.message',n('B',message)),
                ('original_player.0.armor.type',n('B',kind)),
                ('original_player.0.armor.points',n('i',points)),
            ]+fields[at:]
            data=bytearray(b'DSIM'+struct.pack('<IQ',map_scale.VERSION,len(fields)))
            for path,value in fields:
                path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
            return bytes(data)
        old_events,old_frame=physical.events,physical.frame
        physical.events,physical.frame=map_scale.events,frame
        try:
            outputs,inventory=map_scale.execution.build(cases,rows,maps,map_scale.POLICY,map_scale.CONTEXT,prefix,full)
            assert next(selected,None)is None
            path='tests/reference/'+prefix+'_snapshots.csv';lines=outputs[path].decode().splitlines()
            for i in range(1,len(lines)):
                v=lines[i].split(',');v[6]=str(int(v[6])+24);lines[i]=','.join(v)
            outputs[path]=('\n'.join(lines)+'\n').encode()
            return outputs,inventory
        finally:physical.events,physical.frame=old_events,old_frame
    return build
def ranges(slots):
    """Install the F2 ranges (and `slots`) into the frozen chain; returns what restore()
    puts back."""
    old=(map_scale.build,bonus_snapshot.events)
    map_scale.build,bonus_snapshot.events=scaled(slots),bonus_events
    return old
def restore(old):
    map_scale.build,bonus_snapshot.events=old
def dsim36_scaled(cases,rows,maps,prefix,full):
    """The DSIM36 projection (e1m1_snapshot.build, 320 slots) with only the F2 item and
    bonus ranges lifted: the comparison baseline for captures outside the DSIM36 ranges."""
    old=ranges(map_scale.MAX_SLOTS)
    try:return inherited.build(cases,rows,maps,prefix,full)
    finally:restore(old)
def barrel_records(fields,records):
    """Check the barrel records of one DSIM37 capture against the spawned THING records
    of its lump (F rows: ident, x, y, angle, DoomEd type, options, in lump order) and the
    capture-wide bounds. Returns (barrel records, alive, dying, gibbed, with target,
    reaction_time 0, at S_NULL pending removal)."""
    value=dict(fields)
    assert value['store.max_slots']==n('I',MAX_SLOTS)
    slots=struct.unpack('<Q',value['store.slot_count'])[0];assert slots<=MAX_SLOTS
    occupied=[int(p.split('.')[1])for p,v in fields if p.startswith('actor.')and p.endswith('.occupied')and v==b'\1']
    assert len(occupied)<=MAX_SLOTS
    order=value['thinker.order'];idents=[order[k:k+12]for k in range(0,len(order),12)]
    total,items=(struct.unpack('<i',value[k])[0]for k in('level.total_items','original_player.0.pickups.item_count'))
    bonus=struct.unpack('<i',value['original_player.0.pickups.bonus_count'])[0]
    assert 0<=items<=total<=MAX_PICKUPS and 0<=bonus<=MAX_BONUS
    flags={s:struct.unpack('<I',value[f'actor.{s}.flags'])[0]for s in occupied}
    types={s:struct.unpack('<H',value[f'actor.{s}.type'])[0]for s in occupied}
    assert sum(1 for s in occupied if flags[s]&MF_SPECIAL and not flags[s]&MF_DROPPED)<=MAX_PICKUPS
    barrels=[(base.ident(ident),record)for ident,*record in records if record[3]==BARREL]
    live=[(identity,record)for identity,record in barrels if identity in idents]
    by_slot={struct.unpack_from('<I',identity)[0]:record for identity,record in live}
    assert sorted(by_slot)==sorted(s for s in occupied if types[s]==MT_BARREL),'every MT_BARREL actor is a spawned 2035 record'
    assert len(live)<=MAX_BARRELS
    counts=collections.Counter()
    for slot,(x,y,angle,doomed,options)in sorted(by_slot.items()):
        p=f'actor.{slot}';get=lambda k,c:struct.unpack('<'+c,value[p+'.'+k])[0]
        state,health,threshold=get('state','H'),get('health','i'),get('threshold','i')
        ambush=MF_AMBUSH if options&MTF_AMBUSH else 0
        if state in ALIVE:
            kind='alive';assert 1<=health<=20 and 1<=get('tics','i')<=6
            expected=dict(radius=n('i',10*U),height=n('i',42*U),flags=n('I',MF_SOLID|MF_SHOOTABLE|MF_NOBLOOD|ambush))
        elif state in DYING or state==S_NULL:
            # S_BEXP5 ends at S_NULL: P_SetMobjState enters the null pointer (65535,
            # tics 0) and P_RemoveMobj marks the actor; the record stays in the
            # thinker order, pending removal, until P_RunThinkers unlinks it.
            kind='dying'if state in DYING else'removed';assert health<=0
            expected=dict(radius=n('i',10*U),height=n('i',(42*U)>>2),flags=n('I',MF_SOLID|MF_NOBLOOD|MF_CORPSE|MF_DROPOFF|ambush),
                          pending_removal=n('B',state==S_NULL))
            if state==S_NULL:assert get('tics','i')==0
        else:
            kind='gibbed';assert state==S_GIBS and health<=0
            expected=dict(radius=n('i',0),height=n('i',0),flags=n('I',MF_NOBLOOD|MF_CORPSE|MF_DROPOFF|ambush))
        assert threshold in(0,BASETHRESHOLD)and(value[p+'.target'][0]==1)==(threshold==BASETHRESHOLD),(p,threshold)
        # P_SpawnMobj sets info->reactiontime (8); every P_DamageMobj that leaves the
        # target alive sets 0 before the retarget block (p_inter.c), so an untouched barrel
        # (health 20) reads 8 and a retargeted one 0.
        reaction=get('reaction_time','i');assert reaction in(0,REACTION_TIME)
        if kind=='alive'and health==20:assert reaction==REACTION_TIME
        if threshold==BASETHRESHOLD:assert reaction==0
        expected.setdefault('pending_removal',b'\0')
        expected.update({'occupied':b'\1','retired':b'\0','type':n('H',MT_BARREL),
                         'spawnpoint':b'\1'+struct.pack('<hhhhH',x,y,angle,doomed,options),'movedir':b'\0','movecount':n('i',0),'player':b'\xff',
                         'death_requested':b'\0'})
        for key,v in expected.items():assert value[p+'.'+key]==v,(p,key,value[p+'.'+key],v)
        assert value[p+'.lastlook'][0]<4
        counts[kind]+=1;counts['target']+=threshold==BASETHRESHOLD;counts['awake']+=reaction==0
    return len(live),counts['alive'],counts['dying'],counts['gibbed'],counts['target'],counts['awake'],counts['removed']
def build(cases,rows,maps,prefix,full):
    """DSIM37 outputs of the stripped rows (strip() first): the DSIM36 build under the
    new identity and the F2 ranges, every capture checked by barrel_records."""
    records=collections.defaultdict(list)
    for r in rows:
        if r[0]=='F'and r[2]==0 and r[3]>=0 and r[10]>r[9]:records[r[1]].append((r[9],*r[4:9]))
    selected=iter((r[1],r[2])for r in rows if r[0]=='P');counts=[]
    real=map_scale.execution.build
    def hooked(*a,**k):
        inner=physical.frame
        def frame(fields):
            data=inner(fields);case,op=next(selected)
            version,new=specials.decode(data);value=dict(new)
            assert version==VERSION and value['profile']==n('I',POLICY)and value['context.schema']==CONTEXT and value['tic']==n('Q',op)
            counts.append(barrel_records(new,records[case]))
            return data
        physical.frame=frame
        try:return real(*a,**k)
        finally:physical.frame=inner
    old=(inherited.VERSION,inherited.POLICY,inherited.CONTEXT,map_scale.execution.build)
    scale=ranges(MAX_SLOTS)
    try:
        inherited.VERSION,inherited.POLICY,inherited.CONTEXT,map_scale.execution.build=VERSION,POLICY,CONTEXT,hooked
        outputs,inventory=inherited.build(cases,rows,maps,prefix,full)
    finally:
        restore(scale)
        inherited.VERSION,inherited.POLICY,inherited.CONTEXT,map_scale.execution.build=old
    assert next(selected,None)is None and len(counts)==inventory['captures']
    inventory.update(barrel_records=sum(c[0]for c in counts),alive_barrel_records=sum(c[1]for c in counts),
                     dying_barrel_records=sum(c[2]for c in counts),gibbed_barrel_records=sum(c[3]for c in counts),
                     targeted_barrel_records=sum(c[4]for c in counts),awake_barrel_records=sum(c[5]for c in counts),removed_barrel_records=sum(c[6]for c in counts),largest_barrel_count=max(c[0]for c in counts))
    return outputs,inventory
