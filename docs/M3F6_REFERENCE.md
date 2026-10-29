# M3f6 investigation: runtime identity storage and the migration boundary

This investigation precedes implementation. Authority is preserved Linux DOOM
1.10; see [M3f4 metadata](M3F4_REFERENCE.md), [M3f5 execution](M3F5_REFERENCE.md)
and the [identity mapping](ACTOR_IDENTITIES.md). This note records source facts,
the complete consumer audit and the selected design. Support belongs in
[COMPATIBILITY](COMPATIBILITY.md), schemas in [STATE_FORMAT](STATE_FORMAT.md),
commands in [TESTING](TESTING.md), and measurements in the M3f6 evidence record.

## Source findings and classification

Original `mobj_t.type` is `mobjtype_t`; `info` points to `mobjinfo[type]` and
`state` points into `states`. `P_SpawnMobj` stores type/info and the spawn state
without running its action; it draws lastlook before spatial insertion and appends
its thinker. `P_SetMobjState` installs state/timer/sprite/frame before the action,
uses the entered descriptor's next reference after nested callbacks, and removes
on S_NULL. `P_MobjThinker` runs XY/Z before countdown, and a -1 corpse returns
without respawn RNG when respawn is disabled. `P_RunThinkers` follows insertion
links and defers removal. These are source facts and already C-validated behavior,
not reasons to copy C pointer ownership into Rust.

`info.h/info.c`: MT_SERGEANT=12, DoomEd3002 is Demon/Pinky; MT_SHADOWS=13,
DoomEd58 is Spectre; MT_SHOTGUY=2, DoomEd9 is Shotgun Guy. Normal SARG states are
475–495; raise496–501 is metadata only. The source actor has health150, pain180,
speed10, radius30/height56, mass400, COUNTKILL and raise496. M3f4's compiled-C
capture establishes full records; M3f5 runs actual type12/states475–495.

`p_enemy.c`: Look/Chase select descriptor roots; FaceTarget/SargAttack consume
RNG only through admitted branches. `p_inter.c`: lethal damage precedes pain,
corpse setup precedes death entry, Fall later clears SOLID. `p_map.c/p_maputl.c`:
collision uses flags/dimensions and ordered spatial handles; missile/type-special
branches remain excluded. Source12 is not a Vile, boss or skull special case.
`m_random.c` owns separate gameplay/presentation indices. The existing adapters,
GNU arithmetic assumptions, target135 roster, and disabled respawn remain.

| Finding | Classification |
| --- | --- |
| Original enum ordinals, descriptor references, normal/raise graph | Independently observed C metadata |
| Spawn/entry/thinker/RNG/movement/melee/pain/death/statistics chronology | Already implemented and independently validated for the bounded fixtures |
| Native136/aliases, target135, direct-entry whitelist, bounded transactional errors | Rust/fixture policy |
| Reinterpreting a registry slot as an original enum merely because it fits | Invalid namespace assumption; explicitly reject |
| Typed native boundary and named canonical classification of Profile20 guards | Candidate admitted by M3f6; no new gameplay |
| Source IDs stored directly across all profiles; raise, Spectre, players and other excluded contexts | Unsupported/unverified migration or behavior |

## Identity and exposure inventory

The audit searched production code, tests, examples, cross-crate APIs, snapshot
field writers and fuzz targets. Numeric matches in sine/angle tables are unrelated
arithmetic data. Historical fixture numbers in other profiles are intentional;
they must not be globally replaced.

| Original identity / current native identity | Storage and consumers | Exposure, risk, evidence and disposition |
| --- | --- | --- |
| MT_SERGEANT12 / type136 | ActorLife.actor_type; spawning, lifecycle admission, monsters, Demon map-prefix and infighting guards | Public read-only ActorLife, events and DSIM. Changing only storage breaks owner admission. Preserve; qualify via native execution type. C type12 observes the declared source correspondence. |
| SARG475–495 / native1–21 (per mapping) | ActorLife.state, registry roots/next; state entry, tick, callbacks, direct-entry and corpse guards | Public StateId, state-bearing events and snapshots. Numbers are not interchangeable. Preserve; centralize the existing bijection and use typed classification in Profile20 guards. |
| Raise496–501 / no execution mapping | Source registry actor raise root496; native raise0 | Source metadata remains; never map or enter. Changing to canonical actor storage would require separating metadata roots from executable roots. |
| MT_MISC86=136 / overwritten fixture136 | Vanilla registry versus Demon/weapon/projectile/shotgunner/chaingunner profiles | A global136-to12 substitution would corrupt unrelated fixtures and original metadata. Preserve registry/profile selection and full-record checks. |
| Original weapon states1–21 / overwritten Demon aliases | Owned profile registry and cached snapshot metadata | Same integer identifies different records. Same-record M3f4 resolution must continue rejecting substitutions. Preserve registry definitions and serialized names/properties. |
| Source-player-derived unbound target / type135 | ActorLife, perception roster, damage target and body suffix | Explicit fixture adaptation, not a real player. Remains separate; canonical Demon wrapper rejects it. |
| MT_PUFF37/MT_BLOOD38 and states90–96 / original slots | Shared damage/effect spawning and lifecycle | Already original IDs for these admitted effects. No renumbering or wider effect admission. |
| Live object / ActorId(slot,generation) | ThinkerStore, spatial order, target, projectile owner, bindings, events | Object identity is independent of definition identity. Preserve reuse/generation and traversal; compare ordered C-derived traces plus Rust stale-handle tests. |

### Complete consumer dependency audit

| Consumer | Observed dependency and migration disposition |
| --- | --- |
| 1. Allocation/initialization | spawning.rs/lifecycle::raw_actor copy registry type/state, timer, flags and dimensions. Source lookup must precede fixture mapping and draws. No allocation/store change. |
| 2. Ownership/generations | store.rs owns slots, insertion links, pending removal and generations. No numeric type sort; retain ActorId. |
| 3. THING conversion | spawning.rs filters first, resolves original DoomEd record, validates MT_SERGEANT then maps to native owner. Reject selected unsupported types; preserve source angles and batch rollback. |
| 4. Definition lookup | ActorRegistry indexes ActorTypeId/StateId; Original* resolution compares full records. Native136 and aliases are intentionally different records. Keep both APIs distinct. |
| 5. Entry/scheduling | LifeHost enters registry states immediately; next edges and countdown use registry slots. Change storage without changing registry invalidates actions and timers. Typed classification changes neither service. |
| 6. Thinker iteration | thinkers.rs/store.rs traverse handles in insertion order, physics then state countdown. No second scheduler or identity sort. |
| 7. Damage/pain/death/corpse | combat.rs reads actor roots; demon_lifecycle.rs tests native death range and Fall boundary. Profile20 will classify through the checked native state type; older fixture subsets remain frozen. |
| 8. Collision/BLOCKMAP | movement/geometry/multisector/sight reference ActorId, flags and dimensions. List order is observable; no representation migration required there. |
| 9. Target/tracer | ActorLife.target and projectile owner are generational handles. There is no general runtime tracer field or A_Tracer admission. Never reinterpret a target as a type. |
| 10. Movement/perception | monsters/perception/pursuit test profile/owner roles, then use descriptor roots. Native generic fixtures19/21/136 also exist; preserve their separate paths. |
| 11. RNG | Spawn, walk selection, sounds, melee, pain/death consume the owned stream in existing callbacks. Identity conversion must be pure and allocation-free; queries must not draw. |
| 12. Type/state guards | demon.rs, demon_lifecycle.rs, monsters.rs, demon_map_spawn.rs, lifecycle.rs restrict owner, normal closure, COUNTKILL and direct entry. Global source renumbering would bypass/reject these differently. Scope the new named classifiers to Profile20. |
| 13. Raw assumptions | canonical_demon::STATES is the intentional original/native bijection. Other native136 fixtures and early stand/run self loops are not the same source graph. Keep historical constructors/guards unchanged. |
| 14. Internal snapshots | Snapshot is built from named scalar fields and cached registry/map metadata. There is no opaque alternate actor save representation. Keep storage and cached metadata byte-identical. |
| 15. DSIM/replay | snapshot.rs serializes raw type/state and event payloads as registry slots. compare_traces compares fields; no general public savegame/demo decoder exists. Keep DSIM-v20 contract. |
| 16. Fixtures | Historical encoders, full bytes, digests and event streams embed native slots. C-derived identity projection is explicit. Do not regenerate old expectations. |
| 17. Debug/events | Public ActorLife copies and ActorState/selection/pain/kill event payloads expose raw IDs. Existing callers retain signatures and values; native wrappers add explicit conversions only. |
| 18. Examples/helpers | Canonical example observes source identity; old examples/support helpers intentionally use raw aliases. Preserve all old bytes and add a separate asserted boundary example. |
| 19. Cross-crate APIs | doom-map THING.kind is i16 DoomEd, not mobjtype_t; doom-types has numeric/RNG types, doom-wad has no actor identity policy. ActorTypeId/StateId remain registry candidates; no implicit Original* conversion. |
| 20. Fuzz/invalid input | Existing targets mutate raw slots and canonical requests separately. Add a bounded native-boundary target; exhaust u16 candidates and namespace collisions in Rust safety tests. Build versus run counts remain separate. |

## Design comparison and decision before implementation

| Approach | Fidelity/determinism/type safety | Compatibility, memory and maintenance |
| --- | --- | --- |
| A. Direct original runtime storage | Closest ordinal storage, but requires coordinated owner/role/registry/root changes; source raise metadata cannot become execution permission | In-place changes break ActorLife/event/DSIM contracts. A new opt-in profile plus versioned independent encoder could make a future slice defensible. No field growth, but substantial profile/registry/serialization surface. Do not globally renumber. |
| B. Persist canonical plus native execution IDs | Explicit namespaces if independently typed, but two authoritative mutable identities can diverge | Extra actor fields/snapshot policy and synchronization invariants; unnecessary duplicate truth. Reject persistent duplication. A transient typed view is useful. |
| C. Preserve storage with stronger typed projection | Keeps the independently validated correspondence, with namespace rejection and named state policy at the boundary | No persistent memory growth or format/API changes. Reuses one mapping and existing services; smallest complete prerequisite. Selected for M3f6. |

An in-place migration is unsafe: canonical state475 would index a different
runtime record from alias1, original type12 would fail the existing owner and
COUNTKILL proof, and emitted identity/event/registry bytes would silently change
under DSIM-v20. These are concrete consumer failures, not an assertion that an
opt-in migration is impossible. New profile/registry/schema isolation is the next
prerequisite if direct storage is desired. M3f6 deliberately implements the user-
authorized narrower investigation/guard outcome rather than introducing that
larger representation surface now.

Add sealed NativeDemonType/NativeDemonState values produced only through the
validated borrowed binding. They name native execution slots explicitly and
project to canonical source identities; they do not certify a live handle, grant
entry or imply that another registry/profile uses those numbers. Raw candidates
are checked before wrapping. Existing runtime_type/runtime_state/original_state
methods delegate without signature/result changes. A typed native identity query
checks Profile20, owner and state; original_demon_identity delegates to it.

The original/native table remains the sole mapping. Derive a bounded reverse
lookup without allocation, unchecked conversion or a second hand-written table.
Use named original states for Profile20 direct-entry, death and pre-Fall tests.
Keep earlier Demon profiles' narrower fixture policies unchanged. There is no
new per-actor field, capacity, RNG draw, action, scheduler, profile or DSIM version.

## Independent validation boundary

Reuse pristine M3f4 compiled-C records and M3f5 raw canonical traces. A new test
checks the explicit native wrapper against actual stored ActorLife fields and
raw C type/state observations, across all normal states, then asserts existing
independent snapshots and ordered fields. This is new boundary verification over
existing C observations, not new C inputs/functions/branches or raw canonical
runtime storage. Re-run all original generators/encoders and sanitizer checks.

Separate exhaustive namespace, admission, direct-entry/death guards, stale/reused
handles, replay and complete-state-error tests from C comparisons. Preserve every
historical fixture/helper byte. Matching original/native projection alone is not
whole-gameplay equivalence. No DSIM-v21 is needed when no observable format or
stored ID changes; old capture interpretation and decoders stay unchanged.

## Implemented outcome

The implementation selects C. No stored actor/state ID becomes canonical and no
historical profile or format changes. The sealed native views reuse the original
binding and its single mapping; Profile20 uses named classification without
broadening direct entry or corpse behavior. Small boundary methods are inline to
avoid preventable call overhead. Actual measurements, including pre-inline
outliers and final-source results, belong in [the evidence record](validation/M3F6.md);
this annotation is not itself a performance claim.

The next prerequisite for direct storage is a separate, independently encoded
original-ID registry/profile and diagnostic-format contract, tested alongside
unchanged v20 interpretation before changing public runtime observations. Existing
raw canonical C traces can supply its independent input; they do not authorize
new actors, raise, respawn or broader gameplay.
