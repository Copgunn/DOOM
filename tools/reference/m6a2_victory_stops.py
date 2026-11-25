#!/usr/bin/env python3
"""The genuine linuxdoom-1.10 runner when the finale's text crawl (F_TextWrite, f_finale.c:261-327) cannot be drawn
(2026-11-25), never Rust output. FINREG (the FINALE demo, E1M8 into the finale) with lumps renamed in memory ('_' first,
so W_CheckNumForName misses them, w_wad.c:376-389; nothing moves): NOFLAT FLOOR4_8 (f_finale.c:119, 274); NOGLYPH STCFN079
('O', E1TEXT's first character); NOFONT STCFN033..STCFN095 (HU_Init, hu_stuff.c:404-410). Strict and address builds
(detect_leaks=0: leak detection disabled) must agree; each golden projection (session, frames, sound) must be the first
lines of FINREG's golden, then the stop's E row. Writes tests/reference/m6a2_victory_stops/stops.json; --check writes
nothing; --selftest needs a changed FINREG frame row and a changed stop text caught.  [--work DIR] [--cc CC]
"""
import argparse, hashlib, json, struct, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m6a_build                  # noqa: E402
import m6a_goldens as G           # noqa: E402  (read-only: items, run, golden projections)
import m6a_wad as W               # noqa: E402

OUT = ROOT / 'tests/reference/m6a2_victory_stops/stops.json'
ITEMS = [('NOFLAT', f'{W.WAD_DIR}/m6a_gfx.wad', ['FLOOR4_8']), ('NOGLYPH', W.GFX_M5B, ['STCFN079']),
         ('NOFONT', W.GFX_M5B, [f'STCFN{c:03d}' for c in range(33, 96)])]
STREAMS = [('session', lambda t: t, 'm6a_session/FINREG.session'), ('frames', G.golden_frames, 'm6a_frames/FINREG.sb10.frames'),
           ('sound', G.golden_sound, 'm6a_sound/FINREG.sound')]
sha = lambda b: hashlib.sha256(b).hexdigest()


def renamed(path, names):
    b = bytearray((ROOT / path).read_bytes())
    n, off = struct.unpack_from('<ii', b, 4)
    hit = [o for o in range(off + 8, off + 16 * n, 16) if bytes(b[o:o + 8]).split(b'\0')[0].decode() in names]
    assert len(hit) == len(names), names
    for o in hit: b[o] = ord('_')
    return bytes(b)


def compare(name, texts, golden):
    """(record part, problems) for one run's stream texts against FINREG's golden texts."""
    out, problems, stop = {}, [], None
    for kind, proj, _g in STREAMS:
        lines = proj(texts[kind]).splitlines()
        if lines and lines[-1].startswith('E,'):
            e = lines.pop().split(',', 2)
            if stop not in (None, (int(e[1]), e[2])): problems.append(f'{name}: {kind}: E row {e}')
            stop = (int(e[1]), e[2])
        if golden[kind].splitlines()[:len(lines)] != lines: problems.append(f'{name}: {kind} is not a prefix of FINREG\'s')
        out[f'{kind}_lines_equal_to_finreg'] = len(lines)
    if stop is None: problems.append(f'{name}: no stop')
    tics = sum(1 for l in texts['session'].splitlines() if l.startswith('G,'))
    return dict(out, stop_tic=stop and stop[0], stop=stop and stop[1], tics_run=tics), problems


def record(work, cc):
    golden = {k: (ROOT / 'tests/reference' / g).read_text() for k, _p, g in STREAMS}
    base = next(i for i in G.items() if i['name'] == 'FINREG')
    bins = {v: m6a_build.build(v, work / 'build', cc) for v in ('strict', 'address')}
    rec, problems, first = dict(schema='DOOM-M6A2-VICTORY-STOPS-v1', generator='tools/reference/m6a2_victory_stops.py',
                                base_item='FINREG', base_lmp_sha256=sha(base['lmp_path'].read_bytes()), items={}), [], None
    for name, path, names in ITEMS:
        wad = renamed(path, names)
        (work / f'{name}.wad').write_bytes(wad)
        item = dict(base, name=name, wads=[work / f'{name}.wad' if w == ROOT / path else w for w in base['wads']])
        runs = {v: G.run(b, item, work / f'{name}_{v}', 10) for v, b in bins.items()}
        texts = {k: runs['strict'][k] for k, _p, _g in STREAMS}
        if any({k: r[k] for k in texts} != texts or r['end'] != runs['strict']['end'] for r in runs.values()):
            problems.append(f'{name}: the builds differ')
        r, p = compare(name, texts, golden)
        problems, first = problems + p, first or texts
        rec['items'][name] = dict(wad=path, renamed=names if len(names) < 4 else f'{names[0]}..{names[-1]}',
                                  wad_sha256=sha(wad), end=runs['strict']['end'], **r)
    return rec, golden, first, problems


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    for f in ('--check', '--selftest'): ap.add_argument(f, action='store_true')
    ap.add_argument('--work')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = Path(a.work or tempfile.mkdtemp(prefix='m6a2_victory_stops_')).resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    rec, golden, first, problems = record(work, a.cc)
    text = json.dumps(rec, indent=1) + '\n'
    same = lambda t: OUT.exists() and OUT.read_text().replace('\r\n', '\n') == t
    if a.selftest:
        changed = dict(golden, frames=golden['frames'].replace('\nF,100,', '\nF,100,0', 1))
        res = [('a changed FINREG frame row', bool(compare('NOFLAT', first, changed)[1]) and not compare('NOFLAT', first, golden)[1]),
               ('a changed stop text', same(text) and not same(text.replace('FLOOR4_8 not found', 'FLOOR4_9 not found')))]
        for what, ok in res: print(f"selftest {what} is caught: {'pass' if ok else 'FAIL'}")
        if not all(ok for _w, ok in res): sys.exit(1)
    if problems: sys.exit('\n'.join(problems))
    if a.check and not same(text): sys.exit(f'differs: {OUT.relative_to(ROOT)}')
    if not a.check:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(text)
    [print(f"{n}: {r['end']} at runner tic {r['stop_tic']} after {r['tics_run']} tics") for n, r in rec['items'].items()]


if __name__ == '__main__':
    main()
