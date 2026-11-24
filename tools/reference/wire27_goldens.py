#!/usr/bin/env python3
"""Wire 27 against the genuine engine: DSIM43 state digests and the wrap witness of every item (owner's rulings
of 2026-11-24, R3-R8). Expected values come from the genuine linuxdoom-1.10 runner and the independent encoder, never
from Rust output.

The runner is the M6a runner (tools/reference/m6a_build.py, imported read-only: its generated driver, stubs, sources,
flags and link list), built here by this tool's own build() in five builds under --work:

  strict    the genuine text (with the frozen ABI adapters), -fwrapv: THE CLAIM. Dumps, sound and session streams.
  overflow  -fno-wrapv, UBSan signed overflow and the rest (base.VARIANTS), the frozen sanitizer adapters plus the two
            declared adapters ADAPTERS (R3: p_maputl.c:240 and 246-248, the same statements in unsigned arithmetic,
            the form of the frozen ones), applied by this tool to its own copy of the sources only.
  address   ASan + UBSan with the same adapters; run with ASAN_OPTIONS=detect_leaks=0 (leak detection disabled:
            not LeakSanitizer coverage).
  witness   strict plus wire27_witness.c: p_maputl.c compiled with -finstrument-functions and -Wl,--wrap of FixedMul,
            FixedDiv and P_PathTraverse; one row per P_InterceptVector call (the traverser, PIT_AddLineIntercepts or
            PIT_AddThingIntercepts, every operand and result). Observers only, so its streams must equal strict's.
            (A linker wrap alone cannot see P_InterceptVector: its callers are in its own file, p_maputl.c.)
  nowrap    the overflow build WITHOUT the two adapters: UBSan stops the run at the first int addition or subtraction
            of p_maputl.c:240 or 246-248 that overflows (a FixedMul narrowing is no UB and does not stop it); the
            stop's file:line and session tic must be the witness's first such call, and it must not stop otherwise.

Per item of tools/reference/wire27_wad.py ITEMS (one new-game level of tests/reference/wire27_wads/wire27.wad, the
M6a WAD set, --render none, the M5c recommended switches): strict, overflow, address and witness must give identical
dumps, sound and session streams and a clean exit; the strict dumps are projected by the independent DSIM43 encoder
(level_snapshot_wire27.py; also by the DSIM42 one, and every capture must differ in profile and context.schema only).

The witness classification (module text) of each P_InterceptVector call, from its logged operands: a call WRAPS when a
FixedMul's 64-bit product >> 16 leaves int, or the den subtraction or the num sum of the narrowed products does; cases:
cancel (wrapped, the int num and den equal the exact ones), den_zero (int den 0, exact den not), num_overflow (int num
not the exact num), wrong (wrapped and the int intercept differs from the exact one, trunc(65536 num/den)), saturation
(wrapped operands reach FixedDiv's saturation branch, m_fixed.c:62-63), absmin (an INT_MIN operand). Callers: aim
(PTR_AimTraverse), shoot_line and shoot_thing (PTR_ShootTraverse, by PIT_Add*Intercepts), slide, use. Every logged
FixedMul and FixedDiv result is checked against m_fixed.c's formulas, the den's operand order against FixedDiv's den.
Derived: the expected wire-26 refusal (the first tic with a wrapping call; wires 23-26 refuse every wrap), and the
expected wire-27 end (the first tic with a wrapping saturation, an absmin or a wrap in a caller wire 27 does not wrap,
else complete).

Tic convention (owner): a session tic is the 0-based index of a G_Ticker, equal to the number of tics completed
before it; a refusal at session tic N leaves the state after N tics; capture j is the state after j tics.

Outputs (written without --check; --check regenerates and compares, writing nothing):
  tests/reference/wire27_state.txt    DOOM-WIRE27-STATE-v1: per item an I row (map, tics, the expected ends, the map
                                      digest, the sequence digest), a C row per capture (state digest: the DSIM43
                                      capture without map.* and events.* in level_compare.stripped framing, then its
                                      events.sounds.* fields, as tests/reference/m6a_state.txt) and W rows (the wrapping
                                      calls per caller and case), then T rows (the totals over every item)
  tests/reference/wire27_results.json DOOM-WIRE27-RESULTS-v1: revision, builds and flags, the adapters with digests,
                                      tool and input digests, the witness method, per item every wrapping call, the
                                      nowrap stop, the barrel and puff facts; `generated_with` (the compiler) is
                                      informational and left out of the --check comparison (it is printed instead)

    wire27_goldens.py [--check] [--selftest] [--work DIR] [--cc CC]
"""
import argparse, hashlib, json, os, re, shutil, struct, subprocess, sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m3h9c_build as base          # noqa: E402  (read-only: flags, revision, layout probe, text)
import m3h9c_dump as dump           # noqa: E402
import m5a_build                    # noqa: E402
import m5c_build as C               # noqa: E402
import m5c_reference as M5C         # noqa: E402  (read-only: RECOMMENDED)
import m6a_build as B               # noqa: E402  (read-only: the M6a runner's sources, driver, stubs, units, links)
import m6a_state as ST              # noqa: E402  (read-only: Shifted, extra_lumps)
import level_snapshot as L          # noqa: E402
import level_snapshot_m5c as M5S    # noqa: E402
import level_snapshot_m6a as S6     # noqa: E402
import level_snapshot_wire27 as E   # noqa: E402
import level_compare as K           # noqa: E402
import wire27_wad as WAD            # noqa: E402
from m3h9_geometry import enum_index  # noqa: E402

STATE, STATE_SCHEMA = ROOT / 'tests/reference/wire27_state.txt', 'DOOM-WIRE27-STATE-v1'
RESULTS, SCHEMA = ROOT / 'tests/reference/wire27_results.json', 'DOOM-WIRE27-RESULTS-v1'
WITNESS_C = ROOT / 'tools/reference/wire27_witness.c'
TOOLS = ['wire27_goldens.py', 'wire27_wad.py', 'wire27_witness.c', 'level_snapshot_wire27.py']
ENV = dict(os.environ, ASAN_OPTIONS='detect_leaks=0', UBSAN_OPTIONS='print_stacktrace=1')
ASAN_NOTE = 'detect_leaks=0 (leak detection disabled in the address build; not LeakSanitizer coverage)'
INT_MIN, INT_MAX = -2 ** 31, 2 ** 31 - 1
MT_PUFF, MT_BARREL = enum_index('mobjtype_t', 'MT_PUFF'), enum_index('mobjtype_t', 'MT_BARREL')
CLAIM = ('strict', 'overflow', 'address', 'witness')
# build: (flag set of base.VARIANTS, sources of m6a_build.adapted_sources, the two adapters, the witness observers)
BUILDS = {'strict': ('strict', False, False), 'overflow': ('overflow', True, False), 'address': ('address', True, False),
          'witness': ('strict', False, True), 'nowrap': ('overflow', False, False)}
WITNESS_LINK = ['-Wl,--wrap=FixedMul,--wrap=FixedDiv,--wrap=P_PathTraverse']
WITNESS_FILE_FLAGS = {'p_maputl.c': ['-finstrument-functions']}
REASON = ('P_InterceptVector: the int {what} leaves int on the long-wall traces of the wire 27 items; the original '
          'targets and the strict -fwrapv build wrap it modulo 2^32, as the unsigned {op} does (the form of the frozen '
          'sanitizer adapters, m3h9c_build.SANITIZER_ADAPTERS). Sanitizer builds only (owner\'s ruling R3); the '
          'strict build compiles the genuine text and is the claim.')
ADAPTERS = [
    dict(file='p_maputl.c', lines='240',
         original='    den = FixedMul (v1->dy>>8,v2->dx) - FixedMul(v1->dx>>8,v2->dy);',
         adapted='    den = (fixed_t)((unsigned)FixedMul (v1->dy>>8,v2->dx) - (unsigned)FixedMul(v1->dx>>8,v2->dy));',
         reason=REASON.format(what='difference of the den products', op='difference')),
    dict(file='p_maputl.c', lines='246-248',
         original='    num =\n\tFixedMul ( (v1->x - v2->x)>>8 ,v1->dy )\n\t+FixedMul ( (v2->y - v1->y)>>8, v1->dx );',
         adapted='    num = (fixed_t)((unsigned)\n\tFixedMul ( (v1->x - v2->x)>>8 ,v1->dy )\n'
                 '\t+(unsigned)FixedMul ( (v2->y - v1->y)>>8, v1->dx ));',
         reason=REASON.format(what='sum of the num products', op='sum')
         + ' The coordinate differences stay signed: the Episode 1 spans (under 8,192) keep them inside int.'),
]
UB = re.compile(r'^(?:\S*/)?(?P<file>[\w.]+):(?P<line>\d+):\d+: runtime error: (?P<msg>.*)$', re.M)
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()
wrap32 = lambda v: (v + 2 ** 31) % 2 ** 32 - 2 ** 31
fits = lambda v: INT_MIN <= v <= INT_MAX


def adapter_table():
    return [dict(a, original_sha256=sha(a['original']), adapted_sha256=sha(a['adapted']), builds=['overflow', 'address'])
            for a in ADAPTERS]


def apply(src, a):
    """One declared adapter: the original text must occur exactly once, at its declared lines."""
    first, last = (int(x) for x in (a['lines'] + '-' + a['lines']).split('-')[:2])
    if src.count(a['original']) != 1 or '\n'.join(src.split('\n')[first - 1:last]) != a['original']:
        sys.exit(f"adapter FAILED: {a['file']}:{a['lines']}")
    assert a['adapted'].count('\n') == a['original'].count('\n'), 'an adapter keeps the line count'
    return src.replace(a['original'], a['adapted'])


def sources(build):
    flags, adapt, _ = BUILDS[build]
    out = B.adapted_sources(flags)
    for a in ADAPTERS if adapt else []: out[a['file']] = apply(out[a['file']], a)
    return out


def build(name, out, cc):
    """One build of the M6a runner under out/name (module text)."""
    flags_name, _, witness = BUILDS[name]
    work = out / name
    if work.exists(): shutil.rmtree(work)
    src, obj = work / 'src', work / 'obj'
    src.mkdir(parents=True); obj.mkdir()
    for h in base.SOURCE.glob('*.h'): shutil.copy2(h, src / h.name)
    srcs = sources(name)
    for f, s in srcs.items(): (src / f).write_bytes(s.encode('latin-1'))
    (src / 'layout_probe.c').write_text(base.layout_probe(srcs['r_data.c']))
    (src / 'm5a_types.h').write_text(m5a_build.types_header(srcs))
    (src / B.DRIVER_NAME).write_bytes(B.driver_text().encode('latin-1'))
    (src / B.STUBS_NAME).write_bytes(B.stubs_text().encode('latin-1'))
    for f in (B.SESSION_C, B.WI_ACCESS, WITNESS_C): shutil.copy2(f, src / f.name)
    units = B.UNITS + ['layout_probe.c', B.STUBS_NAME, B.DRIVER_NAME] + ([WITNESS_C.name] if witness else [])
    flags = base.VARIANTS[flags_name]
    for f in units:
        extra = C.FILE_FLAGS.get(f, []) + (C.DRIVER_FLAGS.get(flags_name, []) if f == B.DRIVER_NAME else []) \
            + (WITNESS_FILE_FLAGS.get(f, []) if witness else [])
        p = subprocess.run([cc] + flags + extra + ['-I', str(src), '-c', str(src / f), '-o', str(obj / (f[:-2] + '.o'))],
                           capture_output=True, text=True)
        if p.returncode: sys.stderr.write(p.stderr); sys.exit(f'compile FAILED: {name}: {f}')
    binary = work / 'crunner'
    p = subprocess.run([cc] + flags + B.LINK_FLAGS + (WITNESS_LINK if witness else [])
                       + [str(obj / (f[:-2] + '.o')) for f in units] + C.LIBS + ['-o', str(binary)], capture_output=True, text=True)
    if p.returncode: sys.stderr.write(p.stderr); sys.exit(f'link FAILED: {name}')
    return binary


def items():
    out = {}
    for name, (m, *_rest) in WAD.ITEMS.items():
        out[name] = dict(name=name, map=m, wads=[ROOT / w for w in WAD.WADSET], lmp_path=ROOT / f'{WAD.SCRIPT_DIR}/{name}.lmp')
    return out


def run(binary, item, out, witness=False, extra=()):
    """One runner process with dumps, sound and session streams (and the witness file)."""
    if out.exists(): shutil.rmtree(out)
    (out / 'dumps').mkdir(parents=True)
    cmd = [str(binary), *[x for w in item['wads'] + [item['lmp_path']] for x in ('--wad', str(w))], '--demo', item['name'],
           '--game-mode', 'registered', '--out', str(out / 'dumps'), *[x for k, v in M5C.RECOMMENDED.items() for x in (k, v)],
           '--render', 'none', *extra, '--sound-out', str(out / 'sound.txt'), '--session-out', str(out / 'session.txt')]
    env = dict(ENV, WIRE27_WITNESS=str(out / 'witness.txt')) if witness else ENV
    p = subprocess.run(cmd, capture_output=True, text=True, env=env)
    rd = lambda n: (out / n).read_text() if (out / n).exists() else ''
    names = dump.snapshot_names(str(out / 'dumps'))
    return dict(code=p.returncode, stderr=p.stderr, marker=' end=marker ' in p.stdout, sound=rd('sound.txt'), session=rd('session.txt'), witness=rd('witness.txt'),
                dumps=out / 'dumps', snapshots=[(n, sha((out / 'dumps' / n).read_bytes())) for n in names])


def state_digest(fields):
    body, _ = K.stripped(E.VERSION, fields)
    return sha(body + S6.framed([(p, v) for p, v in fields if p.startswith('events.sounds.')]))


def project(r, item):
    """[(capture, session tic, state digest)] and the map digest of the strict run, by the DSIM43 encoder; every
    capture's DSIM42 projection must differ in profile and context.schema only."""
    session, sound = r['session'], r['sound']
    loads = S6.rows(session, 'L')
    if len(loads) != 1: sys.exit(f"{item['name']}: {len(loads)} loads")
    kind, p, _h, _s = S6.start_from_rows(session, sound, 0, str(item['wads'][0]))
    assert kind == 'new', kind
    mapdata = L.MapData(str(item['wads'][0]), f"E1M{item['map']}", ST.extra_lumps(item))
    skill, offset = L.demo_skill(str(item['lmp_path'])), max(loads[0]['tic'], 0)
    proj = [cls(mapdata, skill, item['map'], ST.Shifted(M5S.SoundStream(text=sound), offset), (kind, dict(p)))
            for cls in (E.Projector, S6.Projector)]
    names = ['load.txt'] + [n for n in sorted(os.listdir(r['dumps'])) if n.startswith('tic_')]
    caps, mapg = [], None
    for j, n in enumerate(names):
        nxt = names[j + 1] if j + 1 < len(names) else None
        f43, f42 = [pr.step(L.Dump(str(r['dumps'] / n)), L.Dump(str(r['dumps'] / nxt)) if nxt else None)[0] for pr in proj]
        E.compare42(f43, f42)
        if mapg is None: mapg = sha(S6.framed([x for x in f43 if x[0].startswith('map.')]))
        caps.append((j, int(n[4:9]) + 1 if n != 'load.txt' else 0, state_digest(f43)))
    if [c[0] for c in caps] != [c[1] for c in caps]: sys.exit(f"{item['name']}: captures are not consecutive tics")
    return caps, mapg


def mobj_facts(r):
    """The barrels' health after the last tic and every puff's first dump (tic, x, y in units)."""
    names = [n for n in sorted(os.listdir(r['dumps'])) if n.startswith('tic_')]
    puffs, seen, last = [], set(), None
    for n in names:
        last = L.Dump(str(r['dumps'] / n))
        for k, m in last.thinkers:
            if k == 'M' and m['type'] == MT_PUFF and last.serials[m['idx']][0] not in seen:
                seen.add(last.serials[m['idx']][0]); puffs.append([int(n[4:9]), m['x'] / 65536, m['y'] / 65536])
    barrels = [m['health'] for k, m in last.thinkers if k == 'M' and m['type'] == MT_BARREL] if last else []
    return dict(barrel_health_end=barrels, puffs=puffs)


def fixeddiv(a, b):
    """FixedDiv (m_fixed.c:57-64) and FixedDiv2's double arithmetic (78-87) on operands that are not INT_MIN."""
    if (abs(a) >> 14) >= abs(b): return INT_MIN if (a ^ b) < 0 else INT_MAX
    return int(float(a) / float(b) * 65536.0)


def map_lines(item):
    """[(v1.x, v1.y, dx, dy)] in 16.16 of the item's map, in LINEDEFS order."""
    data = item['wads'][0].read_bytes()
    count, offset = int.from_bytes(data[4:8], 'little'), int.from_bytes(data[8:12], 'little')
    entry = lambda k: (int.from_bytes(data[offset + 16 * k:offset + 16 * k + 4], 'little'),
                       int.from_bytes(data[offset + 16 * k + 4:offset + 16 * k + 8], 'little'),
                       data[offset + 16 * k + 8:offset + 16 * k + 16].rstrip(b'\0').decode())
    at = next(k for k in range(count) if entry(k)[2] == f"E1M{item['map']}")
    lump = {entry(k)[2]: data[entry(k)[0]:entry(k)[0] + entry(k)[1]] for k in range(at + 1, at + 11)}
    vs = [struct.unpack_from('<hh', lump['VERTEXES'], 4 * i) for i in range(len(lump['VERTEXES']) // 4)]
    ls = [struct.unpack_from('<hh', lump['LINEDEFS'], 14 * i) for i in range(len(lump['LINEDEFS']) // 14)]
    return [(vs[a][0] << 16, vs[a][1] << 16, (vs[b][0] - vs[a][0]) << 16, (vs[b][1] - vs[a][1]) << 16) for a, b in ls]


def classify(row, sites, lines):
    """One witness row (module text) as a dict; `sites` collects the FixedMul call sites in call order, `lines` is
    map_lines() of the item's map (a line call is matched to its linedef by its operands)."""
    f = row.split()
    tic, trav, who, (tx, ty, tdx, tdy), n = int(f[1]), f[2], f[3], [int(x) for x in f[4:8]], int(f[8])
    if n not in (2, 4): raise ValueError(f'a row with {n} products: {row}')
    muls = [tuple(int(x) for x in f[9 + 4 * i:13 + 4 * i]) for i in range(n)]
    rest = f[9 + 4 * n:]
    div = tuple(int(x) for x in rest[1:4]) if rest else None
    if any(wrap32((a * b) >> 16) != res for _s, a, b, res in muls): raise ValueError(f'a FixedMul result: {row}')
    sites.setdefault(n, [m[0] for m in muls])
    if [m[0] for m in muls] != sites[n] or sites.get(2, sites[n][:2]) != sites[n][:2]: raise ValueError(f'call sites: {row}')
    # p_maputl.c:240 is evaluated before 246-248; the order of the operands of its `-` is the compiler's: the minuend
    # FixedMul(v1->dy>>8, v2->dx) is the den product whose b is trace.dx and, with FixedDiv, whose difference is its den
    order = [o for o in ((0, 1), (1, 0)) if muls[o[0]][2] == tdx and muls[o[1]][2] == tdy
             and (not div or wrap32(muls[o[0]][3] - muls[o[1]][3]) == div[1])]
    if not order or len(order) == 2 and muls[0][1:] != muls[1][1:]: raise ValueError(f'the den operands: {row}')
    muls = [muls[order[0][0]], muls[order[0][1]]] + muls[2:]
    wide = [(a * b) >> 16 for _s, a, b, _r in muls]                     # FixedMul's 64-bit product >> 16, m_fixed.c:47
    r = [m[3] for m in muls]
    den_exact, den_int, sub = wide[0] - wide[1], wrap32(r[0] - r[1]), r[0] - r[1]
    if (den_int == 0) != (div is None) or (n == 4) != (div is not None): raise ValueError(f'den 0 and FixedDiv: {row}')
    caller = {'aim': 'aim', 'slide': 'slide', 'use': 'use'}.get(trav, f'shoot_{who}' if trav == 'shoot' else '?')
    if who not in ('line', 'thing') or caller == '?': raise ValueError(f'unknown traverser or caller: {row}')
    line = None
    if who == 'line':      # the den operands are (v1->dy>>8, v1->dx>>8), the num ones exact differences (spans < 8,192)
        hits = [i for i, (x, y, dx, dy) in enumerate(lines) if (muls[0][1], muls[1][1]) == (dy >> 8, dx >> 8)
                and (not div or sorted(m[1:3] for m in muls[2:]) == sorted([((x - tx) >> 8, dy), ((ty - y) >> 8, dx)]))]
        if len(hits) != 1: raise ValueError(f'{len(hits)} linedefs match: {row}')
        line = hits[0]
    x, y, dx, dy = lines[line] if line is not None else (0, 0, 0, 0)
    num_exact = wide[2] + wide[3] if div else ((((x - tx) >> 8) * dy) >> 16) + ((((ty - y) >> 8) * dx) >> 16)
    num_int = wrap32(r[2] + r[3]) if div else None
    if div and div[:2] != (num_int, den_int): raise ValueError(f'FixedDiv operands are not (num, den): {row}')
    wrapped = not all(fits(v) for v in wide + [sub] + ([r[2] + r[3]] if div else []))
    exact = 0 if den_exact == 0 else fixeddiv(num_exact, den_exact) if fits(num_exact) and fits(den_exact) \
        else int(Fraction(65536 * num_exact, den_exact))
    out = dict(tic=tic, caller=caller, line=line, wrapped=wrapped, den_int=den_int, den_exact=den_exact,
               num_int=num_int, num_exact=num_exact if (div or line is not None) else None, frac=div[2] if div else 0,
               frac_exact=exact if (div or line is not None) else None, branch=None, cases=[],
               addsub_overflow=not fits(sub) or bool(div and not fits(r[2] + r[3])))
    if div:
        a, b, res = div
        out['branch'] = 'absmin' if INT_MIN in (a, b) else 'saturation' if (abs(a) >> 14) >= abs(b) else 'div2'
        if out['branch'] != 'absmin' and res != fixeddiv(a, b): raise ValueError(f'a FixedDiv result: {row}')
    cases = out['cases']
    if wrapped:
        if den_int == den_exact and (not div or num_int == num_exact): cases.append('cancel')
        if den_int == 0 and den_exact != 0: cases.append('den_zero')
        if div and num_int != num_exact: cases.append('num_overflow')
        if out['frac_exact'] is not None and out['frac'] != out['frac_exact']: cases.append('wrong')
        if out['branch'] == 'saturation': cases.append('saturation')
    if out['branch'] == 'absmin': cases.append('absmin')
    return out


NOT_WRAPPED = ('shoot_thing',)   # ruling R6: a caller with no genuine wrapping item stays checked under wire 27


def witness_facts(item, text):
    """The classified calls of one witness file: counts, the wrapping calls and the derived ends (module text)."""
    sites, lines = {}, map_lines(item)
    calls = [classify(l, sites, lines) for l in text.splitlines() if l.startswith('V ')]
    if not calls or len(calls) != len(text.splitlines()): raise ValueError(f"{item['name']}: an empty or foreign witness")
    wraps = [c for c in calls if c['wrapped'] or c['cases']]
    stop = next((c for c in calls if 'absmin' in c['cases'] or c['wrapped'] and (
        'saturation' in c['cases'] or c['caller'] in NOT_WRAPPED)), None)
    reason = None if stop is None else 'AbsMin' if 'absmin' in stop['cases'] else \
        'Saturation' if 'saturation' in stop['cases'] else 'checked'
    counts = {}
    for c in wraps:
        for case in c['cases']: counts[(c['caller'], case)] = counts.get((c['caller'], case), 0) + 1
    return dict(calls=len(calls), callers={k: sum(c['caller'] == k for c in calls) for k in WAD.CALLERS},
                first_wrap=min((c['tic'] for c in calls if c['wrapped']), default=None),
                end27='complete' if stop is None else f"refused:{stop['tic']}:{reason}",
                first_addsub=min((c['tic'] for c in calls if c['addsub_overflow']), default=None), counts=counts,
                wraps=[[c[k] for k in ('tic', 'caller', 'line', 'cases', 'den_int', 'den_exact', 'num_int', 'num_exact',
                                       'frac', 'frac_exact', 'branch')] for c in wraps])


def item_record(item, bins, work):
    name, fails = item['name'], []
    runs = {b: run(bins[b], item, work / 'runs' / f'{name}_{b}', witness=b == 'witness') for b in bins}
    s = runs['strict']
    tics = len(s['snapshots']) - 1
    if s['code'] or UB.search(s['stderr']) or not s['marker'] or tics != WAD.S.lump(WAD.script_text(name))[1]:
        fails.append(f'{name}: strict exit {s["code"]}, not at the demo marker or not every tic')
    for b in CLAIM[1:]:
        r = runs[b]
        if r['code'] or r['stderr'] != s['stderr']: fails.append(f'{name}: {b} exit {r["code"]} or stderr not strict\'s')
        if (r['snapshots'], r['sound'], r['session']) != (s['snapshots'], s['sound'], s['session']):
            fails.append(f'{name}: {b} streams differ from strict')
    caps, mapg = project(s, item)
    w = witness_facts(item, runs['witness']['witness'])
    n = runs['nowrap']
    m = UB.search(n['stderr'])
    # the site as the statement: a compiler may place the `+` of 246-248 on any of its lines
    site = None if not m else f"{m['file']}:{'246-248' if m['line'] in ('246', '247', '248') else m['line']}"
    trap = dict(exit=n['code'], site=site, tic=len(n['snapshots']) - 1 if m else None)
    if w['first_addsub'] is None:
        if n['code'] or m or n['snapshots'] != s['snapshots']: fails.append(f'{name}: nowrap stopped without an int overflow')
    elif trap['site'] not in ('p_maputl.c:240', 'p_maputl.c:246-248') or trap['tic'] != w['first_addsub'] \
            or n['snapshots'] != s['snapshots'][:len(n['snapshots'])]:
        fails.append(f'{name}: nowrap stop {trap} is not the witness\'s first int overflow at tic {w["first_addsub"]}')
    _m, _dx, _p, _b, script, callers, cases, _what = WAD.ITEMS[name]
    missing = [c for c in callers if not any(cc == c for cc, _ in w['counts'])] + \
              [k for k in cases if not any(kk == k for _, kk in w['counts'])]
    if missing: fails.append(f'{name}: no witnessed wrapping call for {missing}')
    facts = mobj_facts(s)
    shutil.rmtree(work / 'runs', ignore_errors=True)
    return dict(name=name, map=item['map'], tics=tics, captures=caps, mapg=mapg, witness=w, nowrap=trap, facts=facts,
                fails=fails, lmp_sha256=sha(item['lmp_path'].read_bytes()))


def state_text(recs):
    head = [f'#schema,{STATE_SCHEMA}',
            '#generator,tools/reference/wire27_goldens.py: the genuine M6a runner (strict build) and the independent DSIM43 '
            'encoder tools/reference/level_snapshot_wire27.py; the wrap witness of the witness build; never from Rust output',
            '#tics,a session tic is the 0-based index of a G_Ticker, equal to the number of tics completed before it; '
            'capture j is the state after j tics; a refusal at session tic N keeps capture N',
            '#state,SHA-256 of the DSIM43 capture without map.* and events.* (level_compare.stripped framing) followed by its '
            'events.sounds.* fields (per-field framing); map: SHA-256 of the map.* fields of capture 0',
            '#columns,I,item,map,tics,captures,wire26_refusal_tic,wire27_end,map_sha256,sequence_sha256',
            '#columns,C,item,capture,state_sha256',
            '#columns,W,item,caller,case,wrapping_calls',
            '#columns,T,caller,case,wrapping_calls (every item)']
    rows, total = [], {}
    for r in recs:
        w = r['witness']
        seq = sha(b''.join(bytes.fromhex(c[2]) for c in r['captures']))
        rows.append(f"I,{r['name']},{r['map']},{r['tics']},{len(r['captures'])},{w['first_wrap']},{w['end27']},{r['mapg']},{seq}")
        rows += [f"C,{r['name']},{j},{d}" for j, _t, d in r['captures']]
        for (c, k), v in sorted(w['counts'].items()):
            rows.append(f"W,{r['name']},{c},{k},{v}"); total[(c, k)] = total.get((c, k), 0) + v
    rows += [f'T,{c},{k},{total.get((c, k), 0)}' for c in WAD.CALLERS for k in WAD.CASES + ('absmin',)]
    return '\n'.join(head + rows) + '\n'


def results(recs, cc):
    digest = lambda p: sha((ROOT / p).read_bytes())
    return dict(
        schema=SCHEMA, revision=base.REVISION,
        claim='every state digest is the genuine engine\'s strict build (-fwrapv, the genuine p_maputl.c text) projected by '
              'the independent DSIM43 encoder; the overflow, address and witness builds gave identical dumps, sound and '
              'session streams',
        runner='tools/reference/m6a_build.py (generated driver and stubs, sources, units and link list), built by '
               'wire27_goldens.build; m6a_runner_contract.json unchanged',
        builds={b: dict(flags=base.VARIANTS[f], adapters=['p_maputl.c:240', 'p_maputl.c:246-248'] if a else [],
                        witness=dict(file_flags=WITNESS_FILE_FLAGS, link=WITNESS_LINK) if w else None)
                for b, (f, a, w) in BUILDS.items()},
        sanitizer_adapters=adapter_table(), asan_options=ASAN_NOTE, ubsan_options='print_stacktrace=1',
        witness_method='the witness build (module text): -finstrument-functions on p_maputl.c and -Wl,--wrap of FixedMul, '
                       'FixedDiv and P_PathTraverse (tools/reference/wire27_witness.c); one row per P_InterceptVector call, '
                       'classified by wire27_goldens.classify; cross-checked by the nowrap build\'s UBSan stop',
        not_wrapped_callers=list(NOT_WRAPPED),
        tools_sha256={t: digest(f'tools/reference/{t}') for t in TOOLS},
        inputs_sha256={p: digest(p) for p in WAD.WADSET + [f'{WAD.SCRIPT_DIR}/{n}.lmp' for n in WAD.ITEMS]},
        generated_with=dict(compiler=subprocess.run([cc, '--version'], capture_output=True, text=True).stdout.split('\n')[0]),
        items={r['name']: dict(map=r['map'], tics=r['tics'], captures=len(r['captures']), what=WAD.ITEMS[r['name']][7],
                               designed_callers=WAD.ITEMS[r['name']][5], designed_cases=WAD.ITEMS[r['name']][6],
                               intercept_calls=r['witness']['calls'], calls_by_caller=r['witness']['callers'],
                               wire26_refusal_tic=r['witness']['first_wrap'], wire27_end=r['witness']['end27'],
                               nowrap_stop=r['nowrap'], genuine_facts=r['facts'],
                               wrapping_calls_columns='tic caller line cases den_int den_exact num_int num_exact frac '
                                                      'frac_exact branch',
                               wrapping_calls=r['witness']['wraps']) for r in recs})


def selftest(bins, work, recs):
    """The comparison and the classification can fail (module text)."""
    out, it = [], items()['USE']
    base_caps = next(r for r in recs if r['name'] == 'USE')['captures']
    r = run(bins['strict'], it, work / 'self_draw', extra=['--selftest-extra-draw', '3'])
    caps, _ = project(r, it)
    differ = [j for (j, _t, a), (_j, _u, b) in zip(caps, base_caps) if a != b]
    out.append(('an extra P_Random draw before tic 3 of USE', f'captures {differ[:3]}... differ',
                differ == list(range(4, len(base_caps)))))
    w = run(bins['witness'], it, work / 'self_witness', witness=True)['witness'].splitlines()
    row = next(l for l in w if classify(l, {}, map_lines(it))['wrapped']).split()
    for what, change in (('a FixedMul result one off', lambda f: f.__setitem__(12, str(int(f[12]) + 1))),
                         ('an unknown traverser', lambda f: f.__setitem__(2, 'other'))):
        bent = list(row); change(bent)
        try: classify(' '.join(bent), {}, map_lines(it)); ok = False
        except ValueError: ok = True
        out.append((f'a witness row with {what}', 'refused' if ok else 'accepted', ok))
    shutil.rmtree(work / 'self_draw', ignore_errors=True); shutil.rmtree(work / 'self_witness', ignore_errors=True)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'wire27' / 'goldens')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    bins = {b: build(b, work / 'build', a.cc) for b in BUILDS}
    recs, fails = [], []
    for name, it in items().items():
        r = item_record(it, bins, work)
        recs.append(r); fails += r['fails']
        w = r['witness']
        print(f"{name}: {r['tics']} tics, {len(r['captures'])} captures, {w['calls']} intercept calls, "
              f"{len(w['wraps'])} wrapping; wire 26 refuses at {w['first_wrap']}; wire 27 {w['end27']}; nowrap {r['nowrap']}; "
              f"{dict(sorted((f'{c}/{k}', v) for (c, k), v in w['counts'].items()))}; {r['facts']}")
    if a.selftest:
        for what, detail, ok in selftest(bins, work, recs):
            print(f'selftest {"pass" if ok else "FAIL"}: {what}: {detail}')
            if not ok: fails.append(f'selftest: {what}')
    if fails: print('\n'.join(['FAILURES:'] + fails)); return 1
    state, res = state_text(recs), results(recs, a.cc)
    text = json.dumps(res, indent=1) + '\n'
    print(f"compiler: {res['generated_with']['compiler']}; {ASAN_NOTE}")
    if a.check:
        old = json.loads(RESULTS.read_text()) if RESULTS.exists() else {}
        old.pop('generated_with', None); res.pop('generated_with')
        bad = [f.name for f, ok in ((STATE, STATE.exists() and STATE.read_text() == state), (RESULTS, old == res)) if not ok]
        if bad: print(f'differs from the regenerated text: {", ".join(bad)}'); return 1
        print(f'{STATE.relative_to(ROOT)} and {RESULTS.relative_to(ROOT)} match ({len(recs)} items)')
    else:
        STATE.write_text(state); RESULTS.write_text(text)
        print(f'wrote {STATE.relative_to(ROOT)} and {RESULTS.relative_to(ROOT)}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
