import find_scenes as fs, pt
# candidates: fine near 45-degree thing angles reachable with one turn byte (b in -16..15)
best = []
for base in (1024, 3072, 5120, 7168):
    for b in range(-12, 13):
        fine = base + 32 * b
        sc = fs.scene(fine, (1792, 1792, 1920, 1920), units=1024, margin=12, pillar=0)
        for c in sc:
            if 300 <= c['dist'] <= 900:
                best.append((c['dist'], fine, b, base, c))
best.sort(key=lambda t: t[0])
print(len(best))
for d, fine, b, base, c in best[:12]:
    print(fine, b, base, {k: v for k, v in c.items() if k != 'cells'})
