# M3f9 investigation: game-instance provenance

This investigation precedes runtime implementation. Preserved Linux DOOM 1.10
is behavioral authority; its pointer architecture is not a Rust ownership policy.
[M3f6](M3F6_REFERENCE.md), [M3f7](M3F7_REFERENCE.md) and
[M3f8](M3F8_REFERENCE.md) establish the identity and lifetime foundation. This
note owns the audit and design. Support belongs in COMPATIBILITY, schema in
STATE_FORMAT, commands in TESTING, and measured results in validation/M3F9.

## Original source and independent evidence boundary

P_SpawnMobj/P_SetMobjState/P_MobjThinker in p_mobj.c use definition/state pointers
and one execution context's mobj allocations. Target/source relationships in
p_enemy.c/p_inter.c and spatial references in p_map.c/p_maputl.c are local pointers.
P_RunThinkers in p_tick.c supplies insertion traversal and deferred retirement.
Death preserves an allocation; retirement is different. These findings and the
bounded inputs are already independently validated in M3f5/M3f7/M3f8.

Additional source reading: p_setup.c P_SetupLevel releases PU_LEVEL allocations
through Z_FreeTags and reinitializes thinkers before loading/spawning the level.
p_saveg.c P_ArchiveThinkers copies native mobj records, converting state/player
pointers to indexes; P_UnArchiveThinkers allocates new records, reconstructs
state/info/player links and explicitly clears target. Those routines provide
neither portable global actor identity nor a trustworthy external provenance
protocol. They are source readings, not new compiled observations or implemented
savegame compatibility. Do not execute dangling-pointer continuation or emulate
native pointer serialization in Rust.

The new contract is Rust policy. No new C gameplay scenarios are required because
it adds no gameplay decision, actor state or event. Re-run all existing observers
and encoders unchanged; independently compare the selected existing C replacement/
clear scenario through imported tokens. Its source inputs and expected outputs
remain reused evidence, not new C coverage. No new raw trace is necessary.

## Provenance dependency matrix

| Representation / consumer | Namespace and owner / scope | Validation and failure mode | Deterministic or serialization exposure | Required disposition |
| --- | --- | --- | --- | --- |
| ActorId | u32 slot/u64 generation in one ThinkerStore | Checks generation; equal values in different stores still resolve locally | Targets, events and DSIM carry local values | Preserve; explicit erasure/readmission stays local |
| ThinkerStore / ThinkerContext | One owner's slots and linked traversal | Stale/retired checks; generation exhaustion seals slot | Lowest reusable slot and append order are observable | No store/layout/scheduler change |
| Simulation | Owned actors/RNG/map/registry; derives Clone | No persistent instance provenance | Equal deterministic states and IDs are intentional | Keep Clone as state copying, never identity copying |
| ExecutionContext | Exclusive generative callback over one Simulation | Invariant brand prevents escape/cross-scope token use | Brand is absent from state/snapshots | Reuse, with an optional borrowed boundary identity |
| ContextActor | ActorId plus zero-sized invariant brand | Current generation/state checked on each access; not a lease | raw_id explicitly erases brand | Preserve representation and lifetime guarantees |
| DefinitionIdentity / OriginalExecutionRegistry | Original type or synthetic definition role; immutable package | Definition recognition is not actor ownership or execution permission | Type/state/schema meanings remain versioned | Require expected definition on every qualified import |
| Actor allocation / spawn | Existing local owner and deterministic free-slot policy | Bounded spawn/role checks before commit | Spawn IDs, lastlook draw and events | No provenance allocation or lookup per spawn |
| Retirement / target retention | Existing pending flag/generation and stored local target | Pending readable, retirement/reuse unavailable; no incoming sweep | Old target may remain inert in DSIM | Import rechecks availability, never rebinds stale generation |
| Target acquisition / replacement | Local actor relationship and admitted role | Current service validates target before mutation | Target event and subsequent RNG chronology | Qualify at boundary, then delegate existing service |
| Damage source / inflictor | Separate local references | Existing preflight and atomic damage plan | Damage/pain/retarget/death order | Import each external reference independently; no new damage path |
| Collision / sight / movement | Local spatial/thinker handles and geometry | Existing arithmetic/ordering/role checks | Candidate/order/RNG observations | No boundary token in hot-path spatial records |
| Public actor/identity observations | Actor copies/references or definition views | Local generation/definition only | Read-only diagnostic fields | Read-only underlying Simulation remains explicit local API |
| Raw setters/spawn APIs | ActorId candidates in selected Simulation | No cross-world inference from numbers | Existing signatures and semantics | Retain; owning wrapper exposes no mutable Simulation escape |
| Snapshot / replay comparisons | Named deterministic fields and local IDs | Equality proves state equality, not same incarnation | Canonical bytes and digests must stay identical | Keep runtime incarnation entirely outside Snapshot |
| DSIM20 | Native profile/registry IDs and local handles | Historical reader validates its namespace | Frozen historical bytes | Preserve all fixtures; decoding cannot create provenance |
| DSIM21 / Diagnostic::actor | Original/context identities and local handles | Bounded diagnostic validation; not world restoration | Registry/context schemas are definitions, not instance IDs | Preserve; external reference with missing provenance rejects |
| Fresh construction | New owned Simulation, even with same map/state | Equal raw IDs can represent distinct actors | Must not alter spawn/RNG/snapshot | New wrapper always creates distinct opaque incarnation |
| Restart | Existing restart_demon_map_context builds fresh state with RNG continuity | Old simulation remains unchanged if setup fails | Existing reset/spawn chronology remains | Successful wrapper restart replaces state and incarnation together |
| Clone / deterministic replay branch | Simulation::clone copies local state | Copying token with state would create false ownership | Cloned snapshot equality is expected | GameInstance is not Clone; wrapping a state clone creates new identity |
| Move / raw extraction | Whole owning wrapper moves; extraction consumes it | Moving is not restart; rewrapping must be fresh | No game-state changes | Identity follows move; into_simulation drops wrapper ownership |
| Example/test helpers | Existing raw fixtures and assertions | Deliberately local adapters | C-derived expectations must remain immutable | Reuse public fixtures; new evidence is policy/reused C only |
| Fuzz / malformed candidates | Bounded local IDs and optional opaque provenance | Missing/foreign key, invalid generation/slot, wrong definition reject | Failed import/export cannot change snapshots or RNG | New bounded model; historical diagnostic fuzz remains separate |
| Future serialized/transport reference | No existing trusted instance resolver | Numeric tag or decoded local handle cannot prove origin | Neither DSIM format is a transport/savegame | Reject missing provenance; no byte-to-instance constructor/loader |

All raw-handle consumers were searched across crates, examples, tests and fuzz
targets. doom-map owns THING/editor identities, not actor-instance provenance;
doom-types owns numeric/RNG types; doom-wad owns bounded resources. There is no
other persistent actor-reference representation or executable savegame loader.

## Alternatives and chosen boundary

| Approach | Correctness / determinism | Cost and maintenance | Decision |
| --- | --- | --- | --- |
| A. Opaque runtime instance token | Exact allocation identity distinguishes simultaneously retained incarnations; never gameplay state | One small allocation per wrapper/incarnation, shared token references | Selected for in-process uniqueness |
| B. Numeric session/incarnation epochs | Requires a uniqueness authority, collision/exhaustion and restore rules; caller-chosen equal numbers alone fail | Global counters or external session management add unrelated policy | No numeric/session/persistent key in this milestone |
| C. Explicit external reference envelope | Qualifies local generation without changing actors; imports still validate liveness/definition | Bounded token+handle value; no second store | Selected in-memory envelope; deliberately not serializable |
| D. Strictly local / reject unsupported boundary | Existing scope already solves temporary access; persisted bytes lack trusted provenance | Retains historical APIs and formats | Selected for raw/serialized boundaries without an opaque token |

Use GameInstance as an owning boundary over one unchanged Simulation, with one
private Arc-backed incarnation identity. InstanceId compares Arc identity, never
its contents, address-as-number or a mutable global counter. A retained identity
keeps its allocation distinct from a newly created one. Its Debug output is opaque.
It contains no Simulation/actor reference, so an exported reference retains neither
the world nor an actor lease. Identity is process-local and not meaningful after
all tokens are gone; it is not a persistent globally unique identifier.

GameInstance exposes an immutable Simulation view and explicit consuming extraction,
not &mut Simulation: arbitrary replacement behind an existing identity must be
impossible. It is not Clone. Wrapping a cloned/extracted/restored-in-future raw
Simulation always creates a new identity. Moves preserve identity. Successful
restart calls the existing fresh constructor/spawn service and installs a new
identity only after success; failed restart preserves old state and identity.
Current scope admission remains OriginalDemon-only, even if the wrapper owns an
older profile. Wrapping a state does not authorize unsupported execution.

ExecutionContext borrows an optional InstanceId. Existing raw Simulation scopes
remain unbound and preserve their exact API/lifetime semantics. GameInstance's
callback supplies its identity and the same context/actor/service types. A checked
export verifies current actor state before cloning only the small identity token.
Import checks bound context, present/matching provenance, then the existing exact
generation/state/expected-definition admission. No token is a liveness lease.
Pending storage remains observable; current target/action guards still reject
unsupported mutations. Dead storage is not treated as retired merely due to health.

Reference candidate construction is explicit: optional opaque provenance plus
ActorId. Missing provenance is representable and dynamically rejected. Foreign
or corrupted routing to another opaque identity rejects before actor lookup.
Zero/out-of-range/stale/vacant/retired candidates use existing availability errors.
Wrong/unsupported expected definitions reject through current identity policy.
An unbound Simulation context rejects provenance import/export explicitly. Errors
remain separate provenance versus existing ContextError/SimError classifications.

### Trust, serialization and resource limits

Possession of the local identity permits explicit local candidate readmission;
it is not authentication, authorization of gameplay roles, secure networking or
proof that a malicious in-process caller told the truth about an erased raw value.
There is no numeric/byte constructor for InstanceId. A serialized local handle,
diagnostic stream or map/registry/state digest cannot manufacture one. Existing
DSIM decoding followed by missing-provenance import rejects. No save restoration,
remote resolver, networking or DSIM22 is implemented. Future persistence must
specify a separately trusted instance resolver and fresh-incarnation policy.

Opening a scope borrows the token; import checks it in constant time before existing
bounded actor admission. Export/reference cloning increments Arc ownership and
does not allocate another identity. Only wrapper creation/successful restart adds
one small key allocation, subject to the existing process-level allocation-failure
policy. No allocation counter is claimed unless measured. No new per-actor field,
global counter, RNG draw, wall clock, OS randomness, actor capacity or geometry
limit is needed. Token identity/refcounts never enter deterministic decisions,
snapshots, events or DSIM. Rejected imports/exports are pure; successful earlier
gameplay operations in a callback retain M3f8's nontransactional-scope contract.

## Validation and next gameplay boundary

Test equal local IDs in independent same-map worlds, same-instance across scopes,
restart after retirement and matching raw IDs, moves/extraction/state clone,
pending/corpse/retired/stale/vacant/invalid actors, missing/foreign provenance,
expected-definition errors, direct state/target guards, rejected-operation purity,
replay, exact snapshot/RNG equality, and unsupported profile/restart atomicity.
Inspect compile-fail diagnostics for escaped scoped tokens and cloning the wrapper.
No nonexistent savegame API is treated as tested functionality.

Keep all old observers/encoders/fixtures and all runtime modes/toolchains/examples.
Run new bounded provenance fuzz plus existing registry and diagnostic targets;
report built versus executed targets and leak0 limitations. Retain quiet matched
three-process before/after measurements from all four existing tools and a separate
import/export benchmark. Raw lookup and full validated import are different work.

After this contract, recommend a bounded original-player mo/attacker and damage-
ownership admission milestone, with source-derived health/death prerequisites,
through the same owner. Do not automatically add another identity layer or admit
that gameplay here. Production panic paths/named flags and historical evidence
storage efficiency remain separately scoped maintenance work with measurable
acceptance criteria; this milestone does not perform a repository-wide cleanup.
