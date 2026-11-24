#!/usr/bin/env python3
"""Synthetic map and demo scripts of the wire 27 genuine-engine items (owner's rulings of 2026-11-24, R6-R8).

Everything here is drawn by this tool: nothing comes from any IWAD. One IWAD, tests/reference/wire27_wads/wire27.wad,
holds eight map slots E1M1-E1M8 sharing one room layout (the M5a texture and flat directory and framing, m5a_wad and
m6a_wad imported read-only); each item runs one slot from a new game with the M6a WAD set after it
(tests/reference/m5b_gfx.wad, m5c_sfx.wad, m6a_wads/m6a_gfx.wad and m6a_music.wad). The layout is one convex room, a
quadrilateral whose east wall is long and slanted, so that the int arithmetic of P_InterceptVector (p_maputl.c:240,
246-248) leaves `int` for the traces that cross it:

    (-300, 4000) -> (484, 4000)          north wall, line 0 (listed in every BLOCKMAP cell by its leading 0)
    (484, 4000) -> (484 + DX, -4059)     east wall, 8,059 units south and DX east (DX 2,098; E1M5 2,097; E1M7 2,099)
    (484 + DX, -4059) -> (-300, -4059)   south wall
    (-300, -4059) -> (-300, 4000)        west wall

The walls are one-sided and unsplit (the builder chain m3h9b_geometry with pieces of up to 16,384 units). Two in-process
overrides, declared in OVERRIDES (and in tests/reference/wire27_results.json): no file is edited, and both are restored
in a finally block. The maps satisfy the Episode 1 load rules of wires 26 and 27 (player_map_scale.rs
ArithmeticCertificate::derive_episode1: spans up to 2,883 x 8,059 and line components under 8,192, the grid, the side
tests and the reach; the closed-cell BLOCKMAP and the slide window), so a wire-26 run refuses at run time, not at the
load. Things: the player start and, in E1M4-E1M5, one barrel. The east wall, for a trace from (x, y) with the wall's
v1 = (484, 4000) and delta (DX, -8059):

    num/256 = (484 - x)(-8059) + (y - 4000) DX       den/256 = -8059 tdx - DX tdy   (trace delta in units)

The second num product is below INT_MIN for DX >= 2,098, so it narrows; the den products leave `int` for the 2,048-unit
pistol trace. With the trace at fine angle 96 (three turns of 32 from east: finecosine 65356, finesine 4846, tables.c)
and DX 2,098 the exact den is -2^32 (8059*65356 + 2098*4846 = 2^29), which the int arithmetic makes 0.

ITEMS: name, map slot, what it shows, the callers and the wrap classes (tools/reference/wire27_goldens.py CLASSES,
without `wrap.`) it is designed to witness (tools/reference/wire27_goldens.py checks them against the genuine run),
things and script lines (the tools/reference/m4b_scripts.py format).

    wire27_wad.py            write the WAD and the scripts
    wire27_wad.py --check    regenerate in memory and compare with the committed files (writes nothing)
    wire27_wad.py --list     files, digests, items
"""
import argparse, hashlib, math, struct, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m3h6_geometry as H        # noqa: E402  the line-norm assert (raised in-process for these maps only)
import m3h9b_geometry as G       # noqa: E402  the builder chain (read-only)
import m4a_wad as A              # noqa: E402  the lump order (read-only)
import m4b_scripts as S          # noqa: E402  the script format (read-only)
import m5a_wad as R              # noqa: E402  Room, apply_styles, framing (read-only)
import m6a_wad as W              # noqa: E402  the WAD writer (read-only)

WAD = 'tests/reference/wire27_wads/wire27.wad'
SCRIPT_DIR = 'tests/reference/wire27_scripts'
WADSET = [WAD, 'tests/reference/m5b_gfx.wad', 'tests/reference/m5c_sfx.wad', 'tests/reference/m6a_wads/m6a_gfx.wad',
          'tests/reference/m6a_wads/m6a_music.wad']
EAST, PLAYER1, BARREL = 0, 1, 2035
TOP, BOTTOM, WEST, V1X = 4000, -4059, -300, 484
LONG_NORM = 16382                 # line components under 8,192 (derive_episode1 clause iv)
CALLERS = ('aim', 'shoot_line', 'shoot_thing', 'slide', 'use')
OVERRIDES = [
    dict(what='m3h6_geometry.MAX_NORM raised to LONG_NORM (16,382) around the m3h9b_geometry.room call of map_lumps',
         why='the builder chain asserts a line norm under its historical bound; the east wall (DX x 8,059 units) is '
             'longer, and it must stay one unsplit linedef for the traces to meet it in one P_InterceptVector call'),
    dict(what='m5a_wad.seg_angle replaced by a rounded atan2 around the m5a_wad.apply_styles call of map_lumps',
         why='m5a_wad knows the axis and diagonal seg directions only; the slanted wall\'s SEGS angle (BAM >> 16) is read '
             'by the renderer alone (p_setup.c:181), and no renderer runs on these maps'),
]
for o in OVERRIDES: o['scope'] = 'in-process only, while map_lumps runs: no file is edited, and the original is restored'
SHOT = ['cmd 20 0 0 0 ATTACK', 'idle 15']               # the pistol rises, one accurate shot, released before A_ReFire
TURNED = ['cmd 3 0 0 1 ATTACK', 'cmd 17 0 0 0 ATTACK', 'idle 15']   # the same at fine angle 96
# name: (map, east wall DX, player (x, y), barrels, script, callers, cases, what)
ITEMS = {
    'USE': (1, 2098, (1490, 0), [], ['idle 5', 'cmd 1 0 0 0 USE', 'idle 10'], ['use'], ['den_exact.num_exact.div2'],
            'a use press 34 units from the east wall: both num products and their sum leave int and cancel; the exact '
            'intercept, the wall stops the use with sfx_noway (p_map.c:1095-1120)'),
    'SLIDE': (2, 2098, (1400, 0), [], ['cmd 35 50 0 0 0'], ['slide'], ['den_exact.num_exact.div2'],
              'running east into the slanted east wall: every slide trace (p_map.c:731-743) meets it with wraps that '
              'cancel, and the player slides south along it'),
    'AIMSHOT': (3, 2098, (700, 0), [], SHOT, ['aim', 'shoot_line'],
                ['den_exact.num_exact.div2', 'den_wrong.num_exact.div2'],
                'one pistol shot east 825 units from the east wall: the aim traces (1,024 units, P_BulletSlope) meet it '
                'with wraps that cancel and stop there; the 2,048-unit shot\'s den wraps to a positive value, so the '
                'intercept is negative and the wall is not hit: no puff (exact: a puff at the wall)'),
    'SHOOTZ': (4, 2098, (0, 0), [(997, 74)], TURNED, ['shoot_line'], ['den_zero'],
               'one pistol shot at fine angle 96: the exact den is -2^32, the int den 0, so P_InterceptVector returns 0 '
               '(p_maputl.c:242-243) and the wall is hit at the shooter (a puff 4 units behind the player); the barrel '
               '1,000 units out on the line of fire is not hit (exact: the barrel at 0.49 before the wall at 0.73)'),
    'SATUR': (5, 2097, (0, 0), [(997, 74)], TURNED, ['shoot_line'], ['den_wrong.num_wrong.max'],
              'the same shot with the east wall one unit steeper: the int den is 38,768, the num sum leaves int, and '
              'FixedDiv saturates (m_fixed.c:62-63): MAXINT, an intercept that P_TraverseIntercepts never reaches '
              '(p_maputl.c:698-709); the barrel is hit'),
    'NUMWRAP': (6, 2098, (0, -2150), [], ['cmd 4 0 0 1 ATTACK', 'cmd 16 0 0 0 ATTACK', 'idle 15'], ['shoot_line'],
                ['den_wrong.num_wrong.div2'],
                'one pistol shot at fine angle 128 from 1,007 units or more west of the east wall: the num sum and the den '
                'leave int and FixedDiv2 (m_fixed.c:69-87) divides the wrapped -6,666,240 by -17,926,360: the wall at '
                '0.37, a puff in the open room 757 units out (exact: a puff at the wall at 0.997)'),
    'SATNEG': (7, 2099, (0, 0), [], TURNED, ['shoot_line'], ['den_wrong.num_wrong.min'],
               'the SATUR shot with the east wall one unit shallower and no barrel: the int den is -38,768 and the num '
               'sum leaves int, so FixedDiv saturates to MININT (m_fixed.c:62-63), the wall is "behind source" '
               '(p_maputl.c:591-592) and the shot hits nothing: no puff (exact: a puff at the wall at 0.73)'),
    'SHOOTC': (8, 2098, (1500, -200), [], ['cmd 1 0 0 -50 ATTACK', 'cmd 19 0 0 0 ATTACK', 'idle 15'],
               ['shoot_line', 'aim'], ['den_exact.num_exact.div2'],
               'one pistol shot at fine angle -1,600, 130 units from the east wall: the den and num products leave int '
               'and cancel; the exact intercept, a puff at the wall'),
}


def room(name):
    m, dx, (px, py), barrels, *_ = ITEMS[name]
    p = R.Room(f'wire27_{name.lower()}')
    p.sector('ROOM', [(-304, -4064, 2592, 4000)], light=160, fpic='M5FLAT1', cpic='M5FLAT2')   # a placeholder rect:
    # m5a's Plan takes 16-unit rectangles; the room itself is the ring below (rings_override)
    p.rings_override = [[[(WEST, TOP), (V1X, TOP), (V1X + dx, BOTTOM), (WEST, BOTTOM)]]]   # clockwise, y up
    p.thing(px, py, EAST, PLAYER1)
    for x, y in barrels: p.thing(x, y, EAST, BARREL)
    return p


def map_lumps(name):
    """The map lumps of an item's room: the builder chain with unsplit walls, then the M5a styles."""
    p = room(name)
    sectors = [(rings, s['floor'], s['ceil'], dict(s['attrs'])) for rings, s in zip(p.rings_override, p.sectors)]
    old = H.MAX_NORM
    H.MAX_NORM = LONG_NORM
    try:
        m = G.room(sectors, portals={}, reject=(), tree=None, segment=LONG_NORM, lines={})
    finally:
        H.MAX_NORM = old
    assert len(m['lines']) == 4 and m['vertices'][m['lines'][0]['a']] == (WEST, TOP), 'line 0 is the north wall'
    # m5a's seg angle table knows the axis and diagonal directions only; the slanted wall's SEGS angle (BAM >> 16,
    # read by the renderer alone, p_setup.c:181; no renderer runs here) is rounded from atan2, also in-process only.
    plain = R.seg_angle
    R.seg_angle = lambda dx, dy: (round(math.atan2(dy, dx) * 32768 / math.pi) + 32768) % 65536 - 32768
    try:
        R.apply_styles(p, m)
    finally:
        R.seg_angle = plain
    lumps = dict(m['lumps'])
    lumps['THINGS'] = b''.join(struct.pack('<hhhhh', *t) for t in p.things)
    return [(n, lumps[n]) for n in A.MAP_LUMPS]


def wad_bytes():
    out = R.framing()
    for name, (m, *_) in sorted(ITEMS.items(), key=lambda kv: kv[1][0]):
        out += [(f'E1M{m}', b'')] + map_lumps(name)
    return W.wad(b'IWAD', out)


def script_text(name):
    m, *_, lines, _callers, _cases, what = ITEMS[name]
    return (f'# wire 27 item {name}: {what}.\n# Generated by tools/reference/wire27_wad.py.\n'
            + '\n'.join([f'map E1M{m}'] + lines) + '\n')


def files():
    """{repository path: bytes} of every generated file."""
    out = {WAD: wad_bytes()}
    for name in ITEMS:
        text = script_text(name)
        out[f'{SCRIPT_DIR}/{name}.txt'] = text.encode()
        out[f'{SCRIPT_DIR}/{name}.lmp'] = S.lump(text)[0]
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--list', action='store_true')
    a = ap.parse_args()
    generated = files()
    if a.list:
        for path, data in generated.items(): print(f'{path} {len(data)} bytes sha256 {hashlib.sha256(data).hexdigest()}')
        for name, (m, *_, callers, cases, what) in ITEMS.items():
            print(f'{name}: E1M{m}, {S.lump(script_text(name))[1]} tics, callers {callers}, cases {cases}')
        return 0
    if a.check:
        bad = [p for p, d in generated.items() if not (ROOT / p).exists() or (ROOT / p).read_bytes() != d]
        if bad: print('differs or missing: ' + ', '.join(bad)); return 1
        print(f'{len(generated)} files match ({len(ITEMS)} items)')
        return 0
    for path, data in generated.items():
        (ROOT / path).parent.mkdir(parents=True, exist_ok=True)
        (ROOT / path).write_bytes(data)
        print(f'wrote {path}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
