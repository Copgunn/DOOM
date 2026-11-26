#!/usr/bin/env python3
"""Synthetic map slots and demo scripts of the wire 28 genuine-engine items (M6a note section 1.12, ruling U12 (iii);
section 1.17 option 1 and its "Planned evidence").

Everything here is drawn by this tool: nothing comes from any IWAD. One IWAD, tests/reference/wire28_wads/wire28.wad,
holds nine map slots built with the M6a room builder (m6a_wad.Room and m6a_wad.iwad, imported read-only: the M5a
texture and flat directory and framing); each item runs one slot from a new game with the M6a WAD set after it
(tests/reference/m5b_gfx.wad, m5c_sfx.wad, m6a_wads/m6a_gfx.wad and m6a_music.wad). Things are placed by doomednum
(info.c); angles in degrees.

  E1M1 AMMO    a corridor walked east over every P_GiveAmmo path of Episode 1 (p_inter.c:73-160): a clip, two boxes
               of bullets (the second capped at maxammo), shells from zero with the pistol ready (no switch), a box of
               shells, a shotgun (2 loads), a rocket launcher (2 loads); the rockets fired at the far wall, the fist
               selected, then a rocket from zero with the fist ready (the switch to the launcher, p_inter.c:140-147),
               a box of rockets, a backpack (one load of each type, maxammo doubled, p_inter.c:589-599), a box of
               bullets above the old cap
  E1M2 DROP    a 64-wide corridor: a zombieman and a shotgun guy shot dead, their dropped clip (half a clip,
               p_inter.c:91-93) and dropped shotgun (one load, p_inter.c:270-285) walked over
  E1M3 HURT    the player on a 5-damage floor (special 7, p_spec.c:1001-1007) after green armour, two imps (fireball
               and claw), a zombieman (hitscan) and a demon (bite) attacking
  E1M4 SPLASH  two barrels: the first exploding at a Chebyshev distance of 143 from the player (128 - (143 - 16) = 1
               damage, p_map.c:1179-1194: 0 on skill 0, still a P_DamageMobj with its pain roll, p_inter.c:893-900),
               the second killed by the first and exploding 74 units from the player
  E1M5 BITE    a room of sector special 11 (p_spec.c:1054-1063) with a demon biting the player: the cap at health - 1
               (p_inter.c:837-842)
  E1M6 ESCAPE  one convex room with a 45-degree north-east corner; three imps embedded below spawn rule S''s dropped
               margin (31 - r = 11 for r = 20): 1 unit from the west wall, 10 from the south wall (the boundary
               value) and about 9.9 from the diagonal wall, facing their walls; the fist (no P_NoiseAlert,
               p_pspr.c:629-660) breaks a barrel whose blast kills two more, which explode in one tic beside the
               west imp: a non-lethal blast and then a lethal one before its thinker moves it (p_inter.c:806-832),
               so the corpse's westward momentum is one P_TryMove (p_mobj.c:150-164) that clears the wall
  E1M7 SMALL   the same room with the three imps awake (the pistol's noise): stuck in place while chasing; the south
               imp shot by the pistol and the shotgun (no escape)
  E1M8 EMBBAR  a barrel 10 units from the west wall (below its margin of 21): refused at the load (Rust only)
  E1M9 EMBPLY  the player start 10 units from the west wall (below its margin of 15): refused at the load (Rust only)

ITEMS: name, map slot, skill (the demo header's byte 1, patched after the frozen format as m6a_wad.SKILL_OVERRIDE
does: tools/reference/m4b_scripts.py admits 2 and 3 only), script lines (the m4b_scripts format) and what it shows.

    wire28_wad.py            write the WAD and the scripts
    wire28_wad.py --check    regenerate in memory and compare with the committed files (writes nothing)
    wire28_wad.py --list     files, digests, items
"""
import argparse, hashlib, sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/reference'))
import m4b_scripts as S          # noqa: E402  the script format (read-only)
import m6a_wad as W              # noqa: E402  the room builder and WAD writer (read-only)

WAD = 'tests/reference/wire28_wads/wire28.wad'
SCRIPT_DIR = 'tests/reference/wire28_scripts'
WADSET = [WAD, 'tests/reference/m5b_gfx.wad', 'tests/reference/m5c_sfx.wad', 'tests/reference/m6a_wads/m6a_gfx.wad',
          'tests/reference/m6a_wads/m6a_music.wad']
E, N, WE, SO, NE = 0, 90, 180, 270, 45
PLAYER, ZOMBIE, SHOTGUY, IMP, DEMON, BARREL = 1, 3004, 9, 3001, 3002, 2035
CLIP, BULLETS, SHELLS, SHELLBOX, ROCKET, ROCKETS, BACKPACK, SHOTGUN, LAUNCHER, GREEN = \
    2007, 2048, 2008, 2049, 2010, 2046, 8, 2001, 2003, 2018
HURT5, EXIT11 = 7, 11
TAP = ['cmd 1 0 0 0 ATTACK', 'idle 20']                 # one accurate pistol shot (refire 0, p_pspr.c:700-712)
PUNCH = ['cmd 1 0 0 0 ATTACK', 'idle 24']               # one punch, released before A_ReFire


def corridor(name, x0, x1, half):
    p = W.Room(name, wall='M5WALLA')
    p.sector('HALL', [(x0, -half, x1, half)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    return p


def ammo():
    p = corridor('ammo', -64, 1600, 64)
    p.thing(0, 0, E, PLAYER)
    for x, kind in ((64, CLIP), (128, BULLETS), (192, BULLETS), (256, SHELLS), (320, SHELLBOX), (384, SHOTGUN),
                    (448, LAUNCHER), (704, ROCKET), (768, ROCKETS), (832, BACKPACK), (896, BULLETS)):
        p.thing(x, 0, E, kind)
    return p


def drop():
    p = corridor('drop', -64, 1024, 32)
    p.thing(0, 0, E, PLAYER)
    p.thing(384, 0, WE, ZOMBIE)
    p.thing(640, 0, WE, SHOTGUY)
    return p


def hurt():
    p = W.Room('hurt', wall='M5WALLA')
    p.sector('ROOM', [(-256, -256, 256, 256)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    p.sector('HURT', [(-32, -32, 64, 32)], light=192, special=HURT5, fpic='NUKAGE1', cpic='M5FLAT2', layer=1)
    p.thing(0, 0, E, PLAYER)
    p.thing(32, 0, E, GREEN)
    for x, y, a, kind in ((224, 192, WE, IMP), (128, -96, N, IMP), (-224, 192, E, ZOMBIE), (-208, -208, NE, DEMON)):
        p.thing(x, y, a, kind)
    return p


def splash():
    p = W.Room('splash', wall='M5WALLA')
    p.sector('ROOM', [(-128, -128, 384, 192)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')
    p.thing(0, 0, E, PLAYER)
    p.thing(115, 0, E, BARREL)        # pushed 28.46 east by the shots before it explodes (genuine run)
    p.thing(90, 60, E, BARREL)
    return p


def bite():
    p = W.Room('bite', wall='M5WALLA')
    p.sector('ROOM', [(-256, -128, 256, 128)], light=192, special=EXIT11, fpic='M5FLAT1', cpic='M5FLAT2')
    p.thing(0, 0, E, PLAYER)
    p.thing(224, 0, WE, DEMON)
    return p


RING = [(-256, 512), (896, 512), (1024, 384), (1024, 0), (0, 0), (0, 448), (-256, 448)]   # clockwise, y up


def escape_room(name, things):
    p = W.Room(name, wall='M5WALLA')
    p.sector('ROOM', [(0, 0, 1024, 512)], light=192, fpic='M5FLAT1', cpic='M5FLAT2')   # placeholder: the ring below
    p.rings_override = [[RING]]
    for t in things: p.thing(*t)
    return p


EMBEDDED = [(512, 10, SO, IMP), (960, 434, NE, IMP)]   # 10 from the south wall; x + y = 1394: 14/sqrt(2) from x + y = 1408


FIRESTICK = 46                                  # MT_MISC43, MF_SOLID, radius 16 (info.c:3526-3550)


def escape():
    # THINGS order is thinker order: B2 and B1 explode in one tic, B2's thinker first, then I1's moves it
    return escape_room('escape', [(15, 470, 315, PLAYER), (100, 262, E, FIRESTICK), (123, 363, E, BARREL),
                                  (100, 256, E, BARREL), (21, 256, E, BARREL), (1, 256, WE, IMP)])


def small():
    return escape_room('small', [(512, 300, SO, PLAYER), (512, 360, E, SHOTGUN)] + EMBEDDED + [(1, 256, WE, IMP)])


def embbar():
    return escape_room('embbar', [(320, 256, WE, PLAYER), (5, 128, E, BARREL)])


def embply():
    return escape_room('embply', [(10, 256, E, PLAYER)])


ROOMS = {1: ammo, 2: drop, 3: hurt, 4: splash, 5: bite, 6: escape, 7: small, 8: embbar, 9: embply}
WALK = ['cmd 64 25 0 0 0', 'idle 10', 'cmd 85 0 0 0 ATTACK', 'idle 20', 'cmd 1 0 0 0 CHANGE|W0', 'idle 60',
        'cmd 60 25 0 0 0', 'idle 10']
SHOOT = ['cmd 260 0 0 0 ATTACK', 'cmd 140 25 0 0 0', 'idle 10']
STAND = ['cmd 2 25 0 0 0', 'idle 298']
BLAST = TAP * 4 + ['idle 60']
CHOMP = ['idle 300']
FIST = ['idle 5'] + ['cmd 1 0 0 0 ATTACK', 'idle 20'] * 5 + ['idle 60']
SHOTS = ['idle 35'] + TAP * 3 + ['cmd 6 -25 0 0 0', 'idle 30'] + ['cmd 1 0 0 0 ATTACK', 'idle 40'] * 3 + ['idle 20']
# name: (map slot, skill, script lines, what)
ITEMS = {
    'AMMO0': (1, 0, WALK, 'skill 0: every P_GiveAmmo path of Episode 1 doubled before the cap'),
    'AMMO2': (1, 2, WALK, 'skill 2: the same commands, nothing doubled'),
    'DROP0': (2, 0, SHOOT, 'skill 0: a dropped clip (half a clip, doubled) and a dropped shotgun (one load, doubled)'),
    'DROP2': (2, 2, SHOOT, 'skill 2: the same commands'),
    'HURT0': (3, 0, STAND, 'skill 0: fireball, claw, hitscan, bite and floor damage halved before the thrust and armour'),
    'HURT2': (3, 2, STAND, 'skill 2: the same commands'),
    'SPLASH0': (4, 0, BLAST, 'skill 0: a barrel blast of 1 halved to 0 (its pain roll kept) and a blast of 54 halved'),
    'SPLASH2': (4, 2, BLAST, 'skill 2: the same commands'),
    'BITE2': (5, 2, CHOMP, 'skill 2: a demon bite of damage >= health on sector special 11 capped at health - 1'),
    'BITE0': (5, 0, CHOMP, 'skill 0: the same room, every blow halved before the cap'),
    'ESCAPE': (6, 2, FIST, 'skill 2: idle embedded imps stuck in place; a two-barrel blast takes a corpse out of the room'),
    'SMALL': (7, 2, SHOTS, 'skill 2: embedded imps chasing, stuck in place; pistol and shotgun thrust, no escape'),
}


def wad_bytes():
    return W.iwad([(f'E1M{m}', build()) for m, build in sorted(ROOMS.items())])


def script_text(name):
    m, skill, lines, what = ITEMS[name]
    return (f'# wire 28 item {name}: {what}.\n# Generated by tools/reference/wire28_wad.py; skill byte {skill}.\n'
            + '\n'.join([f'map E1M{m}'] + lines) + '\n')


def lump(name):
    data, tics = S.lump(script_text(name))
    return data[:1] + bytes([ITEMS[name][1]]) + data[2:], tics


def files():
    """{repository path: bytes} of every generated file."""
    out = {WAD: wad_bytes()}
    for name in ITEMS:
        out[f'{SCRIPT_DIR}/{name}.txt'] = script_text(name).encode()
        out[f'{SCRIPT_DIR}/{name}.lmp'] = lump(name)[0]
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true')
    ap.add_argument('--list', action='store_true')
    a = ap.parse_args()
    generated = files()
    if a.list:
        for path, data in generated.items(): print(f'{path} {len(data)} bytes sha256 {hashlib.sha256(data).hexdigest()}')
        for name, (m, skill, _l, what) in ITEMS.items(): print(f'{name}: E1M{m}, skill {skill}, {lump(name)[1]} tics: {what}')
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
