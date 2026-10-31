# M3g6 investigation: autonomous canonical Demon-to-player melee

This investigation precedes runtime edits. Preserved Linux DOOM 1.10 is the
behavioral authority. [M3g5](M3G5_REFERENCE.md) establishes the active player and
shared acquisition/pursuit boundary; [M3g3](M3G3_REFERENCE.md) establishes checked
physical player damage and grounded movement. Source observations, diagnostic
projections and Rust safety restrictions remain separate evidence families.

## Source and integration matrix

| Original source and fact | Existing service | Selected disposition and required evidence |
| --- | --- | --- |
| [info.c](../linuxdoom-1.10/info.c)/info.h: MT_PLAYER=0, MT_SERGEANT=12, Demon DoomEd3002; MT_SHOTGUY/MT_SHADOWS are different definitions | Original identity registry, pristine descriptors and canonical storage | Reuse existing roles and ordinals. Add no actor, alias, persistent identity or capacity. |
| [p_enemy.c A_Chase](../linuxdoom-1.10/p_enemy.c) tests melee before pursuit, requests attacksound, then immediately enters meleestate | monsters::select, LifeHost state entry | New opt-in capability may enter the attack graph; acquisition-only policy5 must keep its exact transactional rejection. Compare selection/sound/action chronology. |
| S_SARG_ATK1/2/3 are original485/486/487, each8 tics, actions FaceTarget/FaceTarget/SargAttack | Existing registry states and one thinker countdown | Strike occurs16 tics after selection; return to RUN1 occurs8 later. Never collapse selection into damage or bypass state entry. |
| A_FaceTarget returns on no target, clears AMBUSH and computes current angle; SHADOW can add draws | Existing shared face service | Current owned MT_PLAYER cannot carry SHADOW in this domain. Observe changing angles and ambush clearing; do not invent face RNG. |
| P_CheckMeleeRange requires approximate distance strictly below MELEERANGE-20U+target.info.radius, then sight | Shared melee_host/action_sight and original sight geometry | For pristine player radius16, threshold60U is strict. Preserve59/60 behavior and actual sight; do not replace with ideal distance. |
| A_SargAttack returns on no target, faces, rechecks range, then draws ((P_Random()%10)+1)*4 | Shared strike dispatch and DamagePlan | Use actual staged state at action time. Misses draw no damage; hits draw once and request4..40 damage. Compare movement during windup and repeated attacks. |
| P_CheckMeleeRange does not test health or shootability; P_DamageMobj later returns early for a dead/nonshootable body | Existing ignored DamagePlan/application | A dead allocated target during windup may pass range and consume the attack draw, then receive no thrust/health/pain/kill effects. Keep this distinct from a range miss. |
| [p_inter.c P_DamageMobj](../linuxdoom-1.10/p_inter.c): source and inflictor are separate; SARG supplies actor for both | DamageRequest and LifeHost::apply_damage | Validate the current live Demon, its actual target, and current player association. Keep both request fields explicit. No synthetic target plan or future damage cache. |
| Thrust precedes player health/attacker/damagecount/tactile, signed mobj health, then pain/reaction/retarget or kill | Shared physical damage, player record and checked travel proof | Reuse execution and arithmetic. Extend the existing impulse proof to current staged actor copies, with unchanged coordinate/momentum/geometry limits. |
| Reverse thrust draws only after damage<40, damage>health and height difference>64U | Existing shared conditional branch | Both admitted actors have z0/momz0. The height predicate is false, so no reverse-thrust RNG or coverage claim. Source lethal cases exercise the earlier predicates without reaching that draw. |
| Player health clamps0 while mobj health remains signed; damagecount caps100; nonlethal pain always draws and chance255 fails on draw255 | Existing source player fields, pain and retarget services | Preserve synchronization, attacker assignment and exact draw/action order. Reaction reset can end a declared initial command delay; observe commands on following tics. |
| P_KillMobj clears shootability/float/skullfly/nogravity, sets corpse/dropoff, quarters height, clears player solidity, sets PST_DEAD, requests weapon lowering, enters death and draws timer jitter | Existing kill/player/death services | Retain the same owned allocation, mo/attacker, source target and fixed activity. Demon source gives no player frag; player lacks COUNTKILL. Weapon lowering/tactile remain existing observed service boundaries. |
| A live target has health>=1 and SARG damage<=40, so resulting health>=-39 | Existing normal/extreme death logic | Autonomous extreme death(<-100) and commercial high scream(<-50) are unreachable here. Existing controlled-damage extreme-death behavior remains historical coverage. |
| [p_mobj.c](../linuxdoom-1.10/p_mobj.c): physics precedes state countdown, death does not remove momentum, -1 terminal corpse does not retire | Shared XY/friction/slide and thinker services | Newly delivered impulse moves at the player's later thinker in the same tic. Compare moving corpses, wall collision and terminal allocation. |
| [p_tick.c](../linuxdoom-1.10/p_tick.c) places player processing before thinkers; [p_user.c](../linuxdoom-1.10/p_user.c) contains many other player responsibilities | Existing bounded command/reaction stage and advance core | Keep that declared slice, not full P_Ticker/PlayerThink/DeathThink. Do not add view, damagecount decay, camera, weapon or respawn behavior silently. |
| A_Chase sees a dead/nonshootable target, scans the nonempty active roster and enters spawn if unsuccessful without clearing target | Existing roster view and source scanner | Death retains player activity/body and the Demon's target. Preserve source lastlook and repeated dead-player encounters; never execute all-inactive C scanning. |
| [p_map.c](../linuxdoom-1.10/p_map.c)/p_maputl.c/p_sight.c use source spatial order, BLOCKMAP and fixed arithmetic | Existing collision, sight and spatial owners | Keep one flat sector, static axis-aligned one-sided walls and existing limits. No reordered blockers or floating-point physics. |

## Selected execution and transaction boundary

Add `PlayerDemonMeleeV1` as a separately selected capability. It composes the fixed
active player0 and at most one map-THING Demon with existing grounded commands,
perception, pursuit, damage and lifecycle. The single OriginalPlayer and actor
store remain authoritative. No player manager, actor field, identity mirror,
scheduler, RNG stream or second simulation loop is necessary.

The existing late map-melee plan assumes an unbound context body. Keep that path
for historical profiles. For the genuine player, resolve current actors from the
existing staged host when the actual Sarg action runs. Face, check current range,
then draw damage in source order. Validate the current player/mo/health and live
inflictor, apply the unchanged checked impulse/travel proof, and construct the
existing shared DamagePlan from those current records. The same damage executor
performs all health, pain, death, momentum and reference updates.

The bounded actor/spatial/player/RNG/statistics projection already supplies a safe
late planning boundary. A checked combat failure uses its existing failure latch
and discards the whole operation before commit, including earlier commands,
facing, sounds, RNG, queues, spatial order and tic. It does not predict a future
hit or prepare health from a prior tic. No full-world clone or unchecked release
mode is justified. `advance` remains diagnostic-free; `step` delegates and captures
once on success. Complete fields and ordered events must match at both APIs.

The new wrapper validates the actual Demon target against the owned mo and exact
current generation. Existing global preflight checks definition/state roles,
pending removal, geometry and references. The fixed player cannot retire, and the
one-Demon canonical action closure cannot invalidate it during the staged pass.
Dead allocation remains valid storage; stale or reused handles never redirect.
Foreign instances remain guarded by existing GameInstance/ExecutionContext APIs.
These are Rust safety policies, not original C pointer-lifetime equivalence.

## Independent evidence and diagnostic decision

Reuse the frozen active-player C driver, source extraction and declared command
gate. New inputs exercise genuine original A_Look/A_Chase/A_SargAttack and
P_DamageMobj/P_KillMobj through source thinker/state scheduling. The autonomous
chain starts at health100 and uses no injected attack or damage. Separate cases
declare initial health reduction, reaction delay or between-tic target changes;
those controls are not automatic gameplay initialization.

Commit C expectations and independently encoded bytes before the corresponding
Rust implementation. Preserve every old tool/helper/fixture. Count compiled
pristine bodies separately from actual function entries and newly exercised
input families; inventory is not branch coverage. Retain compact selected full
captures, complete per-tic event/motion decisions and a few useful full fixtures.
All-inactive scanning, freed-pointer dereference and unsafe pointer emulation are
excluded. Prior quarantine/generation evidence remains reused evidence.

DSIM-v22 already has all actor/player/target, momentum, state/timing, RNG, statistics
and ordered melee/damage event fields. Keep its layout with wire policy6 and exact
context `PLAYER-DEMON-1-MEDIUM-GROUNDED-MELEE-v1`. Policy5 retains its no-melee
context and rejection. Unknown or mismatched pairs reject; v20/v21 and all previous
v22 bytes remain intact. Diagnostic decoding is observation, not world restoration.

New turn cases use nonnegative literal defined source expressions. Existing
negative-turn unsigned-bit interpretation and its intentional UB probe remain
qualified reused evidence; no portable ISO-C equivalence is inferred for negative
signed left shift. Existing GNU ABI/fixed arithmetic and presentation service
boundaries remain declared by the inherited observers.

## Excluded work

Full player/death thinkers, view/camera, damagecount decay, player weapon firing,
inventory, armor/powers/cheats, respawn, roster changes/multiplayer, other monsters,
infighting, sound propagation, airborne/general geometry, dynamic sectors, Fast or
Nightmare, resurrection and rendering remain excluded. Future player weapon
admission must independently establish its own source state/ammo/damage closure
through these same shared services.

## Implemented disposition

The selected path uses the existing current-body DamagePlan builder after checked
genuine-player ownership and impulse validation. The authoritative capability
stays in PlayerRuntime and is copied only into the operation's transient roster
selector; OriginalPlayer and actors gain no persistent fields. Source expectations
and independent bytes were committed before runtime integration. Complete C field/
event comparison and every-tic step/advance equivalence passed without changing
those expectations. [Measured evidence](validation/M3G6.md) owns actual totals,
safety campaigns and performance; [support](COMPATIBILITY.md#autonomous-original-demon-player-melee)
owns the accepted domain and [format](STATE_FORMAT.md#dsim-v22-autonomous-demon-player-melee-policy)
owns the new policy/context pair.
