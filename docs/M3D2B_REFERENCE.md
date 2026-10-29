# M3d2b source investigation: bounded projectile foundations

Historical delivery accounts are retained in the [complete original investigation](validation/archive/pre-m3d3j-m3d2b_reference.md); the source findings below retain their technical scope.

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

## Delivered profile 7 admission and observed evidence

The preceding investigation was recorded before implementation; the following
contracts describe the delivered runtime and verified comparisons.

| Area | Executable admission |
| --- | --- |
| Missile | Exact original MT_PLASMA 34 only; metadata/states unchanged |
| Launch | Explicit Actor(destination) or PlayerAutoAim from an active unbound source |
| Missile flags | Initial 67088; explosion clears only MISSILE, retaining 1552 |
| Source/target actors | Admitted ordinary original nonshootable types, unbound MT_PLAYER, or labelled player-state fixtures 15/17/136 |
| Native fixtures | Copy source player properties/closures; only name/doomednum, health, reaction, pain chance, mass, radius/height and admitted flags may vary; health 1..1000, mass >=10, pain 0..256 |
| Dimensions/flags | Existing map-wide positive-height/radius <=32 admission; baseline lifecycle flags plus NOBLOOD, with MISSILE added only for exact type 34; SPECIAL/pickup, NOCLIP, skull charge/AI and unsupported actions reject |
| World | Existing static-sector/BLOCKMAP bounds (128 lines/sectors, 4096 cells/BSP work, 65536 words); no sky or specials |
| Arithmetic | Map vertices, sector heights, BLOCKMAP origin and BSP node origin/delta within +/-4096 map units; setup actor XYZ within +/-4096 |
| Runtime arithmetic | Step active XYZ and XY momenta within +/-8192, Z momentum within +/-1024; setup momentum setters cap XY +/-4096 and Z +/-64; launch source/destination XYZ within +/-4096 and derived momz within +/-64 |
| Work | At most 64 actor slots; original state work bounds; no callback spawning/removal during impact |
| Relationships | Existing generation-aware target field retains owner. Intended destination is validated at launch only; no tracer field |
| Player behavior | No bindings or weapon-player initialization in profile 7; actual player damage remains rejected |

Changing missile state/death/target/momentum through setup setters rejects;
use launch/impact/lifecycle services. Ordinary explicit actor setup remains
available. A failed launch (invalid/stale/pending identity, unsupported type,
unsafe slope/arithmetic or full capacity) preserves full canonical state. Live
missile owner removal makes the next unpaused step reject before mutation; remove
the missile too before retiring its owner. Explosion frames can safely retain a
stale owner. A removed intended victim does not invalidate a missile's launch
intent because no victim/tracer reference is stored.

Admission proves source direct damage cannot fail after its RNG draw: every
shootable target uses admitted player see/pain/death closures, has bounded positive
metadata, cannot be bound, cannot drop items or score, and cannot spawn/remove
actors during a damage callback. Current geometry/handle/arithmetic and state
work are checked before gameplay mutation. Storage reservation changes allocator
capacity only, which is deliberately absent from canonical state. No per-tic
Simulation, map or store clone is used.

### Original-C execution and comparisons

`generate_m3d2b.py` compiles 51 listed extracted original function bodies,
including the five admitted source actions, with real original headers/info,
m_fixed.c and tables.c. The fixture loader, allocation retention, observations
and synthetic launch wrapper are adapters. Unsupported original services abort
if reached. Source, all headers, adapters, inputs/outputs and compiler/ABI are
hashed in `m3d2b_provenance.json`. Compiler observed: Debian GCC 14.2.0,
GNU89/O0/fwrapv/no-strict-aliasing, contraction off, standard excess precision,
signed char, 32-bit int/fixed and 64-bit pointer/function-union host ABI.
Retaining retired C memory defines observation/traversal without dereferencing
freed storage; Rust retires generations normally. Ordered shadow/jitter draws
replace undefined signed negative shifts with multiplication. Wrapping BAM and
source shift/division distinctions are explicit. Extreme undefined arithmetic
outside the admission slice is rejected rather than claimed equivalent.

41 scenarios independently compare **157,872 live actor fields**, **13,189 ordered
observations**, **1,194 movement decisions**, **6,229 candidate callbacks**,
**16 openings** and **549 raw gameplay RNG draws**. The 40 explicit API scenarios
account for 155,352/12,991/1,151/6,013/16/542 respectively. One synthetic wrapper
launching during an original thinker accounts for 2,520/198/43/216/0/7; it proves
same-pass append scheduling without claiming an original monster firing action.
Damage request observations compare actual source/inflictor/target handles and
amounts, alongside health, momentum, pain/death states and RNG.

Scenarios include actor/player-helper launch and +/- autoaim probes, shadow
spread, initial wall/actor obstruction, positive/negative/diagonal movement,
clear flight, valid/narrow/closed/exact-eight-unit openings, blocking flags,
floor/ceiling impact/equality, zero-momentum floor skimming, nonshootable
solid/nonsolid contacts, source/same-species/player/baron-knight distinctions,
vertical pass-over, things before walls, equal-contact head order, nonsolid
shootable residual motion, lethal/pain transitions, two missiles, both 50-missile
batches crossing RNG wrap and reusing retired slots, omission BLOCKMAP behavior,
explosion progression, null unlink and next-pass retirement.

`m3d2b_snapshot_goldens.py` produces **129 independent DSIM-v7 captures and 35
full byte streams**, entirely from C rows, synthetic inputs and explicit schema.
It covers every setup, complete clear-wall/dropoff lifecycles, settled lethal
state and first/second-batch generations. Transient damage event bytes are not
independently encoded here; their semantic ordering/actor results are checked
against C and the established v5 damage tests. Project schema goldens are distinct
from source equivalence. Named-field comparison precedes complete-byte/hash checks.

Seven additional unit/policy tests cover invalid/pending/stale/reused identities,
owner retention, capacity, setup/arithmetic/state/action guards, exact XY/Z touch
predicates, bound-player protection, pause, independent simulations and repeated
presentation reads. These are consistency/contract evidence, apart from the
explicit C scheduling comparison in the same unit suite.

The asserted `projectiles` example checks four C-derived full lifecycles including
portal passage, wall collision, lethal direct hit and complete explosion/removal.
Its independently encoded final digest is **e060071ebbb323f7**. All six older
examples and v1-v6 schemas/fixtures/generators/encoders/helpers remain unchanged.
No complete plasma-rifle, rocket, BFG, other missile, splash, AI, armor/player
injury, renderer, demo or authentic-IWAD synchronization claim is made.
