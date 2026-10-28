# M3d2b source investigation: bounded projectile foundations

Recorded before runtime changes. Authority: untouched Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. M3d2 correction `7007dae` passed
CI 40625682243, was regularly merged as `955f690d141ccc6a9074a8a8017f78833854af6f`,
and post-merge CI 40626027970 passed before this branch was created.

## Original functions and selected type

Inspected p_mobj.c/h, p_map.c/p_maputl.c, p_inter.c, p_enemy.c, p_pspr.c,
p_local.h, info.c/h, m_random.c, r_main.c, thinker/list services and earlier
movement/state/combat/weapon investigations, including their fixed/trig/RNG and
spatial dependencies. No original file is modified.

Select MT_PLASMA (source type 34). Its speed is 25*FRACUNIT, radius 13, height 8,
direct damage metadata 5 and flags 67088 = MISSILE 65536 | NOGRAVITY 512 |
DROPOFF 1024 | NOBLOCKMAP 16. Flight states 107/108 each last six tics and loop;
explosion states 109..113 each last four tics and end at null. All actions are
None. Launch sound is source ID 8, explosion sound 17. It needs no splash,
A_Explode, BFG spray, trail spawning, AI or boss behavior. Rocket, other fireballs,
BFG and full plasma-rifle weapon operation remain unadmitted.

## Spawn and ownership

P_SpawnMissile spawns at source X/Y, source Z+32, using ordinary P_SpawnMobj
(lastlook RNG, direct state assignment, spatial/sector link, thinker append).
It emits seesound, assigns mobj.target=source (OWNER, not intended victim), computes
R_PointToAngle2(source,dest), and adds two shadow spread draws at <<20 when dest
has MF_SHADOW. XY momentum is source FixedMul(speed, original fine cosine/sine).
Travel time is P_AproxDistance(destXY-sourceXY)/speed, integer division, clamped
at one; momz=(dest.z-source.z)/time. Spawn height +32 is deliberately absent from
that vertical difference. The destination is not a retained projectile reference.

P_SpawnPlayerMissile instead probes base/+/-2^26 at range 1024, retaining the
selected AIM ANGLE (unlike BulletSlope). If all miss, restore source angle and
zero slope. It uses the same spawn/owner/sound and XY momentum, with momz equal
to FixedMul(speed,slope). No player_t dereference is needed by this helper; an
unbound source pawn can isolate it without enabling actual player damage.

P_CheckMissileSpawn draws tics randomization and clamps >=1, directly advances
X/Y/Z by arithmetic momentum>>1, THEN calls P_TryMove at those new coordinates.
The direct half-step does not relink first. Success relinks and updates constraints;
failure explodes at the advanced coordinates while retaining old constraints/
subsector until later movement. Preserve that distinction in canonical state.

P_ExplodeMissile zeros all momentum, enters registered death state, subtracts one
P_Random()&3 from CURRENT tics and clamps >=1, clears MISSILE, then emits deathsound.
NOGRAVITY/DROPOFF/NOBLOCKMAP and owner remain. Impact does not immediately remove
plasma; its complete explosion chain precedes null spatial removal and next-pass
thinker retirement. A normal in-thinker impact is followed by that same thinker's
state countdown, so the initial explosion frame may already advance that tic.

## XY/Z and collision ordering

P_MobjThinker runs XY, checks pending removal, runs Z when z!=floor or momz!=0,
checks removal, then state countdown. XY clamps each momentum axis to +/-30.
Subdivision is triggered only by POSITIVE xmove>15 or ymove>15; the first increment
uses /2 (toward zero), residual uses >>1. Negative/positive odd values can therefore
lose different low bits. No swept ray or floating integration is appropriate.

On a blocked missile move, explode but do not break the residual XY loop. Remaining
attempts use the now non-missile flags. This can move an explosion on a later
attempt if the original shootable contact was nonsolid. Active missiles skip all
ground friction. Z applies momentum, clips at floor/ceiling, zeroes the relevant
vertical component, explodes and returns immediately at contact. Exact z==floor
with zero momz does not invoke Z at all. Plasma's NOGRAVITY is preserved.
Sky suppression is a separate source branch and remains rejected by map admission.

P_CheckPosition visits expanded BLOCKMAP thing cells (X outer/Y inner) before any
lines; each cell follows its current head order. NOBLOCKMAP plasma is not itself
in thing collision lists, although it retains sector links. Other missiles lacking
SOLID/SPECIAL/SHOOTABLE are skipped. Bbox boundaries use >= rejection (touching XY
is not overlap), while vertical over/under uses strict >/< (touching Z can hit).
Self is skipped. Source owner immunity is tested after vertical overlap.

If the owner type equals the contacted type, or is the knight/baron pair, owner
identity itself is ignored. Other same-species actors cause an explosion without
damage except MT_PLAYER, for which other unbound player objects can receive damage.
Nonshootable solids block without damage; nonshootable nonsolids pass. Shootable
contacts draw ((P_Random()%8)+1)*info.damage, call P_DamageMobj(contact,missile,owner),
and stop candidate traversal. Damage precedes explosion. Actor contacts can win
before a simultaneously relevant wall because things precede lines.

PIT_CheckLine ignores ML_BLOCKING/ML_BLOCKMONSTERS only while MISSILE is set;
one-sided lines still block and two-sided openings update floor/ceiling/dropoff.
P_TryMove still enforces interval fit, Z headroom and the source 24-unit step test.
DROPOFF exempts plasma from the dropoff-height restriction, not all grounded tests.
Missing BLOCKMAP coverage keeps source-list behavior; do not silently rebuild it.

## Planned profile, safety and integration

Explicit profile 7 will compose existing typed actors, static-sector geometry and
M3d1 damage. No new allocator, actor animation scheduler or damage implementation
is needed. Existing actor type/flags/XYZ/momentum/state/timer/target and immutable
registry already represent all persistent plasma state. target is its owner;
intended victim and launch aim are transient. V7 need not invent redundant owner,
tracer or damage fields. Older profiles and schemas 1..6 remain unchanged.

Use the same generational ThinkerStore and append-sensitive traversal. Factor its
internal cursor if needed so an actor can execute accepted damage services without
holding an aliasing store callback borrow; both traversal routes must share the
same retirement/append rules. Plasma state updates use LifeHost. No original AI
or weapon callback is added merely to launch missiles; an explicit typed launch API
for source-target and player-autoaim helpers is sufficient. Synthetic scheduling
tests can inject an admitted launch during a thinker without claiming monster AI.

Bound to 64 actors, exact original states/types except labelled player-state
fixtures, and a declared arithmetic slice. Only plasma receives missile flags.
Shootable native fixtures retain source player pain/death closures, bounded health
and positive mass; knight/baron source-ID fixtures may isolate the species rule
without admitting their AI. Reject bound players, unsupported actions/types, sky/
special maps, invalid/pending/stale launch handles, unsafe coordinate/momentum
setup and exhausted capacity before authoritative changes. Live missiles require
an active generation-valid owner; source removal must never become a dangling read
or alias a reused slot. Explosion frames may retain an unresolved old owner because
they no longer evaluate missile ownership. Direct player damage remains protected.

Admission proves impact damage/state operations cannot fail midway through a tic:
no admitted immediate callbacks spawn/remove actors during damage, no bound targets,
no scoring/drops, bounded mass/health/arithmetic and no explosion spawning. Reserve
launch capacity before any sound, RNG or actor mutation. Preflight current world/
handles/arithmetic before every step; no full Simulation/Map/store clone per tic.
Record exact implemented bounds and safe departures after validation.

## Actual player damage: dependencies and future acceptance plan

This milestone investigates these dependencies only; existing guards stay active.
P_DamageMobj uses target.player, not merely MT_PLAYER type. Baby skill halves damage
before thrust. Thrust precedes player immunity/armor checks (chainsaw-source thrust
exemption depends on source.player.readyweapon). Sector special 11 prevents lethal
player damage. God/invulnerability ignore damage below 1000 AFTER possible thrust.
Armor type 1 saves damage/3, other nonzero armor saves damage/2; exhausted armor
clamps savings to armorpoints and clears type. Mirrored player health clamps at
zero while mobj health may become negative. Store attacker handle; accumulate
post-armor damagecount capped at 100 and decrement feedback in P_PlayerThink.
Console tactile output must become presentation-neutral without reordering state.

Lethal player handling clears solidity, changes playerstate to DEAD, calls
P_DropWeapon, handles automap, updates environment/source frag and kill counters,
then uses the existing death/xdeath actions and randomized timer. P_DeathThink
moves psprites, lowers viewheight, turns toward attacker and gates rebirth on USE.
A_Lower differs for DEAD versus zero-health LIVE players. Respawn, scoring and
multiplayer assumptions cannot be guessed or enabled by removing a combat guard.

A future player-damage milestone needs canonical playerstate, mirrored health,
armor amount/type, relevant cheats/power counters, attacker generation, damage/
bonus feedback, viewheight/delta/viewZ where implemented, weapon lowering/death
interaction, required stats and mode assumptions. Independently compare baby/
normal skill, immunity threshold 999/1000, both armor types/exhaustion, zero/lethal/
extreme health, attacker retirement/reuse, death/psprites, and failed-operation
atomicity. Actual armor/inventory/respawn/multiplayer remains unauthorized here.

## Reference plan and C policies

Compile original spawn/check/explode, XY/Z, position/thing/line/TryMove, damage/
kill, state/thinker/list, BSP/angle, fixed/trig and RNG bodies with real headers/
info.c. Unsupported services abort only on unreachable paths. Use synthetic maps
and unbound source/target pawns, documenting modified fixture metadata separately.
Compare complete flight/impact/explosion/removal traces, candidate order, RNG,
relations and full independent v7 bytes; no expected data may come from Rust.

Replace source shadow spread's negative shift/unspecified operand order with
explicit left then right draws and multiplication by 2^20, wrapping unsigned BAM
addition. Original arithmetic right shift and toward-zero division are explicit
Rust operations. Record compiler/flags/ABI, source/function/header/adapter/input/
output hashes and excluded C-undefined extremes. Retain retired C bytes solely
for observation. Record observed evidence later; nothing planned here is a pass.
