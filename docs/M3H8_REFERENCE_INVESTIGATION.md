# M3h8 — Doors, lifts, floors, switches and the exit (Stage B)

This note records the original-C facts, the Rust inventory, the admitted domain with its
proofs, the load rules, the budgets, the DSIM35 proposal, the evidence plan, the
implementation plan, the E1M1 readiness list and the owner decisions for Stage B of the
[M3h7 investigation](M3H7_REFERENCE_INVESTIGATION.md#43-stage-b-m3h8-doors-lifts-floors-switches-and-the-exit):
doors, lifts, floors, switches and the level exit. It was written and committed before
any runtime change (Phase 1). Support and limits belong to
[COMPATIBILITY](COMPATIBILITY.md); DSIM schemas belong to [STATE_FORMAT](STATE_FORMAT.md);
measured observations will belong to the evidence record written in Phase 2. All
references are to the preserved `linuxdoom-1.10/` sources at `a77dfb96`; every C fact
below was re-read for this note, including those it shares with the M3h7 note, and
corrections to that note are listed in §2.12. Every number is in map units unless it
says raw (16.16 bits) or tics. Line numbers name the first line of the cited statement.
Real-map figures are the owner's local, read-only measurements committed in
[M3h8 IWAD counts](validation/M3H8_IWAD_COUNTS.md) (commit `61d6087`) plus one local run
of `tools/iwad_counts.py --readiness E1M1` for this note (counts only, §13).

## 1. Starting point

`master` is at `37d4e5c` (merge of PR #51, M3h7). M3h7 delivered Stage A,
`PlayerMonstersSpecialsV1` (wire 18, DSIM34, context
`PLAYER-MONSTERS-SPECIALS-1-MEDIUM-v1`,
[COMPATIBILITY](COMPATIBILITY.md#original-monsters-with-sky-and-static-specials)): the
real-map policy of M3h6 plus `F_SKY1` ceilings (the missile removal and the two hitscan
returns), the light sector specials 1, 2, 3, 4, 8, 12, 13, 17, the damage specials 4, 5,
7, 16, secrets (9), line special 48 **on one-sided lines only**, the shared thinker order
of mobjs and sector thinkers, `Simulation::spawn_level_specials`, environmental damage
and the runtime sector light and special staged with the checked plan. Everything this
note adds is still rejected there: every line special but one-sided 48
(`PursuitError::Specials`, `demon_expanded_map.rs:220–248`), sector specials 10, 11, 14,
`BT_USE` (`SimError::UnsupportedButtons`, `player_commands.rs:80–91`), and no code path
writes a sector height after construction (`lib.rs:703–713`).

The branch `feat/m3h8-movers` adds one commit to `master`, `61d6087`, the local IWAD
mover counts. They confirm the Stage B breadth (E1M1's tag/use/cross types are exactly
{1, 63, 109, 88, 36, 38, 11}) and reveal that **all eight of E1M1's scrolling walls are
two-sided**, so E1M1 fails the Stage A scroller rule before any Stage B rule (§2.9, §14
decision N1).

## 2. Original C facts

### 2.1 Thinkers, the tic and the player

- **`P_AddThinker`** ([p_tick.c:65](../linuxdoom-1.10/p_tick.c)) appends at the tail;
  **`P_RemoveThinker`** (80) only sets `function.acv = -1`; **`P_RunThinkers`** (101)
  frees a marked entry when it reaches it (108–114) and runs the others, so a thinker
  appended during the pass runs in the same pass. Every `EV_*` below appends its mover
  with `P_AddThinker` (doors [p_doors.c:285, 460](../linuxdoom-1.10/p_doors.c), floors
  [p_floor.c:283](../linuxdoom-1.10/p_floor.c), plats
  [p_plats.c:174](../linuxdoom-1.10/p_plats.c)): **a new mover always joins the shared
  order at its tail**, after every mobj and light created before it.
- **`P_Ticker`** (p_tick.c:130): `P_PlayerThink` (148–150), `P_RunThinkers` (152),
  `P_UpdateSpecials` (153), `P_RespawnSpecials` (154), `leveltime++` (157). A door opened
  by **use** is created in `P_PlayerThink`, before the pass, and steps in the same tic; a
  mover created by a **crossing** is created inside the pass (the player's and the
  monsters' moves happen in their mobj thinkers) and steps later in the same pass.
- A mover that removes itself (`pastdest`) keeps its slot until the next pass reaches it,
  while its sector's `specialdata` is cleared at once; a later thinker of the same pass
  can create a replacement (a monster bumping a DR line whose door just closed, a monster
  crossing the WR lift line after the lift stopped), which then also steps this tic. So a
  sector can step **twice** in one tic. For the admitted set the replacement always moves
  in the harmless direction (a new `normal` or `blazeOpen` door rises, a new
  `downWaitUpStay` plat lowers; §2.4, §2.5), and a sector never has more than two mover
  entries in the order: the old one precedes the new one and is freed first.
- **`P_PlayerThink`** ([p_user.c](../linuxdoom-1.10/p_user.c)): a dead player runs
  `P_DeathThink` and returns (258–262); otherwise `P_MovePlayer` (267–270),
  `P_CalcHeight` (272), `P_PlayerInSpecialSector` (274–275), `BT_SPECIAL` clears the
  buttons (280–281), weapon change (283–319), then **use**: with `BT_USE` held and
  `usedown` false it calls `P_UseLines` and sets `usedown`; without `BT_USE` it clears
  `usedown` (322–331); `P_MovePsprites` (334). `G_PlayerReborn` starts a player with
  `usedown = attackdown = true` ([g_game.c:822](../linuxdoom-1.10/g_game.c)), so a use
  held from level start does nothing until released.
- **`P_DeathThink`** (p_user.c:182): **`BT_USE` while dead sets `playerstate =
  PST_REBORN`** (227–228); the next `G_Ticker` calls `G_DoReborn` (g_game.c:613–614),
  which in single player sets `gameaction = ga_loadlevel` (924–931), a level restart. The
  M3h7 note does not mention this; it is reachable as soon as `BT_USE` is admitted (§14
  decision N3).
- **`G_ExitLevel`** (g_game.c:1002) sets `secretexit = false` and `gameaction =
  ga_completed` only; the current tic finishes (thinkers, specials, `leveltime++`) and the
  next `G_Ticker` runs `G_DoCompleted` (636).

### 2.2 Line triggers: `spechit`, crossing and monster door use

- **`PIT_CheckLine`** ([p_map.c:189](../linuxdoom-1.10/p_map.c)): after the bbox and
  `P_BoxOnLineSide` tests (191–199), a one-sided line blocks (211–212); for a non-missile,
  `ML_BLOCKING` blocks and `ML_BLOCKMONSTERS` blocks non-players (214–221); then
  `P_LineOpening`, the `tmceilingz`/`ceilingline`, `tmfloorz` and `tmdropoffz` updates
  (224–235) and **`if (ld->special) spechit[numspechit++] = ld`** with no bound check
  (240–244) against `MAXSPECIALCROSS 8` (67–69). So a line enters `spechit` only if it is
  two-sided and did not block; a **missile** skips the two blocking-flag tests, so a
  blocking two-sided special line enters a missile's `spechit`.
- **`P_CheckPosition`** (375): `ceilingline = NULL` (400), base heights from the centre's
  subsector (406–407), `numspechit = 0` (410); `MF_NOCLIP` returns at once (412–413);
  things in the cells of `tmbbox ± MAXRADIUS` first (420–428), returning false at the
  first blocking thing; then lines in the cells of `tmbbox` only (431–439), `bx` outer,
  `by` inner, each line once per call (`validcount`,
  [p_maputl.c:497–500](../linuxdoom-1.10/p_maputl.c)); `P_BlockLinesIterator` returns
  true for a cell outside the grid (481–487). The `spechit` order is cell order, then
  BLOCKMAP list order, first listing wins.
- **`spechit` is filled by every `P_CheckPosition`**, not only by moves: `P_TryMove`
  (451, for players, monsters and fireballs, including `P_CheckMissileSpawn`) and
  **`P_ThingHeightClip`** (536, for every thing re-clipped by a moving sector). Only two
  places read it: the crossing loop of `P_TryMove` and the failure branch of `P_Move`.
- **`P_TryMove`** (451): on success it sets `floorz`/`ceilingz`, relinks, and unless
  `MF_TELEPORT|MF_NOCLIP` walks `spechit` **in reverse** (`while (numspechit--)`, 502),
  calling `P_CrossSpecialLine(ld - lines, oldside, thing)` for each line whose
  `P_PointOnLineSide` differs between the old and new centres and whose special is still
  nonzero (505–512). Every successful step of `P_SlideMove`, of the split `P_XYMovement`
  steps and of `P_Move` goes through it.
- **`P_CrossSpecialLine`** ([p_spec.c:492](../linuxdoom-1.10/p_spec.c)): a non-player
  returns at once for six missile types including `MT_TROOPSHOT` (503–518) and acts only
  on 39, 97, 125, 126, 4, 10, **88** (520–534). The main switch (539) has **no default
  and no case 48**. Player cases of the admitted set: 36 `EV_DoFloor(turboLower)`, special
  cleared (640–644); 38 `EV_DoFloor(lowerFloorToLowest)`, cleared (652–656); 109
  `EV_DoDoor(blazeOpen)`, cleared (730–734); 88 `EV_DoPlat(downWaitUpStay, 0)`, kept
  (857–860). A W1 line is cleared even when its `EV_*` started nothing. A monster
  crossing 36, 38 or 109 does nothing; **a monster crossing 88 lowers the lift**; a
  fireball crossing anything does nothing.
- **`P_Move`** ([p_enemy.c:272](../linuxdoom-1.10/p_enemy.c)): when `P_TryMove` fails, a
  floater adjusts height (298–308, no admitted floater); else **`if (!numspechit) return
  false`** (310–311); else `movedir = DI_NODIR` and, for every `spechit` entry in reverse,
  `P_UseSpecialLine(actor, ld, 0)`, returning true if any call returned true (313–324). On
  success a non-floater snaps `z = floorz` (332–333). **`P_TryWalk`** (349) draws
  `movecount = P_Random() & 15` (356) only when `P_Move` returned true. The `spechit`
  entries are those of the failed `P_CheckPosition`: none when a thing blocked, the lines
  before the blocker when a line blocked, all of them when the fit, headroom, step or
  dropoff test of `P_TryMove` failed (p_map.c:466–484). A closed DR door's line is
  two-sided, not blocking and has zero opening, so it is in `spechit` and the fit test
  fails: this is how monsters open doors.
- **The `DI_NODIR` reset matters for steering.** `A_Chase` calls `P_Move` directly and
  on failure `P_NewChaseDir` (p_enemy.c:764–767); `P_NewChaseDir` takes `olddir =
  movedir` and `turnaround = opposite[olddir]` (378–379), excludes `turnaround` from the
  direct candidates (403, 416–419) and retries `olddir` (441–446). After the reset both
  are `DI_NODIR`: no direction is excluded and the `olddir` retry is skipped, which
  changes the chosen direction and the number of `P_Random` draws that follow. This
  happens for **any** special line in `spechit` of a failed move, including one no
  monster can use (§2.9).

### 2.3 Use: `P_UseLines`, `PTR_UseTraverse`, `P_UseSpecialLine`

- **`P_UseLines`** (p_map.c:1130): `x2 = x1 + 64 · finecosine[angle]`, `y2` likewise
  (1140–1145), `P_PathTraverse(..., PT_ADDLINES, PTR_UseTraverse)` (1147). The trace is 64
  units long, so `max(|dx|, |dy|) ≥ 45 > 16` units and `PIT_AddLineIntercepts` always
  uses the divline side test (p_maputl.c:569–583); there is no early-out without
  `PT_EARLYOUT` (595–600). `P_TraverseIntercepts` visits by increasing `frac`, the first
  inserted on ties (`<`, 698–706), and stops past `frac = 1` (708–709).
- **`PTR_UseTraverse`** (p_map.c:1095): a line **without** a special continues if its
  opening is positive (`P_LineOpening` gives `openrange = 0` for a one-sided line,
  p_maputl.c:305–310) and otherwise plays `sfx_noway` at the player and stops
  (1099–1111); a line **with** any special is used with `side = 1` when the player's
  centre is on its back (1113–1117) and the trace **stops whatever `P_UseSpecialLine`
  returns** (1119–1122).
- **`P_UseSpecialLine`** ([p_switch.c:276](../linuxdoom-1.10/p_switch.c)): `side != 0`
  returns false except for 124 (284–297); a non-player returns false on an `ML_SECRET`
  line (304–305) or on anything but 1, 32, 33, 34 (307–319); 1 (and 26–28, 31–34, 117,
  118) calls `EV_VerticalDoor(line, thing)` (326–338); 11 calls
  `P_ChangeSwitchTexture(line, 0)` then `G_ExitLevel` (359–363); 63 calls
  `EV_DoDoor(line, normal)` and only on success `P_ChangeSwitchTexture(line, 1)`
  (550–554); it returns true at the end (652) — **for every front-side use of any
  special**, including specials with no use case such as 48.
- **`P_ShootSpecialLine`** (p_spec.c:959), called for every special line a hitscan
  reaches before the two-sided test (p_map.c:919–920): a non-player returns unless 46
  (966–978); the player switch has only 24, 46, 47 (980–999). No admitted line does
  anything when shot.

### 2.4 Doors

- `vldoor_t` ([p_spec.h:343](../linuxdoom-1.10/p_spec.h)): `type`, `sector`,
  `topheight`, `speed`, `direction`, `topwait`, `topcountdown`; `VDOORSPEED` 2 units,
  `VDOORWAIT` 150 tics (364–365).
- **`EV_VerticalDoor`** (p_doors.c:354), line 1: `side = 0` (364), locks for 26–28 and
  32–34 only (369–410); `sec = sides[line->sidenum[side ^ 1]].sector` (413): **the back
  sector, read with no two-sided check** (a one-sided line reads `sides[-1]`). If
  `sec->specialdata` is set it is **cast to a door** (418) and, for 1, 26–28, 117: a door
  going down (`direction == -1`) goes back up; otherwise a monster returns ("bad guys never
  close doors", 431) and a player sets `direction = -1` (421–435); then return. Otherwise
  it plays `sfx_doropn` for line 1 **unconditionally** (447–449; no `topheight` test,
  unlike `EV_DoDoor`), appends a door (459–466) with `direction 1`, speed 2, wait 150,
  `type = normal` for line 1 (470–474), and `topheight = P_FindLowestCeilingSurrounding(sec)
  − 4` (497–498). Line 1 is not cleared (DR). `topcountdown` is not initialised
  (`Z_Malloc` does not zero) and is first read after the door has waited (§2.4 last
  bullet).
- **`EV_DoDoor`** (p_doors.c:264): for each tagged sector in index order
  (`P_FindSectorFromLineTag`, p_spec.c:434–442, tag 0 selects every untagged sector) whose
  `specialdata` is null (278–279), a door (284–291) with wait 150, speed 2; `blazeOpen`:
  `direction 1`, top − 4, **speed 8**, `sfx_bdopn` only if `topheight != ceiling`
  (320–329); `normal`: `direction 1`, top − 4, `sfx_doropn` only if `topheight !=
  ceiling` (331–339). It returns 1 if any door was created.
- **`P_FindLowestCeilingSurrounding`** (p_spec.c:382) starts at `MAXINT` (387) and takes
  the minimum ceiling of `getNextSector` neighbours (251–261: `ML_TWOSIDED` lines only,
  the other side, **the sector itself for a line with that sector on both sides**). A
  door sector without a two-sided neighbour gets `MAXINT − 4` as its top.
- **`T_VerticalDoor`** (p_doors.c:63), plane `1` (ceiling), `crush = false`:
  waiting (`direction 0`): `--topcountdown`, at 0 a `normal` door turns down with
  `sfx_dorcls` (69–85); down (118–123): `pastdest` removes a `normal` door and clears
  `specialdata` (124–140); **`crushed` turns any door but `close`/`blazeClose` back up
  with `sfx_doropn`** (151–162) — this is the reopen on obstruction; up (168–173):
  `pastdest` makes a `normal` door wait `topwait` tics (175–183) and removes a
  `blazeOpen` door (185–190). A `blazeOpen` door that is going down (only possible if a DR
  line on the same sector reversed it) has no `pastdest` case going down (`default`,
  147–148): it stays as a thinker at the floor, stepping to `dest` every tic, until a DR
  use sends it up again; `crushed` sends it up (159–162). No `P_Random`.

### 2.5 The lift (88) and `activeplats`

- `plat_t` (p_spec.h:285): `sector`, `speed`, `low`, `high`, `wait`, `count`, `status`,
  `oldstatus`, `crush`, `tag`, `type`; `PLATWAIT` 3, `PLATSPEED` 1, `MAXPLATS` 30
  (304–306); `activeplats[MAXPLATS]` (p_plats.c:45).
- **`EV_DoPlat`** (p_plats.c:139): for each tagged sector without `specialdata`
  (168–169): a plat appended (173–174), `crush = false`, `tag` (180–181);
  `downWaitUpStay` (207–218): speed **4**, `low = P_FindLowestFloorSurrounding(sec)`
  (starts at the sector's own floor, p_spec.c:270–288) clamped to the current floor,
  **`high = the current floor`**, `wait = 105`, `status = down`, `sfx_pstart`; then
  `P_AddActivePlat` (251), which stores it in the first free of 30 slots and `I_Error`s
  without one (288–299). `oldstatus` is never written for this type. Only
  `perpetualRaise` draws `P_Random` (246).
- **`T_PlatRaise`** (p_plats.c:52): up (58–106): `T_MovePlane(floor, up, crush)`;
  **`crushed` sends it down** with `count = wait`, `sfx_pstart` (73–79); `pastdest` waits,
  `sfx_pstop`, and for `downWaitUpStay` **`P_RemoveActivePlat`** (82–94), which clears
  `specialdata` and the slot and marks the thinker (301–313); down (108–117): `pastdest`
  waits with `sfx_pstop`; waiting (119–127): `--count`, at 0 `up` if `floor == low` else
  `down`, `sfx_pstart`. Because `high` is the floor at activation and the last up step can
  stop short (§2.7), a lift whose last step was refused is re-armed with a lower `high`
  the next time: the floor can ratchet down by less than one step per cycle, never up.
- `activeplats` holds at most one entry per plat sector (`specialdata` excludes a second
  plat), its slot index is read only by `EV_StopPlat`, `P_ActivateInStasis` and
  `P_RemoveActivePlat`'s search, none of which the admitted set reaches observably. Every
  slot is cleared at load (p_spec.c:1354–1355).

### 2.6 Floors (36, 38)

- `floormove_t` (p_spec.h:584): `type`, `crush`, `sector`, `direction`, `newspecial`,
  `texture`, `floordestheight`, `speed`; `FLOORSPEED` 1 (600).
- **`EV_DoFloor`** (p_floor.c:260): for each tagged sector without `specialdata`
  (277–278), a floor appended (282–284), `crush = false` (287);
  `lowerFloorToLowest` (299–305): down, speed 1, to `P_FindLowestFloorSurrounding`;
  `turboLower` (307–315): down, **speed 4**, to `P_FindHighestFloorSurrounding` (starts at
  −500, p_spec.c:302), **+ 8 unless it equals the current floor**. `newspecial` and
  `texture` stay uninitialised for both types and are never read for them.
- **`T_MoveFloor`** (p_floor.c:209): `T_MovePlane(floor, speed, dest, crush, 0,
  direction)`; `sfx_stnmov` when `!(leveltime & 7)` (218–220); on `pastdest` clears
  `specialdata`, applies `newspecial`/`texture` only for `donutRaise`/`lowerAndChange`,
  removes itself and plays `sfx_pstop` (222–252).
- A turbo destination above the floor is reached in **one tic** (the down branch sets the
  floor to `dest` once `floor − speed < dest`, §2.7): the *turbo jump-up*. E1M1's 36
  target has none (measured jump-up 0).

### 2.7 `T_MovePlane`, `P_ChangeSector`, `PIT_ChangeSector`, `P_ThingHeightClip`

- **`T_MovePlane`** (p_floor.c:49), every admitted call with `crush = false`:
  - **Last step** (`floor − speed < dest` going down, 67; `floor + speed > dest` up, 96;
    `ceiling − speed < dest` down, 134; `ceiling + speed > dest` up, 168): set the plane to
    `dest`, `P_ChangeSector`; if it returns true restore the old height and call it again;
    **return `pastdest` either way** (67–80, 96–109, 134–148, 168–181). A blocked last step
    stops short and the mover ends there.
  - **Ordinary step** down or up by `speed`: on true restore, call again and return
    `crushed` (83–92, 112–124, 151–164) — for floor up and ceiling down the `crush ==
    true` early return (118–119, 158–159) is unreachable here. A **rising ceiling never
    restores** (the restore is under `#if 0`, 187–195), so it ignores `nofit`.
  - Otherwise `ok` (199).
- **`P_ChangeSector`** (p_map.c:1321): `nofit = false`, `crushchange = crunch` (1328–1329),
  then `P_BlockThingsIterator(x, y, PIT_ChangeSector)` over the sector's `blockbox`,
  **x outer, y inner** (1332–1334); returns `nofit`. The `blockbox` is the sector's line
  bbox ± `MAXRADIUS`, clamped to the grid ([p_setup.c:559–574](../linuxdoom-1.10/p_setup.c)).
  Only BLOCKMAP-linked things are visited (fireballs, puffs and blood are
  `MF_NOBLOCKMAP`); every linked thing whose centre lies in those cells is re-clipped
  whether or not it touches the sector, and **`nofit` is global**: one shootable thing
  that does not fit anywhere in the blockbox refuses the step, even if the moving sector
  is not under it.
- **`P_BlockThingsIterator`** (p_maputl.c:513) follows `bnext` after each call (531–537).
  `P_RemoveMobj` ([p_mobj.c:546](../linuxdoom-1.10/p_mobj.c)) unlinks with
  `P_UnsetThingPosition` (p_maputl.c:347), which patches the predecessor's `bnext` and
  leaves the removed thing's own `bnext` intact, then only marks the thinker (p_mobj.c:569).
  So a thing removed while it is the current element (a dropped item crushed) is left
  safely, a thing removed elsewhere in the cell (an item picked up inside a nested clip)
  is simply skipped, and no removed thing is visited.
- **`PIT_ChangeSector`** (p_map.c:1257): if `P_ThingHeightClip` succeeds, continue
  (1261–1265); else a thing with `health <= 0` becomes `S_GIBS`, loses `MF_SOLID`, height
  and radius 0 (1269–1279); else an `MF_DROPPED` thing is removed (1282–1288); else a
  non-shootable thing is left as it is (1290–1294); else `nofit = true` (1296), and the
  damage and blood branch (1298–1309) runs only with `crushchange`, **unreachable** for
  the admitted set.
- **`P_ThingHeightClip`** (p_map.c:530): `onfloor = (z == floorz)` (534); a full
  `P_CheckPosition(thing, x, y)` whose **result is ignored** (536); `floorz`/`ceilingz`
  from `tmfloorz`/`tmceilingz` (539–540); an onfloor thing is set to the new floor (542–546),
  otherwise only pushed down to `ceilingz − height` when its top is above it (548–552);
  it fails when `ceilingz − floorz < height` (554–555). Consequences:
  1. **Pickups inside a clip.** The clip runs `PIT_CheckThing` with the clipped thing as
     `tmthing`, so a re-clipped player (`MF_PICKUP`) touches every special thing
     overlapping its box (`P_TouchSpecialThing`, p_map.c:333–339; its z test and dead-
     toucher return, [p_inter.c:350, 363](../linuxdoom-1.10/p_inter.c)).
  2. **Thing overlap.** `PIT_CheckThing` returns false for an overlapping `MF_SOLID`
     thing (p_map.c:258–272, 342), so the clip takes the **centre subsector's** floor and
     ceiling and skips the line pass. This is **routine** for a non-solid clipped thing
     (an item under a monster, a corpse under the player). For a solid clipped thing it
     needs two overlapping solid linked things: every admitted motion of a solid thing
     goes through `P_TryMove`, which refuses an overlap; no admitted body spawns a solid
     thing at runtime (drops are `MF_SPECIAL` non-solid, p_inter.c:756–757; missiles,
     puffs and blood are not linked); `MF_SOLID` is only ever cleared (`A_Fall`,
     p_enemy.c:1588; gibs); and the clip never moves `x`, `y`. So it arises only from
     **spawn placement** (map THINGs or the player body), which the Rust spawn rules allow
     (§3).
  3. **First box-wide clip.** `P_SpawnMobj` takes the centre sector's heights
     (p_mobj.c:519–520) and places an `ONFLOORZ` thing on that floor; a thing never moved
     since gets its first box-wide `floorz`/`ceilingz` at its first clip, rising onto a
     step its box overlaps, and possibly not fitting there.
  4. **Partial line pass.** A thing whose box crosses a one-sided or (for its kind)
     blocking line gets heights from the lines before that line only; for linked things
     this too needs spawn placement (M3h5 escape witness and `P_TryMove` exclude it
     otherwise).

### 2.8 Switches, buttons and the exit line

- **`P_ChangeSwitchTexture`** (p_switch.c:201): `if (!useAgain) line->special = 0`
  (211–212) **before** the `line->special == 11` exit-sound test (220–222), so the exit
  switch plays `sfx_swtchn`; the sound origin is `buttonlist->soundorg`, the first slot's
  (228, 240, 252); it swaps the front side's texture against `switchlist` (224–262) and,
  for a reusable switch, calls `P_StartButton` — **only if the texture was found** in
  `switchlist`.
- **`P_StartButton`** (p_switch.c:155): returns if a running button already holds this
  line (162–170), so a line never has two slots; else takes the first free of
  `MAXBUTTONS 16` slots (p_spec.h:244) with `BUTTONTIME 35` (247), or `I_Error`s (189).
  Concurrent buttons are therefore at most the number of reusable switch lines.
- `P_UpdateSpecials` counts buttons down and restores the texture with `sfx_swtchn`
  (p_spec.c:1127–1153); buttons and textures are cleared at load (1357–1358).
- **Gameplay content**: the cleared special (11 is S1, cleared; 63 is SR, never cleared,
  and its texture only changes when `EV_DoDoor` started a door), the `EV_*` effects and
  the `I_Error` bounds. Texture numbers are read in game code only by `raiseToTexture`
  (not admitted), so switch textures, button timers and every sound are presentation;
  `S_StartSound` draws `M_Random` only.
- **Exit (11)**: front-side use by the player clears the special and calls `G_ExitLevel`;
  back-side use returns false (the trace still stops); a monster cannot use it.

### 2.9 What a two-sided scrolling wall (48) does in game code

E1M1's eight scrollers are two-sided and not blocking (`61d6087`). Every game-code read of
a line's special, checked for 48:

| Path | Effect of a two-sided 48 | Where |
| --- | --- | --- |
| `PIT_CheckLine` | **enters `spechit`** for every `P_CheckPosition` whose box crosses it (players, monsters, fireballs, re-clipped things), consuming one of the 8 slots; blocking flags would keep it out for non-missiles only | p_map.c:240–244 |
| `P_TryMove` crossing loop | calls `P_CrossSpecialLine` on a crossing: monsters and fireballs return before the switch, the player's switch has no 48 case — **no effect** | p_map.c:502–512; p_spec.c:503–534, 539 |
| `P_Move` failure branch | a failed monster move with the line in `spechit` **resets `movedir` to `DI_NODIR`** and calls `P_UseSpecialLine(actor, 48, 0)`, which returns false (non-player, not 1/32/33/34); `P_Move` returns false unless another entry opened a door — **steering changes** (§2.2) | p_enemy.c:310–324; p_switch.c:301–319 |
| `PTR_UseTraverse` | a use trace reaching it calls `P_UseSpecialLine` (front side: true, nothing happens; back side: false) and **stops**: no `sfx_noway`, and no door or switch beyond it within the 64 units is used | p_map.c:1113–1122 |
| `PTR_ShootTraverse` | calls `P_ShootSpecialLine`, which does nothing for 48, then treats the line as any two-sided line — **no effect** | p_map.c:919–920; p_spec.c:966–999 |
| `P_UpdateSpecials` | `textureoffset += 1` of side 0 every tic, presentation (as for one-sided 48) | p_spec.c:1114–1124 |
| Sight, autoaim, noise, `P_LineOpening` | no special test — no effect | p_sight.c, p_map.c `PTR_AimTraverse`, p_enemy.c `P_RecursiveSound` |

Ordering: a 48 entry changes no other entry's effect (its crossing and use calls do
nothing), so `spechit` order matters only through the count. With at most 8 entries per
`P_CheckPosition` (rule 6, §6) there is no overflow; E1M1's largest window holds exactly
8 lines, all of them scrollers (the largest window of its other two-sided special lines
holds 4). So every body a two-sided 48 needs — `spechit`, the `P_Move` failure branch,
the special stop of the use trace — is a Stage B body anyway; under Stage A (no `spechit`
reader, no use) only the `DI_NODIR` reset would have been missing, which is why M3h7
excluded it.

### 2.10 Sounds

Admitted sector sounds, all at `sector->soundorg` (the line-bbox centre,
p_setup.c:556–557): `sfx_doropn` (door start, DR always, `EV_DoDoor` normal only when the
top differs; reversal), `sfx_bdopn` (109 when the top differs), `sfx_dorcls` (a waiting
door turning down), `sfx_pstart`/`sfx_pstop` (plat), `sfx_stnmov` (floor, every 8 tics),
`sfx_pstop` (floor stop); switch sounds at `buttonlist[0].soundorg`; `sfx_noway` at the
player. None is gameplay (decision 5 of M3h7: recorded like the existing sound requests).

### 2.11 Every `P_Random` draw on the Stage B paths

| Path | Draws |
| --- | --- |
| `T_MovePlane`, `P_ChangeSector`, `P_ThingHeightClip`, gibs (`S_GIBS` has no action), dropped-item removal | none (crush blood unreachable, §2.7) |
| `EV_DoDoor`, `EV_VerticalDoor`, `T_VerticalDoor`, `EV_DoPlat(downWaitUpStay)`, `T_PlatRaise`, `EV_DoFloor`, `T_MoveFloor` | none |
| `P_UseLines`, `PTR_UseTraverse`, `P_UseSpecialLine`, switches, `G_ExitLevel`, `P_CrossSpecialLine` | none |
| Pickups inside a clip (`P_TouchSpecialThing`) | none |
| Monster door use through `P_TryWalk` | `movecount = P_Random() & 15` (p_enemy.c:356) — and, through the `DI_NODIR` reset, a different `P_NewChaseDir` path with its own existing draws (390–460) |
| Sounds | `M_Random` only |

### 2.12 Corrections and refinements to the M3h7 note

Re-reading the sources for this note confirmed §2.1–§2.8 of the M3h7 note, with these
refinements: (1) `spechit` is also filled by `P_ThingHeightClip` and by fireball moves,
and a missile skips the blocking-flag tests (§2.2); (2) a sector can step twice in one
tic and hold two mover entries (§2.1), so "at most one mover per sector" in M3h7 §4.5 is
per live `specialdata`, not per order entry; (3) `BT_USE` while dead requests a level
restart (§2.1); (4) `usedown` starts true (§2.1); (5) `EV_VerticalDoor` plays its sound
unconditionally (§2.4); (6) buttons are deduplicated per line (§2.8); (7) the overlap
case of `P_ThingHeightClip` is routine for non-solid things and spawn-only for solid ones
(§2.7); (8) uninitialised mover fields (`topcountdown`, `newspecial`, `texture`,
`oldstatus`) are never read before written for the admitted types; (9) the removal of a
thing during the `P_ChangeSector` iteration is safe (§2.7).

## 3. Rust inventory

Paths are under `crates/doom-sim/src/` unless stated.

| Source body | Rust today | Stage B change |
| --- | --- | --- |
| Policy | `OriginalExecutionProfile` (`original_registry.rs:12`), `PlayerMonstersSpecialsV1` (62), `wire_id` 18 (85), `context_schema` (108), predicates `has_specials` (208) and the other `has_*` (111–291), `OriginalExecutionRegistry::new` (654), `validate` (705), `with_original_registry` (896, `specials:` at 969); context constant `PLAYER_SPECIALS_CONTEXT_SCHEMA` (`player_monsters.rs:29`); constructor `Simulation::original_player_specials_map` (`player_monsters.rs:151–163`) | New variant `PlayerMonstersMoversV1` (wire 19), `has_movers()`, `has_specials()` true for both; context `PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1`; constructor `original_player_movers_map` |
| Stage A load rules | `validate_specials` (`demon_expanded_map.rs:220–248`): admitted sector specials, **48 only if `line.sides[1].is_none()` (233)**, `MAX_LINE_ANIMS` (237–239), `LIGHT_RANGE` (240–246); errors `PursuitError::Specials` (`pursuit.rs:24`) and `MapScaleError::Light` | Under movers only: 48 two-sided (decision N1) and the seven types, plus the §7 rules; Stage A unchanged |
| Errors | `MapScaleError` (`player_map_scale.rs:52–101`: `Walls` … `Intercepts`); no `Specials`, no `SpecialCross` | New `MapScaleError::SpecialCross` (runtime witness) and field-less load-rule variants (`Movers` or one per rule) |
| Thinker store | `ThinkerStore<T, S = SectorThinker>` (`store.rs:268–276`), `ThinkerId::{Actor, Sector}` (51–56), `SectorThinkerId` (29), `Visit` (120–129), `with_sector_capacity` (297), `visit_bound = max_slots + max_sector_slots` (316–318), `spawn_sector` (412), `mark_remove_sector` (432), `shared_order` (439), `run` (550), `ThinkerContext::{sector_get_mut, spawn_sector}` (606, 622): one linked order, lazy removal, appends visible in the same pass | Unchanged mechanism; the sector slot class capacity becomes `L + 2M` (§8) |
| Sector thinkers | `SectorThinker::{Flash, Strobe, Glow, Flicker}` (`sector_thinkers.rs:93–102`), `think` (137), `visit` (221); run from `thinkers::run` (`thinkers.rs:24`, sector arm 49–62); capacity from `install_specials` (`demon_expanded_map.rs:164–192`) | New variants `Door`, `Plat`, `Floor` |
| Sector heights | `SectorState { floor, ceiling }` (`multisector.rs:16–21`), `Simulation::sectors` (`lib.rs:527`), built once (`lib.rs:703–713`); the plan borrows `&self.sectors` read-only (`demon_expanded_map.rs:529, 554, 614, 686`) | Written inside the pass, visible to later thinkers, staged with the plan (sparse overlay, §9) |
| Runtime light/special | `SectorSpecial { light, special }` (`sector_thinkers.rs:118`), `Simulation::specials`/`spare_specials` (`lib.rs:555, 558`), staged in `DemonPlan::specials` | Unchanged; plus runtime line specials (W1/S1 clears) |
| Checked tic plan | `DemonPlan` (`demon_expanded_map.rs:317–330`: actors, spatial, rng, statistics, player, bindings, sound targets, specials, events), `demon_plan` (332–392), `commit_demon_plan` (478–502), `advance_expanded_demons` (580–732), `preflight_expanded_demons_for` (419) | Gains the sector overlay, the line-special overlay, `level.exit`, the player's use latch |
| Collision | `multisector::opening` (283–301), `check_move` (303), `check_move_from_things` (315–396) returning `SectorMoveAttempt { …, openings: Vec<OpeningVisit> }` (95–112) in visit order; a thing block returns with no openings (335–338); a one-sided or blocking line returns at that line (361–366) | `spechit` = the special subsequence of `openings` (no new array); Phase 2 must confirm that a line failure keeps the openings before the blocker and that a missile's attempt keeps blocking two-sided lines |
| `ceilingline` | derived from `openings` (`projectiles.rs:59–69`) | unchanged |
| `P_Move` | `pursuit::move_actor` (`pursuit.rs:164–198`): on success sets position, heights, `z = floor`, relinks; on failure returns false — no `DI_NODIR` reset, no `spechit` loop; `try_walk` (199); chase failure → `new_direction` (`pursuit/chase.rs:64–66`) | The failure branch of §2.2 and the crossing loop on success |
| Player move | `Mover::attempt` (`movement.rs:540–590`), `Mover::slide` (597–679), `move_xy_profile` (727), `move_xy_contacts` (813, called from `thinkers.rs:128`) | Crossing loop after every accepted step |
| Fireball move | `projectiles::try_move` (191–220), `move_xy` (223–258) | `spechit` filled (witness), crossing loop returns at once for `MT_TROOPSHOT` |
| Path traversal | `movement::path` (`movement.rs:330–426`, private, lines only in fraction order, `InterceptOverflow` 323); `combat_trace::collect_limited` (`combat_trace.rs:140–320`) with the 128 cap | `P_UseLines` reuses `path` (made crate-visible) |
| Vertical invariant | `vertical_admitted` (`player_map_scale.rs:602–611`: z and momz witnesses, `floor ≤ z`, `z + height ≤ ceiling` on the actor's own fields), called in preflight (`demon_expanded_map.rs:465`) and for the player (`original_player.rs:351`) | V′ under movers only (§4.1) |
| Item witness | `validate_item` (`world_pickups.rs:301`): under heights `z == floor ==` centre-sector floor and `ceiling ==` centre-sector ceiling (309–318, 345–347); callers 517, 766, `demon_map_spawn.rs:94` | Relaxed under movers only (§4.2) |
| Spawn | `spawn_fits` (`player_map_scale.rs:564–578`) checks the **centre sector only**; `spawn_admitted` (705–711: room box plus `Boundary::spawn_admitted`, 507, centre in Ω, box meets no one-sided wall, `box_meets_wall` 494); `make_actor` (`lifecycle.rs:1184–1248`) sets `floor`/`ceiling` from the centre sector (1196–1211); **overlapping initial placements are allowed** (`lib.rs:923`) | Unchanged (faithful); it makes §2.7 consequences 2–3 reachable (§4.1) |
| Roster | demon radius 30 (largest), former humans and imp 20, player 16, fireball 6, pickups 20 × 16 (`source_actors.rs:1305–1337`, `world_pickups.rs:341–342`); `MAX_PLAYER_MONSTERS` 8 (`player_monsters.rs:12`), `MAX_WORLD_PICKUPS` 16 (`world_pickups.rs:7`), `MAX_LINKED_THINGS` 33 (`player_map_scale.rs:48`), 320 slots (12), 208 fireballs (18), `MAX_INTERCEPTS` 128 (749) | unchanged |
| Buttons | `preflight_player_commands` (`player_commands.rs:80–91`) allows `BT_ATTACK | BT_CHANGE | BT_WEAPONMASK` (`weapons.rs:8–12`); no `BT_USE` constant; no use latch (`PlayerRuntime`, `original_player.rs:105–116`; `attack_down` starts true, `player_weapons.rs:42`) | `BT_USE` admitted under movers, `use_down` starting true |
| Sounds | `ActorStateEvent::Sound { actor: Option<ActorId>, sound }` (`lifecycle.rs:1108–1113`), `None` = global; encoded tag 3 (`snapshot.rs:1258–1261`) | A sector-origin sound request |
| Level | `LevelStatistics` (`demon_map_context.rs:7–19`); no exit or reborn state | `level.exit`, the reborn request (decision N3) |
| Phases | `Phase::{Players, Thinkers, Specials, Respawn}` (`lib.rs:350–359`); `Specials` is a marker (`demon_expanded_map.rs:727–728`) | Still a marker: buttons and scrolls are presentation |
| DSIM34 | version chosen at `snapshot.rs:565–566`; `thinker.shared_order` (673–686); sector thinker fields (811–813, 1046–1105); `sector.{i}.floor`/`ceiling` already encoded (844–850), `.light`/`.special` (879–891); `original_player.0.secret_count` (1014–1021); event tag 25 (201–219); `Diagnostic::decode_specials` (`diagnostic.rs:226–231`) reads 34 only, `decode_selected` rejects other versions (260–275) | DSIM35 (§10) |
| M3h7 evidence | `tools/reference/m3h7_{geometry,cases,reference,snapshot_goldens}.py`, `specials_snapshot.py`, `check_player_specials.py`; `tests/reference/m3h7_*`; `player_specials_tests.rs`; example `player_specials`; fuzz `player_specials`; `tools/seed_player_specials_fuzz.py`, `tools/measure_specials.py` | M3h8 counterparts (§11) |

What does not exist and must be written: `spechit` reading, crossed-line recording, the
use trace, the use latch, mover thinkers, any writer of sector heights, the per-sector
`blockbox`, `P_ChangeSector`, a box-wide `P_ThingHeightClip`, sector-origin sounds and
the exit and reborn state.

## 4. Admitted domain and proofs

### 4.0 The admitted domain

`PlayerMonstersMoversV1` (wire 19, DSIM35, `PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1`) is a
superset of `PlayerMonstersSpecialsV1` with the same roster, caps, certificate, boundary
rules, witnesses and transaction, plus:

- line specials **1, 63, 109, 88, 36, 38, 11** (exactly E1M1's tag/use/cross types,
  approved), under the load rules of §7;
- line special **48 on two-sided lines** under this policy only, if decision N1 is
  approved (Stage A keeps its one-sided rule);
- `BT_USE` with the `usedown` latch;
- moving floors and ceilings, `P_ChangeSector`, crossing, use, monster door use, the
  `spechit` window rule and `SpecialCross` witness, the exit.

Still rejected (unchanged or new load rules): every other line special — locked doors
(26–28, 32–34; no keys in the roster), the other door, plat, floor, stair, donut,
ceiling and crusher types (crushers deferred: 6, 25, 44, 56, 73, 77, 94, 141 and every
ceiling), teleports (39, 97, 125, 126), `raiseToTexture`, perpetual and stopping plats
(53, 54, 87, 89), light-switching lines, gun lines (24, 46, 47), the other exits (51, 52,
124); sector specials 10, 11, 14 and unknown values; `F_SKY1` ceilings on mover sectors
(decision N4); and every map failing §7. Everything rejected by M3h7 for other reasons
stays rejected.

### 4.1 B2: the vertical invariant V′

**Admitted directions.** Ceiling down: doors 1 and 63 closing, speed 2 (`s = 2`). Ceiling
up: doors opening, speed 2 or 8, and a door ceiling above its top dropping to the top in
one tic (*door top drop*, measured 0 on every IWAD map). Floor down: 38 speed 1, 36
speed 4 toward a lower destination, 88 lowering speed 4. Floor up: 88 rising speed 4
(`r = 4`) and the 36 jump-up `J` (0 on E1M1 and every IWAD map). `crush` is false
everywhere. Let **Δ = max(largest one-tic ceiling drop, largest one-tic floor rise)** —
exactly the tool's *B2 Δ* (§5), 4 for E1M1.

**Case table** (`T` a linked thing in the blockbox of the moving sector, `f`, `c` its
stored heights before the step, `f′`, `c′` the clip's result; "fits" is `c′ − f′ ≥ h`):

| Step | `T` on its floor (`z == f`) | `T` not on its floor |
| --- | --- | --- |
| any, fits | `z = f′` → (a) | `z` unchanged or pushed to `c′ − h ≥ f′` → (a); below `f′` only after a floor rise: `z > f ≥ f′ − r` → (c) |
| any, does not fit, shootable | `nofit`; the step is undone (or stops short at `pastdest`) and the restore clip gives back the pre-step box values; `z = f` → the pre-step state, (a) if `T` fitted box-wide before, else (d) | `nofit`, undone; a ceiling push leaves `z = c′ − h ≥ c − s − h`, and with `c − h ≥ f` (pre-step fit) `f − s ≤ z < f` → (c) |
| any, does not fit, non-shootable | the step stands; corpse → gibs (height 0); dropped item → removed; other → (b) | corpse pushed to `c′ − h` then gibs; others unchanged (no non-shootable non-corpse is ever off its floor: items spawn `ONFLOORZ` and onfloor things follow their floor) → (e) |
| rising ceiling, any | ignores `nofit` except at `pastdest`; never lowers a ceiling except the door top drop, which is a ceiling drop covered by Δ | same |

**V′** (checked at every tic boundary for every non-impact actor, with the existing `z`
and `momz` witnesses, under the movers policy only):

- **(a)** `floorz ≤ z ≤ ceilingz − height`;
- **(b)** not `MF_SHOOTABLE`, `z == floorz`, `ceilingz < z + height` (stuck under a
  closed door);
- **(c)** `floorz − Δ ≤ z < floorz` and `z + height ≤ ceilingz` (sunk by at most one
  step; landed by the thing's next `P_ZMovement`, which every linked mobj runs when
  `z != floorz`, p_mobj.c:428–431, 284–311);
- **(d)** `MF_SHOOTABLE`, `z == floorz`, `ceilingz < z + height` (stuck shootable, spawn
  origin only — below);
- **(e)** a corpse or gibs (`health ≤ 0`, not shootable) off its floor:
  `R_min − H_max ≤ z < floorz`, `z + height ≤ ceilingz`, with `R_min` the smallest
  reachable height (§5) and `H_max` the largest admitted height (56).

**Why (c) holds with Δ.** Between two consecutive steps of one sector a linked thing runs
its own thinker exactly once (one entry each in the shared order), so it is landed at most
one tic after any sink. A floor rise sinks it by less than `r` relative to its `z` at its
last landing; a ceiling push puts it at `c − s − h` with `c` the *restored* box ceiling,
so a second push in the same tic (another door) does not stack, and a later rise of
another sector that still fits keeps `z ≥ f′ − s` (fit gives `f′ ≤ c − h`). Two steps of
the same sector in one tic (§2.1) are a down-then-up or up-then-down pair whose second
step is harmless. Hence `z ≥ floorz − max(s, r, J, door top drop) = floorz − Δ`.

**Why (e) is needed.** A non-shootable corpse is not restored: a push to `c′ − h` that
does not fit gibs it and the step stands, and later closers can push the gibs again, by
at most `s` each, before its next landing. The bound `z ≥ R_min − H_max` (`c′` is at least
the lowest reachable height, minus the pre-gib height) is what the M3h4 arithmetic needs
(B5); a tighter per-map bound is possible but buys nothing.

**The overlap / restore-fails case (M3h7 §2.6 consequence 2) is reachable.** For a
non-solid clipped thing it is routine (an item under a monster, §2.7) and only affects
the stored heights, which (a)–(c) are stated on. For a **shootable** thing the restore
clip can fail only if the thing did not fit box-wide before the step, which needs stale
spawn heights (first box-wide clip, consequence 3), a solid overlap (consequence 2) or a
partial line pass (consequence 4) — every one of them a spawn-placement fact, since every
admitted motion re-establishes a fitting box-wide state and every undone step restores
it. The Rust spawn rules check the centre sector only and allow overlapping placements
(§3), so (d) is reachable on admitted maps (for example a monster placed with its box
overlapping a closed door sector, at the first tic that door moves). **Recommendation
(decision N2): admit (d) and (e) explicitly in V′** rather than adding spawn rules that
would reject maps; a stuck shootable thing then refuses every non-rising step of every
mover whose blockbox contains it, exactly as in the source.

### 4.2 B3: the relaxed item witness

Under the movers policy `validate_item` keeps every non-height check and requires
**`z == floorz`** with `floorz`/`ceilingz` the item's own stored fields, plus either
`ceilingz − floorz ≥ height` or case (b); it no longer requires equality with the
centre sector's heights. Witness: items spawn `ONFLOORZ` (p_mobj.c:522–523,
p_inter.c:756); no admitted body gives an item momentum (M3h6 `set_momentum` refuses
pushes to items); `P_ThingHeightClip` keeps an onfloor thing on its new floor (542–546),
box-wide or centre-only (overlap); so `z == floorz` holds after every clip. A dropped
item that does not fit is removed, an undropped one stays in (b). Older policies keep the
centre-sector form.

### 4.3 B1, B5, B7, B10 re-checked

- **B1 (Invariant Ω, escape witness)**: unchanged. `P_ThingHeightClip` never changes `x`,
  `y`; no admitted special teleports; one-sided walls never move; no mover changes a
  line's sidedness or flags; `spechit`, crossing and use add no horizontal motion. The
  `DI_NODIR` reset only changes which direction a later `P_TryMove` tries, which Ω
  already covers.
- **B5 (M3h4 vertical arithmetic)**: every reachable height is within ±512 (rule 4, §5),
  every `z` is within `[R_min − 56, R_max]` by V′, so the fireball chord, the 2,048 `z`
  witness and the eye and opening slopes carry over unchanged.
- **B7 (intercept witness)**: the use trace is 64 units, adds lines only, and changes
  cell at most once per axis, so its lines are among one 2 × 2 window's: the slide-window
  rule (≤ 128 distinct lines) bounds it, and the existing witness stays unreachable from
  use.
- **B10 (pickups in a clip)**: the player's clip reuses the existing touch path
  (`move_xy_contacts`/`world_pickups::Contacts`) in `PIT_CheckThing` cell order; an item
  picked up inside a clip is unlinked at once and, by §2.7, is not visited again by the
  enclosing `P_ChangeSector`; a dead player picks nothing up (p_inter.c:363).

## 5. The reachable-height fixpoint

**Definition.** For every sector `i`, intervals `F[i] = [lo, hi]` of its floor and
`C[i]` of its ceiling, initialised to the static heights, closed under the admitted rules
over the map's admitted lines, with `N(i)` the `getNextSector` neighbours (`ML_TWOSIDED`
lines, the sector itself for a self-referencing line) and `top(i) = [min_{n∈N(i)} C[n].lo
− 4, min_{n∈N(i)} C[n].hi − 4]`:

- door sectors of 1 (back sector) and 63 (tagged): `C[i] ⊇ [min(F[i].lo, top.lo), top.hi]`;
- 109 targets: `C[i] ⊇ top(i)`, plus `C[i]` as it is (the top drop is inside the hull);
- 88 targets: `F[i] ⊇ [min_{n} F[n].lo, F[i].hi]`;
- 38 targets: `F[i] ⊇ [min_{n} F[n].lo, F[i].hi]`;
- 36 targets: `F[i] ⊇ [max_{n} F[n].lo + 8, max_{n} F[n].hi + 8]` unless every neighbour
  floor and the own floor are one equal value (no + 8, no movement).

Every admitted body moves a plane monotonically toward a destination computed from the
current neighbour heights by `min`, `max` and constants, stops short only at heights
between its start and the destination, and never reads anything else; so every height a
plane takes lies in the hull of its start and a destination that the rules above contain,
and the least fixpoint over-approximates every reachable height (including the lift's
downward ratchet, §2.5).

**Termination.** Endpoints only widen and are integers (every speed and destination is an
integer number of units). The Rust admission stops and rejects (rule 4) as soon as an
endpoint leaves ±512, so before that each round widens some endpoint of `2 · sectors`
intervals by at least one unit inside `[−512, 512]`: at most `4 · sectors · 1,025`
rounds, each `O(lines)`. An inverting door is detected after the fixpoint (rule 4b).

**What it yields.** `R_min`, `R_max` (rule 4), the inverting-door test (rule 4b: the
smallest possible top `top(i).lo` below `F[i].hi`), the turbo jump-up `J = max(max_n
F[n].hi + 8 − F[i].lo)` over 36 targets that move up, the door top drop
`max(C[i].hi − top(i).lo)` over door sectors, and **Δ = max(2 if 1 or 63 is present, 4 if
88 is present, J, door top drop)**.

**Against `tools/iwad_counts.py` (`_reachable`, read for this note).** The rules, the
neighbour definition, the inverting test and Δ match. Differences:

1. The tool applies each W1 rule (109, 36, 38) at most once per round for at most as many
   rounds as there are W1 lines, stopping early when nothing changes; if it is still
   changing after that cap it stops without a fixpoint. The proposed Rust admission
   treats W1 rules like the repeatable ones (applied every round to a global fixpoint),
   which is a sound over-approximation and never narrower. For E1M1 the measured result
   (reachable [−136, 264], the static range) is expected to be unchanged; the Rust
   admission will print its own result and Phase 2 records it.
2. The tool bounds divergence at 2¹⁶ and reports it; the Rust admission stops at ±512.
3. Δ for non-shootable corpses is not Δ (they are covered by (e), §4.1); the tool's Δ is
   correct for (c), which is what it is used for.
4. The tool counts the 2 × 2 `spechit` window over windows inside the grid (clipped when
   the grid is one cell wide), on static specials, over every two-sided line with a
   nonzero special whatever its flags — exactly the §6 rule.

## 6. The `spechit` load rule and the `SpecialCross` witness

**Rule 6.** At load, for every 2 × 2 window of BLOCKMAP cells inside the grid (clipped
when the grid is one cell wide or high), the number of distinct lines listed in the
window that have a back side and a nonzero special is at most 8.

**Theorem.** Under rule 6 no `P_CheckPosition` of the admitted domain writes a ninth
`spechit` entry. *Proof.* Entries of one call are distinct lines (`validcount`), each
two-sided with a nonzero special at that time (p_map.c:211, 240), listed in a cell of
`[xl, xh] × [yl, yh]` computed from `tmbbox` without `MAXRADIUS` (431–434). The callers
are `P_TryMove` and `P_ThingHeightClip`, whose things have radius at most 30 (demon;
gibs 0); with `2r < 128` units, `xh − xl ≤ 1` and `yh − yl ≤ 1`, so the cells lie in one
2 × 2 block; cells outside the grid list nothing (p_maputl.c:481–487), and the in-grid
part of any 2 × 2 block is contained in an in-grid window (or is the clipped window). No
admitted body ever sets a line special (W1 and S1 only clear), so the static count bounds
every runtime count. Hence `numspechit ≤ 8`. ∎ (It needs every admitted radius below 64;
roster scale must re-check this if it admits larger radii.)

**`MapScaleError::SpecialCross`** refuses the tic, with the complete state preserved, at
any attempt to append a ninth entry, in any of the three fill sites (player, monster and
fireball moves; clips). It is unreachable on admitted maps and is exercised only by a
`cfg(test)` fixture that bypasses rule 6 (the 9-line lane, §11). The ninth write is
undefined in C (demo ports emulate the DOS memory layout); like the corner jump and the
128-intercept refusal it is recorded as an **M4 fidelity gap**: maps beyond rule 6 are
rejected, never emulated. E1M1's maximum is exactly 8.

## 7. Mover load rules (refining M3h7 §4.4)

All computed once at load from the static map in exact integers; each failure is a
field-less error (`MapScaleError::Movers` or one variant per rule), state untouched.

1. **Types and sides.** Every line special is 0, 48 (one-sided, or two-sided if N1), or
   one of 1, 63, 109, 88, 36, 38, 11. Every line with special 1 has a back side (no
   `sides[-1]`, §2.4).
2. **Disjoint mover families.** No sector is in two of {door: back sectors of 1 and
   targets of 63/109; plat: targets of 88; floor: targets of 36/38}, so the
   `specialdata` cast of `EV_VerticalDoor` always reads a door. Door kinds 1, 63 and 109
   may share a sector (all `vldoor_t`; decision N5).
3. **Neighbours.** Every mover sector has at least one `getNextSector` neighbour (no
   `MAXINT − 4` top, no −500 turbo destination).
4. **Reachable heights.** The §5 fixpoint stays within ±512; (4b) no door's smallest
   possible top is below its floor; (4c) no door sector has a self-referencing two-sided
   line (it would be its own neighbour and its closed top would be `floor − 4`).
5. **Counts.** Plat sectors ≤ 30 (`MAXPLATS`); reusable switch lines (63) ≤ 16
   (`MAXBUTTONS`; one slot per line, §2.8); scrollers ≤ 64 (`MAXLINEANIMS`, Stage A).
6. **`spechit` window** (§6).
7. **No crushers** (none in the admitted set; every crusher type is outside rule 1).
8. **No sky movers** (decision N4): no mover sector has an `F_SKY1` ceiling.

Tag 0 is faithful: a tag-0 63, 109, 88, 36 or 38 acts on every untagged sector, in index
order (`P_FindSectorFromLineTag`); rules 2–5 and the fixpoint are evaluated with that
target set. One-sided DR lines (rule 1), inverting doors and self-referencing door lines
(rule 4), shared families (rule 2) and doors without neighbours (rule 3) are rejected, not
emulated, as approved.

## 8. Budgets

Let `L` be the light sectors (Stage A), `M` the mover sectors (one `specialdata` each;
E1M1 9), `P ≤ M` the plat sectors, `R` the reusable switch lines, `N` the linked-thing
bound (`1 + monsters + pickups + drops`, 33 today), `B(s)` the blockbox cell count of
sector `s`, and `Q` the collision work of one position check (M3h6 §4.1 row 15).

- **Sector thinker slots**: `L + 2M` (a removed mover keeps its slot until the next pass
  reaches it while a replacement can exist, §2.1; never three, since the old entry
  precedes the new one and is freed first). The 320 actor slots are untouched (M3h7
  decision 4).
- **Visits per pass**: `320 + L + 2M`.
- **`activeplats`**: at most `P ≤ 30` live entries; **buttons**: not simulated, at most `R
  ≤ 16` would be live.
- **`P_ChangeSector`**: at most two steps per sector per tic, two calls per step:
  `≤ 4M` calls; work per tic `≤ Σ_{s ∈ movers} 4 · (B(s) + N · Q)`, computed at load and
  recorded with the M3h6 work values. With roster scale, `N` grows and the bound is
  re-evaluated, not re-proved.
- **Use**: at most one trace per tic (the latch), ≤ 128 intercepts (B7); a use trace
  fills no `spechit`.
- **`spechit`**: ≤ 8 per position check (§6).
- **Sector staging writes per tic**: distinct sectors with a height written `≤ M`, with a
  light written `≤ L`, with a special cleared `≤` the secret sectors; individual height
  writes `≤ 4 · 2M` (two steps, set plus restore, two calls); line-special clears `≤` the
  W1 and S1 lines (E1M1: 36, 38, 109, 11).

## 9. Sector staging

Approved: a **sparse copy-on-write overlay** of the sectors written in the tic. The plan
keeps a small vector of `(sector, SectorState)` written entries (and of line-special
clears), read through by every collision, sight, noise and clip query of later thinkers
in the same pass, and committed into `Simulation::sectors` with the rest of the plan; a
refused tic drops it, leaving the table byte-identical. Older policies stage nothing and
allocate nothing (as the M3h7 spare buffers). The alternative is a full copy of the
sector table per tic into a reused spare buffer, as M3h7 does for `SectorSpecial`.

The measurement (Phase 2, the measurement step of §12), alone on an idle host: matched
release timings of `Simulation::advance` for the M3h7 workloads (must not move) and for
`doorway_patrol`, `lift_cycle` and a synthetic E1M1-sized table (88 sectors) and a
format-sized one (4,096 sectors) with the overlay and with the full copy; the overlay is
kept unless the full copy is cheaper on the E1M1-sized table, and the result is recorded
either way. A lookup cost of the overlay (a linear scan of at most `M` entries per sector
read) is part of what is measured.

## 10. DSIM35 / wire 19

DSIM35 is DSIM34 plus, only where source state demands:

- `sector.{i}.floor`/`ceiling`: now moving (fields exist since v3).
- `line.{i}.special` for every line whose map special is nonzero: the runtime value (W1
  and S1 clears); lines with a zero map special stay zero and are not encoded.
- Mover records in `thinker.shared_order` (new kinds `door`, `plat`, `floor`), each naming
  its sector: door `type` (`normal`, `blazeOpen`), `direction`, `topheight`,
  `topcountdown` (encoded 0 until first set; the C harness zero-fills allocations as a
  declared adapter, decision N6); plat `status`, `count`, `low`, `high` (type
  `downWaitUpStay`, speed 4, wait 105, `crush` and `tag` are implied); floor `type`
  (`lowerFloorToLowest`, `turboLower`) and `floordestheight` (speed and direction
  implied). `sector->specialdata` is derived (each record names its sector, at most one
  live record per sector) and the decoder checks it; the `activeplats` slot is not
  encoded (unobservable, §2.5).
- `original_player.0.use_down`.
- `level.exit` (bool) and the reborn request (decision N3).
- Events (one new tag): use (line, side, result), `sfx_noway` stop, crossing (line,
  side, actor), mover start, step result (`ok`/`crushed`/`pastdest`) and stop,
  `P_ChangeSector` outcomes (gibs, dropped-item removal, `nofit`), sector sounds as sound
  requests with a sector origin, the exit and the reborn request. Switch textures,
  button timers and scroll offsets stay out (M3h7 decision 5).

Every earlier reader rejects 35; `decode_specials` stays 34-only; a new
`decode_movers` reads 35 only and checks the encoded map: one live mover per mover
sector at most, record kinds matching the sector's family, heights within the
fixpoint's intervals, cleared line specials only on W1/S1 lines, `use_down` and the exit
flags.

## 11. Evidence plan

**C harness.** `tools/reference/m3h8_geometry.py` extends the M3h7 builder (heights,
tags, line flags, specials on two-sided lines); `m3h8_cases.py` and `m3h8_reference.py`
compile genuine bodies: p_doors.c (`EV_DoDoor`, `EV_VerticalDoor`, `T_VerticalDoor`),
p_plats.c (`EV_DoPlat`, `T_PlatRaise`, `P_AddActivePlat`, `P_RemoveActivePlat`),
p_floor.c (`T_MovePlane`, `EV_DoFloor`, `T_MoveFloor`), p_switch.c (`P_UseSpecialLine`,
`P_ChangeSwitchTexture`, `P_StartButton`), p_spec.c (`P_CrossSpecialLine`,
`P_ShootSpecialLine`, the `P_Find*` helpers, `getNextSector`, `P_UpdateSpecials`),
p_map.c (`P_UseLines`, `PTR_UseTraverse`, `P_ChangeSector`, `PIT_ChangeSector`,
`P_ThingHeightClip`), p_enemy.c (`P_Move`, `P_TryWalk`, `P_NewChaseDir`), p_user.c
(`P_PlayerThink` use, `P_DeathThink`), reusing every M3h7 body. Declared adapters:
`S_StartSound` recorded with its origin (mobj, sector `soundorg`, button slot);
`G_ExitLevel` and `G_DoReborn` recorded; `R_TextureNumForName` and `switchlist` from a
declared synthetic table; allocations zero-filled; `spechit[8]` keeps its source size so
the sanitizer build would report an overflow. New observers: every sector's floor and
ceiling each tic, every line's special, mover records in thinker order, `numspechit`
after every `P_CheckPosition`, every `P_ChangeSector` with each visited thing's clip
outcome, use and crossing calls.

| Room | Purpose |
| --- | --- |
| `doorway` | two rooms joined by DR doors; a monster behind each; a former human placed to die in the doorway (dropped clip) and a health bonus inside the door sector; a monster placed with its box overlapping the closed door sector (case (d)) |
| `switchyard` | SR 63 switch and its tagged door; the S1 exit switch; a non-special wall (`sfx_noway`); a two-sided scroller in front of a switch; back-side use |
| `lift` | WR 88 lift with a ceiling too low for a monster at the top; monster and player riders; a monster crossing the lift line |
| `floors` | W1 36 beside a higher neighbour (jump-up) and beside lower ones; W1 38; W1 109; things standing and items straddling steps near them |
| `dense` | a lane with 8 two-sided special lines (scrollers and doors) in one 2 × 2 window (admitted); Rust-only 9 |

Scenarios (about 32): `door_use`, `door_wait_close`, `door_reuse_closes`,
`door_reverse_on_player`, `door_reverse_on_monster`, `door_monster_bump`,
`door_monster_back_side`, `door_monster_cannot_close`, `door_monster_reopens_closing`,
`door_gibs_corpse`, `door_removes_drop`, `door_traps_bonus`, `door_stuck_shootable`,
`door_blocks_sight_and_sound`, `switch_sr_door`, `switch_sr_while_active`, `switch_exit`,
`exit_then_advance_refused`, `use_noway`, `use_back_side`, `use_scroll_wall_stops_trace`,
`use_held_latch`, `dead_use_reborn`, `lift_player`, `lift_monster_cross`,
`lift_reverse_at_top`, `lift_last_step_short`, `floor_turbo_jump_up`,
`floor_lower_to_lowest`, `blaze_open`, `item_rises_onto_step`, `player_picks_up_in_clip`,
`scroll_spechit_nodir` (a monster's failed move against a two-sided scroller),
`dense_spechit_8`. Strict and ASan/UBSan builds compared as before. Sky over movers is
not exercised: E1M1 has none and rule 8 rejects it.

**Independent DSIM35 encoder.** `tools/reference/movers_snapshot.py` extends
`specials_snapshot.py` without editing it; `m3h8_snapshot_goldens.py` writes
`tests/reference/m3h8_snapshots.csv`, `m3h8_events.dref`, full-byte `m3h8_state_*.bin`
fixtures and `m3h8_snapshot_contract.json` from the C rows alone, before the Rust change.

**Rust-only tests.** Rejection with unchanged state for every §7 rule (an unadmitted
special of each family, a one-sided DR line, a shared door/floor sector, a door without
neighbours, an inverting door, a self-referencing door line, a reachable height of 513,
31 plat sectors, 17 reusable switches, 65 scrollers, 9 special lines in a window, a sky
mover sector); the two-sided scroller still rejected under Stage A; the `SpecialCross`
witness on the 9-line lane, byte-identical before and after; a refused tic in the middle
of a door movement leaving sectors, lines, thinkers and RNG byte-identical; V′ and the
relaxed item witness after every tic of 1,200-tic soaks of `doorway`, `lift` and `floors`
with four seeds; the exit and reborn refusals; `step` equal to `advance`; DSIM35 reader
version and policy rejections; the slot and visit budget `320 + L + 2M`.

**Fuzz.** Target `player_movers` (Stage B rooms, commands including `BT_USE`) with the
M3h7 properties plus V′, the item witness, the transaction and the witness refusals;
seeds from `tools/seed_player_movers_fuzz.py`. Reported as target compiled, seed replay,
coverage-guided campaign and sanitizer campaign separately, with leak detection stated.

**Measurements** (alone on an idle host): matched release timings of the M3h7 head and
the branch head for every existing `--measure` workload (digests compared; older
policies must not move), `doorway_patrol`, `lift_cycle`, the admission time with the
fixpoint and the window rules, the recorded per-tic `P_ChangeSector` work, and the §9
staging comparison.

## 12. Implementation plan (one commit per step; evidence before the runtime change)

Every older policy keeps exact admission, behaviour, digests and DSIM bytes at every
step.

1. **This note.**
2. **Owner decisions** recorded in §14 (addendum commit).
3. **C evidence**: `m3h8_geometry.py`, `m3h8_cases.py`, `m3h8_reference.py`; generated
   `tests/reference/m3h8_trace.dref`, cases, geometry lumps, `m3h8_maps.rs`,
   `m3h8_contract.json`; strict and sanitizer builds compared.
4. **DSIM35 projection**: `movers_snapshot.py`, `m3h8_snapshot_goldens.py`, outputs.
5. **Staging refactor, no behaviour change**: the sparse sector and line-special overlay
   in `DemonPlan`, read-through accessors replacing `&self.sectors` in the plan
   (`demon_expanded_map.rs:529, 554, 614, 686`), the per-sector `blockbox`; the §9
   measurement recorded here.
6. **Policy and admission**: `PlayerMonstersMoversV1` (wire 19), `has_movers`, context,
   constructor, `validate_movers` (§7, §5 fixpoint, rule 6), the error variants, the
   sector slot capacity `L + 2M`.
7. **Planes**: `T_MovePlane`, `P_ChangeSector`, `PIT_ChangeSector`, `P_ThingHeightClip`
   (box-wide clip with the centre-only fallback on a solid overlap, pickups inside the
   clip, gibs, dropped-item removal).
8. **Movers**: `SectorThinker::{Door, Plat, Floor}` in the shared order, `EV_DoDoor`,
   `EV_VerticalDoor`, `EV_DoPlat`, `EV_DoFloor`, `T_*`, `activeplats` count, sector
   sounds.
9. **Crossing and `spechit`**: the special subsequence of `openings` in every fill site,
   the reverse crossing loop in `Mover::attempt`, `pursuit::move_actor` and
   `projectiles::try_move`, `P_CrossSpecialLine`, the `SpecialCross` witness.
10. **Use and switches**: `BT_USE`, `use_down` (starting true), `P_UseLines` over
    `movement::path`, `PTR_UseTraverse`, `P_UseSpecialLine`, switches, the exit,
    `BT_USE` while dead.
11. **Monster door use**: the `P_Move` failure branch (`DI_NODIR`, reverse
    `P_UseSpecialLine` loop) in `pursuit::move_actor`.
12. **V′ and the relaxed item witness** under movers only.
13. **DSIM35**: `snapshot.rs`, `diagnostic.rs` (`decode_movers`), STATE_FORMAT; replay of
    every scenario against the C rows and the independent captures.
14. **Tests, example, tools, fuzz, CI** on the M3h7 pattern (`player_movers_tests.rs`,
    example `player_movers`, `check_player_movers.py`, `measure_movers.py --check`, fuzz
    target and seeds).
15. **Docs**: README, ROADMAP, COMPATIBILITY, STATE_FORMAT, ARCHITECTURE, TESTING,
    docs/README, `tools/reference/README.md`, the evidence record.
16. **Timings** (§11), alone on the idle host.
17. **Fuzz campaigns** (coverage-guided and sanitizer).
18. **Independent review** and fixes.
19. **Gates**, draft PR, ready only after every gate passes.

## 13. E1M1 readiness under the movers policy

From `61d6087` and `iwad_counts.py --readiness E1M1` (run locally for this note): E1M1
passes the M3h6 geometry rules, the Stage A sector-special rule, sky, the heights rule
(reachable [−136, 264]), and Stage B rules 1–6 and 8 (one-sided DR lines 0, shared
families 0, doors without neighbours 0, self-referencing door lines 0, inverting doors 0,
1 plat sector, 1 reusable switch, 8 scrollers, window maximum 8, sky movers 0, tag-0
tag-using lines 0, B2 Δ 4). What would still block loading E1M1:

1. **The two-sided scroller rule**: all 8 scrollers are two-sided; blocked unless
   decision N1 admits them under this policy.
2. **Roster scale**: on UV single player 29 monsters (shotgun guy 16, zombieman 9, imp
   4; all admitted types) against 8, and 53 pickups (all admitted kinds) against 16; the
   linked-thing bound 33 and every roster-formula budget of §8 follow.
3. **Unadmitted THING types** (33 on UV): floor lamp ×8, pool of blood and flesh ×7,
   barrel ×6, dead player ×4, bloody mess ×2 + ×2, tech pillar ×2, candelabra ×2
   (`UnsupportedType`).
4. **Not measured**: whether every E1M1 THING satisfies the M3h5 spawn rule (centre in Ω,
   box meeting no one-sided wall); whether its door kinds share a sector (admitted
   either way by decision N5); whether its one `ML_SECRET` line is a DR line (faithful
   either way); whether any spawned monster starts in case (d) (admitted either way by
   decision N2).

The exit leads nowhere (next map out of scope), and the player starts are records (M3h6).

## 14. Owner decisions

**Approved before this note (recorded verbatim):**

- Policy PlayerMonstersMoversV1, wire 19, DSIM35, context PLAYER-MONSTERS-MOVERS-1-MEDIUM-v1, a superset of PlayerMonstersSpecialsV1.
- Breadth: exactly E1M1's seven tag/use/cross line types (confirmed by the IWAD measurement); siblings later.
- spechit: the 2×2 window load rule plus the SpecialCross runtime witness (refuse the tic, state preserved; record as an M4 fidelity gap like the corner jump and intercepts).
- V′ vertical invariant and the relaxed item witness under the movers policy only; older policies keep theirs.
- Mover load rules: reject inverting doors, self-referencing door lines, shared mover families and doors without two-sided neighbors, rather than emulating them.
- Exit: the tic completes, level.exit is set, the next advance returns a checked error preserving state; intermission and next map out of scope.
- Tag 0: faithful.
- Sector staging: sparse copy-on-write overlay of sectors written in the tic, unless the step 5.4 measurement says a full copy is cheaper; record the measurement either way.
- Crushers: deferred (reject at load).

**New decisions.** Each assessment answers: (a) consistent with the approvals above?
(b) avoids admitting behaviour that silently diverges from vanilla? (c) adds a new kind
of fidelity gap beyond refuse-the-tic witnesses? (d) needs roster scale first?

**N1. Two-sided scrolling walls (48).**
Options: (i) admit 48 on two-sided lines (blocking or not) under the movers policy only,
faithfully — its only game-code effects are the `spechit` entry, the `P_Move` `DI_NODIR`
reset and the use-trace stop, all Stage B bodies — and counted by rule 6; Stage A keeps
its one-sided rule; (ii) admit only non-blocking two-sided 48 (E1M1's shape); (iii) keep
rejecting, and E1M1 stays unloadable under every policy. **Recommendation: (i).**
Assessment: (a) not covered by the breadth approval, which names the seven
tag/use/cross types and "siblings later"; 48 is not a sibling but a type already admitted
by Stage A, so this widens its geometry, not the type list — it needs explicit approval
and changes no approved item; (b) yes, provided Phase 2 evidences the three effects
(scenarios `scroll_spechit_nodir`, `use_scroll_wall_stops_trace`, `dense_spechit_8`);
nothing is approximated; (c) no; (d) no. (ii) is equally faithful but narrower with no
benefit (the bodies already handle the flags); (iii) is the only option under which E1M1
can never load.

**N2. Content of V′: admit the stuck shootable (d) and the corpse envelope (e).**
Options: (i) admit (d) and (e) explicitly (§4.1), faithful; (ii) prove (d) unreachable by
new spawn load rules (every shootable map THING fits box-wide at every reachable height of
the sectors its box touches, no solid overlap, no box crossing a blocking line) and
bound corpses tighter; (iii) a runtime witness refusing any tic that would produce (d).
**Recommendation: (i).** Assessment: (a) yes, V′ under movers only is approved and its
content was left to this note; (b) yes, (d) and (e) are what the source does; (ii) would
reject legal maps (E1M1 not measured for it) and (iii) would refuse legal, defined vanilla
play; (c) no for (i); (iii) would add a refusal of defined behaviour, a new kind; (d) no.

**N3. `BT_USE` while dead (`P_DeathThink` → `PST_REBORN` → level restart).**
Options: (i) mirror the exit: the tic completes unchanged, the reborn request is set in
the state, the next `advance` returns a checked error preserving state; (ii) refuse the
command before the tic (checked error, state unchanged); (iii) implement the level
reload. **Recommendation: (i).** Assessment: (a) yes, it applies the approved exit
treatment to the other `gameaction` the admitted inputs can request; (b) yes, the tic
itself is exact and the restart is refused, not approximated; (c) no new kind: it is the
exit's "next advance refused"; (ii) would refuse a legal input in its own tic, (iii) is
out of scope; (d) no.

**N4. Sky ceilings on mover sectors.**
Options: (i) reject at load (rule 8); (ii) admit (the M3h7 sky bodies read heights at
query time and would compose). **Recommendation: (i).** Assessment: (a) yes (the brief
asks for sky over movers only if E1M1 has any; it has none); (b) yes, a rejection; (c)
no, a load rule; (d) no. (ii) is plausible but unevidenced; 190 sky mover sectors exist
on other IWAD maps, for a later milestone.

**N5. Door kinds sharing a sector (1, 63, 109 on one `vldoor_t` sector).**
Options: (i) admit faithfully (M3h7 rule 2 allows it), with a C scenario for the
`blazeOpen` door reversed by a DR use, which stays as a thinker at the floor (§2.4); (ii)
reject at load a sector targeted by two door kinds. **Recommendation: (i).** Assessment:
(a) yes, the approved rule rejects shared *families* only; (b) yes if the scenario
evidences the reversed `blazeOpen` path, otherwise it would be admitted unevidenced —
Phase 2 must add it or fall back to (ii); (c) no; (d) no. Whether E1M1 has such a sector
is not measured.

**N6. Uninitialised mover fields in DSIM35.**
Options: (i) the C harness zero-fills mover allocations (declared adapter), Rust starts
the same fields at 0, DSIM35 encodes only `topcountdown` (read once the door waits) and
omits `newspecial`, `texture` and `oldstatus` (never read for the admitted types); (ii)
encode every source field. **Recommendation: (i).** Assessment: (a) yes ("persistent
fields only where source state demands"); (b) yes, no admitted path reads an
uninitialised field before writing it (§2.4–§2.6); (c) no; (d) no.

**N7. Fixpoint semantics for W1 lines.**
Options: (i) the Rust admission applies W1 rules like repeatable ones to a global
fixpoint (sound, possibly wider than the tool); (ii) the tool's capped rounds.
**Recommendation: (i)**, and the IWAD tool gains the same change only with owner
approval. Assessment: (a) yes (the approved rule rejects maps whose reachable heights
leave ±512; a wider sound bound can only reject more); (b) yes; (c) no; (d) no.

**N8. Sector-thinker slot capacity `L + 2M`.**
Options: (i) `L + 2M` slots and `320 + L + 2M` visits (§8); (ii) `L + M` with a witness
refusing the tic that would need a second entry for one sector. **Recommendation: (i).**
Assessment: (a) yes, it is the approved separate slot class bounded at load (M3h7
decision 4); (b) yes; (c) no, while (ii) would refuse defined play (a new kind); (d) no.

**Provisionally adopted overnight, pending owner confirmation (2026-11-03).** The
milestone ran unattended under the owner's rule: continue into Phase 2 only with
recommendations consistent with the approvals that admit no silent divergence. Each item
below can be revised in review.

- **N1: option (iii), keep rejecting two-sided 48 under every policy, including
  movers.** This is *not* the note's recommendation (i). Option (i) widens the admitted
  geometry beyond the approved breadth, which assessment (a) says needs explicit approval,
  so it is left to the owner. Rejecting costs E1M1 nothing today: roster scale and the 33
  unadmitted THINGs (§13) block it regardless. Admitting it later is a load-rule
  relaxation plus the three scenarios named under N1.
- **N2: (i)**, V′ admits the stuck shootable (d) and the corpse envelope (e).
- **N3: (i)**, `BT_USE` while dead completes its tic, records the reborn request, and the
  next `advance` is refused with a checked error that preserves state.
- **N4: (i)**, sky ceilings on mover sectors are rejected at load.
- **N5: (i)**, shared door kinds are admitted faithfully if the C harness evidences the
  reversed `blazeOpen` path. Otherwise (ii): they are rejected at load, and the evidence
  record says which.
- **N6: (i)**, zero-filled mover allocations as a declared adapter; DSIM35 encodes
  `topcountdown` and omits `newspecial`, `texture` and `oldstatus`.
- **N7: (i)**, the Rust admission runs W1 rules to a global fixpoint. `tools/iwad_counts.py`
  is unchanged.
- **N8: (i)**, `L + 2M` sector-thinker slots and `320 + L + 2M` visits.

## 15. Follow-ups, risks, status

- **Roster scale** is the largest remaining E1M1 blocker; every §8 budget is a formula of
  `N`, and §6 needs every admitted radius below 64.
- **Decorations and barrels** (eight types on E1M1) remain a separate milestone.
- **Deferred types**: crushers and ceilings (and the crush-blood evaluation order of M3h7
  §2.10), locked doors and keys, teleports, stairs, donuts, `raiseToTexture`, perpetual
  and stopping plats, light-switching lines, gun lines, the other exits, sky movers,
  sector specials 10, 11, 14, the intermission and the next map.
- **Undefined source behaviour refused rather than emulated** (M4 questions): the ninth
  `spechit` write, `sides[-1]` of a one-sided DR line, the door cast of a non-door
  `specialdata`, the 31st plat and 17th button, inverting doors.
- **Spawn-origin states** (§4.1 (d), §2.7 consequences 2–4) are faithful but rare; the
  `doorway` room must place a monster in (d) or the path goes unobserved.
- **Rust `openings` on a line failure** must keep the entries before the blocker
  (§3); otherwise a monster's door use after a partial pass would differ.
- **Performance**: `P_ChangeSector` runs a full position check per linked thing in the
  blockbox per step; small at the current roster, multiplied by roster scale; measured
  in Phase 2.
- **IWAD tool**: the fixpoint round cap (§5, N7), the door-kind sharing count, the spawn
  rule count and a case-(d) count are not measured; adding them needs owner approval.
- **CI headroom**: two generators and one fuzz target more; estimated when the harness
  exists.

**Status / next step.** Phase 1 is complete with this note. Step 2 of §12 is done: N1–N8
are provisionally adopted (end of §14, N1 as option (iii)). Step 3 is done: the C
evidence of §11 (`tools/reference/m3h8_{geometry,cases,reference}.py`, outputs
`tests/reference/m3h8_*`, [commands](TESTING.md#original-doors-lifts-floors-switches-and-the-exit-c-evidence)),
30 scenarios over the `doorway`, `hall`, `switchyard`, `lift` (ceilings 192, 118, 112),
`floors` and `dense` rooms, with the strict and both sanitizer traces byte-identical.
Differences from the §11 list: under N1 (iii) no two-sided scroller is built, so
`scroll_spechit_nodir`, `use_scroll_wall_stops_trace` and the scrollers of `dense` are
replaced by the same bodies on admitted lines (a monster's failed move with a WR 88 line
in `spechit`, a use stopped by a W1 36 line, eight WR 88 lines); several listed scenarios
are observed inside a combined one (the contract's per-case inventory names them);
`exit_then_advance_refused` is Rust-only; the reversed `blazeOpen` door of N5 is
evidenced (`blaze_reversed`); V′ case (e) is not constructed. Step 4 is done: the
independent DSIM35 projection (`tools/reference/movers_snapshot.py`,
`m3h8_snapshot_goldens.py`; outputs `tests/reference/m3h8_snapshots.csv`,
`m3h8_events.dref`, 13 `m3h8_state_*.bin` fixtures, `m3h8_snapshot_contract.json`) from
the C rows alone, with the byte layout recorded as a
[draft DSIM v35 section](STATE_FORMAT.md#dsim-v35--original-doors-lifts-floors-switches-and-the-exit-draft)
for the later Rust writer. Choices made there within §10: the reborn request is the
source `playerstate` (`PST_REBORN`, value 2) in the existing `original_player.0.state`
field rather than a second flag; a removed mover not yet unlinked is a store record of
kind 8 naming its sector; movers take the lowest free sector-thinker slot and a slot's
generation grows when its mover is unlinked; a case that requested an exit or a reborn
ends at that tic. Next is step 5, the staging refactor.
