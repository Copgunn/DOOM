# M3h3 — Wall-aware travel bounds, no-stall guarantees and real-map scale

This note records the bound inventory, source facts, the reproduced M3h2 stalls and
the admitted domain of `PlayerMonstersMapScaleV1`. It was written and committed before
the runtime change. Support and limits belong to
[COMPATIBILITY](COMPATIBILITY.md#original-monsters-at-real-map-scale); DSIM30 belongs to
[STATE_FORMAT](STATE_FORMAT.md#dsim-v30-original-monsters-at-real-map-scale); measured
observations belong to [the evidence record](validation/M3H3.md). All references are to
the preserved `linuxdoom-1.10/` sources. Every number below is in map units unless it
says raw (16.16 bits).

## 1. Bound inventory of the admitted combat world

The checked player world (`Profile::OriginalDemon` with a player policy up to
`PlayerMonstersImpV1`) applies these bounds. "Protects" names the original arithmetic
the bound keeps representable; "slack" says where the bound is stricter than that
arithmetic needs.

| Bound | Where | Protects | Slack |
| --- | --- | ---: | --- |
| Vertices, node origins/deltas and BLOCKMAP origin within 2,048; line norm `abs(dx)+abs(dy)` ≤ 1,536; sector heights ≤ 1,024 | `map_extent::validate`, run by `with_demon_expanded_profile` | `P_InterceptVector`/`P_InterceptVector2` sums (`256·(G+A)·L` raw, see [M3f2](M3F2_REFERENCE.md)), `P_DivlineSide` products, BSP side tests | None for the arithmetic; this is the proven geometry envelope. Actors are held far inside it (below). |
| Sector heights ≤ 512 | `with_demon_expanded_profile` | Eye/opening clipping | Irrelevant to the flat `[0,128]` player world |
| Player policies: one sector with heights `[0,128]`, one-sided axis-aligned `ML_BLOCKING` lines only | `with_original_registry`, `validate_grounded_geometry` | Keeps `P_HitSlideLine` on its horizontal/vertical early returns, `P_LineOpening` unreachable | Defines the domain; not arithmetic slack |
| Linedefs ≤ 128 when weapons are admitted | `with_original_registry` | Fixed `seen: [bool; 128]` in `combat_trace::collect_limited` and `MAXINTERCEPTS` 128 | Does not account for things in the same trace (see §4) |
| Actor XY and Z within 1,024 before every tic | `sight::actor_bounds` in `preflight_expanded_demons_for` | Sight eye/slope sums | Sight is proven to 2,048 by M3f2 |
| Owner Demons within 978, every other actor within 994, every actor within 1,008 | `preflight_monster_owner` (`monster_actor(id, 978)`, `within(v, 994)`), `preflight_map_demons` (`monster_actor(id, 1008)`) | Historical margins: 1,024 minus `MAXMOVE` (30), minus radius 16, minus half of `MAXRADIUS` | Rust-only margins inherited from the Demon map profiles; a fireball past 994 rejects its tic (§3.3) |
| Items within 978 | `world_pickups::validate_item`, `spawn_dropped_pickup` | Same | Same |
| Spawn positions: player 994, THINGs 978, typed spawns 1,008, absolute Z 1,018 | `player_spawn_guard`, `spawn_map_things`, `preflight_monster_spawn` | Same | Same |
| Travel envelope `abs(position) + ceil(160·M/3) ≤ 994` and `M ≤ 30` for the player and any moving roster monster | `original_player::travel_bounds`, used by `preflight_player_body`, `preflight_impulse` (every damage plan), `player_commands::validate_result`, the post-tic monster check in `advance_expanded_demons` | The [M3g3](M3G3_REFERENCE.md) forward proof that friction-decayed momentum never carries a body past 994 without walls | Enormous: a 24-point fireball hit (3 units of thrust, `M = 196,608` raw) reserves 160 units of future travel; a 3-point pellet reserves 20. It also rejects `M > 30`, which the original simply clamps. It ignores that walls stop every mover. |
| Command stage `abs(position) + 4·M + 8 ≤ 994` | `player_commands::attempt_bounds` | One whole axis-slide pass ([M3g4](M3G4_REFERENCE.md)) | Same ignorance of walls |
| XY and Z momentum within 64 | `preflight_expanded_demons_for`, `launch_fireball` (`momz`) | `P_XYMovement` clamp input, `P_ZMovement` | The clamp accepts any `fixed_t`; in-world impulses never exceed 125 units per hit |
| Actor slots 64; fireballs 24 (`MAX_LIVE_PROJECTILES`); monsters 8; items 16 + 8 drops | `map_extent::validate` (`max_actors`), `launch_fireball`, `spawn_map_things`, `max_world_items` | Store capacity and the per-tic sight budget | 24 is a measured margin from a 768-unit room; §5 derives the real bound from flight time and attack cadence |
| 64 sight queries and 1,048,576 sight work units per tic | `demon_expanded_map::action_sight` | CPU | Eight monsters issue at most three queries each per tic |
| 64 BLOCKMAP cells per hitscan, 128 intercepts, `64·(128+1024)` candidates | `combat_trace` | Source `P_PathTraverse` loop, `MAXINTERCEPTS`, CPU | Source behavior, not slack |
| `ThinkerStore::run` visit budget = slot count | `thinkers::run` | Termination with same-pass refills | Reachable only when the store is full (§5) |

Everything in the first three rows survives into the new policy. The 978/994/1,008/1,024
margins and both travel envelopes are the rows this milestone replaces for the new
policy only.

### Per-tic revalidation of immutable data

`preflight_map_demons` calls `monster_actor` → `validate_monster_type` →
`validate_demon_lifecycle_type` for every live actor every tic. For an item that reaches
`world_pickups::validate_definition`, which rebuilds the whole `OriginalActorDef` through
`descriptor()` (the sound-array closure that stopped inlining in M3h1), and
`world_pickups::validate_item` repeats `validate_definition` for the same item in the
same tic. For a monster it reaches `ActorRegistry::validate_type_original`, which walks
the state closure with `BTreeSet`s. All inputs are immutable after construction: the
registry is an `Arc<ActorRegistry>` that never changes, static sectors never move in
this profile, and the identity tables are compile-time constants. M3h2 measured about
40% of the `sixteen` samples here. Package E validates each type once per registry and
reuses the result; no check is removed.

## 2. Original C facts

- **`P_XYMovement`** ([p_mobj.c:114](../linuxdoom-1.10/p_mobj.c)): clamps `momx`/`momy`
  to `±MAXMOVE` (30 units) first. Then it loops: if `xmove > MAXMOVE/2 || ymove > MAXMOVE/2`
  it tries half of each component (`xmove/2`, then `>>= 1`), otherwise the whole remainder.
  Only *positive* components trigger the split, so `(-30, -30)` is one 30-unit attempt.
  Each attempt is one `P_TryMove`. A blocked player slides (`P_SlideMove`); a blocked
  missile explodes; anything else zeroes its momentum. **Nothing is ever rejected**:
  the original has no notion of an unsupported position.
- **`P_TryMove`/`P_CheckPosition`** ([p_map.c:375,451](../linuxdoom-1.10/p_map.c)): only
  the *destination* is tested. Things first over the bbox extended by `MAXRADIUS` (32),
  then lines over the exact bbox. `PIT_CheckLine` skips a line whose bbox does not strictly
  overlap the moving bbox (`<=`/`>=` comparisons), then `P_BoxOnLineSide != -1` skips a box
  entirely on one side. A one-sided line always blocks; `ML_BLOCKING` blocks
  non-missiles. So a destination is blocked exactly when the **open** destination bbox
  contains a point of a one-sided wall segment. Touching a wall is allowed.
- **`P_SlideMove`** ([p_map.c:560–740](../linuxdoom-1.10/p_map.c)): three corner
  traces of length `mom`, a flush move `FixedMul(mom, bestslidefrac - 0x800)`, then
  `P_HitSlideLine`, then one more `P_TryMove`, at most two retries, finally the
  stairstep (`y` then `x`). For `ST_HORIZONTAL`/`ST_VERTICAL` lines `P_HitSlideLine`
  zeroes one component and never reaches the angle arithmetic. Every destination it
  tries is at most `abs(mom) ≤ 30` from the current position per axis. For an
  **arbitrary** wall angle the projected move `newlen·cos(lineangle)` can exceed the
  component bound: with `P_AproxDistance(30,30) = 45` and a 22.5° wall the x move is
  `45·cos(22.5°)·cos(22.5°) ≈ 38.4`, more than a player's 32-unit box. That is why this
  milestone admits axis-aligned walls only (§4).
- **`P_Move`** ([p_enemy.c](../linuxdoom-1.10/p_enemy.c)): `tryx = x + speed·xspeed[dir]`,
  speed 8 (former humans, Imp) or 10 (Demon), diagonal factor 47000/65536; one
  `P_TryMove`, so a monster step is at most 10 units per axis.
- **Missiles** ([p_mobj.c:868–930](../linuxdoom-1.10/p_mobj.c)): `P_SpawnMissile` sets
  `mom = FixedMul(speed, cos/sin)`, so a fireball (speed `10·FRACUNIT`) moves 10 units
  per tic with each component at most 10; the half-step is 5. Its momentum never changes
  until it explodes, so the `MAXMOVE/2` split is unreachable for it.
- **BLOCKMAP** ([p_setup.c:479–487](../linuxdoom-1.10/p_setup.c),
  [p_maputl.c:376–430](../linuxdoom-1.10/p_maputl.c)): `bmaporgx = blockmaplump[0]<<FRACBITS`,
  cell = `(x - bmaporgx) >> MAPBLOCKSHIFT`. `P_BlockLinesIterator`/`P_BlockThingsIterator`
  return true (nothing to check) for a cell outside `[0,bmapwidth)×[0,bmapheight)`, and
  `P_SetThingPosition` does not link a thing whose cell is outside the grid. A wall
  outside the grid, or missing from a cell list, therefore does not stop anything.
- **Representable range of the operations** (fixed_t is a 32-bit `int`; coordinates
  are at most 32,767 units by the WAD format, but the arithmetic is tighter):
  - `P_AproxDistance(dx,dy) = dx+dy-min/2` needs `abs(dx)+abs(dy) < 32,768` units.
  - `R_PointToAngle` (`SlopeDiv`) computes `(unsigned)num<<3` with `num = abs(dy)` raw, so
    `abs(dy) < 8,192` units; `den>>8` is harmless.
  - `P_InterceptVector(v2,v1)`: `num = FixedMul((v1.x-v2.x)>>8, v1.dy) + FixedMul((v2.y-v1.y)>>8, v1.dx)`
    is bounded by `256·(G+A)·L` raw where `G` bounds wall vertices, `A` the trace start
    and `L = abs(v1.dx)+abs(v1.dy)`; `den` by `256·L·T` with `T` the trace component
    bound (2,048 for `MISSILERANGE`, 30 for a slide). `P_InterceptVector2` in p_sight.c is
    the same form (observer start, target delta).
  - `P_PointOnLineSide`: `FixedMul(line.dy>>FRACBITS, x - v1.x)`, a whole-unit times a
    raw difference: `L·(G+A)` raw, below `2^31` for `G+A ≤ 4,096` and `L ≤ 1,536`.
  - `P_DivlineSide`: whole-unit products `dx·dy < 2^24` for 4,096-unit differences.
  - `P_DamageMobj` thrust: `damage·(FRACUNIT>>3)·100/mass` is at most `819,200,000` for
    damage ≤ 1,000 and mass ≥ 100 (both admitted bounds), then two `FixedMul`s and two
    `int` additions into momentum.
  - `P_Thrust`: `move = forwardmove·2048`, at most `102,400` raw per tic.
  - `P_XYMovement` friction `FixedMul(mom, 0xe800)` and the clamp accept any `fixed_t`.
  - BLOCKMAP indexing: `(x - bmaporgx) >> 23` needs `abs(x - bmaporgx) < 32,768` units.
- **No rejection anywhere**: the original never refuses a move, a hit or a position.
  Zone memory is the only resource bound; `MAXINTERCEPTS` overflow is undefined.

## 3. The two open M3h2 stalls, reproduced

All three reproductions are committed Rust tests in
`crates/doom-sim/src/player_map_scale_tests.rs` (`stall_*`). They build one flat room
`x ∈ [-1000, 1200]`, `y ∈ [-1000, 1000]` with the test-only `Room` builder (axis walls
split at 1,024 units, one sector, cell-exact BLOCKMAP), which the historical policies
admit because its vertices stay within 2,048 and the player spawns within 994. Each test
asserts that the rejected tic leaves snapshot, RNG and events unchanged and that the
same rejection recurs for 64 further zero-command tics. The tests keep passing after
this milestone: the historical policies retain these contracts.

### 3.1 Near-wall hit (PlayerMonstersImpV1)

The player stands at `(984, 0)`, 16 units from the east wall at 1,000. An Imp at
`(800, 0)` is given the player as target and enters `S_TROO_ATK1` through the declared
controlled entry, so it launches at tic 16; the fireball reaches the player at tic 31.
`DamagePlan::current_original` runs `preflight_missile_impulse`, which adds the thrust
to the player's momentum and calls `travel_bounds`: `984 + ceil(160·M/3)` exceeds 994
for every admitted fireball damage (3 points give `M = 24,576` raw and 20 units of
reserved travel). Tic 31 fails with `PlayerBodyError::Unsupported`, rolls back, and the
restored fireball hits again on every later tic. **Reproduced; a deterministic freeze.**
In the original the player would simply be pushed 0.4–3 units east until the wall
stopped the body.

### 3.2 Base-master hitscan variant (PlayerMonstersV1)

The same room and player position with a shotgunner at `(936, 0)` and no Imp. The
shotgunner wakes through its own `A_Look`, waits out its reaction time and fires
`A_SPosAttack` at point-blank range. Each pellet that hits plans its own impulse through
`preflight_grounded_impulse`; one 3-point pellet already reserves 20 units past 994.
**Reproduced on the base policy with no Imp involved**, confirming the M3h2 note that
fireballs only make the envelope easier to reach.

### 3.3 "Fireballs past 994 stop the sim"

The player stands at `(-600, 0)`, far from the east wall so that strafing stays inside
the historical envelope; an Imp at `(-900, 100)` launches at tic 16 and the player
strafes for the next five tics, so the fireball misses and continues east along its
line. At tic 216 its x is 1,002: `preflight_monster_owner` rejects the non-owner actor
with `MonsterError::Arithmetic` (`monsters.rs`, `within(*v, 994)`) and keeps rejecting
because rollback restores the fireball. **Confirmed, not refuted.** The room is admitted
(vertices within 2,048) but the actor margin is 994, so any fireball that flies more
than 994 units from the origin freezes the world; the strafing itself would be rejected
by the same envelope from `x ≈ 530` on (`160·M/3` for a 7-unit slide is 373 units).

## 4. The new admitted domain and its proof

`PlayerMonstersMapScaleV1` (wire 14, context
`PLAYER-MONSTERS-MAP-SCALE-1-MEDIUM-GROUNDED-v1`, DSIM30) is a superset of
`PlayerMonstersImpV1`: same roster, weapons, items, drops, fireballs and transaction
rules. It changes only the geometry/coordinate admission and the resource budgets, and
only for itself. Historical policies keep every number in §1.

### 4.1 Geometry admitted at load (all checked once, before construction)

Let the vertex rectangle be `R = [xmin, xmax] × [ymin, ymax]` over all VERTEXES.

1. The inherited checks: one sector `[0,128]`, one-sided axis-aligned `ML_BLOCKING`
   lines, vertices/nodes/BLOCKMAP origin within **2,048** (`G = 2048`), line norm
   ≤ 1,536, no specials, no sky, THINGS ≤ 256, cells ≤ 8,192, raw words ≤ 65,536.
2. **Walled rectangle**: each of the four sides of `R` is completely covered by the
   union of linedefs lying on that side (closed-interval coverage, integer endpoints).
   Interior walls and pillars are free.
3. **BLOCKMAP covers and lists the walls**: `orgx ≤ xmin`, `orgy ≤ ymin`,
   `xmax < orgx + 128·w`, `ymax < orgy + 128·h` (so every point of `R` has a cell), and
   every linedef appears in the list of every cell its bbox touches (the M3a
   completeness test, applied to this policy).
4. **Linedefs ≤ 95**, so lines plus the at most 33 BLOCKMAP-linked things (player,
   8 monsters, 16 items, 8 drops; fireballs, puffs and blood carry `MF_NOBLOCKMAP`)
   never exceed `MAXINTERCEPTS` = 128 in one trace. This replaces the weapon policy's
   128-line bound for this policy; the fixed `seen` buffer still holds 128.
5. Every spawned actor's closed bbox lies inside `R`: THINGs at the batch, the player
   body at `spawn_player_body`, controlled dropped pickups at setup. An actor whose
   box crosses a wall at spawn could leave `R` with one step (its box is already on
   both sides), so such maps are rejected at load, not tolerated.

E1M1 spans about 3,900 × 3,200 units with longest single walls under 1,536, so a flat
room of its extent, with long walls split at vertices, fits this domain. Two-sided
lines, height changes and non-axis walls stay excluded.

### 4.2 Invariant and proof sketch

**Invariant I:** every actor that can move in XY (player, monsters, fireballs, corpses)
has its *open* bbox `(x−r, x+r) × (y−r, y+r)` inside the open rectangle `R°`; every
other actor (items, puffs, blood, exploded fireballs) has its center inside `R`.

*Preservation under one `P_TryMove`.* Let the mover start with its open box inside `R°`
and attempt a destination at most `s` units away per axis with `s < 2r`. The two open
boxes overlap (they are translates by less than their width). If the destination box
were not inside `R°` it would contain a point outside `R°` and, being connected and
containing an overlap point inside `R°`, a point of the boundary of `R`. That point lies
on a wall segment by condition 2, in a cell by condition 3, and that cell lists the
wall by condition 3; `P_CheckPosition` visits every cell that the destination bbox
touches (the point is strictly inside the box, so its cell is in `[xl,xh]×[yl,yh]`),
`PIT_CheckLine` sees the strict overlap and `P_BoxOnLineSide == -1`, and the move is
blocked. A blocked `P_TryMove` leaves the position unchanged. Hence any accepted
destination keeps I.

*Every XY move is such a `P_TryMove` with `s < 2r`:*

| Mover | Radius, `2r` | Step per attempt | Source |
| --- | ---: | ---: | --- |
| Player | 16, 32 | ≤ 30: clamp to `MAXMOVE`, halves above 15, whole negative components; slide flush ≤ `abs(mom)`, projected axis slide ≤ `abs(mom)`, stairstep ≤ `abs(mom)` | `P_XYMovement`, `P_SlideMove`, `P_HitSlideLine` axis cases |
| Former humans, Imp (walk) | 20, 40 | ≤ 8 (`speed·xspeed`) | `P_Move` |
| Demon (walk) | 30, 60 | ≤ 10 | `P_Move` |
| Monsters and corpses (thrust) | 20/30 | ≤ 30 after the clamp; blocked → momentum zeroed | `P_XYMovement` non-player branch |
| Fireball | 6, 12 | 5 (half-step), then 10 per tic; momentum constant | `P_SpawnMissile`, `P_CheckMissileSpawn`, `P_XYMovement` missile branch |

A blocked fireball explodes without moving; a blocked spawn check explodes at the
half-stepped position without relinking, so an exploded fireball's center is within 5
units of an Imp's center, hence inside `R` (an Imp's center is at least 20 inside).
Puffs are spawned 4 units before a wall intercept or 10 before a thing intercept along a
trace whose start and target are inside `R` (convex), blood at a thing hit, drops at the
dying monster's position, the fireball at the Imp's position: all centers inside `R`.
Items never move. I therefore holds after every tic, every public controlled operation
(damage adds momentum, not position; target/state entry/removal do not move) and every
spawn.

*Arithmetic under I.* All actor coordinates satisfy `abs(x), abs(y) ≤ 2048` (`A = 2048`)
and all wall vertices `G = 2048`, `L ≤ 1536`:

| Operation | Bound under the domain | Headroom |
| --- | ---: | --- |
| `P_InterceptVector`, `P_InterceptVector2` numerator | `256·(G+A+1)·L = 1,610,613,760` raw plus ≤ 1,538 rounding units | `2^31 − 1 = 2,147,483,647`, as in M3f2 |
| Same, denominator | hitscan `256·L·2048 = 805,306,368`; slide `256·L·30`; sight uses the target delta ≤ 4,096 per component: `256·L·4096 = 1,610,612,736` | fits |
| `P_PointOnLineSide` / `R_PointOnSide` | `FixedMul(dy >> 16, x − v1.x)`: a whole-unit factor ≤ 1,536 times a raw difference ≤ `4,096·65,536`; the 64-bit product shifted by 16 is ≤ 6,291,456 raw | fits |
| `P_DivlineSide` | whole-unit products ≤ `4,096²` | fits |
| `P_AproxDistance` of any actor pair or actor–wall difference | `abs(dx)+abs(dy) ≤ 8,192` units = `2^29` raw | fits |
| `R_PointToAngle` `SlopeDiv` | `num ≤ 4,096` units = `2^28` raw, `<<3` = `2^31` in `unsigned` | fits |
| BLOCKMAP cell index, hitscan DDA start/end minus origin | `abs(v − org) ≤ 4,096 + 2,048 + 32` units | fits; the DDA skips cells outside the grid |
| `P_DamageMobj` thrust and momentum sum | per hit ≤ 125 units (`damage ≤ 1000`, `mass ≥ 100`); in-world hits in one pass ≤ 8 attacks of ≤ 45 points plus 192 fireballs of ≤ 24 points ≈ 650 units; previous momentum ≤ 30 after the clamp | the policy's momentum bound is **4,096 units** (`2^28` raw) per component, checked with `checked_add`; any overflow is unreachable by two orders of magnitude |
| `P_SpawnMissile` `dist/speed`, `momz` | `dist ≤ 8,192·65,536 / 655,360 = 819`; `momz = 0` (flat) | fits |
| Sight BSP, eye, slopes | M3f2 domain (`A = G = 2048`, heights 128) | proven there |
| Hitscan `P_PathTraverse` | 2,048 range crosses at most 32 cells of 64; intercepts ≤ 95 + 33 = 128 | exact |

So the per-tic preflight of the new policy checks: every actor center inside `R` (a
derived constant of the loaded map, not 994), every XY momentum component within 4,096
units, Z and `momz` as before. By I these checks never fail; they stay as the proof's
runtime witness. `travel_bounds` and `attempt_bounds` are not applied to this policy.
Legal source moves, wall slides and hits at walls are therefore never rejected.

### 4.3 What stays rejected at load

Non-axis or two-sided lines, gaps in the bounding walls, a BLOCKMAP that does not cover
`R` or omits a wall from a cell, more than 95 linedefs, any vertex beyond 2,048 or line
norm above 1,536, heights other than `[0,128]`, specials, sky, more than 256 THINGs,
spawns whose box crosses a wall, more than 8 roster monsters or 16 items.

## 5. Caps and budgets: derived, not measured

The actor store is a hard capacity; a full store is a deterministic rejection. The
original has none, so the budget must exceed what admitted play can hold at any instant.

**Fireballs.** An Imp fires on entering `S_TROO_ATK3`. The shortest cycle back to a
launch is not the undisturbed one (`S_TROO_ATK3` 6 → `S_TROO_RUN1` 3 → `ATK1` 8 →
`ATK2` 8 = 25 tics) but the one cut short by damage: `P_DamageMobj` enters
`S_TROO_PAIN` (2) → `S_TROO_PAIN2` (2) → `S_TROO_RUN1` without clearing
`MF_JUSTATTACKED` and with `MF_JUSTHIT` set, so `A_Chase` at `RUN1` only clears the
flag and picks a direction, `RUN1` (3) ends, and `A_Chase` at `RUN2` may select the
missile state at once because `P_CheckMissileRange` returns true on `MF_JUSTHIT`:
`2 + 2 + 3 + 8 + 8 =` **23 tics minimum** between launches (found by the independent
review; the first draft said 25). A fireball holds a slot from its launch pass through
retirement: the launch pass (half step and first 10-unit move), then 10 units per pass
along a straight chord of the room, then `S_TBALLX1` (6 minus `P_Random()&3`, at
least 3) + `S_TBALLX2` (6) + `S_TBALLX3` (6) + one retirement visit. For the largest
room (`R` = 4,096 × 4,096, chord of the 12-unit-shrunk rectangle 5,776) that is at
most `1 + 578 + 18 + 1 = 598` passes, so one Imp holds at most `ceil(598/23) = 26`
fireballs and eight Imps **208**. This policy sets
`MAP_SCALE_MAX_LIVE_PROJECTILES = 208`; the launch check stays as the schema witness and
is unreachable in play (it needs eight Imps hit after every launch and winning every
range roll for 600 tics).

**Impacts.** A puff holds at most 17 passes (`S_PUFF1` 1–4, then 4+4+4, retirement), blood
at most 25 (`S_BLOOD1` 5–8, then 8+8, retirement; shorter when damage ≤ 12). The fastest
shooters are a chaingunner (`A_CPosAttack` every 4 or 5 passes: ≤ 8 live) and the
player's chaingun (one bullet per 4 passes: ≤ 7; a 7-pellet shotgun volley every ≥ 37
passes: ≤ 7; a weapon switch takes ≥ 32 passes, so volleys of two weapons never
overlap): player impacts ≤ 8. A shotgunner (3 per ≥ 33 passes) ≤ 3, a zombieman ≤ 2.

**Slots.** With `k` Imps and `8 − k` former humans:
`1 + 8 + 16 + 26k + (8 − k) drops + 8·(8 − k) impacts + 8 player impacts = 105 + 17k ≤ 241`.
Spawns in one pass ≤ `k + 7 + 4·(8 − k) = 39 − 3k`, so the visit count of one thinker
pass is at most `144 + 14k ≤ 256`. The policy fixes **320 slots**
(`MAP_SCALE_ACTOR_SLOTS`): the store never fills, `StoreError::Capacity` is unreachable,
and `StoreError::VisitLimit` keeps a 64-visit margin below its budget; the
recoverable-refill latch stays only for defense. The DSIM30 reader admits
`store.max_slots ≤ 320` and at most 208 non-pending fireballs.

**Unchanged caps.** Monsters 8 (sight budget: ≤ 3 queries each per tic, 64 allowed),
items 16, drops 8 (one per former-human death), 64 sight queries, hitscan 64 cells and
128 intercepts (now provably sufficient, §4.1 item 4).

## 6. Evidence plan

`tools/reference/m3h3_reference.py` reuses the inherited genuine harness (M3h2 bodies)
with `m3h3_geometry.py`, a walled-rectangle builder with split walls and a complete
cell-exact BLOCKMAP, and `m3h3_cases.py`: the three §3 scenarios at full scale now
progressing in C, near-wall and corner impulses, long-distance fireballs, full-room
traversal at `±2048`, mixed-roster fights at walls and corners, and drops/pickups far
from the origin. `tools/reference/map_scale_snapshot.py` and
`m3h3_snapshot_goldens.py` independently project DSIM30 (the DSIM29 layout with
version 30, policy 14, the new context and 320 slots). Both are committed before the
matching runtime. Cap and budget soaks are Rust-only tests with the full item and drop
budget filled; the C harness has no cap to reach.
