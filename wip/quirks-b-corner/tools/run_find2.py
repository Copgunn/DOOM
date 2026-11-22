import find_scenes as fs, pt
ok = lambda c: all(64 <= v <= 2240 for v in c['pillar'])
cls = {'pp': (1024, (512, 512, 640, 640)), 'mp': (3072, (1664, 512, 1792, 640)),
       'mm': (5120, (1664, 1664, 1792, 1792)), 'pm': (7168, (512, 1664, 640, 1792))}
for name, (fine, box) in cls.items():
    s = [c for c in fs.scene(fine, box) if ok(c)]
    print(name, fine, len(s))
    for c in s[:2]:
        print('  ', {k: v for k, v in c.items() if k != 'cells'}, 'ctrl', fs.control(fine, c['start'], c['pillar_cell']))
# nudge: starts on block lines x=512 or y=512, any fine multiple of 32 within +-15 turn bytes of a thing angle
res = []
for base in range(0, 8192, 1024):
    for b in range(-15, 16):
        fine = (base + 32 * b) % 8192
        pts = [(512, 512 + k) for k in range(128)] + [(512 + k, 512) for k in range(1, 128)]
        for (sx, sy) in pts:
            X, Y = sx << 16, sy << 16
            ex, ey = pt.hitscan_end(X, Y, fine, 2048)
            r = pt.traverse(X, Y, ex, ey, 0, 0)
            if 'none' not in r['branches']: continue
            for c in fs.scene(fine, (sx, sy, sx + 1, sy + 1), pillar=8, margin=4):
                if ok(c): res.append((base, b, fine, c))
print('nudge', len(res))
for base, b, fine, c in res[:6]:
    print('  ', base // 1024 * 45, b, fine, {k: v for k, v in c.items() if k != 'cells'}, 'ctrl', fs.control(fine, c['start'], c['pillar_cell']))
