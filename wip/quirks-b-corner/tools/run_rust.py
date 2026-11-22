#!/usr/bin/env python3
"""Run every scene through the Rust examples on wires 22, 23 and 24 (player_level_run) and
compare each run tic for tic with the genuine strict dumps through the repository's own
independent comparators (level_compare.py DSIM38 / --profile episode1 DSIM39,
level_compare_m4b.py DSIM40). Comparators are run read-only from the repository with
PYTHONDONTWRITEBYTECODE=1.

    run_rust.py [--bin PATH] [NAME ...]
"""
import json, os, subprocess, sys
from pathlib import Path

C = Path('/home/copgunn/scratch/quirks/followup-b/corner')
REPO = Path('/home/copgunn/src/DOOM')
BIN = Path('/home/copgunn/scratch/quirks/followup-b/target/release/examples/player_level_run')
WIRES = {22: ('level', ['tools/reference/level_compare.py'], []),
         23: ('episode1', ['tools/reference/level_compare.py'], ['--profile', 'episode1']),
         24: ('episode1v2', ['tools/reference/level_compare_m4b.py'], [])}
ENV = dict(os.environ, PYTHONDONTWRITEBYTECODE='1', TMPDIR='/home/copgunn/scratch/quirks/followup-b/tmp')

def main():
    args = sys.argv[1:]
    binary, tag = BIN, 'rust'
    if args and args[0] == '--bin':
        binary, tag = Path(args[1]), args[2]; args = args[3:]
    scenes = json.loads((C / 'wads/scenes.json').read_text())
    names = args or [n for n, v in scenes.items() if not v['optional']]
    res = {}
    for name in names:
        wad, lmp, dumps = C / f'wads/{name}.wad', C / f'wads/{name}.lmp', C / f'runs/strict_{name}'
        for wire, (policy, tool, extra) in WIRES.items():
            mapargs = [] if wire == 24 else ['--map', 'E1M1']
            out = C / f'{tag}/w{wire}_{name}'
            out.mkdir(parents=True, exist_ok=True)
            for f in out.glob('*'): f.unlink()
            p = subprocess.run([str(binary), '--wad', str(wad), '--map', 'E1M1', '--demo', str(lmp), '--out', str(out),
                                '--captures', '--policy', policy], capture_output=True, text=True, env=ENV)
            (out.parent / f'w{wire}_{name}.stdout').write_text(p.stdout + p.stderr)
            refused = (out / 'refused.txt').read_text().strip() if (out / 'refused.txt').exists() else None
            stats = (out / 'stats.txt').read_text() if (out / 'stats.txt').exists() else ''
            mi = next((l.split()[1] for l in stats.splitlines() if l.startswith('max_intercepts')), None)
            js = out.parent / f'w{wire}_{name}.cmp.json'
            q = subprocess.run(['python3', str(REPO / tool[0]), str(dumps), str(out), '--wad', str(wad)] + mapargs + [
                                '--demo', str(lmp)] + extra + ['--json', str(js)], capture_output=True, text=True, cwd=REPO, env=ENV)
            last = (q.stdout.strip().splitlines() or [''])[-1]
            res[f'{name}/w{wire}'] = dict(rust_rc=p.returncode, refused=refused, max_intercepts=mi, cmp_rc=q.returncode,
                                          cmp=q.stdout.strip()[-600:], cmp_err=q.stderr.strip()[-600:])
            print(f'{name} wire {wire}: rust rc {p.returncode} refused {refused!r} max_intercepts {mi} | cmp rc {q.returncode}: {last}')
            if p.returncode: print('   rust stderr:', p.stderr.strip()[-400:])
            if q.returncode and q.stderr.strip(): print('   cmp stderr:', q.stderr.strip()[-400:])
    sp = C / f'{tag}/summary.json'
    old = json.loads(sp.read_text()) if sp.exists() else {}
    old.update(res); sp.write_text(json.dumps(old, indent=1))

if __name__ == '__main__':
    main()
