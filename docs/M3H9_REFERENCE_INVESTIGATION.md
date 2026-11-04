# M3h9 reference investigation: E1M1 runs

Phase 1 note for the milestone that loads and simulates the owner's real first map under
a new opt-in policy, tic for tic against linuxdoom-1.10 on the same map, skill and input.
It records the original C facts (function, file and line), the E1M1 THING census (counts
only), the admitted domain and budgets of the new policy, the evidence and implementation
plans, the design of the local-only real-map harness, and the new owner decisions with
recommendations. It follows the conventions of the [M3h8 note](M3H8_REFERENCE_INVESTIGATION.md).

Source authority is `linuxdoom-1.10/` at the repository's upstream commit; line numbers
are of those files. Milestone status lives in [ROADMAP](ROADMAP.md), support in
[COMPATIBILITY](COMPATIBILITY.md), schemas in [STATE_FORMAT](STATE_FORMAT.md), commands in
[TESTING](TESTING.md), observations in [validation/M3H9.md](validation/M3H9.md) and the
census in [validation/M3H9_IWAD_COUNTS.md](validation/M3H9_IWAD_COUNTS.md).

## 1. Starting point

Master contains M3h8 (`PlayerMonstersMoversV1`, wire 19, DSIM35, context
`PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1`). Its [§13](M3H8_REFERENCE_INVESTIGATION.md#13-e1m1-readiness-under-the-movers-policy)
names four blockers for E1M1 under that policy: the 8 two-sided scrolling walls (line
type 48, rejected by decision N1 as option (iii)), the monster roster (29 on UV against
8), the pickup roster (53 on UV against 16) and 33 THINGs of unadmitted types. Its §15
lists the unmeasured items (the M3h5 spawn rule over every E1M1 THING, door-kind
sharing, the `ML_SECRET` DR line, case-(d) spawns).

Owner decisions already made (section 4 of the brief): N1–N9a of M3h8 stand; N1 is
reopened here only for type 48 under the new policy, older policies keep rejecting it;
every older policy keeps its exact admission, behaviour, digests and DSIM bytes; fidelity
gaps stay refuse-the-tic witnesses with state preserved, recorded for M4, never a silent
divergence; crushers, the intermission and next map, multiplayer and rendering stay out.

The census of §3 changes the picture: on skill 3 ("Hurt Me Plenty", `sk_medium`, the only
skill every player policy accepts, §4.2) E1M1 spawns **6 monsters**, not 29, because all
16 shotgun guys and 5 of the 9 zombiemen carry the hard bit only. The monster bound of 8
already holds. The remaining blockers on skill 3 are the scrollers, 27 static
decorations of 7 types, 6 exploding barrels and 52 pickups against 16.

## 2. Original C facts

### 2.1 `P_SpawnMapThing` (p_mobj.c:708–795) and the spawn draws

Control flow, in order:

1. Deathmatch starts (718–726): `if (mthing->type == 11)` copies the record into
   `deathmatchstarts` only while `deathmatch_p < &deathmatchstarts[10]`
   (`MAX_DEATHMATCH_STARTS` 10, p_setup.c:109–111); extra starts are dropped silently;
   `return`.
2. Player starts (729–737): `if (mthing->type <= 4)` stores `playerstarts[type-1]`
   and calls `P_SpawnPlayer(mthing)` only when `!deathmatch`; `return`. Type 0 is
   unguarded (writes `playerstarts[-1]`; the census finds no non-positive type on any
   of the 68 slots).
3. Not in single player (740): `if (!netgame && (mthing->options & 16)) return;`. The
   literal `16` has no macro in 1.10; doomdef.h:140–145 defines only `MTF_EASY` 1,
   `MTF_NORMAL` 2, `MTF_HARD` 4 and `MTF_AMBUSH` 8.
4. Skill bit (742–750): `bit = 1` for `sk_baby`, `4` for `sk_nightmare`, else
   `1 << (gameskill-1)` (enum doomdef.h:149–153: baby 0, easy 1, medium 2, hard 3,
   nightmare 4), so baby and easy read bit 1, medium bit 2, hard and nightmare bit 4;
   `if (!(mthing->options & bit)) return;`.
5. Type lookup (753–760): a linear scan of `mobjinfo[i].doomednum`;
   `I_Error("P_SpawnMapThing: Unknown type %i at (%i, %i)")` on a miss.
6. `if (deathmatch && mobjinfo[i].flags & MF_NOTDMATCH) return;` (763–764). There is
   no `MF_SPECIAL` skip in deathmatch.
7. `if (nomonsters && (i == MT_SKULL || (mobjinfo[i].flags & MF_COUNTKILL))) return;`
   (767–772).
8. Spawn (775–783): `z = ONCEILINGZ` if `MF_SPAWNCEILING`, else `ONFLOORZ`
   (p_local.h:95–96); `mobj = P_SpawnMobj(x, y, z, i); mobj->spawnpoint = *mthing;`.
9. `if (mobj->tics > 0) mobj->tics = 1 + (P_Random () % mobj->tics);` (785–786).
10. `totalkills++` for `MF_COUNTKILL`, `totalitems++` for `MF_COUNTITEM` (787–790),
    reading the live flags, which still equal the info flags here.
11. `mobj->angle = ANG45 * (mthing->angle/45)`; `MTF_AMBUSH` sets `MF_AMBUSH`
    (792–794), on every type, decorations included (E1M1 flags 4 floor lamps ambush;
    the bit is read only by `A_Look`, so it has no effect on a decoration but is part
    of its flags word).

`P_SpawnMobj` (p_mobj.c:480–534) draws **one `P_Random` for every spawned mobj**:
`mobj->lastlook = P_Random () % MAXPLAYERS;` (506). So each spawned THING draws once,
and a second time at step 9 if its spawn state has `tics > 0`. Spawn states with tics:
barrels (`S_BAR1`, 6, info.c:942), the admitted monsters (`S_POSS_STND`,
`S_SPOS_STND`, `S_TROO_STND`, 10; info.c:310, 343, 578), armour (`S_ARM1`/`S_ARM2`, 6;
info.c:938, 940) and the bonuses (`S_BON1`/`S_BON2`, 6; info.c:952, 958). The seven
E1M1 decoration types, the clips, shells, stimpack, medikit and the player (`S_PLAY`,
info.c:285) have `tics` −1 and draw only the `lastlook` value. A record filtered at
steps 1–7 draws nothing. The draws happen in THINGS-lump order (`P_LoadThings`,
§2.2). `P_SpawnMobj` sets the state without running its action (507–514), links the
thing (`P_SetThingPosition`, 517), reads `floorz`/`ceilingz` from the subsector
(519–520) and appends the thinker at the tail (`P_AddThinker`, p_tick.c:65–71), so
the thinker order is lump order. `reactiontime` is set only when
`gameskill != sk_nightmare` (503–504).

`P_SpawnPlayer` (p_mobj.c:642–700) returns unless `playeringame[type-1]` (654), so in
single player only start 1 spawns a body; it draws the `lastlook` value through
`P_SpawnMobj(MT_PLAYER)` (665), sets `PST_LIVE` (676), `viewheight` (683) and
`P_SetupPsprites` (686). `G_PlayerReborn` (g_game.c:800–833) gives health 100, the
pistol, the fist, 50 bullets and `usedown = attackdown = true` (822).

### 2.2 `P_SetupLevel` and `P_LoadThings` (p_setup.c)

`P_SetupLevel` (584–693): `totalkills = totalitems = totalsecret = 0`, each player's
kill, secret and item counts cleared (594–600); `P_InitThinkers` (622); `leveltime = 0`
(646); lumps in the order BLOCKMAP, VERTEXES, SECTORS, SIDEDEFS, LINEDEFS, SSECTORS,
NODES, SEGS, REJECT, `P_GroupLines` (649–660, "most of this ordering is important");
`bodyqueslot = 0`, `deathmatch_p = deathmatchstarts` (662–663); `P_LoadThings` (664);
deathmatch bodies only in deathmatch (667–676); `P_SpawnSpecials` (682);
`R_PrecacheLevel` only `if (precache)` (688–689). `P_LoadThings` (301–349) walks the
lump in order (313), byte-swaps the five shorts (339–343) and calls `P_SpawnMapThing`
(345). For `gamemode != commercial` a Doom II type (68, 64, 88, 89, 69, 67, 71, 65, 66,
84) sets `spawn = false` and the loop **breaks** (335–336), abandoning every later
record; the census finds none of those types on any Doom 1 slot, so the branch is
unreachable on the owner's IWAD and is recorded only as a quirk.

### 2.3 Level start: RNG, skill and state tables (g_game.c, m_random.c)

`G_InitNew` (1365–1482) clamps the skill (1379–1380), calls **`M_ClearRandom()`**
(1414; m_random.c:69–72, `rndindex = prndindex = 0`), sets
`respawnmonsters = (skill == sk_nightmare || respawnparm)` (1416–1419), halves the
Demon run and pain tics and sets the fireball speeds to 20 when `fastparm` or entering
nightmare from another skill (1421–1428) and restores them when leaving nightmare
(1429–1436), sets every player `PST_REBORN` (1440–1441), assigns `gameskill`
(1450) and calls `G_DoLoadLevel` (1481). `G_DoLoadLevel` (445–497) turns `PST_DEAD`
into `PST_REBORN` (479–480) and calls `P_SetupLevel` (484); it does not touch the
RNG, so on a later level the index carries over. `P_Random` pre-increments
(m_random.c:57–61): the first draw after a clear is `rndtable[1]` = 8. `M_Random`
has its own index and is used for sound pitch only (s_sound.c:330, 340).

The RNG index at the first tic is therefore the number of spawned THINGs plus the
animated ones (§2.1) plus the light thinkers that draw at spawn (§2.4), modulo 256.
Skill-dependent gameplay beyond the filter: ammo doubles on baby and nightmare
(`P_GiveAmmo`, p_inter.c:95–101); player damage halves on baby (`P_DamageMobj`,
p_inter.c:798–800); `reactiontime` is skipped on nightmare (§2.1). Medium has none
of these, which is why every player policy fixes it (§4.2).

### 2.4 `P_SpawnSpecials` and `P_UpdateSpecials` (p_spec.c)

`P_SpawnSpecials` (1239–1366) runs after every THING. Sector specials 1 and 2–4 draw at
spawn (`P_SpawnLightFlash`, p_lights.c:142, `(P_Random()&maxtime)+1`;
`P_SpawnStrobeFlash`, p_lights.c:205–206, `(P_Random()&7)+1`); 12, 13, 8, 17 and the
secret 9 (`totalsecret++`) do not. E1M1 has sector specials 1 ×1, 7 ×4, 8 ×2, 9 ×3 and
12 ×1 (M3h8 counts), all Stage A. Then `numlinespecials = 0` and every type 48 line
is appended to `linespeciallist[MAXLINEANIMS]` (1336–1346) **without a bounds check**
(`MAXLINEANIMS` 64, line 141); the 65th write is undefined behaviour and stays a load
rejection (rule 5 of M3h8 §7, kept).

`P_UpdateSpecials` (1083–1155) runs after `P_RunThinkers` each tic (p_tick.c:152–154).
For every listed 48 line it does only `sides[line->sidenum[0]].textureoffset += FRACUNIT`
(1114–1124). `textureoffset` is otherwise read only by the renderer (r_segs.c) and the
savegame code (p_saveg.c:151, 203); it is never read by movement, collision, sight, use,
shooting or any `P_Random` caller. The animated flats and textures (1100–1110) write
`texturetranslation[]`/`flattranslation[]` from `leveltime` for the renderer only; the
button restore (1128–1154) is already modelled (M3h8). So a two-sided 48 has exactly the
three game-code effects of M3h8 §2.9: it enters `spechit` from `PIT_CheckLine`
(p_map.c:240–244) when the position box crosses it and it does not block; a monster whose
`P_TryMove` fails with it in `spechit` resets `movedir = DI_NODIR` and calls
`P_UseSpecialLine(actor, line, 0)`, which returns false for 48 because the monster
filter admits only 1, 32, 33 and 34 (p_enemy.c:310–324, p_switch.c:301–319); for a
player the switch has no case 48 and the trailing `return true` is reached with no
effect (observed in `use_scroll_wall_stops_trace`); `PTR_UseTraverse` stops the use trace at any line with a nonzero
special (p_map.c:1113–1122). `P_CrossSpecialLine` has no case 48 (p_spec.c:503–534),
`PTR_ShootTraverse` ignores specials other than through `P_ShootSpecialLine`
(p_map.c:919–920, p_spec.c:966–999, no case 48), and sight, noise and `P_LineOpening`
never read `special`. A two-sided 48 with `ML_BLOCKING` blocks like any blocking line
and never enters `spechit` (PIT_CheckLine returns before the special test). The scroll
offset of a 48 line after `t` tics is its SIDEDEFS offset plus `t × FRACUNIT`: derivable
from `leveltime`, never state.

### 2.5 The seven E1M1 decoration types (info.c)

| DoomEd | `mobjinfo` (info.c) | Spawn state (line) | Tics | Radius / height | Flags (line) |
| --- | --- | --- | --- | --- | --- |
| 2028 | `MT_MISC31`, floor lamp (3215) | `S_COLU` (1022) | −1 | 16 / 16 | `MF_SOLID` (3236) |
| 48 | `MT_MISC48`, tall techno pillar (3657) | `S_TECHPILLAR` (1052) | −1 | 16 / 16 | `MF_SOLID` (3678) |
| 35 | `MT_MISC50`, candelabra (3709) | `S_CANDELABRA` (1048) | −1 | 16 / 16 | `MF_SOLID` (3730) |
| 24 | `MT_MISC71`, pool of blood and flesh (4255) | `S_GIBS` (1031) | −1 | 20 / 16 | 0 (4276) |
| 15 | `MT_MISC62`, dead player (4021) | `S_PLAY_DIE7` (300) | −1 | 20 / 16 | 0 (4042) |
| 10 | `MT_MISC68`, bloody mess (4177) | `S_PLAY_XDIE9` (309) | −1 | 20 / 16 | 0 (4198) |
| 12 | `MT_MISC69`, bloody mess (4203) | `S_PLAY_XDIE9` (309) | −1 | 20 / 16 | 0 (4224) |

None is `MF_NOBLOCKMAP`, `MF_NOSECTOR`, `MF_NOGRAVITY` or `MF_SPAWNCEILING`: all seven are
BLOCKMAP- and sector-linked. Every spawn state has `tics` −1 and no action; no state of
these types has an action. Spawn health is 1000, `painchance` 0, `mass` 100. None is
`MF_SHOOTABLE`, `MF_SPECIAL` or `MF_COUNTKILL`/`MF_COUNTITEM`. The four corpse and gore
types have flags 0 (not `MF_CORPSE`; that flag is set only by `P_KillMobj`,
p_inter.c:680, and read only by corpse friction, p_mobj.c:207, and the Arch-vile,
p_enemy.c:1134). Every radius is below 64, so the M3h8 §6 `spechit` window proof still
holds.

Per-tic behaviour (`P_MobjThinker`, p_mobj.c:415–475): XY movement only
`if (momx || momy || MF_SKULLFLY)` (418–427), Z movement only `if (z != floorz || momz)`
(428–436), state cycling only `if (tics != -1)` (441–449); the nightmare respawn path
returns unless `MF_COUNTKILL` (453–457). A static decoration therefore does nothing per
tic unless a mover changes its floor (then `P_ChangeSector` moves it, §2.7, and
`z != floorz` is restored by the clip, not by `P_ZMovement`). Momentum is never applied
to it: thrust comes only from `P_DamageMobj`, which needs `MF_SHOOTABLE`
(`PTR_ShootTraverse`, p_map.c:1013–1014; `PIT_CheckThing` missile branch, 304–305;
`PIT_RadiusAttack`, 1170–1171).

### 2.6 Collision with decorations

`PIT_CheckThing` (p_map.c:252–345): `if (!(thing->flags & (MF_SOLID|MF_SPECIAL|MF_SHOOTABLE)))
return true;` (258–259), so the flags-0 corpses and gore are passed through by everything.
For a solid decoration the box test is `blockdist = thing->radius + tmthing->radius`
(261–268); a missile against a non-shootable thing returns `!(thing->flags & MF_SOLID)`
(304–305), so an Imp fireball explodes on a lamp (`P_ExplodeMissile`) and passes through
gore; for every other mover the last line `return !(thing->flags & MF_SOLID);` (345)
blocks with **no height check**: a 16-unit lamp blocks a player or monster like an
infinitely tall pillar. `P_CheckPosition` (375–441) visits things over the box ±`MAXRADIUS`
with `bx` outer, `by` inner (418–426), then lines (429–438). Hitscan passes through every
non-shootable thing (`PTR_ShootTraverse`, 1013–1014). `P_CheckSight` (p_sight.c) walks
REJECT and the BSP only; things never block sight. `P_LookForPlayers` (p_enemy.c:499–)
iterates `players[]` through `lastlook`, so no monster ever targets a decoration.
Decorations enter `intercepts` of a hitscan or aim trace like any linked thing
(`PIT_AddThingIntercepts`, p_maputl.c, adds every thing whose box the trace crosses,
flags unread); this feeds the intercept budget of §5.3.

### 2.7 Decorations under movers: `PIT_ChangeSector` (p_map.c:1257–1314)

1. `if (P_ThingHeightClip(thing)) return true;` (1261–1265): a thing that fits keeps its
   floor contact (`P_ThingHeightClip`, 530–558, `P_CheckPosition` at the thing's own
   position, `onfloor` follow at 542–546), so decorations ride lifts and floors.
2. `if (thing->health <= 0)` → `S_GIBS`, `MF_SOLID` cleared, height and radius 0
   (1269–1279): never true for a decoration (health 1000; nothing damages it).
3. `if (thing->flags & MF_DROPPED) P_RemoveMobj` (1282–1288): never set on a decoration.
4. `if (!(thing->flags & MF_SHOOTABLE)) return true;` (1290–1294, "assume it is bloody
   gibs"): every decoration returns here. A door or floor closing onto a decoration
   never sets `nofit`, never damages it and never spawns blood. A decoration is always
   on its floor (spawned at `ONFLOORZ`, never given `momz`), so `P_ThingHeightClip`
   keeps `z = floorz` (542–546) and the ceiling simply ends below its top: V′ case (b)
   of M3h8 §4.1, already admitted. The Rust `height_clip` and `change_sector` of M3h8
   (`planes.rs`) already gate on `health <= 0` and `MF_SHOOTABLE` in this order (audit
   of step 1, [evidence record](validation/M3H9.md)).

### 2.8 Barrels: `MT_BARREL` (2035) and the explosion chain

`MT_BARREL` (info.c:1889): `S_BAR1` (6 tics) ↔ `S_BAR2` (6 tics), health 20, radius 10,
height 42, `MF_SOLID|MF_SHOOTABLE|MF_NOBLOOD` (1910), death `S_BEXP` (5) → `S_BEXP2`
(5, `A_Scream`) → `S_BEXP3` (5) → `S_BEXP4` (10, `A_Explode`) → `S_BEXP5` (10) →
`S_NULL`, removed by `P_SetMobjState` (p_mobj.c:62–66); no `A_BarrelDone` exists in
1.10. A barrel draws two values at spawn (§2.1) and changes state every 6 tics with no
RNG. It is damaged by player and monster hitscan (`PTR_ShootTraverse`; `MF_NOBLOOD` spawns
a puff instead of blood, p_map.c:1024–1027), by fireballs (`PIT_CheckThing` missile
branch, `P_Random()%8` damage) and by other explosions. `P_DamageMobj` (p_inter.c:775–917)
thrusts it with mass 100 (806–832, the fall-forward `P_Random()&1` at 820–823),
kills it at health ≤ 0 (888–891; `P_KillMobj`, 668–757: `MF_SHOOTABLE` cleared, **`MF_SOLID`
kept** since only players lose it at 705, `MF_CORPSE|MF_DROPOFF`, `height >>= 2`,
`tics -= P_Random()&3`, no drop, no `killcount` since not `MF_COUNTKILL`) and otherwise
**always draws the pain `P_Random()`** (894) even with `painchance` 0. A surviving barrel
remembers its first non-lethal damager as `target` (904–911, `BASETHRESHOLD`); a barrel
killed in one hit has `target` NULL. `A_Explode` (p_enemy.c:1598–1601) is
`P_RadiusAttack(thingy, thingy->target, 128)`. `P_RadiusAttack` (p_map.c:1206–1229)
computes `dist = (damage+MAXRADIUS)<<FRACBITS` with `MAXRADIUS` already `32*FRACUNIT`
(p_local.h:51), so the shift **overflows 32 bits** and wraps to a ±128-unit box; it walks
blocks `y` outer, `x` inner (1226–1228), things newest-linked first (p_maputl.c:436–441,
529–535). `PIT_RadiusAttack` (1164–1199) skips non-shootables and the Cyberdemon and
Spider, uses the Chebyshev distance minus the radius, and damages through
`P_CheckSight(thing, bombspot)` with `bombdamage - dist` and `source = bombsource`. The
exploding barrel is skipped because `P_KillMobj` cleared its `MF_SHOOTABLE`. Chain
explosions and damage to the player (armour, `P_DamageMobj` thrust from the barrel's
position) and to monsters (pain, death, `target = source` when the source is not NULL)
follow. This is a new body set (`P_RadiusAttack`, `PIT_RadiusAttack`, `A_Explode`,
barrel `A_Scream`), a new actor role (solid, shootable, not a monster, momentum from
damage with the existing knockback bodies) and a new RNG order dependence on link order;
see decision E3.

### 2.9 Counters

`totalkills`/`totalitems` (p_mobj.c:787–790) and `totalsecret` (p_spec.c, sector 9) are
level totals; the Rust `statistics` already records the first two per spawn
(`spawning.rs`). `killcount` goes to `source->player` for `MF_COUNTKILL` targets, or to
`players[0]` in single player when the source has no player (p_inter.c:683–697); a barrel
never counts. `itemcount` and `secretcount` are already modelled.

### 2.10 Corrections to assumptions of the brief

No `MTF_NOTSINGLE` macro exists (literal 16); the skill bit is a shift, not a table;
there is no deathmatch `MF_SPECIAL` skip (`MF_NOTDMATCH`); `MT_MISC71` is not
`MF_NOBLOCKMAP`; the tech pillar is `MT_MISC48` and the candelabra `MT_MISC50`; corpse
decorations are not `MF_CORPSE`; `PIT_ChangeSector` tests `health`, not `MF_CORPSE`.

## 3. E1M1 THING census (counts only)

From `tools/iwad_counts.py --things E1M1` (local, read-only; the full tables are in
[validation/M3H9_IWAD_COUNTS.md](validation/M3H9_IWAD_COUNTS.md)). 143 records, 31
distinct DoomEd numbers, all matched in `info.c`, no non-positive type.

Single player by skill (easy / medium / hard):

| | Easy | Medium | Hard |
| --- | --- | --- | --- |
| Spawned things | 89 | 91 | 115 |
| Monsters (`totalkills`) | 4 | 6 | 29 |
| Pickups | 52 | 52 | 53 |
| Solid decorations | 12 | 12 | 12 |
| Non-solid decorations (all corpses and gore) | 15 | 15 | 15 |
| Barrels | 6 | 6 | 6 |
| `totalitems` | 37 | 37 | 38 |

Monsters on medium: zombieman (3004) 4 of 9 records, imp (3001) 2 of 4; the 16 shotgun
guys (9) are hard-only; 15 of the 16 shotgun guys and every zombieman and imp are
`MTF_AMBUSH`. Pickups on medium, all of admitted kinds: shotgun 1, clip 2, shells 2,
stimpack 1, medikit 3, health bonus 12, armour bonus 25, green armour 1, blue armour 1,
box of bullets 1, box of shells 3 (= 52). The chaingun (1), rocket launcher (1), berserk
(1), boxes of rockets (3), 5 boxes of bullets, 3 boxes of shells, 1 shotgun and 1 shells
carry bit 16 and never spawn in single player, so the three unadmitted pickup kinds
(2002 here, 2003, 2023, 2046) are irrelevant to single player. Decorations: floor lamp
(2028) 8 (4 ambush), tech pillar (48) 2, candelabra (35) 2, pool of blood and flesh (24)
7, dead player (15) 4, bloody mess (10, 12) 2 + 2. **Barrels: 9 records, 3 with bit 16,
6 spawn on every skill** (3 ambush). Starts: player 1 ×1, players 2–4 ×3 (stored, not
spawned), deathmatch ×5 (stored). Spawn draws on medium: 91 `lastlook` values plus
the player's, plus 6 barrels, 6 monsters, 2 armours and 37 bonuses with tics = 143
`P_Random` calls, then the one sector-1 light flash draws once: `prndindex` = 144 at
tic 0 if every THING spawns (the player start precedes every other draw only if its
record precedes them in the lump; the C harness of §10 observes the actual order). Scale over all 68 slots (for later milestones, not targets here): monsters up to
174 on medium, pickups 272, solid decorations 75, non-solid 45, barrels 208.

## 4. Rust inventory

Full citations are in the M3h8 note §3 and the code; what M3h9 touches:

- **Policies.** `OriginalExecutionProfile` (`original_registry.rs:12–70`), wire ids
  (`:93`), context schemas (`:117`), fifteen `matches!` predicates (`:120–322`) each
  listing the variants it covers, exhaustive matches in `new` (`:687`) and `validate`
  (`:739`), `player_roster.rs:51, :138`, and the DSIM version ladder
  (`snapshot.rs:715–747`) with range checks `20..=35` throughout `snapshot.rs` and
  `diagnostic.rs` (`policy()` at `:424–432`; readers `decode_specials` 34 only,
  `decode_movers` 35 only).
- **Spawning.** `Simulation::spawn_map_things` (`spawning.rs:365–583`) walks THINGs in
  lump order, stores starts (1–4, 11), applies the skill bit and bit 16 exactly as
  §2.1 (`:398–406`; `ActorOptions::default()` fixes medium, `lifecycle.rs:163–165`;
  nightmare, non-single-player and respawn rejected, `demon_expanded_map.rs:66–74`),
  returns `MapSpawnError::UnsupportedType` for anything but a pickup kind
  (`world_pickups::kind_for`), a policy monster or `MT_SERGEANT` (`:429`), enforces
  the monster cap and `MAX_WORLD_PICKUPS` (`:433–449`), and `spawn_placed_thing`
  (`:602–650`) draws `lastlook` through `spawn` (`lifecycle.rs:1318`) and the tics
  value, counts totals, sets the angle and `MF_AMBUSH`.
- **Bounds.** `MAX_PLAYER_MONSTERS` 8 (`player_monsters.rs:12`), `MAX_WORLD_PICKUPS` 16
  and `MAX_BONUS = 6 × 16` (`world_pickups.rs:7, :15`), drops 8, `MAX_LINKED_THINGS =
  1 + 8 + 16 + 8 = 33` (`player_map_scale.rs:48`, private), `MAP_SCALE_ACTOR_SLOTS` 320
  with the live-actor formula `1 + 8 + 16 + 26k + (8−k) + 8(8−k) + 8 ≤ 241` plus at
  most `39 − 3k` spawns in one pass (`:9–12`), `MAP_SCALE_MAX_LIVE_PROJECTILES` 208,
  `MAX_INTERCEPTS` 128 (`:864`), `MAX_PLATS` 30, `MAX_BUTTONS` 16, `MAX_SPECIAL_CROSS` 8
  (`mover_rules.rs:23–28`), `MAX_LINE_ANIMS` 64 (`sector_thinkers.rs:21`), sector-thinker
  slots `L + 2M`, visits `320 + L + 2M`, sight budget `64·(sight_work + 5·lines)`,
  collision work `Q = nodes + 4·lines + 4·320` (`map_extent.rs:190–210`). The arithmetic
  certificate (`:865–1022`, clauses (i)–(viii)) has no roster term. The static assert
  `MAP_SCALE_WALL_LINES + MAX_LINKED_THINGS ≤ 128` (`:1088`) belongs to the older
  95-linedef map-scale policy; the real-map policy's slide-window rule
  (`slide_window_lines ≤ MAX_INTERCEPTS`, `:1031–1068`) counts lines only, because a
  slide trace adds lines only.
- **Scrollers.** `validate_specials` (`demon_expanded_map.rs:248–275`) and
  `validate_movers` (`mover_rules.rs:252, :267`) admit 48 on one-sided lines and
  return `PursuitError::Specials` for a two-sided one; `special_window_lines` (`:452`)
  counts rule 6. The offset is "tic units, never stored" (`sector_thinkers.rs:321–322`);
  DSIM35 stores every line with a nonzero map special (`snapshot.rs:1050, :1282`), so a
  48 line already appears as `line.{i}.special` = 48. The runtime readers of a special
  are `line_triggers::spechit` (`line_triggers.rs:56`), crossings (`:111`) and
  `use_lines.rs:263`.
- **States and types.** `source_actors.rs` holds the full `states[]` table including
  `S_COLU`, `S_TECHPILLAR`, `S_CANDELABRA`, `S_GIBS`, `S_PLAY_DIE7`, `S_PLAY_XDIE9` and
  `S_BAR1..S_BEXP5`; `S_GIBS` actors are admitted by `decode_movers`
  (`diagnostic.rs:416–420`).
- **Movers.** `planes.rs` `change_sector` (`:206`) and `height_clip` (`:294`) gate on
  `health <= 0` (`:254`) then `MF_SHOOTABLE` (`:268`) as §2.7.
- **Harness.** `tools/reference/m3h8_{geometry,cases,reference}.py` build synthetic
  rooms, compile genuine bodies with declared adapters under strict, UBSan and
  ASan/UBSan flags, and write `tests/reference/m3h8_*`; `movers_snapshot.py` and
  `m3h8_snapshot_goldens.py` project the C rows to DSIM35 independently of Rust.
  `tools/measure_real_map.py` uses synthetic rooms only; only the `#[ignore]`
  `DOOM_IWAD_DIR` tests and `tools/iwad_counts.py` read IWADs.

## 5. Admitted domain of `PlayerMonstersE1M1V1` and its budgets

### 5.1 The domain

A superset of `PlayerMonstersMoversV1`: the same bodies, rooms, movers, Stage A sector
specials, skill (medium, single player) and roster bounds, plus:

1. **Two-sided scrolling walls (48)**, blocking or not, admitted under this policy
   only (decision E1); counted by rule 6 (window ≤ 8) and rule 5 (≤ 64 scrollers);
   no state, no event, no DSIM field (the offset is `t × FRACUNIT`, §2.4). The three
   game-code effects of §2.4 are already Stage B bodies; the C evidence shows them on
   two-sided lines (§7).
2. **The seven static decoration types** of §2.5 (decision E2), spawned by
   `P_SpawnMapThing` with their info flags, radius, height, health 1000, spawnpoint,
   angle and `MF_AMBUSH`, one `lastlook` draw, tics −1, linked to BLOCKMAP and sector,
   a thinker in lump order, blocking per §2.6 (solid: infinite height; flags 0: passed
   through; fireballs explode on solids), riding movers per §2.7, never gibbed,
   removed, damaged, targeted or sight-checked, and present in hitscan intercepts. Bound
   `MAX_DECORATIONS` = 32 per map (E1M1 27 on every skill; §5.3 budgets).
3. **Skill and flag filtering** as today, newly evidenced in C with records carrying
   each skill bit, bit 16 and ambush (§7), so that the draws and the thinker order of a
   filtered lump are source-matched.

Still rejected at load under this policy (checked errors, state unchanged, not fidelity
gaps): barrels and every other THING type (`MapSpawnError::UnsupportedType`), more than
8 monsters or 16 pickups (`WorkLimit`), more than 32 decorations (new field-less
`MapSpawnError::DecorationLimit`), every M3h8 mover rule, 65 scrollers, crushers and the
other deferred specials, non-medium skills, netgame and nightmare options. The
refuse-the-tic witnesses (escape, intercepts, `SpecialCross`, `Vertical`, exit, reborn)
are unchanged. E1M1 therefore still does not load under this policy (barrels, pickups):
see the split in §8.

### 5.2 Faithfulness arguments

- *Scrollers.* M3h8 §2.9 and §2.4 here: `spechit` order matters only through the count
  (a 48 entry changes no other entry's effect), the `DI_NODIR` reset and the use stop
  are existing bodies, crossing is a no-op, shooting ignores it. Admitting blocking
  two-sided 48 too costs nothing: `PIT_CheckLine` returns before the special test.
- *Decorations.* Every per-tic body that could touch them already exists in Rust for
  monsters, pickups and the player (position checks, height clip, blockmap iteration,
  intercept collection, thinker run); what changes is only that a thing may be
  non-shootable and non-special yet `MF_SOLID`, or have flags 0 and health 1000. The
  Rust `PIT_CheckThing` equivalent must be checked for the exact flag tests of §2.6
  (step 1 of §9) and the missile branch's `!(flags & MF_SOLID)` return.
- *Spawn RNG.* Already the §2.1 order; the C evidence pins the draws for filtered
  records (none) and animated spawns (one more).

### 5.3 Budgets as formulas (with the values of this policy)

Symbols as in M3h8 §8, with `D` = decorations ≤ 32, `K` = monsters ≤ 8, `P` = pickups
≤ 16, `R` = drops ≤ 8, `k` = Imps ≤ `K`.

| Budget | Formula | Value here | Store or certificate it feeds |
| --- | --- | --- | --- |
| Linked things `N` | `1 + K + P + R + D` | 65 (was 33) | `P_ChangeSector` work, intercept reasoning |
| Live actors | `1 + K + P + 26k + (K−k) + 8(K−k) + R + D` | ≤ 273 at `k = 8` | `MAP_SCALE_ACTOR_SLOTS` 320 |
| Spawns in one pass | `39 − 3k` | ≤ 39 | 273 + 15 = 288 ≤ 320 at `k = 8`; 241 + 39 = 280 at `k = 0` |
| Thinker visits per pass | `320 + L + 2M` | unchanged | visit budget (decorations are actors inside the 320) |
| Sector-thinker slots | `L + 2M` | unchanged | DSIM `sector_thinkers.slot_count` |
| `P_ChangeSector` per tic | `Σ_movers 4·(B(s) + N·Q)` | `N` 33 → 65; E1M1 `Q` 3,462 | re-evaluated, not re-proved (M3h8 §8); measured in step 13 |
| `spechit` | ≤ 8 per position check, every radius < 64 | radii 16, 20 | rule 6 unchanged |
| Hitscan intercepts | lines of the cells along the trace + things in those cells ≤ 128 | E1M1 window 28 lines; crude bound 28 + 65 = 93 | the M3h6 intercept witness stays unreachable under the crude bound; no new load rule |
| Slide intercepts | window lines ≤ 128 | 28 | unchanged (lines only) |
| Sight queries | 64 per operation, monsters only | unchanged | decorations never queried |
| Scrollers | ≤ 64 | 8 | rule 5 unchanged |
| Blockmap links | `N` per cell at most | 65 | no fixed-capacity store (linked lists) |
| Sector thing lists | `N` | 65 | linked lists |
| `MAX_BONUS` | `6 × P` | 96 | unchanged |

No fixed-capacity store grows in M3h9a; no arithmetic certificate has a roster term.
For M3h9b (pickups 64, §8): live actors `1 + 8 + 64 + 208 + 8 + 32 = 321 > 320`, so the
slot count must become policy-specific (384 recommended) and `N` = 113 breaks the crude
intercept bound (28 + 113 = 141 > 128), so the hitscan intercept argument must count
things in the cells along a trace at load (a per-map rule) or accept the witness as
reachable; both are M3h9b design items, recorded here so the owner sees them early.

## 6. New owner decisions (E1–E8)

Each assessment answers: (a) consistent with section 4 of the brief and M3h8 N1–N9a?
(b) admits no behaviour that silently diverges from vanilla? (c) adds a new kind of
fidelity gap beyond refuse-the-tic witnesses? (d) bounded stores only?

**E1. Two-sided scrolling walls (48) under `PlayerMonstersE1M1V1`.** Options: (i) admit
48 on two-sided lines, blocking or not, faithfully (N1 option (i)), under this policy
only; (ii) non-blocking two-sided only; (iii) keep rejecting. **Recommendation: (i).**
(a) yes: N1 is reopened for the new policy only and every older policy keeps rejecting
(the rule stays in `validate_specials` and `validate_movers` for wires 18 and 19);
(b) yes, the three effects are evidenced on two-sided lines in C (§7); (c) no; (d) yes
(rules 5 and 6). (ii) is narrower for no benefit; (iii) keeps E1M1 unloadable.

**E2. Static decorations.** Options: (i) admit the seven E1M1 types faithfully with a
bound of 32 per map; (ii) admit every static (`tics` −1, no action, not shootable, not
special) `mobjinfo` type with a DoomEd number, same bound; (iii) defer. **Recommendation:
(i)**, with the admission written as a predicate over the info table (static, not
shootable, not special, not `MF_NOBLOCKMAP`/`MF_NOSECTOR`/`MF_SPAWNCEILING`) **restricted
to an explicit list of the seven DoomEd numbers**, so that (ii) later is a list change
with its own evidence (hanging corpses are `MF_SPAWNCEILING|MF_NOGRAVITY`, a new height
path). (a) yes; (b) yes, §2.5–2.7 show no body beyond the existing ones; (c) no; (d) yes.

**E3. Barrels (2035).** Options: (i) admit barrels fully in this milestone (§2.8: the
radius attack with its overflow quirk, chain explosions, thrust, the pain draw, link-order
dependence); (ii) admit barrels as inert solid shootable things and refuse any tic that
would damage one (a witness); (iii) defer barrels to the next part, keeping 2035
`UnsupportedType` at load. **Recommendation: (iii).** (a) yes; (b) yes, a load rejection
diverges from nothing; (c) no for (iii); **(ii) would refuse defined vanilla play
(shooting a barrel) and is a new kind of gap**, which the brief's stop rule forbids;
(i) is faithful but is a monster-sized body set and would push this milestone far past
M3h8 (§8). Barrels are the first item of M3h9b.

**E4. Skill.** Options: (i) medium only, as every player policy already fixes
(`ActorOptions::default()`), newly evidenced; (ii) also admit easy and hard, which differ
from medium only in the spawn filter (§2.3); (iii) also baby or nightmare. **Recommendation:
(i)** now; (ii) is a cheap later option (an `ActorOptions` skill field accepted by this
policy, with the filter evidenced per skill); (iii) changes ammo, damage, reaction time
and the state tables and is out of scope. (a) yes; (b) yes; (c) no; (d) yes. Note that on
medium E1M1 has 6 monsters, so the monster bound of 8 needs no change for the real run.

**E5. Roster bounds in this part.** Options: (i) keep 8 monsters and 16 pickups, add
decorations ≤ 32 and re-evaluate the §5.3 budgets; (ii) also raise pickups to 64 now.
**Recommendation: (i)**; (ii) needs policy-specific actor slots and the intercept
argument of §5.3 and belongs with barrels in M3h9b, where the real-map `N` is known. (a)
yes, older bounds untouched; (b) yes; (c) no; (d) yes.

**E6. Split.** Options: (i) one milestone for everything in the brief; (ii) M3h9a
(this run): scrollers, decorations, skill/spawn evidence, DSIM36, wire 20; M3h9b:
barrels and pickup scale (slots, intercept argument, `N` re-evaluation); M3h9c: the
local-only real-E1M1 harness (§10) and the matched run; (iii) a/b only with the harness in
b. **Recommendation: (ii).** §8 estimates (i) at two to three times M3h8; (ii) keeps
each part reviewable and the harness, which needs a genuine-C level loader, gets its
own evidence record.

**E7. IWAD tool additions.** M3h8 §15 said adding the fixpoint round cap, door-kind
sharing, spawn-rule and case-(d) counts needs owner approval. Options: (i) add the
counts-only THING census now (done, §3) and the spawn-rule count (centre parity and box
against one-sided walls) in M3h9c; (ii) nothing beyond the census. **Recommendation:
(i)**; the census is required by the brief and prints no coordinate; the spawn-rule
count is the one §13 item that decides whether E1M1 loads at all and is pass/fail only.

**E8. DSIM36 content.** Options: (i) version 36 = the DSIM35 layout with decorations as
ordinary actor records (type, state, flags, radius, height, spawnpoint, `lastlook`) and
no new field or event; (ii) also encode the scroll offsets. **Recommendation: (i)**: the
offset is `t × FRACUNIT` (§2.4), derivable and presentation-only; a reader
`decode_e1m1` accepts 36 only and admits the seven types and their states; every earlier
reader rejects 36; stripping nothing recovers DSIM35 for a capture without decorations
(the same bytes except the version). A later part that adds barrels adds their events.

**Unattended rule.** Every decision above has a clear recommendation consistent with
section 4, admits no silent divergence, adds no new kind of fidelity gap and needs no
unbounded store; the split chooses the first part. **E1–E8 are provisionally adopted,
pending owner confirmation (2026-11-03), and Phase 2 proceeds with M3h9a.**

## 7. Evidence plan (C first, genuine bodies)

New generator set `tools/reference/m3h9_{geometry,cases,reference}.py` layered on
`m3h8_*` as M3h8 layered on M3h7, outputs `tests/reference/m3h9_*` (trace `.dref`,
`cases.csv`, `geometry_{i}.bin`, `maps.rs`, `contract.json`), strict, UBSan and
ASan/UBSan traces byte-identical. Rooms and scenarios:

| Room | Scenarios (names) | What they pin |
| --- | --- | --- |
| `scrollhall`: a hall with a non-blocking two-sided 48 across it, a door beyond, a monster alcove | `scroll_cross_noop` (player crosses, nothing happens, no draw); `scroll_spechit_nodir` (a Zombieman's failed move with the 48 line in `spechit`: `DI_NODIR`, `P_UseSpecialLine` false, the next `P_NewChaseDir` draws); `use_scroll_wall_stops_trace` (`BT_USE` through the 48 toward the door: trace stopped, door stays shut, no `sfx_noway`); `shoot_through_scroller` (pistol trace unaffected); `fireball_through_scroller` | §2.4 effects |
| `scrollblock`: a 48 line with `ML_BLOCKING` | `blocking_scroller_wall` (player and monster blocked, nothing in `spechit`) | §2.4 blocking |
| `dense8`: eight two-sided 48 lines in one 2 × 2 window | `dense_spechit_8` (position checks filling `spechit` to 8 with no overflow; the sanitizer build asserts it) and the Rust-only `dense9` (`SpecialCross` witness) | rule 6 |
| `gallery`: the seven decoration types on a flat floor with the player and a Zombieman and an Imp | `walk_into_lamp`, `walk_over_gore` (block / pass), `monster_path_around_pillar` (`P_Move` failures and `P_NewChaseDir` draws), `fireball_hits_candelabra` (explodes), `fireball_over_corpse` (passes), `pistol_through_lamp` (hitscan passes, puff beyond), `monster_hitscan_through_pillar` | §2.6 |
| `liftroom`: a lamp and a corpse on the M3h8 lift sector; a pillar under a door | `decorations_ride_lift`, `door_closes_on_pillar` (no `nofit`, ceiling push) | §2.7 |
| `census`: THINGs of every admitted kind carrying each skill-bit combination, bit 16 and ambush, plus a sector-1 light | `spawn_filter_medium` (which records spawn, the draws per record, `totalkills`/`totalitems`, thinker order, `prndindex` after `P_SpawnSpecials`), and C-only `spawn_filter_easy`/`hard` rows for the record | §2.1–2.4 |

Each scenario's room is also loaded by the Rust replay (`m3h9_replay`) and by the new
example `player_e1m1` (name chosen for the policy, not a real-map claim). Independent
DSIM36 projection: `tools/reference/e1m1_snapshot.py` extending `movers_snapshot.py`
(version 36, policy 20, the decoration actor rows) and `m3h9_snapshot_goldens.py`
(`tests/reference/m3h9_snapshots.csv`, `m3h9_events.dref`, `m3h9_state_*.bin`,
`m3h9_snapshot_contract.json`), written from C rows before the Rust writer.

Rust-only evidence: rejection tests (barrels, a hanging corpse type, 33 decorations,
two-sided 48 under wires 18 and 19, 65 scrollers), rollback/atomicity of a failed spawn
batch, the `dense9` witness, soaks, DSIM36 reader tests (36 only; 35 rejected; decoration
state validation), fuzz target `player_e1m1` with a seeder.

## 8. Size estimate and split

M3h8 was 130 files, +20,488 / −408 lines. M3h9a as planned: C fixtures and generators
≈ 6–8k (five rooms, ≈ 16 scenarios, contracts), Rust ≈ 3–4k (policy plumbing, decoration
admission and spawn, DSIM36 writer and reader, replay, example, tests, fuzz target),
tools ≈ 1k, docs ≈ 2–3k: **≈ 12–16k lines, below M3h8**. Barrels would add a body set
and events comparable to the Imp milestone (≈ 8–10k); the real-map harness needs a
genuine-C level loader and a second encoder path (≈ 5k plus the C adapter); pickup scale
touches bounds, slots and the intercept argument (≈ 2k). Together ≈ 30–35k: hence E6.

## 9. Implementation plan (one commit per step, pushed; evidence before runtime)

1. **Read-only audit.** Confirm the Rust `PIT_CheckThing` flag tests (§2.6), the missile
   branch, intercept collection and `height_clip` for non-shootable solid and flags-0
   things; list every divergence. No behaviour change.
2. **C evidence.** `m3h9_geometry.py`, `m3h9_cases.py`, `m3h9_reference.py` with the §7
   rooms, strict + `--overflow-check` + `--address-check`; commit `tests/reference/m3h9_*`.
3. **DSIM36 projection.** `e1m1_snapshot.py`, `m3h9_snapshot_goldens.py`, the draft
   STATE_FORMAT "DSIM v36" section, goldens committed.
4. **Policy plumbing.** `PlayerMonstersE1M1V1` (wire 20, context
   `PLAYER-MONSTERS-E1M1-1-MEDIUM-v1`, `has_e1m1`, every inherited predicate),
   `Simulation::original_player_e1m1_map`, the version ladder (36), range checks,
   `policy()`; captures of the policy identical to DSIM35 until step 7 except the
   version; rejection of wires 18/19 unchanged.
5. **Scrollers.** `validate_e1m1` admits two-sided 48 (blocking or not) under wire 20
   only; rules 5 and 6 unchanged; tests for both older policies still rejecting.
6. **Decorations.** The seven-type list and static predicate; `MAX_DECORATIONS` 32 and
   `DecorationLimit`; spawn through `spawn_placed_thing` (the `lastlook` draw, no tics
   draw, flags with ambush, health 1000); actor role `Decoration`; blocking and clip
   paths per step 1; `MAX_LINKED_THINGS` under this policy = 65 and the §5.3 table in
   code comments.
7. **DSIM36 writer and reader.** `decode_e1m1`, the decoration actor validation, the
   goldens of step 3 matched byte for byte by the Rust writer; `Diagnostic` round trip.
8. **Replay, example, support module.** `m3h9_replay.rs`, `tests/support/m3h9.rs`,
   `examples/player_e1m1.rs`, `tools/reference/check_player_e1m1.py`, CI steps.
9. **Tests.** Rejections, rollback, soaks, DSIM reader negatives, the `dense9` witness,
   skill-filter tests, decoration ride and block tests, the §5.3 formulas as asserted
   constants.
10. **Fuzz.** `fuzz/fuzz_targets/player_e1m1.rs`, `tools/seed_player_e1m1_fuzz.py`;
    seed replay, one coverage-guided and one ASan campaign (`detect_leaks=0` disclosed),
    plus seed replays of the older targets touched.
11. **Docs.** COMPATIBILITY section and fidelity gaps, STATE_FORMAT DSIM v36 final,
    TESTING commands, ROADMAP, ARCHITECTURE/TESTING as needed, README one row,
    validation/M3H9.md (observed results only), this note's status.
12. **Independent review** of the full diff against the C source; fix and record.
13. **Matched timings** against master, alone, all digests compared, callgrind for any
    older-policy slowdown over 3 %, the M3h8 `player_demon retirement` and M3h6
    `player_armor` leftovers re-checked.
14. **Full gates** from clean; draft PR; ready only when every CI job is green on the
    exact head; scratch deleted and space reported.

## 10. Real-E1M1 evidence without committing the IWAD (design for M3h9c)

CI never sees the IWAD. Two complementary pieces:

**(a) Local-only harness `tools/local/e1m1_reference.py`** (never run by CI; a no-op
without `DOOM_IWAD_DIR`). It compiles a headless linuxdoom-1.10 from genuine sources:
`p_*.c`, `m_random.c`, `m_bbox.c`, `m_fixed.c`, `tables.c`, `info.c`, `w_wad.c`,
`z_zone.c`, `r_main.c`'s `R_PointToAngle2`/`R_PointInSubsector` and the other pure
functions the game code calls, with declared adapters (cited line by line in the
contract) replacing `I_*`, `S_*`, `R_*` rendering, `HU_`/`ST_`/`AM_`/`V_`/`D_` and the
menu, `precache = 0`, and a minimal driver that performs `G_InitNew(sk_medium, 1, 1)`'s
observable steps (`M_ClearRandom`, `gameskill`, `PST_REBORN`), `P_SetupLevel(1, 1, 0,
sk_medium)` from the real lumps and then, per tic, fills `players[0].cmd` from a script and
calls `P_Ticker`. After each tic it writes the same row kinds the synthetic harness
writes (every mobj, sector heights and specials, line specials, buttons, player, RNG
index, counters) to a scratch directory; the independent `e1m1_snapshot.py` projects each
tic's rows to DSIM36 bytes and digests them. The Rust side loads the same lumps through
`doom-wad`/`doom-map` (the `DOOM_IWAD_DIR` path that already exists for the `#[ignore]`
tests), runs `Simulation::original_player_e1m1_map` for the same tics and the same
commands, and prints its per-tic digests. The comparison reports the first differing
tic, if any. Scripts: `idle` (N zero commands), `walk_to_exit` (a hand-authored command
sequence refined locally until the exit tic is reached; the sequence is not IWAD data and
is committed as `tests/reference/e1m1_scripts/*.csv`), `fight` (turn toward the first
Zombieman and fire, then advance), `use_everything` (use on every door and the lift).
Committed: `docs/validation/M3H9C.md` with per-script pass/fail, tic counts, the first
and last digests and a digest of the digest list; never a coordinate, lump name or lump
byte. Scratch rows are deleted after the record.

**(b) Synthetic CI scenarios at E1M1 scale** (M3h9b/c): a generated room with 9 mover
sectors, 8 two-sided scrollers in one window, 27 decorations, 6 barrels, 6 monsters and
52 pickups of E1M1's kinds and skill flags, exercised by the C harness and the Rust
replay, so that every new mechanic is covered in CI at the real counts without IWAD
content. Its geometry is synthetic and need not resemble E1M1.

**Still unmeasured on E1M1** (M3h8 §13 item 4; E7): the spawn rule over every THING, the
door-kind sharing count, the `ML_SECRET` DR line and case-(d) spawns. The spawn-rule
count is added to `iwad_counts.py` in M3h9c as a pass/fail fact per slot.

## 11. Policy proposal

`OriginalExecutionProfile::PlayerMonstersE1M1V1`, wire 20, DSIM36, context
`PLAYER-MONSTERS-E1M1-1-MEDIUM-v1`, constructor `Simulation::original_player_e1m1_map`,
a superset of `PlayerMonstersMoversV1`: every Stage A and Stage B predicate plus
`has_e1m1` (two-sided scrollers and static decorations). Load rules: the M3h8 rules with
rule 1 relaxed for 48 on two-sided lines, rules 5 and 6 unchanged, plus decorations ≤ 32
and the seven-type list. Runtime witnesses unchanged. DSIM36 as E8. Older policies: byte
and behaviour identical, confirmed by the unchanged goldens and matched digests.

## 12. Follow-ups, risks, status

- **M3h9b**: barrels (§2.8 bodies, events for explosions, the link-order RNG
  dependence, `P_RadiusAttack`'s wrapping shift as a declared source quirk), pickups
  ≤ 64, policy-specific actor slots (384), the hitscan intercept argument with things
  per cell, `N` re-evaluation and the `P_ChangeSector` measurement; optional E4 (ii).
- **M3h9c**: the harness of §10, the spawn-rule count, the E1M1-scale synthetic room,
  the matched run; the `walk_to_exit` script authoring is iterative and local.
- **Step 1 audit result**: the collision (`movement.rs` `check_things`, `Solidity` from
  `MF_SOLID`), missile (`projectiles.rs` `missile_contact`), hitscan and aim
  (`combat_trace.rs`), change-sector and height-clip (`planes.rs`) and thinker
  (`thinkers.rs`) paths already treat a non-shootable solid or flags-0 thing as C does;
  every gap is a role or type gate: the per-tic monster preflights (`monsters.rs`
  `monster_actor`, `demon_map_spawn.rs` pickup/roster/suffix order), spawn
  classification (`spawning.rs:418–471`, the owner rewrite), `require_role_for` and
  `state_role_for`, `validate_type_original`, the DSIM reader's `identity`,
  `gibs_admitted` and the state-keyed `S_GIBS` rule (`diagnostic.rs:1256–1275`, which
  must become type-keyed since `MT_MISC71` spawns in `S_GIBS` with height 16), and the
  shared `MAX_LINKED_THINGS` static assert (a per-policy constant instead). The registry
  already holds the seven types and their states. `Simulation::set_momentum` should
  refuse a decoration push at the API as it does for items.
- **Risks**: hitscan intercepts now include decorations, so a long trace through
  a crowded corridor adds thing intercepts (witness unchanged, budget in §5.3); the
  `MF_AMBUSH` bit on decorations must survive into the flags word of the digest.
- **Quirks recorded, not emulated**: the `P_LoadThings` break on Doom II types (§2.2,
  unreachable on the owner's IWAD), the unchecked `linespeciallist` (rejected at 65),
  the unguarded type 0 (rejected), the eleventh deathmatch start dropped (faithful;
  E1M6 has 11).

**Status / next step.** Phase 1 is complete with this note and the census. E1–E8 are
provisionally adopted (§6). Phase 2 progress (M3h9a):

- Step 1 (audit) is recorded in §12 above.
- Step 2 (C evidence) is done: `tools/reference/m3h9_{geometry,cases,reference}.py`,
  outputs `tests/reference/m3h9_*`. It has 20 scenarios over 6 rooms and 1,157 tics, 18
  replayed by Rust and 2 C-only (easy and hard spawn filters). The strict, UBSan and
  ASan/UBSan traces are byte-identical. No new genuine body or adapter was needed.
- Step 3 (DSIM36 projection) is done: `tools/reference/e1m1_snapshot.py` and
  `m3h9_snapshot_goldens.py` produce 290 captures and 15 pinned fixtures. A DSIM36
  capture differs from the DSIM35 projection of the same rows only in the version, the
  profile and the context bytes. The draft section is in STATE_FORMAT.
- Steps 4–6 are done: the profile (wire 20, `has_e1m1`, `decode_e1m1`, version 36),
  two-sided 48 under wire 20 only, and the seven decoration types with
  `MAX_DECORATIONS` 32 and `DecorationLimit`. The workspace tests pass and the M3h8
  goldens and replay are unchanged.

Next: step 7, the Rust DSIM36 writer and reader matched byte for byte against the
step 3 goldens; then step 8, the replay, example and CI check.
