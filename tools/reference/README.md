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
