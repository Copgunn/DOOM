#!/usr/bin/env python3
"""The block-walk overshoot item against the genuine engine: DSIM43 state digests and the observed walks (certificate
audit 1, item 4). Expected values come from the genuine linuxdoom-1.10 runner and the independent encoder, never from
Rust output.

The runner and its strict build are tools/reference/wire27_goldens.py's (imported read-only: the M6a runner, the
strict -fwrapv flags, the run and the DSIM43 projection by tools/reference/level_snapshot_wire27.py). Two more builds of
the same runner, each from a scratch copy of the sources edited by this tool, never in the checkout; and the
overflow, address and witness builds of wire27_goldens.py unchanged (the address build runs with
ASAN_OPTIONS=detect_leaks=0: leak detection disabled, not LeakSanitizer coverage):

  observer  strict plus prints to stderr, without changing a step: every P_PathTraverse walk (gametic, flags, the
            nudged trace, the cell of every iteration, the line intercepts) and every P_SlideMove's best line and
            fraction. Its dumps, sound and session streams must equal strict's.
  fault     strict with a planted fault in the walk: the loop stops before a cell past the trace's end cell, the
            reach-bounded walk this audit asks about. It must differ from strict, so the item depends on the overshoot.

Per item of tools/reference/overshoot_wad.py: strict, overflow, address, witness and observer give identical dumps,
sound and session streams and a clean exit; the strict and fault dumps are projected by the independent DSIM43 encoder
(wire27_goldens.project); the witness build classifies every P_InterceptVector call (wire27_goldens.witness_facts),
which gives the expected ends: wire 26 refuses the first tic with a wrapping call, wire 27 (and wire 28, whose intercept
rule is wire 27's) the first tic with a call whose class it refuses for its caller. The observer must show, at the
item's tic, a slide walk whose cells pass its end cell (the overshoot) and the far line it meets.

Output (written without --check; --check regenerates and compares, writing nothing): tests/reference/overshoot_state.txt,
DOOM-OVERSHOOT-STATE-v1 (rows in its header). Tic convention as wire27_goldens.py: capture j is the state after j tics.

    overshoot_goldens.py [--check] [--work DIR] [--cc CC]
"""
import argparse, hashlib, shutil, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import wire27_goldens as G          # noqa: E402  (read-only: build, run, project, the strict flags)
import overshoot_wad as WAD         # noqa: E402

STATE, STATE_SCHEMA = ROOT / 'tests/reference/overshoot_state.txt', 'DOOM-OVERSHOOT-STATE-v1'
OBSERVE = [  # (marker, text inserted before it) in p_maputl.c and p_map.c
    ('p_maputl.c', '    for (count = 0 ; count < 64 ; count++)\n',
     '    fprintf (stderr, "walk %d %d %d %d %d %d %d %d", gametic, flags, trace.x, trace.y, trace.dx, trace.dy,'
     ' xt2, yt2);\n'),
    ('p_maputl.c', '\tif (flags & PT_ADDLINES)\n', '        fprintf (stderr, " %d %d", mapx, mapy);\n'),
    ('p_maputl.c', '    return P_TraverseIntercepts ( trav, FRACUNIT );\n',
     '    { intercept_t *in; fprintf (stderr, " :");\n'
     '      for (in = intercepts; in < intercept_p; in++)\n'
     '        if (in->isaline) fprintf (stderr, " %ld %d", (long)(in->d.line - lines), in->frac);\n'
     '      fputc (\'\\n\', stderr); }\n'),
    ('p_map.c', '    // move up to the wall\n',
     '    fprintf (stderr, "best %d %ld %d\\n", gametic, bestslideline ? (long)(bestslideline - lines) : -1L,'
     ' bestslidefrac);\n'),
]
FAULT = ('p_maputl.c', '\tif (flags & PT_ADDLINES)\n',
         '\tif ((mapxstep && (mapx - xt2) * mapxstep > 0) || (mapystep && (mapy - yt2) * mapystep > 0))\n'
         '\t    break;\t// planted fault: never a cell past the end cell\n')


def edited(sources, edits):
    out = dict(sources)
    for f, marker, add in edits:
        if out[f].count(marker) != 1: sys.exit(f'edit FAILED: {f}: {marker!r}')
        head = '#include <stdio.h>\n#include "doomstat.h"\n' if not out[f].startswith('#include <stdio.h>') else ''
        out[f] = head + out[f].replace(marker, add + marker)
    return out


def build(kind, out, cc):
    """strict (wire27_goldens.build unchanged), observer or fault: the strict build of edited scratch sources."""
    if kind == 'strict': return G.build('strict', out, cc)
    plain = G.sources
    G.BUILDS[kind] = G.BUILDS['strict']
    G.sources = lambda _b: edited(plain('strict'), OBSERVE if kind == 'observer' else [FAULT])
    try:
        return G.build(kind, out, cc)
    finally:
        G.sources = plain
        del G.BUILDS[kind]


def item(name):
    m = WAD.ITEMS[name][0]
    return dict(name=name, map=m, wads=[ROOT / w for w in WAD.WADSET], lmp_path=ROOT / f'{WAD.SCRIPT_DIR}/{name}.lmp')


def walks(stderr):
    """[(tic, flags, (x, y, dx, dy), end cell, [cells], [(line, frac)])] and [(tic, line, frac)] of the observer."""
    out, best = [], []
    for row in stderr.splitlines():
        if row.startswith('best '):
            best.append(tuple(int(v) for v in row.split()[1:])); continue
        if not row.startswith('walk '): continue
        head, tail = row.split(' :')
        v = [int(x) for x in head.split()[1:]]
        cells, lines = v[8:], [int(x) for x in tail.split()]
        out.append((v[0], v[1], tuple(v[2:6]), tuple(v[6:8]), list(zip(cells[::2], cells[1::2])),
                    list(zip(lines[::2], lines[1::2]))))
    return out, best


def overshoot(w, wad_item):
    """The item's overshooting slide walk (module text): its row facts and the far line it meets."""
    tic = wad_item['tic']
    found = [x for x in w if x[0] == tic and x[1] == 1 and len(x[4]) == 64
             and any((c[0] - x[3][0]) * (1 if x[3][0] >= x[4][0][0] else -1) > 0 for c in x[4])]
    if len(found) != 1: sys.exit(f"{wad_item['name']}: {len(found)} overshooting walks at tic {tic}")
    tic, _f, (x, y, dx, dy), end, cells, lines = found[0]
    return dict(tic=tic, trace=(x, y, dx, dy), end=end, visits=len(cells), distinct=len(set(cells)), first=cells[0],
                last=cells[-1], lines=lines)


def item_record(name, bins, work):
    it, fails = item(name), []
    runs = {b: G.run(bins[b], it, work / 'runs' / f'{name}_{b}', witness=b == 'witness') for b in bins}
    s = runs['strict']
    tics = len(s['snapshots']) - 1
    if s['code'] or not s['marker'] or tics != WAD.S.lump(WAD.script_text(name))[1]: fails.append(f'{name}: strict run')
    stops = []
    for b in ('overflow', 'address', 'witness', 'observer'):
        r, m = runs[b], G.UB.search(runs[b]['stderr'])
        if m and b in ('overflow', 'address'):   # a sanitizer stop: the strict run's prefix, at a declared site
            stops.append((b, f"{m['file']}:{m['line']}", len(r['snapshots']) - 1))
            if r['snapshots'] != s['snapshots'][:len(r['snapshots'])]: fails.append(f'{name}: {b} stop is not a prefix')
        elif r['code'] or (r['snapshots'], r['sound'], r['session']) != (s['snapshots'], s['sound'], s['session']):
            fails.append(f'{name}: {b} exit {r["code"]} or streams differ from strict')
    if [x[1:] for x in stops] != [WAD.ITEMS[name][6]] * len(stops) or len(stops) not in (0, 2):
        fails.append(f'{name}: sanitizer stops {stops}, declared {WAD.ITEMS[name][6]}')
    caps, mapg = G.project(s, it)
    fault, _ = G.project(runs['fault'], it)
    wit = G.witness_facts(it, runs['witness']['witness'])
    walk, best = walks(runs['observer']['stderr'])
    o = overshoot(walk, dict(name=name, tic=WAD.ITEMS[name][5]))
    o['best'] = [b for b in best if b[0] == o['tic']]
    shutil.rmtree(work / 'runs', ignore_errors=True)
    return dict(name=name, map=it['map'], tics=tics, caps=caps, fault=fault, mapg=mapg, wit=wit, o=o, stops=stops,
                fails=fails)


def state_text(recs):
    head = [f'#schema,{STATE_SCHEMA}',
            '#generator,tools/reference/overshoot_goldens.py: the genuine M6a runner (strict build), its observer and '
            'planted-fault builds, the wrap witness build and the independent DSIM43 encoder; never from Rust output',
            '#tics,capture j is the state after j tics; a refusal at session tic N keeps capture N',
            '#state,as tests/reference/wire27_state.txt (wire27_goldens.state_digest)',
            '#columns,I,item,map,tics,captures,wire26_end,wire27_end,map_sha256,sequence_sha256',
            '#columns,O,item,tic,trace_x,trace_y,trace_dx,trace_dy,end_x,end_y,visits,distinct_cells,first_x,first_y,'
            'last_x,last_y (the observed overshooting slide walk, raw fixed_t and BLOCKMAP cells)',
            '#columns,L,item,line,frac (its line intercepts); B,item,line,frac (P_SlideMove best line at that tic)',
            '#columns,W,item,caller,class,intercept_calls (wire27_goldens.CLASSES)',
            '#columns,U,item,build,site,session_tic (a UBSan stop of the overflow or address build: undefined in C, the '
            'strict -fwrapv build wraps; the stopped run is the strict run\'s prefix)',
            '#columns,C,item,capture,state_sha256 (strict); F,item,capture,state_sha256 (the planted-fault build)']
    rows = []
    for r in recs:
        w, o = r['wit'], r['o']
        end26 = 'complete' if w['first_wrap'] is None else f"refused:{w['first_wrap']}"
        seq = sha(b''.join(bytes.fromhex(c[2]) for c in r['caps']))
        rows.append(f"I,{r['name']},{r['map']},{r['tics']},{len(r['caps'])},{end26},{w['end27']},{r['mapg']},{seq}")
        rows.append(','.join(map(str, ['O', r['name'], o['tic'], *o['trace'], *o['end'], o['visits'], o['distinct'],
                                       *o['first'], *o['last']])))
        rows += [f"L,{r['name']},{line},{frac}" for line, frac in o['lines']]
        rows += [f"B,{r['name']},{line},{frac}" for _t, line, frac in o['best']]
        rows += [f"W,{r['name']},{c},{k},{v}" for (c, k), v in sorted(w['counts'].items())]
        rows += [f"U,{r['name']},{b},{site},{tic}" for b, site, tic in r['stops']]
        rows += [f"C,{r['name']},{j},{d}" for j, _t, d in r['caps']]
        rows += [f"F,{r['name']},{j},{d}" for j, _t, d in r['fault']]
    return '\n'.join(head + rows) + '\n'


sha = lambda b: hashlib.sha256(b).hexdigest()


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'overshoot' / 'goldens')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    bins = {b: (build(b, work / 'build', a.cc) if b in ('strict', 'observer', 'fault') else G.build(b, work / 'build', a.cc))
            for b in ('strict', 'overflow', 'address', 'witness', 'observer', 'fault')}
    recs, fails = [], []
    for name in WAD.ITEMS:
        r = item_record(name, bins, work)
        recs.append(r); fails += r['fails']
        first = next((j for (j, _t, a1), (_j, _u, b1) in zip(r['caps'], r['fault']) if a1 != b1), None)
        o = r['o']
        print(f"{name}: {r['tics']} tics; overshoot at tic {o['tic']}: {o['visits']} visits, {o['distinct']} cells "
              f"{o['first']}..{o['last']}, end cell {o['end']}, lines {o['lines']}, best {o['best']}; wire 26 "
              f"{r['wit']['first_wrap']}, wire 27 {r['wit']['end27']}; the fault build differs first at capture {first}")
        if name == 'SLIDE' and first != o['tic'] + 1: fails.append(f'{name}: the planted fault is not detected at its tic')
    if fails: print('\n'.join(['FAILURES:'] + fails)); return 1
    text = state_text(recs)
    cc = subprocess.run([a.cc, '--version'], capture_output=True, text=True).stdout.split('\n')[0]
    print(f'compiler: {cc}; {G.ASAN_NOTE}')
    if a.check:
        ok = STATE.exists() and STATE.read_text() == text
        print(f'{STATE.relative_to(ROOT)} ' + ('matches' if ok else 'differs from the regenerated text'))
        return 0 if ok else 1
    STATE.write_text(text)
    print(f'wrote {STATE.relative_to(ROOT)}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
