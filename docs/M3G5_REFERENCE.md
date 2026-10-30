# M3g5 investigation: active original player and Demon acquisition

This design precedes runtime changes. Original id Software Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0` is the behavioral authority.
[Compatibility](COMPATIBILITY.md) owns admitted inputs; [STATE_FORMAT](STATE_FORMAT.md)
owns diagnostics; [TESTING](TESTING.md) owns commands; the
[engineering record](validation/M3G5.md) records observed results separately.

## Source and dependency matrix

| Source fact and location | Existing service | M3g5 disposition and evidence |
| --- | --- | --- |
| [d_player.h](../linuxdoom-1.10/d_player.h) separates playerstate, health, mo and attacker; [doomstat.h](../linuxdoom-1.10/doomstat.h) declares playeringame separately | OriginalPlayer owns health/state/mo/attacker; ActorStore owns the body | Retain one canonical record. Explicit policy establishes fixed active slot 0; derive the transient scan view from current player fields. Death does not deactivate. |
| [p_enemy.c P_LookForPlayers](../linuxdoom-1.10/p_enemy.c) initializes stop=(lastlook-1)&3 and skips inactive slots before its count/stop check | perception::scan already implements source scan order with a bounded four-slot walk | Reuse this scanner, never nearest-player sorting. Independently observe lastlook 0,1,2,3. With only slot0 active and initial lastlook1, the first encounter is the stopping slot and fails before health/sight. |
| P_LookForPlayers checks player.health, then sight, then rear angle/distance, then assigns player.mo | Same scan service and checked sight | A living active canonical player is eligible; an active dead player remains listed but fails health. Rear delta strictly between ANG90 and ANG270 requires approximate distance <=64. Do not substitute mobj shootability for source player-health filtering. |
| All-inactive source search can loop forever because inactive slots bypass its stopping check | Historical EmptyRoster event is a Rust safety policy | New execution requires a current owned body and fixed nonempty active roster before scanning/stepping. Never execute all-inactive C. Preserve historical empty-roster behavior in its original profile. |
| A_Look resets threshold, considers sector soundtarget/AMBUSH, then player fallback | Shared Look and target/lifecycle services | Admit visual fallback with all sector soundtargets absent; AMBUSH map flag remains valid. Reject sound-target configuration/propagation rather than inventing it. |
| MT_SERGEANT seesound is single sgtsit, not the randomized posit/bgsit groups | Shared Look sound dispatch | Sound precedes immediate see-state entry; no wake-selection RNG for this Demon. Subsequent A_Chase can consume RNG. Compare complete ordered events. |
| [info.c](../linuxdoom-1.10/info.c) defines MT_PLAYER=0, MT_SERGEANT=12, SARG stand475/476, run477..484, attack485..487 | Original registry preserves these ordinals | Genuine player and genuine Demon only; no synthetic target alias. Keep all metadata, restrict this policy's executable attack states. No Shotgun Guy/Spectre admission. |
| A_Chase decrements reaction, modifies threshold/facing, then checks target shootability and scans allaround on loss | Shared pursuit::chase and perception scanner | Admit real bounded pursuit. On failed fallback the source enters spawn but retains a dead allocated target pointer; keep target handle without clearing it automatically. |
| A_Chase checks melee before pursuit; existing late map-melee planning assumes an unbound synthetic target | Shared melee planner is not yet independently established for autonomous player damage | Reject the actual melee-selection branch transactionally before attack sound/state/damage. Do not silently skip the source branch or fabricate pursuit through it. Canonical metadata recognition does not admit attacks. |
| [p_tick.c P_Ticker](../linuxdoom-1.10/p_tick.c) runs active player stages before P_RunThinkers | M3g4 command stage in the shared execution core | Compose the same bounded movement/reaction gate before thinkers. Demon-first insertion means it can observe the player's pre-XY position despite newly applied player momentum. This is not full P_Ticker/P_PlayerThink. |
| [p_mobj.c](../linuxdoom-1.10/p_mobj.c) distinguishes death/corpse from deferred removal; source pointers lack generational safety | Existing handles, execution scope and GameInstance provenance | Preserve corpse association/activity. Reject removal of this fixed active player's body; no roster deactivation/respawn is admitted. Demon/effect retirement and generation reuse remain shared store behavior. Test stale/foreign/pending references as Rust safety policy. |
| [p_inter.c](../linuxdoom-1.10/p_inter.c) synchronizes damage/player health and retains attacker; physical damage may change momentum/reaction | Existing DamagePlan and M3g2/M3g3 player services | Reuse controlled damage/knockback and commands. Observe target death/fallback and source retirement without dereferencing freed C pointers. This does not admit autonomous melee. |
| [p_sight.c](../linuxdoom-1.10/p_sight.c) and [p_maputl.c](../linuxdoom-1.10/p_maputl.c) retain original side/intercept quirks | Shared checked sight and source BLOCKMAP | Observe both a genuinely blocking axis wall and the inherited x=0 horizontal-ray side-test quirk separately; no idealized geometric substitution. |

## Selected ownership and execution boundary

Use `PlayerDemonAcquisitionV1`, a new opt-in OriginalExecutionProfile beside all
historical policies. The context fixes source playeringame to `[true,false,false,false]`.
Existing player-body creation/binding supplies the actual current MT_PLAYER actor.
Activity is independent of health and body existence; incomplete setup may be
observed but cannot scan or tick. Only one map-THING Demon is admitted; zero is
allowed for player-only execution. Actor, map and geometry capacities are unchanged.

The existing OriginalPlayer and generational actor store remain authoritative.
A small borrowed/derived roster selector supplies the current player's health/mo
at each shared Look/Chase scan. No independently mutable roster health copy, new
player manager, persistent scope token, actor identity mirror or second scheduler
is introduced. A derived public roster observation is not execution permission.
The historical fixture roster setter/getter retain their historical contract.

Current body ownership, actor definition, generation, player/mobj health/state,
geometry and supported roles validate at operation boundaries. The fixed player
body cannot be marked for removal under this policy; death retains its allocation,
association and active status. Invalid/pending/retired/stale bodies reject before
mutation. Existing scoped/instance API boundaries reject foreign provenance.
Local raw ActorId values retain their documented local-simulation meaning.

Existing source target assignment writes the real player's ActorId. Dead allocated
references remain distinguishable from stale generations. Demon retirement does
not erase the player's attacker reference; later observation resolves it using
existing lifetime rules, never by raw slot alone.

Use the existing transaction, shared thinker pass, RNG, spatial order and command
services. `advance` must still construct no Snapshot; `step` delegates to the same
core and captures once. A late unsupported melee branch must roll back commands,
movement, health, targets, counters, RNG, statistics and both event queues together.
No recoverable error may commit a partial wake or chase.

## Diagnostics and independent evidence plan

DSIM-v22 already represents canonical player/body/attacker, original actor target,
lastlook, states/timers, commands, motion, RNG, events and statistics. Retain its
layout with policy wire ID5 and a distinct fixed-roster/no-melee context identifier.
Fixed activity is declared by that schema, not duplicated as mutable serialized
player health. The independent encoder must verify source roster rows and current
mo association. Preserve historical v20/v21/v22 policy meanings and bytes.
Decoding remains observation, never executable-world restoration.

Reuse frozen compiled-source extraction and independent field/event primitives.
New scenarios must execute genuine P_LookForPlayers/A_Look/A_Chase with the genuine
source player/mo association. The driver sequences only the admitted command gate
before P_RunThinkers, not complete P_PlayerThink or P_Ticker. Controlled initial
reaction, target, state, damage and retirement operations are declared setup.
Keep compact selected full captures, per-tic motion/decisions/events and a bounded
set of complete independent fixtures. Record actual exercised functions separately
from the inherited compiled inventory; adding scenarios is not new source-body or
branch coverage. Expected values must never come from Rust observations.

Negative angle turns inherit M3g4's explicit unsigned bit-pattern interpretation
of a source expression with undefined negative signed left shift. Literal defined
nonnegative cases, normalized observations and an intentional UB probe remain
separate. No portable ISO-C equivalence claim is made for that undefined expression.
Retirement observations use the established quarantine boundary and never
read freed source memory; Rust stale-generation rejection is separately tested.

## Excluded dependencies and next gameplay slice

Full PlayerThink/DeathThink, camera, weapons, inventory, armor/powers/cheats,
respawn, multiplayer, roster deactivation, sound propagation, other monsters,
infighting, dynamic geometry and autonomous melee remain excluded. Pursuit ends
at an explicit recoverable admission boundary when source melee becomes reachable.

The next narrow gameplay slice is a checked autonomous Demon-to-player melee
DamagePlan closure: source inflictor/source, physical travel proof, synchronized
player health, pain/death/frags, late RNG and whole-operation rollback. It should
extend shared combat services rather than introduce a parallel player-versus-monster
engine. Existing panic paths, unnamed flags, evidence growth and transaction costs
remain measurable maintenance priorities, not authorization for a broad refactor.
