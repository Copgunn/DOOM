# Original health/armor bonuses and counted items

Original id Software Linux DOOM 1.10, at the revision recorded in AGENTS, is the
behavioral authority. This investigation and compiled-C expectations precede
matching Rust execution. Source facts, admitted gameplay, observation adapters
and Rust safety restrictions are distinct.

| Source fact | Existing service used | Boundary |
| --- | --- | --- |
| info.h/info.c MT_MISC2=45, DoomEd2014, SPR_BON1=60; MT_MISC3=46, DoomEd2015, SPR_BON2=61 | Canonical registry and ActorLife | Original identities, no synthetic aliases |
| Health states816–821, armor822–827; six tics each; frames0,1,2,3,2,1; NULL actions; closed36-tic loops | Existing LifeHost/state countdown and thinker traversal | No presentation clock or second update pass |
| Both definitions: SPECIAL\|COUNTITEM (0x800001), health1000, radius20, height16, mass100, reaction8 | Narrow definition/item validation | Older SPECIAL-only item contracts remain |
| P_SpawnMobj draws lastlook and installs raw spawn state without action | Existing actor allocator and thinker insertion | One existing gameplay RNG stream |
| P_SpawnMapThing filters first, then randomizes positive tics with1+P_Random()%tics, then increments totalitems for COUNTITEM | Existing atomic THING batch and LevelStatistics | One extra timer draw per positive-tic placed item, in source THING order; animation itself draws no RNG |
| P_MobjThinker executes movement before the state countdown, P_SetMobjState installs metadata then action | Shared scheduler and lifecycle entry | Bon1/Bon2 state entry has no action or gameplay draws |
| P_TouchSpecialThing checks reach then positive body health before sprite dispatch | Existing movement contact service | Things precede lines; an ordinary later wall block retains collection |
| BON1 increments player.health, caps200, assigns mo.health, then GOTHTHBONUS | OriginalPlayer and synchronized mobj field | Even200 collects; ordinary P_GiveBody still refuses>=100 |
| BON2 increments points, caps200, sets type1 only if type0, then GOTARMBONUS | PlayerArmorState | Green can exceed100 and stays type1; blue stays type2 |
| P_GiveArmor tests current points>=requested type*100 | Existing ordinary armor grant | Green120 refuses green100; lower points may accept blue200 |
| P_DamageMobj computes raw physical thrust, then type1 damage/3 or type2 damage/2 capped by points | Shared DamagePlan | Effective damage supplies health/feedback/pain/death; green>100 remains one-third |
| Touch tail: itemcount++ for COUNTITEM, P_RemoveMobj, bonuscount+=6, sound | PlayerPickupState and existing lifecycle/spatial unlink | Count at cap still credits; ordinary items keep original uncounted flags |
| P_RemoveMobj may write inactive respawn queue and defers thinker retirement | Existing generational arena | Queue observations are source-only; no item respawn, freed-pointer or C allocator-reuse equivalence |

The opt-in capability is PlayerDemonBonusesV1, wire11, with a separate
PLAYER-DEMON-1-MEDIUM-GROUNDED-BONUSES-v1 context and DSIM27. Historical policies,
wire IDs, DSIM20–26 and their domains remain unchanged. The bounded world retains
one active original player0, at most one map Demon12, four weapons, Medium and
commercial settings, grounded static geometry and existing trace/travel budgets.
The two new kinds share the existing16-item and64-actor limits.

Health, armor, pickup fields and level statistics retain their existing owners.
Policy-aware checks admit live health1..200 and green armor1..200 only for the new
capability. Ordinary green's grant remains100; absorption depends on type, never
on points crossing100. No duplicate resource or statistics store is appropriate.
Map totalitems is established at spawn and persists through collection/removal;
player itemcount changes only on successful counted contact. Explicit external
removal need not collect an item and never decreases the map total.

The C observer compiles genuine source bodies through the established extraction
infrastructure. Read-only observations surround bonus resource mutations,
COUNTITEM increment and map totalitems increment. Existing source observations
retain actual reach/contact/THING order, RNG, state transitions, damage, removal
and requested sounds. The declared command/weapon/bonus fragments precede original
thinkers; this is not full P_Ticker, P_PlayerThink or P_DeathThink. Function-entry
counts establish entered bodies, not branch coverage.

Integrated scenarios start from ordinary health100 and no armor, acquiring every
bonus point through real movement and map contact. Unticked components declare
controlled initial health/armor, including resource caps and states unreachable
from a single16-item map; they are not integrated encounter initialization.
Inactive item-respawn queue writes, airborne reach components and any arithmetic
inputs beyond the bounded Rust travel proof remain source-only observations.
Generations, instance provenance, conservative bounds, invalid-input rejection,
transaction rollback and step/advance equality are Rust safety policy.

Recoverable failures must discard the existing bounded actor/spatial/RNG/player/
statistics/event transaction. Ordinary source movement refusal is a successful
tic and retains earlier contacts. No second scheduler, actor store, RNG, inventory,
health or damage service is admitted. Diagnostic decoding remains observation,
not executable savegame restoration.

Soulspheres, MegaSpheres, powers, keys, backpacks, new weapons/monsters, generalized
loot, item respawning, multiplayer, full player/death thinkers, dynamic geometry,
audio playback and rendering remain excluded. The inherited ordered-operand,
signed-shift/BAM and GNU ABI observation adapters remain qualified separately;
positive bonus arithmetic adds no floating-point gameplay.

Physical component T40 first collects health at199, then requests raw300 damage.
Its source impulse exceeds the existing30-unit momentum admission. The post-hit
C result remains source-only; Rust compares its pickup to the identical T4 pickup
and proves atomic rejection of the physical request. Other admitted direct
resource/damage components retain complete post-action comparisons. This does
not widen physical bounds or substitute reduced/effective damage for raw thrust.
