#!/usr/bin/env python3
"""Synthetic multi-map WADs, intermission and finale pictures, music placeholders and session scripts for M6a
(docs/M6A_REFERENCE_INVESTIGATION.md, owner's ruling U11).

Everything here is drawn or written by this tool: nothing comes from any IWAD. The genuine engine runs every item
(tools/reference/m6a_goldens.py) with the item's IWAD first, then tests/reference/m5b_gfx.wad (the M5b status bar,
HUD font, textures, sprites and palette), tests/reference/m5c_sfx.wad (the M5c sounds), the M6a picture PWAD, the
M6a music PWAD and the demo lump. Written under tests/reference/m6a_wads/ and tests/reference/m6a_scripts/:

  episode.wad   an IWAD with nine small maps E1M1-E1M9 (the M5a texture and flat directory, m5a_wad's framing and
                room builder, imported read-only). Every map is a corridor along +x with the player start at (0,0)
                facing east, so one block of commands walks to its exit whatever the player carries:
                  E1M1  exit switch 11 on the east wall (p_switch.c:359-363); a 5-damage floor (special 7) and a
                        secret sector (special 9, p_spec.c:1030-1052); the chainsaw, the blue keycard, the radiation
                        suit, green armor, a backpack and two health bonuses on the path; off the path, one health
                        bonus for skills 0-1, one stimpack for skill 2 and one armor bonus for skills 3-4 (the spawn
                        bits of P_SpawnMapThing, p_mobj.c:743-751), so every skill spawns a different thing set
                  E1M2  a 20-damage floor (special 16) near the start, for deaths and reborns; the shotgun just
                        before a W1 exit line 52 (p_spec.c:677-680), so the weapon change it starts is pending when
                        the level ends (cancelled by P_SetupPsprites, p_pspr.c:831-842)
                  E1M3  the secret exit switch 51 on the east wall (p_switch.c:431-435); the exit switch 11 on the
                        north wall of an alcove
                  E1M4  every floor at 64 except the last part at 0: a ledge, then a W1 exit line 52 a few tics'
                        walk past it, so the player lands just before the exit (deltaviewheight, p_mobj.c:250-258,
                        carried by G_PlayerFinishLevel, g_game.c:779-792); a 20-damage floor near the start
                  E1M5  a W1 teleport 97 into a separate room where a zombieman stands on the destination (the
                        telefrag gives the kill count) and an exit switch 11 on that room's east wall
                  E1M6  the yellow keycard, partial invisibility and the light amplification visor on the path
                        (keys and every power dropped at the exit, g_game.c:785-787) and an exit switch 11
                  E1M7  a W1 exit line 52
                  E1M8  a W1 teleport 97 into a separate room where a Baron of Hell stands on the destination: the
                        player telefrags it (PIT_StompThing, p_map.c), its death runs A_BossDeath, which lowers the
                        floors tagged 666 (p_enemy.c:1627-1635, 1729-1735); the lowered floor carries sector special
                        11, which damages 20 every 32 tics and exits at health 10 or less (p_spec.c:1054-1063)
                  E1M9  the exit switch 11 on the east wall (back to E1M4, g_game.c:1089-1096) and the secret exit
                        switch 51 on the north wall of an alcove (E1M9 again, g_game.c:1087-1088)
  rbx.wad       the reborn-beats-a-same-tic-exit witness (U8 item 12, observed in Phase 2 step 1): E1M1 and E1M2
                are one corridor whose west part is a 20-damage floor and whose W1 exit line 52 stands at x=1024; the
                player starts at (12,0) facing east, dies on the floor and slides across the line
  m6a_gfx.wad   our own pictures for the 77 intermission names wi_stuff.c caches that the M5b set lacks (WI_loadData,
                wi_stuff.c:1538-1704; the other two, STFST01 and STFDEAD0, are the M5b status bar faces, which this
                PWAD must not replace, since they appear in every level frame), FLOOR4_8 (a 4,096-byte flat read
                with W_CacheLumpName, f_finale.c:120, 340), HELP2 and CREDIT (f_finale.c:714-720). Sizes and
                offsets are chosen from the positions the C fixes and checked here: every picture drawn lies inside
                the screen (V_DrawPatch's range check, v_video.c), the pointer and the splat fit at every episode 1
                node (WI_drawOnLnode, wi_stuff.c:458-498, lnodes 117-130)
  m6a_music.wad the placeholders S_ChangeMusic looks up (s_sound.c:649-686): D_E1M1-D_E1M9 at the loads,
                D_INTER at the intermission's first tic (wi_stuff.c:1508-1515), D_VICTOR at F_StartFinale
                (f_finale.c:114); our own ASCII lines, not music (I_RegisterSong never reads them, i_sound.c:875-881)
  variants      for the refusal witnesses (U10): m6a_gfx_nohelp2.wad (no HELP2), m6a_music_nointer.wad (no
                D_INTER), m6a_music_novictor.wad (no D_VICTOR), m6a_music_noe1m2.wad (no D_E1M2, a later map's music)

The scripts (tools/reference/m4b_scripts.py format, imported read-only; SKILL_OVERRIDE patches header byte 1 for
skills 0 and 1, which that format does not admit) are in SCRIPTS; their tics were chosen by running the genuine
engine. Each item names its WAD set and game mode; the manifest tests/reference/m6a_manifest.json
(DOOM-M6A-MANIFEST-v1) lists every file with its digest and every item with its inputs.

    m6a_wad.py              write the WADs, the scripts and the manifest
    m6a_wad.py --list       name, file, digest; item, script, tics, mode
    m6a_wad.py --check      regenerate in memory and compare with every committed file (writes nothing)
    m6a_wad.py --out DIR    write under DIR instead (authoring)
"""
import argparse, hashlib, json, struct, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m4a_wad as A              # noqa: E402  frozen M4a lump order (read-only)
import m4b_wad as B              # noqa: E402  frozen M4b special set and spawn counts (read-only)
import m4b_scripts as S          # noqa: E402  frozen script format (read-only)
import m5a_wad as R              # noqa: E402  the M5a builder, styles and framing (read-only)
import m5a_gfx as X              # noqa: E402  the M5a name helper (read-only)

WAD_DIR = 'tests/reference/m6a_wads'
SCRIPT_DIR = 'tests/reference/m6a_scripts'
MANIFEST = 'tests/reference/m6a_manifest.json'
MANIFEST_SCHEMA = 'DOOM-M6A-MANIFEST-v1'
GFX_M5B, SFX_M5C = 'tests/reference/m5b_gfx.wad', 'tests/reference/m5c_sfx.wad'
EAST, NORTH, WEST, SOUTH = 0, 90, 180, 270
PLAYER1, ZOMBIE, BARON, TELEPORTMAN = 1, 3004, 3003, 14
SHOTGUN, CHAINSAW, BACKPACK, GREENARMOR = 2001, 2005, 8, 2018
HEALTHBONUS, ARMORBONUS, STIMPACK = 2014, 2015, 2011
BLUECARD, YELLOWCARD, SUIT, INVIS, LIGHTAMP = 5, 6, 2025, 2024, 2045
EASY, MEDIUM, HARD = 1, 2, 4                     # thing option bits (p_mobj.c:743-751)
S1_EXIT, S1_SECRET, W1_EXIT, WR_TELEPORT = 11, 51, 52, 97
HURT5, SECRET, EXIT11, HURT20 = 7, 9, 11, 16      # sector specials (p_spec.c:1030-1063)
SPECIALS = B.SPECIALS | {W1_EXIT}                 # M4b's set (part 1 with 11 and 51, plus 97) and the W1 exit


class Room(R.Room):
    """The M5a room with the M4b line special set plus the W1 exit 52."""

    def special(self, front, back, at, special, tag=0):
        assert special in SPECIALS, special
        self.specials.append((front, back, at, special, tag))

    def switch(self, front, at, special):
        self.special(front, None, at, special)
        self.style(front, None, at=at, s0=dict(mid='SW1COMM'))


def hall(p, label, x0, x1, special=0, floor=0, fpic='M5FLAT1', light=192):
    return p.sector(label, [(x0, -64, x1, 64)], floor=floor, ceil=floor + 128, light=light, special=special, fpic=fpic,
                    cpic='M5FLAT2')


def alcove(p, x0, x1, special, floor=0):
    """A 128-wide alcove north of the corridor with a switch on its north wall."""
    al = p.sector(f'ALCOVE{x0}', [(x0, 64, x1, 96)], floor=floor, ceil=floor + 128, light=176, fpic='M5FLAT3', cpic='M5FLAT2')
    p.switch(al, ((x0 + x1) // 2, 96), special)
    return al


# ---------------------------------------------------------------- the nine maps -------------------------
def e1m1():
    p = Room('e1m1', wall='M5WALLA')
    hall(p, 'START', -64, 128)
    hall(p, 'HURT5', 128, 256, special=HURT5, fpic='NUKAGE1')
    hall(p, 'SECRET', 256, 384, special=SECRET, fpic='M5FLAT4')
    last = hall(p, 'HALL', 384, 1024)
    p.switch(last, (1024, 0), S1_EXIT)
    p.thing(0, 0, EAST, PLAYER1)
    for x, kind in ((64, CHAINSAW), (448, BLUECARD), (512, SUIT), (576, GREENARMOR), (640, BACKPACK), (704, HEALTHBONUS),
                    (768, HEALTHBONUS)):
        p.thing(x, 0, EAST, kind)
    p.thing(832, 48, EAST, HEALTHBONUS, EASY)            # off the path (48 >= 16 + 20: never touched at y = 0)
    p.thing(832, -48, EAST, STIMPACK, MEDIUM)
    p.thing(896, 48, EAST, ARMORBONUS, HARD)
    return p


def e1m2():
    p = Room('e1m2', wall='M5WALLB')
    hall(p, 'START', -64, 128)
    hall(p, 'PIT', 128, 256, special=HURT20, fpic='NUKAGE1')
    h = hall(p, 'HALL', 256, 896)
    z = hall(p, 'EXITZ', 896, 1024, fpic='M5FLAT5')
    p.special(h, z, (896, 0), W1_EXIT)
    p.thing(0, 0, EAST, PLAYER1)
    p.thing(848, 0, EAST, SHOTGUN)
    return p


def e1m3():
    p = Room('e1m3', wall='M5WALLC')
    hall(p, 'START', -64, 128)
    last = hall(p, 'HALL', 128, 768)
    alcove(p, 352, 480, S1_EXIT)
    p.switch(last, (768, 0), S1_SECRET)
    p.thing(0, 0, EAST, PLAYER1)
    p.thing(256, 0, EAST, HEALTHBONUS)
    return p


def e1m4():
    p = Room('e1m4', wall='M5WALLE')
    hall(p, 'START', -64, 128, floor=64)
    hall(p, 'PIT', 128, 256, special=HURT20, floor=64, fpic='NUKAGE1')
    hall(p, 'LEDGE', 256, 640, floor=64)
    low = hall(p, 'LOW', 640, 768, fpic='M5FLAT6')
    z = hall(p, 'EXITZ', 768, 1024, fpic='M5FLAT5')
    p.special(low, z, (768, 0), W1_EXIT)
    p.thing(0, 0, EAST, PLAYER1)
    return p


def e1m5():
    p = Room('e1m5', wall='M5WALLF')
    s = hall(p, 'START', -64, 128)
    t = hall(p, 'TELE', 128, 256, fpic='M5FLAT7')
    p.special(s, t, (128, 0), WR_TELEPORT, 2)
    # a separate room (the zombieman neither sees nor hears the player before the teleport) whose destination is
    # on the zombieman: the telefrag kills it whatever the random numbers are (PIT_StompThing, p_map.c)
    last = p.sector('ROOM', [(512, -64, 1024, 64)], light=176, tag=2, fpic='M5FLAT8', cpic='M5FLAT2')
    p.switch(last, (1024, 0), S1_EXIT)
    p.thing(0, 0, EAST, PLAYER1)
    p.thing(576, 0, EAST, TELEPORTMAN)
    p.thing(576, 0, WEST, ZOMBIE)
    return p


def e1m6():
    p = Room('e1m6', wall='M5WALLA')
    hall(p, 'START', -64, 128)
    last = hall(p, 'HALL', 128, 768)
    p.switch(last, (768, 0), S1_EXIT)
    p.thing(0, 0, EAST, PLAYER1)
    for x, kind in ((192, YELLOWCARD), (320, INVIS), (448, LIGHTAMP)):
        p.thing(x, 0, EAST, kind)
    return p


def e1m7():
    p = Room('e1m7', wall='M5WALLB')
    hall(p, 'START', -64, 128)
    h = hall(p, 'HALL', 128, 384)
    z = hall(p, 'EXITZ', 384, 512, fpic='M5FLAT5')
    p.special(h, z, (384, 0), W1_EXIT)
    p.thing(0, 0, EAST, PLAYER1)
    return p


def e1m8():
    p = Room('e1m8', wall='M5WALLC')
    s = hall(p, 'START', -64, 128)
    t = hall(p, 'TELE', 128, 256, fpic='M5FLAT7')
    p.special(s, t, (128, 0), WR_TELEPORT, 1)
    # the arena: a separate room (no line joins it to the start, so the Baron neither sees nor hears the player
    # before the teleport); the destination is on the Baron
    p.sector('ARENA', [(512, -128, 768, 128)], light=160, tag=1, fpic='M5FLAT8', cpic='M5FLAT2')
    p.sector('BOSSFLOOR', [(768, -64, 896, 64)], floor=64, ceil=192, light=160, special=EXIT11, tag=666,
             fpic='NUKAGE1', cpic='M5FLAT2')
    p.thing(0, 0, EAST, PLAYER1)
    p.thing(576, 0, EAST, TELEPORTMAN)
    p.thing(576, 0, WEST, BARON)
    return p


def e1m9():
    p = Room('e1m9', wall='M5WALLE')
    hall(p, 'START', -64, 128)
    last = hall(p, 'HALL', 128, 768)
    alcove(p, 352, 480, S1_SECRET)
    p.switch(last, (768, 0), S1_EXIT)
    p.thing(0, 0, EAST, PLAYER1)
    return p


MAPS = {'E1M1': e1m1, 'E1M2': e1m2, 'E1M3': e1m3, 'E1M4': e1m4, 'E1M5': e1m5, 'E1M6': e1m6, 'E1M7': e1m7, 'E1M8': e1m8,
        'E1M9': e1m9}


def rbx(x0=12, line_x=1024):
    """The step 1 witness corridor (scratch rbx.py 12 1024), rebuilt byte for byte."""
    p = Room('rbx', wall='M5WALLB')
    h1 = p.sector('H1', [(-64, -64, line_x, 64)], light=192, special=HURT20, fpic='M5FLAT1', cpic='M5FLAT2')
    h2 = p.sector('H2', [(line_x, -64, line_x + 512, 64)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    p.special(h1, h2, (line_x, 0), W1_EXIT)
    p.thing(x0, 0, EAST, PLAYER1)
    return p


def wad(kind, lumps):
    data = bytearray(kind + struct.pack('<ii', len(lumps), 0))
    directory = b''
    for n, b in lumps:
        directory += struct.pack('<ii8s', len(data), len(b), X.name8(n))
        data += b
    struct.pack_into('<i', data, 8, len(data))
    return bytes(data + directory)


def map_lumps(room):
    m = R.build(room)
    R.apply_styles(room, m)
    lumps = dict(m['lumps'])
    lumps['THINGS'] = b''.join(struct.pack('<hhhhh', *t) for t in room.things)
    assert list(lumps) == ['MAP01'] + A.MAP_LUMPS, list(lumps)
    return [(n, lumps[n]) for n in A.MAP_LUMPS], m


def iwad(rooms):
    out = R.framing()
    for slot, room in rooms:
        lumps, _ = map_lumps(room)
        out += [(slot, b'')] + lumps
    return wad(b'IWAD', out)


# ---------------------------------------------------------------- pictures -------------------------------
# A 5x7 font of our own, scaled by an integer factor.
FONT = {
    'A': '01110 10001 10001 11111 10001 10001 10001', 'B': '11110 10001 10001 11110 10001 10001 11110',
    'C': '01111 10000 10000 10000 10000 10000 01111', 'D': '11110 10001 10001 10001 10001 10001 11110',
    'E': '11111 10000 10000 11110 10000 10000 11111', 'F': '11111 10000 10000 11110 10000 10000 10000',
    'G': '01111 10000 10000 10011 10001 10001 01111', 'H': '10001 10001 10001 11111 10001 10001 10001',
    'I': '11111 00100 00100 00100 00100 00100 11111', 'K': '10001 10010 10100 11000 10100 10010 10001',
    'L': '10000 10000 10000 10000 10000 10000 11111', 'M': '10001 11011 10101 10101 10001 10001 10001',
    'N': '10001 11001 10101 10011 10001 10001 10001', 'O': '01110 10001 10001 10001 10001 10001 01110',
    'P': '11110 10001 10001 11110 10000 10000 10000', 'R': '11110 10001 10001 11110 10100 10010 10001',
    'S': '01111 10000 10000 01110 00001 00001 11110', 'T': '11111 00100 00100 00100 00100 00100 00100',
    'U': '10001 10001 10001 10001 10001 10001 01110', 'V': '10001 10001 10001 10001 10001 01010 00100',
    'W': '10001 10001 10001 10101 10101 11011 10001', 'Y': '10001 10001 01010 00100 00100 00100 00100',
    '0': '01110 10011 10101 10101 11001 10001 01110', '1': '00100 01100 00100 00100 00100 00100 01110',
    '2': '01110 10001 00001 00110 01000 10000 11111', '3': '11110 00001 00001 01110 00001 00001 11110',
    '4': '00010 00110 01010 10010 11111 00010 00010', '5': '11111 10000 11110 00001 00001 10001 01110',
    '6': '00110 01000 10000 11110 10001 10001 01110', '7': '11111 00001 00010 00100 01000 01000 01000',
    '8': '01110 10001 10001 01110 10001 10001 01110', '9': '01110 10001 10001 01111 00001 00010 01100',
    '%': '11001 11010 00010 00100 01000 01011 10011', ':': '00000 01100 01100 00000 01100 01100 00000',
    '-': '00000 00000 00000 11111 00000 00000 00000', ' ': '00000 00000 00000 00000 00000 00000 00000',
    '!': '00100 00100 00100 00100 00100 00000 00100', 'X': '10001 10001 01010 00100 01010 10001 10001',
}


def text_pixels(text, scale, ink):
    """(w, h, rows) of `text` in FONT at `scale`, one blank column between letters; transparent is None."""
    cols = []
    for k, ch in enumerate(text):
        glyph = [row for row in FONT[ch].split()]
        for c in range(5):
            cols += [[ink if glyph[r][c] == '1' else None for r in range(7)]] * scale
        if k != len(text) - 1: cols += [[None] * 7] * scale
    w, h = len(cols), 7 * scale
    return w, h, [[cols[x][y // scale] for x in range(w)] for y in range(h)]


def patch(rows, left=0, top=0):
    """A patch_t (r_defs.h): header, column offsets, and per column one post per opaque run (topdelta, length, pad,
    pixels, pad), then 0xff. rows[y][x] is a palette index or None (transparent)."""
    h, w = len(rows), len(rows[0])
    assert 0 < w and 0 < h <= 254, (w, h)
    body, offs = b'', []
    base = 8 + 4 * w
    for x in range(w):
        offs.append(base + len(body))
        y = 0
        while y < h:
            if rows[y][x] is None: y += 1; continue
            y1 = y
            while y1 < h and rows[y1][x] is not None: y1 += 1
            body += bytes([y, y1 - y, 0]) + bytes(rows[k][x] for k in range(y, y1)) + b'\0'
            y = y1
        body += b'\xff'
    return struct.pack('<hhhh', w, h, left, top) + struct.pack(f'<{w}i', *offs) + body


def boxed(text, scale, ink, paper, pad=2, border=None):
    """`text` on an opaque `paper` box with an optional border colour."""
    w, h, px = text_pixels(text, scale, ink)
    W, H = w + 2 * pad, h + 2 * pad
    rows = [[paper] * W for _ in range(H)]
    for y in range(h):
        for x in range(w):
            if px[y][x] is not None: rows[y + pad][x + pad] = px[y][x]
    if border is not None:
        for x in range(W): rows[0][x] = rows[H - 1][x] = border
        for y in range(H): rows[y][0] = rows[y][W - 1] = border
    return rows


def label(text, scale, ink):
    """`text` with transparent paper."""
    return text_pixels(text, scale, ink)[2]


SCREEN_W, SCREEN_H = 320, 200
LNODES_E1 = [(185, 164), (148, 143), (69, 122), (209, 102), (116, 89), (166, 55), (71, 56), (135, 29), (71, 24)]  # wi_stuff.c:117-130


def wimap0():
    """The episode 1 background (WI_loadData draws it at 0,0 into screen 1, wi_stuff.c:1555-1556): a dark field with
    a grid, a frame and a small square at each of the nine nodes."""
    rows = [[(0 if (x % 32 == 0 or y % 25 == 0) else 4) for x in range(SCREEN_W)] for y in range(SCREEN_H)]
    for x in range(SCREEN_W): rows[0][x] = rows[SCREEN_H - 1][x] = 112
    for y in range(SCREEN_H): rows[y][0] = rows[y][SCREEN_W - 1] = 112
    for k, (nx, ny) in enumerate(LNODES_E1):
        for y in range(ny - 3, ny + 4):
            for x in range(nx - 3, nx + 4): rows[y][x] = 96 + 4 * k
    return [r[:] for r in rows]


def screen(title, sub, paper, ink):
    """A full-screen picture (HELP2, CREDIT) drawn at 0,0 (f_finale.c:714-720)."""
    rows = [[paper if (x // 20 + y // 20) % 2 else paper + 2 for x in range(SCREEN_W)] for y in range(SCREEN_H)]
    for txt, scale, y0 in ((title, 4, 60), (sub, 2, 110)):
        w, h, px = text_pixels(txt, scale, ink)
        x0 = (SCREEN_W - w) // 2
        for y in range(h):
            for x in range(w):
                if px[y][x] is not None: rows[y0 + y][x0 + x] = px[y][x]
    return rows


def gfx_lumps():
    """The 77 intermission pictures the M5b set lacks, FLOOR4_8, HELP2 and CREDIT."""
    out = [('WIMAP0', patch(wimap0()))]
    out += [(f'WILV0{i}', patch(boxed(f'E1M{i + 1}', 2, 160, 0, pad=2))) for i in range(9)]
    arrow = [[(176 if abs(x - 7) <= (15 - y) // 2 else None) for x in range(15)] for y in range(16)]
    out += [('WIURH0', patch(arrow, left=7, top=16))]                                   # points down at the node
    out += [('WIURH1', patch([[(208 if abs(x - 7) <= y // 2 else None) for x in range(15)] for y in range(16)], left=7, top=0))]
    out += [('WISPLAT', patch(label('X', 2, 176), left=5, top=7))]
    out += [(f'WIA0{j:02d}{i:02d}', patch(boxed(f'{j}{i}', 1, 4, 32 + 8 * i + j, pad=1))) for j in range(10) for i in range(3)]
    out += [('WIMINUS', patch(label('-', 2, 160)))]
    out += [(f'WINUM{i}', patch(label(str(i), 2, 160))) for i in range(10)]
    words = [('WIPCNT', '%'), ('WIF', 'FINISHED'), ('WIENTER', 'ENTERING'), ('WIOSTK', 'KILLS'), ('WIOSTS', 'SCRT'),
             ('WISCRT2', 'SECRET'), ('WIOSTI', 'ITEMS'), ('WIFRGS', 'FRGS'), ('WICOLON', ':'), ('WITIME', 'TIME'),
             ('WISUCKS', 'SUCKS'), ('WIPAR', 'PAR'), ('WIKILRS', 'KILLERS'), ('WIVCTMS', 'VICTIMS'), ('WIMSTT', 'TOTAL')]
    out += [(n, patch(label(t, 2, 208 if n in ('WIF', 'WIENTER') else 160))) for n, t in words]
    out += [(f'STPB{i}', patch(boxed(f'P{i + 1}', 1, 4, 112 + 16 * i))) for i in range(4)]
    out += [(f'WIBP{i}', patch(boxed(f'B{i}', 1, 4, 120 + 16 * i))) for i in range(1, 5)]
    out += [('FLOOR4_8', bytes((((x // 8) + (y // 8)) % 2) * 6 + 98 + (x + y) % 3 for y in range(64) for x in range(64)))]
    out += [('HELP2', patch(screen('HELP2', 'REGISTERED', 192, 4)))]
    out += [('CREDIT', patch(screen('CREDIT', 'RETAIL', 224, 4)))]
    return out


def music_lumps():
    names = [f'D_E1M{m}' for m in range(1, 10)] + ['D_INTER', 'D_VICTOR']
    return [(n, f'M6A placeholder {n}: not music; S_ChangeMusic caches it, I_RegisterSong ignores it.\n'.encode()) for n in names]


def drop(lumps, name):
    assert name in [n for n, _ in lumps], name
    return [(n, b) for n, b in lumps if n != name]


WIDTHS = lambda lumps: {n: struct.unpack_from('<hhhh', b, 0) for n, b in lumps if n != 'FLOOR4_8'}


def check_layout(lumps):
    """The checks of the module docstring, from the positions wi_stuff.c and f_finale.c fix."""
    d = WIDTHS(lumps)
    inside = lambda x, y, n: (0 <= x - d[n][2] and x - d[n][2] + d[n][0] <= SCREEN_W and 0 <= y - d[n][3]
                              and y - d[n][3] + d[n][1] <= SCREEN_H)
    for n in ('WIMAP0', 'HELP2', 'CREDIT'): assert d[n] == (SCREEN_W, SCREEN_H, 0, 0), n
    for n in ('WIURH0', 'WISPLAT'):                     # WI_drawOnLnode's own test (wi_stuff.c:470-480): strict right/bottom
        for x, y in LNODES_E1:
            l, t = x - d[n][2], y - d[n][3]
            assert l >= 0 and l + d[n][0] < SCREEN_W and t >= 0 and t + d[n][1] < SCREEN_H, (n, x, y)
    fh, fw = d['WINUM0'][1], d['WINUM0'][0]
    assert all(d[f'WINUM{i}'][:2] == (fw, fh) for i in range(10))
    for i in range(9):                                   # WI_drawLF / WI_drawEL (wi_stuff.c:420-454)
        w, h = d[f'WILV0{i}'][:2]
        assert inside((SCREEN_W - w) // 2, 2, f'WILV0{i}')
        for word in ('WIF', 'WIENTER'): assert inside((SCREEN_W - d[word][0]) // 2, 2 + 5 * h // 4, word)
    lh = 3 * fh // 2                                     # WI_drawStats (wi_stuff.c:1436-1466)
    for k, word in enumerate(('WIOSTK', 'WIOSTI', 'WISCRT2')):
        y = 50 + k * lh
        assert inside(50, y, word) and inside(270, y, 'WIPCNT') and inside(270 - 3 * fw, y, 'WINUM0')
    for x in (16, 176): assert inside(x, SCREEN_H - 32, 'WITIME' if x == 16 else 'WIPAR')
    assert inside(144 - 6 * fw - 2 * d['WICOLON'][0], SCREEN_H - 32, 'WINUM0')
    assert inside(304 - d['WISUCKS'][0], SCREEN_H - 32, 'WISUCKS')


# ---------------------------------------------------------------- scripts --------------------------------
SKILL_OVERRIDE = {}                                      # lump -> 0 or 1 (header byte 1, g_game.c:1596)

# The WAD sets: the item's IWAD, the M5b and M5c sets, the M6a pictures and music (or a variant).
WADSETS = {
    'episode': ['episode.wad', GFX_M5B, SFX_M5C, 'm6a_gfx.wad', 'm6a_music.wad'],
    'rbx': ['rbx.wad', GFX_M5B, SFX_M5C, 'm6a_gfx.wad', 'm6a_music.wad'],
    'nohelp2': ['episode.wad', GFX_M5B, SFX_M5C, 'm6a_gfx_nohelp2.wad', 'm6a_music.wad'],
    'nointer': ['episode.wad', GFX_M5B, SFX_M5C, 'm6a_gfx.wad', 'm6a_music_nointer.wad'],
    'novictor': ['episode.wad', GFX_M5B, SFX_M5C, 'm6a_gfx.wad', 'm6a_music_novictor.wad'],
    'noe1m2': ['episode.wad', GFX_M5B, SFX_M5C, 'm6a_gfx.wad', 'm6a_music_noe1m2.wad'],
}

# Script items: (item name, lump, game mode, WAD set, what, lines). The level-chain items (owner's order of work,
# docs/M6A_REFERENCE_INVESTIGATION.md section 6.5); every press tic was chosen by running the genuine engine.
SCRIPTS = [
    ("CHAIN", "CHAIN", "registered", "episode",
     "E1M1 to E1M8 with the carried player: the chainsaw ready at the E1M1 to E1M4 exits (sfx_sawup at the next four loads), keys and the radiation suit dropped, armor, health and the backpack carried, the shotgun change pending at the E1M2 exit (cancelled), a landing just before the E1M4 exit (the squat at the E1M5 start), a weapon change at the E1M5 load tic, partial invisibility and the light amplification visor dropped; each intermission skipped by three presses",
     ["map E1M1", "cmd 125 25 0 0 0", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1",
     "cmd 1 0 0 0 USE", "idle 10", "cmd 118 25 0 0 0", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1",
     "cmd 1 0 0 0 USE", "idle 10", "cmd 50 25 0 0 0", "idle 20", "cmd 1 0 0 64 0", "cmd 12 25 0 0 0", "cmd 1 0 0 0 USE", "idle 1",
     "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10", "cmd 102 25 0 0 0", "idle 1",
     "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10", "cmd 1 0 0 0 CHANGE|W2", "cmd 99 25 0 0 0",
     "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10",
     "cmd 96 25 0 0 0", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE",
     "idle 10", "cmd 56 25 0 0 0", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE",
     "idle 10", "idle 35"]),
    ("INTSTA", "INTSTA", "registered", "episode",
     "E1M1 to E1M8; the first press of each intermission in tally state 1 (fire on the first intermission tic), 2, 3, 4, 5, 6 and 7",
     ["map E1M1", "cmd 125 25 0 0 0", "cmd 1 0 0 0 USE", "cmd 1 0 0 0 ATTACK", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE",
     "idle 10", "cmd 118 25 0 0 0", "idle 35", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE",
     "idle 10", "cmd 50 25 0 0 0", "idle 20", "cmd 1 0 0 64 0", "cmd 12 25 0 0 0", "cmd 1 0 0 0 USE", "idle 36", "cmd 1 0 0 0 USE",
     "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10", "cmd 102 25 0 0 0", "idle 71", "cmd 1 0 0 0 USE",
     "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10", "cmd 100 25 0 0 0", "cmd 1 0 0 0 USE", "idle 122",
     "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10", "cmd 96 25 0 0 0", "cmd 1 0 0 0 USE",
     "idle 157", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10", "cmd 56 25 0 0 0",
     "idle 108", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10", "idle 10"]),
    ("INTSTB", "INTSTB", "registered", "episode",
     "E1M2 to E1M8 (a session started at map 2); the first press in tally state 8, then 9; state 10 reached with no press and held 20 tics; a press in ShowNextLoc; ShowNextLoc left to time out (140 tics); a press in NoState, which does nothing",
     ["map E1M2", "cmd 118 25 0 0 0", "idle 143", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE",
     "idle 10", "cmd 50 25 0 0 0", "idle 20", "cmd 1 0 0 64 0", "cmd 12 25 0 0 0", "cmd 1 0 0 0 USE", "idle 234", "cmd 1 0 0 0 USE",
     "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10", "cmd 102 25 0 0 0", "idle 209", "idle 20",
     "cmd 1 0 0 0 USE", "idle 30", "cmd 1 0 0 0 USE", "idle 10", "cmd 100 25 0 0 0", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE",
     "idle 1", "cmd 1 0 0 0 USE", "idle 150", "cmd 96 25 0 0 0", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1",
     "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 3", "cmd 1 0 0 0 USE", "idle 6", "cmd 56 25 0 0 0", "idle 1",
     "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10", "idle 10"]),
    ("INTIDLE", "INTIDLE", "registered", "episode",
     "E1M7 (a session started at map 7), then 400 intermission tics with no press: the tally counts to state 10 and waits",
     ["map E1M7", "cmd 56 25 0 0 0", "idle 400"]),
    ("INTHELD", "INTHELD", "registered", "episode",
     "use held from the E1M1 exit switch 80 tics into the intermission, then released and pressed; fire held from the level through the E1M2 exit 80 tics into the intermission, then released and pressed",
     ["map E1M1", "cmd 125 25 0 0 0", "cmd 81 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1",
     "cmd 1 0 0 0 USE", "idle 10", "cmd 60 25 0 0 0", "cmd 58 25 0 0 ATTACK", "cmd 80 0 0 0 ATTACK", "idle 1", "cmd 1 0 0 0 ATTACK", "idle 1",
     "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10", "idle 10"]),
    ("INTFAST", "INTFAST", "registered", "episode",
     "the shortest intermission: E1M7 left by its W1 line with no button held, then use, fire and use on the first three intermission tics; E1M8 loads on the 14th",
     ["map E1M7", "cmd 56 25 0 0 0", "cmd 1 0 0 0 USE", "cmd 1 0 0 0 ATTACK", "cmd 1 0 0 0 USE", "idle 10", "idle 10"]),
    ("REBHELD", "REBHELD", "registered", "episode",
     "E1M2: use held while dying on the 20-damage floor, so the reborn comes on the first tic of P_DeathThink; the reloaded map, its exit and an intermission",
     ["map E1M2", "cmd 22 25 0 0 0", "cmd 141 0 0 0 USE", "cmd 118 25 0 0 0", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE",
     "idle 1", "cmd 1 0 0 0 USE", "idle 10", "idle 10"]),
    ("REBLATE", "REBLATE", "registered", "episode",
     "E1M2: death on the 20-damage floor, use pressed 40 tics later; the reloaded map, its exit and an intermission",
     ["map E1M2", "cmd 22 25 0 0 0", "idle 178", "cmd 1 0 0 0 USE", "idle 1", "cmd 118 25 0 0 0", "idle 1", "cmd 1 0 0 0 USE",
     "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10", "idle 10"]),
    ("SECRET", "SECRET", "registered", "episode",
     "E1M3 secret exit to E1M9, the secret exit on E1M9 itself (E1M9 again), the E1M9 exit back to E1M4, the E1M4 exit; ShowNextLoc shown 30 tics each time (the E1M9 splat)",
     ["map E1M3", "cmd 96 25 0 0 0", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 30",
     "cmd 1 0 0 0 USE", "idle 10", "cmd 50 25 0 0 0", "idle 20", "cmd 1 0 0 64 0", "cmd 12 25 0 0 0", "cmd 1 0 0 0 USE", "idle 1",
     "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 30", "cmd 1 0 0 0 USE", "idle 10", "cmd 96 25 0 0 0", "cmd 1 0 0 0 USE",
     "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 30", "cmd 1 0 0 0 USE", "idle 10", "cmd 102 25 0 0 0",
     "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 30", "cmd 1 0 0 0 USE", "idle 10", "idle 10"]),
    ("REBSECRT", "REBSECRT", "registered", "episode",
     "E1M3 secret exit, E1M9, back to E1M4, death and a reborn there (didsecret cleared), then the E1M4 exit: the intermission without the E1M9 splat",
     ["map E1M3", "cmd 96 25 0 0 0", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 30",
     "cmd 1 0 0 0 USE", "idle 10", "cmd 96 25 0 0 0", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE",
     "idle 30", "cmd 1 0 0 0 USE", "idle 10", "cmd 22 25 0 0 0", "idle 190", "cmd 1 0 0 0 USE", "idle 1", "cmd 102 25 0 0 0",
     "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 30", "cmd 1 0 0 0 USE", "idle 10", "idle 10"]),
    ("PROBE", "PROBE", "registered", "rbx",
     "U8 item 12 probe: forward every tic; the player dies on the floor and the sliding body crosses the W1 exit on tic 131 (the exit taken by a dead player)",
     ["cmd 300 25 0 0 0"]),
    ("USEAT130", "USEAT130", "registered", "rbx",
     "U8 item 12 control: use on tic 130, before the crossing: the reborn reloads the map",
     ["cmd 130 25 0 0 0", "cmd 1 25 0 0 USE", "cmd 60 25 0 0 0"]),
    ("USEAT131", "USEAT131", "registered", "rbx",
     "U8 item 12 witness: use on tic 131, the tic the dead body crosses the exit: the reborn wins and the map reloads",
     ["cmd 131 25 0 0 0", "cmd 1 25 0 0 USE", "cmd 60 25 0 0 0"]),
    ("USEAT132", "USEAT132", "registered", "rbx",
     "U8 item 12 control: use on tic 132, after the crossing: the exit stands and the use is a tally press",
     ["cmd 132 25 0 0 0", "cmd 1 25 0 0 USE", "cmd 60 25 0 0 0"]),
    ("RBXDEAD", "RBXDEAD", "registered", "rbx",
     "a player dead at the exit (the probe's crossing), the intermission, and the next map started with a pistol (PST_DEAD made PST_REBORN at the load)",
     ["cmd 132 25 0 0 0", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 10",
     "idle 40"]),
    ("NOINTER", "NOINTER", "registered", "nointer",
     "refusal witness: D_INTER missing; the genuine engine stops on the intermission's first tic",
     ["map E1M7", "cmd 56 25 0 0 0", "idle 20"]),
    ("NOE1M2", "NOE1M2", "registered", "noe1m2",
     "refusal witness: D_E1M2 missing; the genuine engine stops at the E1M2 load tic after the E1M1 intermission",
     ["map E1M1", "cmd 125 25 0 0 0", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1", "cmd 1 0 0 0 USE", "idle 1",
     "cmd 1 0 0 0 USE", "idle 10", "idle 20"]),
    ("NOE1M2AT", "NOE1M2AT", "registered", "noe1m2",
     "refusal witness: D_E1M2 missing and the session starts at E1M2; the genuine engine stops at the first load, before tic 0",
     ["map E1M2", "idle 10"]),
]


def files():
    """(relative path, bytes) of every generated WAD."""
    gfx, mus = gfx_lumps(), music_lumps()
    check_layout(gfx)
    rb = rbx()
    return [(f'{WAD_DIR}/episode.wad', iwad([(slot, f()) for slot, f in MAPS.items()])),
            (f'{WAD_DIR}/rbx.wad', iwad([('E1M1', rb), ('E1M2', rb)])),
            (f'{WAD_DIR}/m6a_gfx.wad', wad(b'PWAD', gfx)),
            (f'{WAD_DIR}/m6a_music.wad', wad(b'PWAD', mus)),
            (f'{WAD_DIR}/m6a_gfx_nohelp2.wad', wad(b'PWAD', drop(gfx, 'HELP2'))),
            (f'{WAD_DIR}/m6a_music_nointer.wad', wad(b'PWAD', drop(mus, 'D_INTER'))),
            (f'{WAD_DIR}/m6a_music_novictor.wad', wad(b'PWAD', drop(mus, 'D_VICTOR'))),
            (f'{WAD_DIR}/m6a_music_noe1m2.wad', wad(b'PWAD', drop(mus, 'D_E1M2')))]


def script_text(lump, what, lines):
    return f'# M6a script {lump}: {what}.\n# Generated by tools/reference/m6a_wad.py.\n' + '\n'.join(lines) + '\n'


def script_lump(lump, what, lines):
    data, tics = S.lump(script_text(lump, what, lines))
    if lump in SKILL_OVERRIDE:
        data = data[:1] + bytes([SKILL_OVERRIDE[lump]]) + data[2:]
    return data, tics


def scripts():
    """(lump, txt, lmp, tics) per distinct lump, in first-use order."""
    out, seen = [], {}
    for item in SCRIPTS:
        name, lump, mode, wadset, what, lines = item
        txt = script_text(lump, what, lines)
        if lump in seen:
            assert seen[lump] == txt, f'{lump}: two items give one lump different text'
            continue
        seen[lump] = txt
        lmp, tics = script_lump(lump, what, lines)
        out.append((lump, txt, lmp, tics))
    return out


def manifest(wads, scr):
    tics = {l: t for l, _x, _b, t in scr}
    lmps = {l: b for l, _x, b, _t in scr}
    ep = {slot: f() for slot, f in MAPS.items()}
    maps = []
    for slot, room in ep.items():
        _, m = map_lumps(room)
        maps.append(dict(slot=slot, sectors=len(room.sectors), lines=len(m['lines']),
                         things={str(s): B.spawned(room, s) for s in range(4)}))
    return dict(schema=MANIFEST_SCHEMA, generator='tools/reference/m6a_wad.py',
                files=[dict(file=f, bytes=len(b), sha256=hashlib.sha256(b).hexdigest()) for f, b in wads],
                episode_maps=maps,
                wadsets={k: [w if '/' in w else f'{WAD_DIR}/{w}' for w in v] for k, v in WADSETS.items()},
                items=[dict(name=n, lump=l, mode=mode, wadset=ws, what=what, map=lmps[l][3], skill=lmps[l][1], tics=tics[l],
                            txt=f'{SCRIPT_DIR}/{l}.txt', lmp=f'{SCRIPT_DIR}/{l}.lmp',
                            lmp_sha256=hashlib.sha256(lmps[l]).hexdigest()) for n, l, mode, ws, what, _ in SCRIPTS])


def generate():
    wads = files()
    scr = scripts()
    out = list(wads)
    for lump, txt, lmp, _t in scr:
        out += [(f'{SCRIPT_DIR}/{lump}.txt', txt.encode()), (f'{SCRIPT_DIR}/{lump}.lmp', lmp)]
    out.append((MANIFEST, (json.dumps(manifest(wads, scr), indent=1) + '\n').encode()))
    return out, scr


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--list', action='store_true')
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--out', type=Path, help='write under this directory instead (authoring)')
    a = ap.parse_args()
    out, scr = generate()
    if a.list:
        for f, b in out:
            if f.endswith('.wad'): print(f'{f}: {len(b)} bytes, sha256 {hashlib.sha256(b).hexdigest()}')
        tics = {l: t for l, _x, _b, t in scr}
        for n, l, mode, ws, what, _ in SCRIPTS: print(f'{n}: {l}.lmp {tics[l]} tics, {mode}, {ws}: {what}')
        return
    if a.check:
        bad = [f for f, b in out if not (ROOT / f).exists() or (ROOT / f).read_bytes() != b]
        want = {f for f, _ in out}
        extra = [str(p.relative_to(ROOT)) for d in (WAD_DIR, SCRIPT_DIR) if (ROOT / d).is_dir()
                 for p in sorted((ROOT / d).iterdir()) if str(p.relative_to(ROOT)) not in want]
        if bad or extra: sys.exit('differs or missing: ' + ', '.join(bad) + ('; not generated: ' + ', '.join(extra) if extra else ''))
        print(f'{len(out)} files match ({len(SCRIPTS)} items, {len(scr)} scripts)')
        return
    base = a.out.resolve() if a.out else ROOT
    for f, b in out:
        (base / f).parent.mkdir(parents=True, exist_ok=True)
        (base / f).write_bytes(b)
    print(f'wrote {len(out)} files under {base}')


if __name__ == '__main__':
    main()
