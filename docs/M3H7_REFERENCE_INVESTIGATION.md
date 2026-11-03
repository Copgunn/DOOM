# M3h7 — Doors, lifts, line and sector specials, and sky

This note records the original-C facts, the Rust inventory, the proposed admitted
domain with its proof obligations, the evidence plan, the implementation plan and the
owner decisions for the milestone after M3h6, named by the owner: doors, lifts and the
line and sector specials, plus sky, as listed in the
[M3h6 E1M1 readiness report](M3H6_REFERENCE_INVESTIGATION.md#9-e1m1-readiness-report).
It was written and committed before any runtime change (Phase 1). Support and limits
belong to [COMPATIBILITY](COMPATIBILITY.md); DSIM schemas belong to
[STATE_FORMAT](STATE_FORMAT.md); measured observations will belong to the evidence
record written in Phase 2. All references are to the preserved `linuxdoom-1.10/`
sources at `a77dfb96`. Every number is in map units unless it says raw (16.16 bits) or
tics. Line numbers name the first line of the cited statement. No IWAD was available
for this note: every real-map figure is quoted from the committed
[M3h6 counts](validation/M3H6_IWAD_COUNTS.md) and M3h6 §9, and every figure that is not
committed is listed in §5.6 as **to be measured locally by the owner**.

## 1. Starting point

`PlayerMonstersRealMapV1` (wire 17, DSIM33,
[COMPATIBILITY](COMPATIBILITY.md#original-monsters-at-classic-map-scale)) admits the
diagonal world of M3h5 at the WAD format's own table bounds, with a per-map arithmetic
certificate (i)–(viii), the slide-window rule, the 128-intercept runtime witness,
player starts as records, the eight-monster / sixteen-pickup / eight-drop roster, 320
slots and 208 live fireballs ([M3h6 §4](M3H6_REFERENCE_INVESTIGATION.md#4-the-admitted-domain-and-the-proof)).
It **rejects at load** every nonzero line special and sector special
(`PursuitError::Specials`, `demon_expanded_map.rs:69–73`), every `F_SKY1` ceiling
(`CombatError::Sky`, 80–86), heights outside ±512 (74–79), `BT_USE`
(`SimError::UnsupportedButtons`, `player_commands.rs:80–91`) and every THING of an
unadmitted type ([M3h6 §4.8](M3H6_REFERENCE_INVESTIGATION.md#48-what-stays-rejected-at-load)).
Sector heights are runtime state (`Simulation::sectors`, `lib.rs:518`) but no code path
writes them after construction (`lib.rs:684–689`).

E1M1 after M3h6 needs, besides the roster scale and eight decoration and barrel types
(not part of this milestone), exactly (M3h6 §9):

| E1M1 item | Count | Source body |
| --- | ---: | --- |
| Line 1, DR manual door raise | 8 | `EV_VerticalDoor`, `T_VerticalDoor` |
| Line 48, scrolling wall | 8 | `P_UpdateSpecials` |
| Line 11, S1 exit switch | 1 | `P_UseSpecialLine`, `G_ExitLevel` |
| Line 36, W1 turbo lower floor | 1 | `EV_DoFloor(turboLower)`, `T_MoveFloor` |
| Line 38, W1 lower floor to lowest | 1 | `EV_DoFloor(lowerFloorToLowest)` |
| Line 63, SR door raise | 1 | `EV_DoDoor(normal)`, switch button |
| Line 88, WR lift | 1 | `EV_DoPlat(downWaitUpStay)`, `T_PlatRaise` |
| Line 109, W1 blazing door open | 1 | `EV_DoDoor(blazeOpen)` |
| Sector 7, 5% damage | 4 | `P_PlayerInSpecialSector` |
| Sector 9, secret | 3 | `P_SpawnSpecials`, `P_PlayerInSpecialSector` |
| Sector 8, glowing light | 2 | `P_SpawnGlowingLight`, `T_Glow` |
| Sector 1, blinking light | 1 | `P_SpawnLightFlash`, `T_LightFlash` |
| Sector 12, synchronized slow strobe | 1 | `P_SpawnStrobeFlash(…, 1)`, `T_StrobeFlash` |
| `F_SKY1` ceilings | 7 sectors | `P_XYMovement`, `PTR_ShootTraverse` |

22 special lines of which 5 carry a tag, 11 special sectors of which 5 carry a tag
(M3h6 §9). Which lines and sectors those tags join, whether any door sector is also a
sky sector, and every per-map figure beyond these counts are not committed (§5.6).

## 2. Original C facts

### 2.1 Load order, thinker order and the tic

- **`P_SetupLevel`** ([p_setup.c:584](../linuxdoom-1.10/p_setup.c)): `leveltime = 0`
  (646); BLOCKMAP … SEGS, REJECT, `P_GroupLines` (649–660); `P_LoadThings` (664); then
  `P_SpawnSpecials` (682). **Every special thinker created at load follows every
  map-THING mobj in the thinker list**, and every `P_Random` call of `P_SpawnSpecials`
  follows every `P_Random` call of THING spawning (`P_SpawnMobj`'s `lastlook`,
  [p_mobj.c:506](../linuxdoom-1.10/p_mobj.c)).
- **`P_GroupLines`** computes each sector's `blockbox`, its line bbox extended by
  `MAXRADIUS` and clamped to the grid (p_setup.c:560–574), and its `soundorg`, the bbox
  center (556–557). `P_ChangeSector` iterates that blockbox (§2.6).
- **Thinkers** ([p_tick.c](../linuxdoom-1.10/p_tick.c)): `P_AddThinker` appends to the one
  list (65–71); `P_RemoveThinker` only marks (80–84); `P_RunThinkers` walks the list,
  freeing marked entries when reached and running the others (101–122), so a thinker
  appended during the pass (a door opened by a monster in `P_Move`, a missile) runs in
  the same pass. Mobjs and sector thinkers share this list; their relative order is the
  order of creation.
- **`P_Ticker`** (130): `P_PlayerThink` for each player (148–150), `P_RunThinkers`
  (152), `P_UpdateSpecials` (153), `P_RespawnSpecials` (154), `leveltime++` (157). So
  during tic *n* of a level `leveltime == n` in every body below. `P_RespawnSpecials`
  ([p_mobj.c:578](../linuxdoom-1.10/p_mobj.c)) returns unless `deathmatch == 2` (591).
- **`P_PlayerThink`** ([p_user.c](../linuxdoom-1.10/p_user.c)): a dead player runs
  `P_DeathThink` and returns (258–262) — no specials, no use; otherwise `P_MovePlayer`
  (267–270, thrust only; the move itself happens in the player's mobj thinker),
  `P_CalcHeight` (272), **`P_PlayerInSpecialSector` if `sector->special`** (274–275),
  weapon change (283–319), **`BT_USE`: `P_UseLines` once per press, gated by
  `player->usedown`** (322–331), `P_MovePsprites` (334). A door opened by the player is
  appended before the thinker pass and moves in the same tic.
- **The savegame is not an ordering authority**: `P_ArchiveThinkers` writes mobjs only
  ([p_saveg.c:232](../linuxdoom-1.10/p_saveg.c)) and `P_ArchiveSpecials` writes the
  sector thinkers afterwards (355–467), so a loaded game runs every special after every
  mobj, and `specials_e` has no fire-flicker kind (331–338). DSIM must keep the
  interleaved creation order instead.

### 2.2 Specials at load: `P_SpawnSpecials`

`P_SpawnSpecials` ([p_spec.c:1239](../linuxdoom-1.10/p_spec.c)) scans sectors in index
order (1270–1332): 1 → `P_SpawnLightFlash` (1278); 2, 3 → `P_SpawnStrobeFlash` with
`FASTDARK` or `SLOWDARK` and `inSync = 0` (1283–1291); 4 → fast strobe, then `sector->special = 4` again
(1293–1297); 8 → `P_SpawnGlowingLight` (1299); **9 → `totalsecret++` only** (1303–1306);
10 → `P_SpawnDoorCloseIn30` (1308); 12, 13 → `P_SpawnStrobeFlash(…, 1)` (1313–1321); 14
→ `P_SpawnDoorRaiseIn5Mins` (1323); 17 → `P_SpawnFireFlicker` (1328). Types 5, 7, 11, 16
spawn nothing. It then collects type-48 lines into `linespeciallist[MAXLINEANIMS = 64]`
(141, 1336–1347) **without a bound check**, and clears `activeceilings`,
`activeplats` and `buttonlist` (1351–1358). The `-avg`/`-timer` level timer applies to
deathmatch only (1253–1267).

**Light spawners clear the sector special.** `P_SpawnFireFlicker`
([p_lights.c:74](../linuxdoom-1.10/p_lights.c)), `P_SpawnLightFlash` (129),
`P_SpawnStrobeFlash` (203), `P_SpawnGlowingLight` (355), `P_SpawnDoorCloseIn30`
([p_doors.c:514](../linuxdoom-1.10/p_doors.c)) and `P_SpawnDoorRaiseIn5Mins` (539) all set
`sector->special = 0`. After load only types 4, 5, 7, 9, 11, 16 and unknown values stay
nonzero; this is why `P_PlayerInSpecialSector`'s `I_Error` default (§2.3) is never hit
by a light sector. The M3h6 note's summary of this function (§2.5) is correct but does
not mention the clearing; it matters because the sector special becomes runtime state.

### 2.3 Sector specials during play

**`P_PlayerInSpecialSector`** (p_spec.c:1009), called for a live player whose
subsector's sector has a nonzero special:

- It returns unless `player->mo->z == sector->floorheight` (1016) — the **sector's**
  floor under the player's center, not `mo->floorz`. A player standing on a ledge whose
  center overhangs a nukage pit is not hurt.
- 5 (10%) and 7 (5%): if no `pw_ironfeet` and `!(leveltime & 0x1f)`,
  `P_DamageMobj(player->mo, NULL, NULL, 10 | 5)` (1022–1034). No `P_Random` of its own.
- 4 and 16 (20%): `if (!ironfeet || P_Random() < 5)` then the same 32-tic gate and 20
  damage (1036–1046). **The draw is short-circuited away without the radiation suit**,
  which no admitted roster carries.
- 9: `player->secretcount++; sector->special = 0` (1048–1052).
- 11: clears `CF_GODMODE`, 20 damage on the gate, `G_ExitLevel` at health ≤ 10
  (1054–1063).
- Any other value: `I_Error` (1065–1068).

**Environmental damage** is `P_DamageMobj(target, NULL, NULL, n)`
([p_inter.c:775](../linuxdoom-1.10/p_inter.c)): no thrust and no fall-forward draw
(`inflictor` null, 806–832); for a player the special-11 cap (838–842), armor absorption
(854–869), `player->attacker = NULL` (874), `damagecount`; on death `P_KillMobj(NULL,
target)` (887–891); otherwise the pain draw `P_Random() < painchance` (894). In
`P_KillMobj` (668) a null source credits a `MF_COUNTKILL` death to `players[0]`
(692–697), **counts an environmental player death as one of the player's own frags**
(699–703), and draws `tics -= P_Random() & 3` (726).

### 2.4 Light thinkers and their random draws

All structures are in [p_spec.h](../linuxdoom-1.10/p_spec.h) (`fireflicker_t` 124,
`lightflash_t` 136, `strobe_t` 150, `glow_t` 165; `GLOWSPEED 8`, `STROBEBRIGHT 5`,
`FASTDARK 15`, `SLOWDARK 35` at 176–179). `P_FindMinSurroundingLight`
(p_spec.c:454) reads neighbors through `getNextSector` (251), which requires
`ML_TWOSIDED` (255).

| Thinker | Spawn draws | Per-tic body | Draws while running |
| --- | --- | --- | --- |
| `T_LightFlash` (p_lights.c:98), type 1 | `count = (P_Random() & 64) + 1` (142); `maxlight` = own, `minlight` = darkest neighbor, `maxtime 64`, `mintime 7` | `--count`; at 0 toggles between `maxlight` and `minlight` by comparing the **current** `lightlevel == maxlight` (103) | **one per toggle**: `(P_Random() & 7) + 1` going dark (106), `(P_Random() & 64) + 1` going bright (111) |
| `T_StrobeFlash` (155), types 2, 3, 4 unsynced; 12, 13 synced | unsynced `count = (P_Random() & 7) + 1` (206); synced `count = 1` (208), no draw; `minlight` 0 if equal to `maxlight` (199–200) | toggles `brighttime 5` / `darktime 15 or 35` | none |
| `T_Glow` (314), type 8 | none; starts going down (353) | ±8 per tic, bouncing at `minlight`/`maxlight` (316–337) | none |
| `T_FireFlicker` (46), type 17 | none; `minlight` = darkest neighbor + 16 (83) | every 4 tics | `amount = (P_Random() & 3) * 16` (53) |

`lightlevel` is read in game code only by these thinkers and by `EV_LightTurnOn`,
`EV_TurnTagLightsOff`, `P_FindMinSurroundingLight` (p_lights.c:236–307, p_spec.c:472);
the renderer reads it for presentation. It is nevertheless gameplay-relevant state:
`T_LightFlash` branches on it and consumes the shared `P_Random` stream at each toggle.
`P_Random() & 64` masks with the value 64, not 63, so a
blinking light's bright phase lasts 1 or 65 tics and nothing between. For E1M1 the load adds **one** draw (its one type-1 sector; its type-12 sector is
synced, its type-8 sectors draw nothing) after every THING draw, and its blinking
light draws once per toggle for the whole level.

### 2.5 Line triggers: crossing, use, shooting

- **`PIT_CheckLine`** ([p_map.c:189](../linuxdoom-1.10/p_map.c)) appends a line to
  `spechit[numspechit++]` only after it passed the bbox and `P_BoxOnLineSide` tests and
  is two-sided and not blocking (211–221), with `if (ld->special)` (240–244), **without
  a bound check** against `MAXSPECIALCROSS 8` (67–69). It also sets `ceilingline` to the
  latest line that strictly lowered `tmceilingz` (227–231). `P_CheckPosition` (375)
  resets `ceilingline` (400) and `numspechit` (410), checks things in
  `[tmbbox ± MAXRADIUS]` cells first (420–428) and lines in the `tmbbox` cells only
  (431–439), `bx` outer and `by` inner, returning at the first blocker.
- **`P_TryMove`** (451): after the move is accepted and relinked, unless
  `MF_TELEPORT|MF_NOCLIP`, it walks `spechit` **in reverse** (`while (numspechit--)`,
  502) and calls `P_CrossSpecialLine(ld - lines, oldside, thing)` for every line whose
  `P_PointOnLineSide` differs between the old and new centers and whose special is still
  nonzero (505–512). Every successful step of `P_SlideMove` goes through it.
- **`P_CrossSpecialLine`** (p_spec.c:492): a non-player returns at once if it is one of
  six missile types including `MT_TROOPSHOT` (506–518), and otherwise acts only on
  39, 97, 125, 126 (teleports), **4 (W1 door raise), 10 and 88 (lifts)** (521–534). So
  **a monster crossing E1M1's WR lift line lowers the lift**. Player cases used by E1M1:
  36 `EV_DoFloor(turboLower)`, special cleared (640–644); 38
  `EV_DoFloor(lowerFloorToLowest)`, cleared (652–656); 88 `EV_DoPlat(downWaitUpStay, 0)`,
  kept (857–860); 109 `EV_DoDoor(blazeOpen)`, cleared (730–734). A W1 line clears its
  special even when `EV_*` started nothing.
- **`P_UseLines`** (p_map.c:1130): a `PT_ADDLINES` trace of `USERANGE` 64 units from the
  player's center along `angle >> ANGLETOFINESHIFT`, `x2 = x1 + 64 · finecosine[angle]`
  (1140–1147). **`PTR_UseTraverse`** (1095): a line without special continues if its
  opening is positive and otherwise plays `sfx_noway` and stops (1099–1111); a special
  line is used with `side = P_PointOnLineSide(...) == 1` and the trace stops (1113–1122)
  whatever the special does.
- **`P_UseSpecialLine`** ([p_switch.c:276](../linuxdoom-1.10/p_switch.c)): back-side
  use returns false except for 124 (284–297); a non-player returns false for an
  `ML_SECRET` line or anything but 1, 32, 33, 34 (301–319); 1, 26–28, 31–34, 117, 118 call
  `EV_VerticalDoor` (326–339); 11 calls `P_ChangeSwitchTexture(line, 0)` then
  `G_ExitLevel` (359–363); 63 calls `EV_DoDoor(normal)` and on success
  `P_ChangeSwitchTexture(line, 1)` (550–554). It returns **true for every front-side
  player use of any special**, including specials with no use case (a scrolling wall:
  nothing happens, the trace stops, no `sfx_noway`).
- **Monsters open doors by bumping.** `P_Move` ([p_enemy.c:272](../linuxdoom-1.10/p_enemy.c)):
  when `P_TryMove` fails and the actor does not float, `if (!numspechit) return false`;
  else `movedir = DI_NODIR` and `P_UseSpecialLine(actor, ld, 0)` for each `spechit`
  entry in reverse, returning true if any returned true (310–324). The `movedir` reset
  happens for any special line in `spechit`, even one no monster can use (a two-sided
  scroller), so admitting any two-sided special changes monster steering. The `spechit`
  entries are those added before the failure: none when a thing blocked (things are
  checked first), the lines before the blocker when a line blocked, all of them when the
  fit, headroom, step or dropoff test failed — a closed DR door's line is two-sided with
  zero opening, so it is in `spechit` and the fit test fails. A true return makes
  `P_TryWalk` draw `movecount = P_Random() & 15` (349–357). Monsters always pass side 0,
  so they can open a DR door from either side.
- **`P_ShootSpecialLine`** (p_spec.c:959) is called from `PTR_ShootTraverse` for every
  special line a hitscan reaches, pass-through or not (p_map.c:919–920); it acts only on
  24, 46, 47 (980–999), none of which E1M1 has.
- **Tags**: `P_FindSectorFromLineTag` (p_spec.c:434) returns sectors in ascending index
  order with `sectors[i].tag == line->tag` (440–442) — **tag 0 selects every untagged
  sector**; every `EV_*` skips a sector whose `specialdata` is set (e.g. p_doors.c:278,
  [p_floor.c:277](../linuxdoom-1.10/p_floor.c), [p_plats.c:168](../linuxdoom-1.10/p_plats.c)).

### 2.6 Moving planes and `P_ChangeSector`

- **`T_MovePlane`** (p_floor.c:49) moves a floor or ceiling by `speed` toward `dest`. If
  the step would pass `dest` it sets the height to `dest`, calls `P_ChangeSector`, and on
  "no fit" restores the old height and calls it again — **but still returns `pastdest`**
  (66–80, 95–109, 133–148, 167–181): a door or lift whose last step is blocked stops short
  and its thinker is removed there. An ordinary step that does not fit restores and
  returns `crushed` (81–92, 110–124, 149–164), except with `crush == true` when moving a
  floor up or a ceiling down, where the height stays and `crushed` is returned (118–119,
  158–159). A rising ceiling never checks (the restore is under `#if 0`, 187–195).
- **`P_ChangeSector`** (p_map.c:1321) sets `nofit = false`, `crushchange = crunch`, and
  calls `P_BlockThingsIterator(x, y, PIT_ChangeSector)` over the sector's `blockbox`,
  **x outer, y inner** (1332–1334). Only BLOCKMAP-linked things are visited: **missiles,
  puffs and blood (`MF_NOBLOCKMAP`) are never re-clipped**, and every linked thing in
  those cells is re-clipped whether or not it touches the sector.
- **`PIT_ChangeSector`** (1257): if `P_ThingHeightClip` succeeds, continue (1261–1265);
  else a thing with `health <= 0` becomes `S_GIBS` with `MF_SOLID` cleared, height and
  radius 0 (1269–1279); a `MF_DROPPED` thing is removed (1282–1288); a non-shootable
  thing is left as it is (1290–1294); otherwise `nofit = true` (1296) and, only if
  `crushchange && !(leveltime & 3)`, 10 damage and a `MT_BLOOD` at mid-height with
  `momx = (P_Random() - P_Random()) << 12` and the same for `momy` (1298–1309).
- **`P_ThingHeightClip`** (530): `onfloor = (z == floorz)`; `P_CheckPosition(thing, x,
  y)` with its result **ignored** (536); `floorz`/`ceilingz` from `tmfloorz`/`tmceilingz`;
  an onfloor thing is set to the new floor (rises and falls with it, 542–546), otherwise
  only pushed down to `ceilingz - height` (550–551); it fails when `ceilingz - floorz <
  height` (554). Three consequences that the Rust collision does not exercise today:
  (1) the clip calls `PIT_CheckThing` with the clipped thing as `tmthing`, so **a player
  re-clipped by a moving sector picks up items overlapping its box**
  (`P_TouchSpecialThing` under `MF_PICKUP`); (2) when a solid thing overlaps the clipped
  one, `P_CheckPosition` returns before the line pass and the clip takes the
  subsector's own floor and ceiling; (3) **a thing never moved since it spawned (map
  items, idle monsters) gets its first box-wide `floorz`**: spawn takes the center
  sector's heights ([p_mobj.c:519–520](../linuxdoom-1.10/p_mobj.c)), so an item or monster
  straddling a step rises onto the step the first time any nearby sector moves.
- **Stuck things.** A non-shootable thing (an undropped pickup, gibs) under a closing
  door does not stop it: it stays with `z == floorz` and `ceilingz - floorz < height`. A
  thing that is not on its floor and is pushed down by a lowering ceiling keeps the
  lowered `z` when the step is undone (550–551 then the restore), so `z < floorz` by up
  to one step can hold until its next `P_ZMovement` lands it.

### 2.7 Doors, lifts and floors

- **Doors** (`vldoor_t`, p_spec.h:343; `VDOORSPEED` 2 units, `VDOORWAIT` 150 tics,
  364–365). **`EV_VerticalDoor`** (p_doors.c:354), the DR door: locks (369–410);
  `sec = sides[line->sidenum[side ^ 1]].sector` with `side = 0` (364, 413) — **the back
  sector, read with no two-sided check**; if `sec->specialdata` is set it is **cast to
  a door whatever thinker it is** and, for 1, 26–28, 117, a closing door reopens and an
  open or waiting door closes at once for a player only ("bad guys never close doors",
  416–437); otherwise the open sound (440–455) and a new door thinker appended
  (459–498) of type `normal` for line 1 with `topheight =
  P_FindLowestCeilingSurrounding(sec) - 4` (497–498). **`EV_DoDoor`** (264): per tagged
  sector without `specialdata`, a door thinker with `blazeOpen` speed 8 and `normal` speed
  2, `topheight` lowest neighbor ceiling − 4, sound only if `topheight != ceiling`
  (320–339). `P_FindLowestCeilingSurrounding` (p_spec.c:382) starts at `MAXINT` (387): a
  sector with no two-sided neighbor gets `MAXINT − 4` as its top.
- **`T_VerticalDoor`** (p_doors.c:63): waiting (`direction 0`) counts `topcountdown` and
  turns a `normal` door down with `sfx_dorcls` (69–97); down moves the ceiling toward the
  floor with `crush = false` (118–123) — `pastdest` removes a `normal` door (136–140),
  `crushed` reverses it with `sfx_doropn` (151–165); up moves toward `topheight` —
  `pastdest` makes a `normal` door wait `topwait` tics (179–183) and removes an `open` or
  `blazeOpen` door (185–190). No draws.
- **Lifts** (`plat_t`, p_spec.h:285; `PLATSPEED` 1, `PLATWAIT` 3, `MAXPLATS` 30,
  304–306). **`EV_DoPlat(downWaitUpStay)`** (p_plats.c:139, 207–218): speed 4, `low` = the
  lowest neighbor floor clamped to the current floor, `high` = the current floor, wait
  105 tics, `status = down`; `P_AddActivePlat` stores it in `activeplats[30]` and
  `I_Error`s on the 31st (288–299). Only `perpetualRaise` draws (`status = P_Random() &
  1`, 246). **`T_PlatRaise`** (52): up with the plat's `crush` (false here, 180); a
  `crushed` result sends it back down (73–79); `pastdest` waits and, for
  `downWaitUpStay`, removes it from `activeplats` and the thinker list (82–94, 301–313);
  down never reverses (108–117); waiting counts down and picks `up` or `down` by
  comparing the floor with `low` (119–127).
- **Floors** (`floormove_t`, p_spec.h:584; `FLOORSPEED` 1, 600). **`EV_DoFloor`**
  (p_floor.c:260): `lowerFloorToLowest` speed 1 down to the lowest neighbor floor
  (299–305); `turboLower` speed 4 down to the **highest** neighbor floor, plus 8 when it
  differs from the current floor (307–315). Because `T_MovePlane`'s down branch sets the
  floor to `dest` as soon as `floor − speed < dest` (68–71), **a turbo lower beside a
  higher neighbor raises the floor to that neighbor + 8 in one tic**. `T_MoveFloor`
  (209) plays `sfx_stnmov` every 8 tics (218–220) and removes itself on `pastdest`
  (222–252). `P_FindNextHighestFloor` (p_spec.c:329) keeps at most 20 neighbors
  (`MAX_ADJOINING_SECTORS`, 326, 355–360); none of E1M1's types call it.
- **Ceilings** ([p_ceilng.c](../linuxdoom-1.10/p_ceilng.c)): `T_MoveCeiling` (52) and
  `EV_DoCeiling` (172) are the only bodies with `crush = true` besides
  `raiseFloorCrush`; E1M1 has none (§4.2).

### 2.8 Switches, buttons, scrolling walls and the level exit

- **`P_ChangeSwitchTexture`** (p_switch.c:201) clears `line->special` when the switch is
  single-use (211–212) **before** testing `line->special == 11` for the exit sound
  (221–222), so the exit switch never plays `sfx_swtchx`; it plays its sound at
  `buttonlist->soundorg`, the first button slot's origin (228, 240, 252), swaps the
  front side's top, middle or bottom texture against `switchlist` (224–262) and, for a
  reusable switch, starts a 35-tic button (`P_StartButton`, 155; `I_Error` with no free
  slot of `MAXBUTTONS 16`, 189; p_spec.h:244–247). `P_UpdateSpecials` counts buttons down
  and restores the texture with `sfx_swtchn` (p_spec.c:1128–1154).
- **What is gameplay here**: the cleared special (a used S1/W1 line is inert), the
  `EV_*` effects and the `I_Error` bounds. Texture numbers are read in game code only by
  `raiseToTexture` (`textureheight`, p_floor.c:372–401), which E1M1 does not use;
  switch textures, button timers and every sound are presentation (`S_StartSound` draws
  `M_Random`, not `P_Random`, [s_sound.c:330, 340](../linuxdoom-1.10/s_sound.c)).
- **Scrolling walls** (48): `sides[sidenum[0]].textureoffset += FRACUNIT` every tic in
  `P_UpdateSpecials` (p_spec.c:1114–1124); `textureoffset` is read in game code nowhere
  else (only the renderer and the savegame, p_saveg.c:151). It is pure presentation and
  equals its load value plus `leveltime` units.
- **`P_UpdateSpecials`** also animates flats and textures from `leveltime` (1100–1110,
  presentation) and runs the deathmatch level timer (1092–1097).
- **`G_ExitLevel`** ([g_game.c:1002](../linuxdoom-1.10/g_game.c)) only sets `gameaction =
  ga_completed`; the current tic finishes (thinkers, specials, `leveltime++`) and the
  next `G_Ticker` ends the level (617–638).

### 2.9 Sky in game code (re-read for this note)

As recorded in [M3h6 §2.4](M3H6_REFERENCE_INVESTIGATION.md#24-sky-in-the-game-code) and
re-checked: `skyflatnum = R_FlatNumForName("F_SKY1")` (g_game.c:454,
[r_sky.h:32](../linuxdoom-1.10/r_sky.h), declared at [r_sky.c:47](../linuxdoom-1.10/r_sky.c));
outside the renderer it is compared only with **ceiling** pics, in two places:

- `P_XYMovement` ([p_mobj.c:173–187](../linuxdoom-1.10/p_mobj.c)): a missile whose
  `P_TryMove` fails is **removed** (`P_RemoveMobj`, no explosion, so **no `tics -=
  P_Random() & 3` draw** of `P_ExplodeMissile`, 96, no radius or contact damage) when
  `ceilingline && ceilingline->backsector && ceilingline->backsector->ceilingpic ==
  skyflatnum`. Because things are checked before lines and `ceilingline` is reset first
  (p_map.c:400, 420–439), a missile refused by a thing always explodes; one refused by a
  line, or by the fit/headroom/step tests after the full pass, tests the latest line
  that strictly lowered the ceiling. It reads the **back** sector of that line, whatever
  side the missile is on. `P_ZMovement` has no sky test: a missile reaching a sky
  ceiling explodes (342–347).
- `PTR_ShootTraverse` (p_map.c:956–965): at `hitline`, a front-sector sky ceiling ends
  the trace without a puff when `z > frontsector->ceilingheight` or the back sector is
  sky too; otherwise `P_SpawnPuff` (968), whose two draws (p_mobj.c:819, 823) are then
  not made. It reads the **front** sector whatever side the shot came from.

Sky floors have no game-code effect. Autoaim, sight, noise and movement have no sky
test. The Rust load check rejects only `F_SKY1` **ceilings** (`flats[1]`,
`demon_expanded_map.rs:80–86`, `combat.rs:494`); COMPATIBILITY's "sky ceilings and floors
stay excluded" overstates the floor half, harmlessly (§8).

### 2.10 Every gameplay random draw on these paths

| Path | Draw | Where |
| --- | --- | --- |
| Light flash spawn / toggle | `P_Random() & 64`, `& 7` | p_lights.c:142, 106, 111 |
| Unsynced strobe spawn | `P_Random() & 7` | p_lights.c:206 |
| Fire flicker cycle | `P_Random() & 3` | p_lights.c:53 |
| Perpetual platform start | `P_Random() & 1` | p_plats.c:246 |
| Strobe-hurt / super-slime with the suit | `P_Random() < 5` (short-circuited without it) | p_spec.c:1041 |
| Environmental damage (floors, crush) | pain `P_Random() < painchance`; on death `tics -= P_Random() & 3`; a monster's drop draws `lastlook` | p_inter.c:894, 726; p_mobj.c:506 |
| Crush blood | `P_SpawnMobj` `lastlook` + four `P_Random()` in two differences of **unspecified evaluation order** | p_mobj.c:506; p_map.c:1307–1308 |
| Monster door use through `P_TryWalk` | `movecount = P_Random() & 15` | p_enemy.c:356 |
| Sky missile removal | **removes** the `P_ExplodeMissile` draw | p_mobj.c:183 vs 96 |
| Sky shot | **removes** the two `P_SpawnPuff` draws | p_map.c:960, 964 vs p_mobj.c:819, 823 |
| Doors, lifts (except perpetual), floors, glow, synced strobe, switches, scrollers, secrets, use, crossing | none | — |

Sounds draw `M_Random` only. Teleports (`EV_Teleport`, p_telept.c:48) are not admitted.

## 3. Rust inventory

| Source body | Rust today | What moving sectors and specials change |
| --- | --- | --- |
| Sector heights | `SectorState { floor, ceiling }` (`multisector.rs:15`), `Simulation::sectors` (`lib.rs:518`), filled from the map (684–689), **never written**; every reader borrows `&[SectorState]` (`thinkers.rs:4`, `sight.rs:233–292`, `combat_trace.rs:353, 452, 510`, `movement.rs:709–820`, `perception.rs:223`, `monsters.rs:135–145`) | Becomes mutable gameplay state, written inside the thinker pass and visible to later thinkers of the same pass. The checked transaction (`demon_plan`, `demon_expanded_map.rs:234`) stages actors, spatial order, RNG, statistics, player, bindings and sound targets but passes `sectors: &self.sectors` read-only (400, 426, 485, 524): sector writes must be staged and committed with the plan |
| Light level, sector special, line special | immutable map data (`doom_map::Sector::light`, `special`; `snapshot.rs:988, 1056` encode them as map lumps) | Runtime fields: lights change every few tics, specials are cleared at load (§2.2), on secret entry and by W1/S1 lines |
| `P_LineOpening`, `PIT_CheckLine` accumulation | `multisector::opening` (283) and `check_move_from_things` (315) read runtime sectors and return `openings` in visit order | Unchanged; `spechit` is the special subsequence of `openings`, `ceilingline` its last strictly lowering entry — both derivable, no new array |
| Sight, hitscan, autoaim, noise | read sector heights at query time (`sight.rs:233`, `combat_trace.rs`, `noise::flood` `noise.rs:15`) | A closed door blocks sight, shots and sound with no code change |
| Prepared caches | `Topology` leaf→sector table (`multisector.rs:137`), collision cell lists, `sector_lines` (`player_map_scale.rs:952`, used by `noise.rs:28`), the sight expansion `expanded_work` (`sight.rs:174`), REJECT, the boundary and parity test | **All height-independent: they stay valid.** New static cache: each sector's `blockbox` (§2.1) |
| Type envelope and its memo | `type_envelope` = tallest static sector at construction (`demon_expanded_map.rs:122–130`); `AdmittedTypes` memo "validated against … these static sectors" (`lib.rs:529–531`) reads `dimension_sectors()` (720), which under the heights policies returns the envelope | Must stay the load-time value; never recomputed from runtime heights |
| Vertical invariant | `vertical_admitted`: `floor <= z <= ceiling − height` for every non-impact actor before every tic (`player_map_scale.rs:598`, preflight `demon_expanded_map.rs:341`) | **Fails on legal play** (§4.3) |
| Item witness | `validate_item`: an item's floor and ceiling equal its own sector's (`world_pickups.rs:301–318`; DSIM31+) | **Fails on legal play**: re-clipped items take the box-wide `floorz` (§2.6) |
| Spawn fit | `spawn_fits` against the spawn sector's runtime heights (`player_map_scale.rs:560`) | Unchanged meaning; now reads moving heights |
| Height certificate | clause (vii), heights within ±512, on the static map (`ArithmeticCertificate`, `player_map_scale.rs:754`) | Must bound every **reachable** height (§4.4) |
| Thinker store | `ThinkerStore<Actor>` (`store.rs:96`): generational slots, one linked processing order, lazy removal, appends visible in the same pass (`store.rs:193, 299`), exactly `P_RunThinkers`; `thinkers::run` (`thinkers.rs:21`) | Holds mobjs only. Sector thinkers must join the **same** order (§2.1); the 320-slot store and visit budget gain their count |
| Tic phases | `Phase::{Players, Thinkers, Specials, Respawn}` (`lib.rs:343–352`); `Specials` and `Respawn` are markers only (1282–1283; `demon_expanded_map.rs:565`) | `Specials` gets `P_UpdateSpecials` (scrollers derived, buttons presentation) |
| Use and crossing | `BT_USE` refused (`player_commands.rs:80–91`); no `P_UseLines`, `P_CrossSpecialLine`, `P_UseSpecialLine`, `spechit` loop in `P_Move` | New bodies; the 64-unit use trace reuses `movement::path` with `PT_ADDLINES` |
| `P_ChangeSector`, `P_ThingHeightClip` | absent | New body over the per-sector blockbox, reusing `check_move_from_things` and the pickup touch path |
| Environmental damage | `DamageRequest { inflictor: Option, source: Option }` (`combat.rs:56–58`); player `attacker`, `frags` exist (`original_player.rs:84–88`) | Reachable with `None`/`None` for the first time; the null-source branches (no thrust, own frag, `players[0]` credit) must be confirmed in Phase 2 |
| Statistics | `LevelStatistics::total_secrets` "initialized but specials are excluded" (`demon_map_context.rs:12–13`), encoded as `level.total_secrets` (`snapshot.rs:622`) | Counted at load; a player `secretcount` field is new |
| Gibs | `S_GIBS` identity present (`identity/names.rs:1038`, `source_actors.rs:1231`) | Entered by `PIT_ChangeSector` for any corpse, including the player's |
| Sky | `CombatError::Sky` load rejection only | `ceiling_line` derived from `openings`; missile removal in the projectile mover; the two shoot returns before the puff |
| `leveltime` | the simulation `tic` (`lib.rs:1284`); weapons take a `leveltime` argument (`player_pistol.rs:332`) | Damage floors and crush gates read `leveltime & 31` / `& 3`; Phase 2 must show `tic == leveltime` since construction under the new policy |

What breaks among the M3h4–M3h6 proofs if sectors move: nothing horizontal (Invariant Ω
and Theorem B read only one-sided walls, which never move, and no admitted body
changes an actor's `x`, `y` when a plane moves); the vertical invariant and the item
witness (§4.3); the static height clause (vii) of the certificate (§4.4); the slot and
visit budgets (§4.5); the claim that `spechit` is unreachable (M3h6 §4.4, which relied
on specials being rejected; §4.6). The arithmetic clauses (i)–(vi), (viii), the
slide-window rule, the intercept witness and every prepared cache are unaffected.

## 4. Proposed admitted domain and proof plan

### 4.1 Ordering against roster scale, and the split

M3h6 §8.4 recommended roster scale first, then decorations and barrels, sky, specials,
then doors and lifts. The dependencies, from §2 and §3:

- **None of sky, specials, doors or lifts needs roster scale.** Synthetic rooms with at
  most eight monsters and sixteen pickups exercise every body; E1M1 itself stays out
  until the roster scale and decorations land, whichever order is chosen.
- **Roster scale needs none of them**, but several new budgets are functions of the
  roster: the slot and visit budgets gain the sector-thinker count, the
  `P_ChangeSector` work per mover scales with linked things, and the intercept witness
  keeps its linked-thing term. Writing each new budget as a formula of the roster
  bounds makes the later milestone re-evaluate them rather than re-prove them.
- **Sky and the static sector specials** (lights, secrets, damage floors, scrolling
  walls) change no height: they need the shared thinker order (lights), mutable light
  level and special fields, environmental damage and the two sky bodies. None of the
  vertical proofs moves.
- **Doors, lifts, floors, switches and the exit** need everything above plus mutable
  staged heights, `P_ChangeSector`, use, crossing, monster door use, `spechit`, and the
  re-proved vertical invariant, item witness and height certificate.

**Recommendation (owner decision 1): split into two milestones and keep the owner's
priority over roster scale.** M3h7 = sky plus the static specials (Stage A below);
M3h8 = the movers, switches and exit (Stage B). Roster scale and the decoration types
stay independent and may come before or after M3h8; every Stage B budget is written as
a formula of the roster bounds. A single milestone is feasible but doubles the
evidence and the proof delta of one review; roster scale first (the M3h6 order) is also
sound and only delays E1M1's specials.

### 4.2 Stage A (M3h7): sky and static specials

Suggested policy `PlayerMonstersSpecialsV1` (wire 18, DSIM34, context
`PLAYER-MONSTERS-SPECIALS-1-MEDIUM-v1`), a superset of `PlayerMonstersRealMapV1` with the
same roster, caps, certificate, boundary rules, witnesses and transaction. Admitted at
load in addition:

- **Sector specials 1, 2, 3, 8, 12, 13, 17** (light thinkers, §2.4), **4 and 16** (strobe
  or none plus 20% damage; the suit draw is unreachable), **5 and 7** (damage), **9**
  (secret). E1M1 uses 1, 7, 8, 9, 12; the others are the same bodies and cost one
  scenario each (owner decision 3 may narrow to E1M1's five).
- **Line special 48** (scrolling wall) as presentation: no state, the offset is
  derivable from the tic (§2.8); at most 64 such lines (`MAXLINEANIMS`); **one-sided
  only**, because a two-sided special line enters `spechit`, and a monster whose move
  fails with a nonempty `spechit` resets `movedir` to `DI_NODIR` even when no line
  opens (§2.5), which belongs to Stage B. Whether E1M1's eight scrollers are one-sided
  is to be measured (§5.6).
- **`F_SKY1` ceilings** with the two bodies of §2.9.
- `level.total_secrets` counted at load; light thinkers spawned by an explicit
  operation that must follow the THING spawns and the player body, in sector order, as
  `P_SetupLevel` does (owner decision 12).

Still rejected: every other line special (including tag-using ones), sector specials
10, 11, 14 and unknown values, `BT_USE`, everything rejected by M3h6.

Proof delta: none of the vertical or horizontal invariants changes, because no height
changes. New obligations: (A1) thinker order — lights after every map-spawned mobj and
before every later-created thinker, in one order with the mobjs; (A2) RNG — spawn draws
in sector order after the THING draws, toggle draws at the thinker's turn; (A3) the
32-tic damage gate on `leveltime == tic`; (A4) environmental damage and death with null
inflictor and source; (A5) the secret test on `z == sector floor` of the center's
subsector, the special cleared, `secretcount` and `totalsecret`; (A6) sky —
`ceiling_line` is the last strictly lowering opening of the refused check, the removal
path retires the fireball without explosion, and the two shoot returns precede the puff;
(A7) the slot and visit budgets plus the number of light sectors.

### 4.3 Stage B (M3h8): doors, lifts, floors, switches and the exit

Suggested policy `PlayerMonstersMoversV1` (wire 19, DSIM35, context
`PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1`), a superset of Stage A. Admitted at load in
addition: line specials **1, 63, 109, 88, 36, 38, 11** (E1M1's set; owner decision 3),
`BT_USE`, under the load rules of §4.4. Still rejected: crushers (every type with `crush
= true`, all ceilings), locked doors (no keys in the roster), teleports, stairs,
donuts, `raiseToTexture` (needs `TEXTURE1` heights), perpetual and stopping platforms,
light-switching lines, gun lines, sector specials 10, 11, 14, and any map failing §4.4.

For the admitted set `crushchange` is false in every `P_ChangeSector` call (doors, the
lift and both floors pass `crush = false`), so the damage and blood branch of
`PIT_ChangeSector` is unreachable; the gib, dropped-item and non-shootable branches are
reachable (a door closing on a corpse, a dropped clip, a health bonus).

**What must be re-proved under moving planes:**

- **(B1) Invariant Ω and the escape witness**: unchanged. Movers change heights only;
  `P_ThingHeightClip` never changes `x`, `y`; no admitted special teleports.
- **(B2) The vertical invariant is replaced.** The M3h4 form fails on legal play
  (§2.6): a non-shootable thing under a closed door keeps `z == floorz > ceilingz −
  height`; a thing not on its floor is never raised by a rising floor and keeps a `z`
  lowered by a lowering ceiling when the step is undone, so it can end a tic below its
  `floorz` by up to one plane step `Δ` (2 for a door ceiling, 4 for the lift, and for
  `turboLower` the largest one-tic jump-up of §2.7, all computed at load). Proposed V′,
  checked at every tic boundary like the old one: every non-impact actor has `z` and
  `momz` within the existing witnesses and either (a) `floorz ≤ z ≤ ceilingz − height`,
  or (b) it is not `MF_SHOOTABLE`, `z == floorz` and `ceilingz < z + height`, or (c)
  `floorz − Δ ≤ z < floorz`. The proof is a case analysis of `T_MovePlane` ×
  `P_ThingHeightClip` over the admitted directions; (c) relies on `P_ZMovement` landing
  the thing, whose arithmetic M3h4 already bounds. Every non-fitting step of an admitted
  mover is undone (`crush` is false), so a shootable thing ends a tic in (b) only through
  the overlap case (2) of §2.6, where the restore call itself can fail; Phase 2 either
  proves that case unreachable under the spawn and movement rules or admits it in V′
  explicitly.
- **(B3) The item witness is relaxed**: `z == floorz` with `floorz`/`ceilingz` the
  actor's own fields (whatever the last clip set), plus case (b); no longer "equal to
  its own sector".
- **(B4) The reachable-height certificate** replaces clause (vii) (§4.4).
- **(B5) The vertical arithmetic of M3h4** (fireball chord 968, the 2,048 `z` witness,
  eye and opening slopes) depends only on heights within ±512, so (B4) carries it over.
- **(B6) `spechit`** (§4.6).
- **(B7) The intercept witness**: a use trace adds lines only and is 64 units long, so
  it changes cell at most once per axis and stays inside one 2 × 2 window: the
  slide-window rule (≤ 128 distinct lines) covers it unchanged.
- **(B8) Budgets** (§4.5): sector-thinker slots, visits, and the `P_ChangeSector` work.
- **(B9) The transaction**: every sector write, light, special and line-special clear
  of a tic is staged and committed with the plan; a refused tic leaves the sector table
  byte-identical.
- **(B10) Pickups inside a clip**: the player's `P_ThingHeightClip` reuses the existing
  touch path, in the clip's cell order.
- **(B11) Exit**: the tic of `G_ExitLevel` completes; the level is marked exited and the
  next `advance` is refused with a checked error (owner decision 10).

### 4.4 Load rules for movers (Stage B)

All computed once at load from the static map, in exact integers, rejected with a new
field-less `MapScaleError::Specials` variant (or a more specific one per rule):

1. Every admitted line special is in the admitted set; every DR (1) line is two-sided
   (otherwise `EV_VerticalDoor` reads `sides[-1]`, §2.7).
2. **Mover families are disjoint**: no sector is both the back sector of a DR line and
   a tagged target of a non-door special, so the `specialdata` cast of §2.7 always
   reads a door; doors from 1, 63 and 109 may share sectors (all `vldoor_t`).
3. Every mover sector has at least one two-sided neighbor (no `MAXINT − 4` door top).
4. **Reachable heights.** A fixpoint over intervals `[lo, hi]` of each sector's floor
   and ceiling, starting from the static heights and applying the admitted rules: a door
   ceiling ranges over `[floor, top]` with `top` at most the largest and at least the
   smallest reachable lowest-neighbor ceiling minus 4; a lift floor over `[lowest
   reachable neighbor floor, own floor]`; `lowerFloorToLowest` reaches a neighbor
   floor; `turboLower` a neighbor floor + 8; each W1 line is applied once. Every bound
   must stay within ±512, and every door's smallest possible `top` must be at or above
   its floor: a door whose neighbor ceiling can come within 4 units of its floor, or
   whose sector has a line with that sector on both sides (it is then its own
   neighbor, so a closed door's top is its floor − 4), would invert; such maps are
   rejected rather than emulated (owner decision 9). The fixpoint also yields `Δ` of
   (B2).
5. At most 30 plat sectors (`MAXPLATS`), at most 16 reusable switch lines
   (`MAXBUTTONS`), at most 64 scrollers.
6. The `spechit` window rule (§4.6).

Tag 0 is admitted faithfully (a tag-0 W1/SR/WR line acts on every untagged sector,
§2.5); the IWAD tool counts how often it happens (§5.6, owner decision 11).

### 4.5 Budgets

- **Thinker slots and visits.** Sector thinkers are bounded at load: light sectors
  (Stage A) plus mover sectors (Stage B, one `specialdata` each). The store bound
  becomes `320 + lights + movers` and the visit budget likewise, or sector thinkers get
  their own slot class in the same order (owner decision 4).
- **`P_ChangeSector`**: one call visits the blockbox cells (`(right − left + 1) · (top −
  bottom + 1)` of the sector bbox ± 32, clamped) and at most the linked-thing bound (33, `MAX_LINKED_THINGS`,
  `player_map_scale.rs:48`) of `P_CheckPosition` work; at most two calls per mover per
  tic; at most one mover per sector. The per-tic bound `2 · Σ_movers (cells + 33 ·
  collision work)` is computed at load and recorded, like the M3h6 work values.
- **Sector staging**: at most `2 · movers + lights` sector writes per tic.

### 4.6 `spechit` (`MAXSPECIALCROSS`)

Once specials are admitted, `spechit[8]` can overflow (§2.5): the ninth two-sided,
non-blocking special line crossing one box is written past the array (undefined; demo
ports emulate the DOS layout). `PIT_CheckLine` reads only the cells of `tmbbox`
(p_map.c:431–439, no `MAXRADIUS`); with every admitted radius ≤ 30 a box spans at most
two cells per axis, so the lines that can enter `spechit` are among the two-sided
special lines listed in one 2 × 2 window. **Recommendation (owner decision 7)**: a load
rule — at most 8 distinct two-sided special lines in any 2 × 2 cell window — which
proves the overflow unreachable, plus a runtime witness `MapScaleError::SpecialCross`
that refuses the tic at a ninth write (reachable only on a Rust-only fixture), like the
intercept witness. Scrolling walls are usually one-sided and never enter `spechit` (a
one-sided line returns before the special test, p_map.c:211–212). E1M1's window maximum
is not committed (§5.6).

## 5. Evidence plan

### 5.1 C harness

`tools/reference/m3h7_geometry.py` extends the M3h6 builder with line specials, tags,
sector specials, light levels and flat names; `m3h7_cases.py` and `m3h7_reference.py`
extract genuine bodies from `p_spec.c` (`P_SpawnSpecials`, `P_PlayerInSpecialSector`,
`P_UpdateSpecials`, `P_CrossSpecialLine`, `P_ShootSpecialLine`, the `P_Find*` helpers,
`getNextSector`), `p_lights.c`, `p_doors.c`, `p_plats.c`, `p_floor.c`, `p_switch.c`
(`P_UseSpecialLine`, `P_ChangeSwitchTexture`, `P_StartButton`), `p_map.c`
(`P_UseLines`, `PTR_UseTraverse`, `P_ChangeSector`, `PIT_ChangeSector`,
`P_ThingHeightClip`), `p_enemy.c` (`P_Move`, `P_TryWalk`) and `p_tick.c`, reusing every
M3h6 body. Declared adapters: `S_StartSound` recorded as an observation with its origin
(mobj or sector `soundorg`); `G_ExitLevel` recorded; `R_TextureNumForName` and the
switch list from a declared synthetic texture table (presentation, recorded only);
`skyflatnum` a declared flat number. New observers: the thinker list with each entry's
kind at every tic, every sector's floor, ceiling, light and special, `numspechit` at
the end of every `P_CheckPosition`, `ceilingline` at every refused missile step, every
`P_ChangeSector` call with each visited thing's clip outcome.

| Room | Stage | Purpose |
| --- | --- | --- |
| `pools` | A | sectors 1, 2, 3, 4, 5, 7, 8, 9, 12, 13, 16, 17 in a row of basins; a ledge whose edge overhangs nukage; armor on the floor; a monster roster that fights in the room so fireball explosions and light toggles meet in one tic |
| `sky` | A | the M3h6 §6 room: an outdoor court under `F_SKY1` with sky-hack walls (two-sided, both ceilings sky), one-sided walls under sky, a window into a sky court; imps firing over them; a player shooting at the sky |
| `doorway` | B | two rooms joined by DR doors; a monster behind each; a former human placed to die in the doorway (dropped clip), a health bonus inside the door sector |
| `switchyard` | B | an SR 63 switch and its tagged door; the S1 exit switch; a non-special wall (`sfx_noway`); a scrolling wall; a window in front of a switch; back-side use |
| `lift` | B | a WR 88 lift with a ceiling too low for a monster at the top; W1 36 beside a higher neighbor (the + 8 jump-up) and beside lower ones; W1 38; W1 109 |
| `dense` | B | a lane with 8 two-sided special lines in one 2 × 2 window (admitted); Rust-only 9 |

Scenarios (about 30): `pools_lights_soak`, `pools_nukage_damage`, `pools_nukage_death`,
`pools_ledge_edge`, `pools_secret`, `pools_light_and_fireball`, `sky_hack_wall`,
`sky_one_sided_wall`, `sky_ceiling_fireball`, `sky_shot_high`, `sky_shot_hack`,
`sky_window_order` (ceilingline order with two lowering lines), `door_use`,
`door_reuse_closes`, `door_reverse_on_player`, `door_monster_bump`,
`door_monster_cannot_close`, `door_gibs_corpse`, `door_removes_drop`,
`door_traps_bonus`, `door_blocks_sight_and_sound`, `switch_sr_door`,
`switch_exit`, `use_noway`, `use_back_side`, `use_scroll_wall`, `lift_player`,
`lift_monster_cross`, `lift_reverse_at_top`, `lift_last_step_short`,
`floor_turbo_jump_up`, `floor_lower_to_lowest`, `blaze_open`, `item_rises_onto_step`,
`player_picks_up_in_clip`, `dense_spechit_8`, plus the M3h6 captures. Strict and
ASan/UBSan builds compared as before; `spechit[8]` stays its source size so the
sanitizer would report an overflow of the C harness.

### 5.2 Independent DSIM encoders

Stage A: `tools/reference/specials_snapshot.py` extends `real_map_snapshot.py` without
editing it; `m3h7_snapshot_goldens.py`, `m3h7_snapshots.csv`, `m3h7_events.dref`, written
before the Rust change. DSIM34 adds persistent fields (wire 18):

- `sector.{i}.light`, `sector.{i}.special` (runtime values; the map lumps keep the
  original);
- one record per sector thinker in the shared thinker order: `kind` (flash, strobe,
  glow, flicker), `sector`, `count`, `minlight`, `maxlight`, and `maxtime`/`mintime`,
  `darktime`/`brighttime` or `direction` by kind;
- `player.secretcount`;
- events: light changes (or none, owner decision 5), secret entry, environmental
  damage through the existing damage events with no source, missile removal at sky,
  shot ended at sky.

Stage B: `movers_snapshot.py` extends `specials_snapshot.py`; DSIM35 (wire 19) adds
`sector.{i}.floor`/`ceiling` as moving values (the fields exist since v3),
`sector.{i}.thinker` (the `specialdata` handle), `line.{i}.special` (runtime), door
records (`type`, `topheight`, `speed`, `direction`, `topwait`, `topcountdown`), plat
records (`type`, `speed`, `low`, `high`, `wait`, `count`, `status`, `oldstatus`, `crush`,
`tag`, `activeplats` slot), floor records (`type`, `crush`, `direction`, `newspecial`,
`texture`, `floordestheight`, `speed`), `player.usedown`, `level.exit`; events for
use (line, side, result), crossing (line, side, actor), mover start, step result
(`ok`/`crushed`/`pastdest`) and stop, `P_ChangeSector` clip outcomes (gibs, removal),
sector sounds, and the exit. Switch textures, button timers and scroll offsets stay out
(owner decision 5). Every earlier reader rejects 34 and 35; the 34 reader rejects 35.

### 5.3 Rust-only evidence

Rejection tests with unchanged state for every load rule of §4.2 and §4.4 (an
unadmitted special of each family, a two-sided scroller under Stage A, a one-sided DR line, a shared door/floor sector, a
door without two-sided neighbors, an inverting door, a self-referencing door line, a
reachable height of 513, 65 scrollers, 31 plat sectors, 17 reusable switches, 9
special lines in a window); the `SpecialCross` witness on the 9-line lane, byte-identical
before and after; a refused tic in the middle of a door movement leaving sectors,
lights, specials and thinkers byte-identical; V′ and the relaxed item witness after
every tic of 1,200-tic soaks of `pools`, `doorway` and `lift` with four seeds;
`tic == leveltime` across construction and level reset; the DSIM34/35 readers' version
and policy rejections. Fuzz target `player_specials` (Stage A rooms) and
`player_movers` (Stage B rooms, commands including `BT_USE`) with the M3h6 properties
plus V′, the item witness and the transaction; seeds from
`tools/seed_player_specials_fuzz.py`.

### 5.4 Measurements

Alone on the idle host, as AGENTS.md and CLAUDE.md require: matched release timings of
the M3h6 head and the branch head for every existing `--measure` workload (digests
compared; older policies must not move), plus `pools_lights_soak`, `doorway_patrol` and
`lift_cycle`; admission time with the reachable-height fixpoint and the window rules;
the recorded per-tic `P_ChangeSector` work.

### 5.5 C facts each scenario pins

The quirks of §2 that a faithful implementation could plausibly "fix" each get a
dedicated scenario: the reverse `spechit` order (`lift_monster_cross` with two specials),
the short-stopping last step (`lift_last_step_short`), the + 8 jump-up
(`floor_turbo_jump_up`), the first box-wide clip (`item_rises_onto_step`), the pickup in
a clip (`player_picks_up_in_clip`), the sector-floor test of damage and secrets
(`pools_ledge_edge`), the front/back sector asymmetry of the two sky tests
(`sky_window_order`, `sky_shot_hack`), monster door use from the back side
(`door_monster_bump`), and the frag on an environmental death (`pools_nukage_death`).

### 5.6 To be measured locally by the owner

`tools/iwad_counts.py` would gain (local-only, counts and pass/fail only, as decided in
M3h6 decision 9): per map, line-special and sector-special type counts; sky sectors and
sky sectors that are mover sectors; distinct mover sectors and plat sectors; reusable
switch lines; scrollers, and how many are two-sided; one-sided DR lines; sectors in more than one mover family;
door sectors with a self-referencing line or a reachable top below their floor; the
reachable height range and whether it stays within ±512; tag-0 lines with tag-using
specials; and the maximum number of two-sided special lines in a 2 × 2 window. For
E1M1 none of these is committed beyond the counts of §1. The tool now prints these
counts (text report and `--specials-markdown`; the reachable-height fixpoint prints
"not computed"), and their table awaits a local run by the owner.

## 6. Implementation plan (one commit per step; evidence before the runtime change)

**Stage A (M3h7).**

1. **This note.**
2. **IWAD count tool** extension (§5.6), local-only; its table is committed only on
   owner approval.
3. **C evidence**: `m3h7_geometry.py`, `m3h7_cases.py`, `m3h7_reference.py`; generated
   `tests/reference/m3h7_trace.dref`, cases, geometry lumps, `m3h7_maps.rs`,
   `m3h7_contract.json`; strict and sanitizer builds compared.
4. **DSIM34 projection**: `specials_snapshot.py`, goldens, outputs.
5. **Thinker order refactor**, no behavior change: the store's entry becomes a mobj or a
   sector thinker sharing one order (owner decision 4); every historical fixture and
   digest unchanged.
6. **Admission and bodies** (one commit, since the replays need all of it): policy,
   wire 18 and predicates in `original_registry.rs`; load rules; mutable light and
   special fields staged with the plan; the light thinkers; `spawn_level_specials`;
   `P_PlayerInSpecialSector` in the player phase; environmental damage; secrets; sky
   (`ceiling_line`, removal in `projectiles.rs`, the shoot returns in `combat_trace.rs`).
7. **DSIM34**: `snapshot.rs`, `diagnostic.rs` (`decode_specials`), STATE_FORMAT.
8. **Tests, example, tools, fuzz, CI** on the M3h6 pattern.
9. **Docs**: README, ROADMAP, COMPATIBILITY (new section, limits rows, the sky-floor
   wording), STATE_FORMAT, ARCHITECTURE, TESTING, docs/README, `tools/reference/README.md`,
   the evidence record.
10. **Gates, review, draft PR**, ready only after every gate passes.

**Stage B (M3h8)**, each step one commit, in this order: an addendum to this note with
the owner's Stage B decisions; C evidence for the B rooms; the DSIM35 projection;
staged mutable heights and the per-sector blockbox (no behavior change for older
policies); `T_MovePlane`, `P_ChangeSector`, `PIT_ChangeSector`, `P_ThingHeightClip`;
door, plat and floor thinkers with the §4.4 load rules; crossing, use, `BT_USE` with
`usedown`, monster door use and the `spechit` rule and witness; switches and the exit;
V′ and the relaxed item witness; DSIM35; tests, fuzz, measurements; docs; gates and
draft PR.

If the owner prefers one milestone, steps 3–4 and the Stage B evidence merge, and the
runtime steps keep their order.

## 7. Owner decisions needed before Phase 2

1. **Scope and order** (§4.1): (a) split — M3h7 sky and static specials, M3h8 movers,
   roster scale independent with roster-formula budgets (**recommended**); (b) one
   milestone for everything; (c) roster scale first, as M3h6 §8.4 recommended.
2. **Policy identities**: `PlayerMonstersSpecialsV1`, wire 18, DSIM34,
   `PLAYER-MONSTERS-SPECIALS-1-MEDIUM-v1`; `PlayerMonstersMoversV1`, wire 19, DSIM35,
   `PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1` (**recommended**).
3. **Breadth**: Stage A every non-mover, non-exit sector special (1–5, 7–9, 12, 13, 16,
   17) plus 48 (**recommended**, same bodies) or E1M1's five; Stage B exactly E1M1's
   seven tag/use/cross types (**recommended**) with siblings later.
4. **Thinker representation**: sector thinkers in the same ordered store as mobjs,
   with their own handle kind and slot class bounded at load, so the roster's 320-slot
   proof is untouched (**recommended**); or sharing the 320 actor slots with the bound
   raised by the sector-thinker count.
5. **Presentation**: scroll offsets derived, switch textures and button timers not
   simulated, the `MAXBUTTONS` overflow made unreachable by the load rule; sector sounds
   and light changes recorded as events (**recommended**: record sounds as the existing
   sound requests are recorded; light changes are visible in the persistent fields, so
   no event).
6. **Sky** in Stage A with the M3h6 §6 plan (**recommended**).
7. **`spechit`**: the 2 × 2 window load rule plus the `SpecialCross` runtime witness
   (**recommended**); or the witness alone.
8. **Vertical invariant V′ and the relaxed item witness** under the movers policy only
   (**recommended**); older policies keep theirs.
9. **Mover load rules** of §4.4, rejecting inverting doors, self-referencing door lines,
   shared mover families and doors without two-sided neighbors (**recommended**), rather
   than emulating inverted sectors and the `specialdata` type confusion.
10. **Exit**: the tic completes, `level.exit` is set, the next `advance` returns a checked
    error and preserves the state (**recommended**); intermission and the next map are
    out of scope.
11. **Tag 0**: faithful (**recommended**), counted by the IWAD tool; or rejected at load.
12. **Load order**: an explicit `spawn_level_specials` that must follow the THING spawns
    and the player body and runs once (**recommended**), mirroring `P_SetupLevel`.
13. **Sector staging**: a sparse copy-on-write overlay of the sectors written in the tic
    (**recommended**, no per-tic copy of the table) or a full copy, decided by the
    measurement of step 5.4.
14. **Crushers** (and with them the crush-blood evaluation order of §2.10): deferred
    (**recommended**); E1M1 does not need them.

**Owner decisions, recorded 2026-11-03 before any runtime change (Phase 2 implements
Stage A exactly as §4.2 and §6 describe):**

1. Approved: split. M3h7 is Stage A (sky and static specials); M3h8 is Stage B
   (movers). Every new budget is written as a formula of the roster bounds.
2. Approved: `PlayerMonstersSpecialsV1`, wire 18, DSIM34, context
   `PLAYER-MONSTERS-SPECIALS-1-MEDIUM-v1` now; `PlayerMonstersMoversV1`, wire 19,
   DSIM35, `PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1` reserved for M3h8.
3. Approved: Stage A admits every non-mover, non-exit sector special (1–5, 7–9, 12, 13,
   16, 17) plus line special 48 on one-sided lines only. Stage B later admits exactly
   E1M1's seven line types.
4. Approved: sector thinkers live in the same ordered store as mobjs, with their own
   handle kind and a slot class bounded at load; the 320-slot roster proof is untouched.
5. Approved: scroll offsets are derived, not stored; sector sounds are recorded like
   the existing sound requests; light changes are visible in the persistent fields and
   get no extra event.
6. Approved: sky in Stage A with the M3h6 §6 plan.
7. Approved as recommended, for M3h8 (recorded, not implemented in M3h7): the `spechit`
   2 × 2 window load rule plus the `SpecialCross` runtime witness.
8. Approved as recommended, for M3h8: V′ and the relaxed item witness under the movers
   policy only.
9. Approved as recommended, for M3h8: the §4.4 mover load rules.
10. Approved, for M3h8: the exit tic completes, `level.exit` is set and the next
    `advance` returns a checked error that preserves the state.
11. Approved: tag 0 is faithful, counted by the IWAD tool.
12. Approved: an explicit `spawn_level_specials`, run once, after the THING spawns and
    the player body.
13. Approved as recommended, for M3h8: a sparse copy-on-write sector overlay.
14. Approved: crushers deferred.

Out of scope for M3h7: everything in Stage B, every other line special, sector specials
10, 11 and 14, `BT_USE` and roster scale. No IWAD was available to the Phase 2 session
(no owner-authorized Drive connector); the §5.6 measurements stay **to be measured
locally by the owner**.

## 8. Follow-ups and risks

- **Not needed by E1M1, deferred**: crushers and ceilings, locked doors and keys,
  teleports (monster and player), stairs, donuts, `raiseToTexture` (a `TEXTURE1`
  dependency), perpetual and stopping platforms, light-switching lines, gun lines,
  sector specials 10, 11, 14, the level exit's intermission and next map.
- **Crush blood** draws four values in two `(P_Random() - P_Random())` expressions whose
  evaluation order C leaves unspecified (§2.10); admitting crushers needs a declared
  compiler order like the other adapter-defined orders.
- **Undefined source behavior refused rather than emulated**: the ninth `spechit` write,
  `sides[-1]` for a one-sided DR line, the door cast of a non-door `specialdata`, the
  65th scroller, the 31st plat and 17th button (`I_Error`), inverting doors. Like the
  intercept and corner-jump refusals, these are demo-compatibility questions for M4.
- **COMPATIBILITY wording**: "sky ceilings and floors stay excluded" overstates the
  code, which rejects only `F_SKY1` ceilings; sky floors have no game-code effect
  (§2.9). To be corrected in Stage A's documentation step, not here.
- **Thinker order at construction**: the Rust player body is spawned by the caller
  (M3h6 §4.7); the light thinkers must come after it and after every THING, as in the
  source, which constrains the construction API (decision 12).
- **First box-wide clip** (§2.6, consequence 3) changes `floorz` and `z` of things that
  never moved; scenarios must start some items and monsters straddling steps near a
  mover, or the quirk goes unobserved.
- **Performance**: `P_ChangeSector` runs a full position check per linked thing in the
  blockbox per mover per tic; with the roster bound it is small, but roster scale
  multiplies it. Measured in Stage B.
- **CI headroom**: two new generators and fuzz targets; estimated as in M3h6 §8.3 when
  the harness exists.
- **Unknown E1M1 details** (§5.6): whether its door sectors pass §4.4, its `spechit`
  window maximum, its reachable heights, and whether any sky sector moves.

## 9. Status / next step

Status: **Phase 2 Stage A is implemented and at owner review**; milestone status is
kept in [ROADMAP](ROADMAP.md#m3h7-sky-and-static-specials) and exact review state in
GitHub. The paragraphs below are the history of each Phase 2 step of §6 as it landed.

Phase 2 step 1 (decisions): the owner decisions of §7 are recorded.

Phase 2 step 2 (IWAD count tool): `tools/iwad_counts.py` gains the §5.6 counts per map
and in a summary (line and sector special types, tagged lines and sectors, the Stage A
admission facts and the Stage B count facts) with a `--specials-markdown` table; every
earlier output line is unchanged, the reachable-height fixpoint prints "not computed",
and no IWAD was read, so the table awaits a local run by the owner (§5.6).

Phase 2 step 3 (original-C evidence): `tools/reference/m3h7_geometry.py`,
`m3h7_cases.py` and `m3h7_reference.py` generate `tests/reference/m3h7_*` (17
scenarios in the `pools` and `sky` rooms; counts, row formats and adapters in
`m3h7_contract.json`). Departures from §5.1: only the Stage A observers exist (no
`numspechit` or `P_ChangeSector` rows; sector rows carry light and special, not the
static heights); `P_ShootSpecialLine` is compiled because shots reach the scrollers,
while `P_CrossSpecialLine`, `P_UseSpecialLine` and every mover body stay aborting
adapters; there is no sector sound in Stage A, so `S_StartSound` keeps its actor-only
adapter; sector thinkers are allocated from a separate harness store so actor idents
keep their meaning; a back-sided line without `ML_TWOSIDED` would be rejected by the
admitted load rules, so the neighbor that `getNextSector` skips is a dark gallery
behind one-sided walls; five scenarios were added (`pools_strobe_hurt`,
`sky_wall_after_hack`, `sky_shot_window`, `sky_window_strict`, `sky_window_back`); the
null-source `players[0]` kill credit stays unreachable (only players read damage
floors). During harness tic *t* `leveltime` is *t* − 1. Refinements of §2.9: the
refused check starts from the destination sector's ceiling (p_map.c:406–407), so a
fireball whose center has already entered the lower sector when its box first meets
the line has no `ceilingline` and explodes even when that line's back is sky; and
`P_CheckMissileSpawn` explodes a missile refused at its spawn half-step with no sky
test (p_mobj.c:880–881).

Phase 2 step 4 (independent DSIM34 projection): `tools/reference/specials_snapshot.py`
and `m3h7_snapshot_goldens.py` generate `tests/reference/m3h7_snapshots.csv` (427
captures), `m3h7_events.dref`, nine full-byte `m3h7_state_*.bin` fixtures and
`m3h7_snapshot_contract.json` from the C rows alone; the layout is specified in
[STATE_FORMAT](STATE_FORMAT.md#dsim-v34--original-monsters-with-sky-and-static-specials).
Departures from §5.2: the shared order is a separate `thinker.shared_order` field of
kind-tagged handles, while `thinker.order`, `store.head`/`tail` and actor
`previous`/`next` keep their actor-only DSIM33 meaning; the player's count is
`original_player.0.secret_count`; a glow record has no `count` (`glow_t` has none);
light changes, light draws and sector-thinker runs emit no event (decision 5); the
sky-shot event follows the trace record of the hitscan it ended instead of sitting
inside it; there is no sector sound in Stage A.

Phase 2 step 5 (thinker order refactor, no behavior change): `ThinkerStore` keeps one
linked order of `ThinkerId::{Actor, Sector}` entries over the unchanged actor slot class
and a separate `SectorThinkerId` slot class whose capacity is fixed at construction (zero
for every existing policy, payload `SectorThinker` still uninhabited), the actor order,
`store.head`/`tail` and actor `previous`/`next` are derived as its actor subsequence, and
every historical fixture, digest and the two-allocation tic plan are unchanged.

Phase 2 step 6 (admission and bodies): `PlayerMonstersSpecialsV1` (wire 18, context
`PLAYER-MONSTERS-SPECIALS-1-MEDIUM-v1`, `Simulation::original_player_specials_map`) is
the real-map policy plus the §4.2 load rules, the runtime sector light and special
staged with the checked plan through a reused spare buffer, the four light thinkers in
a sector-thinker slot class bounded by the light-sector count (visit budget 320 plus
that count), the explicit `Simulation::spawn_level_specials` (refused, state unchanged,
before the player body, twice, after a tic or under another policy; `advance` is
refused until it ran and THING or body spawns after it), `P_PlayerInSpecialSector`
between `P_CalcHeight` and the weapon change, null-source environmental damage through
the shared damage path, secrets, the `ceilingline` sky removal (derived from the
refused check's openings) and the two sky returns of `PTR_ShootTraverse`; sky identity
is the `F_SKY1` lookup name of each ceiling, and sector lights outside
`i16::MIN + 64 ..= i16::MAX - 64` are refused (`MapScaleError::Light`).

Phase 2 step 7 (DSIM34 and replay): captures of the policy are DSIM34 exactly as
[STATE_FORMAT](STATE_FORMAT.md#dsim-v34--original-monsters-with-sky-and-static-specials)
specifies (the capture side landed with step 6 because the tag-25 events need an
encoding); `Diagnostic::decode_specials` reads 34 only and every earlier reader rejects
it; `player_specials_tests` replays all 17 committed scenarios against the C rows
(actor, player, weapon, armor, pickup, statistics, every `SX` and `TK` value, RNG), the
427 independent captures, the 9 full-byte fixtures and the ordered events, with `step`
equal to `advance` at every tic and no scenario refused, plus Rust-only rejection,
load-order, refused-tic and identity tests. No STATE_FORMAT error was found.

Phase 2 step 8 (tests, example, tools, fuzz, CI): the `player_specials` example replays
every scenario and asserts the committed headline counts (`--measure` and `--admit`
on the M3h6 pattern), with `check_player_specials.py`, `measure_specials.py --check`,
the fuzz target `player_specials` with `tools/seed_player_specials_fuzz.py`, the
1,200-tic `pools` and `sky` soaks on four seeds, the Rust-only sky-shot boundary
`z == front ceiling` (fails under the `>=` mutation), the A7 slot and visit budget test
and three CI steps; observations are in [the evidence record](validation/M3H7.md).

Phase 2 step 9 (documentation): README, [ROADMAP](ROADMAP.md#m3h7-sky-and-static-specials)
(M3h7 current, M3h8 next), the
[COMPATIBILITY section](COMPATIBILITY.md#original-monsters-with-sky-and-static-specials)
with its limits rows and the corrected sky wording of the real-map section (only
`F_SKY1` ceilings were ever rejected; §2.9, §8), ARCHITECTURE, the
[TESTING commands](TESTING.md#original-monsters-with-sky-and-static-specials), the
documentation index, VALIDATION and the tools README; STATE_FORMAT needed no change.

Independent review of Stage A (fix pass): the review reported nine findings, addressed
as follows. (1) `ThinkerStore::order()` filtered the shared order through a per-entry
link copy, which cost older policies, which have no sector thinkers, +0.45 % to +2.4 %
instructions in `Simulation::advance`; it now walks the links directly with one slot
lookup per entry, and the cursor reads an entry's links once (no change to order,
encoding or digests; the per-tic set in `preflight_expanded_demons_for` is master
behavior and was left unchanged). (2) The evidence record's "no timing is a
measurement" sentence now covers its correctness sections only. (3) This status. (4) The
sector-thinker slot class was copied into a fresh vector every specials tic; it is now
staged in a reused spare buffer (`spare_sector_slots`) like the runtime specials, and
older policies still stage an empty, never-allocating vector. (5) `decode_specials`
also checks against the encoded map: one thinker per light sector, of its kind; a slot
count equal to the light-sector count (zero only at tic 0, before the load operation);
cleared light-sector specials (4 for special 4); the lines of the sky events; and that
a sky removal names an occupied fireball. (6) The unreachable `ML_TWOSIDED` skip of
`P_FindMinSurroundingLight` is commented as such. (7) A Rust-only test shows that the
body above a secret sector's floor does not find the secret and finds it once on
landing. (8) COMPATIBILITY states which part of `P_CalcHeight` is computed. (9) The
specials projection asserts that each `KS` row is followed by its hitscan's `X` end
row; every generated output is unchanged except the projection's own input hash in
`m3h7_snapshot_contract.json`. Observations are in [the evidence record](validation/M3H7.md).
