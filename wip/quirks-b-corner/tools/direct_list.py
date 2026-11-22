"""The direct-call probe list: exact corner hits per direction class, near misses of one fixed
unit, and starts on block lines (nudge). Writes LABEL x1 y1 x2 y2 (fixed) to direct_labels.txt and
'x1 y1 x2 y2 flags' to direct_calls.txt (flags 1 = PT_ADDLINES, the slide/use walk)."""
import sys
U = 65536
P = []
def add(label, a, b, c, d): P.append((label, a, b, c, d))
ex = {  # exact slope-1 and slope-2 traces through the block corner (640, 640) from non-aligned starts
 'PP1': (600, 600, 1000, 1000), 'PM1': (600, 680, 1000, 280), 'MP1': (680, 600, 280, 1000), 'MM1': (680, 680, 280, 280),
 'PP2': (620, 600, 820, 1000), 'PM2': (620, 680, 820, 280), 'MP2': (660, 600, 460, 1000), 'MM2': (660, 680, 460, 280),
}
for k, (a, b, c, d) in ex.items():
    add(k, a * U, b * U, c * U, d * U)
    for dx, dy, tag in ((1, 0, 'x+1'), (-1, 0, 'x-1'), (0, 1, 'y+1'), (0, -1, 'y-1')):
        add(f'{k} {tag}', a * U + dx, b * U + dy, c * U, d * U)
# nudge: starts exactly on a block corner / a block line
for k, (a, b, c, d) in {'NPP': (640, 640, 1040, 1040), 'NPM': (640, 640, 1040, 240), 'NMP': (640, 640, 240, 1040),
                        'NMM': (640, 640, 240, 240), 'NVX': (640, 600, 1040, 1000), 'NHY': (600, 640, 1000, 1040)}.items():
    add(k, a * U, b * U, c * U, d * U)
out = sys.argv[1]
with open(f'{out}/direct_labels.txt', 'w') as f:
    for p in P: f.write(' '.join(map(str, p)).replace(' ', '\t', 0) + '\n')
with open(f'{out}/direct_calls.txt', 'w') as f:
    for _, a, b, c, d in P: f.write(f'{a} {b} {c} {d} 1\n')
with open(f'{out}/direct_rust.txt', 'w') as f:
    for _, a, b, c, d in P: f.write(f'{a} {b} {c} {d}\n')
print(len(P), 'probe calls')
