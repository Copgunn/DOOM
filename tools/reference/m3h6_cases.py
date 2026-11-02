"""Bounded source inputs at real-map scale: large synthetic rooms for the M3h6 observer.

Every monster and pickup is an ordinary map THING spawned by the genuine
P_SpawnMapThing in a polygon room built by m3h6_geometry (automatic node builder,
closed-cell BLOCKMAP); the player body spawns ONFLOORZ at (px, py). The controlled
between-tic inputs of M3h2/M3h5 keep their meaning (`hit_*`, `attack_*`, `push_*`).
The five rooms stress the bounds the policy raises (M3h6 note §7.1):

- **town**: a 6 x 4 grid of octagonal rooms (chamfered corners) joined by 64-unit
  corridors, with a raised step or a diamond pillar in every room and three window
  bands; about 580 linedefs and 74 sectors over 2,176 x 1,184 units. E1M1-sized
  admission, patrols across many sectors, a fight with the full roster and 16
  pickups, sight down a row of aligned openings.
- **gallery**: a 2,048-unit hall whose long walls carry 32 triangular teeth each
  (64-unit pitch, 24-unit base, 48 deep) with 16 pickups along the line of fire: hitscan and
  autoaim traces with dozens of intercepts, a monster firing back along the hall.
- **extent**: a 2,400 x 1,800 room whose boundary holds a 2,750-unit diagonal wall
  (norm 3,850) and which a 2,683-unit two-sided diagonal line splits in two, so
  certificate (i) sits at 99.9% and (ii) at 98.3% of 2^31: long hitscans at the
  wall, long sight across the two-sided line, a slide along the wall, a fireball
  over 2,100 units.
- **dense**: a 512 x 256 lane whose middle 240 units carry 30 teeth (8-unit pitch,
  32 deep) on both walls: 124 distinct lines in one 2 x 2 cell window; slides into
  the teeth and a shot along the band.
- **wide**: four 384-unit rooms at the corners of a 7,552-unit square joined by
  64-unit corridors (walls in 424-unit pieces): a 60 x 60 cell BLOCKMAP, pursuit
  and sight along a 6,784-unit corridor, hitscans walking empty cells.

The original has no map-size assumption; these scenarios observe what the compiled
bodies do at these sizes. No Rust output."""
IMP, POSSESSED, SHOTGUY, CHAINGUY, DEMON = 3001, 3004, 9, 65, 3002
ROSTER = (IMP, POSSESSED, SHOTGUY, CHAINGUY, DEMON)
CLIP, CLIPBOX, SHELLS, SHELLBOX, STIMPACK, MEDIKIT, SHOTGUN, CHAINGUN, GREENARMOR, BLUEARMOR, HEALTHBONUS, ARMORBONUS = 2007, 2048, 2008, 2049, 2011, 2012, 2001, 2002, 2018, 2019, 2014, 2015
PICKUPS = (CLIP, CLIPBOX, SHELLS, SHELLBOX, STIMPACK, MEDIKIT, SHOTGUN, CHAINGUN, GREENARMOR, BLUEARMOR, HEALTHBONUS, ARMORBONUS)
MAX_PICKUPS = 16

def town(columns=6, rows=4):
    """Octagonal rooms on a grid; see the module docstring. Cell (i, j) spans
    [384i, 384i + 384) x [320j, 320j + 320); the room is [32, 288] x [32, 256] with
    48-unit chamfers; corridors are 64 wide at x in [128, 192] (north) and y in
    [112, 176] (east)."""
    sectors = []
    room_index = {}
    for j in range(rows):
        for i in range(columns):
            x0, y0 = 384 * i, 320 * j
            P = lambda x, y: (x0 + x, y0 + y)
            ring = [P(80, 256)]
            if j < rows - 1: ring += [P(128, 256), P(192, 256)]
            ring += [P(240, 256), P(288, 208)]
            if i < columns - 1: ring += [P(288, 176), P(288, 112)]
            ring += [P(288, 80), P(240, 32)]
            if j > 0: ring += [P(192, 32), P(128, 32)]
            ring += [P(80, 32), P(32, 80)]
            if i > 0: ring += [P(32, 112), P(32, 176)]
            ring += [P(32, 208)]
            rings = [ring]
            if (i + j) % 2 == 0:
                rings.append([P(96, 96), P(160, 96), P(160, 160), P(96, 160)])  # step hole (ccw)
            else:
                rings.append([P(200, 200), P(224, 176), P(248, 200), P(224, 224)])  # pillar (ccw)
            room_index[(i, j)] = len(sectors)
            sectors.append((rings, 0, 128))
            if (i + j) % 2 == 0:
                sectors.append(([[P(96, 160), P(160, 160), P(160, 96), P(96, 96)]], 24, 128))
    for j in range(rows):
        for i in range(columns):
            x0, y0 = 384 * i, 320 * j
            P = lambda x, y: (x0 + x, y0 + y)
            if i < columns - 1:
                window = j > 0 and (i + j) % 3 == 0
                sectors.append(([[P(288, 176), P(416, 176), P(416, 112), P(288, 112)]], 40 if window else 0, 88 if window else 128))
            if j < rows - 1:
                sectors.append(([[P(128, 352), P(192, 352), P(192, 256), P(128, 256)]], 0, 128))
    return (sectors, {}, (), None, 1024)

def gallery(pitch=64, depth=48, teeth=32):
    top = [(-1024, 128)]
    for k in range(teeth):
        x = -1024 + pitch * k
        top += [(x + 5 * pitch // 16, 128), (x + pitch // 2, 128 - depth), (x + 11 * pitch // 16, 128)]
    top.append((1024, 128))
    bottom = [(x, -y) for x, y in reversed(top)]
    ring = top + bottom
    return ([(([ring]), 0, 128)], {}, (), None, 4096)

def extent():
    lower = [(0, 0), (0, 600), (2400, 1800), (2400, 1650), (2200, 1650)]
    upper = [(0, 600), (0, 1800), (2400, 1800)]
    return ([([lower], 0, 128), ([upper], 32, 128)], {}, (), None, 4096)

def dense(pitch=8, depth=32, teeth=30):
    half = pitch * teeth // 2
    top = [(-256, 128), (-half, 128)]
    for k in range(teeth):
        x = -half + pitch * k
        top += [(x + pitch // 2, 128 - depth), (x + pitch, 128)]
    top.append((256, 128))
    bottom = [(x, -y) for x, y in reversed(top)]
    return ([([top + bottom], 0, 128)], {}, (), None, 4096)

def wide(size=7552, room=384, width=64):
    """Four corner rooms and four edge corridors; corridor center lines at 192."""
    lo, hi = 192 - width // 2, 192 + width // 2  # 160, 224
    far_lo, far_hi = size - 192 - width // 2, size - 192 + width // 2  # 7328, 7392
    s = size
    r = room
    sw = [(0, 0), (0, lo), (0, hi), (0, r), (lo, r), (hi, r), (r, r), (r, hi), (r, lo), (r, 0)]
    se = [(s - r, 0), (s - r, lo), (s - r, hi), (s - r, r), (far_lo, r), (far_hi, r), (s, r), (s, hi), (s, lo), (s, 0)]
    nw = [(0, s - r), (0, far_lo), (0, far_hi), (0, s), (r, s), (r, far_hi), (r, far_lo), (r, s - r), (hi, s - r), (lo, s - r)]
    ne = [(s - r, s - r), (s - r, far_lo), (s - r, far_hi), (s - r, s), (s, s), (s, far_hi), (s, far_lo), (s, s - r), (far_hi, s - r), (far_lo, s - r)]
    south = [(r, hi), (s - r, hi), (s - r, lo), (r, lo)]
    north = [(r, far_hi), (s - r, far_hi), (s - r, far_lo), (r, far_lo)]
    west = [(lo, r), (lo, s - r), (hi, s - r), (hi, r)]
    east = [(far_lo, r), (far_lo, s - r), (far_hi, s - r), (far_hi, r)]
    sectors = [([ring], 0, 128) for ring in (sw, se, nw, ne, south, north, west, east)]
    return (sectors, {}, (), None, 424)

ROOMS = [town(), gallery(), extent(), dense(), wide()]
ROOM_NAMES = ['town', 'gallery', 'extent', 'dense', 'wide']

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
    north = dict(turn=16384, turn_hold=1); south = dict(turn=-16384, turn_hold=1); west = dict(turn=16384, turn_hold=2)
    southeast = dict(turn=-8192, turn_hold=1)
    pistol = dict(attack_start=10, attack_end=200)
    chaingun = dict(ready=3, owned=11, clips=200)
    shotgun = dict(ready=2, owned=7, shells=20)
    # --- town -------------------------------------------------------------------
    cell = lambda i, j, x, y: (384 * i + x, 320 * j + y)
    town_pickups = [item(kind, *cell(i, j, 140, 200)) for (i, j), kind in zip(
        [(1, 0), (2, 0), (3, 0), (4, 0), (0, 1), (1, 1), (2, 1), (3, 1), (4, 1), (5, 1), (0, 2), (1, 2), (2, 2), (3, 2), (4, 2), (5, 2)],
        [CLIP, SHELLS, STIMPACK, HEALTHBONUS, ARMORBONUS, CLIPBOX, SHELLBOX, MEDIKIT, SHOTGUN, CHAINGUN, GREENARMOR, BLUEARMOR, HEALTHBONUS, ARMORBONUS, CLIP, SHELLS])]
    town_roster = [monster(POSSESSED, *cell(2, 0, 220, 80), -180), monster(SHOTGUY, *cell(3, 1, 220, 80), -180), monster(IMP, *cell(1, 1, 220, 80), -90),
                   monster(CHAINGUY, *cell(4, 0, 220, 80), -180), monster(DEMON, *cell(2, 1, 220, 80), -90), monster(IMP, *cell(0, 2, 220, 80), -90),
                   monster(POSSESSED, *cell(5, 2, 220, 80), -180), monster(DEMON, *cell(3, 2, 220, 80), -180)]
    # A walk east along the bottom row through five corridors, past a step and pillars.
    add('town_patrol', 0, [monster(DEMON, *cell(5, 0, 220, 80), -180)], px=64, py=144, forward=50, move_hold=300, tics=360)
    # The full roster and sixteen pickups: a chaingun fight spreading across the grid.
    add('town_fight', 0, town_roster + town_pickups, px=64, py=144, forward=50, move_start=20, move_hold=120, attack_start=1, attack_end=400, tics=420, **chaingun)
    # A zombie five rooms away sees the player down the aligned bottom-row openings.
    add('town_sight_far', 0, [monster(POSSESSED, *cell(5, 0, 160, 144), -180)], px=64, py=144, tics=200, **pistol)
    # Running into the north-east chamfer of the first room, then along it.
    add('town_slide_chamfer', 0, [], px=200, py=200, forward=50, move_hold=120, tics=150, **dict(turn=8192, turn_hold=1))
    # A shot through a window band (floor 40, ceiling 88) at a zombie beyond it.
    add('town_window_shot', 0, [monster(POSSESSED, *cell(3, 1, 60, 144), -180)], px=cell(2, 1, 200, 144)[0], py=cell(2, 1, 200, 144)[1], tics=120, **pistol)
    # A fireball across a step and a pillar room.
    add('town_fireball', 0, [monster(IMP, *cell(1, 0, 160, 144), -180)], px=64, py=144, attack_at=1, tics=200)
    # --- gallery ----------------------------------------------------------------
    gallery_pickups = [item(kind, -1024 + 64 * k, 80) for k, kind in zip(range(2, 18), [CLIP, SHELLS, STIMPACK, HEALTHBONUS, ARMORBONUS, CLIPBOX, SHELLBOX, MEDIKIT,
                                                                                         SHOTGUN, CHAINGUN, GREENARMOR, BLUEARMOR, HEALTHBONUS, ARMORBONUS, CLIP, SHELLS])]
    # Pistol shots east along the toothed wall: autoaim and bullet traces with dozens of intercepts.
    add('gallery_shot', 1, gallery_pickups, px=-960, py=84, tics=90, attack_start=10, attack_end=60)
    # Autoaim over sixteen pickups with no target at all.
    add('gallery_aim_pickups', 1, gallery_pickups, px=-960, py=84, tics=60, attack_start=10, attack_end=40)
    # A zombie at the far end fires back along the whole hall (MISSILERANGE aim over every tooth and pickup).
    add('gallery_monster_shot', 1, gallery_pickups + [monster(POSSESSED, 960, 84, -180)], px=-960, py=84, tics=240)
    # A shotgun volley: seven traces with spread after one autoaim.
    add('gallery_shotgun', 1, gallery_pickups + [monster(SHOTGUY, 576, 84, -180)], px=-960, py=84, tics=120, attack_start=10, attack_end=20, **shotgun)
    # --- extent -----------------------------------------------------------------
    # Hitscans at the 2,750-unit diagonal wall from a distance.
    add('extent_shot_long_wall', 2, [], px=200, py=400, tics=90, attack_start=10, attack_end=60, **dict(turn=-8192, turn_hold=1))
    # A zombie 2,400 units away across the long two-sided line: sight and a return shot.
    add('extent_sight_diagonal', 2, [monster(POSSESSED, 2200, 1750, -150)], px=200, py=400, tics=200, **pistol)
    # Sliding along the long wall.
    add('extent_slide_long_wall', 2, [], px=400, py=340, forward=50, move_hold=150, tics=180, **southeast)
    # A fireball launched from 2,300 units away.
    add('extent_fireball_far', 2, [monster(IMP, 2000, 1560, -150)], px=200, py=400, attack_at=1, tics=300)
    # --- dense ------------------------------------------------------------------
    # Pressing into the teeth while running: slides whose traces list the comb.
    add('dense_slide', 3, [], px=-200, py=60, forward=50, side=-20, move_hold=100, tics=120)
    add('dense_slide_steep', 3, [], px=-200, py=60, forward=50, side=-50, move_hold=100, tics=120)
    # A shot along the upper band: every tooth within range straddles the trace.
    add('dense_shot', 3, [], px=-250, py=100, tics=60, attack_start=10, attack_end=40, **dict(turn=512, turn_hold=1))
    # --- wide -------------------------------------------------------------------
    # A Demon 2,600 units down the south corridor pursues into the south-west room.
    add('wide_pursuit', 4, [monster(DEMON, 3000, 192, -180)], px=300, py=192, tics=400)
    # A zombie 6,700 units away sees down the corridor and fires at MISSILERANGE.
    add('wide_sight', 4, [monster(POSSESSED, 7000, 192, -180)], px=300, py=192, tics=300)
    # Pistol shots east along the empty corridor: the DDA walks seventeen cells.
    add('wide_shot_corridor', 4, [], px=300, py=192, tics=60, attack_start=10, attack_end=40)
    return layouts
