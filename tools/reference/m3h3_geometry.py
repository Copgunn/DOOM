"""Walled-rectangle rooms for the M3h3 observer.

One flat sector (floor 0, ceiling 128), one-sided axis-aligned ML_BLOCKING walls
with the room on their right side, optional interior pillars, walls split into
pieces of at most `segment` units, no BSP nodes (one subsector) and a cell-exact
BLOCKMAP whose origin is the rectangle's minimum corner. The same construction
exists as the Rust test-only `Room` builder; both are synthetic public inputs and
neither reads the other's output."""
import generate_m3b as old

def tdiv(a,b):
    """Truncating division, so Python and the Rust builder split walls identically."""
    return -((-a)//b) if a<0 else a//b
def walls(rect,pillars,segment):
    (x0,y0,x1,y1)=rect;out=[]
    def push(a,b):
        length=abs(b[0]-a[0])+abs(b[1]-a[1]);pieces=max(1,-(-length//segment))
        at=lambda p,q,k:p+tdiv((q-p)*k,pieces)
        for k in range(pieces):out.append((at(a[0],b[0],k),at(a[1],b[1],k),at(a[0],b[0],k+1),at(a[1],b[1],k+1)))
    # Clockwise perimeter keeps the room on each wall's right (front) side.
    push((x0,y0),(x0,y1));push((x0,y1),(x1,y1));push((x1,y1),(x1,y0));push((x1,y0),(x0,y0))
    for (px0,py0,px1,py1) in pillars:
        # Counterclockwise pillar keeps the room outside, on the right.
        push((px0,py0),(px1,py0));push((px1,py0),(px1,py1));push((px1,py1),(px0,py1));push((px0,py1),(px0,py0))
    return out
def room(rect,pillars=(),segment=1024):
    (x0,y0,x1,y1)=rect;segments=walls(rect,pillars,segment)
    vertices=[]
    def vertex(p):
        if p not in vertices:vertices.append(p)
        return vertices.index(p)
    lines=[];linebytes=b'';sidebytes=b'';segbytes=b''
    for i,w in enumerate(segments):
        a,b=vertex(w[:2]),vertex(w[2:])
        lines.append(dict(a=a,b=b,front=0,back=None,flags=1,sides=[i,-1]))
        linebytes+=old.words([a,b,1,0,0,i,-1])
        sidebytes+=old.words([0,0])+old.name('-')*2+old.name('TESTWALL')+old.words([0])
        segbytes+=old.words([a,b,0,i,0,0])
    ox,oy=x0,y0;width=(x1-ox)//128+1;height=(y1-oy)//128+1
    cells=width*height;lists=[];offsets=[]
    for cell in range(cells):
        cx=ox+(cell%width)*128;cy=oy+(cell//width)*128
        offsets.append(4+cells+len(lists));lists.append(0)
        for i,w in enumerate(segments):
            lx0,lx1=min(w[0],w[2]),max(w[0],w[2]);ly0,ly1=min(w[1],w[3]),max(w[1],w[3])
            if lx1>=cx and lx0<=cx+128 and ly1>=cy and ly0<=cy+128 and i!=0:lists.append(i)
        lists.append(-1)
    blocks=[ox,oy,width,height,*offsets,*lists]
    assert max(offsets)<32768 and len(blocks)<=65536
    lumps=[('MAP01',b''),('THINGS',b''),('LINEDEFS',linebytes),('SIDEDEFS',sidebytes),('VERTEXES',b''.join(old.words(list(v))for v in vertices)),
      ('SEGS',segbytes),('SSECTORS',old.words([len(lines),0])),('NODES',b''),('SECTORS',old.words([0,128])+b'TESTFLR0TESTCEIL'+old.words([160,0,0])),
      ('REJECT',bytes(1)),('BLOCKMAP',old.words(blocks))]
    return dict(heights=[(0,128)],vertices=vertices,lines=lines,nodes=[],blocks=blocks,bytes=old.wad(lumps),lumps=dict(lumps),range=(x0,x1),center=((x0+x1)//2,(y0+y1)//2))
