"""Compare the direct-call probes: genuine P_PathTraverse (probe runner, CORNER_DIRECT) against the
scratch copy's movement::path (harness-probe direct), call by call."""
import sys
sys.path.insert(0, '/home/copgunn/scratch/quirks/followup-b/corner/tools')
from compare_cells import rust
from pathlib import Path
D = Path('/home/copgunn/scratch/quirks/followup-b/corner/direct')
labels = []
for l in (D / 'direct_labels.txt').read_text().splitlines():
    w = l.split(' ')
    labels.append(w[0] + (' ' + w[1] if w[1] in ('x+1', 'x-1', 'y+1', 'y-1') else ''))
g = []
for line in (D / 'genuine.log').read_text().splitlines():
    w = line.split()
    if w[0] == 'PT':
        i, j, f, la = w.index('start'), w.index('step'), w.index('first'), w.index('last')
        g.append(dict(start=(int(w[i + 1]), int(w[i + 2])), step=(int(w[j + 1]), int(w[j + 2])), first=(int(w[f + 1]), int(w[f + 2])),
                      last=(int(w[la + 1]), int(w[la + 2])), yint=int(w[w.index('yint') + 1]), xint=int(w[w.index('xint') + 1]),
                      ystep=int(w[w.index('ystep') + 1]), xstep=int(w[w.index('xstep') + 1]), cells=[], hits=[]))
    elif w[0] == 'C': g[-1]['cells'].append((int(w[1]), int(w[2])))
    elif w[0] == 'E': g[-1].update(iters=int(w[2]), reached=int(w[4]), n=int(w[6]))
    elif w[0] == 'I': g[-1]['hits'].append((int(w[1]), int(w[3])))
r = rust(D / 'rust.log')
rh = [[tuple(int(x) for x in t.split(':')) for t in l.split()[1:]] for l in (D / 'rust_hits.txt').read_text().splitlines()]
assert len(g) == len(r) == len(rh) == len(labels), (len(g), len(r), len(rh), len(labels))

def distinct(cs):
    o = []
    for c in cs:
        if not o or o[-1] != c: o.append(c)
    return o

eq_all = 0
for lab, a, b, h in zip(labels, g, r, rh):
    eq = (a['start'] == b['start'] and a['cells'] == b['cells'] and a['iters'] == b['iters'] and a['reached'] == b['reached']
          and a['n'] == b['n'] and sorted(a['hits']) == sorted(h))
    eq_all += eq
    print(f"{lab:10s} start {a['start'][0] / 65536:.6f},{a['start'][1] / 65536:.6f} step {a['step']} ystep {a['ystep']} xstep {a['xstep']} "
          f"yint0 {a['yint']} xint0 {a['xint']} first {a['first']} last {a['last']} iters {a['iters']:2d} reached {a['reached']} "
          f"stalled {int(a['iters'] == 64 and not a['reached'])} cells {distinct(a['cells'])} | rust equal {eq}")
print(f'{eq_all}/{len(g)} calls equal (start, every iteration cell, iterations, reached, intercept count, hits)')
