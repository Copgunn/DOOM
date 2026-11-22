"""Replica of linuxdoom-1.10 P_PathTraverse's block stepping (p_maputl.c:743-877), for
SEARCHING candidate scenes only. Evidence comes from the genuine engine runs, not from this.
Fixed-point helpers follow m_fixed.c:44-87 (FixedMul: 64-bit product >> 16; FixedDiv: the
abs(a)>>14 >= abs(b) saturation, then the double quotient truncated toward zero)."""
import re
from pathlib import Path

SRC = Path('/home/copgunn/src/DOOM/linuxdoom-1.10')
_t = SRC.joinpath('tables.c').read_text(encoding='latin-1')
_body = _t[_t.index('int finesine[10240] ='):]
_body = _body[_body.index('{') + 1:_body.index('};')]
FINESINE = [int(v) for v in re.findall(r'-?\d+', _body)]
assert len(FINESINE) == 10240, len(FINESINE)
def finecos(i): return FINESINE[i + 2048]

FRACBITS, FRACUNIT = 16, 1 << 16
MAPBLOCKSIZE = 128 << 16
MAPBLOCKSHIFT = 23
MAPBTOFRAC = 7
MAXINT, MININT = 0x7fffffff, -0x80000000

def i32(v):
    v &= 0xffffffff
    return v - (1 << 32) if v & 0x80000000 else v

def fmul(a, b): return i32((a * b) >> 16)

def fdiv(a, b):
    if (abs(a) >> 14) >= abs(b):
        return MININT if (a ^ b) < 0 else MAXINT
    c = a / b * 65536.0
    assert -2147483648.0 <= c < 2147483648.0
    return int(c)

def traverse(x1, y1, x2, y2, orgx, orgy):
    """Returns dict: nudged start, cells visited in order (one per loop iteration, up to 64),
    whether the end cell was reached, the per-iteration branch taken ('x', 'y', 'none', 'end')."""
    orgx <<= 16; orgy <<= 16
    if ((x1 - orgx) & (MAPBLOCKSIZE - 1)) == 0: x1 += FRACUNIT
    if ((y1 - orgy) & (MAPBLOCKSIZE - 1)) == 0: y1 += FRACUNIT
    tx, ty, tdx, tdy = x1, y1, i32(x2 - x1), i32(y2 - y1)
    x1 -= orgx; y1 -= orgy
    xt1, yt1 = x1 >> MAPBLOCKSHIFT, y1 >> MAPBLOCKSHIFT
    x2 -= orgx; y2 -= orgy
    xt2, yt2 = x2 >> MAPBLOCKSHIFT, y2 >> MAPBLOCKSHIFT
    if xt2 > xt1:
        mapxstep = 1; partial = FRACUNIT - ((x1 >> MAPBTOFRAC) & (FRACUNIT - 1)); ystep = fdiv(y2 - y1, abs(x2 - x1))
    elif xt2 < xt1:
        mapxstep = -1; partial = (x1 >> MAPBTOFRAC) & (FRACUNIT - 1); ystep = fdiv(y2 - y1, abs(x2 - x1))
    else:
        mapxstep = 0; partial = FRACUNIT; ystep = 256 * FRACUNIT
    yintercept = (y1 >> MAPBTOFRAC) + fmul(partial, ystep)
    if yt2 > yt1:
        mapystep = 1; partial = FRACUNIT - ((y1 >> MAPBTOFRAC) & (FRACUNIT - 1)); xstep = fdiv(x2 - x1, abs(y2 - y1))
    elif yt2 < yt1:
        mapystep = -1; partial = (y1 >> MAPBTOFRAC) & (FRACUNIT - 1); xstep = fdiv(x2 - x1, abs(y2 - y1))
    else:
        mapystep = 0; partial = FRACUNIT; xstep = 256 * FRACUNIT
    xintercept = (x1 >> MAPBTOFRAC) + fmul(partial, xstep)
    mapx, mapy = xt1, yt1
    cells, branches, reached = [], [], False
    for count in range(64):
        cells.append((mapx, mapy))
        if mapx == xt2 and mapy == yt2:
            reached = True; branches.append('end'); break
        if (yintercept >> 16) == mapy:
            yintercept += ystep; mapx += mapxstep; branches.append('x')
        elif (xintercept >> 16) == mapx:
            xintercept += xstep; mapy += mapystep; branches.append('y')
        else:
            branches.append('none')
    return dict(start=(tx, ty), delta=(tdx, tdy), first=(xt1, yt1), last=(xt2, yt2), cells=cells,
                branches=branches, reached=reached, steps=(mapxstep, mapystep), ystep=ystep, xstep=xstep)

def distinct(cells):
    out = []
    for c in cells:
        if not out or out[-1] != c: out.append(c)
    return out

def hitscan_end(x, y, fine, units=2048):
    return x + units * finecos(fine), y + units * FINESINE[fine]
