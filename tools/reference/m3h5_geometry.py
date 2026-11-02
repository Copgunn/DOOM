"""Polygon rooms with diagonal walls and static heights for the M3h5 observer.

A room is a list of sectors, each a list of integer vertex rings: the first ring is
the sector's outer boundary listed clockwise (y up), so every wall has its sector on
its right, which is the source front side (`P_PointOnLineSide` returns 0 there);
further rings are holes (pillars) listed counterclockwise for the same reason. An
edge shared by two sectors in opposite directions becomes one two-sided linedef
(ML_TWOSIDED plus the caller's extra flags, front = the sector listed first); every
other edge is a one-sided ML_BLOCKING wall split into pieces of at most `segment`
map units at exact integer points. Segs follow the source side convention (front
side keeps the line direction, back side reverses it). The BSP is built by a
recursive splitter over an explicit partition tree: `None` is one leaf, otherwise
`((ax, ay), (bx, by), front, back)` names a linedef by its endpoints and partitions
the current segs along it (seg splits are computed exactly and must land on integer
coordinates; collinear segs go to the side their direction faces); the root is the
last node. Every leaf is checked to be non-empty, convex and of a single sector.
REJECT is zero except for the caller's pairs. The BLOCKMAP lists every linedef in
every cell whose closed 128-unit square meets the closed segment (an exact clip);
its origin is the vertex minimum; line 0 is listed only by each cell's leading 0.
Nothing here reads Rust output."""
import struct
from fractions import Fraction
import generate_m3b as old
BLOCKING,TWOSIDED=1,4
def cross(o,d,p):
    """Twice the signed area: < 0 when `p` is right of the directed line (o, o + d)."""
    return d[0]*(p[1]-o[1])-d[1]*(p[0]-o[0])
def split_wall(a,b,segment):
    length=abs(b[0]-a[0])+abs(b[1]-a[1]);pieces=max(1,-(-length//segment))
    for k in range(pieces+1):
        for p,q in zip(a,b):assert (q-p)*k%pieces==0,(a,b,segment)
    at=lambda k:(a[0]+(b[0]-a[0])*k//pieces,a[1]+(b[1]-a[1])*k//pieces)
    return[(at(k),at(k+1))for k in range(pieces)]
def meets_cell(a,b,cx,cy):
    """Exact Liang-Barsky clip of the closed segment against the closed cell square."""
    t0,t1=Fraction(0),Fraction(1)
    for p,d,lo,hi in((a[0],b[0]-a[0],cx,cx+128),(a[1],b[1]-a[1],cy,cy+128)):
        for q,r in((p-lo,-d),(hi-p,d)):
            # Keep r*t <= q (q is the slack toward this edge, r its rate of loss).
            if r==0:
                if q<0:return False
            elif r>0:t1=min(t1,Fraction(q,r))
            else:t0=max(t0,Fraction(q,r))
    return t0<=t1
def room(sectors,portals=None,reject=(),tree=None,segment=1024):
    """`sectors` lists (rings, floor, ceiling); `portals` maps a sector pair to extra
    linedef flags; `tree` is the explicit partition tree described above."""
    portals=dict(portals or{});n=len(sectors);heights=[(f,c)for _,f,c in sectors]
    vertices=[]
    def vertex(p):
        p=(int(p[0]),int(p[1]))
        if p not in vertices:vertices.append(p)
        return vertices.index(p)
    owner={}
    for s,(rings,_,_) in enumerate(sectors):
        for ring in rings:
            for i,a in enumerate(ring):
                b=ring[(i+1)%len(ring)];assert a!=b and(a,b)not in owner,(s,a,b);owner[(a,b)]=s
    lines=[]
    def add(a,b,front,back,flags):
        assert abs(b[0]-a[0])+abs(b[1]-a[1])<=1536,(a,b)
        lines.append(dict(a=vertex(a),b=vertex(b),front=front,back=back,flags=flags))
    done=set()
    for (a,b),s in owner.items():
        if (a,b)in done:continue
        k=owner.get((b,a))
        if k is None:
            for p,q in split_wall(a,b,segment):add(p,q,s,None,BLOCKING)
        else:
            assert k!=s,(s,a,b);done.add((b,a))
            add(a,b,s,k,TWOSIDED|portals.get((min(s,k),max(s,k)),0))
    linebytes=b'';sidebytes=b'';sidecount=0
    for l in lines:
        indexes=[]
        for sector in[l['front'],l['back']]:
            if sector is None:indexes.append(-1);continue
            indexes.append(sidecount);sidecount+=1
            sidebytes+=old.words([0,0])+old.name('-')*2+old.name('TESTWALL')+old.words([sector])
        l['sides']=indexes;linebytes+=old.words([l['a'],l['b'],l['flags'],0,0,*indexes])
    segs=[]
    for i,l in enumerate(lines):
        segs.append(dict(a=l['a'],b=l['b'],line=i,side=0,sector=l['front']))
        if l['back']is not None:segs.append(dict(a=l['b'],b=l['a'],line=i,side=1,sector=l['back']))
    nodes=[];leaves=[];nodebytes=b''
    def bbox(group):
        points=[vertices[s[k]]for s in group for k in('a','b')]
        return[max(p[1]for p in points),min(p[1]for p in points),min(p[0]for p in points),max(p[0]for p in points)]
    def leaf(group):
        assert group,'empty leaf'
        sector={s['sector']for s in group};assert len(sector)==1,('leaf spans sectors',sector)
        for s in group:
            o=vertices[s['a']];d=(vertices[s['b']][0]-o[0],vertices[s['b']][1]-o[1])
            for t in group:
                for k in('a','b'):assert cross(o,d,vertices[t[k]])<=0,('concave leaf',o,d,vertices[t[k]])
        leaves.append(dict(segs=group,sector=sector.pop()));return 0x8000|(len(leaves)-1)
    def build(group,subtree):
        nonlocal nodebytes
        if subtree is None:return leaf(group)
        a,b,front_tree,back_tree=subtree
        index=next(i for i,l in enumerate(lines)if vertices[l['a']]==tuple(a)and vertices[l['b']]==tuple(b))
        o=tuple(a);d=(b[0]-a[0],b[1]-a[1]);front=[];back=[]
        for s in group:
            pa,pb=vertices[s['a']],vertices[s['b']];ca,cb=cross(o,d,pa),cross(o,d,pb)
            if ca==0 and cb==0:
                (front if d[0]*(pb[0]-pa[0])+d[1]*(pb[1]-pa[1])>0 else back).append(s)
            elif ca<=0 and cb<=0:front.append(s)
            elif ca>=0 and cb>=0:back.append(s)
            else:
                t=Fraction(ca,ca-cb);m=(pa[0]+(pb[0]-pa[0])*t,pa[1]+(pb[1]-pa[1])*t)
                assert m[0].denominator==1 and m[1].denominator==1,('non-integer split',pa,pb,o,d)
                mid=vertex(m);first=dict(s,b=mid);second=dict(s,a=mid)
                (front if ca<0 else back).append(first);(front if cb<0 else back).append(second)
        assert front and back,('partition does not split',a,b)
        children=[build(front,front_tree),build(back,back_tree)]
        nodes.append(dict(x=o[0],y=o[1],dx=d[0],dy=d[1],children=children,line=index))
        nodebytes+=old.words([o[0],o[1],d[0],d[1],*bbox(front),*bbox(back)])+struct.pack('<HH',*children)
        return len(nodes)-1
    build(segs,tree)
    assert {l['sector']for l in leaves}==set(range(n)),'every sector needs a subsector'
    segbytes=b'';subbytes=b'';first=0
    for l in leaves:
        for s in l['segs']:segbytes+=old.words([s['a'],s['b'],0,s['line'],s['side'],0])
        subbytes+=old.words([len(l['segs']),first]);first+=len(l['segs'])
    rejectbytes=bytearray((n*n+7)//8)
    for a,b in reject:
        for pair in(a*n+b,b*n+a):rejectbytes[pair>>3]|=1<<(pair&7)
    ox=min(x for x,_ in vertices);oy=min(y for _,y in vertices)
    width=(max(x for x,_ in vertices)-ox)//128+1;height=(max(y for _,y in vertices)-oy)//128+1
    cells=width*height;lists=[];offsets=[]
    for cell in range(cells):
        cx=ox+(cell%width)*128;cy=oy+(cell//width)*128
        offsets.append(4+cells+len(lists));lists.append(0)
        for i,l in enumerate(lines):
            if i!=0 and meets_cell(vertices[l['a']],vertices[l['b']],cx,cy):lists.append(i)
        lists.append(-1)
    blocks=[ox,oy,width,height,*offsets,*lists]
    assert max(offsets)<32768 and len(blocks)<=65536 and cells<=4096
    assert len(lines)<=128 and len(vertices)<=256 and len(leaves)<=128 and len(nodes)<=128 and sidecount<=256
    lumps=[('MAP01',b''),('THINGS',b''),('LINEDEFS',linebytes),('SIDEDEFS',sidebytes),('VERTEXES',b''.join(old.words(list(v))for v in vertices)),
      ('SEGS',segbytes),('SSECTORS',subbytes),('NODES',nodebytes),('SECTORS',b''.join(old.words([f,c])+b'TESTFLR0TESTCEIL'+old.words([160,0,0])for f,c in heights)),
      ('REJECT',bytes(rejectbytes)),('BLOCKMAP',old.words(blocks))]
    xs=[x for x,_ in vertices];ys=[y for _,y in vertices]
    return dict(heights=heights,vertices=vertices,lines=lines,nodes=nodes,leaves=[l['sector']for l in leaves],blocks=blocks,bytes=old.wad(lumps),lumps=dict(lumps),
                range=(min(xs),max(xs)),center=((min(xs)+max(xs))//2,(min(ys)+max(ys))//2))
