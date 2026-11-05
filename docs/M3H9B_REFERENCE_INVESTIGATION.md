# M3h9b reference investigation: barrels, pickup scale and full thing angles

Phase 1 note for the second part of the E1M1 split
([M3h9 note §8](M3H9_REFERENCE_INVESTIGATION.md#8-size-estimate-and-split), decision
E6): the milestone after which E1M1 on medium skill passes load admission under a new
opt-in policy. It records the original C facts (function, file and line) for every
exploding-barrel path and for the intercept and pickup budgets, the admitted domain and
the budgets as formulas of the new bounds, the E1M1 census re-checked against the new
policy (counts only), the spawn-rule re-check that the M3h9a note left unmeasured, the
evidence and implementation plans, a size estimate and the new owner decisions with
recommendations. It follows the conventions of the
[M3h9 note](M3H9_REFERENCE_INVESTIGATION.md).

Source authority is `linuxdoom-1.10/` at the repository's upstream commit; line numbers
are of those files. Milestone status lives in [ROADMAP](ROADMAP.md), support in
[COMPATIBILITY](COMPATIBILITY.md), schemas in [STATE_FORMAT](STATE_FORMAT.md), commands in
[TESTING](TESTING.md), observations in [validation/M3H9B.md](validation/M3H9B.md) and the
census tables in [validation/M3H9_IWAD_COUNTS.md](validation/M3H9_IWAD_COUNTS.md).

## 1. Starting point

Master contains M3h9a (`PlayerMonstersE1M1V1`, wire 20, DSIM36, context
`PLAYER-MONSTERS-E1M1-1-MEDIUM-v1`, PR #54). Its [§12](M3H9_REFERENCE_INVESTIGATION.md#12-follow-ups-risks-status)
lists this part's work: barrels (the §2.8 bodies, the link-order RNG dependence,
`P_RadiusAttack`'s wrapping shift as a declared source quirk), pickups ≤ 64,
policy-specific actor slots (384), the hitscan intercept argument with things per cell,
the `N` re-evaluation and the `P_ChangeSector` measurement, decision E9 (every THING
angle) and, as maintenance, the docs-only CI fast path. The owner confirmed E1–E9 on
2026-11-04 and decided (section 4 of the brief) that older policies stay frozen in
admission, behavior, digests and DSIM bytes, that fidelity gaps stay refuse-the-tic
witnesses with state preserved, and that barrels are emulated in full ("inert, refuse
when damaged" is rejected because it would refuse ordinary play).

Under wire 20, E1M1 on medium does not load for four reasons, three of them named by the
M3h9a note and one measured here for the first time (§3):

1. 6 barrels (`MapSpawnError::UnsupportedType`);
2. 52 pickups against 16 (`MapSpawnError::WorkLimit`);
3. 45 of 91 THINGs with angle octant 4..7 (`MapSpawnError::Angle`);
4. **16 THINGs whose closed box touches a one-sided wall** (`MonsterError::Arithmetic`,
   the inherited M3h5 spawn rule), newly counted in §3.4: six floor lamps and
   candelabras whose box edge lies exactly on a wall, nine health and armor bonuses whose
   box overlaps a wall, and one Imp overlapping a wall by at most 4 units.

The first three are the subject of the brief. The fourth was listed as "unmeasured" by
M3h8 §13 and M3h9 §10 and planned for M3h9c; it blocks loading, so it is resolved here
(decision F3), faithfully to `P_SpawnMapThing`, which checks no position.

## 2. Original C facts

Every citation is `function (file:lines)`; "draw" means one `P_Random()` call
(m_random.c:57–61). `M_Random` is a separate stream used only for sound pitch
(s_sound.c:330, 340) and is never on a gameplay path here.

### 2.1 `MT_BARREL` and its states (info.c)

`mobjinfo[MT_BARREL]` (info.c:1888–1912): `doomednum` 2035 (1889), `spawnstate` `S_BAR1`
(1890), `spawnhealth` 20 (1891), `seestate` `S_NULL` (1892), `reactiontime` 8 (1894),
`painstate` `S_NULL` (1896), `painchance` 0 (1897), `deathstate` `S_BEXP` (1901),
`xdeathstate` `S_NULL` (1902), `deathsound` `sfx_barexp` (1903), `speed` 0 (1904),
`radius` 10 (1905), `height` 42 (1906), `mass` 100 (1907), `damage` 0 (1908), `flags`
`MF_SOLID|MF_SHOOTABLE|MF_NOBLOOD` (1910; p_mobj.h:122, 124, 170). It is not
`MF_COUNTKILL`, `MF_DROPOFF`, `MF_PICKUP`, `MF_SPECIAL` or `MF_FLOAT`.

| State (info.c) | Tics | Action | Next |
| --- | --- | --- | --- |
| `S_BAR1` (942) | 6 | none | `S_BAR2` |
| `S_BAR2` (943) | 6 | none | `S_BAR1` |
| `S_BEXP` (944) | 5 | none | `S_BEXP2` |
| `S_BEXP2` (945) | 5 | `A_Scream` | `S_BEXP3` |
| `S_BEXP3` (946) | 5 | none | `S_BEXP4` |
| `S_BEXP4` (947) | 10 | `A_Explode` | `S_BEXP5` |
| `S_BEXP5` (948) | 10 | none | `S_NULL` |

The live barrel alternates `S_BAR1`/`S_BAR2` forever with no draw; its tics are never −1,
so the nightmare-respawn branch (p_mobj.c:450–471) is never reached. Spawn
(`P_SpawnMapThing`, p_mobj.c:708–797): the bit-16 and skill filters (740–751), the type
scan (754–761), `P_SpawnMobj` with its `lastlook` draw (784; p_mobj.c:506), the tics
draw `1 + (P_Random () % 6)` (787–788), no counter change (789–792), the angle (794) and
`MF_AMBUSH` (795–796). The spawn-state action is not run (p_mobj.c:507–514);
`P_SetThingPosition` (517) and `P_AddThinker` (531, tail insertion, p_tick.c:65–71)
follow, so a barrel is a thinker in lump order like every THING.

### 2.2 Damage paths into a barrel

**Hitscan.** `P_AimLineAttack` (p_map.c:1023–1054) and `PTR_AimTraverse` (816–893):
a thing is an autoaim target iff it is `MF_SHOOTABLE` (867–868), so **a live barrel is
an autoaim target** (`linetarget = th`, 890; the trace stops, 892) and a dying one is
not. `PTR_ShootTraverse` (899–1016), thing branch: skip the shooter (975–977), skip
non-shootables (979–980: a dying barrel lets bullets through), the slope window
(984–992), `frac = in->frac - FixedDiv (10*FRACUNIT, attackrange)` (997), then
**`if (in->d.thing->flags & MF_NOBLOOD) P_SpawnPuff (x,y,z); else P_SpawnBlood (...)`**
(1005–1008) and `P_DamageMobj (th, shootthing, shootthing, la_damage)` (1010–1011);
`return false` (1014). `P_SpawnPuff` (p_mobj.c:812–831) draws four times: `z +=
((P_Random()-P_Random())<<10)` (819), `lastlook` through `P_SpawnMobj` (821; 506),
`tics -= P_Random()&3` (823, clamped to 1 at 825–826), and enters `S_PUFF3` only when
`attackrange == MELEERANGE` (829–830: the fist; the saw uses `MELEERANGE+1`,
p_pspr.c:515–516). Call sites: `P_GunShot` damage `5*(P_Random ()%3+1)` (p_pspr.c:633),
spread when `!accurate` (636–637), `P_LineAttack` (639); `A_Punch` damage
`(P_Random ()%10+1)<<1` (476), berserk ×10 (478–479), spread (482), aim and attack
(483–484), and the turn toward `linetarget` (487–494, a barrel included); monster
hitscans (`A_PosAttack`, p_enemy.c:802–819; `A_SPosAttack`, `A_CPosAttack`) aim and
shoot the first shootable thing on the line, barrels included. **One bullet or pellet
(5, 10 or 15) never kills a full-health barrel (20); a fist (2..20) can.**

**Missile.** `PIT_CheckThing` (p_map.c:252–343) with `tmthing` a missile: the flag test
(258–259), the box test (261–268), the height tests (294–297), the same-species test on
`tmthing->target->type` (299–314; a barrel launches nothing), then
**`if (! (thing->flags & MF_SHOOTABLE) ) return !(thing->flags & MF_SOLID);`**
(316–320): a dying barrel is solid and not shootable, so a fireball **explodes on it
and does no damage**; otherwise the draw `damage = ((P_Random()%8)+1)*tmthing->info->damage`
(323) and `P_DamageMobj (thing, tmthing, tmthing->target, damage)` (324), `return false`
(327). `P_ExplodeMissile` (p_mobj.c:90–105) follows from `P_XYMovement` (173–187).

**Explosion.** `A_Explode` (p_enemy.c:1598–1601) is exactly
`P_RadiusAttack ( thingy, thingy->target, 128 );` (1600); the only other caller is
`A_VileAttack` (1337, not admitted). `P_RadiusAttack` (p_map.c:1206–1233):

```c
1221    dist = (damage+MAXRADIUS)<<FRACBITS;
1222    yh = (spot->y + dist - bmaporgy)>>MAPBLOCKSHIFT;
1223    yl = (spot->y - dist - bmaporgy)>>MAPBLOCKSHIFT;
1224    xh = (spot->x + dist - bmaporgx)>>MAPBLOCKSHIFT;
1225    xl = (spot->x - dist - bmaporgx)>>MAPBLOCKSHIFT;
1226    bombspot = spot;
1227    bombsource = source;
1228    bombdamage = damage;
1230    for (y=yl ; y<=yh ; y++)
1231        for (x=xl ; x<=xh ; x++)
1232            P_BlockThingsIterator (x, y, PIT_RadiusAttack );
```

`#define MAXRADIUS 32*FRACUNIT` (p_local.h:51) has no parentheses, so line 1221 is
`(128 + 32*65536) << 16`: `0x00200080 << 16` is `0x20_0080_0000`, whose low 32 bits are
**`0x00800000` = `128*FRACUNIT`**. The `MAXRADIUS` term vanishes (`32 << 32` is 0 mod
2³²) and `dist` is exactly `damage << FRACBITS`: the block window is spot ± 128 units,
which is **always exactly 3 × 3 blocks** around the spot's block
(`⌊(v+128)/128⌋ − ⌊(v−128)/128⌋ = 2` for every `v`; `MAPBLOCKSHIFT` is `FRACBITS+7`,
p_local.h:40). A thing linked in a fourth block is never visited even when its
Chebyshev distance minus its radius is below 128: the intended 32-unit margin is lost.
Signed overflow is undefined in ISO C; the value above is the two's-complement wrap the
shipped executable computes, and it is reproduced as a declared quirk (decision F5,
like E9's angle product). The loop is **`y` outer, `x` inner** (1230–1231), unlike
`P_CheckPosition` (`bx` outer, p_map.c:425–426, 436–437) and `P_ChangeSector` (`x`
outer, 1332–1333). `P_BlockThingsIterator` (p_maputl.c:513–537) skips blocks off the
grid (520–526) and walks `blocklinks[y*bmapwidth+x]` by `bnext` (529–535); the return
value is ignored and `PIT_RadiusAttack` always returns true. `P_SetThingPosition`
**prepends** (p_maputl.c:436–442), so the walk is **newest-linked first**; every
successful `P_TryMove` unlinks and relinks (p_maputl.c:347–386, 396; p_map.c:488, 497),
moving the thing to the head of its block, and `MF_NOBLOCKMAP` things (puffs) are never
linked (425). This is the link-order dependence of §5.2.

`PIT_RadiusAttack` (p_map.c:1164–1198):

```c
1170    if (!(thing->flags & MF_SHOOTABLE) ) return true;
1175    if (thing->type == MT_CYBORG || thing->type == MT_SPIDER) return true;
1179    dx = abs(thing->x - bombspot->x);
1180    dy = abs(thing->y - bombspot->y);
1182    dist = dx>dy ? dx : dy;
1183    dist = (dist - thing->radius) >> FRACBITS;
1185    if (dist < 0) dist = 0;
1188    if (dist >= bombdamage) return true;
1191    if ( P_CheckSight (thing, bombspot) )
1194        P_DamageMobj (thing, bombspot, bombsource, bombdamage - dist);
1197    return true;
```

The distance is 2-D Chebyshev minus the victim's radius, floored by the arithmetic
shift; there is no z test except through `P_CheckSight`; damage is `128 − dist` in
1..128; **there is no self-exclusion for `bombsource`** (the shooter is hurt like anyone
else); the exploding barrel itself is skipped only because `P_KillMobj` cleared its
`MF_SHOOTABLE` (p_inter.c:675). `P_CheckSight (thing, bombspot)` (p_sight.c:300–347) has
the victim as looker: REJECT by both sectors (313–326), `validcount++` (332),
`sightzstart = t1->z + t1->height - (t1->height>>2)` (334), `topslope = (t2->z+t2->height)
- sightzstart; bottomslope = (t2->z) - sightzstart` (335–336), then the BSP walk
(338–346). Since `P_KillMobj` did `height >>= 2` (p_inter.c:681), **the bomb spot's
window is the dead barrel's bottom 10.5 units**, not 42; a step taller than that between
the victim's eye and the barrel's base blocks the damage. p_sight.c has no `P_Random`.

**Melee.** Monster melee reads `actor->target` only (`A_TroopAttack`, p_enemy.c:913–932;
`P_CheckMeleeRange`, 174–192) and no monster ever targets a barrel: `P_LookForPlayers`
(499–558) iterates `players[]`, `A_Look` reads `soundtarget` (609), and `P_DamageMobj`'s
retarget uses `source`, which is a shooter, a missile's `target` or `bombsource` and
never a barrel (`A_Explode` passes the barrel as inflictor only). The MAP30 stomp
(`PIT_StompThing`, p_map.c:102–105) is the one path that could make a barrel a source
and is out of scope (teleporters).

### 2.3 `P_DamageMobj` (p_inter.c:775–917) for barrels and explosion victims

1. `if ( !(target->flags & MF_SHOOTABLE) ) return;` (787–788); `if (target->health <= 0)
   return;` (790–791); `MF_SKULLFLY` momentum reset (793–796); baby halving for players
   only (798–800).
2. **Thrust** (806–832): condition `inflictor && !(target->flags & MF_NOCLIP) && (!source
   || !source->player || source->player->readyweapon != wp_chainsaw)` (806–810), so an
   explosion whose `bombsource` holds the chainsaw pushes nobody (unreachable: the saw is
   not admitted). `ang = R_PointToAngle2 (inflictor->x, inflictor->y, target->x,
   target->y)` (812–815); **`thrust = damage*(FRACUNIT>>3)*100/target->info->mass`**
   (817: `int` arithmetic, `damage*8192` for mass 100, so 128 damage is 16 units per tic);
   the fall-forward branch **`if ( damage < 40 && damage > target->health && target->z -
   inflictor->z > 64*FRACUNIT && (P_Random ()&1) )`** (820–827) draws only when the three
   conditions before it hold (a lethal hit under 40 against a target more than 64 units
   above the inflictor; `damage == health` kills without drawing); `ang >>=
   ANGLETOFINESHIFT` and `momx/momy += FixedMul (thrust, finecosine/finesine[ang])`
   (829–831). The thrust is applied **before** the death test, so a killed barrel or
   monster keeps the momentum. Inflictor and source by path: explosion `bombspot` /
   `bombsource`; hitscan shooter / shooter; missile the missile / its `target`;
   crusher NULL / NULL (no thrust, p_map.c:1300).
3. **Player branch** (835–884): the sector-11 hack (838–842), god mode and
   invulnerability returning after the thrust (847–852), armor absorption (854–869),
   `player->health -= damage` floored at 0 (870–872), **`player->attacker = source`**
   (874: NULL for a barrel nobody wounded), `damagecount` (875–878), `I_Tactile`
   (880–883, no gameplay effect).
4. **`target->health -= damage; if (target->health <= 0) { P_KillMobj (source, target);
   return; }`** (887–892).
5. **Pain draw** `if ( (P_Random () < target->info->painchance) && !(target->flags&MF_SKULLFLY) )`
   (894–900): `P_Random()` is the first operand, so **every surviving target draws once,
   a barrel with `painchance` 0 included** (the test is then always false and the barrel
   enters no state). The player's `painchance` is 255 (info.c:1117), so pain follows
   unless the draw is 255 (`S_PLAY_PAIN`, info.c:292).
6. `target->reactiontime = 0` (902).
7. **Retarget** (904–915): `if ( (!target->threshold || target->type == MT_VILE) &&
   source && source != target && source->type != MT_VILE) { target->target = source;
   target->threshold = BASETHRESHOLD; if (target->state == &states[target->info->spawnstate]
   && target->info->seestate != S_NULL) P_SetMobjState (target, target->info->seestate); }`;
   `BASETHRESHOLD` is 100 (p_local.h:61). **This runs for a barrel too**: a surviving
   barrel hit with a non-NULL source other than itself gets `target = source`,
   `threshold = 100` and no state change (`seestate` `S_NULL`). Nothing ever lowers a
   barrel's threshold (the only decrements are in `A_Chase`, p_enemy.c:681–690, and the
   reset in `A_Look`, 608), so **a barrel's `target` is its first non-lethal wounder with
   a non-NULL source, for the rest of its life**, and that is the `bombsource` of its
   explosion: kill credit, frags, `player->attacker` and the victims' retargets come from
   the first wounder, not the killer. A barrel killed in one blow by an explosion keeps
   its old target, usually NULL. For a monster in exactly its spawn state with threshold
   0, the `seestate` entry **runs `A_Chase` at once inside `P_DamageMobj`**, with its own
   draws and a possible `P_Move`, which unlinks and relinks the monster in the middle of
   the explosion's block walk (§5.2).

### 2.4 `P_KillMobj` (p_inter.c:668–758)

`flags &= ~(MF_SHOOTABLE|MF_FLOAT|MF_SKULLFLY)` (675); `MF_NOGRAVITY` cleared unless
`MT_SKULL` (677–678); **`flags |= MF_CORPSE|MF_DROPOFF`** (680); **`height >>= 2`** (681:
42 → 10.5 units). Counting (683–697): `if (source && source->player)` credits
`killcount` for `MF_COUNTKILL` and `frags` for a player target (683–691); **`else if
(!netgame && (target->flags & MF_COUNTKILL)) players[0].killcount++`** (692–697). So a
barrel's death changes no counter; a monster killed by an explosion credits
`bombsource->player` when the bomb source is a player, and `players[0]` when it is NULL
**or a monster** (`source->player` is NULL, the else-if applies). Player target
(699–717): `frags[self]++` when `!source` (702–703), `MF_SOLID` cleared **for players
only** (705), `PST_DEAD` (706), `P_DropWeapon` (707). Death state (719–725): `xdeathstate`
when `health < -spawnhealth` and the type has one (the barrel's is `S_NULL`, so always
`S_BEXP`; a player is gibbed only when post-armor `health − damage < −100`, p_inter.c:719).
**Draw** `target->tics -= P_Random()&3; if (target->tics < 1) target->tics = 1;`
(726–729): the barrel's `S_BEXP` tics become 5 − (r & 3) ∈ {2..5}. Drops (737–757):
`MT_POSSESSED`/`MT_WOLFSS` → `MT_CLIP`, `MT_SHOTGUY` → `MT_SHOTGUN`, `MT_CHAINGUY` →
`MT_CHAINGUN`, default return; the drop's `P_SpawnMobj` draws `lastlook` (756; 506).
Nothing here clears `MF_SOLID` for a non-player: **a dying barrel stays solid** until
`S_NULL` removes it, blocking and being blocked by movers and absorbing fireballs
(§2.2) while bullets and autoaim pass through it.

### 2.5 The death chain and its timing

`P_SetMobjState` (p_mobj.c:54–84): `S_NULL` → `mobj->state = NULL; P_RemoveMobj; return
false` (62–66); otherwise state, tics, sprite, frame (70–73), **the action on entry**
(77–78), and 0-tic states chain (81). `P_MobjThinker` (p_mobj.c:415–473): XY movement
only `if (momx || momy || MF_SKULLFLY)` (418–427), Z movement only `if (z != floorz ||
momz)` (428–436), then **`if (tics != -1) { tics--; if (!tics) if (!P_SetMobjState (mobj,
mobj->state->nextstate)) return; }`** (441–449). So **`A_Explode` runs inside the
barrel's own thinker, at its position in thinker order**, in the tic its `S_BEXP3` tics
reach 0, after its own movement that tic. `P_RunThinkers` (p_tick.c:101–122) walks from
the head and frees removed thinkers lazily (108–114); `P_RemoveThinker` only marks
(80–84); `P_Ticker` runs `P_PlayerThink` for every player before `P_RunThinkers`
(130–158), so player weapon actions precede every mobj thinker.

**Timing.** A barrel killed during tic `T` gets `t0 ∈ {2..5}` `S_BEXP` tics (§2.4). Let
`d = 0` if the barrel's thinker runs later in tic `T` than the killing code (always for
a player hitscan; for a kill by another barrel's `A_Explode` or by a missile, when the
victim follows the killer in thinker order) and `d = 1` otherwise. Then `S_BEXP2`
(`A_Scream`) is entered at `T + d + t0 − 1`, `S_BEXP3` at `T + d + t0 + 4`, `S_BEXP4`
(**`A_Explode`**) at `T + d + t0 + 9` ∈ `T + d + 11..14`, `S_BEXP5` at `T + d + t0 + 19`
and `S_NULL` (removal) at `T + d + t0 + 29`.

`A_Scream` (p_enemy.c:1535–1569): `sfx_podth*` and `sfx_bgdth*` draw (1544–1553); every
other death sound takes the default (1555–1557), so **the barrel's `sfx_barexp` draws
nothing**; `S_StartSound (actor, sound)` (1561–1568). `P_RemoveMobj` (p_mobj.c:546–570):
the respawn queue only for `MF_SPECIAL` non-dropped items (548–560; never a barrel),
`P_UnsetThingPosition` (563), `S_StopSound` (566), `P_RemoveThinker` (569).

### 2.6 A barrel as a moving thing

A barrel's momentum comes only from the thrust of §2.3 (speed 0, no AI).
`P_XYMovement` (p_mobj.c:114–241): `player = mo->player` is NULL (135); clamp to
±`MAXMOVE` (30, p_local.h:54; 137–145); the split loop **`if (xmove > MAXMOVE/2 || ymove
> MAXMOVE/2)`** halves only positive moves over 15 (150–164: a negative move of −30 is one
step); `P_TryMove` failure for a non-player non-missile is **`mo->momx = mo->momy = 0`**
(188–189: a blocked barrel stops dead); friction: `MF_MISSILE|MF_SKULLFLY` return
(201–202), airborne return (204–205), **the corpse rule** `if (mo->flags & MF_CORPSE) {
if (|momx| > FRACUNIT/4 || |momy| > FRACUNIT/4) if (mo->floorz != mo->subsector->sector->floorheight)
return; }` (207–219, which applies to a dying barrel since `MF_CORPSE` is set), the
`STOPSPEED` stop with `!player` true (221–235), else `FRICTION` (236–240). `P_TryMove`
(p_map.c:451–517): the height tests (468–479, with the current height, so a dying
barrel's 10.5 fits lower openings), **the dropoff test `!(thing->flags&(MF_DROPOFF|MF_FLOAT))
&& tmfloorz - tmdropoffz > 24*FRACUNIT`** (481–483: a live barrel cannot be pushed over a
drop of more than 24; a dying one can, since `P_KillMobj` set `MF_DROPOFF`), relink
(488–497) and **the special-line crossing loop** (500–514): `P_CrossSpecialLine`
(p_spec.c:492–534) excludes only missiles for non-players (503–518) and admits 39, 97,
125, 126 (teleports), 4 (raise door), **10 and 88 (plats)** (520–534), so **a sliding
barrel crossing an 88 line lowers the lift**, an E1M1 line type (`PlayerMonstersMoversV1`
already runs this crossing for monsters). `spechit[MAXSPECIALCROSS]` (8, p_map.c:67–69)
is filled without a bounds check (242–243); the window rule of M3h8 §6 covers it since
the barrel's radius 10 is below 64. `PIT_CheckThing` with `tmthing` a moving barrel: not
`MF_SKULLFLY`, not `MF_MISSILE`; `MF_SPECIAL` things are passed through without pickup
since the barrel lacks `MF_PICKUP` (331–340); `return !(thing->flags & MF_SOLID)` (342)
blocks on every solid thing, dying barrels and monsters before `A_Fall` included.
`P_ZMovement` (p_mobj.c:246–349) gives a dying barrel pushed over a ledge gravity
(`-GRAVITY*2` first, then `-GRAVITY`, 320–326) and the floor landing (284–311); an
airborne thing skips friction (204–205).

**Under movers.** `PIT_ChangeSector` (p_map.c:1257–1313) with a barrel: the height clip
(1261–1265); **`if (thing->health <= 0) { P_SetMobjState (thing, S_GIBS); flags &=
~MF_SOLID; height = radius = 0; return true; }`** (1269–1279), so **a dying barrel that no
longer fits under a closing door becomes `S_GIBS` (tics −1), never explodes and is never
removed** (reachable in E1M1: a wounded-to-death barrel under a DR door); `MF_DROPPED`
removal (1282–1288); non-shootables return (1290–1294); `nofit = true` (1296) for a live
barrel, which reverses a non-crushing door like a monster would; the crushing branch
(1298–1309) damages with inflictor NULL and sprays `MT_BLOOD` **despite `MF_NOBLOOD`**,
unreachable here since every crusher line is rejected at load. `T_MovePlane` has no retry
loop: it sets the height, calls `P_ChangeSector`, and on failure restores and calls it
again (p_floor.c:68–79, 83–91, 97–108, 113–123).

### 2.7 The player hit by a barrel

`P_DamageMobj` with inflictor the barrel and source its `target` (§2.3): thrust away
from the barrel, armor, `attacker = source`, pain unless the draw is 255. Death:
`P_KillMobj (source, player)` with `frags[self]++` when the source is NULL (702–703) or
`source->player->frags[self]++` when the player had wounded the barrel (690),
`PST_DEAD`, `P_DropWeapon`, `S_PLAY_DIE1` or `S_PLAY_XDIE1` (719–725), the tics draw.
`P_DeathThink` (p_user.c:182–229) turns the view toward `attacker` only `if
(player->attacker && player->attacker != player->mo)` (200–221): an anonymous barrel
(NULL) or the player's own wounded barrel gives no turn. The reborn request stays a
refused tic (M3h8).

### 2.8 Intercepts, pickups and iteration budgets

`MAXINTERCEPTS` 128 (p_local.h:154); `intercepts[MAXINTERCEPTS]`, `intercept_p`
(p_maputl.c:544–545). `P_PathTraverse` (743–880): `validcount++` (774), `intercept_p =
intercepts` (775), the block-edge nudge (777–781), the step loop **`for (count = 0 ;
count < 64 ; count++)`** (846) adding lines then things per block (848–858), stepping on
whichever axis matches (866–875; when neither matches the same block is re-added, and
duplicate things are not filtered). `PIT_AddLineIntercepts` (562–609) and
`PIT_AddThingIntercepts` (616–674) write **`intercept_p++` with no bounds check**
(603–606, 668–671) and **with no flag filter**: every block-linked thing whose diagonal
crosses the trace is added (items, corpses, dying barrels); the traverser filters later.
**More pickups and barrels raise the intercept count of every hitscan and autoaim
trace** (§5.3). `P_TraverseIntercepts` (683–730) selects the smallest `frac` with a
strict `<` (earliest-added wins a tie, 696–706), `if (dist > maxfrac) return true`
(708–709), calls the traverser (723–724) and marks `in->frac = MAXINT` (726).
`P_BlockLinesIterator` (472–506) dedups by `validcount` and offers the block list's
leading word (linedef 0) from every block. `P_TouchSpecialThing` (p_inter.c:339–) has no
array and no budget. `P_ChangeSector` (p_map.c:1321–1338) iterates the sector's
`blockbox` `x` outer, `y` inner (1332–1334).

### 2.9 THING angles (E9)

`mobj->angle = ANG45 * (mthing->angle/45);` (p_mobj.c:794), `ANG45` `0x20000000`
(tables.h:68), `angle_t` `unsigned` (tables.h:78), `mapthing_t.angle` `short`
(doomdata.h:207, byte-swapped at p_setup.c:342). The quotient truncates toward zero;
the `int` product overflows for quotients outside −4..3 and wraps on the original
targets to `q × 0x20000000 mod 2³²`, which is Rust's
`(q as u32).wrapping_mul(0x2000_0000)` for every `short` (M3h9 note E9). Nightmare
respawn uses the same formula (p_mobj.c:400; out of scope).

### 2.10 RNG draw sequences on the barrel paths

Complete `P_Random` sites: p_inter.c:726, 823, 894; p_map.c:277, 323, 1307, 1308;
p_mobj.c:96, 467, 506, 788, 819 (×2), 823, 847 (×2), 850, 870, 910. No draw occurs in
p_sight.c, p_maputl.c, `P_RadiusAttack`, `PIT_RadiusAttack`, `A_Explode`, `A_Scream` for
`sfx_barexp`, `P_SetMobjState`, `P_RemoveMobj`, `P_TryMove`, `P_XYMovement`,
`P_DeathThink` or `P_DropWeapon`. In order:

- **Pistol shot wounding a barrel**: damage (p_pspr.c:633), spread 0 or 2 (637), puff 4
  (p_mobj.c:819 ×2, 506, 823), pain 1 (p_inter.c:894); the retarget sets
  `barrel->target = player`. Total 6 or 8.
- **Pistol shot killing an already wounded barrel**: damage, spread 0 or 2, puff 4,
  fall-forward 0 or 1 (823; only with `damage > health`, `damage < 40` and the barrel
  more than 64 units above the player), `P_KillMobj` tics 1 (726). No pain draw (the
  function returned at 891). Total 6 to 9.
- **One explosion**, per victim in walk order (`y` outer, `x` inner blocks, newest-linked
  first within a block; no draw in the candidate tests): a surviving player: pain 1 (the
  fall-forward draw needs `damage > health`, so a survivor never draws it; a god-mode
  player returns before it); a dying player: fall-forward 0 or 1, tics 1; a surviving
  monster: pain 1, then, if the pain test failed and it stands in its spawn state with
  threshold 0 and a non-NULL source other than itself, `A_Chase` at once with its own
  draws; a dying monster: fall-forward 0 or 1, tics 1, the drop's `lastlook` 1 for the
  former humans; a surviving barrel: pain 1 (and the retarget when the source is
  non-NULL); a dying barrel: fall-forward 0 or 1, tics 1. The fall-forward test compares
  the victim's `z` with the exploding barrel's.
- **Barrel spawn**: `lastlook`, then tics (p_mobj.c:506, 788).

### 2.11 Skill reads (for decision F11)

`grep -n gameskill linuxdoom-1.10/*.c`: `P_SpawnMobj` reaction time (p_mobj.c:503),
`P_SpawnMapThing` bit (743–748), **`A_Chase`** (p_enemy.c:719, `P_NewChaseDir` after
`MF_JUSTATTACKED` except nightmare or `-fast`; 738, missile gating below nightmare),
`A_BrainSpit` (1909, Doom II), `P_GiveAmmo` doubling (p_inter.c:95–96), `P_DamageMobj`
halving (799), and g_game.c/st_stuff.c bookkeeping (99, 484, 1220, 1227, 1293, 1421,
1429, 1450, 1556; st_stuff.c:546, 721). The M3h9 note's list missed the two `A_Chase`
reads; both behave identically on easy, medium and hard and are already in the admitted
bodies. Respawn comes from `skill`/`respawnparm` (g_game.c:1416–1419).

### 2.12 Corrections to assumptions of the brief and the M3h9 note

The scan window of `P_RadiusAttack` is ±128 units (M3h9 §2.8 said so) and that is
exactly 3 × 3 blocks, not "a box that may touch a fourth block"; the barrel's `target`
is fixed by the first non-lethal wounder, not the killer; the pain draw is
unconditional for every survivor; the fall-forward draw is conditional; dying barrels
stay solid; the explosion's sight window is 10.5 units; `P_XYMovement` halves only
positive moves; a sliding barrel can trigger E1M1's lift lines; there is no
`P_TraverseIntercepts` "bad scan" diagnostic and no `while (!crushchange)` retry loop in
1.10.

## 3. E1M1 census re-checked against the new policy (counts only)

All numbers come from `tools/iwad_counts.py` over the owner's IWADs at the local read-only
directory (never committed, never read by CI; the tool is a no-op without the directory).
The `--things` and `--things-markdown` tables were re-run and are byte-identical to
[validation/M3H9_IWAD_COUNTS.md](validation/M3H9_IWAD_COUNTS.md) (182 table lines). Three
counts-only modes were added for this note (`--spawn-rule [SLOT]`,
`--spawn-rule-markdown`, `--things-budget [SLOT]`, `--spawn-fit [SLOT]`); their E1M1 and
all-slot output is recorded in the [evidence record](validation/M3H9B.md#iwad-counts). No
coordinate, angle, lump byte or per-object datum is printed by any mode.

### 3.1 Roster on medium against the proposed bounds (§5)

| Kind | E1M1 medium | Bound today (wire 20) | Bound proposed (wire 21) |
| --- | --- | --- | --- |
| Monsters | 6 (zombieman 4, imp 2) | 8 | 8 (unchanged) |
| Pickups | 52 | 16 | **64** |
| Decorations | 27 (12 solid, 15 flags 0) | 32 | 32 (unchanged) |
| Barrels | 6 (of 9 records; 3 carry bit 16) | rejected | **16** |
| Spawned THINGs | 91 (+ the player body) | | |
| THINGs with angle octant 4..7 | 45 | rejected | **admitted** (E9) |

The 52 pickups are shotgun 1, clip 2, shells 2, stimpack 1, medikit 3, health bonus 12,
armor bonus 25, green armor 1, blue armor 1, box of bullets 1, box of shells 3: every kind
is already admitted by `world_pickups::kind_for`, so the pickup change is the bound alone.
Spawn draws on medium become 91 + 1 (player) + 6 barrels + 6 monsters + 2 armors + 37
bonuses = 143 tics draws, then the sector-1 light: `prndindex` 144 at tic 0 if every
THING spawns (unchanged from M3h9 §3, which already counted the barrels).

### 3.2 Spawn height fit (`--spawn-fit`)

The heights policies refuse a spawn taller than its sector (`spawn_fits`,
`MapScaleError::Fit`). The tool locates each spawned thing's sector by the exact
`R_PointInSubsector` walk over NODES. E1M1: **0 failures** on every skill (90/92/116
spawned things including the player body); the minimum height of a sector that contains a
spawned thing is 72, above the player's 56 and the barrel's 42. Over all 68 slots, 65 pass
at every skill; E2M1, E4M2 and MAP32 have 16 failures each (monster 4, pickup 10,
non-solid decoration 2).

### 3.3 Budget facts (`--things-budget E1M1`, medium)

| Fact | E1M1 |
| --- | --- |
| BLOCKMAP | 36 × 23 cells |
| Most lines listed in one cell (leading linedef-0 word included, as the slide window counts) | 13 |
| Distinct lines in any 2 × 2 window (the slide bound) | 28 |
| Most spawned things linked in one cell (by centre, player included) | 4 (easy 3) |
| Lines + things summed over a whole row / column / diagonal of cells (crude) | 150 / 100 / 79 |
| Lines + things over any 18 consecutive cells of a row, column or diagonal | **103** (hard 104) |

A hitscan or autoaim trace is at most `MISSILERANGE` = 2,048 units (p_local.h:58) plus the
sub-cell start, so it crosses at most 17 cell boundaries and visits at most 18 cells
(`P_PathTraverse` stops at the end block, p_maputl.c:860–864). The 18-cell figure counts
a line listed in `k` cells `k` times and every linked thing of the cells, so it is an
upper estimate of the intercepts of any single straight trace on E1M1: **103 < 128**.
The whole-row figure (150) exceeds 128 but no admitted trace spans a row. This is the
"things per cell re-derived" argument of §5.3; the witness stays (decision F7).

### 3.4 The spawn rule (`--spawn-rule E1M1`): the fourth blocker

The inherited M3h5 spawn rule (`Boundary::spawn_admitted`, player_map_scale.rs:548;
`Simulation::spawn_admitted` :820; `MonsterError::Arithmetic` at spawning.rs:495) applies
to **every spawned kind** with its own `mobjinfo` radius (player body 16 through
`player_spawn_guard`, dropped pickups too) and requires three things: the closed box
inside the closed VERTEXES rectangle (`RoomBounds::contains_box` :185), odd ray parity of
the centre against the one-sided walls (`inside` :520), and the closed box meeting **no
one-sided segment** (`box_meets_wall` :532, `segment_meets_box` :376: endpoint, edge and
corner contact all count). Two-sided lines and `ML_BLOCKING` are irrelevant. Measured on
E1M1 (player body included):

| Skill | Spawned | Violations | Edge contact only | Open-box overlap [movers, statics] | Overlap depth buckets 0 / 1–4 / 5–8 / 9+ |
| --- | --- | --- | --- | --- | --- |
| easy | 90 | 16 | 6 | 10 [1, 9] | 2 / 8 / 0 / 0 |
| medium | 92 | **16** | 6 | 10 [1, 9] | 2 / 8 / 0 / 0 |
| hard | 116 | 17 | 6 | 11 [2, 9] | 2 / 9 / 0 / 0 |

Every violation is a one-sided wall contact; parity, rectangle and BLOCKMAP-bounds
violations are 0. By type on medium: floor lamp (2028) 4 and candelabra (35) 2, all six
with the wall exactly on the box edge (the open box clear); health bonus (2014) 4 and
armor bonus (2015) 5 overlapping a wall; **one Imp (3001) overlapping a wall by at most
4 units** (hard adds one shotgun guy). The first violating record in lump order returns
`MonsterError::Arithmetic` and the batch rolls back: **E1M1 does not load at any skill
under the rule as written**, whatever the other three blockers do. Over the 68 slots only
4 pass it at medium (E3M8, MAP07, MAP21, MAP32); 1,447 of 13,923 spawned things violate
it, 972 by overlap (62 movers, 910 statics) and 443 by edge contact.

`P_SpawnMapThing` checks no position (M3h9a already recorded this when it widened the
`monster_hitscan_through_pillar` alcove). Decision F3 relaxes the rule under wire 21 to
exactly what the M3h5 proof needs, so that E1M1's 16 spawns are admitted faithfully.

## 4. Rust inventory

Verified `path:line` references of the working tree at `43460e2` (the M3h9 note §4
citations have drifted; the detailed inventory is kept with the evidence record's
scratch summary and the gaps below are what matters). `S/` is `crates/doom-sim/src/`.

- **Policies.** `OriginalExecutionProfile` (S/original_registry.rs:12–79, last variant
  `PlayerMonstersE1M1V1` :78), `wire_id` :82–105, `context_schema` :107–130 with the
  strings in S/player_monsters.rs:31–33. Eighteen `has_*` predicates :131–357, each a
  `matches!` list ending in the E1M1 variant; `has_movers` :257 = Movers | E1M1,
  **`has_e1m1` :266 = E1M1 only**. Exhaustive `new` :701 and `validate` :751. **The DSIM
  version comes from the predicate ladder** (S/snapshot.rs:715–747, `has_e1m1` → 36 first),
  so a new variant covered by `has_e1m1` would emit DSIM36 and decode as wire 20
  (S/diagnostic.rs:437): the new `has_barrels` must be tested before it and `has_e1m1`
  must include the new variant (as `has_movers` includes E1M1). Fourteen `..=36` ranges in
  snapshot.rs and about fifty in diagnostic.rs (`decode_e1m1` :254–256, `slot_bound_for`
  :2007–2013, `diagnostic_bytes` :2018, event validators :2324, :2366–2368, the per-event
  actor allow-list :738–768, `gibs_admitted` :430–435). Constructor
  `Simulation::original_player_e1m1_map` S/player_monsters.rs:197–209; `e1m1_rules.rs`
  holds the decoration list, `MAX_DECORATIONS` :54, `E1M1_LINKED_THINGS` :81–83 (65),
  `spawn_angle` :145–147 and the live decoration witness :164; the two-sided-48 rule is
  `validate_e1m1` in S/mover_rules.rs:263.
- **Spawning.** `spawn_map_things` S/spawning.rs:386–632: starts, the skill and bit-16
  filter :417–425, classification :436–508 with the barrel refusal
  **`UnsupportedType` :455**, **the angle refusal `!(-4..=3).contains(&(thing.angle / 45))`
  :457–459**, caps :460–486 (a barrel would be counted as a roster monster by the
  `!pickup && !decoration` expression), **the spawn rule :487–496** and the height fit
  :497–504, the owner rewrite :505–507 (must skip barrels), type validation :509–540,
  counter preflight :543–580 (`MAX_WORLD_PICKUPS` at :574–577). Atomicity is preflight
  plus a staged plan committed only on success (`checked_map_spawn_batch`
  S/demon_expanded_map.rs:594–627), not snapshot and restore. `spawn_placed_thing`
  :638–687 draws `lastlook` and the tics value and converts the angle at :682 with
  `((i32::from(thing.angle) / 45) as u32).wrapping_mul(0x20000000)`: **the wrap already
  matches C; only :457 refuses octants outside −4..3**.
- **Bounds.** `MAX_PLAYER_MONSTERS` 8 (S/player_monsters.rs:12); `MAX_WORLD_PICKUPS` 16
  (S/world_pickups.rs:7; used at spawning.rs:483, 574, 577, world_pickups.rs:181–183,
  808, 845, 893–895, diagnostic.rs:659, 840, 1562, 2469, 2488); `MAX_BONUS` = 6 × 16
  (world_pickups.rs:15; diagnostic.rs:837, 2467); `MAX_DECORATIONS` 32; `MAX_LINKED_THINGS`
  33 (player_map_scale.rs:48, private, one static assert :1088 for the 95-line policy);
  **`MAP_SCALE_ACTOR_SLOTS` 320** (player_map_scale.rs:12; passed by seven constructors,
  required as an equality at original_registry.rs:1007–1013, `slot_bound_for`
  diagnostic.rs:2007, collision work `4 × actor_cap` map_extent.rs:203–208, Python
  `MAX_SLOTS` map_scale_snapshot.py:10 inherited by every later encoder) and **encoded as
  `store.max_slots`** (snapshot.rs:813–816, checked at diagnostic.rs:562);
  `MAP_SCALE_MAX_LIVE_PROJECTILES` 208; the hitscan intercept cap is the literal 128 in
  S/combat_trace.rs:321–330 (`CombatError::InterceptLimit`, mapped to
  `MapScaleError::Intercepts` under real-map policies, demon_expanded_map.rs:660–667);
  the sight budget is **64 queries and `sight_work_budget` per tic**, one `SightControl`
  per tic shared by every action (demon_expanded_map.rs:306–343, `PerceptionError::WorkLimit`);
  `MAX_SPECIAL_CROSS` 8; visit bound `max_slots + sector slots` (store.rs:316). The
  arithmetic certificate has no actor term.
- **Combat bodies.** `LifeHost::apply_damage` S/combat.rs:828–1068 is `P_DamageMobj`
  with `P_KillMobj` inline: early return :216/850, thrust :855–886 (fall-forward draw
  :862–868), player branch :887–908, kill :917–1019 (flags, `height >>= 2`, the
  `players[0]` kill credit :930–940, player-only solidity and death, state entry and
  `tics -= r & 3` :976–981, drops :988–1017), **pain draw always** :1021, reaction time,
  retarget and seestate :1046–1064, where **`self.access.get(source).unwrap()` :1047
  panics on an unreadable source handle**. `DamagePlan::current_original` :364–460
  **refuses any target that is not the player or a policy monster (`MonsterError::Target`,
  :427–429)**, and `finish_with` :256–290, `require_role_for` (original_registry.rs:853),
  `state_role_for` (:896), `validate_type_original` (lifecycle.rs:881–1000),
  `preflight_map_demons` (demon_map_spawn.rs:73–141, which would treat a barrel as the
  suffix actor at :127) and `validate_monster_type` (monsters.rs:449) all gate on roles a
  barrel lacks. The hitscan thing hit already spawns **a puff for `MF_NOBLOOD`**
  (combat.rs:1088–1112, `S_PUFF3` at melee range); the missile contact already returns
  `!(flags & MF_SOLID)` for a non-shootable and damages a shootable one
  (projectiles.rs:423–430) through `strike` :510–522 → `current_original`.
  **`P_RadiusAttack`, `PIT_RadiusAttack` and `A_Explode` exist nowhere**: the compile-time
  action table (S/actions.rs:10–45) has no `Explode`, `action_supported`
  (lifecycle.rs:382–387) admits only Fall/Pain/Scream/XScream/PlayerScream plus the
  profile exceptions, so `S_BEXP4` is refused at admission with
  `ActorError::UnsupportedAction` (:414–453) and the dispatcher's `unreachable!` :1570 is
  never reached. `A_Scream` (:1552–1569) draws only for the `podth`/`bgdth` sounds, so
  the barrel's `sfx_barexp` draws nothing, as C. The pattern a radius attack reuses:
  actions run on a `LifeHost` inside the staged tic with mutable access to every actor;
  a body builds `DamagePlan::current_original`, `reserve`s the spawn demand, calls
  `apply_damage`, and latches any error with `fail_sight` so the whole tic is refused
  (S/player_monsters.rs:331–416).
- **Movement.** S/thinkers.rs:111–366 runs `move_xy_multisector` and `move_z` for **every
  actor** with momentum or off its floor, with the ±30 clamp, the positive-only halving
  :800–812, blocked non-command actors zeroing momentum :828–832, the corpse rule
  :839–855 and `STOPSPEED`/`FRICTION` :856–862 (movement.rs); `allow_dropoff` from
  `MF_DROPOFF` at spawn and set on kill (combat.rs:924), read at multisector.rs:469. So a
  thrust barrel needs no new movement code. The API refusals of pushes, vertical momentum
  and removal (S/lib.rs:1109, :1127–1139, :1178) key on `is_decoration` or `MF_SPECIAL`
  and let a barrel through. `planes.rs:254` gibs any `health <= 0` thing that no longer
  fits, which is C's behaviour for a dying barrel; the DSIM reader's `S_GIBS` rule must
  learn the barrel type.
- **Sight.** `sight::checked_query` S/sight.rs:282–293 takes two plain `Actor`s and reads
  `location.sector`, `z` and `height` as C (eye at ¾ height :320, slopes :321); nothing
  restricts the pair to monsters.
- **DSIM.** `life_fields` (snapshot.rs:1539–1577) already encodes type, state, tics,
  health, reaction time, sprite, frame, `target`, death request and spawnpoint;
  `threshold`, `movedir`/`movecount`, `player`, `momz`, subsector and sector follow. Event
  tags 12 (`ActorState`: Entered, Action, Random, Sound, Death, Target, ...) and 13
  (`Combat`: Attack, Trace, Damage, Random, Knockback, Health, Pain, Kill, DeathTimer,
  Reaction, Retarget, Impact, Ignored) cover every record an explosion produces; **no
  radius-attack tag exists**. The independent encoders chain `e1m1_snapshot.py` →
  `movers_snapshot.py` → … → `map_scale_snapshot.py` (`MAX_SLOTS` 320).
- **Evidence tooling.** `m3h9_reference.py` layers on `m3h8_reference.py` with
  `ADDED = []` genuine bodies and observer rewrites; the compiled contract has 151
  genuine functions including `P_DamageMobj`, `P_KillMobj`, `P_SpawnPuff`, `A_Scream`,
  `P_RemoveMobj`, `P_CheckSight`, `P_BlockThingsIterator` and `P_SetThingPosition`, **but
  not `P_RadiusAttack`, `PIT_RadiusAttack` or `A_Explode`**: every unlisted action is an
  `abort()` stub (generate_m3d3j.py:181), so a `S_BEXP4` entry aborts the C run today.
  The scenario `add()` of m3h9_cases.py:193 asserts every THING kind is a roster monster,
  pickup or decoration, and m3h8_cases.py:49 caps pickups at 16. Builds: strict
  (`-fwrapv`, generate.py:11–15), `--overflow-check` (`-fno-wrapv
  -fsanitize=signed-integer-overflow,integer-divide-by-zero,float-cast-overflow`) and
  `--address-check` (`address,undefined`).
- **Census tool.** `things_census` (tools/iwad_counts.py:1003–1109) already classifies
  barrels; the new `spawn_rule_census`, budget and fit modes are described in §3.
- **CI.** One workflow, jobs `Stable / ${{ matrix.os }}` (three) and `Minimum Rust /
  1.85.0`; no `paths`, `paths-ignore`, `needs` or job-level `if`; `check_docs.py` verifies
  relative links, anchors and the archive digests only.

### 4.1 Gaps the new policy must close

1. `original_registry.rs`: the variant, wire 21, the context, all eighteen predicate
   lists, `has_barrels` (and `has_e1m1` including the new variant), the two exhaustive
   matches, `require_role_for`/`state_role_for` for `MT_BARREL` and `S_BAR1..S_BEXP5`
   (never through `is_policy_monster`), the per-policy actor cap at :1007–1013.
2. `spawning.rs`: a barrel kind with its own cap, no owner rewrite, no `Angle` refusal
   under wire 21, the per-kind spawn rule S′ (F3), the pickup bound as a policy value.
3. `lifecycle.rs`/`actions.rs`: `OriginalAction::Explode`, admission of `A_Explode` for
   barrels only, a barrel branch in `validate_type_original` (flags need
   `MF_NOBLOOD` 0x80000, absent from `ALLOWED_FLAGS` :224).
4. `combat.rs`: the barrel as a damage target in `current_original`/`finish_with` (no
   kill credit, no drop, death-chain spawn demand), **the new radius-attack body**
   (§5.2), a checked source handle instead of the `unwrap` at :1047.
5. `demon_map_spawn.rs`/`monsters.rs`/`demon_lifecycle.rs`: the per-tic preflights must
   admit an animating, shootable, pushable non-monster.
6. Sight: a per-explosion allowance (F6).
7. `snapshot.rs`/`diagnostic.rs`: DSIM37, `decode_barrels`, `validate_barrels`, the
   barrel in the event actor allow-list and the type-keyed `S_GIBS` rule, `slot_bound_for`
   384, pickup and bonus ranges per version.
8. Tools: `m3h9b_*` generators compiling `P_RadiusAttack`, `PIT_RadiusAttack` and
   `A_Explode` as genuine bodies with observers, scenario assertions admitting 2035 and
   64 pickups, `barrels_snapshot.py` (DSIM37, `MAX_SLOTS` 384), the replay, example,
   check and measure scripts, a fuzz target and seeder.
9. CI: the detect-changes job and step-level skips.

## 5. Admitted domain of `PlayerMonstersBarrelsV1` and its budgets

### 5.1 The domain

A superset of `PlayerMonstersE1M1V1`: the same bodies, rooms, movers, specials,
scrollers, decorations, skill (medium, single player) and monster bound, plus:

1. **Exploding barrels** (`MT_BARREL`, 2035), at most `MAX_BARRELS` = 16 per map, spawned
   by `P_SpawnMapThing` with their info values (health 20, radius 10, height 42, mass 100,
   `MF_SOLID|MF_SHOOTABLE|MF_NOBLOOD`, `MF_AMBUSH` from the record), the `lastlook` and
   tics draws, `S_BAR1`/`S_BAR2` cycling, blocking like any solid thing; damaged by player
   and monster hitscans (puff, not blood), by fireballs and by explosions through the
   existing `P_DamageMobj` body (thrust, the unconditional pain draw, `target` fixed by
   the first non-lethal wounder); killed into the `S_BEXP` chain (`A_Scream` without a
   draw, `A_Explode` nine tics later) while staying solid and non-shootable, and removed
   at `S_NULL`; sliding under thrust with friction, the corpse rule, the dropoff rule and
   the walk-over crossings (an 88 line lowers the lift); gibbed by a closing door when
   dying; autoaim target while alive. `P_RadiusAttack` with the ±128-unit 3 × 3 block
   window (the wrapped shift, F5), `y` outer, `x` inner, newest-linked first, the live
   block-list walk, `PIT_RadiusAttack`'s tests, `P_CheckSight` from each candidate to the
   dead barrel (10.5 units high), and `P_DamageMobj` on each victim: player (armor,
   thrust, attacker, pain or death with the reborn tic refused as before), monsters
   (pain, death with drops and the `players[0]` kill credit when the bomb source is NULL
   or a monster, `source->player` credit when it is the player, infighting retarget to the
   bomb source) and other barrels (chain explosions with their own `S_BEXP` timers).
2. **Pickups up to 64**, bonuses up to `6 × 64`, drops 8 as before, with 384 actor slots
   under this policy only.
3. **Every THING angle** (E9), converted as `spawn_placed_thing` already does; the
   `Angle` refusal stays in every older policy.
4. **The spawn rule S′** (F3) in place of the closed-box rule, under this policy only.

Still rejected at load (checked errors, state unchanged): every other THING type
(`UnsupportedType`), a 17th barrel (`MapSpawnError::BarrelLimit`), a 65th pickup, a 9th
monster, a 33rd decoration, every M3h8 mover rule, crushers and the other deferred
specials, non-medium skills, netgame and nightmare options, and an embedded barrel or
player (S′). Still refused per tic with state preserved (fidelity-gap witnesses,
unchanged in kind): the 129th intercept, the sight budget, `SpecialCross`, `Vertical`,
the exit and reborn tics, plus one new witness: an explosion whose bomb source handle is
stale (F4). Out of scope: rockets and every other `A_Explode` caller, the Arch-vile's
`P_RadiusAttack`, crushers, teleporters, the MAP30 stomp, nightmare respawn, deathmatch.

### 5.2 Faithfulness arguments

- *Bodies.* Every per-victim body of an explosion exists in Rust (`apply_damage` with
  thrust, pain, kill, drops, credit, retarget and seestate entry); what is new is the walk
  (`P_RadiusAttack`/`PIT_RadiusAttack`), the barrel as a damage target, `A_Explode` as an
  action and the barrel role gates. The puff on a `MF_NOBLOOD` thing, the missile
  contact's `!(flags & MF_SOLID)` return, movement with friction and the corpse and
  dropoff rules, the `S_GIBS` crunch and the walk-over crossing are existing bodies.
- *Link order.* The walk reads the BLOCKMAP cell lists in link order (newest first) and
  re-reads each thing's next link after its callback, so a victim whose seestate `A_Chase`
  moves and relinks it mid-walk changes the walk exactly as in C (double visit or skip).
  The Rust spatial index keeps per-cell linked lists in link order ("never sort handles
  to select blockers"); the C harness observes every candidate with its outcome.
- *Timing.* `A_Explode` runs in the barrel's thinker when `S_BEXP3`'s tics reach 0; the
  shared thinker order and the lazy retirement of removed thinkers are the M3d3j bodies.
- *Draw order.* §2.10 lists the draws; the C harness pins them per scenario.
- *Target semantics.* The barrel's `target` is set by the existing retarget block and
  never cleared; a stale handle at explosion time is undefined in C and a witness here.
- *Spawn rule S′.* Proven in F3 from the M3h5 invariant: no admitted spawn can make the
  `Escape`/`Position` witnesses reachable, and `P_SpawnMapThing` checks no position.

### 5.3 Budgets as formulas (values of this policy)

Symbols: `K` monsters ≤ 8, `k` Imps ≤ `K`, `P` pickups ≤ **64**, `R` drops ≤ 8, `D`
decorations ≤ 32, `B` barrels ≤ **16**, `L`/`M` light and mover sectors, `Q` collision
work of one position check (E1M1 3,462), `S` = `sight_work + 5·lines` (one query's work).

| Budget | Formula | Value here | Store or certificate |
| --- | --- | --- | --- |
| Linked things `N` | `1 + K + P + R + D + B` | **129** (was 65) | `P_ChangeSector` work, intercept estimate |
| Live actors | `1 + K + P + 26k + (K−k) + 8(K−k) + R + D + B` | ≤ 337 at `k = 8` (201 at `k = 0`) | **`MAP_SCALE_ACTOR_SLOTS` 384 under wire 21** |
| Spawns in one pass | `39 − 3k` | 337 + 15 = 352 at `k = 8`; 201 + 39 = 240 at `k = 0`; crude 337 + 39 = 376 ≤ 384 | slot count |
| Thinker visits per pass | `384 + L + 2M` | | visit budget |
| `P_ChangeSector` per tic | `Σ_movers 4·(B(s) + N·Q)` | `N` 65 → 129 | re-evaluated, measured in step 7 |
| `spechit` | ≤ 8 per position check; every radius < 64 (barrel 10) | | rule 6 unchanged |
| Hitscan intercepts | lines (with multiplicity) + things of the ≤ 18 cells of a trace | E1M1 ≤ 103 (§3.3) against 128 | witness kept (F7), no load rule |
| Explosions per tic | ≤ `B` | 16 | |
| Sight queries per explosion | ≤ `(1 + K + B − 1)·(1 + K)`: the shootable candidates of 3 × 3 blocks, revisited after each of at most `K` wake-up relinks | ≤ 216; ≤ 3,456 per tic | per-explosion `SightControl` (F6, refined in §13), work ≤ `216·S` |
| `P_DamageMobj` calls per tic | ≤ `B·(K + B)` + the existing attack bodies | ≤ 384 | each a bounded body |
| `MAX_BONUS` | `6 × P` | 384 | pickup state range |
| Barrel removals per tic | ≤ `B` | 16 | retirement list |

No new fixed-capacity store; the only grown stores are the actor slots (384, encoded)
and the per-policy counters. The arithmetic certificate has no roster term and needs no
new clause: a barrel's momentum is bounded like a monster's (`MAP_SCALE_MOMENTUM_UNITS`,
clamp 30), its thrust uses the existing `preflight_*_impulse` proof with mass 100, and
`P_RadiusAttack`'s arithmetic is `|dx|`, `|dy|` of two positions inside the rectangle
(below 2¹⁵ units each) and a shift by 16 of a value below 2¹⁵·2¹⁶ after the wrap.

## 6. New owner decisions (F1–F11) with recommendations

Each: (a) consistent with section 4 of the brief and E1–E9? (b) no silent divergence?
(c) no new kind of fidelity gap beyond refuse-the-tic witnesses? (d) bounded stores?

**F1. Policy identity.** `OriginalExecutionProfile::PlayerMonstersBarrelsV1`, wire **21**,
DSIM**37**, context **`PLAYER-MONSTERS-BARRELS-1-MEDIUM-v1`**, constructor
`Simulation::original_player_barrels_map`, predicate `has_barrels` (tested before
`has_e1m1` in the version ladder; `has_e1m1` and every other list include the new
variant). Alternative: `PlayerMonstersE1M1LoadV1`. **Recommendation: the barrel name**,
following the pattern that names the headline mechanic, with the README row saying what
it means for E1M1. (a)–(d) yes.

**F2. Bounds.** Pickups 64, `MAX_BONUS` 6 × 64, barrels 16, actor slots 384, `N` 129,
monsters 8, drops 8, decorations 32, all under wire 21 only (every older constructor keeps
320 and 16; the DSIM≤36 readers keep their ranges). Options for barrels: 8 (E1M1 + 2),
**16**, 32 (every Episode 1 map). **Recommendation: 16**: 32 would need 400+ slots
(`337 + 16 + 39 > 384`) for no E1M1 benefit; raising it later is a bound change with its
own evidence. (a)–(d) yes; the slot count is a canonical DSIM field, so 384 is part of
the DSIM37 contract.

**F3. The spawn rule under wire 21 (S′).** The M3h5 invariant Ω needs only (M3h5 §4.3):
every XY mover's centre in Ω with its *open* box containing no point of the one-sided
walls `S`; every other actor's centre in `R`. The closed-box rule for every kind is
stronger than the proof needs and refuses E1M1 (§3.4). Options: (i) keep it (E1M1 never
loads); (ii) relax closed to open box for every kind (admits the six lamps and
candelabras, still refuses the nine bonuses and the Imp); (iii) **S′**: for every spawn,
odd centre parity and centre inside `R` (no box clause); statics (pickups, decorations)
nothing more, as the invariant already asks only their centre; movers (monsters, barrels,
the player) whose open box meets a wall `W` admitted iff the centre is in front of `W`'s
line by more than `(31 − r)·w(θ)` units, `w(θ) = |cos θ| + |sin θ|`, in exact integers:
`|dx·(cy − ay) − dy·(cx − ax)| ≥ (31 − r)·(|dx| + |dy|)` for every such `W`. **Recommendation:
(iii).** Proof sketch: a box of half-width `r` lies entirely on one side of a line of
direction `θ` iff its centre is at least `r·w(θ)` from the line, and `P_BoxOnLineSide`
blocks every destination box that straddles a one-sided line (p_map.c:214–230; the
reading lemma's degenerate case is within `1/L` of that bound). One `P_TryMove` moves at
most `MAXMOVE` = 30 per axis (p_mobj.c:137–145; negative moves are not halved, 150–164;
`P_SlideMove` uses the same momentum), so its component across `W` is at most `30·w(θ)`.
An embedded mover's centre could cross `W` only if `30·w ≥ d_f + r·w − 1/L`, i.e.
`d_f ≤ (30 − r)·w + 1/L`; S′ requires `d_f ≥ (31 − r)·w ≥ (30 − r)·w + 1`, so no accepted
move puts the box behind `W`; a move whose centre leaves Ω elsewhere needs a crossing
point of `S` inside the accepted box, which Lemma A (M3h5 §4.3) excludes for a box
accepted with its centre in Ω, and the segment from `c` to `c′` is covered by the two
boxes since `30·w < 2r·w`. Hence the centre stays in Ω, the first accepted move
re-establishes the box clause by Lemma A, and the `Escape` and `Position` witnesses stay
unreachable on an admitted map, exactly as today. For `r` = 10 the condition cannot hold
while the box overlaps (overlap means `d_f < 10·w`), so **an embedded barrel is refused**;
for the player (`r` 16) it means 15·w; for the admitted monsters (`r` 20) 11·w, and
E1M1's Imp, embedded at most 4 units, has `d_f ≥ 16`. Statics never move (decorations by
E2; items have no momentum, and the API refuses pushes), so their boxes are read only by
`PIT_CheckThing` against movers and never against walls. Edge contact (the wall exactly
on the closed box's edge) is admitted for every kind since the open box is clear.
Measured exactly by the census tool: E1M1 has **0 S′ violations at every skill**, and its
embedded movers' margin `⌊cross / (|dx| + |dy|)⌋` is 16 against the required 11
([evidence record](validation/M3H9B.md#iwad-counts)); over all 68 slots 52 pass S′ at
medium against 4 for the closed-box rule. The
closed-box clause of `RoomBounds::contains_box` is relaxed to the centre for the same
reason (Ω ⊂ R; the position witness checks the centre). Older policies keep the rule.
(a) yes: `P_SpawnMapThing` checks no position, so admitting more spawns is faithful and
refusing fewer changes no admitted outcome; (b) yes; (c) no, no witness is added;
(d) yes.

**F4. Barrels in full**, as section 4 decides, with these sub-decisions. (i) The barrel
is a new actor role (`Barrel`: solid, shootable, animating, pushable, removable by its
own `S_NULL`, never a monster, never counted, never a drop source, never a target of
monsters), gated by `has_barrels`. (ii) `A_Explode` is admitted for `MT_BARREL` only
(the rocket and the Arch-vile fire stay refused by type). (iii) The explosion walks the
live cell lists in link order and re-reads the next link after every victim (§5.2).
(iv) **Stale bomb source**: the barrel's `target` is a generational handle; if it is not
readable when `A_Explode` runs (C reads freed memory: undefined), the tic is refused
(`PlayerBodyError::Invariant`, state preserved) rather than passing NULL. Unreachable in
the admitted domain (monster corpses and the player are never removed), recorded as a
witness. (v) `set_actor_target`, `mark_remove`, `enter_actor_state` and
`request_actor_death` on a barrel are refused (`Unsupported`), as for decorations;
`damage_actor` with an inflictor is admitted (it is how the C harness and tests wound
barrels), and `set_momentum` is admitted within the shared bound (a thrust barrel is
ordinary play). **Recommendation: all five.** (a)–(d) yes.

**F5. The wrapping shift as a declared reproduced quirk.** `dist = (damage+MAXRADIUS)<<FRACBITS`
(p_map.c:1221) overflows `int`; the original executable computes `128 << 16`, a 3 × 3
block window. Options: (i) emulate the wrap (`(damage.wrapping_add(MAXRADIUS)) << 16` in
`u32`, documented); (ii) compute the "intended" 160-unit window. **Recommendation: (i)**,
with the C evidence compiled under `-fwrapv` in the strict build and a declared,
cited adapter for that one statement in the sanitizer builds
(`(fixed_t)(((unsigned)damage + (unsigned)MAXRADIUS) << FRACBITS)`, bit-identical),
because `-fsanitize=undefined` would report the shift; the three traces must still be
byte-identical. The same adapter pattern serves E9's angle product in the
`--overflow-check` build (F8). (a) yes: behavioural authority is the original build;
(b) yes; (c) no; (d) yes.

**F6. Sight budget for explosions.** The 64-query `SightControl` per tic is shared by
every monster action. Options: (i) let explosions draw from it (an E1M1 chain of 6
barrels with 6 monsters and the player could spend 6 × 13 = 78 and refuse the tic);
(ii) a per-explosion control with at most `1 + K + B − 1` queries and work `≤ (1 + K +
B − 1)·S`, exhausted only by a map that breaks the certificate. **Recommendation: (ii)**;
exhaustion stays `PerceptionError::WorkLimit`, unreachable on admitted maps because every
query's work is below `S` by the sight certificate. *Refined in Phase 2 (§13):* the bound
is `24 × (1 + K)` = 216 queries, since a victim woken inside the walk relinks and makes
the walk revisit things, at most once per monster. (a)–(d) yes.

**F7. The hitscan intercept argument.** With `N` = 129 the crude M3h9 estimate (window
lines + every thing) exceeds 128. Options: (i) a per-map load rule counting lines and
things over every 18-cell straight path; (ii) keep the per-tic witness (`Intercepts`,
state preserved) and record the E1M1 estimate of §3.3 (≤ 103). **Recommendation: (ii)**:
C has no bound either (the array overflows silently), the witness is the faithful
refusal, and a load rule would reject maps on which the source plays normally; M3h9c's
real run observes the actual maximum. (a)–(d) yes.

**F8. Every THING angle (E9, confirmed).** Remove the `Angle` refusal under wire 21 only;
the conversion is unchanged and proven for the eight octants and for non-multiples of 45
(30 → 0, 100 → 2, 359 → 7, −1 → 0, −45 → −1 ≡ 7, −100 → −2 ≡ 6, 400 → 8 ≡ 0, −32768 →
−728 ≡ 0, 32767 → 728 ≡ 0): `q × 0x20000000 mod 2³²` depends on `q mod 8` only. C
evidence: a census room whose records carry those angles, the strict build genuine, the
`--overflow-check` build with the declared adapter of F5 for p_mobj.c:794. (a)–(d) yes.

**F9. DSIM37 content.** Options: (i) the DSIM36 layout with barrels as ordinary actor
records (type `MT_BARREL`, states `S_BAR1..S_BEXP5` or `S_GIBS`, `target` present after a
wound, `MF_CORPSE|MF_DROPOFF` and `height >> 2` after death), `store.max_slots` 384, the
pickup and bonus ranges of F2, and **no new event**: an explosion is the tag-12 `Action`
record of `A_Explode` followed by the existing tag-13 `Damage`/`Knockback`/`Health`/
`Pain`/`Kill`/`DeathTimer`/`Retarget`/`Ignored` records of each victim in walk order; the
candidates skipped by range or sight are C comparison rows, not state; (ii) a new
radius-attack event with the candidate list. **Recommendation: (i)**: the source has no
explosion state (`bombspot`/`bombsource`/`bombdamage` are transient globals), and the walk
order is already pinned by the order of the damage records and the RNG stream. Reader
`decode_barrels` accepts 37 only and validates every barrel (type, state in the chain,
health ≤ 20 or ≤ 0, flags, radius 10, height 42 or 10.5, `target` semantics, the
type-keyed `S_GIBS` crunch); every earlier reader rejects 37; a capture without barrels
differs from DSIM36 in the version, profile, context, `max_slots` and the ranges. (a)–(d)
yes.

**F10. Easy and hard skill (E4 option ii).** Optional in the brief. **Recommendation:
defer** to a later part: hard still cannot load E1M1 (29 monsters against 8), the medium
run needs neither, and admitting them means a skill field in the constructor, a DSIM37
reader rule for the skill byte and per-skill C rows for every new scenario, which is not
"cheap and clean" inside a milestone of this size. The census room's easy and hard C-only
rows stay as evidence. (a)–(d) yes.

**F11. Docs-only CI fast path.** A first job `changes` (ubuntu, one `git diff --name-only`
against the pull request's base or the push's previous commit, no third-party action)
sets `docs_only = true` when every changed path matches `*.md`; the four required jobs
`need` it and every heavy step carries `if: needs.changes.outputs.docs_only != 'true'`,
while checkout, the documentation link check and a one-line report always run, so every
required check still reports success; `paths-ignore` is not used. Names, the
`macos-26-intel` runner and the timeouts are unchanged. Proof: one docs-only commit and
one code commit in the PR, both observed on GitHub. **Recommendation: adopt.**

**Unattended rule.** Every decision has a clear recommendation consistent with section 4,
admits no silent divergence (F3 admits spawns C admits; F5 and F8 reproduce the original
executable's arithmetic; F4(iv), F6 and F7 are witnesses), adds no new kind of fidelity
gap and no unbounded store, and the size estimate of §8 fits one milestone. Phase 2
proceeds with F1–F11 **provisionally adopted, pending owner confirmation**.

## 7. Evidence plan (C first, genuine bodies)

New generator set `tools/reference/m3h9b_{geometry,cases,reference}.py` layered on
`m3h9_*` as M3h9 layered on M3h8, outputs `tests/reference/m3h9b_*` (trace, `cases.csv`,
`geometry_{i}.bin`, `maps.rs`, `contract.json`), strict, `--overflow-check` and
`--address-check` traces byte-identical. **Genuine bodies added**: `P_RadiusAttack`,
`PIT_RadiusAttack` (p_map.c) and `A_Explode` (p_enemy.c, replacing the abort stub), with
observer rows for every `PIT_RadiusAttack` candidate (block, link position, outcome:
non-shootable, out of range, sight blocked, damaged with the amount) and for the
`P_RadiusAttack` window. Declared adapters: the two sanitizer-build rewrites of F5/F8,
cited and hashed. Scenario assertions admit 2035, up to 16 barrels and 64 pickups.

| Room | Scenarios | What they pin |
| --- | --- | --- |
| `range`: a hall with one barrel, the player with the pistol | `barrel_wounded` (one shot: puff, pain draw, `target` = player, thrust); `barrel_killed` (second shot: `S_BEXP`, tics draw, solid until removal, timing `T + t0 + 9`, `A_Scream` without a draw, the explosion with nobody in range, `S_NULL`); `fist_kills_barrel` (one blow ≥ 20, no `target`, `S_PUFF3`); `autoaim_barrel` (the aim trace picks the live barrel; a dying barrel is passed) | §2.1–2.5 |
| `cluster`: five barrels in a tight group, one at 120 units, one at 140 units behind a cell boundary | `chain_reaction` (the first explosion kills the four in its window with distinct `S_BEXP` timers; the 140-unit barrel sits in a fourth block and survives although `dist − radius < 128`: the wrap quirk; the second-generation explosions in thinker order) | F5, chain, timing, `d` |
| `player_range`: the player at several distances and above a 64-unit ledge | `player_hurt_by_barrel` (armor absorption, thrust, `attacker` NULL, pain), `player_killed_by_barrel` (`frags[self]`, `PST_DEAD`, the reborn tic refused), `player_hurt_by_own_barrel` (`attacker` = player after wounding it), `fall_forward_draw` (victim above the barrel, lethal hit under 40) | §2.3, §2.7 |
| `monsters`: two Zombiemen, an Imp, barrels between them | `barrel_kills_monsters` (drops, `players[0]` credit with a NULL source, pain draws), `barrel_infight` (Zombieman A wounds the barrel, the explosion hurts Imp B: B targets A, credit to `players[0]`), `player_credit` (the player wounded it: `killcount`), `monster_wakes_mid_walk` (an idle monster hit in its spawn state enters `A_Chase` inside the walk and relinks, with the walk order observed) | §2.3–2.4, link order |
| `walls`: barrels around a 16-unit step and a thin wall, a lift sector with an 88 line | `radius_through_window` (sight through an opening), `radius_blocked_by_step` (a 12-unit step blocks the 10.5-unit window), `barrel_pushed_into_wall` (momentum zeroed), `barrel_slides_over_lift_line` (a wounded barrel crossing an 88 line lowers the lift), `dying_barrel_dropoff` (a dying barrel pushed over a 32-unit ledge falls and explodes below) | §2.2, §2.6 |
| `door`: a DR door over a barrel | `door_gibs_dying_barrel` (the dying barrel becomes `S_GIBS`, never explodes, the door closes), `door_reverses_on_live_barrel` | §2.6 |
| `fireball`: an Imp and barrels | `fireball_kills_barrel` (missile damage draw, `source` = Imp, later `players[0]` credit if a monster dies), `fireball_on_dying_barrel` (explodes, no damage) | §2.2 |
| `dense64`: 64 pickups of E1M1's kinds and 16 barrels, 6 monsters, 27 decorations | `dense_pickups` (every pickup spawns, counts, `prndindex`), `intercept_heavy_trace` (a 2,048-unit hitscan across the densest row: intercept count observed, below 128), `near_witness` (Rust-only: a 129th intercept refuses the tic) | F2, F7 |
| `angles`: records at 0, 45, …, 315, 30, 100, 359, −1, −45, −100, 400, 32767, −32768 | `all_octants` (every spawned angle equals the wrapped product) | F8 |
| `embedded`: an Imp overlapping a wall by 4 units, a lamp with the wall on its edge, a bonus overlapping a wall | `embedded_spawns` (C spawns them all; the Imp walks out and never through; the pickup is collected) and Rust-only `embedded_barrel_refused`, `embedded_player_refused` | F3 |

Each scenario's room is also loaded by the Rust replay (`barrels_replay`) and the new
example `player_barrels`. Independent DSIM37 projection: `tools/reference/barrels_snapshot.py`
extending `e1m1_snapshot.py` (version 37, policy 21, `MAX_SLOTS` 384, the barrel actor
rows) and `m3h9b_snapshot_goldens.py` (`tests/reference/m3h9b_snapshots.csv`,
`m3h9b_events.dref`, `m3h9b_state_*.bin`, `m3h9b_snapshot_contract.json`), written from C
rows before the Rust writer.

Rust-only evidence: rejections (a 17th barrel, a 65th pickup, barrels under wires 18–20,
rockets, an embedded barrel or player, octants under wires ≤ 20), rollback of a failed
batch after accepted barrels, the stale-source witness, the per-explosion sight budget,
`decode_barrels` negatives (36 rejected, each barrel check mutated), soaks, the `N` and
slot formulas as asserted constants, fuzz target `player_barrels` with a seeder.

## 8. Size estimate and split

M3h9a was 87 files, +13,192 / −346. Here: C fixtures and generators ≈ 7–9k (ten rooms,
≈ 30 scenarios, three new genuine bodies with observers, contracts), Rust ≈ 5–6k (the
variant with its ≈ 70 range sites, the barrel role and `A_Explode`, the radius attack,
S′, the bounds, DSIM37 writer checks and reader, replay, example, tests, fuzz target),
tools ≈ 1.5k (encoder, goldens, check and measure scripts, the census additions already
written: +460), CI ≈ 0.1k, docs ≈ 3k: **≈ 17–20k lines, below the 25k split threshold**.
One milestone; no split.

## 9. Implementation plan (one commit per step, pushed; evidence before runtime)

1. **Read-only audit** of the gaps in §4.1 against the C facts; confirm the sanitizer
   flags of the three builds for F5/F8; no behaviour change.
2. **C evidence**: `m3h9b_{geometry,cases,reference}.py`, the three genuine bodies and
   observers, the §7 rooms; commit `tests/reference/m3h9b_*`; strict, UBSan and
   ASan/UBSan traces identical.
3. **DSIM37 projection**: `barrels_snapshot.py`, `m3h9b_snapshot_goldens.py`, the draft
   STATE_FORMAT section, goldens committed.
4. **Policy plumbing**: the variant, wire 21, context, predicates, `has_barrels`, the
   version ladder (37), every range, `policy()`, `decode_barrels` skeleton, the constructor
   with 384 slots; captures identical to DSIM36 except version, profile, context and
   `max_slots`.
5. **Bounds and angles**: pickups 64 and `MAX_BONUS` per policy, `MAX_BARRELS`, `N`, the
   angle refusal lifted under wire 21, S′ with its exact integer test and tests for every
   older policy keeping the old rules.
6. **Barrels**: role, admission, `A_Explode`, the radius attack, the barrel as a damage
   target, puff on hit, the type-keyed `S_GIBS` rule, the API refusals of F4(v), the
   stale-source witness, the per-explosion sight control.
7. **DSIM37 writer and reader**: `validate_barrels`, goldens matched byte for byte,
   round trip.
8. **Replay, example, support module, check and measure scripts, CI steps.**
9. **Tests**: rejections, rollback, soaks, reader negatives, the witnesses, the formulas.
10. **Fuzz**: target and seeder; seed replay, one coverage-guided and one ASan campaign
    (`detect_leaks=0` disclosed) on the final target.
11. **CI docs-only fast path** (F11) with its proof commits.
12. **Docs**: COMPATIBILITY section, STATE_FORMAT final, TESTING, ROADMAP, README (one
    row, "Where it stands", the two owner-approved edits), validation/M3H9B.md, this note.
13. **Matched timings** against master, alone, all digests compared, callgrind for any
    older-policy slowdown over 3 %, the `P_ChangeSector` measurement at `N` = 129.
14. **Independent review** (Opus) of the full diff against the C source; fix and record.
15. **Full gates** from clean; draft PR; ready when every job is green on the exact head;
    scratch deleted, space reported.

## 10. E1M1 medium load admission under the new policy

**Yes, provisionally**: with F2 (6 barrels ≤ 16, 52 pickups ≤ 64, 6 monsters ≤ 8, 27
decorations ≤ 32), F3 (the 16 wall contacts: six edge contacts, nine overlapping statics,
one Imp whose centre is at least 16 units in front of the wall it overlaps), F4 (barrels
admitted) and F8 (45 octants admitted), every load rule E1M1 is measured against passes:
the M3h6 geometry rules, boundary C1–C3, the arithmetic certificate, the slide window
(28 ≤ 128), the M3h7 Stage A rule, the M3h8 Stage B rules with 8 two-sided scrollers
under wire 20's relaxation and a `spechit` window of exactly 8, the height fit (0
failures, minimum sector height 72) and the spawn filter (medium, single player). What
still blocks it: **nothing measured** (S′ measured exactly: 0 violations). The real run that confirms the load and the tic-for-tic match is M3h9c. The items
M3h8 §13 left unmeasured (door-kind sharing, the `ML_SECRET` DR line, case-(d) monsters)
are admitted either way by decisions N2 and N5. The per-tic witnesses remain reachable in
principle (intercepts: E1M1 estimate 103 of 128; the sight and `spechit` budgets).

## 11. Policy proposal

`OriginalExecutionProfile::PlayerMonstersBarrelsV1`, wire 21, DSIM37, context
`PLAYER-MONSTERS-BARRELS-1-MEDIUM-v1`, constructor `Simulation::original_player_barrels_map`,
a superset of `PlayerMonstersE1M1V1`: every Stage A and Stage B predicate plus
`has_barrels`. Load rules: wire 20's, with barrels ≤ 16, pickups ≤ 64, every angle, S′,
384 slots. Runtime witnesses: unchanged plus the stale bomb source. DSIM37 as F9. Older
policies byte and behaviour identical, confirmed by the unchanged goldens and matched
digests.

## 12. Follow-ups, risks, status

- **M3h9c**: the local-only real-E1M1 harness (M3h9 §10) and the matched run, which also
  observes the actual intercept maximum, the `P_ChangeSector` work and whether any
  witness fires.
- **Later bounds**: barrels 32 and pickups beyond 64 for the rest of Episode 1 need
  more slots (the formula of §5.3) and their own evidence; easy and hard skill (F10).
- **Risks**: the barrel `target` semantics (first wounder) must be reproduced exactly or
  kill credit diverges; the live walk must re-read links after each victim; the
  per-explosion sight control must not disturb the monsters' shared budget; the
  `A_Explode` action must be refused for every type but `MT_BARREL` at admission; the
  seventy-odd version-range sites are the usual plumbing risk, covered by the identical
  older goldens.
- **Quirks reproduced**: the wrapped shift (F5), the angle product (F8), the first-wounder
  bomb source, dying barrels solid, the 10.5-unit sight window, the positive-only move
  halving, the `S_GIBS` crunch of a dying barrel.
- **Quirks recorded, not emulated**: crusher blood on a `MF_NOBLOOD` barrel (crushers
  rejected), the chainsaw thrust exception (saw not admitted), the MAP30 stomp, the
  unchecked `intercepts` and `spechit` arrays (witnesses), the dangling `target`
  (witness).

**Status / next step.** Phase 1 is complete with this note and the census additions.
F1–F11 are provisionally adopted pending owner confirmation. Phase 2: steps 1–8 and 11 are
done (the C evidence with 26 scenarios, the DSIM37 projection, the policy, bounds, angles,
S′, barrels, the reader, the replay matching every row and golden on its first run, the
example and scripts, the CI fast path); steps 9–10 and 12–15 follow. Observed results are
in the [evidence record](validation/M3H9B.md); the corrections found on the way are in
§13.

## 13. Corrections and findings from Phase 2 (kept with the original text above)

Recorded here rather than rewritten, so the Phase 1 reasoning stays readable:

- **F5, the sanitizer adapter for the shift.** The declared rewrite of p_map.c:1221 is
  committed but was observed *not* to be needed: with `-std=gnu89` GCC checks `shift-base`
  only for C99 and later, so neither sanitizer build reports the overflowing shift. The
  rewrite of p_mobj.c:794 (F8) *is* needed: without it both sanitizer builds abort with
  `signed integer overflow: 4 * 536870912`. The contract records both facts.
- **F6, the per-explosion sight bound.** 24 counts distinct candidates; a victim relinked
  by its own `A_Chase` makes the walk revisit things and query them again. Each monster can
  wake at most once per explosion, so the queries are bounded by `24 × (1 + K)` = 216 and
  the work by `216 × S`; the constants were raised to that bound (step 8) and the refusal
  stays unreachable on admitted maps. The C projection has 33 sight records over 27
  explosions.
- **§2.2, a fireball on a dying barrel.** It explodes on the barrel only when their heights
  overlap: on a common floor the fireball flies 32 units above the feet line and passes
  over the 10.5-unit corpse (p_map.c:294–297). The C scenario `fireball_on_dying_barrel`
  puts the barrel on a 24-unit pedestal; the Rust test lowers a gallery pedestal to 24.
- **§2.6, a dying barrel under a door.** A live barrel holds a DR door at 42 units
  (`nofit`), and a barrel killed in place explodes (11–14 tics) before the door, closing
  at 2 units per tic, can crunch it (16 tics). Only a dying barrel that slides under an
  already-low door is gibbed; the C scenario pushes it there with the killing thrust.
- **§7, "a 12-unit step blocks".** With the eye at ¾ height and the barrel clear of the
  step, blocking the 10.5-unit window needs a step of about 14 or more; the Rust test uses
  12 (visible) against 16 (blocked), with the threshold derived in its comment.
- **§2.3, reaction time.** A barrel that survives a wound has `reaction_time` 0
  (p_inter.c:902); 8 only while untouched. The readers accept {8, 0}.
- **§4.1, gaps not listed.** `preflight_player_body` refused any moving non-monster and the
  post-tic travel loop skipped non-monsters (a thrust barrel failed the tic);
  `preflight_player_damage` refused every inflictor request on a non-player under E1M1;
  `validate_entry` refused the `S_BEXP` chain; the spawn-rule check has seven call sites,
  not two. All follow S′ and the barrel role under wire 21.
- **§7, the fist scenarios.** `fist_kills_barrel` and `player_killed_by_barrel` first gave
  the player the fist alone, which the DSIM weapon layout cannot encode (DSIM v23 requires
  fist and pistol owned); they now own both with the fist ready, with every observed
  outcome unchanged (nothing on the punch path reads the pistol bit).
- **§2.8, trace cells.** Things are offered to a trace only in the cell of their centre,
  and a trace on a row boundary is nudged into one row (p_maputl.c:777–781): a pistol
  trace at `y = −0.007` never sees barrels at `y = 0` linked in the next row. Not a quirk of
  the harness but of `P_PathTraverse`; it matters for matching E1M1 hitscans.
- **§7, the C harness's intercept array.** The M3d3 instrumentation aborts before a 129th
  intercept, so the unchecked overflow of the source is never observed in C; the Rust-only
  lane `m3h9b_intercepts129.bin` exercises the witness.
- **Chain timers** in `chain_reaction` were 5, 3, 5, 5, not all distinct as §7 assumed.
