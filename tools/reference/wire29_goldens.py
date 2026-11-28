#!/usr/bin/env python3
"""Wire 29 against the genuine engine: DSIM45 state digests of the 19 M6b items. Expected values come from the genuine
linuxdoom-1.10 runner and the independent DSIM45 encoder level_snapshot_wire29.py, never from Rust output.

The runner is the M6b runner (tools/reference/m6b_build.py, read-only), strict build (the genuine text, -fwrapv: THE
CLAIM), built under --work and run per item of tools/reference/m6b_wad.py ITEMS as m6b_goldens.run does (read-only:
the same switches and streams). Each run's dump set digest must equal the D column of the item's I row in the frozen
tests/reference/m6b_genuine.txt, where m6b_goldens.py has shown the overflow and address builds identical, and its
header bytes, tics, captures and end must equal that row's. The M6b stream's T row of the demo's G_InitNew must name
the skill, respawnparm and fastparm the encoder is given, `respawnmonsters` as skill 4 or respawnparm
(g_game.c:1416-1419), and the thirteen S_SARG_RUN1..S_SARG_PAIN2 durations of the fast table exactly when the skill is
4 or fastparm is set (g_game.c:1421-1428; ruling N4 (a), which these fresh runs witness). Every capture is projected by
the DSIM45 encoder; for skills 0 to 3 without respawnparm also by the DSIM44 one (level_snapshot_wire28.py), and
level_snapshot_wire29.compare44 must hold.

What is projected: the first level of each run, its load dump and the dump of every session tic until the level ends,
paused tics included (a paused tic's dump has the level unchanged and `level.gametic_lead` one higher). That is every
dump of every item but PAUSEWI, whose run also dumps 18 tics of its second level (E1M2, loaded at session tic 142 after
the paused intermission); the M6b runner writes no load dump for a later level, which the projection starts from, so
those 18 are pinned by m6b_genuine.txt's dump digest only. PAUSEWI and PAUSEFIN pause in the intermission and the
finale, after their first level's last dump.

Tic convention (owner): a session tic is the 0-based index of a G_Ticker; capture j is the state after j session tics.

Output (written without --check; --check regenerates and compares, writing nothing):
  tests/reference/wire29_state.txt   DOOM-WIRE29-STATE-v1: per item an I row (source map, skill as played, respawnparm,
                                     fastparm, session tics, the run's captures and dump set digest as in
                                     m6b_genuine.txt, the projected captures, the map digest, the sequence digest) and a
                                     C row per projected capture: tests/reference/m6a_state.txt's state digest over
                                     DSIM45; V rows (variant_rows): where each altered genuine engine of
                                     m6b_goldens.VARIANTS first differs over DSIM45, for a later Rust test that a port
                                     behaving like the variant is caught there

    wire29_goldens.py [--check] [--only ITEM[,ITEM]] [--work DIR] [--cc CC]
"""
import argparse, hashlib, os, shutil, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import level_snapshot as L           # noqa: E402
import level_snapshot_m5c as M5S     # noqa: E402
import level_snapshot_m6a as S6      # noqa: E402
import level_snapshot_wire28 as E28  # noqa: E402
import level_snapshot_wire29 as E    # noqa: E402
import level_compare as K            # noqa: E402
import m6a_state as ST               # noqa: E402  (read-only: Shifted, extra_lumps)
import m6b_build as B                # noqa: E402  (read-only)
import m6b_goldens as G              # noqa: E402  (read-only: run, tables_of)
import m6b_wad as WAD                # noqa: E402  (read-only)

STATE, SCHEMA = ROOT / 'tests/reference/wire29_state.txt', 'DOOM-WIRE29-STATE-v1'
FAST_SARG = '1 1 1 1 1 1 1 1 4 4 4 1 1'       # info.c:613-625 halved (g_game.c:1423-1424)
SLOW_SARG = '2 2 2 2 2 2 2 2 8 8 8 2 2'       # info.c:613-625
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()


def pinned():
    """The I rows of tests/reference/m6b_genuine.txt by item."""
    out = {}
    for line in G.GENUINE.read_text().splitlines():
        f = line.split(',')
        if f[0] == 'I':
            out[f[1]] = dict(source=f[2], skill=int(f[3]), respawn=int(f[4]), fast=int(f[5]), tics=int(f[6]),
                             captures=int(f[7]), end=f[8], dumps=f[9])
    return out


def item(name):
    return dict(name=name, wads=[ROOT / w for w in WAD.wads(name)], lmp_path=ROOT / f'{WAD.SCRIPT_DIR}/{name}.lmp')


def digest(fields):
    """m6a_state.txt's state digest of one capture over DSIM45."""
    body, _ = K.stripped(E.VERSION, fields)
    return sha(body + S6.framed([(p, v) for p, v in fields if p.startswith('events.sounds.')]))


def check_table(name, r, skill, respawn, fast):
    """The M6b stream's T row of the demo's G_InitNew against what the encoder is given (module text)."""
    rows = [t for t in G.tables_of(r) if t[0] == 'demo']
    want = [str(skill), str(respawn), str(fast), str(int(skill == 4 or respawn == 1)),
            FAST_SARG if skill == 4 or fast else SLOW_SARG]
    if len(rows) != 1 or rows[0][1:6] != want: sys.exit(f'{name}: the demo T row {rows} is not {want}')


def project(name, r, skill, respawn, fast):
    """[capture digests], the map digest; the DSIM44 projection alongside for skills 0 to 3 without respawnparm, which
    DSIM44 cannot hold (compare44)."""
    it = item(name)
    loads = S6.rows(r['session'], 'L')
    kind, p, _h, _s = S6.start_from_rows(r['session'], r['sound'], 0, str(it['wads'][0]))
    mapdata = L.MapData(str(it['wads'][0]), f"E1M{loads[0]['map']}", ST.extra_lumps(it))
    sounds = ST.Shifted(M5S.SoundStream(text=r['sound']), max(loads[0]['tic'], 0))
    start = (kind, dict(p))
    proj = E.Projector(mapdata, skill, loads[0]['map'], sounds, start, respawn, fast)
    old = E28.Projector(mapdata, skill, loads[0]['map'], sounds, (kind, dict(p))) if skill <= 3 and not respawn else None
    names = r['names']
    first = next((j for j, n in enumerate(names[1:]) if int(n[4:9]) != j), len(names) - 1)
    names = names[:first + 1]                     # the first level: the load and tics 0 .. first - 1 (module text)
    if len(loads) == 1 and len(names) != len(r['names']): sys.exit(f'{name}: a gap in the dumps of one level')
    caps, mapg = [], None
    following = L.Dump(str(r['dumps'] / names[0]))
    for j in range(len(names)):
        d, following = following, L.Dump(str(r['dumps'] / names[j + 1])) if j + 1 < len(names) else None
        fields, _ = proj.step(d, following)
        if old is not None:
            # the same in-memory gametic rewrite and sound reading as the DSIM45 projector's (level_snapshot_wire29.py)
            twin, nxt = (L.Dump(str(r['dumps'] / n)) if n else None for n in (names[j], (names + [None])[j + 1]))
            old.sounds = E.Ahead(sounds, twin.h['gametic'] - twin.h['leveltime'] - proj.first_lead)
            twin.h['gametic'] = proj.first_lead + twin.h['leveltime']
            E.compare44(fields, old.step(twin, nxt)[0])
        mapg = mapg or sha(S6.framed([x for x in fields if x[0].startswith('map.')]))
        caps.append(digest(fields))
    return caps, mapg


def item_record(name, binary, work, pin):
    data = WAD.lump(name)[0]
    skill, respawn, fast = min(data[1], 4), data[5], data[6]       # g_game.c:1379-1380, 1600-1601
    r = G.run(binary, name, work / 'runs' / name, keep=True)
    got = dict(skill=data[1], respawn=respawn, fast=fast, tics=WAD.lump(name)[1], captures=len(r['digests']),
               dumps=sha(''.join(r['digests'])))
    want = {k: pin[k] for k in got}
    if not G.clean(r) or got != want: sys.exit(f'{name}: the run is not the pinned genuine run: {got} against {want}')
    check_table(name, r, skill, respawn, fast)
    caps, mapg = project(name, r, skill, respawn, fast)
    shutil.rmtree(work / 'runs', ignore_errors=True)
    return dict(name=name, source=pin['source'], skill=skill, respawn=respawn, fast=fast, tics=got['tics'],
                run_captures=got['captures'], dumps=got['dumps'], captures=caps, mapg=mapg)


def variant_rows(work, cc, recs):
    """DSIM45 V rows: per m6b_goldens.VARIANTS row of m6b_genuine.txt whose first difference is a capture inside the
    projected level, the variant's build (m6b_build.variant, read-only), its run and projection, and the first DSIM45
    capture that differs from the genuine one with the variant's digest there; '-' when every projected capture is
    equal (the variant changes only what DSIM45 does not hold); 'refused' with the encoder's refusal when the variant's
    run is outside DSIM45."""
    out = []
    for line in G.GENUINE.read_text().splitlines():
        f = line.split(',')
        if f[0] != 'V' or f[3] != 'capture' or f[2] not in recs or int(f[4]) >= len(recs[f[2]]['captures']): continue
        v, name, place = f[1], f[2], int(f[4])
        binary = B.variant(v, G.VARIANTS[v][0], work / 'build', cc)
        r = G.run(binary, name, work / 'runs' / f'{name}_{v}', keep=True)
        rec = recs[name]
        try: caps, _m = project(name, r, rec['skill'], rec['respawn'], rec['fast'])
        except L.ProjectionError as e:      # outside DSIM45 (the `clamp` variant plays skill 5)
            out.append((v, name, place, 'refused', f'{e.kind} at level tic {e.tic}: {e.text}'))
            continue
        k = next((j for j, (a, b) in enumerate(zip(rec['captures'], caps)) if a != b), None)
        out.append((v, name, place, '-' if k is None else k, '-' if k is None else caps[k]))
        shutil.rmtree(work / 'runs', ignore_errors=True)
    return out


def state_text(recs, variants):
    head = [f'#schema,{SCHEMA}',
            '#generator,tools/reference/wire29_goldens.py: the genuine M6b runner (strict build) and the independent DSIM45 '
            'encoder tools/reference/level_snapshot_wire29.py; never from Rust output',
            '#tics,a session tic is the 0-based index of a G_Ticker; capture j is the state after j session tics, paused '
            'tics included',
            '#state,SHA-256 of the DSIM45 capture without map.* and events.* (level_compare.stripped framing) followed by '
            'its events.sounds.* fields (per-field framing); map: SHA-256 of the map.* fields of capture 0',
            '#dumps,the dumps_sha256 of the item\'s I row in tests/reference/m6b_genuine.txt',
            '#columns,I,item,source,skill,respawnparm,fastparm,tics,run_captures,dumps_sha256,captures,map_sha256,'
            'sequence_sha256',
            '#columns,C,item,capture,state_sha256',
            '#columns,V,variant,item,dump_place,first_differing_capture,variant_state_sha256 (dump_place: m6b_genuine.txt; '
            '- when every projected capture is equal; refused and the encoder\'s refusal when the run is outside DSIM45)']
    rows = []
    for r in recs:
        seq = sha(b''.join(bytes.fromhex(c) for c in r['captures']))
        rows.append(f"I,{r['name']},{r['source']},{r['skill']},{r['respawn']},{r['fast']},{r['tics']},{r['run_captures']},"
                    f"{r['dumps']},{len(r['captures'])},{r['mapg']},{seq}")
        rows += [f"C,{r['name']},{j},{d}" for j, d in enumerate(r['captures'])]
    rows += [f'V,{v},{n},{p},{k},{d}' for v, n, p, k, d in variants]
    return '\n'.join(head + rows) + '\n'


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--only', help='authoring: these items only (nothing is written or compared)')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'wire29' / 'goldens')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    only = a.only.split(',') if a.only else None
    binary = B.build('strict', work / 'build', a.cc)
    pin, recs = pinned(), []
    for name in WAD.ITEMS:
        if only and name not in only: continue
        r = item_record(name, binary, work, pin[name])
        recs.append(r)
        print(f"{name}: skill {r['skill']}, respawnparm {r['respawn']}, fastparm {r['fast']}, {r['tics']} tics, "
              f"{len(r['captures'])} captures (DSIM45)")
    if only: print('--only: nothing written or compared'); return 0
    variants = variant_rows(work, a.cc, {r['name']: r for r in recs})
    for v, n, p, k, _d in variants: print(f'variant {v}: {n} dumps first differ at capture {p}, DSIM45 at {k}')
    text = state_text(recs, variants)
    total = sum(len(r['captures']) for r in recs)
    if a.check:
        if not STATE.exists() or STATE.read_text() != text:
            print(f'{STATE.relative_to(ROOT)} differs from the regenerated text'); return 1
        print(f'{STATE.name} matches ({len(recs)} items, {total} captures, {len(variants)} variant rows)')
    else:
        STATE.write_text(text)
        print(f'wrote {STATE.relative_to(ROOT)} ({len(recs)} items, {total} captures, {len(variants)} variant rows)')
    return 0


if __name__ == '__main__':
    sys.exit(main())
