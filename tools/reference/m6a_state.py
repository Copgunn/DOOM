#!/usr/bin/env python3
"""Whole simulation state of the M6a level-chain items from the genuine engine: DSIM42 digests (M6a Phase 2 step 8).

The goldens of tools/reference/m6a_goldens.py hold session, frame and sound rows but no per-level state. This tool
runs every level-chain item of tests/reference/m6a_manifest.json (CHAIN_ITEMS) through the genuine M6a runner with
dumps, projects every dump with the INDEPENDENT DSIM42 encoder (tools/reference/level_snapshot_m6a.py, itself the
frozen DSIM41 projection of level_snapshot_m5c.py plus wire 26's start group) and writes digests only. Never from
Rust output.

The runner. The M6a runner (tools/reference/m6a_build.py) dumps every tic that ends in a level (tic_NNNNN.txt, NNNNN
the session tic) and the first load (load.txt), but not the later loads: a later load and its level's tic 0 share
one G_Ticker (g_game.c:617-650 then P_Ticker, 727-729), so no tic ends between them. The projection needs a load
dump to start each level's actor store (an actor freed in tic 0 still had its slot). This tool therefore builds a
STATE variant of the runner, in its own work directory, from the M6a runner's generated driver and
tools/reference/m6a_session.c with two exact-once patches made in memory (nothing on disk changes):
  - the driver keeps the dump directory (`m6a_state_dir`) after the first load's dump;
  - the P_SetupLevel observer (__wrap_P_SetupLevel, called by G_DoLoadLevel, g_game.c:484) writes, after every load
    but the first, the dump loads/load_NNNNN.txt (NNNNN the session tic of the load) of the state P_SetupLevel left,
    and then restores the observer's lists of freed and removed thinkers, so every tic dump is byte for byte the
    plain runner's (checked on every run against the committed dumps_per_load of m6a_results.json).
Every other stream of the run (session, sound, frames at screenblocks 10) must equal the committed golden.

The projection (declared rules, beyond those of level_snapshot_m6a.py):
  - one level is one load: its load dump (load.txt or loads/load_T.txt) and the tic dumps of the session tics from
    its load tic T to the tic before the next load; capture j (H.leveltime) is the state after level tic j - 1,
    whose genuine sound rows are those of session tic T + j - 1 (the stream is read with that offset);
  - the P_InitThinkers observer (m5c_driver.c __wrap_P_InitThinkers) records the previous level's thinkers as
    freed at the load (p_setup.c:622 Z_FreeTags the zone); serials only grow, so in a later level a K or RM row
    naming a serial below the level's first serial (the least serial of its load dump) is the previous level's and
    is dropped;
  - the start group: kind, P_Random index, health, armour, weapons, ammunition, the backpack, cheats and the
    latches from the session rows (level_snapshot_m6a.start_from_rows); bob, psprites[ps_weapon].sx and the null
    flash psprite's tics, sx and sy (which the C rows do not record) from the previous level's last dump's P row,
    which neither G_PlayerFinishLevel (g_game.c:779-792), the intermission nor P_SetupPsprites (p_pspr.c:831-842,
    states only) changes; the load dump must hold the same values (checked);
  - compared state of a capture: the DSIM42 fields without `map.*` and `events.*` in level_compare.stripped's
    framing ('DSIM', u32 42, u64 field count, then per field u32 path length, path, u64 value length, value),
    followed by the `events.sounds.*` fields in the same per-field framing; its SHA-256 is the capture's state
    digest. `events.last` and `events.pending` (the encoder writes both with count 0, as DSIM40, M5c note 4.4.4)
    are left out; `map.*` is digested once per level (capture 1) in the per-field framing;
  - the session points: every capture j >= 1 (the state after each level tic) is in its level's sequence digest
    (SHA-256 of the captures' state digests, 32 bytes each, in order); D rows name capture 1 (the load tic), every
    capture whose leveltime is a multiple of STRIDE, the last capture of each level (the tic before a boundary or
    the run's last level tic). Capture 0 of a later level is no session state (its tic 0 runs in the same tic).

Results: tests/reference/m6a_state.txt (DOOM-M6A-STATE-v1), written without --check, compared byte for byte with
--check (which writes nothing). Counts and digests only.

--selftest proves the digests can fail: a planted extra P_Random draw at session tic 300 of CHAIN changes the
sequence digest of exactly the level holding tic 300 and the later ones; a bob one unit off (lower, or 1 for 0) in
the carried start groups changes every capture digest of the first carried level; and the previous level's K rows
left in (the serial rule off) are refused by the projection.

    m6a_state.py [--check] [--selftest] [--only ITEM[,ITEM]] [--work DIR] [--jobs N] [--cc CC]
"""
import argparse, concurrent.futures as cf, hashlib, json, os, shutil, struct, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m3h9c_dump as dump      # noqa: E402
import level_snapshot as L     # noqa: E402
import level_snapshot_m5c as M5C   # noqa: E402
import level_snapshot_m6a as S     # noqa: E402
import level_compare as K      # noqa: E402
import m6a_build as B          # noqa: E402
import m6a_goldens as G        # noqa: E402  (read-only: items, the recommended switches, golden_* and segments)

RESULTS = ROOT / 'tests/reference/m6a_state.txt'
SCHEMA = 'DOOM-M6A-STATE-v1'
CHAIN_ITEMS = ['CHAIN', 'INTSTA', 'INTSTB', 'INTIDLE', 'INTHELD', 'INTFAST', 'REBHELD', 'REBLATE', 'SECRET', 'REBSECRT',
               'PROBE', 'USEAT130', 'USEAT131', 'USEAT132', 'RBXDEAD', 'NOINTER', 'NOE1M2', 'NOE1M2AT']
STRIDE = 35
UNRECORDED = dict(bob='bob', weapon_x='ps0sx', flash_tics='ps1tics', flash_x='ps1sx', flash_y='ps1sy')
sha = lambda b: hashlib.sha256(b).hexdigest()

DRIVER_PATCH = ('    if (!nodump) dump (out, "load.txt", -1);\n',
                '    if (!nodump) dump (out, "load.txt", -1);\n'
                '    m6a_state_dir = nodump ? NULL : out;      /* M6a state variant: the later loads\' dumps */\n')
SESSION_PATCHES = [
    ('void __real_P_SetupLevel (int episode, int map, int playermask, skill_t skill);\n',
     '/* M6a state variant (tools/reference/m6a_state.py): the dump of a later load, then the observer\'s freed and\n'
     ' * removed lists restored, so the next tic dump is the plain runner\'s byte for byte. */\n'
     'static char* m6a_state_dir;\n'
     'static void m6a_state_load_dump (void)\n'
     '{\n'
     '    char name[64]; int nf = nfreed, nr = nremoved, fs = freeseq;\n'
     '    snprintf (name, sizeof name, "loads/load_%05d.txt", snd_tic);\n'
     '    dump (m6a_state_dir, name, snd_tic);\n'
     '    nfreed = nf; nremoved = nr; freeseq = fs;\n'
     '}\n'
     'void __real_P_SetupLevel (int episode, int map, int playermask, skill_t skill);\n'),
    ('    m6a_loads++;\n',
     '    m6a_loads++;\n'
     '    if (m6a_loads > 1 && m6a_state_dir) m6a_state_load_dump ();   /* M6a state variant */\n'),
]


def patched(text, patches):
    for old, new in patches:
        if text.count(old) != 1: sys.exit(f'state patch does not apply exactly once ({text.count(old)}): {old[:60]!r}')
        text = text.replace(old, new)
    return text


def build(work, cc):
    """The state variant of the strict M6a runner (module text), built under `work`."""
    work.mkdir(parents=True, exist_ok=True)
    session = work / 'patched' / B.SESSION_C.name
    session.parent.mkdir(exist_ok=True)
    session.write_bytes(patched(B.text(B.SESSION_C), SESSION_PATCHES).encode('latin-1'))
    plain_driver, plain_session = B.driver_text, B.SESSION_C
    B.driver_text = lambda: patched(plain_driver(), [DRIVER_PATCH])
    B.SESSION_C = session
    try:
        with open(os.devnull, 'w') as q:
            so, sys.stdout = sys.stdout, q
            try: return B.build('strict', work / 'build', cc)
            finally: sys.stdout = so
    finally:
        B.driver_text, B.SESSION_C = plain_driver, plain_session


def run(binary, item, out, extra=()):
    """One state-variant run at screenblocks 10 with every stream (m6a_goldens.run's command plus the load dumps)."""
    if out.exists(): shutil.rmtree(out)
    (out / 'dumps' / 'loads').mkdir(parents=True)
    cmd = [str(binary), *[x for w in item['wads'] + [item['lmp_path']] for x in ('--wad', str(w))], '--demo', item['lump'],
           '--game-mode', item['mode'], '--out', str(out / 'dumps'),
           *[x for k, v in G.M5C.RECOMMENDED.items() for x in (k, v)], '--render', 'every', '--screenblocks', '10', *extra,
           '--frames', str(out / 'frames.txt'), '--sound-out', str(out / 'sound.txt'), '--session-out', str(out / 'session.txt')]
    p = subprocess.run(cmd, capture_output=True, text=True, env=G.ENV)
    rd = lambda n: (out / n).read_text() if (out / n).exists() else ''
    return dict(code=p.returncode, stdout=p.stdout, stderr=p.stderr, frames=rd('frames.txt'), sound=rd('sound.txt'),
                session=rd('session.txt'), dumps=out / 'dumps')


class Shifted:
    """A genuine sound stream read with an offset: rows(j) are the rows of session tic j + offset."""
    def __init__(self, base, offset):
        self.base, self.offset, self.last_tic = base, offset, base.last_tic - offset

    def rows(self, tic):
        return self.base.rows(tic + self.offset)


def extra_lumps(item):
    """The lump names after the first WAD (W_AddFile order) and the demo's (D_AddFile), for R_InitFlats."""
    names = []
    for w in item['wads'][1:]:
        data = Path(w).read_bytes()
        _, count, offset = struct.unpack_from('<4sii', data, 0)
        names += [struct.unpack_from('<8s', data, offset + 16 * k + 8)[0].split(b'\0')[0].decode('latin-1') for k in range(count)]
    return tuple(names) + L.demo_lumps(str(item['lmp_path']))


def framed(fields):
    return S.framed(fields)


def state_digest(fields):
    """The capture's state digest (module text)."""
    body, _ = K.stripped(S.VERSION, fields)
    return hashlib.sha256(body + framed([(p, v) for p, v in fields if p.startswith('events.sounds.')])).digest()


def drop_previous(d, first):
    """The dump without the K and RM rows of serials below `first` (module text)."""
    d.freed = [(q, s) for q, s in d.freed if s >= first]
    d.removed = {s: r for s, r in d.removed.items() if s >= first}
    if d.adds is not None: d.adds = {s: a for s, a in d.adds.items() if s >= first}
    return d


def p_row(d):
    return next(p for p in d.players if p['idx'] == 0)


def levels(r, item, rule=True, bob_delta=0):
    """Project every level of a run: [dict(load, tic, map, kind, captures [(leveltime, session tic, digest)], map)]."""
    session, sound = r['session'], r['sound']
    stream = M5C.SoundStream(text=sound)
    loads = S.rows(session, 'L')
    skill = L.demo_skill(str(item['lmp_path']))
    lumps = extra_lumps(item)
    ddir = r['dumps']
    tics = sorted(int(n[4:9]) for n in os.listdir(ddir) if n.startswith('tic_'))
    out, previous = [], None
    for k, l in enumerate(loads):
        T = l['tic']; end = loads[k + 1]['tic'] if k + 1 < len(loads) else 1 << 30
        names = (['load.txt'] if k == 0 else [f'loads/load_{T:05}.txt']) + [f'tic_{t:05}.txt' for t in tics if max(T, 0) <= t < end]
        ds = [L.Dump(str(ddir / n)) for n in names]
        if k:
            first = min(s for s, _ in ds[0].serials.values())
            if rule: ds = [drop_previous(d, first) for d in ds]
        kind, p, _h, _s = S.start_from_rows(session, sound, k, str(item['wads'][0]))
        if kind == 'carried':
            want = {f: p_row(previous)[c] for f, c in UNRECORDED.items()}
            if {f: p_row(ds[0])[c] for f, c in UNRECORDED.items()} != want:
                raise L.ProjectionError('DumpDeficiency', 0, f'load {k}: the load dump does not hold the carried bob and psprites')
            p.update(want)
            if bob_delta: p['bob'] = p['bob'] - 1 if p['bob'] > 0 else 1
        mapdata = L.MapData(str(item['wads'][0]), f'E1M{l["map"]}', lumps)
        proj = S.Projector(mapdata, skill, l['map'], Shifted(stream, max(T, 0)), (kind, p))
        caps, mapg = [], None
        for j, d in enumerate(ds):
            fields, _ = proj.step(d, ds[j + 1] if j + 1 < len(ds) else None)
            if d.tic == 0: continue
            if mapg is None: mapg = sha(framed([x for x in fields if x[0].startswith('map.')]))
            caps.append((d.tic, d.h['tic'], state_digest(fields)))
        if [c[0] for c in caps] != list(range(1, len(caps) + 1)):
            raise L.ProjectionError('DumpDeficiency', 0, f'load {k}: level tics not consecutive')
        out.append(dict(load=k, tic=T, map=l['map'], kind=kind, captures=caps, mapg=mapg))
        previous = ds[-1]
    return out


def rows_of(name, lv):
    """The L and D rows of one item."""
    out = []
    for v in lv:
        caps = v['captures']
        seq = sha(b''.join(c[2] for c in caps))
        out.append(f'L,{name},{v["load"]},{v["tic"]},{v["map"]},{v["kind"]},{len(caps)},{v["mapg"] or "-"},{seq}')
        for j, (lt, tic, d) in enumerate(caps):
            why = [w for w, c in (('load', j == 0), ('stride', lt % STRIDE == 0), ('last', j == len(caps) - 1)) if c]
            if why: out.append(f'D,{name},{v["load"]},{tic},{lt},{"+".join(why)},{d.hex()}')
    return out


def check_run(r, item, results):
    """Failures if the state variant's run is not the plain runner's (the committed goldens and dumps_per_load)."""
    name, fails = item['name'], []
    want = lambda p: (ROOT / p).read_text()
    if r['session'] != want(f'tests/reference/m6a_session/{name}.session'): fails.append(f'{name}: session stream differs')
    if G.golden_sound(r['sound']) != want(f'tests/reference/m6a_sound/{name}.sound'): fails.append(f'{name}: sound stream differs')
    if G.golden_frames(r['frames']) != want(f'tests/reference/m6a_frames/{name}.sb10.frames'):
        fails.append(f'{name}: frame stream differs')
    snaps = [(n, sha((r['dumps'] / n).read_bytes())) for n in dump.snapshot_names(str(r['dumps']))]
    if G.segments(snaps, r['session']) != results['items'][name]['dumps_per_load']:
        fails.append(f'{name}: tic dumps differ from the committed dumps_per_load')
    return fails


def run_item(binary, item, work, results):
    name = item['name']
    r = run(binary, item, work / 'runs' / name)
    fails = check_run(r, item, results)
    lv = levels(r, item) if (r['dumps'] / 'load.txt').exists() else []
    shutil.rmtree(work / 'runs' / name)
    return name, rows_of(name, lv), fails, sum(len(v['captures']) for v in lv), len(lv)


def results_text(per_item):
    head = [f'#schema,{SCHEMA}',
            '#generator,tools/reference/m6a_state.py: the genuine M6a runner (state variant, strict) and the independent '
            'DSIM42 encoder tools/reference/level_snapshot_m6a.py; never from Rust output',
            '#state,SHA-256 of the DSIM42 capture without map.* and events.* (level_compare.stripped framing) followed by '
            'its events.sounds.* fields (per-field framing); map: SHA-256 of the map.* fields of capture 1',
            f'#points,L: every capture j >= 1 of a level in sequence_sha256 (SHA-256 of the 32-byte state digests in order); '
            f'D: capture 1 (load), every leveltime multiple of {STRIDE} (stride) and the level\'s last capture (last)',
            '#columns,L,item,load,tic,map,kind,captures,map_sha256,sequence_sha256',
            '#columns,D,item,load,tic,leveltime,why,state_sha256']
    return '\n'.join(head + [l for name in CHAIN_ITEMS for l in per_item.get(name, [])]) + '\n'


def selftest(binary, items_by_name, work, results):
    out = []
    it = items_by_name['CHAIN']
    base = run(binary, it, work / 'self_base')
    lv = levels(base, it)
    seqs = [sha(b''.join(c[2] for c in v['captures'])) for v in lv]
    r = run(binary, it, work / 'self_draw', extra=['--selftest-extra-draw', '300'])
    got = [sha(b''.join(c[2] for c in v['captures'])) for v in levels(r, it)]
    at = max(k for k, v in enumerate(lv) if v['tic'] <= 300)
    differ = [k for k, (a, b) in enumerate(zip(seqs, got)) if a != b]
    ok = differ == list(range(at, len(lv)))
    out.append(('extra P_Random draw at tic 300', f'levels {differ} differ (want {at}..{len(lv) - 1})', ok))
    bent = levels(base, it, bob_delta=1)
    k = next(k for k, v in enumerate(lv) if v['kind'] == 'carried')
    diff = [j for j, v in enumerate(bent) if [c[2] for c in v['captures']] != [c[2] for c in lv[j]['captures']]]
    every = all(a[2] != b[2] for a, b in zip(bent[k]['captures'], lv[k]['captures']))
    out.append(('carried bob one unit off in the start group', f'levels {diff} differ, every capture of level {k}: {every}', k in diff and every))
    try:
        levels(base, it, rule=False); refused = None
    except L.ProjectionError as e: refused = f'{e.kind} at tic {e.tic}: {e.text}'
    out.append(('previous level\'s K rows kept', f'refused: {refused}', refused is not None))
    for d in ('self_base', 'self_draw'): shutil.rmtree(work / d, ignore_errors=True)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--only'); ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'doom-m6a-state')
    ap.add_argument('--jobs', type=int, default=2); ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    binary = build(work, a.cc)
    results = json.loads(G.RESULTS.read_text())
    items = {i['name']: i for i in G.items() if i['name'] in CHAIN_ITEMS}
    want = [n for n in CHAIN_ITEMS if not a.only or n in {x.upper() for x in a.only.split(',')}]
    per, fails, total = {}, [], [0, 0]
    with cf.ThreadPoolExecutor(max_workers=max(1, min(a.jobs, 2))) as ex:
        for name, rows, f, caps, nlv in ex.map(lambda n: run_item(binary, items[n], work, results), want):
            per[name] = rows; fails += f; total[0] += caps; total[1] += nlv
            print(f'{name}: {nlv} levels, {caps} captures, {sum(r.startswith("D,") for r in rows)} D rows'
                  + ('' if not f else '; ' + '; '.join(f)))
    if a.selftest:
        for what, detail, ok in selftest(binary, items, work, results):
            print(f'selftest {"pass" if ok else "FAIL"}: {what}: {detail}')
            if not ok: fails.append(f'selftest: {what}')
    print(f'{len(want)} items, {total[1]} levels, {total[0]} captures projected')
    if fails:
        print('\n'.join(['FAILURES:'] + fails)); return 1
    if a.only:
        print('--only: the results file is neither written nor compared'); return 0
    text = results_text(per)
    if a.check:
        if not RESULTS.exists() or RESULTS.read_text() != text:
            print(f'{RESULTS.relative_to(ROOT)} differs from the regenerated digests'); return 1
        print(f'{RESULTS.relative_to(ROOT)} matches ({total[1]} levels, {total[0]} captures)')
    else:
        RESULTS.write_text(text); print(f'wrote {RESULTS.relative_to(ROOT)}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
