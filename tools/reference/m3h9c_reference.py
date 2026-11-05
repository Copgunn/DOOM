#!/usr/bin/env python3
"""Three-variant reference run of the M3h9c whole-engine runner on the synthetic game WAD.

Builds the runner's strict, --overflow-check and --address-check variants
(m3h9c_build.py) under --work, runs every tests/reference/m3h9c_scripts/*.lmp on
tests/reference/m3h9c_synthetic.wad in each variant, asserts that the three dump sets
are byte-identical (SHA-256 of the concatenated snapshots, m3h9c_dump.py order) and that
they end the same way, and writes tests/reference/m3h9c_synthetic_results.json:
per script the tic count, the ending (marker: the demo marker reached; exit: the run
stopped on the exit tic) and its tic, prndindex after the load and at the end, the
thinker and mobj counts after the load, the snapshot count and the dump-set digest.
--check compares the regenerated results with the committed file instead of writing.
Everything here is synthetic. The address variant runs with ASAN_OPTIONS=detect_leaks=0
(leak detection disabled). Dumps are deleted after hashing unless --keep-dumps is given.

    m3h9c_reference.py [--work DIR] [--check] [--keep-dumps] [--cc CC]
"""
import argparse, hashlib, json, os, re, shutil, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m3h9c_build as build  # noqa: E402
import m3h9c_dump as dump    # noqa: E402

WAD = ROOT / 'tests/reference/m3h9c_synthetic.wad'
SCRIPTS = ROOT / 'tests/reference/m3h9c_scripts'
RESULTS = ROOT / 'tests/reference/m3h9c_synthetic_results.json'
VARIANTS = ['strict', 'overflow', 'address']
SUMMARY = re.compile(r'^summary: tics (\d+) end=(marker|exit) end_tic (\d+) gamestate (\d+) prndindex (\d+) .* observer_misses (\d+) unknown_messages (\d+) .* us_per_tic ([\d.]+)$', re.M)
sha = lambda b: hashlib.sha256(b).hexdigest()

def run(binary, lmp, out):
    if out.exists(): shutil.rmtree(out)
    out.mkdir(parents=True)
    p = subprocess.run([str(binary), '--wad', str(WAD), '--wad', str(lmp), '--demo', lmp.stem.upper(), '--out', str(out)],
                       capture_output=True, text=True, env=dict(os.environ, ASAN_OPTIONS='detect_leaks=0'))
    log = out.with_suffix('.log'); log.write_text(p.stdout + p.stderr)
    m = SUMMARY.search(p.stdout)
    if p.returncode or not m: sys.exit(f'FAILED: {binary.parent.name} {lmp.name}: exit status {p.returncode}; log {log}')
    digest = hashlib.sha256(); names = dump.snapshot_names(out)
    for n in names: digest.update((out / n).read_bytes())
    if m[6] != '0': sys.exit(f'FAILED: {binary.parent.name} {lmp.name}: {m[6]} thinkers without an observer serial')
    rec = dump.records(out / 'load.txt')
    return dict(tics=int(m[1]), end=m[2], end_tic=int(m[3]), prndindex_after_load=rec['H'][0]['prndindex'],
                prndindex_end=int(m[5]), thinkers_at_load=rec['H'][0]['numthinkers'], mobjs_at_load=len(rec.get('M', [])),
                unknown_messages=int(m[7]), snapshots=len(names), dumps_sha256=digest.hexdigest()), float(m[8])

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path(os.environ.get('HOME', '/tmp')) / 'scratch/doom-m3h9c-reference')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--keep-dumps', action='store_true')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    start = time.monotonic()
    lmps = sorted(SCRIPTS.glob('*.lmp'))
    if not lmps: sys.exit('no scripts')
    binaries = {}
    for v in VARIANTS:
        with open(os.devnull, 'w') as quiet:
            stdout, sys.stdout = sys.stdout, quiet
            try: binaries[v] = build.build(v, work / 'build', a.cc)
            finally: sys.stdout = stdout
    built = time.monotonic()
    scripts = {}
    for lmp in lmps:
        per, timing = {}, {}
        for v in VARIANTS:
            out = work / 'runs' / f'{v}_{lmp.stem}'
            per[v], timing[v] = run(binaries[v], lmp, out)
            if not a.keep_dumps: shutil.rmtree(out)
        if len({json.dumps(r, sort_keys=True) for r in per.values()}) != 1:
            sys.exit(f'FAILED: {lmp.name}: the three variants differ')
        scripts[lmp.stem] = dict(lmp_sha256=sha(lmp.read_bytes()), **per['strict'])
        r = per['strict']
        print(f"{lmp.stem}: tics {r['tics']} end={r['end']} at tic {r['end_tic']} prndindex {r['prndindex_after_load']}->{r['prndindex_end']} "
              f"thinkers {r['thinkers_at_load']} mobjs {r['mobjs_at_load']} variants identical: yes; us/tic "
              + ' '.join(f'{v} {t}' for v, t in timing.items()))
    results = dict(schema='DOOM-M3H9C-SYNTHETIC-RESULTS-v1', wad_sha256=sha(WAD.read_bytes()),
                   runner_contract_sha256=sha((ROOT / 'tests/reference/m3h9c_runner_contract.json').read_bytes()),
                   variants=VARIANTS, variants_identical=True, asan_options='detect_leaks=0 (leak detection disabled)',
                   scripts=scripts)
    text = json.dumps(results, indent=1) + '\n'
    print(f'wall time: build {built - start:.1f} s, runs {time.monotonic() - built:.1f} s, total {time.monotonic() - start:.1f} s')
    if a.check:
        if not RESULTS.exists() or RESULTS.read_text() != text: sys.exit(f'{RESULTS.relative_to(ROOT)} differs from this run')
        print(f'{RESULTS.relative_to(ROOT)} matches ({len(scripts)} scripts)')
    else:
        RESULTS.write_text(text); print(f'wrote {RESULTS.relative_to(ROOT)}')

if __name__ == '__main__':
    main()
