# M3g1 investigation: diagnostic-free stepping and transaction costs

This investigation is committed before runtime edits. Preserved Linux DOOM 1.10
at a77dfb96cb91780ca334d0d4cfd86957558007e0 is behavioral authority. M3g1
changes observation placement, not admitted gameplay, ownership or diagnostic schemas.
[COMPATIBILITY](COMPATIBILITY.md) owns API guarantees, [STATE_FORMAT](STATE_FORMAT.md)
the schema, [TESTING](TESTING.md) reproduction, and validation/M3G1 the measurements.

## Original source and evidence boundary

[p_tick.c](../linuxdoom-1.10/p_tick.c) P_Ticker returns on paused, then visits active players in slot order,
P_RunThinkers, P_UpdateSpecials, P_RespawnSpecials and finally increments leveltime.
The source also has a menu/demo pause guard; no menu/demo or complete player
execution is added here. There is no full named diagnostic snapshot in that sequence. P_RunThinkers follows
insertion links and deferred removal, with the source zone-allocation assumptions
documented in M3f8; this change does not reinterpret dangling C pointers.
[p_mobj.c](../linuxdoom-1.10/p_mobj.c) P_MobjThinker supplies physics-before-state countdown; P_SetMobjState
installs a state and immediately invokes its action. [p_enemy.c](../linuxdoom-1.10/p_enemy.c) action decisions,
[p_inter.c](../linuxdoom-1.10/p_inter.c) damage/death and
[m_random.c](../linuxdoom-1.10/m_random.c) gameplay draw ordering remain unchanged.
These are source readings and previously observed behavior, not new C coverage.

The existing independently compiled C observers and independent DSIM encoders
remain the expected-output authority. Old step-based comparisons will execute the
same new core. Explicit advance-path checks additionally replay frozen ownership
inputs against those observations. Complete snapshot equality is a Rust API
refinement test, not an independent gameplay oracle. No new C input, source body,
branch coverage, raw trace, gameplay profile or DSIM version is necessary.

## Complete stepping and allocation inventory

Locations below name functions rather than unstable line numbers. Allocations
listed are source-established possibilities; actual counts belong in the measured
record, and depend on current actors/actions/events and retained capacities.

| Consumer | Current work / ownership | Copies or allocations | Selected disposition |
| --- | --- | --- | --- |
| lib.rs Simulation::step | Pause before overflow/buttons, profile-specific preflight, one ordered mutation core, then snapshot | Core work plus full dynamic capture on every successful call, including pause | Keep return type/behavior; delegate to advance and capture once |
| demon_expanded_map.rs step_expanded_demons | Shared Profile20/OriginalDemon core with preflight and staged commit | Bounded mutable projection and events, then capture | Return unit from private core; retain complete transaction |
| ExecutionContext::step | Delegates Simulation::step, maps SimError | Same capture, no additional engine | Preserve; add matching advance delegation |
| GameInstance::with_execution_context | Borrows existing bound context/key | No new per-tic provenance allocation/lookup | Expose advance through the existing context, no second owner API |
| lib.rs run_commands | Maps each input through step and collects snapshots | Snapshot collection plus each dynamic capture | Preserve diagnostic API exactly; ordinary caller loops advance |
| Simulation::snapshot / snapshot::capture | Dynamic ordered field vector, names, value buffers; actor/spatial/sector/player/event projections | Vec growth, format strings, optional IDs, per-sector actor lists, serialized last/pending event payloads | Keep explicit in optimized/debug builds, omit from advance |
| snapshot.rs map_fields / registry_fields | Setup-time immutable map fields; historical typed registry fields appended to map cache | Initial construction only, then Arc sharing | Do not misattribute cached registry/map construction to each tic |
| OriginalDemon capture | Dynamic registry/context schema fields; immutable canonical registry is separate | Small schema values plus ordinary dynamic capture; no historical full registry projection | Preserve v21 field/namespace distinction |
| Snapshot::canonical_bytes | Frames all dynamic and cached static fields | New byte Vec and growth; copies full encoding | Separate timer/count from capture |
| Snapshot::digest | Calls canonical_bytes then FNV fold | Full encoding allocation even when only digest is returned | Preserve; exclude final digest from stepping timer |
| Last/pending events | Between-tic pending appended before phases; consumed by take only on successful unpaused tic | Event Vec growth, nested sight/trace vectors/boxes; old last buffer is replaced | Recording remains enabled on both paths; advance is not event-free |
| Actor thinker traversal | Same thinkers::run, handle generation, insertion and spatial order | Query scratch/events depending on profile | One implementation; no alternate scheduler |
| Paused / rejected calls | Pause ignores commands/overflow; errors retain canonical state including queues/RNG | Old paused step captures; validation may allocate transient scratch on rejection | Advance pause returns unit without capture; retain precedence/atomicity |

Snapshot::capture shares sim.map_fields by Arc clone. It does not call
canonical_bytes; event payload serialization is nevertheless part of capture.
The historical measure_demon_expanded_map snapshot100_us timer includes BOTH
capture and encoding. Its label must not be reported as pure capture or encoding.

## Transaction inventory and migration boundary

| Source | Plan / preflight | Work copied or allocated | Safety role and disposition |
| --- | --- | --- | --- |
| combat.rs DamagePlan | Copyable request/health/death/state/demand/credit scalars | Small scalar plan; PreparedAttack additionally owns attack/intercept result vectors | Shared preflight before health/RNG/statistics mutation; retain |
| monsters.rs MonsterShotPlan / prepare_monster_shot | Current owner, staged RNG, angle/damage and prepared attack | Owned aim/attack observations; single_attack clones the shot plan and aim for event ownership | Preserve validated action order; future smaller ownership plan requires separate evidence |
| pursuit.rs preflight_pursuit_step | Validates current owner/target/map/action dependencies | Registry clone in validate_registry is construction-time metadata projection, not per-tic world cloning | Keep validation; do not claim setup clone as tic cost |
| perception.rs ScanPlan / preflight_perception_roster | Ordered target/lastlook decisions and reachable sight work | Scan event Vec, sight visits and traversal scratch; registry graph checks can allocate sets/vectors | Events and source order remain observable; retain |
| demon.rs MeleePlan | Owner, initial RNG, range acceptance and optional shared DamagePlan | Bounded scalar fields; clone is a small plan, not a world | Retain exact preview/actual draw checks |
| lifecycle.rs demand_entry / spawning.rs preflight_actor_step | Reachable state/action work and spawn demand before mutation | Bounded graph sets/vectors and fallible store/spatial reservation; capacity is not serialized | Retain validation and error classification |
| demon_expanded_map.rs preflight_expanded_demons | Statistics/sector ownership, linked membership, actor bounds/location/life, roster/owner validation | BTreeSet membership plus existing action/state query validation | Measure separately; no change to admission |
| demon_expanded_map.rs demon_plan | ThinkerStore slots/head/tail/max_slots, spatial order, RNG, stats, bindings, new event buffer | try_reserve_exact copies all logical allocated slots INCLUDING vacant slots; spatial reserve uses max_slots while copy uses current order length | Bounded actor projection, not full-world clone; preserve |
| checked_demon_operation / checked tic | Execute shared LifeHost/thinkers against projection; inspect SightControl failure then commit once | Staged actors/spatial/RNG/stats/bindings/events; borrowed map, sectors, immutable registry, roster and sound targets | Late checked sight/work failure means in-place mutation is not proven infallible; preserve rollback in release too |

The normal advance path still records events and may allocate preflight scratch,
staging buffers and gameplay observations. Removing diagnostic capture does not
establish zero allocations. No plan, allocator strategy or error contract changes
are selected for M3g1. A future optimization can investigate reusable transient
scratch or eliminating proven redundant projection work, but must handle stale
capacity, failed operations, event ownership and late failures without adding
persistent identity or diagnostic fields. An unchecked in-place release mode is
not justified by this audit.

## Selected API and validation design

Simulation::advance owns the existing exact stepping body and returns unit. A
successful unpaused call completes one tic; a successful paused call is a no-op.
Simulation::step calls advance and then snapshot. The checked-Demon private body
returns unit. ExecutionContext adds only delegation. run_commands, event APIs,
Snapshot, explicit capture, byte encoding, digests and first_difference stay intact.
No wall clock, test feature or host instrumentation enters authoritative gameplay.

A test-only thread-local capture counter at the capture boundary supplies a
structural regression guard: advance performs zero implicit captures on success,
pause or error; step performs one on success/pause and zero on error. It adds no
production state, API or RNG. Complete per-tic snapshots and canonical bytes are
compared across all historical profiles and OriginalDemon, including errors,
pause/overflow, RNG wraparound, lifecycle/retirement/reuse and scoped/provenance APIs.
Frozen C observations remain separate expected outcomes.

Matched three-process release measurements retain compiler/workload fingerprints,
individual timing/RSS samples and outliers. Existing step benchmarks are kept
unchanged as before/after controls. New measurements isolate step, advance,
advance+snapshot, frozen-state capture, and prebuilt-snapshot encoding. Private
transaction instrumentation separates preflight and staging without a public
benchmark backdoor. Actual allocation counting is attempted with external host
tooling, never an unsafe Rust allocator or a production dependency. Counted request
bytes are not peak live heap; whole-process RSS has a separate meaning. CI checks
benchmark execution and deterministic structural invariants, with advisory times
and no shared-runner speed threshold.
