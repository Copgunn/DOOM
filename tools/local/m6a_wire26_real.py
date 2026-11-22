#!/usr/bin/env python3
"""Local-only: wire 26 started as a new game against wire 25 on the 61 real Episode 1 runs (M6a step 4, the M5c
method of docs/M6A_REFERENCE_INVESTIGATION.md section 4.3; Rust against Rust, never in CI: it reads the
commercial IWAD).

For every run of tests/reference/m4b_real_results.json (the committed E1M1-E1M9 scripts and DEMO1, extracted from
the IWAD copy and patched 109 to 110 by tools/local/m5a_real.py), `wire26_newgame_check` (release) runs the demo
under both wires and requires every DSIM42 capture, with its start group removed and its names set back to wire
25's, to have the DSIM41 capture's bytes and digest, equal sound records and the same refusal. Prints one line per
run and the totals; writes nothing in the repository (the demo copies go to --work, deleted after each run).

    python3 tools/local/m6a_wire26_real.py --binary PATH [--iwad PATH] [--work DIR] [--maps E1M1,..] [--only WALK,..]
"""
import argparse, re, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/local'))
sys.dont_write_bytecode = True
from m5a_real import runs_selected, demo_path  # noqa: E402


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--binary', type=Path, required=True, help='wire26_newgame_check (release)')
    ap.add_argument('--iwad', type=Path, default=Path.home() / 'scratch/m6a/s1/iwad/doom.wad')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/m6a/wire26-real')
    ap.add_argument('--maps'); ap.add_argument('--only')
    a = ap.parse_args()
    if not a.iwad.is_file():
        print(f'skip: no IWAD at {a.iwad} (local-only wire-26 real-map check)'); return 0
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    todo = runs_selected(a.maps.split(',') if a.maps else None, {x.upper() for x in a.only.split(',')} if a.only else None)
    runs = captures = refused = 0
    fails = []
    for r in todo:
        name = f"{r['map']}_{Path(r['script']).stem.upper()}"
        w = work / name
        if w.exists(): shutil.rmtree(w)
        w.mkdir(parents=True)
        try:
            lmp = demo_path(r, w, a.iwad)
            p = subprocess.run([str(a.binary), '--wad', str(a.iwad), '--map', r['map'], '--demo', str(lmp)],
                               capture_output=True, text=True)
        finally:
            shutil.rmtree(w, ignore_errors=True)
        m = re.search(r': (\d+) captures equal, (\d+) tics, refused (.*), load refused (.*)$', p.stdout.strip())
        if p.returncode or not m or m[4] != 'None':
            fails.append(f'{name}: exit {p.returncode} {p.stdout.strip()} {p.stderr.strip()[-300:]}')
            print(f'{name}: FAILED', flush=True); continue
        runs += 1; captures += int(m[1]); refused += m[3] != 'None'
        print(f'{name}: {m[1]} captures equal, refused {m[3]}', flush=True)
    print(f'totals: {runs} of {len(todo)} runs, {captures} captures equal, {refused} runs end in a refusal, {len(fails)} failed')
    for f in fails: print(f)
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
