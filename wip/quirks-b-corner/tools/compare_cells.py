#!/usr/bin/env python3
"""Per-call comparison of the trace walk: the genuine probe log (runs/probe_NAME.log, every
P_PathTraverse call) against the instrumented Rust copy's log (every combat_trace collect and
movement::path call) on wires 22, 23, 24 (player_level_run) and 25 (sound_records_run). Compares,
call by call in order: the nudged start, the cell of every loop iteration (repeats included), the
iteration count, whether the end cell was reached and the number of collected intercepts. Also
requires the instrumented Rust run's digests.txt to equal the pristine Rust run's (wires 22-24).

    compare_cells.py [NAME ...]
"""
import json, os, subprocess, sys
from pathlib import Path

C = Path('/home/copgunn/scratch/quirks/followup-b/corner')
PB = Path('/home/copgunn/scratch/quirks/followup-b/target/probe/release/examples')
POL = {22: 'level', 23: 'episode1', 24: 'episode1v2'}

def genuine(name):
    calls = []
    for line in (C / f'runs/probe_{name}.log').read_text().splitlines():
        w = line.split()
        if w[0] == 'PT':
            i = w.index('start'); calls.append(dict(start=(int(w[i + 1]), int(w[i + 2])), cells=[]))
        elif w[0] == 'C': calls[-1]['cells'].append((int(w[1]), int(w[2])))
        elif w[0] == 'E': calls[-1].update(iters=int(w[2]), reached=int(w[4]), n=int(w[6]))
    return calls

def rust(path):
    calls = []
    if not path.exists(): return calls
    for line in path.read_text().splitlines():
        w = line.split()
        k = w.index('cells')
        calls.append(dict(kind=w[0], start=(int(w[2]), int(w[3])), iters=int(w[w.index('iters') + 1]),
                          reached=int(w[w.index('reached') + 1]), n=int(w[w.index('intercepts') + 1]),
                          cells=[tuple(int(v) for v in c.split(',')) for c in w[k + 1:]]))
    return calls

def main():
    scenes = json.loads((C / 'wads/scenes.json').read_text())
    names = sys.argv[1:] or [n for n, v in scenes.items() if not v['optional']]
    total = dict(runs=0, calls=0, equal_runs=0)
    rows = {}
    for name in names:
        g = genuine(name)
        wad, lmp = C / f'wads/{name}.wad', C / f'wads/{name}.lmp'
        for wire in (22, 23, 24, 25):
            log = C / f'rustprobe/w{wire}_{name}.log'
            log.parent.mkdir(parents=True, exist_ok=True)
            if log.exists(): log.unlink()
            out = C / f'rustprobe/w{wire}_{name}'
            out.mkdir(parents=True, exist_ok=True)
            env = dict(os.environ, CORNER_RUST_LOG=str(log))
            if wire == 25:
                cmd = [str(PB / 'sound_records_run'), '--wad', str(wad), '--map', 'E1M1', '--demo', str(lmp), '--alone']
            else:
                cmd = [str(PB / 'player_level_run'), '--wad', str(wad), '--map', 'E1M1', '--demo', str(lmp), '--out', str(out),
                       '--policy', POL[wire]]
            p = subprocess.run(cmd, capture_output=True, text=True, env=env)
            r = rust(log)
            same_digests = None
            if wire != 25:
                same_digests = (out / 'digests.txt').read_text() == (C / f'rust/w{wire}_{name}/digests.txt').read_text()
            eq = len(r) == len(g) and all(a['start'] == tuple(b['start']) and a['cells'] == b['cells'] and a['iters'] == b['iters']
                                          and a['reached'] == b['reached'] and a['n'] == b['n'] for a, b in zip(r, g))
            first = None
            if not eq:
                for i, (a, b) in enumerate(zip(r, g)):
                    if not (a['start'] == tuple(b['start']) and a['cells'] == b['cells'] and a['iters'] == b['iters']
                            and a['reached'] == b['reached'] and a['n'] == b['n']):
                        first = dict(call=i, rust={k: a[k] for k in ('start', 'iters', 'reached', 'n')},
                                     genuine={k: b[k] for k in ('start', 'iters', 'reached', 'n')}); break
                if first is None: first = dict(calls_rust=len(r), calls_genuine=len(g))
            total['runs'] += 1; total['calls'] += len(g); total['equal_runs'] += eq
            stalled = sum(1 for b in g if b['iters'] == 64 and not b['reached'])
            rows[f'{name}/w{wire}'] = dict(rc=p.returncode, calls=len(g), rust_calls=len(r), kinds=sorted(set(a['kind'] for a in r)),
                                           stalled_calls=stalled, equal=eq, digests_equal_pristine=same_digests, first_difference=first)
            print(f'{name} wire {wire}: rc {p.returncode} genuine calls {len(g)} (stalled {stalled}) rust calls {len(r)} '
                  f'{sorted(set(a["kind"] for a in r))} walk-equal {eq} digests==pristine {same_digests}' + (f' FIRST {first}' if first else ''))
    print('TOTAL', total)
    (C / 'rustprobe/cells_summary.json').write_text(json.dumps(dict(total=total, rows=rows), indent=1))

if __name__ == '__main__':
    main()
