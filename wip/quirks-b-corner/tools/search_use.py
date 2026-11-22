import pt, collections
res = collections.defaultdict(list)
for ang in range(8):
    fine = ang * 1024
    for ox in range(0, 128):
        for oy in range(0, 128):
            x, y = (512 + ox) << 16, (512 + oy) << 16
            ex, ey = pt.hitscan_end(x, y, fine, 64)
            r = pt.traverse(x, y, ex, ey, 0, 0)
            if 'none' in r['branches']:
                k = r['branches'].index('none')
                res[ang].append((ox, oy, k, r['cells'][k], pt.distinct(r['cells']), r['last']))
for ang in range(8):
    print(ang*45, len(res[ang]), res[ang][:4])
