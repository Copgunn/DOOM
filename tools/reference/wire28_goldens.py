#!/usr/bin/env python3
"""Wire 28 against the genuine engine: DSIM44 state digests of the synthetic wire 28 items and of the M6a items SKILL0
and SKILL1, altered-engine variants for the self-tests, and the facts the items show. Expected values come from the
genuine linuxdoom-1.10 runner and the independent encoder, never from Rust output.

The runner is the M6a runner (tools/reference/m6a_build.py, read-only), built by wire27_goldens.build (read-only) in
three builds under --work: strict (the genuine text, -fwrapv: THE CLAIM), overflow and address (UBSan, ASan; run with
ASAN_OPTIONS=detect_leaks=0: leak detection disabled, not LeakSanitizer coverage). Every item of
tools/reference/wire28_wad.py ITEMS (one new-game level of tests/reference/wire28_wads/wire28.wad, the M6a WAD set,
--render none, the M5c recommended switches) must give identical dumps, sound and session streams and a clean exit in
the three builds; the strict dumps are projected by the independent DSIM44 encoder level_snapshot_wire28.py (and, for
skills 2 and 3, by the DSIM43 one, every capture differing in profile and context.schema only).

Variants (the altered genuine engine of the self-tests; VARIANTS): strict builds of p_inter.c with one rule removed or
moved, each an exact-once text replacement made in memory (no file changes): `nodouble` (sk_baby no longer doubles
P_GiveAmmo, p_inter.c:95-101), `nohalf` (no halving, 799-800), `half_after_thrust` (the halving after the thrust,
806-832: the cap and armour still read the halved value) and `half_after_cap` (the cap of 837-842 reads the unhalved
value and the halving follows it; the thrust still reads the halved one). Each variant must differ from the strict run of each
of its items; the first differing capture and the variant's digest there are written as V rows, so a Rust test can
show that a port behaving like the variant is caught at that capture (an altered golden, not a Rust fault).

SKILL0 and SKILL1 (tests/reference/m6a_manifest.json): the state variant of tools/reference/m6a_state.py (read-only:
build, run, check_run, levels, rows_of), whose session, sound and frame streams must equal the committed m6a goldens,
projected by the DSIM44 encoder instead of the DSIM42 one: an in-process override, declared in OVERRIDES.

Tic convention (owner): a session tic is the 0-based index of a G_Ticker, equal to the number of tics completed
before it; capture j is the state after j tics; a refusal at session tic N keeps capture N.

Outputs (written without --check; --check regenerates and compares, writing nothing):
  tests/reference/wire28_state.txt        DOOM-WIRE28-STATE-v1: per item an I row (map, skill, tics, captures, the end
                                          the port must reach: complete, exit:N (the genuine level ends after session
                                          tic N; no capture after N + 1) or escape:N (wire 28 refuses session tic N with
                                          capture N kept; the genuine run goes on), the map digest, the sequence
                                          digest), a C row per capture (the state digest of tests/reference/
                                          m6a_state.txt's definition over DSIM44), then V rows per variant and item
  tests/reference/wire28_skill_state.txt  DOOM-WIRE28-SKILL-STATE-v1: tests/reference/m6a_state.txt's L and D rows for
                                          SKILL0 and SKILL1 over DSIM44
  tests/reference/wire28_results.json     DOOM-WIRE28-RESULTS-v1: builds, variants with their replacements, overrides,
                                          tool and input digests, and per item the observed facts (pickups, hits,
                                          the escape, the stuck imps); `generated_with` is left out of --check

    wire28_goldens.py [--check] [--selftest] [--work DIR] [--cc CC]
"""
import argparse, hashlib, json, os, shutil, subprocess, sys, types
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import level_snapshot as L          # noqa: E402
import level_snapshot_m5c as M5S    # noqa: E402
import level_snapshot_m6a as S6     # noqa: E402
import level_snapshot_wire27 as E27  # noqa: E402
import level_snapshot_wire28 as E   # noqa: E402
import level_compare as K           # noqa: E402
import m6a_goldens as G             # noqa: E402  (read-only: the results with dumps_per_load)
import m6a_state as ST              # noqa: E402  (read-only: the state variant, levels, rows)
import wire27_goldens as W27        # noqa: E402  (read-only: build, run)
import wire28_wad as WAD            # noqa: E402
from m3h9_geometry import enum_index  # noqa: E402

STATE, STATE_SCHEMA = ROOT / 'tests/reference/wire28_state.txt', 'DOOM-WIRE28-STATE-v1'
SKILL, SKILL_SCHEMA = ROOT / 'tests/reference/wire28_skill_state.txt', 'DOOM-WIRE28-SKILL-STATE-v1'
RESULTS, SCHEMA = ROOT / 'tests/reference/wire28_results.json', 'DOOM-WIRE28-RESULTS-v1'
TOOLS = ['wire28_goldens.py', 'wire28_wad.py', 'level_snapshot_wire28.py']
CLAIM = ('strict', 'overflow', 'address')
SKILL_ITEMS = ['SKILL0', 'SKILL1']
MT = {enum_index('mobjtype_t', n): n[3:] for n in ('MT_PLAYER', 'MT_POSSESSED', 'MT_SHOTGUY', 'MT_TROOP',
                                                    'MT_SERGEANT', 'MT_BARREL')}
S_PLAY_PAIN = enum_index('statenum_t', 'S_PLAY_PAIN')
S_BEXP4 = enum_index('statenum_t', 'S_BEXP4')                 # A_Explode (info.c)
BABY_HALF = '    if (player && gameskill == sk_baby)\n\tdamage >>= 1; \t// take half damage in trainer mode\n'
THRUST_END = '    }\n    \n    // player specific\n'
CAP_END = '\t    damage = target->health - 1;\n\t}\n'
VARIANTS = {   # name: (exact-once replacements of p_inter.c, items)
    'nodouble': ([('    if (gameskill == sk_baby\n\t|| gameskill == sk_nightmare)',
                   '    if (gameskill == sk_nightmare\n\t|| gameskill == sk_nightmare)')], ['AMMO0', 'DROP0']),
    'nohalf': ([(BABY_HALF, '\n\n')], ['HURT0', 'SPLASH0', 'BITE0']),
    'half_after_thrust': ([(BABY_HALF, '\n\n'), (THRUST_END, '    }\n    if (player && gameskill == sk_baby)\n'
                                                            '\tdamage >>= 1;\n    // player specific\n')], ['HURT0', 'BITE0']),
    'half_after_cap': ([(BABY_HALF, '    w28_full = damage;\n' + BABY_HALF[:-1]),
                        ('\t// end of game hell hack\n', '\tif (gameskill == sk_baby) damage = w28_full;\n'),
                        (CAP_END, CAP_END + '\tif (gameskill == sk_baby)\n\t    damage >>= 1;\n'),
                        ('    int\t\ttemp;\n', '    int\t\ttemp;\n    int\t\tw28_full;\n')], ['BITE0']),
}
OVERRIDES = [
    dict(what='m6a_state.S (the DSIM42 projection module) replaced by a namespace whose Projector is '
              'level_snapshot_wire28.Projector and VERSION 44, around the m6a_state.levels calls of SKILL0 and SKILL1',
         why='m6a_state projects with DSIM42; wire 28\'s schema is DSIM44 (DSIM43 field for field, three values naming '
             'the wire), and its projector admits skills 0 and 1'),
    dict(what='wire27_goldens.BUILDS given one entry per variant and wire27_goldens.sources wrapped to apply the '
              'variant\'s replacements to p_inter.c, around each variant build',
         why='a variant is the strict build of the genuine text with one rule of p_inter.c removed or moved (VARIANTS)'),
]
for o in OVERRIDES: o['scope'] = 'in-process only: no file is edited, and the original is restored'
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()


def build(name, out, cc):
    if name not in VARIANTS: return W27.build(name, out, cc)
    plain = W27.sources

    def sources(build_name):
        src = plain('strict')
        for old, new in VARIANTS[name][0]:
            if src['p_inter.c'].count(old) != 1: sys.exit(f'variant {name}: a replacement does not apply once')
            src['p_inter.c'] = src['p_inter.c'].replace(old, new)
        return src
    W27.BUILDS[name], W27.sources = ('strict', False, False), sources
    try: return W27.build(name, out, cc)
    finally: W27.sources = plain; del W27.BUILDS[name]


def item(name):
    return dict(name=name, map=WAD.ITEMS[name][0], skill=WAD.ITEMS[name][1], wads=[ROOT / w for w in WAD.WADSET],
                lmp_path=ROOT / f'{WAD.SCRIPT_DIR}/{name}.lmp')


def digest(version, fields):
    """m6a_state.txt's state digest of one capture over the schema `version`."""
    body, _ = K.stripped(version, fields)
    return sha(body + S6.framed([(p, v) for p, v in fields if p.startswith('events.sounds.')]))


def project(r, it):
    """[(capture, digest)], the map digest and the dumps of one strict run (DSIM44; DSIM43 alongside for skill >= 2)."""
    loads = S6.rows(r['session'], 'L')
    kind, p, _h, _s = S6.start_from_rows(r['session'], r['sound'], 0, str(it['wads'][0]))
    assert kind == 'new' and len(loads) == 1, (kind, loads)
    mapdata = L.MapData(str(it['wads'][0]), f"E1M{it['map']}", ST.extra_lumps(it))
    sounds = ST.Shifted(M5S.SoundStream(text=r['sound']), max(loads[0]['tic'], 0))
    proj = [E.Projector(mapdata, it['skill'], it['map'], sounds, (kind, dict(p)))]
    if it['skill'] >= 2: proj.append(E27.Projector(mapdata, it['skill'], it['map'], sounds, (kind, dict(p))))
    names = ['load.txt'] + sorted(n for n in os.listdir(r['dumps']) if n.startswith('tic_'))
    caps, dumps, mapg = [], [], None
    for j, n in enumerate(names):
        nxt = L.Dump(str(r['dumps'] / names[j + 1])) if j + 1 < len(names) else None
        d = L.Dump(str(r['dumps'] / n))
        fs = [pr.step(d, nxt)[0] for pr in proj]
        if len(fs) == 2: E.compare43(*fs)
        mapg = mapg or sha(S6.framed([x for x in fs[0] if x[0].startswith('map.')]))
        if (int(n[4:9]) + 1 if j else 0) != j: sys.exit(f"{it['name']}: captures are not consecutive tics")
        caps.append(digest(E.VERSION, fs[0])); dumps.append(d)
    return caps, mapg, dumps


# ------------------------------------------------------------------ observed facts (module text) -------------
def mobjs(d, kind=None):
    return [m for k, m in d.thinkers if k == 'M' and (kind is None or MT.get(m['type']) == kind)]


def body(d):
    return mobjs(d, 'PLAYER')[0]


def pickups(dumps):
    """Every tic the player's ammunition, maxammo or pending weapon changes: [tic, ammo0-3, maxammo0, ready, pending]."""
    keys = ('ammo0', 'ammo1', 'ammo2', 'ammo3', 'maxammo0', 'readyweapon', 'pendingweapon')
    out, prev = [], None
    for j, d in enumerate(dumps):
        cur = [d.players[0][k] for k in keys]
        if prev is not None and cur != prev: out.append([j - 1] + cur)
        prev = cur
    return out


def hits(dumps):
    """Every tic the player is hurt or enters S_PLAY_PAIN: [tic, health, armour before and after, source, pain] with
    the source the attacker's type (claw when an imp is within 64 units: P_CheckMeleeRange, p_enemy.c), or `none`."""
    out = []
    for j in range(1, len(dumps)):
        a, b = dumps[j - 1].players[0], dumps[j].players[0]
        pain = body(dumps[j])['state'] == S_PLAY_PAIN and body(dumps[j - 1])['state'] != S_PLAY_PAIN
        if (a['health'], a['armorpoints']) == (b['health'], b['armorpoints']) and not pain: continue
        if b['health'] > a['health']: continue
        att = {m['idx']: m for m in mobjs(dumps[j])}.get(b['attacker'])
        src = 'none' if att is None else MT.get(att['type'], att['type'])
        if src == 'TROOP':
            me = body(dumps[j])
            src = 'TROOP:claw' if max(abs(att['x'] - me['x']), abs(att['y'] - me['y'])) < 64 << 16 else 'TROOP:fireball'
        out.append([j - 1, a['health'], b['health'], a['armorpoints'], b['armorpoints'], src, pain])
    return out


def inside(x, y):
    """The point inside wire28_wad.RING (exact, in 16.16 units): the parity of the crossings of a ray to +x."""
    ring, c = WAD.RING, False
    for (ax, ay), (bx, by) in zip(ring, ring[1:] + ring[:1]):
        ax, ay, bx, by = ax << 16, ay << 16, bx << 16, by << 16
        if (ay > y) != (by > y) and x < ax + (bx - ax) * (y - ay) / (by - ay): c = not c
    return c


def room_facts(dumps):
    """ESCAPE and SMALL: per embedded imp (by spawn point) the tics it stood at its spawn point and whether it ever
    moved while inside the room, and the first tic whose move takes a body out of the room, with the barrels that
    exploded in that tic (entered S_BEXP4) and the body before and after."""
    out, stood, moved = dict(escape=None), {}, set()
    for j, d in enumerate(dumps):
        for m in mobjs(d, 'TROOP'):
            key = f"{m['sp_x']},{m['sp_y']}"
            if (m['x'] >> 16, m['y'] >> 16, m['x'] & 0xffff, m['y'] & 0xffff) == (m['sp_x'], m['sp_y'], 0, 0):
                stood[key] = stood.get(key, 0) + 1
            elif inside(m['x'], m['y']): moved.add(key)
            elif out['escape'] is None:
                p = next(x for x in mobjs(dumps[j - 1], 'TROOP') if (x['sp_x'], x['sp_y']) == (m['sp_x'], m['sp_y']))
                out['escape'] = dict(tic=j - 1, spawn=key, before=[p['x'] / 65536, p['y'] / 65536, p['health']],
                                     after=[m['x'] / 65536, m['y'] / 65536, m['health']],
                                     momentum_after=[m['momx'] / 65536, m['momy'] / 65536],
                                     blasts=[b['idx'] for b in mobjs(d, 'BARREL') if b['state'] == S_BEXP4 and b['tics'] == 10])
    out['stood'] = dict(sorted(stood.items()))
    out['moved_inside'] = sorted(moved)
    return out


def facts(name, dumps):
    f = dict(hits=hits(dumps))
    if name.startswith(('AMMO', 'DROP')): f['pickups'] = pickups(dumps)
    if WAD.ITEMS[name][0] in (6, 7): f.update(room_facts(dumps))
    return f


# ------------------------------------------------------------------ runs -----------------------------------
def item_record(it, bins, work):
    name, fails = it['name'], []
    runs = {b: W27.run(bins[b], it, work / 'runs' / f'{name}_{b}') for b in CLAIM}
    s = runs['strict']
    if s['code'] or W27.UB.search(s['stderr']) or not s['marker']: fails.append(f'{name}: strict exit {s["code"]}')
    for b in CLAIM[1:]:
        r = runs[b]
        if r['code'] or (r['snapshots'], r['sound'], r['session']) != (s['snapshots'], s['sound'], s['session']):
            fails.append(f'{name}: {b} exit {r["code"]} or streams differ from strict')
    caps, mapg, dumps = project(s, it)
    tics = WAD.lump(name)[1]
    f = facts(name, dumps)
    end = f'escape:{f["escape"]["tic"]}' if f.get('escape') else 'complete' if len(caps) == tics + 1 else f'exit:{len(caps) - 2}'
    shutil.rmtree(work / 'runs', ignore_errors=True)
    return dict(name=name, tics=tics, captures=caps, mapg=mapg, end=end, facts=f, fails=fails)


def variant_rows(bins, recs, work):
    """[(variant, item, first differing capture, the variant's digest there)]; a variant equal to strict fails."""
    out, fails = [], []
    for v, (_r, names) in VARIANTS.items():
        for name in names:
            it = item(name)
            r = W27.run(bins[v], it, work / 'runs' / f'{name}_{v}')
            caps, _m, _d = project(r, it)
            base = recs[name]['captures']
            k = next((j for j, (a, b) in enumerate(zip(base, caps)) if a != b), None)
            if k is None: fails.append(f'variant {v}: {name} equals the genuine run'); continue
            out.append((v, name, k, caps[k]))
            shutil.rmtree(work / 'runs', ignore_errors=True)
    return out, fails


def skill_rows(work, cc):
    """m6a_state.txt's rows of SKILL0 and SKILL1 over DSIM44 (the declared override), and the failures."""
    binary = ST.build(work / 'skill', cc)
    results = json.loads(G.RESULTS.read_text())
    items = {i['name']: i for i in G.items() if i['name'] in SKILL_ITEMS}
    shim = types.SimpleNamespace(**{k: getattr(S6, k) for k in dir(S6) if not k.startswith('__')})
    shim.Projector, shim.VERSION = E.Projector, E.VERSION
    rows, fails, plain = [], [], ST.S
    ST.S = shim
    try:
        for name in SKILL_ITEMS:
            r = ST.run(binary, items[name], work / 'skill_runs' / name)
            fails += ST.check_run(r, items[name], results)
            lv = ST.levels(r, items[name])
            rows += ST.rows_of(name, lv)
            print(f'{name}: {len(lv)} levels, {sum(len(v["captures"]) for v in lv)} captures (DSIM44)')
    finally:
        ST.S = plain
    shutil.rmtree(work / 'skill_runs', ignore_errors=True)
    return rows, fails


def state_text(recs, variants):
    head = [f'#schema,{STATE_SCHEMA}',
            '#generator,tools/reference/wire28_goldens.py: the genuine M6a runner (strict build) and the independent DSIM44 '
            'encoder tools/reference/level_snapshot_wire28.py; never from Rust output',
            '#tics,a session tic is the 0-based index of a G_Ticker; capture j is the state after j tics; a refusal at '
            'session tic N keeps capture N',
            '#state,SHA-256 of the DSIM44 capture without map.* and events.* (level_compare.stripped framing) followed by '
            'its events.sounds.* fields (per-field framing); map: SHA-256 of the map.* fields of capture 0',
            '#end,complete | exit:N (the level ends after session tic N) | escape:N (wire 28 refuses session tic N)',
            '#columns,I,item,map,skill,tics,captures,end,map_sha256,sequence_sha256',
            '#columns,C,item,capture,state_sha256',
            '#columns,V,variant,item,first_differing_capture,variant_state_sha256']
    rows = []
    for r in recs.values():
        seq = sha(b''.join(bytes.fromhex(c) for c in r['captures']))
        rows.append(f"I,{r['name']},{WAD.ITEMS[r['name']][0]},{WAD.ITEMS[r['name']][1]},{r['tics']},"
                    f"{len(r['captures'])},{r['end']},{r['mapg']},{seq}")
        rows += [f"C,{r['name']},{j},{d}" for j, d in enumerate(r['captures'])]
    rows += [f'V,{v},{n},{k},{d}' for v, n, k, d in variants]
    return '\n'.join(head + rows) + '\n'


def skill_text(rows):
    head = [f'#schema,{SKILL_SCHEMA}',
            '#generator,tools/reference/wire28_goldens.py: the genuine M6a runner (m6a_state.py\'s state variant, strict) '
            'and the independent DSIM44 encoder tools/reference/level_snapshot_wire28.py; never from Rust output',
            '#rows,the L and D rows of tests/reference/m6a_state.txt (DOOM-M6A-STATE-v1) over DSIM44 for SKILL0 and SKILL1']
    return '\n'.join(head + [l for l in ST.results_text({}).splitlines() if l.startswith('#columns') or l.startswith('#points')]
                     + rows) + '\n'


def results(recs, variants, cc):
    digest_of = lambda p: sha((ROOT / p).read_bytes())
    return dict(
        schema=SCHEMA, revision=W27.base.REVISION,
        claim='every state digest is the genuine engine\'s strict build projected by the independent DSIM44 encoder; '
              'the overflow and address builds gave identical dumps, sound and session streams',
        builds={b: W27.base.VARIANTS[W27.BUILDS[b][0]] for b in CLAIM}, asan_options=W27.ASAN_NOTE,
        variants={v: dict(replacements=[dict(original=o, replacement=n) for o, n in r], items=i)
                  for v, (r, i) in VARIANTS.items()},
        variant_first_differences=[dict(variant=v, item=n, capture=k) for v, n, k, _d in variants],
        in_process_overrides=OVERRIDES,
        tools_sha256={t: digest_of(f'tools/reference/{t}') for t in TOOLS},
        inputs_sha256={p: digest_of(p) for p in WAD.WADSET + [f'{WAD.SCRIPT_DIR}/{n}.lmp' for n in WAD.ITEMS]},
        generated_with=dict(compiler=subprocess.run([cc, '--version'], capture_output=True, text=True).stdout.split('\n')[0]),
        items={r['name']: dict(map=WAD.ITEMS[r['name']][0], skill=WAD.ITEMS[r['name']][1], tics=r['tics'],
                               captures=len(r['captures']), end=r['end'], what=WAD.ITEMS[r['name']][3],
                               observed=r['facts']) for r in recs.values()})


def selftest(bins, work, recs):
    """The digests can fail: an extra P_Random draw before session tic 40 of HURT0 changes capture 41 and later."""
    it = item('HURT0')
    r = W27.run(bins['strict'], it, work / 'self_draw', extra=['--selftest-extra-draw', '40'])
    caps, _m, _d = project(r, it)
    differ = [j for j, (a, b) in enumerate(zip(caps, recs['HURT0']['captures'])) if a != b]
    shutil.rmtree(work / 'self_draw', ignore_errors=True)
    return [('an extra P_Random draw before session tic 40 of HURT0', f'first differing capture {differ[:1]}',
             bool(differ) and differ[0] == 41)]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true'); ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--work', type=Path, default=Path.home() / 'scratch' / 'wire28' / 'goldens')
    ap.add_argument('--cc', default='cc')
    a = ap.parse_args()
    work = a.work.resolve()
    if work == ROOT or ROOT in work.parents: sys.exit('--work must lie outside the repository')
    bins = {b: build(b, work / 'build', a.cc) for b in CLAIM + tuple(VARIANTS)}
    recs, fails = {}, []
    for name in WAD.ITEMS:
        r = recs[name] = item_record(item(name), bins, work)
        fails += r['fails']
        print(f"{name}: {r['tics']} tics, {len(r['captures'])} captures, end {r['end']}, {len(r['facts']['hits'])} hits"
              + (f", escape {r['facts']['escape']}" if r['facts'].get('escape') else ''))
    variants, f = variant_rows(bins, recs, work)
    fails += f
    for v, n, k, _d in variants: print(f'variant {v}: {n} first differs at capture {k}')
    skill, f = skill_rows(work, a.cc)
    fails += f
    if a.selftest:
        for what, detail, ok in selftest(bins, work, recs):
            print(f'selftest {"pass" if ok else "FAIL"}: {what}: {detail}')
            if not ok: fails.append(f'selftest: {what}')
    if fails: print('\n'.join(['FAILURES:'] + fails)); return 1
    texts = {STATE: state_text(recs, variants), SKILL: skill_text(skill)}
    res = results(recs, variants, a.cc)
    print(f"compiler: {res['generated_with']['compiler']}; {W27.ASAN_NOTE}")
    if a.check:
        old = json.loads(RESULTS.read_text()) if RESULTS.exists() else {}
        old.pop('generated_with', None); res.pop('generated_with')
        bad = [p.name for p, t in texts.items() if not p.exists() or p.read_text() != t] + ([] if old == res else [RESULTS.name])
        if bad: print(f'differs from the regenerated text: {", ".join(bad)}'); return 1
        print(f'{", ".join(p.name for p in texts)} and {RESULTS.name} match ({len(recs)} items, {len(SKILL_ITEMS)} skill items)')
    else:
        for p, t in texts.items(): p.write_text(t)
        RESULTS.write_text(json.dumps(res, indent=1) + '\n')
        print(f'wrote {", ".join(p.name for p in texts)} and {RESULTS.name}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
