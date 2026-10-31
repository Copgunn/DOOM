# Original-C references and independent snapshots

The harnesses compile genuine bodies from the preserved Linux DOOM 1.10 source.
Synthetic adapters supply explicit geometry, actors and unsupported-service
boundaries; they do not constitute original whole-game execution. Compiled
function inventory is not branch coverage.

## Reproduction and ownership

[TESTING](../../docs/TESTING.md#original-c-and-independent-canonical-bytes) owns
the commands and host requirements for reproducing all source/encoder pairs or a
selected slice. This guide describes harness organization and adapter assumptions.

Generators read original C/public inputs, never Rust output. A deliberate fixture
regeneration omits `--check`; review the source, adapter and output differences
before accepting it. Preserve all historical generators, encoders, helper bytes
and fixtures. [Testing](../../docs/TESTING.md) lists Rust comparisons and examples.

## Evidence layout

- `generate.py` / `snapshot_goldens.py`: original foundations and DSIM-v1.
- `generate_<milestone>.py` / `<milestone>_snapshot_goldens.py`: subsequent source
  closures and independent versioned DSIM encoders; [STATE_FORMAT](../../docs/STATE_FORMAT.md)
  owns the schema definitions.
- `tests/reference/`: synthetic maps, exact C rows, input tables, provenance,
  complete byte streams and per-tic digest indexes.
- `tests/support/`: Rust readers/adapters for those public inputs and observations.
- [Source investigations](../../docs/README.md): behavior, scope and safe differences.
- [State format](../../docs/STATE_FORMAT.md): fixed encodings and event chronology.

Provenance records source/body/adapter hashes, compiler flags, ABI assumptions and
output hashes. Signed shifts or unspecified evaluation order need explicit adapter
policies; they cannot silently become claims about all original C executions.
C source comparisons, fixture projection, independent encoding and Rust safety
policy tests are reported separately.

## M3d3j map context

The generator compiles genuine bodies for the existing load/spawn/movement/
combat/thinker/RNG closure. Original P_SetupLevel initialization fragments are
extracted verbatim and hashed separately; the complete level loader is not compiled
or claimed. Original structs/tables are used. Unsupported runtime callbacks abort.

A separate unadapted source-spawn pass retains type 12, original states/flags and
map totals. The gameplay pass maps identity/state IDs to the explicit native136
fixture while retaining original COUNTKILL flags. C observes available kills,
unbound single-player fallback credit, corpse scheduling, source relationships,
reset and continuing RNG. Separate credit components include actual
player-pointer/network branches as source observations only; Rust does not admit
those subsystems. Filter components distinguish admitted original
filters from recoverable Rust rejection policies.

The independent v17 encoder uses only C observations, source metadata, public
inputs and the documented schema. It supplies complete bytes and per-tic digests;
Rust comparisons identify the first differing field when full fixtures exist.
Head-of-list retirement belongs inside the thinker phase even when it precedes
the first live Think observation. This is covered by the corpse-removal fixture.
No commercial assets or network services are used.

For exact observed counts and sanitizer limits see
[M3d3j evidence](../../docs/validation/M3D3J.md). Earlier implementation and
provenance details remain in the complete
[historical tooling guide](../../docs/validation/archive/pre-m3d3j-reference-tools.md).

## M3f1 map scale

The new generator compiles genuine geometry, movement, original thinker and sight
bodies under a declared named-field synthetic ABI adapter. `P_LoadBlockMap` executes
with bounded fixture-backed cache/allocation services; remaining geometry/seg
initialization is a public-input adapter, not the complete original map loader.
The terminal synthetic mobj state admits unbound physics only. Unsupported
state/respawn/player/damage/special callbacks abort if reached. Compiled body
inventory must not be described as coverage of every source branch.

Raw classic fixtures cover larger line/sector counts, sparse/dense/shared/suffix
lists, ordering, openings, BSP boundaries, REJECT and multi-tic motion. Rust compares
actual C fields and ordered observations. The independent encoder composes the
untyped schema under the new tag; the two-actor ordering case remains a C comparison
and is explicitly outside its full-state capture set. Bounds, stale handles,
atomicity and unsupported API rejection are separate Rust policies.

[TESTING](../../docs/TESTING.md) owns reproduction commands;
[M3F1_REFERENCE](../../docs/M3F1_REFERENCE.md) owns source findings and
[measured evidence](../../docs/validation/M3F1.md) owns observed comparison counts,
performance, sanitizer settings and limitations. Historical tools/fixtures/helpers
remain unchanged; the new scripts run alongside frozen aggregators.

## M3f2 expanded extents and spatial work

`m3f2_reference.py` reuses the declared extraction/geometry fixture services while
compiling genuine original physics, ordering and sight bodies. It generates new
classic inputs and comparison tables; it never reads Rust outputs. The named-field
ABI/terminal state is an explicit unbound projection, not complete original actor
structures, P_SetupLevel or P_GroupLines. Unsupported callbacks abort. Compiler,
source, adapters, exact inputs and outputs are hashed in its provenance manifest.

Original sight expressions are instrumented once to record full node/leaf/seg,
side, opening, fraction, slope and blocked chronology. Repeated-child fixtures
count actual full expansion. Conventionally empty and shared/suffix lists retain
the parser's leading-zero rule; original iteration processes that zero as line0.
A separate primitive table probes near-extreme fixed inputs and partition products.
The optional signed/division/cast audit is scoped explicitly, not a claim of all
undefined-behavior or branch coverage.

`m3f2_snapshot_goldens.py` independently composes the untyped canonical layout
under tag19 from C physical/event chronology and synthetic map bytes. Source
predicates define diagnostic vertical contact bits. It excludes the two-actor
script from complete-state encoding while Rust still compares that script directly
to C. `summarize_m3f2.py` counts actual records; seg-only assertions overlap the full
sight trace and must not inflate unique observation claims. Bounds, capacities,
identity reuse and atomic rejection remain separately labeled Rust policies.

[TESTING](../../docs/TESTING.md#expanded-map-extents-and-spatial-work) owns commands,
[M3F2_REFERENCE](../../docs/M3F2_REFERENCE.md) owns the arithmetic design, and
[the evidence record](../../docs/validation/M3F2.md) owns observations and limitations.
All earlier tools, discovery inventories, encoders, helpers and fixtures stay intact.

The separate boundary component retains a defined original intercept of INT_MIN
and its source side decisions. It has no full-sight expected result: the next
original clipping division invokes abs(INT_MIN). Profile19 rejects that query
atomically; old profiles retain their numeric policy. The optional expected-failure
probe explicitly demonstrates the excluded source operation. Standard checked modes
and the independent encoder do not silently invent a result for undefined C.

## M3f3 composed Demon gameplay

The main M3f3 writer reuses unchanged original-body extraction and instrumentation
inside a temporary directory, with a separately hashed table/geometry adapter.
Only new paths are published. Historical `generate*.py` discovery and expected
bytes remain frozen. The clipping adapter uses the same genuine bodies with
additional non-cardinal opening/closed-sight inputs; do not count those bodies as
another set of unique source functions.

Both compile real actor structures, metadata, THING/spawn, thinker, AI, movement,
sight, damage and lifecycle bodies. Named-field geometry, native136/state aliases,
unbound135 roster projection and two setup fragments remain explicit adaptations.
This is not full original level/player setup or function/branch coverage.

The new profile's source-defined signed-angle interval is separate from older
GNU-wrap observations. The expected-failure angle component copies the original
expression verbatim and demonstrates its excluded overflow. The independent
DSIM-v20 encoders consume C rows/public bytes/schema only; the main adapter's
Rust-only capacity bytes are counted separately. Each encoder also writes an
`*_events.bin` projection: all ordered event fields for every capture, compared
byte-for-byte with named-field diagnostics. Its `EV20` header is followed by
repeated u32 case/operation/field counts and the documented DSIM name/value
records, including both event-count fields. This is distinct from sampled full
DSIM streams and recorded C row counts. Source visits, raw C draws, Rust
events/indexes and statistics are qualified in the evidence.

[TESTING](../../docs/TESTING.md#expanded-map-demon-gameplay-composition) owns commands;
[source findings](../../docs/M3F3_REFERENCE.md) own the composition proof;
[observed evidence](../../docs/validation/M3F3.md) owns counts/results.

## M3f4 canonical identity observation

`m3f4_reference.py --check` preserves the separately versioned DOOM-IDENTITY-v1
capture of every original enum symbol, actor/state field, action association and
sprite resource name. Add `--sanitize` for ASan/UBSan; disclose any disabled leak
detection. Original action stubs abort: this is metadata, not callback behavior.
See [investigation](../../docs/M3F4_REFERENCE.md),
[mapping](../../docs/ACTOR_IDENTITIES.md) and [tests](../../docs/TESTING.md#original-actorstate-identity-metadata).
All historical tools, table generation and DSIM fixtures remain unchanged.

## Canonical Demon execution binding (M3f5)

[m3f5_reference.py](m3f5_reference.py) reuses immutable M3f3 extraction and public
geometry construction, removes the Demon setup/table and spawn-ID adaptations,
and executes pristine MT_SERGEANT/SARG records throughout the same genuine source
bodies. Only the explicitly declared target135/roster and inherited arithmetic,
geometry and observation adapters remain. Raw trace/spawn data carries the new
DOOM-DEMON-EXEC-v1 tag. Normal, strict arithmetic and ASan/UBSan runs must agree.

[m3f5_snapshot_goldens.py](m3f5_snapshot_goldens.py) independently maps canonical
C identity fields into the frozen native representation and invokes the unchanged
DSIM-v20 encoder in a temporary tree. It reads C-derived metadata, not Rust output.
Both raw C and projected expectations remain available with hashes/provenance.
Historical tools and captures are unchanged. [check_canonical_demon.py](check_canonical_demon.py)
checks debug/release example output against the independently encoded captures.
[Investigation](../../docs/M3F5_REFERENCE.md), [commands](../../docs/TESTING.md#canonical-demon-execution-binding)
and [observations](../../docs/validation/M3F5.md) separate source behavior,
projection, encoding and Rust policy evidence.

## M3f6 native runtime boundary

The new m3f6_reference Rust test reads the unchanged M3f5 raw canonical observations
and independent DSIM-v20/event fixtures. It compares the new typed native view to
actual ActorLife slots and projected source definitions. No C generator, adapter,
capture or encoder changes; existing provenance remains intact. Reuse is explicitly
separate from new C-input coverage. The new check_runtime_identity helper asserts
its example in debug/release against the existing independent digest index.
See [audit](../../docs/M3F6_REFERENCE.md) and
[observations](../../docs/validation/M3F6.md).

## M3f7 compact original-ID evidence

[m3f7_snapshot_goldens.py](m3f7_snapshot_goldens.py) independently re-encodes the
immutable M3f5 C dataset into DSIM-v21. It retains actual source type/state IDs;
only the explicitly synthetic body binds to the separately identified context.
It invokes unchanged field/event framing in a temporary tree, emits no redundant
registry rows and never reads Rust source/output. Sorted discovery/provenance
makes output independent of filesystem ordering. New evidence is a compact index,
two full byte streams and input/output SHA256 provenance, not another trace/event
copy. Regenerate without `--check`; verify with `--check`.

The Rust harness reads actual ActorLife fields and reuses old EV20 payloads,
substituting identity fields directly from C chronology. No native Demon state
mapping enters the new registry. This is representation evidence using reused
inputs, not new source execution coverage. [Commands](../../docs/TESTING.md#original-id-registry-and-diagnostic-contract),
[contract](../../docs/STATE_FORMAT.md#dsim-v21--original-id-demon-diagnostics) and
[observations](../../docs/validation/M3F7.md) separate those claims.

## M3f8 compact target/lifetime evidence

[m3f8_reference.py](m3f8_reference.py) supplies new controlled inputs to the
unchanged genuine-body M3f5 extraction. Manual target writes are declared inputs;
damage replacement, death and deferred thinker retirement execute original bodies.
The inherited storage quarantine observes logical retirement without dereferencing
retired gameplay targets or proving real zone/address reuse. Original type/state
IDs stay pristine; the unbound observer-body adapter remains explicitly separate.
Normal, strict arithmetic and ASan/UBSan outputs must agree. No historical writer,
trace, map or fixture is rewritten.

[m3f8_snapshot_goldens.py](m3f8_snapshot_goldens.py) consumes those C rows and the
existing independent schema encoder in a temporary tree. It preserves original
IDs, binds only the synthetic observer body to the context definition, and applies
declared slot/generation policy to C birth order. A compact digest index, complete
ordered event payloads and two uniquely useful pending/reuse full-byte fixtures
avoid redundant large snapshots. This is still DSIM-v21, not a new save format.
Input/output digests and inherited source provenance remain explicit. The
[example checker](check_context_ownership.py), [commands](../../docs/TESTING.md#context-and-target-lifetime-validation)
and [observations](../../docs/validation/M3F8.md) separate new C observations,
historical evidence reuse, schema projection and Rust-only safety.

## Game-instance boundary checks

`check_game_instance.py` compares debug/release output against the unchanged
M3f8 replacement/clear C-derived digest. The Rust reference helper drives that
same input through checked provenance imports; no new C observer, raw trace or
fixture is generated. Run every historical observer/encoder and the M3f8 pair
unchanged. In-process instance keys, missing/foreign routing, restart and clone
policy are Rust-specific safety checks, not original-C identity equivalence.
See [the audit](../../docs/M3F9_REFERENCE.md),
[commands](../../docs/TESTING.md#game-instance-provenance-validation) and
[observations](../../docs/validation/M3F9.md).

## M3g1 observation placement

[Investigation](../../docs/M3G1_REFERENCE.md) separates stepping, dynamic capture,
byte encoding and bounded transaction costs. `check_diagnostic_free.py` checks the
new example against the unchanged M3f8 independent capture index in debug/release.
The new API comparisons replay the same frozen inputs and reuse the same C/DSIM
oracles; there is no new C input, behavioral coverage, encoder or raw fixture.
[Testing](../../docs/TESTING.md#snapshot-free-stepping-validation) describes the
separate structural/safety and host measurement checks.

## Original player body lifecycle

`m3g2_reference.py` extends the frozen genuine-body extractor with actual pristine
MT_PLAYER0 and a verbatim source association/counter fragment. It does not claim
full P_SpawnPlayer or PlayerThink. A separately selected unlisted, stationary,
Medium/commercial context supports direct damage without a physical inflictor,
pain, death and retained corpses. Live Demon sources are explicitly retired before
traversal or killed into allocated corpses; the all-inactive C scan is never run.

Genuine P_DropWeapon is added, with original weapon metadata. P_SetPsprite and
I_Tactile record declared service requests; actual weapon animation and hardware
presentation remain excluded. Executed function entries are inventoried separately
from compiled bodies and new branch/input observations. Retired allocations remain
quarantined and opaque; no freed-pointer or address-reuse equivalence is claimed.

`m3g2_snapshot_goldens.py` reuses independent field/event framing over C rows and
explicit v22 schema policy, without player fixture aliases or Rust output. Compact
selected physical captures and full per-tic event payloads retain provenance while
limiting new reference growth. No old output/helper/tool bytes change.
[The investigation](../../docs/M3G2_REFERENCE.md),
[diagnostic schema](../../docs/STATE_FORMAT.md#dsim-v22--original-player-body),
[evidence record](../../docs/validation/M3G2.md) and
[reproduction commands](../../docs/TESTING.md#original-player-body-validation)
separate C behavior, encoding and Rust safety policy.

## Original-player physical knockback

`m3g3_reference.py` extends the frozen source extraction runner with actual original
slide bodies and bounded map-spawned origins; it never reads Rust outputs. New
C trace/case/provenance files retain selected physical captures and every compared
damage/movement/state/RNG event. Uncompared auxiliary path-trace instrumentation is
omitted. `m3g3_snapshot_goldens.py` uses reusable `player_snapshot.py` to encode
DSIM22 policy3, selected full fixtures and separately versioned PBE1 compact event
frames. Historical tools/fixtures remain unchanged. Commands belong in
[TESTING](../../docs/TESTING.md#grounded-original-player-knockback), source/adapters in
[the investigation](../../docs/M3G3_REFERENCE.md), and observed counts/limitations in
[the engineering record](../../docs/validation/M3G3.md).


M3g4: `m3g4_reference.py` extracts actual P_Thrust/P_MovePlayer plus the declared
verbatim reaction conditional before P_RunThinkers. `m3g4_snapshot_goldens.py`
uses reusable `player_execution_snapshot.py` and frozen field/event primitives;
old encoders are unchanged. PM/UC compact rows, PG phase markers and PBE1 events
have explicit provenance. See [investigation](../../docs/M3G4_REFERENCE.md),
[reproduction](../../docs/TESTING.md#original-player-command-movement) and
[observations](../../docs/validation/M3G4.md). No full PlayerThink claim.

M3g5 adds [the active-player/Demon observer](m3g5_reference.py) and [independent DSIM22 encoder](m3g5_snapshot_goldens.py); [source boundaries](../../docs/M3G5_REFERENCE.md) and [reproduction](../../docs/TESTING.md#active-original-player-and-demon-acquisition) qualify the fixed roster and nonmelee pursuit slice. Historical tools and expected bytes remain unchanged.

M3g6 [melee observations](m3g6_reference.py) reuse that exact frozen source driver
with new genuine-player combat inputs, and [independent encoding](m3g6_snapshot_goldens.py)
reuses generic player/event primitives for DSIM22 policy6. Selection and later
attack range decisions remain distinct; source function hashes/entries, partial
captures and complete events have separate provenance. No Rust output creates
expectations. See [source qualifications](../../docs/M3G6_REFERENCE.md),
[commands](../../docs/TESTING.md#autonomous-demon-player-melee) and
[observations](../../docs/validation/M3G6.md). Old capture formats and files remain immutable.


M3g7's [genuine pistol observer](m3g7_reference.py) and
[independent DSIM23 encoder](m3g7_snapshot_goldens.py) add original reborn/psprite,
clip/refire, aim/trace/effect/noise and Demon combat composition. Historical
observers/helpers remain immutable. Source rows and expected bytes precede
matching Rust integration; read-only aim queries are not monster-aim events.
See [source qualifications](../../docs/M3G7_REFERENCE.md),
[evidence counts](../../docs/validation/M3G7.md) and
[commands](../../docs/TESTING.md#genuine-player-pistol). No whole-player or
portable unmodified ISO-C equivalence is inferred.

M3g8 extends the frozen M3g7 extraction with actual original fist/shotgun/chaingun
and Light2 bodies plus the declared source weapon-selection fragment.
[m3g8_reference.py](m3g8_reference.py) produces new compact C observations;
[m3g8_snapshot_goldens.py](m3g8_snapshot_goldens.py) and
[weapon_snapshot.py](weapon_snapshot.py) independently encode DSIM24.
[evidence_pack.py](evidence_pack.py) specifies bounded DREFLZ1 storage for new test
evidence only; it does not migrate any historical fixture or define a game format.
Packed and expanded hashes retain provenance and inspection/reproduction needs
only standard-library Python. A safe bounded test-only Rust decoder supplies the
same committed bytes to comparisons. See [investigation](../../docs/M3G8_REFERENCE_INVESTIGATION.md),
[commands](../../docs/TESTING.md#original-player-weapon-expansion) and
[observations](../../docs/validation/M3G8.md).


M3g9 adds actual original GiveAmmo/GiveBody/GiveWeapon/TouchSpecialThing bodies
through [m3g9_reference.py](m3g9_reference.py). Its case/observation/component
modules are source-harness inputs, not production item managers. The independent
[pickup encoder](pickup_snapshot.py) and
[fixture writer](m3g9_snapshot_goldens.py) specify DSIM25 from source rows.
Things-before-lines candidate prefixes survive interleaved touch events. Actual
trace callback roles keep geometry-only slide observations distinct from shot/aim
events. C-only inactive item-respawn queue rows remain in packed evidence.
Historical observers/helpers/fixtures are unchanged. See
[source investigation](../../docs/M3G9_REFERENCE_INVESTIGATION.md),
[commands](../../docs/TESTING.md#original-world-pickups) and
[observations](../../docs/validation/M3G9.md).
