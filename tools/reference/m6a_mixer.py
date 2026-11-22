#!/usr/bin/env python3
"""Mixer goldens of the M6a session items across level boundaries, from the genuine mixer only
(docs/M6A_REFERENCE_INVESTIGATION.md section 1.16.4 and Phase 2 step 6). Never from Rust output.

The M5c mixer harness, unchanged and imported read-only, run on the M6a sound streams: for every item of
tests/reference/m6a_sound/<ITEM>.sound (the genuine M6a runner's sound-logic stream, tools/reference/m6a_goldens.py):

  starts   tools/reference/m5c_starts.py converts the stream to a start list (the S rows with outcome started; their
           gametic is the session's, continuous across loads, intermissions and the finale);
  mixer    the genuine mixer driver of tools/reference/m5c_mixer_build.py, built strict, overflow and address under
           --work, and the independent model (m5c_mixer_model.py) run the list under both schedules through
           m5c_mixer_reference.run_sessions: identical rows, no sanitizer report, every handle equal to the sound-logic
           run's (no m row); the golden is the strict build's rows under ticbuffers (buffer k after tic floor(512k/315));
  melts    the declared melt schedule (section 1.16.4 and the owner's ruling of 2026-11-21: nothing is mixed or
           submitted while a melt runs, the buffer due after the melt's tic is mixed after it, nothing skipped): for
           every melt of the item (the Y rows of tests/reference/m6a_melt/<ITEM>.melt and the number of its V rows,
           the genuine runner under --melt-clock unit), a row `w TIC STEPS K`: the melt that the display of session tic
           TIC starts has STEPS steps and falls after buffer K-1 and before buffer K, with K = ceil(315 * TIC / 512)
           (the buffers of tics 0..TIC-1). The tool requires the genuine golden to have mixed exactly K buffers after
           tics before TIC. The genuine mixer itself knows nothing of melts: that no buffer is produced during a melt is
           read in d_main.c:331-344 (the melt loop calls no sound function) and declared, and these rows place each melt
           in the buffer sequence the genuine mixer produced.

Written without --check (compared byte for byte with --check, which writes nothing to the repository):
  tests/reference/m6a_mixer/starts/<ITEM>.txt     the start list (DOOM-M5C-STARTS-v1)
  tests/reference/m6a_mixer/goldens/<ITEM>.golden the genuine strict rows (DOOM-M5C-MIXER-v1)
  tests/reference/m6a_mixer/melts/<ITEM>.txt      DOOM-M6A-MIXER-MELTS-v1: the w rows
  tests/reference/m6a_mixer_results.json          DOOM-M6A-MIXER-RESULTS-v1: per item the stream's digest, starts, tics,
                                                  buffers, the digests, the melts and the end
--selftest: one flipped stream bit (only that buffer's row and the stream digest differ), one start moved by a tic (the
first difference is that start's echo row) and a w row with K off by one (refused by the melt check).

    m6a_mixer.py [--work DIR] [--check] [--selftest] [--jobs N] [--cc CC]
"""
import argparse, hashlib, json, os, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5c_mixer_build as mb        # noqa: E402  (read-only)
import m5c_mixer_reference as MR    # noqa: E402  (read-only: run_sessions, run_driver, diff_rows, expand)
import m5c_starts                   # noqa: E402  (read-only)

SOUND = ROOT / 'tests/reference/m6a_sound'
MELT = ROOT / 'tests/reference/m6a_melt'
OUT = ROOT / 'tests/reference/m6a_mixer'
RESULTS = ROOT / 'tests/reference/m6a_mixer_results.json'
SCHEMA = 'DOOM-M6A-MIXER-RESULTS-v1'
MELTS_SCHEMA = 'DOOM-M6A-MIXER-MELTS-v1'
SELFTEST_ITEM = 'CHAIN'
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()


def buffers_before(tic):
    """ceil(315 * tic / 512): the buffers mixed after tics 0..tic-1 under ticbuffers."""
    return (315 * tic + 511) // 512


def melts(name):
    """[(tic, steps)] of an item's melt golden (Y rows; V-row counts)."""
    text = (MELT / f'{name}.melt').read_text()
    ys = [int(l.split(',')[1]) for l in text.splitlines() if l.startswith('Y,')]
    vs = {}
    for l in text.splitlines():
        if l.startswith('V,'): t = int(l.split(',')[1]); vs[t] = vs.get(t, 0) + 1
    return [(t, vs.get(t, 0)) for t in ys]


def melt_text(name, rows):
    """The melts file of an item, w rows placed in the golden's buffer sequence; raises ValueError when the genuine
    golden did not mix exactly K buffers before a melt's tic."""
    exp = MR.expand(rows, MR.GOLDEN_SCHEDULE)
    btics = [int(line.split()[2]) for key, line in exp if key.startswith('b ')]
    out = [f'schema {MELTS_SCHEMA}',
           f'# {name}: w TIC STEPS K, the melt started by the display of session tic TIC (STEPS steps, one per tic under the',
           '# declared schedule) falls between buffer K-1 and buffer K: no buffer is mixed during it (d_main.c:331-344).']
    for tic, steps in melts(name):
        k = buffers_before(tic)
        mixed = sum(1 for t in btics if t < tic)
        if steps < 34 or steps > 43: raise ValueError(f'{name}: melt at tic {tic} has {steps} steps (34..43 expected)')
        if mixed != k:
            raise ValueError(f'{name}: {mixed} buffers mixed before tic {tic}, the schedule gives {k}')
        out.append(f'w {tic} {steps} {k}')
    return '\n'.join(out) + '\n'


def check_melts(text, rows):
    """Every w row's K is the schedule's and equals the buffers the golden mixed after tics before its tic."""
    exp = MR.expand(rows, MR.GOLDEN_SCHEDULE)
    btics = [int(line.split()[2]) for key, line in exp if key.startswith('b ')]
    bad = []
    for l in text.splitlines():
        if not l.startswith('w '): continue
        _w, tic, steps, k = l.split()
        mixed = sum(1 for t in btics if t < int(tic))
        if int(k) != buffers_before(int(tic)) or mixed != int(k): bad.append(l)
    return bad


def convert_all():
    out = {}
    for f in sorted(SOUND.glob('*.sound')):
        data = f.read_bytes()
        out[f.stem] = (data, m5c_starts.convert(data.decode(), f.name))
    return out


def generate(bins, work, jobs):
    """(files {path: text}, results doc, failures)."""
    conv = convert_all()
    so = sys.stdout
    res, fails = MR.run_sessions({n: t for n, (_d, t) in conv.items()}, bins, MR.VARIANTS, work, jobs, work / 'lists')
    sys.stdout = so
    files, per = {}, {}
    for n, (data, text) in conv.items():
        rows = res[(n, 'strict', MR.GOLDEN_SCHEDULE)]['rows']
        prow = res[(n, 'strict', 'pertic')]['rows']
        try: mt = melt_text(n, rows)
        except ValueError as e: fails.append(str(e)); continue
        z, pz = rows.splitlines()[-1].split(), prow.splitlines()[-1].split()
        files[OUT / 'starts' / f'{n}.txt'] = text
        files[OUT / 'goldens' / f'{n}.golden'] = rows
        files[OUT / 'melts' / f'{n}.txt'] = mt
        per[n] = dict(sound_sha256=sha(data), starts=sum(1 for l in text.splitlines() if l.startswith('s ')),
                      tics=int(next(l.split()[1] for l in text.splitlines() if l.startswith('tics '))),
                      starts_sha256=sha(text), buffers=int(z[1]), stream_sha256=z[2], golden_sha256=sha(rows),
                      melts=[[int(x) for x in l.split()[1:]] for l in mt.splitlines() if l.startswith('w ')],
                      end=next((' '.join(l.split()[:3]) for l in rows.splitlines() if l.startswith('x ')), 'drained'),
                      sound_stream_end=next((l for l in text.splitlines() if l.startswith('# the session ended')), 'marker'),
                      model='identical', variants='identical',
                      pertic=dict(buffers=int(pz[1]), stream_sha256=pz[2], rows_sha256=sha(prow)))
    doc = dict(schema=SCHEMA, mixer_contract_sha256=sha(mb.CONTRACT.read_bytes()),
               source='tests/reference/m6a_sound/*.sound (the genuine M6a runner, tools/reference/m6a_goldens.py), converted '
                      'by tools/reference/m5c_starts.py; the genuine mixer of tools/reference/m5c_mixer_build.py',
               switches=dict(configuration='integrated', schedule=MR.GOLDEN_SCHEDULE, frames_per_buffer=512, frames_per_tic=315,
                             rule=mb.SCHEDULES[MR.GOLDEN_SCHEDULE], also_recorded='pertic (digests only)',
                             melts='no buffer during a melt; the buffer of the melt\'s tic after it (declared, d_main.c:331-405)'),
               wad=MR.WADS['main'][0], variants=MR.VARIANTS,
               asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)',
               tools_sha256={f'tools/reference/{t}': sha((ROOT / 'tools/reference' / t).read_bytes())
                             for t in ('m6a_mixer.py', 'm5c_starts.py', 'm5c_mixer_driver.c')},
               items=len(per), starts=sum(p['starts'] for p in per.values()), tics=sum(p['tics'] for p in per.values()),
               buffers=sum(p['buffers'] for p in per.values()), melts=sum(len(p['melts']) for p in per.values()),
               goldens_sha256=sha(''.join(f'{n}\n{files[OUT / "goldens" / f"{n}.golden"]}' for n in sorted(per))), per_item=per)
    files[RESULTS] = json.dumps(doc, indent=1) + '\n'
    return files, doc, fails


def selftest(bins, work):
    out = []
    n = SELFTEST_ITEM
    golden = (OUT / 'goldens' / f'{n}.golden').read_text()
    lst = OUT / 'starts' / f'{n}.txt'
    sc = dict(group='golden', wad='main', lumps=False, probe=False)
    byte = 2048 * 300 + 7
    rows = MR.run_driver(bins['strict'], sc, lst, MR.GOLDEN_SCHEDULE, work / 'runs', ['--perturb-byte', str(byte)])['rows']
    keys = [k for k, _x, _y in MR.diff_rows(golden, rows, MR.GOLDEN_SCHEDULE)]
    out.append(('flipped stream bit', f'byte {byte}: differing rows {keys}', keys == ['b 300', 'z']))
    text = lst.read_text().splitlines(keepends=True)
    i = next(i for i, l in enumerate(text) if l.startswith('s ') and int(l.split()[1]) > 126)   # a start after the first boundary
    f = text[i].split()
    moved_tic = int(f[1]) + 1
    nxt = next((l for l in text[i + 1:] if l.startswith('s ')), None)
    if nxt is not None and int(nxt.split()[1]) <= moved_tic: moved_tic = int(f[1])         # keep TIC SEQ increasing
    f[1] = str(moved_tic); f[2] = str(int(f[2])); f[3] = str(moved_tic); f[-1] = '-'
    planted = work / 'selftest_moved.txt'
    planted.write_text(''.join(text[:i] + [' '.join(f) + '\n'] + text[i + 1:]))
    rows = MR.run_driver(bins['strict'], sc, planted, MR.GOLDEN_SCHEDULE, work / 'runs')['rows']
    keys = [k for k, _x, _y in MR.diff_rows(golden, rows, MR.GOLDEN_SCHEDULE)]
    orig = text[i].split()
    out.append(('start moved by a tic', f'start {orig[1]} {orig[2]} to tic {moved_tic}: first difference {keys[:1]}',
                bool(keys) and keys[0] == f's {orig[1]} {orig[2]}' and moved_tic != int(orig[1])))
    planted.unlink()
    mt = (OUT / 'melts' / f'{n}.txt').read_text().splitlines(keepends=True)
    j = next(j for j, l in enumerate(mt) if l.startswith('w ') and int(l.split()[1]) > 0)
    w = mt[j].split(); w[3] = str(int(w[3]) + 1)
    bad = check_melts(''.join(mt[:j] + [' '.join(w) + '\n'] + mt[j + 1:]), golden)
    out.append(('melt row K off by one', f'{" ".join(w)}: refused {bad}', bad == [' '.join(w)]))
    out.append(('committed melt rows', 'none refused', not any(check_melts((OUT / 'melts' / f'{m.stem}.txt').read_text(),
                                                                            (OUT / 'goldens' / f'{m.stem}.golden').read_text())
                                                               for m in sorted((OUT / 'goldens').glob('*.golden')))))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m6a-mixer')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--jobs', type=int, default=1)
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    so, sys.stdout = sys.stdout, open(os.devnull, 'w')
    try: bins = {v: mb.build(v, work / 'build', a.cc, quiet=True) for v in MR.VARIANTS}
    finally: sys.stdout.close(); sys.stdout = so
    if a.check or not a.selftest:
        files, doc, fails = generate(bins, work, max(1, a.jobs))
        if fails: sys.exit('FAILED:\n  ' + '\n  '.join(fails))
        if a.check:
            bad = [str(p.relative_to(ROOT)) for p, t in files.items() if not p.exists() or p.read_text() != t]
            extra = [str(p.relative_to(ROOT)) for d in ('starts', 'goldens', 'melts') if (OUT / d).is_dir()
                     for p in sorted((OUT / d).iterdir()) if p not in files]
            if bad or extra: sys.exit(f'differs: {bad[:20]} ({len(bad)}); not produced by this run: {extra}')
            print(f'{len(files)} files match ({doc["items"]} items, {doc["starts"]} starts, {doc["tics"]} tics, '
                  f'{doc["buffers"]} buffers, {doc["melts"]} melts)')
        else:
            for d in ('starts', 'goldens', 'melts'): (OUT / d).mkdir(parents=True, exist_ok=True)
            for p, t in files.items(): p.write_text(t)
            print(f'wrote {len(files)} files ({doc["items"]} items, {doc["starts"]} starts, {doc["tics"]} tics, '
                  f'{doc["buffers"]} buffers, {doc["melts"]} melts)')
    if a.selftest:
        res = selftest(bins, work)
        for what, detail, ok in res: print(f'selftest {what}: {detail}: {"pass" if ok else "FAIL"}')
        if not all(ok for *_x, ok in res): sys.exit(1)


if __name__ == '__main__':
    main()
