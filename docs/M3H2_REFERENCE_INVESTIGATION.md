# M3h2 — Original Imp, fireballs and roster maintenance

This note records source facts, their mapping to existing Rust owners and the
admitted boundary for `PlayerMonstersImpV1`. It was written and committed before
the runtime change. Support and limits belong to
[COMPATIBILITY](COMPATIBILITY.md#original-imps-and-fireballs); DSIM29 belongs to
[STATE_FORMAT](STATE_FORMAT.md#dsim-v29-original-imps-and-fireballs); measured
observations belong to [the evidence record](validation/M3H2.md). All references
are to the preserved `linuxdoom-1.10/` sources.

## Actor definitions (info.c/info.h)

| Field | MT_TROOP (11), info.c:1394 | MT_TROOPSHOT (31), info.c:1914 |
| --- | --- | --- |
| DoomEd | 3001 | -1 (never a THING) |
| Health / mass / reaction time | 60 / 100 / 8 | 1000 / 100 / 8 |
| Speed | 8 (map units per step) | `10*FRACUNIT` (fixed per tic) |
| Radius / height | 20 / 56 | 6 / 8 |
| Damage | 0 | 3 |
| Pain chance | 200 | 0 |
| Sounds | see `sfx_bgsit1`, pain `sfx_popain`, death `sfx_bgdth1`, active `sfx_bgact`; no attack sound | see `sfx_firsht`, death `sfx_firxpl` |
| Flags | SOLID, SHOOTABLE, COUNTKILL | NOBLOCKMAP, MISSILE, DROPOFF, NOGRAVITY |
| States | spawn, see, pain, melee = missile, death, xdeath, raise | spawn, death; every other root is S_NULL |

### State closures (info.c:233–237, 578–607)

Ordinals are from the info.h enumeration; sprites are `SPR_TROO` (0) and `SPR_BAL1` (18).

- Imp spawn `S_TROO_STND`(442): two 10-tic `A_Look` states looping.
- See `S_TROO_RUN1`(444): eight 3-tic `A_Chase` states looping.
- Melee and missile share `S_TROO_ATK1`(452): `A_FaceTarget` 8 → `A_FaceTarget` 8 →
  `A_TroopAttack` 6 → run. The one attack state makes the melee/missile choice itself.
- Pain `S_TROO_PAIN`(455): 2 tics, then `A_Pain` 2 tics, then run.
- Death `S_TROO_DIE1`(457): 8, `A_Scream` 8, 6, `A_Fall` 6, `-1` corpse (461).
- XDeath `S_TROO_XDIE1`(462): 5, `A_XScream` 5, 5, `A_Fall` 5, 5, 5, 5, `-1` corpse (469).
- Raise `S_TROO_RAISE1`(470) is reachable only from `A_VileChase`; it is excluded.
- Fireball `S_TBALL1`(97)/`S_TBALL2`(98): two 4-tic fullbright frames looping.
- Explosion `S_TBALLX1`(99), `X2`, `X3`(101): three 6-tic states, then S_NULL.

No fireball state has an action. The Rust registry derives each closure once from
its roots, rejects any other callback and proves the raise root is unreachable.

## Actions (p_enemy.c)

| Source | Facts | Rust owner |
| --- | --- | --- |
| `A_TroopAttack` (p_enemy.c:913) | No target returns. `A_FaceTarget`, then `P_CheckMeleeRange`. Melee: `sfx_claw`, `(P_Random()%8+1)*3`, `P_DamageMobj(target, actor, actor, damage)`, return. Otherwise `P_SpawnMissile(actor, target, MT_TROOPSHOT)`. Never a draw in the choice itself. | new `player_monsters::troop_attack` using `monsters::face`, `monsters::melee_host`, shared `DamagePlan`, shared missile service |
| `P_CheckMeleeRange` (p_enemy.c:174) | `dist >= MELEERANGE-20*FRACUNIT+target radius` rejects, then sight. Boundary 60 units against the player, reached by `imp_melee_boundary`. | `monsters::melee_host`, unchanged |
| `A_FaceTarget` (p_enemy.c:782) | Clear AMBUSH, face, SHADOW adds two draws (no SHADOW actor is admitted). | `monsters::face`, unchanged |
| `A_Chase`, `P_CheckMissileRange` | An Imp has a melee state, so the `-128` near-range adjustment does not apply. Attack selection is shared. | `pursuit::chase`, `monsters::select`, unchanged |
| `A_Look` | Imp see sound `sfx_bgsit1..2` draws `P_Random()%2`. | `perception::look`, unchanged |
| `A_Scream` (p_enemy.c:1535) | `sfx_bgdth1..2` draws `P_Random()%2`. | `LifeHost` scream arm, unchanged |
| `A_XScream`, `A_Pain`, `A_Fall` | `sfx_slop`, pain sound, clear SOLID. | `LifeHost` arms, unchanged |

## Missile lifecycle (p_mobj.c, p_map.c)

- `P_SpawnMissile` (p_mobj.c:889): `P_SpawnMobj(source x, y, z + 32 units)` draws
  `lastlook`; see sound; `th->target = source`; angle to `dest`; a SHADOW `dest` adds
  `(P_Random()-P_Random())<<20`; momentum `FixedMul(speed, cos/sin)`;
  `dist = P_AproxDistance / speed`, at least 1; `momz = (dest z - source z) / dist`;
  then `P_CheckMissileSpawn`.
- `P_CheckMissileSpawn` (p_mobj.c:868): `tics -= P_Random()&3`, minimum 1; raw half-step
  of x, y, z by `mom>>1` without relinking; `P_TryMove` to that same position,
  exploding when it fails. A failed move keeps the half-stepped coordinates with the
  old subsector, blockmap and sector links.
- `P_ExplodeMissile` (p_mobj.c:90): clear all momentum, enter the death state,
  `tics -= P_Random()&3` minimum 1, clear MF_MISSILE, death sound.
- `P_MobjThinker` (p_mobj.c:415): XY when momentum is nonzero, Z when `z != floorz` or
  `momz`, then ordinary state countdown. Thinkers run in list order, so a fireball
  launched by an Imp thinks in the same pass, after every older thinker.
- `P_XYMovement` (p_mobj.c:114): clamp to MAXMOVE; one step unless a component exceeds
  `MAXMOVE/2` (a 10-unit fireball never splits); blocked missiles explode unless the
  sky hack applies; no friction for missiles.
- `P_ZMovement` (p_mobj.c:246): `z += momz`; floor or ceiling contact explodes a live
  missile. NOGRAVITY keeps `momz` unchanged, including after the explosion, when the
  state still runs with `z != floorz` and momentum zero.
- `PIT_CheckLine` (p_map.c:189): one-sided lines always stop a missile; BLOCKING and
  BLOCKMONSTERS flags are skipped for MF_MISSILE.

### PIT_CheckThing missile branch (p_map.c:252)

In order, for a candidate with SOLID, SPECIAL or SHOOTABLE inside the summed radii:

1. `z > thing z + height` passes overhead; `z + height < thing z` passes underneath.
2. Same species as the owner (or Knight/Baron): the owner itself passes. Any other
   same-species non-player returns false — **explodes, draws nothing, no damage, no
   retarget**. Only the owner's *type* matters, so a dead owner still applies.
3. Not SHOOTABLE: `return !(flags & MF_SOLID)`, so items pass.
4. `damage = ((P_Random()%8)+1)*tmthing->info->damage`, range 3–24.
5. `P_DamageMobj(thing, tmthing, tmthing->target, damage)`, then return false. The
   inflictor is the fireball (thrust origin), the source is the Imp (retarget and
   `player->attacker`), even when the Imp is already a corpse.

The damage runs inside the block iterator, before `P_TryMove` reports failure, so a
kill, drop or pain state precedes the explosion. Lines are checked only when every
thing passed. `P_DamageMobj`'s `damage < 40 && damage > health && target z - inflictor
z > 64` branch cannot fire: a grounded target is below the 32-unit fireball.

Infighting needs no new rule: P_DamageMobj retargets to `source` when the threshold
is zero, exactly as M3h1 records. The same-species exemption above is the only source
of Imp fireball immunity; a *hitscan* or claw from an Imp's own kind has no exemption.

## Mapping to the existing projectile service

`projectiles.rs` (historical M3d2b plasma, profile 7) already implements launch,
half-step, explosion, the contact query, XY and Z. Those algorithms are source-generic;
what is plasma-specific is the fixture around them:

| Concern | Shared after M3h2 | Frozen historical |
| --- | --- | --- |
| Contact query (`PIT_CheckThing` missile branch) | yes, one function over any actor lookup | |
| Launch tail: sound, owner, shadow, momentum, spawn draw, half-step, spawn check | yes | |
| Explosion, try-move, XY step, Z step | yes, over a small `MissileWorld` host | |
| Event vocabulary (`ProjectileEvent`, tag 15) | yes, same layout | |
| Request validation, profile 7 bounds, MT_PLASMA/fixture pawn admission, between-tic launch API | | yes, plus profile 7 thinker loop |
| Damage | checked world: `DamagePlan::current_original` | profile 7: `damage_actor` |
| Transaction | checked world: staged plan, latched failure | profile 7: per-operation preflight |

Fixture expectations for M3d2b are the proof that the extraction is behavior-neutral.

## Reachability findings from the observer

- **Blocked spawn needs overlap.** An Imp's half-step is 5 units. Against a thing of
  radius R the fireball contacts below a centre gap of `11 + R`, while an Imp only
  reaches the missile check after a successful step, which needs a gap of at least
  `12 + R` (blockdist `20 + R` minus the 8-unit step). A monster that cannot step
  keeps a nonzero `movecount` and `A_Chase` never reaches the missile check. Walls
  are unreachable the same way (radius 20 versus 6 + 5). `fireball_spawn_blocked`
  therefore overlaps two THINGs and uses a declared controlled target and
  `S_TROO_ATK1` entry.
- **XDeath needs more damage than the world admits.** XDeath requires
  `health < -60`, so one hit above 60; the largest admitted hit is a 40-point Demon
  bite. `imp_xdeath` uses one declared controlled `P_DamageMobj`.
- **Spawn angle.** `ANG45 * (angle/45)` is signed overflow for THING angle 180; the
  committed cases use -180, as M3h1 does, and need no new adapter.

## Admitted boundary

`PlayerMonstersImpV1` (wire 13, context `PLAYER-MONSTERS-IMPS-1-MEDIUM-GROUNDED-v1`,
DSIM29) is a superset of `PlayerMonstersV1`:

- Up to eight map-THING monsters of MT_TROOP, MT_SERGEANT, MT_POSSESSED, MT_SHOTGUY
  and MT_CHAINGUY in any mix.
- Live MT_TROOPSHOT actors, flying or exploding, spawned only by `A_TroopAttack`.
- The geometry is the existing grounded domain: one floor height, one-sided
  axis-aligned ML_BLOCKING walls, no sky, no specials. Every shootable actor stands at
  `z = 0`, so `dest z - source z` is zero and **`momz` is always zero**: a fireball
  flies level at z = 32. Floor and ceiling contact (top 40 is below any admitted
  56-unit-high sector), the sky hack, two-sided line openings and the SHADOW spread
  are unreachable here. The shared code keeps them and M3d2b keeps covering them, but
  this domain makes no observation claim for them.
- Excluded: raise/resurrection, MT_VILE, nightmare/fast monsters, respawn, every other
  projectile and monster type, moving sectors, multiplayer.
- Older policies, wires, DSIM20–28 and their fixtures are unchanged; they keep
  rejecting the Imp and the fireball.

## Evidence plan

`tools/reference/m3h2_reference.py` reuses the inherited genuine harness and adds
compiled `A_TroopAttack`, `P_SpawnMissile`, `P_CheckMissileSpawn` and
`P_ExplodeMissile` (previously abort stubs, so reaching them could not pass silently).
`m3h2_observers.py` adds read-only MS rows around genuine statements.
`tools/reference/m3h2_snapshot_goldens.py` independently projects DSIM29 bytes and
ordered events. Both were committed before the matching runtime.
