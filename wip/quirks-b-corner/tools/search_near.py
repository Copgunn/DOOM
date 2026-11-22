import pt, math
found = []
for a in range(256):
    fine = a * 32
    for ox in range(128):
        for oy in range(128):
            X, Y = (1792 + ox) << 16, (1792 + oy) << 16
            ex, ey = pt.hitscan_end(X, Y, fine, 1024)
            r = pt.traverse(X, Y, ex, ey, 0, 0)
            if 'none' in r['branches']:
                k = r['branches'].index('none')
                c = r['cells'][k]
                d = math.hypot(c[0]*128+64 - (1792+ox), c[1]*128+64 - (1792+oy))
                found.append((round(d), fine, 1792+ox, 1792+oy, k, c))
found.sort()
print(len(found))
for f in found[:30]: print(f)
