"""Bounded source inputs for doors, lifts, floors, switches and the exit (M3h8 Stage B).

Every monster and pickup is an ordinary map THING spawned by the genuine
P_SpawnMapThing in a polygon room built by m3h8_geometry (the M3h6 automatic node
builder plus sector heights, tags and line specials); the player body spawns ONFLOORZ
at (px, py) facing east. The controlled between-tic inputs of M3h2/M3h5/M3h6 keep their
meaning (`hit_*`, `attack_*`, `push_*`; `script` 30 gives the player 200 health); M3h8
adds `BT_USE` windows (`use_start`..`use_end`, `use2_*`, `use3_*`: the button is held on
every tic of a window), one more turn (`turn2` on tic `turn2_at`) and two more forward
windows (`forward2` from `move2_start` to `move2_hold`, `forward3` likewise). Map THING
angles are written -180 and -90 rather than 180 and 270 (the same BAM after
P_SpawnMapThing's `ANG45 * (angle / 45)`, without the signed overflow the sanitizer build
would trap). Rooms (M3h8 note §11; two-sided scrollers stay rejected by decision N1, so
no room has one):

- **doorway**: rooms A (west) and B (east) joined by three DR doors 64 units deep, each
  with a line 1 on both faces and the door behind both. D1 (y in [-64, 64]) is closed
  (floor 0, ceiling 0); D2 (y in [128, 256]) starts open at its top (ceiling 124) on a
  32-unit floor step, so a monster standing in it is refused by the drop-off test with
  the DR lines in `spechit` and uses a DR line from the line's back side; D3 (y in
  [-256, -128]) starts open on a level floor, with things inside it. **hall** is the
  same room with D1 only (no sound path around the closed door).
- **switchyard**: a hall with an SR 63 switch (tag 1) on its north wall, the door it
  opens (sector T, tag 1, closed) in its east wall with a closet behind, an S1 exit
  switch (11) on its west wall, a plain east wall and an open DR door N in its south
  wall: a player standing in N is on the back side of N's lines.
- **lift**: a lower hall (floor 2) and an upper ledge (floor 64) joined by a 128 x 128
  WR 88 lift (tag 1, floor 64, `low` 2: 62 units, not a multiple of the speed 4); a
  second WR 88 line (tag 1) crosses the lower hall; the lift ceiling is a parameter:
  192 (riders fit), 118 (the last 62 -> 64 step does not fit the player: the lift
  stops short at 62, and the next cycle's `high` is 62) or 112 (an ordinary step
  does not fit: the lift reverses).
- **floors**: a corridor crossed by W1 lines, with a W1 36 target that jumps up (floor
  32 between neighbours at 64 and 0: destination 72), a W1 36 target that lowers (floor
  128 beside 64 and 0: destination 72), a W1 38 target (floor 48, lowest neighbour 0)
  and a W1 109 blazeOpen door (sector Z, tag 5) which also has a DR line (decision N5);
  items straddle step edges so their first box-wide clip lifts them.
- **dense**: a lane crossed by eight two-sided WR 88 lines four units apart (tag 5,
  one remote lift), all in one 2 x 2 cell window: a player box covers all eight, so a
  single P_CheckPosition fills `spechit` to exactly its source size (eight); the
  nine-line lane (`DENSE9`) is a Rust-only fixture the C harness never runs.

The sector order of each room fixes every line's front side and the BLOCKMAP list
order (lines are numbered in sector order, then ring order). No Rust output."""
IMP, POSSESSED, SHOTGUY, CHAINGUY, DEMON = 3001, 3004, 9, 65, 3002
ROSTER = (IMP, POSSESSED, SHOTGUY, CHAINGUY, DEMON)
CLIP, CLIPBOX, SHELLS, SHELLBOX, STIMPACK, MEDIKIT, SHOTGUN, CHAINGUN, GREENARMOR, BLUEARMOR, HEALTHBONUS, ARMORBONUS = 2007, 2048, 2008, 2049, 2011, 2012, 2001, 2002, 2018, 2019, 2014, 2015
PICKUPS = (CLIP, CLIPBOX, SHELLS, SHELLBOX, STIMPACK, MEDIKIT, SHOTGUN, CHAINGUN, GREENARMOR, BLUEARMOR, HEALTHBONUS, ARMORBONUS)
MAX_PICKUPS = 16
DR, SR_DOOR, W1_BLAZE, WR_LIFT, W1_TURBO, W1_LOWEST, S1_EXIT = 1, 63, 109, 88, 36, 38, 11


def doorway(extra=True):
    """Sector order: 0 A (west room), 1 B (east room), 2 D1 (closed door), then with
    `extra` 3 D2 (open door with a 32-unit step) and 4 D3 (open door, level floor).
    A spans x in [-512, 0], B x in [64, 576], both y in [-384, 384], floor 0, ceiling
    128; the doors span x in [0, 64]: D1 y in [-64, 64], D2 y in [128, 256], D3 y in
    [-256, -128]. Every door face carries a line 1 with the door behind it; the
    variant without `extra` (the `hall` room) has D1 only."""
    left = [(0, 384)] + ([(0, 256), (0, 128)] if extra else []) + [(0, 64), (0, -64)] + ([(0, -128), (0, -256)] if extra else []) + [(0, -384)]
    a = [(-512, 384)] + left + [(-512, -384)]
    right = [(64, -384)] + ([(64, -256), (64, -128)] if extra else []) + [(64, -64), (64, 64)] + ([(64, 128), (64, 256)] if extra else []) + [(64, 384)]
    b = [(576, 384), (576, -384)] + right
    sectors = [([a], 0, 128), ([b], 0, 128), ([[(0, 64), (64, 64), (64, -64), (0, -64)]], 0, 0)]
    lines = {((0, 64), (0, -64)): (DR, 0), ((64, -64), (64, 64)): (DR, 0)}
    if extra:
        sectors += [([[(0, 256), (64, 256), (64, 128), (0, 128)]], 32, 124), ([[(0, -128), (64, -128), (64, -256), (0, -256)]], 0, 124)]
        lines.update({((0, 256), (0, 128)): (DR, 0), ((64, 128), (64, 256)): (DR, 0), ((0, -128), (0, -256)): (DR, 0), ((64, -256), (64, -128)): (DR, 0)})
    return (sectors, {}, (), None, 4096, lines)


def switchyard():
    """Sector order: 0 H (hall x in [0, 512], y in [0, 384]), 1 S2 (south room x in
    [192, 320], y in [-192, -64]), 2 K (closet east of the tagged door), 3 N (open DR
    door x in [224, 288], y in [-64, 0] between H and S2: both its lines have N behind
    them), 4 T (door tag 1, x in [512, 576], y in [160, 224], closed). The SR 63 switch
    (tag 1) is the north wall piece x in [32, 96]; the S1 exit switch (11) is the west
    wall piece y in [288, 352]; the east wall below the door is plain."""
    hall = [(0, 384), (32, 384), (96, 384), (512, 384), (512, 224), (512, 160), (512, 0), (288, 0), (224, 0), (0, 0), (0, 288), (0, 352)]
    south = [(192, -64), (224, -64), (288, -64), (320, -64), (320, -192), (192, -192)]
    closet = [(576, 288), (704, 288), (704, 96), (576, 96), (576, 160), (576, 224)]
    sectors = [([hall], 0, 128), ([south], 0, 128), ([closet], 0, 128),
               ([[(224, 0), (288, 0), (288, -64), (224, -64)]], 0, 124),
               ([[(512, 224), (576, 224), (576, 160), (512, 160)]], 0, 0, dict(tag=1))]
    lines = {((32, 384), (96, 384)): (SR_DOOR, 1), ((0, 288), (0, 352)): (S1_EXIT, 0),
             ((288, 0), (224, 0)): (DR, 0), ((224, -64), (288, -64)): (DR, 0)}
    return (sectors, {}, (), None, 4096, lines)


def lift(ceiling=192):
    """Sector order: 0 L0a (x in [0, 512], y in [0, 128]), 1 L0b (y in [128, 256]),
    both floor 2, ceiling 192; 2 U (upper ledge y in [384, 640], floor 64, ceiling
    192); 3 P (the lift x in [192, 320], y in [256, 384], floor 64, tag 1, ceiling
    `ceiling`). The line between L0a and L0b and the line between U and P carry WR 88
    (tag 1); P's neighbours are L0b and U, so `low` is 2 and `high` the floor at
    activation."""
    l0a = [(0, 128), (512, 128), (512, 0), (0, 0)]
    l0b = [(0, 256), (192, 256), (320, 256), (512, 256), (512, 128), (0, 128)]
    upper = [(0, 640), (512, 640), (512, 384), (320, 384), (192, 384), (0, 384)]
    sectors = [([l0a], 2, 192), ([l0b], 2, 192), ([upper], 64, 192), ([[(192, 384), (320, 384), (320, 256), (192, 256)]], 64, ceiling, dict(tag=1))]
    lines = {((0, 128), (512, 128)): (WR_LIFT, 1), ((320, 384), (192, 384)): (WR_LIFT, 1)}
    return (sectors, {}, (), None, 4096, lines)


def floors():
    """A corridor y in [0, 128] (floor 0, ceiling 192) of sectors 0 C0 x in [0, 256],
    1 C1 [256, 512], 2 C2 [512, 768], 3 C3a [768, 896], 4 C3b [896, 1152], whose
    boundaries carry W1 36 (tag 2) at x = 256, W1 36 (tag 3) at 512, W1 38 (tag 4)
    at 768 and W1 109 (tag 5) at 896. North of it: 5 J (tag 2, x in [32, 160], y in
    [128, 256], floor 32) beside 6 Hj (x in [160, 224], floor 64): turbo destination
    72, above J (the jump-up); 7 K (tag 3, x in [288, 416], floor 128) beside 8 Hk
    (x in [416, 480], floor 64): destination 72, below K; 9 L (tag 4, x in [544, 672],
    floor 48, lowest neighbour 0); 10 R (room x in [800, 992], y in [192, 320],
    ceiling 128) behind 11 Z (tag 5, door x in [832, 960], y in [128, 192], closed),
    whose south face toward C3b carries a DR line (door kinds 1 and 109 share Z)."""
    c = lambda x0, x1, top: [(x0, 128)] + top + [(x1, 128), (x1, 0), (x0, 0)]
    sectors = [([c(0, 256, [(32, 128), (160, 128), (224, 128)])], 0, 192),
               ([c(256, 512, [(288, 128), (416, 128), (480, 128)])], 0, 192),
               ([c(512, 768, [(544, 128), (672, 128)])], 0, 192),
               ([c(768, 896, [(832, 128)])], 0, 192),
               ([c(896, 1152, [(960, 128)])], 0, 192),
               ([[(32, 256), (160, 256), (160, 128), (32, 128)]], 32, 192, dict(tag=2)),
               ([[(160, 256), (224, 256), (224, 128), (160, 128)]], 64, 192),
               ([[(288, 256), (416, 256), (416, 128), (288, 128)]], 128, 192, dict(tag=3)),
               ([[(416, 256), (480, 256), (480, 128), (416, 128)]], 64, 192),
               ([[(544, 256), (672, 256), (672, 128), (544, 128)]], 48, 192, dict(tag=4)),
               ([[(800, 320), (992, 320), (992, 192), (960, 192), (832, 192), (800, 192)]], 0, 128),
               ([[(832, 192), (960, 192), (960, 128), (896, 128), (832, 128)]], 0, 0, dict(tag=5))]
    lines = {((256, 128), (256, 0)): (W1_TURBO, 2), ((512, 128), (512, 0)): (W1_TURBO, 3),
             ((768, 128), (768, 0)): (W1_LOWEST, 4), ((896, 128), (896, 0)): (W1_BLAZE, 5),
             ((896, 128), (960, 128)): (DR, 0)}
    return (sectors, {}, (), None, 4096, lines)


def dense(count=8):
    """A lane y in [0, 128] (floor 0, ceiling 128): 0 Lw x in [0, 240], then count-1
    strips four units wide, then Le up to x = 512, separated by `count` two-sided WR 88
    lines (tag 5) at x = 240, 244, ...; and the remote lift Ld (tag 5, x in [384, 512],
    y in [128, 256], floor 32) north of Le. The eight lines (x in [240, 268]) lie in
    BLOCKMAP columns 1 and 2 of row 0: one 2 x 2 window. `count` 9 is the Rust-only
    fixture of the SpecialCross witness (never run in C)."""
    xs = [240 + 4 * i for i in range(count)]
    bounds = [0] + xs + [512]
    sectors = []
    for i in range(len(bounds) - 1):
        x0, x1 = bounds[i], bounds[i + 1]
        top = [(384, 128)] if x0 < 384 < x1 else []
        sectors.append(([[(x0, 128)] + top + [(x1, 128), (x1, 0), (x0, 0)]], 0, 128))
    sectors.append(([[(384, 256), (512, 256), (512, 128), (384, 128)]], 32, 128, dict(tag=5)))
    lines = {((x, 128), (x, 0)): (WR_LIFT, 5) for x in xs}
    return (sectors, {}, (), None, 4096, lines)


ROOMS = [doorway(), doorway(extra=False), switchyard(), lift(192), lift(118), lift(112), floors(), dense()]
ROOM_NAMES = ['doorway', 'hall', 'switchyard', 'lift', 'lift_short', 'lift_low', 'floors', 'dense']
DENSE9 = dense(9)


def populate(pack):
    core = pack.base; core.CASES.clear(); layouts = []
    def monster(kind, x, y=0, angle=0, options=7): return (x, y, angle, kind, options)
    def item(kind, x, y): return (x, y, 0, kind, 7)
    def add(label, room, things=(), hit_at=0, hit_thing=0, hit_amount=0, attack_at=0, attack_thing=0, push_at=0, push_x=0, push_y=0,
            use_start=0, use_end=0, use2_start=0, use2_end=0, use3_start=0, use3_end=0, turn2_at=0, turn2=0,
            forward2=0, move2_start=0, move2_hold=0, forward3=0, move3_start=0, move3_hold=0, **kw):
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
                              push_at=push_at, push_x=push_x, push_y=push_y,
                              use_start=use_start, use_end=use_end, use2_start=use2_start, use2_end=use2_end, use3_start=use3_start, use3_end=use3_end,
                              turn2_at=turn2_at, turn2=turn2, forward2=forward2, move2_start=move2_start, move2_hold=move2_hold,
                              forward3=forward3, move3_start=move3_start, move3_hold=move3_hold)
    north = dict(turn=16384, turn_hold=1); south = dict(turn=-16384, turn_hold=1); west = dict(turn=-32768, turn_hold=1)
    pistol = dict(attack_start=10, attack_end=200)
    tough = dict(script=30)
    # --- doorway --------------------------------------------------------------------
    # Walking east to door D1 and using it: open (62 tics), wait 150 tics, close.
    add('door_use_wait_close', 0, [], px=-64, py=0, forward=10, move_hold=8, use_start=20, use_end=21, tics=320)
    # Using D1 again while it rises (it closes at once) and again while it closes (it
    # goes back up).
    add('door_reuse_closes', 0, [], px=-64, py=0, forward=10, move_hold=8, use_start=20, use_end=21, use2_start=40, use2_end=41, use3_start=52, use3_end=53, tics=320)
    # Opening D1 and walking into the doorway: the closing door meets the player and
    # goes back up (crushed), repeatedly.
    add('door_reverse_on_player', 0, [], px=-64, py=0, forward=10, move_hold=8, use_start=20, use_end=21, forward2=25, move2_start=60, move2_hold=67, tics=520)
    # A zombieman standing in the open door D2 (32-unit step on both faces) chases the
    # player in A: every move is refused by the drop-off test with D2's DR lines in
    # spechit, so it uses a DR line from the line's back side (side 0 in P_Move); the
    # door opens (already at its top), waits, closes on it and reverses; a monster bump
    # while the door waits or rises does nothing, one while it closes sends it back up.
    add('door_monster_back_side', 0, [monster(POSSESSED, 32, 192, -180)], px=-200, py=192, tics=560, **tough)
    # A zombieman killed at tic 1 inside the open door D3 leaves a corpse and a dropped
    # clip beside a health bonus; the player uses D3 (already open: it waits and closes):
    # the closing door removes the dropped clip, leaves the bonus stuck under it and
    # crunches the corpse to gibs.
    add('door_gibs_drop_bonus', 0, [monster(POSSESSED, 32, -160, -180), item(HEALTHBONUS, 32, -224)], px=-48, py=-192, hit_at=1, hit_thing=0, hit_amount=100,
        use_start=10, use_end=11, tics=300)
    # A sleeping zombieman whose box overlaps the closed door D1 from room A (centre at
    # x = -12): the player opens D1 from B; the first rising step clips the zombieman
    # box-wide into a ceiling below its top (stuck shootable, V' case (d)) and later it
    # refuses every closing step under it.
    add('door_stuck_shootable', 0, [monster(POSSESSED, -12, 0, -180)], px=100, py=0, use_start=5, use_end=6, tics=520, **west)
    # --- hall (D1 only) -------------------------------------------------------------
    # Shots behind the closed door wake nobody (no sound through a zero opening, no
    # sight); the player then opens it and the zombieman sees the player.
    add('door_blocks_sight_and_sound', 1, [monster(POSSESSED, 300, 0, -180)], px=-40, py=0, attack_start=10, attack_end=30, use_start=40, use_end=41, tics=320, **tough)
    # An imp told to attack the player behind the closed door D1: its fireball explodes
    # on the door line, then its chase bumps D1 (two-sided, zero opening, in spechit) and
    # P_UseSpecialLine opens it.
    add('door_monster_bump', 1, [monster(IMP, 300, 0, -180)], px=-200, py=0, attack_at=1, attack_thing=0, tics=420, **tough)
    # --- switchyard -----------------------------------------------------------------
    # The SR 63 switch: the tagged door T opens and the switch texture changes and
    # restores after 35 tics; a use while T is active starts nothing (no texture
    # change); a use after T has closed opens it again.
    add('switch_sr_door', 2, [], px=64, py=330, use_start=3, use_end=4, use2_start=60, use2_end=61, use3_start=300, use3_end=301, tics=420, **north)
    # The S1 exit switch: the special is cleared, then G_ExitLevel; the case ends at the
    # end of that tic (the next G_Ticker would run G_DoCompleted).
    add('switch_exit', 2, [], px=40, py=320, use_start=3, use_end=4, tics=20, **west)
    # The exit switch used while the SR switch's button counts down: the exit sound
    # comes from buttonlist[0]'s sound origin, the hall.
    add('switch_exit_after_button', 2, [], px=40, py=330, use_start=3, use_end=4, turn2_at=5, turn2=16384, use2_start=8, use2_end=9, tics=20, **north)
    # BT_USE held from the first tic: usedown starts true, so nothing happens until the
    # button is released and pressed again.
    add('use_held_latch', 2, [], px=64, py=330, use_start=1, use_end=30, use2_start=40, use2_end=41, tics=80, **north)
    # Use against a plain one-sided wall: sfx_noway at the player, the trace stops.
    add('use_noway_wall', 2, [], px=450, py=64, use_start=3, use_end=4, tics=10)
    # Use against the closed tagged door T (a two-sided line without a special and a
    # zero opening): sfx_noway too.
    add('use_noway_closed_door', 2, [], px=450, py=192, use_start=3, use_end=4, tics=10)
    # Use from inside the open DR door N against its north line, whose back side faces
    # the player: P_UseSpecialLine returns false at once and the trace stops.
    add('use_back_side', 2, [], px=256, py=-32, use_start=3, use_end=4, tics=10, **north)
    # The player killed at tic 2, then BT_USE while dead: P_DeathThink requests a reborn
    # (PST_REBORN); the case ends at the end of that tic (the next G_Ticker would run
    # G_DoReborn, a level restart).
    add('dead_use_reborn', 2, [], px=256, py=192, hit_at=2, hit_thing=0, hit_amount=200, use_start=20, use_end=22, tics=40)
    # --- lift -----------------------------------------------------------------------
    # The player crosses the WR 88 line in the lower hall: the lift lowers (62 units at
    # 4 per tic, the last step short), the player walks on, waits against the ledge, and
    # stands still while it rides up; a health bonus on the ledge overlapping the player's
    # box is out of reach (62 units above) until the rising lift's clip brings the player
    # within 56: it is picked up inside P_ChangeSector. At the top the player walks onto
    # the ledge across the other 88 line one tic after the lift's thinker was removed:
    # a new lift starts.
    add('lift_player', 3, [item(HEALTHBONUS, 256, 400)], px=256, py=64, forward=25, move_hold=60, forward2=25, move2_start=160, move2_hold=220, tics=300, **north)
    # The same ride pushing north throughout: the player steps onto the ledge across the
    # other 88 line in the op after the lift's thinker removed itself, before P_RunThinkers
    # reaches the removed entry, so the new lift joins the list beside it (two entries for
    # one sector within a pass, M3h8 note §2.1 and §8).
    add('lift_retrigger_beside_removed', 3, [], px=256, py=64, forward=25, move_hold=220, tics=200, **north)
    # A zombieman asleep on the lift rides it down and up.
    add('lift_monster_rider', 3, [monster(POSSESSED, 256, 320, 90)], px=256, py=64, forward=25, move_hold=10, tics=300, **north)
    # A zombieman on the ledge woken by shots chases the player below and crosses the
    # WR 88 line onto the lift: a monster lowers the lift.
    add('lift_monster_cross', 3, [monster(POSSESSED, 256, 560, -90)], px=256, py=200, attack_start=5, attack_end=8, tics=400, **dict(tough, **north))
    # Lift ceiling 118: the last 62 -> 64 step does not fit the player and the lift stops
    # at 62; the player walks off, crosses the 88 line again (high is now 62) and rides
    # the second cycle to 62.
    add('lift_last_step_short', 4, [], px=256, py=64, forward=25, move_hold=150, forward2=-25, move2_start=170, move2_hold=200, forward3=25, move3_start=230, move3_hold=520, tics=600, **north)
    # Lift ceiling 112: an ordinary rising step does not fit the player: the lift
    # reverses (crushed) and cycles.
    add('lift_reverse_at_top', 5, [], px=256, py=64, forward=25, move_hold=380, tics=400, **north)
    # --- floors ---------------------------------------------------------------------
    # W1 36 at x = 256: J (floor 32 between 0 and the 64 pedestal) turbo-lowers to 72,
    # above itself, in one tic; a zombieman and a clip on J rise with it, and a clip
    # whose centre is in the corridor but whose box overlaps J rises onto it at its
    # first box-wide clip.
    add('floor_turbo_jump_up', 6, [monster(POSSESSED, 64, 208, 90), item(CLIP, 128, 192), item(CLIP, 96, 116)], px=200, py=64, forward=25, move_hold=8, tics=60)
    # W1 36 at x = 512: K (floor 128) lowers to 72 at speed 4 with a clip on it.
    add('floor_turbo_lower', 6, [item(CLIP, 352, 192)], px=450, py=64, forward=25, move_hold=8, tics=60)
    # W1 38 at x = 768: L lowers from 48 to 0 at speed 1 (sfx_stnmov every 8 tics) with a
    # sleeping zombieman and a health bonus on it, and a stimpack straddling its edge.
    add('floor_lower_to_lowest', 6, [monster(POSSESSED, 576, 200, 90), item(HEALTHBONUS, 640, 224), item(STIMPACK, 608, 116)], px=720, py=64, forward=25, move_hold=12, tics=120)
    # W1 109 at x = 896: door Z blaze-opens (speed 8) and its thinker is removed at the top.
    add('blaze_open', 6, [], px=850, py=64, forward=25, move_hold=8, tics=60)
    # Decision N5: the player crosses the 109 line, turns north and uses the DR line of
    # the same sector while the blazeOpen door rises: the door reverses down and stays at
    # the floor as a live thinker (blazeOpen has no closing pastdest case); a second use
    # sends it up and it is removed at its top.
    add('blaze_reversed', 6, [], px=880, py=100, forward=25, move_hold=3, turn2_at=13, turn2=16384, use_start=16, use_end=17, use2_start=60, use2_end=61, tics=120)
    # Use against the W1 36 line from its front: P_UseSpecialLine returns true and does
    # nothing (no use case), the trace stops, the special stays.
    add('use_walkover_line', 6, [], px=220, py=64, use_start=3, use_end=4, tics=10)
    # --- dense ----------------------------------------------------------------------
    # The player walks through eight WR 88 lines four units apart: a position check
    # whose box covers all eight fills spechit to its source size; each crossing calls
    # EV_DoPlat (the first one starts the remote lift).
    add('dense_spechit_8', 7, [], px=200, py=64, forward=25, move_hold=12, tics=200)
    # An imp told to attack the player: its fireballs cross the same lines (a missile's
    # spechit; P_CrossSpecialLine returns at once for MT_TROOPSHOT).
    add('dense_imp_fireballs', 7, [monster(IMP, 400, 64, -180)], px=100, py=64, attack_at=1, attack_thing=0, tics=300, **tough)
    return layouts
