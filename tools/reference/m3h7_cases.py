"""Bounded source inputs for sky ceilings and the static sector specials (M3h7 Stage A).

Every monster and pickup is an ordinary map THING spawned by the genuine
P_SpawnMapThing in a polygon room built by m3h7_geometry (the M3h6 automatic node
builder plus sector light, special, tag, flats and line specials); the player body
spawns ONFLOORZ at (px, py). The controlled between-tic inputs of M3h2/M3h5/M3h6 keep
their meaning (`hit_*`, `attack_*`, `push_*`). Two rooms (M3h7 note §5.1):

- **pools**: a 2,304 x 384 hall (floor 0, ceiling 128, light 160) whose north wall is
  notched by a row of twelve 128-unit basins (floor -8), one per Stage A sector
  special in this order: 7, 5, 16, 4, 9, 1, 2, 3, 8, 12, 13, 17. Their light levels
  differ so P_FindMinSurroundingLight picks different neighbors (the secret basin at
  112 is the flash basin's darkest neighbor; the slow strobe's neighbors are all at
  least its own level, so its minlight falls to 0). A dark gallery (light 0) lies 64
  units north of the basins behind their one-sided north walls and a void strip,
  reached only through a corridor at the west end, so no basin counts it. The hall's
  edge over each basin is a ledge (0 over -8). Three one-sided walls carry line
  special 48 (the fire-flicker basin's east wall and two pieces of the hall's south
  wall).
- **sky**: an outdoor court (floor 0, ceiling 256, `F_SKY1`) and a low court (ceiling
  144, `F_SKY1`) with a tower (floor 128, ceiling 256, `F_SKY1`). In the court: two
  sky-hack blocks (floor 0, ceiling 32, `F_SKY1`; one listed before the court so its
  lines' front is the block, one after so the front is the court), a sky block of
  ceiling 200 against the north wall (listed before the court, its shared west line
  created first), and a building with an indoor room (ceiling 128) behind three
  windows (ceiling 34, not sky): window A behind a 4-unit sky sill (ceiling 48) whose
  sill-to-window line is created first, window A' behind a sill whose court-to-sill
  line is created first, and window B listed before the court so its line's back is
  the court.

The sector order of each room fixes every line's front side and the BLOCKMAP list
order (lines are numbered in sector order, then ring order). No Rust output."""
IMP, POSSESSED, SHOTGUY, CHAINGUY, DEMON = 3001, 3004, 9, 65, 3002
ROSTER = (IMP, POSSESSED, SHOTGUY, CHAINGUY, DEMON)
CLIP, SHELLS, STIMPACK, GREENARMOR, HEALTHBONUS, ARMORBONUS = 2007, 2008, 2011, 2018, 2014, 2015
PICKUPS = (2007, 2048, 2008, 2049, 2011, 2012, 2001, 2002, 2018, 2019, 2014, 2015)
MAX_PICKUPS = 16
SKY = 'F_SKY1'

BASINS = [(7, 144), (5, 176), (16, 144), (4, 208), (9, 112), (1, 255), (2, 224), (3, 160), (8, 200), (12, 240), (13, 176), (17, 192)]


def pools():
    """See the module docstring. Sector 0 is the hall, 1..12 the basins (west to east),
    13 the corridor, 14 the gallery."""
    top = [(128 * i, 0) for i in range(13)]
    hall = [(-384, 0), (-320, 0)] + top + [(1920, 0), (1920, -384), (1152, -384), (384, -384), (-384, -384)]
    sectors = [([hall], 0, 128, dict(light=160))]
    for i, (special, light) in enumerate(BASINS):
        x0, x1 = 128 * i, 128 * i + 128
        floor = 'NUKAGE1' if special in (4, 5, 7, 16) else 'FLOOR7_1'
        sectors.append(([[(x0, 128), (x1, 128), (x1, 0), (x0, 0)]], -8, 128, dict(light=light, special=special, floor=floor)))
    sectors.append(([[(-384, 192), (-320, 192), (-320, 0), (-384, 0)]], 0, 128, dict(light=96)))
    sectors.append(([[(-384, 192), (-384, 256), (1536, 256), (1536, 192), (-320, 192)]], 0, 128, dict(light=0)))
    scrollers = {((1536, 128), (1536, 0)): (48, 0), ((1920, -384), (1152, -384)): (48, 0), ((384, -384), (-384, -384)): (48, 0)}
    return (sectors, {}, (), None, 4096, scrollers)


def sky():
    """See the module docstring. Sector order: 0 H3 (north-wall block), 1 H1 (hack, front
    side), 2 S' (sill of window A'), 3 S (sill of window A), 4 W_B, 5 C (court), 6 W_A,
    7 W_A', 8 R (room), 9 H2 (hack, court in front), 10 L (low court), 11 T (tower)."""
    sky = dict(ceiling=SKY, light=255)
    lit = dict(light=192)
    court = [(-1024, -256), (-1024, 1024), (700, 1024), (700, 900), (716, 900), (716, 1024), (1024, 1024), (1024, -256)]
    # Holes (counterclockwise): H1, H2, the building outline (west wall split at the windows).
    h1 = [(0, 608), (16, 608), (16, 800), (0, 800)]
    h2 = [(0, 288), (16, 288), (16, 480), (0, 480)]
    building = [(200, -160), (456, -160), (456, 224), (200, 224), (200, 192), (200, 128), (200, 64), (200, 0), (200, -64), (200, -128)]
    sectors = [
        ([[(700, 900), (700, 1024), (716, 1024), (716, 900)]], 0, 200, sky),           # 0 H3: shared west edge first
        ([[(0, 800), (16, 800), (16, 608), (0, 608)]], 0, 32, sky),                    # 1 H1
        ([[(200, -128), (200, -64), (204, -64), (204, -128)]], 0, 48, sky),            # 2 S': court line first
        ([[(204, 64), (204, 0), (200, 0), (200, 64)]], 0, 48, sky),                    # 3 S: window line first
        ([[(200, 192), (224, 192), (224, 128), (200, 128)]], 0, 34, lit),              # 4 W_B
        ([court, h1, h2, building], 0, 256, sky),                                       # 5 C
        ([[(204, 64), (224, 64), (224, 0), (204, 0)]], 0, 34, lit),                    # 6 W_A
        ([[(204, -64), (224, -64), (224, -128), (204, -128)]], 0, 34, lit),            # 7 W_A'
        ([[(224, 200), (432, 200), (432, -136), (224, -136), (224, -128), (224, -64), (224, 0), (224, 64), (224, 128), (224, 192)]], 0, 128, dict(light=128)),  # 8 R
        ([[(0, 480), (16, 480), (16, 288), (0, 288)]], 0, 32, sky),                    # 9 H2
        ([[(-1024, -1024), (-1024, -256), (1024, -256), (1024, -1024)], [(400, -700), (464, -700), (464, -636), (400, -636)]], 0, 144, sky),  # 10 L
        ([[(400, -636), (464, -636), (464, -700), (400, -700)]], 128, 256, sky),       # 11 T
    ]
    return (sectors, {}, (), None, 4096, {})


ROOMS = [pools(), sky()]
ROOM_NAMES = ['pools', 'sky']


def populate(pack):
    core = pack.base; core.CASES.clear(); layouts = []
    def monster(kind, x, y=0, angle=0, options=7): return (x, y, angle, kind, options)
    def item(kind, x, y): return (x, y, 0, kind, 7)
    def add(label, room, things=(), hit_at=0, hit_thing=0, hit_amount=0, attack_at=0, attack_thing=0, push_at=0, push_x=0, push_y=0, **kw):
        things = list(things)
        assert sum(t[3] in ROSTER for t in things) <= 8
        assert sum(t[3] in PICKUPS for t in things) <= MAX_PICKUPS
        assert all(t[3] in ROSTER or t[3] in PICKUPS for t in things)
        if things not in layouts: layouts.append(things)
        values = dict(layout=layouts.index(things), drop=0, drop_x=36, drop_y=0,
                      move_start=1, physical=0, demon_last=0, part=3, tics=256,
                      thing=0, forward=0, move_hold=0, attack_end=0, seed=6)
        values.update(kw); pack.case(label, **values)
        core.CASES[-1].update(room=room, hit_at=hit_at, hit_thing=hit_thing, hit_amount=hit_amount, attack_at=attack_at, attack_thing=attack_thing,
                              push_at=push_at, push_x=push_x, push_y=push_y)
    north = dict(turn=16384, turn_hold=1); south = dict(turn=-16384, turn_hold=1)
    pistol = dict(attack_start=10, attack_end=60)
    chaingun = dict(ready=3, owned=11, clips=200)
    # --- pools ----------------------------------------------------------------------
    # Every light kind toggling for 1,200 tics; the player idles in the hall (special 0).
    add('pools_lights_soak', 0, [], px=768, py=-200, tics=1200)
    # A slow walk east through the 5%, 10%, 20% and strobe-hurt basins, picking up green
    # armor in the 10% basin: damage on every leveltime&31 gate while on a basin floor.
    add('pools_nukage_damage', 0, [item(GREENARMOR, 216, 64)], px=40, py=64, forward=6, move_hold=230, tics=240)
    # Standing in the 20% basin with no armor until death: an environmental death.
    add('pools_nukage_death', 0, [], px=320, py=64, tics=240)
    # Pushed north from the hall until the center overhangs the 5% basin while the box
    # still rests on the ledge (z 0 over a -8 sector floor), then walking in.
    add('pools_ledge_edge', 0, [], px=64, py=-12, push_at=1, push_x=0, push_y=2, forward=10, move_start=100, move_hold=106, tics=220, **north)
    # Walking north into the secret basin and on to its north wall.
    add('pools_secret', 0, [], px=576, py=-40, forward=10, move_hold=40, tics=120, **north)
    # Standing in the strobe-hurt basin: strobe toggles and 20% damage in one sector.
    add('pools_strobe_hurt', 0, [], px=448, py=64, tics=100)
    # A fight in the hall under the toggling lights: imps' fireballs explode at the walls
    # and at the player while the light thinkers run in the same tics.
    fight = [monster(IMP, 300, -320, 90), monster(IMP, 1200, -320, 90), monster(POSSESSED, 760, -340, 90), monster(IMP, 1600, -300, 90),
             monster(SHOTGUY, 100, -300, 90), monster(IMP, 900, -360, 90)]
    add('pools_light_and_fireball', 0, fight + [item(STIMPACK, 700, -100), item(CLIP, 820, -100)], px=768, py=-60, attack_start=20, attack_end=300, tics=400, **chaingun, **south)
    # --- sky ------------------------------------------------------------------------
    # Imps west of the hack block H1 fire at a player just east of it: every fireball
    # meets the block's two-sided line under a sky ceiling and is removed.
    add('sky_hack_wall', 1, [monster(IMP, -700, 704), monster(IMP, -640, 744)], px=80, py=704, tics=400)
    # A fireball at a plain one-sided court wall under the sky explodes.
    add('sky_one_sided_wall', 1, [monster(IMP, -400, 300, 90)], px=-400, py=960, attack_at=2, attack_thing=0, forward=50, move_start=28, move_hold=36, tics=200)
    # A fireball refused by the north wall right after the block H3's lowering line was
    # visited in the same check: removed although the blocker is a one-sided wall.
    add('sky_wall_after_hack', 1, [monster(IMP, 697, 300, 90)], px=697, py=860, attack_at=2, attack_thing=0, forward=50, move_start=28, move_hold=36, tics=200)
    # An imp in the low court fires up at a player on the tower: the rising fireball
    # meets the sky ceiling (P_ZMovement has no sky test) and explodes.
    add('sky_ceiling_fireball', 1, [monster(IMP, -500, -668)], px=432, py=-668, tics=400)
    # Pistol shots from the tower over the low court: z above the front sector's sky
    # ceiling ends the trace with no puff.
    add('sky_shot_high', 1, [], px=432, py=-668, tics=80, **pistol)
    # Pistol shots at the hack block H2 (court in front): a sky hack wall, no puff.
    add('sky_shot_hack', 1, [], px=-64, py=384, tics=80, **pistol)
    # Pistol shots at window A's sill line (front sky, z below it, back not sky): a puff.
    add('sky_shot_window', 1, [], px=-200, py=32, tics=80, **pistol)
    # Window A': the court-to-sill line (48) then the sill-to-window line (34) both lower
    # the ceiling in one refused check; the latest (window, not sky) decides: explode.
    add('sky_window_order', 1, [monster(IMP, -596, -96)], px=264, py=-96, attack_at=1, attack_thing=0, tics=400)
    # Window A: the sill-to-window line (34) is visited first, the later court-to-sill
    # line (48) does not lower: still the window line, explode.
    add('sky_window_strict', 1, [monster(IMP, -596, 32)], px=264, py=32, attack_at=1, attack_thing=0, tics=400)
    # Window B, listed before the court: the refused line's back is the court (sky), so
    # the fireball is removed although it hit a window lintel that is not sky.
    add('sky_window_back', 1, [monster(IMP, -596, 160)], px=264, py=160, attack_at=1, attack_thing=0, tics=400)
    return layouts
