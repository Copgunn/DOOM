import find_scenes as fs, sys
cls = {'pp': (1024, (512, 512, 640, 640)), 'mp': (3072, (2944, 512, 3072, 640)),
       'mm': (5120, (2944, 2944, 3072, 3072)), 'pm': (7168, (512, 2944, 640, 3072))}
for name, (fine, box) in cls.items():
    s = fs.scene(fine, box)
    print(name, fine, len(s))
    for c in s[:3]:
        print('  ', {k: v for k, v in c.items() if k != 'cells'}, 'ctrl', fs.control(fine, c['start'], c['pillar_cell']))
