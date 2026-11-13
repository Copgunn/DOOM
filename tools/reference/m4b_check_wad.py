#!/usr/bin/env python3
"""Independent verifier for the M4b synthetic mechanism WADs (stdlib only; does not import
the generator m4b_wad.py, the M4a generator or the room builders).

    m4b_check_wad.py WAD [--name NAME]     exits nonzero on the first violated rule
    m4b_check_wad.py --check               every WAD of tests/reference/m4b_manifest.json and
                                           tests/reference/m4b_manifest2.json

The lump parser and the format rules are the frozen M4a checker's (m4a_check_wad.Map and
mobjinfo, imported read-only: directory, resources, textures, switches, record sizes, index
ranges, ML_TWOSIDED, REJECT and BLOCKMAP sizes, the map lumps after the slot). On top of
them this file checks, from the parsed map and the genuine linuxdoom-1.10/info.c table:

- things: exactly one player 1 start, not the last THING; every type in info.c; the spawned
  counts on gameskill 2 and 3 in single player (P_SpawnMapThing, p_mobj.c:740-751) equal the
  manifest's; every thing spawned on either skill fits its sector's height, and no player,
  shootable thing touches a one-sided wall;
- lines: every special in M4a's part-1 set or 97 (WR teleport, p_spec.c:904-907); every
  tagged special's tag on some sector; every 97 line two-sided; the M4a switch-texture rules;
- teleports (EV_Teleport, p_telept.c:74-98): per 97 tag the tagged sectors in index order
  and the landings (doomednum 14) each holds, located through the nodes, with the skill bits;
  the landing the search picks on skills 2 and 3 (the first, in THINGS order, of the
  lowest-indexed tagged sector that holds one); per-WAD expectations in EXPECT;
- the special-line box: the largest number of special two-sided lines that one 32x32
  player box can overlap (PIT_CheckLine's bbox test, p_map.c:191-199, and a straddle for
  these axis-aligned lines, P_BoxOnLineSide), which is what one P_CheckPosition can append to
  spechit (p_map.c:240-244); 8 is the most the array holds; SPECH9 is built to reach 9;
- E1M8: the slot named by the manifest, a tag-666 sector and two Barons (A_BossDeath,
  p_enemy.c:1629-1635, 1729-1734).
"""
import json, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
sys.dont_write_bytecode = True
import m4a_check_wad as C   # noqa: E402  frozen M4a parser and format rules (read-only)
from m4a_check_wad import need, fail  # noqa: E402,F401

MANIFEST = ROOT / 'tests/reference/m4b_manifest.json'
MANIFEST2 = ROOT / 'tests/reference/m4b_manifest2.json'
SPECIALS = C.PART1 | {97}
LANDING, BARON, PLAYER1 = 14, 3003, 1
# Per-WAD expectations: the landing picked per tag on (skill 2, skill 3) as a THINGS index (None: no
# teleport), the largest special-line box count, the slot, the tag-666 sector and Baron count.
EXPECT = {
    'TELE': dict(pick={1: (4, 4), 2: (2, 2)}, box=1),
    'TELEMOVE': dict(pick={1: (1, 1), 2: (3, 3)}, box=2),
    'TFRAG': dict(pick={1: (4, 4), 2: (5, 5), 3: (6, 6)}, box=1),
    'MONTELE': dict(pick={1: (3, 3), 2: (4, 4)}, box=1),
    'SLIDEB': dict(pick={1: (3, 3)}, box=1),
    'SLIDEC': dict(pick={1: (3, 3)}, box=1),
    'BARON': dict(pick={}, box=0),
    'INFIGHT': dict(pick={}, box=0),
    'BVB': dict(pick={}, box=0),
    'BOSS8': dict(pick={1: (3, 3), 2: (4, 4), 3: (5, 5)}, box=1, boss=True),
    'BOSSM1': dict(pick={1: (3, 3), 2: (4, 4), 3: (5, 5)}, box=1, boss=True),
    'SPECH8': dict(pick={9: (None, None)}, box=8),
    'SPECH9': dict(pick={9: (None, None)}, box=9),
    'SKILL': dict(pick={1: (None, 1)}, box=1),
    # The second set (tests/reference/m4b_manifest2.json, M4b steps 5 and 7).
    'TELE2X': dict(pick={1: (1, 1), 2: (2, 2)}, box=1),
    'SAWBITE': dict(pick={}, box=0),
}


def spawned(things, skill):
    bit = 1 if skill <= 1 else 4 if skill >= 3 else 1 << (skill - 1)
    return [t for t in things if t[3] == PLAYER1 or (not t[4] & 16 and t[4] & bit)]


def counts(things):
    out = {}
    for t in things: out[str(t[3])] = out.get(str(t[3]), 0) + 1
    return dict(sorted(out.items(), key=lambda kv: int(kv[0])))


def box_max(m):
    """Largest count of special two-sided lines one 32x32 box overlaps (axis-aligned lines only)."""
    lines = [L for L in m.lines if L['special'] and L['s1'] != -1]
    if not lines: return 0
    segs = []
    for L in lines:
        (x0, y0), (x1, y1) = m.verts[L['a']], m.verts[L['b']]
        need(x0 == x1 or y0 == y1, 'special two-sided line not axis-aligned')
        segs.append((min(x0, x1), min(y0, y1), max(x0, x1), max(y0, y1)))
    def centres(vals):
        edges = sorted({v + d for v in vals for d in (-16, 16)})
        return [(a + b) / 2 for a, b in zip(edges, edges[1:])]
    xs = centres({c for s in segs for c in (s[0], s[2])})
    ys = centres({c for s in segs for c in (s[1], s[3])})
    best = 0
    for cx in xs:
        for cy in ys:
            l, r, b, t = cx - 16, cx + 16, cy - 16, cy + 16
            n = 0
            for x0, y0, x1, y1 in segs:
                if x0 == x1: n += l < x0 < r and not (t <= y0 or b >= y1)      # vertical: straddled, y overlap
                else: n += b < y0 < t and not (r <= x0 or l >= x1)              # horizontal
            best = max(best, n)
    return best


def locate(m, x, y):
    """m4a_check_wad.Map.locate, plus the one-subsector map with no node (R_PointInSubsector returns
    subsectors[0] when numnodes is 0, r_main.c:807-808)."""
    if not m.nodes:
        a, b, ang, ln, sd, off = m.segs[m.ss[0][1]]
        L = m.lines[ln]
        return m.sides[L['s1'] if sd else L['s0']]['sector']
    return m.locate(x, y)


def check(path, name, slot, expect_things):
    info = C.mobjinfo()
    m = C.Map(path, slot)
    th = m.things
    starts = [i for i, t in enumerate(th) if t[3] == PLAYER1]
    need(len(starts) == 1 and starts[0] != len(th) - 1, 'one player 1 start, not the last THING')
    need(all(t[3] in info or t[3] == PLAYER1 for t in th), 'unknown thing types')
    for sk in (2, 3):
        got = counts(spawned(th, sk))
        if expect_things is not None:
            need(got == expect_things[str(sk)], 'spawned counts on skill %d differ from the manifest' % sk)
    one_sided = [(m.verts[L['a']], m.verts[L['b']]) for L in m.lines if L['s1'] == -1]
    for t in {t for sk in (2, 3) for t in spawned(th, sk)}:
        s = m.secs[locate(m, t[0], t[1])]
        h = 56 if t[3] == PLAYER1 else info[t[3]]['height']
        need(s['ceil'] - s['floor'] >= h, 'thing %d at (%d,%d) taller than its sector' % (t[3], t[0], t[1]))
        if t[3] == PLAYER1 or info[t[3]]['flags'] & C.MF_SHOOTABLE:
            r = 16 if t[3] == PLAYER1 else info[t[3]]['radius']
            need(not any(C.box_meets(p, q, t[0], t[1], r) for p, q in one_sided), 'thing %d at (%d,%d) touches a wall' % (t[3], t[0], t[1]))
    tags = {s['tag'] for s in m.secs}
    spec = {}
    for L in m.lines:
        sp = L['special']
        if not sp: continue
        spec[sp] = spec.get(sp, 0) + 1
        need(sp in SPECIALS, 'special %d outside the M4b set' % sp)
        if sp not in C.UNTAGGED: need(L['tag'] and L['tag'] in tags, 'special %d tag %d on no sector' % (sp, L['tag']))
        if sp == 97: need(L['s1'] != -1, 'teleport line one-sided')
        side = m.sides[L['s0']]
        has_switch = any(side[k] in C.SWITCH_NAMES for k in ('top', 'mid', 'bot'))
        if sp in C.SWITCHED and L['s1'] == -1: need(has_switch, 'switch line %d without a switch texture' % sp)
    for L in m.lines:
        if L['special'] in C.SWITCHED and L['s1'] == -1: continue
        for sd in (L['s0'], L['s1']):
            if sd != -1: need(not any(m.sides[sd][k] in C.SWITCH_NAMES for k in ('top', 'mid', 'bot')), 'switch texture on a non-switch line')
    exp = EXPECT.get(name)
    need(exp is not None, 'no expectations for ' + name)
    report = []
    landing_sector = {i: locate(m, t[0], t[1]) for i, t in enumerate(th) if t[3] == LANDING}
    tele_tags = sorted({L['tag'] for L in m.lines if L['special'] == 97})
    need(set(tele_tags) == set(exp['pick']), 'teleport tags %s, expected %s' % (tele_tags, sorted(exp['pick'])))
    for tag in tele_tags:
        tagged = [i for i, s in enumerate(m.secs) if s['tag'] == tag]
        picks = []
        for sk in (2, 3):
            live = {i for i, t in enumerate(th) if t[3] == LANDING and t in spawned(th, sk)}
            pick = next((i for s in tagged for i in sorted(live) if landing_sector[i] == s), None)
            picks.append(pick)
        report.append('tag %d: tagged sectors %s, landings %s, picked on skills 2/3: %s' % (
            tag, tagged, {i: (landing_sector[i], th[i][4]) for i in landing_sector if landing_sector[i] in tagged}, picks))
        need(tuple(picks) == exp['pick'][tag], 'tag %d picks %s, expected %s' % (tag, picks, exp['pick'][tag]))
    bm = box_max(m)
    need(bm == exp['box'], 'special-line box maximum %d, expected %d' % (bm, exp['box']))
    if exp.get('boss'):
        need(any(s['tag'] == 666 for s in m.secs), 'no tag-666 sector')
        need(sum(t[3] == BARON for t in spawned(th, 2)) == 2, 'two Barons')
    print('OK %s (%s, %s): %d bytes; sectors %d, lines %d, things %d (spawned %d on skill 2, %d on skill 3), specials %s, '
          'special-line box maximum %d' % (path, name, slot, len(m.d), len(m.secs), len(m.lines), len(th), len(spawned(th, 2)),
                                           len(spawned(th, 3)), ' '.join('%dx%d' % kv for kv in sorted(spec.items())), bm))
    for r in report: print('  ' + r)


def main():
    args = sys.argv[1:]
    wads = [w for m in (MANIFEST, MANIFEST2) for w in json.loads(m.read_text())['wads']]
    if '--check' in args:
        for w in wads: check(ROOT / w['file'], w['name'], w['slot'], w['things'])
        return
    if not args or args[0].startswith('--'): sys.exit(__doc__)
    path = Path(args[0])
    rel = path.resolve().relative_to(ROOT).as_posix() if path.resolve().is_relative_to(ROOT) else str(path)
    w = {x['file']: x for x in wads}.get(rel)
    name = args[args.index('--name') + 1] if '--name' in args else (w['name'] if w else path.stem.upper())
    check(path, name, w['slot'] if w else 'E1M1', w['things'] if w else None)


if __name__ == '__main__':
    main()
