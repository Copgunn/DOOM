import pt, collections
found = []
for a in range(256):
    fine = a * 32
    for ox in range(0, 128):
        for oy in range(0, 128):
            x, y = (512 + ox) << 16, (512 + oy) << 16
            ex, ey = pt.hitscan_end(x, y, fine, 64)
            r = pt.traverse(x, y, ex, ey, 0, 0)
            if 'none' in r['branches']:
                k = r['branches'].index('none')
                found.append((fine, ox, oy, k, r['cells'][k], pt.distinct(r['cells']), r['last']))
print(len(found))
for f in found[:40]: print(f)
