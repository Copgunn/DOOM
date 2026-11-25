#!/usr/bin/env python3
"""What the genuine engine does when the finale's first frame, the text crawl (F_TextWrite, f_finale.c:261-327), cannot
be drawn (the M6a part 2 follow-up of 2026-11-25). From the genuine linuxdoom-1.10 runner only, never from Rust output.

Each item is part 1's FINREG (the FINALE demo: E1M8 into the finale, registered mode) on its WAD set with one WAD changed
in memory: the directory name of each listed lump gets '_' as its first byte, so W_CheckNumForName no longer finds it
(w_wad.c:376-389) and no lump moves (each lies outside every marker range):

  NOFLAT   FLOOR4_8 in m6a_wads/m6a_gfx.wad, the crawl's flat (f_finale.c:119, 274)
  NOGLYPH  STCFN079 ('O', the first character of E1TEXT) in m5b_gfx.wad
  NOFONT   all 63 font patches STCFN033..STCFN095 in m5b_gfx.wad (HU_Init, hu_stuff.c:404-410)

The strict and address builds run each item at screenblocks 10 (address with ASAN_OPTIONS=detect_leaks=0: leak detection
disabled, so not LeakSanitizer coverage) and must write the same streams. The golden projections of m6a_goldens (session,
frames, sound) of every run must be, line for line, the first lines of FINREG's committed goldens followed only by the
stop's E row. Written to tests/reference/m6a2_victory_stops/stops.json (DOOM-M6A2-VICTORY-STOPS-v1): per item the changed
WAD and its digest, the stop (the I_Error text and the runner's tic of it, -1 before tic 0), the tics run (session G rows)
and the lines of each FINREG golden matched.

    m6a2_victory_stops.py [--check] [--selftest] [--work DIR] [--cc CC]

--check writes nothing to the repository. --selftest proves the comparisons can fail: FINREG's frames golden with one row
changed (in memory) must make the prefix comparison fail, and a record with the stop text changed must differ from the
committed file.
"""
import argparse, hashlib, json, struct, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m6a_build                  # noqa: E402
import m6a_goldens as G           # noqa: E402  (read-only: items, run, ENV, golden projections)
import m6a_wad as W               # noqa: E402

OUT = ROOT / 'tests/reference/m6a2_victory_stops/stops.json'
FONT = [f'STCFN{c:03d}' for c in range(33, 96)]
ITEMS = [('NOFLAT', f'{W.WAD_DIR}/m6a_gfx.wad', ['FLOOR4_8']), ('NOGLYPH', W.GFX_M5B, ['STCFN079']),
         ('NOFONT', W.GFX_M5B, FONT)]
STREAMS = [('session', lambda t: t, 'm6a_session/FINREG.session'), ('frames', G.golden_frames, 'm6a_frames/FINREG.sb10.frames'),
           ('sound', G.golden_sound, 'm6a_sound/FINREG.sound')]
sha = lambda b: hashlib.sha256(b).hexdigest()


def renamed(path, names):
    """The WAD at `path` with '_' as the first byte of every directory entry named in `names`."""
    b = bytearray((ROOT / path).read_bytes())
    n, off = struct.unpack_from('<ii', b, 4)
    found = set()
    for k in range(n):
        o = off + 16 * k + 8
        nm = bytes(b[o:o + 8]).split(b'\0')[0].decode('latin-1')
        if nm in names: b[o] = ord('_'); found.add(nm)
    assert found == set(names), set(names) - found
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
    first = None
    rec, problems = dict(schema='DOOM-M6A2-VICTORY-STOPS-v1', generator='tools/reference/m6a2_victory_stops.py',
                         base_item='FINREG', base_lmp_sha256=sha(base['lmp_path'].read_bytes()), items={}), []
    for name, path, names in ITEMS:
        wad = renamed(path, names)
        (work / f'{name}.wad').write_bytes(wad)
        item = dict(base, name=name, wads=[work / f'{name}.wad' if w == ROOT / path else w for w in base['wads']])
        runs = {v: G.run(b, item, work / f'{name}_{v}', 10) for v, b in bins.items()}
        texts = {k: runs['strict'][k] for k, _p, _g in STREAMS}
        if any({k: r[k] for k in texts} != texts or r['end'] != runs['strict']['end'] for r in runs.values()):
            problems.append(f'{name}: the builds differ')
        r, p = compare(name, texts, golden)
        problems += p
        first = first or texts
        rec['items'][name] = dict(wad=path, renamed=names if len(names) < 4 else f'{names[0]}..{names[-1]}',
                                  wad_sha256=sha(wad), end=runs['strict']['end'], **r)
    return rec, golden, first, problems


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--selftest', action='store_true')
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
        bad_prefix = bool(compare('NOFLAT', first, changed)[1]) and not compare('NOFLAT', first, golden)[1]
        bad_record = same(text) and not same(text.replace('FLOOR4_8 not found', 'FLOOR4_9 not found'))
        for what, ok in (('a changed FINREG frame row is caught', bad_prefix), ('a changed stop text is caught', bad_record)):
            print(f"selftest {what}: {'pass' if ok else 'FAIL'}")
        if not (bad_prefix and bad_record): sys.exit(1)
    if problems: sys.exit('\n'.join(problems))
    if a.check:
        if not same(text): sys.exit(f'differs: {OUT.relative_to(ROOT)}')
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(text)
    for n, r in rec['items'].items():
        print(f"{n}: {r['end']} at runner tic {r['stop_tic']} after {r['tics_run']} tics; lines equal to FINREG's goldens: "
              f"{r['session_lines_equal_to_finreg']}, {r['frames_lines_equal_to_finreg']}, {r['sound_lines_equal_to_finreg']}")


if __name__ == '__main__':
    main()
