# M3h6 — Real-map scale: classic map limits toward E1M1

This note records the original-C facts, the inventory of every static-geometry cap of
the diagonal world with its replacement and derivation, the `MAXINTERCEPTS` analysis
and the options it leaves to the owner, the real-map data behind the boundary and
BLOCKMAP rules, the sky investigation, the E1M1 readiness list, the evidence plan and
the implementation plan of a new policy (suggested `PlayerMonstersRealMapV1`). It was
written and committed before any runtime change. Support and limits belong to
[COMPATIBILITY](COMPATIBILITY.md); DSIM33 belongs to [STATE_FORMAT](STATE_FORMAT.md);
measured observations belong to the evidence record (`validation/M3H6.md`, written in
Phase 2). All references are to the preserved `linuxdoom-1.10/` sources at `a77dfb96`.
Every number is in map units unless it says raw (16.16 bits). Line numbers name the
first line of the cited statement. Real-map figures come from the owner's IWADs, read
locally and read-only; only counts and pass/fail facts are recorded here.

## 1. Starting point

`PlayerMonstersDiagonalV1` ([M3h5](M3H5_REFERENCE_INVESTIGATION.md)) admits a closed
one-sided boundary of arbitrary wall angles with two-sided lines, static heights and
vertical movement, under caps inherited from M3h3: 95 linedefs, 256 sectors, vertices
within ±2,048, line norm `|dx| + |dy| ≤ 1,536`, 8,192 BLOCKMAP cells, 256 THINGs,
eight roster monsters, sixteen pickups. E1M1 has 486 linedefs, 88 sectors, 470
vertices, 747 segs, 239 subsectors, 238 nodes, 143 THINGs, a 36 × 23 BLOCKMAP and a
vertex rectangle of 4,576 × 2,816 units (§9). The reasons for the caps are of five
kinds, and the replacement of each depends on its kind:

- **(a) arithmetic**: the M3f2/M3h3 overflow envelope `G = A = 2,048`, `L ≤ 1,536`
  ([M3H3 §4.2](M3H3_REFERENCE_INVESTIGATION.md)); replaced by a certificate computed from
  the loaded map against the same source expressions (§4.2);
- **(b) `MAXINTERCEPTS`**: 95 lines plus at most 33 BLOCKMAP-linked things never exceed
  128 in one trace; replaced by the exact runtime count of the source (§4.4);
- **(c) allocation and fixed buffers**: `SeenLines` for 1,024 lines, `seen[128]` in the
  hitscan, 65,536 BLOCKMAP words, 262,144 cell references, 4 MiB of preparation;
  replaced by bounds taken from the WAD format's own `short` fields (§4.1);
- **(d) proof convenience**: 320 slots, 208 live fireballs, the roster; kept (§4.6);
- **(e) arbitrary or CPU budgets**: 256 sectors, 4,096 nodes and subsectors, 8,192
  cells, 256 THINGs, 16,384 sight and collision work units; replaced by values
  computed from the loaded map (§4.1).

Nothing in the movers, the collision, the traversal, the sight or the scheduler
changes. The milestone adds a policy, an admission, one runtime witness and the
evidence that large maps need.

## 2. Original C facts

### 2.1 Map loading has no limits of its own

- Every loader derives its count from the lump length: `P_LoadVertexes`
  ([p_setup.c:122](../linuxdoom-1.10/p_setup.c), count at 131), `P_LoadSegs` (159, 169),
  `P_LoadSubsectors` (202, 209), `P_LoadSectors` (231, 238), `P_LoadNodes` (264, 273),
  `P_LoadThings` (301, 310), `P_LoadLineDefs` (357, 366), `P_LoadSideDefs` (438, 445).
  No loader checks a count, an index or a reference; the only error is
  `P_GroupLines: miscounted` (552).
- The record types ([doomdata.h](../linuxdoom-1.10/doomdata.h)) are all `short`:
  `mapvertex_t` (60), `mapsidedef_t` (69, `sector`), `maplinedef_t` (84, `sidenum[2]`
  with −1 for "no side"), `mapsector_t` (141), `mapsubsector_t` (153), `mapseg_t` (163,
  `linedef`, `side`), `mapnode_t` (180, `unsigned short children[2]` with `NF_SUBSECTOR
  0x8000`, 178), `mapthing_t` (203). In memory `line_t::sidenum` is `short`
  ([r_defs.h:196](../linuxdoom-1.10/r_defs.h)) and `node_t::children` is `unsigned short`
  (277). **Hence the format itself bounds every table at 32,767 records** (a sidedef,
  linedef, vertex, sector or seg reference is a non-negative `short`; a node child is
  15 bits plus the leaf flag).
- **`P_LoadBlockMap`** (469): `blockmap = blockmaplump + 4` (475), every word byte-swapped
  as `short` (478–479), `bmaporgx/y = blockmaplump[0..1] << FRACBITS` (481–482),
  `bmapwidth/height = blockmaplump[2..3]` (483–484); `blocklinks` is `bmapwidth ·
  bmapheight` pointers (487–489). No size check. `P_BlockLinesIterator`
  ([p_maputl.c:472](../linuxdoom-1.10/p_maputl.c)) reads `offset = *(blockmap + offset)`
  (491) as `short`: **a list offset is a non-negative `short`, so the lump holds at most
  32,768 words**.
- **REJECT** is cached by one statement in `P_SetupLevel` (659) and read by
  `P_CheckSight` ([p_sight.c:313–325](../linuxdoom-1.10/p_sight.c)) as `pnum = s1 ·
  numsectors + s2; bytenum = pnum >> 3; bitnum = 1 << (pnum & 7); if
  (rejectmatrix[bytenum] & bitnum) return false`. A lump shorter than
  `ceil(numsectors² / 8)` bytes is read past its end (undefined).
- **`P_GroupLines`** (499) builds `sector->lines` from one pass over the linedefs
  (521–551) and the sector `blockbox` extended by `MAXRADIUS` and clamped to the grid
  (560–574); `P_SetupLevel` (584) loads BLOCKMAP, VERTEXES, SECTORS, SIDEDEFS, LINEDEFS,
  SSECTORS, NODES, SEGS, REJECT, then `P_GroupLines`, then THINGS (649–664) and
  `P_SpawnSpecials` (682).
- **`P_SpawnMapThing`** ([p_mobj.c:708](../linuxdoom-1.10/p_mobj.c)): type 11 is copied
  into `deathmatchstarts` while fewer than 10 are stored (718–725); types 1–4 are
  copied into `playerstarts` and `P_SpawnPlayer` is called outside deathmatch
  (729–736), which spawns nothing for a player not in the game; `options & 16` outside a
  net game returns (740); the skill bit is `1` for baby, `4` for nightmare, else `1 <<
  (gameskill − 1)` (743–751); an unknown `doomednum` is `I_Error` (758–761);
  `MF_COUNTKILL`/`MF_COUNTITEM` add to the totals (790–792); the angle is `ANG45 ·
  (angle / 45)` (794); `options & MTF_AMBUSH` sets `MF_AMBUSH` (795–796).
- **`P_SetThingPosition`** ([p_maputl.c:396](../linuxdoom-1.10/p_maputl.c)) links a thing
  without `MF_NOBLOCKMAP` into exactly one cell, that of its origin (425–442), and leaves
  it unlinked when the cell is outside the grid (447). Missiles, puffs and blood carry
  `MF_NOBLOCKMAP` ([info.c](../linuxdoom-1.10/info.c), `MT_TROOPSHOT`, `MT_PUFF`,
  `MT_BLOOD`); every pickup, decoration, corpse, monster and the player are linked.

### 2.2 Intercepts

- `#define MAXINTERCEPTS 128` ([p_local.h:154](../linuxdoom-1.10/p_local.h));
  `intercept_t intercepts[MAXINTERCEPTS]; intercept_t* intercept_p;`
  ([p_maputl.c:544–545](../linuxdoom-1.10/p_maputl.c)). **No statement in the source
  compares against `MAXINTERCEPTS`**: `PIT_AddLineIntercepts` writes `intercept_p->frac =
  frac; … intercept_p++` (603–606) and `PIT_AddThingIntercepts` likewise (668–671). The
  129th intercept of one trace is written past the array.
- **`P_PathTraverse`** (743): `validcount++` (774), `intercept_p = intercepts` (775), the
  `FRACUNIT` nudge off a cell edge (777–781), the 64-iteration DDA (846) that calls
  `P_BlockLinesIterator` under `PT_ADDLINES` (848–850) and `P_BlockThingsIterator` under
  `PT_ADDTHINGS` (854–856), stops at the end cell (860) and otherwise steps one axis
  (866–875); when neither step condition holds the same cell is visited again until the
  cap. `P_TraverseIntercepts` (683) then scans `count = intercept_p − intercepts` (692)
  entries in O(n²) (696–726).
- **Lines are added at most once per trace** (`ld->validcount == validcount → continue`,
  497–500). **Things are not deduplicated**: `P_BlockThingsIterator` (513) walks the cell's
  `blocklinks` chain (529–531) and `mobj_t::validcount` ([p_mobj.h:247](../linuxdoom-1.10/p_mobj.h))
  is never read or written. A thing is linked in one cell, so it is added once per visit
  of that cell, and more than once only when the DDA revisits a cell.
- A line is added when its endpoints read differently against the **infinite** trace
  line (`P_PointOnDivlineSide`, 570–576, for traces longer than 16 units; the `2` return
  for a point exactly on an axis trace, 161–181, counts as a side) and `frac ≥ 0`
  (584–591): lines beyond the trace's end but inside a visited cell are intercepts too.
  `PT_EARLYOUT` (595–599) has no caller.
- **Callers** (all in [p_map.c](../linuxdoom-1.10/p_map.c)):

  | Caller | Flags | Length | Things |
  | --- | --- | --- | --- |
  | `P_SlideMove` 695: three traces at 738, 740, 742 (`PTR_SlideTraverse`); up to three passes (709) | `PT_ADDLINES` | `(momx, momy)`, each within ±`MAXMOVE` = 30 after the clamp (p_mobj.c:137–145): at most 42.4 units | no |
  | `P_AimLineAttack` 1023 (call 1045, `PTR_AimTraverse`) | `PT_ADDLINES \| PT_ADDTHINGS` | `distance`: 1,024 for the player's bullet slope (p_pspr.c:607–616) and `P_SpawnPlayerMissile` (p_mobj.c:949–959); `MISSILERANGE` 2,048 for monsters (p_enemy.c:813, 835, 858); 64 for the fist | yes |
  | `P_LineAttack` 1063 (call 1082, `PTR_ShootTraverse`) | `PT_ADDLINES \| PT_ADDTHINGS` | `MISSILERANGE` 2,048 (p_pspr.c:639, 721; p_enemy.c:818, 841, 862); 64/65 for fist and saw | yes |
  | `P_UseLines` 1130 (call 1147, `PTR_UseTraverse`) | `PT_ADDLINES` | `USERANGE` 64 | no |

  `P_CheckSight` ([p_sight.c:300](../linuxdoom-1.10/p_sight.c)) uses no intercepts: it
  walks the BSP (`P_CrossBSPNode` 257, `P_CrossSubsector` 135) and deduplicates lines by
  `validcount` (171–174); its only fixed array is `sightcounts[2]` (47).
- A 2,048-unit trace spans at most 17 cells per axis, so the DDA reaches the end cell in
  at most 33 iterations; a 42.4-unit trace changes cell at most once per axis, so it
  visits at most three cells, all inside one 2 × 2 cell window.
- **`spechit[MAXSPECIALCROSS]`** (`#define MAXSPECIALCROSS 8`, p_map.c:67; array at 69) is
  written by `PIT_CheckLine` only `if (ld->special)` (240–243), unchecked, and read by
  `P_TryMove` (502–505) and `P_Move` (p_enemy.c:310–317).

### 2.3 The arithmetic of long lines and wide maps

The expressions bounded by M3f2/M3h3 (`G = A = 2,048`, `L ≤ 1,536`) are, with the exact
form of each product:

- `P_InterceptVector(v2 = trace, v1 = line)` ([p_maputl.c:231](../linuxdoom-1.10/p_maputl.c)):
  `den = FixedMul(line.dy >> 8, trace.dx) − FixedMul(line.dx >> 8, trace.dy)` and `num =
  FixedMul((line.x − trace.x) >> 8, line.dy) + FixedMul((trace.y − line.y) >> 8,
  line.dx)`. With integer vertices, `line.dy >> 8` is exact, and each `FixedMul` is a
  64-bit product shifted by 16, so in units `den = 256 · (dy·tx − dx·ty)` and `num =
  256 · (Δx·dy − Δy·dx)` up to floors, where `Δ = line.v1 − trace.start`. The numerator
  is a cross product, so it is the same for **every** point of the line: `|num| ≤ 256 ·
  |Δ_Q|_∞ · (|dx| + |dy|)` for any `Q` on the line's infinite line, plus at most `L + 2`
  raw units of rounding (the two `>> 8` floors contribute less than `L`, the two
  `FixedMul` floors one each, [M3F2](M3F2_REFERENCE.md#derived-arithmetic-domain)). A line is only
  tested when it is listed in a visited cell, and every point of a visited cell is within
  `2,048 + 128 + 1` units per axis of the trace start (trace length, one cell, the nudge).
  So for hitscans `|num| ≤ 256 · 2,177 · L + L + 2` and `|den| ≤ 256 · L · 2,049`,
  **independent of the map's extent**.
- `P_InterceptVector2(v1 = strace, v2 = line)` ([p_sight.c:109](../linuxdoom-1.10/p_sight.c))
  has the roles swapped: `num = 256 · (Δ × s)` with `s` the sight delta and `Δ` from the
  observer to a vertex of a linedef whose endpoints straddle the sight line (`P_DivlineSide`
  55, used at 165–169). The vertex is then within the line's Euclidean length `L₂` of the
  sight line, so `|num| ≤ 256 · |s|₂ · L₂ + L + 2` and `|den| ≤ 256 · |s|₂ · L₂ + 2`.
  Observer and target are actor centers in the vertex rectangle `R`, so `|s|₂ ≤ diag(R)`.
- `P_PointOnLineSide` ([p_maputl.c:91–99](../linuxdoom-1.10/p_maputl.c)), `R_PointOnSide`
  ([r_main.c:201–204](../linuxdoom-1.10/r_main.c)) and `P_DivlineSide` multiply a
  whole-unit delta by a raw or whole-unit difference: `(|dx| + |dy|) · D` with `D` the
  map's span, which fits for every `short` map.
- `P_AproxDistance` ([p_maputl.c:49](../linuxdoom-1.10/p_maputl.c)) needs `|dx| + |dy| <
  32,768` units for any actor pair: `span_x + span_y < 32,768`.
- `R_PointToAngle` ([r_main.c:292](../linuxdoom-1.10/r_main.c)) → `SlopeDiv`
  ([tables.c:50](../linuxdoom-1.10/tables.c)) computes `(num << 3) / (den >> 8)` in
  `unsigned`; `num` is the smaller raw component, so the quotient is the source's for
  `|Δ| < 8,192` units per axis; beyond that the unsigned shift wraps (defined in C, but a
  different angle than the geometric one). It is applied to actor pairs (`A_FaceTarget`,
  `P_SpawnMissile`) and, in `P_HitSlideLine` (p_map.c:610), to a line's `(dx, dy)`.
- BLOCKMAP indexing `(x − bmaporgx) >> MAPBLOCKSHIFT` needs `|x − bmaporgx| < 32,768`.
- The DDA's `FixedDiv(y2 − y1, abs(x2 − x1))` saturates (`m_fixed.c`), as before.

### 2.4 Sky in the game code

`skyflatnum` is set in `G_DoLoadLevel` ([g_game.c:454](../linuxdoom-1.10/g_game.c)) from
`SKYFLATNAME "F_SKY1"` ([r_sky.h:32](../linuxdoom-1.10/r_sky.h)). Outside the renderer it
is read in exactly two places:

- **`P_XYMovement`** ([p_mobj.c:173–186](../linuxdoom-1.10/p_mobj.c)): a missile whose
  `P_TryMove` fails is **removed** (not exploded) when `ceilingline && ceilingline->backsector
  && ceilingline->backsector->ceilingpic == skyflatnum` ("Hack to prevent missiles exploding
  against the sky. Does not handle sky floors"). `ceilingline` ([p_map.c:63](../linuxdoom-1.10/p_map.c))
  is reset to `NULL` at the start of `P_CheckPosition` (400) and set by `PIT_CheckLine`
  to the **latest visited** two-sided line that lowered `tmceilingz` (`if (opentop <
  tmceilingz) { tmceilingz = opentop; ceilingline = ld; }`, 227–231). Because
  `P_BlockLinesIterator` stops at the first blocking line, `ceilingline` at the moment of
  the refusal is whatever two-sided line was visited **before** the blocker in the same
  check, in cell and list order. A fireball that hits a one-sided wall under a sky ceiling
  explodes unless such a line preceded it. `P_ZMovement` (246) has no sky test: a missile
  that reaches the ceiling explodes (342–347).
- **`PTR_ShootTraverse`** ([p_map.c:956–964](../linuxdoom-1.10/p_map.c)): at `hitline:`, `if
  (li->frontsector->ceilingpic == skyflatnum) { if (z > li->frontsector->ceilingheight)
  return false; if (li->backsector && li->backsector->ceilingpic == skyflatnum) return
  false; }` — no puff and the trace ends ("don't shoot the sky!", "it's a sky hack
  wall"). `P_SpawnPuff` and `P_SpawnBlood` have no sky dependency.

### 2.5 Specials at load

`P_SpawnSpecials` ([p_spec.c:1239](../linuxdoom-1.10/p_spec.c)) starts light thinkers for
sector specials 1, 2, 3, 4, 8, 12, 13, 17 (1276–1329), `totalsecret++` for 9 (1305), door
thinkers for 10 and 14 (1310, 1325) and collects type-48 scrollers into
`linespeciallist[MAXLINEANIMS = 64]` without a bound check (1336–1344). Everything in this
section stays rejected (§4.8).

## 3. Rust inventory (what exists, what changes)

| Source body | Rust | Status for M3h6 |
| --- | --- | --- |
| Loaders, `short` record types, offsets | `doom-map` (`parse.rs:16–37` limits; references ≤ `i16::MAX` at 423; BLOCKMAP offsets ≤ `i16::MAX` at 541; REJECT length at 207–222) | Present. The format bounds of §2.1 are already enforced by the parser. No change. |
| `P_CheckSight` REJECT read | `sight.rs:260, 302` | Present, source form. No change. |
| `P_PathTraverse`, `PIT_Add*Intercepts` (hitscan) | `combat_trace::collect_limited` (`combat_trace.rs:140–319`): 64-cell loop (224), `seen: [bool; 128]` by **line index** (221, 229), `InterceptLimit` at the 129th intercept (320–323), candidate budget `64 · (128 + 1024)` (276) | **Change** the `seen` buffer (a line index ≥ 128 is an error today, so no map above 128 lines can fire a hitscan). Keep the 128 count as the runtime witness (§4.4). |
| `P_PathTraverse` (slide) | `movement::path` (`movement.rs:322–410`): unbounded `Vec` of hits (364), `SeenLines` for 1,024 lines (363), 64-cell loop (365) | **Add** the 128 count. `SeenLines` becomes format-sized. |
| `P_UseLines` | absent; `BT_USE` is `SimError::UnsupportedButtons` (`player_commands.rs:80–91`) | Stays rejected. |
| `P_GroupLines` sector line lists | absent; `noise::flood` scans every linedef per visited sector (`noise.rs:12–47`) | **Add** prepared per-sector line lists for this policy (same results, O(lines) per flood). |
| `P_SpawnMapThing` types 1–4, 11 | rejected by `MapSpawnError::StartOrInvalidType` under every map-demon profile (`spawning.rs:374–375`); the skill and `options & 16` filters exist (387–395) | **Admit** as source records for this policy (§4.7). |
| Admission | `validate_diagonal_geometry` (`player_map_scale.rs:476`), `Boundary::derive` (352), `RoomBounds::derive_diagonal` (219, the 95-line check at 220), `admitted_lines` (688), `with_demon_expanded_profile` (`demon_expanded_map.rs:38–122`: 256 THINGs at 60, specials 63–67, ±512 at 68–74, `F_SKY1` at 75–81, `map_extent::validate_with_actor_cap` at 97), `create_bounded` (`lib.rs:586–666`: `Topology::new_bounded(&world, 256)` at 619, `prepare_map_extent` at 615) | **Add** `validate_real_map_geometry`, the arithmetic certificate, the slide-window certificate, `RoomBounds::derive_real_map`; per-map values replace the (c)/(e) constants listed in §4.1. Older policies keep every existing function unchanged. |
| Per-tic budgets | sight queries ≤ 64 and work ≤ 1,048,576 with `extent_sight_work + 5 · lines` per query (`demon_expanded_map.rs:162–170`); collision work `nodes + 4·lines + 4·actors ≤ 16,384` (`map_extent.rs:94–103`) | **Derive** from the map and the roster (§4.1 rows 14–15). |
| Policy plumbing | `original_registry.rs` (variant, `wire_id` 54–73, predicates 95–252, `with_original_registry` 834–916), `player_monsters.rs:106` | **Add** the variant (wire 17), `has_real_map`; `has_diagonals`/`has_heights`/`has_map_scale` include it. |
| DSIM | `snapshot.rs:522–570` (version), ranges `21..=32` etc.; `diagnostic.rs:204–217`, `378–381`, `1189–1195` (slot bound), `758` (THINGS bytes ≤ 2,560) | **Add** version 33 (§5). |

## 4. The admitted domain and the proof

`PlayerMonstersRealMapV1` (suggested; wire 17, context
`PLAYER-MONSTERS-REALMAP-1-MEDIUM-v1`, DSIM33) is a superset of
`PlayerMonstersDiagonalV1`: same roster, weapons, items, drops, fireballs, infighting,
statistics, 320 slots, 208 live fireballs, the same boundary rules C1–C3, the same
closed-cell BLOCKMAP rule, the same spawn admission and the same escape witness. It
changes the caps, the admission arithmetic, the player-start handling and adds one
runtime witness, for itself only.

### 4.1 Every cap, before → after, with its derivation

| # | Cap | Diagonal (before) | Real map (after) | Derivation of the new bound | Kind |
| --- | --- | ---: | ---: | --- | --- |
| 1 | Linedefs | 95 (`MAP_SCALE_WALL_LINES`, `player_map_scale.rs:21`) | 32,767 | `short` references (§2.1): `mapseg_t::linedef`, `line_t::sidenum`; the parser's `max_records` (32,768) and `≤ i16::MAX` reference check. The `MAXINTERCEPTS` role of the 95 moves to the runtime witness (§4.4). | (b) → format |
| 2 | Line-index buffers | 1,024 (`MAX_LINES`, `line_work.rs:4`; `seen: [bool; 128]`, `combat_trace.rs:221`) | 32,768 bits | a `[u64; 512]` bitset (4 KiB, zeroed per trace) sized by row 1; no allocation in a movement or combat decision. | (c) → format |
| 3 | Sidedefs, vertices, segs | none / coordinates only / none | 32,767 each | `short` references (§2.1); parser. No new check. | format |
| 4 | Sectors | 256 (`MAP_SCALE_MAX_SECTORS`, `map_scale.rs:8`; `Topology::new_bounded`) | 32,767 | `mapsidedef_t::sector` is `short`; `Topology::new_bounded(&world, map.sectors().len())`. REJECT length is `ceil(n²/8)` by the parser. | (e) → format |
| 5 | Subsectors, nodes | 4,096 each (`map_extent.rs:61–62`, `multisector.rs:148–149`) | 32,767 each | `children[2]` is `unsigned short` with `NF_SUBSECTOR` as bit 15: 15 bits of index. | (e) → format |
| 6 | BLOCKMAP cells, words | 8,192 cells (`MAP_EXTENT_MAX_CELLS`), 65,536 words (`map_extent.rs:63–64`) | words ≤ 32,768; cells ≤ words − 4 | list offsets are `short` word indices (§2.1, `P_BlockLinesIterator` 491); the parser already requires every offset ≤ `i16::MAX`. | (e)/(c) → format |
| 7 | Prepared cell references, preparation bytes | 262,144 refs (`MAP_SCALE_MAX_REFERENCES`), 4 MiB (`MAP_EXTENT_PREPARATION_BYTES`) | computed | references are the lump's own listings (≤ 32,768 words); the byte total is a function of rows 1–6 and is recorded, not capped (its format maximum is a few MiB; the parser's 128 MiB decoded bound stays). | (c) → derived |
| 8 | BLOCKMAP covers `R`; every line listed in every closed cell it meets | required | required, unchanged | the M3h5 proof needs it; real-map data in §4.5. | (d) kept |
| 9 | Coordinates | vertices, BLOCKMAP origin, node origins/deltas within ±2,048 (`QUERY_UNITS`, `map_extent.rs:13`) | the arithmetic certificate of §4.2, evaluated on the loaded map | §2.3; source constants `MISSILERANGE`, `MAPBLOCKUNITS`, `FRACBITS`, `i32`. | (a) → derived |
| 10 | Line norm | 1,536 (`LINE_NORM_UNITS`, `map_extent.rs:14`) | ≤ 3,853 from the hitscan numerator and the per-map sight inequality | §4.2 (i), (ii). | (a) → derived |
| 11 | Sector heights | ±512 (`demon_expanded_map.rs:68–74`); outer 1,024 | ±512, **kept** | the M3h4 vertical envelope (fireball chord 968, z witness 2,048, eye/opening clipping) is not re-derived here; E1M1 needs 264 (§9), 58 of 68 IWAD maps fit. Follow-up §8.3. | (a) kept |
| 12 | THING records | 256 (`MAX_DEMON_MAP_THINGS`) | 32,768 | lump length / 10; the parser. Spawned actors are still bounded by the roster (row 16). The DSIM reader's `≤ 2,560` THINGS bytes (`diagnostic.rs:758`) is derived from the stream under 33. | (e) → format |
| 13 | Player starts 1–4 and 11 | rejected (`StartOrInvalidType`) | admitted as records (§4.7) | `P_SpawnMapThing` 718–736. | domain |
| 14 | Sight work per query | `MAP_EXTENT_SIGHT_WORK` 16,384 (`map_extent.rs:8`) | `expanded_work(map)` exactly (`sight.rs:174`), no fixed threshold | the full BSP expansion `2 · nodes + Σ(1 + segs)` is a function of the map (E1M1 1,462; the largest IWAD map about 5,400). The per-tic budget becomes `queries ≤ 3 · monsters` and `work ≤ queries · (expanded_work + 5 · lines)` ([M3H3 §1](M3H3_REFERENCE_INVESTIGATION.md): at most three queries per monster per tic). | (e) → derived |
| 15 | Collision work per query | `nodes + 4·lines + 4·actors ≤ 16,384` (`map_extent.rs:94–103`) | computed and recorded | a function of rows 1, 5 and the 320 slots (E1M1 3,462; format maximum about 165,000 — a CPU figure, not a safety bound). | (e) → derived |
| 16 | Roster, slots, fireballs, pickups, drops | 8 / 320 / 208 / 16 / 8 | **unchanged** | (d) bounds of M3h3 §4.1; they also keep the linked-thing term of §4.4 at 33. Raising them is the suggested next milestone (§8.4). | (d) kept |
| 17 | Momentum, z, momz witnesses | 4,096 / 2,048 / 1,024 | unchanged | `2^28` raw plus any coordinate within the `short` range cannot overflow a checked add. | (a) kept |
| 18 | DDA iterations | 64 | 64 | source constant (`P_PathTraverse` 846). | source |

### 4.2 The arithmetic certificate

At load, with `R` the vertex rectangle (`span_x`, `span_y`, `diag² = span_x² + span_y²`),
`L₁ = max(|dx| + |dy|)` and `L₂² = max(dx² + dy²)` over all linedefs, `C = max(|dx|,
|dy|)` over all linedefs, `N = max(|dx| + |dy|)` over all node partitions, and `org` the
BLOCKMAP origin, the policy requires (all in exact `i64`/`u128` integer arithmetic):

- **(i) hitscan intercepts**: `256 · 2,177 · L₁ + L₁ + 2 ≤ 2³¹ − 1` and `256 · L₁ · 2,049 ≤
  2³¹ − 1`, i.e. `L₁ ≤ 3,853`;
- **(ii) sight intercepts**: `65,536 · diag² · L₂² ≤ (2³¹ − 1 − L₁ − 2)²` (the squared form
  of `256 · diag · L₂ + L₁ + 2 ≤ 2³¹ − 1`);
- **(iii) `P_AproxDistance`**: `span_x + span_y < 32,768`;
- **(iv) `SlopeDiv`**: `max(span_x, span_y) < 8,192` and `C < 8,192` (dropped in Phase 2
  if `movement::slope_div` is shown to reproduce the unsigned wrap bit-exactly; owner
  decision 10);
- **(v) grid**: `org ≤ min(R)`, `max(R) < org + 128 · (width, height)`, and `max(R) − org <
  32,768`;
- **(vi) side tests**: `(L₁ + N) · max(span_x, span_y) < 2³¹` (always true for `short`
  maps; stated for completeness);
- **(vii) heights**: within ±512 (row 11).

Every actor center stays in `R` (Invariant Ω of M3h5, unchanged), so every actor–vertex
and actor–actor difference is bounded by the spans used above; the per-tic witnesses
(center in `R`, momentum, z) are unchanged and never fire. The slide traces are a special
case of (i) with `2,177` replaced by `171`. E1M1: `L₁ = L₂ = 832`, `diag = 5,373`, `256 ·
diag · L₂ ≈ 1.14 · 10⁹`; (i)–(vii) all hold. Across the 68 IWAD maps: (i) holds for 64,
(ii) for 46, (iii) for 68, (iv) for 67, (vii) for 58, all together for 39 (every E1
map except E1M2, E1M3 and E1M8). A failing map is rejected with
`MapScaleError::Arithmetic` (a new field-less variant) — the Rust arithmetic is checked
everywhere, so admitting such a map would be a checked error at the first overflowing
query rather than undefined behavior; the certificate keeps the M3h3–M3h5 standard that
the admitted domain is proven, not merely caught (owner decision 8).

### 4.3 The boundary proof carries over

Lemma A, Theorem B and the escape witness of [M3h5 §4.2–4.5](M3H5_REFERENCE_INVESTIGATION.md)
use only C1–C3, the closed-cell BLOCKMAP rule, the spawn rule and the reading lemma;
none of them reads a count or a coordinate bound. With rows 8 and 16 unchanged the proof
is unchanged. Load-time cost: `Boundary::derive` is O(n²) over one-sided walls (E1M1 306
walls: 46,665 pairs; the largest IWAD wall set, 1,276 walls: 813,450 pairs), the
closed-cell check is O(listings), the parity ray is O(walls) per spawn.

### 4.4 `MAXINTERCEPTS`

**When a real-map trace exceeds 128.** From §2.2, the intercepts of one trace are the
distinct lines listed in the visited cells whose endpoints straddle the infinite trace
line with `frac ≥ 0`, plus every linked thing in a visited cell whose diagonal straddles
it, counted once per visit. Things dominate on real maps: an exact replica of the cell
walk and of both `PIT_Add*Intercepts` bodies, run over E1M1 with every UV THING linked at
its spawn position (116 things), 7,927 start points (a 24-unit grid of odd-parity points
plus every THING) and 512 directions of a 2,048-unit trace, gives a maximum of
**67 intercepts** (lines alone at most 27, things alone
at most 63, from a row of armor bonuses). The source overflows only on
maps with long sight lines through dozens of linked things, or when the DDA stalls and
re-adds a cell's things up to 64 times.

**Under this policy's roster** the linked things are at most 33 (`MAX_LINKED_THINGS`,
`player_map_scale.rs:48`: the player, eight monsters or corpses, sixteen pickups, eight
drops; missiles, puffs and blood are `MF_NOBLOCKMAP`), unchanged by row 16, but the stall
quirk can re-add them per iteration, and the line term is no longer bounded by 95.

**Options.**

- **(a) Prove per map at load.** For slide traces this is exact and cheap: a slide trace
  adds lines only, from at most three cells inside one 2 × 2 window (§2.2), so `max over
  windows of distinct listed lines ≤ 128` proves every slide (E1M1: 28; IWAD maximum 101).
  For hitscans it is not: a 2,048-unit trace visits up to 33 cells chosen by a continuous
  start and direction, the exact maximum over all digital paths is an enumeration the
  loader cannot afford, and the sound shortcut (all lines of a 17 × 17 cell window) is
  larger than 128 on E1M1 (several hundred lines); the thing term also depends on the
  stall quirk.
- **(b) Refuse the tic with a checked error** when the 129th intercept of a trace would
  be written: exact (the count is the source's own), cheap (one comparison per
  intercept), and in the spirit of the corner jump: the complete canonical state is
  preserved and the error names the cause. The hitscan path already returns
  `CombatError::InterceptLimit` at 128 (`combat_trace.rs:320–323`); the slide path does
  not count. The one legal source behavior refused is the undefined overflow itself.
- **(c) Emulate the overflow.** The bytes after `intercepts[]` are whatever the linker
  placed there; demo-compatible ports emulate the DOS executable's layout
  (`intercept_p` and the globals of the DOS build), not the Linux one. This needs the
  M4 layout decision and must not be attempted here.

**Recommendation: (b) for every trace, plus the slide half of (a) as an admission rule.**
Concretely: `movement::path` and `combat_trace::add` refuse the 129th intercept with
`MapScaleError::Intercepts` (hitscan: the existing `InterceptLimit`, surfaced under the
new name through the checked transaction so the tic is discarded and the state is
byte-identical, tested like `Escape`); the slide-window certificate (`≤ 128` distinct
lines in every 2 × 2 cell window) is an admission check so that the slide witness never
fires; the hitscan witness is reachable only by construction (a Rust-only variant with
129 fins across one trace) and is never observed in C, because in C it is undefined.
`PT_ADDTHINGS` re-adding on a stalled DDA is reproduced, not deduplicated (Phase 2 checks
that `collect_limited` does so today).

**`spechit`** stays unreachable: `PIT_CheckLine` writes it only `if (ld->special)`
(p_map.c:240), and every line special is rejected at load (§4.8), so `numspechit` is 0 in
every `P_TryMove` and `P_Move`.

### 4.5 Real node-builder output: C3, dangling walls, BLOCKMAPs, REJECT

Measured over the 68 maps of the two IWADs (counts only; the per-map table is
[validation/M3H6_IWAD_COUNTS.md](validation/M3H6_IWAD_COUNTS.md), the output of
`tools/iwad_counts.py --markdown`):

| Rule | E1M1 | Maps satisfying it | Exceptions |
| --- | --- | ---: | --- |
| C1 endpoint-only contacts of one-sided walls | 0 violations | 65 / 68 | E1M3 (1), E4M1 (2), MAP02 (1) |
| C2 even degree at every one-sided vertex | 0 odd vertices | 56 / 68 | E1M3 (2), E4M1 (8), E4M3 (6), E4M8 (2), MAP02 (2), MAP14 (12), MAP16 (2), MAP17 (2), MAP18 (4), MAP22 (12), MAP26 (2), MAP30 (2) |
| C3 one-sided front faces odd parity (tested where C1 and C2 hold) | 0 of 306 | 55 / 68 | MAP19 (1 backwards wall) |
| Closed-cell BLOCKMAP listing | 0 missing (831 extra) | 67 / 68 | E4M7 (8 listings missing on 5 lines) |
| BLOCKMAP covers every vertex | yes | 68 / 68 | — |
| REJECT length = `ceil(n²/8)` | 968 = 968 | 68 / 68 | — |
| Two-sided lines with the same sector on both sides | 6 | — | harmless: nothing in the proof reads two-sided lines |

So id's builder lists every line in every cell it meets (with extra bbox cells), the
boundary rules hold on E1M1 and on 55 of 68 maps, and the exceptions are the map errors
M3h5 anticipated (a wall ending where only two-sided lines continue, one backwards wall,
one BLOCKMAP with eight missing listings). **Recommendation for M3h6: keep C1–C3 and the
closed-cell rule unchanged** (they are what the proof needs and what E1M1 satisfies) and
record the exceptions. The relaxation for the 13 other maps is a later question with two
shapes: admit a map with odd-degree walls by closing the parity domain with the two-sided
lines that continue the boundary (a new definition of `S` whose proof must handle lines
the mover can cross), or admit it with a per-tic parity witness for every mover (M3h5
option 2(c), about 3,000 `i64` operations per tic on E1M1) and no load-time proof. A
missing BLOCKMAP listing cannot be proven away: the source walks through such a wall at
that cell, and the only faithful admission is the per-tic witness.

REJECT needs nothing: the parser rejects a short lump (`RejectTooShort`, the one source
read past a lump the policy refuses), sight reads the bit the way the source does, and
the table is static data copied into DSIM. A wrong REJECT changes sight results exactly
as it does in the source.

### 4.6 What the per-tic budgets become

With the roster unchanged, the per-tic sight budget is `queries ≤ 24` and `work ≤ 24 ·
(expanded_work(map) + 5 · lines)` (E1M1: 93,408; the largest IWAD map about 340,000),
both derived at load and recorded; the thinker visit budget (320) and the fireball bound
(208) are unchanged. Per-sector line lists (row 3 of §3) make `noise::flood` O(lines)
per noise instead of O(sectors · lines) (E1M1: 486 instead of 42,768 line reads per
flood); results are identical because the flood only asks "which two-sided lines leave
this sector", in linedef order.

### 4.7 Player starts and THING filtering

For this policy `spawn_map_things` records types 1–4 in `player_starts` and up to ten
type-11 records in `deathmatch_starts` (`MapSpawnReport`, the generic path at
`spawning.rs:377–386`) instead of rejecting them; the player body is still spawned by the
caller, whose position must satisfy the M3h5 spawn rule as before, and the example and
harness read the type-1 record for it. The skill bits and `options & 16` are already
applied as `P_SpawnMapThing` does (387–395); a THING of an unadmitted `doomednum` still
rejects the map (`UnsupportedType`), which is what keeps E1M1 out (§9).

### 4.8 What stays rejected at load

Every line special and sector special (§2.5), doors, lifts, crushers, moving sectors,
teleporters, damaging floors, sky ceilings and floors (§6), view bob, presentation,
unadmitted THING types, the roster above 8 / 16, heights outside ±512, maps failing the
certificate (§4.2), maps failing C1–C3 or the closed-cell rule, a short REJECT, more than
128 distinct lines in a 2 × 2 cell window, `BT_USE`.

## 5. DSIM33 and wire 17

DSIM33 is the DSIM32 field layout and event vocabulary under version 33, policy 17 and
the new context. **No persistent field is added.** The caps that the DSIM32 reader checks
as constants become values derived from the map carried in the stream: the slot bound
stays 320 (`slot_bound_for`, `30..=33`), the THINGS byte bound is `10 · things` from the
stream, sector and line indices are already `u16`, positions are checked against the
stream's own vertex rectangle, and `Diagnostic::decode_real_map` reads 33 only; every
earlier reader rejects 33. `MapScaleError::{Arithmetic, Intercepts}` are checked errors,
not events. The independent encoder `real_map_snapshot.py` extends `diagonal_snapshot.py`
without editing it, as M3h5 did with M3h4's.

## 6. Sky ceilings: recommendation **out** for M3h6

What admitting `F_SKY1` ceilings statically would take (§2.4): a `ceiling_line` in the
collision result set to the latest visited two-sided line that lowered the ceiling, in
the source's cell and list order, and read only on a refused missile step
(`P_XYMovement`); and the two `PTR_ShootTraverse` sky returns before the puff. Both are
small bodies with exact citations, and `CombatError::Sky` plus the `F_SKY1` load check
already mark the places. Against it: the `ceilingline` value is **order-dependent and
truncated by the first blocker** (§2.4), so its evidence needs scenarios that pin the
visit order of two-sided and one-sided lines in one cell, which is a new kind of
observation; vertical fireballs at sky ceilings explode as before (no `P_ZMovement`
test), which must be observed too; and nothing in the M3h6 goal needs it — E1M1-sized
synthetic geometry is admitted without sky, and E1M1 itself stays out for the reasons of
§9 regardless. Recommendation: keep sky rejected in M3h6 and make it the first item of
the milestone that admits E1M1's decorations and remaining THINGs, with the plan above.
If the owner prefers it in, Phase 2 adds step 5b (§7) and the `sky_hack_wall`,
`sky_one_sided_wall`, `sky_ceiling_fireball`, `sky_shot_high`, `sky_shot_hack` scenarios.

## 7. Evidence plan

### 7.1 Synthetic maps and the C harness

`tools/reference/m3h6_geometry.py` builds rooms from polygons like `m3h5_geometry.py`
and adds an **automatic recursive node builder**: partitions are chosen among the seg
lines by the idbsp cost (splits weighted against imbalance), splits are computed exactly
and rounded to integer vertices the way DoomBSP does, every leaf is asserted non-empty
and single-sector by its segs' sides, and the BLOCKMAP lists every line in every closed
cell it meets with the origin at the vertex minimum. The Python asserts of the M3h5
builder (128 lines, 256 vertices, 128 leaves) are lifted; a new `m3h6_shim.h` sizes the
harness arrays from the generated maps (lines 4,096, sides 8,192, sectors 1,024, segs
16,384, leaves and nodes 8,192, cells 16,384, REJECT 131,072 bytes) and keeps
`intercepts[128]` exactly, so the sanitizer build would report any overflow of the C
harness. A new observer prints `intercept_p − intercepts` on entry to
`P_TraverseIntercepts` so every trace's intercept count is a recorded observation.

| Map | Geometry | Purpose |
| --- | --- | --- |
| town | 516 linedefs, 74 sectors, 182 subsectors, 2,176 × 1,184 units: a 6 × 4 grid of octagonal rooms with corridors, steps, pillars and window bands, 16 pickups, the roster | admission at E1M1 size; soak; pursuit across many sectors; sight across dozens of subsectors |
| gallery | a 2,048-unit hall whose long walls carry 32 triangular teeth each (64-unit pitch, 48 deep; a fin with a free end would break C2) and 16 pickups along the line of fire | hitscan and autoaim traces with 60–110 intercepts; thing re-adding on a stalled DDA if one occurs |
| extent | a 2,400 × 1,800 room whose boundary holds a 2,750-unit diagonal wall of norm 3,850 and which a 2,683-unit two-sided diagonal splits, so (i) sits at 99.9% and (ii) at 98.3% of `2³¹` (Phase 2 correction: the Phase 1 sketch of a 3,800-unit wall in a 4,000 × 3,000 span fails (ii), which bounds `diag · L₂` by `2²³`, so no wall near the (i) limit can share a map with a span near it) | `P_InterceptVector`/`P_InterceptVector2` sums near the limit under the overflow sanitizer; long-range sight across the two-sided line; a slide along the long wall; a fireball over 2,100 units |
| dense | a 512 × 256 lane whose middle 240 units carry 30 teeth on each wall: 123 distinct lines in one 2 × 2 cell window | slides into the comb and a shot along its band; the slide window certificate at 123 (admitted) and, Rust-only, 129 (rejected) |
| wide | four 384-unit rooms at the corners of a 7,552-unit square joined by 64-unit corridors: a 60 × 60-cell grid (3,600 cells, the IWAD maximum class) | wide BLOCKMAP preparation, pursuit and sight along a 6,784-unit corridor, long empty DDA walks, admission time |

Scenarios (about 20): `town_patrol`, `town_fight`, `town_sight_far`, `gallery_shot`,
`gallery_aim_pickups`, `gallery_monster_shot`, `extent_shot_long_wall`,
`extent_sight_diagonal`, `extent_slide_long_wall`, `dense_slide`, `dense_shot`,
`wide_pursuit`, `wide_sight`, plus the M3h5 captures. Contract counts include the
maximum intercepts per trace observed in C.

### 7.2 Independent DSIM33

`tools/reference/real_map_snapshot.py`, `m3h6_snapshot_goldens.py`, `m3h6_snapshots.csv`,
`m3h6_events.dref`, selected full-byte fixtures, written before the Rust change.

### 7.3 Rust-only evidence

A test-only `real_map_room.rs` patches the committed lumps: a line stretched to norm
3,854 (i), a vertex moved so that (ii) fails, a span failing (iii), a 129-line window,
a short REJECT, the M3h5 variants (C1–C3, missing listing, spawn in a wall), a THING of an
unadmitted type, starts 1–4 and 11 present. Every rejection is tested for unchanged
state; the `Intercepts` witness is reached on a Rust-only map with 129 fins across one
hitscan and asserted byte-identical before and after the failed tic; the slide witness is
asserted unreachable on `dense`. Soaks: `town` with the full roster, 1,200 tics, four
seeds, Invariant Ω after every tic. Fuzz target `player_real_map` on `town`, `gallery`
and `dense` with the M3h5 properties plus the intercept count; seeds from
`tools/seed_player_real_map_fuzz.py`. A `#[ignore]` test gated on `DOOM_IWAD_DIR`
(pattern of `crates/doom-map/tests/private_iwad.rs`) runs the certificate, the boundary
rules and the slide window on every IWAD map and prints pass/fail counts and the
admission time of E1M1's geometry, never a name or a byte of the map.

### 7.4 Measurements

Matched release timings of `de042dd` and the branch head for every existing `--measure`
workload (digests compared) plus `town_patrol`, `gallery_shot` and `wide_sight` from the
new example, eleven interleaved process pairs per workload, alone on the idle host;
admission time of `town`, `wide` and (locally, time only) E1M1's geometry;
`tools/measure_real_map.py --check` on the `measure_stepping.py` smoke pattern.

## 8. Implementation plan, owner decisions, follow-ups

### 8.1 Plan (one commit per step; steps 1–4 land before the runtime change)

1. **This note.**
2. **IWAD count tool**: `tools/iwad_counts.py` (read-only, path argument, prints counts
   and pass/fail facts of §4.2, §4.5 and §9; never run in CI); the §9 table is its output.
3. **C evidence**: `tools/reference/m3h6_geometry.py` (polygon rooms, automatic node
   builder, BLOCKMAP), `m3h6_cases.py`, `m3h6_reference.py`, `m3h6_shim.h`; generated
   `tests/reference/m3h6_trace.dref`, `m3h6_cases.csv`, `m3h6_geometry_*.bin`,
   `m3h6_maps.rs`, `m3h6_contract.json`; strict and ASan/UBSan builds compared.
4. **DSIM33 projection**: `real_map_snapshot.py`, `m3h6_snapshot_goldens.py`, outputs.
5. **Admission and witnesses** (one commit, since the replays need all of it):
   `player_map_scale.rs` (`validate_real_map_geometry`, `ArithmeticCertificate`,
   `slide_window_lines`, `RoomBounds::derive_real_map`, `MapScaleError::{Arithmetic,
   Intercepts}`), `map_extent.rs` (a per-map limits constructor for this policy: format
   bounds, derived work values, no fixed cells/words/references/bytes/sight/collision
   thresholds), `multisector.rs` (`new_bounded` with the map's sector count),
   `line_work.rs` (format-sized `SeenLines`), `combat_trace.rs` (`seen` → `SeenLines`;
   `InterceptLimit` kept), `movement.rs` (`path` counts to 128), `spawning.rs` (starts as
   records), `noise.rs` (per-sector line lists), `demon_expanded_map.rs` (derived per-tic
   sight budget, THINGS bound), `original_registry.rs` (variant, wire 17, predicates,
   `with_original_registry` branch), `player_monsters.rs` (schema,
   `original_player_real_map`), `lib.rs` exports. 5b (only if sky is approved):
   `multisector.rs` `ceiling_line`, `projectiles.rs` removal, `combat_trace.rs` sky returns.
6. **DSIM33**: `snapshot.rs` (33, every `..=32` range), `diagnostic.rs`
   (`decode_real_map`, `policy`, derived bounds), `STATE_FORMAT.md`.
7. **Tests and tools**: `real_map_room.rs`, `player_real_map_tests.rs`,
   `tests/support/m3h6.rs`, `examples/player_real_map.rs` (asserted replay plus
   `--measure`), `tools/reference/check_player_real_map.py`,
   `fuzz/fuzz_targets/player_real_map.rs`, `tools/seed_player_real_map_fuzz.py`,
   `tools/measure_real_map.py`, the private IWAD test, CI steps on the M3h5 pattern.
8. **Docs**: README, ROADMAP, COMPATIBILITY (limits inventory rows and a new section),
   STATE_FORMAT, ARCHITECTURE, TESTING, docs/README, `tools/reference/README.md`,
   `docs/validation/M3H6.md`.
9. **Gates, review, PR.**

### 8.2 Owner decisions requested before Phase 2

1. **Policy identity**: `PlayerMonstersRealMapV1`, wire 17, DSIM33, context
   `PLAYER-MONSTERS-REALMAP-1-MEDIUM-v1`.
2. **`MAXINTERCEPTS`** (§4.4): (b) recommended — exact runtime refusal with
   `MapScaleError::Intercepts` in both collectors, plus the slide-window certificate as
   an admission rule; (a) alone is not available for hitscans; (c) deferred to M4.
3. **Roster, slots, fireballs** (row 16): keep unchanged (recommended) or derive from
   the loaded THINGs now (changes the slot formula, the sight budget, the C harness
   roster and the linked-thing term; suggested as the next milestone, §8.4).
4. **Boundary rules and BLOCKMAP rule** (§4.5): keep C1–C3 and the closed-cell rule
   unchanged (recommended); the 13 IWAD exceptions are recorded for a later milestone.
5. **Sky** (§6): out (recommended) or in with step 5b and its scenarios.
6. **Player starts** (§4.7): admit types 1–4 and 11 as source records (recommended).
7. **Heights** (row 11): keep ±512 (recommended); E1M1 needs 264.
8. **Arithmetic certificate** (§4.2): admission requirement (recommended) rather than
   relying on the checked arithmetic at runtime.
9. **IWAD tooling**: commit `tools/iwad_counts.py` and the §9 count tables (counts and
   pass/fail only, no names of lumps beyond the map slot, no coordinates of any object)
   (recommended), or keep both out of the repository and report counts in the PR only.
10. **`SlopeDiv` bound** (iv): keep 8,192 unless Phase 2 proves the wrap is reproduced.

**Owner decisions, recorded 2026-11-02 before any runtime change (Phase 2 implements
exactly these):**

1. Approved: `PlayerMonstersRealMapV1`, wire 17, DSIM33, context
   `PLAYER-MONSTERS-REALMAP-1-MEDIUM-v1`.
2. Approved: option (b), checked refusal of the tic at the 129th intercept in both
   collectors, plus the slide-window admission rule. Documented as a known fidelity gap
   for M4 demo compatibility, like the corner jump. The line-indexed `seen[128]` hitscan
   buffer is fixed so that maps above 128 lines can fire, with a regression test that
   fails before the fix.
3. Approved: roster, slots and fireballs unchanged.
4. Approved: boundary rules C1–C3 and the closed-cell BLOCKMAP rule kept; the IWAD maps
   failing them are recorded (counts only, §4.5) for the real-map loading follow-up.
5. Approved: sky stays out; the recorded plan (§6) is kept for the doors/specials
   milestone.
6. Approved: player starts 1–4 and 11 admitted as source records.
7. Approved: heights stay ±512.
8. Approved: the arithmetic certificate is an admission requirement.
9. Approved: `tools/iwad_counts.py` and the counts table are committed, on condition
   that the script is local-only (never invoked by CI, skips cleanly when
   `/mnt/d/DoomIWADs` is absent) and the committed table holds counts and pass/fail
   facts only.
10. Approved: the `SlopeDiv` 8,192 bound is kept unless Phase 2 proves the unsigned wrap
    is reproduced.

### 8.3 Follow-ups and risks

- **Heights beyond ±512** need the M3h4 vertical arithmetic re-derived per map (fireball
  chord, eye and opening slopes, `FixedDiv` saturation in `P_CrossSubsector`); 10 of 68
  IWAD maps exceed 512.
- **The automatic node builder** is new evidence infrastructure; its correctness is
  checked only by the C and Rust bodies agreeing on the same lumps and by the per-leaf
  single-sector assertion. Rounded split vertices make segs that are not exactly on their
  linedef, as in real maps; both sides consume them identically.
- **Maps failing (ii)**: 22 IWAD maps; the alternative (runtime checked arithmetic as the
  witness) is a one-line policy change if the owner later prefers reach over proof.
- **Load time**: `Boundary::derive` O(walls²) and the certificate are linear or
  quadratic in the map, measured in Phase 2 (§7.4); a format-maximum map (32,767 walls)
  would take about a second and is not an admitted target.
- **CI headroom**: the Ubuntu job is at 27 of 45 minutes; the M3h6 generator (five maps,
  twenty scenarios, three builds) is expected to add 3–4 minutes on Linux and about a
  minute elsewhere; the M3h5 `--jobs 4` suite is unchanged.

### 8.4 Recommended next milestone

Roster scale (derive monsters, pickups, drops, slots, fireballs and the sight budget
from the loaded THINGs; E1M1 UV needs 29 monsters and 53 pickups), then the E1M1
feature list of §9 in dependency order: solid and non-solid decorations and barrels, sky
(§6), then specials (lights, secrets, damage floors), then doors and lifts.

## 9. E1M1 readiness report

Counts from the owner's `DOOM.WAD` E1M1, read locally and read-only. "Admitted after
M3h6" assumes the recommendations of §8.2.

| Lump or feature | E1M1 | After M3h6 |
| --- | ---: | --- |
| Vertices / linedefs / sidedefs / sectors | 470 / 486 / 666 / 88 | admitted (format bounds) |
| Segs / subsectors / nodes | 747 / 239 / 238 | admitted |
| BLOCKMAP | 36 × 23 = 828 cells, 3,474 words, at most 13 lines per cell, 0 missing closed-cell listings | admitted |
| REJECT | 968 bytes (exact), 756 of 7,744 bits set, symmetric | admitted, read by sight |
| Vertex rectangle | 4,576 × 2,816 (x −768…3,808, y −4,864…−2,048) | certificate (i)–(vii) holds |
| Longest line norm | 832 | admitted (≤ 3,853) |
| Heights | floors −136…136, ceilings −40…264 | admitted (±512) |
| One-sided boundary | 306 walls, C1 = C2 = C3 = 0 violations | admitted |
| Slide window | 28 distinct lines at most | admitted (≤ 128) |
| Hitscan intercepts | replica maximum 67 of 128 at UV spawn | runtime witness, not expected to fire |
| Line specials | 22 lines, 8 types: 1 (DR door) ×8, 48 (scroll) ×8, 11, 36, 38, 63, 88, 109 ×1 each; 5 tagged lines | **rejected** |
| Sector specials | 11 sectors: 7 (5% damage) ×4, 9 (secret) ×3, 8 (glow) ×2, 1 (blink) ×1, 12 (sync blink) ×1; 5 tagged sectors | **rejected** |
| Sky ceilings | 7 sectors | **rejected** (§6) |
| Line flags | BLOCKING 319, BLOCKMONSTERS 13, TWOSIDED 180, DONTPEGTOP 115, DONTPEGBOTTOM 100, SECRET 1, DONTDRAW 8; 0 inconsistent `ML_TWOSIDED` | admitted |
| THINGs | 143 records; 124 present on UV single player | count admitted |
| Player starts | type 1 ×1, types 2–4 ×3, type 11 ×5 | admitted as records (§4.7) |
| Monsters on UV | shotgun guy 16, zombieman 9, imp 4 = 29 (all admitted types) | **rejected: roster is 8** |
| Pickups on UV | armor bonus 25, health bonus 13, medikit 3, shell box 3, clip 2, shells 2, stimpack 1, clip box 1, blue armor 1, green armor 1, shotgun 1 = 53 (all admitted kinds) | **rejected: pickups are 16** |
| Unadmitted THING types on UV | floor lamp 8, gibs 7, barrel 6, dead player 4, tech pillar 2, candelabra 2, bloody mess 2 + 2 = 33 | **rejected** (`UnsupportedType`) |
| THINGs absent on UV single player | rocket box 3, rocket launcher 1, chaingun 1, berserk 1, barrel 3 … (19 records with `options & 16`) | filtered as the source does |

E1M1 therefore needs, after M3h6: the roster scale, eight decoration and barrel types,
sky, eight line-special types, five sector-special types and the door, lift and floor
movers behind them. Its geometry, BLOCKMAP, REJECT, boundary and arithmetic are within
the new policy.

## 10. Status / next step

Status: **investigation complete**; owner decisions of §8.2 recorded 2026-11-02. Phase 2
in progress: steps 1–3 of §8.1 done; next step is 4 (DSIM33 projection).
