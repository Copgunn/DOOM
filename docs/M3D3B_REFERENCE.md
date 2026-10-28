# M3d3b source investigation: pursuit and movement decisions

Written before runtime changes. Authority is untouched Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. M3d3a reviewed `3ca419e`,
corrected `856d25f`, regularly merged PR #10 as
`839fbabf6eae984e1832f7ab7307851c4adb7a3f`. Exact merge/CI evidence is in
[VALIDATION.md](VALIDATION.md). The runtime work starts only after its successful
post-merge gate.

## Dependencies and execution boundary

Investigated p_enemy.c, p_map.c/p_maputl.c, p_mobj.c/h, p_tick.c, p_sight.c,
p_local.h, info.c/h, doomstat.h, m_random.c and r_main.c, with the prior M3b,
M3c and M3d3a findings. P_Move uses source info.speed, xspeed/yspeed, P_TryMove,
actor flags, source query constraints and sector/BLOCKMAP relinking. P_TryWalk
adds gameplay RNG and movecount. P_NewChaseDir adds target, prior direction,
opposite/diagonal tables, fixed deltas, source abs and finite search priorities.
A_Chase adds reactiontime/threshold, angle, flags, target shootability/health,
player reacquisition, source state entry, skill/fast/netgame and active sounds.
The existing thinker loop already orders XY, Z, state countdown and immediate
actions; no new scheduler or collision implementation is needed.

P_CheckMeleeRange uses approximate distance against MELEERANGE-20+target radius
then sight. P_CheckMissileRange first checks sight, then JUSTHIT clear/early true,
reaction delay, approximate distance minus64 (minus128 without melee), source
type-specific VILE/UNDEAD/CYBORG/SPIDER/SKULL adjustments/caps and a gameplay RNG
comparison. M3d3b will not execute either branch: admitted pursuit metadata must
have both melee and missile states absent. All original monster attack handoffs
remain rejected before mutation. Recognition of original types is not execution.

## Direction state and direct walking

Source order is E=0,NE=1,N=2,NW=3,W=4,SW=5,S=6,SE=7,NODIR=8.
Opposites are [4,5,6,7,0,1,2,3,8]; diagonal lookup is [3,1,5,7].
X components are [65536,47000,0,-47000,-65536,-47000,0,47000];
Y components are [0,47000,65536,47000,0,-47000,-65536,-47000].
These are source approximations, not normalized vectors. Source info.speed for
walking is an integer multiplied directly by these components, not FixedMul of
an already-fixed speed. P_SpawnMobj zeroes the entire mobj: movedir starts EAST
and movecount starts0, with no additional RNG draw. It does not start NODIR.

P_Move returns false immediately for NODIR. Values outside0..8 are fatal in C;
Rust must reject invalid setup before mutation. Otherwise try x/y plus speed
components with P_TryMove. Collision visits things before lines, BLOCKMAP cells
X-outer/Y-inner and source head/list order; duplicate linedefs are stamped before
callbacks. A thing blocks by strict radius overlap and solidity irrespective of
Z for this non-missile slice. Player-shaped unbound actors have null player_t,
so ML_BLOCKMONSTERS applies. Preserve supplied lists, including omissions.

P_TryMove checks collision, total height fit, current-Z headroom, step-up>24 and
floor-dropoff>24 (unless DROPOFF/FLOAT). Success unlinks/relinks even within the
same cell, sets x/y/floor/ceiling, derives BSP sector membership, then handles
special crossings. P_Move then clears INFLOAT and, without FLOAT, snaps Z to the
contacted floor. A blocked movement must retain position/Z/constraints/links.
This is immediate positional motion, separate from ordinary momentum/friction.

On failed TryMove, FLOAT+floatok adjusts Z by4, sets INFLOAT and succeeds; otherwise
blocked special lines are used in reverse numspechit order, set movedir=NODIR,
and return whether any P_UseSpecialLine succeeded. Those float/door/special paths
are excluded, not replaced by a fabricated success. Admit static maps without
line/sector gameplay specials and non-floating actors only. INFLOAT can still be
an admitted setup bit whose source clear-on-success behavior is tested.

P_TryWalk calls P_Move first. On failure it consumes no successful-walk RNG and
leaves movecount. On success set movecount=P_Random()&15, then return true.

## New direction search

Require a target; C errors for null. Save olddir and opposite[olddir]. Deltas are
target minus actor XY. X chooses E only for >10, W only for <-10; otherwise NODIR.
Y chooses S only for <-10, N only for >10. Exact +/-10 is neutral. If both axes
are nonneutral, assign the source diagonal and TryWalk unless it is turnaround.
Successful diagonal returns before the axis-swap RNG.

Next ALWAYS draw P_Random, then swap axis priorities if >200 OR abs(dy)>abs(dx).
The second condition does not suppress the draw. Remove turnaround from the two
axis candidates, try first then second, each via TryWalk. Retry olddir if not
NODIR (even when it duplicates an earlier failed attempt). Then draw search-order
RNG: odd scans0..7, even scans7..0, excluding turnaround. Finally try turnaround
if not NODIR. If every attempt fails, assign NODIR without changing movecount.
Only successful moves change position; at most one succeeds before return.
The finite algorithm needs at most13 TryWalk calls, each a bounded existing query.
Never sort candidates by distance or replace this with a pathfinding heuristic.

## A_Chase order

1. If reactiontime!=0, decrement it. This does not postpone walking.
2. If threshold!=0, clear it for null target or target health<=0; else decrement.
3. If movedir<8, quantize angle with 7<<29, calculate signed wrapping difference
   from movedir<<29, and rotate by one ANG45 toward it. Preserve the signed
   half-turn choice and wrapping unsigned BAM arithmetic; this is not FaceTarget.
4. If no target or target not SHOOTABLE, scan players allaround=true. Success
   returns immediately without movement/active-sound draw. Failure immediately
   enters spawnstate, whose A_Look may itself run; preserve nested action timing.
5. JUSTATTACKED clears. At non-nightmare with fastparm=false call NewChaseDir;
   nightmare skips that call. Return before the normal movement/active-sound path.
6. Melee/missile branches are genuinely unreachable because both states are0.
   No attack decision or callback is silently skipped.
7. netgame target switching is excluded (single-player only).
8. Pre-decrement movecount; if now<0, skip P_Move and call NewChaseDir. Otherwise
   call P_Move, then NewChaseDir only if blocked. No extra successful-walk draw
   occurs for ordinary P_Move success; only NewChaseDir's successful TryWalk draws.
9. Only if metadata activesound!=0 draw P_Random; play if value<3. A draw that
   selects silence still occurs. Target/JUSTATTACKED early returns do not draw it.

A_Look will share the existing source algorithm, roster, noise, sounds and LifeHost
but may now enter the native Chase see state immediately. Spawn state assignment
still invokes no action. Positive native look/chase durations and stable target
profiles bound nested transitions; source actions never no-op.

## Planned admission, ownership and arithmetic

Profile9 composes profile8 perception/noise/typed static-sector services, without
weapon players, missiles, damage, doors, floating AI or attacks. Extend only
labelled native look/chase states1/2 and explicit fixture types19/21/136 copied
from source player metadata. See uses A_Chase, both attack states absent. Source
walking speed and positive state duration are bounded; original monster state
closures remain unsupported. Single-player source skills are admitted with
fastparm=false and netgame=false; no new gameplay option is implied.

Retain profile8's distinct active/health/generation roster and source scanning.
Missing/nonshootable current targets take original reacquisition/spawn behavior;
a shootable target with health<=0 still gets pursued (health only clears threshold
here). Pending/stale generations will reject before mutation rather than emulate
readable freed C pointers or alias slot reuse. Inactive old roster IDs can remain
encoded. Referenced bodies/native states must be non-removing throughout a tic.

Use bounded local validation/records and existing LifeHost/ActorAccess/spatial
services, not a full simulation/map clone. All recoverable errors must precede
position, relinking, counters, target/state, events and RNG. Keep +/-512 map,
+/-1024 query coordinates,64 actors and8192 expanded sight work. Prove margins
for one30-unit ordinary physical move plus at most one successful direct walk
per callback chain. Reject INT_MIN countdown/delta-overflow inputs in the C
comparison slice, with explicit safe Rust policies. Use an unsigned BAM shift
adapter in C where needed to define original signed-shift intent; record its
exact transformation and ABI assumption, never rewrite decision bodies.

## Reference and schema plan

Use a new isolated generator/shim, extracting genuine P_Move/P_TryWalk/
P_NewChaseDir/A_Chase plus original perception, movement, structs/metadata,
thinkers, states and RNG. No Rust expected output. Reuse historical builders as
imports without modifying them. Instrument call/return, direction assignments,
RNG, movement constraints/candidates, relinking, actor fields and state/target
order. Include cardinal/diagonal/threshold ties, blocked axes/old direction,
turnaround/fallback/traps, step/dropoff/openings, solid actors, skill/counters,
turn angles, target changes, active sound and long multi-tic RNG wrap. Record
source/header/function/adapter/input/output hashes, flags/compiler and ABI.
Rust-only failure policies are separate from C equivalence.

DSIM-v9 adds movedir and movecount to the existing actor fields; existing angle,
flags, health, target, threshold and reactiontime are not duplicated. Compose
profile8 roster/sound state and explicit pursuit observations. Scratch direction
arrays, candidate lists and plans are not persistent authority. Independent
Python bytes/digests must come only from C rows, public setup and documented
schema, with named-field first-divergence and selected complete byte fixtures.
All v1-v8 code fixtures/generators/encoders and expected bytes remain unchanged.
No implementation or test-count claim is made in this pre-runtime plan.
