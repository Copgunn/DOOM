#!/usr/bin/env python3
"""Local-only M3h9c reference run: the genuine engine on the real E1M1 (never in CI).

Requires DOOM_IWAD_DIR naming a directory that holds DOOM.WAD; otherwise prints one skip
line and exits 0. The IWAD is opened read-only by the runner and is never copied.
Builds the three runner variants (tools/reference/m3h9c_build.py: strict,
--overflow-check, --address-check) under --work, runs every selected committed demo
script (tests/reference/e1m1_scripts/*.lmp) in each variant, asserts that the three dump
sets are byte-identical (SHA-256 of the concatenated snapshots) and writes
<work>/results/<script>.json. Dumps, logs and binaries stay under --work, which must lie
outside the repository; delete it once the results are recorded.

Everything printed or written to results is a count, a yes/no, an RNG index, a time or
a digest: never a coordinate, angle, lump byte or other map-derived value. A failing
run prints only the variant, the exit status and the log path (its text can hold such
values and stays in --work).

    DOOM_IWAD_DIR=/path tools/local/e1m1_reference.py [--work DIR] [--script NAME]...
                                                       [--selftest TIC]
"""
import argparse, hashlib, json, os, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m3h9c_build as build  # noqa: E402
import m3h9c_dump as dump    # noqa: E402

SCRIPTS = ROOT / 'tests/reference/e1m1_scripts'
VARIANTS = ['strict', 'overflow', 'address']
SUMMARY = re.compile(r'^summary: tics (\d+) end=(marker|exit) end_tic (\d+) gamestate (\d+) prndindex (\d+) .* observer_misses (\d+) unknown_messages (\d+) .* us_per_tic ([\d.]+)$', re.M)

def run(binary, iwad, lmp, out, extra=()):
    out.mkdir(parents=True, exist_ok=True)
    for old in out.glob('*.txt'): old.unlink()
    env = dict(os.environ, ASAN_OPTIONS='detect_leaks=0')
    cmd = [str(binary), '--wad', str(iwad), '--wad', str(lmp), '--demo', build_name(lmp), '--out', str(out), *extra]
    p = subprocess.run(cmd, capture_output=True, text=True, env=env)
    log = out.with_suffix('.log'); log.write_text(p.stdout + p.stderr)
    m = SUMMARY.search(p.stdout)
    if p.returncode or not m:
        sys.exit(f'FAILED: {binary.parent.name} {lmp.stem}: exit status {p.returncode}; log {log}')
    digest = hashlib.sha256()
    for name in dump.snapshot_names(out): digest.update((out / name).read_bytes())
    if m[6] != '0': sys.exit(f'FAILED: {binary.parent.name} {lmp.stem}: {m[6]} thinkers without an observer serial')
    return dict(tics=int(m[1]), end=m[2], end_tic=int(m[3]), prndindex_end=int(m[5]), unknown_messages=int(m[7]),
                us_per_tic=float(m[8]), dumps_sha256=digest.hexdigest())

def build_name(lmp): return lmp.stem.upper()

def load_counts(out):
    rec = dump.records(out / 'load.txt')
    h = rec['H'][0]
    return dict(prndindex_after_load=h['prndindex'], thinkers_at_load=h['numthinkers'], mobjs_at_load=len(rec.get('M', [])),
                snapshots=len(dump.snapshot_names(out)))

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path(os.environ.get('HOME', '/tmp')) / 'scratch/doom-e1m1-reference')
    ap.add_argument('--script', action='append', help='script name (base name of a committed .lmp); default all')
    ap.add_argument('--selftest', type=int, metavar='TIC', help='also check that an extra P_Random draw before TIC on idle is first seen at TIC')
    a = ap.parse_args()
    iwad_dir = os.environ.get('DOOM_IWAD_DIR')
    iwad = Path(iwad_dir) / 'DOOM.WAD' if iwad_dir else None
    if not iwad or not iwad.is_file():
        print('skip: DOOM_IWAD_DIR is unset or holds no DOOM.WAD (local-only E1M1 reference run)'); return 0
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    names = a.script or sorted(p.stem for p in SCRIPTS.glob('*.lmp'))
    lmps = [SCRIPTS / f'{n}.lmp' for n in names]
    for p in lmps:
        if not p.is_file(): sys.exit(f'no committed script {p.name}')
    work.mkdir(parents=True, exist_ok=True); (work / 'results').mkdir(exist_ok=True)
    binaries = {}
    for v in VARIANTS:
        with open(os.devnull, 'w') as quiet:
            stdout, sys.stdout = sys.stdout, quiet
            try: binaries[v] = build.build(v, work / 'build')
            finally: sys.stdout = stdout
    print(f'built {len(binaries)} variants under {work / "build"}')
    for lmp in lmps:
        per = {v: run(binaries[v], iwad, lmp, work / 'runs' / f'{v}_{lmp.stem}') for v in VARIANTS}
        digests = {r['dumps_sha256'] for r in per.values()}
        identical = len(digests) == 1
        counts = load_counts(work / 'runs' / f'strict_{lmp.stem}')
        result = dict(script=lmp.name, lmp_sha256=hashlib.sha256(lmp.read_bytes()).hexdigest(),
                      tics=per['strict']['tics'], end=per['strict']['end'], end_tic=per['strict']['end_tic'],
                      ends_agree=len({(r['end'], r['end_tic'], r['tics']) for r in per.values()}) == 1,
                      **counts, prndindex_end=per['strict']['prndindex_end'], variants_identical=identical,
                      dumps_sha256={v: r['dumps_sha256'] for v, r in per.items()},
                      us_per_tic={v: r['us_per_tic'] for v, r in per.items()},
                      timing_note='G_Ticker wall time only (dump writing excluded); not an idle-machine measurement',
                      asan_options='detect_leaks=0 (leak detection disabled)')
        (work / 'results' / f'{lmp.stem}.json').write_text(json.dumps(result, indent=1) + '\n')
        print(f"{lmp.stem}: tics {result['tics']} end={result['end']} at tic {result['end_tic']} "
              f"prndindex load {result['prndindex_after_load']} end {result['prndindex_end']} "
              f"thinkers {result['thinkers_at_load']} mobjs {result['mobjs_at_load']} "
              f"variants_identical {'yes' if identical else 'NO'} dumps_sha256 {per['strict']['dumps_sha256']} "
              + ' '.join(f'{v}_us_per_tic {r["us_per_tic"]}' for v, r in per.items()))
        if not identical: sys.exit(f'FAILED: {lmp.stem}: the three variants dump different bytes')
    if a.selftest is not None:
        idle = SCRIPTS / 'idle.lmp'
        plain, planted = work / 'runs' / 'strict_idle', work / 'runs' / 'selftest_idle'
        if not (plain / 'load.txt').exists(): run(binaries['strict'], iwad, idle, plain)
        run(binaries['strict'], iwad, idle, planted, ['--selftest-extra-draw', str(a.selftest)])
        diff = dump.first_difference(str(plain), str(planted))
        expected = f'tic {a.selftest}: row kind H #0 column prndindex'
        print(f'selftest: first difference {diff!r}: ' + ('pass' if diff == expected else 'FAIL'))
        if diff != expected: return 1
    return 0

if __name__ == '__main__':
    sys.exit(main())
