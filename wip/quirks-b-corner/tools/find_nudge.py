import find_scenes as fs, pt
res = []
for a in range(256):
    fine = a * 32
    # starts on the vertical block line x=1792 and on the horizontal line y=1792, inside cell rows/cols 14
    pts = [(1792, 1792 + k) for k in range(128)] + [(1792 + k, 1792) for k in range(1, 128)]
    for (sx, sy) in pts:
        X, Y = sx << 16, sy << 16
        ex, ey = pt.hitscan_end(X, Y, fine, 2048)
        r = pt.traverse(X, Y, ex, ey, 0, 0)
        if 'none' not in r['branches']: continue
        # would it stall without the nudge? compare replica with the nudge disabled
        res.append((fine, sx, sy, r['branches'].index('none')))
print(len(res))
import collections
print(collections.Counter(f for f, *_ in res).most_common(10))
good = []
for fine, sx, sy, k in res:
    sc = fs.scene(fine, (sx, sy, sx + 1, sy + 1), pillar=8, margin=4)
    for c in sc:
        if c['dist'] < 2000: good.append((fine, c))
print(len(good))
for fine, c in good[:10]: print(fine, {k: v for k, v in c.items() if k != 'cells'})
