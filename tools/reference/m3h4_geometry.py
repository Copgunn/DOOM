"""Walled rooms of axis-aligned sectors with static heights for the M3h4 observer.

A room is a strip of rectangular sectors tiled along x inside a walled rectangle:
one-sided axis-aligned ML_BLOCKING boundary walls with the room on their right side,
split into pieces of at most `segment` units; one two-sided linedef between each pair
of adjacent sectors carrying ML_TWOSIDED plus the caller's extra flags; one subsector
per sector whose segs follow the source side convention (front side keeps the line
direction, back side reverses it); a guillotine BSP over the vertical boundaries whose
root is the last node; a REJECT table with optional set pairs; and a cell-exact
BLOCKMAP whose origin is the rectangle's minimum corner. The same construction exists
as the Rust test-only `HeightsRoom` builder; both are synthetic public inputs and
neither reads the other's output."""
import struct
import generate_m3b as old
from m3h3_geometry import tdiv
TWOSIDED=4
def split(a,b,segment):
    length=abs(b[0]-a[0])+abs(b[1]-a[1]);pieces=max(1,-(-length//segment))
    at=lambda p,q,k:p+tdiv((q-p)*k,pieces)
    return[((at(a[0],b[0],k),at(a[1],b[1],k)),(at(a[0],b[0],k+1),at(a[1],b[1],k+1)))for k in range(pieces)]
def room(spans,heights,y=(-512,512),portals=None,reject=(),segment=1024):
    """`spans` lists the n+1 x boundaries of n sectors; `heights` their (floor, ceiling);
    `portals` maps an interior boundary index 1..n-1 to extra linedef flags."""
    n=len(heights);assert len(spans)==n+1 and all(spans[i]<spans[i+1]for i in range(n))
    portals=dict(portals or{});y0,y1=y
    vertices=[]
    def vertex(p):
        if p not in vertices:vertices.append(p)
        return vertices.index(p)
    lines=[]
    def add(a,b,front,back,flags):
        for p,q in split(a,b,segment):lines.append(dict(a=vertex(p),b=vertex(q),front=front,back=back,flags=flags))
    # Clockwise perimeter keeps each sector on its walls' right (front) side.
    add((spans[0],y0),(spans[0],y1),0,None,1)
    for i in range(n):add((spans[i],y1),(spans[i+1],y1),i,None,1)
    add((spans[n],y1),(spans[n],y0),n-1,None,1)
    for i in reversed(range(n)):add((spans[i+1],y0),(spans[i],y0),i,None,1)
    # Interior boundaries run +y, so their right (front) side is the eastern sector.
    for k in range(1,n):add((spans[k],y0),(spans[k],y1),k,k-1,TWOSIDED|portals.get(k,0))
    linebytes=b'';sidebytes=b'';sidecount=0
    for l in lines:
        indexes=[]
        for sector in[l['front'],l['back']]:
            if sector is None:indexes.append(-1);continue
            indexes.append(sidecount);sidecount+=1
            sidebytes+=old.words([0,0])+old.name('-')*2+old.name('TESTWALL')+old.words([sector])
        l['sides']=indexes;linebytes+=old.words([l['a'],l['b'],l['flags'],0,0,*indexes])
    segbytes=b'';subbytes=b'';first=0
    for sector in range(n):
        count=0
        for i,l in enumerate(lines):
            if l['front']==sector:side=0;a,b=l['a'],l['b']
            elif l['back']==sector:side=1;a,b=l['b'],l['a']
            else:continue
            segbytes+=old.words([a,b,0,i,side,0]);count+=1
        subbytes+=old.words([count,first]);first+=count
    nodes=[];nodebytes=b''
    def build(lo,hi):
        nonlocal nodebytes
        if lo==hi:return 0x8000|lo
        mid=(lo+hi+1)//2;left=build(lo,mid-1);right=build(mid,hi);x=spans[mid]
        boxes=[[y1,y0,x,spans[hi+1]],[y1,y0,spans[lo],x]]
        nodes.append(dict(x=x,y=y0,dx=0,dy=y1-y0,children=[right,left]))
        nodebytes+=old.words([x,y0,0,y1-y0,*boxes[0],*boxes[1]])+struct.pack('<HH',right,left)
        return len(nodes)-1
    build(0,n-1)
    rejectbytes=bytearray((n*n+7)//8)
    for a,b in reject:
        for pair in(a*n+b,b*n+a):rejectbytes[pair>>3]|=1<<(pair&7)
    ox,oy=spans[0],y0;width=(spans[n]-ox)//128+1;height=(y1-oy)//128+1
    cells=width*height;lists=[];offsets=[]
    for cell in range(cells):
        cx=ox+(cell%width)*128;cy=oy+(cell//width)*128
        offsets.append(4+cells+len(lists));lists.append(0)
        for i,l in enumerate(lines):
            (ax,ay),(bx,by)=vertices[l['a']],vertices[l['b']]
            if max(ax,bx)>=cx and min(ax,bx)<=cx+128 and max(ay,by)>=cy and min(ay,by)<=cy+128 and i!=0:lists.append(i)
        lists.append(-1)
    blocks=[ox,oy,width,height,*offsets,*lists]
    assert max(offsets)<32768 and len(blocks)<=65536
    lumps=[('MAP01',b''),('THINGS',b''),('LINEDEFS',linebytes),('SIDEDEFS',sidebytes),('VERTEXES',b''.join(old.words(list(v))for v in vertices)),
      ('SEGS',segbytes),('SSECTORS',subbytes),('NODES',nodebytes),('SECTORS',b''.join(old.words([f,c])+b'TESTFLR0TESTCEIL'+old.words([160,0,0])for f,c in heights)),
      ('REJECT',bytes(rejectbytes)),('BLOCKMAP',old.words(blocks))]
    return dict(heights=list(heights),vertices=vertices,lines=lines,nodes=nodes,blocks=blocks,bytes=old.wad(lumps),lumps=dict(lumps),range=(spans[0],spans[n]),center=((spans[0]+spans[n])//2,(y0+y1)//2))
