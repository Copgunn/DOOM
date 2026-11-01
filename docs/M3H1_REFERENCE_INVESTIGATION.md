# M3h1 — Original former humans and multi-monster encounters

This note records source facts, their mapping to existing Rust owners and the
admitted boundary for `PlayerMonstersV1`. It was written before the runtime
change. Support and limits belong to
[COMPATIBILITY](COMPATIBILITY.md#original-former-humans-and-multi-monster-encounters);
DSIM28 belongs to [STATE_FORMAT](STATE_FORMAT.md#dsim-v28-original-former-humans-and-mixed-rosters);
measured observations belong to [the evidence record](validation/M3H1.md).
All references are to the preserved `linuxdoom-1.10/` sources.

## Actor definitions (info.c)

| Type (ordinal) | DoomEd | Health | Speed | Pain chance | Sounds | Flags |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| MT_POSSESSED (1), info.c:1134 | 3004 | 20 | 8 | 200 | posit1 / pistol / popain / podth1 / posact | SOLID, SHOOTABLE, COUNTKILL |
| MT_SHOTGUY (2), info.c:1160 | 9 | 30 | 8 | 170 | posit2 / – / popain / podth2 / posact | SOLID, SHOOTABLE, COUNTKILL |
| MT_CHAINGUY (10), info.c:1368 | 65 | 70 | 8 | 170 | posit2 / – / popain / podth2 / posact | SOLID, SHOOTABLE, COUNTKILL |

All three are radius 20, height 56, mass 100, reaction time 8 and have no
melee state. Their sprites/frames are the POSS/SPOS/CPOS records in info.c.

### State closures (info.c:310–571)

- Spawn: two 10-tic `A_Look` states looping (`S_xPOS_STND`, `STND2`).
- See: eight `A_Chase` states looping; 4 tics for POSS, 3 for SPOS/CPOS.
- Missile: POSS `ATK1` `A_FaceTarget` 10 → `ATK2` `A_PosAttack` 8 → `ATK3` 8 →
  run; SPOS `A_FaceTarget` 10 → `A_SPosAttack` 10 (fullbright) → 10 → run;
  CPOS `A_FaceTarget` 10 → `A_CPosAttack` 4 → `A_CPosAttack` 4 → `A_CPosRefire`
  1 → back to `ATK2`.
- Pain: 3 tics, then `A_Pain` 3 tics, then run.
- Death: 5-tic states with `A_Scream` on the second and `A_Fall` on the third,
  ending in a `-1` corpse. XDeath: `A_XScream` on the second, `A_Fall` on the
  third, ending in a `-1` corpse.
- Raise (`S_xPOS_RAISE*`) is reachable only from `A_VileChase`; it is excluded.

The Rust registry derives each closure once from spawn/see/pain/melee/missile/
death/xdeath roots, rejects any other callback and proves the raise root is not
reachable (`original_registry.rs` former-human policy).

## Actions (p_enemy.c)

| Source | Facts | Rust owner |
| --- | --- | --- |
| `A_Look` (p_enemy.c:604) | Sector soundtarget first, then `P_LookForPlayers(actor, false)`; seesound `sfx_posit1..3` draws `P_Random()%3`. | `perception::look`, unchanged |
| `A_Chase` (p_enemy.c:672) | reaction/threshold countdown, turn, retarget via `P_LookForPlayers(true)` when the target is not SHOOTABLE, JUSTATTACKED, melee, missile when `movecount == 0` and `P_CheckMissileRange`, then move and `P_Random() < 3` active sound. | `pursuit::chase` and `monsters::select`, unchanged order |
| `P_CheckMissileRange` (p_enemy.c:197) | Sight, JUSTHIT shortcut, reaction time, `dist - 64 - (no melee ? 128 : 0)`, clamp 200, `P_Random() < dist` rejects. | `monsters::missile`; the checked expanded sight query is used when a bounded operation control exists |
| `A_FaceTarget` (p_enemy.c:782) | Clear AMBUSH, face target, SHADOW adds two draws. | `monsters::face`, unchanged |
| `A_PosAttack` (p_enemy.c:802) | Face, aim at `MISSILERANGE`, pistol sound, `(P_Random()-P_Random())<<20` spread, `((P_Random()%5)+1)*3` damage, one `P_LineAttack`. | shared `weapons::monster_shot` and `player_monsters::attack` |
| `A_SPosAttack` (p_enemy.c:821) | Shotgun sound, face, one aim, three complete bullets with their own draws. | same shared path, three pellets |
| `A_CPosAttack` (p_enemy.c:845) | Shotgun sound, face, aim, one bullet. | same shared path |
| `A_CPosRefire` (p_enemy.c:865) | Face, `P_Random() < 40` continues, otherwise missing/dead/not visible target enters seestate. | `chaingunner::decide`, with checked sight |
| `A_SargAttack` (p_enemy.c:935) | Face, `P_CheckMeleeRange`, `((P_Random()%10)+1)*4`, `P_DamageMobj`. | `demon::strike`; monster targets use the shared current damage plan |
| `A_Pain`, `A_Scream`, `A_XScream`, `A_Fall` | Pain sound; podth1..3 grouped death sound draws `P_Random()%3`; slop; clear SOLID. | `LifeHost::action`, unchanged |

Dispatch remains the single `LifeHost::action` table keyed by original action
name. Former-human attacks branch there to one current-state implementation;
there is no per-species adapter or second scheduler.

## Damage, infighting and death (p_inter.c)

- `P_DamageMobj` (p_inter.c:775) applies thrust from the inflictor, player armor
  and health, the pain draw, `reactiontime = 0`, then retargets when
  `(!threshold || type == MT_VILE) && source && source != target &&
  source->type != MT_VILE` (p_inter.c:904): `target = source`,
  `threshold = BASETHRESHOLD` (100, p_local.h:61), and spawnstate actors enter
  seestate. This already exists in the shared `LifeHost::apply_damage`; M3h1 only
  lets monster sources and monster targets reach it. No MT_VILE is admitted, so
  both VILE terms are always false. There is no same-species rule in the source.
- `P_KillMobj` (p_inter.c:668) clears SHOOTABLE/FLOAT/SKULLFLY, sets CORPSE and
  DROPOFF, quarters height, credits `players[0].killcount` for every COUNTKILL
  death in single player (including monster-caused deaths), enters the death
  state, subtracts `P_Random()&3` from its timer, and finally drops
  MT_CLIP / MT_SHOTGUN / MT_CHAINGUN for POSSESSED / SHOTGUY / CHAINGUY with
  `P_SpawnMobj(x, y, ONFLOORZ)` and `MF_DROPPED` (p_inter.c:736–757).
  `P_SpawnMobj` draws `lastlook = P_Random() % MAXPLAYERS` (p_mobj.c:506).
  Rust adds this drop to the existing kill branch after the death timer, using
  `LifeHost::spawn`; the existing pickup service already halves dropped ammo.

## Spawn and thinker order (p_mobj.c)

`P_SpawnMapThing` (p_mobj.c:708) applies the skill/single-player filter, spawns,
randomizes positive tics and increments `totalkills` for COUNTKILL (p_mobj.c:790)
in THING order. Thinkers run in insertion order; drops and impacts are appended.
The existing atomic THING batch and shared thinker pass carry these unchanged.

## Historical fixture profiles

M3d3c/d/e implemented `A_PosAttack`, `A_SPosAttack` and `A_CPosAttack/Refire` on
synthetic single-owner fixtures with a pure preflight plan. Those profiles,
their fixtures, DSIM versions and plan checks stay unchanged. M3h1 shares the
source formula (`weapons::monster_shot`), the trace/aim/hit algorithms
(`combat_trace`), the impact/damage executor (`combat::execute_attack`,
`LifeHost::apply_damage`), the refire decision (`chaingunner::decide`) and the
event types, but computes each attack from the current staged transaction,
because several shooters per tic can change each other's targets and RNG.

## Admitted boundary

`PlayerMonstersV1` (wire 12, context `PLAYER-MONSTERS-1-MEDIUM-GROUNDED-v1`,
DSIM28) is a superset of `PlayerDemonBonusesV1`:

- Up to eight map-THING monsters of MT_SERGEANT, MT_POSSESSED, MT_SHOTGUY and
  MT_CHAINGUY in any mix (`MAX_PLAYER_MONSTERS`). With one player, at most 16
  placed items, at most eight drops and transient impacts this fits the 64-slot
  store; each tic's checked sight budget (64 queries) covers eight monsters.
- Infighting, kill credit and drops exactly as above.
- Excluded: raise/resurrection, MT_VILE, nightmare/fast monsters, respawn,
  projectiles, other monster types, moving sectors, multiplayer.
- Older policies, wires, DSIM20–27 and their fixtures are unchanged; they keep
  rejecting the former humans and monster drops.

## Evidence plan

`tools/reference/m3h1_reference.py` reuses the inherited genuine harness: all
monsters are ordinary THINGs; attacks, `P_DamageMobj`, `P_KillMobj` and
`P_SpawnMobj` are compiled original bodies. Two read-only observer adapters
were added: shotgunner rows are renamed `SG` to separate them from player weapon
rows, and `SR` attributes the existing `A_Scream` grouped draw.
`tools/reference/m3h1_snapshot_goldens.py` independently projects DSIM28 bytes
and ordered events from those rows. Both were committed before the runtime.
