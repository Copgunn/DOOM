# M3d2 source investigation: player weapons and psprites

Recorded before runtime changes. Authority: untouched Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. M3d1 corrected head `838b401` passed
CI 40619300366 and was regularly merged as `c924f18424a7a58f892fc8b6a7c2b5011db05c5b`.
Post-merge CI 40619889131 passed all four jobs before this branch was created.

## Source dependencies and ordering

Inspected p_pspr.c/h, p_user.c, p_mobj.c, p_inter.c, p_map.c/p_maputl.c,
p_local.h, d_player.h, d_ticcmd.h, d_items.c/h, g_game.c, doomdef.h, d_event.h,
info.c/h, m_random.c, p_tick.c and P_NoiseAlert/P_RecursiveSound in p_enemy.c,
plus earlier state/physics/combat investigations. Original files remain unchanged.

P_Ticker visits active player slots 0..3, then P_RunThinkers, specials/respawn,
then increments leveltime. P_PlayerThink handles reaction-time movement lock,
P_MovePlayer, P_CalcHeight, weapon selection, use, then P_MovePsprites before
power/flash counters. Bound-player XY/Z occurs later in its actor thinker.
P_MovePlayer turns before thrust, only thrusts on ground, and enters PLAY_RUN1
when movement commands are nonzero and the body is in PLAY. Ground friction may
restore PLAY from the four run states. P_CalcHeight computes weapon bob from
pre-friction XY momentum: (FixedMul(mx,mx)+FixedMul(my,my))>>2, capped at 0x100000.
Camera/viewheight and powers are separate dependencies; full player parity is
not implied by reproducing weapon bob.

G_PlayerReborn clears player state except intermission counters, sets attackdown
and usedown true, LIVE, health 100, ready/pending pistol, fist+pistol ownership,
50 clips and maxammo [200,50,300,50]. P_SpawnPlayer spawns/binds MT_PLAYER, assigns
slot color flags and actor angle/health, clears refire/light/counters, sets viewheight
and calls P_SetupPsprites. This milestone will initialize the weapon-relevant
subset explicitly on a validated existing pawn; full spawn/respawn, HUD, camera,
armor/inventory, intermission and pickups are excluded.

## Psprites

P_SetupPsprites clears two state pointers, makes pending=ready and calls BringUp.
BringUp substitutes ready when pending is nochange (source value 10), clears pending,
sets weapon Y=128 units, then enters upstate. A_Raise runs immediately, subtracting
6 units: initial Y is 122, not 128. Later raising reaches/clamps 32 and recursively
enters ready; readiness/attack can therefore run on that same tic.

P_SetPsprite sets state/tics, changes BOTH coordinates only when misc1 != 0,
then invokes the two-argument action. Null entry clears only the state pointer,
retaining tics/coordinates. After an action it follows the CURRENT psprite state
and tics, preserving recursive replacements. Zero durations chain immediately;
-1 never counts down. P_MovePsprites processes weapon then flash: a newly created
flash can decrement on its birth tic. Finally flash X/Y copy weapon X/Y even if
flash has no active state. Psprites are player-owned records, never thinkers.

A_WeaponReady resets body ATK1/ATK2 to PLAY, handles pending/dead lowering before
attack, then sets attackdown and calls FireWeapon. Pistol/shotgun/chaingun can
fire while held. Releasing clears attackdown only when Ready executes. Idle bob
uses angle=(128*leveltime)&8191, X=1+FixedMul(bob,cos), Y=32+FixedMul(bob,sin),
with the Y phase masked to the positive half wave. A_ReFire increments refire
only while attack is held, no switch pending and health nonzero; otherwise resets
refire=0 and checks ammo. A_Lower adds 6 until Y>=128, changes ready=pending and
calls BringUp immediately; it does not first clamp Y to 128 for a live player.

## Weapon definitions and attacks

weaponinfo[] holds source ammo/up/down/ready/attack/flash states for all nine
recognized weapons. Executable core: pistol and shotgun. Chaingun is conditional.
Unpowered fist support is appropriate only to preserve source exhausted-ammo
fallback; chainsaw, supershotgun and projectile weapons remain excluded.

P_CheckAmmo uses one round for the admitted firearms; fist uses no ammo. Source
fallback priority is plasma, supershotgun (>2 shells), chaingun, shotgun, pistol,
chainsaw, rocket, BFG (>40 cells), fist. The admitted ownership subset reduces
that exact branch order to chaingun/shotgun/pistol/fist, never silently granting
ownership or ammunition. Out of ammo sets pending and enters current downstate.
P_FireWeapon checks ammo, enters body ATK1, enters attack psprite, THEN calls
P_NoiseAlert. Actual firing actions may occur immediately or several tics later.

A_FirePistol: sound, body ATK2, deduct one clip, flash (Light1), BulletSlope, GunShot
accurate iff refire==0. Its attack begins with a four-tic nonfiring frame.
A_FireShotgun: sound, body ATK2, deduct one shell, flash, BulletSlope ONCE, then
seven sequential inaccurate GunShots. Its first firing frame begins after three
tics. Each GunShot draws damage first: 5*(P_Random()%3+1), then two spread draws
when inaccurate, angle += (left-right)*2^18, then P_LineAttack at range 2048.
Blood/puff draws and target damage/pain/kill draws finish before the next pellet.
BulletSlope tries angle, angle+2^26, angle-2^26 at range 1024, stopping when a target
is found; it adjusts the slope, not the actual firing angle.

A_FireCGun sounds BEFORE its zero-ammo return; otherwise it consumes one clip,
selects flashstate+(currentState-S_CHAIN1), aims and fires like pistol. Its two
four-tic firing states precede a zero-tic ReFire state. Support requires independent
last-round/zero-ammo/flash/refire cases. A_GunFlash (not required by the admitted firearms) sets body ATK2 and flash;
Light0/1/2 change persistent extralight. A_Punch (if admitted for fallback) draws
2*(random%10+1), two spread draws, aims/shoots at range 64, and on actor hit emits
punch sound and turns toward the target. Berserk remains excluded.

P_NoiseAlert is unconditionally reachable from FireWeapon, so it cannot be stubbed.
P_RecursiveSound visits sector lines in source grouping order, follows positive
openings, allows crossing at most one ML_SOUNDBLOCK line, revisits a sector only
if the route crosses fewer blockers, and assigns persistent sector.soundtarget.
This propagation does not itself execute monster AI. Preserve target generations;
serialize sound targets, not traversal-validcount cache generations.

## Planned bounded profile and atomicity

An explicit profile 6 / DSIM-v6 will preserve versions 1..5. One weapon-capable
LIVE player may occupy any of four ordered command slots. Others remain inactive;
validate commands in all slots before mutation. Supported buttons are ATTACK=1,
CHANGE=4 and weapon bits 3..5 (mask 56); require CHANGE with selection bits and
reject USE=2, SPECIAL=128, unknown bits and deferred weapon selections.

Keep direct combat APIs' bound-player restriction. A private weapon-only route
may admit the validated shooter as source/inflictor while damageable targets
remain unbound. Reuse the M3d1 trace/damage implementation, not another raycaster.
Narrow registry admission will retain exact original state tables and source type
metadata except explicitly labelled type-136 player-state fixtures. Damageable
pawns must have source player pain/see/death closures. This excludes immediate
spawn/remove combat probes and AI; hit eligibility can only disappear through
kill flags during a multi-pellet burst, leaving trace geometry unchanged. Source
impact thinkers are NOBLOCKMAP. No second bound damageable target is admitted.

Preflight the weapon interpreter on a copy of only the small player record,
one preview pawn and the two RNG indexes. Replay prospective source draws and
keep an overlay only for targets actually damaged (health/shootability/height),
using the same pure aim/hit traversal with an actor lookup. Source pawn pain/death
entry frames have no immediate actions; their one pain/kill draw plus optional
forward-fall and four impact draws can therefore be proven before committing.
Validate every actual prospective trace/damage prerequisite and conservative
impact capacity. No authoritative RNG is consumed by preflight. Reserve up to
seven impacts for one firing group; pending retirement supplies no credit. Bound
state entry/recursion, traversal/intercepts and actor capacity. A conservative
64-slot profile keeps aggregate state work bounded. Runtime replay uses admitted
infallible operations; an invariant failure is an engine bug, never a recoverable
partly committed tic. No full Simulation/Map/actor-store clone is used per tic.
Source movement/body state, original player bob, psprites, ammo and events must
execute in original order; renderer reads never advance them.

## Evidence plan and C policies

Build a separate original-body C harness with real player_t/weaponinfo/state data,
full supported P_PlayerThink/psprite/fire functions, M3d1 geometry/effects/damage,
noise propagation and thinker updates. Reachable behavior under test must remain
original bodies. Abort stubs are limited to excluded paths (player damage, use,
powers, AI/projectiles/specials, unsupported weapons). Record each command, state,
actor outcome, ammo/refire/flash, RNG draw/index and action/thinker order per tic.
Independent Python v6 encoding must use only C rows and explicit schema/setup.
Add an asserted player_weapons example and safety/determinism/fuzz checks.

Original spread/jitter expressions shift negative signed differences and have
unspecified operand evaluation order. Define ordered left/right RNG draws and
representable multiplication in the adapter; use wrapping BAM angle addition
in Rust. Signed negative command shifts become explicit multiplication. Use the
recorded GNU C ABI, fixed widths, source/header/function/adapter/input/output
hashes, compiler flags and documented source-undefined exclusions. Never derive
C expectations from Rust output. C-undefined extremes test safe Rust errors.

No complete player damage, armor, powers, pickups, projectiles, AI, moving sectors,
demos, rendering, audio playback or scripting is implied. Private assets and
private repositories remain outside this assignment. Evidence and implemented
bounds will be updated after observation, without claiming planned checks passed.


## Observed implementation and evidence

Implemented profile 6 admits pistol, shotgun, chaingun and unpowered fallback fist,
one LIVE weapon player in any slot, and at most 64 actors. Source tables/types
remain exact except the labelled type-136 player-state fixtures. Player damage,
armor/powers/pickups/full view/spawn/respawn and projectile weapons remain excluded.
The private shooter route preserves direct APIs' bound-player restrictions.

38 original-body C scenarios pass 4,890 captures: 132,030 player and 290,913 live
actor fields (422,943 total), plus actor angles, sector noise targets and combined
source weapon/lifecycle event order. Cases include all slots, movement/bob/stop,
positive/negative turn, raising/ready/refire/switching, zero/last ammo, held fire,
sequential shotgun targets, source effects/deaths, and one/two sound blockers.
Held shotgun consumes 323 source RNG draws; long pistol consumes 268, covering wrap.
The source nochange sentinel is 10 (NUMWEAPONS itself occupies enum value 9).

The independent v6 encoder passes 69 captures and 24 full byte fixtures. The
asserted example checks nine C scenarios per tic and ends at independent digest
`1cc5a89450da5585`. Eleven unit/policy tests separately check safe errors, capacity,
identity reuse, recursion/timing edges and deterministic read schedules. Public
admission excludes native psprite/actor probes; internal tests isolate malformed
interpreter graphs without claiming original-C compatibility for them.

`generate_m3d2.py` lists each original extracted function and hashes all source/
headers, adapters and outputs. It compiles real G_PlayerReborn, P_PlayerThink,
psprite/ammo/fire actions, BulletSlope/GunShot, noise propagation, M3d1 damage/
hitscan/effects/physics/state/thinker functions, info.c, d_items.c and original
fixed/trig/RNG tables. Fixture inventory grants are explicit after source rebirth.
A driver supplies the source active-slot/player-before-thinker ordering and static
special/respawn exclusions; no weapon behavior under test is stubbed. Sliding and
unsupported player/power/use/AI/projectile/special paths abort if reached; no fixture
reaches them. Player camera bookkeeping runs in C but is not claimed as Rust player
parity. C retirement retains memory only for observations. C allocation ordinals
map independently to generational Rust identities in the reference test.

Reproduce with `generate_m3d2.py --check` and `m3d2_snapshot_goldens.py --check`.
All earlier generators/encoders remain unchanged. Refer to TESTING/VALIDATION for
actual toolchain results and sanitizer limitations; no optimization is claimed.
