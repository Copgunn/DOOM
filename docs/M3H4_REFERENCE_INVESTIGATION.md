# M3h4 — Two-sided lines, static floor/ceiling heights and vertical movement

This note records the original-C facts, the admitted domain, the invariant plan, the
evidence plan and the implementation plan of `PlayerMonstersHeightsV1`. It was written
and committed before any runtime change. Support and limits belong to
[COMPATIBILITY](COMPATIBILITY.md); DSIM31 belongs to [STATE_FORMAT](STATE_FORMAT.md);
measured observations belong to [the evidence record](validation/M3H4.md). All references
are to the preserved `linuxdoom-1.10/` sources at `a77dfb96`. Every number is in map units
unless it says raw (16.16 bits). Line numbers name the first line of the cited statement.

## 1. Starting point

`PlayerMonstersMapScaleV1` ([M3h3](M3H3_REFERENCE_INVESTIGATION.md)) admits one flat sector
`[0,128]`, one-sided axis-aligned `ML_BLOCKING` walls, and a walled vertex rectangle `R`
of up to 2,048 units with a complete BLOCKMAP. Inside that domain every mover's XY box
stays inside `R` and the policy never rejects a legal source move.

The engine below the player policies is not flat. The M3b profile (`multisector.rs`)
already implements the static-sector half of `P_TryMove`: `R_PointInSubsector` lookups
from the real BSP (`Topology::locate`), `P_LineOpening` (`multisector::opening`),
the `tmfloorz`/`tmceilingz`/`tmdropoffz` accumulation with the fit, headroom, step and
dropoff decisions in source order (`check_move_from_things`), the two-sided slide test
(`slide_blocks`) and `P_ZMovement` for non-missiles (`move_z`). Sight (`sight.rs`)
clips through openings with REJECT; hitscan aim and shoot (`combat_trace.rs`) clip
through openings; missiles (`projectiles.rs`) carry `momz`, explode on floor and ceiling
contact and apply the missile branch of `PIT_CheckThing`. The player policies simply
refuse multi-sector maps at admission (`with_original_registry`: `sectors.len() != 1 ||
heights != [0,128]`, `validate_grounded_geometry`) and carry flat-world preflight checks
(`z == floor`, `momz == 0`, items at `floor 0 / ceiling 128`, one-sector noise).

M3h4 therefore adds a policy, an admission, the removal of the flat-world guards for that
policy only, two behaviors that the flat world could never reach (the landing sound and
cross-sector noise), and the evidence. It adds no second mover, collision or scheduler.

## 2. Original C facts

### 2.1 Position checks and the move decision

- **`P_CheckPosition`** ([p_map.c:375](../linuxdoom-1.10/p_map.c)): `newsubsec =
  R_PointInSubsector(x, y)`; `tmfloorz = tmdropoffz = newsubsec->sector->floorheight;
  tmceilingz = newsubsec->sector->ceilingheight`. Things are tested over the box extended by
  `MAXRADIUS` (32), then lines over the exact box.
- **`PIT_CheckLine`** ([p_map.c:189](../linuxdoom-1.10/p_map.c)): skips lines whose bbox
  does not strictly overlap the destination box and lines with `P_BoxOnLineSide != -1`.
  Then `if (!ld->backsector) return false` (line 211): a one-sided line always blocks.
  For non-missiles, `ML_BLOCKING` (216) blocks everything and `ML_BLOCKMONSTERS` (219)
  blocks `!tmthing->player`. Otherwise `P_LineOpening(ld)` (224) and `tmceilingz =
  min(tmceilingz, opentop)` (also recording `ceilingline`), `tmfloorz = max(tmfloorz,
  openbottom)`, `tmdropoffz = min(tmdropoffz, lowfloor)` (227–236). Specials are appended
  to `spechit` (242); this milestone rejects specials at load, so that list stays empty.
- **`P_LineOpening`** ([p_maputl.c:300](../linuxdoom-1.10/p_maputl.c)): one-sided
  (`sidenum[1] == -1`) gives `openrange = 0` and returns; otherwise `opentop =
  min(ceilings)`, `openbottom = max(floors)`, `lowfloor = min(floors)`, `openrange =
  opentop - openbottom` (315–331). `front`/`back` are `line->frontsector/backsector`,
  which `P_LoadLineDefs` takes from `sidenum[0]`/`sidenum[1]`.
- **`PIT_CheckThing`** ([p_map.c:252](../linuxdoom-1.10/p_map.c)): for ordinary things the
  test is XY only (`blockdist` on radii); the z-range test (`tmthing->z > thing->z +
  thing->height || tmthing->z + tmthing->height < thing->z`, lines 294–296) exists only in
  the `MF_MISSILE` branch. A monster standing on a 96-unit ledge therefore still blocks a
  player walking below it in XY, exactly as in the original.
- **`P_TryMove`** ([p_map.c:451](../linuxdoom-1.10/p_map.c)): after a successful
  `P_CheckPosition`, for non-`MF_NOCLIP` things in this order: `tmceilingz - tmfloorz <
  height → false` (469, does not fit); `floatok = true` (471); `tmceilingz - thing->z < height →
  false` (475, must lower itself); `tmfloorz - thing->z > 24*FRACUNIT → false` (479, too big a step
  up); `!(flags & (MF_DROPOFF|MF_FLOAT)) && tmfloorz - tmdropoffz > 24*FRACUNIT → false`
  (481, do not stand over a dropoff). Then `thing->floorz = tmfloorz; thing->ceilingz =
  tmceilingz; x, y; P_SetThingPosition` (492–497). The thing's `z` is **not** changed
  by `P_TryMove`: a step up leaves `z < floorz` until `P_ZMovement`.
- **`P_SetThingPosition`** ([p_maputl.c:396](../linuxdoom-1.10/p_maputl.c)): the subsector
  comes from `R_PointInSubsector`; sector thing lists skip `MF_NOSECTOR` (409), BLOCKMAP
  links skip `MF_NOBLOCKMAP` (425) and cells outside the grid.
- **`R_PointInSubsector`** ([r_main.c:801](../linuxdoom-1.10/r_main.c)): with no nodes
  returns `subsectors[0]`; otherwise descends from `numnodes-1` by `R_PointOnSide`. The
  subsector's sector is the first seg's sidedef sector (`P_GroupLines`,
  [p_setup.c:517](../linuxdoom-1.10/p_setup.c)); `P_LoadSegs`
  ([p_setup.c:188](../linuxdoom-1.10/p_setup.c)) sets `seg->frontsector` from `sidenum[side]`
  and `seg->backsector` from `sidenum[side^1]` only when `ML_TWOSIDED` is set.

### 2.2 Horizontal movement and sliding

- **`P_XYMovement`** ([p_mobj.c:114](../linuxdoom-1.10/p_mobj.c)): the clamp/half-step
  loop of M3h3 is unchanged. A blocked player slides; a blocked missile explodes, except
  that a missile blocked by a `ceilingline` whose back sector has a sky ceiling is removed
  (176–184, excluded here by rejecting sky at load); anything else zeroes its momentum.
  After the loop: `if (mo->z > mo->floorz) return` (205, no friction when airborne); a
  `MF_CORPSE` with momentum above `FRACUNIT/4` and `floorz != subsector->sector->floorheight`
  keeps sliding (207–218); otherwise the `STOPSPEED`/`FRICTION` rule and the player's
  `S_PLAY_RUN` → `S_PLAY` stop.
- **`P_SlideMove`** ([p_map.c:695](../linuxdoom-1.10/p_map.c)) is the M3h3 algorithm: three
  corner traces, `bestslidefrac -= 0x800` (756), the flush move, `FRACUNIT -
  (bestslidefrac + 0x800)` (768), `P_HitSlideLine` (779), two retries (`++hitcount == 3`,
  709) then the stairstep (749).
- **`PTR_SlideTraverse`** ([p_map.c:636](../linuxdoom-1.10/p_map.c)): a one-sided line
  blocks unless the mover is on its back side (`P_PointOnLineSide`, 645–651). A two-sided
  line blocks when `openrange < height`, `opentop - z < height` or `openbottom - z >
  24*FRACUNIT` (656–664); it ignores `ML_BLOCKING`, so a blocking fence makes `P_TryMove`
  fail and the slide trace then finds no blocking line and falls through to the stairstep.
  `P_HitSlideLine` ([p_map.c:584](../linuxdoom-1.10/p_map.c)) keeps the M3h3 axis cases.

### 2.3 Vertical movement

- **`P_ZMovement`** ([p_mobj.c:246](../linuxdoom-1.10/p_mobj.c)): for a player with `z <
  floorz` it adjusts `viewheight`/`deltaviewheight` (252–257, presentation only). `z +=
  momz` (261). `MF_FLOAT` steering (263–278) is unreachable for the admitted roster.
  Floor branch (`z <= floorz`, 284): `if (momz < 0) { if (player && momz < -GRAVITY*8) {
  deltaviewheight = momz>>3; S_StartSound(mo, sfx_oof); } momz = 0; } z = floorz;` then a
  missile explodes (`MF_MISSILE && !MF_NOCLIP`, 313–318) and returns. Otherwise gravity:
  `momz = -GRAVITY*2` when `momz == 0`, else `momz -= GRAVITY` (320–326, `GRAVITY =
  FRACUNIT`, p_local.h:53). Ceiling branch (`z + height > ceilingz`): `if (momz > 0) momz =
  0; z = ceilingz - height;` then a missile explodes (328–347). The landing "oof" is a
  sound request only; `deltaviewheight` and `viewheight` are `P_CalcHeight` inputs, which
  stay excluded.
- **`P_MobjThinker`** ([p_mobj.c:415](../linuxdoom-1.10/p_mobj.c)): `P_XYMovement` when
  `momx || momy || MF_SKULLFLY`; `P_ZMovement` when `z != floorz || momz`; then the state
  countdown. So a step up (`z < floorz` after `P_TryMove`) is snapped to the floor in the
  same thinker call, and a step down starts falling in the same call.
- **`P_MovePlayer`** ([p_user.c:148](../linuxdoom-1.10/p_user.c)): `onground = (z <=
  floorz)` (158); forward and side thrust apply only `onground` (160–164); turning always
  applies; `S_PLAY_RUN1` entry needs a nonzero move command (166–170). `P_PlayerThink`
  ([p_user.c:236](../linuxdoom-1.10/p_user.c)) runs it behind the reaction-time gate that
  the player command stage already executes.
- **`P_SpawnMobj`** ([p_mobj.c:480](../linuxdoom-1.10/p_mobj.c)): `floorz`/`ceilingz` from
  the spawn subsector's sector (519–520); `ONFLOORZ` → `z = floorz`, `ONCEILINGZ` →
  `ceilingz - height`, else the given `z` (522–527). Nothing checks that the thing fits.

### 2.4 Monster movement on ledges

- **`P_Move`** ([p_enemy.c:272](../linuxdoom-1.10/p_enemy.c)): `DI_NODIR → false` (284);
  `tryx = x + info->speed * xspeed[movedir]` (290); one `P_TryMove` (293). On failure the
  `MF_FLOAT` branch (298) is unreachable and `numspechit == 0` returns false (310). On
  success `if (!(flags & MF_FLOAT)) actor->z = actor->floorz` (332–333): a walking monster
  never falls off a step it is allowed to take; `tmfloorz - tmdropoffz > 24` is refused
  because no admitted monster has `MF_DROPOFF` (info.c flags of MT_POSSESSED, MT_SHOTGUY,
  MT_CHAINGUY, MT_TROOP, MT_SERGEANT), while MT_PLAYER has it
  ([info.c:1130](../linuxdoom-1.10/info.c)).
- **`P_TryWalk`** ([p_enemy.c:349](../linuxdoom-1.10/p_enemy.c)) draws `movecount` only on
  success; **`P_NewChaseDir`** ([p_enemy.c:363](../linuxdoom-1.10/p_enemy.c)) tries the
  diagonal, the axis orders, the old direction (441), the eight-direction sweep, the
  turnaround (481) and finally `DI_NODIR` (488). A monster on a ledge above its target
  therefore cycles through refusals and either walks along the ledge or stands with
  `DI_NODIR`, which is the "ledge a monster refuses" observation.

### 2.5 Sight through openings

- **`P_CheckSight`** ([p_sight.c:300](../linuxdoom-1.10/p_sight.c)): REJECT by `(sector of
  t1) * numsectors + (sector of t2)` (320); `sightzstart = z + height - (height>>2)` (334);
  `topslope = t2->z + t2->height - sightzstart`, `bottomslope = t2->z - sightzstart`
  (335–336).
- **`P_CrossSubsector`** ([p_sight.c:135](../linuxdoom-1.10/p_sight.c)): per seg, skip
  lines already marked by `validcount`, the two side tests, `!ML_TWOSIDED → false` (198),
  `front = seg->frontsector; back = seg->backsector`, identical heights `→ continue` (206),
  `opentop`/`openbottom` (212–221), `openbottom >= opentop → false` (224), `frac =
  P_InterceptVector2` (227), `bottomslope = max(bottomslope, FixedDiv(openbottom -
  sightzstart, frac))` when floors differ, `topslope = min(...)` when ceilings differ
  (229–241), `topslope <= bottomslope → false`.

### 2.6 Hitscan through openings

- **`P_AimLineAttack`** ([p_map.c:1023](../linuxdoom-1.10/p_map.c)): `shootz = z +
  (height>>1) + 8*FRACUNIT` (1036); `topslope = 100*FRACUNIT/160`, `bottomslope =
  -100*FRACUNIT/160` (1039–1040).
- **`PTR_AimTraverse`** ([p_map.c:816](../linuxdoom-1.10/p_map.c)): a one-sided line stops
  (829); `P_LineOpening` (835); `openbottom >= opentop → stop`; `dist = FixedMul(attackrange,
  frac)`; the floor side raises `bottomslope` and the ceiling side lowers `topslope` only
  when the respective heights differ, using `li->frontsector`/`backsector`; `topslope <=
  bottomslope → stop`. A thing is aimed at when its slopes overlap the window;
  `aimslope = (clipped top + clipped bottom) / 2`.
- **`PTR_ShootTraverse`** ([p_map.c:899](../linuxdoom-1.10/p_map.c)): a one-sided line goes
  to `hitline` (922); a two-sided line hits when `FixedDiv(openbottom - shootz, dist) >
  aimslope` (lower wall) or `FixedDiv(opentop - shootz, dist) < aimslope` (upper wall),
  each only when the heights differ. `hitline`: `frac = in->frac - FixedDiv(4*FRACUNIT,
  attackrange)`, `z = shootz + FixedMul(aimslope, FixedMul(frac, attackrange))`; a sky front
  ceiling suppresses the puff (956–963, excluded here by rejecting sky at load);
  `P_SpawnPuff(x, y, z)`. A thing is hit when `thingtopslope >= aimslope >=
  thingbottomslope`, 10 units short of the intercept.
- **`P_SpawnPuff`** ([p_mobj.c:812](../linuxdoom-1.10/p_mobj.c)) and **`P_SpawnBlood`**
  (839) spawn at the given absolute `z` (plus the `(P_Random()-P_Random())<<10` jitter),
  so an impact may sit outside its sector's vertical span; `P_ZMovement` later clips it.
  Impacts never move in XY.

### 2.7 Missiles across height changes

- **`P_SpawnMissile`** ([p_mobj.c:889](../linuxdoom-1.10/p_mobj.c)): spawned at `source->z +
  4*8*FRACUNIT` (898–900); `dist = P_AproxDistance(...) / speed; if (dist < 1) dist = 1;
  momz = (dest->z - source->z) / dist` (918–923), an integer division of raw values, so a
  fireball flies a straight chord to the target's current `z`. `momz` is never clamped
  anywhere in the source.
- **`P_CheckMissileSpawn`** ([p_mobj.c:868](../linuxdoom-1.10/p_mobj.c)): tics draw, the
  half-step in x, y **and z**, one `P_TryMove`, else `P_ExplodeMissile`.
- **`P_ExplodeMissile`** ([p_mobj.c:90](../linuxdoom-1.10/p_mobj.c)): zeroes all three
  momenta, enters the death state, draws the tics, clears `MF_MISSILE`, plays the death
  sound. MT_TROOPSHOT has `MF_NOBLOCKMAP|MF_MISSILE|MF_DROPOFF|MF_NOGRAVITY`
  ([info.c:1914](../linuxdoom-1.10/info.c)), so the dropoff rule never stops it and it is
  the only admitted mover that `P_TryMove` can reject with the fit, headroom or step rule
  while flying (it then explodes). The floor and ceiling branches of `P_ZMovement` (§2.3)
  explode it on contact.

### 2.8 Noise across sectors

- **`P_NoiseAlert`** ([p_enemy.c:159](../linuxdoom-1.10/p_enemy.c)): `soundtarget =
  target; validcount++; P_RecursiveSound(emitter->subsector->sector, 0)`. `P_FireWeapon`
  calls it after `P_SetMobjState(player->mo, S_PLAY_ATK1)` and the attack psprite entry.
- **`P_RecursiveSound`** ([p_enemy.c:106](../linuxdoom-1.10/p_enemy.c)): `if (sec->validcount
  == validcount && sec->soundtraversed <= soundblocks+1) return;` then marks the sector,
  sets `sec->soundtarget` and, for each line of `sec->lines` in order (`P_GroupLines`
  order is linedef index order, [p_setup.c:541](../linuxdoom-1.10/p_setup.c)): skip
  `!ML_TWOSIDED`, `P_LineOpening`, skip `openrange <= 0`, `other` is the sector on the
  other side (136–139), and recurse with `soundblocks` or, through an `ML_SOUNDBLOCK` line,
  with 1 only when `soundblocks == 0` (141–147). Depth-first, pre-order, line order.
  `A_Look` then reads `actor->subsector->sector->soundtarget`.

### 2.9 Damage and death with heights

- **`P_DamageMobj`** ([p_inter.c:775](../linuxdoom-1.10/p_inter.c)): the thrust reads only
  XY, but `if (damage < 40 && damage > target->health && target->z - inflictor->z >
  64*FRACUNIT && (P_Random() & 1)) { ang += ANG180; thrust *= 4; }` (819–826). The flat
  world could never satisfy the height test; the heights world can (a weak hit from
  below on a player standing on a ledge of more than 64 units), which adds one gameplay
  RNG draw at that point. `LifeHost::apply_damage` already implements it.
- **`P_KillMobj`** ([p_inter.c:668](../linuxdoom-1.10/p_inter.c)): `flags |=
  MF_CORPSE|MF_DROPOFF` (680) and `height >>= 2`, so corpses may be pushed off ledges and
  fall; drops spawn `ONFLOORZ` at the corpse's position (756, `MF_DROPPED` 757), so a drop sits
  on the corpse's sector floor even when the corpse is still airborne.

### 2.10 Constants and flags

`GRAVITY = FRACUNIT` ([p_local.h:53](../linuxdoom-1.10/p_local.h)); `MAXMOVE 30`,
`MAXRADIUS 32`, `MELEERANGE 64`, `MISSILERANGE 2048` (p_local.h:51–58). Linedef flags
([doomdata.h:101–135](../linuxdoom-1.10/doomdata.h)): `ML_BLOCKING 1`, `ML_BLOCKMONSTERS
2`, `ML_TWOSIDED 4`, `ML_DONTPEGTOP 8`, `ML_DONTPEGBOTTOM 16`, `ML_SECRET 32`,
`ML_SOUNDBLOCK 64`, `ML_DONTDRAW 128`, `ML_MAPPED 256`. Of these, gameplay reads 1, 2, 4
and 64; 8, 16, 32, 128 and 256 are read only by the renderer and the automap, which are
not part of this project. `P_CheckMeleeRange` and `P_CheckMissileRange` use
`P_AproxDistance` on XY only; neither reads `z`.

## 3. What exists and what changes (Rust inventory)

| Source body | Rust | Status for M3h4 |
| --- | --- | --- |
| `P_CheckPosition`, `PIT_CheckLine`, `P_LineOpening`, `P_TryMove` | `multisector::check_move_from_things`, `opening` | Present (M3b); used by the player world through `movement::move_xy_profile` with `sectors: Some` and by `pursuit::move_actor`. No change. |
| `PIT_CheckThing` | `movement::check_things` (XY), `projectiles::missile_contact` (missile z test) | Present. No change. |
| `P_SlideMove`, `PTR_SlideTraverse`, `P_HitSlideLine` | `movement::Mover::slide`, `multisector::slide_blocks` | Present. No change. |
| `P_XYMovement` | `movement::move_xy_profile` (airborne friction skip, corpse rule), `projectiles::move_xy` | Present. No change. |
| `P_ZMovement` | `multisector::move_z`, `projectiles::move_z` | Present. **Add** the player landing sound (`sfx_oof`) as an `ActorStateEvent::Sound` before the `Vertical` event, for the canonical player only. |
| `P_MovePlayer` on-ground gate | `apply_player_motion` | Present. No change. |
| `P_Move`, `P_TryWalk`, `P_NewChaseDir` | `pursuit::move_actor` (sets `z = floor`), `try_walk`, `new_direction` | Present. No change. |
| `P_CheckSight`, `P_CrossSubsector`, REJECT | `sight::query_prepared`, `try_extent_query` | Present. No change. |
| `P_AimLineAttack`, `PTR_AimTraverse` | `combat_trace::aim_trace` | Present. No change. |
| `P_LineAttack`, `PTR_ShootTraverse`, puff placement | `combat_trace::hit_trace`, `LifeHost::execute_attack` | Present. No change. |
| `P_SpawnMissile` `momz`, `P_CheckMissileSpawn`, `P_ExplodeMissile` | `projectiles::actor_aim`, `finish_launch`, `explode` | Present. **Widen** the Rust-only `momz` bound for this policy (§4.4). |
| `P_RecursiveSound`, `P_NoiseAlert` | `Simulation::emit_noise` (historical weapon profile only) | **Add** the same flood for the canonical player's weapon noise under this policy; the staged plan carries every sector's sound target. |
| `P_DamageMobj` fall-forward | `LifeHost::apply_damage` | Present. **Remove** the flat-world `z == 0`/`momz == 0` guards of the impulse preflights for this policy. |
| `P_KillMobj` corpse `MF_DROPOFF`, drops `ONFLOORZ` | `apply_damage` | Present. No change. |
| `R_PointInSubsector`, `P_GroupLines` subsector→sector | `Topology::locate`, `Topology::new_bounded` | Present. No change. |
| `P_SpawnMobj` floor/ceiling from the spawn sector | `lifecycle::make_actor` | Present. **Add** the Rust-only spawn fit check (§4.3). |

The flat-world guards to relax for the new policy only: `with_original_registry`
(sector count and `[0,128]`, `validate_grounded_geometry`), `preflight_player_body`
(player and monsters `z == floor`, `momz == 0`), `preflight_impulse` (target/origin `z ==
0`, `momz == 0`), `demon::strike_player` (owner and body `z == 0`, `momz == 0`),
`world_pickups::validate_item` (`floor == 0`, `ceiling == 128`), `player_pistol::Host::noise`
(`sector == 0 && sectors.len() == 1`), the `z + momz` within 1,024 witnesses in
`preflight_monster_owner` and `preflight_map_demons`, the `momz` within 64 witness in
`preflight_expanded_demons_for`, and the map-wide type dimension check of
`ActorRegistry::admit_type` (`sectors.iter().any(|s| height > span)`), which would reject
every type as soon as the map contains a window or a closed sector.

## 4. The admitted domain

`PlayerMonstersHeightsV1` (wire 15, context `PLAYER-MONSTERS-HEIGHTS-1-MEDIUM-v1`, DSIM31)
is a superset of `PlayerMonstersMapScaleV1`: same roster, weapons, items, drops,
fireballs, infighting, statistics, 320 slots, 208 live fireballs, the walled rectangle
and the 4,096-unit momentum bound. It changes only the geometry admission and the
vertical domain, for itself. Every older policy keeps every check of §1 of the M3h3 note.

### 4.1 Geometry admitted at load (checked once, before construction)

1. Inherited: vertices, node origins/deltas and the BLOCKMAP origin within 2,048; line
   norm ≤ 1,536; sector heights within ±512 (`with_demon_expanded_profile`); floor ≤
   ceiling and `sides[1].is_some() == (flags & ML_TWOSIDED != 0)` (`Topology`); no line or
   sector specials; no `F_SKY1` ceiling; THINGs ≤ 256; cells ≤ 8,192; raw BLOCKMAP words ≤
   65,536; nodes and subsectors ≤ 4,096; sectors ≤ 256; linedefs ≤ 95.
2. Every linedef is axis-aligned (`P_HitSlideLine` axis cases; the non-axis slide needs
   its own proof and stays excluded).
3. Linedef flags: one-sided lines may carry any subset of `{1, 2, 8, 16, 32, 64, 128,
   256}`; two-sided lines carry `ML_TWOSIDED` plus any subset of `{1, 2, 8, 16, 32, 64, 128,
   256}`. Bits 1, 2, 4 and 64 are the gameplay flags the admitted bodies read (§2.10); the
   others are renderer/automap data with no reader in this project. Any other bit rejects.
   The M3h3 requirement that one-sided lines be exactly `ML_BLOCKING` is dropped for this
   policy only: the source blocks a one-sided line regardless of its flags (§2.1).
4. Walled rectangle: each side of `R` is completely covered by **one-sided** linedefs
   lying on it (M3h3 counted every linedef; every M3h3 line is one-sided, so that policy's
   result is unchanged).
5. BLOCKMAP covers `R` and lists every linedef, one- or two-sided, in every cell its bbox
   touches.
6. Every sector's span may be smaller than an actor: windows (`[40,88]`), crawlspaces
   (`[0,32]`) and closed sectors (`floor == ceiling`) are admitted. The registry's
   map-wide dimension fit is therefore evaluated for this policy against the tallest
   sector only ("every type fits somewhere"), and fitting is instead proved per spawn
   (§4.3).
7. Spawns: the closed bounding box of every THING, the player body and every controlled
   dropped pickup lies inside `R` (unchanged) **and** the spawned type's height fits the
   spawn sector's span (new).

### 4.2 Horizontal invariant (extended to openings)

**Invariant I** (unchanged statement): every XY mover's open box lies inside `R°`.

The M3h3 proof uses only that a destination box leaving `R°` contains a boundary point of
`R`, that this point lies on a wall segment listed in the destination's cells, and that
`PIT_CheckLine` blocks that wall. Conditions 4 and 5 keep the first two facts; `if
(!ld->backsector) return false` keeps the third because the covering segment is one-sided,
regardless of any two-sided line that also touches the boundary. Two-sided lines in the
interior of `R` never enter the argument: whether a mover crosses one or is stopped by it
(blocking flags, fit, headroom, step, dropoff), the accepted destination is still inside
`R°`. The per-attempt step bounds of the M3h3 table are unchanged (the fireball's
half-step now also moves `z`, which is irrelevant to I).

**Opening lemma** ("no mover enters solid space through a two-sided line"): after any
accepted `P_TryMove`, `tmceilingz − z ≥ height` holds by the headroom test and `tmfloorz −
z ≤ 24` by the step test, where `tmfloorz`/`tmceilingz` are the destination sector's heights
tightened by every opening the destination box crosses. Hence the mover's vertical extent
fits the destination sector and every crossed opening, up to the 24-unit step below the
new floor that `P_ZMovement` (or `P_Move`) closes in the same thinker call.

**Invariant V** (new, Rust-only runtime witness): at every tic boundary, every actor that
is not an impact (puff, blood) satisfies `floor ≤ z ≤ ceiling − height` with its own
`floorz`/`ceilingz`. Proof sketch: spawns satisfy it by condition 7 (THINGs, the player
body, drops `ONFLOORZ` into a sector that already fits the taller corpse; a fireball spawns
at `source.z + 32` with height 8 inside a source that fits, so `z + 8 ≤ source.z + 40 ≤
source.z + 56 ≤ ceiling`). A move accepted by `P_TryMove` keeps `z + height ≤ ceilingz`;
`P_Move` snaps `z = floorz`; `P_ZMovement` clips `z` into `[floorz, ceilingz − height]`
and runs in the same thinker call whenever `z != floorz`; damage changes momentum only;
state entry, target changes and removal do not move. Impacts are spawned at absolute `z`
by the source itself and are excluded, as M3h3 excluded their boxes.

### 4.3 Spawn fit

`spawn_map_things` (the THING batch), `spawn_type` (the player body, controlled puffs,
blood and the context body), `spawn_dropped_pickup` and the P_KillMobj drop check
`info.height() ≤ ceiling − floor` of the spawn position's sector before any mutation. A
failure rejects with a new `MapScaleError::Fit` and unchanged state. The source has no such
check: a too-tall thing would simply stand through the ceiling. Rejecting it keeps V and
costs no legal play, because a THING that does not fit its sector is a broken map.

### 4.4 Vertical arithmetic and the momz bound

Heights are within ±512 and actor heights within 56, so every `z`, `floorz`, `ceilingz`,
`opentop`, `openbottom` and `sightzstart` is within 1,024 raw-safe units; the M3f2 sight
proof covers `A = 2048` with heights ≤ 1,024, so slopes stay within its domain. Hitscan
`FixedDiv(open − shootz, dist)` and `FixedDiv(thing − shootz, dist)` saturate to
`±MAXINT` by the source guard for tiny `dist`, which `Fixed::fixed_div` reproduces; the
puff `z = shootz + aimslope · frac · range` is bounded by `|aimslope| ≤ 100/160` (the
autoaim window) and `range ≤ 2048`.

The fireball's `momz = (dest.z − source.z) / time` with `time ≥ 1` is bounded by `|dz| ≤
1,024 − 56 = 968`. The historical Rust-only `momz` witness (64) would reject such a launch
and stall the world, so this policy's witness is `HEIGHTS_MOMZ_UNITS = 1,024` for every
actor; gravity reaches at most `−46` in a 1,024-unit fall. `z + momz` is then within 2,048
and every `P_ZMovement` addition is representable. The witnesses `z within 2,048` (the sight
margin) and `momz within 1,024` replace the historical `z + momz within 1,024` for this
policy; by V they never fail and stay as the proof's runtime check.

### 4.5 Newly reachable behavior in the admitted roster

- Landing sound: `sfx_oof` when a player lands with `momz < −8` (a drop of at least 44
  units). Emitted as the existing `ActorStateEvent::Sound` immediately before the
  `Vertical` event, matching the source order (`S_StartSound` inside `P_ZMovement`).
- Cross-sector noise: `P_RecursiveSound` through open two-sided lines with
  `ML_SOUNDBLOCK` counting, emitted as the existing `WeaponEvent::Noise` per flooded sector
  in recursion order. Per-sector `soundtarget` is already a DSIM field since layout 6.
- Fall-forward damage: one extra `CombatEvent::Random` (kind 0) and reversed, quadrupled
  thrust when a weak lethal hit comes from more than 64 units below.
- Step snapping, falling, landing, a monster refusing a dropoff, sliding along a blocking
  two-sided fence, fireballs exploding on floors and ceilings, autoaim up and down, upper
  and lower wall puffs, sight and noise through windows and closed sectors.

### 4.6 What stays rejected at load

Non-axis lines (any sidedness), linedef flag bits outside the admitted sets, two-sided
lines on the boundary of `R` without a one-sided covering line, line or sector specials,
`F_SKY1` ceilings, inverted sectors, inconsistent `ML_TWOSIDED`/back sidedef pairs,
heights beyond ±512, more than 256 sectors or 95 linedefs, and spawns whose box crosses a
wall or whose type does not fit the spawn sector. Doors, lifts, crushers, teleporters,
damaging floors, moving sectors and `P_CalcHeight` are unreachable because every special
is rejected and view fields are not simulated.

## 5. DSIM31 and wire 15

DSIM31 uses the DSIM30 field layout and event vocabulary unchanged under version 31,
policy 15 and the new context. The source state that heights need (`z`, `momz`, `floorz`,
`ceilingz`, subsector, sector, per-sector `soundtarget`) has been in the layout since
DSIM3 and DSIM6; no persistent field is added. Readers: `Diagnostic::decode_heights` reads
31 only; every earlier reader rejects 31. Under 31 the reader widens: item `z == floor` with
`floor`/`ceiling` equal to the item's sector heights in `map.sectors` (instead of `0/128`),
`momz` within 1,024, and the v30 bounds otherwise.

## 6. Evidence plan

### 6.1 C harness

`tools/reference/m3h4_reference.py` inherits the M3h3 harness (M3h2 bodies, observers and
controlled inputs) and replaces only the geometry and the cases. `m3h4_geometry.py` builds
rooms of axis-aligned rectangular sectors tiled inside a walled rectangle: one-sided
boundary walls split at 1,024 units, two-sided lines between adjacent sectors with
configurable flags, one subsector per sector, a guillotine BSP over axis-aligned
partitions, segs per subsector with the source side convention, a REJECT table with
optional set bits, and a cell-exact BLOCKMAP. The C adapter already installs sector line
lists, sidedef sectors, segs, subsectors, node partitions and REJECT from these lumps
(`generate_m3d3a.post_loader`). `m3h4_cases.py` places THINGs and scripts commands.

Rooms as generated (`m3h4_cases.py`; ceilings 256 unless stated; y spans −512..512):

| Room | Sectors along x | Purpose |
| --- | --- | --- |
| stairs | floors 97, 72, 48, 24 in 256-unit treads (the first riser is 25 units), then 0 over `[256,1024]` | steps up and down, the too-high step, Imp and Demon pursuit down the stairs, a former human refusing the 25-unit riser, autoaim up/down a step |
| ledge | `[−1024,0]` floor 96; `[0,64]` floor 0; `[64,1024]` floor 0 ceiling 100 | player drop with landing (`momz < −8` → oof), headroom refusal while airborne, a former human refusing the ledge, a corpse knocked off the ledge, the fall-forward branch, fireballs down the ledge (floor explosion when the player dodges, upper-wall explosion under the low ceiling) |
| window | `[−1024,−64]` floor 0 ceiling 128; `[−64,64]` floor 40 ceiling 88; `[64,512]` floor −64; `[512,1024]` floor 64 | sight blocked by the window's bottom slope from the pit, allowed from the step; hitscan into the window's lower wall (from floor 0) and upper wall (from floor 64); noise through the window; variants with a REJECT pair (0,3) and with `ML_SOUNDBLOCK` on both window lines |
| closed | `[−512,0]`; `[0,64]` floor 128 ceiling 128; `[64,512]` | sight, noise, hitscan and movement stopped by a zero-opening line |
| fence | two equal sectors split by a two-sided `ML_BLOCKING` line, and a variant with `ML_BLOCKMONSTERS` | player slide along a blocking fence, monster blocked, fireball and hitscan passing, sight unaffected; the player passing a monsters-only fence |
| crawl | `[−512,0]` floor 0 ceiling 128; `[0,512]` floor 0 ceiling 32 | too-low ceiling (fit refusal), upper-wall puff at slope 0 |
| tower | `[−1024,0]` floor 0 ceiling 160; `[0,1024]` floor 96 ceiling 160 | fireball from 900 units below rising past a dodging player into the ceiling |

Two source facts shaped the placements, and both are observations worth keeping:

- Every fresh mobj has `movedir == 0 == DI_EAST` (`P_SpawnMobj` zero-fills), so a chase
  target due west is the first `P_NewChaseDir`'s `turnaround` (`opposite[DI_EAST]`), is
  discarded (`d[1] = DI_NODIR`, [p_enemy.c:414](../linuxdoom-1.10/p_enemy.c)) and the
  monster first walks east by `olddir` ([p_enemy.c:441](../linuxdoom-1.10/p_enemy.c)) until
  blocked or until `movecount` expires. The high sides therefore lie west and the player
  east, so that the first move heads for the edge.
- `P_PathTraverse` ([p_maputl.c:632](../linuxdoom-1.10/p_maputl.c)) follows the exact trace
  end point: at angle 180° `finesine[4096] = −25`, so a 1,024-unit autoaim trace ends 0.39
  units below its start, and together with the `y1 += FRACUNIT` nudge the block walk steps
  down one block row before reaching the target's block. Things are linked only in the
  block of their center, so the aim misses a zombie standing on the trace while the
  2,048-unit `P_LineAttack` trace, whose descent is halved, finds it (`autoaim_down_step`,
  tics 19–75 versus 89–117). Neither trace is a geometry claim of this milestone; the Rust
  trace already reproduces the block walk (M3f3).

The 24 scenarios (`m3h4_cases.csv`): `stairs_up`, `stairs_down`, `too_high_step`,
`imp_pursuit_stairs`, `demon_pursuit_stairs`, `monster_refuses_riser`, `autoaim_up_step`,
`autoaim_down_step`, `drop_landing_oof`, `monster_refuses_ledge`, `corpse_pushed_off_ledge`
(shotgun), `fall_forward_from_below` (controlled 12-point hit at tic 1, then a pistol kill
from 96 units below), `fireball_down_ledge_floor`, `window_sight_pit_then_step`,
`window_lower_wall_puff`, `window_upper_wall_puff`, `noise_through_window`,
`noise_through_soundblock`, `reject_bit`, `closed_opening_blocks_all`, `fence_slide`,
`fence_blocks_monsters_only`, `crawlspace_refusal`, `fireball_up_ceiling`. Captures are
selected as in M3h3 plus every tic with a hitscan, a noise flood, a landing sound, a
blocked move with reason 3–6 and every `Z` row whose floor-contact flag changes. The fireball
chords follow `momz = (dest.z − source.z) / (distance / speed)` from a start 32 units above
the source, which fixes the Imp positions: an Imp 124 units behind the ledge edge aiming at
a player 300 units past it crosses the edge at `z = 100` and enters the 100-unit ceiling at
`z = 85`; an Imp 900 units from a 96-unit ledge reaches the 160-unit ceiling 300 units past
the dodging player.

Observed in the committed trace (`m3h4_contract.json` has the per-case counts): 3 snapped
steps up and 4 falls down the stairs; 104 step-up refusals (reason 5) at the 25-unit riser
and the ledge, 33 dropoff refusals (reason 6) by former humans at the riser and the ledge,
70 fit refusals (reason 3) at the closed sector and the crawlspace, 3 headroom refusals
(reason 4, two by the falling player and one by a fireball under the 100-unit ceiling);
one landing sound (`E,7,ident,34`) after a 13-tic fall with `momz = −14`; a corpse sliding
over the ledge without friction, falling and landing silently; the fall-forward branch
(`P_Random() & 1` on draw 161) throwing a 96-unit-higher zombie toward and off the ledge
with the thrust quadrupled; one fireball exploding on the floor, one on the ceiling, one
against an upper wall and one stepping onto a 96-unit ledge floor before exploding there;
sight blocked from the pit and allowed from the step, 24 REJECT rejections, 0 of 24
queries through the closed sector; 8 noise floods of all four window sectors, floods
stopped after the second `ML_SOUNDBLOCK` line (`soundblocks == 1` observed) and at the
closed sector; lower-wall puffs at `z = 36` under the 40-unit sill and upper-wall puffs at
`z = 100` over the 88-unit lintel; a puff spawned 30 units below the floor when a spread
shot missed down a staircase; 318 line refusals of an Imp at a blocking fence while its
fireballs and the player's bullets crossed it; and a player passing an `ML_BLOCKMONSTERS`
fence that refused a zombie twice.

### 6.2 Independent DSIM31

`tools/reference/heights_snapshot.py` extends `map_scale_snapshot.py` without editing it:
version 31, policy 15, the new context, the same 320-slot and 208-fireball bounds. The
event vocabulary already projects `Z` rows (Vertical, tag 11), `M`/`C`/`O` rows
(SectorMovement with openings, tag 10), `N` rows (Noise, tag 14/6) and `E,7` rows (Sound,
tag 12/3). `m3h4_snapshot_goldens.py` writes `m3h4_snapshots.csv`, `m3h4_events.dref` and
selected full-byte fixtures before the Rust change.

### 6.3 Rust-only evidence

A test-only `HeightsRoom` builder mirrors the Python geometry so rejection variants exist
without committed bytes: a diagonal two-sided line, a forbidden flag bit, a two-sided
boundary line without a wall, an unlisted two-sided line, a sector special, a line
special, an `F_SKY1` ceiling, heights beyond ±512, an inverted sector, a THING in a
window sector, a player spawn in a crawlspace. Rollback tests for every rejection, the
vertical witness and the momz witness. Soaks: the stairs and ledge rooms with the full
item and roster budget for 1,200 tics on four seeds, asserting Invariant V after every
tic. The fuzz target `player_heights` reuses the committed rooms with the M3h3 progress
property plus the V assertion. Historical fuzz targets gain nothing but the two Clippy
fixes.

### 6.4 Measurements

Matched release timings of `d9ad98f` and the branch head for every existing `--measure`
workload (digests compared) plus new workloads from the heights example: `stairs`,
`ledge_drop`, `window_fight`, `fence_slide`. Run alone on the idle host, eleven
interleaved process pairs per workload, as in M3h3.

## 7. Implementation plan

Each step is a separate commit unless noted. Commits 1–3 land before the runtime change.

1. **This note** (`docs/M3H4_REFERENCE_INVESTIGATION.md`).
2. **C evidence**: `tools/reference/m3h4_geometry.py`, `m3h4_cases.py`,
   `m3h4_reference.py`; generated `tests/reference/m3h4_trace.dref`, `m3h4_cases.csv`,
   `m3h4_geometry_*.bin`, `m3h4_maps.rs`, `m3h4_contract.json`. Verify the rows show
   steps, falls, the oof, blocked reasons 3–6, window sight, upper/lower puffs, floor and
   ceiling explosions, noise floods, and that strict and ASan/UBSan builds match.
3. **DSIM31 projection**: `tools/reference/heights_snapshot.py`,
   `m3h4_snapshot_goldens.py`; generated `m3h4_snapshots.csv`, `m3h4_events.dref`,
   `m3h4_state_*.bin`, `m3h4_snapshot_contract.json`.
4. **Policy and admission** (`crates/doom-sim/src`): `original_registry.rs` (variant,
   wire 15, `context_schema`, `has_heights`, `has_map_scale` includes it, every `matches!`
   roster list, `with_original_registry` admission: skip the one-sector test, call
   `validate_heights_geometry`, derive `RoomBounds` with the one-sided filter, validate
   PUFF/BLOOD after the policy is installed); `player_monsters.rs` (schema constant,
   `original_player_heights_map`); `player_map_scale.rs` (`HEIGHTS_MOMZ_UNITS`,
   `MapScaleError::{Geometry, Fit, Vertical}`, `validate_heights_geometry`, `spawn_fits`,
   `momz_bound`, `vertical_admitted`); `lib.rs` exports.
5. **Flat-world guard relaxations, gated on `has_heights`**: `original_player.rs`
   (`preflight_player_body`, `preflight_impulse`), `demon.rs` (`strike_player`),
   `world_pickups.rs` (`validate_item` compares with the item's sector heights),
   `monsters.rs` and `demon_map_spawn.rs` (`z + momz` witness), `demon_expanded_map.rs`
   (`momz` witness, vertical witness call, `DemonPlan.sound_targets: Vec`),
   `demon_lifecycle.rs` (dimension envelope = tallest sector), `spawning.rs`
   (`spawn_map_things` fit, `preflight_monster_spawn` fit), `world_pickups.rs`
   (`spawn_dropped_pickup` fit).
6. **New reachable behavior**: `thinkers.rs` (landing sound before the Vertical event for
   the canonical player), new `noise.rs` (pure `P_RecursiveSound` flood over a target
   vector, used by `player_pistol::Host::noise` under this policy; `Simulation::emit_noise`
   keeps its code and bytes).
7. **DSIM31**: `snapshot.rs` (version 31 selection, every `21..=30` range), `diagnostic.rs`
   (`decode_heights`, `policy`, `slot_bound_for`, pickup item floor/ceiling against
   `map.sectors`, `momz` bound), `STATE_FORMAT.md` section.
8. **Tests and tools**: `crates/doom-sim/src/heights_room.rs` (test-only builder),
   `player_heights_tests.rs`, `tests/support/m3h4.rs`, `examples/player_heights.rs`
   (asserted replay plus `--measure`), `tools/reference/check_player_heights.py`,
   `fuzz/fuzz_targets/player_heights.rs` plus the `fuzz/Cargo.toml` entry,
   `tools/seed_player_heights_fuzz.py`, CI steps in `.github/workflows/rust.yml`.
9. **Follow-up 1**: `Simulation::admitted_types` becomes `[AtomicU64; 4]` behind a small
   `Clone`/`Debug` wrapper with relaxed loads and stores; a compile-time `Send + Sync`
   assertion on `Simulation`; matched timing of the memo workloads.
10. **Follow-up 2**: the two stable-Clippy lints in `fuzz/fuzz_targets` (`chunks_exact`
    with a constant size, manual `is_multiple_of`) without behavior change; `cargo clippy
    --manifest-path fuzz/Cargo.toml --all-targets -- -D warnings` clean.
11. **Docs**: README status and example list, ROADMAP current milestone, COMPATIBILITY
    section, STATE_FORMAT, ARCHITECTURE, TESTING, docs/README, VALIDATION,
    `tools/reference/README.md`, `docs/validation/M3H4.md` (gates, fuzz, timings, review).
12. **Gates, review, PR**: the listed gates, the independent reviewer over the full diff
    against the C source, fixes recorded in the evidence record, draft PR, ready after CI.

## 8. Risks and open points

- A hitscan puff can land in a sector shorter than its height (span < 16) only when a shot
  ends inside such a sector; impacts are excluded from V and clipped by `P_ZMovement` as in
  the source. The Rust `Vertical` observation computes its ceiling flag after floor
  clipping while the C harness computes it before; the two differ only for an actor taller
  than its sector span, which V excludes for movers and which the planned rooms avoid for
  impacts (no sector span below 32).
- Two-sided lines with the same sector on both sides are consistent with the source but
  break the one-subsector-per-sector room builder; the fence rooms use two sectors of equal
  heights instead. Admission does not forbid same-sector two-sided lines.
- The dimension envelope ("fits the tallest sector") and the spawn fit are Rust-only
  policy, like every admission bound in this project; they are labelled as such.
- `ML_SOUNDBLOCK` is admitted because `P_RecursiveSound` is implemented; if the noise
  evidence does not exercise a sound-blocking line, the flag is still admitted by the
  source reading and covered by a Rust-only test.
- Nothing here admits non-axis walls; if the evidence turns up a zero-risk proof for them,
  the owner is asked before any admission.

### 8.1 Owner decisions (2026-11-01)

1. **Dimension envelope and spawn fit (§4.1 item 6, §4.3): approved.** Type dimensions are
   evaluated against the tallest admitted sector; fitting is proved per spawn. Short and
   zero-height sectors are not rejected: a closed-door sector (`floor == ceiling`) must load
   as a static wall.
2. **momz witness (§4.4): approved.** `PlayerMonstersHeightsV1` uses the derived 1,024-unit
   `momz` witness; every older policy keeps the historical 64.

## 9. Status / next step

Kept current while the milestone is in progress; the final state is summarized in
[the evidence record](validation/M3H4.md).

- Step 1 (this note) committed as `7707e9f`; owner decisions recorded above.
- Step 2 (C evidence) generated with `--overflow-check --address-check`; the local
  generation ran without `ASAN_OPTIONS`, so LeakSanitizer was active and reported nothing.
- Step 3 (DSIM31 projection) generated: 693 captures, 8 full-byte fixtures. The new
  encoder pre-filters slide-traverse trace rows (see `heights_snapshot.py`), which the
  inherited bonus layer drops anyway; without the pre-filter the inherited monster layer
  reads a stage-0 slide trace in a monster-free scenario as an unterminated aim.
- Next: step 4 (policy and admission).
