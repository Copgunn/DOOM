"""Independent DSIM34 projection from compiled original-C sky and static-specials rows.

Extends the frozen DSIM33 projection without editing it: the DSIM33 field layout and
event vocabulary under version 34, policy 18 and the specials context, plus exactly the
Stage A state (docs/STATE_FORMAT.md, "DSIM v34"):

- `thinker.shared_order` after `thinker.order`: the one C thinker list (TK rows) as
  13-byte entries, u8 kind (0 actor, 1 sector thinker) + u32 slot + u64 generation;
- the sector-thinker store before `spatial.link_order`: `sector_thinkers.slot_count`
  and, per slot, generation, occupied, kind, sector, count (not for a glow), minlight,
  maxlight and the kind's own fields (TK rows);
- `sector.{i}.light` and `sector.{i}.special` (i16 runtime values, SX rows) after each
  `sector.{i}.soundtarget`;
- `original_player.0.secret_count` (i32, SX rows) after `original_player.0.armor.points`;
- event tag 25: 0 secret entry (SE), 1 fireball removed at a sky ceiling line (KR with
  its KY line), 2 hitscan ended at a sky ceiling (KS), the last placed right after the
  trace record of the hitscan it ended.

Every other new observer row (TK, SX, LW, LE, LR, LC, SP, KY, KL, KZ) is persistent
state or a read-only observation and is dropped from the event chain; light draws are
`E,...,8` rows, which no DSIM vocabulary turns into events. Environmental damage and
death are the existing G/AR/PL rows with no inflictor and no source. Sector thinkers
are never created after load nor removed in Stage A, so every sector-thinker handle has
generation 1 and slot = creation index; the projection asserts it."""
import collections,struct
import real_map_snapshot as inherited
diagonal=inherited.inherited;heights=diagonal.inherited;map_scale=diagonal.map_scale
base=map_scale.base;n=base.n;physical=map_scale.physical
CONTEXT=b'PLAYER-MONSTERS-SPECIALS-1-MEDIUM-v1'
VERSION=34;POLICY=18;MAX_SLOTS=inherited.MAX_SLOTS;MAX_LIVE_PROJECTILES=inherited.MAX_LIVE_PROJECTILES
SECRET=9
LIGHT_KINDS=(1,2,3,4)  # flash, strobe, glow, flicker (the TK kind codes)
SPECIAL_EVENTS=25
DROPPED={'TK','SX','LW','LE','LR','LC','SP','KY','KL','KZ'}
def events(rows):
    """Tag-25 events in source position; every inherited row goes through the frozen
    chain in the same chunks it would see without them (a chunk is flushed only at an
    SE or KR row, or right after the X end row of a hitscan that a KS row ended)."""
    out=[];chunk=[];line={};shot=None
    def flush():
        nonlocal chunk
        out.extend(INHERITED_EVENTS(chunk));chunk=[]
    for r in rows:
        tag=r[0]
        if tag=='KY':line[r[3]]=r[4:]
        if tag in DROPPED:continue
        if tag=='SE':
            flush();sector,count=r[3:];assert count>=1
            out.append(bytes([SPECIAL_EVENTS,0])+n('H',sector)+n('i',count));continue
        if tag=='KR':
            ceiling,back,sky=line.pop(r[3]);assert ceiling>=0 and back>=0 and sky==1
            flush();out.append(bytes([SPECIAL_EVENTS,1])+base.ident(r[3])+n('H',ceiling));continue
        if tag=='KS':
            code,li,_front,back,z=r[3:];assert shot is None and code in(0,1)and(code==0 or back>=0)
            shot=bytes([SPECIAL_EVENTS,2,code])+n('H',li)+n('i',z);continue
        chunk.append(r)
        if tag=='X'and r[3]==1 and shot is not None:
            assert r[4]==-1,'a sky shot has no hit'
            flush();out.append(shot);shot=None
    flush()
    if shot is not None:out.append(shot)  # only in the discarded non-stage scaffolding
    return out
INHERITED_EVENTS=heights.events
def sector_thinker_fields(entries):
    fields=[('sector_thinkers.slot_count',n('Q',len(entries)))]
    for slot,(kind,sid,sector,count,minlight,maxlight,a,b)in enumerate(entries):
        assert sid==slot and kind in LIGHT_KINDS
        p=f'sector_thinker.{slot}'
        fields+=[(p+'.generation',n('Q',1)),(p+'.occupied',b'\1'),(p+'.kind',n('B',kind)),(p+'.sector',n('H',sector))]
        if kind==3:assert count==-1 and b==0 and a in(-1,1)
        else:fields.append((p+'.count',n('i',count)))
        fields+=[(p+'.minlight',n('i',minlight)),(p+'.maxlight',n('i',maxlight))]
        if kind==1:fields+=[(p+'.maxtime',n('i',a)),(p+'.mintime',n('i',b))]
        elif kind==2:fields+=[(p+'.darktime',n('i',a)),(p+'.brighttime',n('i',b))]
        elif kind==3:fields.append((p+'.direction',n('i',a)))
        else:assert a==b==0
    return fields
def decode(data):
    assert data[:4]==b'DSIM';version,count=struct.unpack_from('<IQ',data,4);at=16;fields=[]
    for _ in range(count):
        size,=struct.unpack_from('<I',data,at);at+=4;path=data[at:at+size].decode();at+=size
        size,=struct.unpack_from('<Q',data,at);at+=8;fields.append((path,data[at:at+size]));at+=size
    assert at==len(data);return version,fields
def encode(fields):
    data=bytearray(b'DSIM'+struct.pack('<IQ',VERSION,len(fields)))
    for path,value in fields:
        path=path.encode();data+=n('I',len(path))+path+n('Q',len(value))+value
    return bytes(data)
def specials(fields,thinkers,sx,initial):
    """Insert the DSIM34 fields into one finished DSIM33-layout capture."""
    value=dict(fields)
    leveltime,secrets,total,count,*pairs=sx
    assert len(pairs)==2*count and struct.unpack('<I',value['sectors.count'])[0]==count
    # level.total_secrets is P_SpawnSpecials' totalsecret, the special-9 sectors of the lump.
    lump=value['map.sectors'];assert len(lump)==26*count
    assert total==sum(struct.unpack_from('<h',lump,26*i+22)[0]==SECRET for i in range(count))
    assert struct.unpack('<i',value['level.total_secrets'])[0]==total and 0<=secrets<=total
    assert struct.unpack('<Q',value['tic'])[0]==leveltime
    order=b'';actors=[];lights=[]
    for kind,ident,*rest in thinkers:
        if kind in(0,9):
            actors.append(base.ident(ident));order+=b'\0'+base.ident(ident)
        else:
            lights.append((kind,ident,*rest));order+=b'\1'+struct.pack('<IQ',ident,1)
    assert b''.join(actors)==value['thinker.order']
    assert [l[:3]for l in lights]==[l[:3]for l in initial],'sector thinkers are created at load only'
    out=[]
    for path,v in fields:
        if path=='spatial.link_order':out+=sector_thinker_fields(lights)
        out.append((path,v))
        if path=='thinker.order':out.append(('thinker.shared_order',order))
        elif path.startswith('sector.')and path.endswith('.soundtarget'):
            i=int(path.split('.')[1]);light,special=pairs[2*i:2*i+2]
            out+=[(f'sector.{i}.light',n('h',light)),(f'sector.{i}.special',n('h',special))]
        elif path=='original_player.0.armor.points':out.append(('original_player.0.secret_count',n('i',secrets)))
    return out
def build(cases,rows,maps,prefix,full):
    rows=[r for r in rows if r[0]!='TI']
    thinkers=collections.defaultdict(list);sx={}
    for r in rows:
        if r[0]=='TK':
            assert r[3]==len(thinkers[r[1],r[2]])
            thinkers[r[1],r[2]].append((r[4],r[5],*r[6:]))
        elif r[0]=='SX':sx[r[1],r[2]]=r[3:]
    secrets={(r[1],r[2]):r[3:]for r in rows if r[0]=='SE'}
    for (case,op),(sector,count) in secrets.items():
        assert sx[case,op][1]==count and sx[case,op][5+2*sector]==0
    selected=iter((r[1],r[2])for r in rows if r[0]=='P');added=[]
    real=map_scale.execution.build
    def hooked(*a,**k):
        inner=physical.frame
        def frame(fields):
            data=inner(fields);case,op=next(selected)
            initial=[t for t in thinkers[case,0]if t[0]in LIGHT_KINDS]
            version,old=decode(data);assert version==VERSION
            new=specials(old,thinkers[case,op],sx[case,op],initial);added.append(len(new)-len(old))
            return encode(new)
        physical.frame=frame
        try:outputs,inventory=real(*a,**k)
        finally:physical.frame=inner
        path='tests/reference/'+a[5]+'_snapshots.csv';lines=outputs[path].decode().splitlines()
        assert len(lines)-1==len(added)
        for i in range(1,len(lines)):
            v=lines[i].split(',');v[6]=str(int(v[6])+added[i-1]);lines[i]=','.join(v)
        outputs[path]=('\n'.join(lines)+'\n').encode()
        return outputs,inventory
    old=(inherited.VERSION,inherited.POLICY,inherited.CONTEXT,heights.events,map_scale.execution.build)
    inherited.VERSION,inherited.POLICY,inherited.CONTEXT,heights.events,map_scale.execution.build=VERSION,POLICY,CONTEXT,events,hooked
    try:outputs,inventory=inherited.build(cases,rows,maps,prefix,full)
    finally:inherited.VERSION,inherited.POLICY,inherited.CONTEXT,heights.events,map_scale.execution.build=old
    assert next(selected,None)is None
    return outputs,inventory
