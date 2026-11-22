#!/usr/bin/env python3
"""Older wires 7..21 on the corner scenes (harness `wires`, public API): per tic the gameplay RNG
index and the actor count against the genuine strict dumps (H.prndindex and the number of M
rows), plus, with the instrumented copy's harness, the trace walk of every call against the
genuine probe log (start, every iteration cell, iterations, reached, intercepts).

    run_old_wires.py SCENE[:WAD] ... (default: every scene with a 45-degree based start)
"""
import json, os, subprocess, sys
from pathlib import Path
sys.path.insert(0, '/home/copgunn/scratch/quirks/followup-b/corner/tools')
from compare_cells import genuine, rust

C = Path('/home/copgunn/scratch/quirks/followup-b/corner')
PRISTINE = Path('/home/copgunn/scratch/quirks/followup-b/target/harness/release/wires')
PROBE = Path('/home/copgunn/scratch/quirks/followup-b/target/probe-harness/release/wires')

def genuine_tics(name):
    d = C / f'runs/strict_{name}'
    out = []
    for f in sorted(d.glob('tic_*.txt')):
        rng, mobjs, pos = None, 0, []
        for line in f.read_text().splitlines():
            w = line.split(',')
            if w[0] == 'H': rng = int(w[4]) & 0xff
            elif w[0] in ('M', 'RR'): mobjs += 1; pos.append((int(w[4]), int(w[5])))  # RR: a pending removal, still listed
        out.append((int(f.stem[4:]) + 1, rng, mobjs, tuple(sorted(pos))))
    return out

def main():
    scenes = json.loads((C / 'wads/scenes.json').read_text())
    names = sys.argv[1:] or [n for n, v in scenes.items() if not v['optional']]
    rows = {}
    for name in names:
        sc = scenes[name]
        turn = (sc['angle'] * 65536 // 360 + sc['turn'] * 256)
        turn = turn - 65536 if turn >= 32768 else turn
        g = genuine_tics(name)
        gw = genuine(name)
        for wire in range(7, 22):
            args = [str(wire), str(C / f'wads/{name}.wad'), str(sc['start'][0]), str(sc['start'][1]), str(turn), sc['action']]
            p = subprocess.run([str(PRISTINE)] + args, capture_output=True, text=True)
            text = p.stdout + p.stderr
            ticks = []
            for l in text.splitlines():
                if not l.startswith('tic '): continue
                w = l.split()
                pos = tuple(sorted(tuple(int(v) for v in q.split(',')) for q in w[5].split(';'))) if len(w) > 5 and w[5] else ()
                ticks.append((int(w[1]), int(w[2]), int(w[3]), pos))
            refused = next((l for l in text.splitlines() if l.startswith('refused') or l.startswith('Error')), None)
            setup = next((l for l in text.splitlines() if l.startswith('setup')), None)
            # genuine tic t (dump after G_Ticker of gametic t-1) <-> Rust snapshot tic t
            gm = {t: (r, m, q) for t, r, m, q in g}
            diffs = [(t, (r, a, q), gm.get(t)) for t, r, a, q in ticks[1:] if t in gm and gm[t] != (r, a, q)]
            compared = sum(1 for t, *_ in ticks[1:] if t in gm)
            log = C / f'oldwires/w{wire}_{name}.log'
            log.parent.mkdir(parents=True, exist_ok=True)
            if log.exists(): log.unlink()
            subprocess.run([str(PROBE)] + args, capture_output=True, text=True, env=dict(os.environ, CORNER_RUST_LOG=str(log)))
            rw = rust(log)
            walk_eq = len(rw) == len(gw) and all(a['start'] == tuple(b['start']) and a['cells'] == b['cells'] and a['iters'] == b['iters']
                                                 and a['reached'] == b['reached'] and a['n'] == b['n'] for a, b in zip(rw, gw))
            kinds = sorted(set(a['kind'] for a in rw))
            rows[f'{name}/w{wire}'] = dict(setup=setup, refused=refused, tics=len(ticks) - 1, compared=compared, rng_actor_diffs=len(diffs),
                                           first_diff=diffs[0] if diffs else None, walk_calls=len(rw), genuine_calls=len(gw),
                                           walk_equal=walk_eq, kinds=kinds)
            print(f'{name} wire {wire}: {setup or ""} refused {refused} tics {len(ticks) - 1} compared {compared} rng/actor/position diffs {len(diffs)}'
                  f'{" first " + str(diffs[0]) if diffs else ""} | walks rust {len(rw)} genuine {len(gw)} equal {walk_eq} {kinds}')
    (C / 'oldwires/summary.json').write_text(json.dumps(rows, indent=1))

if __name__ == '__main__':
    main()
