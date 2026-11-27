#!/usr/bin/env python3
"""Audit 1, item 1: the sector-11 cap of P_DamageMobj (p_inter.c:836-842), run against the genuine engine.

In linuxdoom-1.10 every blow on the player goes through P_DamageMobj, and on a sector of special 11 a blow of at least
the player's health leaves health 1 ("end of game hell hack"); the sector's own 20 points every 32 tics go through the
same function and the level exits once health is 10 or less (p_spec.c:1054-1063). Each item below is one synthetic
room whose every sector has special 11, with one damage path that delivers a lethal blow there. Nothing comes from any
IWAD: the rooms are drawn with the M6a room builder (m6a_wad, read-only) and the scripts with the m4b_scripts format.

  FLOOR     the floor alone: 100, 80, 60, 40, 20, then the fifth 20-point blow is capped (session tic 128)
  ZOMBIE    former humans (hitscan, p_enemy.c A_PosAttack) on ledges they cannot leave
  SHOTGUY   shotgun guys (three pellets, each its own P_DamageMobj, A_SPosAttack) on ledges
  IMPBALL   imps on ledges: fireball contact only (out of melee range, P_CheckMeleeRange reads no z)
  IMPCLAW   an imp in a closet with the player: always in melee range, claw only (A_TroopAttack)
  BARONFB   barons on ledges: fireball contact only (A_BruisAttack)
  BARONCL   a baron in a closet with the player: claw only
  BARREL    the player shoots a barrel beside it: the splash (P_RadiusAttack, p_map.c:1179-1194)
  ROCKET    the player picks up a rocket launcher and fires at the wall in front of it: its own splash
  SPECTRE   wire28.wad's BITE room with a spectre for the demon (A_SargAttack)
  BITE2     the wire 28 item itself (tests/reference/wire28_wads/wire28.wad E1M5, its committed demo): the demon bite

Genuine runs (wire28_goldens.build, read-only): strict (the claim), overflow and address (ASAN_OPTIONS=detect_leaks=0:
leak detection disabled, not LeakSanitizer coverage) must give identical streams; the strict dumps are projected by the
independent DSIM44 encoder (wire28_goldens.project). The altered engine `nocap` (an in-process addition to
wire28_goldens.VARIANTS: the cap's condition made false) must differ from every item at the capture after its lethal
blow, with the player dead there; that capture and blow are the item's L row. Never Rust output.

Output tests/reference/audit1_cap/cap_state.txt (DOOM-AUDIT1-CAP-STATE-v1):
  I,item,wad,map,skill,tics,captures,end           end: exit:N, the genuine level ends after session tic N
  L,item,blow_tic,health_before,attacker           the lethal blow nocap shows (the variant's player dead at blow_tic + 1)
  C,item,capture,whole,projected                   whole: wire28_state.txt's state digest over DSIM44;
                                                   projected: SHA-256 of level_compare.stripped(44, the fields without
                                                   PROJECTION_DROP), the part DSIM39 to DSIM44 share
  V,nocap,item,first_differing_capture,whole       the variant's whole digest there
and cap_results.json (DOOM-AUDIT1-CAP-RESULTS-v1: builds, the variant, tool and input digests, every blow per item).

    audit1_cap.py [--check] [--work DIR] [--cc CC] [--only ITEM ..]
"""
import argparse, hashlib, json, math, shutil, subprocess, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import level_compare as K          # noqa: E402
import m4b_scripts as S            # noqa: E402  the script format (read-only)
import m6a_wad as W                # noqa: E402  the room builder (read-only)
import wire28_goldens as G         # noqa: E402  build, project, digest, the runner (read-only)
import wire28_wad as W28           # noqa: E402  the BITE room and the WAD set (read-only)
from m3h9_geometry import enum_index  # noqa: E402

OUT = 'tests/reference/audit1_cap'
STATE, RESULTS = ROOT / OUT / 'cap_state.txt', ROOT / OUT / 'cap_results.json'
WADS = [f'{OUT}/cap1.wad', f'{OUT}/cap2.wad']
PROJECTION_DROP = ('profile', 'context.schema', 'level.map', 'level.start.')
E, N, WE, SO = 0, 90, 180, 270
PLAYER, ZOMBIE, SHOTGUY, IMP, BARON, SPECTRE, BARREL, LAUNCHER = 1, 3004, 9, 3001, 3003, 58, 2035, 2003
CAP_OLD = '\tif (target->subsector->sector->special == 11\n\t    && damage >= target->health)\n'
CAP_NEW = '\tif (target->subsector->sector->special == 11\n\t    && 0)\n'
NAMES = {enum_index('mobjtype_t', n): n[3:] for n in ('MT_PLAYER', 'MT_POSSESSED', 'MT_SHOTGUY', 'MT_TROOP',
                                                      'MT_SERGEANT', 'MT_SHADOWS', 'MT_BRUISER', 'MT_BARREL',
                                                      'MT_TROOPSHOT', 'MT_BRUISERSHOT')}
WHOLE = G.digest
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()


def room(name, box, things, ledges=(), safe=None):
    """One sector of special 11 (ceiling 256), ledges (x, y): 64-square sectors of special 11 with floor 64 at
    (x, y), each holding its monster (a drop of 64 > 24 keeps it there, p_map.c P_TryMove), and an optional `safe`
    rectangle of special 0 where the player starts and is worn down before it steps onto special 11."""
    p = W.Room(name, wall='M5WALLA')
    p.sector('ROOM', [box], ceil=256, light=192, special=11, fpic='M5FLAT1', cpic='M5FLAT2')
    if safe: p.sector('SAFE', [safe], ceil=256, light=192, fpic='M5FLAT3', cpic='M5FLAT2', layer=1)
    for x, y, kind in ledges:
        p.sector(f'LEDGE{x}_{y}', [(x - 32, y - 32, x + 32, y + 32)], floor=64, ceil=256, light=192, special=11,
                 fpic='M5FLAT3', cpic='M5FLAT2', layer=1)
        p.thing(x, y, round(math.degrees(math.atan2(-y, -x))) % 360, kind)   # facing the centre
    for t in things: p.thing(*t)
    return p


ARENA = (-384, -384, 384, 384)
LONE = [(0, 0, E, PLAYER)]
TEN = ['idle 160']
# name: (wad index, map, room, script lines, what, the lethal blow's attacker, ball (a fireball in flight and the
# attacker at least 64 away on both axes after the blow, beyond P_CheckMeleeRange's 60) or claw (no fireball ever) or None). The ledge layouts were found by trying layouts
# against the genuine engine until its run had a lethal blow from the item's path (the search is not committed).
ITEMS = {
    'FLOOR': (0, 1, lambda: room('floor', (-128, -128, 128, 128), LONE), TEN, 'the floor of special 11 alone',
              'none', None),
    'ZOMBIE': (0, 2, lambda: room('zombie', ARENA, LONE, [(x, y, ZOMBIE) for x, y in (
        (-96, 80), (-128, -32), (-48, -128), (80, -96), (112, 16), (32, 112))]), TEN,
        'former humans on ledges: hitscan', 'POSSESSED', None),
    'SHOTGUY': (0, 3, lambda: room('shotguy', ARENA, LONE, [(x, y, SHOTGUY) for x, y in (
        (144, 48), (-64, 144), (-160, -64), (48, -160))]), TEN, 'shotgun guys on ledges: pellets', 'SHOTGUY', None),
    'IMPBALL': (0, 4, lambda: room('impball', ARENA, LONE, [(128, 0, IMP), (-128, 0, IMP)]), TEN,
                'imps on ledges: fireball contact', 'TROOP', 'ball'),
    'IMPCLAW': (0, 5, lambda: room('impclaw', (-48, -32, 64, 32), [(-13, 0, E, PLAYER), (42, 4, N, IMP)]), TEN,
                'an imp in a closet with the player: claw', 'TROOP', 'claw'),
    'BARONFB': (0, 6, lambda: room('baronball', ARENA, LONE, [(128, 0, BARON), (-64, 96, BARON), (-64, -112, BARON)]),
                TEN, 'barons on ledges: fireball contact', 'BRUISER', 'ball'),
    'BARONCL': (0, 7, lambda: room('baronclaw', (-48, -32, 64, 32), [(-8, 0, E, PLAYER), (40, 0, WE, BARON)]), TEN,
                'a baron in a closet with the player: claw', 'BRUISER', 'claw'),
    'BARREL': (0, 8, lambda: room('barrel', (-128, -128, 256, 128), [(0, 0, E, PLAYER), (36, 0, E, BARREL)]),
               ['idle 8'] + ['cmd 1 0 0 0 ATTACK', 'idle 20'] * 4, 'the player shoots a barrel beside it: splash',
               'PLAYER', None),
    'ROCKET': (0, 9, lambda: room('rocket', (-128, -128, 64, 128), [(0, 0, E, PLAYER), (32, 0, E, LAUNCHER)]),
               ['cmd 2 25 0 0 0', 'idle 50', 'cmd 1 0 0 0 ATTACK', 'idle 60'],
               'the player fires its own rocket at the wall in front of it: splash', 'PLAYER', None),
    'SPECTRE': (1, 1, lambda: room('spectre', (-256, -128, 256, 128), [(0, 0, E, PLAYER), (224, 0, WE, SPECTRE)]),
                ['idle 300'], 'a spectre bite (wire28.wad BITE with a spectre)', 'SHADOWS', None),
}
BITE2 = dict(wads=[ROOT / w for w in W28.WADSET], map=5, lmp_path=ROOT / f'{W28.SCRIPT_DIR}/BITE2.lmp',
             what='the wire 28 item BITE2: a demon bite', attacker='SERGEANT')
ORDER = list(ITEMS) + ['BITE2']


def script_text(name):
    _w, m, _r, lines, what = ITEMS[name][:5]
    return f'# audit 1 cap item {name}: {what}.\n# Generated by tools/reference/audit1_cap.py.\n' + \
        '\n'.join([f'map E1M{m}'] + lines) + '\n'


def files():
    out = {}
    for k, path in enumerate(WADS):
        out[path] = W.iwad([(f'E1M{m}', build()) for w, m, build, *_x in ITEMS.values() if w == k])
    for name in ITEMS:
        out[f'{OUT}/{name}.txt'] = script_text(name).encode()
        out[f'{OUT}/{name}.lmp'] = S.lump(script_text(name))[0]
    return out


def item(name):
    if name == 'BITE2': return dict(BITE2, name=name, skill=2)
    w, m = ITEMS[name][:2]
    return dict(name=name, map=m, skill=2, wads=[ROOT / WADS[w]] + [ROOT / x for x in W28.WADSET[1:]],
                lmp_path=ROOT / f'{OUT}/{name}.lmp')


def digests(version, fields):
    """(whole, projected) of one capture (module documentation)."""
    kept = [(p, v) for p, v in fields if p not in PROJECTION_DROP[:3] and not p.startswith(PROJECTION_DROP[3])]
    return WHOLE(version, fields), sha(K.stripped(version, kept)[0])


def run(binary, it, work):
    """[(whole, projected)] per capture and the dumps of one run."""
    r = G.W27.run(binary, it, work)
    caught, plain = [], G.digest
    G.digest = lambda v, f: caught.append(digests(v, f)) or caught[-1][0]
    try: _caps, _m, dumps = G.project(r, it)
    finally: G.digest = plain
    return r, caught, dumps


def blows(dumps):
    """Every tic the player's health falls: [tic, before, after, attacker type or none]."""
    out = []
    for j in range(1, len(dumps)):
        a, b = dumps[j - 1].players[0], dumps[j].players[0]
        if b['health'] < a['health']:
            att = {m['idx']: m for k, m in dumps[j].thinkers if k == 'M'}.get(b['attacker'])
            out.append([j - 1, a['health'], b['health'], 'none' if att is None else NAMES.get(att['type'], att['type'])])
    return out


def record(name, bins, work):
    it, fails = item(name), []
    runs = {b: run(bins[b], it, work / 'runs' / f'{name}_{b}') for b in G.CLAIM + ('nocap',)}
    s, caps, dumps = runs['strict']
    if s['code'] or G.W27.UB.search(s['stderr']) or not s['marker']: fails.append(f'{name}: strict exit {s["code"]}')
    for b in G.CLAIM[1:]:
        r = runs[b][0]
        if r['code'] or (r['snapshots'], r['sound'], r['session']) != (s['snapshots'], s['sound'], s['session']):
            fails.append(f'{name}: {b} exit {r["code"]} or streams differ from strict')
    _r, vcaps, vdumps = runs['nocap']
    k = next((j for j, (a, b) in enumerate(zip(caps, vcaps)) if a != b), None)
    hit = [h for h in blows(dumps) if k is not None and h[0] == k - 1]
    if k is None or not hit or hit[0][2] != 1 or vdumps[k].players[0]['health'] > 0:
        fails.append(f'{name}: nocap does not die at the capture after a capped blow ({k}, {hit})')
    attacker, mode = (ITEMS[name][5:] if name in ITEMS else (BITE2['attacker'], None))
    balls = sum(any(NAMES.get(m['type'], '').endswith('SHOT') for t, m in d.thinkers if t == 'M') for d in dumps)
    reach = None
    if hit and k is not None:
        me = G.body(dumps[k])
        att = {m['idx']: m for t, m in dumps[k].thinkers if t == 'M'}.get(dumps[k].players[0]['attacker'])
        reach = att and max(abs(att['x'] - me['x']), abs(att['y'] - me['y'])) >> 16
        if hit[0][3] != attacker or (attacker != 'none' and hit[0][0] % 32 == 0 and hit[0][1] <= 20):
            fails.append(f'{name}: the lethal blow is not from {attacker} alone ({hit})')
        if (mode == 'claw' and balls) or (mode == 'ball' and (not balls or reach < 64)):
            fails.append(f'{name}: {mode} expected; {balls} captures with a fireball, attacker {reach} away')
    shutil.rmtree(work / 'runs', ignore_errors=True)
    return dict(name=name, it=it, caps=caps, blows=blows(dumps), k=k, hit=hit[0] if hit else None, balls=balls,
                reach=reach,
                vdigest=vcaps[k][0] if k is not None else None, fails=fails)


def state_text(recs):
    head = ['#schema,DOOM-AUDIT1-CAP-STATE-v1',
            '#generator,tools/reference/audit1_cap.py: the genuine M6a runner (strict build) and the independent DSIM44 '
            'encoder tools/reference/level_snapshot_wire28.py; never from Rust output',
            '#whole,tests/reference/wire28_state.txt\'s state digest over DSIM44',
            '#projected,SHA-256 of level_compare.stripped(44) of the DSIM44 fields without profile, context.schema, '
            'level.map and level.start.*',
            '#columns,I,item,wad,map,skill,tics,captures,end',
            '#columns,L,item,blow_tic,health_before,attacker',
            '#columns,C,item,capture,whole_sha256,projected_sha256',
            '#columns,V,variant,item,first_differing_capture,variant_whole_sha256']
    rows = []
    for r in recs.values():
        it = r['it']
        wad = Path(it['wads'][0]).relative_to(ROOT).as_posix()
        tics = S.lump(script_text(r['name']))[1] if r['name'] in ITEMS else W28.lump('BITE2')[1]
        rows.append(f"I,{r['name']},{wad},{it['map']},2,{tics},{len(r['caps'])},exit:{len(r['caps']) - 2}")
        rows.append(f"L,{r['name']},{r['hit'][0]},{r['hit'][1]},{r['hit'][3]}")
        rows += [f"C,{r['name']},{j},{w},{p}" for j, (w, p) in enumerate(r['caps'])]
    rows += [f"V,nocap,{r['name']},{r['k']},{r['vdigest']}" for r in recs.values()]
    return '\n'.join(head + rows) + '\n'


def results(recs, cc):
    d = lambda p: sha((ROOT / p).read_bytes())
    return dict(schema='DOOM-AUDIT1-CAP-RESULTS-v1', revision=G.W27.base.REVISION,
                claim='every digest is the genuine engine\'s strict build projected by the independent DSIM44 encoder; '
                      'the overflow and address builds gave identical dumps, sound and session streams',
                builds={b: G.W27.base.VARIANTS[G.W27.BUILDS[b][0]] for b in G.CLAIM}, asan_options=G.W27.ASAN_NOTE,
                variant=dict(name='nocap', file='p_inter.c', original=CAP_OLD, replacement=CAP_NEW,
                             scope='in-process addition to wire28_goldens.VARIANTS; no file is edited'),
                tools_sha256={t: d(f'tools/reference/{t}') for t in ['audit1_cap.py'] + G.TOOLS},
                inputs_sha256={Path(p).relative_to(ROOT).as_posix(): d(Path(p).relative_to(ROOT))
                               for p in sorted({str(x) for r in recs.values() for x in r['it']['wads'] + [r['it']['lmp_path']]})},
                generated_with=dict(compiler=subprocess.run([cc, '--version'], capture_output=True, text=True).stdout.split('\n')[0]),
                items={r['name']: dict(what=ITEMS[r['name']][4] if r['name'] in ITEMS else BITE2['what'],
                                       captures=len(r['caps']), lethal=r['hit'], nocap_differs_at=r['k'],
                                       captures_with_a_fireball=r['balls'], attacker_distance_after=r['reach'],
                                       blows=r['blows']) for r in recs.values()})


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true'); ap.add_argument('--files', action='store_true')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'audit1' / 'cap')
    ap.add_argument('--cc', default='cc'); ap.add_argument('--only', nargs='*')
    a = ap.parse_args()
    generated = files()
    bad = [p for p, d in generated.items() if not (ROOT / p).exists() or (ROOT / p).read_bytes() != d]
    if a.check and bad: print('differs or missing: ' + ', '.join(bad)); return 1
    if not a.check:
        for p in bad: (ROOT / p).parent.mkdir(parents=True, exist_ok=True); (ROOT / p).write_bytes(generated[p])
    print(f'{len(generated)} WAD and script files {"match" if a.check else "written or unchanged"}')
    if a.files: return 0
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    G.VARIANTS['nocap'] = ([(CAP_OLD, CAP_NEW)], [])
    bins = {b: G.build(b, work / 'build', a.cc) for b in G.CLAIM + ('nocap',)}
    recs, fails = {}, []
    for name in a.only or ORDER:
        r = recs[name] = record(name, bins, work)
        fails += r['fails']
        print(f"{name}: {len(r['caps'])} captures, lethal {r['hit']}, nocap differs at {r['k']}, blows {r['blows']}")
    if fails: print('\n'.join(['FAILURES:'] + fails)); return 1
    if a.only: return 0
    text, res = state_text(recs), results(recs, a.cc)
    if a.check:
        old = json.loads(RESULTS.read_text()) if RESULTS.exists() else {}
        old.pop('generated_with', None); res.pop('generated_with')
        bad = ([] if STATE.exists() and STATE.read_text() == text else [STATE.name]) + ([] if old == res else [RESULTS.name])
        if bad: print(f'differs from the regenerated text: {", ".join(bad)}'); return 1
        print(f'{STATE.name} and {RESULTS.name} match ({len(recs)} items)')
    else:
        STATE.write_text(text); RESULTS.write_text(json.dumps(res, indent=1) + '\n')
        print(f'wrote {STATE.name} and {RESULTS.name}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
