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

## Real-map-scale rooms (M3h3)

[m3h3_reference.py](m3h3_reference.py) reuses every compiled M3h2 body, observer and
controlled input unchanged and replaces only the room: [m3h3_geometry.py](m3h3_geometry.py)
builds walled rectangles (one flat sector, axis walls split at 1,024 units, a cell-exact
BLOCKMAP with the rectangle minimum as origin) up to the 2,048-unit vertex envelope,
including an offset room and a pillar. The C side has no coordinate limit, so the
scenarios that stalled the historical Rust policies simply continue. The inventory
counts slides (`SL`), blocked moves, near-wall impulses, peak live fireballs and the
largest coordinate per scenario. [map_scale_snapshot.py](map_scale_snapshot.py) and
[m3h3_snapshot_goldens.py](m3h3_snapshot_goldens.py) project DSIM30 from the C rows:
the DSIM29 layout under version 30, policy 14, the map-scale context, 320 slots and
the 208 live-fireball bound. See [investigation](../../docs/M3H3_REFERENCE_INVESTIGATION.md),
[commands](../../docs/TESTING.md#original-monsters-at-real-map-scale) and
[observations](../../docs/validation/M3H3.md).

## Static-heights rooms (M3h4)

[m3h4_reference.py](m3h4_reference.py) reuses every compiled M3h2 body, observer and
controlled input unchanged and replaces only the room: [m3h4_geometry.py](m3h4_geometry.py)
builds strips of axis-aligned sectors with static floor/ceiling heights inside a walled
rectangle (one-sided boundary walls split at 1,024 units, two-sided lines with
configurable flags between neighbours, one subsector per sector, a guillotine BSP,
optional REJECT pairs and a cell-exact BLOCKMAP); [m3h4_cases.py](m3h4_cases.py)
places THINGs on stairs, a ledge, a window, a closed sector, fences, a crawlspace and
a tower. The inventory counts blocked moves by `P_TryMove` reason, vertical steps,
falls, floor/ceiling contacts, landing sounds, sight queries, hitscan impacts, noise
floods and fireball floor/ceiling explosions. [heights_snapshot.py](heights_snapshot.py)
and [m3h4_snapshot_goldens.py](m3h4_snapshot_goldens.py) project DSIM31 from the C
rows: the DSIM30 layout under version 31, policy 15 and the heights context, with
slide-traverse trace rows filtered before the inherited chain. See
[investigation](../../docs/M3H4_REFERENCE_INVESTIGATION.md),
[commands](../../docs/TESTING.md#original-monsters-with-static-heights) and
[observations](../../docs/validation/M3H4.md).

## Diagonal-wall rooms (M3h5)

[m3h5_reference.py](m3h5_reference.py) reuses every compiled M3h2 body, observer and
controlled input unchanged and replaces only the room: [m3h5_geometry.py](m3h5_geometry.py)
builds sectors from explicit polygon rings (clockwise outer rings, counterclockwise
holes), shares edges listed in opposite directions as two-sided lines, splits one-sided
walls at exact integer points, builds the BSP from an explicit partition tree with
convex single-sector leaves (seg splits asserted integral), and lists every line in
every BLOCKMAP cell whose closed square meets it (an exact clip); the harness patches
the inherited loader so subsectors carry their leaf sectors. [m3h5_cases.py](m3h5_cases.py)
places a wedge with 45°, 12:5 and 5:1 walls, a convex tip and a concave corner, an
octagonal pillar, a 45° corridor, a diagonal riser, a diagonal window and the corner
of the investigation note, with one harness-only `push_*` input (the player's momentum
before a tic) declared beside `hit_*`. The inventory counts slides and blocked moves at
diagonal lines, refusals by reason, sight, hitscans, noise, launches, explosions and
captured player positions outside the room. [diagonal_snapshot.py](diagonal_snapshot.py)
and [m3h5_snapshot_goldens.py](m3h5_snapshot_goldens.py) project DSIM32 from the C
rows: the DSIM31 layout under version 32, policy 16 and the diagonal context, deriving
each actor's sector from its subsector's first seg because these rooms have several
subsectors per sector. See [investigation](../../docs/M3H5_REFERENCE_INVESTIGATION.md),
[commands](../../docs/TESTING.md#original-monsters-among-diagonal-walls) and
[observations](../../docs/validation/M3H5.md).

## Real-map-scale rooms (M3h6)

[m3h6_reference.py](m3h6_reference.py) reuses every compiled M3h2/M3h5 body, observer
and controlled input unchanged over rooms of classic-map size. [m3h6_geometry.py](m3h6_geometry.py)
is the M3h5 polygon builder with an automatic recursive node builder (partitions chosen
among the seg lines by the DoomBSP grade, split vertices rounded to integers as DoomBSP
rounds them, leaves asserted single-sector and convex within rounding) and the sizes of
the enlarged harness arrays instead of the M3h5 asserts; [m3h6_cases.py](m3h6_cases.py)
builds the town (a 6 × 4 grid of octagonal rooms, corridors, steps, pillars and window
bands: 516 linedefs, 74 sectors), the gallery (27 two-sided teeth per wall and sixteen pickups
along the line of fire), the extent room (a 2,750-unit diagonal wall of norm 3,850 and a
2,683-unit two-sided line, at 99.9% and 98.3% of the certificate bounds), the dense comb
(123 distinct lines in one 2 × 2 cell window), the wide square (a 60 × 60 cell BLOCKMAP)
and, Rust-only, the witness hall whose long shot straddles 130 lines. The harness keeps
`intercept_t intercepts[128]` and its abort guard exactly, enlarges the shim's line,
side, sector, seg, subsector, node, cell, REJECT and owned-object arrays through the
`m3d3j_shim.h` write hook, and adds one observer, `TI` (the intercept count on entry to
`P_TraverseIntercepts`), whose count the generator checks against the `I` rows of its
trace. The inventory records the largest trace by traverser kind, thing re-adds and the
most sight queries in one tic. [real_map_snapshot.py](real_map_snapshot.py) and
[m3h6_snapshot_goldens.py](m3h6_snapshot_goldens.py) project DSIM33 from the C rows: the
DSIM32 layout under version 33, policy 17 and the real-map context, with the `TI` rows
dropped before the chain. See [investigation](../../docs/M3H6_REFERENCE_INVESTIGATION.md),
[commands](../../docs/TESTING.md#original-monsters-at-classic-map-scale) and
[observations](../../docs/validation/M3H6.md).

## Sky and static sector specials (M3h7)

[m3h7_reference.py](m3h7_reference.py) reuses every compiled M3h6 body, observer and
controlled input unchanged and adds the genuine `P_SpawnSpecials`,
`P_PlayerInSpecialSector`, `P_UpdateSpecials`, `P_ShootSpecialLine`, `getNextSector`,
`P_FindMinSurroundingLight` and the `p_lights.c` spawners and thinkers.
`P_SpawnSpecials` runs once after the THING spawns and the player body,
`P_PlayerInSpecialSector` after `P_CalcHeight` and `P_UpdateSpecials` after the thinker
pass; sector thinkers come from a separate harness store so actor idents keep their
meaning, and `skyflatnum` is the declared index of `F_SKY1`.
[m3h7_geometry.py](m3h7_geometry.py) is the M3h6 builder plus sector light, special, tag
and flats and line specials; [m3h7_cases.py](m3h7_cases.py) builds the pools (twelve
basins, one per Stage A sector special, a ledge over nukage, a dark gallery behind
one-sided walls, three scrolling walls) and the sky courts (sky-hack blocks, a sky block
against a one-sided wall, a tower in a low sky court and three windows whose line
order and sides pin `ceilingline`). New observer rows (thinker list, sector light and
special, light draws and changes, special-sector calls, secrets, sky refusals, removals,
shots and ceiling hits) are documented in the contract's `row_formats`.
[specials_snapshot.py](specials_snapshot.py) and
[m3h7_snapshot_goldens.py](m3h7_snapshot_goldens.py) project DSIM34 from the C rows:
the DSIM33 layout under version 34, policy 18 and the specials context, plus the shared
thinker order and the sector-thinker store (`TK` rows), runtime sector light and special
and the player's secret count (`SX` rows) and tag-25 secret, sky-removal and sky-shot
events (`SE`, `KR`, `KS`); the generator also checks that the stream without tag 25 is
the DSIM33 projection of the same rows. See
[investigation](../../docs/M3H7_REFERENCE_INVESTIGATION.md), the
[DSIM v34 schema](../../docs/STATE_FORMAT.md#dsim-v34--original-monsters-with-sky-and-static-specials),
[commands](../../docs/TESTING.md#original-monsters-with-sky-and-static-specials) and
[observations](../../docs/validation/M3H7.md). [check_player_specials.py](check_player_specials.py)
runs the `player_specials` example in debug and release and asserts equal output.

## Doors, lifts, floors, switches and the exit (M3h8)

[m3h8_reference.py](m3h8_reference.py) reuses every compiled M3h7 body, observer and
controlled input unchanged and adds the genuine `p_doors.c` (`T_VerticalDoor`,
`EV_DoDoor`, `EV_VerticalDoor`), `p_plats.c` (`T_PlatRaise`, `EV_DoPlat`,
`P_AddActivePlat`, `P_RemoveActivePlat`), `p_floor.c` (`T_MovePlane`, `T_MoveFloor`,
`EV_DoFloor`), `p_switch.c` (`P_StartButton`, `P_ChangeSwitchTexture`,
`P_UseSpecialLine`), `p_spec.c` (`P_CrossSpecialLine` and the `P_Find*` helpers), `p_map.c`
(`P_ThingHeightClip`, `PIT_ChangeSector`, `P_ChangeSector`, `PTR_UseTraverse`,
`P_UseLines`), `m_bbox.c` and `G_ExitLevel` bodies, plus three genuine fragments: the use
block of `P_PlayerThink`, the `BT_USE` statement of `P_DeathThink` and the soundorg and
blockbox statements of `P_GroupLines`. The `P_TryMove` crossing loop and the `P_Move`
failure branch, compiled since M3h2, now reach real bodies; `spechit` keeps its source
size of eight. Declared adapters: sound origins are recorded (sector, button field, mobj or
global), side textures are resolved from the committed SIDEDEFS names through a
declared synthetic texture table and the switch list is a declared pair, `G_DoCompleted` and `G_DoReborn`
end the case instead of running, and mover allocations are zero-filled (decision N6).
[m3h8_geometry.py](m3h8_geometry.py) is the M3h7 builder plus optional line flags and
`SW1COMM` on the front middle texture of every 11/63 line; [m3h8_cases.py](m3h8_cases.py) builds the doorway (DR doors closed,
open and stepped), its one-door hall variant, the switchyard (SR 63 and its tagged door,
the S1 exit, a plain wall, a DR door used from its back side), the lift at three
ceilings (riders fit, the last step does not fit, an ordinary step does not fit), the
floors corridor (W1 36 jump-up and lowering, W1 38, W1 109 sharing its door with a DR
line), the dense lane (eight two-sided WR 88 lines in one 2 × 2 window) and the pit (a
W1 36 target whose only neighbour floor lies below −500), 33 scenarios in all, two of
them with the controlled between-tic see-state entry `chase_at`; two-sided scrollers are not built (decision N1 keeps them rejected). The
Rust-only nine-line lane is written as `m3h8_dense9.bin` and never run in C. New observer
rows (sector heights, runtime line specials, mover records, `spechit` entries, use and
crossing calls, plane steps, `P_ChangeSector` outcomes, sector sounds, the exit and the
reborn request) are documented in the contract's `row_formats`.
[movers_snapshot.py](movers_snapshot.py) and
[m3h8_snapshot_goldens.py](m3h8_snapshot_goldens.py) project DSIM35 from the C
rows: the DSIM34 layout under version 35, policy 19 and the movers context, plus
`level.exit`, the moving sector heights (`MH`), the runtime line specials (`ML`), the
player's `use_down` (`MZ`), the button slots (`MB`), door, plat, floor and removed-mover records in the
sector-thinker store and the shared order (`TK`, `MM`, `SD`, `SF`) and tag-26 events for
use, crossing, mover starts and calls, plane steps, `P_ChangeSector` clips, mover removal
and unlinking, sector sounds, the exit and the reborn request; the generator also checks
that the stream without tag 26 is the DSIM34 projection of the same rows.
[check_player_movers.py](check_player_movers.py) runs the full Rust replay against those
rows and goldens in debug and release, then the asserted `player_movers` example in
both profiles, and asserts equal output. See
[investigation](../../docs/M3H8_REFERENCE_INVESTIGATION.md), the
[DSIM v35 schema](../../docs/STATE_FORMAT.md#dsim-v35--original-doors-lifts-floors-switches-and-the-exit),
[commands](../../docs/TESTING.md#original-doors-lifts-floors-switches-and-the-exit-c-evidence) and
[observations](../../docs/validation/M3H8.md).

## Two-sided scrolling walls, static decorations and the spawn filter (M3h9)

[m3h9_reference.py](m3h9_reference.py) reuses every compiled M3h8 body, observer,
declared adapter and controlled input unchanged and adds no genuine body: the two-sided
type-48 effects, `P_SpawnMapThing` over the pristine `info.c` rows of the seven E1M1
decoration types, their collision (`PIT_CheckThing`, `PTR_AimTraverse`,
`PTR_ShootTraverse`) and their clipping under movers (`PIT_ChangeSector`,
`P_ThingHeightClip`) are bodies compiled since M3d3, M3h2 and M3h8 reaching branches these
rooms exercise for the first time. It adds three observer rows (`DT` for the
`PIT_CheckThing` flag tests, `XT` for the aim and shoot traverser skips, `NS` for
`numspechit` before each movement result; see the contract's `row_formats`) and one
per-case harness input, `skill`, the genuine `gameskill` the spawn filter reads.
[m3h9_geometry.py](m3h9_geometry.py) is the M3h8 builder plus the two-sided scroller
inventory and the declared decoration table; [m3h9_cases.py](m3h9_cases.py) builds the
scroller hall, the blocking scroller, the eight-scroller lane, the decoration gallery,
the lift room with a lamp, a corpse and a pillar under a door, the census room and,
from the independent review, the door-step room (a pillar straddling a door and a higher
step) plus an Imp beyond the blocking scroller, 22 scenarios in all, 20 at medium skill
and two C-only census rows at easy and hard. The
Rust-only nine-scroller lane is written as `m3h9_dense9.bin` and never run in C.
[e1m1_snapshot.py](e1m1_snapshot.py) and
[m3h9_snapshot_goldens.py](m3h9_snapshot_goldens.py) project DSIM36 from the C rows of
the 20 medium scenarios: the DSIM35 layout under version 36, policy 20 and the E1M1
context with no new field or event, the decorations checked against the declared table
and their lump order, after dropping the `DT`, `XT` and `NS` rows; the generator asserts
that every capture equals the DSIM35 projection of the same rows except the version,
the profile value and the context bytes. [check_player_e1m1.py](check_player_e1m1.py)
runs the full Rust replay against those rows and goldens in debug and release, then the
asserted `player_e1m1` example in both profiles, and asserts equal output. See
[investigation](../../docs/M3H9_REFERENCE_INVESTIGATION.md), the
[DSIM v36 schema](../../docs/STATE_FORMAT.md#dsim-v36--original-two-sided-scrollers-and-static-decorations),
[commands](../../docs/TESTING.md#original-two-sided-scrollers-static-decorations-and-the-spawn-filter-c-evidence) and
[observations](../../docs/validation/M3H9.md).

## Historical raw-trace loading

Selected large CSV traces now use standard zstd storage. Run frozen C/encoder
scripts through `run_reference.py`; it restores their original raw paths in a
private copied workspace and runs their unchanged checks. The complete suite
retains every previous source command and sanitizer flag. See the
[storage contract](../../docs/EVIDENCE_STORAGE.md),
[authoritative commands](../../docs/TESTING.md#historical-zstd-evidence) and
[migration observations](../../docs/validation/ZSTD_WEAPON_MAINTENANCE.md).
Direct legacy generators still describe their original raw-tree interface.
Existing DREFLZ1 streams, complete DSIM fixtures and provenance are unchanged.


## Original armor source family

[m3g10_reference.py](m3g10_reference.py) reuses the frozen world-pickup source
harness and adds actual P_GiveArmor plus explicit armor observations.
[m3g10_components.py](m3g10_components.py) declares controlled source component
inputs separately from normal-health world encounters. [armor_snapshot.py](armor_snapshot.py)
and [m3g10_snapshot_goldens.py](m3g10_snapshot_goldens.py) independently encode
DSIM26; no Rust result is an expected-output source. New files deliberately avoid
frozen legacy generator/inventory globs. Historical tool/helper bytes and every old
fixture remain unchanged. See [investigation](../../docs/M3G10_REFERENCE_INVESTIGATION.md),
[commands](../../docs/TESTING.md#original-armor-and-sustained-combat) and
[observations](../../docs/validation/M3G10.md).
