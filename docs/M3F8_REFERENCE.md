# M3f8 investigation: execution context and target lifetimes

This investigation is committed before runtime edits. It records source findings
and the selected implementation boundary, rather than claims of completed tests.
The preserved Linux DOOM 1.10 source at a77dfb96cb91780ca334d0d4cfd86957558007e0
is historical authority. Existing engineering records and actual Rust code were
read before this design. No new actor, gameplay branch or resource bound is proposed.

## Source findings and classification

- Source fact: p_mobj.c P_SpawnMobj zeroes its allocation, installs type/info and
  the raw spawn state without an action, draws lastlook, links spatial membership
  and appends its thinker. target/tracer/player therefore start NULL.
- Source fact, historically observed: P_SetMobjState installs state/timer/frame,
  calls the action immediately, follows the entered descriptor next reference,
  and removes on S_NULL. A terminal -1 corpse timer does not retire the actor.
- Source fact, historically observed: p_enemy.c P_LookForPlayers/A_Look assign
  ordered roster or shootable sound targets; A_Chase retains its target during
  pursuit and reads health/flags before reacquisition. Null/dead/nonshootable are
  not interchangeable with a pointer to freed storage.
- Source fact, new input candidate: p_inter.c P_DamageMobj returns immediately on
  lethal damage. Otherwise pain consumes RNG and may enter state/action before
  reaction reset and conditional replacement with source (threshold/type/self
  restrictions). Mere explicit pointer assignment consumes no RNG; subsequent
  state/action execution can consume draws.
- Source fact, historically observed; new retention input candidate: P_KillMobj
  changes flags, dimensions, statistics and death state/timer without clearing
  target. A corpse is still the same thinker allocation.
- Source fact, new pending-retirement input candidate: P_RemoveMobj immediately
  unlinks spatial membership, stops sound and marks deferred P_RemoveThinker.
  Neither function sweeps incoming or outgoing target references.
- Source fact: p_tick.c P_RunThinkers follows append order, unlinks marked
  thinkers and calls Z_Free. It subsequently reads currentthinker->next. Original
  z_zone.c Z_Free marks/coalesces zone blocks without releasing the backing heap
  allocation. Replacing this with libc free is not a valid observer adaptation.
- Observer policy: the existing shim quarantines allocated mobj storage until
  case reset and records logical retirement. New evidence must retain this
  adapter, never claim ISO-C freed-pointer safety or allocator/address-reuse
  equivalence, and never dereference a retired gameplay target.
- Unsupported: mobj.target also means missile originator (p_mobj.h and missile
  spawning), while tracer is another pointer. Player mo/attacker relationships
  (d_player.h, p_inter.c, p_user.c P_DeathThink) are separate. Resurrection clears
  a corpse target in its own branch. These facts do not authorize execution here.
- Already independently observed: M3f5 canonical type12/SARG execution and M3f7
  original-ID runtime/DSIM21 use shared physics, movement, collision/BLOCKMAP,
  sight, lifecycle/damage, RNG and thinker services. M3f8 must reuse those paths.

## Ownership and lifetime matrix

| Subject | Original C representation | Current Rust representation | Required observable behavior | Validated / unverified | Dangling risk | Rust invariant | DSIM implications | Independent evidence | Disposition |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Definition | mobjtype_t/info and state pointers | OriginalActorType/OriginalState; DefinitionIdentity | Preserve12 and475–495, separate context137 | Complete metadata and current closure validated; other execution excluded | Definition pointer is not object identity | Never convert definition into live handle | Existing v21 source/context namespaces | Reuse M3f4/M3f5/M3f7 | No storage change |
| Live instance | zone mobj allocation/pointer | ActorId(slot,generation), ThinkerStore | Stable identity and insertion order | Current allocation/reuse validated; C address reuse unverified | Pointer can alias recycled allocation | Resolve exact generation; never redirect stale ID | Existing slot/generation bytes | Rust reuse tests, C birth chronology | Derive availability from existing slots |
| Context | level globals/player arrays | Owned Simulation; ContextRole is a definition role | No cross-world role assumptions | Raw IDs intentionally store-scoped | Equal raw IDs across simulations lack provenance | Typed scoped tokens cannot escape or mix | No persistent context field or schema change | Compile-fail scope tests; local admission policies | Add generative exclusive context view |
| Acquisition | target assignment/Look/player scan | ActorLife.target + validated services | Preserve chosen target/events/RNG | Historical ordered selection validated | Candidate pointer lifetime assumed | Explicit local handle+definition admission, existing target role guards | Existing optional handle/event bytes | New controlled assignment and existing perception | Delegate set_actor_target |
| Replacement / clear | pointer overwrite / NULL | replace Option<ActorId> | No implicit clear or extra RNG | Existing service; new input coverage planned | Old reference need not resolve to clear it | Validate new reference before mutation; allow explicit None repair | Existing Target event order | New script writes and genuine damage retarget | No second target store |
| Damage source | mobj source/inflictor pointers | DamageRequest handles | Pain/retarget chronology unchanged | Shared original-C closure validated | Source lifetime can expire | Validate each scoped token then shared damage preflight | Existing damage/target/action bytes | New source-body retarget input | Reuse DamagePlan and checked operation |
| Corpse | same mobj, flags/state/height changed | Same ActorId and ActorLife | Preserve retained reference and statistics | Death/corpse validated; new targeted retention planned | Death itself is not free | Do not treat health<=0 as retirement | No new field | Target/owner death retention inputs | Observe corpse health/flags; preserve admission |
| Pending removal | thinker function sentinel, unlinked spatial pointers | Entry.pending; immediate spatial unlink | Mark now, retire on traversal | Scheduling validated; new retained-target snapshot planned | Caller can still hold pointer | Pending storage readable, new target admission rejects | Existing pending/list/events | Defined pre-tick C snapshot then explicit repair | No global reference sweep |
| Logical retirement | unlink then Z_Free zone mark | Entry removed; generation advances / exhausted slot sealed | Preserve traversal/event order | Rust generation safety; C logical retirement only | Pointer reuse not modeled/proven | Old generation cannot resolve new occupant | Preserve vacant-slot/generation bytes | New safe C retirement ordering + Rust reuse | Separate retirement from stale/vacant states |
| Retained invalid target | pointer may remain after removal | Raw stored generation retained, actor_target returns None | Read must not mutate or consume RNG | Existing Rust policy; invalid C execution excluded | C dangling dereference unsafe to claim | Explicit unavailable result; no fallback/rebinding | Existing inert target bytes remain | Rust-only rejection/repair/atomicity | Live-owner checked step rejects until explicit repair |
| Roster / sound targets | player.mo/sector.soundtarget | Separate roster/sound handle arrays | Preserve ordered scanning and source noise | Existing synthetic context validated | Removal does not clear every reference | Repair explicit bindings before tick; do not hide stale input | Existing fields unchanged | Existing fixtures plus retirement scenario | No automatic whole-world clearing |
| Player/tracer/projectile | mo/attacker/tracer/originator pointers | Limited separate historical profile services | No false admission | Real-player/general relationships unsupported | Enemy-only semantics would be wrong | Future roles require independent relationship policies | No new executable roles | Source audit only | Future prerequisite, excluded |

## Alternatives and selected narrow boundary

Persisting another per-actor world ID or parallel ownership manager would duplicate
state and require globally deterministic context allocation/serialization policy.
Pointer-address equality would be nondeterministic and allocator-dependent. Keeping
only raw handles would leave cross-context typed misuse unchecked.

Selected candidate: an additive, generative, invariant-lifetime ExecutionContext
borrows one Simulation exclusively. Its private ContextActor tokens carry the
existing ActorId plus a zero-sized invariant brand. A higher-ranked callback
prevents a token escaping or crossing a distinct scope, including nested contexts.
Every operation still checks generation/current definition. Tokens are not liveness
leases. No host address, global counter, allocation, RNG or persistent duplicate
identity is needed for context identity.

Raw ActorId compatibility remains explicit: admitting a raw candidate interprets
it locally with an expected DefinitionIdentity; it cannot infer foreign provenance
from equal raw values. Exporting raw_id deliberately erases the brand. Persistence,
network worlds and cross-world message references need a later explicit provenance
contract; this milestone must not claim that boundary solved.

Pure derived availability distinguishes live, pending removal, most recently
retired occupant before reuse, stale generation, a vacant current-generation
candidate, invalid zero generation and out-of-range slot. It uses checked indexing
and adds no arena field. After reuse the retired token is stale, never another actor.
Queries return copies/typed observations and do not mutate target fields or events.

Target observation distinguishes absent, available context body (including corpse
health/flags), pending/retired/stale/invalid/vacant, and context-incompatible
current definitions. Assignment, replacement, clearing, damage, removal, state
entry, roster repair and stepping delegate to existing validated services. Dead
owner retained references remain inert; living owners with unavailable targets
retain the established checked-step error until explicit caller repair. Duplicate
removal stays idempotent. Unsupported actors/states remain rejected.

## Evidence and schema plan

Add a small set of new C inputs: explicit replace/clear; genuine damage-source
retarget; target death; owner death; a pending targeted body snapshot followed by
explicit clear/roster repair and retirement; replacement after retirement; pending
owner retirement. Preserve source type12/SARG and source body135 adapter, with an
explicit oracle-only context137 projection. No C stale-pointer gameplay or
allocator-reuse equivalence. Reuse the genuine70-body observer rather than adding
an independent engine. Report source fields, draws, statistics, events and captures
separately from Rust safety and fixture encoding.

Keep compact raw rows, a digest index, bounded complete event payload evidence and
two uniquely useful full-byte fixtures with provenance. Expected bytes come from
C rows and independently declared schema inputs, never Rust outputs. Existing
metadata/fixtures/helpers/tools remain immutable. Existing M3f5/M3f7 evidence is
reused regression coverage, not new observations.

No persistent field or event meaning changes: retain DSIM20/21 and policy/schema
identifiers. Typed scope and derived status are observational API policy, not
savegame state or an execution-profile increment. Re-run all historical source/
encoder pairs and new observers, all Rust modes/toolchains/examples/docs, build
all fuzz targets and execute bounded relevant campaigns with disclosed leak0.
Keep matched three-process before/after samples and separately measure ownership
operations; no universal performance claim or isolated allocation claim.

## Review refinements before branching

The scope is an exclusive access boundary, not an aggregate transaction. Each
existing mutation retains its documented atomicity; successful earlier operations
must not be rolled back merely because a later callback operation fails. Document
and test that distinction. Raw handle admission deliberately interprets a candidate
inside the selected local context; equal cross-world raw values cannot establish
provenance and must not be advertised as rejected by magic.

Target self-policy is role-dependent: a Demon cannot target itself because the
existing profile requires a context body. Do not invent a blanket self-target ban
for synthetic bodies or future missile originators. Pending bodies remain readable
but cannot be newly acquired through current gameplay guards. An available corpse
observation is not a proof of combat permission.

Availability classification must distinguish a most recently retired identity in
a vacant slot from an unissued current-generation candidate. After slot reuse,
older generations are stale. At exhausted generation, only that final occupant is
retired; earlier generations stay stale. All results are derived from existing
slot state without a new history table.

Add one small inert-corpse retention input if the C observer can demonstrate it
without gameplay dereference: kill owner, explicitly unbind roster and retire its
target while owner remains in non-target-reading death actions. Quarantined storage
can identify the retained pointer value, but neither real zone reuse nor freed-
pointer safety is proved. Keep that qualification separate from Rust generation
rejection. Other new scenarios retain explicit repair before a live owner runs.

## Source and contract navigation

- [p_mobj.c](../linuxdoom-1.10/p_mobj.c): P_SpawnMobj, P_SetMobjState,
  P_RemoveMobj and P_MobjThinker; [mobj fields](../linuxdoom-1.10/p_mobj.h).
- [p_enemy.c](../linuxdoom-1.10/p_enemy.c): P_LookForPlayers, A_Look, A_Chase,
  missile/tracer and resurrection relationships outside this admission.
- [p_inter.c](../linuxdoom-1.10/p_inter.c): P_DamageMobj/P_KillMobj ordering.
- [p_tick.c](../linuxdoom-1.10/p_tick.c): insertion and deferred thinker retirement;
  [z_zone.c](../linuxdoom-1.10/z_zone.c): zone retirement versus libc free.
- [p_map.c](../linuxdoom-1.10/p_map.c) and
  [p_maputl.c](../linuxdoom-1.10/p_maputl.c): collision candidates and spatial unlinking.
- [p_user.c](../linuxdoom-1.10/p_user.c) P_DeathThink and
  [d_player.h](../linuxdoom-1.10/d_player.h): distinct mo/attacker dependencies.
- [info.c](../linuxdoom-1.10/info.c): pristine definition/state roots, not object ownership.
- [M3f7 contract](M3F7_REFERENCE.md), [identity mapping](ACTOR_IDENTITIES.md),
  [architecture](ARCHITECTURE.md), [support](COMPATIBILITY.md) and
  [DSIM schemas](STATE_FORMAT.md) retain their separate authority.

The branded view is selected after checking the existing ownership/target guards.
It adds a reusable access boundary over the same Simulation and ThinkerStore;
it does not replace their authority. Live actor identity remains ActorId and
original definition identity remains DefinitionIdentity. Implementation and measured
findings will be recorded separately after validation.
