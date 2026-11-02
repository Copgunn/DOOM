"""Independent DSIM32 projection from compiled original-C diagonal-wall rows.

Extends the frozen DSIM31 projection without editing it: the same field layout
and event vocabulary under version 32, policy 16 and the diagonal context, with
the 320-slot store and the 208 live-fireball bound of the heights policy. Diagonal
walls add no actor or sector state: a slide's outcome is already in the mover's
position, momentum and location fields and the slide event carries its line, so
no field is added.

One derivation changes. Every earlier room builder gave each sector exactly one
subsector, so the frozen layers wrote the C `subsector` index into both
`actor.N.subsector` and `actor.N.sector` and grouped `sector.N.actors` by it. The
M3h5 rooms carry a real BSP with several leaves per sector, so this layer derives
each leaf's sector from the map lumps in the frame itself (SSECTORS first seg ->
SEGS linedef and side -> LINEDEFS sidedef -> SIDEDEFS sector, as `P_LoadSubsectors`
does) and rewrites those fields; `actor.N.subsector` keeps the C leaf. The
`corner_jump` rows after the push are projected like every other row; the Rust
replay refuses that tic (`MapScaleError::Escape`) and never reaches them, which
its test states explicitly."""
import struct
import heights_snapshot as inherited
import map_scale_snapshot as map_scale
CONTEXT=b'PLAYER-MONSTERS-DIAGONAL-1-MEDIUM-v1'
VERSION=32;POLICY=16;MAX_SLOTS=inherited.MAX_SLOTS;MAX_LIVE_PROJECTILES=inherited.MAX_LIVE_PROJECTILES
MF_NOSECTOR=8
def leaf_sectors(lumps):
    """Sector of every subsector, from the lumps alone."""
    linedefs=lumps['linedefs'];sidedefs=lumps['sidedefs'];segs=lumps['segs'];out=[]
    for count,first in struct.iter_unpack('<HH',lumps['subsectors']):
        assert count>0
        _,_,_,line,side,_=struct.unpack_from('<6H',segs,12*first)
        sidedef=struct.unpack_from('<2h',linedefs,14*line+10)[side];assert sidedef>=0
        out.append(struct.unpack_from('<h',sidedefs,30*sidedef+28)[0])
    return out
def fix_sectors(fields):
    lumps={p[4:]:v for p,v in fields if p.startswith('map.')}
    sectors=leaf_sectors(lumps)
    leaf={int(p.split('.')[1]):struct.unpack('<H',v)[0]for p,v in fields if p.startswith('actor.')and p.endswith('.subsector')}
    flags={int(p.split('.')[1]):struct.unpack('<I',v)[0]for p,v in fields if p.startswith('actor.')and p.endswith('.flags')}
    link=next(v for p,v in fields if p=='spatial.link_order');idents=[link[k:k+12]for k in range(0,len(link),12)]
    def actors(sector):
        return b''.join(i for i in idents if sectors[leaf[struct.unpack('<I',i[:4])[0]]]==sector and not flags[struct.unpack('<I',i[:4])[0]]&MF_NOSECTOR)
    out=[]
    for p,v in fields:
        parts=p.split('.')
        if parts[0]=='actor'and parts[-1]=='sector':v=struct.pack('<H',sectors[leaf[int(parts[1])]])
        elif parts[0]=='sector'and parts[-1]=='actors':v=actors(int(parts[1]))
        out.append((p,v))
    return out
def build(cases,rows,maps,prefix,full):
    real=map_scale.execution.build
    def hooked(*a,**k):
        inner=map_scale.physical.frame
        map_scale.physical.frame=lambda fields:inner(fix_sectors(fields))
        try:return real(*a,**k)
        finally:map_scale.physical.frame=inner
    old=(inherited.VERSION,inherited.POLICY,inherited.CONTEXT,map_scale.execution.build)
    inherited.VERSION,inherited.POLICY,inherited.CONTEXT,map_scale.execution.build=VERSION,POLICY,CONTEXT,hooked
    try:return inherited.build(cases,rows,maps,prefix,full)
    finally:inherited.VERSION,inherited.POLICY,inherited.CONTEXT,map_scale.execution.build=old
