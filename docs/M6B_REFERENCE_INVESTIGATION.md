# M6b: Nightmare skill and the original's pause: reference investigation

Phase 1 note for milestone M6b, named by the owner's ruling of 2026-11-21: the Nightmare skill and the original's pause command. Phase 1 investigates, lists what needs the owner's ruling, builds the genuine-engine evidence that no ruling can change, and stops. Nothing under any crate's `src` changes in this phase, no existing test, fixture, golden or trace changes, and wires 1 to 28 and every merged format version stay as they are.

The behavioural authority is the original id Software Linux DOOM 1.10 at `a77dfb96cb91780ca334d0d4cfd86957558007e0`; every `file:line` without a directory is in `linuxdoom-1.10/`. Milestone status lives in [ROADMAP](ROADMAP.md), support in [COMPATIBILITY](COMPATIBILITY.md), schemas in [STATE_FORMAT](STATE_FORMAT.md) and commands in [TESTING](TESTING.md) (see the [documentation authority map](README.md#documentation-authority)).

Each fact is labelled. **(run)**: seen in the genuine engine, built from the original source by the M6b runner ([§6.1](#61-the-genuine-side-built-in-this-phase)), in a committed item whose golden is checked in CI unless the sentence says it was an authoring run. **(read)**: read in the C or Rust source and not run. **(inferred)**: reasoned from cited lines.

**Status (2026-11-26, America/Los_Angeles).** Phase 1 is written and the genuine-engine side is built: 19 synthetic items, 24 planted faults, 7 runs after an earlier game of the same process, two self-tests, all in CI's M6a genuine session job. Decisions N1 to N11 in [§8](#8-decisions-for-the-owner) were ruled on by the owner on 2026-11-26 ([rulings](#rulings)); Phase 2 builds on them. Not built in Phase 1: everything in the Rust crates, and the state digests over the wire 29 schema, which wait on N2, N4, N5 and N6.

## Summary

- **Nightmare is nine code sites and one function.** Seven are one-line tests of the skill; two are the table edit made once per game and its undoing; the function is the corpse respawn. Every site was run in the genuine engine on a synthetic map. Eight of the nine, and the respawn, have a planted fault in the original's text that changes the run at a place written down in the goldens; the undoing of the table edit is run and has none.
- **The simulation already saves almost everything Nightmare needs.** Each monster's spawn point, `movecount` (the respawn timer), `reactiontime` and `lastlook` are in the canonical state today, corpses stay actors with `MF_COUNTKILL`, the teleport fog and its sound exist, and both `A_Chase` rules are already written against the skill (read). New are: two options (respawn and fast), the fast table, the respawn itself, and kills above the level total, which the state reader refuses today.
- **Three things were found that the M6a note did not have.** The respawn timer does not start at 0: it is the corpse's `movecount`, which keeps what the chase left. A corpse crushed by a door still respawns, with radius 0, inside the closed door. And `P_NightmareRespawn` has the same signed overflow in its angle line as the two spawn sites, which no run had reached before; the sanitizer builds get one new declared adapter for it ([§6.1](#61-the-genuine-side-built-in-this-phase)).
- **The table edit is hidden process state only when `-fast` or a saved game is involved.** In the three histories of plain skills that were run, the tables are those of the current skill; the code says the same for every new game without `-fast` (read). With `-fast` an earlier game leaves the tables fast with `fastparm` cleared, or halved twice so that the demon's run states last 0 tics (run).
- **The pause is mostly a session matter, with one small simulation part.** `G_Ticker` owns the flag; while it is set the level does not tick, but the status bar (with its `M_Random` draw), the message timer, the intermission, the finale, the sound logic and `gametic` go on (run). The simulation has two parts: a command byte with bit 7 on a tic the level runs (a live player's buttons are zeroed, a dead player's are read raw), and the turbo message, which the Rust simulation tests against a `gametic` that the pause moves away from the level's tic count.
- **Three saved formats tie a level's tic count to the session's.** `DSES`, `DDSP` and `DSND` each have a rule that one session tic in a level is one simulation tic (read); the pause breaks it, so each needs a new version.
- **Size.** Phase 2 is estimated at about 10,900 hand-written lines ([§9](#9-step-plan-and-size)); the recommendation is one wire (29), one pull request and a ceiling of 13,000.

## 1. Nightmare, site by site

`sk_nightmare` is 4 (doomdef.h:153). The M6a note's [§1.12](M6A_REFERENCE_INVESTIGATION.md#112-skills-sized) table and ruling U13 were checked line by line against the source; every citation holds, with two corrections recorded in [§1.9](#19-citations-checked).

### 1.1 The spawn bit

(read) `P_SpawnMapThing` skips a thing whose options have bit 16 outside a net game (p_mobj.c:740-741), then picks the skill bit: 1 for `sk_baby`, **4 for `sk_nightmare`**, `1<<(gameskill-1)` otherwise (p_mobj.c:743-748), and skips a thing without it (750-751). So Nightmare spawns exactly the hard set.

(run) Items `BITS4` and `BITS3`: of five things with the option patterns 4, 1|8, 2, 4|16 and 1, both skills spawn the first only. The planted fault `bit` computes Nightmare's bit like the other skills', which gives 8, the ambush flag, and spawns the 1|8 thing instead: the run differs at capture 0.

The Rust spawn code already maps Nightmare to 4 in two places (`crates/doom-sim/src/spawning.rs:548-552`, `episode1.rs:58-64`; read). The state reader's skill table does not: it reads a skill byte of 4 as medium (`diagnostic.rs:576-584`), which nothing reaches today because the reader refuses the byte first (`diagnostic.rs:1845-1851`).

### 1.2 Double ammunition

(read) `P_GiveAmmo` doubles the amount on `sk_baby` and `sk_nightmare`, after the clip arithmetic and before the cap (p_inter.c:95-101). Wire 28 built this path for skill 0.

(run) Item `AMMO4`: wire 28's corridor over every `P_GiveAmmo` path of Episode 1, on skill 4. Planted fault `nodouble` (Nightmare no longer doubles): differs at capture 10.

Nightmare does **not** halve damage: that test names `sk_baby` alone (p_inter.c:799-800, read).

### 1.3 `G_InitNew`: the clamp, `respawnmonsters` and the fast set-up

(read) In order:

1. If the game is paused it is unpaused, with `S_ResumeSound` (g_game.c:1372-1376).
2. A skill above 4 becomes 4 (g_game.c:1379-1380). (run) Item `CLAMP5`, a demo whose skill byte is 5: every dump equals `BITS4`'s. Planted fault `clamp`: differs at capture 0.
3. `M_ClearRandom` (g_game.c:1414).
4. `respawnmonsters = (skill == sk_nightmare || respawnparm)` (g_game.c:1416-1419).
5. The table edit (g_game.c:1421-1436). **If** `fastparm`, or the new skill is Nightmare and the **previous** `gameskill` was not: the `tics` of the thirteen states `S_SARG_RUN1` to `S_SARG_PAIN2` are shifted right by one (run 2 to 1, attack 8 to 4, pain 2 to 1; info.c:613-625) and the speeds of `MT_BRUISERSHOT`, `MT_HEADSHOT` and `MT_TROOPSHOT` are set to 20 units. **Else if** the new skill is not Nightmare and the previous one was: the same thirteen are shifted left by one and the speeds set back to 15, 10 and 10.
6. `gameskill = skill` (g_game.c:1450), after the edit, which is what makes step 5 depend on the game before.

`states[]` and `mobjinfo[]` are process globals (info.c), so the edit outlives the game. `gameskill` starts at 0 (g_game.c:99, a zeroed global).

(run) Items `FAST4`, `FAST3` and `FAST3F` (the same commands on skill 4, skill 3, and skill 3 with the demo header's `fastparm` byte set): the M6b runner's `T` rows print the thirteen durations and the three speeds after `G_InitNew`. Skill 4 and skill 3 with `fastparm` both give `1 1 1 1 1 1 1 1 4 4 4 1 1` and 20, 20, 20; skill 3 gives `2 2 2 2 2 2 2 2 8 8 8 2 2` and 15, 10, 10. In the dumps the imp's and the Baron's missiles move 20 units a tic in the plane on skill 4 and on skill 3 with `fastparm`; on plain skill 3 the Baron's moves 15 and the imp throws nothing within the run (`REACT` and `PAUSE` show the imp's at 10). The demon and the spectre show state durations of 1 to 4. Planted faults: `fast_states` (no halving) differs at capture 2, `fast_speeds` (speeds not raised) at the capture of the tic the first missile is spawned.

**What Episode 1 has of this.** The Demon and the Spectre share the thirteen states (info.c:1422-1469); the imp throws `MT_TROOPSHOT` and the Baron `MT_BRUISERSHOT`. `MT_HEADSHOT` is the Cacodemon's, which Episode 1 does not place (thing types only; inferred from the wire 24 thing set).

#### A second game in the same process

(run) The runner's `--prior-game SKILL,RESPAWN,FAST` calls the genuine `G_InitNew` before the demo starts, as an earlier game of the process would have. Seven runs, in `tests/reference/m6b_genuine.txt` (`T` rows):

| Earlier games | Then | Durations the demo plays with | Speeds | Game rows equal to |
| --- | --- | --- | --- | --- |
| skill 3 | skill 3 | 2 … 8 8 8 2 2 | 15, 10, 10 | the fresh skill 3 run (control) |
| Nightmare | skill 3 | 2 … 8 8 8 2 2 | 15, 10, 10 | the fresh skill 3 run: put back exactly |
| Nightmare | Nightmare | 1 … 4 4 4 1 1 | 20, 20, 20 | the fresh Nightmare run: halved once, not twice |
| Nightmare with `-fast` | Nightmare | 1 … 4 4 4 1 1 | 20, 20, 20 | the fresh Nightmare run |
| skill 3 with `-fast` | skill 3, no `-fast` | 1 … 4 4 4 1 1 | 20, 20, 20 | neither: the tables stay fast while `fastparm` is 0, so `A_Chase` uses its slow rules |
| skill 3 with `-fast` | skill 3 with `-fast` | **0 … 2 2 2 0 0** | 20, 20, 20 | neither: halved twice |
| Nightmare with `-fast`, twice | skill 3 | **0 … 4 4 4 0 0** | 15, 10, 10 | neither: halved twice, doubled once, and 0 doubled is 0 |

So in the three histories of plain skills that were run, the game rows are the fresh run's; by the code, any sequence of new games without `fastparm` leaves the tables those of the current skill (read). Every surprise in the table needs `fastparm`, because step 5 halves on **every** `G_InitNew` while it is set. A duration of 0 makes `P_SetMobjState` run the next state in the same call (p_mobj.c:60-81): a demon whose eight run states all last 0 tics calls `A_Chase` over and over inside one tic until it attacks or loses its target (read; the run above ended normally in that room; a demon that can neither reach nor lose its target would not return, inferred).

How a second game gets `fastparm` in the original (read): `G_DoNewGame`, the menu's new game, clears `respawnparm` and `fastparm` before `G_InitNew` (g_game.c:1352-1356), which gives row 5; `G_DoPlayDemo` sets both from each demo's header (g_game.c:1600-1601), which gives row 6 for two `-fast` demos played in one process; `G_CheckDemoStatus` clears them when a demo ends (g_game.c:1669-1670). Two more routes, neither run: the level-warp cheat goes through `G_DoNewGame` (st_stuff.c:721), so it gives row 5 without a menu; and `G_DoLoadGame` writes `gameskill` from the save **before** it calls `G_InitNew` (g_game.c:1220, 1227), so the edit's test of the previous skill sees the loaded one: a Nightmare save loaded into a fresh process plays with slow tables, and a non-Nightmare save loaded after a Nightmare game keeps fast ones.

The Rust app has no menu, no demo loop, no saved games and no cheats. Its restart (`crates/doom-game/src/app/mod.rs:1393-1396`) builds a new session with both random indices 0 (`mod.rs:1066-1067`; read), which behaves as a new process does. Decision N4.

### 1.4 `reactiontime`

(read) `P_SpawnMobj` copies the type's `reactiontime` only when the skill is not Nightmare (p_mobj.c:503-504); the mobj was zero-filled just before (491), so on Nightmare every mobj starts with 0. The field is read in `P_CheckMissileRange`, which refuses a missile while it is non-zero (p_enemy.c:212), counted down in `A_Chase` (676-677) and for the player in `P_PlayerThink` (p_user.c:267-268), and cleared by damage (p_inter.c:902). This test names the skill only: `-fast` does not change it.

(run) `REACT4`, `REACT3`, `REACT3F`: every monster's `reactiontime` at load is 0 on skill 4 and 8 on skill 3 with or without `fastparm`. The imp's first fireball exists after session tic 29 on skill 4, after tic 53 on skill 3 with `fastparm`, and not within 240 tics on plain skill 3. Planted fault `react`: differs at capture 0.

The Rust spawn already has this line (`crates/doom-sim/src/lifecycle.rs:1412-1416`, read), unreachable today.

### 1.5 `A_Chase`'s two rules

(read) Both name the skill **and** `fastparm`:

- After an attack (`MF_JUSTATTACKED`), the flag is cleared and `P_NewChaseDir` is called only if the skill is not Nightmare and `fastparm` is off (p_enemy.c:716-722). On Nightmare the monster returns without choosing a direction, so it does not turn away.
- Before a missile, a monster with `movecount` left skips the missile only below Nightmare and without `fastparm` (p_enemy.c:738-742). On Nightmare it may fire while still walking a leg.

(run) Planted faults on `REACT4` and `REACT3F`: `chase_dir` (the direction always chosen) differs at captures 32 and 60; `chase_count` (the `movecount` test always made) at captures 6 and 30.

The Rust `A_Chase` has both tests against the skill (`crates/doom-sim/src/pursuit/chase.rs:49-55`, `monsters.rs:313-315`, read); the `fastparm` half is new.

### 1.6 The corpse respawn

(read) `P_MobjThinker`, for a mobj whose `tics` is -1 (p_mobj.c:450-471):

1. return unless it has `MF_COUNTKILL`;
2. return unless `respawnmonsters`;
3. `movecount++`;
4. return while `movecount < 12*35` (420);
5. return unless `leveltime & 31` is 0;
6. `P_Random()`; return if the value is above 4;
7. `P_NightmareRespawn`.

**The timer** is the corpse's `movecount`, which `A_Chase` used for the length of a walk leg (p_enemy.c:356, 764). It is not reset at death, so the wait is 420 minus what the chase left. (run) `GIB4`: a zombieman that never moved reaches `tics` -1 at session tic 118 with `movecount` 0 and respawns at 1024 with `movecount` 906, one count per tic. (run, authoring run only) a zombieman stuck against a wall showed `movecount` -10 while alive: `--actor->movecount < 0` runs on every `A_Chase` call and `P_NewChaseDir` sets a new count only when it finds a direction, so a stuck monster's corpse waits longer.

**The gate.** Six entries of `rndtable` are 4 or less (m_random.c; counted), so an eligible corpse passes 6 times in 256 draws, on tics whose `leveltime` is a multiple of 32. The draw is consumed whether or not the respawn then happens. Every eligible corpse draws, in thinker order.

**`P_NightmareRespawn`** (p_mobj.c:356-409; read, each step run):

1. `P_CheckPosition (corpse, spawn x, spawn y)` (p_mobj.c:370). If it fails, return: nothing else happens and the corpse tries again at a later gate. The check is made for the **corpse** as it is now, at the spawn point: its radius (0 for a body turned to gibs) and its flags, not the living monster's. It moves nothing. It fails on a one-sided line, a line with `ML_BLOCKING` or `ML_BLOCKMONSTERS`, or a solid thing in reach (p_map.c:211-221, 258, 342); it does not test whether the monster's height fits (that is `P_TryMove`'s test, not this function's).
2. A fog, `MT_TFOG`, at the corpse's x and y, at its sector's floor (375-377), and `sfx_telept` on it (379).
3. A second fog at the spawn point, at that sector's floor (382-384), and `sfx_telept` on it (386).
4. The monster: `P_SpawnMobj` of the corpse's type at the spawn point, on the floor or the ceiling by `MF_SPAWNCEILING` (392-398). It inherits the spawn point (399), the spawn angle (400) and the ambush flag (402-403), and gets `reactiontime` 18 (405) on every skill, Nightmare's 0 included.
5. `P_RemoveMobj (corpse)` (408).

**Random draws:** one for the gate, then one per `P_SpawnMobj` (`lastlook`, p_mobj.c:506): four in all for a respawn, one for a blocked or failed gate. The spawn-time randomisation of `tics` (p_mobj.c:787-788) is in `P_SpawnMapThing`, so a respawned monster does not get it.

**Thinker order.** `P_SpawnMobj` adds at the tail (p_mobj.c:531, `P_AddThinker`), and `P_RunThinkers` walks to the tail (p_tick.c:101-121), so the two fogs and the monster think **in the same tic**, after everything that existed, in the order fog, fog, monster. (run) All five respawns in the goldens show the two thinkers before the monster as fogs, nothing after it, and the monster with `tics` 9 of 10 after that tic; the four whose tic could be resolved (below) show the fogs with `tics` 5 of 6.

**Resolved and unresolved gates.** The goldens' `R` rows list a gate only when every draw of its tic can be assigned to a corpse. On a tic where a live monster also draws, the tool counts the tic as unresolved and writes no `R` row: 5 such tics in `RESP4` (its second respawn, at 896, among them) and 1 in `RESP3R`. Every respawn is still recorded, by structure, in `m6b_results.json`.

**Kills.** `totalkills` is counted in `P_SpawnMapThing` only (p_mobj.c:789-790), so a respawned monster is not in the level total, but killing it counts (p_inter.c:685-687).

**The boss death, more than once** (read, not run). `A_BossDeath` has no memory of having fired: it returns only while another thinker of the same type has health above 0 (p_enemy.c:1693-1704). On E1M8 with respawning, each death of a respawned Baron while the other is dead calls `EV_DoFloor` on tag 666 again (p_enemy.c:1730-1733). The real-map runs of Phase 2 will reach this; how the Rust `boss.rs` takes a repeat was not checked.

(run) What the items show:

| Item | What |
| --- | --- |
| `RESP4` (skill 4) | Two demons shot from a ledge. Respawns at session tics 736 and 896; the first corpse lay away from its spawn point, so the two fogs are at different places; the second demon was placed with the ambush flag facing north and comes back with both. Both are shot again: 4 kills of 2. |
| `RESP3R` (skill 3, `respawnparm` byte) | The same commands: one respawn, at tic 832; 3 kills of 2; `reactiontime` 8 at load and nothing doubled. |
| `RESP3` (skill 3) | The same commands: no respawn; 2 of 2. |
| `BLOCK4` | Three zombiemen shot dead. One's spawn point is 1 unit from a wall, one's overlaps a solid thing, one is free and the player stands on it. Gates pass at tics 512, 1248 and 1408, one for each corpse, and each is **blocked**: one draw, no fog, no sound, the corpse stays. The player steps off; the free corpse's next gate, at 1568, respawns it. |
| `GIB4` | A zombieman killed in a doorway. The door closes on the corpse at tic 334: `PIT_ChangeSector` sets `S_GIBS`, radius 0, height 0 (p_map.c:1268-1275). The gibs keep `MF_COUNTKILL` and `tics` -1, so the timer runs on, and at tic 1024 the zombieman respawns **inside the closed door**, where floor and ceiling meet. |

Planted faults: `respawn_flag` (`respawnmonsters` never set; differs as soon as the first corpse would count, capture 159 on `RESP4`), `timer` (13 seconds), `clock` (every 64 tics), `gate` (0 to 3 only; the first `RESP4` gate drew 4), `nocheck` (no `P_CheckPosition`; `BLOCK4` capture 513), `fog_spot` (the first fog at the spawn point), `react18`, `ambush`, `count` (the respawn counted in `totalkills`). Each differs at the capture in the `V` rows.

**A wake-up that comes with it** (read): a respawned monster starts in its spawn state and runs `A_Look`, which takes the sector's `soundtarget` (p_enemy.c:609). That pointer is left from the last noise the player made there, so a monster without the ambush flag wakes at once, and one with it wakes as soon as it has a line of sight in any direction. This is ordinary `A_Look`, already built; it is named here because Nightmare makes it common.

### 1.7 Kills past 100 % at the intermission

(read) `G_DoCompleted` passes `totalkills` and the player's `killcount` (g_game.c:1112, 1125); `WI_updateStats` computes `(skills * 100) / maxkills` with no clamp (wi_stuff.c:1338) and counts up to it in steps of 2 (1349-1356).

(run) `RESP4` uses the exit switch after the fourth kill: the intermission's kill counter runs to **200**. `RESP3R`: **150**. `RESP3`: 100. The session streams and the frame of every intermission tic at screen size 10 are committed. This is the witness the quirk register's row Q-wi_stuff-1338 lacked ("needs nightmare, M6b"); the "sucks" half of that row (a level over an hour) is not touched here. Decision N10.

The Rust state reader refuses a capture whose `killcount` is above `totalkills` (`crates/doom-sim/src/diagnostic.rs:2525-2548`, read), so that guard must change for any Nightmare run with a second kill.

### 1.8 The menu and the status bar

(read) The menu asks for confirmation before Nightmare (m_menu.c:898-912). Menus are a later milestone; `doom-play` takes the skill on its command line. A commented-out test in the status bar's cheat code names the skill (st_stuff.c:546) and does nothing.

### 1.9 Citations checked

Checked on 2026-11-26 against `linuxdoom-1.10/`: every C `file:line` of the M6a note's §1.12 and §1.13, of ruling U13 and of ROADMAP's M6b entry and its three checklist rows. All hold, with two corrections. One Rust citation in §1.13 is stale: the demo writer's refusal is now at `crates/doom-sim/src/demo.rs:333`, not 108-112. The corrections: `P_NightmareRespawn` is p_mobj.c:356-409 (cited as 357-407; the removal of the corpse is line 408), and §1.12's "after 420 tics" is 420 counts of a timer that does not start at 0 ([§1.6](#16-the-corpse-respawn)). U13's "8 sites" is nine by this note's count: 1.1, 1.2, four in 1.3 (clamp, flag, edit, restore), 1.4 and two in 1.5.

## 2. What Nightmare needs that the simulation does not have

From a read of the Rust source at master `d48ea556`; nothing here was run.

**Already there.** `Skill::Nightmare = 4` (`crates/doom-sim/src/lifecycle.rs:155-170`); per monster the spawn point, `movecount`, `reactiontime` and `lastlook`, all in the canonical state (`snapshot.rs:1731-1738`, `1015-1020`, `1712-1717`, `990`); corpses as actors with `MF_COUNTKILL` (`combat.rs:1181-1189`); `MT_TFOG` and `sfx_telept` (`teleport.rs:203-221`); the spawn bit; Nightmare's `reactiontime`; both `A_Chase` rules for the skill; double ammunition for skill 0 (`world_pickups.rs:706-725`).

**Refused today.** Skill 4 is refused in eight places: the `doom-play` command line, the single-level load, the session start, `DSES` restore, the demo header reader and writer (`demo.rs:41`), the simulation constructor (`original_registry.rs:1690-1713`), the spawn profile (`demon_expanded_map.rs:84-92`, which also refuses `respawn_monsters`) and the `DSIM` reader (`diagnostic.rs:1843-1851`).

**New saved state.**

| What | Why | Today |
| --- | --- | --- |
| Two options: respawn and fast | `respawnmonsters` and the `fastparm` half of `A_Chase` are separate from the skill | `actors.options` is (skill, mode, nomonsters); STATE_FORMAT says "fastparm … remain fixed false" |
| The fast table | Thirteen durations and three speeds differ | Tables are compile-time constants copied into a per-simulation registry, and original wires require every record pristine (`original_registry.rs:1279-1405`); state validators compare `tics` against the descriptor (`diagnostic.rs:2040-2044`) |
| Kills above the total | [§1.7](#17-kills-past-100--at-the-intermission) | Refused by the reader |
| Nothing per monster | The timer is `movecount`, the spawn point is saved | — |

**New random draws:** the gate (one per eligible corpse per 32 tics) and three per respawn. **New sound starts:** two `sfx_telept`, each on its fog. **New behaviour:** `P_MobjThinker`'s `tics == -1` branch, which is a bare return today (`lifecycle.rs:1792-1812`); `P_CheckPosition` for a corpse at another place, with radius 0 for gibs; three spawns at the tail in one tic.

**Episode 1 things it changes:** the Demon and Spectre states (durations 1, 4, 1), the imp's and the Baron's missile speeds (20), every monster's first reaction, every ammunition pickup, and every corpse.

## 3. The `-fast` and `-respawn` parameters

(read) `D_DoomMain` sets `respawnparm` and `fastparm` from the command line (d_main.c:809-810). Apart from Nightmare:

- `-respawn`: `respawnmonsters` on any skill ([§1.3](#13-g_initnew-the-clamp-respawnmonsters-and-the-fast-set-up) step 4). Nothing else: `reactiontime` stays the type's, ammunition is not doubled, `A_Chase` keeps its slow rules. `P_RespawnSpecials` (items) is for deathmatch 2 only (p_mobj.c:591-592) and is not involved.
- `-fast`: the table edit on every `G_InitNew`, and both `A_Chase` rules. `reactiontime` stays the type's.

**The demo header carries both**: bytes 5 and 6 (`G_BeginRecording`, g_game.c:1560-1561; `G_DoPlayDemo`, 1600-1601), next to the skill in byte 1. So a demo recorded with either is a valid input that names it. (run) `REACT3F`, `FAST3F` and `RESP3R` are such demos, each next to the same commands without the byte and on Nightmare.

Today the Rust demo reader requires both bytes 0 (`demo.rs:25`, `203-209`, read). Decision N1.

## 4. The pause

### 4.1 The key and the command

(read) `G_Responder` turns a `KEY_PAUSE` key-down into `sendpause = true` (g_game.c:561-565); `I_GetEvent` maps the X11 Pause key to it (i_video.c:127). `G_BuildTiccmd` then **replaces** the command's buttons with `BT_SPECIAL | BTS_PAUSE`, 0x81, leaving the movement fields as built (g_game.c:426-430). `G_DoLoadLevel` clears `sendpause` and `paused` (g_game.c:494).

`G_Ticker`, after reading every player's command and before the game-state switch, looks at each command with `BT_SPECIAL` (bit 7) and switches on its low two bits (g_game.c:698-724): 1 toggles `paused` and calls `S_PauseSound` or `S_ResumeSound`; 2 is the save command, which sets `ga_savegame`; 0 and 3 do nothing. The other bits are not looked at.

(run) Item `PAUSE`: 0x81 toggles; so do 0x85 and 0xfd (low bits 01 with other bits set); 0x80 and 0x83 do not. Two 0x81 commands in consecutive tics pause for exactly one tic. Planted fault `pause_mask` (only the byte 0x81 pauses): differs at capture 137.

### 4.2 What stops and what keeps running

(read) `P_Ticker` returns at once while `paused` (p_tick.c:135-136): no `P_PlayerThink`, no thinkers, no specials, and `leveltime` does not advance. The toggle comes before the switch, so the tic that pauses already skips the level, and the tic that unpauses already runs it. Everything else in the `GS_LEVEL` case runs: `ST_Ticker`, `AM_Ticker`, `HU_Ticker` (g_game.c:729-734). `WI_Ticker` and `F_Ticker` are not behind any pause test (736-742). `gametic` is incremented by the loop, not by `P_Ticker` (d_main.c:384).

(run) Item `PAUSE`, four paused intervals, 214 paused tics; `PAUSE0`, paused from session tic 0:

| While paused | Evidence |
| --- | --- |
| `leveltime` and the `P_Random` index stand still; every dump's game rows are equal | 72 for 41 tics; index 71 |
| `gametic` advances by 1 a tic | 73 to 113 |
| the `M_Random` index advances by 1 a tic in a level (`ST_Ticker`, st_stuff.c:992) | 139 to 179 |
| the status bar face changes | the `P` rows of the frame stream |
| the message timer runs out (`HU_Ticker`, hu_stuff.c:512) and the pickup message leaves the screen | the longest interval; planted fault `pause_hu` differs in the frames only |
| held movement, turning and fire in the paused commands do nothing | the 40 commands of the first interval |
| a demo command is consumed every tic | the demo's length |

Planted faults: `pause_run` (`P_Ticker` runs while paused) differs at capture 73, and at capture 1 in `PAUSE0`; `pause_st` (`ST_Ticker` stopped) differs in the session stream's `rndindex` only.

**A special byte on a tic the level runs** (read, run). That is the tic that ends a pause, and any tic with a special byte that is not the pause command (0x80 and 0x83 in `PAUSE`). `P_PlayerThink` runs with the command. A live player's movement fields apply; then "a special event has no other buttons" zeroes the byte (p_user.c:280-281), so the fire and use latches see a release. A **dead** player returns before that line (p_user.c:258-262) and `P_DeathThink` reads the raw byte (p_user.c:227). A byte that toggles the pause has low bits 01 and so no use bit; a byte with low bits 11, such as 0x83, has it and asks a dead player for the reborn (read, not run). Planted fault `pause_btn` (the zeroing removed) fires the pistol on the unpausing tic: capture 114.

**In the intermission** (read, run item `PAUSEWI`). `WI_checkForAccelerate` reads the raw byte (wi_stuff.c:1480, 1488): 0x81 has bit 0, so the pause command is also a fire press and skips a tally stage. The tally goes on while paused. The next level's `G_DoLoadLevel` clears `paused` without `S_ResumeSound`, and `S_Start` clears the sound code's own `mus_paused` flag without `I_ResumeSong` (s_sound.c:214): the M6b stream shows a pause call and no resume before the load. In the intermission and the finale there is no status bar draw, but the `M_Random` index can still move while paused: the tally's sounds draw their pitch (wi_stuff.c:1343, 1351-1352; s_sound.c:337-340), and the finale's change to its art screen starts a melt. In `PAUSEWI` the index moves on the pausing tic itself (24 to 25: the fire press ends a tally stage with a sound) and not during the two paused intervals. Planted faults `pause_keep` and `pause_wi`: capture 127.

**In the finale** (run item `PAUSEFIN`): `F_Ticker` goes on, the text keeps typing, the picture is drawn over the text and over the art screen, and the demo ends paused.

**Not behind the pause** (read): the turbo message test reads the command and `gametic` every tic (g_game.c:672-679), so it can fire while paused, and `HU_Ticker` then shows it; no item reaches it (it needs a forward value above the keyboard's 50, which a foreign demo may hold). In the Rust code this test is **inside the simulation** (`crates/doom-sim/src/demon_expanded_map.rs:819-835`), against a `gametic` computed as the load's plus the level's tic count (`carried_start.rs:596-606`), which is wrong after any pause. `gametic` advances in the loop: d_main.c:384 with `-singletics`, d_net.c:745 otherwise. The menu's own stop, `menuactive` outside a net game and outside demo playback (p_tick.c:139-145), is a second way to stop `P_Ticker`, with no command and nothing in the demo.

### 4.3 Sound

(read) `S_PauseSound` and `S_ResumeSound` touch music only: `I_PauseSong` and `I_ResumeSong` under a `mus_paused` flag (s_sound.c:497-513), both empty in linuxdoom (i_sound.c:848-858). Nothing pauses sound effects: `S_UpdateSounds` and the mixer run every frame (d_main.c:392-405) with no test of `paused`.

(run) The sound streams of the pause items: channels started before the pause play on and end during it; the `MP` rows give one `I_PauseSong` or `I_ResumeSong` call per toggle in these runs. (read, not run) A music change while paused makes one more: `S_StopMusic` calls `I_ResumeSong` when `mus_paused` is set and leaves the flag (s_sound.c:689-696), which a pause command on the very tic a level is completed would reach at the intermission's music change (wi_stuff.c:1508-1515). The port has no music, so the pause changes nothing a listener hears except that nothing new starts.

### 4.4 The picture

(read) `D_Display` draws the frame as always, then, if `paused`, the patch `M_PAUSE` at x = `viewwindowx + (scaledviewwidth - 68) / 2` and y = `viewwindowy + 4`, or y = 4 with the automap up (d_main.c:303-311), in every game state. A missing `M_PAUSE` lump is a fatal `W_CacheLumpName` error on the first paused frame.

(run) The frame of every paused tic of `PAUSE`, `PAUSEWI` and `PAUSEFIN` at screen sizes 10 and 11 and of `PAUSE0` at 10, with our own `M_PAUSE`. The self-test draws the picture one row lower and the first paused frame's digest changes. **(read, not run)**: the position at screen sizes below 10, where the view window is inset. The synthetic WAD set lacks the border flat, and the genuine engine stops at load with `W_GetNumForName: FLOOR7_2 not found!` (observed). At sizes 10 and 11 the draw call's origin is (126, 4); the patch's own offsets are subtracted from it (v_video.c:219-220).

### 4.5 In a recorded demo

(read) `G_WriteDemoTiccmd` writes the button byte as it is and `G_ReadDemoTiccmd` reads it back (g_game.c:1491-1520); only the first byte of a command is tested against the 0x80 end marker. So a demo carries 0x81 as an ordinary command, the commands of the paused tics are in the file, and a demo may end paused. During `-playdemo` the Pause key does nothing: the command it builds is overwritten by the demo's (g_game.c:664-667).

### 4.6 Two things already in the repository

**`doom-play`'s Esc pause** (read; `crates/doom-game/src/app/mod.rs:1324-1343`, `1383-1388`). Esc and the Pause key both stop the **app**: no session tic runs, so no simulation, display or sound tic; the audio queue is dropped; a dimmed frame with an overlay is shown; nothing is recorded. The game never sees it. `KEY_PAUSE` is withheld from the game's input (`crates/doom-game/src/input/mod.rs:107-124`). The nearest thing in the original is the menu's stop ([§4.2](#42-what-stops-and-what-keeps-running)), which is also outside the demo; it differs in that the original's status bar and sound keep running under a menu. Decision N7.

**Audit 1 item 8a** ([record](validation/AUDIT1.md)). The demo readers keep any button byte; the writers refuse a byte with bit 6 or bit 7 (`crates/doom-sim/src/demo.rs:333`); the session refuses a tic whose command has bit 7 (`crates/doom-game/src/session/mod.rs:558-567`, witness `Special`), and the simulation refuses it too (`player_commands.rs:110-131`). Bit 7 is exactly where the pause command lives. With the pause built, 0x81 and the other bytes of [§4.1](#41-the-key-and-the-command) become admitted commands and the writers must write them; the save command (low bits 10) stays refused until saving exists. Decision N5.

## 5. What must be new

**Simulation: wire 29, `DSIM` 45.** Everything that changes what a level tic does:

- skill 4 admitted, with the options respawn and fast (N1);
- the fast table (N4), the respawn, kills above the total;
- a command byte with bit 7 on a tic the level runs: zeroed for a live player, read raw by a dead one ([§4.2](#42-what-stops-and-what-keeps-running));
- the turbo message: its `gametic` no longer follows from the level's tic count, and the original tests it on paused tics too ([§4.2](#42-what-stops-and-what-keeps-running));
- if the owner places it here, the block-walk overshoot ([§7](#7-two-candidates-from-audit-1)).

**Session.** Everything `G_Ticker` does around the level: the toggle, the level not ticking, the tickers that go on, the load that clears the flag, the intermission's fire press, `gametic` running ahead of `leveltime`. The simulation already has a `paused` field in its canonical state (`snapshot.rs:858`), always 0 so far, and `advance` returns early while it is set (`lib.rs:1353-1362`); who owns the flag is decision N6.

**Formats.** Each of the three session formats has a rule that the pause breaks (read):

| Format | Now | Rule that breaks | New |
| --- | --- | --- | --- |
| `DSIM` | 44 | skill 0 to 3; `killcount` ≤ `totalkills`; durations equal the table's | 45 |
| `DSES` | 3 | no paused flag; skill 0 to 3; "the level capture's tic equals the sound state's level tics" (`state_v2.rs:435-438`) | 4 |
| `DDSP` | 4 | a level scene's tic equals the session tic minus the tic the level was entered (`d_display/session.rs:920`); no paused flag for the picture | 5 |
| `DSND` | 3 | level tics are session tics minus the level's start (`chain_state.rs:332-337`) | 4 |
| Demo reader | "version 3" | skill 0 to 3, parameter bytes 0, writers refuse bit 7 | a version 4 reader and writer |

The demo **file** format is the original's and does not change; "version" names what the port's reader admits.

## 6. Evidence plan

### 6.1 The genuine side, built in this phase

| File | What |
| --- | --- |
| `tools/reference/m6b_build.py`, `m6b_session.c` | The M6b runner: the M6a runner's genuine files, adapters, stubs and flags, with a generated driver. Seven exact-once patches of the driver and one of the M6a session code put the five lines of d_main.c:303-311 where the M5b and M6a display mirrors aborted on a paused frame, observe `I_PauseSong` and `I_ResumeSong`, add `--prior-game`, and write the M6b stream. Contract: `tests/reference/m6b_runner_contract.json`. |
| `tools/reference/m6b_wad.py` | Eight synthetic map slots, our own `M_PAUSE`, 19 demo scripts. Nothing from any IWAD. |
| `tools/reference/m6b_goldens.py` | Runs every item in the strict, overflow and address builds (identical dumps and streams required), the pause items and the two over-100 % items rendered, 24 planted-fault variants, 7 table runs, 2 self-tests; writes or `--check`s the goldens. |

**One new declared sanitizer adapter.** `mo->angle = ANG45 * (mthing->angle/45)` at p_mobj.c:400 overflows `int` for a spawn angle of 180 or more. The frozen adapters cover the same statement at lines 671 and 794; no run reached line 400 before. UBSan stopped the unadapted overflow build on the first respawn of a demon placed facing west (run). The adapter is the unsigned product, in the overflow and address builds only; the strict build compiles the original text and is the claim.

**One declared address stop.** `PAUSEWI` loads a second level, where the address build reports the M6a-declared `st_stuff.c:1292` index (the key boxes). `m6a_goldens.py` runs such items in a build that recovers from that one report; here the address build of `PAUSEWI` must show exactly that report after a prefix of the strict dumps (127 captures), and the overflow build runs it whole. The address runs use `detect_leaks=0`: leak detection is disabled, which is not LeakSanitizer coverage.

**Goldens** (`tests/reference/`): `m6b_genuine.txt` (per item the header bytes, tics, captures and digests of the dump set and of each stream; every resolved pass of the respawn gate; the table runs; the first differing place of every variant), `m6b_session/`, `m6b_sound/`, `m6b_frames/`, `m6b_pause/` (the frame digest of every paused tic) and `m6b_results.json` (the observed facts quoted in this note).

**Left out, and what it waits on.** State digests over the wire 29 schema. Their shape depends on N2 (which wire carries what), N4 (how the table is carried), N5 (which bytes a command may hold) and N6 (who owns `paused`). The dumps they will be projected from are pinned now by digest, so Phase 2 adds an encoder and no new genuine run. Mixer goldens for the pause items are also left to Phase 2: they are made from the committed sound streams by the frozen M5c mixer harness and add no genuine fact, but their placement in the port's buffer count depends on N6.

### 6.2 Phase 2 evidence

1. **Synthetic items against the genuine engine**, in CI: the 19 items through an independent `DSIM` 45 encoder, state for state; the session, sound and frame streams as they are; each planted fault as a Rust test that a port behaving like the variant is caught at the `V` row's place.
2. **Real maps, local only.** The 61 committed runs replayed with the skill byte 4, each to its last command. They are valid inputs even where the player now dies or misses the exit; both engines must agree on every tic. Counts and digests only. Also with the `respawnparm` and `fastparm` bytes if N1 admits them, and with pause commands inserted in a level (the level's rows must then equal the unpaused run's, shifted).
3. **The owner's hand test** (N8).
4. **Restore and continue**: save and restore at a paused tic, on the tic a corpse's gate passes, between the two fogs' lifetimes, and across the load that clears a pause; continue and compare with the unbroken run.
5. **Fuzz targets** for each new reader (`DSIM` 45, `DSES` 4, `DDSP` 5, `DSND` 4, the demo reader), seeded from the goldens; a sanitizer campaign on the same.
6. **Mutation testing** with the cross-crate method now in CI (a `doom-sim` mutant judged by `doom-sim`'s and `doom-game`'s tests), on the diff.
7. **An independent read** by a fresh reviewer of the diff against the C, without this note's reasoning.

## 7. Two candidates from audit 1

**The block-walk overshoot wrap** (p_maputl.c:873; [audit record §4](validation/AUDIT1.md)). Facts (read from the record): when `P_PathTraverse`'s walk over the block map overshoots (p_maputl.c:846-876) and a far line's intercept wraps, the genuine engine drops the wrapped intercepts and plays on; wires 23 to 28 refuse that tic (`MapScale(Arithmetic)`) and keep the state. A genuine golden exists (item `WRAP`). It is reachable by ordinary play in principle and was not observed on a real map. The audit recommended no wire for it alone. Nightmare does not make it more or less reachable by any rule in [§1](#1-nightmare-site-by-site) (inferred). Decision N9.

**The quirk-6 witness** (percentages above 100). Nightmare makes it reachable and this phase ran it ([§1.7](#17-kills-past-100--at-the-intermission)). The simulation's reader must stop refusing `killcount` above `totalkills` for any real Nightmare play, so the witness is not optional once Nightmare is in. Decision N10.

## 8. Decisions for the owner

Each has the facts, the options and one recommendation. Nothing in Phase 2 starts before the rulings.

### Rulings

The owner ruled on all eleven on 2026-11-26 (America/Los_Angeles). The decisions below are kept as they were put to him.

| Decision | Ruling |
| --- | --- |
| N1 | (a): `-fast` and `-respawn` are in, as two options of wire 29, with `doom-play` flags. |
| N2 | (a): one wire, 29, carries Nightmare and the pause's simulation part. |
| N3 | (a): one pull request. If the line count reaches the ceiling before the pause is built, the pause moves to wire 30 in a second pull request. |
| N4 | (a): the fast table is a function of the options. A `doom-play` restart counts as a new process. The other table histories stay in the quirk register with their goldens, to be reopened when menus, saves or cheats can reach them. |
| N5 | (a): every command byte with bit 7 is admitted except the save command (low bits 10). The recorder writes only 0x81. Bit 6 alone stays as audit 1 left it. |
| N6 | (a): the session owns the paused flag. New versions: `DSIM` 45, `DSES` 4, `DDSP` 5, `DSND` 4. |
| N7 | (a): the Pause key sends the original's command. Esc and focus loss keep the app's own pause, outside the demo. |
| N8 | (a): one hand test by the owner before the pull request is marked ready. |
| N9 | (a): the p_maputl.c:873 block-walk overshoot is admitted under wire 29, as the last step, and drops to a later wire if the ceiling is near. |
| N10 | (a): kill percentages above 100 are in, with `RESP4` and `RESP3R` as the witness. |
| N11 | The hand-written line ceiling for Phase 2 is 13,000, counted with `git diff --numstat` from the merge base, added lines, without `tests/reference` and `Cargo.lock`. The ceiling decides which pull request something lands in, never whether it lands. |

**Cheats.** The original does not lock cheats on Nightmare: the test at st_stuff.c:546 is commented out ([§1.8](#18-the-menu-and-the-status-bar)). Cheats are not in this milestone, and no Nightmare lockout is added anywhere.

**N1. Scope: are `-fast` and `-respawn` in M6b?** Facts: they are the same code as Nightmare with the conditions split ([§3](#3-the--fast-and--respawn-parameters)); the demo header carries them; three genuine items exist. Their one hazard is the table history of [§1.3](#13-g_initnew-the-clamp-respawnmonsters-and-the-fast-set-up), which N4 handles. Options: (a) in, as two options of wire 29, with `doom-play` flags; (b) in the simulation and the demo reader only, no `doom-play` flag; (c) out: the bytes stay refused and a later wire adds them. **Recommendation: (a).** The cost is two booleans where the skill test already is, and leaving them out costs another wire later for the same lines.

**N2. One wire or two?** Facts: Nightmare is a simulation change; the pause's simulation part is small: the rule about a command byte and the turbo message's `gametic` ([§5](#5-what-must-be-new)). A wire is frozen when its pull request merges, so two wires allow two pull requests and one wire means one. Each wire costs a profile, a `DSIM` version, an encoder, a fuzz target and their documents. Options: (a) one wire 29 with both; (b) wire 29 Nightmare, wire 30 the pause. **Recommendation: (a).** The two parts do not interact in the simulation, and one wire makes one set of format versions instead of two.

**N3. One pull request or split, and where?** Facts: the estimate is about 10,900 lines ([§9](#9-step-plan-and-size)); M6a's rule was that the cap decides which milestone an item lands in, never whether it lands. Options: (a) one pull request for Phase 2; (b) two, split between Nightmare and the pause (which needs N2 (b)). **Recommendation: (a), with M6a's fallback:** if the count reaches the ceiling of N11 before the pause is built, the pause moves to wire 30 in a second pull request and the owner is told then. This Phase 1 pull request stays separate and merges first.

**N4. How the fast table is carried.** Facts: with plain skills and new games only, the table is that of the current skill (three histories run, the rest read). With `-fast`, an earlier game of the same process can leave three other tables (run), two of which make states last 0 tics; a loaded save or the level-warp cheat can leave others (read). `doom-play` has no menu, no demo loop, no saves and no cheats; its restart is a new session. Options: (a) the table is a function of the options (fast if the skill is Nightmare or the fast option is set), a `doom-play` restart counts as a new process as it does today, and the other tables are recorded in the quirk register with their goldens and land with the menus, saves and cheats that reach them; (b) the sixteen values are saved state and every history is reproduced, with a refusal where a state would never end. **Recommendation: (a).** It is exact for everything the app can do, adds no saved state, and loses nothing: the goldens for (b) are already committed.

**N5. Which special bytes a command may hold.** Facts: [§4.1](#41-the-key-and-the-command), [§4.2](#42-what-stops-and-what-keeps-running); the item `PAUSE` covers 0x81, 0x85, 0xfd, 0x80 and 0x83. Bit 6 without bit 7 is read by nothing in a level (read, not run) and is refused today. Options: (a) admit every byte with bit 7 except the save command (low bits 10), as the original treats them; the recorder writes only 0x81; (b) admit 0x81 alone. **Recommendation: (a).** A foreign demo may hold any of them and the genuine runs exist. The save command stays refused until saving exists, and bit 6 alone stays as audit 1 left it.

**N6. Who owns `paused`, and the format versions.** Facts: the original has one global that `G_Ticker` toggles and `G_DoLoadLevel` clears; the simulation's canonical state has a `paused` byte that has always been 0. Options: (a) the session owns it (`DSES` 4); a paused level tic does not call the simulation, whose own byte stays 0; (b) the simulation's byte mirrors it. **Recommendation: (a)**, with the new versions of [§5](#5-what-must-be-new): `DSIM` 45, `DSES` 4, `DDSP` 5, `DSND` 4. One owner, as in the original. It has one cost, the turbo message: under (a) the session must give the simulation the true `gametic` (the simulation saves how far `gametic` is ahead of its own tic count, new state in `DSIM` 45) and must make the message's test itself on a paused tic, through a simulation call that changes the message and nothing else.

**N7. The Pause key and Esc in `doom-play`.** Facts: [§4.6](#46-two-things-already-in-the-repository). Options: (a) the Pause key sends the original's command (recorded, the picture, sounds play out); Esc and losing focus keep the app's own pause, which stays outside the demo, until menus exist; (b) both keys send the original's command; (c) a setting. **Recommendation: (a).** The Pause key is what the original binds, and the app's stop is still needed for focus loss. While the app is stopped the Pause key does nothing; Esc works during an original pause and stops the app on top of it.

**N8. The hand-test gate.** Options: (a) one hand test before the pull request is marked ready: the owner plays E1M1 on Nightmare until he has seen a respawn, uses the Pause key in a level and in an intermission, and the recordings replay in the genuine engine tic for tic; (b) none. **Recommendation: (a)**, as for both parts of M6a.

**N9. The block-walk overshoot.** Options: (a) admit it under wire 29, as the last step, with its existing golden; (b) leave it refused. **Recommendation: (a), last and droppable:** a wire is being cut anyway and the golden exists, so the refusal can become the original's behaviour for about 500 lines; if the ceiling of N11 is near, it waits for the next wire and nothing else is held up.

**N10. The quirk-6 witness.** Options: (a) percentages above 100 are in M6b, with the committed `RESP4` and `RESP3R` runs as the witness; the "sucks" half stays open; (b) later. **Recommendation: (a).** The reader's guard has to go for Nightmare in any case, and the witness is already run.

**N11. A line ceiling per pull request.** Counted as before: `git diff --numstat` from the merge base, added lines, without `tests/reference` and `Cargo.lock`. **Recommendation:** 3,000 for this Phase 1 pull request and 13,000 for Phase 2 (the estimate plus a fifth).

## 9. Step plan and size

Steps of Phase 2, each pushed when done and compared with the genuine engine before the next:

1. Wire 29's profile and options; skill 4 admitted through the eight refusals; the `DSIM` 45 encoder (Python, independent) and reader.
2. Double ammunition, the spawn bit and `reactiontime` on Nightmare: `AMMO4`, `BITS4`, `REACT4`.
3. The fast table and `A_Chase`'s `fastparm` half: `FAST4`, `FAST3F`, `REACT3F`.
4. The respawn: `RESP4`, `RESP3R`, `BLOCK4`, `GIB4`; kills above the total; the intermission past 100 %.
5. Real maps on skill 4, local.
6. The pause in the simulation's command domain and the turbo message's `gametic`; `DSES` 4 with the flag; the session's paused tic.
7. `DDSP` 5 and the picture; `DSND` 4; the pause items' frames, sound and mixer.
8. The demo reader and writer, the recorder, the Pause key and the `doom-play` flags.
9. Restore-and-continue, fuzz targets and campaigns, mutation testing.
10. The overshoot (N9), if placed here.
11. Documents, the evidence record, the independent read, the hand test.

| Part | Hand-written lines (estimate) |
| --- | --- |
| Phase 1 (this pull request): tools 1,250, this note and the other documents 750 | 2,000 |
| Wire 29 profile, options, skill 4 admission | 500 |
| The respawn | 450 |
| The fast table and its validators | 400 |
| `DSIM` 45: reader, writer, the Python encoder | 600 |
| The pause: simulation command domain and the turbo message 400, `DSES` 4 500, `DDSP` 5 and the picture 550, `DSND` 4 300 | 1,750 |
| Demo reader and writer, recorder, input, `doom-play` key and flags | 600 |
| Tests: simulation 900, session 900, restore-and-continue 400, mutation survivors 600 | 2,800 |
| Fuzz targets and seed tools | 600 |
| Tools: wire 29 goldens, the real-map tool, the hand-test check | 900 |
| The overshoot (N9) | 500 |
| Documents and the evidence record | 1,800 |
| **Phase 2 total** | **10,900** |

The estimate is inferred from the sizes of wire 28 and M6a part 2's steps; the test and document rows are the least certain.

## 10. What was run and what was not

**Run** (2026-11-26, cc 15.2.0, Linux; binaries under `~/scratch/m6b`, deleted at the end of the phase): `m6b_wad.py --check`, `m6b_build.py --check`, `m6b_goldens.py --selftest` and `--check`, all exit 0 at the committed state. Counts: 19 items, each in three builds; 6 items also rendered (9 rendered runs); 37 variant rows from 24 variants; 7 table runs; 2 self-tests.

**Exited nonzero while authoring**, each resolved before the commit: the overflow and address builds at p_mobj.c:400 (the new adapter); the address build of `PAUSEWI` at st_stuff.c:1292 (the declared stop); a rendered run at screen size 9 (`FLOOR7_2` missing, so size 9 is not run); the variant `gate` on `BLOCK4`, which equalled the genuine run and was removed from that variant's items (a gate value of 4 was blocked anyway); a 9-letter item name, which the engine refuses as a lump name.

**Not run:** anything in Rust; any real map (no IWAD was read in this phase); the picture's position below screen size 10; the turbo message during a pause; a special byte on a dead player's tic; bit 6 alone; the save command; a table with 0-tic states against a monster that cannot reach its target; a table after a loaded save or the level-warp cheat; a second boss death on E1M8; a music change while paused; timings. An independent reviewer (a fresh agent that did not write this note) checked the citations and a sample of the run claims against the source and the goldens on 2026-11-26; its 23 findings are folded in above.
