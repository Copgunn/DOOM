#!/usr/bin/env python3
"""Synthetic game WAD for the M3h9c whole-engine runner (decision G8).

Writes tests/reference/m3h9c_synthetic.wad: a small, fully synthetic IWAD holding
everything the headless genuine linuxdoom-1.10 engine needs to load map E1M1 in
`registered` mode without the renderer, plus an E1M1-scale synthetic map built by the
repository's existing room builders (m3h9b_geometry.room, the M3h6..M3h9b chain,
imported unchanged; thing kinds from m3h9b_cases). No commercial data is read.

Formats (linuxdoom-1.10):
- Header `IWAD`, int numlumps, int infotableofs; directory entries int filepos, int size,
  char[8] name, NUL padded (w_wad.c:143-232, ident test 185-190). Names are stored upper
  case; lookup upper-cases and scans backwards (w_wad.c:351-392).
- `PNAMES`: int count and 8-byte names, each resolved by W_CheckNumForName (r_data.c:437-452).
- `TEXTURE1` (no TEXTURE2): int n, int offset[n], then on-disk maptexture_t (name[8], int
  masked, short width, short height, int columndirectory, short patchcount; 22 bytes) and
  10-byte mappatch_t entries (r_data.c:73-83, 505-545). Width 64 (texturewidthmask 63,
  r_data.c:550-555), height 128.
- Patch `BLANKP`: patch_t short width, height, leftoffset, topoffset, int columnofs[64], all
  pointing at one 0xFF terminator (R_GenerateLookup, r_data.c:300-364).
- Flats between F_START and F_END, 4096 bytes each (R_InitFlats r_data.c:581-595); F_SKY1
  (G_DoLoadLevel g_game.c:454); NUKAGE1..3 complete (animdefs p_spec.c:98-118,
  P_InitPicAnims 131-176).
- Every alphSwitchList entry with episode <= 2, both names (p_switch.c:46-90; registered
  uses episode 2, P_InitSwitchList 107-148); SKY1..SKY3 (G_InitNew g_game.c:1466-1474).
- S_START/S_END empty (r_data.c:493-494, 603-624); COLORMAP 34*256 zeros (633-645);
  PLAYPAL 768 zeros.
- Map lumps E1M1, THINGS, LINEDEFS, SIDEDEFS, VERTEXES, SEGS, SSECTORS, NODES, SECTORS,
  REJECT, BLOCKMAP in vanilla order (p_setup.c:649-664); THING = 5 shorts.

    m3h9c_wad.py [--out PATH]     write the WAD (default tests/reference/m3h9c_synthetic.wad)
    m3h9c_wad.py --check          regenerate in memory, compare with the committed bytes, then
                                  run the independent verifier m3h9c_check_wad.py on them
"""
import argparse, hashlib, struct, subprocess, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m3h9b_cases as C            # noqa: E402  thing kinds
import m3h9b_geometry as G         # noqa: E402  room() (the M3h6..M3h9b builder chain)
WAD = ROOT / 'tests/reference/m3h9c_synthetic.wad'
CHECKER = ROOT / 'tools/reference/m3h9c_check_wad.py'

# ---------------------------------------------------------------- game data -------------
# alphSwitchList, p_switch.c:46-90: every entry with episode <= 2 (registered mode,
# P_InitSwitchList p_switch.c:107-148).  Both names of each pair must be textures.
SWITCHES = ['SW1BRCOM', 'SW1BRN1', 'SW1BRN2', 'SW1BRNGN', 'SW1BROWN', 'SW1COMM', 'SW1COMP', 'SW1DIRT',
            'SW1EXIT', 'SW1GRAY', 'SW1GRAY1', 'SW1METAL', 'SW1PIPE', 'SW1SLAD', 'SW1STARG', 'SW1STON1',
            'SW1STON2', 'SW1STONE', 'SW1STRTN',
            'SW1BLUE', 'SW1CMT', 'SW1GARG', 'SW1GSTON', 'SW1HOT', 'SW1LION', 'SW1SATYR', 'SW1SKIN',
            'SW1VINE', 'SW1WOOD']
SWITCH_TEXTURES = [n for s in SWITCHES for n in (s, 'SW2' + s[3:])]
PLAIN = ['TESTWALL', 'STARTAN2', 'BIGDOOR2', 'DOORTRAK', 'PLAT1', 'SUPPORT2']   # TESTWALL = texture 0
SKIES = ['SKY1', 'SKY2', 'SKY3']
TEXTURES = PLAIN + SKIES + SWITCH_TEXTURES
FLATS = ['FLOOR7_1', 'CEIL3_5', 'TESTFLR0', 'TESTCEIL', 'F_SKY1', 'NUKAGE1', 'NUKAGE2', 'NUKAGE3']
TEX_W, TEX_H = 64, 128          # power of two: texturewidthmask = 63 (r_data.c:559-562)
PATCH = 'BLANKP'                # one blank 64x128 patch shared by every texture


def name8(s):
    return s.upper().encode('ascii').ljust(8, b'\0')


def blank_patch(w, h):
    """patch_t (r_defs.h): short width,height,leftoffset,topoffset; int columnofs[w]; every column
    points at one 0xFF terminator (an empty column, no posts)."""
    end = 8 + 4 * w
    return struct.pack('<hhhh', w, h, 0, 0) + struct.pack('<%di' % w, *([end] * w)) + b'\xff'


def pnames():
    return struct.pack('<i', 1) + name8(PATCH)


def texture1():
    """TEXTURE1: int numtextures; int offset[n]; maptexture_t (on disk: name[8], int masked,
    short w, short h, int columndirectory, short patchcount, mappatch_t[patchcount] of five
    shorts)  -- 22 + 10*patchcount bytes (r_data.c:73-83, 505-526)."""
    n = len(TEXTURES)
    body = b''
    offsets = []
    base = 4 + 4 * n
    for t in TEXTURES:
        offsets.append(base + len(body))
        body += name8(t) + struct.pack('<ihhihhhhhh', 0, TEX_W, TEX_H, 0, 1, 0, 0, 0, 1, 0)
    return struct.pack('<i', n) + struct.pack('<%di' % n, *offsets) + body


def playpal():
    return bytes(768)


def colormap():
    return bytes(34 * 256)


# ---------------------------------------------------------------- the map ----------------
# Layout (map units).  Sector order fixes line fronts (see m3h9b_cases docstring):
#   0 hall (x -1152..1152, y -512..512) with 9 island holes
#   1..8 light/special islands, 9 sky island, 10 lift strip S, 11 lift P, 12..19 doors, 20..27 closets
HALL = (-1152, 1152, -512, 512)
ISLAND_SPECIALS = [1, 2, 3, 8, 12, 17, 7, 9]       # flicker, strobe fast, strobe slow, glow, strobe sync, fire flicker, nukage, secret
ISLAND_X = [-1000 + 270 * i for i in range(8)]
ISLAND_Y = (-200, -136)
SKY_X = 1020
DOOR_X = [-1030 + 280 * i for i in range(8)]
DOOR_W, DOOR_Y0, CLOSET_Y1 = 64, 512, 704      # door y 512..576, closet y 576..704
LIFT_S = (-1000, -700, -704, -512)             # strip S below the hall
LIFT_P = (-930, -770, -680, -600)
EXIT_Y = (0, 128)                              # exit switch on the west wall, x = -1152
DR, WR_LIFT, S1_EXIT, SCROLL = 1, 88, 11, 48
MTF_EASY, MTF_HARD, MTF_AMBUSH, MTF_NOTSINGLE = 1, 4, 8, 16


def build_map():
    x0, x1, y0, y1 = HALL
    # hall outer ring, clockwise y-up, with the extra vertices the shared edges need
    top = [(x0, y1)]
    for a in DOOR_X:
        top += [(a, y1), (a + DOOR_W, y1)]
    top += [(x1, y1)]
    bottom = [(x1, y0), (LIFT_S[1], y0), (LIFT_S[0], y0), (x0, y0)]
    west = [(x0, EXIT_Y[0]), (x0, EXIT_Y[1])]   # going up from y0: (x0,0) then (x0,128) then back to (x0,y1)
    outer = top + bottom + west
    # drop repeated consecutive points (corners)
    ring = []
    for p in outer:
        if not ring or ring[-1] != p:
            ring.append(p)
    # CCW holes (islands)
    holes = []
    islands = []
    for x in ISLAND_X + [SKY_X]:
        a, b = (x, ISLAND_Y[0]), (x + 64, ISLAND_Y[1])
        holes.append([(a[0], a[1]), (b[0], a[1]), (b[0], b[1]), (a[0], b[1])])
        islands.append([(a[0], b[1]), (b[0], b[1]), (b[0], a[1]), (a[0], a[1])])
    sx0, sx1, sy0, sy1 = LIFT_S
    px0, px1, py0, py1 = LIFT_P
    s_outer = [(sx0, sy1), (sx1, sy1), (sx1, sy0), (sx0, sy0)]
    s_hole = [(px0, py0), (px1, py0), (px1, py1), (px0, py1)]
    p_ring = [(px0, py1), (px1, py1), (px1, py0), (px0, py0)]
    sectors = [([ring] + holes, 0, 128, dict(light=160, floor='FLOOR7_1', ceiling='CEIL3_5'))]
    for i, sp in enumerate(ISLAND_SPECIALS):
        a = dict(light=192, special=sp, floor='TESTFLR0', ceiling='TESTCEIL')
        if sp == 7:
            a['floor'] = 'NUKAGE1'
        sectors.append(([islands[i]], 0, 128, a))
    sectors.append(([islands[8]], 0, 128, dict(light=192, floor='TESTFLR0', ceiling='F_SKY1')))  # sky island (not a mover)
    sectors.append(([s_outer, s_hole], 0, 128, dict(light=160, floor='FLOOR7_1', ceiling='CEIL3_5')))
    sectors.append(([p_ring], 48, 128, dict(light=160, tag=1, floor='FLOOR7_1', ceiling='CEIL3_5')))
    for a in DOOR_X:   # doors: closed, ceiling == floor (vanilla)
        sectors.append(([[(a, 576), (a + DOOR_W, 576), (a + DOOR_W, 512), (a, 512)]], 0, 0, dict(light=160, floor='FLOOR7_1', ceiling='CEIL3_5')))
    for a in DOOR_X:
        sectors.append(([[(a, CLOSET_Y1), (a + DOOR_W, CLOSET_Y1), (a + DOOR_W, 576), (a, 576)]], 0, 128, dict(light=160, floor='FLOOR7_1', ceiling='CEIL3_5')))
    lines = {}
    for a in DOOR_X:
        lines[((a, 512), (a + DOOR_W, 512))] = (DR, 0)             # hall's edge (hall is sector 0)
        lines[((a, 576), (a + DOOR_W, 576))] = (DR, 0)             # door's edge (door sector precedes closet)
    for x in ISLAND_X:                                              # 8 two-sided scrollers: hole bottom edges
        lines[((x, ISLAND_Y[0]), (x + 64, ISLAND_Y[0]))] = (SCROLL, 0)
    lines[((LIFT_S[1], y0), (LIFT_S[0], y0))] = (WR_LIFT, 1)       # WR lift trigger on the hall|S edge
    lines[((x0, EXIT_Y[0]), (x0, EXIT_Y[1]))] = (S1_EXIT, 0)       # S1 exit on the west wall
    return G.room(sectors, portals={}, reject=(), tree=None, segment=4096, lines=lines)


def make_things():
    """E1M1-scale THINGS: the 52 pickups, one player 1 start in the MIDDLE of the lump (45
    THINGs follow), 27 decorations, 6 barrels, 6 monsters and 3 filtered extras."""
    kinds = ([C.SHOTGUN] + [C.CLIP] * 2 + [C.SHELLS] * 2 + [C.STIMPACK] + [C.MEDIKIT] * 3 + [C.HEALTHBONUS] * 12 +
             [C.ARMORBONUS] * 25 + [C.GREENARMOR, C.BLUEARMOR, C.CLIPBOX] + [C.SHELLBOX] * 3)
    assert len(kinds) == 52
    things = []
    for i, k in enumerate(kinds):
        things.append((-1000 + 36 * i, 68 if i % 2 else 60, (0, 90, 180, 270)[i % 4], k, (7, 6, 3)[i % 3]))
    things.append((-1100, 64, 0, 1, 7))                                    # player 1 start (type 1)
    decos = [C.LAMP] * 6 + [C.PILLAR] * 3 + [C.CANDELABRA] * 3 + [C.GIBS] * 5 + [C.DEADPLAYER] * 4 + [C.MESS10] * 3 + [C.MESS12] * 3
    assert len(decos) == 27
    angles = (0, 45, 90, 135, 180, 225, 270, 315)
    for i, k in enumerate(decos):
        things.append((-980 + 72 * i, 250 + 30 * (i % 2), angles[i % 8], k, 7))
    for i in range(6):
        things.append((-900 + 120 * i, -300, angles[(i + 3) % 8], C.BARREL, (7, 6, 3)[i % 3]))
    for i, k in enumerate([C.POSSESSED] * 4 + [C.IMP] * 2):
        things.append((-800 + 300 * i, 420, angles[(i * 3 + 4) % 8], k, 7 | MTF_AMBUSH if i % 2 else 7))
    # skill / mode filtered extras (not spawned on medium single player)
    things.append((-500, 150, 0, C.CLIP, MTF_EASY))
    things.append((-400, 150, 270, C.POSSESSED, MTF_HARD))
    things.append((-300, 150, 180, C.LAMP, 7 | MTF_NOTSINGLE))
    return things


def fix_sides(m):
    """Give every side texture a name this WAD defines.  one-sided: STARTAN2 middle; two-sided:
    '-' middle; door lines (special 1): BIGDOOR2 upper on both sides; lift line: PLAT1 lower;
    the exit switch line keeps the builder's SW1COMM middle (found by P_ChangeSwitchTexture)."""
    side = bytearray(m['lumps']['SIDEDEFS'])
    for l, (special, _) in zip(m['lines'], m['line_specials']):
        for si, sd in enumerate(l['sides']):
            if sd < 0:
                continue
            at = sd * 30
            two = l['back'] is not None
            top = bot = mid = '-'
            if not two:
                mid = 'STARTAN2'
            if special == DR:
                top = 'BIGDOOR2'
            if special == WR_LIFT:
                bot = 'PLAT1'
            if special in (S1_EXIT,):
                mid = 'SW1COMM'
            side[at + 4:at + 12] = name8(top)
            side[at + 12:at + 20] = name8(bot)
            side[at + 20:at + 28] = name8(mid)
    m['lumps']['SIDEDEFS'] = bytes(side)


def build():
    m = build_map()
    fix_sides(m)
    things = make_things()
    lumps = dict(m['lumps'])
    lumps['THINGS'] = b''.join(struct.pack('<hhhhh', *t) for t in things)
    order = ['MAP01', 'THINGS', 'LINEDEFS', 'SIDEDEFS', 'VERTEXES', 'SEGS', 'SSECTORS', 'NODES', 'SECTORS', 'REJECT', 'BLOCKMAP']
    assert list(lumps) == order, list(lumps)
    mapl = [('E1M1', b'')] + [(n, lumps[n]) for n in order[1:]]
    flat = bytes(4096)
    wad = [('PLAYPAL', playpal()), ('COLORMAP', colormap()), ('S_START', b''), ('S_END', b''),
           ('F_START', b'')] + [(f, flat) for f in FLATS] + [('F_END', b''),
          ('PNAMES', pnames()), ('TEXTURE1', texture1()), (PATCH, blank_patch(TEX_W, TEX_H))] + mapl
    return wad, things, m


def wad_bytes():
    wad, things, m = build()
    data = bytearray(b'IWAD' + struct.pack('<ii', len(wad), 0))
    directory = b''
    for n, b in wad:
        directory += struct.pack('<ii8s', len(data), len(b), name8(n))
        data += b
    struct.pack_into('<i', data, 8, len(data))
    return bytes(data + directory), len(wad)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--out', type=Path, default=WAD)
    ap.add_argument('--check', action='store_true')
    a = ap.parse_args()
    data, n = wad_bytes()
    if a.check:
        if not WAD.exists() or WAD.read_bytes() != data:
            sys.exit(f'{WAD.relative_to(ROOT)} differs from the regenerated WAD')
        print(f'{WAD.relative_to(ROOT)} matches: {len(data)} bytes, {n} lumps, sha256 {hashlib.sha256(data).hexdigest()}', flush=True)
        sys.exit(subprocess.run([sys.executable, str(CHECKER), str(WAD.relative_to(ROOT))], cwd=ROOT).returncode)
    a.out.write_bytes(data)
    print(f'wrote {a.out}: {len(data)} bytes, {n} lumps')


if __name__ == '__main__':
    main()
