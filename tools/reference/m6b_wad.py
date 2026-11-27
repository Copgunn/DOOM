#!/usr/bin/env python3
"""Synthetic map slots, the pause picture and the demo scripts of the M6b genuine-engine items
(docs/M6B_REFERENCE_INVESTIGATION.md section 6: Nightmare's rules, the -respawn and -fast parameters, the pause).

Everything here is drawn by this tool: nothing comes from any IWAD. Written under tests/reference/m6b_wads/ and
tests/reference/m6b_scripts/:

  m6b.wad      an IWAD with eight map slots built with the M6a room builder (m6a_wad.Room and m6a_wad.iwad, imported
               read-only). Things are placed by doomednum (info.c); angles in degrees; options are the THINGS flags
               (1 easy, 2 medium, 4 hard, 8 ambush, 16 not in single player; p_mobj.c:740-751, 795-796).
    E1M1 AMMO    wire 28's corridor over every P_GiveAmmo path of Episode 1 (wire28_wad.ammo, read-only)
    E1M2 BITS    five sleeping things facing away from the player, one per option pattern: a zombieman (4), an imp
                 (1|8), a demon (2), a shotgun guy (4|16) and a barrel (1). Nightmare spawns the zombieman only; a
                 port computing the bit as 1 << (skill - 1) would spawn the imp (bit 8 is the ambush flag)
    E1M3 REACT   an imp and a zombieman facing the player across a room: the first attack with reactiontime 0
                 (p_mobj.c:503-504) and A_Chase's two rules (p_enemy.c:719-720, 738-742)
    E1M4 FAST    a demon and a spectre running at the player, an imp and a Baron of Hell throwing from a ledge they
                 cannot leave: the halved state durations and the missile speeds of g_game.c:1421-1428
    E1M5 RESPAWN the player on a ledge 32 above a 128-wide pit corridor with two demons (the second with the ambush flag, facing
                 north); a shotgun and shells on the ledge; an exit switch behind the player. The demons cannot climb
                 the ledge (p_map.c P_TryMove, 24 units) and their bite does not reach (p_enemy.c P_CheckMeleeRange)
    E1M6 BLOCK   three zombiemen in the player's line of fire, asleep with their backs to him (ambush, facing east):
                 Z1's spawn point is 1 unit from the east wall, Z2's overlaps a solid firestick (MT_MISC43, doomednum 46, radius 16), Z3
                 stands free. All three are shot dead; P_NightmareRespawn's P_CheckPosition (p_mobj.c:370) fails
                 for Z1 and Z2 at every gate, and for Z3 while the player stands on its spawn point
    E1M7 GIB     a zombieman asleep inside a door sector (ambush, facing east); the player opens the door (DR 1),
                 kills it in the doorway, and the door closes on the corpse: PIT_ChangeSector turns it into S_GIBS
                 with radius 0 and height 0 (p_map.c:1247-1258) and it still respawns, inside the closed door
    E1M8 PAUSE   an imp on a ledge it cannot leave throwing at the player, a clip on the player's path (a message)
                 and a demon in the room: what stops and what keeps running while paused
  m6b_gfx.wad  a PWAD with M_PAUSE, our own picture (d_main.c:310); 62 by 18, drawn where the original centres its
               68-wide one
  scripts      tools/reference/m4b_scripts.py format (read-only) plus `cmdp N FORWARD SIDE TURN 0xBB`: N tics with the
               raw button byte 0xBB, BT_SPECIAL (0x80) allowed, which that format and m6a_wad's cmdx refuse. The demo
               header's skill, respawnparm and fastparm bytes (bytes 1, 5 and 6; g_game.c:1596-1601) are patched
               after the frozen format, which admits skills 2 and 3 and zero parameters only.

ITEMS: name -> dict(map, skill, respawn, fast, lines, what) for a new-game level of m6b.wad, or dict(base, edits,
what) for an item made from a committed M6a script lump (tests/reference/m6a_scripts/<base>.lmp, read-only) by byte
edits: ('set', tic, count, buttons) writes the button byte of `count` commands from session tic `tic`; ('cut', tic)
ends the demo before session tic `tic`. Their WAD set is the M6a episode set plus m6b_gfx.wad.

    m6b_wad.py            write the WADs and the scripts
    m6b_wad.py --check    regenerate in memory and compare with the committed files (writes nothing)
    m6b_wad.py --list     files, digests, items
"""
import argparse, hashlib, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m4b_scripts as S          # noqa: E402  the script format (read-only)
import m6a_wad as W              # noqa: E402  the room builder, WAD writer and picture helpers (read-only)
import wire28_wad as W28         # noqa: E402  the ammunition corridor and its walk (read-only)

WAD = 'tests/reference/m6b_wads/m6b.wad'
GFX = 'tests/reference/m6b_wads/m6b_gfx.wad'
SCRIPT_DIR = 'tests/reference/m6b_scripts'
SHARED = ['tests/reference/m5b_gfx.wad', 'tests/reference/m5c_sfx.wad', 'tests/reference/m6a_wads/m6a_gfx.wad',
          'tests/reference/m6a_wads/m6a_music.wad', GFX]
WADSET = [WAD] + SHARED
EPISODE = ['tests/reference/m6a_wads/episode.wad'] + SHARED          # the M6a episode set plus the pause picture
E, N, WE, SO = 0, 90, 180, 270
PLAYER, ZOMBIE, SHOTGUY, IMP, DEMON, SPECTRE, BARON, BARREL, FIRESTICK = 1, 3004, 9, 3001, 3002, 58, 3003, 2035, 46
CLIP, SHELLBOX, SHOTGUN, STIM = 2007, 2049, 2001, 2011
EASY, MEDIUM, HARD, AMBUSH, NOTSINGLE = 1, 2, 4, 8, 16
ALL = EASY | MEDIUM | HARD
DR_DOOR, EXIT = 1, 11
PAUSE = 0x81                                              # BT_SPECIAL | BTS_PAUSE (d_event.h:80, 91)


def room(name, x0, y0, x1, y1, **kw):
    p = W.Room(name, wall='M5WALLA')
    p.sector('ROOM', [(x0, y0, x1, y1)], light=192, fpic='M5FLAT1', cpic='M5FLAT2', **kw)
    return p


def bits():
    p = room('bits', -64, -64, 1088, 64)
    p.thing(0, 0, E, PLAYER)
    for x, kind, options in ((256, ZOMBIE, HARD), (448, IMP, EASY | AMBUSH), (640, DEMON, MEDIUM),
                             (832, SHOTGUY, HARD | NOTSINGLE), (1024, BARREL, EASY)):
        p.thing(x, 0, E, kind, options)
    return p


def react():
    p = room('react', -128, -256, 640, 256)
    p.thing(0, 0, E, PLAYER)
    p.thing(448, 0, WE, IMP)
    p.thing(448, 160, WE, ZOMBIE)
    return p


def ledge_room(name, x0, y0, x1, y1, ledge):
    """A room with a ledge sector (floor 48) along its north wall that no monster leaves (p_map.c P_TryMove)."""
    p = room(name, x0, y0, x1, y1)
    p.sector('LEDGE', [ledge], floor=48, ceil=128, light=192, fpic='M5FLAT3', cpic='M5FLAT2')
    return p


def fast():
    p = ledge_room('fast', -128, -256, 1280, 256, (256, 256, 1024, 384))
    p.thing(0, 0, E, PLAYER)
    p.thing(1152, -128, WE, DEMON)
    p.thing(1152, 128, WE, SPECTRE)
    p.thing(448, 320, SO, IMP)
    p.thing(832, 320, SO, BARON)
    return p


def respawn():
    p = W.Room('respawn', wall='M5WALLA')
    p.sector('PIT', [(0, 192, 1024, 320)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    led = p.sector('LEDGE', [(-192, 192, 0, 320)], floor=32, ceil=128, light=192, fpic='M5FLAT3', cpic='M5FLAT2')
    p.switch(led, (-192, 256), EXIT)
    p.thing(-96, 256, E, PLAYER)
    p.thing(-64, 256, E, SHOTGUN)
    p.thing(-48, 256, E, SHELLBOX)
    p.thing(-32, 256, E, SHELLBOX)
    p.thing(640, 256, WE, DEMON)
    p.thing(832, 256, N, DEMON, ALL | AMBUSH)
    return p


Z3 = (448, 0)                                             # the free zombieman's spawn point


def block():
    p = room('block', -64, -128, 1024, 128)
    p.thing(0, 0, E, PLAYER)
    p.thing(32, 0, E, SHOTGUN)
    p.thing(Z3[0], Z3[1], E, ZOMBIE, ALL | AMBUSH)        # Z3
    p.thing(640, 0, E, ZOMBIE, ALL | AMBUSH)              # Z2: overlaps the firestick (20 + 16 > 30)
    p.thing(640, 30, E, FIRESTICK)
    p.thing(1023, 0, E, ZOMBIE, ALL | AMBUSH)             # Z1: 1 unit from the east wall
    return p


def gib():
    p = W.Room('gib', wall='M5WALLA')
    a = p.sector('WEST', [(-64, -64, 192, 64)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    b = p.sector('EAST', [(256, -64, 512, 64)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    d = p.sector('DOOR', [(192, -64, 256, 64)], floor=0, ceil=0, light=192, fpic='M5FLAT1', cpic='M5FLAT2', door=True)
    W.R.Room.special(p, a, d, (192, 0), DR_DOOR)          # the M5a room special set (m5a_wad.ROOM_SPECIALS) has DR 1
    W.R.Room.special(p, b, d, (256, 0), DR_DOOR)
    p.thing(0, 0, E, PLAYER)
    p.thing(32, 0, E, SHOTGUN)
    p.thing(224, 0, E, ZOMBIE, ALL | AMBUSH)
    return p


def pause():
    p = ledge_room('pause', -128, -256, 768, 256, (256, 256, 640, 384))
    p.thing(0, 0, E, PLAYER)
    p.thing(48, 0, E, CLIP)
    p.thing(448, 320, SO, IMP)
    p.thing(704, -192, WE, DEMON)
    return p


ROOMS = {1: W28.ammo, 2: bits, 3: react, 4: fast, 5: respawn, 6: block, 7: gib, 8: pause}

BLAST = ['cmd 1 0 0 0 ATTACK', 'idle 44']                 # one shotgun blast, released before A_ReFire
FAST_LINES = ['cmd 90 0 0 0 ATTACK', 'idle 150']
REACT_LINES = ['idle 240']
# The shotgun picked up, both demons shot from the ledge, a wait over both respawns (session tics 736 and 896 on
# Nightmare; the initial 15 idle tics were chosen from genuine runs for early gate passes), both shot again, a half turn,
# the walk to the exit switch, the use, and 200 tics of the intermission.
RESPAWN_LINES = ['idle 15', 'cmd 8 25 0 0 0', 'idle 30', 'cmd 400 0 0 0 ATTACK', 'idle 500', 'cmd 300 0 0 0 ATTACK', 'idle 20',
                 'cmd 2 0 0 64 0', 'cmd 40 25 0 0 0', 'cmd 1 0 0 0 USE', 'idle 200']
# The shotgun, one blast held until all three are dead (the first kills Z3 and Z2, later ones Z1), the walk onto Z3's
# spawn point, a wait over one blocked gate of each corpse (session tics 512, 1248 and 1408 in the genuine run), 20 tics
# backwards, and a wait over Z3's next gate (1568), which now respawns it.
BLOCK_LINES = ['cmd 2 25 0 0 0', 'idle 40', 'cmd 150 0 0 0 ATTACK', 'idle 30', 'cmd 52 25 0 0 0', 'idle 1150', 'cmd 20 -25 0 0 0',
               'idle 200']
# The shotgun, the walk to the door, the use, a blast into the opening door, and a wait over the door's closing on the
# corpse (S_GIBS at session tic 334 in the genuine run) and the respawn (1024).
GIB_LINES = ['cmd 2 25 0 0 0', 'idle 40', 'cmd 24 25 0 0 0', 'cmd 1 0 0 0 USE', 'idle 30', 'cmd 10 0 0 0 ATTACK', 'idle 960']
# A clip picked up (a message), the imp awake; pause on with movement, turning and fire held for 40 tics; pause off by
# a command that also moves; fire; on and off in consecutive tics; on by 0x85 (other bits set); 150 tics (the message
# times out and the face changes while paused); off by 0xfd; 0x80 and 0x83 (BT_SPECIAL without the pause command);
# on again, and the demo ends paused.
PAUSE_LINES = ['cmd 12 25 0 0 0', 'idle 60', 'cmdp 1 0 0 0 0x81', 'cmd 40 25 10 4 ATTACK', 'cmdp 1 25 0 0 0x81', 'cmd 20 0 0 0 ATTACK',
               'cmdp 2 0 0 0 0x81', 'cmdp 1 0 0 0 0x85', 'idle 150', 'cmdp 1 0 0 0 0xfd', 'cmdp 1 0 0 0 0x80', 'cmdp 1 0 0 0 0x83',
               'idle 30', 'cmdp 1 0 0 0 0x81', 'idle 20']


def item(m, skill, lines, what, respawn=0, fast=0):
    return dict(map=m, skill=skill, respawn=respawn, fast=fast, lines=lines, what=what)


ITEMS = {
    'AMMO4': item(1, 4, W28.WALK, 'Nightmare: every P_GiveAmmo path of Episode 1 doubled before the cap (p_inter.c:95-101)'),
    'BITS4': item(2, 4, ['idle 20'], 'Nightmare spawns the things of the hard bit, 4 (p_mobj.c:745-746)'),
    'BITS3': item(2, 3, ['idle 20'], 'skill 3: the same set'),
    'REACT4': item(3, 4, REACT_LINES, 'Nightmare: reactiontime 0 at spawn; A_Chase without the new direction after an attack and '
                                      'without the movecount test before a missile'),
    'REACT3': item(3, 3, REACT_LINES, 'skill 3: the same commands (reactiontime 8, both A_Chase rules)'),
    'REACT3F': item(3, 3, REACT_LINES, 'skill 3 with fastparm: reactiontime 8, A_Chase as on Nightmare', fast=1),
    'FAST4': item(4, 4, FAST_LINES, 'Nightmare: demon and spectre state durations halved, imp and Baron missiles at 20'),
    'FAST3': item(4, 3, FAST_LINES, 'skill 3: the same commands (durations 2, 8 and 2; missiles at 10 and 15)'),
    'FAST3F': item(4, 3, FAST_LINES, 'skill 3 with fastparm: the tables of Nightmare, reactiontime 8', fast=1),
    'RESP4': item(5, 4, RESPAWN_LINES, 'Nightmare: two demons shot from a ledge respawn; one is shot again (kills above the '
                                          'level total) and the exit switch is used: the intermission counts past 100 %'),
    'RESP3': item(5, 3, RESPAWN_LINES, 'skill 3: the same commands; no corpse respawns'),
    'RESP3R': item(5, 3, RESPAWN_LINES, 'skill 3 with respawnparm: corpses respawn; reactiontime 8, nothing doubled', respawn=1),
    'BLOCK4': item(6, 4, BLOCK_LINES, 'Nightmare: a spawn point blocked by a wall, by a solid thing, and by the player and then free'),
    'GIB4': item(7, 4, GIB_LINES, 'Nightmare: a corpse crushed to gibs by a door respawns inside the closed door'),
    'PAUSE': item(8, 3, PAUSE_LINES, 'the pause command in a level: what stops, what runs, odd special bytes, the demo ending paused'),
    'PAUSE0': item(8, 3, ['cmdp 1 0 0 0 0x81', 'cmd 20 25 0 0 0', 'cmdp 1 0 0 0 0x81', 'cmd 20 25 0 0 0'],
                   'the pause command as the first command of a demo'),
    'PAUSEWI': dict(base='CHAIN', edits=[('set', 127, 1, PAUSE), ('set', 129, 1, PAUSE), ('set', 135, 1, PAUSE), ('cut', 160)],
                    what='the pause command in the intermission: a fire press as well (wi_stuff.c reads the raw byte), the '
                         'tally goes on, the picture is drawn, and the next load clears paused without S_ResumeSound '
                         '(g_game.c:494)'),
    'PAUSEFIN': dict(base='FINALE', edits=[('set', 400, 1, PAUSE), ('set', 460, 1, PAUSE), ('set', 1900, 1, PAUSE)],
                     what='the pause command in the finale: F_Ticker goes on, the picture is drawn over the text and the art '
                          'screen, the demo ends paused'),
}


def raw_split(text):
    """(the text with every `cmdp` line as a zero `cmd` line of the same count, [(first tic, count, ticcmd bytes)], the
    tic count)."""
    out, patches, tic = [], [], 0
    for number, raw in enumerate(text.splitlines(), 1):
        words = raw.split('#', 1)[0].split()
        if words and words[0] == 'cmdp':
            def bad(why): raise S.E.ScriptError(f'line {number}: cmdp: {why}')
            if len(words) != 6 or not words[5].lower().startswith('0x'): bad('expected "cmdp N FORWARD SIDE TURN 0xBB"')
            n, f, s, t, b = *(int(w) for w in words[1:5]), int(words[5], 16)
            if n < 1: bad('count below 1')
            if not -127 <= f <= 127: bad('forward outside -127..127 (-128 is the 0x80 demo marker)')
            if not -128 <= s <= 127 or not -128 <= t <= 255 or not 0 <= b <= 0xff: bad('a byte out of range')
            if b & 0x83 == 0x82: bad('BTS_SAVEGAME writes a file (g_game.c:714-720): not a command of these items')
            patches.append((tic, n, bytes([f & 0xff, s & 0xff, t & 0xff, b])))
            out.append(f'cmd {n} 0 0 0 0'); tic += n
            continue
        if words and words[0] in ('idle', 'cmd'): tic += int(words[1])
        elif words and words[0] == 'seed': raise S.E.ScriptError(f'line {number}: chaos lines are not counted here')
        out.append(raw)
    return '\n'.join(out) + '\n', patches, tic


def script_text(name):
    it = ITEMS[name]
    if 'base' in it:
        return (f'# M6b item {name}: {it["what"]}.\n# Generated by tools/reference/m6b_wad.py from '
                f'tests/reference/m6a_scripts/{it["base"]}.lmp by the byte edits below (session tics).\n'
                + '\n'.join(' '.join(str(x) if not isinstance(x, int) or i < 1 or e[0] != 'set' or i < 3 else hex(x)
                                     for i, x in enumerate(e)) for e in it['edits']) + '\n')
    return (f'# M6b item {name}: {it["what"]}.\n# Generated by tools/reference/m6b_wad.py; header bytes: skill {it["skill"]}, '
            f'respawnparm {it["respawn"]}, fastparm {it["fast"]}.\n' + '\n'.join([f'map E1M{it["map"]}'] + it['lines']) + '\n')


def lump(name):
    """(the demo lump, its tic count)."""
    it = ITEMS[name]
    if 'base' in it:
        data = bytearray((ROOT / f'{W.SCRIPT_DIR}/{it["base"]}.lmp').read_bytes())
        assert data[-1] == 0x80 and (len(data) - 14) % 4 == 0
        for e in it['edits']:
            tics = (len(data) - 14) // 4
            if e[0] == 'set':
                _, tic, count, b = e
                assert tic + count <= tics, (name, e)
                for t in range(tic, tic + count): data[13 + 4 * t + 3] = b
            elif e[0] == 'cut':
                assert e[1] <= tics, (name, e)
                data = data[:13 + 4 * e[1]] + b'\x80'
            else: raise ValueError(e)
        return bytes(data), (len(data) - 14) // 4
    plain, patches, count = raw_split(script_text(name))
    data, tics = S.lump(plain)
    assert tics == count, f'{name}: {tics} tics against {count} counted'
    data = bytearray(data)
    for first, n, cmd in patches:
        for t in range(first, first + n): data[13 + 4 * t:17 + 4 * t] = cmd
    data[1], data[5], data[6] = it['skill'], it['respawn'], it['fast']
    return bytes(data), tics


def wads(name):
    return EPISODE if 'base' in ITEMS[name] else WADSET


def lump_name(name):
    """The lump the runner plays: the file's base name (the base script's for an edited item is replaced by the item's)."""
    return name


def pause_picture():
    return W.patch(W.boxed('PAUSE', 2, 176, 0, pad=2, border=160))


def wad_bytes():
    return W.iwad([(f'E1M{m}', build()) for m, build in sorted(ROOMS.items())])


def gfx_bytes():
    return W.wad(b'PWAD', [('M_PAUSE', pause_picture())])


def files():
    """{repository path: bytes} of every generated file."""
    out = {WAD: wad_bytes(), GFX: gfx_bytes()}
    for name in ITEMS:
        assert len(name) <= 8, name                       # a lump name (W_AddFile, w_wad.c)
        out[f'{SCRIPT_DIR}/{name}.txt'] = script_text(name).encode()
        out[f'{SCRIPT_DIR}/{name}.lmp'] = lump(name)[0]
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--list', action='store_true')
    ap.add_argument('--out', type=Path, help='write under DIR instead of the repository (authoring)')
    a = ap.parse_args()
    generated = files()
    if a.list:
        for path, data in generated.items(): print(f'{path} {len(data)} bytes sha256 {hashlib.sha256(data).hexdigest()}')
        for name, it in ITEMS.items(): print(f'{name}: {lump(name)[1]} tics: {it["what"]}')
        return 0
    if a.check:
        bad = [p for p, d in generated.items() if not (ROOT / p).exists() or (ROOT / p).read_bytes() != d]
        if bad: print('differs or missing: ' + ', '.join(bad)); return 1
        print(f'{len(generated)} files match ({len(ITEMS)} items)')
        return 0
    root = a.out or ROOT
    for path, data in generated.items():
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_bytes(data)
    print(f'wrote {len(generated)} files under {root}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
