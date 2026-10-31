# Original armor, animation and damage investigation

This investigation and the independent C/encoding checkpoint precede matching
Rust gameplay. Preserved id Software Linux DOOM1.10 at the revision in AGENTS is
the authority. Source facts, observed behavior, executable admission and Rust
safety policy are separate claims. The new policy is a bounded gameplay
composition, not full game compatibility.

| Original behavior / exact source | Existing service and selected integration | Evidence and boundary |
| --- | --- | --- |
| `info.h/info.c` MT_MISC0=43/DoomEd2018 and MT_MISC1=44/DoomEd2019; SPR_ARM1=55/SPR_ARM2=56 | Pristine original registry, checked definition roles | Admit these two real actors only in a new policy; never synthetic aliases or metadata-as-execution permission |
| S_ARM1/ARM1A=802/803:6/7 tics; S_ARM2/ARM2A=804/805:6/6; frames0/32769; NULL actions | Existing ActorLife/state descriptors, LifeHost entry and thinker clock | Fullbright remains the original frame bits; no presentation animation, translated states or second scheduler |
| Both actors: health1000, radius20, height16, mass100, MF_SPECIAL, no COUNTITEM | Existing item/geometry/store ownership | Within16 total world items and64 actor slots, unchanged static grounded bounds |
| `p_mobj.c:P_SpawnMobj` draws lastlook and installs raw spawn state without executing its action | Existing raw allocation, registry lookup and insertion | Original actor IDs, actual source draws and birth order; no extra RNG stream |
| `P_SpawnMapThing` filters first, then draws `1+P_Random()%tics` for positive spawn tics | Existing atomic THING batch already randomizes positive timers | Armor needs the second draw; the old static-item `tics=-1` validator must not be reused unchanged |
| `P_MobjThinker` runs physics before countdown; `P_SetMobjState` installs state/timer/frame then calls any action | Shared thinker traversal and state entry | Armor loops are action-free, but their real positive countdown and entry chronology still execute |
| `p_inter.c:P_GiveArmor` accepts only when current points are below requested type*100, then replaces type and points | Armor-only attached record on the existing PlayerRuntime; shared contact service | Blue80 accepts green100; Blue100/180 refuses it; green100 accepts blue200. Type rank is not the acceptance rule |
| `P_TouchSpecialThing` checks reach and live body health, dispatches by source sprite, assigns GOTARMOR/GOTMEGA | Existing things-before-lines contact hook | Green/blue sound is sfx_itemup; GOTMEGA is the blue armor message, not a MegaSphere grant |
| Accepted contact assigns message, removes item, adds BONUSADD6 and requests sound; no COUNTITEM increment | Existing mark/unlink/deferred retirement and pickup events | Normal later wall blocking retains the grant; recoverable execution failure discards the whole staged transaction |
| `P_DamageMobj` rejects nonshootable/dead targets before thrust or player work | Existing shared DamagePlan and damage executor | Repeated damage to an allocated corpse must not consume armor or RNG |
| Physical thrust uses incoming damage before player armor; ground z0 excludes the reverse-thrust draw | Existing checked impulse and forward-travel proof | Raw60 gives the same physical impulse with no armor, green and blue; effective health damage differs. Armor cannot weaken the travel guard |
| Green saves damage/3; blue saves damage/2; saved is capped to points; points<=saved clears type before subtraction | One calculated armor consequence in the shared DamagePlan | Integer truncation, ties, depletion, zero damage and subsequent unarmored damage are independently observed |
| Player health clamps0; attacker, damagecount and tactile use post-armor damage; signed mobj health then determines death or pain | Existing authoritative OriginalPlayer, shared combat/lifecycle and weapon lowering | Preserve pain RNG, reaction/retarget ordering, source/inflictor distinction, allocated corpses, roster and statistics |
| `p_tick.c:P_RunThinkers`, `P_RemoveMobj` unlink/defer and retain source pointer assumptions | Existing generational store and transaction | Quarantined C observation is not allocator-reuse or freed-pointer safety equivalence; Rust stale generations never redirect |

The selected `PlayerDemonArmorV1` policy uses wire10 and context
`PLAYER-DEMON-1-MEDIUM-GROUNDED-ARMOR-v1`. It composes the existing genuine player0,
at most one map Demon12, fixed active roster, four weapons, commands, perception,
pursuit, melee, traces, effects and resource contacts. Medium/commercial settings,
one static floor0/ceiling128 sector, supported axis walls and all prior actor,
item, coordinate, momentum, sight and trace budgets remain. Historical policies,
including pickup-only policy9, must keep their old domains and bytes.

Armor fields have one owner beside the existing source player fields. Ordinary
green armor permits1..100 points, blue1..200; no armor is type0/points0. These are
Rust admission limits for this no-bonus policy, not a claim that every original
powerup combination has these bounds. Invalid combinations reject rather than
normalize. Original player and mobj health retain their distinct source-required
synchronization; there is no second player, inventory, damage system or lifetime
manager. A valid armor definition does not establish a live actor or ownership.

The shared DamagePlan retains raw request damage for source request events and
thrust. It calculates effective damage and the armor consequence once, using
current staged fields at the actual attack action. That consequence commits after
thrust and before player health/feedback, signed mobj health and death/pain.
Preview/commit must not independently recompute protection or cache a future hit.
The same bounded player/actor/spatial/RNG/event transaction owns rollback.

New persistent armor scalars and grant/absorption observations justify DSIM26,
with an explicit policy/context and reader. Historical v20–v25 meanings, full
fixtures and encoders remain unchanged. Existing actor state/sprite/frame/tics
already represent animation; redundant immutable item metadata is unnecessary.
Diagnostics remain observations, never executable-world restoration.

The [new observer](../tools/reference/m3g10_reference.py) reuses frozen extraction
and the declared command/weapon/bonus stage before original thinkers. It replaces
the old aborting GiveArmor service with the actual original body and adds read-only
before/after observations. It is not complete P_Ticker, P_PlayerThink or P_DeathThink.
The [source contract](../tests/reference/m3g10_contract.json) records exact source,
body, input and output hashes, instrumented function entries and evidence counts.
Compiled body inventory is not branch coverage; inherited bodies are not newly
implemented source functions. P_GiveArmor is the one newly extracted body.

Scripted encounters start with no armor and acquire it through actual movement
and world contact. Long depletion cases use actual medikits, never runtime health
or armor edits. Separate declared initial health/loadouts and unticked grant,
damage-phase RNG and low-point setups are component evidence, not ordinary player
initialization. Rare non-pain255 is an actual P_Random result under a declared
component seed, not an injected draw. Source-only inactive respawn-queue rows
remain qualified observations outside the runtime/diagnostic policy.

An initial source experiment retained redundant post-outcome tics. Its raw output
is preserved outside the repository; the final inputs shorten those intervals
while retaining sustained depletion, death and corpse continuation. No historical
expectation changes. New filenames avoid frozen historical generator/map/provenance
discovery globs. Future investigators should include this explicit contract when
determining inherited source-function coverage.

Source arithmetic retains the inherited explicit ordered-draw, negative signed
shift/BAM and GNU ABI qualifications. Armor's positive integer division introduces
no floating point. No invalid C indexes, all-inactive scan or freed-pointer gameplay
is exercised. Generations, provenance, conservative travel limits, invalid-input
rejection, rollback and step/advance equivalence are separate Rust safety policies.

Armor bonuses, other powers, cheats, Baby scaling, sector11 protection, telefrag
systems, generalized crushing, item respawn, new weapons/monsters, full player/death
thinkers, dynamic sectors and rendering remain unsupported. Source branch presence
does not imply admission. Measured implementation/test/performance outcomes belong
in the milestone evidence record; support, schemas and commands belong in their
authoritative guides.
