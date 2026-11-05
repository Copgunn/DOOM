#!/usr/bin/env python3
"""Synthetic mechanism-group WADs for the M4a whole-engine runs (decision H6).

Writes nine small, fully synthetic IWADs under tests/reference/m4a_wads/, one per
mechanism group of M4a part 1, each holding everything the headless genuine
linuxdoom-1.10 engine needs to load its map (slot E1M1 by default) in `registered` mode:

    doors    2, 16, 76, 86, 90, 103, 31, 26/27/28, 32/33/34 and 46 (door line and GR switch)
    floors   5, 91, 18, 23, 82, 70, 98, a blocked raiseFloor, the 21-neighbour comb
    plats    62 SR lift, 20 S1 and 22 W1 raiseToNearestAndChange, a blocked 22
    stairs   7 S1 and 8 W1 stairs (decoy flat, busy neighbour), a blocked stair, donut 9
    exits    35 light, 11 exit, 51 secret exit, sector special 11, a barrel death
    weapons  chainsaw 2005, rocket launcher 2003 with rockets 2010/2046, a sky alcove
    items    backpack, soulsphere, light amp, map, invisibility, suit, keys; specials 7/16
    spectre  Spectre 58 fights and infighting
    scale    12 monsters, 83 pickups, 24 barrels, 40 statics (no teleport landing 14), heights -640..1100, a
             608-unit lift and a 3,680-unit linedef

The lump framing and the game data are the frozen M3h9c generator's (m3h9c_wad.py,
imported read-only: TEXTURES, FLATS, the blank patch, PNAMES, TEXTURE1, PLAYPAL,
COLORMAP); no texture or flat is added. The geometry goes through the repository's
builder chain unchanged (m3h9b_geometry.room, i.e. m3h6..m3h9b: automatic BSP, SEGS,
SSECTORS, NODES, exact-clip BLOCKMAP, zero REJECT). What this file adds:

- A grid painter. Each sector is a list of axis-aligned rectangles painted in
  (layer, declaration) order, a later paint winning a cell; the boundary between
  different owners becomes directed unit edges with the owner on the right (clockwise
  outer rings with y up, counter-clockwise holes), merged into maximal collinear runs
  with one neighbour each, so both sides of a shared run carry the same two vertices.
  Each ring starts at its top-left vertex (largest y, then smallest x). Sectors are
  emitted in declaration order, so the front of every two-sided line is the sector
  declared first (the builder's rule), and `sec->lines` (P_GroupLines,
  p_setup.c:519-575) lists a sector's lines in line-index order: the rings of
  lower-numbered sectors first, then the sector's own rings from the top-left vertex.
- Lines by name: `special(front, back, at, special, tag)` puts a special on the unique
  run of `front`'s rings bordering `back` (None: a one-sided wall) that contains `at`.
- Side textures: one-sided STARTAN2; two-sided `-`; BIGDOOR2 on the upper textures of
  every line bordering a door sector; PLAT1 on the lower textures of two-sided lines
  whose floors differ; SW1COMM on the front middle texture of every one-sided S1/SR
  line, SW1BRCOM for the one-sided GR 46 target (both in alphSwitchList, p_switch.c:46-90,
  so P_ChangeSwitchTexture finds them, p_switch.c:197-256).
- Two-pass checks: the comb's 21st qualifying neighbour by line index gets the low
  floor (built once with every cell high, inspected, rebuilt); the stair line order
  (a3|bA before a3|a4) and the donut layout are asserted on the built map.

The independent checker tools/reference/m4a_check_wad.py re-parses every WAD and checks
the formats and the per-mechanism invariants without importing this file.
tests/reference/m4a_manifest.json (schema DOOM-M4A-MANIFEST-v1) names each WAD with its
slot, SHA-256 and spawned-thing counts (written here with --manifest, checked by --check)
and its scripts with their C expectations (authored, checked by m4a_reference.py).

    m4a_wad.py [--out-dir DIR] [--only NAME] [--slot E1M1]   write the WADs
    m4a_wad.py --list                                        name, file, lines, things
    m4a_wad.py --manifest                                    also refresh the manifest's
                                                             wad fields (sha256, things)
    m4a_wad.py --check                                       regenerate in memory, compare
                                                             with the committed WADs and the
                                                             manifest's wad fields, then run
                                                             m4a_check_wad.py on each WAD
"""
import argparse, bisect, hashlib, json, struct, subprocess, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m3h9c_wad as W            # noqa: E402  frozen M3h9c game data and lump framing (read-only)
import m3h9b_geometry as G       # noqa: E402  room() (the M3h6..M3h9b builder chain)

OUT_DIR = ROOT / 'tests/reference/m4a_wads'
MANIFEST = ROOT / 'tests/reference/m4a_manifest.json'
CHECKER = ROOT / 'tools/reference/m4a_check_wad.py'
MANIFEST_SCHEMA = 'DOOM-M4A-MANIFEST-v1'
MAP_LUMPS = ['THINGS', 'LINEDEFS', 'SIDEDEFS', 'VERTEXES', 'SEGS', 'SSECTORS', 'NODES', 'SECTORS', 'REJECT', 'BLOCKMAP']

# DoomEd numbers (info.c mobjinfo doomednum fields)
PLAYER1, ZOMBIE, SHOTGUY, IMP, SPECTRE = 1, 3004, 9, 3001, 58
BARREL = 2035
SHOTGUN, CHAINSAW, LAUNCHER = 2001, 2005, 2003
CLIP, CLIPBOX, SHELLS, SHELLBOX, ROCKET, ROCKETBOX, BACKPACK = 2007, 2048, 2008, 2049, 2010, 2046, 8
STIMPACK, MEDIKIT, HEALTHBONUS, ARMORBONUS, GREENARMOR, BLUEARMOR = 2011, 2012, 2014, 2015, 2018, 2019
SOULSPHERE, INVIS, SUIT, ALLMAP, LIGHTAMP = 2013, 2024, 2025, 2026, 2045
BLUECARD, YELLOWCARD, REDCARD = 5, 6, 13
CANDLE, FIRESTICK, LAMP, PILLAR, CANDELABRA, GIBS, DEADPLAYER, MESS = 34, 46, 2028, 48, 35, 24, 15, 10
ALL_SKILLS, AMBUSH = 7, 8
EAST, NORTH, WEST, SOUTH = 0, 90, 180, 270

# Line specials by class (p_spec.c P_CrossSpecialLine, p_switch.c P_UseSpecialLine, p_spec.c
# P_ShootSpecialLine): switch-textured one-sided walls carry the S1/SR/GR ones.
SWITCHED = {7, 9, 11, 18, 20, 23, 51, 62, 70, 103}      # S1 and SR
GUN = 46                                                # GR (P_ShootSpecialLine)
DOOR_SPECIALS = {1, 26, 27, 28, 31, 32, 33, 34, 46}     # manual / gun doors on a hall|door edge
PART1 = {2, 5, 7, 8, 9, 11, 16, 18, 20, 22, 23, 26, 27, 28, 31, 32, 33, 34, 35, 46, 51, 62, 70, 76, 82, 86, 90, 91, 98, 103}


def rect(x0, y0, x1, y1):
    assert x0 < x1 and y0 < y1 and all(v % 16 == 0 for v in (x0, y0, x1, y1)), (x0, y0, x1, y1)
    return (x0, y0, x1, y1)


class Plan:
    """One map: sectors (rectangles, heights, attributes), named special lines, THINGS."""

    def __init__(self, name):
        self.name, self.sectors, self.specials, self.things, self.doors = name, [], [], [], set()

    def sector(self, label, rects, floor=0, ceil=128, light=160, special=0, tag=0, fpic='FLOOR7_1', cpic='CEIL3_5', layer=0, door=False):
        self.sectors.append(dict(label=label, rects=[rect(*r) for r in rects], floor=floor, ceil=ceil, layer=layer,
                                 attrs=dict(light=light, special=special, tag=tag, floor=fpic, ceiling=cpic)))
        if door: self.doors.add(len(self.sectors) - 1)
        return len(self.sectors) - 1

    def special(self, front, back, at, special, tag=0):
        assert special in PART1, special
        self.specials.append((front, back, at, special, tag))

    def thing(self, x, y, angle, kind, options=ALL_SKILLS):
        self.things.append((x, y, angle, kind, options))

    def index(self, label):
        return [s['label'] for s in self.sectors].index(label)


# ---------------------------------------------------------------- the grid painter -------
def paint(plan):
    """Cell owners on the grid of every rectangle edge: owner[i][j] for x cell i, y cell j."""
    xs = sorted({v for s in plan.sectors for r in s['rects'] for v in (r[0], r[2])})
    ys = sorted({v for s in plan.sectors for r in s['rects'] for v in (r[1], r[3])})
    owner = [[None] * (len(ys) - 1) for _ in range(len(xs) - 1)]
    for si in sorted(range(len(plan.sectors)), key=lambda k: (plan.sectors[k]['layer'], k)):
        for x0, y0, x1, y1 in plan.sectors[si]['rects']:
            for i in range(bisect.bisect_left(xs, x0), bisect.bisect_left(xs, x1)):
                for j in range(bisect.bisect_left(ys, y0), bisect.bisect_left(ys, y1)):
                    owner[i][j] = si
    seen = {o for col in owner for o in col}
    missing = [plan.sectors[k]['label'] for k in range(len(plan.sectors)) if k not in seen]
    assert not missing, ('sector painted over completely', missing)
    return xs, ys, owner


def rings(plan):
    """Per sector: [outer ring, holes...], each a list of (x, y) vertices starting at the
    top-left vertex, plus the neighbour of every edge (None for a wall)."""
    xs, ys, owner = paint(plan)
    nx, ny = len(xs) - 1, len(ys) - 1
    at = lambda i, j: owner[i][j] if 0 <= i < nx and 0 <= j < ny else None
    out = {}
    for i in range(nx):
        for j in range(ny):
            a = owner[i][j]
            if a is None: continue
            x0, x1, y0, y1 = xs[i], xs[i + 1], ys[j], ys[j + 1]
            for p, q, nb in (((x0, y1), (x1, y1), at(i, j + 1)), ((x1, y1), (x1, y0), at(i + 1, j)),
                             ((x1, y0), (x0, y0), at(i, j - 1)), ((x0, y0), (x0, y1), at(i - 1, j))):
                if nb == a: continue
                assert p not in out.setdefault(a, {}), ('pinch: two boundary edges leave one vertex', plan.sectors[a]['label'], p)
                out[a][p] = (q, nb)
    result = []
    for s in range(len(plan.sectors)):
        edges, loops = dict(out[s]), []
        while edges:
            start = min(edges)
            loop, p = [], start
            while True:
                q, nb = edges.pop(p)
                loop.append((p, q, nb))
                p = q
                if p == start: break
            loops.append(merge(loop))
        area = lambda r: sum(r[k][0][0] * r[(k + 1) % len(r)][0][1] - r[(k + 1) % len(r)][0][0] * r[k][0][1] for k in range(len(r)))
        outer = [r for r in loops if area(r) < 0]
        holes = sorted((r for r in loops if area(r) > 0), key=lambda r: (-r[0][0][1], r[0][0][0]))
        assert len(outer) == 1, ('sector must be one connected region', plan.sectors[s]['label'], len(outer))
        result.append(outer + holes)
    return result


def merge(loop):
    """Maximal collinear runs with one neighbour, as [(vertex, neighbour of the edge from it)],
    rotated to start at the top-left vertex."""
    direction = lambda e: ((e[1][0] > e[0][0]) - (e[1][0] < e[0][0]), (e[1][1] > e[0][1]) - (e[1][1] < e[0][1]))
    key = lambda e: (direction(e), e[2])
    n = len(loop)
    first = next(k for k in range(n) if key(loop[k]) != key(loop[k - 1]))
    runs = []
    for k in range(n):
        e = loop[(first + k) % n]
        if runs and key(runs[-1][1]) == key(e): runs[-1][1] = e
        else: runs.append([e, e])
    ring = [(a[0], a[2]) for a, _ in runs]
    top_left = min(range(len(ring)), key=lambda k: (-ring[k][0][1], ring[k][0][0]))
    return ring[top_left:] + ring[:top_left]


def build_map(plan):
    """The builder chain's map for a plan, plus the sector rings used."""
    rs = rings(plan)
    sectors = []
    for s, sr in zip(plan.sectors, rs):
        sectors.append(([[v for v, _ in r] for r in sr], s['floor'], s['ceil'], dict(s['attrs'])))
    lines = {}
    for front, back, point, special, tag in plan.specials:
        if back is not None:
            assert front < back, ('the front of a two-sided line is the lower sector index', front, back)
        found = []
        for r in rs[front]:
            for k, (v, nb) in enumerate(r):
                w = r[(k + 1) % len(r)][0]
                if nb == back and on_segment(point, v, w): found.append((v, w))
        assert len(found) == 1, ('special line not found exactly once', plan.name, front, back, point, found)
        assert found[0] not in lines, ('two specials on one line', found[0])
        lines[found[0]] = (special, tag)
    m = G.room(sectors, portals={}, reject=(), tree=None, segment=4096, lines=lines)
    m['rings'] = rs
    return m


def on_segment(p, a, b):
    return (min(a[0], b[0]) <= p[0] <= max(a[0], b[0]) and min(a[1], b[1]) <= p[1] <= max(a[1], b[1]) and
            (b[0] - a[0]) * (p[1] - a[1]) == (b[1] - a[1]) * (p[0] - a[0]))


def sector_lines(m, s):
    """`sec->lines` of sector s in P_GroupLines order (line index): (line index, other sector)."""
    return [(i, l['back'] if l['front'] == s else l['front']) for i, l in enumerate(m['lines']) if s in (l['front'], l['back'])]


def fix_sides(plan, m):
    """Side textures (see the module docstring); every name is in m3h9c_wad.TEXTURES."""
    side = bytearray(m['lumps']['SIDEDEFS'])
    for l, (special, _) in zip(m['lines'], m['line_specials']):
        two = l['back'] is not None
        door = two and (special in DOOR_SPECIALS or l['front'] in plan.doors or l['back'] in plan.doors)
        step = two and plan.sectors[l['front']]['floor'] != plan.sectors[l['back']]['floor']
        for k, sd in enumerate(l['sides']):
            if sd < 0: continue
            top, bot, mid = ('BIGDOOR2' if door else '-'), ('PLAT1' if step else '-'), ('-' if two else 'STARTAN2')
            if not two and k == 0 and special in SWITCHED: mid = 'SW1COMM'
            if not two and k == 0 and special == GUN: mid = 'SW1BRCOM'
            for off, name in ((4, top), (12, bot), (20, mid)):
                assert name == '-' or name in W.TEXTURES, name
                side[sd * 30 + off:sd * 30 + off + 8] = W.name8(name)
    m['lumps']['SIDEDEFS'] = bytes(side)


# ---------------------------------------------------------------- the nine maps ----------
def doors():
    """Hall with eight manual doors (north), six remote doors (south) and two switch alcoves."""
    p = Plan('doors')
    hall = p.sector('HALL', [(-640, -256, 640, 256)])
    xs = [-576 + 144 * i for i in range(8)]
    md = [p.sector(f'MD{i}', [(a, 256, a + 64, 320)], ceil=0, tag=8 if i == 7 else 0, door=True) for i, a in enumerate(xs)]
    for i, a in enumerate(xs): p.sector(f'CL{i}', [(a, 320, a + 64, 448)], light=128)
    bs = [-544, -352, -160, 32, 224, 416]
    rd = [p.sector(f'RD{i}', [(b, -320, b + 64, -256)], ceil=128 if i >= 4 else 0, tag=2 + i, door=True) for i, b in enumerate(bs)]
    for i, b in enumerate(bs): p.sector(f'RC{i}', [(b, -448, b + 64, -320)])
    strips = {}
    for i in (0, 1, 2, 4, 5):
        strips[i] = p.sector(f'ST{i}', [(bs[i] - 32, -208, bs[i] + 96, -192)])
    alw = p.sector('ALW', [(-656, -32, -640, 32)])
    ale = p.sector('ALE', [(640, -32, 656, 32)])
    for i, (a, sp) in enumerate(zip(xs, [31, 26, 27, 28, 32, 34, 33, 46])):
        p.special(hall, md[i], (a + 32, 256), sp, 8 if sp == 46 else 0)
    for i, sp in ((0, 2), (1, 86), (2, 90), (4, 16), (5, 76)):
        p.special(hall, strips[i], (bs[i] + 32, -192), sp, 2 + i)
    p.special(alw, None, (-656, 0), 46, 8)
    p.special(ale, None, (656, 0), 103, 5)
    p.thing(-544, -100, NORTH, PLAYER1)
    p.thing(-400, -60, EAST, BLUECARD)
    p.thing(-256, -60, EAST, YELLOWCARD)
    p.thing(-112, -60, EAST, REDCARD)
    p.thing(464, 400, SOUTH, ZOMBIE, ALL_SKILLS | AMBUSH)
    p.thing(448, -288, EAST, BARREL)
    return p


def floors(comb_low=None):
    """Eight floor modules on the north wall, three switch alcoves, the 21-cell comb."""
    p = Plan('floors')
    hall = p.sector('HALL', [(-960, -256, 960, 256)])
    xa = [-848 + 240 * i for i in range(8)]
    pad = {}
    pad[0] = p.sector('M5', [(xa[0], 256, xa[0] + 96, 352)], tag=1)
    p.sector('M5N', [(xa[0], 352, xa[0] + 96, 400)], ceil=48)
    pad[1] = p.sector('M91', [(xa[1], 256, xa[1] + 96, 352)], tag=2)
    p.sector('M91N', [(xa[1], 352, xa[1] + 96, 400)], ceil=48)
    pad[2] = p.sector('M91B', [(xa[2], 256, xa[2] + 96, 352)], ceil=64, tag=3)
    p.sector('M91BN', [(xa[2], 352, xa[2] + 96, 400)], ceil=96)
    pad[3] = p.sector('M18', [(xa[3], 256, xa[3] + 96, 352)], tag=4)
    p.sector('M18A', [(xa[3] - 64, 256, xa[3], 352)], floor=16)
    p.sector('M18B', [(xa[3] + 96, 256, xa[3] + 160, 352)], floor=64)
    pad[4] = p.sector('M23', [(xa[4], 256, xa[4] + 96, 352)], floor=32, tag=5)
    p.sector('M23T', [(xa[4], 352, xa[4] + 96, 400)], floor=-32)
    pad[5] = p.sector('M70', [(xa[5], 256, xa[5] + 96, 352)], floor=64, tag=6)
    p.sector('M70L', [(xa[5] + 96, 256, xa[5] + 160, 352)], floor=24)
    pad[6] = p.sector('M98', [(xa[6], 256, xa[6] + 96, 352)], floor=64, tag=7)
    p.sector('M98L', [(xa[6] + 96, 256, xa[6] + 160, 352)], floor=16)
    pad[7] = p.sector('M82', [(xa[7], 256, xa[7] + 96, 352)], tag=8)
    p.sector('M82T', [(xa[7], 352, xa[7] + 96, 400)], floor=-32)
    for i, sp, tag in ((0, 5, 1), (1, 91, 2), (2, 91, 3), (6, 98, 7), (7, 82, 8)):
        st = p.sector(f'S{i}', [(xa[i] - 16, 176, xa[i] + 112, 192)])
        p.special(hall, st, (xa[i] + 48, 192), sp, tag)
    for i, sp, tag in ((3, 18, 4), (4, 23, 5), (5, 70, 6)):
        al = p.sector(f'A{i}', [(xa[i] + 16, -272, xa[i] + 80, -256)])
        p.special(al, None, (xa[i] + 48, -272), sp, tag)
    comb = p.sector('COMB', [(-960, -320, -288, -256)], tag=9)
    for k in range(21):
        p.sector(f'C{k}', [(-960 + 32 * k, -384, -928 + 32 * k, -320)], floor=8 if comb_low == k else 64)
    p.special(comb, None, (-960, -288), 18, 9)
    p.thing(0, 0, EAST, PLAYER1)
    p.thing(xa[2] + 48, 304, EAST, BARREL)
    return p


def plats():
    """62 SR lift, 20 S1 and 22 W1 raiseToNearestAndChange, a blocked 22."""
    p = Plan('plats')
    hall = p.sector('HALL', [(-640, -256, 640, 256)])
    p.sector('PL', [(-512, 256, -416, 352)], floor=48, tag=1)
    p.sector('P20', [(-192, 256, -96, 352)], fpic='NUKAGE1', special=7, tag=2)
    p.sector('E20', [(-96, 256, -32, 352)], floor=40)
    p.sector('P22', [(96, 256, 192, 352)], fpic='NUKAGE1', special=7, tag=3)
    p.sector('E22', [(192, 256, 256, 352)], floor=40)
    p.sector('P22B', [(352, 256, 448, 352)], ceil=64, fpic='NUKAGE1', special=7, tag=4)
    p.sector('E22B', [(448, 256, 512, 352)], floor=40)
    a62 = p.sector('A62', [(-496, -272, -432, -256)])
    a20 = p.sector('A20', [(-176, -272, -112, -256)], fpic='TESTFLR0')
    s22 = p.sector('S22', [(80, 176, 208, 192)], fpic='TESTFLR0')
    s22b = p.sector('S22B', [(336, 176, 464, 192)])
    p.special(a62, None, (-464, -272), 62, 1)
    p.special(a20, None, (-144, -272), 20, 2)
    p.special(hall, s22, (144, 192), 22, 3)
    p.special(hall, s22b, (400, 192), 22, 4)
    p.thing(0, -100, EAST, PLAYER1)
    p.thing(-464, 320, NORTH, ZOMBIE, ALL_SKILLS | AMBUSH)   # facing away: asleep while it rides
    p.thing(-464, 288, EAST, CLIP)
    p.thing(400, 304, EAST, BARREL)
    return p


def stairs():
    """Staircase A (S1 7, decoy and busy neighbour), B (W1 8), blocked C (W1 8), donut 9."""
    p = Plan('stairs')
    hall = p.sector('HALL', [(-640, -256, 640, 256)])
    for k in range(1, 7):
        p.sector(f'a{k}', [(-576, -256 - 64 * k, -480, -192 - 64 * k)], tag=7 if k == 1 else 0)
    p.sector('dA', [(-480, -384, -384, -320)], fpic='TESTFLR0')
    p.sector('bA', [(-480, -448, -384, -384)], tag=9)
    p.sector('pA', [(-384, -448, -288, -384)], floor=-64)
    a7 = p.sector('A7', [(-400, -272, -336, -256)])
    s82 = p.sector('S82', [(-560, -208, -432, -192)])
    for k in range(1, 6):
        p.sector(f'b{k}', [(96, 192 + 64 * k, 192, 256 + 64 * k)], tag=8 if k == 1 else 0)
    sb = p.sector('SB', [(80, 176, 208, 192)])
    for k in range(1, 6):
        p.sector(f'c{k}', [(352, 192 + 64 * k, 448, 256 + 64 * k)], ceil=64, tag=10 if k == 1 else 0)
    sc = p.sector('SC', [(336, 176, 464, 192)])
    p.sector('ring', [(704, -64, 848, 64)], floor=-32, fpic='NUKAGE1', special=7)
    p.sector('pillar', [(736, -32, 816, 32)], floor=48, tag=11)
    p.sector('pool', [(640, -128, 912, 128)], layer=-1)
    a9 = p.sector('A9', [(240, -272, 304, -256)])
    p.special(a7, None, (-368, -272), 7, 7)
    p.special(hall, s82, (-496, -192), 82, 9)
    p.special(hall, sb, (144, 192), 8, 8)
    p.special(hall, sc, (400, 192), 8, 10)
    p.special(a9, None, (272, -272), 9, 11)
    p.thing(0, 0, EAST, PLAYER1)
    p.thing(776, 0, WEST, ZOMBIE, ALL_SKILLS | AMBUSH)
    p.thing(776, -16, EAST, CLIP)
    p.thing(400, 480, EAST, BARREL)
    return p


def exits():
    """Light 35, exit 11, secret exit 51, sector special 11, a barrel outside 11."""
    p = Plan('exits')
    hall = p.sector('HALL', [(-640, -256, 640, 256)])
    p.sector('DK1', [(-160, 256, -64, 352)], light=192, tag=3)
    p.sector('DK2', [(64, 256, 160, 352)], light=96, special=8, tag=3)
    s35 = p.sector('S35', [(-176, 176, 176, 192)])
    a11 = p.sector('A11', [(-656, -32, -640, 32)])
    a51 = p.sector('A51', [(-400, 256, -336, 272)])
    p.sector('END', [(640, -96, 832, 96)], special=11)
    p.special(hall, s35, (0, 192), 35, 3)
    p.special(a11, None, (-656, 0), 11, 0)
    p.special(a51, None, (-368, 272), 51, 0)
    p.thing(0, 0, NORTH, PLAYER1)
    p.thing(792, 48, EAST, BARREL)
    p.thing(480, -120, EAST, BARREL)
    p.thing(496, -160, EAST, BARREL)   # its neighbour: one blast alone leaves the player alive (EXDIE)
    return p


def weapons():
    """West room (chainsaw, launcher, rockets, two sleeping zombiemen), east room (barrel
    cluster, two imps and a zombieman, asleep facing east), joined by a north corridor
    so the rooms have no line of sight; a sky alcove on the east room's north wall."""
    p = Plan('weapons')
    p.sector('HALL', [(-640, -256, -64, 256), (-64, 160, 64, 256), (64, -256, 640, 256)])
    p.sector('SK', [(96, 256, 160, 272)], floor=64, ceil=112, cpic='F_SKY1')
    p.thing(-600, -100, EAST, PLAYER1)
    p.thing(-540, -100, EAST, CHAINSAW)
    p.thing(-540, 60, EAST, LAUNCHER)
    for x in (-500, -460, -420): p.thing(x, 60, EAST, ROCKET)
    p.thing(-380, 60, EAST, ROCKETBOX)
    p.thing(-300, -100, EAST, ZOMBIE, ALL_SKILLS | AMBUSH)
    p.thing(-280, -60, EAST, ZOMBIE, ALL_SKILLS | AMBUSH)
    p.thing(-380, -100, EAST, MEDIKIT)
    for b in range(2):
        for a in range(3): p.thing(500 + 40 * a, -40 + 40 * b, EAST, BARREL)
    p.thing(580, -80, EAST, IMP, ALL_SKILLS | AMBUSH)
    p.thing(580, 80, EAST, IMP, ALL_SKILLS | AMBUSH)
    p.thing(440, 0, EAST, ZOMBIE, ALL_SKILLS | AMBUSH)
    return p


def items():
    """Pickups along y = 150, ponds with specials 7 and 16, a monster pen (asleep, facing
    away from its gap)."""
    p = Plan('items')
    p.sector('HALL', [(-640, -256, 480, 256)])
    p.sector('P7', [(-256, -192, 0, -64)], fpic='NUKAGE1', special=7)
    p.sector('P16', [(64, -192, 256, -64)], fpic='NUKAGE1', special=16)
    p.sector('GAP', [(480, -32, 512, 32)])
    p.sector('PEN', [(512, -128, 672, 128)])
    p.thing(0, 0, NORTH, PLAYER1)
    for x in (-560, -520, -480): p.thing(x, 150, EAST, SHELLBOX)
    p.thing(-440, 150, EAST, BACKPACK)
    for x in (-400, -360, -320, -280): p.thing(x, 150, EAST, SHELLBOX)
    for x, k in ((-200, SOULSPHERE), (-120, LIGHTAMP), (-40, ALLMAP), (40, INVIS), (120, SUIT),
                 (200, BLUECARD), (240, YELLOWCARD), (280, REDCARD), (320, ROCKET), (360, ROCKET), (400, ROCKETBOX)):
        p.thing(x, 150, EAST, k)
    p.thing(-600, 100, EAST, SHOTGUN)
    p.thing(580, -60, EAST, ZOMBIE, ALL_SKILLS | AMBUSH)
    p.thing(580, 60, EAST, ZOMBIE, ALL_SKILLS | AMBUSH)
    p.thing(620, 0, EAST, IMP, ALL_SKILLS | AMBUSH)
    p.thing(560, 100, EAST, SHOTGUY, ALL_SKILLS | AMBUSH)
    return p


def spectre():
    """A lane with three Spectres and an imp, all awake and facing the player, and a ledge at its west end."""
    p = Plan('spectre')
    p.sector('LANE', [(-640, -96, 640, 96)])
    # a 40-unit ledge at the west end: Spectres cannot climb it (step 24) nor bite across it (melee range 60 from
    # the far side), so the imp's fireballs at a player on it pass through the Spectres crowding below (SPECT2)
    p.sector('LEDGE', [(-640, -96, -576, 96)], floor=40)
    p.thing(-616, 0, EAST, PLAYER1)
    p.thing(-600, 48, EAST, SHOTGUN)        # on the ledge
    p.thing(-600, -48, EAST, SHELLBOX)      # on the ledge
    p.thing(-540, -60, EAST, GREENARMOR)
    p.thing(-520, 60, EAST, MEDIKIT)
    p.thing(-500, -60, EAST, MEDIKIT)
    p.thing(-150, 0, WEST, SPECTRE)
    p.thing(100, 0, WEST, IMP)
    p.thing(350, -40, WEST, SPECTRE)
    p.thing(450, 40, WEST, SPECTRE)
    return p


def scale():
    """The lifted bounds: a 3,680 x 2,016 hall, a -640 pit, a 544..1100 tower, a 608-unit
    lift, and 12 monsters, 83 pickups, 24 barrels and 40 statics (no teleport landing 14: M4b)."""
    p = Plan('scale')
    p.sector('HALL', [(-1840, -1008, 1840, 1008)])
    p.sector('PIT', [(-1504, -608, -1296, -400)], floor=-640)
    p.sector('TOWER', [(1296, 400, 1504, 608)], floor=544, ceil=1100)
    p.sector('PAD', [(192, 1008, 288, 1104)], floor=560, ceil=700, tag=1)
    p.sector('PD', [(288, 1008, 384, 1104)], floor=-48, ceil=700)
    alc = p.sector('ALC', [(-32, -1024, 32, -1008)])
    p.special(alc, None, (0, -1024), 62, 1)
    p.thing(0, -900, NORTH, PLAYER1)
    monsters = [ZOMBIE] * 4 + [IMP] * 3 + [SHOTGUY] * 2 + [SPECTRE] * 3
    for i, k in enumerate(monsters): p.thing(-1100 + 200 * i, 600, NORTH, k, ALL_SKILLS | AMBUSH)
    pickups = ([CLIP] * 6 + [CLIPBOX] * 4 + [SHELLS] * 6 + [SHELLBOX] * 6 + [STIMPACK] * 6 + [MEDIKIT] * 6 +
               [HEALTHBONUS] * 8 + [ARMORBONUS] * 8 + [GREENARMOR] * 2 + [BLUEARMOR] * 2 + [SOULSPHERE] * 2 +
               [INVIS] * 2 + [SUIT] * 2 + [ALLMAP] * 2 + [LIGHTAMP] * 2 + [BACKPACK] * 2 + [ROCKET] * 6 +
               [ROCKETBOX] * 4 + [CHAINSAW, LAUNCHER, BLUECARD, YELLOWCARD, REDCARD] + [SHOTGUN] * 2)
    assert len(pickups) == 83
    for i, k in enumerate(pickups): p.thing(160 + 80 * (i % 21), -700 + 80 * (i // 21), (EAST, NORTH, WEST, SOUTH)[i % 4], k)
    for i in range(24): p.thing(-1000 + 160 * (i % 8), 200 * (i // 8), EAST, BARREL)
    # no teleport landing (14, MT_TELEPORTMAN, spawn state S_NULL): deferred to M4b with teleportation (decision D6)
    statics = ([CANDLE] * 10 + [FIRESTICK] * 8 + [LAMP] * 6 + [PILLAR] * 4 + [CANDELABRA] * 4 +
               [GIBS] * 4 + [DEADPLAYER] * 2 + [MESS] * 2)
    assert len(statics) == 40
    for i, k in enumerate(statics): p.thing(400 + 96 * (i % 11), 120 * (i // 11), (EAST, NORTH, WEST, SOUTH)[i % 4], k)
    return p


MAPS = {'doors': doors, 'floors': floors, 'plats': plats, 'stairs': stairs, 'exits': exits,
        'weapons': weapons, 'items': items, 'spectre': spectre, 'scale': scale}


# ---------------------------------------------------------------- two-pass rules ---------
def comb_neighbours(plan, m):
    comb = plan.index('COMB')
    floor = lambda s: plan.sectors[s]['floor']
    return comb, [(i, o) for i, o in sector_lines(m, comb) if o is not None and floor(o) > floor(comb)]


def built(name):
    """(plan, map) with the map's two-pass rules applied and its layout assertions checked."""
    plan = MAPS[name]()
    m = build_map(plan)
    if name == 'floors':
        comb, q = comb_neighbours(plan, m)
        assert len(q) == 21, len(q)
        low = plan.sectors[q[20][1]]['label']
        plan = floors(comb_low=int(low[1:]))
        m2 = build_map(plan)
        assert [l['a'] for l in m2['lines']] == [l['a'] for l in m['lines']], 'the rebuild changed the line order'
        m = m2
        comb, q = comb_neighbours(plan, m)
        fl = [plan.sectors[o]['floor'] for _, o in q]
        assert len(q) == 21 and fl[:20] == [64] * 20 and fl[20] == 8, fl
    if name == 'stairs':
        a3, ba, a4 = plan.index('a3'), plan.index('bA'), plan.index('a4')
        order = [o for _, o in sector_lines(m, a3)]
        assert order.index(ba) < order.index(a4), order
        ring, pillar, pool = plan.index('ring'), plan.index('pillar'), plan.index('pool')
        assert ring < pillar and ring < pool
        pl = sector_lines(m, pillar)
        assert all(o == ring for _, o in pl), pl
        rl = sector_lines(m, ring)
        assert all(o in (pillar, pool) for _, o in rl) and rl[0][1] == pool, rl
    fix_sides(plan, m)
    return plan, m


def wad_bytes(name, slot='E1M1'):
    """The WAD of one map in `slot`, with the m3h9c_wad lump framing."""
    plan, m = built(name)
    lumps = dict(m['lumps'])
    lumps['THINGS'] = b''.join(struct.pack('<hhhhh', *t) for t in plan.things)
    assert list(lumps) == ['MAP01'] + MAP_LUMPS, list(lumps)
    flat = bytes(4096)
    wad = ([('PLAYPAL', W.playpal()), ('COLORMAP', W.colormap()), ('S_START', b''), ('S_END', b''), ('F_START', b'')] +
           [(f, flat) for f in W.FLATS] + [('F_END', b''), ('PNAMES', W.pnames()), ('TEXTURE1', W.texture1()),
           (W.PATCH, W.blank_patch(W.TEX_W, W.TEX_H)), (slot, b'')] + [(n, lumps[n]) for n in MAP_LUMPS])
    data = bytearray(b'IWAD' + struct.pack('<ii', len(wad), 0))
    directory = b''
    for n, b in wad:
        directory += struct.pack('<ii8s', len(data), len(b), W.name8(n))
        data += b
    struct.pack_into('<i', data, 8, len(data))
    return bytes(data + directory), plan, m


def spawned(plan, skill=2):
    """THING counts spawned on medium single player (P_SpawnMapThing, p_mobj.c:740-750),
    keyed by DoomEd number (the player start included)."""
    bit = {1: 1, 2: 2, 3: 4}[skill]
    out = {}
    for t in plan.things:
        if t[4] & 16 or not t[4] & bit: continue
        out[str(t[3])] = out.get(str(t[3]), 0) + 1
    return dict(sorted(out.items(), key=lambda kv: int(kv[0])))


def wad_fields(name, data, plan, m, slot):
    return dict(name=name.upper(), file=f'tests/reference/m4a_wads/{name}.wad', slot=slot, episode=int(slot[1]), map=int(slot[3]),
                sha256=hashlib.sha256(data).hexdigest(), bytes=len(data), sectors=len(m['heights']), lines=len(m['lines']),
                things=spawned(plan))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--out-dir', type=Path, default=OUT_DIR)
    ap.add_argument('--only', choices=list(MAPS))
    ap.add_argument('--slot', default='E1M1')
    ap.add_argument('--list', action='store_true')
    ap.add_argument('--manifest', action='store_true')
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    names = [a.only] if a.only else list(MAPS)
    fields = {}
    bad = []
    for n in names:
        data, plan, m = wad_bytes(n, a.slot)
        fields[n] = wad_fields(n, data, plan, m, a.slot)
        f = fields[n]
        if a.list:
            print(f"{f['name']}: {f['file']} {f['bytes']} bytes, sectors {f['sectors']}, lines {f['lines']}, things {sum(f['things'].values())}")
            continue
        if a.check:
            path = ROOT / f['file']
            if not path.exists() or path.read_bytes() != data: bad.append(f['file'])
            else: print(f"{f['file']} matches: {len(data)} bytes, sha256 {f['sha256']}", flush=True)
        else:
            a.out_dir.mkdir(parents=True, exist_ok=True)
            (a.out_dir / f'{n}.wad').write_bytes(data)
            print(f"wrote {a.out_dir / (n + '.wad')}: {len(data)} bytes")
    if a.list: return
    if a.check:
        if bad: sys.exit('differs or missing: ' + ', '.join(bad))
        man = json.loads(MANIFEST.read_text())
        if man.get('schema') != MANIFEST_SCHEMA: sys.exit('manifest schema')
        by = {w['name']: w for w in man['wads']}
        for n in names:
            w = by.get(n.upper())
            if w is None: sys.exit(f'{n}: not in the manifest')
            for k, v in fields[n].items():
                if w.get(k) != v: sys.exit(f'{n}: manifest field {k} differs from the regenerated WAD')
        print(f'{MANIFEST.relative_to(ROOT)}: wad fields match ({len(names)} WADs)', flush=True)
        status = 0
        for n in names:
            status |= subprocess.run([sys.executable, str(CHECKER), fields[n]['file']], cwd=ROOT).returncode
        sys.exit(status)
    if a.manifest:
        man = json.loads(MANIFEST.read_text()) if MANIFEST.exists() else dict(schema=MANIFEST_SCHEMA, wads=[])
        by = {w['name']: w for w in man['wads']}
        for n in names:
            w = by.setdefault(n.upper(), {})
            scripts = w.pop('scripts', [])
            w.clear(); w.update(fields[n]); w['scripts'] = scripts
        order = [n.upper() for n in MAPS]
        man['wads'] = sorted(by.values(), key=lambda w: order.index(w['name']))
        MANIFEST.write_text(json.dumps(man, indent=1) + '\n')
        print(f'updated {MANIFEST.relative_to(ROOT)}')


if __name__ == '__main__':
    main()
