# M3f7 investigation: original-ID registry, execution and diagnostic contract

This investigation is prepared before implementation. Behavioral authority is the
preserved original Linux DOOM 1.10 source; [M3f4](M3F4_REFERENCE.md),
[M3f5](M3F5_REFERENCE.md) and [M3f6](M3F6_REFERENCE.md) establish metadata,
execution observations and the complete consumer inventory respectively.
Measured outcomes belong in the M3f7 evidence record; proposed work below is not
a claim of completed execution or whole-game compatibility.

## Original-C facts and investigation

`info.h/info.c` define MT_SERGEANT12 / DoomEd3002, normal SARG475–495 and raise
496–501. MT_SHADOWS13 / DoomEd58 shares the SARG graph but differs in identity and
flags; MT_SHOTGUY2 / DoomEd9 is a separate actor. Source136 is MT_MISC86, and
source135 is MT_MISC85. Complete original metadata has137 types/967 states/138
sprites, independently observed by the unchanged compiled-C metadata tool.

`p_mobj.h` stores mobjtype_t plus an info pointer and a state pointer. P_SpawnMobj
selects info directly by original type, draws lastlook, installs spawn state without
an action and appends a thinker. P_SetMobjState installs timer/sprite/frame before
calling the action, preserves the entered descriptor's next reference after nested
entry, and removes on S_NULL. P_MobjThinker runs physics before countdown; the
infinite corpse timer does not advance into S_NULL. Disabled respawn returns before
its RNG/movecount branch. P_RunThinkers follows insertion links and defers removal.

`p_enemy.c` selects see/melee/pain-related roots through info. A_FaceTarget,
A_SargAttack, A_Look, A_Chase and movement helpers already execute in the shared
Rust services. `p_inter.c` supplies damage/pain/death/corpse/statistics ordering.
`p_map.c/p_maputl.c` depend on flags, dimensions and ordered spatial handles;
missile/special-actor branches remain outside the Demon closure. Source12 and
old fixture136 do not acquire boss, Vile, skull or player branches. Source target135
and the planned context body are both unbound synthetic bodies, not source players.

The unchanged M3f5 compiled-C observer already executes real type12 and states
475–495 throughout its nine scenarios. It retains one explicit target135/roster
adapter. Its C metadata/body observations justify a new original-ID input slice;
they do not authorize Spectre, raise, players, generalized infighting or Fast.
The original source and observer remain untouched. All newly encoded observations
must be labeled reuse of this dataset, not new C inputs/functions/branches.

## Alternatives and decision

| Alternative | Fidelity and execution | Cost, extensibility and compatibility |
| --- | --- | --- |
| A. Dedicated original-ID registry | Direct original indexes and pristine descriptors; metadata and admitted roles remain separate | No per-monster native translation table. Requires an explicit new profile/diagnostic contract; old profiles stay frozen. Selected registry model. |
| B. Canonical roles using existing execution | Original IDs are authoritative for admitted source actors; shared movement/AI/damage/thinker services execute them | No second engine, actor store, scheduler or persistent duplicate identity. Selected execution mechanism within A. |
| C. Native execution plus external views | Already established by M3f6 | Keep for historical Profile20 only, or use as an explicitly reported fallback if an audited new guard cannot be proved. Do not make it the new registry's foundation. |

Implement an opt-in original-ID profile for the existing Demon closure if all
consumer guards can be qualified without broadening behavior. Do not modify
Profile20 bytes or globally renumber actors. Separate source definition identity,
context definitions, executable roles and live generational handles.

## Registry and role contract

The new immutable registry retains all137 original actor records and all967 state
records at their exact original indexes. No source record may be overwritten or
renamed. Source state and sprite ordinals remain untouched. Full-record validation
certifies the known immutable registry package; type/state recognition alone never
authorizes execution.

A synthetic unbound body must not occupy source135 or masquerade as MT_PLAYER.
Represent it as an explicitly typed context definition, in a separate definition
namespace. A context slot can be appended after OriginalActorType::COUNT in the
existing bounded registry container; its number is derived, not a magic native
Demon/target alias. Canonical conversion of that count sentinel still rejects.
DefinitionIdentity distinguishes Original(OriginalActorType) from Context(role).
The original portion remains pristine and independently checkable, including
MT_MISC85/86. Context descriptors carry their own immutable schema and admission.
This adds no live actor slot or geometry capacity.

The initial execution contract admits only the existing Demon, existing puff/blood
effects, and the bounded unbound body context. Only MT_SERGEANT can enter through
the existing atomic Demon THING batch. Other original types remain metadata.
Validate an actor's executable closure from original descriptor roots and next
references with bounded state sets. Exclude raise explicitly. Validate actions
against the existing admitted capability policy. Future admission adds a policy
for an original actor; it must not need a native state-number translation table.

Direct stand/run/melee entry remains allowed only for live owners. Pain/death
remain damage-controlled. Death and pre-Fall phases derive from death roots and
the original A_Fall association. Null is a transition sentinel, not a live state.
Metadata keeps its true raise reference, without admitting resurrection.

The new simulation profile stores source actor12 and actual source SARG states
475–495 in ActorLife. It stores no duplicate native identity. Context bodies have
explicit context definition slots; they are not canonical original actor IDs.
All actor handles, target handles, generation/reuse, insertion/spatial order,
physics, state entry, damage, RNG and checked transactional planning are shared.
The existing four-owner/64-slot and geometry/arithmetic bounds remain unchanged.

## Complete contract responsibility matrix

| Responsibility | Selected boundary |
| --- | --- |
| Actor/state identity | Private bounded OriginalActorType/OriginalState; explicit definition key for context |
| Registry ownership/lifecycle | One immutable owned registry, shared by existing Arc ownership; no mutable mirror |
| Actor/state lookup | Direct original ordinal indexes, full-record/schema validation |
| Spawn admission | Original DoomEd lookup/filter order; role check then existing batch, no native Demon projection |
| Entry permissions | Root-derived admitted state sets; live direct subset, pain/death controlled, raise excluded |
| Transition resolution | Original next references through the existing LifeHost, no second state interpreter |
| Damage handling | Existing DamagePlan/lifecycle services and original source roots |
| Thinker scheduling | Existing linked insertion order/physics-before-clock/deferred retirement |
| Allocation/handles | Existing bounded ThinkerStore/generations/reuse; definitions never identify instances |
| Targets/lifetimes | Existing generational references and role/lifetime checks; no real player/tracer expansion |
| Metadata/capability | Complete immutable source metadata, separately bounded role admission |
| Unsupported identities | Explicit errors before mutation; no alias fallback or numeric clamping |
| Profile selection | Explicit opt-in enum/constructor, never numeric version inheritance |
| Public observations | Actual original IDs for admitted source actors; context keys remain distinct; old APIs unchanged |
| Snapshot representation | New version only; actual stored identities, ordered events/RNG and handles |
| Version negotiation | Explicit accepted format/profile/registry/context identifiers; unknown combinations reject |
| Historical reading | v20 retains native semantics and full metadata exactly; old helpers remain unchanged |
| Cross-representation proof | Both runs compare against the same independently compiled C dataset |

## DSIM original-ID diagnostic contract

A new version is justified by actual differences: original raw actor/state/event
identities, an explicit context definition namespace, an independently identified
execution profile, and omission of redundant immutable registry records.

Retain the existing bounded-field framing: ASCII DSIM, little-endian u32 format
version, u64 field count; each ordered field is u32 UTF-8 path length/path and
u64 value length/value. Use format21 for this actual contract difference.
This is not a DOOM savegame, demo or executable world-restoration interface.

For v21, execution profile ID1 means the normal bounded original-ID Demon closure.
It is independent of format version21 and of the Rust Profile enum's ordering.
After tic/tic_rate/profile, include registry.schema=`DOOM-INFO-1.10-v1` and
context.schema=`UNBOUND-BODY-v1` identifiers. The registry ID fixes the original Linux DOOM source metadata package;
the context ID fixes the unbound-body descriptor/role. Unknown identifiers and
incompatible profile/registry/context combinations reject explicitly.

All dynamic actor/lifecycle/physical fields, handle encoding (u32 slot/u64
generation), list order, statistics, RNG and event order retain the documented
shared field shapes. Raw source type/state values are original ordinals. A context
slot resolves to a context key, never an original count sentinel. State65535 means
unavailable/removed, not S_NULL. S_NULL remains a valid metadata/transition reference.

Immutable original registry metadata is derived from its identified schema, not
copied into every v21 capture. This includes type/state counts, names, descriptor
fields, action associations and original next references. The context definition
is also fixed by its schema. Keep complete map fields and dynamic observations;
this is no broad evidence-storage redesign. Preserve the original v20 field layout,
registry records and readers byte-for-byte.

Provide explicit bounded encoding/reading for v20 and v21 diagnostic data, with
version-specific identity interpretation. A v20-only reader rejects v21. No decoder
silently casts legacy136 into original MT_MISC86 or a Demon. Reject malformed
framing, unknown/incompatible identifiers, invalid definition/state IDs, duplicate
paths, truncated/overflowing lengths and out-of-policy resource requests. Bound
bytes, field count, path/value lengths and allocations before reserving storage.
Use checked arithmetic and fallible reservations. Reads do not construct or mutate
an executable Simulation. A decoded handle lookup validates generation; stored
unavailable references must never redirect to a reused slot.

The existing trusted Snapshot convenience encoder retains its existing API;
the new contract exposes a bounded Result-returning encoder for explicit budgets.
Document that decoding is structural/identity validation and observation, not proof
that arbitrary physical state can safely resume execution.

## Independent verification and evidence size

Reuse immutable M3f4 metadata and M3f5 raw canonical C observations. Do not duplicate
large traces or regenerate historical outputs. Encode new v21 expectations directly
from raw C source12/SARG rows. The only necessary context identity adaptation maps
the observer's synthetic body135 to the separately declared context definition;
this is neither a source actor mapping nor new C behavior coverage.

Use the unchanged independent field/event encoder as bounded observation machinery
where appropriate; remove native Demon identity projection and immutable registry
serialization for v21, and add the explicit profile/schema fields. Expected output
must never read Rust execution/source definitions. Retain source/tool/input/output
hashes and a deterministic regeneration/check command. Keep a compact per-capture
index and a small number of complete byte fixtures, avoiding another large event
or raw-trace copy.

Compare actual raw original ActorLife fields to C, including all normal states,
RNG indexes/draw-dependent outcomes, physical/lifecycle fields, statistics and
chronology. For exact ordered event comparisons, reuse the existing C-derived v20
event field stream, replacing only identity payloads using their actual raw C
observations plus the declared body-context binding. This test-only historical
reader is not a new runtime translation table. Label any digest-only comparisons
and full-byte checks distinctly. Re-run every historical generator/encoder,
metadata/source sanitizer check, test/example and relevant bounded fuzz campaign.

## Adapter discipline and removal criteria

| Boundary | Disposition |
| --- | --- |
| Native136, state aliases, target135 in old profiles/fixtures | Intentionally permanent historical contracts/decoders; untouched |
| Original definition key to registry slot | Necessary checked lookup; source keys are direct ordinals, not a translation table |
| Original role to shared execution services | Necessary admission boundary, not another engine implementation |
| Unbound synthetic body context | Explicit experimental/test context; replace in a future new profile only after real game context is independently admitted; preserve this schema historically |
| Old C observer body135 to context key | Temporary oracle adapter, isolated to evidence readers/encoders; remove from future new observers with explicit context roles, retain old evidence |
| DSIM version/profile/schema dispatch | Necessary permanent diagnostic compatibility boundary; separate from simulation mutation |

## Validation and reporting plan

Check complete source metadata, root/graph/action admission, original/native
namespace separation, invalid/unsupported IDs, registry/profile mismatches,
pain/death/raise guards, stale/reused handles, deterministic order/replay,
malformed diagnostics, unsupported versions/schemas, bounded allocation and stable
serialization. Compare v20 unchanged plus v21 independently against C. Run all
workspace/MSRV/release/format/Clippy/rustdoc/example/reference gates and fresh CI.
Build all fuzz targets; execute bounded registry/diagnostic campaigns with exact
counts/options and disclose disabled leak detection. Measure matching old workloads
before/after; identify new-profile measurements separately because diagnostic
contents differ. Retain all three-process samples/outliers, RSS and limitations.
