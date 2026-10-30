# M3g3 investigation: bounded player knockback and grounded movement

This investigation precedes runtime implementation. Preserved Linux DOOM 1.10
is behavioral authority. Source readings below are not new compiled observations.
COMPATIBILITY owns accepted inputs, STATE_FORMAT owns diagnostics, TESTING owns
commands, and the milestone evidence record will own observed results. Historical
M3g2 stationary-body inputs, fixtures and tools remain immutable.

## Source and integration matrix

| Original source / behavior | Existing Rust service and required integration | Defined domain, dependencies and independent evidence |
| --- | --- | --- |
| [p_inter.c P_DamageMobj](../linuxdoom-1.10/p_inter.c): reject nonshootable/dead targets before thrust | DamagePlan and LifeHost::apply_damage already implement the early return | Validate current generational references first as Rust safety policy. Observe physical repeated damage to dead bodies without new thrust/RNG. |
| Baby halves damage before thrust; chainsaw source-player, MF_NOCLIP and inflictor absence suppress thrust | Shared combat already has normal physical thrust; genuine owned players explicitly reject it | New opt-in policy retains Medium/commercial/single-player, pistol, no armor/powers/cheats, no NOCLIP. Inflictor may be a current grounded original player or Demon. Source is a distinct optional player/Demon reference. No weapon/projectile system is implied. |
| R_PointToAngle2(inflictor XY, target XY), damage*(FRACUNIT>>3)*100/mass, FixedMul cosine/sine, add to existing momentum | Reuse shared integer point_angle, original fine tables, Fixed and damage execution | Pristine MT_PLAYER0 mass100. Damage0..1000 makes the original ordered numerator at most819200000. Check deltas, accumulation and future travel before mutation. Observe cardinal/diagonal, repeated, zero and lethal physical hits independently. No floating-point substitute. |
| Reverse thrust requires damage<40 AND damage>health AND target.z-inflictor.z>64U before drawing P_Random | Existing conditional source branch remains shared | Both admitted target and inflictor are grounded at floor0; vertical predicate is false. No thrust RNG draw or reverse-thrust compatibility claim. Pain/death draws remain in source order. |
| Thrust precedes player health/attacker/damagecount/tactile, then signed mobj health, pain/retarget or kill | Same DamagePlan/LifeHost and player source record | Reuse M3g2 health, frags, death, corpse and presentation-request boundaries. Death does not clear momentum. New C inputs must compare this combined chronology, not merely calculate an impulse. |
| [p_mobj.c P_MobjThinker](../linuxdoom-1.10/p_mobj.c): XY, conditional Z, then state countdown | Shared thinkers::run, existing XY/Z and lifecycle services | No second scheduler. Grounded player z=floor0 and momz0 makes source player Z/view-height/landing branches unreachable. Compare unchanged Z fields and absence of vertical events, not full airborne compatibility. |
| P_XYMovement clamps components to MAXMOVE30U; subdivision tests only positive x/y>15U; division and signed shift differ | Shared movement::move_xy_multisector already preserves this arithmetic | Retain asymmetric source subdivision. Bounded physical hits must demonstrate actual positive two-attempt and negative one-attempt movement. Inputs exceeding the forward travel proof reject instead of inventing a clamp. |
| Failed player movement calls P_SlideMove, including dead player bodies; three corner traces, two retries and stairstep | Existing player slide path is selected by an optional command, separate from actor input bindings | Supply a zero command from exact original player/mo association. Restrict new policy to axis-aligned, one-sided, nonspecial walls in a flat single sector; no angled projection, steps/dropoffs, dynamic sectors or generalized sliding claim. Compile genuine slide functions instead of the old aborting service stub. |
| [p_map.c P_CheckPosition/P_TryMove/PIT_CheckThing/PIT_CheckLine](../linuxdoom-1.10/p_map.c): things before lines, source BLOCKMAP order, strict solid overlap | Existing collision, SpatialIndex and prepared source lists | Reuse candidate order and relinking, never sort handles. Other admitted definitions have no MF_SPECIAL pickup branch. Observe wall and solid dying-Demon blocking, selected slide lines, allowed destinations and spatial order. |
| Ground friction0xe800, strict STOPSPEED0x1000; corpse off-step exception | Shared movement already has friction/stop and corpse exception | Flat floor0 equals subsector floor: corpse exception cannot skip friction. No airborne/no-momentum-cheat/missile/skull behavior. Independently observe positive/negative decay and stopping. |
| A stopped RUN frame calls P_SetMobjState(player.mo,S_PLAY) before countdown | Existing PlayerWeapons stop-state hook, shared LifeHost::enter | Extend the same hook through checked player/mo association. No PlayerThink movement or automatic RUN entry. Nonpain255 plus source retargeting can independently reach RUN and then friction-stop into PLAY. |
| [p_tick.c](../linuxdoom-1.10/p_tick.c) thinker insertion/deferred removal; [p_enemy.c](../linuxdoom-1.10/p_enemy.c) all-inactive roster scan does not terminate | Existing generational owner, context/provenance guards and M3g2 live-AI rejection | Kill/retire live Demon sources before traversal. Dying/corpse actors still precede the player in validated owner order. Never fabricate automatic acquisition or unsafe dangling-pointer execution. |

## Selected execution and representation boundary

Add an explicit PlayerBodyKnockbackV1 capability beside PlayerBodyLifecycleV1;
do not broaden the stationary policy. Both execute the same genuine MT_PLAYER0,
original states149–153/156–173, one source player record, actor store, DamagePlan,
checked bounded transaction and thinker core. Preserve actor/geometry capacities.
The unbound historical context body is not admitted or relabeled.

The selected immutable player policy must remain stored once alongside the source
player record in a private runtime wrapper. Public OriginalPlayer observations
remain source fields; no per-actor policy/type/health mirror, new owner, scheduler
or engine-global counter is necessary. ActorId generation and game-instance
qualification retain their existing meanings. Pending, stale, retired, reused or
foreign references reject through existing guards; source pointer assumptions are
not Rust lifetime guarantees.

Only this new policy admits controlled physical damage to the bound body. Its
source and inflictor are independent, and a dead-but-allocated grounded Demon is
a valid controlled origin. Inputs remain zero; PlayerThink, active roster,
weapons, camera, inventory, armor/powers, respawn and multiplayer are excluded.
Direct momentum control remains unavailable; shared damage is the admitted cause
of player XY momentum. Grounded physics is real subsequent thinker execution.

All unsupported references, geometry and arithmetic reject before authoritative
mutation. Shared bounded actor/player/RNG/statistics/spatial/event staging remains
active in debug and release; late failure discards the plan. Paused/error
precedence and advance without implicit Snapshot remain unchanged.

## Forward coordinate proof

M3g2 admits initial player coordinates through ±994U. A new physical hit must not
commit momentum that makes a later required tic unsupported. This is a Rust
support proof, not an original gameplay clamp or a second physics predictor.

Let M be max(abs(momx),abs(momy)) in raw16.16 bits after checked source impulse
addition. Require M<=30U and, for each coordinate, abs(position)+P(M)<=994U,
where P(M)=ceil(160M/3), calculated in i64 only for admission.

MAXMOVE permits at most two outer XY iterations. An axis-wall slide multiplies
by nonnegative fractions<=1 and only zeros a projected component. Its two retries,
successful flushes and final stairstep give a conservative per-axis displacement
bound4M+8 raw bits for the whole pass. Signed fixed rounding cannot increase a
component above M before friction. Afterwards M'<=floor(29M/32)+1, and
P(M)-P(M')>=5M-55. For M>=4096 this exceeds4M+8. When0<M<4096 source stops after
one pass and P(M)>=4M+8; M=0 moves nothing. Thus the coordinate condition remains
true on subsequent supported tics, including axis sliding, solid blocking and
moving corpses. Additional hits must establish it again. Conservative rejections
near boundaries do not approximate an accepted source operation.

Validate these assumptions against actual shared services and independent C
observations before claiming admission. In particular, arbitrary angled slides,
airborne origins, direct momentum mutation and unsupported geometry must not
silently escape the proof.

## Diagnostic and evidence decision

Existing DSIM-v22 already carries actor XY/Z/momentum, ownership/attacker, health,
states, movement/slide, damage and ordered event fields. Retain its layout rather
than create a milestone-driven version. Select a distinct wire execution policy
and exact grounded-context schema, while old policy2/NOTHRUST and every v20/v21/v22
fixture retain their interpretation. Reject mixed policy/schema pairs. Diagnostic
decoding stays bounded/read-only, not executable-world restoration.

New expected fields/events/full bytes must come from compiled original C and an
independent schema encoder. Reuse frozen extraction/encoding primitives without
editing historical tools; add actual P_HitSlideLine, PTR_SlideTraverse and
P_SlideMove source bodies where the player observer previously aborted. These
bodies already occur in historical movement evidence: distinguish added observer
inventory from globally new source functions, and count actual executed functions
and entries separately. Keep compact selected state captures, all compared
chronology and a small set of full fixtures; do not duplicate huge raw traces.

Rust-only policy tests additionally cover reference generations, provenance,
unsupported settings/geometry, exact travel boundaries, rejection atomicity,
retirement/reuse, replay and step versus advance+snapshot. Existing fixtures and
source/encoder pairs remain immutable regression gates. Matched three-process
measurements preserve existing before/after workloads, raw samples, allocation
observations and RSS; new physical workloads are reported separately.

## Integration finding: preserve Demon map admission

The first Rust comparison rejected raw Demon creation at the established
MapSpawn::DirectSpawn boundary. This is an intentional ownership/provenance guard,
not a physical-thrust mismatch. Do not widen that guard to make a test pass.
Use synthetic THING batches for the new Demon origins/blockers instead: genuine
P_LoadThings/P_SpawnMapThing in C and the existing original map-spawn API in Rust.
Include their actual spawnpoint, initial timer, RNG and available-kill observations.
The player remains a genuine bound raw MT_PLAYER body, as in M3g2. Revise only new
unreviewed M3g3 inputs and their C-derived outputs; preserve historical bytes.

Physical hits additionally require an already executable thinker closure: live
unlisted Demons must be killed or retired before the hit, rather than committing
momentum into an AI-blocked next tic. This is Rust admission policy, not a claim
that original P_DamageMobj rejects live origins. Existing no-inflictor controlled
source operations retain their M3g2 boundaries. No new per-tic nonplayer scan is
introduced for profiles without a genuine player body.

## Implemented disposition

The integration uses the existing LifeHost thrust code unchanged and validates
its defined arithmetic/future travel before staging. Shared movement selects the
player path through exact mo association, and the existing stop hook now uses typed
OriginalState::S_PLAY_RUN1/S_PLAY constants instead of two inherited string lookups
and unwraps. This preserves the historical weapon-player hook as well; no state
translation table or additional callback is added. New source fields, complete
events and independent full fixtures agree, while old player fixtures remain
unchanged. [The engineering record](validation/M3G3.md) owns observed counts,
checks, raw measurements and limitations. The rest of the graph/ownership design
above remains the declared bounded contract, not whole-player compatibility.
