import pt, sys
from fractions import Fraction as F
def detail(sx, sy, fine, org=(0,0), units=2048):
    x, y = sx << 16, sy << 16
    ex, ey = pt.hitscan_end(x, y, fine, units)
    r = pt.traverse(x, y, ex, ey, *org)
    print('start', sx, sy, 'fine', fine, 'end', ex/65536, ey/65536, 'first', r['first'], 'last', r['last'], 'steps', r['steps'], 'ystep', r['ystep'], 'xstep', r['xstep'])
    print(' cells', pt.distinct(r['cells']), 'iters', len(r['cells']), 'reached', r['reached'])
    if 'none' in r['branches']:
        k = r['branches'].index('none'); cx, cy = r['cells'][k]
        # geometric position at the cell's exit boundaries
        tx, ty = r['start']; dx, dy = r['delta']
        bx = (cx + (1 if r['steps'][0] > 0 else 0)) * 128 + org[0]
        by = (cy + (1 if r['steps'][1] > 0 else 0)) * 128 + org[1]
        yat = F(ty, 65536) + (F(bx) - F(tx, 65536)) * F(dy, dx)
        xat = F(tx, 65536) + (F(by) - F(ty, 65536)) * F(dx, dy)
        print(' stall at iter', k, 'cell', (cx, cy), 'corner', (bx, by), 'geometric y at x=%d: %.5f  x at y=%d: %.5f' % (bx, float(yat), by, float(xat)))
    return r
if __name__ == '__main__':
    a = list(map(int, sys.argv[1:]))
    detail(*a)
