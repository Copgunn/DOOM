#!/usr/bin/env python3
"""Run every scene through the unmodified strict genuine runner and the probe runner, one job at
a time; require byte-identical dump sets; summarise the probe log per P_PathTraverse call.

    run_scenes.py [NAME ...]        (default: every non-optional scene in wads/scenes.json)
"""
import filecmp, json, os, subprocess, sys
from pathlib import Path

C = Path('/home/copgunn/scratch/quirks/followup-b/corner')
STRICT, PROBE = C / 'build/strict/crunner', C / 'build/probe/crunner'

def run(binary, name, out, env=None):
    out.mkdir(parents=True, exist_ok=True)
    for f in out.glob('*'): f.unlink()
    e = dict(os.environ); e.update(env or {})
    p = subprocess.run([str(binary), '--wad', str(C / f'wads/{name}.wad'), '--wad', str(C / f'wads/{name}.lmp'),
                        '--demo', name, '--out', str(out)], capture_output=True, text=True, env=e, timeout=600)
    (out.parent / f'{out.name}.stdout').write_text(p.stdout + p.stderr)
    return p.returncode, p.stdout.strip().splitlines()[-1] if p.stdout.strip() else p.stderr.strip()

def parse_log(path):
    calls, cur = [], None
    for line in Path(path).read_text().splitlines():
        w = line.split()
        if w[0] == 'PT':
            kv = {}
            i = 1
            while i < len(w):
                k = w[i]; n = {'args': 4, 'start': 2, 'first': 2, 'last': 2, 'step': 2}.get(k, 1)
                kv[k] = [int(v) for v in w[i + 1:i + 1 + n]]; i += 1 + n
            cur = dict(kv, cells=[], intercepts=[]); calls.append(cur)
        elif w[0] == 'C': cur['cells'].append((int(w[1]), int(w[2])))
        elif w[0] == 'E': cur.update(iters=int(w[2]), reached=int(w[4]), n=int(w[6]))
        elif w[0] == 'I': cur['intercepts'].append((int(w[1]), int(w[2]), int(w[3])))
    return calls

def distinct(cells):
    out = []
    for c in cells:
        if not out or out[-1] != c: out.append(c)
    return out

def main():
    scenes = json.loads((C / 'wads/scenes.json').read_text())
    names = sys.argv[1:] or [n for n, v in scenes.items() if not v['optional']]
    summary = {}
    for name in names:
        rs, ls = run(STRICT, name, C / f'runs/strict_{name}')
        rp, lp = run(PROBE, name, C / f'runs/probe_{name}', {'CORNER_PROBE_LOG': str(C / f'runs/probe_{name}.log')})
        a, b = C / f'runs/strict_{name}', C / f'runs/probe_{name}'
        fa, fb = sorted(p.name for p in a.iterdir()), sorted(p.name for p in b.iterdir())
        same = fa == fb and all(filecmp.cmp(a / f, b / f, shallow=False) for f in fa)
        calls = parse_log(C / f'runs/probe_{name}.log')
        rows = []
        for k in calls:
            stalled = k['iters'] == 64 and not k['reached']
            rows.append(dict(tic=k['tic'][0], flags=k['flags'][0], args=k['args'], start=k['start'], first=k['first'],
                             last=k['last'], step=k['step'], iters=k['iters'], reached=k['reached'], stalled=stalled,
                             cells=distinct(k['cells']), intercepts=k['n'],
                             items=sorted(set((i[1], i[2]) for i in k['intercepts']))))
        summary[name] = dict(strict_rc=rs, strict_last=ls, probe_rc=rp, probe_last=lp, dumps=len(fa), identical=same, calls=rows)
        print(f'{name}: strict rc {rs} [{ls}] probe rc {rp}; dumps {len(fa)} identical {same}; P_PathTraverse calls {len(rows)}')
        for r in rows:
            print(f"   tic {r['tic']} flags {r['flags']} start {r['start']} first {r['first']} last {r['last']} step {r['step']} "
                  f"iters {r['iters']} reached {r['reached']} stalled {r['stalled']} intercepts {r['intercepts']} items {r['items']}")
            print(f"      cells {r['cells']}")
    old = {}
    sp = C / 'runs/genuine_summary.json'
    if sp.exists(): old = json.loads(sp.read_text())
    old.update(summary)
    sp.write_text(json.dumps(old, indent=1))

if __name__ == '__main__':
    main()
