"""Bounded source inputs for exploding barrels, the radius attack, pickup scale and every
THING angle (M3h9b).

Every barrel, monster, pickup and decoration is an ordinary map THING spawned by the
genuine P_SpawnMapThing in a polygon room built by m3h9b_geometry (the M3h9 builder);
the player body spawns ONFLOORZ at (px, py) facing east after the THINGs, so it is the
newest thing linked in its BLOCKMAP cell. Every controlled between-tic input of
M3h2..M3h9 keeps its meaning (`attack_*` the player's BT_ATTACK window, `ready`/`owned`
the weapon, `script` 30 gives the player 200 health and 31 also 200 green-armor points,
`chase_*` a target assignment plus see-state entry, `attack_at` an Imp's S_TROO_ATK1
entry, `hit_*` a controlled P_DamageMobj(thing, NULL, NULL, amount), the `BT_USE`
windows, `turn2`, `forward2/3`, `pre_damage`). No input is added. Scenarios prefer real
shooters (the player's pistol or fist, an Imp's fireball, a Zombieman's hitscan) so a
barrel's `target` is set as in play; a `hit_*` input is used only where no real shooter
can produce the situation, and the case comment says so.

M3h9b admits DoomEd 2035 (MT_BARREL), at most 16 barrels, at most 64 pickups, 32
decorations, 8 monsters and every `short` THING angle (`add` below; the M3h9 `add`
refuses all four). Rooms (M3h9b note §7):

- **range**: a hall (x in [-512, 512], y in [-128, 128], floor 0, ceiling 128).
- **cluster**: a hall (x in [-512, 512], y in [-512, 512]); BLOCKMAP origin (-512, -512).
- **player_range**: a hall (x in [-512, 512], y in [-256, 256], ceiling 200) with a
  72-unit ledge (x in [256, 512], y in [128, 256]) and a void pillar hiding it from the
  south.
- **monsters**: `corridor_hall()`: a hall (x in [0, 512], y in [-320, 320]) entered by a
  corridor (x in [-512, 0], y in [-32, 32]); ambush monsters off the corridor axis hear
  the pistol but never see the player.
- **walls**: `walls()`: a thin wall with a 32-unit window gap, a 12-unit curb, a WR 88
  line and the lift it lowers, a 32-unit ledge.
- **door**: `door()`: a hall, a DR door (ceiling 124) and a closet.
- **fireball**: `corridor_hall(pedestal=True)`: the monsters room with a 24-unit pedestal
  on the corridor axis.
- **dense64**: a hall (x in [-1152, 1152], y in [-512, 512]) holding 64 pickups of the
  E1M1 kinds, 16 barrels, 6 monsters and 27 decorations.
- **angles**: a flat room with 17 floor lamps whose records carry the angles of decision
  F8.
- **embedded**: a flat room; things overlap or touch its one-sided walls.

Lines of fire stay inside a BLOCKMAP row or column (never on its boundary): a trace is
offered only the things linked by their centre in the cells it crosses
(p_maputl.c:846-858), so a barrel straddling a cell boundary can be missed by a trace
that runs along it. The scenarios whose outcome depends on gameplay draws start at a
declared seed (the starting prndindex) chosen by trying every rndtable index: FIST_SEED
(a 20-point blow), WAKE_SEED (a failed pain draw inside the walk), FALL_SEED (an odd
fall-forward draw), INFIGHT_SEED and FIREBALL_SEED (a monster wounds the barrel first).

The sector order of each room fixes every line's front side and the BLOCKMAP list
order. No Rust output."""
from m3h8_cases import IMP, POSSESSED, ROSTER, CLIP, CLIPBOX, SHELLS, SHELLBOX, STIMPACK, MEDIKIT, SHOTGUN, CHAINGUN, GREENARMOR, BLUEARMOR, HEALTHBONUS, ARMORBONUS, PICKUPS, DR, WR_LIFT  # noqa: F401
from m3h9_cases import LAMP, PILLAR, CANDELABRA, GIBS, DEADPLAYER, MESS10, MESS12, MEDIUM, spawns, SPAWN_TICS_DRAW as M3H9_SPAWN_TICS_DRAW  # noqa: F401
from m3h9b_geometry import DECORATIONS, BARREL as BARREL_INFO
BARREL = BARREL_INFO['doomednum']
MAX_MONSTERS, MAX_PICKUPS, MAX_DECORATIONS, MAX_BARRELS = 8, 64, 32, 16
SPAWN_TICS_DRAW = set(M3H9_SPAWN_TICS_DRAW) | {BARREL}  # S_BAR1 has 6 tics: the spawn tics draw (p_mobj.c:787-788)
MTF_AMBUSH = 8
FIST_SEED, WAKE_SEED, FALL_SEED = 2, 22, 1
INFIGHT_TICS, FIREBALL_TICS, INFIGHT_SEED, FIREBALL_SEED = 430, 140, 114, 36


def hall(x0, x1, y0, y1):
    """A clockwise rectangle (y up)."""
    return [(x0, y1), (x1, y1), (x1, y0), (x0, y0)]


def range_room():
    """Sector order: 0 H (x in [-512, 512], y in [-128, 128])."""
    return ([([hall(-512, 512, -128, 128)], 0, 128)], {}, (), None, 4096, {})


def cluster():
    """Sector order: 0 H (x in [-512, 512], y in [-512, 512]); BLOCKMAP origin (-512, -512)."""
    return ([([hall(-512, 512, -512, 512)], 0, 128)], {}, (), None, 4096, {})


def player_range():
    """Sector order: 0 H (the hall x in [-512, 512], y in [-256, 256] minus the ledge, with a
    50 x 60 void pillar at x in [200, 250], y in [-60, 0] that hides the ledge from the
    south), 1 L (the ledge x in [256, 512], y in [128, 256], floor 72). Ceiling 200."""
    outer = [(-512, 256), (256, 256), (256, 128), (512, 128), (512, -256), (-512, -256)]
    pillar = [(200, -60), (250, -60), (250, 0), (200, 0)]
    ledge = [(256, 256), (512, 256), (512, 128), (256, 128)]
    return ([([outer, pillar], 0, 200), ([ledge], 72, 200)], {}, (), None, 4096, {})


def corridor_hall(pedestal=False):
    """Sector order: 0 E (the hall x in [0, 512], y in [-320, 320]), 1 C (a corridor x in
    [-512, 0], y in [-32, 32] opening into the hall's west wall). A player in the corridor
    sees only the strip of the hall in line with it: an ambush monster elsewhere in the
    hall hears the pistol but cannot see the player, so it keeps its spawn state
    (A_Look, p_enemy.c:609-626). BLOCKMAP origin (-512, -320).
    With `pedestal` (the fireball room) a 24-unit pedestal island P x in [130, 170], y in
    [-20, 20] (sector 2, floor 24) stands on the axis."""
    east = [(0, 320), (512, 320), (512, -320), (0, -320), (0, -32), (0, 32)]
    lane = [(-512, 32), (0, 32), (0, -32), (-512, -32)]
    if not pedestal:
        return ([([east], 0, 128), ([lane], 0, 128)], {}, (), None, 4096, {})
    hole = [(130, -20), (170, -20), (170, 20), (130, 20)]
    island = [(130, 20), (170, 20), (170, -20), (130, -20)]
    return ([([east, hole], 0, 128), ([lane], 0, 128), ([island], 24, 128)], {}, (), None, 4096, {})


def walls():
    """Sector order: 0 W (west hall x in [-512, 0], y in [-128, 256]), 1 G (window gap x in
    [0, 16], y in [-16, 16]: the only opening of the thin wall x in [0, 16]), 2 E (east
    hall x in [16, 512], y in [-256, 256] minus the ledge), 3 K (a 12-unit curb island x
    in [-320, -288], y in [96, 224], floor 12, a hole of W), 4 S (south strip x in [-512,
    0], y in [-256, -128]; its shared edge with W is a WR 88 line, tag 1), 5 P (the lift,
    an island of S at x in [-300, -200], y in [-230, -170], floor 48, tag 1), 6 U (a
    32-unit ledge x in [256, 512], y in [128, 256], floor 32, a notch of E)."""
    west = [(-512, 256), (0, 256), (0, 16), (0, -16), (0, -128), (-512, -128)]
    curb_hole = [(-320, 96), (-288, 96), (-288, 224), (-320, 224)]
    gap = [(0, 16), (16, 16), (16, -16), (0, -16)]
    east = [(16, 256), (256, 256), (256, 128), (512, 128), (512, -256), (16, -256), (16, -16), (16, 16)]
    curb = [(-320, 224), (-288, 224), (-288, 96), (-320, 96)]
    south = [(-512, -128), (0, -128), (0, -256), (-512, -256)]
    lift_hole = [(-300, -230), (-200, -230), (-200, -170), (-300, -170)]
    lift = [(-300, -170), (-200, -170), (-200, -230), (-300, -230)]
    ledge = [(256, 256), (512, 256), (512, 128), (256, 128)]
    sectors = [([west, curb_hole], 0, 128), ([gap], 0, 128), ([east], 0, 128), ([curb], 12, 128), ([south, lift_hole], 0, 128),
               ([lift], 48, 128, dict(tag=1)), ([ledge], 32, 128)]
    return (sectors, {}, (), None, 4096, {((0, -128), (-512, -128)): (WR_LIFT, 1)})


def door():
    """Sector order: 0 H (hall x in [-512, 0], y in [-256, 256]), 1 K (closet x in [64, 192],
    y in [-128, 128]), 2 D (DR door x in [0, 64], y in [-64, 64], floor 0, ceiling 124:
    open at the top a DR use computes, so a use leaves it waiting VDOORWAIT tics and a
    second use closes it at once; line 1 on both faces)."""
    h = [(-512, 256), (0, 256), (0, 64), (0, -64), (0, -256), (-512, -256)]
    closet = [(64, 128), (192, 128), (192, -128), (64, -128), (64, -64), (64, 64)]
    d = [(0, 64), (64, 64), (64, -64), (0, -64)]
    lines = {((0, 64), (0, -64)): (DR, 0), ((64, -64), (64, 64)): (DR, 0)}
    return ([([h], 0, 128), ([closet], 0, 128), ([d], 0, 124)], {}, (), None, 4096, lines)


def dense():
    """Sector order: 0 H (x in [-1152, 1152], y in [-512, 512]); BLOCKMAP origin (-1152, -512)."""
    return ([([hall(-1152, 1152, -512, 512)], 0, 128)], {}, (), None, 4096, {})


def angles_room():
    """Sector order: 0 H (x in [-512, 512], y in [-256, 256])."""
    return ([([hall(-512, 512, -256, 256)], 0, 128)], {}, (), None, 4096, {})


def embedded():
    """Sector order: 0 H (x in [-512, 512], y in [-256, 256]); every wall one-sided."""
    return ([([hall(-512, 512, -256, 256)], 0, 128)], {}, (), None, 4096, {})


ROOMS = [range_room(), cluster(), player_range(), corridor_hall(), walls(), door(), corridor_hall(pedestal=True), dense(), angles_room(), embedded()]
ROOM_NAMES = ['range', 'cluster', 'player_range', 'monsters', 'walls', 'door', 'fireball', 'dense64', 'angles', 'embedded']
ANGLES = (0, 45, 90, 135, 180, 225, 270, 315, 30, 100, 359, -1, -45, -100, 400, 32767, -32768)


def embedded_barrel_lane():
    """Rust-only (never run in C): the embedded room with a barrel whose box overlaps the
    west wall by 4 units (centre 6 units from it). The S' spawn rule (decision F3)
    refuses it: for radius 10 the required margin (31 - 10) * w(theta) exceeds any
    overlapping position."""
    return (embedded(), [(-506, 0, 0, BARREL, 7)], 'barrel at (-506, 0), box x in [-516, -496] overlapping the west wall x = -512 by 4 units')


def intercept_lane(strips=80):
    """Rust-only (never run in C): a lane x in [-1152, 1152], y in [-64, 64] split into
    `strips` sectors by `strips - 1` two-sided lines, with 64 pickups along y = +-4. A
    pistol trace east from (-1100, 0) crosses every internal line east of x = -1100 and
    every pickup: more than 128 intercepts, so the C harness (which aborts before the
    129th entry) never runs it; it is the input of the Rust-only Intercepts witness."""
    xs = [-1152 + (2304 * k) // strips for k in range(strips + 1)]
    sectors = [([[(x0, 64), (x1, 64), (x1, -64), (x0, -64)]], 0, 128) for x0, x1 in zip(xs, xs[1:])]
    things = [(-1000 + 30 * i, 4 if i % 2 else -4, 0, ARMORBONUS, 7) for i in range(MAX_PICKUPS)]
    crossed = sum(-1100 < x < -1100 + 2048 for x in xs[1:-1])
    note = (f'{strips - 1} internal two-sided lines at x = {xs[1]}..{xs[-2]}; the 2,048-unit trace from x = -1100 crosses {crossed} of them '
            f'and the {len(things)} armor bonuses at x = -1000..{-1000 + 30 * (MAX_PICKUPS - 1)}: at least {crossed + len(things)} intercepts')
    return ((sectors, {}, (), None, 4096, {}), things, note)


def dense_things():
    """64 pickups of E1M1's kinds (52 as E1M1 plus 12 more bonuses, clips and shells; skill
    bits 7, 6 or 3, all spawning on medium), 16 barrels, 6 monsters (4 Zombiemen, 2 Imps,
    ambush) and 27 decorations of the seven E1M1 types. Every pickup and decoration
    stands within 8 units of the row y = 64 crossed by the player's trace, 64 units inside
    BLOCKMAP row 4 of the origin (-1152, -512) (a trace along a row boundary is nudged
    into one row, p_maputl.c:777-781, and never offered the things linked in the other);
    barrels and monsters stand elsewhere."""
    kinds = ([SHOTGUN] + [CLIP] * 4 + [SHELLS] * 4 + [STIMPACK] + [MEDIKIT] * 3 + [HEALTHBONUS] * 16 + [ARMORBONUS] * 29 +
             [GREENARMOR, BLUEARMOR, CLIPBOX] + [SHELLBOX] * 3)
    assert len(kinds) == 64
    decorations = [LAMP] * 6 + [PILLAR] * 3 + [CANDELABRA] * 3 + [GIBS] * 5 + [DEADPLAYER] * 4 + [MESS10] * 3 + [MESS12] * 3
    assert len(decorations) == 27
    things = []
    for i, kind in enumerate(kinds):
        things.append((-1000 + 30 * i, 68 if i % 2 else 60, 0, kind, (7, 6, 3)[i % 3]))
    for i, kind in enumerate(decorations):
        things.append((-985 + 72 * i, 72 if i % 2 else 56, 0, kind, 7))
    for i in range(16):
        things.append((-900 + 120 * (i % 8), -300 - 64 * (i // 8), 0, BARREL, 7))
    for i, kind in enumerate([POSSESSED] * 4 + [IMP] * 2):
        things.append((-800 + 300 * i, 420, 90, kind, 7 | MTF_AMBUSH))
    return things


def populate(pack):
    core = pack.base; core.CASES.clear(); layouts = []
    def thing(kind, x, y, angle=0, options=7): return (x, y, angle, kind, options)
    def add(label, room, things=(), skill=MEDIUM, hit_at=0, hit_thing=0, hit_amount=0, attack_at=0, attack_thing=0, push_at=0, push_x=0, push_y=0,
            use_start=0, use_end=0, use2_start=0, use2_end=0, use3_start=0, use3_end=0, turn2_at=0, turn2=0,
            forward2=0, move2_start=0, move2_hold=0, forward3=0, move3_start=0, move3_hold=0, chase_at=0, chase_thing=0, **kw):
        things = list(things)
        assert all(t[3] in ROSTER or t[3] in PICKUPS or t[3] in DECORATIONS or t[3] == BARREL for t in things)
        assert all(-32768 <= t[2] <= 32767 for t in things)  # mapthing_t.angle is a short
        spawned = [t for t in things if spawns(t, skill)]
        assert sum(t[3] in ROSTER for t in spawned) <= MAX_MONSTERS
        assert sum(t[3] in PICKUPS for t in spawned) <= MAX_PICKUPS
        assert sum(t[3] in DECORATIONS for t in spawned) <= MAX_DECORATIONS
        assert sum(t[3] == BARREL for t in spawned) <= MAX_BARRELS
        assert all(abs(t[0]) <= 1200 and abs(t[1]) <= 1200 for t in things)
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
    tough = dict(script=30)
    twice = dict(script=1)  # BT_ATTACK on tics 21..34 and 70..90: an accurate shot once the pistol is up (24 or 25), another at 74, a refire shot at 88
    north = dict(turn=16384, turn_hold=1); south = dict(turn=-16384, turn_hold=1); west = dict(turn=-32768, turn_hold=1)
    ambush = 7 | MTF_AMBUSH
    # --- range ------------------------------------------------------------------------
    # One pistol shot east at the barrel: accurate (refire 0), so damage, puff (4 draws:
    # jitter 2, lastlook, tics) and the pain draw; the barrel survives, takes the player
    # as target with threshold 100 (no state change: seestate S_NULL) and is thrust east.
    add('barrel_wounded', 0, [thing(BARREL, 0, 0)], px=-256, py=0, attack_start=20, attack_end=20, tics=40)
    # Two accurate shots (script 1): the first wounds, the second kills; S_BEXP with the
    # tics draw, A_Scream (sfx_barexp, no draw), A_Explode nine tics after S_BEXP3 with no
    # shootable thing in its 3 x 3 window (the player stands two cells west), S_NULL
    # removal. The barrel stays MF_SOLID until removal; the refire shot at tic 88 passes
    # the dying barrel.
    add('barrel_killed', 0, [thing(BARREL, 0, 0)], px=-256, py=0, tics=110, **twice)
    # One fist blow (always spread) for 20: the barrel dies in one blow, so the retarget
    # never runs and its target stays NULL; the puff enters S_PUFF3 (attackrange ==
    # MELEERANGE). The explosion's bomb source is NULL; the player beside it is hurt
    # with attacker NULL.
    add('fist_kills_barrel', 0, [thing(BARREL, 0, 0)], px=-36, py=0, ready=0, attack_start=20, attack_end=20, tics=50, seed=FIST_SEED, **tough)
    # The same blow without the extra health; the player runs after the sliding barrel and
    # the explosion (110) kills him with a NULL source (frags[0]++, PST_DEAD, P_DropWeapon, no attacker); BT_USE afterwards
    # requests the reborn and the case ends (the C harness does not run G_DoReborn).
    add('player_killed_by_barrel', 0, [thing(BARREL, 0, 0)], px=-36, py=0, ready=0, attack_start=20, attack_end=20, forward2=50, move2_start=25, move2_hold=36, use_start=60, use_end=60, tics=70, seed=FIST_SEED)
    # The aim of every shot picks the first live barrel; once it dies the next shot's aim
    # passes it (XT, the non-shootable skip) and picks the second barrel 240 units behind.
    add('autoaim_barrel', 0, [thing(BARREL, 0, 0), thing(BARREL, 240, 0)], px=-256, py=0, tics=90, **twice)
    # --- cluster ----------------------------------------------------------------------
    # B0 at (0, -4): BLOCKMAP row (-4 + 512) >> 7 = 3, 124 units into it, so its window
    # covers rows 2..4 (y in [-256, 128)). B1..B4 within 40 units die in its explosion;
    # W at (0, -124) (120 units, same rows) takes 128 - 110 = 18 and survives; Q at
    # (0, 132) is 136 units away (dist - radius = 126 < 128) but in row 5, outside the
    # wrapped 3 x 3 window, so the first explosion never visits it; the intended
    # 160-unit window would cover row 5. The second-generation explosions run in
    # thinker order (lump order) in the barrels' own thinkers.
    cluster_things = [thing(BARREL, 0, -4), thing(BARREL, 28, -4), thing(BARREL, 14, 22), thing(BARREL, 14, -30), thing(BARREL, 40, 20),
                      thing(BARREL, 0, -124), thing(BARREL, 0, 132)]
    add('chain_reaction', 1, cluster_things, px=-400, py=-4, tics=140, **twice)
    # --- player_range -----------------------------------------------------------------
    # The player picks up green armor (walking two tics west over it), then kills barrel
    # A; A's explosion kills barrel B in one blow (B's target stays NULL) and grazes the
    # player (attacker = the player); B's explosion then hurts the player: armor
    # absorption, thrust away from B, attacker NULL, pain draw. (The line of fire is y =
    # 40, 40 units inside BLOCKMAP row 2: a trace along a row boundary is never offered
    # the things linked in the next row, p_maputl.c:846-858.)
    add('player_hurt_by_barrel', 2, [thing(BARREL, -200, 40), thing(BARREL, -220, 120), thing(GREENARMOR, -115, 40)], px=-90, py=40,
        forward=25, move_hold=2, tics=150, **dict(twice, **west))
    # The player wounds and kills barrel C 100 units east; C's explosion hurts the
    # player with C's target, the player himself, as source: attacker = the player.
    add('player_hurt_by_own_barrel', 2, [thing(BARREL, -300, 40)], px=-400, py=40, tics=110, **twice)
    # A Zombieman on the 72-unit ledge (ambush, hidden from the player by the pillar)
    # 110 units from the barrel: damage 128 - 90 = 38 < 40 kills it (health 20) and it
    # stands 72 > 64 units above the dead barrel, so the fall-forward draw is taken; the
    # bomb source is the player (killcount credit); the clip drop draws lastlook. At
    # FALL_SEED the draw is odd: the Zombieman is thrown toward the barrel with 4x thrust.
    add('fall_forward_draw', 2, [thing(BARREL, 166, 100), thing(POSSESSED, 276, 148, 90, ambush)], px=166, py=-200, tics=110, seed=FALL_SEED, **dict(twice, **north))
    # --- monsters (corridor hall) -----------------------------------------------------
    # The player in the corridor kills barrel A; A's explosion kills barrel B in one blow
    # (B's target NULL); B's explosion kills two ambush Zombiemen (players[0] credit: a
    # NULL source takes the else branch of p_inter.c:692-697), drops their clips and
    # hurts an ambush Imp (pain draw, no retarget: source NULL).
    monsters = [thing(BARREL, 40, 0), thing(BARREL, 100, 100), thing(POSSESSED, 220, 170, 90, ambush), thing(POSSESSED, 40, 200, 90, ambush), thing(IMP, 140, 250, 90, ambush)]
    add('barrel_kills_monsters', 3, monsters, px=-300, py=0, tics=150, **twice)
    # A Zombieman on the corridor axis, told to attack the player (chase), shoots west;
    # a bullet wounds the barrel in its line of fire (the barrel's target becomes the
    # Zombieman, threshold 100) and a later one kills it; the explosion kills an ambush
    # Zombieman (credit players[0]: the source is a monster, p_inter.c:692-697), hurts
    # an ambush Imp, which retargets the shooter (infighting), and hurts the shooter, which
    # has walked into range (no self-exclusion for the bomb source).
    infight = [thing(POSSESSED, 300, 0, -180), thing(BARREL, 40, 0), thing(IMP, 60, 100, 90, ambush), thing(POSSESSED, 40, -80, -90, ambush)]
    add('barrel_infight', 3, infight, px=-450, py=0, chase_at=1, chase_thing=0, tics=INFIGHT_TICS, seed=INFIGHT_SEED, **tough)
    # The player wounds and kills barrel A; its explosion kills an ambush Zombieman:
    # killcount credit through source->player.
    add('player_credit', 3, [thing(BARREL, 40, 0), thing(POSSESSED, 120, 90, 90, ambush)], px=-300, py=0, tics=110, **twice)
    # Survivors in their spawn state with threshold 0 and a non-NULL bomb source: a failed
    # pain draw enters the see state inside P_DamageMobj, whose A_Chase may move and
    # relink the monster in the middle of the block walk.
    wake = [thing(BARREL, 40, 0), thing(IMP, 120, 100, 90, ambush), thing(IMP, 130, -100, -90, ambush), thing(POSSESSED, 160, 80, 90, ambush), thing(IMP, 40, 110, 90, ambush)]
    add('monster_wakes_mid_walk', 3, wake, px=-300, py=0, tics=110, seed=WAKE_SEED, **twice)
    # --- walls ------------------------------------------------------------------------
    # The player in W shoots barrel X in E through the window gap; X's explosion sees the
    # player through the gap (damage through the opening).
    add('radius_through_window', 4, [thing(BARREL, 60, 0)], px=-40, py=0, tics=110, **twice)
    # A barrel west of the 12-unit curb (its box overlaps the curb's west line by 7) dies
    # in place and explodes; the player east of the curb is in range (dist 107) but
    # P_CheckSight (player, dead barrel) is cut by the curb's west line, 3 units from the
    # barrel's centre: the dead barrel's window is its bottom 10.5 units. Controlled
    # kill (hit_*: inflictor and source NULL): every real shooter thrusts the barrel away
    # from itself, and the curb must stay within 3.8 units of the dead barrel's centre on
    # the victim's side for a 12-unit step to cut the 10.5-unit window.
    add('radius_blocked_by_step', 4, [thing(BARREL, -323, 160)], px=-200, py=160, hit_at=10, hit_thing=0, hit_amount=20, tics=40, **west)
    # One shot pushes a barrel 2 units from the east wall into it: P_TryMove fails and a
    # non-player non-missile stops dead (momx = momy = 0).
    add('barrel_pushed_into_wall', 4, [thing(BARREL, 498, -100)], px=200, py=-100, attack_start=20, attack_end=20, tics=40)
    # One shot pushes a barrel 6 units north of the WR 88 line south across it: the
    # crossing loop calls P_CrossSpecialLine for the barrel, EV_DoPlat lowers the lift.
    add('barrel_slides_over_lift_line', 4, [thing(BARREL, -100, -122)], px=-100, py=100, attack_start=20, attack_end=20, tics=60, **south)
    # The player kills barrel P on the 32-unit ledge; P's explosion kills barrel T beside
    # it in one blow and thrusts it south: T (MF_DROPOFF since P_KillMobj) slides off the
    # ledge, falls and explodes below.
    add('dying_barrel_dropoff', 4, [thing(BARREL, 300, 190), thing(BARREL, 330, 150)], px=300, py=-200, tics=120, **dict(twice, **north))
    # --- door -------------------------------------------------------------------------
    # The player uses the open door at tic 3 (it waits VDOORWAIT tics, then closes),
    # turns toward barrel P and kills it; P's explosion kills barrel T beside the door
    # and pushes it under the closing door, whose ceiling then falls below T's 10.5 units:
    # PIT_ChangeSector gives the dying barrel S_GIBS before its A_Explode; it never
    # explodes and is never removed; the door closes.
    add('door_gibs_dying_barrel', 5, [thing(BARREL, -90, -55), thing(BARREL, -12, -40)], px=-40, py=50, use_start=3, use_end=3,
        turn2_at=5, turn2=-21019, attack_start=172, attack_end=188, tics=240, **tough)
    # A live barrel inside the door sector: the second use closes the door, which meets
    # the barrel's 42 units: nofit, the door reverses.
    add('door_reverses_on_live_barrel', 5, [thing(BARREL, 32, 0)], px=-40, py=0, use_start=3, use_end=3, use2_start=6, use2_end=6, tics=80)
    # --- fireball --------------------------------------------------------------------
    # (In the monsters room, whose axis has no pedestal.) An Imp told to chase the player fires along the corridor axis; a fireball hits the
    # barrel in between (the missile damage draw, P_DamageMobj(barrel, missile, imp)); the
    # barrel's target becomes the Imp; its explosion kills an ambush Zombieman (credit
    # players[0]).
    add('fireball_kills_barrel', 3, [thing(IMP, 300, 0, -180), thing(BARREL, 40, 0), thing(POSSESSED, 40, -80, -90, ambush)], px=-450, py=0, chase_at=1, chase_thing=0, tics=FIREBALL_TICS, seed=FIREBALL_SEED, **tough)
    # The barrel on the 24-unit pedestal dies at tic 10 and an Imp facing away from the
    # player (so it never sees it: P_LookForPlayers, p_enemy.c:536-551) is made to attack
    # at tic 5 (controlled S_TROO_ATK1 entry): its fireball (z 32..40) meets the dying
    # barrel (z 24..34.5), PIT_CheckThing returns !(MF_SOLID) for the non-shootable solid
    # thing, the missile explodes, no damage. (On the hall floor the 10.5-unit corpse is
    # below the fireball, which passes over it: the overhead return, p_map.c:294-295.)
    # Controlled kill (hit_*): a pistol or fist kill alerts the Imp (P_NoiseAlert,
    # p_pspr.c:256) and its own fireballs would then reach the live barrel first.
    add('fireball_on_dying_barrel', 6, [thing(BARREL, 150, 0), thing(IMP, 300, 0, 0, ambush)], px=-300, py=0, hit_at=10, hit_thing=0, hit_amount=20, attack_at=5, attack_thing=1, tics=60)
    # --- dense64 ----------------------------------------------------------------------
    add('dense_pickups', 7, dense_things(), px=-1100, py=64, tics=4, seed=0)
    # A 2,048-unit pistol trace east along the densest row (y = 64): every pickup and
    # decoration on it is an intercept (no flag filter), and so is the shooter's own box;
    # the count is observed (< 128).
    add('intercept_heavy_trace', 7, dense_things(), px=-1100, py=64, attack_start=20, attack_end=20, tics=26, seed=0)
    # --- angles -----------------------------------------------------------------------
    add('all_octants', 8, [thing(LAMP, -448 + 112 * (i % 9), -128 + 128 * (i // 9), a) for i, a in enumerate(ANGLES)], px=-448, py=200, tics=1)
    # --- embedded ---------------------------------------------------------------------
    # An Imp overlapping the west wall by 4 units (centre 16 from it), a floor lamp whose
    # box edge lies on the north wall and a health bonus overlapping the north wall by
    # 10: P_SpawnMapThing checks no position, all spawn; the Imp (chase) walks away from
    # the wall and never through it; the player walks north and collects the bonus.
    add('embedded_spawns', 9, [thing(IMP, -496, 0), thing(LAMP, 0, 240), thing(HEALTHBONUS, 200, 246)], px=200, py=120, forward=25, move_hold=30,
        chase_at=1, chase_thing=0, tics=80, **dict(tough, **north))
    return layouts
