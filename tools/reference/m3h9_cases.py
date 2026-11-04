"""Bounded source inputs for two-sided scrollers, static decorations and the spawn
filter (M3h9a).

Every monster, pickup and decoration is an ordinary map THING spawned by the genuine
P_SpawnMapThing in a polygon room built by m3h9_geometry (the M3h8 builder); the player
body spawns ONFLOORZ at (px, py) facing east. Every controlled between-tic input of
M3h2..M3h8 keeps its meaning (`hit_*`, `attack_*`, `push_*`, `chase_*`, the `BT_USE`
windows, `turn2`, `forward2/3`; `script` 30 gives the player 200 health). M3h9 adds one
per-case field, `skill` (the harness `gameskill`; 2 = medium for every scenario the Rust
side replays, 1 and 3 for the two C-only census rows). Map THING angles are written -180
and -90 rather than 180 and 270 (M3h8 convention). Rooms (M3h9 note §7):

- **scrollhall**: a hall A (x in [-512, 0], floor 0) whose east edge carries two
  non-blocking two-sided type-48 lines (`ML_TWOSIDED` only): the piece y in [-256, 128]
  shared with the strip B (x in [0, 48], floor 0) and the piece y in [128, 256] shared
  with the ledge L (x in [0, 48], floor 32, the monster alcove). East of B a closed DR
  door D (x in [48, 112], y in [-64, 64], line 1 on both faces) with a closet K behind.
  A player at x = -8 is within use range (64) of the door line through the 48 line.
- **scrollblock**: two halls A and B separated by a two-sided type-48 line carrying
  `ML_BLOCKING`.
- **dense8**: the M3h8 `dense` lane with eight two-sided type-48 lines four units
  apart (x = 240, 244, ..., 268) in one 2 x 2 BLOCKMAP window and no lift; `DENSE9` is
  the nine-line lane written as a Rust-only fixture, never run in C.
- **gallery**: a flat hall (x in [-512, 512], y in [-384, 256], floor 0, ceiling 128)
  with a ledge E along the north (y in [256, 384], floor 32), two 64 x 64 pedestals P1
  at (0, 0) and P2 at (0, -192) (floor 32) and a 48 x 48 alcove in the west wall (x in
  [-560, -512], y in [-88, -40]) that pins a zombieman. The alcove is four units wider
  on each side than the zombieman's box at (-536, -64), so that the box meets no wall
  (the closed-box spawn rule the Rust policies share, M3h5 note §4.1 item 4). Ten decorations stand in every
  gallery scenario: lamps at (0, 128) and, just south of the ledge, (-256, 226);
  pillars at (-160, -64) and at the alcove mouth (-480, -64); the candelabra on P1; a pool of blood on P2; and a row of flags-0 gore at
  y = -300 (24, 15, 10, 12). Monsters and the player are placed per scenario.
- **liftroom**: the M3h8 `lift` room (ceiling 192) plus a DR door D (x in [512, 576], y
  in [32, 96], floor 2, ceiling 124 = its top: the closet K behind it has ceiling 128)
  in the lower hall's east wall. A lamp and a dead player stand on the lift; a pillar
  stands inside the door sector.
- **doorstep**: a hall H (x in [0, 512], y in [0, 128], floor 0, ceiling 128), a DR
  door D (x in [512, 576], y in [32, 96], floor 0, ceiling 124: open at the top a DR
  use computes) and east of it a step S (x in [576, 704], y in [0, 128], floor 8, higher
  than the door floor, ceiling 128). A solid pillar (radius 16) stands at (568, 64), its
  box straddling the D|S line, so P_CheckPosition over its box reads the step's floor:
  when the door closes onto it the pillar's floorz (8) ends above its ceilingz.
- **census**: a flat room with one sector of special 1 (light flash) and forty THINGs:
  one of every admitted kind with every skill bit (options 7) plus duplicates carrying
  easy-only (1), medium-only (2), hard-only (4), bit 16, `MTF_AMBUSH` (15 and 10) and
  the two-bit combinations 3, 5 and 6. The same layout is run at skill 2 (replayed by
  Rust) and at skills 1 and 3 (C-only rows).

The sector order of each room fixes every line's front side and the BLOCKMAP list
order (lines are numbered in sector order, then ring order). No Rust output."""
from m3h8_cases import IMP, POSSESSED, ROSTER, CLIP, CLIPBOX, SHELLS, SHELLBOX, STIMPACK, MEDIKIT, SHOTGUN, CHAINGUN, GREENARMOR, BLUEARMOR, HEALTHBONUS, ARMORBONUS, PICKUPS, MAX_PICKUPS, DR, WR_LIFT  # noqa: F401
from m3h9_geometry import DECORATIONS, ML_BLOCKING, SCROLL
LAMP, PILLAR, CANDELABRA, GIBS, DEADPLAYER, MESS10, MESS12 = 2028, 48, 35, 24, 15, 10, 12
assert set(DECORATIONS) == {LAMP, PILLAR, CANDELABRA, GIBS, DEADPLAYER, MESS10, MESS12}
MAX_MONSTERS, MAX_DECORATIONS = 8, 32
MTF_EASY, MTF_NORMAL, MTF_HARD, MTF_AMBUSH, MTF_NOTSINGLE = 1, 2, 4, 8, 16
MEDIUM, EASY, HARD = 2, 1, 3


def scrollhall():
    """Sector order: 0 A (hall), 1 L (ledge, floor 32), 2 B (strip), 3 K (closet),
    4 D (door, closed). The 48 lines are A|B (y in [-256, 128]) and A|L (y in
    [128, 256]); the DR lines face B and K."""
    a = [(-512, 256), (0, 256), (0, 128), (0, -256), (-512, -256)]
    ledge = [(0, 256), (48, 256), (48, 128), (0, 128)]
    b = [(0, 128), (48, 128), (48, 64), (48, -64), (48, -256), (0, -256)]
    closet = [(112, 128), (240, 128), (240, -128), (112, -128), (112, -64), (112, 64)]
    door = [(48, 64), (112, 64), (112, -64), (48, -64)]
    sectors = [([a], 0, 128), ([ledge], 32, 128), ([b], 0, 128), ([closet], 0, 128), ([door], 0, 0)]
    lines = {((0, 128), (0, -256)): (SCROLL, 0), ((0, 256), (0, 128)): (SCROLL, 0),
             ((48, 64), (48, -64)): (DR, 0), ((112, -64), (112, 64)): (DR, 0)}
    return (sectors, {}, (), None, 4096, lines)


def scrollblock():
    """Sector order: 0 A (x in [-512, 0]), 1 B (x in [0, 256]); the shared edge at
    x = 0 is a two-sided 48 line with ML_BLOCKING."""
    a = [(-512, 256), (0, 256), (0, -256), (-512, -256)]
    b = [(0, 256), (256, 256), (256, -256), (0, -256)]
    return ([([a], 0, 128), ([b], 0, 128)], {}, (), None, 4096, {((0, 256), (0, -256)): (SCROLL, 0, ML_BLOCKING)})


def dense(count=8):
    """A lane y in [0, 128] (floor 0, ceiling 128): 0 Lw x in [0, 240], then count-1
    strips four units wide, then Le up to x = 512, separated by `count` two-sided 48
    lines at x = 240, 244, ... (BLOCKMAP columns 1 and 2 of row 0: one 2 x 2 window).
    `count` 9 is the Rust-only fixture of the SpecialCross witness (never run in C)."""
    xs = [240 + 4 * i for i in range(count)]
    bounds = [0] + xs + [512]
    sectors = [([[(x0, 128), (x1, 128), (x1, 0), (x0, 0)]], 0, 128) for x0, x1 in zip(bounds, bounds[1:])]
    return (sectors, {}, (), None, 4096, {((x, 128), (x, 0)): (SCROLL, 0) for x in xs})


def gallery():
    """Sector order: 0 G (the hall, with the two pedestal holes and a 48 x 48 alcove
    notched into its west wall at y in [-88, -40]), 1 E (north ledge, floor 32), 2 P1
    (pedestal at the origin, floor 32), 3 P2 (pedestal at (0, -192), floor 32)."""
    outer = [(-512, 256), (512, 256), (512, -384), (-512, -384), (-512, -88), (-560, -88), (-560, -40), (-512, -40)]
    hole1 = [(-32, -32), (32, -32), (32, 32), (-32, 32)]
    hole2 = [(-32, -224), (32, -224), (32, -160), (-32, -160)]
    ledge = [(-512, 384), (512, 384), (512, 256), (-512, 256)]
    p1 = [(-32, 32), (32, 32), (32, -32), (-32, -32)]
    p2 = [(-32, -160), (32, -160), (32, -224), (-32, -224)]
    sectors = [([outer, hole1, hole2], 0, 128), ([ledge], 32, 128), ([p1], 32, 128), ([p2], 32, 128)]
    return (sectors, {}, (), None, 4096, {})


def liftroom():
    """Sector order: 0 L0a (x in [0, 512], y in [0, 128]), 1 L0b (y in [128, 256]), both
    floor 2, ceiling 192; 2 U (upper ledge y in [384, 640], floor 64); 3 P (the lift x
    in [192, 320], y in [256, 384], floor 64, tag 1); 4 K (closet x in [576, 704], y in
    [0, 128], floor 2, ceiling 128); 5 D (door x in [512, 576], y in [32, 96], floor 2,
    ceiling 124: already at the top a DR use computes, so a use waits and a second use
    closes it). WR 88 (tag 1) between L0a and L0b and between U and P as in M3h8."""
    l0a = [(0, 128), (512, 128), (512, 96), (512, 32), (512, 0), (0, 0)]
    l0b = [(0, 256), (192, 256), (320, 256), (512, 256), (512, 128), (0, 128)]
    upper = [(0, 640), (512, 640), (512, 384), (320, 384), (192, 384), (0, 384)]
    lift = [(192, 384), (320, 384), (320, 256), (192, 256)]
    closet = [(576, 128), (704, 128), (704, 0), (576, 0), (576, 32), (576, 96)]
    door = [(512, 96), (576, 96), (576, 32), (512, 32)]
    sectors = [([l0a], 2, 192), ([l0b], 2, 192), ([upper], 64, 192), ([lift], 64, 192, dict(tag=1)), ([closet], 2, 128), ([door], 2, 124)]
    lines = {((0, 128), (512, 128)): (WR_LIFT, 1), ((320, 384), (192, 384)): (WR_LIFT, 1),
             ((512, 96), (512, 32)): (DR, 0), ((576, 32), (576, 96)): (DR, 0)}
    return (sectors, {}, (), None, 4096, lines)


def doorstep():
    """Sector order: 0 H (hall), 1 D (door x in [512, 576], y in [32, 96], floor 0,
    ceiling 124), 2 S (step x in [576, 704], y in [0, 128], floor 8). The DR line faces
    H only; the D|S line carries no special."""
    hall = [(0, 128), (512, 128), (512, 96), (512, 32), (512, 0), (0, 0)]
    door = [(512, 96), (576, 96), (576, 32), (512, 32)]
    step = [(576, 128), (704, 128), (704, 0), (576, 0), (576, 32), (576, 96)]
    sectors = [([hall], 0, 128), ([door], 0, 124), ([step], 8, 128)]
    return (sectors, {}, (), None, 4096, {((512, 96), (512, 32)): (DR, 0)})


def census():
    """Sector order: 0 M (x in [-512, 512], y in [-384, 384] minus the corner), 1 F (x in
    [384, 512], y in [256, 384], special 1: P_SpawnLightFlash draws once at spawn)."""
    main = [(-512, 384), (384, 384), (384, 256), (512, 256), (512, -384), (-512, -384)]
    flash = [(384, 384), (512, 384), (512, 256), (384, 256)]
    return ([([main], 0, 128), ([flash], 0, 128, dict(special=1))], {}, (), None, 4096, {})


ROOMS = [scrollhall(), scrollblock(), dense(), gallery(), liftroom(), census(), doorstep()]
ROOM_NAMES = ['scrollhall', 'scrollblock', 'dense8', 'gallery', 'liftroom', 'census', 'doorstep']
DENSE9 = dense(9)
SPAWN_TICS_DRAW = {POSSESSED, IMP, GREENARMOR, BLUEARMOR, HEALTHBONUS, ARMORBONUS}  # spawn states with tics > 0 (M3h9 note §2.1)


def spawns(thing, skill):
    """The P_SpawnMapThing filter of a single-player game (p_mobj.c:740-750) for a
    THING record and a harness skill (1 easy, 2 medium, 3 hard)."""
    options = thing[4]
    if options & MTF_NOTSINGLE:
        return False
    return bool(options & (1 << (skill - 1)))


def census_things():
    kinds = [CLIP, CLIPBOX, SHELLS, SHELLBOX, STIMPACK, MEDIKIT, SHOTGUN, CHAINGUN, GREENARMOR, BLUEARMOR, HEALTHBONUS, ARMORBONUS,
             POSSESSED, IMP, LAMP, PILLAR, CANDELABRA, GIBS, DEADPLAYER, MESS10, MESS12]
    duplicates = [(CLIP, MTF_EASY), (POSSESSED, MTF_EASY), (LAMP, MTF_EASY),
                  (SHELLS, MTF_NORMAL), (IMP, MTF_NORMAL), (PILLAR, MTF_NORMAL),
                  (MEDIKIT, MTF_HARD), (POSSESSED, MTF_HARD), (CANDELABRA, MTF_HARD),
                  (STIMPACK, 7 | MTF_NOTSINGLE), (IMP, 7 | MTF_NOTSINGLE), (DEADPLAYER, 7 | MTF_NOTSINGLE),
                  (POSSESSED, 7 | MTF_AMBUSH), (LAMP, 7 | MTF_AMBUSH), (IMP, 7 | MTF_AMBUSH), (GIBS, MTF_NORMAL | MTF_AMBUSH),
                  (HEALTHBONUS, MTF_EASY | MTF_NORMAL), (ARMORBONUS, MTF_NORMAL | MTF_HARD), (SHOTGUN, MTF_EASY | MTF_HARD)]
    records = []
    for i, kind in enumerate(kinds):
        records.append((kind, 7))
        if i < len(duplicates):
            records.append(duplicates[i])
    assert len(records) == len(kinds) + len(duplicates) == 40
    things = []
    for i, (kind, options) in enumerate(records):
        x, y = -384 + 96 * (i % 9), -256 + 96 * (i // 9)
        angle = -180 if kind in ROSTER else 0
        things.append((x, y, angle, kind, options))
    return things


def populate(pack):
    core = pack.base; core.CASES.clear(); layouts = []
    def monster(kind, x, y=0, angle=0, options=7): return (x, y, angle, kind, options)
    def deco(kind, x, y, angle=0, options=7): return (x, y, angle, kind, options)
    def add(label, room, things=(), skill=MEDIUM, hit_at=0, hit_thing=0, hit_amount=0, attack_at=0, attack_thing=0, push_at=0, push_x=0, push_y=0,
            use_start=0, use_end=0, use2_start=0, use2_end=0, use3_start=0, use3_end=0, turn2_at=0, turn2=0,
            forward2=0, move2_start=0, move2_hold=0, forward3=0, move3_start=0, move3_hold=0, chase_at=0, chase_thing=0, **kw):
        things = list(things)
        assert all(t[3] in ROSTER or t[3] in PICKUPS or t[3] in DECORATIONS for t in things)
        spawned = [t for t in things if spawns(t, skill)]
        assert sum(t[3] in ROSTER for t in spawned) <= MAX_MONSTERS
        assert sum(t[3] in PICKUPS for t in spawned) <= MAX_PICKUPS
        assert sum(t[3] in DECORATIONS for t in spawned) <= MAX_DECORATIONS
        assert all(abs(t[0]) <= 900 and abs(t[1]) <= 900 for t in things)
        if things not in layouts: layouts.append(things)
        values = dict(layout=layouts.index(things), drop=0, drop_x=36, drop_y=0,
                      move_start=1, physical=0, demon_last=0, part=3, tics=256,
                      thing=0, forward=0, move_hold=0, attack_end=0, seed=6)
        values.update(kw); pack.case(label, **values)
        core.CASES[-1].update(room=room, hit_at=hit_at, hit_thing=hit_thing, hit_amount=hit_amount, attack_at=attack_at, attack_thing=attack_thing,
                              push_at=push_at, push_x=push_x, push_y=push_y,
                              use_start=use_start, use_end=use_end, use2_start=use2_start, use2_end=use2_end, use3_start=use3_start, use3_end=use3_end,
                              turn2_at=turn2_at, turn2=turn2, forward2=forward2, move2_start=move2_start, move2_hold=move2_hold,
                              forward3=forward3, move3_start=move3_start, move3_hold=move3_hold, chase_at=chase_at, chase_thing=chase_thing, skill=skill)
    north = dict(turn=16384, turn_hold=1); south = dict(turn=-16384, turn_hold=1); west = dict(turn=-32768, turn_hold=1)
    tough = dict(script=30)
    # --- scrollhall -------------------------------------------------------------------
    # The player walks east across the non-blocking 48 line: it enters spechit, the
    # crossing loop calls P_CrossSpecialLine (no case 48), nothing else happens and
    # nothing draws.
    add('scroll_cross_noop', 0, [], px=-40, py=0, forward=25, move_hold=6, tics=40)
    # A zombieman on the ledge L (floor 32) sees the player below and tries to walk west
    # or southwest: its box crosses the A|L 48 line into A (floor 0), the drop-off test
    # refuses the move with the 48 line in spechit, movedir is reset to DI_NODIR,
    # P_UseSpecialLine returns false for 48 and the following P_NewChaseDir draws.
    add('scroll_spechit_nodir', 0, [monster(POSSESSED, 24, 200, -180)], px=-200, py=72, tics=60, **tough)
    # BT_USE from x = -8 toward the door line at x = 48 (within USERANGE): the use trace
    # stops at the 48 line (P_UseSpecialLine false), the door stays shut, no sfx_noway.
    add('use_scroll_wall_stops_trace', 0, [], px=-8, py=0, use_start=3, use_end=4, tics=10)
    # One pistol shot east (the pistol is raised by tic 17): the hitscan crosses the 48
    # line (P_ShootSpecialLine has no case 48) and puffs on the closed door line beyond.
    add('shoot_through_scroller', 0, [], px=-64, py=0, attack_start=20, attack_end=20, tics=40)
    # An imp in the strip B told to attack the player in A: its fireball crosses the 48
    # line (P_CrossSpecialLine returns at once for MT_TROOPSHOT) and flies on.
    add('fireball_through_scroller', 0, [monster(IMP, 24, -160, -180)], px=-300, py=-160, attack_at=1, attack_thing=0, tics=45, **tough)
    # --- scrollblock ------------------------------------------------------------------
    # The 48 line with ML_BLOCKING: the player walking west and a chasing zombieman
    # walking east are both refused by PIT_CheckLine before the special test (a
    # missile skips it: fireball_through_blocking_scroller below); spechit
    # stays empty (NS rows) and no P_Move failure branch runs. (A monster's first
    # P_NewChaseDir never picks the opposite of its spawn movedir 0 = DI_EAST, so the
    # chaser is placed west of the line.)
    add('blocking_scroller_wall', 1, [monster(POSSESSED, -100, -64, 0)], px=48, py=64, forward=25, move_hold=20, chase_at=1, chase_thing=0, tics=60, **dict(tough, **west))
    # --- dense8 -----------------------------------------------------------------------
    # Eight 48 lines in one window: a position check whose box covers all eight fills
    # spechit to its source size and no more; each crossing is a no-op.
    add('dense_spechit_8', 2, [], px=200, py=64, forward=25, move_hold=12, tics=60)
    # --- gallery ----------------------------------------------------------------------
    decorations = [deco(LAMP, 0, 128), deco(LAMP, -256, 226), deco(PILLAR, -160, -64), deco(PILLAR, -480, -64), deco(CANDELABRA, 0, 0), deco(GIBS, 0, -192),
                   deco(GIBS, -120, -300), deco(DEADPLAYER, -40, -300), deco(MESS10, 40, -300), deco(MESS12, 120, -300)]
    # The player walks into the lamp at (0, 128): PIT_CheckThing's last line blocks a
    # solid non-shootable thing.
    add('walk_into_lamp', 3, decorations, px=-128, py=128, forward=25, move_hold=30, tics=40)
    # The player on the ledge (z 32) walks south into the lamp at (-256, 226) whose top is
    # at 16: blocked all the same (no height test for walkers).
    add('walk_into_lamp_from_ledge', 3, decorations, px=-256, py=300, forward=25, move_hold=30, tics=40, **south)
    # The player walks east through the row of flags-0 gore: every one is passed by the
    # first flag test of PIT_CheckThing.
    add('walk_over_gore', 3, decorations, px=-220, py=-300, forward=25, move_hold=50, tics=60)
    # A zombieman given the player as target between tics walks east into the pillar
    # (east, so that its first P_NewChaseDir may take the direct route: the opposite of
    # the spawn movedir DI_EAST is never chosen): P_Move fails on the thing,
    # P_NewChaseDir draws and it goes around.
    add('monster_path_around_pillar', 3, decorations + [monster(POSSESSED, -240, -64, 0)], px=200, py=-64, chase_at=1, chase_thing=len(decorations), tics=120, **tough)
    # A zombieman in the west alcove (walls on three sides, the second pillar at its
    # mouth) is pinned after one step and shoots at the player through the pillar:
    # PTR_ShootTraverse skips the non-shootable thing and the bullet goes on.
    add('monster_hitscan_through_pillar', 3, decorations + [monster(POSSESSED, -536, -64, 0)], px=200, py=-64, chase_at=1, chase_thing=len(decorations), tics=160, **tough)
    # An imp's fireball meets the candelabra on the pedestal (z 32..48 against the
    # fireball's 32..40): the missile branch returns !(flags & MF_SOLID) and it explodes.
    add('fireball_hits_candelabra', 3, decorations + [monster(IMP, 256, 0, -180)], px=-256, py=0, attack_at=1, attack_thing=len(decorations), tics=50, **tough)
    # The same shot over the pool of blood on the other pedestal passes through it.
    add('fireball_over_corpse', 3, decorations + [monster(IMP, 256, -192, -180)], px=-256, py=-192, attack_at=1, attack_thing=len(decorations), tics=50, **tough)
    # One pistol shot east through the lamp at (0, 128): aim and shoot traces skip the
    # non-shootable thing; the puff spawns at the east wall.
    add('pistol_through_lamp', 3, decorations, px=-256, py=128, attack_start=20, attack_end=20, tics=40)
    # --- liftroom ---------------------------------------------------------------------
    # The player crosses the WR 88 line and stays below: the lamp and the dead player on
    # the lift ride it down and up through P_ChangeSector/P_ThingHeightClip.
    add('decorations_ride_lift', 4, [deco(LAMP, 230, 300), deco(DEADPLAYER, 290, 340)], px=256, py=64, forward=25, move_hold=8, tics=190, **north)
    # The player uses the open door (it waits) and uses it again (it closes at once)
    # onto the 16-high pillar: PIT_ChangeSector returns at the MF_SHOOTABLE test, no
    # nofit, the door closes fully.
    add('door_closes_on_pillar', 4, [deco(PILLAR, 544, 64)], px=470, py=64, use_start=3, use_end=3, use2_start=6, use2_end=6, tics=80)
    # --- census -----------------------------------------------------------------------
    # Forty records of every admitted kind with every skill-bit combination, bit 16 and
    # ambush; medium is replayed by Rust, easy and hard are C-only rows (the trailing
    # cases, after the two review scenarios below). seed 0 as after M_ClearRandom.
    add('spawn_filter_medium', 5, census_things(), skill=MEDIUM, px=-448, py=-340, tics=4, seed=0)
    # --- independent review (appended so that every earlier case keeps its index) ------
    # scrollblock with an imp in B told to attack the player in A: PIT_CheckLine skips
    # the ML_BLOCKING test for a missile (p_map.c:214-221), so the fireball's position
    # checks put the blocking 48 line in spechit (240-244), the move is accepted and the
    # crossing loop calls P_CrossSpecialLine, which returns at once for MT_TROOPSHOT.
    add('fireball_through_blocking_scroller', 1, [monster(IMP, 133, 0, -180)], px=-200, py=0, attack_at=1, attack_thing=0, tics=45, **tough)
    # doorstep: the player uses the open door (it waits), uses it again (it closes at
    # once) onto the pillar straddling the door/step line, waits while it is shut and
    # uses it a third time (it reopens). PIT_ChangeSector returns at the MF_SHOOTABLE test
    # (p_map.c:1290-1294) and never sets nofit, so the door closes fully; the pillar's
    # P_ThingHeightClip reads floorz 8 (the step) above ceilingz 0 (the shut door).
    add('pillar_straddles_door_step', 6, [deco(PILLAR, 568, 64)], px=470, py=64, use_start=3, use_end=3, use2_start=6, use2_end=6, use3_start=90, use3_end=90, tics=170)
    for label, skill in [('spawn_filter_easy', EASY), ('spawn_filter_hard', HARD)]:
        add(label, 5, census_things(), skill=skill, px=-448, py=-340, tics=4, seed=0)
    return layouts
