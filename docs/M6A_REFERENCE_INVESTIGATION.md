# M6a: Episode 1 from start to finish in one session: reference investigation

Phase 1 note for milestone M6a, which is to make one `doom-play` session play Episode 1 the way linuxdoom-1.10 does: finish a map by either exit, see the intermission, start the next map with what the player carries, die and be reborn, take the secret exit and come back, and reach the end of E1M8 and what follows it. The proof is to be the usual one: one recorded demo spanning levels, replayed in the genuine engine, with the full state, every frame, every sound row and every mixer buffer compared, on synthetic maps in CI and on the owner's IWAD locally. Vanilla is the default and nothing unproven may diverge silently: anything not proven is a refuse-the-tic witness with the state preserved. Menus, the automap, saving and loading, music and the other episodes are not in M6a; none of them is optional, and each moves to a later milestone, to be named by the owner. The original's pause command and nightmare land in M6b (U13). The screen melt and both easy skills are in M6a (U9, U12), as the owner ruled on 2026-11-21. This note records the C facts, what the Rust code does today, the harness design with the scratch prototypes that were actually run, the Rust design, owner decisions U1–U15 with one recommendation each and the owner's rulings, and the Phase 2 plan, size estimate and cap.

The behavioural authority is the original id Software Linux DOOM 1.10 at `a77dfb96cb91780ca334d0d4cfd86957558007e0`; every `file:line` without a directory is in `linuxdoom-1.10/`. Milestone status lives in [ROADMAP](ROADMAP.md), support in [COMPATIBILITY](COMPATIBILITY.md), schemas in [STATE_FORMAT](STATE_FORMAT.md) and commands in [TESTING](TESTING.md) (see the [documentation authority map](README.md#documentation-authority)); Phase 2's observations go into an evidence record written then. Runs against the IWAD are recorded here only as counts and pass/fail facts: no lump content, coordinates or geometry. Lump names appear only where the C source itself names them. All dates are America/Los_Angeles.

Each statement is labelled: **(read)** read in the source; **(observed)** seen by running a scratch prototype, with the command in [§1.15](#115-what-was-run-and-what-was-not) or [§3](#3-harness-design-evidence-first); **(inferred)** reasoned from cited lines and not run. The facts come from two Phase 1 research reports (C facts; the Rust code and harness), kept with their prototypes under `~/scratch/m6a/` until the milestone ends; the writer of this note re-read a sample of their citations against the source ([§1.15](#115-what-was-run-and-what-was-not)).

**Status / next step (2026-11-21 17:50 PST).** **The owner's rulings on U1–U15 were received on 2026-11-21 and are recorded under each decision in [§5](#5-owner-decisions-u1u15-with-recommendations); Phase 2 is in progress.** U4 and U9 were ruled differently from the recommendations. **Step 1, the genuine `m6a` runner, is done** (`tools/reference/m6a_build.py`, `m6a_session.c`, `m6a_wi_access.c`, `m6a_reference.py`; commands in [TESTING](TESTING.md#the-genuine-m6a-runner-phase-2-step-1)). **Step 2's level-chain part is done**: the synthetic multi-map WADs, pictures, music and variants (`tools/reference/m6a_wad.py`), 18 items for exits, intermission paths, the carried player, reborns, the secret route, the reborn-beats-a-same-tic-exit witness (U8 item 12, now observed: [§1.9](#19-death-and-reborn-in-single-player)) and three music refusals, with goldens from the genuine engine only (`tools/reference/m6a_goldens.py`; commands in [TESTING](TESTING.md#the-synthetic-session-items-and-their-goldens-phase-2-step-2)); the finale, melt-picture and skill items of step 2 follow. The order of work is the owner's ([§6.5](#65-order-of-work-owners-ruling-2026-11-21)): the playable chain first, then a stop for the owner's hand test, then the finale, the melt pictures and the easy skills. No Rust source, `Cargo.toml`, frozen tool, golden or existing file under `tests/reference` has changed; steps 1 and 2 added new `m6a_*` files only. This branch starts at master `d637f7d` and has since merged master; M5d is merged.

## Summary

- **The C path is short and fully traced.** An exit only sets `gameaction`; the next `G_Ticker` runs `G_DoCompleted` and the intermission's first tic together; when the intermission ends, the next `G_Ticker` loads the next map and runs its tic 0 with that tic's command. A dead player's use press reloads the current map the same way. One demo carries all of it: commands are read every tic in every game state, with no per-level header ([§1.1](#11-completing-a-level-call-chain-tic-by-tic), [§1.10](#110-demos-across-levels)).
- **What the player carries** is health, armour, weapons, ammunition, the backpack, the cheat flags, `deltaviewheight` and the fire and use latches; keys and every power-up are dropped. Neither random index is ever reset between levels or at a reborn; only a new game resets them ([§1.2](#12-what-is-carried-and-what-is-reset), [§1.3](#13-statistics-time-and-the-two-random-indices)).
- **Findings that go beyond or against the brief** ([§1.14](#114-findings-that-contradict-or-go-beyond-the-brief)): there is no intermission after E1M8; the intermission and the finale never end by themselves; the owner's four-episode `DOOM.WAD` runs as *registered* because of its file name, and the registered finale's second screen needs a lump (`HELP2`) his IWAD does not have; the genuine engine aborts when a music lump is missing while the Rust app plays on (a pre-existing divergence, documented here, not patched); the intermission's background animation is never drawn although its random draws happen; level sounds continue into the intermission; per-map demos cannot be spliced into a real-map episode.
- **Nothing wrong was found in a frozen wire at the boundary.** Rust against the genuine engine up to the exit, secret-exit, E1M8-exit and reborn tics: 0 differing sound rows and 0 mismatched frames on 2 IWAD runs and 4 synthetic scripts ([§2.7](#27-frozen-wires-at-the-boundary-what-the-first-comparison-found)).
- **The Rust code stops at every boundary today**: the simulation refuses the tic after an exit or a reborn, the app shows its own end screen, a demo holds one map, a level always starts with a fresh player and both random indices at 0, and the display's and sound's saved states fix per-level things ([§2](#2-what-the-rust-code-does-today)).
- **A prototype of the genuine engine ran across levels**: a fork of the M5c runner with `wi_stuff.c` and `f_finale.c` compiled whole went through exits, intermissions, a reborn, the secret level and the finale on a synthetic six-map WAD, and through E1M1 into E1M2 on the owner's IWAD, with no abort and every wipe exactly 320 draws ([§3](#3-harness-design-evidence-first)).
- **The recommended design** keeps the simulation one level at a time and puts the session (the game-action loop, the intermission, the finale, the reborn) in the game layer, with one new wire in `doom-sim` that only adds a carried start ([§4](#4-rust-design), U2).
- **Size.** About 10,400–19,400 hand-written lines are projected (central 14,900, ±30%) against a proposed cap of 22,000 ([§6.3](#63-size)).

## 1. C facts

### 1.1 Completing a level: call chain, tic by tic

Single player, one tic per `G_Ticker`.

1. **(read) The exit tic t.** The exit specials call `G_ExitLevel` (`secretexit = false; gameaction = ga_completed`, g_game.c:1002-1006) or `G_SecretExitLevel` (g_game.c:1009-1018). Callers: walk lines 52 and 124 (p_spec.c:677-680, 760-763), switches 11 and 51 (p_switch.c:359-363, 431-435), sector special 11 (p_spec.c:1054-1063), the deathmatch-only level timer (p_spec.c:1092-1097). `A_BossDeath` on E1M8 lowers floors and does not exit (p_enemy.c:1627-1635, 1682-1705, 1729-1735). Only `gameaction` is set; the rest of tic t's `P_Ticker` runs (p_tick.c:149-157), then `ST_Ticker`, `AM_Ticker`, `HU_Ticker` (g_game.c:729-734). The display after tic t is a level frame.
2. **(read) Tic t+1.** `G_Ticker` runs the reborn loop first (g_game.c:611-614), then the `gameaction` loop (617-650): `ga_completed` → `G_DoCompleted` (g_game.c:1020-1141): `gameaction = ga_nothing`; `G_PlayerFinishLevel` for every player (1026-1028; body 779-792); for Doom 1, map 8 → `gameaction = ga_victory; return` (1033-1038, repeated 1046-1052) and map 9 → `didsecret = true` for every slot (1039-1042); `wminfo` filled: `didsecret` from the console player, `epsd`, `last` (1064-1066), `next` ([§1.4](#14-how-the-next-map-is-chosen)), the level's totals as maxima, `partime = 35*pars[episode][map]` (1112-1119), per player the kill, item and secret counts and `stime = leveltime` (1122-1131); `gamestate = GS_INTERMISSION` (1133); `WI_Start(&wminfo)` (1140). The same `G_Ticker` then copies the tic's command and reads or writes the demo (658-670), checks the special buttons (698-724) and runs `WI_Ticker` (736-738). **The first intermission tic is the `G_Ticker` that ran `G_DoCompleted`.**
3. **(read) End of the intermission.** `WI_updateNoState` calls `WI_End` and `G_WorldDone` when its counter reaches 0 (wi_stuff.c:737-747). `G_WorldDone` sets `gameaction = ga_worlddone` and, after a secret exit, the console player's `didsecret` (g_game.c:1147-1152). This happens inside `WI_Ticker`, after that tic's `gameaction` loop, so it acts at the next tic.
4. **(read) The load tic u.** `G_DoWorldDone` (g_game.c:1172-1179): `gamestate = GS_LEVEL; gamemap = wminfo.next+1; G_DoLoadLevel()`. The same `G_Ticker` then copies tic u's command and runs `P_Ticker`, `ST_Ticker`, `HU_Ticker` (664-669, 729-734): **the new level's tic 0 runs in the same `G_Ticker` as the load, with tic u's command.**
5. **(read) `G_DoLoadLevel`** (g_game.c:445-497): `levelstarttic = gametic` (470; written, never read anywhere in the tree); `if (wipegamestate == GS_LEVEL) wipegamestate = -1` (472-473); a `PST_DEAD` player becomes `PST_REBORN`, every slot's frags are zeroed (477-482); `P_SetupLevel` (484); input state cleared (490-496), but not `turnheld` (184) or the double-click state (193-198). The sky texture is set only at `G_InitNew` for Doom 1 (1464-1479), so it stays the same for the whole episode.
6. **(read) `P_SetupLevel`** (p_setup.c:583-693): totals and every player's kill, item and secret counts zeroed (594-600); `viewz = 1` for the console player (604); `S_Start()` (607): every channel with a sound stopped (s_sound.c:209-211), the level's music changed (235-245); `Z_FreeTags` frees every mobj, thinker and level lump (618); `leveltime = 0` (646); things spawned, each non-player thing drawing `P_Random` once for `lastlook` (p_mobj.c:506) and once more for its start tics if they are positive (787-788); `P_SpawnSpecials` (682).
7. **(read) `P_SpawnPlayer`** (p_mobj.c:642-700): `PST_REBORN` → `G_PlayerReborn` (659-660); a new `MT_PLAYER` (one `P_Random` draw); `mo->health = p->health` (673); `refire`, `message`, `damagecount`, `bonuscount`, `extralight`, `fixedcolormap` cleared and `viewheight = VIEWHEIGHT` (675-683); `P_SetupPsprites` (686); `ST_Start` and `HU_Start` (693-699).
8. **(read) `P_SetupPsprites`** (p_pspr.c:831-842) sets `pendingweapon = readyweapon` (so a weapon change pending at the exit is cancelled) and calls `P_BringUpWeapon` (p_pspr.c:138-154), which **starts `sfx_sawup` when the weapon is the chainsaw** (145-146). (inferred) So a player who carries the chainsaw as the ready weapon into a level makes a sound start, with its `M_Random` pitch draw (s_sound.c:327-334), during the load, after `S_Start` and before tic 0. A fresh game or a reborn never does (the ready weapon is the pistol, g_game.c:825).

### 1.2 What is carried and what is reset

"Finish" is `G_PlayerFinishLevel` (g_game.c:779-792), "spawn" is `P_SpawnPlayer` (p_mobj.c:642-700), "reborn" is `G_PlayerReborn` (g_game.c:800-833: a `memset` of the whole player at 815 keeping only frags and the three counts, then `usedown = attackdown = true`, `PST_LIVE`, health 100, pistol ready, fist and pistol owned, 50 bullets, the `maxammo` table, 822-831). All (read) unless marked.

| Field (d_player.h:83-166) | Across a level exit (alive) | Across a reborn |
| --- | --- | --- |
| `health` | carried; copied to the new mobj (p_mobj.c:673) | 100 |
| `armorpoints`, `armortype` | carried | 0 |
| `weaponowned`, `readyweapon` | carried; a pending change cancelled (p_pspr.c:840, 142-150) | fist and pistol; pistol ready |
| `ammo`, `maxammo`, `backpack` | carried | 50 bullets; the table; no backpack |
| `cards` | cleared at finish (g_game.c:786) | cleared |
| `powers` (all six, berserk and computer map included) | cleared at finish (785); `MF_SHADOW` removed (787) | cleared |
| `cheats` | carried | cleared |
| `deltaviewheight` | carried (inferred: a landing just before the exit gives a short squat at the next start through p_user.c:113-136) | 0 |
| `extralight`, `fixedcolormap`, `damagecount`, `bonuscount` | 0 at finish (788-791) and at spawn | 0 |
| `attackdown`, `usedown` | the values the intermission's last tic left (`WI_checkForAccelerate` rewrites both every intermission tic, wi_stuff.c:1476-1497) | true |
| `killcount`, `itemcount`, `secretcount` | zeroed by `P_SetupLevel` (p_setup.c:596-600) | zeroed there too |
| `didsecret` | carried; set on map 9 (g_game.c:1039-1042) and after a secret exit (1151-1152) | cleared |
| `playerstate` | `PST_LIVE`; a player dead at the load is reborn with a pistol start (g_game.c:477-480) | `PST_LIVE` |
| `attacker` | a stale pointer into freed memory; (inferred) never read stale, because its readers (p_user.c:200-205, st_stuff.c:800-817) need a new `P_DamageMobj`, which writes it first (p_inter.c:874-875) | NULL |
| `frags` | zeroed (g_game.c:481) | zeroed |

(inferred) A player can be dead at an exit: `P_PlayerInSpecialSector` can kill before `P_UseLines` in the same `P_PlayerThink` (p_user.c:274-275, 322-328), and a monster can kill after the exit in the same tic. Sector 11 cannot kill: its damage is clamped to `health - 1` (p_inter.c:836-843).

### 1.3 Statistics, time and the two random indices

- (read) The totals are zeroed at load and counted at spawn (p_setup.c:594; p_mobj.c:789-792; p_spec.c:1305), copied into `wminfo` at completion (g_game.c:1112-1114); `WI_initVariables` writes 1 into a zero maximum of the global `wminfo` (wi_stuff.c:1824-1831). Par times for E1M1–E1M9 are `{30,75,120,90,165,180,180,30,165}` seconds (g_game.c:978-984); E1M8's is never used.
- (read) `gametic` advances after every `G_Ticker` in every game state (d_net.c:745-746). `leveltime` advances only at the end of an unpaused `P_Ticker`, so only in a level (p_tick.c:135-157), and is zeroed by `P_SetupLevel`. So `stime` counts the tics since the last load of that map, the exit tic included; a reborn restarts it.
- (read) `prndindex` and `rndindex` (m_random.c:53-54) are reset only by `M_ClearRandom` (m_random.c:69-72), whose one caller is `G_InitNew` (g_game.c:1414), reached from a new game, a loaded game, demo playback and the start-up warp. **Neither index is reset at an exit, a load or a reborn.** (observed) In the prototype the `P_Random` index ran on through every load and reload, advanced only by the new level's spawns.

### 1.4 How the next map is chosen

- (read) Normal exit from map m (1–7) → map m+1 (g_game.c:1108-1109). Secret exit from any map → E1M9 (1087-1088). Normal exit from E1M9 → **E1M4**, whichever map the secret exit was taken on (1089-1096). A secret exit on E1M9 itself → E1M9 again.
- (read) Exit from E1M8 → `ga_victory` → `F_StartFinale` in the same `G_Ticker` (g_game.c:639-641), **with no intermission and no statistics** (1033-1038).
- (read) `wi_stuff.c` uses `next` only for drawing ("Entering", the pointer, the splats: wi_stuff.c:448-451, 790-802). The secret level's splat: entering E1M9 from E1M3, `wminfo.didsecret` is still false (it is set by `G_WorldDone` after that intermission), so E1M9 is not splatted; leaving E1M9 it is true. (inferred) A death later clears it (the reborn `memset`), so later intermissions stop splatting E1M9.

### 1.5 Game modes, and how the owner's DOOM.WAD is identified

- (read) `IdentifyVersion` (d_main.c:563-717) tests only file names, in order: the Doom II names, then `doomu.wad` → **retail** (690-695), `doom.wad` → **registered** (697-702), `doom1.wad` → shareware (704-709). No lump probe upgrades registered to retail. So the owner's four-episode `DOOM.WAD`, found as `doom.wad`, runs as **registered**; named `doomu.wad` it runs as retail. The Rust app mirrors this (crates/doom-play/src/main.rs:236-248: lower-cased name; `doom1.wad` shareware, `doomu.wad` retail, anything else registered; `GameMode` in crates/doom-render/src/r_data.rs:361-369), and the M5c runner hard-codes registered (tools/reference/m5c_driver.c:1903).
- Along the Episode 1 path the modes differ only in:
  1. (read) **The finale's art screen**: retail draws `CREDIT`, every other Doom 1 mode `HELP2` (f_finale.c:714-720). (observed) The owner's `DOOM.WAD` has `CREDIT` and **does not have `HELP2`** (68 of the 69 finale, music, font and palette names the C needs are present; `HELP2` is the one missing). (inferred, not run) Run as `doom.wad`, the genuine engine would stop with `W_GetNumForName: HELP2 not found!` (w_wad.c:399-409) at the first display of finale stage 1, about 1,571 tics after E1M8 ends.
  2. (read) The switch list: registered has episodes 1–2, retail only episode 1 (p_switch.c:113-119). (observed) None of the nine Episode 1 maps of the owner's IWAD names an episode-2 switch texture (0 of 9), so this changes nothing on those maps.
  3. The title loop, banners and menus: not in M6a; they move to a later milestone, to be named by the owner (not optional). The episode clamp, the intermission (same lumps and code for episode 1), the par times and the finale text are the same in registered and retail.

### 1.6 The intermission (single player, episode 1)

**Start.** (read) `WI_Start` (wi_stuff.c:1838-1850) runs `WI_initVariables` (1796-1836), `WI_loadData` (1538-1706: draws the background into a back screen from inside the ticker and caches every picture) and `WI_initStats` (1318-1328: `sp_state = 1`, counters at -1, `cnt_pause = TICRATE`, `WI_initAnimatedBack()`).

**State machine and timing.** (read) `WI_Ticker` (1503-1536): `bcnt++`; at `bcnt == 1` `S_ChangeMusic(mus_inter)`; `WI_checkForAccelerate()`; the state's update. `WI_updateStats` (1330-1434) counts kills, items, secrets (+2 per tic, `sfx_pistol` every fourth tic, `sfx_barexp` at each target) and time and par (+3 per tic); odd states pause 35 tics; a press before state 10 jumps every counter to its end with one `sfx_barexp`. **State 10 waits for a press forever** (1413-1424): a press there gives `sfx_sgcock` and `WI_initShowNextLoc` (140 tics, the pointer blinking, 752-769); a press or the end of the 140 tics gives `WI_initNoState` (10 tics), whose last tic calls `G_WorldDone` (730-747).

- (observed, scratch probe of the genuine `wi_stuff.c`) With no press, state 10 is reached on intermission tic `175 + n2 + n4 + n6 + n8` (n2 = ⌈(K+1)/2⌉ for kill percentage K, likewise items and secrets, n8 = ⌈(max(T,P)+1)/3⌉ for time and par in seconds): tic 281 for 50/75/50 % and 45 s against 30 s; tic 395 for 100 % and 200 s. No `G_WorldDone` in 20,000 tics without a press. A press on tic 400 → NoState at 540, `G_WorldDone` at 550. (inferred) The shortest intermission is presses on tics 1, 3 and 5 with releases between: `G_WorldDone` on tic 15, the next map's tic 0 on tic 16.

**Buttons.** (read) `WI_checkForAccelerate` (1470-1498) counts only a rising edge of `BT_ATTACK` or `BT_USE` against the player's own `attackdown` and `usedown` latches, which start as the level left them: `attackdown` written by `A_WeaponReady` (p_pspr.c:317-327), `usedown` by `P_PlayerThink` (p_user.c:322-331). (inferred) A use press that hit the exit switch leaves `usedown` set, so holding use into the intermission skips nothing; release and press again does. (observed) A fire button held from the level until intermission tic 499 and pressed again at 520: no acceleration before 520. (read, observed) The original's pause command is 129 = `BT_SPECIAL | BTS_PAUSE` (d_event.h:80, 91), whose bit 0 is `BT_ATTACK`, so in the intermission it counts as a fire press. Movement, turning and weapon bits are ignored.

**Sounds.** (read) `sfx_pistol`, `sfx_barexp` and `sfx_sgcock`, all with no origin; each start draws `M_Random` once for its pitch (s_sound.c:327-346); a start with no origin cuts a still-playing one with no origin (471-483). `sfx_slop` and `sfx_pldeth` are in the multiplayer paths only. **The level's channels are not stopped at completion**: only `S_Start` at the next load stops them (s_sound.c:209-211), and `S_UpdateSounds` keeps running each frame with the frozen listener (d_main.c:392) while the old mobjs stay valid memory until `Z_FreeTags` (p_setup.c:618). (inferred) So the exit switch's sound and anything else still playing carry into the start of the intermission. (observed) Per intermission: 26 pistols, 4 barexps, 1 sgcock for 50/75/50 % and 45 s; 56, 4, 1 for 100 % and 200 s; skipped on tic 50: 4, 1, 1.

**Random draws.** (read) No `P_Random` in `wi_stuff.c`, `f_finale.c` or `f_wipe.c`. Episode 1's animation table has 10 entries, all `ANIM_ALWAYS` (wi_stuff.c:226-238), so each `WI_initAnimatedBack` draws `M_Random` exactly 10 times (523); it runs at `WI_initStats` and at `WI_initShowNextLoc`, and single player always passes through both: **20 draws per intermission** (observed: 10 at `WI_Start`, 10 on the sgcock tic, in every scenario), plus one per sound start, plus the two wipes ([§1.11](#111-the-screen-wipe)). `ST_Ticker`'s per-tic draw does not happen outside a level. (observed) Drawing every tic changes nothing in the tic, state, sound or draw log.

**Graphics.** (read, observed) `WI_loadData` caches **79** names through `W_CacheLumpName`, so every one must exist even if single player never draws it: `WIMAP0`, `WILV00`–`WILV08`, `WIURH0`–`WIURH1`, `WISPLAT`, the 30 animation frames `WIA0jjii`, `WIMINUS`, `WINUM0`–`WINUM9`, `WIPCNT`, `WIF`, `WIENTER`, `WIOSTK`, `WIOSTS`, `WISCRT2`, `WIOSTI`, `WIFRGS`, `WICOLON`, `WITIME`, `WISUCKS`, `WIPAR`, `WIKILRS`, `WIVCTMS`, `WIMSTT`, `STFST01`, `STFDEAD0`, `STPB0`–`STPB3`, `WIBP1`–`WIBP4`. All 79 are in the owner's IWAD. Single player draws the background, "Finished", the three percentages, time, par (episodes 1–3 only, 1462-1466) and "sucks" above 61:59, then the splats, the pointer and "Entering".

- (read) **The animated background is never drawn in this build**: `WI_drawAnimatedBack` begins `if (commercial) return;` (wi_stuff.c:588), and `commercial` is the enum constant 2 (doomdef.h:40-42), so it always returns. The update (537, correctly `gamemode == commercial`) and its `M_Random` draws still run.
- (read, observed) **The first "you are here" frame of a process differs.** `snl_pointeron` is a file static (749), written by `WI_updateShowNextLoc` (768) and set true by `WI_drawNoState` (814). On the frame after the sgcock tic no ShowNextLoc update has run yet, so the pointer shows whatever the previous intermission left: hidden in the first intermission of a process, shown in every later one.

**What `G_Ticker` and `D_Display` do in the intermission.** (read) `G_Ticker`: the reborn loop, the `gameaction` loop, the command copy and the demo, the special buttons, then only `WI_Ticker` (g_game.c:605-748); no `P_Ticker`, `ST_Ticker` or `HU_Ticker`. `D_Display`: `WI_Drawer` (d_main.c:250-252) and `I_SetPalette(PLAYPAL)` once on entering a state other than the level (274-275).

### 1.7 Music, and what its absence changes

- (read) `S_ChangeMusic` (s_sound.c:649-686) returns at once if that music is already playing (665-666); otherwise it looks up `d_<name>` with `W_GetNumForName`, **which stops the engine if the lump is missing** (671-676), once per music (the number is cached), and calls `I_RegisterSong` and `I_PlaySong`, which in linuxdoom do nothing that any game code reads (i_sound.c:841-881; `musicdies` is read only by `I_QrySongPlaying`, whose one call is commented out, s_sound.c:608-612).
- Changes happen at every load (`D_E1M1`–`D_E1M9`), on the intermission's first tic (`D_INTER`, wi_stuff.c:1508-1515) and at `F_StartFinale` (`D_VICTOR`, f_finale.c:114). A reborn on the same map keeps the same music, so no lookup happens.
- (inferred) **Silence changes nothing else**: no state, timing or random draw depends on music. But the lumps must exist, or the genuine engine stops, and each change takes one zone block (layout only).

### 1.8 After E1M8: the finale

- (read) `G_DoCompleted` on map 8 finishes the players and returns with `ga_victory` (1026-1038); the same `G_Ticker` calls `F_StartFinale` (f_finale.c:96-191: `gamestate = GS_FINALE`, `S_ChangeMusic(mus_victor)`, the flat `FLOOR4_8` and the episode-1 text, stage 0, count 0), and the first `F_Ticker` runs in that same `G_Ticker` (g_game.c:740-742).
- (read) E1M8 ends through an exit line or switch or sector special 11, which removes god mode, damages 20 every 32 tics and exits when health is 10 or less (p_spec.c:1054-1063); invulnerability still blocks its damage (p_inter.c:847-852).
- (read, observed) `F_Ticker` (207-249): the button skip is Doom II only (212-227); `finalecount++`; stage 1 begins when `finalecount > strlen(text)*3 + 250` (241-248). The episode-1 text is 440 characters, so **stage 1 begins on finale tic 1,571**, counting the `G_Ticker` that ran `G_DoCompleted` as 1 (observed with the genuine `f_finale.c`), with a forced wipe (245). The text crawl draws `(finalecount-10)/3` characters (261-327; 69 characters of the episode-1 text fall outside the font and are drawn as 4-pixel gaps).
- (read) Stage 1 draws `CREDIT` (retail) or `HELP2` (other Doom 1 modes) every frame, forever ([§1.5](#15-game-modes-and-how-the-owners-doomwad-is-identified)).
- (read) **The finale never ends by itself in Doom 1**: no Doom 1 path writes `gameaction` or `gamestate` (207-249), and `F_Responder` serves only the Doom II cast (195-201). The way out is the menu, which is not in M6a (the menus move to a later milestone, to be named by the owner; not optional). (inferred) A session therefore ends in the finale only when the player quits; a recording keeps one command per tic for as long as it runs.
- (read, observed) The episode-1 finale starts no sound and draws no random number (0 `M_Random` draws in 4,000 finale tics apart from the wipes).

### 1.9 Death and reborn in single player

- (read) `P_DeathThink` (p_user.c:182-229): the view sinks to 6 units above the floor, turns toward the attacker, and **`if (cmd.buttons & BT_USE) playerstate = PST_REBORN`** (227-228), with no edge test and no delay: a use held at death rebirths on the next tic's player think.
- (read) The next tic: the reborn loop calls `G_DoReborn`, which in single player sets `gameaction = ga_loadlevel` (g_game.c:924-932); `G_DoLoadLevel` runs in the same `G_Ticker`; `P_SpawnPlayer` calls `G_PlayerReborn`; the reloaded map's tic 0 runs with this tic's command; the wipe is forced (472-473).
- (read) Reset: the whole map from its lumps, `leveltime`, the totals and counts, the player (pistol start; cheats and `didsecret` cleared; both latches set), every sound channel (`S_Start`).
- (read) **Process state that survives**: both random indices; `gametic`; `secretexit`; the skill and the `G_InitNew` table edits; the status bar statics `ST_initData` leaves alone (`st_facecount`, st_stuff.c:384; the face `priority` and `lastattackdown`, 757-758) while `ST_initData` resets the clock, face index, palette (-1), old health and old weapons (1249-1278); the HUD's `message_counter` (hu_stuff.c:103); `turnheld` and the double-click state; the level music (same number, so not restarted); the sky.
- (read) **Reborn wins over a same-tic exit** (inferred by the writer, not in either report, not run): the reborn loop runs before the `gameaction` loop and `G_DoReborn` overwrites `gameaction` (g_game.c:611-623, 929-931). A dead player's sliding body can still cross a walk exit line (`P_TryMove` calls `P_CrossSpecialLine` for any moving thing, p_map.c:499-512, which tests only `thing->player`, not its health, p_spec.c:502-503, before line 52, 677-680), and `P_PlayerThink` (which sets `PST_REBORN`) runs before the thinkers (p_tick.c:150-152). If both happen in one tic, the map is reloaded and the exit is lost. The Rust simulation checks the exit flag first (crates/doom-sim/src/lib.rs:1341-1346), which today only decides which end screen the app shows; a session must use the C order. **Observed since** (Phase 2 steps 1 and 2): on the committed synthetic corridor `tests/reference/m6a_wads/rbx.wad`, item `USEAT131` (use pressed on the tic the dead body crosses the W1 exit) ends tic 131 with `gameaction` 6 and `PST_REBORN` and reloads the map on tic 132 with no intermission; the controls `USEAT130` (use a tic earlier: the reload comes first) and `USEAT132` (use a tic later: the exit stands and the press is a tally press), and `PROBE` (no use: the dead player's crossing takes the exit), behave as the C order says; the streams are goldens of the genuine engine (`tests/reference/m6a_results.json`, witnesses of item 12).

### 1.10 Demos across levels

- (read) `G_Ticker` reads or writes one command per tic for every player in game, in **every** game state, before the game-state switch (g_game.c:656-670). The 13-byte header is read once by `G_DoPlayDemo`, which calls `G_InitNew` (1582-1620). There is no per-level marker. (inferred, observed in the prototype) **One demo can carry a whole episode** through the genuine engine, intermission and finale tics included.
- (read) Outside a level, commands do only this: in the intermission, fire and use rising edges; in the Doom 1 finale, nothing; everywhere, the pause and save specials and the turbo message (g_game.c:672-679, 698-724).
- (read) `G_CheckDemoStatus` is called only at the playback marker, on the recording `q` key, when the recording buffer is full and on quit (g_game.c:1493-1519, i_system.c:176-177); despite its comment it is not called at a death or a level's end.
- (read, inferred) **Vanilla's recording cap**: `-record` allocates 0x20000 bytes unless `-maxdemo` is given (g_game.c:1538-1541) and stops when the pointer passes the end minus 16 (1515-1519): with the 13-byte header that is **32,761 tics (15 min 36 s)**. Playback reads the whole lump with no limit (1588). This bounds only a vanilla `-record`, not the Rust recorder or genuine playback.
- (read) Desync risks across a level change: none from the random indices (never reset on either side, and no `P_Random` outside a level); `M_Random` depends on the number and place of wipes, and `-nodraw` makes none (d_main.c:209-210); the game mode is not in the header, so playback must use the same IWAD file name; the menu pauses `P_Ticker` while recording but not during playback (p_tick.c:139-145); the menus are not in M6a (a later milestone, to be named by the owner), but this is a reason the app must never let a menu stop the simulation while recording.

### 1.11 The screen wipe

- (read) `D_Display` wipes when `gamestate != wipegamestate` (d_main.c:223-227): it captures the last frame, draws the new one, then runs the melt in a busy loop on `I_GetTime` during which no game tic runs (329-344). The melt's 320 `M_Random` draws (f_wipe.c:159-166) and its 34–43 ticks are in [M5c §1.4.3 and §1.4.4](M5C_REFERENCE_INVESTIGATION.md#14-m_random-accounting-and-the-level-start-wipe), re-read and not re-run here.
- (read) Transitions that wipe on this path: level → intermission; intermission → level; level → finale; the finale's text → art screen (`wipegamestate = -1`, f_finale.c:245); the reborn reload (forced, g_game.c:472-473); the session start (M5c T6). Nothing wipes inside the intermission.
- (inferred) The normal route E1M1…E1M8 makes 1 + 7×2 + 1 + 1 = **17 wipes**; with the secret route 19; plus one per reborn. Each moves the `M_Random` index by 64 (320 mod 256). (observed) The prototype's synthetic run made 10 wipes, each of exactly 320 draws.
- (inferred) Under M5c's declared one-tic-per-display schedule, each wipe's draws fall after the frame of the first tic in the new state: after intermission tic 1 (after its 10 animation draws); after the load tic of a new or reloaded map (after tic 0's sound and status bar draws, and after a chainsaw `sfx_sawup` at the load); after finale tic 1; after finale tic 1,571. Two state changes cannot fall between two displays on this path under that schedule.
- **A side note on M5c, read and not run.** M5c §1.4.4 item 8 says no `I_SetPalette` has run when the first melt starts, reasoning from `st_palette`'s static initial value 0. But `ST_Start` → `ST_initData` sets `st_palette = -1` (st_stuff.c:1266) before the first frame, and `ST_Drawer` calls `ST_doPaletteStuff` unconditionally (st_stuff.c:1115), which calls `I_SetPalette` when the palette differs (1045-1050). The first level display after tic 0 (`gametic` is then 1, so d_main.c:238-239 do not skip it) therefore sets palette 0 before `wipe_EndScreen` (d_main.c:327) and the melt. The start screen is still captured before any palette is set (d_main.c:226). This matters only for the melt's colours, which no claim covers; it is recorded here and the M5c note is left as written.

### 1.12 Skills, sized

Every test of `gameskill`, `respawnmonsters` and `fastparm` outside the menus (read):

| Site | What it does | sk_baby (0) | sk_easy (1) | sk_nightmare (4) |
| --- | --- | --- | --- | --- |
| p_mobj.c:743-751 | spawn bit: baby and easy 1, medium 2, hard and nightmare 4 | x | x | x |
| p_inter.c:95-101 | `P_GiveAmmo`: double ammo | x | | x |
| p_inter.c:799-800 | player damage halved, before the thrust, the sector-11 clamp and armour | x | | |
| g_game.c:1379-1380, 1416-1436 | skill clamp; `respawnmonsters`; fast monsters: halve the tics of 13 demon states, raise three missile speeds; a process-global table edit keyed on the previous skill | | | x |
| p_mobj.c:503-504 | `reactiontime` left 0 | | | x |
| p_mobj.c:452-470, 357-407 | corpse respawn: after 420 tics, every 32 tics one `P_Random`; `P_NightmareRespawn`: two fogs with `sfx_telept`, three `P_SpawnMobj` (three `P_Random`), kills can pass 100 % | | | x |
| p_enemy.c:719-720, 738-742 | `A_Chase`: no new direction after an attack; missile allowed with `movecount` | | | x |

- **sk_easy** differs from medium on this path only in the spawn bit: 1 site. **sk_baby**: 3 sites, no new random draws, no new state; the halved damage also halves knockback. **Nightmare**: 8 sites plus `P_NightmareRespawn`, new `P_Random` draws, new sound starts, a global table edit, and missile speeds that matter on Episode 1 (imp fireballs; baron balls on E1M8). The demo header carries the skill and the two parameters (g_game.c:1556, 1560-1561).

### 1.13 The original's pause

(read) `KEY_PAUSE` makes the next command 129 (g_game.c:426-430, 561-565), which a demo carries as an ordinary command byte. `G_Ticker` toggles `paused` for it in any game state (698-712); while paused `P_Ticker` returns at once (p_tick.c:135-136) but `ST_Ticker` with its `M_Random` draw, `WI_Ticker` and `F_Ticker` keep running (g_game.c:727-747). In the intermission 129 is also a fire press. Not in M6a (the original's pause lands in M6b, U13): the Rust demo writer already refuses `BT_SPECIAL` (crates/doom-sim/src/demo.rs:108-112), and the app's own pause (M5b S12) is not a command.

### 1.14 Findings that contradict or go beyond the brief

Each has its decision in [§5](#5-owner-decisions-u1u15-with-recommendations).

1. **No intermission after E1M8.** E1M8 goes straight to the text crawl with no statistics (§1.4). U8.
2. **The intermission and the finale never end by themselves.** The tally waits for a fire or use press, a rising edge (§1.6); the finale waits for the menu (§1.8). A demo must hold the presses; a session ends in the finale only when the player quits. U5, U7.
3. **The owner's DOOM.WAD runs as registered, and the registered finale's art screen needs `HELP2`, which his IWAD lacks.** The genuine engine would stop about 1,571 tics into the finale (inferred, not run); named `doomu.wad` (retail) it draws `CREDIT`, which is present (§1.5). A game-mode decision. U4.
4. **Missing music lumps: the genuine engine aborts, the Rust app plays on.** The genuine runner stops at load when `D_E1M1` is missing (observed: `W_GetNumForName: d_e1m1 not found!`); `GameData::load_with_sound` plays the same synthetic map to its exit (observed). This divergence exists today in M5c's app load; `D_INTER` and `D_VICTOR` join the class. It is documented here with evidence and not patched in this phase ([§2.7](#27-frozen-wires-at-the-boundary-what-the-first-comparison-found)). U10.
5. **The intermission's background animation is never drawn** (wi_stuff.c:588) but its 20 random draws happen; **the first "you are here" frame** of a process hides the pointer, later ones show it (§1.6). U8.
6. **`doom-render`'s `begin_level` resets the `M_Random` index to 0**, which is right for a new game and wrong for a level change or a reborn (§1.3; [§2.4](#24-frames-sound-the-m_random-index-and-the-wipe)). U3.
7. **The saved display and sound states fix per-level things**: `DDSP` v2 accepts a set wipe flag only at simulation tic 0 or 1, and `DSND` v1 fixes the map and ties its tic count to the simulation's (§2.4). U3.
8. **Per-map real demos cannot be spliced into an episode**: with E1M1's carried player and random index, E1M2's committed walk desynchronised and died (observed, [§3.3](#33-real-maps-why-spliced-demos-do-not-reach-their-exits)). Real-map episode runs need continuous recording. U14.
9. **A carried chainsaw makes `sfx_sawup` at load**, before tic 0: a sound row and an `M_Random` draw no fresh start has (§1.1 item 8). U8.
10. **Level sounds continue into the intermission** with a frozen listener (§1.6). U8.
11. **Vanilla's `-record` stops at 32,761 tics** by default (§1.10); the Rust recorder's cap is S21's. U7.
12. **More wipes**: two per intermission, one into the finale, one inside it, one per reborn, each moving `M_Random` by 64 (§1.11). U9.
13. **The palette before the first melt** (M5c §1.4.4 item 8) looks wrong on reading; it matters only for the melt picture (§1.11). Recorded; U9.
14. **Reborn beats a same-tic exit** in the C order (inferred by the writer, §1.9). U8.

### 1.15 What was run and what was not

- **C probe** (`~/scratch/m6a/r1/wiprobe/`): the genuine `wi_stuff.c` (included unchanged to read its statics), `f_finale.c` and `m_random.c` linked with stubbed drawing, WAD, zone and sound: `gcc -std=gnu89 -w -c -DNORMALUNIX -DLINUX -I$SRC $SRC/m_random.c $SRC/f_finale.c; gcc -std=gnu89 -w -I$SRC drv.c m_random.o f_finale.o -o drv && ./drv > out.txt && DRAW=1 ./drv > out_draw.txt` (exit 0): seven intermission scenarios and one episode-1 finale.
- **Text** (`~/scratch/m6a/r1/e1text/`): `strlen(E1TEXT)` = 440; 69 characters outside the font.
- **IWAD counts** (read-only scan of the owner's `DOOM.WAD`): 2,306 lumps; 79 of 79 intermission names; 68 of 69 finale, music, font and palette names (missing: `HELP2`); E1M1–E1M9 present; 0 of 9 maps use an episode-2 switch texture.
- The harness prototype and its runs are in [§3](#3-harness-design-evidence-first).
- **Citations re-checked by the writer of this note** against the source: g_game.c:445-497, 605-650, 812-832, 920-933, 1002-1018, 1030-1066, 1084-1120, 1145-1180, 1412-1415, 1512-1520, 1536-1541, 1586-1589; p_setup.c:592-618, 646; m_random.c:50-72; p_pspr.c:138-154; p_mobj.c:504-507, 655-662, 740-752; p_inter.c:93-102, 797-801, 836-843; p_spec.c:492-540, 670-682, 1054-1063; p_map.c:495-515; p_user.c:225-229; wi_stuff.c:224-240, 296-300, 518-526, 584-590, 745-770, 786-804, 810-816, 1410-1425, 1460-1467, 1474-1498, 1503-1518, 1822-1836; f_finale.c:52-57, 108-121, 238-249, 708-720; d_main.c:207-211, 220-262, 270-276, 390-393, 686-710; s_sound.c:205-214, 606-613, 649-686; i_sound.c:838-846, 884-890; st_stuff.c:384, 1040-1052, 1105-1122, 1262-1268, 1440-1456; hu_stuff.c:103; d_event.h:78-92; p_switch.c:113-119; and the Rust lines of §2.1–§2.3. **Corrected**: the finale's art switch is f_finale.c:714-720 (one report gave 712-718, another 714-720); state 10 is wi_stuff.c:1413-1424 (one report gave 1408-1416); `S_ChangeMusic(mus_victor)` is f_finale.c:114 (one report gave 113); and `attackdown` does exist in the Rust simulation, as `attack_down` in the weapon state (crates/doom-sim/src/weapons.rs:235), canonical in the capture (crates/doom-sim/src/diagnostic.rs:2612-2649), though one report had not found it.
- **Not run**: the genuine engine on E1M8 or the finale with the owner's IWAD, so the `HELP2` stop stays inferred; the retail mode; the genuine mixer across a boundary; the reborn-beats-exit case; any timing or memory measurement.

## 2. What the Rust code does today

All (read) at master `d637f7d` unless marked.

### 2.1 Loading a map

- `GameData::load` (crates/doom-game/src/app/mod.rs:348-350) is the M5b configuration (wire 24, no sound, the wipe-free display contract); `GameData::load_with_sound` (352-367) is wire 25 with sound, the playable default that `doom-play` uses (crates/doom-play/src/main.rs:373).
- `load_inner` (mod.rs:369-447) admits skills 2 and 3 and maps 1–9 (376-383), loads **one** map, `E1M<map>`, builds a sound session whose map number is fixed at load (`SoundConfig::fresh(map, ..)`, 415-421), constructs the wire-25 simulation and runs the three setup calls (439-444). The result is the tic-0 simulation that every session and restart clones (704-725).
- The simulation's random state starts at both indices 0 (crates/doom-sim/src/lib.rs:817); the map is constant for the run (lib.rs:638-642); `initialize_player_weapons` refuses unless at tic 0 with a fresh player at full health (crates/doom-sim/src/player_weapons.rs:296-302); `WeaponInventory` holds only the ready weapon, the owned bits and four ammunition counts (crates/doom-sim/src/weapons.rs:165-181). **No constructor accepts a carried player or a carried `P_Random` index.**

### 2.2 The exit, the death and the reborn in the simulation

- `SimError::LevelExited` and `SimError::PlayerReborn` (lib.rs:318-325) are refusals of the next tic, checked before anything else (lib.rs:1340-1346), in the order exit then reborn. The exit tic itself is committed and compared.
- The canonical fields include `level.exit`, `level.secret_exit`, `level.map`, the totals, the player's health, armour, weapons, backpack, cards, powers, counts, `use_down`, `attack_down` (weapon state) and both random indices (crates/doom-sim/src/snapshot.rs:850-875). There is no `didsecret`, and the simulation has one tic counter, which is `leveltime` and not `gametic` once a session has passed one level.

### 2.3 The app: endings, restart and the recorder

- `Ending` (mod.rs:532-551) is `Exit { secret }`, `Died`, `Refused { tic, witness }` or `TicCap`; on an `advance` error the recorded command is dropped and the session ends (993-996). `App::end_screen` (1048-1100) shows "LEVEL COMPLETE", "LEVEL COMPLETE: SECRET EXIT", "EPISODE 1 COMPLETE" on map 8, or "YOU DIED", then "R: RESTART Q: QUIT". COMPATIBILITY says "There is no intermission, tally or next level".
- **S11's restart is a new game** (`App::begin`, mod.rs:704-737): a fresh tic-0 simulation, fresh input, recorder and display (both random indices 0), a new file name.
- **The recorder is one map per demo**: `Recorder::new(map, skill, cap)` (crates/doom-game/src/app/recorder.rs:61-68), written with `write_demo_episode1_v2(self.map, ..)` (118-119); `parse_demo_episode1_v2` requires the header's map to equal the caller's (crates/doom-sim/src/demo.rs:179-200). The version-110 format itself spans levels, so a session demo needs no new file format; the recorder must keep recording through intermission and finale tics, and a session parser must stop assuming one map.

### 2.4 Frames, sound, the M_Random index and the wipe

- Per tic with sound: the simulation's tic, then the sound session (records, `S_UpdateSounds`, the mixer buffer due), then the display's observation (sound draws, `ST_Ticker`'s draw, `HU_Ticker`), then the frame; sound and display run on staged copies and commit together (crates/doom-game/src/app/frames.rs:12-30, 300-345).
- **The `M_Random` index** lives in `doom-render`'s status bar state (crates/doom-render/src/st_stuff.rs:180-184), saved in `DDSP`; the sound session takes it in and hands it back (crates/doom-sound/src/session.rs:242-273) and `DSND` does not save it.
- `StatusBarState::level_start` sets the index to 0 (st_stuff.rs:247-253) and `Display::begin_level` calls it (crates/doom-render/src/d_display.rs:325-341): `M_ClearRandom` + `ST_Start` + `HU_Start` + a forced wipe. Right for a new game; **wrong for a level change or a reborn**, where the C clears nothing.
- **The wipe** (M5c T6): 320 draws after the frame on which the wipe flag is set (d_display.rs:48-63, 531-541); the flag is set only by `Display::new` and `begin_level`. **`DDSP` v2's restore rule accepts the flag only as set at simulation tic 0, either value at tic 1, and clear from tic 2** (d_display.rs:787-800).
- **`DSND` v1** fixes the map in its configuration and requires that the simulation has completed exactly the state's tics (STATE_FORMAT, the `DSND` section; `SoundConfig::fresh`, crates/doom-sound/src/session.rs:49-56). `SoundLogic::level_start` (crates/doom-sound/src/logic.rs:475-476) already is `S_Start`'s channel loop.

### 2.5 What could be carried today

| State | Lives in | Carried today? |
| --- | --- | --- |
| `P_Random` index | `doom-sim` (`rng.gameplay`) | no constructor input; 0 at construction |
| health, armour, weapons, ammunition, `maxammo`, backpack, `deltaviewheight` | `doom-sim` player state | no; weapons only through the default-only setup call |
| `usedown`, `attackdown` | `doom-sim` (`use_down`; weapon state's `attack_down`) | no |
| `didsecret`, `gametic`, the intermission and finale | nowhere | no |
| `M_Random` index | `doom-render` status bar state | `begin_level` resets it |
| sound channels and mixer | `doom-sound` | map fixed per session; tic tied to the simulation's |
| map number | `doom-sim` (constant), `GameData` (one map) | no |

### 2.6 What the existing evidence covers

(read; counts from the committed results) **The 61 real runs** (`tests/reference/m4b_real_results.json`; 45 on skill 2, 16 on skill 3): 12 end at an exit, 22 at a reborn, 27 at the demo's end; **none takes a secret exit**; the two E1M8 exits stop before `G_DoCompleted`. **Synthetic**: M4a has 30 items (3 exits, one of them a secret exit; 4 reborns); M4b 18 (1 exit); M5c's 64 items include one that exits and none that reaches a reborn. **No run, real or synthetic, C or Rust, crosses a level boundary**: nothing has exercised `G_DoCompleted`, `wi_stuff.c`, a carried start, the single-player reload, `f_finale.c`, a mid-session wipe or `gametic` running through intermission tics. `wi_stuff.c` and `f_finale.c` have never been compiled by a committed runner; the M5c runner stubs them to abort (tools/reference/m5c_stubs.c:128-138) and stops after the boundary tic (tools/reference/m5c_driver.c:183-200, 1961-1968).

### 2.7 Frozen wires at the boundary: what the first comparison found

- **Nothing wrong in a frozen wire.** (observed) Rust (`m5c_real_rows`, wire 25) against the genuine engine up to the boundary, with `tools/local/m5c_real_full.py` on the owner's IWAD (no `--write`, no `--check`): **E1M1 WALK**, 766 tics: 0 sound rows differing, 0 of 766 frames mismatched at screenblocks 10 and 0 of 766 at 11, 494 mixer buffers equal; **E1M2 M2DIE**, 999 tics through the reborn tic: 0, 0, 0, 618 buffers equal. Synthetic, the frozen runner against Rust: an exit, a death and reborn, a secret exit and an E1M8 exit: 125, 604, 125 and 125 frame rows, 88, 312, 88 and 88 sound rows, 0 differing. The forked prototype runner agreed with the frozen one on every row up to the boundary (WALK 3,181 frame and 889 sound rows; M2DIE 4,091 and 1,232) and reproduced WALK's committed sound digest.
- **The music-lump divergence is separate**: not a wire's state but a missing refusal at the app's load (§1.14 item 4). (observed) `crunner --wad multi.wad --wad m5b_gfx.wad --wad m5c_sfx.wad --wad S1.lmp --demo S1 --no-dump --render every --screenblocks 10` exited 134 with `W_GetNumForName: d_e1m1 not found!` (via p_setup.c:607 → s_sound.c:245, 672-676); `m5c_real_rows` on the same WADs played the 41 tics to `Exit` (exit 0). The requirement was noted in M5c ([§3.1.12](M5C_REFERENCE_INVESTIGATION.md#3112-lumps-the-runner-needs-and-the-interface-with-the-mixer-i-src-obs)), but no refusal exists in the Rust crates and COMPATIBILITY does not mention it. Every IWAD has these lumps, so no real run is affected.

## 3. Harness design, evidence first

### 3.1 Design of the genuine side

A **fork of the M5c runner** (new `m6a_*` files; the frozen ones untouched), importing `m5c_build.py` as a library as M5c imported M5b's, with `wi_stuff.c` and `f_finale.c` compiled whole and unmodified and the five abort stubs removed. What the prototype showed it needs (observed unless marked):

1. Run `G_Ticker` every tic and do not stop at the boundary; the genuine `gameaction` loop does the rest.
2. A `D_Display` mirror for the intermission (`WI_Drawer`) and the finale (`F_Drawer`), with the wipe and the palette set on entering a non-level state (d_main.c:222-344).
3. Level-change bookkeeping: the per-level driver tables must be rebuilt at each load. Detecting a load by the `lines` pointer **missed the same-map reborn reload** on the synthetic WAD (same zone address); use `levelstarttic` or a wrapper around `P_SetupLevel`.
4. Dumps and view rows only in a level; a session row per tic (game state and action, map, `leveltime`, `gametic`, `levelstarttic`, both random indices, draw counters, the carried player, `wminfo`, the finale's stage and count); a whole-frame digest for non-level frames.
5. `wi_stuff.c`'s state is file-static (the counters near wi_stuff.c:289-332, `sp_state` 1316, `snl_pointeron` 749, the animation table): reading it needs a **declared adapter**, a wrapper unit that `#include`s `wi_stuff.c` unchanged and adds accessors. The finale's `finalestage` and `finalecount` are ordinary globals (f_finale.c:52, 54).
6. The `M_Random` observer needs an intermission bucket: the prototype counted those draws as "other", which the M5c contract requires to be 0 (m5c_driver.c:907).
7. Synthetic music WADs need `D_INTER` and `D_VICTOR` besides `D_E1M1`–`D_E1M9` (observed: `W_GetNumForName: d_inter not found!` without them).
8. A game-mode switch (registered or retail) where the M5c driver hard-codes registered (m5c_driver.c:1903).
9. The genuine mixer harness takes its start list from the sound rows, which carry `gametic`, continuous across levels (observed); (inferred, not run) it should work across boundaries unchanged.

The Rust side: a session example like `crates/doom-game/examples/m5c_real_rows.rs` emitting the same row kinds across levels, and comparators for per-level captures, session rows, frames (level frames as today, whole frames in the intermission and finale), every sound row and every mixer buffer.

### 3.2 The prototype and what it showed

Files under `~/scratch/m6a/r2/`: `gen/proto_build.py` (imports `m5c_build`, applies M5c's adapters, adds the two files, writes the driver with ten exact-once patches of about 70 lines of C and the stubs without the five aborts), `gen/multi_wad.py` (a synthetic six-map IWAD from the committed room generator: one hall per map, exit switch 11, a secret-exit switch 51 on E1M3, a damage floor on E1M2; plus our own placeholder graphics for every intermission lump, `FLOOR4_8`, `HELP2`, `CREDIT`, `D_INTER`, `D_VICTOR`), `gen/splice.py`, `run.sh`, `runi.sh`. Builds (observed): `python3 gen/proto_build.py strict build` and the address variant, warnings only (`-Wformat-overflow` in wi_stuff.c:1579, 1613 and f_finale.c:692).

**Synthetic, across six maps** (observed): `./run.sh span SPAN --continue --states runs/span/states.txt` → exit 0, 588 tics, 4 loads, observer misses 0, the heap checked every tic.

| Segment | Tics | `P_Random` index | Notes |
| --- | --- | --- | --- |
| E1M1 | 0–40 | 2→2 | exit switch at tic 40 |
| intermission | 41–82 | 2 | a skip (barexp), sgcock; 10 + 10 animation draws |
| E1M2 | 83–288 | 5→9 | health 80 → 0; reborn at 288 |
| E1M2 reloaded | 289–334 | 12→13 | health back to 100; exit |
| intermission | 335–376 | 13 | |
| E1M3 | 377–422 | 15 | secret exit: last 2, next 8 |
| intermission | 423–464 | | |
| E1M9 | 465–510 | 17 | exit: last 8, next 3, `didsecret` set |
| intermission | 511–552 | | |
| E1M4 | 553–587 | 19 | |

- Ten wipes, each exactly 320 draws (tics 0, 41, 83, 289, 335, 377, 423, 465, 511, 553), 34–42 melt steps each under the runner's declared clock stub. The `P_Random` index never changed in an intermission and was not reset by a load or the reload; the `M_Random` index was not reset either (106 at E1M1's last tic → 180 on the intermission's first = +320 +10, mod 256). Each load and the new level's first `P_Ticker` share a `G_Ticker`; `gametic` ran through every intermission. Carried: ammunition 60 → 60 → 70 → 80 → 90 and health 60 into E1M3, E1M9 and E1M4; the reborn restored health 100.
- `IDLEINT` (no press for 400 intermission tics): the tally finished (sounds at 76, 112, 148, then 184–194) and the intermission waited at state 10.
- `FINALE` (starting on E1M8): exit at 40, finale from 41, stage 1 from tic 1,611 with a forced wipe; 3 wipes; no sound and no other `M_Random` draw in the finale; the run ended at the demo's end in the finale.

**Owner's IWAD** (observed; counts only; a read-only scratch copy): `RSPAN` = E1M1 WALK + a 41-tic intermission + E1M2 walk + an intermission + E1M3 walk + 200 idle tics (9,705 tics): `./runi.sh rspan RSPAN --continue` → exit 0, 5 loads, 9,664 level frames, 7 wipes of 320 draws; E1M1 0–765, intermission 766–806, E1M2 from 807. `M2DIEX` = E1M2's die script + 300 idle tics: reborn at 998, E1M2 reloaded at 999 (the `P_Random` index not reset; `M_Random` +320 +1), then 300 tics of the reloaded level.

**Death and reborn** (observed): with the M5c driver's stop removed (`--continue`) the genuine `G_DoReborn` → `ga_loadlevel` → `G_DoLoadLevel` path ran in the next `G_Ticker` (synthetic tic 289; IWAD tic 999) with a forced wipe, no abort and no observer miss.

**Sanitizer** (observed): the address build of the prototype with `ASAN_OPTIONS=detect_leaks=0` (leak detection disabled, so **not LeakSanitizer coverage**) on `SPAN`, `FINALE`, `IDLEINT` and `RSPAN`: 0 AddressSanitizer or UBSan reports; the synthetic frame, sound and state streams byte-identical to the strict build; on `RSPAN` the sound stream identical and exactly one frame different (E1M2 tic 1682) whose tracker-masked digest is equal (an undefined-read frame of the kind the existing tools compare masked). Overflow and 32-bit variants not built.

### 3.3 Real maps: why spliced demos do not reach their exits

(observed) In `RSPAN`, E1M2's committed walk started from E1M1's carried player and random index instead of a fresh start and **desynchronised**: the player died at E1M2 level tic 1,184 and was reborn four times; E1M2 was never exited. A 42-tic intermission (one tic more) also died. So **a real-map demo that reaches each exit must be recorded from continuous play.** (inferred) A spliced demo is still a valid comparison input: both engines replay the same commands and must agree; it just does not reach the exits it was written for. How the committed per-map scripts were authored was not re-examined in Phase 1, so whether that method can author a chain from carried starts is open.

### 3.4 Not run

The genuine mixer across a boundary; the level-capture encoder and comparator on levels after a boundary; overflow and 32-bit variants; screenblocks 11 in the prototype; E1M8 or the finale on the owner's IWAD; retail mode; anything in Rust past a boundary (nothing exists).

## 4. Rust design

### 4.1 Where the state lives

The frozen-wire rules: AGENTS.md ("Keep every historical profile, canonical schema and fixture expectation unchanged. New behavior requires an explicit configuration/schema choice"), COMPATIBILITY ([older policies never change](COMPATIBILITY.md#policies)), STATE_FORMAT (every earlier reader rejects a newer DSIM version; `DDSP` v1's bytes frozen). Wires 1–25 and their bytes do not change under any option.

State that exists only between levels in the C: `gamestate`, `gameaction`, `gametic`, `secretexit`, `wminfo`, the intermission's statics (state, `sp_state`, counters, `bcnt`, `acceleratestage`, `snl_pointeron`, the animation table's `ctr` and `nexttic`), the finale's stage and count, `D_Display`'s `oldgamestate` and `wipegamestate`, the `M_Random` index, the sound channels and the mixer, the carried player, `didsecret`.

- **Option A: a new wire 26 with the session inside `doom-sim`** (a DSIM42 with a session group; `advance` runs the whole `G_Ticker`). Problems (inferred): `doom-sim` has no WAD access, so a load inside `advance` needs all nine maps handed over at construction or a two-phase load request; the intermission draws `M_Random` and starts sounds, and that index is owned by `doom-render` and threaded through `doom-sound` (M5c T16 (1); ARCHITECTURE), so it would have to be split or moved into the simulation; the intermission's timing is separable from its drawing, but its sounds and draws are presentation-stream coupled. Cost: the largest crate (49,881 non-test lines) and every DSIM rule duplicated for 42.
- **Option B: the session above `doom-sim`, in the game layer, plus a carried start in `doom-sim`.** The simulation keeps simulating one level with wire-25 tic semantics; the game layer runs the tics in which no level runs (the completion tic, the intermission, the finale) and decides loads and reborns. The level tic after an exit or reborn is never handed to `advance` as a tic (the game layer gets `LevelExited` or `PlayerReborn` back with nothing mutated and runs the C's action instead), so those refusals stay exactly as they are. B needs one addition in `doom-sim`: **constructing a map with a carried player and a carried `P_Random` index** (and, for a reborn or a player dead at the exit, a fresh player with the carried index). Either:
  - **B1**: a **new wire 26** = wire 25's tic semantics plus the carried start, with **DSIM42 = DSIM41 plus a small start group** (how the level began: new game, carried or reborn; the carried inputs; the sound records made at load, such as the chainsaw's `sfx_sawup`), so a carried start is distinguishable in every capture; or
  - **B2**: a new constructor on wire 25 with no new wire: a DSIM41 capture of a carried start would then carry the same profile and context as a fresh one, wire 25 would gain behaviour, and the load-time sound record would have no place in DSIM41's per-tic records.
- **The game-layer state** under B: a new saved state (working name `DSES` v1) holding the session group (game state and action, episode and map, `gametic`, `secretexit`, `didsecret`, `wminfo`, the intermission machine, the finale's stage and count, the `D_Display` statics, the carried player between levels) and embedding the level's DSIM, `DSND` and `DDSP` states, as `DDSP` embeds its reference state. During the intermission **the finished level's simulation stays alive**, read-only, because `S_UpdateSounds` keeps using the frozen listener and origins until the next `S_Start` (d_main.c:392; p_setup.c:607, 618).

### 4.2 The session layer (recommended B1)

Per tic, in the C's order (g_game.c:605-748):

1. **Reborn first**: if the level's simulation is waiting for a reborn, reload the current map with a fresh player and the carried `P_Random` index, then run its tic 0 with this tic's command. The reborn is checked before the exit (§1.9), the reverse of the order in which `advance` reports them.
2. **Pending exit** (the simulation answered `LevelExited`): `G_DoCompleted` (`G_PlayerFinishLevel` on the carried player, `wminfo` from the snapshot's totals, counts and tic count); for map 8 `F_StartFinale`, otherwise `WI_Start`; then this tic's `WI_Ticker` or `F_Ticker` with this tic's command.
3. **Pending world-done** (set by the previous tic's intermission): `G_DoWorldDone`: load map `next+1` with the carried player (the latches from the intermission's last tic; a player dead at the exit gets a fresh player), then run its tic 0 with this tic's command. A map that fails admission at this point refuses the tic with the whole session state preserved (the existing per-map bounds; no work beyond one map load).
4. Otherwise, in a level `advance`; in the intermission `WI_Ticker`; in the finale `F_Ticker`.
5. Then sound (`S_UpdateSounds` with the level's or the frozen listener), the display's observation and the frame, as today.

`gametic` is the session's tic count; the simulation's tic count is `leveltime`.

### 4.3 `doom-sim`: wire 26

The tic semantics of wire 25 unchanged; a constructor taking a `CarriedStart` (health, armour points and type, owned weapons, ready weapon, ammunition, `maxammo`, backpack, `deltaviewheight`, `attackdown`, `usedown`, and the `P_Random` index), running `P_SpawnPlayer`'s and `P_SetupPsprites`'s order so that a carried chainsaw records `sfx_sawup` at load; the DSIM42 start group; refusals for a carried value the C could not hold. Proof that nothing else changes, as M5c did: a wire-26 capture of a fresh start, with the start group removed and the names set back to wire 25's, has the wire-25 capture's digest on every synthetic item and the 61 real runs; later levels are compared with the genuine engine only. An independent Python DSIM42 encoder (successor of `tools/reference/level_snapshot_m5c.py`).

### 4.4 The intermission and the finale

- **Logic** (`doom-game`, or a small module the game layer owns): `WI_Ticker`'s single-player path (the update half of `wi_stuff.c`, its 20 `M_Random` draws and three sounds) and `F_Ticker`'s Doom 1 path (a counter and a stage), process-level statics such as `snl_pointeron` kept in the session state (a fresh session starts with it false).
- **Drawing** (`doom-render`): `WI_Drawer`'s single-player screens (no animated background, as the C) and `F_Drawer`'s text crawl and art screen, from the same lumps through the existing patch drawing; the palette set to `PLAYPAL` 0 on entering either state. Frames compared as whole pictures. (inferred) Neither depends on the screen size.

### 4.5 The `M_Random` index and the wipe

The index stays where it is (M5c T16 (1)) and is carried by the session through non-level tics. `begin_level` gains an explicit variant that keeps the index (a level change, a reborn) next to the existing one (a new game). The wipe flag is set at every transition of §1.11 and its 320 draws come after the frame of the first tic in the new state (T6, extended). A new `DDSP` version holds what the non-level frames need and accepts the wipe flag at any level start or transition, keyed on the session's tics.

### 4.6 Sound across levels

The sound session keeps running across levels: at a load, `S_Start` (already `level_start`) and the new map number; in the intermission, starts with no origin and `S_UpdateSounds` against the frozen level; `gametic` continuous. A new `DSND` version records the map per level and ties its tic count to the session's `gametic`, not the simulation's tic.

### 4.7 Recorder and replay

One demo per session in the version-110 format: the header written once with the start map and skill, a command every tic in every game state, no `BT_SPECIAL`. A session parser that reads the header map as the start and does not stop at a boundary; the single-level parser, its tests and its files unchanged. Replay runs the same session layer and stops only at a refusal or the demo's end.

### 4.8 Refusals

Under the session configuration only: a missing music lump at the moment the genuine engine would stop (U10); a missing finale art lump at the first stage-1 frame (U4); a map that fails admission at its load tic (§4.2); a `BT_SPECIAL` command in a replayed demo. Each preserves the complete state; sound and display refusals follow M5c's second-set rule 2 (the simulation's tic stays committed, sound and display stay at the last good tic).

### 4.9 Bounds

Per tic, at most one map load, bounded by the existing per-map admission limits; the intermission's work is constant (10 animations, at most two sound starts per tic); the finale's is constant; the session state is a fixed-size group plus one level's state (two during the intermission, the finished one read-only). No clone of the whole simulation per tic. The session length stays bounded by S21's tic cap.

### 4.10 `doom-play`

The session is the default: start map `--map 1` (any of 1–9, continuing from there as vanilla's warp does), skills as U12 rules; no end screen at an exit or a death; S11's restart from the app's pause screen (U6); the session ends when the player quits or at a refusal or the tic cap, with the exit line naming the map or the finale reached.

## 5. Owner decisions (U1–U15) with recommendations

Each decision is tested against four questions: (a) is it consistent with prior owner decisions (E1–E9, F1–F11, G1–G13, H1–H12, I1–I13, P1–P9, R1–R16, S1–S23, T1–T16) and with the M6a brief? (b) does it admit no silent divergence? (c) does it add no new kind of fidelity gap beyond a declared refusal or a declared presentation contract? (d) are stores and work bounded? "Yes" to all four is written "(a)–(d) yes".

The findings of [§1.14](#114-findings-that-contradict-or-go-beyond-the-brief) that go beyond or against the brief each have a decision below: no intermission after E1M8 (U8); the intermission and finale never end (U5, U7); the game mode and the missing `HELP2` (U4); the missing-music divergence (U10); the undrawn animation and the first pointer frame (U8); the `M_Random` reset in `begin_level` and the per-level saved-state rules (U3); spliced demos (U14); the chainsaw at load and the level sounds in the intermission (U8); vanilla's recording cap (U7); more wipes and the palette side note (U9).

### Scope

**U1. Scope.** In: one single-player Episode 1 session over E1M1–E1M9 on skills 2 and 3 (plus what U12 decides): `G_DoCompleted` and `G_PlayerFinishLevel`; the single-player intermission of `wi_stuff.c`; `G_WorldDone`, `G_DoWorldDone` and the carried start; the single-player reborn reload; the E1M8 victory and the Doom 1 finale (text and art screen); every wipe's draws; sound across levels; one demo spanning the session. Not in M6a, and none of it optional (each moves to a later milestone, to be named by the owner, except the original's pause and nightmare, which are M6b): menus (so also the finale's only exit), the automap, saving and loading, music (only its lumps' presence matters here, U10), episodes 2–4, the original's pause command (never recorded; refused in a replayed demo until then), the deathmatch and network paths of `wi_stuff.c`, nightmare unless U13 says otherwise, the melt picture (U9). (a)–(d) yes. **Recommendation: as stated.**

**Ruled (owner, 2026-11-21):** Scope as stated, plus the melt (U9) and both easy skills (U12). The original's pause command and nightmare are M6b (U13); the other items listed above as not in M6a each move to a later milestone, to be named by the owner.

### Where the state lives

**U2. Where the session state lives.** Facts in [§4.1](#41-where-the-state-lives). Options: **A**, a wire 26 with the whole session inside `doom-sim` (DSIM42 with a session group); **B1**, the session in the game layer as a new saved state (`DSES` v1) plus a wire 26 in `doom-sim` that is wire 25 with a carried start (DSIM42 = DSIM41 plus a start group); **B2**, as B1 but the carried start as a new constructor on wire 25, with no new wire.

(a) A moves the `M_Random` index's draws and the intermission's sounds into the simulation, against T16 (1) and the separation of simulation and presentation; B1 and B2 keep them where M5c put them. B2 gives wire 25 a behaviour it did not have, which "new behaviour requires an explicit configuration or schema choice" rules out in spirit, and leaves the load-time sound record with no place. (b) yes for all three once proven against the genuine engine. (c) yes. (d) A needs nine maps at construction or a load request inside `advance`; B1 and B2 load one map at the load tic under the existing bounds.

**Recommendation: B1.** Wires 1–25, DSIM1–41 and every file under `tests/reference` stay untouched; the proof that wire 26 changes nothing else is the M5c method (§4.3).

**Ruled (owner, 2026-11-21):** B1: the session in the game layer (`DSES` v1), wire 26 = wire 25 plus a carried start, DSIM42.

**U3. The saved display and sound states.** Facts: `begin_level` clears the `M_Random` index; `DDSP` v2 accepts a set wipe flag only at simulation tic 0 or 1; `DSND` v1 fixes the map and ties its tic count to the simulation's ([§2.4](#24-frames-sound-the-m_random-index-and-the-wipe)). Options: (i) **new versions, `DDSP` v3 and `DSND` v2**, keyed on the session's tics, carrying the index across loads and holding the non-level frames' state, embedded in `DSES`; `begin_level` gains an explicit "keep the index" variant; (ii) keep `DDSP` v2 and `DSND` v1 per level and put the cross-level remainder in `DSES` (fragile: `DSND` v1's restore fails on any level but the first, because its tics are not the simulation's). (a) (i) follows S18 and M5c's versioning; (ii) would bend frozen rules. (b)–(d) yes for (i). **Recommendation: (i).** `DDSP` v1 and v2 and `DSND` v1 keep their bytes and rules.

**Ruled (owner, 2026-11-21):** (i): `DDSP` v3 and `DSND` v2. Older versions keep their bytes and rules.

### The game mode, the finale and the session's end

**U4. The game mode for the owner's DOOM.WAD, and the finale's art screen.** Facts ([§1.5](#15-game-modes-and-how-the-owners-doomwad-is-identified)): linuxdoom decides the mode by file name only; `doom.wad` is registered; his IWAD lacks `HELP2` and has `CREDIT`; along Episode 1 the modes differ only in that picture and in a switch list that none of his nine maps uses. Options: (i) **keep identification by file name** (as now in `doom-play` and every recorded run); a registered session whose IWAD has no `HELP2` is refused at the first stage-1 frame, as the genuine engine would stop there (a display refusal: the tic stays committed, the frame stays at the last good one); to see the art screen, the IWAD is given to `doom-play` as `doomu.wad` (a copy or link with that name), which is retail and draws `CREDIT`; (ii) upgrade to retail when the IWAD has a fourth episode (a lump probe linuxdoom does not make); (iii) draw `CREDIT` in registered mode when `HELP2` is missing (an adapter no build of the original has). (a) (i) is linuxdoom and the M4b and M5a practice; (ii) and (iii) diverge from it. (b) (i) yes; (ii) and (iii) diverge silently or by declaration from every genuine run made as `doom.wad`. (c) (i) adds only a declared refusal. (d) yes. **Recommendation: (i)**, with the local whole-episode evidence run both ways (registered up to the refusal at finale tic 1,571, which turns the inference into an observation; retail through the art screen).

**Ruled (owner, 2026-11-21):** **Not as recommended.** The game mode is an explicit, declared setting of the session, not read from the IWAD's file name. (1) The default is retail when the IWAD has a fourth episode and registered otherwise; a flag forces either; the mode is printed at launch, shown on the exit line and recorded with the session, so a replay or a check uses the same mode. (2) The genuine engine runs on a scratch copy named for the mode (`doomu.wad` or `doom.wad`); the owner's file is never renamed, moved or written. (3) Forced registered on the owner's IWAD keeps the declared refusal at the first stage-1 frame, as option (i) above describes. (4) Before any session work relies on it, the 61 real runs are re-run in retail mode and their counts and digests reported against the registered results. (5) Local evidence both ways: registered up to the refusal (turning the `HELP2` inference into an observation) and retail through `CREDIT`. (6) The frozen single-level configurations keep identifying by file name, unchanged. (7) COMPATIBILITY states that linuxdoom makes no such probe and that this is a declared rule. The recommendation text above (keep identification by file name) is left as history.

*Where the mode is recorded (proposal, a design to be confirmed at steps 7 and 9, not done).* The demo header has no room: it is `version, skill, episode, map` and the player flags only. No sidecar or exit record exists today. What `doom-play` writes for a session is one file, the demo, named `E1M<map>-skill<n>-<yyyymmdd-hhmmss>.lmp` by `session_name` (crates/doom-game/src/app/recorder.rs:178-185) in the `--sessions` directory (crates/doom-play/src/main.rs:51, 115-118); its exit lines report write counts and failures, not the setting (crates/doom-play/src/closing.rs:282-296), and today the mode is derived from the IWAD's file name by `game_mode` (main.rs:321-331) and stored nowhere. Proposal: the mode is a declared field of the session state `DSES` v1, and the session demo's file name carries it as a token (`...-retail-...` or `...-registered-...`), so a demo file alone tells a replay or a check which mode to use (the replay tools also accept an explicit flag, which must agree with the token or the check refuses). The exit line prints it. The single-level `session_name` and its files stay unchanged.

**U5. Where a session ends.** Facts ([§1.8](#18-after-e1m8-the-finale)): the Doom 1 finale shows the text, then the art screen forever; the only way out is the menu. Options: (i) **as vanilla**: the finale runs until the player quits through the app (its pause screen or the window's close), with S21's tic cap (1,048,576 tics, about 8 h 19 min) as the only bound; the exit line says the finale was reached and at which tic its art screen began; (ii) the app ends the session itself at the first frame of the art screen (or a fixed number of tics after it): an app action with no vanilla counterpart, though the demo stays valid; (iii) end at E1M8's exit, with no finale: against the brief. (a) (i) is vanilla and S21; (ii) adds an app rule. (b)–(d) yes for (i) and (ii). **Recommendation: (i).** With a registered IWAD lacking `HELP2`, U4's refusal ends the session at finale tic 1,571 anyway.

**Ruled (owner, 2026-11-21):** (i): the session ends when the player quits.

**U6. Reborn, and what happens to S11's restart.** Facts ([§1.9](#19-death-and-reborn-in-single-player)): a faithful reborn reloads the map on the dead player's first use press, keeps both random indices and the other process state, runs inside the same demo, and wipes. S10 made death end the session with the app's own screen; S11 made restart a new game. Options: (i) **the faithful reborn inside the session**, and S11's restart kept as an app action ("new session": a fresh process and a new demo) offered only from the app's pause screen and the refusal screen; death and exits no longer end the session, so S10's "YOU DIED" and "LEVEL COMPLETE" screens leave the session; (ii) the faithful reborn and no restart at all; (iii) keep S10 and S11 as they are: against the brief. (a) (i) supersedes S10 and changes what triggers S11, both as the M6a brief asks; T15's "every session starts as a fresh process" stays true. (b) yes: the reborn is compared tic for tic. (c) yes. (d) yes. **Recommendation: (i).** The single-level M5c configuration stays in the libraries for the frozen tests and tools and is not offered as a launch flag, as T15 did for M5b's.

**Ruled (owner, 2026-11-21):** (i): the faithful reborn in the session; S11's restart stays as an app action.

**U7. The intermission and the finale in the demo and the recorder.** Facts ([§1.10](#110-demos-across-levels)): one version-110 demo carries everything; vanilla's `-record` stops at 32,761 tics by default; genuine playback has no limit; S21's session cap is 1,048,576 tics. Options for the cap: (i) **keep S21's cap**; (ii) cap at 32,761 so that a default vanilla `-record` could have made the same file (it limits the owner to 15 min 36 s, which an episode exceeds). The rest has one sensible option: one demo per session; header map is the start map; a command every tic in every game state, including intermission and finale tics; a session parser that does not stop at a boundary, with the single-level parser and its files unchanged; `BT_SPECIAL` never recorded and refused in replay. (a) (i) is S21; (b)–(d) yes. **Recommendation: (i) and the rest as stated.** Old single-level session files stay valid demos; replayed in the session configuration they give the same game and end at their last tic.

**Ruled (owner, 2026-11-21):** (i), and the rest as stated.

### Fidelity contracts

**U8. Quirks of the transitions, reproduced as they are.** None is fixed; a fix would be a new profile needing its own decision. The owner will see these and should know they are the original's: (1) no intermission and no statistics after E1M8; (2) the tally waits for a press, which counts only as a rising edge, so a fire or use button held from the level does nothing until released; (3) the background animation is not drawn, though its random draws happen; (4) the pointer is hidden on the first "you are here" frame of a session and shown on later ones; (5) E1M9 is not marked as done when entering it, is marked after it, and a later death unmarks it; the secret level always returns to E1M4; (6) percentages above 100 are possible; "sucks" above 61:59; (7) keys and power-ups are dropped at an exit, health, armour, weapons and ammunition kept; a pending weapon change is cancelled; a landing just before the exit gives a short squat; a player dead at the exit starts the next map with a pistol; (8) a carried chainsaw makes its raise sound at load; (9) level sounds play on into the intermission, heard from where the player stood; (10) the reborn comes on the first use press, with no delay, and keeps both random indices; (11) the finale never ends; (12) a reborn and an exit in the same tic reload the map (inferred; it gets a synthetic witness or, if it cannot be built, stays documented as unproven). (a)–(d) yes. **Recommendation: reproduce all of them and list them in COMPATIBILITY.**

**Ruled (owner, 2026-11-21):** Reproduce all twelve and list them in COMPATIBILITY. Item 12 (a reborn and an exit in the same tic reload the map) is inferred: nothing may rely on it until a genuine run shows it; if it cannot be built, it stays documented as unproven and refused, not guessed.

*Item 12, observed (Phase 2 step 2, 2026-11-21):* a genuine run now shows it: the synthetic items `USEAT131` (the witness) and `USEAT130`, `USEAT132` and `PROBE` (the controls), [§1.9](#19-death-and-reborn-in-single-player).

**U9. The wipe in the frame contract.** Facts ([§1.11](#111-the-screen-wipe)): 17 to 19 wipes per episode plus one per reborn, each 320 `M_Random` draws, no tic run during the melt. Options: (i) **T6 extended**: every transition's 320 draws accounted at the declared point (after the frame of the first tic in the new state), the genuine `f_wipe.c` in the runner, the melt not shown and taking no time; the frame shown is the composed frame; (ii) show the melt as presentation, outside every claim: 34–43 extra pictures per transition, about a second each with no tic, needing a declared clock and start screen (and the palette question of §1.11), about 800–1,200 more lines; (iii) model the melt's pictures as compared frames under a declared tick schedule. (a) (i) is T6; (ii) and (iii) change pacing the owner has not seen. (b)–(d) yes for (i). **Recommendation: (i).** The melt picture is original behaviour, so not optional: it moves to a later milestone, to be named by the owner, as T6 left it; the palette side note is checked when the melt is shown.

**Ruled (owner, 2026-11-21):** **Not as recommended.** The melt is in M6a and compared: option (iii). Before step 5 a short section is added to this note, written from the C and from genuine runs, settling: the clock (one melt step per tic, each step's picture compared with the genuine `f_wipe.c` output); what the first melt of a session melts from; which palette is active for each melt; what sound and the mixer do while a melt runs, since no game tic runs then; and whether input during a melt is kept or lost. If any of these needs the owner's ruling, work stops there; otherwise it goes on. The record says plainly that the original's in-between pictures depended on machine speed and that the claim is identity under the declared schedule. The recommendation text above (option (i), the melt moving to a later milestone) is left as history.

**U10. Missing music lumps.** Facts ([§1.7](#17-music-and-what-its-absence-changes), [§2.7](#27-frozen-wires-at-the-boundary-what-the-first-comparison-found)): the genuine engine stops when it first needs a music lump that is missing: the first map's at its load (before tic 0), a later map's at its load tic, `D_INTER` on the intermission's first tic, `D_VICTOR` on the finale's first tic; it does not look again for a music already playing or already found. M5c's app load plays on without them. Options: (i) **under the session configuration only, refuse at the moment the genuine engine stops**: a load error for the first map's music (the original's first load aborts before any tic, so refusing the session at load is exactly the original), and a refuse-the-tic witness with the state preserved at the load tic, the intermission's first tic or the finale's first tic; M5c's single-level load keeps its behaviour and COMPATIBILITY records it as a known pre-existing gap with this evidence; (ii) refuse the session at load if any of the eleven lumps is missing (stricter than the original, which plays until it needs one); (iii) also change M5c's single-level load (changes an existing configuration). (a) (i) follows T11 (refuse where the original stops) and the frozen-configuration rule; (ii) refuses sessions the original plays in part; (iii) breaks the rule. (b) (i) yes, and the M5c gap becomes declared. (c) (i) adds only declared refusals. (d) yes. **Recommendation: (i).** No IWAD session is affected; each refusal gets a synthetic witness.

**Ruled (owner, 2026-11-21):** (i).

### Evidence and contents

**U11. The synthetic WADs.** Proposed, all generated by committed tools with `--check`, our own drawings and placeholders, nothing from the IWAD: (1) nine small synthetic maps E1M1–E1M9 laid out for the paths: normal exits by line and switch, a secret exit on one map, E1M9's return to E1M4, an E1M8 ending by sector 11 with a Baron for `A_BossDeath`, a damage floor for deaths and reborns, keys, power-ups and a chainsaw to carry or drop, more kills than the total where the C allows; (2) our own pictures for all 79 intermission names (visibly distinct, our own sizes and offsets chosen from the positions the C fixes), `FLOOR4_8` as a 4,096-byte raw flat, `CREDIT` and `HELP2`; (3) text placeholders for `D_E1M1`–`D_E1M9`, `D_INTER` and `D_VICTOR` as M5c's `m5c_sfx.py` makes the first nine; (4) variant WADs each missing one lump (`HELP2`, `D_INTER`, `D_VICTOR`, a later map's music) for the refusal witnesses; (5) the status bar, font and sounds reused from the committed M5b and M5c sets; (6) hand-made scripts covering every intermission path (no press, a press in each state, held buttons, presses on consecutive tics), reborns, the secret route and the finale in both modes. (a) as R13, S23 and T10. (b)–(d) yes. **Recommendation: as stated.**

**Ruled (owner, 2026-11-21):** As stated, plus whatever the melt and the two game modes need.

**U12. Easy skills: in M6a or the next milestone (not optional).** Facts ([§1.12](#112-skills-sized)): sk_easy differs from medium on this path only in the spawn bit (1 site); sk_baby adds double ammunition and halved player damage (3 sites); neither draws new random numbers or adds state. Both need admission under wire 26 (skills 2 and 3 only today, mod.rs:376-383, and the demo parser's skill list), synthetic things flagged per skill, genuine evidence, and the owner's own sessions. Size (inferred): about 150–250 lines of source, 300–500 of tests, 100 of tools for both. Options: (i) neither in M6a (both move to the next milestone, to be named by the owner; ROADMAP's proposed M4c names easy skill); (ii) sk_easy only in M6a, sk_baby moving likewise; (iii) **both in M6a**. (a) the brief leaves it open; M4c's proposal named easy skill. (b)–(d) yes. **Recommendation: (iii), as a late step of M6a; if the cap is reached first, both move to the next milestone (to be named by the owner; ROADMAP's proposed M4c names easy skill), never dropped.** The brief's "easy skill" could mean either; both are cheap, and doing them together avoids a second admission change.

**Ruled (owner, 2026-11-21):** (iii): both easy skills in M6a, as a step after the hand-test gate ([§6.5](#65-order-of-work-owners-ruling-2026-11-21)).

**U13. Nightmare: in M6a or a later milestone (not optional).** Facts (§1.12): 8 sites plus the respawn, new `P_Random` draws every 32 tics per eligible corpse, new sound starts, a process-global table edit, faster demons and missiles that matter on E1M8; the menu's confirmation comes with the menus, in a later milestone to be named by the owner. Size (inferred): about 1,500–2,500 lines with its evidence, in the simulation's monster code. Options: (i) **not in M6a**; (ii) in M6a under wire 26. (a) (i) keeps M6a about transitions; (ii) adds a monster-behaviour change with its own real-map evidence. (b)–(d) yes for both if proven. **Recommendation: (i)**: nightmare moves to a later milestone of its own, to be named by the owner; it is not optional.

**Ruled (owner, 2026-11-21):** (i): nightmare lands in M6b, together with the original's pause command. M6b goes on the roadmap and on the checklist (a separate documentation PR; not in this PR).

**U14. The evidence plan.** (1) **CI, synthetic**: the U11 WADs and scripts through the genuine `m6a` runner and Rust: per-level captures, a session row per tic, every frame (level frames as M5c, whole frames in the intermission and finale), every sound row and every mixer buffer; self-tests that prove each comparison can fail; the proof of §4.3 that wire 26 changes nothing else; (2) **local, the owner's IWAD**: the 61 existing runs re-run through every tool (frozen wires unchanged); chained runs across boundaries from spliced demos (valid inputs even where they desynchronise, §3.3); a whole-episode run (E1M1 to the finale, the secret route, at least one reborn) recorded continuously with the Phase 2 `doom-play`, in both modes (U4); (3) **the owner's own sessions** played with the Phase 2 build, each checked with the local session tool and reported with its counts. Counts, pass/fail facts and digests only are committed. (a) as the M4a evidence hierarchy and T10. (b)–(d) yes. **Recommendation: as stated.** The whole-episode real run depends on someone playing a whole episode with the new build; if no agent-made chain is possible, that run is the owner's session.

**Ruled (owner, 2026-11-21):** As stated. For real-map evidence, sessions may start at any map, so E1M8 into the finale does not need a whole-episode run.

### Structure, size and delivery

**U15. Crates, harness, CI, size and cap.** (1) Code as U2 B1: wire 26 in `doom-sim`; the session, intermission and finale logic and `DSES` in `doom-game`; the intermission and finale drawing and `DDSP` v3 in `doom-render`; the level change and `DSND` v2 in `doom-sound`; the default session in `doom-play`. (2) New harness files only (`m6a_*`): no existing runner, driver, stub, contract, golden or file under `tests/reference` changes; the `wi_stuff.c` accessor wrapper is the one new declared adapter. (3) CI: the Rust goldens on Linux, Windows and macOS in the existing jobs; the genuine `m6a` run in one new Linux job, **not a required check** unless the owner says so. (4) A short animation of a synthetic session crossing an intermission, made by a committed script from our own graphics, as the visuals rule asks. (5) Size: a projection of 10,400–19,400 hand-written lines against a **cap of 22,000** ([§6.3](#63-size)); fallback as [§6.4](#64-cap-and-fallback). (a)–(d) yes. **Recommendation: as stated.**

**Ruled (owner, 2026-11-21):** As stated, cap 22,000. The new CI job is named without a "(not required)" suffix from the start. At the cap, work stops and the owner is asked which milestone the remainder moves to; nothing is dropped.

### Why Phase 1 stops here

U2, U4, U5, U6 and U10 rest on facts that contradict what the brief took for granted, and U12 and U13 are the owner's to choose. Per the unattended rule, work continues only where no ruling can invalidate it ([§6.2](#62-what-proceeds-before-the-rulings)).

## 6. Plan, size estimate and cap

### 6.1 Steps

One commit per step (C evidence, then the fix, then a test, each its own commit where a fix is involved), pushed, with the status line at the top of this note updated after each. Each step runs in a fresh named subagent on its pinned model; one writer on the tree at a time.

| Step | What | Agent | Waits for |
| --- | --- | --- | --- |
| 1 | The genuine `m6a` runner: the M5c runner forked, `wi_stuff.c` and `f_finale.c` whole, the accessor wrapper, session rows, the intermission `M_Random` bucket, load detection by `levelstarttic`, the non-level display mirror and frame digests, a game-mode switch; strict, overflow, address and 32-bit variants agreeing | c-evidence | nothing (starts now) |
| 2 | The synthetic multi-map WAD generator with all U11 contents (U11 ruled) and variants as switches, both game modes; the scripts; goldens from the C only | c-evidence | step 1 |
| 3 | Local genuine observations on the owner's IWAD, the genuine engine run on scratch copies named for the mode (`doomu.wad` retail, `doom.wad` registered): the U4 pre-work (the 61 real runs re-run in retail mode, counts and digests against the registered results), then E1M8 into the finale in both modes (the `HELP2` refusal observed in registered, `CREDIT` in retail), sessions started at any map, chained spliced runs, counts only | c-evidence | step 1 |
| 4 | Wire 26 and DSIM42 in `doom-sim`, the independent encoder, the proof that nothing else changes | c-evidence | U2 |
| 5 | `doom-render`: the intermission and finale drawers, the "keep the index" `begin_level`, the wipe at every transition, `DDSP` v3 (designed so the melt pictures and the finale fit without another version bump) | c-evidence | the melt section ([§6.5](#65-order-of-work-owners-ruling-2026-11-21)) |
| 6 | `doom-sound`: the level change, the frozen listener, `DSND` v2 | c-evidence | U3 |
| 7 | `doom-game`: the session layer, the intermission and finale logic, the reborn, recorder and replay across levels, `DSES` v1, the refusals | c-evidence | U2, U4–U8, U10 |
| 8 | Rust against the genuine engine: every synthetic script (state, session rows, frames, sound rows, mixer buffers); then the local chained runs and the 61 runs; every divergence explained from the C, fixed and reproduced by a synthetic script | c-evidence | step 7 |
| 9 | `doom-play`: the session by default, the declared game mode and its flag, the end at quit, the restart from the pause screen; a Windows artifact for the owner's hand test (the label on the draft PR) | c-evidence | steps 7, 8 |
| 10 | Both easy skills (U12 (iii)), after the hand-test gate | c-evidence | the owner's hand test |
| 11 | Restore-and-continue tests across every boundary (mid-intermission, the load tic, the reborn, the finale); fuzz targets for DSIM42, `DSES` v1, `DDSP` v3 and `DSND` v2 with seed replay, coverage-guided and AddressSanitizer campaigns reported separately | tester | steps 4–7 |
| 12 | CI: Rust goldens on three systems; the genuine `m6a` job on Linux, named without a "(not required)" suffix | docs-tools | steps 1–9 |
| 13 | Docs: COMPATIBILITY, STATE_FORMAT, ARCHITECTURE, TESTING, ROADMAP, README, the evidence record; the animation | docs-tools | steps 1–12 |
| 14 | Mutation testing in shards; every survivor dispositioned; the reviewer checks every equivalent and unreachable disposition | tester, reviewer | steps 4–9 |
| 15 | Timings from ext4 on an idle machine: the cost of a load tic and an intermission tic; recorded, not gated | docs-tools | step 9 |
| 16 | Independent review at xhigh effort: the session, intermission and finale against the C line by line; the simulation change confined to wire 26; overclaiming in the docs | reviewer | steps 4–13 |
| 17 | Full gates from clean on ext4, the owner's sessions checked, the PR title and body, CI green on the exact head, mark the PR ready, stop | docs-tools | everything |

### 6.2 What proceeds before the rulings

*Superseded: the rulings were received on 2026-11-21; the order of work is [§6.5](#65-order-of-work-owners-ruling-2026-11-21). Kept as written in Phase 1.*

Steps 1 to 3 only, and inside them only what no ruling can invalidate: the genuine runner, the synthetic WAD generator with every lump and every variant behind a switch, the scripts, and the local genuine observations. The game mode (U4) and the WAD variants (U10, U11) are switches of the tools, not assumptions; goldens are generated for the recommended values and regenerated with another switch if a ruling differs. Nothing is done to the Rust crates, `Cargo.toml` or the app until the owner has ruled.

### 6.3 Size

Hand-written lines, projected (central figures; the inputs are the crate and file sizes measured in Phase 1, the prototype's 328 lines, and M5c's estimate and outcome):

| Part | Lines | Basis |
| --- | ---: | --- |
| Rust source: `doom-sim` wire 26 and DSIM42 (700); `doom-game` session, intermission and finale logic, recorder and replay, `DSES`, refusals (2,000); `doom-render` drawers, display v3 (1,100); `doom-sound` level change, `DSND` v2 (350); `doom-play` (150) | 4,300 | `wi_stuff.c` 1,850, `f_finale.c` 738 lines of C, most of `wi_stuff.c` multiplayer; M5c's wire 25 estimate (900) |
| Rust tests and fuzz targets | 4,500 | about 1:1 with source, as M5c estimated |
| Tools: the runner fork and wrapper (about 950 with the build), the synthetic WAD generator and scripts (900), the DSIM42 and session-row encoders and comparators (550), the local tools (600), CI wiring (100) | 3,100 | the prototype (328 lines, with about 70 of C), M5c's runner (1,983 + 214 + 274) |
| Docs: this note (Phase 1, written), the evidence record, COMPATIBILITY, STATE_FORMAT (four new versions), ARCHITECTURE, TESTING, ROADMAP, README | 3,000 | M5c's docs, minus its 6,000-line note |

**Projection: 14,900 lines, ±30% = about 10,400–19,400.** Easy skills (U12, ruled into M6a) add about 550–850 and are not in the projection above; the melt pictures (U9 (iii), ruled into M6a) are not in it either (U9 (ii)'s estimate for showing them was 800–1,200 lines; comparing them step by step adds to that and is to be re-estimated when the melt section is written). The cap is checked against all of it. M5b's and M5c's Rust landed above their estimates (M5c: about 1.5 times for source and 3 times for tests), which is the main risk to this projection. Generated data checked by `--check` tools (goldens, results files, the synthetic WADs) is reported separately.

### 6.4 Cap and fallback

**Cap: 22,000 hand-written lines** of code, tools and docs (U15, ruled), checked after steps 7 and 8. Easy skills (step 10) and the melt are inside M6a (U12, U9), so the fallback no longer moves them or the retail-mode run automatically. If the count reaches the cap, work stops and the owner names the milestone for the remainder; nothing is dropped. The cap decides which milestone an item lands in, never whether it lands (the owner's standing rule, 2026-11-21). No proof of the core is thinned to fit: the session, the intermission, the finale, the reborn, the wipes' draws, sound and frames across levels, and the frozen-wire proof stay in M6a.

### 6.5 Order of work (owner's ruling, 2026-11-21)

The owner wants to play through the episode early. The order is: (1) the playable chain first (exit, intermission, next map with the carried player, reborn, the secret route), compared against the genuine engine and green; (2) then the windows-artifact label on the draft PR and a stop for the owner's hand test; (3) after that the finale, the melt pictures and the easy skills. Mapped onto the steps of [§6.1](#61-steps):

| Phase | Work |
| --- | --- |
| Before the hand-test gate | Step 1 (the genuine runner) and step 2 (the synthetic WADs, both modes). The U4 pre-work inside step 3: the 61 real runs re-run in retail mode on a scratch copy named `doomu.wad` and compared with the registered results (local evidence both ways, counts and digests only). The **melt section** of this note (U9): written by a c-evidence step before step 5, from the C and from genuine runs, settling the clock, the first melt's source, the palette per melt, sound and the mixer during a melt, and input during a melt; work stops for a ruling if any needs one. Then steps 4 to 9 restricted to the chain, with the display and saved-state versions (`DDSP` v3, `DSND` v2, `DSES` v1) designed so the melt pictures and the finale fit without another version bump. Last, the windows artifact and the stop. |
| After the hand-test gate | The finale (step 7's finale part and its comparison), the melt pictures compared step by step, the easy skills (step 10), then steps 11 to 17. |

The cap of 22,000 lines applies to all of it ([§6.4](#64-cap-and-fallback)).
