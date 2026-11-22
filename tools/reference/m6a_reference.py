#!/usr/bin/env python3
"""Checks of the genuine M6a runner (tools/reference/m6a_build.py; docs/M6A_REFERENCE_INVESTIGATION.md section 3.1).

Expected streams come from the genuine linuxdoom-1.10 code only, never from Rust output.

selfcheck (the default) -- the fork is faithful up to the boundary. Every item the frozen M5c runner runs in
            tools/reference/m5c_reference.py (the M5a rooms, the M4b scripts, the M5b rooms, CHEATS and the M5c rooms)
            and the T4 item TRUNC (tests/reference/m5c_t4) is run by the frozen M5c runner (m5c_build, strict) and by the
            M6a runner (strict, --game-mode registered: the M5c driver's hard-coded mode) with the M5c recommended switches,
            at screenblocks 10 (dumps, frames, sound, view) and 11 (frames, sound). The M6a streams with their declared
            additions removed (frame rows G and Y, sound rows LV, the #schema-m6a, #config-m6a and #columns lines of G and
            Y) must begin with the frozen stream row for row, and equal it with its end rows when both runs end alike
            (frame streams normalised as m5c_reference compares its variants: a tracker-flagged frame's digests, melt
            pictures included, are "undefined"; a raw difference is printed);
            the frozen dump set must be the first snapshots of the M6a dump set, the frozen view stream a prefix of the
            M6a one. Where the frozen runner stops at an exit or a reborn request, the M6a runner carries on (the rows it
            adds are counted). The normalised M6a streams (m5c_reference.normalise) must also begin with, or equal, the
            committed M5c goldens (tests/reference/m5c_frames, tests/reference/m5c_sound). Results:
            tests/reference/m6a_selfcheck.json (DOOM-M6A-SELFCHECK-v1; counts, pass/fail facts and digests of genuine
            streams), written by a plain run and compared by --check (which writes nothing).
--selftest  plants faults in the M6a run of BARON1 at screenblocks 10 and requires the self-check comparison to catch
            each at its place: a started sound's volume (--selftest-sound 10: one S row), an extra P_Random draw
            (--selftest-extra-draw 5: the dumps from tic 5) and a flipped pixel (--selftest-pixel 20 100 100: the F row
            of tic 20).
--variants  runs the given demos (--demo, repeatable) on the given WADs (--wad, repeatable, the IWAD first) in the
            declared --game-mode with the strict, overflow, address and (when a -m32 toolchain exists) strict32 builds
            at screenblocks 10 with dumps, frames, sound, session and view streams, and requires every stream and the
            dump set to be identical across the builds and no sanitizer report. Inputs may be scratch files (the
            synthetic multi-map WAD generator is the next step); nothing is written to the repository.
Address runs use ASAN_OPTIONS=detect_leaks=0: leak detection is disabled, so they are not LeakSanitizer coverage.

    m6a_reference.py [--check] [--selftest] [--only NAME[,NAME]] [--work DIR] [--jobs N] [--cc CC]
    m6a_reference.py --variants --game-mode registered|retail --wad IWAD [--wad PWAD ...] --demo DEMO.lmp [...] [--melt-clock unit]
"""
import argparse, concurrent.futures as cf, hashlib, json, os, re, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m3h9c_build as base    # noqa: E402
import m3h9c_dump as dump     # noqa: E402
import m5c_build              # noqa: E402
import m5c_reference as M5C   # noqa: E402  (read-only: items, normalise, wads)
import m5b_reference as B     # noqa: E402  (read-only: bespoke arguments)
import m5a_reference as A     # noqa: E402  (read-only: sanitizer report)
import m6a_build              # noqa: E402

RESULTS = ROOT / 'tests/reference/m6a_selfcheck.json'
SCHEMA = 'DOOM-M6A-SELFCHECK-v1'
T4 = ROOT / 'tests/reference/m5c_t4'
ENV = dict(os.environ, ASAN_OPTIONS='detect_leaks=0', UBSAN_OPTIONS='print_stacktrace=1')
FRAME_ADDED = ('G', 'Y')
SOUND_ADDED = ('LV',)
HEADER_ADDED = ('#schema-m6a,', '#config-m6a,', '#columns,G,', '#columns,Y,')
sha = lambda b: hashlib.sha256(b).hexdigest()


def items(only=None):
    out = M5C.items()
    out.append(dict(lump='TRUNC', name='TRUNC', wad=T4 / 'truncate.wad', lmp=T4 / 'TRUNC.lmp', kind='m5c_t4'))
    if only:
        want = {x.upper() for x in only.split(',')}
        out = [i for i in out if i['lump'] in want or i['name'].upper() in want]
    return out


def run(binary, item, out, extra, dumps=False, view=False, session=False, m6a=True):
    """One runner process: dict(code, end, stderr, frames, sound, view, session, per_snapshot, stdout)."""
    if out.exists(): shutil.rmtree(out)
    out.mkdir(parents=True)
    wads = item.get('wads') or [item['wad'], M5C.GFX, M5C.SFX, M5C.MUSIC, item['lmp']]
    cmd = [str(binary), *[x for w in wads for x in ('--wad', str(w))], '--demo', item['lmp'].stem.upper(),
           *(['--out', str(out / 'dumps')] if dumps else ['--no-dump']), *B.bespoke_args(item), *extra,
           '--frames', str(out / 'frames.txt'), '--sound-out', str(out / 'sound.txt')]
    if dumps: (out / 'dumps').mkdir()
    if view: cmd += ['--view', str(out / 'view.txt')]
    if m6a: cmd += ['--game-mode', item.get('mode', 'registered')] + (['--session-out', str(out / 'session.txt')] if session else [])
    p = subprocess.run(cmd, capture_output=True, text=True, env=ENV)
    rd = lambda n: (out / n).read_text() if (out / n).exists() else ''
    per = [sha((out / 'dumps' / n).read_bytes()) for n in dump.snapshot_names(str(out / 'dumps'))] if dumps else []
    r = dict(code=p.returncode, summary=A.SUMMARY.search(p.stdout), stderr=p.stderr, stdout=p.stdout, frames=rd('frames.txt'),
             sound=rd('sound.txt'), view=rd('view.txt'), session=rd('session.txt'), per_snapshot=per)
    r['end'] = M5C.end_of(r)             # as m5c_reference: refused (O row), sanitizer, error (E row), the summary's end
    shutil.rmtree(out)
    return r


def strip_added(text, kinds):
    return ''.join(l for l in text.splitlines(keepends=True)
                   if l.split(',', 1)[0] not in kinds and not l.startswith(HEADER_ADDED))


def is_end_row(line):
    f = line.rstrip('\n').split(',')
    return f[0] == 'Z' or (f[0] == 'W' and len(f) > 1 and not f[1].lstrip('-').isdigit())


def prefix_check(frozen, m6a, full):
    """frozen's rows (without its end rows unless full) must be m6a's first rows; with full the streams must be equal.
    Returns dict(rows, equal, first_difference, rows_after)."""
    a = frozen.splitlines()
    b = m6a.splitlines()
    if not full:
        a = [l for l in a if not is_end_row(l)]
        b = [l for l in b if not is_end_row(l)]
    first = next((i for i, (x, y) in enumerate(zip(a, b)) if x != y), None)
    if first is None and len(b) < len(a): first = len(b)
    if full and first is None and len(a) != len(b): first = min(len(a), len(b))
    return dict(rows=len(a), equal=first is None, first_difference=None if first is None else
                [first, a[first] if first < len(a) else None, b[first] if first < len(b) else None], rows_after=max(0, len(b) - len(a)))


def check_item(item, bins, frozen, runs, extra_m6a=()):
    """Every run and comparison of one item: (record, failures)."""
    L = item['lump']; rec = dict(kind=item['kind']); fails = []
    for sb in (10, 11):
        sw = ['--render', 'every', '--screenblocks', str(sb)]
        fz = run(frozen, item, runs / f'{L}_{sb}_frozen', sw, dumps=sb == 10, view=sb == 10, m6a=False)
        m6 = run(bins['strict'], item, runs / f'{L}_{sb}_m6a', sw + list(extra_m6a), dumps=sb == 10, view=sb == 10, session=sb == 10)
        alike = fz['end'] == m6['end']
        last = M5C.last_tic_of(fz)
        v = dict(frozen_end=fz['end'], frozen_last_tic=last, m6a_end=m6['end'], m6a_last_tic=M5C.last_tic_of(m6))
        mf, ms = strip_added(m6['frames'], FRAME_ADDED), strip_added(m6['sound'], SOUND_ADDED)
        # frames as m5c_reference compares its variants: a tracker-flagged frame's whole, face-zeroed and melt digests
        # are "undefined" (they hold bytes read past a source block, which differ with the memory layout; decision R4)
        fzf, mff = M5C.normalise(fz['frames'])[1], M5C.normalise(mf)[1]
        if fz['end'].startswith('refused'):
            # the refused tic's O row and I_Error text hold values read past a renderer array, which vary run to run
            # (docs/M5C_REFERENCE_INVESTIGATION.md section 3.1.4): compared through the tic before, and on the end kind
            cut = lambda t: M5C.truncate(t, last - 1, False)
            v['compared_through_tic'] = last - 1
            v['frames'] = prefix_check(cut(fzf), cut(mff), True)
            v['sound'] = prefix_check(cut(fz['sound']), cut(ms), True)
            cf_, cs_ = cut(fzf), cut(fz['sound'])
            if not alike: fails.append(f'{L} sb{sb}: the frozen run ends {fz["end"]}, the m6a run {m6["end"]}')
        else:
            v['frames'] = prefix_check(fzf, mff, alike)
            v['sound'] = prefix_check(fz['sound'], ms, alike)
            cf_, cs_ = fzf, fz['sound']
        v['frozen_frames_sha256'], v['frozen_sound_sha256'] = sha(cf_.encode()), sha(cs_.encode())   # the compared frozen text
        raw_equal = prefix_check(fz['frames'], mf, alike)['equal']   # printed, not required or recorded (decision R4 above)
        if not raw_equal: print(f'{L} sb{sb}: raw frame streams differ (tracker-flagged or refused-tic values); normalised equal: '
                                f"{v['frames']['equal']}", flush=True)
        n = len(fz['per_snapshot'])
        if sb == 10:
            v['view'] = prefix_check(fz['view'], m6['view'], alike)
            v['dumps'] = dict(snapshots=n, equal=fz['per_snapshot'] == m6['per_snapshot'][:n] and (not alike or len(m6['per_snapshot']) == n),
                              snapshots_after=len(m6['per_snapshot']) - n)
            v['session_tics'] = sum(1 for l in m6['session'].splitlines() if l.startswith('G,'))
            v['session_loads'] = sum(1 for l in m6['session'].splitlines() if l.startswith('L,'))
        # the committed M5c goldens (items of m5c_reference only), truncated as m5c_reference.run_item made them: at their
        # own last tic (the earliest end over the variants), with the strict run's end rows
        gpath, spath = M5C.FRAMES_DIR / f'{L}.sb{sb}.frames', M5C.SOUND_DIR / f'{L}.sound'
        if gpath.exists():
            for key, gold, mine, kind in (('golden_frames', gpath, M5C.normalise(mf)[0], 'F'), ('golden_sound', spath, ms, 'R')):
                if key == 'golden_sound' and sb == 11: continue
                gt = gold.read_text()
                glast = max([int(l.split(',')[1]) for l in gt.splitlines() if l.startswith(kind + ',')], default=-1)
                v[key] = prefix_check(gt, M5C.truncate(mine, glast, alike), alike)
        for k in ('frames', 'sound', 'view', 'dumps', 'golden_frames', 'golden_sound'):
            if v.get(k) is not None and not v[k]['equal']:
                fails.append(f"{L} sb{sb}: {k} differ from the frozen M5c runner: {v[k].get('first_difference')}")
        for r, who in ((fz, 'frozen'), (m6, 'm6a')):
            if r['end'].startswith('sanitizer'): fails.append(f'{L} sb{sb}: {who} {r["end"]}')
        od = re.search(r'^m6a: .* other_draws (\d+) ', m6['stdout'], re.M)
        if od and od[1] != '0': fails.append(f'{L} sb{sb}: m6a other_draws {od[1]}')
        rec[f'sb{sb}'] = v
    return rec, fails


def build_all(work, cc, variants=('strict',), m32=False):
    with open(os.devnull, 'w') as q:
        so, sys.stdout = sys.stdout, q
        try:
            bins = {v: m6a_build.build(v, work / 'build', cc) for v in list(variants) + (['strict32'] if m32 else [])}
            frozen = m5c_build.build('strict', work / 'm5cbuild', cc) if variants == ('strict',) else None
        finally: sys.stdout = so
    return bins, frozen


def selftest(bins, frozen, work):
    item = items('BARON1')[0]
    out = []
    for name, extra, want in (('started volume', ['--selftest-sound', '10'], 'sound'),
                              ('extra P_Random draw', ['--selftest-extra-draw', '5'], 'dumps'),
                              ('flipped pixel', ['--selftest-pixel', '20', '100', '100'], 'frames')):
        rec, fails = check_item(item, bins, frozen, work / 'self', extra)
        v = rec['sb10']
        caught = not v[want]['equal']
        detail = v[want].get('first_difference') if want != 'dumps' else f"dump sets differ: {caught}"
        if want == 'sound': caught = caught and detail and detail[1] and detail[1].startswith('S,') and int(detail[1].split(',')[1]) >= 10
        if want == 'frames': caught = caught and detail and detail[1] and detail[1].startswith('F,20,')
        out.append((name, f'{" ".join(extra)} on BARON1: {want} first difference {detail}', bool(caught)))
    return out


def variants(a, work):
    bins, _ = build_all(work, a.cc, tuple(base.VARIANTS), base.probe_m32(a.cc))
    bins = {k: v for k, v in bins.items() if v}
    ok = True
    for d in a.demo:
        demo = Path(d).resolve()
        item = dict(lump=demo.stem.upper(), wads=[Path(w).resolve() for w in a.wad] + [demo], lmp=demo, mode=a.game_mode)
        extra = ['--render', 'every', '--screenblocks', '10'] + (['--melt-clock', a.melt_clock] if a.melt_clock else [])
        res = {v: run(b, item, work / 'var' / f'{item["lump"]}_{v}', extra, dumps=True, view=True, session=True) for v, b in bins.items()}
        s = res['strict']
        same = {v: all(r[k] == s[k] for k in ('frames', 'sound', 'view', 'session', 'per_snapshot')) and r['end'] == s['end']
                for v, r in res.items() if v != 'strict'}
        san = {v: r['end'] for v, r in res.items() if r['end'].startswith('sanitizer')}
        cnt = lambda t, k: sum(1 for l in t.splitlines() if l.startswith(k + ','))
        m6 = s['stdout'].split('m6a:', 1)[-1].strip().split('\n')[0]
        print(f"{item['lump']}: end {s['end']}; {cnt(s['session'], 'G')} session tics, {cnt(s['session'], 'L')} loads, "
              f"{cnt(s['frames'], 'F')} level frames, {cnt(s['frames'], 'G')} other frames, {cnt(s['frames'], 'Y')} melts, "
              f"{cnt(s['sound'], 'S')} sound starts, {len(s['per_snapshot'])} dumps; identical to strict: {same}; sanitizer {san or 'none'}; m6a: {m6}")
        ok = ok and all(same.values()) and not san
    print(f"variants: {'all identical, no sanitizer report' if ok else 'FAILED'} ({', '.join(bins)}; ASAN_OPTIONS=detect_leaks=0, "
          'not LeakSanitizer coverage)')
    if not ok: sys.exit(1)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch/doom-m6a-reference')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--only')
    ap.add_argument('--jobs', type=int, default=2)
    ap.add_argument('--cc', default='cc')
    ap.add_argument('--variants', action='store_true')
    ap.add_argument('--wad', action='append', default=[])
    ap.add_argument('--demo', action='append', default=[])
    ap.add_argument('--game-mode', choices=['registered', 'retail'])
    ap.add_argument('--melt-clock', choices=['counter', 'unit'])
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    if a.variants:
        if not (a.wad and a.demo and a.game_mode): ap.error('--variants needs --wad, --demo and --game-mode')
        return variants(a, work)
    bins, frozen = build_all(work, a.cc)
    failures, results = [], {}
    todo = items(a.only) if (a.check or a.only or not a.selftest) else []
    with cf.ThreadPoolExecutor(max(1, a.jobs)) as pool:
        futs = [pool.submit(check_item, i, bins, frozen, work / 'runs') for i in todo]
        for item, fut in zip(todo, futs):
            rec, fails = fut.result()
            failures += fails
            results[item['lump']] = dict(wad=item['wad'].relative_to(ROOT).as_posix(), lmp_sha256=sha(item['lmp'].read_bytes()), **rec)
            v = rec['sb10']
            print(f"{item['lump']}: frozen {v['frozen_end']}@{v['frozen_last_tic']} m6a {v['m6a_end']}@{v['m6a_last_tic']}; rows "
                  f"frames {v['frames']['rows']}/{rec['sb11']['frames']['rows']} sound {v['sound']['rows']} view {v['view']['rows']} "
                  f"dumps {v['dumps']['snapshots']}: equal {all(rec[s][k]['equal'] for s in ('sb10', 'sb11') for k in ('frames', 'sound'))} "
                  f"{v['view']['equal']} {v['dumps']['equal']}; m6a rows past the frozen end {v['frames']['rows_after']}", flush=True)
    if failures: sys.exit('FAILED:\n  ' + '\n  '.join(failures))
    if todo and not a.only:
        tot = lambda f: sum(f(r) for r in results.values())
        doc = dict(schema=SCHEMA, runner_contract_sha256=sha(m6a_build.CONTRACT.read_bytes()),
                   m5c_runner_contract_sha256=sha(m5c_build.CONTRACT.read_bytes()),
                   tools_sha256={f'tools/reference/{n}': sha(base.text(ROOT / 'tools/reference' / n).encode('latin-1'))
                                 for n in ('m6a_build.py', 'm6a_session.c', 'm6a_wi_access.c', 'm6a_reference.py')},
                   switches=dict(M5C.RECOMMENDED, **{'--game-mode': 'registered'}), views=[10, 11],
                   claim='the M6a runner (strict) reproduces the frozen M5c runner (strict) row for row through the frozen '
                         'run\'s last row, and with its end rows where both end alike, on every item listed; and its normalised '
                         'streams reproduce the committed M5c goldens the same way',
                   totals=dict(items=len(results),
                               frame_rows=tot(lambda r: r['sb10']['frames']['rows'] + r['sb11']['frames']['rows']),
                               sound_rows=tot(lambda r: r['sb10']['sound']['rows'] + r['sb11']['sound']['rows']),
                               view_rows=tot(lambda r: r['sb10']['view']['rows']), dump_snapshots=tot(lambda r: r['sb10']['dumps']['snapshots']),
                               items_ending_alike=tot(lambda r: int(r['sb10']['frozen_end'] == r['sb10']['m6a_end'])),
                               items_continued_past_frozen_end=tot(lambda r: int(r['sb10']['frozen_end'] != r['sb10']['m6a_end']))),
                   items=results)
        text = json.dumps(doc, indent=1) + '\n'
        if a.check:
            if not RESULTS.exists() or RESULTS.read_text() != text:
                out = work / 'regenerated' / RESULTS.name; out.parent.mkdir(parents=True, exist_ok=True); out.write_text(text)
                sys.exit(f'{RESULTS.relative_to(ROOT)} differs (regenerated text in {out})')
            print(f'{RESULTS.relative_to(ROOT)} matches ({len(results)} items)')
        else:
            RESULTS.write_text(text); print(f'wrote {RESULTS.relative_to(ROOT)} ({len(results)} items)')
    if a.selftest:
        outcomes = selftest(bins, frozen, work)
        for what, detail, ok in outcomes: print(f"selftest {what}: {detail}: {'pass' if ok else 'FAIL'}")
        if not all(ok for _, _, ok in outcomes): sys.exit(1)


if __name__ == '__main__':
    main()
