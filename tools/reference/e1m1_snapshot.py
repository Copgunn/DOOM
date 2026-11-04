"""Independent DSIM36 projection from compiled original-C two-sided scroller, static decoration and spawn-filter rows.

Extends the frozen DSIM35 projection without editing it: the DSIM35 field layout and
event vocabulary under version 36, policy 20 and the E1M1 context, with no new field
and no new event (M3h9 note §6, decision E8; docs/STATE_FORMAT.md, "DSIM v36"):

- a static decoration (the seven E1M1 types of M3h9 note §2.5, admitted by DoomEd
  number) is an ordinary actor record: type (`mobjtype_t` index), state (`statenum_t`
  index of its spawn state), tics -1, health 1000, radius, height and flags from the
  pristine info.c row with `MF_AMBUSH` from the THING record, one `lastlook` draw,
  `spawnpoint` present, no target, no momentum, exactly as the A, F and TK rows give it;
  it sits in `thinker.order`, `thinker.shared_order` (kind 0), `spatial.link_order` and
  `sector.{i}.actors` like every actor, in lump order at load (P_SpawnMapThing per
  THING record, then the player body, then the sector thinkers);
- a two-sided type-48 line is `line.{i}.special` = 48 exactly as any nonzero map
  special in DSIM35 (never cleared: no W1/S1 path); the scroll offset is
  `leveltime * FRACUNIT` (p_spec.c) and is never encoded;
- the spawn filter encodes nothing: a filtered record has no actor and no draw;
- the three M3h9 observer rows (DT in PIT_CheckThing, XT in the aim/shoot traversers,
  NS before every M row) are observations, neither state nor events, and are removed
  before every chain, as TI rows are.

A DSIM36 capture is byte for byte the DSIM35 projection of the same rows except the
version (36), the `profile` value (20) and the `context.schema` bytes; the generator
(m3h9_snapshot_goldens.py) asserts this for every capture. This layer only renames the
identity and checks what the layout may now contain: the decoration records against the
declared table of m3h9_geometry (enum indices read from info.h), their lump order, the
decoration bound (32 per map) and the 48 specials of the two-sided scrollers."""
import collections,struct
import movers_snapshot as inherited
from m3h9_geometry import DECORATIONS,enum_index
specials=inherited.inherited;base=inherited.base;n=base.n;physical=inherited.physical;map_scale=inherited.map_scale
CONTEXT=b'PLAYER-MONSTERS-E1M1-1-MEDIUM-v1'
VERSION=36;POLICY=20;MAX_SLOTS=inherited.MAX_SLOTS;MAX_LIVE_PROJECTILES=inherited.MAX_LIVE_PROJECTILES
MAX_DECORATIONS=32
SCROLL=48
MF_SOLID,MF_AMBUSH=2,32
MTF_AMBUSH=8
REACTION_TIME=8                            # mobjinfo reactiontime of every MT_MISC* row
U=65536
OBSERVERS={'DT','XT','NS'}
# DoomEd -> (mobjtype_t index, statenum_t index, radius, height, info flags), from info.h.
ADMITTED={d:(enum_index('mobjtype_t',member),enum_index('statenum_t',state),radius,height,MF_SOLID if solid else 0)
          for d,(_,member,state,radius,height,solid)in DECORATIONS.items()}
def strip(rows):
    """The M3h9 observer rows removed, then the DSIM35 strip (use-trace traversal rows,
    clip position checks, the synthetic reborn marker)."""
    return inherited.strip([r for r in rows if r[0]not in OBSERVERS])
def decoration_records(fields,records):
    """Check the decoration records of one DSIM36 capture against the spawned THING
    records of its lump (F rows: ident, x, y, angle, DoomEd type, options, in lump
    order). Returns (decoration records, records with MF_AMBUSH, scroller line fields)."""
    value=dict(fields);tic=struct.unpack('<Q',value['tic'])[0]
    order=value['thinker.order'];idents=[order[k:k+12]for k in range(0,len(order),12)]
    spawned=[base.ident(ident)for ident,*_ in records]
    if tic==0:assert idents[:len(spawned)]==spawned and len(idents)==len(spawned)+1,'lump order, then the player body'
    decorations=[(identity,record)for identity,record in zip(spawned,records)if record[4]in ADMITTED]
    assert len(decorations)<=MAX_DECORATIONS
    assert [i for i in idents if i in{d for d,_ in decorations}]==[d for d,_ in decorations],'decorations are never removed: lump order'
    ambush=0
    for identity,(_,x,y,angle,doomed,options)in decorations:
        slot=struct.unpack_from('<I',identity)[0];p=f'actor.{slot}'
        typ,state,radius,height,flags=ADMITTED[doomed]
        flags|=MF_AMBUSH if options&MTF_AMBUSH else 0;ambush+=bool(options&MTF_AMBUSH)
        expected={'occupied':b'\1','retired':b'\0','pending_removal':b'\0','type':n('H',typ),'state':n('H',state),'tics':n('i',-1),
                  'health':n('i',1000),'radius':n('i',radius*U),'height':n('i',height*U),'flags':n('I',flags),'reaction_time':n('i',REACTION_TIME),
                  'spawnpoint':b'\1'+struct.pack('<hhhhH',x,y,angle,doomed,options),'target':bytes(13),'momx':n('i',0),'momy':n('i',0),'momz':n('i',0),
                  'threshold':n('i',0),'movedir':b'\0','movecount':n('i',0),'player':b'\xff','z':value[p+'.floor']}
        for key,v in expected.items():assert value[p+'.'+key]==v,(p,key,value[p+'.'+key],v)
        assert value[p+'.lastlook'][0]<4
    lump=value['map.linedefs'];scrollers=[i for i in range(len(lump)//14)if struct.unpack_from('<h',lump,14*i+6)[0]==SCROLL]
    for i in scrollers:
        assert struct.unpack_from('<h',lump,14*i+12)[0]>=0,'a type-48 line of this policy is two-sided'
        assert value[f'line.{i}.special']==n('h',SCROLL),'a scroller is never cleared'
    return len(decorations),ambush,len(scrollers)
def build(cases,rows,maps,prefix,full):
    """DSIM36 outputs of the stripped rows (strip() first): the DSIM35 build under the
    new identity, every capture checked by decoration_records."""
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
            counts.append(decoration_records(new,records[case]))
            return data
        physical.frame=frame
        try:return real(*a,**k)
        finally:physical.frame=inner
    old=(inherited.VERSION,inherited.POLICY,inherited.CONTEXT,map_scale.execution.build)
    inherited.VERSION,inherited.POLICY,inherited.CONTEXT,map_scale.execution.build=VERSION,POLICY,CONTEXT,hooked
    try:outputs,inventory=inherited.build(cases,rows,maps,prefix,full)
    finally:inherited.VERSION,inherited.POLICY,inherited.CONTEXT,map_scale.execution.build=old
    assert next(selected,None)is None and len(counts)==inventory['captures']
    inventory.update(decoration_records=sum(c[0]for c in counts),ambush_decoration_records=sum(c[1]for c in counts),
                     scroller_line_fields=sum(c[2]for c in counts),largest_decoration_count=max(c[0]for c in counts))
    return outputs,inventory
