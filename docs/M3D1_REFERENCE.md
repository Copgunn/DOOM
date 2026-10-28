# M3d1 investigation: bounded damage and hitscan combat

Recorded before combat runtime changes. Authority: untouched id Software Linux
DOOM 1.10, baseline `a77dfb96cb91780ca334d0d4cfd86957558007e0`. M3c was regularly
merged at `4dbb0a51325119e5578705126f97bdc5d9723049`; correction-head CI
40613240261 and post-merge CI 40613453371 passed before this branch.

## Original source findings

Inspected p_inter.c/h, p_map.c, p_maputl.c, p_mobj.c/h, p_local.h,
p_enemy.c, p_pspr.c, p_user.c, p_sight.c, d_player.h, info.c/h, m_random.c,
fixed/trigonometry/angle routines and the earlier source investigations.

P_DamageMobj:

1. Return without RNG/effects when not SHOOTABLE or health <= 0. SKULLFLY resets
   momenta; actual players on baby skill halve damage. These are distinct branches.
2. With an inflictor, unless NOCLIP or a player source using a chainsaw, derive
   R_PointToAngle2(inflictor,target) and integer thrust = damage*(FRACUNIT>>3)*100/mass.
   For damage < 40, damage > old health, and target Z > inflictor Z+64, draw once;
   an odd result reverses angle by ANG180 and multiplies thrust by four. Apply
   FixedMul with original fine cosine/sine before health reduction.
3. Actual player-pointer branches implement end-game sector protection, god mode,
   invulnerability, armor, mirrored health, attacker, damage flash and tactile
   output. Unbound P_SpawnMobj(MT_PLAYER) has player == NULL and does not use them.
4. Subtract health. Lethal damage calls P_KillMobj and returns before pain RNG,
   reaction reset or retargeting. Nonlethal damage always consumes a pain draw,
   even damage zero or painchance zero; on draw < painchance and not SKULLFLY,
   set JUSTHIT and enter painstate (including null) through P_SetMobjState.
5. Set reactiontime=0 after the pain callback. When threshold==0 or target is VILE,
   and source exists, differs from target and is not VILE, assign target=source,
   threshold=100, then enter seestate only if CURRENT state equals spawnstate.
   Pain callbacks may change state/remove actors; deferred storage remains readable.
   Threshold countdown belongs to A_Chase, not the plain P_MobjThinker.

P_KillMobj clears SHOOTABLE/FLOAT/SKULLFLY; clears NOGRAVITY except for SKULL;
sets CORPSE|DROPOFF and quarters height. Player/scoring branches and four drop
identities require additional systems. Choose xdeath only when health is strictly
less than -spawnhealth and xdeathstate is non-null; equality uses ordinary death.
Enter that state first (actions can consume RNG), then subtract P_Random()&3 from
the CURRENT tics and clamp to >=1, even after a null state. Persistent terminal
objects, null-state removal and subsequent thinker retirement are separate.
M3c request_actor_death remains its older state-entry-only operation.

P_PathTraverse nudges a start coordinate by ONE MAP UNIT when exactly on a block
boundary, builds a fixed divline, then performs the original bounded 64-cell DDA.
In each cell it enumerates source BLOCKMAP lines before center-cell thing heads.
A line validcount suppresses duplicate lines across cells; things have no such
suppression. Line intercepts use the source precision switch at +/-16 units;
thing intercepts use a selected corner-to-corner box diagonal based on the signed
(trace.dx ^ trace.dy)>0 predicate. This is not an axis-aligned box ray slab test.
Negative fractions are discarded; collection can include fractions beyond one.
P_TraverseIntercepts repeatedly selects the strict minimum, preserving collection
order on ties; callbacks stop at the first applicable intercept, with frac==1
included. The original intercept array holds 128 records without overflow checks.

P_AimLineAttack uses fine-angle lookup and endpoints calculated from the INTEGER
map-unit part of distance, while retaining full fixed distance for slopes. Shoot Z
is actor.z + actor.height/2 + 8 units; initial slope window is +/-100/160. One-sided
or closed openings stop aim. Height differences narrow the window using original
FixedDiv/FixedMul. Self and nonshootable actors are skipped; first overlapping
vertical span wins, and the result is the toward-zero average of clipped slopes.
Aim is not P_CheckSight: sight additionally uses REJECT and BSP subsector crossing
with different eye height/side rules. General sight/monster perception remains later.

P_LineAttack uses the requested slope. Shootable actors are accepted on inclusive
vertical slope boundaries. A line's ML_BLOCKING/ML_BLOCKMONSTERS flags are not a
substitute for the source one/two-sided/opening rules. Specials execute before
blocking; sky suppression is another branch. Hit positions are pulled back four
units for lines or ten for actors, without clamping the resulting fraction.

P_SpawnPuff/P_SpawnBlood run BEFORE target damage. Both draw twice for Z jitter,
spawn a real mobj (another lastlook draw), set vertical momentum, then draw for
initial tics and clamp. Blood uses +2 vertical momentum and state BLOOD2 for damage
9..12, BLOOD3 below 9; puff uses +1 and PUFF3 at exactly MELEERANGE. These real
thinkers, RNG draws, links and future updates must not become visual-only events.
Despite its comment, P_LineAttack damage zero still creates impacts and consumes
RNG; it merely omits P_DamageMobj. P_AimLineAttack is the read-only aiming query.
Weapon damage/spread RNG in P_GunShot, P_BulletSlope, psprites/ammo and attack buttons
remain M3d2; A_PosAttack and other perception/AI attack actions remain M3d3.

Corpses use the existing XY/Z thinker. P_XYMovement has a CORPSE friction exception:
when either XY momentum exceeds +/-FRACUNIT/4 and contacted floor differs from the
center sector floor, skip friction. Death may clear gravity and dropoff flags or
later A_Fall may clear solidity; synchronize their physical projections immediately.

## Declared implementation slice and safe policies

New explicit profile 5 / DSIM-v5 composes M3c states and static-sector physics.
Older profiles and v1-v4 bytes remain unchanged. Explicit synchronous aim, line
attack and damage APIs precede future player-button/weapon/AI integration. They
use validated handles and the existing state host, not a parallel lifecycle.
No attack queue, weapon spread, damage formula for a weapon, or extra persistent
attacker field is invented. Target threshold is persistent source state.

Damage is for unbound, living, admitted shootable actors with supported state
closures. Source MT_PLAYER unbound pawns and explicitly labelled native/source
fixture definitions isolate the non-player-pointer branch; this does not implement
player damage/inventory. Reject affected bound players/sources, unsupported flags,
COUNTKILL/scoring, item-drop identities and unavailable pain/see/death/xdeath
closures before mutation. Existing recognized monster metadata remains distinct
from executable combat. NOBLOOD may be admitted only in profile 5, with actual
puff spawning; JUSTHIT/CORPSE are source runtime outcomes, not silent AI support.

Use nonnegative bounded damage, positive mass/spawn health and checked health
subtraction. Reserve conservative callback/effect spawn capacity before any event,
health, momentum, flag or RNG mutation. Preview branch RNG on a copy of DoomRng;
validate all possible selected state closures and conservative service demand.
Do not clone the full Simulation/Map. Pending or stale combat handles reject
before mutation. Existing retained target identities still resolve generation-
safely; removal does not eagerly clear references. State callbacks can reenter,
spawn or mark actors using the accepted host; recursive combat callbacks are not
introduced in this slice.

Trace collection is read-only and bounded before attack effects. Preserve 64-cell
DDA, source line/cell/thing order and stable intercept ties, with an explicit
128-intercept limit. Reject pathological incomplete traversals, invalid ranges,
unsupported sky/special branches and unsafe coordinate setup before mutation.
Use original fixed arithmetic and existing trigonometry/point-angle data; no
floating raycasts, reordered spatial index, graphics or physics dependency.
Effects use the same real typed store and source RNG order before damage.

## C evidence and exclusions

Evidence is pending implementation of the separate original-body harness. Its
adapter must not replace damage, hitscan, effect, state, corpse movement or RNG
behavior under test. Planned comparisons cover complete attack/death tics plus
geometry/range/tie/height cases, branch inputs, exact intermediates, RNG/indexes,
links/thinker order and failure boundaries. Native fixture definitions must be
recorded separately from original actor metadata. Unsupported services may abort
only on demonstrably unreachable paths.

The original jitter expressions left-shift negative signed differences and leave
operand call evaluation order unspecified. The adapter will use explicit ordered
RNG calls and representable multiplication for this defined profile, recording
that substitution. GNU pointer-union/sentinel, arithmetic-shift/narrowing assumptions,
retained retirement storage and source/header/adapter/input/output hashes will be
recorded. Original 165 files remain unchanged; no Rust results define C expected
outputs. C-undefined extremes test safe Rust rejection, not C compatibility.

Full armor/immunity/player death, scoring/drops, AI, missiles, radius attacks,
boss specials, resurrection/nightmare respawn, moving sectors, demos and scripts
remain unsupported. Rendering/audio/IWAD data stay outside authoritative tics.
