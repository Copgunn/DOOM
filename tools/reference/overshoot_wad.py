#!/usr/bin/env python3
"""Synthetic maps and demos of the block-walk overshoot items (certificate audit 1, item 4); nothing from an IWAD.

tests/reference/overshoot_wads/overshoot.wad holds two map slots, each one room whose north wall near the player lies on
a BLOCKMAP row line (the origin is the vertex minimum, m3h6_geometry.py); each item runs from a new game with the M6a
WAD set of the wire 27 items (wire27_wad.py, read-only). The player runs east along the north wall (P_SlideMove's
stairstep, p_map.c:706-710 and 763, keeps it a few raw units under it); one tic of a small forward move after a slide
that zeroed the y momentum (p_map.c:596-599, 779-781) gives a slide trace (p_map.c:738) from a few raw units under the
row line to above it, so P_PathTraverse's `ystep` is 0 (p_maputl.c:802) and the walk steps east 63 times past the end
cell until the count of 64 runs out (p_maputl.c:846, 866-875). The north wall is listed in the row above only.

- SLIDE (E1M1): an 11.3-unit trace (the side test against the line's infinite line, p_maputl.c:570-581) meets
  DIP_LINE 3,557 units east, which separates its ends: the genuine engine slides along it (p_map.c:672-680, 779).
- WRAP (E1M2): an 18.2-unit trace (the divline side test, p_maputl.c:574-576) meets a notch's 2,184-unit edges about
  5,000 units east; a FixedMul product of P_InterceptVector leaves int (m_fixed.c:48) and the fractions come out
  negative ("behind source", p_maputl.c:591-592).

    overshoot_wad.py            write the WAD and the script
    overshoot_wad.py --check    regenerate in memory and compare with the committed files (writes nothing)
"""
import argparse, hashlib, math, struct, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m3h9b_geometry as G       # noqa: E402  the builder chain (read-only)
import m4a_wad as A              # noqa: E402  the lump order (read-only)
import m4b_scripts as S          # noqa: E402  the script format (read-only)
import m5a_wad as R              # noqa: E402  Room, apply_styles, framing (read-only)
import m6a_wad as W              # noqa: E402  the WAD writer (read-only)
import wire27_wad as V           # noqa: E402  the M6a WAD set of the wire 27 items (read-only)

WAD = 'tests/reference/overshoot_wads/overshoot.wad'
SCRIPT_DIR = 'tests/reference/overshoot_scripts'
WADSET = [WAD] + V.WADSET[1:]
EAST, PLAYER1 = 0, 1
TOP = 256                          # the north wall near the player lies on a BLOCKMAP row line
PLAYER = (64, 239)


def wall(y, x0, x1, step=512):
    """Vertices along a long wall every `step` units (so that the linedefs near the player do not depend on what lies
    far east), x0 included, x1 excluded, in the direction of travel."""
    return [(x, y) for x in range(x0, x1, step if x1 > x0 else -step)]


def ring_slide():
    """SLIDE (E1M1), clockwise with y up: 5,408 x 256 units; the north wall dips 111 units at x 4,064 to 4,128, and the
    dip's bottom edge DIP_LINE, (4,064, 145) -> (4,128, 143), faces the room."""
    return [(0, 0)] + wall(TOP, 0, 4064) + [(4064, TOP), (4064, 145), (4128, 143), (4128, TOP)] + \
        wall(TOP, 4608, 5408) + [(5408, TOP)] + wall(0, 5120, 0)


def ring_wrap():
    """WRAP (E1M2), clockwise with y up: the vertex minimum is (0, -1,920), so y 256 is again a row line (17 rows up);
    the north wall rises to y 384 at x 2,048, and a 64-unit notch from y 384 down to y -1,800 at x 6,000 has two
    2,184-unit vertical edges. Line components stay under the real-map certificate's 3,853 (M3h6 note section 4.2 (i))."""
    return [(0, -1920)] + wall(TOP, 0, 2048) + [(2048, TOP), (2048, 384)] + wall(384, 2560, 6000) + \
        [(6000, 384), (6000, -1800), (6064, -1800), (6064, 384)] + wall(384, 6656, 8000) + [(8000, 384)] + \
        wall(-1920, 8000, 0)


# name: (map, ring, player (x, y), script, what, the tic of the overshooting slide trace, the declared UBSan stop of the
# overflow and address builds (site, session tic) or None)
ITEMS = {
    'SLIDE': (1, ring_slide, PLAYER, ['cmd 18 50 0 0 0', 'cmd 1 40 0 1 0', 'cmd 19 40 0 0 0', 'cmd 1 6 0 -1 0', 'idle 6'],
              'the overshooting slide trace meets DIP_LINE 3,557 units east of its start', 38, None),
    'WRAP': (2, ring_wrap, PLAYER, ['cmd 1 50 40 28 0', 'cmd 51 50 40 0 0', 'cmd 1 12 0 -28 0', 'idle 4'],
             'the overshooting slide trace meets the notch\'s 2,184-unit edges and P_InterceptVector wraps', 52,
             ('p_maputl.c:873', 53)),
}


def map_lumps(name):
    m, ring, (px, py), *_ = ITEMS[name]
    p = R.Room(f'overshoot_{name.lower()}')
    p.sector('ROOM', [(0, 0, 16, 16)], light=160, fpic='M5FLAT1', cpic='M5FLAT2')   # a placeholder: the ring below
    p.rings_override = [[ring()]]
    p.thing(px, py, EAST, PLAYER1)
    sectors = [(rings, s['floor'], s['ceil'], dict(s['attrs'])) for rings, s in zip(p.rings_override, p.sectors)]
    built = G.room(sectors, portals={}, reject=(), tree=None, segment=4096, lines={})
    assert (TOP - G.origin(built)[1]) % 128 == 0, G.origin(built)
    plain = R.seg_angle      # the dip line's SEGS angle (read by the renderer alone) as wire27_wad.py rounds it
    R.seg_angle = lambda dx, dy: (round(math.atan2(dy, dx) * 32768 / math.pi) + 32768) % 65536 - 32768
    try:
        R.apply_styles(p, built)
    finally:
        R.seg_angle = plain
    lumps = dict(built['lumps'])
    lumps['THINGS'] = b''.join(struct.pack('<hhhhh', *t) for t in p.things)
    return [(n, lumps[n]) for n in A.MAP_LUMPS]


def wad_bytes():
    out = R.framing()
    for name, (m, *_) in ITEMS.items():
        out += [(f'E1M{m}', b'')] + map_lumps(name)
    return W.wad(b'IWAD', out)


def script_text(name):
    m, _ring, _player, lines, what, *_ = ITEMS[name]
    return f'# overshoot item {name}: {what}.\n# Generated by tools/reference/overshoot_wad.py.\n' + \
        '\n'.join([f'map E1M{m}'] + lines) + '\n'


def files():
    out = {WAD: wad_bytes()}
    for name in ITEMS:
        out[f'{SCRIPT_DIR}/{name}.txt'] = script_text(name).encode()
        out[f'{SCRIPT_DIR}/{name}.lmp'] = S.lump(script_text(name))[0]
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    generated = files()
    if a.check:
        bad = [p for p, d in generated.items() if not (ROOT / p).exists() or (ROOT / p).read_bytes() != d]
        print('differs or missing: ' + ', '.join(bad) if bad else f'{len(generated)} files match')
        return 1 if bad else 0
    for path, data in generated.items():
        (ROOT / path).parent.mkdir(parents=True, exist_ok=True)
        (ROOT / path).write_bytes(data)
        print(f'wrote {path} sha256 {hashlib.sha256(data).hexdigest()}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
