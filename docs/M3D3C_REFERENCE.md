# M3d3c source investigation: attack selection and bounded actions

Recorded before runtime modifications. Authority: original Linux DOOM 1.10,
baseline `a77dfb96cb91780ca334d0d4cfd86957558007e0`. Work starts from verified
combined master `876662d31a79cc9be3307932e83e32faa8bf09f7`, after separately
authorized regular merges of PRs #11 and #12 and their post-merge CI.

## Source and dependency investigation

Inspected p_enemy.c, p_map.c, p_maputl.c, p_mobj.c, p_tick.c, p_sight.c,
p_inter.c, p_pspr.c, p_local.h, info.c/h, m_random.c and r_main.c, together
with [M3d1](M3D1_REFERENCE.md), [perception](M3D3A_REFERENCE.md) and
[pursuit](M3D3B_REFERENCE.md). Original source files remain unchanged.

P_CheckMeleeRange returns false for NULL target. P_AproxDistance uses absolute
XY deltas, max+min-(min>>1). Reject distance >= MELEERANGE-20*FRACUNIT plus
target **info.radius**, then test P_CheckSight. Runtime collision radius is not
the metadata radius. Equality rejects; sight is not evaluated on an earlier
return. The query has no RNG or actor mutation.

P_CheckMissileRange requires a non-null target in source. Sight runs first.
Blocked sight leaves JUSTHIT intact. Otherwise JUSTHIT (64) is cleared and
immediately accepts, even with nonzero reactiontime. Reactiontime then rejects
without RNG. Compute approximate distance minus64 units and minus128 more when
meleestate is null, arithmetic-shift by16, then apply: VILE rejects >896;
UNDEAD rejects <196 and otherwise halves; CYBORG/SPIDER/SKULL halve. Cap at200,
then CYBORG at160. Only the final branch draws P_Random: draw < signed distance
rejects; otherwise accepts. Negative distance is retained, not clamped to zero.
These type identities isolate arithmetic; they do not admit those monsters.

A_Chase retains reaction/threshold countdown, source BAM turn, target
reacquisition and JUSTATTACKED handling before attack selection. Melee state
plus successful melee range has priority: attacksound, immediate state entry,
return. Missile state on ordinary skills skips its range check whenever
movecount !=0 (including negative values), with fastparm=false. Nightmare
bypasses that gate. After successful missile range: immediate state entry,
then JUSTATTACKED (128), then return. Selected attacks skip movement and active
sound. A subsequent JUSTATTACKED branch clears the flag; ordinary skills call
NewChaseDir, nightmare returns directly. netgame=false excludes target switching.

A_FaceTarget returns on NULL target, otherwise clears AMBUSH (32), uses original
R_PointToAngle2 and adds (first P_Random - second P_Random)*2^21 when target has
SHADOW (262144). Define this as wrapping unsigned 32-bit BAM arithmetic, with
explicitly ordered calls. No floating point or renderer geometry is involved.

A_PosAttack returns on NULL target, calls A_FaceTarget internally, aims along
that angle at MISSILERANGE=2048, emits sfx_pistol, draws two horizontal-spread
values (difference*2^20), then damage ((P_Random()%5)+1)*3 and P_LineAttack.
This differs from P_GunShot's player pistol formula and spread/draw order.
S_POSS_ATK1 separately calls FaceTarget for10 tics (POSS frame4), ATK2 calls
PosAttack for8 (frame5), ATK3 has no action for8 (frame4), then RUN1. Both facing
calls must occur, including two distinct pairs of shadow RNG draws.

P_AimLineAttack and P_LineAttack transitively require original path traversal,
line/thing intercepts, fine angles, fixed division, openings and actor geometry.
P_LineAttack creates real blood/puff thinkers before damage: two Z jitter draws,
spawn/lastlook draw, timer draw, optional source state override. Damage includes
thrust, health, pain RNG/state, reaction reset, source retargeting, or original
kill flags/quarter-height/death-state actions and death-timer RNG. Existing
M3d1 services implement these dependencies; execution must share those services.
P_SetMobjState acts immediately on entry; P_MobjThinker counts down after XY/Z;
P_RunThinkers processes appended impacts in the same pass and removes lazily.

A_SPosAttack adds three independent pellet shots; A_CPosAttack and refire add
rapid-fire closures and sight/RNG loops. Sarg adds melee damage; Troop/Head/Bruis
switch between melee and source projectile spawning. SkelMissile introduces
missile ownership, vertical launch and seeking actions. Splash, boss, floating,
door, special and player-pointer branches remain outside this assignment.

## Proposed explicit profile and complete action closure

DSIM-v10 composes v9's state, roster and sound targets with shared M3d1 damage
and effects. No persistent attack-decision cache is needed. Profiles1..9 and
their original-C generators, encoders, fixtures and helpers remain frozen.

Use labelled native definitions copied from unbound source player bodies,
without player_t. Native owner IDs3/5/18/19/21/136 isolate missile type branches;
only136 may select the executable PosAttack animation. States1/2 provide
positive-duration Look/Chase, state3 is an explicit action-free melee marker,
states4/5/6 copy the three POSS attack states with native next indexes5/6/2,
and state7 is an action-free missile marker. Markers prove selection/state entry
only, never melee damage or a source attack. Unchanged original metadata stays
authoritative outside these exact, named fixture adaptations.

| Capability | Admission |
| --- | --- |
| Original monster metadata | Recognized, complete gameplay rejected |
| Labelled native Look/Chase | Selection and existing walking |
| Labelled melee/missile markers | Entry only; no damage action |
| Copied POSS three-state animation on native136 | FaceTarget, PosAttack, return to Chase |
| Unbound dummy135 with source player body states | Shared non-player damage/pain/death |
| Original puff/blood types and states | Exact existing impact lifecycle |
| Other original callbacks, probes or transitions | Reject before mutation |

Bounded ownership: one unshootable native owner, spawned first and remaining
thinker head, with zero physical momentum; direct source walking still runs.
Other admitted shootable actors are unbound dummy135; original body-state
closures cannot chase or recursively attack. Exact impact actors may retire.
Retained target/roster/sound references exclude removable impacts. Actual player
binding, armor, health mirroring and damage remain unsupported and protected.

This restriction allows a read-only shot plan before authoritative changes:
copy only owner/RNG, evaluate internal facing and the exact aim/spread/damage,
prepare existing combat trace/damage and reserve impact capacity. Positive
10/8/8 attack durations mean a Chase selection cannot fire in that same entry.
When ATK1 expires into ATK2, the owner is first, stationary and no earlier
thinker/RNG mutation can invalidate that plan. Shared LifeHost execution then
performs source effects; targets and newborn effects run later in source order.
No full Simulation/Map/store clone per tic and no separate attack scheduler.

Keep existing +/-512 map, +/-1024 query, 8192 sight, 64 actor and 128 intercept
bounds. Owner direct margin1008 and tic margin978 retain the earlier movement
proof. Moving targets/effects require the994 XY margin and checked Z bounds;
preflight must account for new damage thrust and impact jitter/motion. Reject
unsupported state closures, capacity, specials, sky and arithmetic before
events, counters, RNG or state changes. Null missile targets are typed errors;
null melee/facing/PosAttack retain source early-return behavior. Pending/stale
handles never alias reused slots. Rust resource/identity policies are distinct
from source behavioral equivalence.

## Independent evidence requirements

Compile genuine original melee/missile/Chase/Face/Pos bodies together with
the existing real sight, state, thinker, trace, damage and impact bodies.
Use original structs, info, fixed helpers and tables. Instrument source points
without replacing decisions. Explicit ordered multiplication replaces historical
negative signed shifts/unspecified operand call order in shadow/spread/jitter;
unsigned movedir shift retains the prior defined BAM intent. Record every body,
header, adapter, input/output hash, compiler flags and ABI assumptions.

Use source-derived synthetic maps and isolated metadata for radius boundaries,
sight/height rejection, signed distance/type/cap branches, RNG early returns,
ordinary/nightmare gates, action/state/flag ordering, double shadow facing,
real hit/miss/wall/pain/lethal effects, repeated cycles, deferred retirement and
generation reuse. Far-distance decisions can use self-referencing two-sided
boundary lines with equal sectors, retaining genuine P_CheckSight within the
existing geometry bounds; do not fabricate visibility or expand limits.

The independent Python v10 encoder must read C traces and public fixture inputs
only, including ordered transient observations and generation reuse. Full bytes
and first-field diagnostics complement behavioral comparisons. Rust-only tests
cover recoverable-error atomicity and unsupported/extreme inputs; those are not
C-equivalence evidence. Final counts, reproduction and acceptance status belong
in [VALIDATION.md](VALIDATION.md) after actual verification, not this plan.
