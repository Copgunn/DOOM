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

A separate unadapted source-spawn pass retains type12, original states/flags and
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
