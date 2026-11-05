# M3h9c reference investigation: real E1M1 against the genuine engine, tic for tic

Phase 1 note for the third part of the E1M1 split
([M3h9 note §10](M3H9_REFERENCE_INVESTIGATION.md#10-real-e1m1-evidence-without-committing-the-iwad-design-for-m3h9c)
and [M3h9b note §12](M3H9B_REFERENCE_INVESTIGATION.md#12-follow-ups-risks-status)): the
milestone that runs the real E1M1 map on medium skill through the genuine linuxdoom-1.10
engine and through the Rust simulation from the same input, and compares their state
after every tic. It records the original C facts (function, file and line) for the demo
playback path, level setup, the ticker and the player model; the headless genuine build
with its stub inventory and the 64-bit (LP64) defects of the genuine loader; the Rust
loading path, command domain and capture surface; the harness design; the new owner
decisions G1–G13 with recommendations; and the evidence and implementation plans. It
follows the conventions of the [M3h9b note](M3H9B_REFERENCE_INVESTIGATION.md).

Source authority is `linuxdoom-1.10/` at the repository's upstream commit; line numbers
are of those files, and `crates/` line numbers are of master at the start of this
milestone. Milestone status lives in [ROADMAP](ROADMAP.md), support in
[COMPATIBILITY](COMPATIBILITY.md), schemas in [STATE_FORMAT](STATE_FORMAT.md), commands in
[TESTING](TESTING.md), the asset rules in [ASSETS](ASSETS.md) and the census tables in
[validation/M3H9_IWAD_COUNTS.md](validation/M3H9_IWAD_COUNTS.md) (see the
[documentation authority map](README.md#documentation-authority)). The observations of
this milestone will go to `docs/validation/M3H9C.md`, created in Phase 2. This note owns
the source facts, the design, the decisions and the plan; it never contains an
IWAD-derived coordinate, angle, lump byte or per-object datum.

## 1. Starting point

Master contains M3h9b (`PlayerMonstersBarrelsV1`, wire 21, DSIM37, context
`PLAYER-MONSTERS-BARRELS-1-MEDIUM-v1`, PR #55); the owner confirmed F1–F11 on
2026-11-05. The M3h9b note concluded that E1M1 on medium passes load admission under wire
21 with nothing measured blocking it
([§10](M3H9B_REFERENCE_INVESTIGATION.md#10-e1m1-medium-load-admission-under-the-new-policy))
and left the confirmation to this milestone. The brief's goal: run the real E1M1 through
the genuine engine (compiled headless from the original source, with only I/O replaced)
and through Rust, from one input file, compare the complete gameplay state after every
tic, explain and fix every divergence inside the admitted domain, and commit only
results (pass/fail, counts, digests), never map data.

**What the Phase 1 probe found** (a scratch crate outside the repository, path
dependencies on `crates/*`; only counts, booleans, variant names and digests were
written). Real E1M1, **unstripped**, loads and runs under wire 21:

| Fact | Observed |
| --- | --- |
| THING records | 143 (91 spawned on medium, 43 skill-filtered, 4 player starts, 5 deathmatch starts) |
| Actors after setup | 92 (91 THINGs and the player body) |
| `rng.gameplay` after setup | 144 (`rng.miscellaneous` 0), matching the M3h9b derivation (§2.5) |
| Idle | 2,100 and 35,000 tics, every tic admitted, reproducible over three runs |
| Pseudo-random admitted commands | 16 streams × 2,100 and 16 × 35,000 tics, no refusal; the player died in 9 of the long runs, no exit |
| Capture | 4,139 fields, about 203 KB (DSIM37; about 63.5 KB of it the static `map.*` group) |
| Per-tic cost (rough) | about 45 µs `advance`, about 0.54 ms snapshot plus canonical bytes, about 0.74 ms with the digest |

The timings are rough: the machine was not idle during the probe, so they are not a
performance record (G12 measures properly). The stripping of specials and sky ceilings in
the existing `private_iwad_real_map_counts` test (crates/doom-sim/src/player_real_map_tests.rs:653)
exists only because that test admits through the wire-17 policy; wire 21 needs none.

Two structural facts make wire 21 alone insufficient for a vanilla match, whatever the
input:

1. **The player body's place and angle.** The genuine `P_LoadThings` spawns things in
   lump order (p_setup.c:313–347); a player start reaches `P_SpawnMapThing`'s type-≤4
   branch, `playerstarts[mthing->type-1] = *mthing; if (!deathmatch) P_SpawnPlayer
   (mthing); return;` (p_mobj.c:729–737), so the body is spawned **at the start record's
   position in the lump**, between the THINGs before and after it, and it takes the
   start's angle: `mobj->angle = ANG45 * (mthing->angle/45);` (p_mobj.c:671). Rust's
   `spawn_map_things` returns starts as records (crates/doom-sim/src/spawning.rs:436–444)
   and every harness then calls `spawn_player_body` after all THINGs
   (tests/support/m3h9b.rs:149–166), whose body has "fixed fresh health100 and zero
   orientation" (crates/doom-sim/src/original_player.rs:297–317). The thinker order, the
   assignment of each `lastlook` and spawn-tics draw to an actor (p_mobj.c:506, 787–788)
   and the player's angle therefore differ from tic 0, although the draw count (144)
   agrees.
2. **The player's counters and death.** `P_PlayerThink` decrements both palette
   counters every live tic, `if (player->damagecount) player->damagecount--;`
   (p_user.c:355–356) and `if (player->bonuscount) player->bonuscount--;` (358–359), and
   a dead player runs `P_DeathThink` instead (258–262; body 182–229). In Rust
   `damage_count` is only ever raised (crates/doom-sim/src/combat.rs:929) and never
   decays; `bonus_count` does decay, live only, at the end of the player phase
   (crates/doom-sim/src/world_pickups.rs:412–426, called at
   crates/doom-sim/src/demon_expanded_map.rs:799). `P_DeathThink`'s turn toward the
   attacker and its `damagecount` decrement are absent: a dead player's angle stays
   fixed. The first tic after any damage to the player therefore diverges.

## 2. Original C facts

Every citation is `function (file:lines)`; "draw" means one `P_Random ()` call
(m_random.c:57–61). `M_Random` (m_random.c:63–67) is a separate index `rndindex` used by
sound pitch (s_sound.c:330, 340) and the status bar (st_stuff.c:992); no `p_*.c` or
g_game.c code calls it.

### 2.1 The `-playdemo` path and the demo format

- **Command line.** `D_DoomMain` finds `-playdemo name` (or `-timedemo`), builds
  `"%s.lmp"` and calls `D_AddFile (file)` (d_main.c:948–958) before
  `W_InitMultipleFiles (wadfiles)` (1021). The initialisation order is `Z_Init` (1018),
  `W_InitMultipleFiles` (1021), `R_Init` (1095), `P_Init` (1098), `D_CheckNetGame`
  (1104); then `singledemo = true; G_DeferedPlayDemo (myargv[p+1]); D_DoomLoop ();`
  (1135–1141). The game mode is chosen by file name in `IdentifyVersion` (called at
  803): `doomu.wad` → `retail` (690–694), `doom.wad` → `registered` (697–701).
- **The demo lump.** `W_AddFile` (w_wad.c:141–226) treats a file whose name does not end
  in `wad` as one lump: `singleinfo.filepos = 0; singleinfo.size =
  LONG(filelength(handle)); ExtractFileBase (filename, singleinfo.name);` (172–180).
  `ExtractFileBase` (w_wad.c:86–114) upper-cases the base name and stops with `I_Error`
  past 8 characters. The lump name must be unique, since the last match wins.
- **`G_DeferedPlayDemo`** (g_game.c:1576–1580): `defdemoname = name; gameaction =
  ga_playdemo;`.
- **`G_DoPlayDemo`** (g_game.c:1582–1620), in order: `gameaction = ga_nothing` (1587);
  `demobuffer = demo_p = W_CacheLumpName (defdemoname, PU_STATIC)` (1588);
  `if ( *demo_p++ != VERSION)` prints "Demo is from a different game version!" and
  returns with no level loaded (1589–1594); then one byte each for `skill` (1596),
  `episode` (1597), `map` (1598), `deathmatch` (1599), `respawnparm` (1600), `fastparm`
  (1601), `nomonsters` (1602), `consoleplayer` (1603) and `playeringame[0..3]`
  (1605–1606); `netgame = netdemo = true` only if `playeringame[1]` (1607–1611);
  `precache = false` (1614); `G_InitNew (skill, episode, map)` (1615); `precache = true`
  (1616); `usergame = false` (1618); `demoplayback = true` (1619). `demoplayback` is
  therefore false during `G_InitNew`.
- **Version.** `enum { VERSION =  110 };` (doomdef.h:33). The genuine source accepts
  only version-110 demos; the commercial 1.9 executables wrote 109, so a 1.9 demo is
  rejected at g_game.c:1589 unless its first byte is changed (G1).
- **`G_ReadDemoTiccmd`** (g_game.c:1491–1503), with `#define DEMOMARKER 0x80` (1488):
  ```c
  if (*demo_p == DEMOMARKER) { G_CheckDemoStatus (); return; }
  cmd->forwardmove = ((signed char)*demo_p++);
  cmd->sidemove = ((signed char)*demo_p++);
  cmd->angleturn = ((unsigned char)*demo_p++)<<8;
  cmd->buttons = (unsigned char)*demo_p++;
  ```
  Four bytes per tic per player in game; `consistancy` and `chatchar` keep the
  `netcmds` copy (zero in single player). A forward byte of 0x80 is the marker, so no
  ticcmd may carry `forwardmove` −128.
- **`G_CheckDemoStatus`** (g_game.c:1647–1687) on playback: `if (singledemo) I_Quit ();`
  (1660–1661); otherwise it resets the demo globals and calls `D_AdvanceDemo` (1663–1674),
  unreachable with `singledemo`. The timing-demo branch (1651–1656) needs `timingdemo`,
  which `-playdemo` does not set.
- **Layout** (the same as `G_BeginRecording` writes, g_game.c:1555–1566): byte 0 version
  (110), 1 skill (0–4), 2 episode, 3 map, 4 deathmatch, 5 respawn, 6 fast, 7 nomonsters,
  8 consoleplayer, 9–12 `playeringame[0..3]`: a 13-byte header, then the ticcmds, then
  0x80.

### 2.2 `G_InitNew` and `G_DoLoadLevel`

**`G_InitNew`** (g_game.c:1364–1482) writes, in order: if `paused`, `paused = false` and
`S_ResumeSound ()` (1372–1376); the skill clamp (1379–1380); the episode clamp by game
mode (1386–1403; registered allows 1–3); the map clamp (1407–1412); **`M_ClearRandom
()`** (1414; `rndindex = prndindex = 0`, m_random.c:69–72), its only call site;
`respawnmonsters` (1416–1419); the fast-monster edits only if `fastparm || (skill ==
sk_nightmare && gameskill != sk_nightmare)` (1421–1428) and their inverse (1429–1436),
neither taken on medium without `-fast`; `players[i].playerstate = PST_REBORN` for all
four (1440–1441); `usergame = true` (1443); `paused = false` (1444); `demoplayback =
false` (1445); `automapactive = false` (1446); `viewactive = true` (1447);
`gameepisode`, `gamemap`, `gameskill` (1448–1450); `viewactive = true` (1452);
`skytexture = R_TextureNumForName ("SKY1")` for episode 1 (1455–1479); `G_DoLoadLevel ()`
(1481).

**`G_DoLoadLevel`** (g_game.c:445–497): `skyflatnum = R_FlatNumForName ( SKYFLATNAME )`
(454); the commercial sky branch, not taken (458–468); `levelstarttic = gametic` (470);
`wipegamestate` (472–473); `gamestate = GS_LEVEL` (475); for each player, `PST_DEAD →
PST_REBORN` if in game (479–480) and `memset (players[i].frags,0,...)` (481);
`P_SetupLevel (gameepisode, gamemap, 0, gameskill)` (484); `displayplayer =
consoleplayer` (485); `starttime = I_GetTime ()` (486); `gameaction = ga_nothing` (487);
`Z_CheckHeap ()` (488); then the input-building state: `gamekeydown`, joystick and
mouse (491–496), with `sendpause = sendsave = paused = false` (494). No RNG write.

### 2.3 `P_SetupLevel` and the lump loaders

**`P_SetupLevel`** (p_setup.c:583–693): `totalkills = totalitems = totalsecret =
wminfo.maxfrags = 0` (594); `wminfo.partime = 180` (595); every player's `killcount`,
`secretcount`, `itemcount` = 0 (596–600); `players[consoleplayer].viewz = 1` (604);
`S_Start ()` (607); `Z_FreeTags (PU_LEVEL, PU_PURGELEVEL-1)` (618); `P_InitThinkers ()`
(622; `thinkercap.prev = thinkercap.next = &thinkercap`, p_tick.c:53–56); `W_Reload ()`
(625; returns at once without a reload file, w_wad.c:246–247); the lump name `E1M1`
(635–642) and `W_GetNumForName` (644); **`leveltime = 0`** (646); then
`P_LoadBlockMap` (649), `P_LoadVertexes` (650), `P_LoadSectors` (651), `P_LoadSideDefs`
(652), `P_LoadLineDefs` (654), `P_LoadSubsectors` (655), `P_LoadNodes` (656),
`P_LoadSegs` (657), `rejectmatrix = W_CacheLumpNum (lumpnum+ML_REJECT,PU_LEVEL)` (659),
`P_GroupLines ()` (660), `bodyqueslot = 0` (662), `deathmatch_p = deathmatchstarts`
(663), `P_LoadThings` (664), the deathmatch spawns (667–676, not taken),
`iquehead = iquetail = 0` (679), `P_SpawnSpecials ()` (682) and `if (precache)
R_PrecacheLevel ()` (688–689). On the first load `precache` is false (g_game.c:1614); on
a reload during playback `R_PrecacheLevel` returns at once because `demoplayback` is set
(r_data.c:758–759). No RNG reseed.

The loaders' renderer dependencies: `P_LoadSectors` calls `R_FlatNumForName` for the
floor and ceiling flats (p_setup.c:249–250), `P_LoadSideDefs` calls
`R_TextureNumForName` three times per side (456–458), so the flat and texture tables
must exist before the first level load (§3.3). `P_GroupLines` allocates the per-sector
line table (536; see §2.10).

### 2.4 `P_LoadThings`, `P_SpawnMapThing`, `P_SpawnPlayer`, `G_PlayerReborn`

- **`P_LoadThings`** (p_setup.c:301–350): caches the lump `PU_STATIC` (309), walks the
  records in lump order (313); outside commercial mode the ten Doom II types set `spawn
  = false` (318–334) and `break` the whole loop (336–337), abandoning every later record
  (E1M1 has no such type: 0 unknown types in the
  [census](validation/M3H9_IWAD_COUNTS.md#census-tables)); otherwise byte-swaps the
  record in place (340–344) and calls `P_SpawnMapThing (mt)` (346); `Z_Free (data)`
  (349).
- **`P_SpawnMapThing`** (p_mobj.c:708–797): type 11 stores up to ten deathmatch starts
  and returns (718–726); types ≤ 4 store `playerstarts` and call `P_SpawnPlayer` unless
  deathmatch (729–737); `!netgame && (options & 16)` returns (740–741); the skill bit is
  2 on medium (743–751); the `doomednum` scan with `I_Error` for an unknown type
  (754–761); the deathmatch and `nomonsters` filters (764–773); floor or ceiling z
  (779–782); `P_SpawnMobj` (784); `spawnpoint` (785); **`if (mobj->tics > 0) mobj->tics
  = 1 + (P_Random () % mobj->tics);`** (787–788); `totalkills`/`totalitems` (789–792);
  the angle (794) and `MF_AMBUSH` (795–796).
- **`P_SpawnMobj`** (p_mobj.c:480–534): `Z_Malloc` and `memset` (490–491); the fields
  from `mobjinfo` (494–504); **`mobj->lastlook = P_Random () % MAXPLAYERS;`** (506), one
  draw per spawn, the player included; the spawn state without its action (509–514);
  `P_SetThingPosition` (517); floor, ceiling and z (519–527); `P_MobjThinker` and
  `P_AddThinker` (529–531), which appends at the tail (p_tick.c:65–71).
- **`P_SpawnPlayer`** (p_mobj.c:642–700): returns if the slot is not in game (654–655);
  `if (p->playerstate == PST_REBORN) G_PlayerReborn (mthing->type-1);` (659–660);
  `P_SpawnMobj (x,y,z, MT_PLAYER)` at the start's position on the floor (662–665);
  colour translation for types > 1 (668–669); `mobj->angle = ANG45 *
  (mthing->angle/45);` (671); `mobj->player = p; mobj->health = p->health;` (672–673);
  `p->mo`, `PST_LIVE`, `refire = 0`, `message = NULL`, `damagecount = 0`, `bonuscount =
  0`, `extralight = 0`, `fixedcolormap = 0`, `viewheight = VIEWHEIGHT` (675–683);
  `P_SetupPsprites (p)` (686; p_pspr.c:831–842: clears the psprites, `pendingweapon =
  readyweapon`, `P_BringUpWeapon`, no draw); cards in deathmatch (689–691); `ST_Start
  ()` and `HU_Start ()` for the console player (693–699). The player gets no spawn-tics
  draw (that statement is in `P_SpawnMapThing` after the type-≤4 return).
- **`G_PlayerReborn`** (g_game.c:800–833): saves `frags`, `killcount`, `itemcount`,
  `secretcount` (809–812); `memset (p, 0, sizeof(*p))` (815), which also clears the
  `viewz = 1` written at p_setup.c:604; restores the four (817–820); `p->usedown =
  p->attackdown = true` (822); `PST_LIVE` (823); `health = MAXHEALTH` (824); ready and
  pending weapon `wp_pistol` (825); fist and pistol owned (826–827); `ammo[am_clip] = 50`
  (828); `maxammo` (830–831).

### 2.5 `P_SpawnSpecials` and the level-start RNG count

`P_SpawnSpecials` (p_spec.c:1239–1362) reads `texture2` presence (unused), clears
`levelTimer`, checks `-avg`/`-timer` (effective only in deathmatch, 1253–1267), then
walks the sectors (1270–1332): 1 `P_SpawnLightFlash` (1278–1281), 2 and 3
`P_SpawnStrobeFlash (…, 0)` (1283–1291), 4 strobe and the special restored to 4
(1293–1297), 8 `P_SpawnGlowingLight`, 9 `totalsecret++`, 10 and 14 the door
countdowns, 12 and 13 strobes in sync, 17 fire flicker. Then `numlinespecials = 0` and
every type-48 line is appended to `linespeciallist` (1336–1347); `activeceilings`,
`activeplats` and `buttonlist` are cleared (1351–1358).

Which spawns draw: `P_SpawnLightFlash` draws once, `flash->count =
(P_Random()&flash->maxtime)+1;` (p_lights.c:142), and zeroes the special (129);
`P_SpawnStrobeFlash` draws `if (!inSync)` only (p_lights.c:205–208), so 2, 3 and 4 draw
and 12 and 13 do not; glow, fire flicker and the door countdowns do not draw.

**The 144.** `M_ClearRandom` runs in `G_InitNew` (g_game.c:1414) and nothing between it
and the first `P_Ticker` reseeds. On E1M1 medium, single player: 91 spawned THINGs + 1
player body = 92 `lastlook` draws (p_mobj.c:506), plus 51 spawn-tics draws for the
spawned things whose spawn state has `tics > 0` (6 barrels, 6 monsters, 2 armors, 37
bonuses; p_mobj.c:787–788), = **143**; then the sector specials of E1M1 (1 ×1, 7 ×4, 8
×2, 9 ×3, 12 ×1, from the
[M3h9 note §2.4](M3H9_REFERENCE_INVESTIGATION.md#24-p_spawnspecials-and-p_updatespecials-p_specc))
add the one special-1 flash draw, the special 12 being in sync: **143 + 1 = 144**, the
value the probe observed. The count is order-independent; which actor receives which
draw value depends on the lump order, which is why the body's position matters (§1).

### 2.6 `G_Ticker`, the loop and tic alignment

**`G_Ticker`** (g_game.c:605–748), in order:

1. The reborn loop: `if (playeringame[i] && players[i].playerstate == PST_REBORN)
   G_DoReborn (i);` (612–614). It runs **before** the gameaction loop.
2. `while (gameaction != ga_nothing)` (617–652): `ga_loadlevel` → `G_DoLoadLevel`,
   `ga_newgame`, `ga_loadgame`, `ga_savegame`, `ga_playdemo` → `G_DoPlayDemo`,
   `ga_completed` → `G_DoCompleted`, `ga_victory` → `F_StartFinale`, `ga_worlddone`,
   `ga_screenshot`.
3. `buf = (gametic/ticdup)%BACKUPTICS;` (656); for each player in game: `memcpy (cmd,
   &netcmds[i][buf], sizeof(ticcmd_t));` (664), `if (demoplayback) G_ReadDemoTiccmd
   (cmd);` (666–667), the recording write (668–669), the turbo check `cmd->forwardmove >
   TURBOTHRESHOLD && !(gametic&31) && ((gametic>>5)&3) == i` writing
   `players[consoleplayer].message` (672–679; `TURBOTHRESHOLD` 0x32, g_game.c:173) and
   the consistency check only `if (netgame && !netdemo && …)` (681–693).
4. `BT_SPECIAL` handling (698–724): `BTS_PAUSE` toggles `paused`, `BTS_SAVEGAME` sets
   `ga_savegame`.
5. `switch (gamestate)` (727–747): `GS_LEVEL` → `P_Ticker (); ST_Ticker (); AM_Ticker
   (); HU_Ticker ();` (729–734); `GS_INTERMISSION` → `WI_Ticker`; `GS_FINALE` →
   `F_Ticker`; `GS_DEMOSCREEN` → `D_PageTicker`.

**The loop.** `D_DoomLoop` (d_main.c:354–407) either runs the `singletics` branch
(`G_BuildTiccmd`, `M_Ticker`, `G_Ticker`, `gametic++`, 375–386) or `TryRunTics` (389),
whose per-tic body is `if (advancedemo) D_DoAdvanceDemo (); M_Ticker (); G_Ticker ();
gametic++;` (d_net.c:742–746). `M_Ticker` animates the menu skull only; `G_BuildTiccmd`
writes `netcmds`, all four of whose gameplay fields the demo overwrites in playback.
Per tic the gameplay sequence is therefore exactly `G_Ticker (); gametic++;`.

**`ticdup` must be 1.** `D_CheckNetGame` sets it from `doomcom` (d_net.c:567–592); a
headless build defines it, and `G_Ticker` divides by it (g_game.c:656).

**Tic alignment.** The first `G_Ticker` after `G_DeferedPlayDemo` runs `G_DoPlayDemo`
(the level load) inside its gameaction loop **and** then reads ticcmd 0 and runs
`P_Ticker` (`leveltime` 0 → 1) in the same call. Ticcmd `n` feeds `P_Ticker` call `n`.
With `N` ticcmds, `G_Ticker` call `N + 1` reads the marker, `G_CheckDemoStatus` calls
`I_Quit`, and no further `P_Ticker` runs. Rust's tic-0 capture is the state after setup
and before any tic; Rust capture `n` (`n ≥ 1`) is after `P_Ticker` call `n`, fed by
ticcmd `n − 1`.

**`gametic` and `leveltime` in gameplay.** `gametic` is read by gameplay only in
`A_Tracer` (`if (gametic & 3) return;`, p_enemy.c:1029; the revenant, absent here) and
by g_game.c at 470, 656, 673, 681–683 and 1654. `leveltime` is read at p_ceilng.c:68,
112; p_floor.c:218; p_plats.c:67; p_map.c:1298; p_mobj.c:464, 554, 599; p_spec.c:1025,
1032, 1043, 1058, 1104; p_user.c:108; p_pspr.c:123, 126, 330, and reset at
p_setup.c:646. Rust's `leveltime` is its `tic`.

### 2.7 `P_Ticker` and `P_PlayerThink`

**`P_Ticker`** (p_tick.c:130–158): `if (paused) return;` (135–136); the menu pause,
`!netgame && menuactive && !demoplayback && players[consoleplayer].viewz != 1`
(139–145), never true headless (`menuactive` false); `P_PlayerThink` for every player in
game (148–150); `P_RunThinkers ()` (152); `P_UpdateSpecials ()` (153);
`P_RespawnSpecials ()` (154; returns unless `deathmatch == 2`, p_mobj.c:591–592);
`leveltime++` (157). `P_RunThinkers` (p_tick.c:101–122) walks the list from
`thinkercap.next`, frees a thinker marked `(actionf_v)(-1)` with `Z_Free` (108–113) and
then reads its `next` (120), which is safe only because `Z_Free` writes the block header
alone.

`P_UpdateSpecials` (p_spec.c:1083–1155) runs the level timer (deathmatch only), the
flat and texture animation (writes `flattranslation` and `texturetranslation`, read only
by the renderer), the type-48 scroll `sides[line->sidenum[0]].textureoffset += FRACUNIT`
(1114–1124) and the button timers (1128–1154), which restore the switch texture and
clear the slot.

**`P_PlayerThink`** (p_user.c:236–384):

| Lines | Statement |
| --- | --- |
| 242–245 | `CF_NOCLIP` → `MF_NOCLIP` (cheats never set here) |
| 249–255 | `MF_JUSTATTACKED` (chainsaw lunge) rewrites the command |
| 258–262 | `if (player->playerstate == PST_DEAD) { P_DeathThink (player); return; }` |
| 267–270 | `reactiontime--` else `P_MovePlayer` (angle `angleturn<<16`, thrust `forwardmove*2048`, `sidemove*2048`, 154–164) |
| 272 | `P_CalcHeight` (bob from momentum; view height and `viewz`; reads `leveltime`, 108) |
| 274–275 | `P_PlayerInSpecialSector` if the sector's special is nonzero |
| 280–281 | `if (cmd->buttons & BT_SPECIAL) cmd->buttons = 0;` |
| 283–319 | `BT_CHANGE`: chainsaw and super-shotgun substitution, plasma/BFG in shareware, `pendingweapon` if owned and not ready |
| 322–331 | `BT_USE` edge on `usedown` → `P_UseLines` |
| 334 | `P_MovePsprites` |
| 339–353 | power countdowns |
| 355–359 | **`damagecount--`, `bonuscount--`** (each if nonzero) |
| 363–383 | `fixedcolormap` from the powers |

**`P_DeathThink`** (p_user.c:182–229): `P_MovePsprites` (187); `viewheight` falls by
`FRACUNIT` to `6*FRACUNIT` (190–194); `deltaviewheight = 0` (196); `onground` and
`P_CalcHeight` (197–198); if `player->attacker && player->attacker != player->mo`, the
angle to the attacker by `R_PointToAngle2` (200–205), `delta = angle -
player->mo->angle` (207); within `ANG5` (`ANG90/18`, 180) the body faces the attacker and
`damagecount` decrements (209–217), otherwise the body turns `ANG5` toward it (218–221);
with no attacker (or self) `damagecount` decrements (223–224); finally `if
(player->cmd.buttons & BT_USE) player->playerstate = PST_REBORN;` (227–228), on any tic,
with no view-height condition.

**The view height is closed.** `viewheight`, `deltaviewheight` and `viewz` are written
by `P_ZMovement` (p_mobj.c:254–257, 303–306), `P_SpawnPlayer` (683), `P_CalcHeight`,
`P_DeathThink` and `P_TeleportMove` (p_telept.c:108), and read by gameplay only in
`P_CalcHeight`/`P_DeathThink` themselves, the menu pause above and `G_CheckSpot`'s
netgame path (g_game.c:885). No other gameplay value depends on them in single player.

### 2.8 The exit and death paths

**Exit.** `P_UseSpecialLine` case 11 calls `P_ChangeSwitchTexture (line,0);
G_ExitLevel ();` (p_switch.c:359–363) inside `P_Ticker`; `G_ExitLevel` sets `secretexit
= false; gameaction = ga_completed;` (g_game.c:1002–1006). The rest of that tic
(`P_RunThinkers`, `P_UpdateSpecials`, `leveltime++`, the HUD tickers) still runs. The
next `G_Ticker` runs the reborn loop, then `G_DoCompleted` (g_game.c:1020–1141):
`gameaction = ga_nothing` (1024), `G_PlayerFinishLevel` per player (1026–1028; clears
powers, cards, `MF_SHADOW`, `extralight`, `fixedcolormap`, `damagecount`, `bonuscount`,
785–791), `AM_Stop` if the automap is up (1030–1031), the intermission data, `gamestate
= GS_INTERMISSION` (1133) and `WI_Start` (1140); the same `G_Ticker` then reads its
ticcmd and runs `WI_Ticker`, not `P_Ticker`. Rust refuses the next `advance` with
`SimError::LevelExited` (crates/doom-sim/src/lib.rs:1272–1274); the comparison ends at
the exit tic.

**Death.** `P_KillMobj`'s player branch (p_inter.c:699–717): `frags[self]++` with no
source (702–703), `MF_SOLID` cleared (705), `playerstate = PST_DEAD` (706),
`P_DropWeapon` (707), `AM_Stop` under the automap (709–715). Then `P_DeathThink` every
tic (§2.7). After a `BT_USE`, the next `G_Ticker`'s reborn loop calls `G_DoReborn`,
which in single player sets `gameaction = ga_loadlevel` (g_game.c:924–932); the
gameaction loop runs `G_DoLoadLevel` and a complete `P_SetupLevel` (new mobjs,
`leveltime` 0, counts 0, `P_SpawnPlayer` → `G_PlayerReborn`) **without reseeding the
RNG**, and the same `G_Ticker` reads the next ticcmd and runs `P_Ticker`. Rust refuses
that `advance` with `SimError::PlayerReborn` (crates/doom-sim/src/lib.rs:1275–1277).

### 2.9 E1M1 specials and their C handlers

Types only, from the M3h7–M3h9 notes; no count beyond those already recorded.

| Special | C handler per tic | Notes |
| --- | --- | --- |
| Sector 1 | `T_LightFlash` (p_lights.c:98–114) | draws on every flip (106, 111); special zeroed at spawn |
| Sector 7 | `P_PlayerInSpecialSector` case 7 (p_spec.c:1029–1034) | 5 damage when `!(leveltime&0x1f)`, only with `mo->z == sector->floorheight` (1016–1017), no suit |
| Sector 8 | `T_Glow` (p_lights.c:314–338) | no draw; special zeroed |
| Sector 9 | case 9 (p_spec.c:1048–1052) | `secretcount++`, special cleared |
| Sector 12 | `T_StrobeFlash` (p_lights.c:155–171) | in sync, no draw; special zeroed |
| Line 1 | `EV_VerticalDoor` via `P_UseSpecialLine` (p_switch.c:326–338) | DR door, `T_VerticalDoor` |
| Line 63 | `EV_DoDoor(normal)` and `P_ChangeSwitchTexture(line,1)` (p_switch.c:550–554) | SR, button timer in `P_UpdateSpecials` |
| Line 11 | `P_ChangeSwitchTexture(line,0); G_ExitLevel ()` (p_switch.c:359–363) | the exit (§2.8) |
| Line 109 | `EV_DoDoor(blazeOpen)` in `P_CrossSpecialLine` (p_spec.c:730–734) | W1 |
| Line 36 | `EV_DoFloor(turboLower)` (p_spec.c:640–644) | W1 |
| Line 38 | `EV_DoFloor(lowerFloorToLowest)` (p_spec.c:652–656) | W1 |
| Line 88 | `EV_DoPlat(downWaitUpStay,0)` (p_spec.c:857–860) | WR, monsters may cross |
| Line 48 | `textureoffset += FRACUNIT` (p_spec.c:1114–1124) | presentation only |

Switch matching uses `switchlist`, built by `P_InitSwitchList` (p_switch.c:107–148) for
episode 1, 2 or 3 by game mode (113–119): **retail gets the episode-1 list only**,
registered episodes 1–2. Rust matches only the episode-1 prefix, "the same in every game
mode, since the episode-1 slots precede every other" (crates/doom-sim/src/mover_rules.rs:113–118),
so an admitted switch behaves identically under either choice.

### 2.10 The 64-bit defects of the genuine loader, and the sanitizer facts

The genuine source assumes 4-byte pointers. On this host's x86-64 (LP64) build:

- **`P_GroupLines`**: `linebuffer = Z_Malloc (total*4, PU_LEVEL, 0);` (p_setup.c:536)
  then writes `total` `line_t*` values of 8 bytes (547), overrunning the zone block into
  the next block header. Observed in a scratch model with the genuine z_zone.c:
  `Z_CheckHeap: next block doesn't have proper back link`, which `G_DoLoadLevel` would
  hit at g_game.c:488. AddressSanitizer cannot see it: the zone is one `malloc`.
- **`maptexture_t`** (r_data.c:84–93) is the on-disk TEXTURE1 record but contains `void
  **columndirectory;` (90): on LP64 `patchcount` and `patches` move from byte offsets
  20/22 to 24/26, so `R_InitTextures` (526–541) misreads every texture entry.
- **`R_InitTextures`**: `textures`, `texturecolumnlump`, `texturecolumnofs` and
  `texturecomposite` are pointer arrays allocated as `Z_Malloc (numtextures*4, …)`
  (r_data.c:482–485): the same overrun as `P_GroupLines` (486–488 are `int` arrays and
  correct).
- **`R_InitColormaps`**: `colormaps = (byte *)( ((int)colormaps + 255)&~0xff);`
  (r_data.c:642) truncates a pointer.
- **The zone's rounding**: `size = (size + 3) & ~3;` (z_zone.c:195) with a 40-byte LP64
  `memblock_t` leaves 8-byte members misaligned. Observed under
  `-fsanitize=address,undefined`: `z_zone.c:254:17: runtime error: member access within
  misaligned address … for type 'struct memblock_t', which requires 8 byte alignment`.
  x86-64 tolerates the misaligned access; only the alignment check objects.
- **p_saveg.c**: 29 pointer-to-integer casts (`PADSAVEP`, p_saveg.c:40, and 18 other
  sites). It is reachable only through saving and loading (§3.1) and is left out.
- w_wad.c is LP64-clean (`lumpcache` sized `numlumps * sizeof(*lumpcache)`, w_wad.c:309);
  z_zone.c:440's `(unsigned)block->user` truncation applies only to purgeable tags, which
  playback with `singledemo` never sets on an owner-less block.

**Sanitizer facts.** The existing harness declares two sanitizer-only adapters
(tools/reference/m3h9b_reference.py:92–101): p_mobj.c:794 (`ANG45 *
(mthing->angle/45)` overflows `int` for quotients outside −4..3; the targets wrap it,
which an unsigned product reproduces) and p_map.c:1221 (`(damage+MAXRADIUS)<<FRACBITS`,
declared for robustness). **`P_SpawnPlayer` has the same product at p_mobj.c:671**, which
needs the same adapter whenever the start's quotient is outside −4..3. GCC under
`-std=gnu89` does not instrument the negative left shifts of the loaders
(`SHORT(..)<<FRACBITS`, p_setup.c:146–147, 247–248, 282–290, 454–455, 481–482; `cmd->angleturn<<16`,
p_user.c:154); the harness keeps gnu89. Under gcc 15's default (gnu23) doomtype.h fails
to compile; gnu89 compiles.

**`-m32` is impossible on this host**: a 32-bit build fails at `bits/libc-header-start.h:
No such file` (no 32-bit libc headers), and the session has no package-install rights.

## 3. The headless genuine build

### 3.1 Compile inventory

The harness flags (tools/reference/generate.py:14) plus `-DNORMALUNIX -DLINUX`:
`-std=gnu89 -O0 -fwrapv -fno-strict-aliasing -ffp-contract=off
-fexcess-precision=standard -fsigned-char`. Observed in scratch: every file below
compiles whole with exit status 0 and default warnings.

| Group | Whole genuine files |
| --- | --- |
| Game loop | g_game.c |
| Play | p_ceilng.c, p_doors.c, p_enemy.c, p_floor.c, p_inter.c, p_lights.c, p_map.c, p_maputl.c, p_mobj.c, p_plats.c, p_pspr.c, p_setup.c, p_sight.c, p_spec.c, p_switch.c, p_telept.c, p_tick.c, p_user.c |
| Support | m_random.c, m_fixed.c, m_bbox.c, m_argv.c, tables.c |
| Data | info.c, sounds.c, dstrings.c, d_items.c, doomstat.c, doomdef.c |
| Resources | w_wad.c, z_zone.c, r_data.c, r_sky.c |

Warnings: only r_data.c:642 (pointer/integer casts), z_zone.c:440 and p_setup.c:631, 633
(format overflow in the commercial lump name, not taken); none in the gameplay files;
no implicit declaration (checked with `-Wimplicit-function-declaration`). The whole set
also compiles and links under `-fno-wrapv -fsanitize=address,undefined
-fno-sanitize-recover=all`.

**r_main.c is a declared extract.** The whole file pulls 37 renderer symbols. Gameplay
needs only `R_PointInSubsector` (p_map.c:141, 399; p_maputl.c:406; p_mobj.c:382, 608;
g_game.c:878), `R_PointToAngle2` and `R_PointToAngle` with `R_PointOnSide`, and the
globals `validcount` (r_main.c:55), `viewx`/`viewy` (75–76; written by
`R_PointToAngle2`, 384–385) and `finecosine` (113): the same four bodies the existing
harness already extracts, compiled with those globals (observed to link).

**p_saveg.c is left out**: referenced only by `G_DoLoadGame`/`G_DoSaveGame`, reachable
only through `ga_loadgame` (the menu) or `ga_savegame` (`BT_SPECIAL|BTS_SAVEGAME` in a
ticcmd, which G1 forbids). Its eight entry points become aborting stubs.

### 3.2 The stub list

The link of the files above, the r_main.c extract and one stub file leaves 43 functions
and 17 variables undefined (observed). Each stub is a declared adapter with its vanilla
body cited. Class **I** = I/O, presentation or unreachable on the playback path; **R** =
reads game state only; **W** = writes game or loop state.

| Vanilla file | Symbol | Class | Vanilla body and why the stub is faithful |
| --- | --- | --- | --- |
| i_system.c | `I_Error` | I | 162–183: prints and exits; the stub prints and exits nonzero |
| i_system.c | `I_Quit` | W | 116–124: shutdowns then `exit(0)`; the stub sets a flag and longjmps to the driver, which ends the run (the contract classifies it as a writer of loop state) |
| i_system.c | `I_Tactile` | I | 55–62: the vanilla body is empty |
| i_system.c | `I_BaseTiccmd` | I | 65–68: used only by `G_BuildTiccmd` (g_game.c:249), not called |
| i_system.c | `I_ZoneBase` | I | 76–80: `malloc (mb_used*1024*1024)`, `mb_used` 6 (i_system.c:51); the stub allocates the same size (G10) |
| i_system.c | `I_GetTime` | W | result stored in `starttime` (g_game.c:486), read only by the timing-demo branch (1653–1655) |
| s_sound.c | `S_Start` | R | 202–245: music and channels; reads mode, episode, map |
| s_sound.c | `S_StartSound` | R | 304–313 reads the origin; 330, 340 draw `M_Random` (pitch), never `P_Random` |
| s_sound.c | `S_StopSound`, `S_PauseSound`, `S_ResumeSound` | R, I, I | channel state only |
| hu_stuff.c | `HU_Start` | R | 428ff: reads the console player, mode, map |
| hu_stuff.c | `HU_Ticker` | W | `plr->message = 0` (526), `players[i].cmd.chatchar = 0` (573, netgame only) |
| hu_stuff.c | `HU_Responder`, `HU_dequeueChatChar` | I | events; `G_BuildTiccmd` only (g_game.c:328) |
| st_stuff.c | `ST_Start` | R | 1444–1454 → `ST_initData` (1249–1278) reads `weaponowned` |
| st_stuff.c | `ST_Ticker` | R | 988–996: `st_randomnumber = M_Random();`, reads the player |
| st_stuff.c | `ST_Responder` | I | cheats (551–720) only through events; none headless |
| am_map.c | `AM_Stop` | W | `automapactive = false` (565); called only when the automap is up (g_game.c:1030, p_inter.c:709–714), never headless |
| am_map.c | `AM_Ticker`, `AM_Responder` | I | 805–809 returns without the automap; events |
| wi_stuff.c | `WI_Start`, `WI_Ticker` | W | `wminfo` edits (1824–1835), `G_WorldDone` (744): after the exit only |
| f_finale.c | `F_StartFinale` | W | `gameaction`, `gamestate`, … (98–101): the E1M8 victory only |
| f_finale.c | `F_Ticker`, `F_Responder` | I | `GS_FINALE`; events |
| d_main.c | `D_AdvanceDemo` | W | `advancedemo = true` (446); unreachable, `I_Quit` comes first (g_game.c:1660) |
| d_main.c | `D_PageTicker` | I | `GS_DEMOSCREEN` only |
| m_menu.c | `M_StartControlPanel` | I | `G_Responder` only |
| m_misc.c | `M_ReadFile`, `M_WriteFile`, `M_ScreenShot` | I | save, load, recording, screenshot |
| p_saveg.c | `P_ArchivePlayers`, `P_ArchiveWorld`, `P_ArchiveThinkers`, `P_ArchiveSpecials`, `P_UnArchive…` ×4 | I | aborting; saving and loading only |
| r_main.c | `R_ExecuteSetViewSize` | I | view size |
| r_draw.c | `R_FillBackScreen` | I | presentation |
| r_things.c | `R_InitSprites` | I | 299: builds `sprites[]` for the renderer |

Variables (17): `player_names` (turbo message only, g_game.c:677), `automapactive`
(false; written by `G_InitNew`, 1446), `fastparm`, `nomonsters`, `respawnparm` (written
from the demo header, g_game.c:1600–1602), `singletics`, `wipegamestate` (written at
472–473), `ticdup` = 1, `maketic`, `netcmds` (zero), `menuactive` = false (read at
p_tick.c:140), `mouseSensitivity`, `save_p`, `setsizeneeded`, `sprites`, `numsprites`,
`screens`. The driver also sets what `D_CheckNetGame` (d_net.c:567–592) sets in single
player: `consoleplayer = displayplayer = 0`, `ticdup`, `playeringame[0] = true`.

**The eight writers, and why `P_Ticker` cannot observe them.** `I_Quit` writes only the
driver's quit flag and ends the run at the demo marker, as vanilla's `exit` does. `HU_Ticker`'s `message`
is never read by `p_*.c` or g_game.c (every site is a write: p_inter.c, p_doors.c,
p_mobj.c:678, g_game.c:678, 1316), and `chatchar` is netgame-only; `I_GetTime` feeds
`starttime`, read only by the timing demo; `D_AdvanceDemo` is unreachable with
`singledemo`; `WI_Start`/`WI_Ticker` run only after the exit, where the comparison has
ended; `F_StartFinale` needs E1M8; `AM_Stop` runs only with the automap up, never true
headless. **No gameplay code calls `M_Random`** (grep of `p_*.c` and g_game.c: none), so
the presentation stubs that would draw it change only `rndindex`, which gameplay reads
only in the netgame consistency check (g_game.c:692). **No stub ever calls `P_Random`**,
so `prndindex` advances exactly as in vanilla. No `S_StartSound` argument has side
effects: the sound choices that draw are separate statements in the genuine code (for
example p_enemy.c:640, 645).

The headless `rndindex` therefore stays 0 where vanilla's would advance with sound and
status-bar draws; Rust's `rng.miscellaneous` also stays 0 on this path. The comparison
treats it as an explicit projection: equal in the headless build, presentation-only in
vanilla.

### 3.3 The driver

A new C file (the only non-genuine code besides the stubs) performs the `D_DoomMain`
steps the playback path needs, in vanilla order:

1. `myargc`/`myargv` set to an empty command line (g_game.c and p_spec.c:1253 call
   `M_CheckParm`).
2. `Z_Init ()` with the vanilla 6 MB zone (G10).
3. `W_InitMultipleFiles` over `{wad, demo.lmp, NULL}`: the WAD (the IWAD in the local run,
   the synthetic WAD in CI, G8) and the generated demo, whose 8-character base name
   becomes the demo lump.
4. `gamemode = registered`, as `IdentifyVersion` maps `doom.wad` (d_main.c:697–701), and
   the matching `gamemission`/`language` defaults (doomstat.c). Declared; §2.9 shows the
   admitted switches are mode-independent.
5. `R_InitTextures` (genuine, with the G3 adapters) and `R_InitFlats` (genuine,
   r_data.c:581–594) in `R_InitData`'s order, without `R_InitSpriteLumps` and
   `R_InitColormaps` (renderer only); `R_InitTextures` runs `R_GenerateLookup` for every
   texture (566–567), so every patch it names must be a valid patch lump.
6. `P_InitSwitchList ()` and `P_InitPicAnims ()`: `P_Init` minus `R_InitSprites`
   (p_setup.c:700–705).
7. The `D_CheckNetGame` single-player values, `singledemo = true`, `ticdup = 1`.
8. `G_DeferedPlayDemo (name)`, then a **direct `G_DoPlayDemo ()` call** to obtain the
   load snapshot (dump "tic 0"). This equals vanilla's first `G_Ticker` up to the point
   where it reads ticcmd 0: the reborn loop runs first in both, finds no `PST_REBORN`
   (players start zeroed, `PST_LIVE` = 0) before the load and `PST_LIVE` after
   `P_SpawnPlayer`, and the gameaction loop is empty after `G_DoLoadLevel` sets
   `ga_nothing` (g_game.c:487).
9. The loop `G_Ticker (); gametic++;` followed by a dump, `N + 1` times; call `N + 1`
   reaches the marker and the `I_Quit` stub ends the run after flushing.

The driver also provides two observers at the object-file boundary, leaving every
genuine text unmodified: GNU ld `--wrap=Z_Malloc` and `--wrap=Z_Free` record an
allocation serial per block (calls between object files, so `P_SpawnMobj`'s and
`P_RunThinkers`' calls are seen), which gives each thinker a stable identity for the
encoder even when the zone reuses an address inside one tic. `Z_FreeTags`'s internal
frees (a reload, refused in Rust) reset the table.

### 3.4 What the driver emits per tic

After the load and after every `G_Ticker`, one state dump, written to scratch only:

- **Thinkers** in list order from `thinkercap.next`: kind by function pointer
  (`P_MobjThinker`, `T_LightFlash`, `T_StrobeFlash`, `T_Glow`, `T_FireFlicker`,
  `T_VerticalDoor`, `T_MoveFloor`, `T_MoveCeiling`, `T_PlatRaise`, or removed `-1`) and
  allocation serial.
- **Every mobj** in thinker order: type, x, y, z, momx, momy, momz, angle, radius,
  height, floorz, ceilingz, flags, health, state index, tics, sprite, frame,
  reactiontime, threshold, lastlook, movedir, movecount, target (serial), player
  index, subsector and sector index, spawnpoint record.
- **Every sector**: floor and ceiling height, light level, special, `soundtarget`
  (serial), `specialdata` presence, and the thing list in link order; the block lists in
  link order.
- **Sector thinkers' fields** (light counters, door, floor and plat state).
- **Lines**: every nonzero special (runtime value).
- **`buttonlist`**: the 16 slots (`btimer`, line, `where`, `soundorg` sector).
- **The player**: `mo`, `playerstate`, health, armor type and points, `attacker`,
  `damagecount`, `bonuscount`, weapons (ready, pending, owned, ammo), `attackdown`,
  `usedown`, `refire`, `bob`, `extralight`, both psprites (state, tics, sx, sy),
  `killcount`, `itemcount`, `secretcount`, `frags`, `cmd`; `viewheight` and `viewz` are
  dumped but not compared (§2.7).
- **Globals**: `prndindex`, `rndindex`, `leveltime`, `gametic`, `totalkills`,
  `totalitems`, `totalsecret`, `gameaction`, `gamestate`, `iquehead`/`iquetail`, the
  thinker count, and the last `numspechit` and intercept count the driver can read.

This is defined here at the level of field lists. Phase 2 fixes the exact columns to
match the existing encoder's row layout (the `A` actor row and its companions: player
`P`/`WX`/`PX`/`AX`, sector `MH`/`S`, line `ML`, thinker `TK`, button `MB`, link `L`, the
counter rows), so that the independent encoder reads them with the least new code.

## 4. Rust inventory

### 4.1 The load path

`doom_wad::Wad::parse (&bytes, Limits::default())` (crates/doom-wad/src/archive.rs:174)
→ `doom_map::Map::from_wad (&wad, "E1M1", MapLimits::default())`
(crates/doom-map/src/parse.rs:105; the ten lumps required in vanilla order, UDMF/Hexen
refused) → `Simulation::original_player_barrels_map (map, MapContextOptions)`
(crates/doom-sim/src/player_monsters.rs:219). `MapContextOptions { respawn_monsters,
initial_rng }` (crates/doom-sim/src/demon_map_context.rs:22–26) has no skill or mode
field: the constructor uses `ActorOptions::default()`, medium, single player, no
`nomonsters` (crates/doom-sim/src/lifecycle.rs:160–192); the default `initial_rng` is
indices (0, 0), `M_ClearRandom`. The setup sequence of every harness
(tests/support/m3h9b.rs:149–166): `spawn_map_things` (spawning.rs:412) →
`spawn_player_body` (original_player.rs:299) → `initialize_player_weapons`
(player_weapons.rs:229; default pistol ready, fist and pistol owned, 50 bullets) →
`spawn_level_specials` (sector_thinkers.rs:329, mandatory before the first tic) →
optional `set_initial_player_reaction_time`. The player record starts with `use_down`
and `attack_down` true (original_registry.rs:1126; weapons.rs:944), as
`G_PlayerReborn` does.

### 4.2 `TicCommand` and the admitted command domain

`TicCommand { forward: i8, side: i8, angle_turn: i16, buttons: u8 }`
(crates/doom-sim/src/actor.rs:5–14); `Simulation::advance ([TicCommand; 4])`
(lib.rs:1265) runs one tic; `step` (1254) adds a snapshot. Demo mapping: `forward` =
forward byte, `side` = side byte, `angle_turn = (byte << 8) as i16`, `buttons` as is.
Preflight: `preflight_player_commands` (crates/doom-sim/src/player_commands.rs:59–131);
button constants `BT_ATTACK` 1, `BT_USE` 2, `BT_CHANGE` 4, `BT_WEAPONMASK` 56
(weapons.rs:8–14).

| Command | Wire 21 | Vanilla |
| --- | --- | --- |
| forward or side outside −50..50 (`MAX_COMMAND_MOVE`, player_commands.rs:9, 94–98) | `PlayerBody(Unsupported)` | accepted up to ±127 (turbo, demos) |
| any `angle_turn` (live) | admitted | admitted |
| `BT_ATTACK`, `BT_USE`, `BT_CHANGE` with weapon 0–3 | admitted (unowned or ready: no change) | the same |
| `BT_CHANGE` with weapon 4–7 | `Weapon(UnsupportedWeapon)` | ignored if not owned |
| weapon bits without `BT_CHANGE` | `UnsupportedButtons` | ignored |
| bit 64 or `BT_SPECIAL` 128 | `UnsupportedButtons` (80–93) | pause or save; `P_PlayerThink` clears the buttons |
| nonzero slot 1–3 | `PlayerBody(Slot)` | other players |
| dead: anything but zero or `BT_USE` alone (wire 21; wire 22 lifts this, §12) | `PlayerBody(Unsupported)` (114–128) | movement ignored |
| dead + `BT_USE` | admitted; the next `advance` → `SimError::PlayerReborn` | level reload |
| exit switch used | admitted; the next `advance` → `SimError::LevelExited` | intermission |

The Rust domain is a strict subset of vanilla's: every admitted command means the same
thing in both, and G1 restricts the scripts to it.

### 4.3 The capture surface

`Simulation::snapshot` (lib.rs:1418) → `Snapshot::canonical_bytes` (snapshot.rs:67–78:
`DSIM`, u32 version, u64 field count, then u32 path length, path, u64 value length,
value per field, little endian), `digest` (80–86, FNV-1a-64 over the canonical bytes,
"not an equivalence proof") and `first_difference` (88, the first differing named
field); `diagnostic_bytes` (diagnostic.rs:2201) yields the same bytes under limits and
`Diagnostic::decode_barrels` (diagnostic.rs:263) reads them back. The groups: `tic`,
`profile`, the schemas, `paused`, `rng.gameplay`/`rng.miscellaneous`, `actors.options`,
`level.*` counters and `exit`, the store, `thinker.order`/`thinker.shared_order`,
`actor.N.*` for every slot (identity, links, position, momentum, size, floor and
ceiling, angle, `lastlook`, flags, type, state, tics, health, reaction time, sprite,
frame, target, `spawnpoint`, threshold, movement, subsector, sector), the sector
thinkers, `spatial.link_order`, `world.*`, `sector.N.*` (floor, ceiling, actors,
`soundtarget`, light, special), `line.N.special` for every line whose map special is
nonzero (snapshot.rs:1288–1292), `button.0..15.*`, `player.N.*` (the command),
`original_player.0.*` (mo, health, state, attacker, `damage_count`, frags, weapons and
psprites, pickups with `bonus_count`, armor, `secret_count`, `use_down`),
`events.last`/`events.pending`, then the static `map.*` group.

**The capture embeds the map.** `map.things`, `linedefs`, `sidedefs` (texture names),
`vertices`, `segs`, `subsectors`, `nodes`, `sectors`, `reject`, `blockmap` (about
63.5 KB on E1M1), and each actor's `spawnpoint` is its THING record. Real-E1M1 captures
are IWAD-derived and stay in scratch; only digests may be committed.

**Not captured**: `viewheight`, `deltaviewheight`, `viewz` (not modelled; only `bob`),
powers and `fixedcolormap` (no power-up admitted), `tracer` (no revenant), `didsecret`
(no intermission), side textures and texture offsets. `damage_count` is captured but
never decays (§1).

### 4.4 The existing comparison engine

`crates/doom-sim/src/movers_replay.rs` is the shared C-row replay engine (with the thin
specs `e1m1_replay.rs` and `barrels_replay.rs`): `parse_rows` (19) reads a packed trace
into rows keyed by (case, op); `setup_map_with` (154) takes any `Map`; `replay_in`
(278) checks the per-op rows; the full pass checks thinkers, buttons, event frames and
the DSIM captures. It cannot take real-E1M1 rows as it stands: the module is
`#[cfg(test)]` (lib.rs:1564–1565) and `pub(crate)`, its `Evidence.rows` is a
`fn() -> &'static Rows` fed by `include_bytes!` of committed traces, and `Case` carries
synthetic-room fields. The smallest reuse is not to replay rows at all: the real run
compares whole DSIM38 captures (§5), which needs only the public API, and the per-row
engine stays for the committed synthetic evidence.

### 4.5 Witnesses reachable on E1M1, and the admitted special sets

Every witness refuses the tic with state preserved:

| Witness | Error | Site |
| --- | --- | --- |
| 129th intercept of a trace | `MapScaleError::Intercepts` (from `CombatError::InterceptLimit`) | combat_trace.rs:322–325; demon_expanded_map.rs:657–668 |
| sight budget per tic | `Perception(WorkLimit)` | demon_expanded_map.rs:320–345 |
| 9th `spechit` | `MapScaleError::SpecialCross` | line_triggers.rs:47–50 |
| vertical invariant | `MapScaleError::Vertical` | player_map_scale.rs:722, 774 |
| escape, position, momentum | `Escape`, `Position`, `Momentum` | barrels.rs:179; world_pickups.rs:516; player_map_scale.rs:832, 846 |
| stale bomb source, player invariants | `PlayerBody(Invariant)` | combat.rs:385–392; original_player.rs |
| thinker visit bound | (internal bound) | store.rs:314–318 |
| exit, reborn | `LevelExited`, `PlayerReborn` on the next `advance` | lib.rs:1272–1277 |

Vanilla's `intercepts[MAXINTERCEPTS]` (128, p_local.h:154) and
`spechit[MAXSPECIALCROSS]` (8, p_map.c:67–69) are unchecked global arrays: an overrun
is undefined behaviour in C (and an AddressSanitizer report in the address build, since
globals are instrumented), a refusal in Rust.

Sector specials admitted at load: 1, 2, 3, 4, 5, 7, 8, 9, 12, 13, 16, 17
(crates/doom-sim/src/sector_thinkers.rs:25); any other refuses
(`Pursuit(Specials)`, demon_expanded_map.rs:277–288). Line specials: 0, 1, 11, 36, 38,
48, 63, 88, 109 (mover_rules.rs:9–23 and the scroller); E1M1 uses only admitted ones.

### 4.6 Player-model gaps

- **`P_DeathThink`**: absent; the dead body's angle stays fixed and `damage_count` does
  not decrement. The reborn request on `BT_USE` exists
  (demon_expanded_map.rs:784–790).
- **`damagecount` decay**: absent (only combat.rs:929 writes it). **`bonuscount`
  decay**: present, live only, after the use check (world_pickups.rs:412–426), as at
  p_user.c:358–359.
- **`P_CalcHeight`**: the momentum part (`bob`) is modelled (weapons.rs:497–503); the
  view part (`viewheight`, `viewz`) is not, and §2.7 shows it unobservable.

## 5. Design of the harness

**Input.** One `.lmp` file per script (G1) feeds both engines: the C driver loads it as
the demo lump through the genuine `W_AddFile` single-lump path and reads it with the
genuine `G_ReadDemoTiccmd`; the Rust runner parses the same 13-byte header (asserting
the G1 values) and maps each 4-byte ticcmd to a `TicCommand` (§4.2).

**Comparison per tic.** The C dump of each tic is projected by the independent Python
encoder (extending `tools/reference/barrels_snapshot.py`) to DSIM38 bytes; the Rust
runner produces its DSIM38 capture for the same tic. The comparator compares the two
byte strings **minus the `map.*` group and minus `events.*`** (the event frames are
Rust's per-tic records, which the dump does not project). On the first mismatch it
decodes both with the encoder's field reader and names the first differing field path,
its tic and both values, then stops that script. The static map is compared once: each
side reports the SHA-256 of the ten lumps it located (C through the genuine
`W_GetNumForName`/`W_CacheLumpNum`, Rust through `Wad::find`/`data`), and the runner
asserts the two lists are equal before tic 0. Alignment: Rust capture `n` against dump
`n` (§2.6), tic 0 being the post-setup state.

**Scripts** (generated by G1's tool from committed sources):

| Script | Content | Ends |
| --- | --- | --- |
| `idle` | zero commands, at least 2,100 tics | the last tic |
| `walk_to_exit` | a hand-authored route through the doors and the lift to the exit switch | the exit refusal (`LevelExited`) |
| `fighting` | turns, fire at the monsters, weapon changes 0–3 | the last tic or the player's death |
| `barrels` | shots at barrels, near monsters and the player | the last tic |
| `chaos` | a fixed-seed pseudo-random stream inside the admitted domain (zero or `BT_USE` alone while dead; wire 21, wire 22 lifts this, §12) | the last tic or a refusal |

**Runner components.**

- `tools/local/e1m1_reference.py` (C runner, local only): builds the headless engine
  (strict, `--overflow-check`, `--address-check` as the existing harnesses do) in a
  scratch directory, runs it on a WAD and a `.lmp`, and stores the dumps in scratch. It
  takes the WAD as an argument, so the same runner serves the synthetic WAD in CI (G8).
- The C driver and stub file (`tools/reference/m3h9c_driver.c`, `m3h9c_stubs.c`), with
  their vanilla citations and hashes in a contract.
- The Rust runner: an example (or an `#[ignore]` test) reading `DOOM_IWAD_DIR` for the
  local run and a path for the synthetic WAD; it writes per-tic captures (scratch) and
  per-tic digests of the stripped bytes.
- The comparator: a Python script that reads both, applies the encoder, reports the
  first divergence and writes the counts-only result.

**Self-test.** The comparator must detect a planted difference at the tic it was
planted: (a) one input tic perturbed in a copy of the `.lmp` given to one side only;
(b) a test-only C build whose driver calls the public `P_Random ()` once after a chosen
tic, never inside gameplay code. Both must report exactly that tic (for (b), the first
field `rng.gameplay`).

**Measurements.** Rust side, through counters in the runner or test-only accessors: the
maximum intercepts per trace, `spechit` per move, the `P_ChangeSector` work, peak
thinkers and actors. C side, only what the driver reads without edits: thinker counts,
the last `intercept_p − intercepts` and `numspechit` after each tic.

**Committed results** (`docs/validation/M3H9C.md`, Phase 2): per script pass/fail, the
tic count, the per-run final digest, the hash of the per-tic digest sequence, any
witness name with its tic, and counts; never map data, a coordinate, an angle or a lump
byte. Dumps and Rust captures stay in scratch and are deleted after the record (G6).

**No IWAD, no run.** Without `DOOM_IWAD_DIR` the local runner prints a skip line and
exits 0, as tools/iwad_counts.py does; the Rust test is `#[ignore]`. CI never runs the
local harness and no workflow step names the IWAD; CI runs the same runner on the
synthetic WAD only (G8).

## 6. New owner decisions (G1–G13) with recommendations

Each: (a) consistent with the brief and F1–F11? (b) no silent divergence? (c) no new
kind of fidelity gap beyond refuse-the-tic witnesses? (d) bounded stores?

**G1. Input format.** Options: (i) the comma-separated per-operation case format of the
synthetic harness (`tests/reference/m3h9b_cases.csv` style), as the M3h9 note §10
sketched; (ii) **a version-110 demo-format `.lmp`**: the 13-byte header (version 110,
skill 2, episode 1, map 1, deathmatch 0, respawn 0, fast 0, nomonsters 0, consoleplayer
0, `playeringame` 1 0 0 0), 4-byte ticcmds and the 0x80 marker (§2.1), generated by our
own tool `tools/e1m1_scripts.py` from committed human-readable script sources. Sources
and generated `.lmp` files are committed under `tests/reference/e1m1_scripts/`.
Constraints checked by the tool: `forwardmove` never 0x80 (the marker), never button
bits 0x80 or 0x40, |moves| ≤ 50, `BT_CHANGE` only to weapons 0–3, zero or `BT_USE` alone
while the player may be dead: Rust's admitted domain, narrower than vanilla's (§4.2) (wire 21;
wire 22 lifts this, §12).
A real 1.9 demo would need only its version byte patched 109 → 110 in the input copy,
which routes later to real demo playback. Rejected: (i), which would give the C and Rust
sides two input paths and none of them genuine on the C side. (a)–(d) yes; the `.lmp` is
public synthetic input, not IWAD data. **Recommendation:** (ii), the version-110 `.lmp`
from `tools/e1m1_scripts.py`, sources and outputs under `tests/reference/e1m1_scripts/`.
Provisionally adopted; confirmed by the owner on 2026-11-05.

**G2. Headless build.** Whole genuine files (§3.1) + the stub list (§3.2) + the r_main.c
declared extract + p_saveg.c's aborting stubs + the driver of §3.3. The gameplay code
text is unmodified; the stubs are declared adapters with cited vanilla bodies, hashed in
the contract with the genuine files. Alternative: extend the body-extraction harness to
the whole engine (thousands of extracted bodies and a re-implemented loader, the
opposite of the goal). (a)–(d) yes. **Recommendation:** whole genuine files with the
declared stubs, extract and driver. Provisionally adopted; confirmed by the owner on 2026-11-05.

**G3. Declared 64-bit ABI adapters.** Applied in every build, the strict one included:

| Site | Original | Adapted | Why behaviour-neutral |
| --- | --- | --- | --- |
| p_setup.c:536 | `Z_Malloc (total*4, PU_LEVEL, 0)` | `Z_Malloc (total*sizeof(*linebuffer), PU_LEVEL, 0)` | the 32-bit original allocates exactly `total` pointers |
| r_data.c:90 | `void **columndirectory;` | a 4-byte field | the 32-bit original reads `patchcount`/`patches` at the same byte offsets (20/22) |
| r_data.c:482–485 | `Z_Malloc (numtextures*4, …)` ×4 | `Z_Malloc (numtextures*sizeof(void*), …)` | exactly `numtextures` pointers, as on 32 bits |
| r_data.c:642 | `((int)colormaps + 255)&~0xff` | `((intptr_t)colormaps + 255)&~0xff` | the same alignment arithmetic without truncation (the driver does not call `R_InitColormaps`; applied so no truncating text is compiled) |
| z_zone.c:440 | `(unsigned)block->user < 0x100` | `(uintptr_t)block->user < 0x100` | `Z_ChangeTag`'s owner test compares the whole pointer on the 32-bit original; the truncation raised a false "an owner is required for purgable blocks" under ASan on the CI runner (added in Phase 2, §12) |

The original and adapted text of each site is hashed in the contract and asserted to
match the source line, as the existing sanitizer adapters are
(tools/reference/m3h9b_reference.py:99–101). **Sanitizer-only adapters**: p_mobj.c:794
and p_mobj.c:671 (the unsigned angle products), p_map.c:1221 (as declared today), and
`-fno-sanitize=alignment` for the zone's 4-byte rounding (z_zone.c:195; the misaligned
`memblock_t` access is valid on x86-64 and exists in vanilla's own 64-bit builds; the
check is disabled rather than the allocator changed, so block addresses stay those of
the strict build). Alternative rejected on this host: `-m32` (no 32-bit libc headers, no
package rights); it is recorded as a confirmation the owner can run later, since a
32-bit strict build needs none of the five ABI adapters.

**The judgment this hinges on.** These five adapters are allocation-size, record-layout and pointer-width
record-layout edits in level-setup and texture-table code, not gameplay behaviour: on
the 32-bit targets the original text computes exactly what the adapted text computes on
LP64. The brief's STOP condition is "changing gameplay code". This note treats LP64
allocation fixes as ABI adapters in the established sense (M3f1's "declared named-field
synthetic ABI adapter", [tools/reference/README.md](../tools/reference/README.md#m3f1-map-scale)),
and **flags the judgment for the owner**. (a) yes; (b) yes, each site is declared and
hashed; (c) no gap; (d) yes. **Recommendation:** the five declared LP64 ABI adapters in
every build, the three sanitizer-only adapters and `-fno-sanitize=alignment` in the
sanitizer builds, `-m32` recorded as a later owner confirmation. Provisionally adopted;
confirmed by the owner on 2026-11-05.

**G4. Policy identity.** A new policy `OriginalExecutionProfile::PlayerMonstersLevelV1`,
wire **22**, DSIM**38**, context **`PLAYER-MONSTERS-LEVEL-1-MEDIUM-v1`**, constructor
`Simulation::original_player_level_map (map, options)`: a superset of wire 21 differing
only in

- (a) the player start THING spawning **at its lump position with its angle** and
  `G_PlayerReborn`'s values (P_LoadThings order; the player's single `lastlook` draw at
  that point, p_mobj.c:506; the angle `ANG45 * (angle/45)` wrapped as in F8);
- (b) `P_PlayerThink`'s `damagecount` and `bonuscount` decrements (p_user.c:355–359; the
  `bonuscount` decrement already exists under wire 21, world_pickups.rs:412–426, and
  stays; the new part is `damagecount`);
- (c) `P_DeathThink` for a dead player (p_user.c:182–229): psprite motion, the turn
  toward the attacker, the `damagecount` decrement and `PST_REBORN` on `BT_USE`;
  `viewheight` is not modelled (§2.7); the reborn tic is still refused.

Wires ≤ 21 stay frozen byte for byte. DSIM38 is the DSIM37 layout with its version,
profile and context bytes; the reader `decode_level` admits any player angle and
requires the body's slot at its lump position. Rejected: changing wire 21 (frozen by the
owner's rule) and an option flag on wire 21 (a profile must identify behaviour). (a)–(d)
yes. **Recommendation:** `PlayerMonstersLevelV1`, wire 22, DSIM38, as above.
Provisionally adopted; confirmed by the owner on 2026-11-05.

**G5. Comparison surface.** As §5: per tic, the DSIM38 bytes minus `map.*` and
`events.*`; the map lumps compared once by SHA-256; on mismatch the first differing
field named with its tic and values. Alternatives: digests only (cannot name the
field); the event frames too (the dump would have to re-derive Rust's event records, a
second encoder of the same facts). (a)–(d) yes. **Recommendation:** the §5 surface.
Provisionally adopted; confirmed by the owner on 2026-11-05.

**G6. Committed results.** Exactly the brief's section 3: per script pass/fail, tic
counts, the per-run final digest, the digest-sequence hash, witness names and tics, and
counts; never map data. C dump rows and Rust captures stay in scratch and are deleted
after the record. (a)–(d) yes. **Recommendation:** as stated. Provisionally adopted;
confirmed by the owner on 2026-11-05.

**G7. Scripts.** The five of §5; `walk_to_exit` is authored iteratively against the C
build locally (the route is input, not IWAD data: it contains turns and durations, no
coordinate); every script is committed as its source and its `.lmp`. (a)–(d) yes.
**Recommendation:** the five scripts, committed as sources and `.lmp` files.
Provisionally adopted; confirmed by the owner on 2026-11-05.

**G8. CI coverage without IWAD content.** A generated synthetic game WAD
(`tools/reference/m3h9c_wad.py`): `PNAMES`/`TEXTURE1` with blank patches named for every
registered-mode switch texture (`P_InitSwitchList` calls `R_TextureNumForName`, which
`I_Error`s on a missing name, p_switch.c:144–145; r_data.c:726), `SKY1`, `F_START`/`F_END`
flats including `F_SKY1`, empty `S_START`/`S_END`, plus an E1M1-scale synthetic map named
`E1M1` (the M3h9b dense64-style room generator at E1M1's counts: 9 movers, 8 scrollers,
27 decorations, 6 barrels, 6 monsters, 52 pickups), run through the **same** whole-engine
C runner (strict and both sanitizers), the same encoder and the same Rust runner in CI,
with committed goldens (synthetic, committable). The C runner therefore has no IWAD
dependency, only a WAD argument. Rejected: a third extraction-based generator (it would
re-adapt the player stage that the real run exercises genuinely). (a)–(d) yes.
**Recommendation:** the synthetic WAD through the same runner, encoder and Rust runner,
goldens committed. Provisionally adopted; confirmed by the owner on 2026-11-05.

**G9. Fix policy.** On a divergence, find the cause. Rust fixes land under wire 22 only
(older wires frozen), each with a synthetic scenario through the G8 path. A gameplay-code
edit in C is never a fix. Behaviour outside wire 22's domain ends that script's
comparison at that tic and is recorded. (a)–(c) yes; (d) unchanged. **Recommendation:**
as stated. Provisionally adopted; confirmed by the owner on 2026-11-05.

**G10. Zone size.** Vanilla's 6 MB default (`mb_used = 6`, i_system.c:51); raised only
if `Z_Malloc` fails, and then recorded. Addresses are unobservable by gameplay: the
thinker list is a linked list in insertion order (`P_AddThinker` appends,
p_tick.c:65–71; `P_RunThinkers` follows `next`, p_tick.c:105–121) and the sector and
block lists are linked in `P_SetThingPosition`'s order (p_maputl.c:436), none ordered by
address. (a)–(d) yes. **Recommendation:** 6 MB, raised only on failure and recorded.
Provisionally adopted; confirmed by the owner on 2026-11-05.

**G11. Witnesses.** A refusal ends the Rust run; the C run continues uncompared; the
witness name, tic and script are recorded as a finding; a witness is never weakened to
let a run continue. (a)–(d) yes. **Recommendation:** as stated. Provisionally adopted;
confirmed by the owner on 2026-11-05.

**G12. Timings.** Matched timings against master for every older-policy workload
(digests equal), callgrind on any slowdown over 3 %, plus the per-real-E1M1-tic cost of
each runner, all alone on the idle machine. (a)–(d) yes. **Recommendation:** as stated.
Provisionally adopted; confirmed by the owner on 2026-11-05.

**G13. The Demon-profile codegen regression.** The matched timings (step 9) show
`canonical_demon`, `demon_expanded_map` and `diagnostic_free` 6–13% slower than master with
identical digests: `Simulation::advance` runs 13–14.5% more instructions, because the release
build's codegen-unit split no longer lets `BTreeMap<StateId,_>::insert`/`remove` inline into
`ActorRegistry::immediate_cycle` (the cause is in the
[evidence record](validation/M3H9C.md#matched-performance)). Options: (i) accept as explained
(no code change; identical digests; +0.012% Ir with one codegen unit); (ii) `codegen-units = 1`
or LTO in the release profile (slower builds, and still 10.6% above master's favourable split
on these workloads); (iii) replace the `BTreeSet`s in `immediate_cycle` with dense marks (a
data-structure change in a frozen-policy path that needs its own evidence).
**Recommendation:** (i) now; (iii) to be considered in a maintenance milestone with the owner.
Provisionally adopted; confirmed by the owner on 2026-11-05.

## 7. Evidence plan (C first)

1. **The whole-engine C runner** on the synthetic WAD first: strict, `--overflow-check`
   and `--address-check` builds must give **byte-identical dumps** for every synthetic
   script, with `ASAN_OPTIONS=detect_leaks=0` disclosed (LeakSanitizer disabled, not
   passed). The contract records every genuine file's SHA-256, the extract's bodies, each
   stub with its vanilla citation, the G3 adapters (original and adapted text), flags,
   compiler and platform.
2. **The synthetic goldens** (`tests/reference/m3h9c_*`: the WAD generator's output, the
   scripts' `.lmp`, the encoder's DSIM38 digests and selected state fixtures) are
   committed **before** the Rust writer, from the C dumps and the independent encoder,
   never from Rust output.
3. **The Rust runner** must match those goldens byte for byte, then the real run is made
   locally. Its results are committed only as digests and counts (G6).

Distinctions kept in the record: source equivalence (genuine files and the declared
adapters), explicit projection (the dump columns and `rndindex`), independent DSIM
consistency (the encoder) and Rust-only tests (the witnesses, the reader negatives,
the older wires' frozen goldens).

## 8. Size estimate and the unattended-rule verdict

Estimate: C driver, stubs and contract ≈ 1.5–2k lines; the synthetic WAD generator
and scripts tool ≈ 1–1.5k; the encoder extension and comparator ≈ 1–1.5k; Rust (the
variant and its version-range sites, the lump-order body spawn, the two player-model
additions, DSIM38 writer and reader, the runner, tests, fuzz target) ≈ 4–5k; docs ≈ 2k;
goldens are data. **About 10–12k lines, below the 25k split threshold.**

Every decision G1–G12 has a recommendation consistent with the brief's section 5; the
headless C needs only I/O stubs plus the five declared LP64 allocation, layout and pointer-width
adapters (the judgment of G3, flagged); no gameplay code text changes; and the work fits
one milestone. **Phase 2 therefore proceeds with the decisions provisionally adopted.**

## 9. Implementation plan (one commit per step, pushed; evidence before runtime)

1. **C runner**: the driver, the stubs, the r_main.c extract, the G3 adapters, the
   synthetic WAD generator `tools/reference/m3h9c_wad.py`, `tools/e1m1_scripts.py` with
   the script sources; strict and both sanitizer builds identical on the synthetic WAD.
2. **Wire 22 and the Rust runner**: the variant, context, predicates, version ladder
   (38) and every range site; the lump-order body spawn with its angle; the
   `damagecount` decrement; `P_DeathThink`; the DSIM38 writer and `decode_level`; the
   runner (example and `#[ignore]` test reading `DOOM_IWAD_DIR`).
3. **Encoder and comparator**: the DSIM38 projection of the dump, the synthetic goldens
   committed, the comparator with first-field naming.
4. **Self-test**: the perturbed input tic and the extra driver draw, both reported at
   the planted tic.
5. **Runs**: the five scripts on the real E1M1 locally; every divergence explained
   (G9); `walk_to_exit` authored iteratively (G7).
6. **Measurements**: intercepts, `spechit`, `P_ChangeSector` work, peak thinkers and
   actors on the real runs.
7. **CI**: the synthetic room through the runner (three builds), the encoder check and
   the Rust runner in debug and release; the local harness absent from CI.
8. **Docs**: COMPATIBILITY section for wire 22, STATE_FORMAT DSIM38, TESTING (the
   local-only harness and the CI steps), ROADMAP, README (one policy row, "Where it
   stands"), `docs/validation/M3H9C.md`, this note's corrections section.
9. **Matched timings** (G12).
10. **Fuzz**: a target for wire 22 with a seeder; seed replay, one coverage-guided and one
    AddressSanitizer campaign, `detect_leaks=0` disclosed.
11. **Independent review** (Opus) of the full diff against the C source, including a
    check that no stub writes game state `P_Ticker` can observe and that no gameplay text
    differs from upstream; fix and record.
12. **Full gates** from clean; draft PR; ready when every job is green on the exact head;
    scratch (builds, dumps, captures) deleted and free space reported.

## 10. Policy proposal

`OriginalExecutionProfile::PlayerMonstersLevelV1`, wire 22, DSIM38, context
`PLAYER-MONSTERS-LEVEL-1-MEDIUM-v1`, constructor `Simulation::original_player_level_map`,
a superset of `PlayerMonstersBarrelsV1`: every load rule and bound of wire 21, with the
player start spawned at its lump position with its angle and `G_PlayerReborn`'s values,
`damagecount` decay and `P_DeathThink` (no view height); the reborn and exit tics still
refused. Runtime witnesses unchanged. DSIM38 as G4. Older policies byte and behaviour
identical, confirmed by the unchanged goldens and matched digests. Support is documented
in [COMPATIBILITY](COMPATIBILITY.md#original-monsters-with-exploding-barrels-pickup-scale-and-every-thing-angle)'s
successor section and the schema in [STATE_FORMAT](STATE_FORMAT.md) when Phase 2 lands.

## 11. Follow-ups, risks, status

- **Risks.** The encoder must reproduce Rust's slot assignment and thinker order from
  the dump (the `--wrap` allocation serials of §3.3 make identities exact; slot reuse
  must follow Rust's rule, as the existing encoder's birth-identity simulation does).
  The synthetic WAD's texture tables must satisfy `P_InitSwitchList` for registered mode
  and `R_GenerateLookup` for every patch they name. `walk_to_exit` authoring is
  iterative. Wire-22 plumbing across the version-range sites is the usual risk, covered
  by the identical older goldens. The LP64 adapters are the owner's call (G3). Side
  textures are not in DSIM: a switch texture swapped by `P_ChangeSwitchTexture` and
  restored by the button timer is compared only through `buttonlist`. The per-tic
  capture costs about 13–16 times the simulation; a 35,000-tic run is still seconds.
- **Quirks recorded.** The 109/110 version byte; retail's episode-1 switch list in
  `P_InitSwitchList`; `P_LoadThings` abandoning every later record at the first Doom II
  type outside commercial mode; `G_PlayerReborn`'s `memset` clearing `viewz = 1`; the
  reborn reload keeping the RNG; the turbo check writing a message for `forwardmove >
  50`; the unchecked `intercepts` and `spechit` arrays (witnesses in Rust).
- **Later**: real 1.9 demo playback (the version-byte route of G1), easy and hard skill,
  the reborn reload as an admitted transition, the rest of Episode 1.

**Status / next step.** Phase 2 complete: steps 1–12 done. **The owner confirmed decisions
G1–G13 on 2026-11-05.** Pull request #56 is ready for review with every check green on its
head; merging is the owner's call.

## 12. Corrections and findings from Phase 2

- **z_zone.c:440, a fifth LP64 adapter (found by CI).** `Z_ChangeTag` tests `(unsigned)block->user < 0x100`; on LP64 the truncated owner pointer can be small, and under ASan on the GitHub runner `W_CacheLumpNum`'s re-caching of the synthetic WAD's one patch lump raised "an owner is required for purgable blocks" (every local build and the other variants passed by address layout alone). The adapter compares the whole pointer as the 32-bit original does; the dumps are unchanged.
- **Exit tic.** `G_Ticker` handles `gameaction` before the demo read, so the driver stops on
  `ga_completed`; `WI_Start` stays an aborting stub.
- **Zone fill.** The runner zero-fills `Z_Malloc`; before that, uninitialised plat and floor
  fields made the strict and address dumps differ on the barrels run.
- **Closet DR lines.** The synthetic map's closet-side DR lines were refused by Rust mover
  rule 4; DR lines are on the hall side only.
- **p_enemy.c angle spreads.** Three int angle-spread overflows (lines 816, 839, 860) found
  under UBSan became sanitizer-only adapters.
- **Slot assignment.** `K.adds` and `RR` observer rows were needed for exact slot
  assignment in the independent projection.
- **bonuscount.** Already decayed in Rust under wire 21; only `damagecount` was new.
- **The body has no spawnpoint** (`p_mobj.c:736`), so `decode_level` checks lump order via
  the preceding actors.
- **Dead-player bob.** Rust's `P_CalcHeight` bob is computed for a dead player too.
- **Dead-player command domain.** A dead player accepts any admitted command under wire 22
  (`P_DeathThink`, p_user.c:182–229, reads only `BT_USE`); the zero-or-`BT_USE` limit of
  §4.2 and G1 is wire 21's, and the script tool checks nothing about death.
- **Order of the commits.** The plan (§7) put the goldens before the Rust writer; in fact
  the wire-22 Rust writer (4e64d08) preceded the first committed goldens (cb99f8c). The
  encoder never reads Rust output; every correction made to it while matching cites C or
  the schema (the N6 zero projection, then the observer rows `T`/`K`/`RM`/`RR` and
  `K.adds` that replaced its inferences) and is listed in the evidence record.
- **Reborn tic.** The driver also stops on a reborn request (`end=reborn`: the next
  `G_Ticker` would reload the level, g_game.c:612–614, 924–931); the synthetic `SYNDIE`
  script (die, then USE) compares that tic, and Rust refuses the next one with
  `PlayerReborn`. The reload itself stays outside the domain (G11). The real chaos script did
  not die in 5,000 tics; the `walk_to_exit` demo ends exactly on the exit tic, so no refused
  tic is observed there.
- **Demon-profile codegen regression (found by the matched timings).** The older Demon
  workloads run 6–13% slower and `Simulation::advance` 13–14.5% more instructions, though
  `lifecycle.rs` is unchanged: adding `pub mod demo` (commit `2130d9c`) changed the 16-unit
  codegen split so `BTreeMap<StateId,_>::insert`/`remove` no longer inline into
  `immediate_cycle`; with one codegen unit the trees differ by +0.012% Ir. Recorded as G13;
  measurements are in the [evidence record](validation/M3H9C.md#matched-performance).
