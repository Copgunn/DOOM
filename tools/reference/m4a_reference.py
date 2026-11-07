#!/usr/bin/env python3
"""Three-variant reference run of the genuine whole-engine runner on the M4a synthetic WADs.

Manifest-driven: tests/reference/m4a_manifest.json (schema DOOM-M4A-MANIFEST-v1) lists
per WAD (tests/reference/m4a_wads/, written by m4a_wad.py) its scripts
(tests/reference/m4a_scripts/, text and .lmp, written by tools/e1m1_scripts.py) with the
expected C ending (`c_end`: marker, exit or reborn; `end_tic`), the `secretexit` flag on
the last dumped tic, the Rust expectation (`rust`: "match", "tbd" while the Rust side does
not exist, or {"refuse": NAME, "tic": N[, "event": EVENT]}) and `must_show`, the events
the script exists to produce, each with the tic at which it first appears in the dump.

The runner's strict, --overflow-check and --address-check variants are built with
m4a_build.build() (the M4a driver fork, tools/reference/m4a_driver.c) under --work;
every script runs in each variant through m3h9c_reference.run() (imported; its
module-level WAD is pointed at the script's WAD before each call), the three dump sets must be byte-identical (SHA-256 of the
concatenated snapshots) and end the same way, and every expectation is evaluated on the
strict dumps. tests/reference/m4a_synthetic_results.json records per script the tic
count, the ending and its tic, secretexit, prndindex after the load and at the end, the
thinker and mobj counts after the load, the snapshot count, the dump-set digest and the
event tics; its `timing` block (us/tic per variant and the wall time) is informational,
measured on a machine that is not idle, and is not compared by --check.

Event expressions (evaluated over load = tic -1 and every dumped tic):
  first KIND COND...     the first tic with a KIND row (H, M, DR, PL, FL, S, L, B, P, ...)
                         satisfying every COND
  gone KIND COND...      the first tic, after a matching row existed, without one
  edge N KIND COND...    the tic of the N-th appearance of a matching row (a tic with one
                         after a tic, or the load, without one)
  stall SECTOR           the first tic at which an FL thinker on SECTOR existed on the
                         previous tic and still exists while the sector's floor height did
                         not change (a raising floor restored by T_MovePlane, p_floor.c:112-123)
COND is COLUMN OP VALUE with OP one of = != < > <= >= and & (some bit of VALUE set). VALUE is an integer, `Nu` (N map
units, N*65536), `@LABEL` (the sector index of a m4a_wad.py sector label), `%FLAT` (the
flat number of a m3h9c_wad.FLATS name), or an info.h enum member (MT_*, S_*).

Everything here is synthetic. The address variant runs with ASAN_OPTIONS=detect_leaks=0
(leak detection disabled; not LeakSanitizer coverage). Dumps are deleted after the
evaluation unless --keep-dumps is given.

    m4a_reference.py [--work DIR] [--check] [--keep-dumps] [--cc CC] [--only WAD]
                     [--selftest TIC] [--update-events]

--check compares the regenerated results (minus `timing`) with the committed file instead
of writing it. --selftest TIC plants two differences in the strict build on the
manifest's `selftest` script (an extra P_Random draw before G_Ticker TIC, and a changed
forwardmove in command TIC of a copy of the demo) and checks that each is first seen in
the dump of tic TIC (m3h9c_dump.first_difference). --update-events writes the observed
end tic and event tics into the manifest (authoring aid; the result is reviewed and then
held fixed by --check).
"""
import argparse, hashlib, json, os, re, shutil, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m4a_build as build         # noqa: E402
import m3h9c_dump as dump         # noqa: E402
import m3h9c_reference as ref     # noqa: E402
import m3h9c_wad as W             # noqa: E402
import m4a_wad                    # noqa: E402
from m3h9_geometry import enum_index  # noqa: E402

MANIFEST = ROOT / 'tests/reference/m4a_manifest.json'
RESULTS = ROOT / 'tests/reference/m4a_synthetic_results.json'
VARIANTS = ref.VARIANTS
SCHEMA = 'DOOM-M4A-SYNTHETIC-RESULTS-v1'
sha = lambda b: hashlib.sha256(b).hexdigest()
OPS = {'=': lambda a, b: a == b, '!=': lambda a, b: a != b, '<': lambda a, b: a < b, '>': lambda a, b: a > b,
       '<=': lambda a, b: a <= b, '>=': lambda a, b: a >= b, '&': lambda a, b: a & b != 0}
COND = re.compile(r'^(\w+)(!=|<=|>=|=|<|>|&)(.+)$')


class Dumps:
    """Lazy reader of one run's snapshots: tic -1 is load.txt."""

    def __init__(self, directory):
        self.dir, self.cache = directory, {}
        self.names = dump.snapshot_names(directory)
        self.tics = [-1 if n == 'load.txt' else int(n[4:9]) for n in self.names]

    def rows(self, tic):
        if tic not in self.cache:
            self.cache[tic] = dump.records(os.path.join(self.dir, 'load.txt' if tic < 0 else f'tic_{tic:05d}.txt'))
        return self.cache[tic]


def value(token, labels):
    if token.endswith('u') and re.fullmatch(r'-?\d+u', token): return int(token[:-1]) * 65536
    if re.fullmatch(r'-?\d+', token): return int(token)
    if token.startswith('@'): return labels[token[1:]]
    if token.startswith('%'): return W.FLATS.index(token[1:])
    if token.startswith('MT_'): return enum_index('mobjtype_t', token)
    if token.startswith('S_'): return enum_index('statenum_t', token)
    raise ValueError(f'event value {token!r}')


def matcher(words, labels):
    conds = []
    for w in words:
        m = COND.match(w)
        if not m: raise ValueError(f'event condition {w!r}')
        conds.append((m[1], OPS[m[2]], value(m[3], labels)))
    return lambda row: all(c in row and op(row[c], v) for c, op, v in conds)


def event_tic(d, expr, labels):
    """The tic of an event expression on Dumps d, or None if it never happens."""
    words = expr.split()
    if words[0] in ('first', 'gone'):
        kind, ok = words[1], matcher(words[2:], labels)
        seen = False
        for t in d.tics:
            hit = any(ok(r) for r in d.rows(t).get(kind, []))
            if words[0] == 'first' and hit: return t
            if words[0] == 'gone':
                if hit: seen = True
                elif seen: return t
        return None
    if words[0] == 'edge':
        n, kind, ok = int(words[1]), words[2], matcher(words[3:], labels)
        before = False
        for t in d.tics:
            hit = any(ok(r) for r in d.rows(t).get(kind, []))
            if hit and not before:
                n -= 1
                if n == 0: return t
            before = hit
        return None
    if words[0] == 'stall' and len(words) == 2:
        sec = value(words[1], labels)
        prev = None
        for t in d.tics:
            r = d.rows(t)
            fl = any(x['sector'] == sec for x in r.get('FL', []))
            h = r['S'][sec]['floorheight']
            if fl and prev is not None and prev[0] and prev[1] == h: return t
            prev = (fl, h)
        return None
    raise ValueError(f'event {expr!r}')


def evaluate(d, script, labels):
    """Observed values for one script's expectations on the strict dumps."""
    last = d.rows(d.tics[-1])
    obs = dict(secretexit=bool(last['H'][0]['secretexit']),
               must_show=[event_tic(d, e['event'], labels) for e in script.get('must_show', [])])
    rust = script.get('rust')
    if isinstance(rust, dict) and 'event' in rust: obs['rust_tic'] = event_tic(d, rust['event'], labels)
    return obs


def quiet_build(variant, out, cc):
    with open(os.devnull, 'w') as q:
        stdout, sys.stdout = sys.stdout, q
        try: return build.build(variant, out, cc)
        finally: sys.stdout = stdout


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path(os.environ.get('HOME', '/tmp')) / 'scratch/doom-m4a-reference')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--keep-dumps', action='store_true')
    ap.add_argument('--cc', default='cc')
    ap.add_argument('--only', help='run the scripts of one WAD (no results file is written or checked)')
    ap.add_argument('--selftest', type=int, metavar='TIC')
    ap.add_argument('--update-events', action='store_true')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    man = json.loads(MANIFEST.read_text())
    if man.get('schema') != 'DOOM-M4A-MANIFEST-v1': sys.exit('manifest schema')
    start = time.monotonic()
    binaries = {v: quiet_build(v, work / 'build', a.cc) for v in VARIANTS}
    built = time.monotonic()
    results, timing, failures = {}, {}, []
    for w in man['wads']:
        if a.only and w['name'] != a.only.upper(): continue
        wad = ROOT / w['file']
        if sha(wad.read_bytes()) != w['sha256']: sys.exit(f"{w['name']}: WAD differs from the manifest (run m4a_wad.py --check)")
        labels = {s['label']: k for k, s in enumerate(m4a_wad.MAPS[w['name'].lower()]().sectors)}
        ref.WAD = wad
        for s in w['scripts']:
            lmp = ROOT / s['lmp']
            per, us = {}, {}
            for v in VARIANTS:
                out = work / 'runs' / f"{v}_{s['lump']}"
                per[v], us[v] = ref.run(binaries[v], lmp, out)
                if v == 'strict': obs = evaluate(Dumps(str(out)), s, labels)
                if not a.keep_dumps: shutil.rmtree(out)
            if len({json.dumps(r, sort_keys=True) for r in per.values()}) != 1:
                sys.exit(f"FAILED: {s['lump']}: the three variants differ")
            r = per['strict']
            entry = dict(wad=w['name'], lmp_sha256=sha(lmp.read_bytes()), **r, secretexit=obs['secretexit'],
                         must_show={e['what']: t for e, t in zip(s.get('must_show', []), obs['must_show'])})
            if 'rust_tic' in obs: entry['rust_refusal_tic'] = obs['rust_tic']
            results[s['lump']] = entry
            timing[s['lump']] = us
            if a.update_events:
                s['c_end'], s['end_tic'], s['secretexit'] = r['end'], r['end_tic'], obs['secretexit']
                for e, t in zip(s.get('must_show', []), obs['must_show']): e['tic'] = t
                if 'rust_tic' in obs: s['rust']['tic'] = obs['rust_tic']
            bad = []
            if r['end'] != s.get('c_end') or r['end_tic'] != s.get('end_tic'): bad.append(f"end {r['end']} at {r['end_tic']}")
            if obs['secretexit'] != s.get('secretexit'): bad.append(f"secretexit {obs['secretexit']}")
            for e, t in zip(s.get('must_show', []), obs['must_show']):
                if t is None or t != e.get('tic'): bad.append(f"event '{e['what']}' at {t}, expected {e.get('tic')}")
            if 'rust_tic' in obs and obs['rust_tic'] != s['rust'].get('tic'): bad.append(f"rust refusal tic {obs['rust_tic']}")
            if r['unknown_messages']: bad.append(f"{r['unknown_messages']} unknown messages")
            if bad: failures.append(f"{s['lump']}: " + '; '.join(bad))
            print(f"{s['lump']}: tics {r['tics']} end={r['end']} at tic {r['end_tic']} secretexit {int(obs['secretexit'])} "
                  f"prndindex {r['prndindex_after_load']}->{r['prndindex_end']} events {len(obs['must_show'])} "
                  f"variants identical: yes; us/tic " + ' '.join(f'{v} {t}' for v, t in us.items())
                  + ('' if not bad else ' EXPECTATION MISMATCH'), flush=True)
    runs = time.monotonic() - built
    print(f'wall time: build {built - start:.1f} s, runs {runs:.1f} s, total {time.monotonic() - start:.1f} s')
    if a.update_events:
        MANIFEST.write_text(json.dumps(man, indent=1) + '\n'); print(f'updated {MANIFEST.relative_to(ROOT)}')
        failures = []
    if failures:
        sys.exit('FAILED expectations:\n  ' + '\n  '.join(failures))
    if not a.only:
        doc = dict(schema=SCHEMA, manifest_sha256=sha(MANIFEST.read_bytes()),
                   runner_contract_sha256=sha(build.M4A.contract.read_bytes()),
                   variants=VARIANTS, variants_identical=True, asan_options='detect_leaks=0 (leak detection disabled)',
                   scripts=results)
        body = json.dumps(doc, indent=1)
        if a.check:
            old = json.loads(RESULTS.read_text()) if RESULTS.exists() else {}
            old.pop('timing', None)
            if json.dumps(old, indent=1) != body: sys.exit(f'{RESULTS.relative_to(ROOT)} differs from this run')
            print(f'{RESULTS.relative_to(ROOT)} matches ({len(results)} scripts)')
        else:
            doc['timing'] = dict(note='us/tic of G_Ticker per variant from the runner summary and the wall time of this '
                                      'run; measured on a machine that was not idle (informational, not compared by --check)',
                                 wall_seconds=dict(build=round(built - start, 1), runs=round(runs, 1)), us_per_tic=timing)
            RESULTS.write_text(json.dumps(doc, indent=1) + '\n'); print(f'wrote {RESULTS.relative_to(ROOT)}')
    if a.selftest is not None:
        st = man['selftest']
        w = next(x for x in man['wads'] if x['name'] == st['wad'])
        s = next(x for x in w['scripts'] if x['lump'] == st['script'])
        ref.WAD = ROOT / w['file']
        lmp, t = ROOT / s['lmp'], a.selftest
        plain = work / 'runs' / 'selftest_plain'; ref.run(binaries['strict'], lmp, plain)
        drawn = work / 'runs' / 'selftest_draw'; ref.run(binaries['strict'], lmp, drawn, ['--selftest-extra-draw', str(t)])
        data = bytearray(lmp.read_bytes()); at = 13 + 4 * t
        if at >= len(data) - 1: sys.exit('selftest TIC beyond the demo')
        data[at] = (data[at] + 25) & 0xff if data[at] not in range(103, 128) else (data[at] - 25) & 0xff
        changed = work / (s['lump'][:7] + 'C.lmp'); changed.write_bytes(data)
        moved = work / 'runs' / 'selftest_cmd'; ref.run(binaries['strict'], changed, moved)
        draw = dump.first_difference(str(plain), str(drawn)); cmd = dump.first_difference(str(plain), str(moved))
        ok = draw == f'tic {t}: row kind H #0 column prndindex' and cmd is not None and cmd.startswith(f'tic {t}:')
        print(f"selftest on {s['lump']}: extra draw {draw!r}; ticcmd {cmd!r}: " + ('pass' if ok else 'FAIL'))
        for x in (plain, drawn, moved): shutil.rmtree(x, ignore_errors=True)
        if not ok: sys.exit(1)


if __name__ == '__main__':
    main()
