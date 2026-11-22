"""Find whole-engine scene candidates with the replica (search aid only; evidence = genuine runs).

For a thing angle (and optional demo turn, in fine units of 32), find an integer player start
inside the room box whose trace (range units) stalls in P_PathTraverse, and the first cell of
the exact geometric line that the stalled walk never visits; propose an 8x8 void pillar there.
Also find a control start 1 unit away whose walk does not stall and visits the pillar's cell."""
import sys
from fractions import Fraction as F
import pt

def geo_cells(x1, y1, x2, y2, org=(0, 0)):
    """Cells of the exact segment, in order (rational arithmetic), from fixed endpoints."""
    X1, Y1, X2, Y2 = (F(v, 65536) for v in (x1, y1, x2, y2))
    pts = [F(0), F(1)]
    for (a, b, o) in ((X1, X2, org[0]), (Y1, Y2, org[1])):
        if a == b: continue
        lo, hi = sorted((a, b))
        k = (lo - o) // 128 + 1
        while o + k * 128 <= hi:
            pts.append((o + k * 128 - a) / (b - a)); k += 1
    pts = sorted(set(p for p in pts if 0 <= p <= 1))
    cells = []
    for s, t in zip(pts, pts[1:]):
        m = (s + t) / 2
        x = X1 + (X2 - X1) * m; y = Y1 + (Y2 - Y1) * m
        c = (int((x - org[0]) // 128), int((y - org[1]) // 128))
        if not cells or cells[-1] != c: cells.append(c)
    return cells

def point_at(x1, y1, x2, y2, t):
    return (F(x1, 65536) + (F(x2, 65536) - F(x1, 65536)) * t, F(y1, 65536) + (F(y2, 65536) - F(y1, 65536)) * t)

def scene(fine, box, units=2048, margin=2, want_dist=None, org=(0, 0), pillar=8):
    x0, y0, x1b, y1b = box
    out = []
    for sx in range(x0, x1b):
        for sy in range(y0, y1b):
            X, Y = sx << 16, sy << 16
            ex, ey = pt.hitscan_end(X, Y, fine, units)
            r = pt.traverse(X, Y, ex, ey, *org)
            if 'none' not in r['branches']: continue
            k = r['branches'].index('none')
            visited = set(r['cells'])
            g = geo_cells(r['start'][0], r['start'][1], ex, ey, org)
            missing = [c for c in g if c not in visited]
            if not missing: continue
            c = missing[0]
            # trace point at the middle of the geometric segment inside c
            # sample t finely to find the param range inside c
            n = 4096
            ts = [F(i, n) for i in range(n + 1)]
            inside = [t for t in ts if (lambda p: (int((p[0] - org[0]) // 128), int((p[1] - org[1]) // 128)) == c)(point_at(r['start'][0], r['start'][1], ex, ey, t))]
            if not inside: continue
            tm = inside[len(inside) // 2]
            px, py = point_at(r['start'][0], r['start'][1], ex, ey, tm)
            px, py = int(px), int(py)
            h = pillar // 2
            bx0, by0 = c[0] * 128 + org[0], c[1] * 128 + org[1]
            if not (bx0 + margin <= px - h and px + h <= bx0 + 128 - margin and by0 + margin <= py - h and py + h <= by0 + 128 - margin):
                continue
            dist = float(tm) * units
            if want_dist and dist > want_dist: continue
            out.append(dict(start=(sx, sy), stall_iter=k, stall_cell=r['cells'][k], pillar_cell=c,
                            pillar=(px - h, py - h, px + h, py + h), dist=round(dist, 1), cells=pt.distinct(r['cells'])))
    return out

def control(fine, s, pill, units=2048, org=(0, 0)):
    """Starts 1 unit away (8-neighbourhood) that do not stall and visit the pillar cell."""
    res = []
    for dx in (-1, 0, 1):
        for dy in (-1, 0, 1):
            if dx == dy == 0: continue
            X, Y = (s[0] + dx) << 16, (s[1] + dy) << 16
            ex, ey = pt.hitscan_end(X, Y, fine, units)
            r = pt.traverse(X, Y, ex, ey, *org)
            if 'none' in r['branches']: continue
            if pill in r['cells']: res.append((s[0] + dx, s[1] + dy))
    return res
