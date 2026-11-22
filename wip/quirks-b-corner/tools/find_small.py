import find_scenes as fs
ok = lambda c: all(96 <= v <= 1440 for v in c['pillar'])
res = []
for b in range(-15, 16):
    fine = 1024 + 32 * b
    for c in fs.scene(fine, (256, 256, 384, 384), pillar=8, margin=4):
        if ok(c) and c['dist'] < 1200: res.append((c['dist'], b, fine, c))
res.sort(key=lambda t: t[0])
print(len(res))
for d, b, fine, c in res[:8]:
    print(b, fine, {k: v for k, v in c.items() if k != 'cells'}, 'ctrl', fs.control(fine, c['start'], c['pillar_cell']))
