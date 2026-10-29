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
resolution. Fixture type136 is not silently relabeled MT_SERGEANT, and fixture
state aliases do not become canonical merely because their numbers are in range.

The canonical views reuse the static source table. Owned registries and runtime
actors retain their existing storage. Queries neither allocate nor consume RNG,
change thinker/spatial ordering, enter states or change snapshots. Construction
and simulation retain all existing capacities and failure policies. There is no
new profile or diagnostic simulation schema; see [STATE_FORMAT](STATE_FORMAT.md).

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
projection), and raise=0. The source record remains unchanged with raise496.
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
type12/states475–495 throughout gameplay. Comparisons distinguish physical source
fields, this definition projection, independently projected DSIM bytes and
Rust-only rejection policy.
