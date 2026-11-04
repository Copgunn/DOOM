# Roadmap

This is the authoritative development summary and plan. The [documentation index](README.md#documentation-authority)
identifies the other information owners. Exact review state, commits, CI and merge
history belong to GitHub; supported behavior and limits belong to [COMPATIBILITY](COMPATIBILITY.md).

## Done

The merged baseline provides:

- WAD and classic-map parsing with checked references and resource budgets.
- Deterministic simulation, fixed-point arithmetic and original random-number streams.
- Ordered actors, state changes, static-sector movement and collision.
- Selected weapon/monster combat, sight, hearing and pursuit.
- Demon stand/run, melee, pain/death and persistent corpses.
- Bounded classic THING spawning (M3d3i, merged), independently checked against
  original C and versioned simulation snapshots.
- Demon map context and statistics (M3d3j): original kill-count flags, separate
  map and credited-kill totals, and fresh level reset with RNG continuity.
  See the [source investigation](M3D3J_REFERENCE.md) and
  [observed tests](validation/M3D3J.md) for technical evidence.
- M3f1 bounded line/sector scale, regularly merged: reusable source-ordered static
  collision and read-only sight preparation. See [source findings](M3F1_REFERENCE.md)
  and [observations](validation/M3F1.md).

- M3f2 expanded coordinates and spatial work, regularly merged: checked arithmetic,
  bounded collision preparation and fallible sight. See [source findings](M3F2_REFERENCE.md)
  and [observations](validation/M3F2.md).

- M3f3 expanded-map Demon composition is merged. Its opt-in profile reuses
  larger static geometry with bounded map-spawned Demon perception, pursuit,
  melee and lifecycle. See [support](COMPATIBILITY.md#expanded-map-demon-profile),
  [source findings](M3F3_REFERENCE.md) and [observations](validation/M3F3.md).

- M3f4 original actor/state identity admission is merged. Complete original
  identity metadata and checked registry resolution preserve existing execution.
  See [mapping](ACTOR_IDENTITIES.md), [source findings](M3F4_REFERENCE.md) and
  [observations](validation/M3F4.md).

- M3f5 canonical MT_SERGEANT execution binding is merged. Typed source requests
  use the existing native closure, with independent canonical C observations and
  explicit DSIM projection. See [investigation](M3F5_REFERENCE.md) and
  [observations](validation/M3F5.md).

- M3f6 original-ID runtime storage audit is merged. Its native representation
  guards preserve all historical aliases and diagnostics; see
  [audit](M3F6_REFERENCE.md) and [observations](validation/M3F6.md).

- M3f7 original-ID registry/profile and diagnostic contract is merged. Actual
  source IDs, explicit context definitions and root-derived state permissions use
  shared execution services; bounded v20/v21 readers preserve historical meaning.
  See [investigation](M3F7_REFERENCE.md) and [observations](validation/M3F7.md).

- M3f8 context and target-lifetime ownership is merged. Scoped access, derived
  reference availability and compact independent lifetime inputs preserve existing
  gameplay. See [design](M3F8_REFERENCE.md) and [observations](validation/M3F8.md).

M3f9 game-instance provenance is merged. Opaque incarnation keys qualify boundary
references while the same actor store and execution context remain authoritative;
see [design](M3F9_REFERENCE.md) and [observations](validation/M3F9.md).

These foundations support controlled headless tests, not a complete playable game.
Earlier findings and milestones are retained in the [historical records](validation/archive/README.md).

### M3g5: Active original-player roster and canonical Demon acquisition

Opt-in PlayerDemonAcquisitionV1 composes one active genuine player and at most one
map-THING Demon. Existing commands precede shared thinkers; source visual search,
lastlook/stop-slot behavior, wake sound/state entry and bounded autonomous pursuit
use the same simulation. Death retains activity and corpse ownership. The actual
melee branch rejects transactionally; full PlayerThink, roster deactivation and
autonomous combat are not claimed by that capability. M3g5 is merged. See
[source investigation](M3G5_REFERENCE.md), [support](COMPATIBILITY.md#active-original-player-and-demon-acquisition)
and [observations](validation/M3G5.md). Exact review/CI state belongs to GitHub.

M3g4 command/reaction movement, M3g3 physical knockback/corpses, M3g2 genuine-player
lifecycle and M3g1 diagnostic-free stepping are merged. Their investigations and
observations remain linked from the [evidence index](README.md).

## Current milestone

### M3h9a: Two-sided scrolling walls and static decorations

The separately selected PlayerMonstersE1M1V1 (wire 20, DSIM36) is the movers world plus
two parts of E1M1 that need no new state: the scrolling wall 48 on two-sided lines,
blocking or not, with the source's three game-code effects (a `spechit` entry, the
monster's direction reset with a failed use, the player's use stopped with no effect),
and the seven static decoration types E1M1 spawns (floor lamps, tech pillars and
candelabras that block with no height test; pools of blood, dead players and bloody
messes that everything passes through), at most 32 per map. Decorations spawn with
their single `P_Random` draw in lump order, ride movers, stop fireballs when solid,
let hitscans through and are never damaged, gibbed, removed or targeted. DSIM36 is the
DSIM35 layout with no new field or event. Barrels, more than 8 monsters or 16 pickups and
every other THING type stay rejected at load, so **E1M1 itself still does not load**. No
new fidelity gap is added. Older policies keep their admission, digests and DSIM bytes.

Status on `feat/m3h9-e1m1`: in review, not merged. The C evidence (20 scenarios), the
independent DSIM36 projection, the policy, the full replay, the Rust-only tests, the
example, the tools, the fuzz target with its seed replay and this documentation are
complete. The bounded fuzz campaigns, the matched timings, the independent review and
the final gates are pending, and the decisions E1–E8 are provisionally adopted, awaiting
owner confirmation. This branch stops at owner review; exact commits and CI belong to
GitHub. See [source investigation](M3H9_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-with-two-sided-scrolling-walls-and-static-decorations)
and [observations](validation/M3H9.md).

### Previous gameplay foundation — M3h8: Doors, lifts, floors, switches and the exit (Stage B)

The separately selected PlayerMonstersMoversV1 (wire 19, DSIM35) is the specials world
plus E1M1's seven tag, use and crossing line types: the DR door 1, the SR door switch 63,
the W1 blazing door 109, the WR lift 88, the W1 floors 36 and 38 and the S1 exit switch
11. It adds `BT_USE` with the source's use latch, moving floors and ceilings staged with
the checked tic, `P_ChangeSector` (re-clipping, giblets, dropped-item removal, pickups
inside a clip), crossing through the source's `spechit`, monster door use, switches with
their button slots and the exit. The exit tic completes and the next `advance` is refused
with `SimError::LevelExited`; `BT_USE` held while dead completes its tic and the next is
refused with `SimError::PlayerReborn`. Load rules reject what the source leaves undefined
or what is deferred: one-sided DR lines, shared mover families, doors without neighbors,
inverting and self-referencing doors, reachable heights beyond ±512, more than 30 lifts,
16 SR switches or 8 special lines in a BLOCKMAP window, sky ceilings over movers,
switches without a switch texture, crushers and every other line type; a two-sided
scrolling wall stays rejected under this policy (decision N1, option iii). V′ and a relaxed
item witness replace the vertical and item witnesses under this policy only, and the
ninth `spechit` write is a refuse-the-tic witness (an M4 fidelity gap like the corner
jump and the intercepts). Older policies keep their admission, digests and DSIM bytes.

M3h8 is merged. See [source investigation](M3H8_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-with-doors-lifts-floors-switches-and-the-exit)
and [observations](validation/M3H8.md).

### Previous gameplay foundation — M3h7: Sky and static specials

The separately selected PlayerMonstersSpecialsV1 is the real-map world plus the parts of
E1M1's specials that move no plane: sky ceilings and the static sector specials. It
admits the light sector specials 1, 2, 3, 8, 12, 13 and 17 (blinking, strobing, glowing
and flickering lights, with their random draws), the damage floors 4, 5, 7 and 16 on the
source's 32-tic gate, the secret 9, the scrolling wall 48 on at most 64 one-sided lines,
and `F_SKY1` ceilings with their two game-code effects: a fireball refused at a sky
ceiling line is removed without exploding, and a hitscan reaching a sky ceiling ends
without a puff. Light thinkers share one processing order with the actors, in their own
slot class bounded at load and spawned by an explicit `spawn_level_specials` after the
THINGs and the player body; environmental damage and death go through the shared damage
path with no source. Every other line special, a two-sided 48, sector specials 10, 11,
14 and unknown values, `BT_USE` and every moving plane stay rejected; sector lights
outside the range the light arithmetic needs are refused at load. DSIM34 adds the
shared thinker order, the light thinkers, the runtime sector light and special and the
player's secret count. E1M1 still needs the roster scale, eight decoration and barrel
types and the doors, lifts, floors and exit switch of Stage B. M3h7 is merged. See
[source investigation](M3H7_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-with-sky-and-static-specials) and
[observations](validation/M3H7.md).

### Previous gameplay foundation — M3h6: Real-map scale — classic map limits toward E1M1

The separately selected PlayerMonstersRealMapV1 raises the static-geometry limits of the
diagonal world from test-room size to the scale of classic maps. Every cap that stood in
for an allocation or CPU budget (95 linedefs, 256 sectors, 4,096 nodes and subsectors,
8,192 BLOCKMAP cells, 256 THINGs, ±2,048 coordinates, 1,536-unit lines, fixed sight and
collision work) becomes the WAD format's own `short` bound or a value derived from the
loaded map; the arithmetic envelope becomes a certificate computed from the map against
the same source expressions; `MAXINTERCEPTS` becomes a runtime witness that refuses the
tic which would write the source's 129th intercept (`MapScaleError::Intercepts`, the
second refused legal source behavior after the corner jump, a known fidelity gap for
demo compatibility), with a slide-window admission rule that proves slides never reach
it. Player starts are kept as records, the hitscan collector's 128-line buffer is fixed
so maps above 128 lines can fire, and the noise flood reads per-sector line lists.
Roster, slots, fireballs, heights, specials, sky and the boundary rules are unchanged;
E1M1's geometry, BLOCKMAP, REJECT, boundary and arithmetic are inside the new policy,
its THING roster, decorations, sky and specials are not. DSIM33 adds no persistent
field. M3h6 is merged. See
[source investigation](M3H6_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-at-classic-map-scale) and
[observations](validation/M3H6.md).

### Previous gameplay foundation — M3h5: Non-axis (diagonal) walls

The separately selected PlayerMonstersDiagonalV1 takes the heights world off the
axis-aligned grid: one- and two-sided linedefs of any direction inside a closed
boundary of one-sided walls, so wall slides at any angle, convex and concave diagonal
corners, diagonal risers and windows, an octagonal pillar and a 45° corridor execute
through the angle-general bodies the engine already carried (`P_HitSlideLine`'s
projection, `P_BoxOnLineSide`'s corner tests, the intercepts, sight, hitscan, autoaim,
fireballs and `P_NewChaseDir`). The closed boundary is defined by three exact integer
rules (endpoint-only contacts, even vertex degree, front sides facing odd parity) and
the wall invariant is proved for every move of at most `2r − 2` units per axis; the one
move outside that bound, a player's projected slide of more than 30 units per axis, can
jump a corner in the source and is the one legal source move the policy refuses
(`MapScaleError::Escape`, a known fidelity gap to revisit for demo compatibility).
DSIM32 adds no persistent field. Moving sectors, specials, sky ceilings and cap
increases stay rejected at load. M3h5 is merged. See
[source investigation](M3H5_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-among-diagonal-walls) and
[observations](validation/M3H5.md).

### Previous gameplay foundation — M3h4: Two-sided lines, static floor/ceiling heights and vertical movement

The separately selected PlayerMonstersHeightsV1 takes the canonical player and monster
world off flat floors inside the M3h3 walled rectangle: axis-aligned sectors with
different static heights joined by two-sided lines, so steps, ledges, drop-offs,
windows, closed sectors and fences execute through the original bodies the engine
already carried (`P_LineOpening`, `P_TryMove`'s fit, step and dropoff rules,
`P_ZMovement`, sight and hitscan slopes, fireball chords, `P_RecursiveSound`). The
vertical invariant extends the wall invariant to openings and is checked before every
tic; spawns must fit their sector. The landing sound, cross-sector weapon noise and the
fall-forward damage branch become reachable. DSIM31 adds no persistent field. Moving
sectors, specials, sky ceilings and non-axis walls stay rejected at load. This branch
stops at owner review; exact commits and CI belong to GitHub. See
[source investigation](M3H4_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-with-static-heights) and
[observations](validation/M3H4.md).

### Previous gameplay foundation — M3h3: Wall-aware travel bounds, no-stall guarantees and real-map scale

The separately selected PlayerMonstersMapScaleV1 replaces the conservative `160M/3`
travel envelope and the 994-unit actor margins with bounds derived from the loaded
map: a walled rectangle of up to 2,048 units whose one-sided axis walls keep every
mover inside, exactly as the original's `P_TryMove` does. Both M3h2 stalls (a hit
near a wall, and a fireball past 994, which was confirmed rather than refuted) are
reproduced on the historical policies and progress on the new one. Slot and
fireball budgets (320 slots and 208 fireballs) are derived from flight time and attack cadence, so
ordinary play cannot reach a cap, and immutable definitions are validated once
instead of every tic. DSIM30 adds no persistent field. This branch stops at owner
review; exact commits and CI belong to GitHub. See
[source investigation](M3H3_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-monsters-at-real-map-scale) and
[observations](validation/M3H3.md).

### Previous gameplay foundation — M3h2: Original Imp, fireballs and roster maintenance

The separately selected PlayerMonstersImpV1 brings projectiles into the canonical
player world: the original MT_TROOP chooses melee or missile, and MT_TROOPSHOT flies,
collides and explodes through one shared missile service, so fights gain dodgeable
shots and projectile infighting with the source same-species rule. Imps join the
eight-monster roster, at most 24 fireballs are live, and exceeding a bound rolls back
the whole tic. Canonical action dispatch is now typed. DSIM29 adds no persistent
field. This branch stops at owner review; exact commits and CI belong to GitHub. See
[source investigation](M3H2_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-imps-and-fireballs) and
[observations](validation/M3H2.md).

### Previous gameplay foundation — M3h1: Original former humans and multi-monster encounters

PlayerMonstersV1 adds original MT_POSSESSED, MT_SHOTGUY and MT_CHAINGUY beside the
Demon, up to eight monsters, source infighting and P_KillMobj drops with DSIM28.
M3h1 is merged; exact review/CI state belongs to GitHub. See
[source investigation](M3H1_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-former-humans-and-multi-monster-encounters) and
[observations](validation/M3H1.md).

### Previous gameplay foundation — M3g11: Original health/armor bonuses and counted items

PlayerDemonBonusesV1 adds genuine animated health and armor bonuses (+1 up to 200)
and COUNTITEM statistics to the armor world with DSIM27. M3g11 is merged; exact
review/CI state belongs to GitHub. See
[source investigation](M3G11_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-health-and-armor-bonuses) and
[observations](validation/M3G11.md).

### Previous gameplay foundation — M3g10: Original armor, damage absorption and sustained combat

PlayerDemonArmorV1 composes genuine animated green/blue world armor with the
existing player, weapons, contacts and autonomous Demon, using shared damage
planning for absorption, depletion and rollback with DSIM26. M3g10 is merged;
exact review/CI state belongs to GitHub. See
[source investigation](M3G10_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-armor-and-damage-absorption) and
[observations](validation/M3G10.md).

The zstd/formula maintenance milestone is merged. Its independent migration and
formula proofs, unchanged gameplay and preserved DSIM contracts remain in
[storage](EVIDENCE_STORAGE.md), [formulas](WEAPON_FORMULAS.md) and
[observations](validation/ZSTD_WEAPON_MAINTENANCE.md).

### Previous gameplay foundation — M3g9: Original world pickups and combat sustain

PlayerDemonPickupsV1 adds bounded original world items to the existing genuine
player/four-weapon/Demon world. Ammo, healing and source-gated shotgun/chaingun
acquisition share actor storage, collision, inventory, retirement and transactions.
Source-first observations and DSIM25 make pickup ordering and player resource
changes explicit. M3g9 is merged; exact review/CI state belongs to GitHub. See [source investigation](M3G9_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-world-pickups) and
[observations](validation/M3G9.md).

### Previous gameplay foundation — M3g8: Original weapon expansion

M3g8 is merged. Its separately selected no-pickup four-weapon capability and
DSIM24 retain their original contract. See
[investigation](M3G8_REFERENCE_INVESTIGATION.md),
[support](COMPATIBILITY.md#original-player-weapon-expansion) and
[observations](validation/M3G8.md).

### Previous gameplay foundation — M3g7: Genuine canonical player pistol combat

M3g7 is merged. PlayerDemonPistolV1 retains its separately bounded no-switching,
no-fallback contract and DSIM23. See [source investigation](M3G7_REFERENCE.md),
[support](COMPATIBILITY.md#genuine-player-pistol-combat) and
[observations](validation/M3G7.md).

### Previous gameplay foundation — M3g6: Autonomous canonical Demon-to-player melee

Opt-in PlayerDemonMeleeV1 composes the same genuine player, fixed active roster,
commands, sight, pursuit and thinker transaction with original delayed SARG melee.
The attack rechecks current range, then applies damage/physical impulse through
the shared DamagePlan, including pain, death, target retention and persistent
corpses. Misses and dead-body ignored damage retain distinct RNG behavior. Older
policies keep their admission and rejection contracts. M3g6 is merged; exact
commits and CI belong to GitHub. See
[source investigation](M3G6_REFERENCE.md),
[support](COMPATIBILITY.md#autonomous-original-demon-player-melee) and
[observations](validation/M3G6.md).

### Next narrow gameplay milestone

The M3h9 investigation's [§8](M3H9_REFERENCE_INVESTIGATION.md#8-size-estimate-and-split)
splits the rest of the E1M1 run into two parts after M3h9a:

- **M3h9b: barrels and pickup scale.** Exploding barrels (2035) with the radius attack,
  its wrapping distance quirk, chain explosions, thrust, the pain draw and its
  dependence on link order, with their own events; pickups raised to 64 (E1M1 spawns 52
  on skill 3), which needs a policy-specific actor-slot count (384 recommended) and a
  hitscan intercept argument that counts the things in the cells along a trace, and a
  re-evaluated linked-thing bound and `P_ChangeSector` measurement.
- **M3h9c: the real-E1M1 harness.** A local-only harness that builds a headless
  linuxdoom-1.10 from genuine sources, loads the owner's E1M1 and compares it tic for
  tic with the Rust run, committing only pass/fail, tic counts and digests, never IWAD
  content; plus the spawn-rule count over every E1M1 THING (whether each closed box
  stays clear of the walls), which decides whether E1M1 loads at all.

Also revisit the rules the 14 other IWAD maps break (13 with one-sided walls of odd
degree, crossing or facing away from their sector, one BLOCKMAP with missing listings)
and the 22 maps that fail the sight certificate. Require source-first evidence and
unchanged earlier contracts. This is future work, not admission in M3h9a.

### Focused maintenance acceptance criteria

- Inventory production panic paths reachable from public inputs; replace recoverable
  failures with checked errors and preserve complete-state rejection evidence.
  Name remaining gameplay flags/constants with source references, without broad
  unrelated refactoring or changing historical profile semantics.
- Measure reference bytes against independently useful input/event coverage before
  adding large traces. Prefer compact uniquely useful evidence while retaining
  immutable historical fixtures/tools/archives. Any storage migration needs its own
  preservation proof and reviewed scope.

- Inventory repeated preflight walks and transaction staging against the measured
  component/allocation baseline. Any removal must preserve late-error rollback,
  bounded resources and complete source/event/RNG observations, with matched
  three-process timings and actual allocation counters. Do not add an unchecked
  release mode or fragile timing gate. Prioritize a separately reviewed bounded
  scratch reuse or one proven redundant validation over broad redesign.

### Map Scale, Limits, and Real-Map Readiness

**Broader readiness remains planned.** M3f1 supplies the merged line/sector
foundation; M3f2 supplies bounded coordinates/work and M3f3 composes a controlled
Demon gameplay subset; M3h6 replaces the static-geometry caps of the player and monster
world with the format's bounds, a per-map arithmetic certificate and derived work
values, so E1M1-sized geometry is admitted. Further stages
require separate owner direction. The longer-term
purpose is practical support for increasingly complex classic maps,
not unlimited geometry or arbitrarily larger constants.

Start from the [code-backed limits inventory](COMPATIBILITY.md#map-size-and-processing-limits).
It distinguishes original encoding/source constraints, parser allocation budgets,
temporary simulation restrictions, and CPU/traversal work bounds. Parsing a map
successfully does not mean a simulation configuration can execute it.

Acceptance for that future milestone:

- Inventory line, sidedef, sector, vertex, seg, node, subsector, BLOCKMAP and THING
  limits, including interactions between individual table and aggregate budgets.
- Identify unnecessarily restrictive bounds for valid classic maps; document
  encoding/source constraints that must remain and where wider internal indexes,
  dynamic work buffers or other representation changes are required.
- Keep classic compatibility distinct from extended map formats. Specify the
  intended supported slice and compatibility implications before changing limits.
- Preserve checked indexing/arithmetic/allocation, bounded memory and recoverable
  error atomicity; choose justified capacities and work budgets.
- Benchmark larger synthetic or legally redistributable public maps. Verify BSP,
  collision, sight, thinker/spatial ordering and deterministic runtime capacities.
- Add boundary, rejection and regression tests; preserve original-C evidence and
  historical canonical fixtures. Publish measurements and remaining restrictions.

This plan does not authorize commercial-asset access, new map formats or broader
monster/player behavior. Further stages require separate owner direction.

## Later

- Complete player/gameplay integration and broader monster/THING support.
- Moving sectors and map specials (the established M3e direction): doors, lifts,
  triggers and dynamic-sector interactions.
- **M4 — Demo compatibility:** version-specific playback and divergence diagnostics;
  keep DOS and Linux behavior claims separate.
- **M5 — Rendering and platform integration:** presentation/input/audio and a
  custom Vulkan-first rendering direction across supported platforms. Presentation
  must not change authoritative simulation or RNG; no renderer exists yet.
- Multiplayer and later experiments, after their source/game-context dependencies
  can be independently validated.

The established historical labels are retained; this order expresses dependencies,
not old document append order. Current exclusions remain in [COMPATIBILITY](COMPATIBILITY.md#deferred-systems).
