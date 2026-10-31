# Original actor and state identities

The `doom_sim::identity` API identifies original Linux DOOM 1.10 metadata.
It does not grant gameplay execution. [COMPATIBILITY](COMPATIBILITY.md#original-identity-metadata)
owns the executable boundary; [source investigation](M3F4_REFERENCE.md) explains
historical semantics. Generational ActorId handles still identify individual live
objects; they are not actor *types* or states.

## Namespaces and mapping

| Namespace | Representation | Valid ordinals | Count sentinel |
| --- | --- | --- | --- |
| mobjtype_t | OriginalActorType | 0–136 | 137, rejected |
| statenum_t | OriginalState | 0–966 | 967, rejected |
| spritenum_t | OriginalSprite | 0–137 | 138, rejected |
| Registry slots | ActorTypeId / StateId | Registry-dependent | Not canonical |
| C action pointers | Descriptor Option of symbolic name | No original integer ordinal | NULL is None |

Every original symbol has an associated constant retaining its exact name.
[Generated constants](../crates/doom-sim/src/identity/names.rs) give the complete
mapping. The independently compiled [C capture](../tests/reference/m3f4_identities.csv)
records every original symbol value, descriptor and sprite resource name.
`TryFrom<u16>` and `TryFrom<i32>` reject out-of-range values, negative integers and
NUM* sentinels. `ordinal()` and `From<identity> for u16` are lossless.
`from_name` is exact and case sensitive: `MT_DEMON` is not an invented alias for
`MT_SERGEANT`. DoomEd numbers are a separate namespace: editor number 3002 is
not an actor ordinal. `all()` iterates source order without allocating.

MT_SERGEANT=12 and MT_SHADOWS=13 share SARG state references but are distinct
actor types. There is no exclusive state owner: multiple actor types and actions
may refer to a shared state. S_NULL=0 is a represented state sentinel, distinct
from an invalid identity and from an absent runtime state observation. A timer
of -1 prevents ordinary transition even when next is S_NULL. Sprite ordinals
identify inert metadata; `resource_name()` returns the original four-character
name without loading pixels.

## Descriptors and original field order

Descriptors represent pristine info.c initializers, before runtime mode changes.
Fast/Nightmare timing or projectile-speed mutations are not admitted.

`OriginalActorType::descriptor()` exposes named properties and eight typed state
relationships. Radius/height are raw 16.16 integers; speed units depend on actor
type. Sound ordinals and MF_* flag bits remain inert source metadata.
`properties()` also exposes the exact borrowed original 23-field representation:

| Index | Original field | Named view |
| --- | --- | --- |
| 0 | doomednum | doomed_number |
| 1 | spawnstate | states.spawn |
| 2 | spawnhealth | spawn_health |
| 3 | seestate | states.see |
| 4 | seesound | sounds[0] |
| 5 | reactiontime | reaction_time |
| 6 | attacksound | sounds[1] |
| 7 | painstate | states.pain |
| 8 | painchance | pain_chance |
| 9 | painsound | sounds[2] |
| 10 | meleestate | states.melee |
| 11 | missilestate | states.missile |
| 12 | deathstate | states.death |
| 13 | xdeathstate | states.extreme_death |
| 14 | deathsound | sounds[3] |
| 15 | speed | speed |
| 16 | radius | radius |
| 17 | height | height |
| 18 | mass | mass |
| 19 | damage | damage |
| 20 | activesound | sounds[4] |
| 21 | flags | flags |
| 22 | raisestate | states.raise |

`OriginalState::descriptor()` exposes typed sprite/next, frame bits, tics,
symbolic action and both misc fields. This includes weapon/psprite states;
metadata representation does not make them valid mobj callbacks. No C pointer is
serialized. Existing ActionId numeric ordering is a Rust dispatch-table policy,
not an original action identity ordinal.

## Explicit registry conversion

```rust
use doom_sim::{identity::OriginalActorType, ActorRegistry};
let registry = ActorRegistry::vanilla();
let canonical = OriginalActorType::MT_SERGEANT;
let slot = canonical.resolve(&registry).unwrap();
assert_eq!(registry.original_actor_type(slot).unwrap(), canonical);
```

Forward and reverse conversions compare the complete record, including its name,
at its canonical ordinal. Missing, renamed or altered metadata rejects with
RegistryMismatch; out-of-range candidates reject with OutOfRange. No alias lookup,
registry mutation or implicit conversion is performed. This is a local-record
identity check, not certification of other reachable registry records or of an
actor's live timer, flags or game context. Use existing execution admission after
resolution. Fixture type 136 is not silently relabeled MT_SERGEANT, and fixture
state aliases do not become canonical merely because their numbers are in range.

The canonical views reuse the static source table. Owned registries and runtime
actors retain their existing storage. Queries neither allocate nor consume RNG,
change thinker/spatial ordering, enter states or change snapshots. Construction
and simulation retain their capacities and failure policies. M3f4 itself introduced
no execution profile; the opt-in original-ID contract below is separate.

## Canonical Demon execution binding

M3f5 adds `DemonExecutionBinding::admit(&registry)`, separate from the strict
same-record conversion above. It validates MT_SERGEANT's complete source record,
S_NULL, all normal source SARG records, the native actor definition and all
corresponding native state fields/next references. It proves the declared
definition correspondence, not other registry records or arbitrary game contexts.
Profile20's existing exact constructor still validates the remaining registry.

| Canonical source state | Original ordinal | Existing runtime StateId |
| --- | --- | --- |
| S_SARG_STND / STND2 | 475 / 476 | 1 / 3 |
| S_SARG_RUN1 | 477 | 2 |
| S_SARG_RUN2–8 | 478–484 | 15–21 |
| S_SARG_ATK1–3 | 485–487 | 4–6 |
| S_SARG_PAIN / PAIN2 | 488 / 489 | 7 / 8 |
| S_SARG_DIE1–6 | 490–495 | 9–14 |

Original MT_SERGEANT12 / DoomEd3002 maps to native owner136 only through this
explicit execution binding. The native map-context actor copies canonical
properties with translated state references, DoomEd=-1 (source lookup precedes
projection), and raise=0. The source record remains unchanged with raise 496.
S_NULL is an internal reference sentinel, not a public entry binding. Raise496–501
has no executable binding. MT_SHADOWS13 / DoomEd58 (Spectre) and MT_SHOTGUY2 /
DoomEd9 (Shotgun Guy) reject even when descriptors share some references.

`Simulation::canonical_demon_map` selects the existing source-derived Profile20
registry. `spawn_original_demons(OriginalActorType::MT_SERGEANT)` requests its
existing atomic THING batch. `enter_original_demon_state(actor, original_state)`
resolves a state before delegation. Only live stand/run/attack entries pass the
existing owner policy; pain/death are damage-controlled. Unsupported types and
states reject before delegation. Read queries can observe a pending record until
retirement; retired/stale handles and unbound targets reject.

`original_demon_identity(actor)` returns source definition identity, while raw
ActorLife and DSIM retain native IDs. Binding `runtime_state` and `original_state`
round trips cover the explicit table only; they do not authorize entry or silently
convert arbitrary registry slots. No raw runtime-ID migration or full Demon/Spectre
compatibility is claimed. The [C observer](M3F5_REFERENCE.md) runs pristine
type 12/states 475–495 throughout gameplay. Comparisons distinguish physical source
fields, this definition projection, independently projected DSIM bytes and
Rust-only rejection policy.

## Native runtime identity boundary

For historical Profile20, M3f6's [consumer audit and design](M3F6_REFERENCE.md)
retains native runtime storage. Its ActorLife, events and DSIM remain native.
`NativeDemonType` and `NativeDemonState` are sealed transient execution views,
produced by the validated Profile20 binding. They carry no second authoritative
identity or live-owner permission. Existing raw registry APIs remain unchanged.

```rust
use doom_sim::{ActorRegistry, DemonExecutionBinding, StateId, identity::OriginalState};
let registry = ActorRegistry::demon_map_context_fixture();
let binding = DemonExecutionBinding::admit(&registry).unwrap();
let native = binding.native_state(OriginalState::S_SARG_STND).unwrap();
assert_eq!(native.runtime(), StateId(1));
assert_eq!(native.original(), OriginalState::S_SARG_STND);
assert!(binding.check_native_state(StateId(475)).is_err());
```

`native_type(original)` / `native_state(original)` explicitly project canonical
values. `check_native_type(candidate)` / `check_native_state(candidate)` validate
native slots before wrapping. Neither raw number ranges nor shared descriptors
grant a type/state binding: native136 means the bound Demon role, whereas original
136 is MT_MISC86; native1 represents S_SARG_STND, whereas original1 is S_LIGHTDONE.
There is no implicit conversion between these namespaces. Values remain meaningful
only in the binding's declared representation; another registry still needs its
own admission. S_NULL and raise have no native state value.

`Simulation::native_demon_identity(actor)` returns checked raw storage views for
a current Profile20 owner; pending records remain readable until retirement,
while stale/reused local generations reject. Raw values do not prove foreign-world
provenance. Its `.runtime()` values equal the actual
ActorLife fields. `.original()` names the corresponding source definitions.
The existing `original_demon_identity` delegates through this explicit boundary.
Queries do not change state, RNG, actor order or captures.

Profile20 direct-entry and corpse guards classify named original states through
the single native mapping. Stand/run/attack remain the only direct entries;
pain/death remain damage-controlled, and the corpse's pre-Fall solidity boundary
is unchanged. Earlier profiles keep their narrower fixture-specific guards and
all native identities. Source raise metadata remains represented without execution.
The original-ID execution registry below supplies a separate opt-in boundary.
It does not reinterpret historical v20 data or globally renumber136.

## Original-ID execution registry

`OriginalExecutionRegistry::new(OriginalExecutionProfile::DemonNormalV1)` owns
immutable original definitions plus an explicitly named context package. Every
source type/state retains its original ordinal and complete record; MT_MISC85/86
at 135/136 are pristine. `admit` compares the complete identified package before
accepting an externally constructed registry. Registry ownership moves into the
existing shared immutable simulation registry, not a mutable metadata mirror.

`DefinitionIdentity::Original(OriginalActorType)` and `Context(ContextRole)` are
separate typed keys. Source keys resolve directly to original indexes. The sole
context, `UnboundBody`, is appended at `OriginalActorType::COUNT`; that slot is
not an original type and canonical conversion still rejects the count sentinel.
Its descriptor derives from source player metadata with DoomEd=-1, health 1000,
painchance 0, radius 16 units and flags 6. Its state animations are shared source
metadata; it is not a real player. No live capacity increases.

`Simulation::original_id_map` or `with_original_registry` selects
`Profile::OriginalDemon`. Only the normal Demon, existing puff/blood effects and
context body have execution roles. The registry represents other actors without
admitting them. `spawn_original_actor(MT_SERGEANT)` performs the existing atomic
THING batch with direct type 12 storage. `spawn_context_body` uses the explicit
context key. `enter_original_state` uses actual source state indexes; only live
stand/run/melee direct entry passes. Damage controls pain/death; raise rejects.

Normal closure and direct/death/Fall subsets derive from source descriptor roots,
next edges and action associations in bounded immutable bit sets. There is no
native SARG mapping in this registry. Future roles require original-root/capability
policy and independent execution evidence, rather than another translation table.
`original_actor_identity` validates a current generational record and returns
actual source/context identity and optional original state. Pending records remain
readable until retirement; stale/reused handles reject. Historical native views
and canonical projections reject this profile instead of reinterpreting it.

ActorLife stores type 12 and states 475–495 directly; it stores no native duplicate.
Context bodies store their separately identified definition slot. Handles remain
u32 slot/u64 generation. Shared physics, AI, damage, scheduling and checked actor
plans preserve execution order and RNG. The only body 135-to-context binding is in
independent evidence adapters; it is absent from the new registry core.
[Design/adapters](M3F7_REFERENCE.md), [format](STATE_FORMAT.md#dsim-v21--original-id-demon-diagnostics)
and [support](COMPATIBILITY.md#original-id-demon-profile) define the remaining bounds.

## Context-scoped actor instances

DefinitionIdentity identifies metadata/roles; ActorId identifies a generational
instance within one store. ContextRole names a context definition, not a world.
`MapContextOptions` configures setup/RNG, not instance ownership. The new
`Simulation::with_execution_context` establishes an exclusive, generative access
scope for the original-ID profile; its private `ContextActor` brand identifies
that scope without a persistent counter, pointer address or per-actor mirror.

`ctx.admit_local(raw, expected_definition)` validates current storage and exact
definition. Pending storage can be observed but is not thereby authorized for
target acquisition. `ctx.actor(token)` returns a checked copy rather than a
borrow that survives validation. `ctx.target(token)` observes the retained target
through the shared definition-role guard. Mutation wrappers use the existing
simulation services, preserving source IDs, ordering and RNG.

Tokens cannot escape or cross nested/distinct scopes; compile-fail tests check
both. `token.raw_id()` explicitly erases the brand for historical diagnostics and
local APIs. Equivalent simulations intentionally have equivalent raw IDs, so
re-admitting an erased value is a local interpretation, **not** proof of its
origin. The [game-instance boundary](#game-instance-qualified-references) adds
in-process/restart provenance; persistent world IDs and portable serialized
references remain unsupported. No existing raw API is silently strengthened or
given global identity semantics. [Support/lifetime rules](COMPATIBILITY.md#context-and-target-lifetime-contract),
[source investigation](M3F8_REFERENCE.md) and [observations](validation/M3F8.md)
define this narrower boundary.

## Game-instance qualified references

`InstanceId` identifies a runtime incarnation, not a definition, actor slot,
registry/context schema or deterministic state. Each `GameInstance::new` creates
a distinct opaque identity, including when the supplied Simulation is a clone.
`InstanceActorRef` combines that provenance with the existing ActorId; it stores
no duplicate definition. `ctx.export_actor(token)` validates current storage;
`ctx.import_actor(&reference, expected_definition)` checks provenance then delegates
to local actor admission. The resulting ContextActor retains the same invariant
scope restrictions. Pending storage and corpses remain observable without granting
target/state-entry permission. Retirement/reuse still invalidates the local handle.

The wrapper's immutable `simulation()` view remains local; `into_simulation()`
consumes the owner and rewrapping creates a fresh incarnation. Successful
`restart()` rotates the key even when fresh handles numerically match old ones.
`InstanceActorRef::candidate(optional_key, raw)` explicitly assembles a candidate,
not proof of erased origin or authentication. Missing keys reject. Foreign/old
keys reject as InstanceMismatch; matching keys still require current generation,
state and expected definition. A retained reference owns only its small key.

Keys have no numeric/byte encoding and do not enter DSIM20/21. Decoded local handles
alone cannot supply provenance. [Support](COMPATIBILITY.md#game-instance-provenance),
[audit](M3F9_REFERENCE.md) and [evidence](validation/M3F9.md) distinguish this Rust
policy from reused C gameplay observations and unsupported persistence/networking.

## Genuine original player body

PlayerBodyLifecycleV1 adds actual MT_PLAYER0 execution under its explicit
[body contract](COMPATIBILITY.md#original-player-body-lifecycle), preserving
DemonNormalV1 and its synthetic ContextRole::UnboundBody137. Metadata remains the
same complete original package. Player permissions derive from spawn/see/pain/
death/extreme-death roots: S_PLAY149, RUN150–153 and PAIN/death156–173. Weapon
attack154/155 are represented but unexecutable here. Pain and death entry are
controlled by shared damage; direct player state requests reject.

PlayerSlot::One is source index0, not actor slot0. The body uses ordinary ActorLife
with canonical type/state storage, not a native identity alias. One OriginalPlayer
record stores source mo, health, state, attacker, damagecount and frags[0]; its
mo equality derives mobj-to-player association. Existing LevelStatistics supplies
source players[0].killcount. Source health duplication has explicit clamp semantics;
actor definition identity is never duplicated. Scoped/qualified binding validates
the full live handle and definition. Stored relationships survive death/retirement
without redirecting after reuse. [Source matrix](M3G2_REFERENCE.md) distinguishes
these original behaviors from Rust lifetime/provenance policy and exclusions.

PlayerBodyKnockbackV1 adds a separate executable capability using those same
MT_PLAYER0 and original state identities. The private player runtime pairs selected
policy with source state; no canonical/native actor identity mirror or new handle
namespace is introduced. Genuine Demon origins still use map-THING spawning and
canonical MT_SERGEANT12. The synthetic UnboundBody137 remains historical context,
not a player. [Physical admission](COMPATIBILITY.md#original-player-grounded-knockback)
and [source investigation](M3G3_REFERENCE.md) define the narrower behavior contract.


## Original-player command capability

PlayerCommandMovementV1 reuses the genuine MT_PLAYER=0 definition, player/mo binding
and existing original PLAY/RUN/pain/death state closure. It adds no identity alias,
actor fields or roster. Canonical MT_SERGEANT=12 stays distinct from synthetic
context bodies and MT_SHADOWS; only the already supported dying/corpse Demon
closure may coexist with command execution. Metadata recognition is unchanged.
See [accepted commands](COMPATIBILITY.md#original-player-command-movement),
[shared architecture](ARCHITECTURE.md#original-player-command-stage) and
[source ordering](M3G4_REFERENCE.md).


## Active player and canonical Demon execution

`PlayerDemonAcquisitionV1` uses actual MT_PLAYER=0 and MT_SERGEANT=12 storage and
original PLAY/SARG states. PlayerSlot::One remains source roster ordinal0,
independent of the owned body's actor slot. The existing player/mo association is
also the source of the derived active roster view. This is executable gameplay
composition, not a definition alias or another actor identity namespace.

SARG stand and run actions now observe the active canonical player through shared
Look/Chase services. For this capability, S_SARG_ATK1/2/3 and raise remain outside
execution; their original records and ordinals remain represented unchanged.
Source melee eligibility causes atomic rejection rather than silent branch
skipping. Old DemonNormalV1 and player policies retain their state permissions and
synthetic context boundaries. See [support](COMPATIBILITY.md#active-original-player-and-demon-acquisition),
[source matrix](M3G5_REFERENCE.md) and [schema](STATE_FORMAT.md#dsim-v22-active-player-demon-policy).

## Autonomous canonical Demon melee

`PlayerDemonMeleeV1` adds execution permission for the same pristine SARG attack
states485–487 against the existing owned MT_PLAYER=0. MT_SERGEANT=12 remains the
actual owner and physical inflictor; MT_SHOTGUY and MT_SHADOWS remain unadmitted.
Player slot0, actor generation, original definition and execution capability keep
their separate meanings. No alias, actor field or extra identity namespace is added.

The new OriginalExecutionProfile variant is an explicit API addition; downstream
exhaustive matches need its arm. Existing wire IDs and the public historical
Profile enum retain their meanings. The acquisition-only policy still rejects
attack execution; raise and player weapon states remain excluded. See
[combat support](COMPATIBILITY.md#autonomous-original-demon-player-melee),
[source timing](M3G6_REFERENCE.md) and [diagnostics](STATE_FORMAT.md#dsim-v22-autonomous-demon-player-melee-policy).


## Genuine player pistol roles

PlayerDemonPistolV1 retains source MT_PLAYER0 and MT_SERGEANT12 alongside the
existing Puff37/Blood38 effects. It admits player body attack states154/155 under
its explicit capability while older policies keep their state guards. Pistol
10..17 plus immediate LightDone1/Null0 are player-owned psprites, never artificial
mobj states, actor instances or native aliases. OriginalPlayer remains the sole
owner of the body's generational handle and health. Weapon-only state adds no
identity or lifetime manager. Metadata recognition still grants no other weapon
or monster execution. See [support](COMPATIBILITY.md#genuine-player-pistol-combat),
[ownership architecture](ARCHITECTURE.md#canonical-player-pistol-execution),
[source matrix](M3G7_REFERENCE.md) and [DSIM23](STATE_FORMAT.md#dsim-v23-genuine-player-pistol).

## Four-weapon player roles

PlayerDemonWeaponsV1 retains genuine body0, Demon12 and effect37/38 identities.
Fist S_PUNCH..S_PUNCH5, pistol S_PISTOL..S_PISTOLFLASH, shotgun S_SGUN..S_SGUNFLASH2
and chaingun S_CHAIN..S_CHAINFLASH2 are original player psprites, not new actor
identities or thinkers. LightDone/Null and zero-tic Chain3 actions remain immediate;
zero-duration overlay states cannot persist in a captured weapon record.
Existing actor state-role permissions are reused; metadata presence does not
admit any other weapon or monster. Adding this public execution-profile variant
requires downstream exhaustive matches to handle its arm. See
[support](COMPATIBILITY.md#original-player-weapon-expansion) and
[DSIM24](STATE_FORMAT.md#dsim-v24-original-player-weapon-expansion).


## Original world item execution

PlayerDemonPickupsV1 additionally admits static canonical definitions53/54/63/64/
69/70/73/77 with original spawn states840/841/870/871/876/877/880/884 and sprites
STIM/MEDI/CLIP/AMMO/SHEL/SBOX/MGUN/SHOT. Definition recognition never substitutes
for a current generational instance or permission to execute a role. Weapon item
MT_SHOTGUN and MT_CHAINGUN are distinct from monster MT_SHOTGUY/MT_CHAINGUY.
Items are neither damage sources nor AI owners/targets, and public direct state
entry/raw spawning remain gated. Static state roots have tics−1/no action; death,
respawn and arbitrary item behavior are not inferred from metadata. See the
[exact support contract](COMPATIBILITY.md#original-world-pickups).


## Original armor execution boundary

PlayerDemonArmorV1 adds canonical MT_MISC0=43/DoomEd2018 and MT_MISC1=44/DoomEd2019
as genuine map items, using existing actor ownership and original animation states
S_ARM1/S_ARM1A=802/803 and S_ARM2/S_ARM2A=804/805. Sprites ARM1/ARM2 retain55/56 and
frames0/32769. Registry metadata does not admit other items, bonuses or powers.
ArmorType0/1/2 is the source player's protection class, not an actor ordinal or
instance handle. Its points belong to the existing player, never item metadata.
Old policies retain their distinct role gates. See
[support](COMPATIBILITY.md#original-armor-and-damage-absorption) and
[source investigation](M3G10_REFERENCE_INVESTIGATION.md).
