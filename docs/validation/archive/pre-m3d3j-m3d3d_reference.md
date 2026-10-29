> Historical source investigation before public-documentation restructuring. Exact bytes: [pre-m3d3j-m3d3d_reference.txt](pre-m3d3j-m3d3d_reference.txt).

# M3d3d source investigation: sequential shotgunner attacks

Recorded before runtime changes. Authority is untouched Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. The feature branch starts from
verified PR #13 regular merge `9b8fde85f64651f12a5643324e2ab93a9e4937d8`;
correction and exact-head/post-merge CI evidence is in [VALIDATION](../../VALIDATION.md).

## Original source and timing

Inspected p_enemy.c, p_map.c, p_maputl.c, p_mobj.c, p_tick.c, p_inter.c,
p_sight.c, p_local.h, info.c/h, m_random.c, r_main.c and tables.c, together with
[M3d1](../../M3D1_REFERENCE.md), [M3d2](../../M3D2_REFERENCE.md) and
[M3d3c](../../M3D3C_REFERENCE.md). No original source file is changed.

A_SPosAttack returns immediately on NULL target: no sound, facing, aim or pellet
RNG. Otherwise it emits sfx_shotgn BEFORE internal A_FaceTarget, saves bangle,
aims ONCE at MISSILERANGE=2048, then runs exactly three sequential P_LineAttack
calls. This sound ordering differs from A_PosAttack (face, aim, pistol sound).
There is no monster noise-alert call, ammo/psprite processing or aggregate damage.

Each pellet takes the same saved base angle and shared initial aim slope. Draw
left then right for spread: wrapping BAM addition of (left-right)*2^20. Draw
damage next: ((P_Random()%5)+1)*3. Complete that pellet's trace, real impact and
damage before drawing the next pellet. Do not batch nine attack draws or delay
all damage until the end. Earlier pain, death and effect draws change later RNG.

Original SPOS attack metadata is sprite 30 (SPR_SPOS): ATK1 frame4,10 tics,
FaceTarget->ATK2; ATK2 frame 32773 (fullbright5),10 tics,SPosAttack->ATK3;
ATK3 frame4,10 tics,NULL->RUN1. RUN1 is frame0,3 tics,Chase->RUN2 in the full
original eight-state run animation. A labelled native looping Chase handoff
isolates this behavior; it does not admit the whole original monster animation.
Keep fullbright 32773 in authoritative state even without rendering.

Both the separate ATK1 FaceTarget and the internal ATK2 FaceTarget run. Each
clears AMBUSH and uses source R_PointToAngle2, with two ordered shadow draws
(difference*2^21) when the current target has SHADOW. Target motion during the
intervening ten tics and fresh shadow draws can change the second facing angle.
Do not reuse or deduplicate the first facing calculation.

## Transitive effects and sequential visibility

P_AimLineAttack uses original fine angles and fixed-point path/intercept/opening
services. P_LineAttack independently collects candidates for every pellet and
uses the one shared aim slope. It spawns blood/puff BEFORE damage: two Z jitter
draws, real mobj allocation/lastlook draw, initial-timer draw, and source blood
state override. NOBLOOD uses an actual puff. A miss has no impact/effect draws.

Damage can add momentum, change health, enter pain/death, reset reaction, assign
target/threshold, clear SHOOTABLE, set CORPSE/DROPOFF and quarter height. Killing
uses strict extreme-death comparison, immediate state entry and randomized death
timer. Fall-forward has an additional conditional draw and possible reversed,
fourfold thrust. These effects and draws are visible before the next pellet.
After a first/second-pellet kill, later hitscans skip that now-unshootable body
and may hit a different target or a wall. An initial shared aim is not recomputed.

P_SetMobjState invokes actions immediately. P_MobjThinker performs XY/Z before
state countdown; the positive attack durations separate selection from firing.
P_RunThinkers appends real impacts in pellet order and can process them later in
the same pass. Creation, action effects, later thinker processing, deferred mark
and actual retirement/reuse are distinct. No early effect retirement gives credit
for another pellet's capacity.

MT_SHOTGUY is original type 2, with COUNTKILL, source pain/death/raise states and
an item-drop branch that spawns MT_SHOTGUN. Those services are not admitted merely
because SPosAttack works. Actual player pointers add armor/powers/scoring/death
requirements and remain excluded. CPos/refire, other monsters/projectiles/splash,
floating, doors, specials, multiplayer, demos and presentation are deferred.

## Proposed profile and exact admission

Introduce explicit profile 11 / DSIM-v11. Keep profiles1..10, their byte fixtures,
source generators, encoders and historical helpers unchanged. Reuse reserved
native136 in this separate profile; do not enlarge the original registry.

| Role | Admitted closure |
| --- | --- |
| Labelled unshootable native136 | Look/Chase, FaceTarget, SPosAttack, supported return |
| Labelled native states1/2 | Positive-duration SPOS look/looping Chase fixtures |
| Labelled states 4/5/6 | Copied source SPOS 10/10/10, mapped next5/6/2 |
| Unbound dummy135 | Original player body pain/see/death states without player_t |
| Exact original puff37/blood38 | Existing real impact and retirement lifecycle |
| Original type 2, PosAttack in 11, other callbacks/probes | Rejected before mutation |

The original full registry remains authoritative outside these exact named
adaptations. Validate the entire dynamic see/missile/pain/death closure. Do not
change baseline action_supported or grant SPosAttack to v10. Keep actual player
binding/damage and profile 11 public direct combat rejected. Roster health and
actor health remain separate. Retained attack/roster/sound references exclude
removable effects and validate generations before authoritative changes.

## Bounded preflight and shared execution design

Retain the v10 proof: one unshootable, floor-standing owner, spawned first and
remaining thinker head, zero physical momentum, direct walking speed1..16.
No earlier player/thinker mutates its pose, target or gameplay RNG before a
planned delayed attack. Positive SPOS durations prevent a Chase selection from
firing in the same entry. Direct calls/entry use the same preflight.

Use a copied RNG and a small overlay of only actually touched targets (at most
three) and up to three projected impact actors. Existing pure hit/damage planning
reads the overlay. Project each prepared pellet by executing the EXISTING generic
LifeHost combat executor privately against that overlay, scratch observations,
scratch links and copied RNG. That shares all source damage, state and effect
semantics instead of maintaining a second health/RNG/death model. Plan pellet 2
after pellet1 projection and pellet 3 after pellet 2. Authoritative execution draws
and commits each real pellet fully in order, verifying the planned RNG sequence.
No complete Simulation, map or actor-store clone is required.

Within this admission, damage changes eligibility/height/health/state/momentum,
but cannot move/relink an actor during the action, alter radius/BLOCKMAP flags,
remove a body immediately, spawn recursive combat or mutate geometry. Original
puff/blood are NOBLOCKMAP and do not affect later collection. Therefore each
pellet may reuse the shared base spatial/candidate geometry while its hit and
damage decisions consult UPDATED overlay state. This is a proved restricted
projection, not three shots against frozen health or shootability.

Conservatively reserve three real actor spawns for each non-null blast before
mutation, with no pending-removal credit; explicitly reject even an eventual
miss when those three slots cannot be reserved. Null returns need no reservation.
Predict the same lowest-free-slot/generation identities without copying storage;
retired/exhausted slots remain excluded. Validate all three effect coordinates,
trace/intercept work, damage plans and state closures before entry/RNG/sound.
The real commit cannot discover a recoverable error after pellet1.

Keep map/query/sight/intercept/actor bounds +/-512,+/-1024,8192,128,64. Preserve
owner direct/tic margins1008/978, other tic XY994 and impact XY994/Z1018 (covers
jitter and newborn Z motion). Stable mass100/body state closures and checked
health arithmetic retain existing damage bounds. Undefined extremes are safe
Rust rejection tests, not original-C equivalence.

## Canonical and independent evidence plan

V11 composes v10 fields and existing event layouts, adding transient shotgun
begin/pellet-boundary/parameters/completion observations as needed. Record exact
original RNG index after every pellet; never serialize the preflight plan as
persistent state. Source state/tics/frame/flags already encode the attack phase.

Compile genuine SPosAttack/Face/Chase/aim/hit/damage/kill/effect/state/thinker/RNG
bodies with original structs, info, tables and fixed helpers. Explicitly ordered
multiplication defines the same two's-complement shadow/spread/jitter intent as
v10, avoiding historical negative signed shifts and unspecified operand order.
Record compiler/flags/ABI and source/header/function/adapter/input/output hashes.
Abort only genuinely unreachable unsupported services, never fabricate success.

Cover null/sound order/shared aim/three separate spreads, all-hit/miss/wall,
shadow double-facing, early pain, kill on pellet 1/2, later targets after death,
extreme/fall-forward RNG where admitted, blood/puff, narrow/obstructed geometry,
repeated state cycles, same-pass effects, retirement/reuse and RNG wrap. The
independent Python v11 encoder must consume C/public inputs and schema only,
with distinct trace scopes for multiple pellets per operation, full bytes and
first-field diagnostics. Rust policies separately cover capacity and complete
failure atomicity, invalid closures/roles/players/handles, arithmetic, pause and
instance/presentation independence. Counts and final CI belong in VALIDATION
after actual observation. No complete Shotgun Guy/vanilla/demo claim follows.

## Implemented result and observed boundaries

Implemented the proposed strict profile 11 and SPOS 10/10/10 matrix, including
source fullbright 32773 and the separate/internal face calls. Source PosAttack's
v10 contract and every historical expected byte remain unchanged. Every non-null
blast reserves three slots before projection; the nine policy tests explicitly
check two-slot rejection, pending retirement, true-miss conservatism and null
success at capacity, alongside complete canonical error atomicity.

The private projection uses exact predicted slot/generation identities, copied
RNG, at most three touched bodies and three projected impacts. It calls the
existing LifeHost combat executor between preparations; updated lookup feeds
shared hit/damage decisions, without a second health/death/RNG implementation.
Actual commit verifies the planned RNG at each boundary and executes each real
pellet fully before its successor. Body geometry/index stability and NOBLOCKMAP
impacts justify shared base collection; hit eligibility and health are not frozen.

The harness compiles 65 original bodies and executes 105 scenarios across all 18
supplied synthetic maps. Rust compares 147,940 actor fields, 34,350 ordered C
observations and 3,894 raw draws. There are 109 non-null blasts/327 pellets, with
exact post-pellet RNG indices, fall-forward draws, all source damage values,
pain/death, changed later targets and 49 retirements. The independent v11 encoder
checks 949 captures/92 full byte fixtures; the asserted example ends at
`eb5b7166a162544e`. Full commands, sanitizer configuration, local gates and immutable
CI heads/runs are in [VALIDATION](../../VALIDATION.md). These facts cover the declared
slice, not complete original Shotgun Guy or vanilla/demo compatibility.
