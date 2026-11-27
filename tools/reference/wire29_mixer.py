#!/usr/bin/env python3
"""Mixer goldens of the M6b items, the pause items among them, from the genuine mixer only (M6b step 7;
docs/M6B_REFERENCE_INVESTIGATION.md section 6.1, "Left out"). Never from Rust output.

The M5c mixer harness, unchanged and imported read-only, run on the committed M6b sound streams, as
tools/reference/m6a_mixer.py runs it on the M6a ones: for every tests/reference/m6b_sound/<ITEM>.sound (the genuine
M6b runner's sound-logic stream, tools/reference/m6b_goldens.py, frozen) and every tests/reference/wire29_session/<ITEM>.sound
(the same runner's, for the wire 29 items DEADSPEC, LOADP and TURBOP, tools/reference/wire29_pause.py):

  starts   tools/reference/m5c_starts.py converts the stream to a start list (the S rows with outcome started; their
           gametic is the session's, continuous across loads, the intermission, the finale and every paused tic);
  mixer    the genuine mixer driver of tools/reference/m5c_mixer_build.py, built strict, overflow and address under
           --work, and the independent model (m5c_mixer_model.py) run the list under both schedules through
           m5c_mixer_reference.run_sessions: identical rows, no sanitizer report, every handle equal to the sound-logic
           run's (no m row); the golden is the strict build's rows under ticbuffers (buffer k after tic floor(512k/315)).

The pause adds nothing to the mixer: S_UpdateSounds and the mixer run every frame with no test of paused
(d_main.c:392-405), so a paused tic is a tic of the schedule like any other, and its channels play on. No melt rows:
the M6b frame goldens were made with the counter melt clock, so no melt step counts exist; the buffer sequence does not
depend on the melts (m6a_mixer.py, section 1.16.4).

Written without --check (compared byte for byte with --check, which writes nothing to the repository):
  tests/reference/wire29_mixer/starts/<ITEM>.txt     the start list (DOOM-M5C-STARTS-v1)
  tests/reference/wire29_mixer/goldens/<ITEM>.golden the genuine strict rows (DOOM-M5C-MIXER-v1)
  tests/reference/wire29_mixer_results.json          DOOM-WIRE29-MIXER-RESULTS-v1: per item the stream's digest, starts,
                                                     tics, buffers and digests
--selftest: one flipped stream bit (only that buffer's row and the stream digest differ) and one start moved by a tic
(the first difference is that start's echo row), on PAUSE.

    wire29_mixer.py [--work DIR] [--check] [--selftest] [--jobs N] [--cc CC]
"""
import argparse, hashlib, json, os, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m5c_mixer_build as mb        # noqa: E402  (read-only)
import m5c_mixer_reference as MR    # noqa: E402  (read-only: run_sessions, run_driver, diff_rows)
import m5c_starts                   # noqa: E402  (read-only)

SOUNDS = [ROOT / 'tests/reference/m6b_sound', ROOT / 'tests/reference/wire29_session']
OUT = ROOT / 'tests/reference/wire29_mixer'
RESULTS = ROOT / 'tests/reference/wire29_mixer_results.json'
SCHEMA = 'DOOM-WIRE29-MIXER-RESULTS-v1'
SELFTEST_ITEM = 'PAUSE'
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()


def generate(bins, work, jobs):
    """(files {path: text}, results doc, failures)."""
    conv = {f.stem: (f.read_bytes(), m5c_starts.convert(f.read_text(), f.name))
            for d in SOUNDS for f in sorted(d.glob('*.sound'))}
    so = sys.stdout
    res, fails = MR.run_sessions({n: t for n, (_d, t) in conv.items()}, bins, MR.VARIANTS, work, jobs, work / 'lists')
    sys.stdout = so
    files, per = {}, {}
    for n, (data, text) in conv.items():
        rows = res[(n, 'strict', MR.GOLDEN_SCHEDULE)]['rows']
        prow = res[(n, 'strict', 'pertic')]['rows']
        z, pz = rows.splitlines()[-1].split(), prow.splitlines()[-1].split()
        files[OUT / 'starts' / f'{n}.txt'] = text
        files[OUT / 'goldens' / f'{n}.golden'] = rows
        per[n] = dict(sound_sha256=sha(data), starts=sum(1 for l in text.splitlines() if l.startswith('s ')),
                      tics=int(next(l.split()[1] for l in text.splitlines() if l.startswith('tics '))),
                      starts_sha256=sha(text), buffers=int(z[1]), stream_sha256=z[2], golden_sha256=sha(rows),
                      model='identical', variants='identical',
                      pertic=dict(buffers=int(pz[1]), stream_sha256=pz[2], rows_sha256=sha(prow)))
    doc = dict(schema=SCHEMA, mixer_contract_sha256=sha(mb.CONTRACT.read_bytes()),
               source='tests/reference/m6b_sound/*.sound (the genuine M6b runner, tools/reference/m6b_goldens.py), '
                      'and tests/reference/wire29_session/*.sound (tools/reference/wire29_pause.py), converted by '
                      'tools/reference/m5c_starts.py; the genuine mixer of tools/reference/m5c_mixer_build.py',
               switches=dict(configuration='integrated', schedule=MR.GOLDEN_SCHEDULE, frames_per_buffer=512,
                             frames_per_tic=315, rule=mb.SCHEDULES[MR.GOLDEN_SCHEDULE], also_recorded='pertic (digests only)',
                             pause='a paused tic is an ordinary tic of the schedule (d_main.c:392-405)'),
               wad=MR.WADS['main'][0], variants=MR.VARIANTS,
               asan_options='detect_leaks=0 (leak detection disabled; not LeakSanitizer coverage)',
               tools_sha256={f'tools/reference/{t}': sha((ROOT / 'tools/reference' / t).read_bytes())
                             for t in ('wire29_mixer.py', 'm5c_starts.py', 'm5c_mixer_driver.c')},
               items=len(per), starts=sum(p['starts'] for p in per.values()), tics=sum(p['tics'] for p in per.values()),
               buffers=sum(p['buffers'] for p in per.values()),
               goldens_sha256=sha(''.join(f'{n}\n{files[OUT / "goldens" / f"{n}.golden"]}' for n in sorted(per))),
               per_item=per)
    files[RESULTS] = json.dumps(doc, indent=1) + '\n'
    return files, doc, fails


def selftest(bins, work):
    out = []
    n = SELFTEST_ITEM
    golden = (OUT / 'goldens' / f'{n}.golden').read_text()
    lst = OUT / 'starts' / f'{n}.txt'
    sc = dict(group='golden', wad='main', lumps=False, probe=False)
    byte = 2048 * 150 + 7
    rows = MR.run_driver(bins['strict'], sc, lst, MR.GOLDEN_SCHEDULE, work / 'runs', ['--perturb-byte', str(byte)])['rows']
    keys = [k for k, _x, _y in MR.diff_rows(golden, rows, MR.GOLDEN_SCHEDULE)]
    out.append(('flipped stream bit', f'byte {byte}: differing rows {keys}', keys == ['b 150', 'z']))
    text = lst.read_text().splitlines(keepends=True)
    starts = [i for i, l in enumerate(text) if l.startswith('s ')]
    tic = lambda i: int(text[i].split()[1])
    # a start after the first pause whose next start is at least two tics later (keeps TIC SEQ increasing)
    i = next(i for i, j in zip(starts, starts[1:] + [None]) if tic(i) > 113 and (j is None or tic(j) > tic(i) + 1))
    f = text[i].split()
    moved_tic = int(f[1]) + 1
    f[1] = str(moved_tic); f[3] = str(moved_tic); f[-1] = '-'
    planted = work / 'selftest_moved.txt'
    planted.write_text(''.join(text[:i] + [' '.join(f) + '\n'] + text[i + 1:]))
    rows = MR.run_driver(bins['strict'], sc, planted, MR.GOLDEN_SCHEDULE, work / 'runs')['rows']
    keys = [k for k, _x, _y in MR.diff_rows(golden, rows, MR.GOLDEN_SCHEDULE)]
    orig = text[i].split()
    out.append(('start moved by a tic', f'start {orig[1]} {orig[2]} to tic {moved_tic}: first difference {keys[:1]}',
                bool(keys) and keys[0] == f's {orig[1]} {orig[2]}' and moved_tic != int(orig[1])))
    planted.unlink()
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-wire29-mixer')
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
            extra = [str(p.relative_to(ROOT)) for d in ('starts', 'goldens') if (OUT / d).is_dir()
                     for p in sorted((OUT / d).iterdir()) if p not in files]
            if bad or extra: sys.exit(f'differs: {bad[:20]} ({len(bad)}); not produced by this run: {extra}')
            print(f'{len(files)} files match ({doc["items"]} items, {doc["starts"]} starts, {doc["tics"]} tics, '
                  f'{doc["buffers"]} buffers)')
        else:
            for d in ('starts', 'goldens'): (OUT / d).mkdir(parents=True, exist_ok=True)
            for p, t in files.items(): p.write_text(t)
            print(f'wrote {len(files)} files ({doc["items"]} items, {doc["starts"]} starts, {doc["tics"]} tics, '
                  f'{doc["buffers"]} buffers)')
    if a.selftest:
        res = selftest(bins, work)
        for what, detail, ok in res: print(f'selftest {what}: {detail}: {"pass" if ok else "FAIL"}')
        if not all(ok for *_x, ok in res): sys.exit(1)


if __name__ == '__main__':
    main()
