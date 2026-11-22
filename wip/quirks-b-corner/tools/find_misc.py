import find_scenes as fs, pt
# nudge: block-corner starts, cardinal thing angles
for fine in (0, 2048, 4096, 6144):
    for (sx, sy) in ((2560, 1536), (1024, 1536), (1536, 2560), (1536, 1024)):
        sc = fs.scene(fine, (sx, sy, sx + 1, sy + 1), pillar=8, margin=4)
        for c in sc: print('nudge', fine, {k: v for k, v in c.items() if k != 'cells'}, c['cells'], 'ctrl', fs.control(fine, c['start'], c['pillar_cell']))
# autoaim control
print('aim ctrl', fs.control(1248, (1915, 1859), (16, 16), units=1024))
# use
print(fs.scene(1248, (621, 613, 622, 614), units=64, pillar=0, margin=0))
print('use ctrl', fs.control(1248, (621, 613), (5, 5), units=64))
