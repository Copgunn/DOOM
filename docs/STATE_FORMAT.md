# Canonical simulation state, DSIM versions 1, 2, 3 and 4

This is a diagnostic/replay comparison format for explicit M2, M3a, M3b and M3c profiles,
not a DOOM savegame or network/demo format. A field schema change requires an
intentional format-version change and reviewed golden updates. All integers
below are little endian, with explicit widths and two's-complement signed values.
No Rust layout, padding, pointers, addresses, timestamps, paths, or GPU state is
encoded. Shared immutable map storage uses Arc internally; reference counts and
addresses are excluded.

## Framing and field order

Header: ASCII `DSIM`, u32 schema version (`1` for M2, `2` for M3a, `3` for M3b, `4` for M3c),
u64 field count. Each ordered field is
u32 path-byte length, UTF-8 path bytes, u64 value-byte length, then value bytes.
Generated schema paths are shorter than u32 capacity. Value/count lengths fit
u64 on supported Linux/Windows/macOS hosts. No native usize bytes are emitted.

Runtime fields precede static map fields. Runtime order:

| Field/group | Value encoding |
| --- | --- |
| `tic` | u64 completed logical tics |
| `tic_rate`, `profile` | u32 35; u32 profile 1 (M2 flat non-player XY), 2 (M3a player/solid XY), 3 (M3b static-sector XY/Z), or 4 (M3c typed actor lifecycle); profile-specific additions are specified below |
| `paused` | u8 boolean |
| `rng.gameplay`, `rng.miscellaneous` | u8 indexes, in that order |
| `store.max_slots`, `store.slot_count` | u32 capacity policy; u64 logical allocated slot count |
| `store.head`, `store.tail` | optional actor identities |
| `thinker.order` | consecutive actor identities in processing order |
| `actor.N.generation`, `.retired`, `.occupied` | u64, u8 boolean, u8 boolean, for every logical slot including vacant/retired slots |
| Occupied slot `.pending_removal`, `.previous`, `.next` | u8 boolean, optional identities |
| Occupied slot `.x`, `.y`, `.z`, `.momx`, `.momy`, `.radius`, `.height`, `.floor`, `.ceiling` | i32 raw 16.16, in this order |
| Occupied slot `.angle`, `.lastlook` | u32 binary angle; u8 spawn RNG modulo four |
| `player.N.actor`, `.forward`, `.side`, `.angle_turn`, `.buttons` for slots 0..3 | optional identity, i8, i8, i16, u8 |
| `events.last.count`, then `events.last.N` | u64 count, individually framed event records |
| `events.pending.count`, then `events.pending.N` | u64 count, individually framed event records |

An actor identity is u32 slot + u64 generation (12 bytes). An optional identity
is u8 presence + those 12 bytes; absent identities have all-zero payloads. Slots
and generations are logical state, not allocator metadata. Heap capacity is
excluded. The lowest reusable slot is chosen deterministically; processing order
remains a separate append order. Generations never wrap; exhausted slots retire.
Handles belong to a store/instance, not a global namespace. Fresh equivalent
instances intentionally produce equivalent handle values.

## Events

| Tag | Payload |
| --- | --- |
| 0, phase | u8 Players=0, Thinkers=1, Specials=2, Respawn=3 |
| 1, spawned | actor identity |
| 2, removal marked | actor identity |
| 3, retired | actor identity |
| 4, player-command phase | u8 slot + actor identity |
| 5, thinker update | actor identity |
| 6, movement attempt | actor identity; i32 target X/Y; u16 blocker or 65535 for none; u8 fits; u32 candidate count; ordered u16 candidate indexes |
| 7, explicit gameplay draw | u8 lookup result |

Between-tic spawning, removal marks, and explicit gameplay draws queue pending
observations. At the next unpaused step they become the start of the last-event
list, before Players/Thinkers/Specials/Respawn observations. Spawn consumes one
gameplay lookup for lastlook; its event is tag 1, not an additional tag-7 draw.
Specials and respawn are currently phase markers, not implemented gameplay.
Paused step calls leave commands, time, RNG, and both event lists unchanged.
Presentation reads event lists without draining or mutating them.

## Static map fields

After runtime fields: `map.things`, `.linedefs`, `.sidedefs`, `.vertices`, `.segs`,
`.subsectors`, `.nodes`, `.sectors`, `.reject`, `.blockmap.words`,
`.blockmap.cell_ranges`, in that order. The first eight tables encode the exact
classic record fields in original order and widths (10/14/30/4/12/4/28/26 bytes),
including raw eight-byte names and -1 sidedef sentinels. BSP children encode the
original u16 high-bit leaf flag. REJECT retains all parsed bytes. BLOCKMAP words
are all decoded u16 words, including header/table/unused words; row-major cell
ranges are u32 start/end word indexes, excluding terminators. Shared lists are
stored once, so canonicalization is linear rather than expanding every cell list.
Derived line slopes/bboxes are reproducible from these fields and profile 1.

## DSIM-v2 and migration

`Simulation::new` / `with_actor_limit` remain profile 1, version 1, with unchanged
framing, fields, events and committed v1 expectations above. Select M3a explicitly
with `player_collision` / `with_player_collision_limit`; its header version and
`profile` value are both 2. `Snapshot::version()` exposes the version;
first-difference reports `schema.version` before comparing incompatible schemas.
There is no savegame loader or automatic migration. Re-run the same setup/commands
under the chosen profile, retaining version-specific expected traces. Do not
replace v1 regression expectations with v2 bytes.

V2 retains v1 field order with these precise additions:

- Immediately after each occupied `actor.N.lastlook`: `actor.N.flags`, u32 0 or
  MF_SOLID=2. No other actor flags are supported. Vacant slots omit this field.
- After all logical actor slots and before `player.0.actor`: `spatial.link_order`,
  concatenated 12-byte identities, most recent link first. Includes off-map live
  actors; excludes removal-marked actors. Filtering by center cell reconstructs
  cell-head order. This order can differ from thinker/slot order and is required
  state. Allocation capacity is excluded.
- Existing player bindings and last commands encode all implemented player XY
  state. Ground Z/floor/ceiling are already explicit; no hidden player timers,
  view/animation/weapon state is implemented. Movement candidates are observations,
  not persistent pointer relationships.

Two new event tags use the same individually framed event fields:

| Tag | Payload, in order |
| --- | --- |
| 8, actor movement | 12-byte mover; i32 target X/Y; 13-byte optional blocking actor; u32 actor-candidate count; ordered 12-byte candidate IDs; u16 blocking line or 65535; u8 fits; u32 line-candidate count; ordered u16 line IDs |
| 9, slide projection | 12-byte mover; u16 selected linedef |

Profile 2 emits tag 8 instead of tag 6, including ordinary non-player moves.
Actor candidates include self/non-solid observations and stop at the first solid
blocker. A thing blocker leaves the line-candidate list empty. Tag 9 occurs when
P_HitSlideLine projects remaining motion; a failed flush or pure stair fallback
has no projection event. All phase/spawn/removal/RNG tags retain v1 encodings.
Static map fields are identical in layout; derived cell/line caches are omitted.

`m3a_snapshot_goldens.py` independently constructs explicit profile-2 coasting
state from Python v1 encoding rules, source C XY values, and explicit clockwise
synthetic map records. It reads no Rust output. `m3a_snapshot_provenance.json`
records input/encoder/output SHA-256 hashes. New first-tic bytes and twelve digests
are versioned separately; they are project schema goldens, not reference saves.

## DSIM-v3 and static-sector state

M3b constructors explicitly select header version/profile 3. V1/v2 constructors,
fields, framing and fixtures remain available unchanged; there is no savegame
migration/loader. Reinitialize a selected profile and replay its commands, retaining
versioned expectations. Cross-version comparison reports `schema.version`.

V3 retains v2 field order with these exact changes/additions:

- Occupied `actor.N.flags` is u32 combination of MF_SOLID=2, MF_NOGRAVITY=512 and
  MF_DROPOFF=1024; no other bits are supported. After it: `.momz` i32 raw fixed,
  `.subsector` u16, `.sector` u16. Existing `.z`, `.floor`, `.ceiling` retain their
  positions; constraints can differ from the actor center's sector heights.
- After `spatial.link_order`, before player fields: `world.blockmap_policy` u8=1
  (supplied reference lists); `world.conservative_omissions` u32 diagnostic bbox
  cell-pair count; `sectors.count` u32.
- For each runtime sector in numeric order: `sector.N.floor` i32 raw fixed,
  `.ceiling` i32 raw fixed, `.actors` concatenated 12-byte identities in current
  sector-head order. The sector vector is distinct from static map definitions;
  M3b exposes no runtime-height mutation API. Removal marks exclude list membership
  immediately even while actor slot storage remains occupied.
- Spatial order includes all live actors, including off-grid centers; filtering
  by source BSP membership reconstructs sector-head lists. Derived topology,
  line bounds/slopes and compiled cell caches are reproducible and omitted.

New events (all integers little endian):

| Tag | Payload |
| --- | --- |
| 10, sector movement | The complete tag-8 payload without its tag byte, followed by u16 proposed subsector, u16 sector, i32 floor/ceiling/dropoff, u8 floatok, u8 first failure, u32 opening count; each opening is u16 line + i32 top/bottom/range/lowfloor |
| 11, Z update | 12-byte actor; i32 before Z/momz, after Z/momz, floor/ceiling (six i32s); u8 floor-contact, ceiling-contact, gravity-applied booleans |

Failure values: 0 allowed, 1 actor, 2 line, 3 interval fit, 4 headroom, 5 step,
6 dropoff. They follow source evaluation order. `fits` in the tag-8 subpayload is
false only after an evaluated interval-fit failure; an earlier actor/line blocker
leaves it true (not evaluated). `floatok` becomes true after interval fit passes.
Opening records include only actual crossing two-sided candidates evaluated before
rejection, preserving order. SectorMovement emits for each attempted destination,
including slide retries/fallback; tag 9 slide projections retain v2 encoding and
appear at their original chronological position. Z emits only when its source
guard (z!=floor or nonzero momz) runs, after that actor's XY events. No camera,
viewheight, audio or state-machine data is implied.

`m3b_snapshot_goldens.py` independently encodes 96 states from explicit setup,
synthetic map bytes, C per-attempt counts/order and physical XY/Z results. It reads
no Rust output. Contact bits derive from pre-Z C values and original predicates.
Five complete byte fixtures cover first fall, step-up clipping, center-sector
transition, airborne drop and landing; tests compare named fields as well as full
bytes. Input/encoder/output hashes are in `m3b_snapshot_provenance.json`. These are
project schema goldens with independent physical evidence, not engine saves.

## Digests, comparison, and evidence

FNV-1a-64: initial value `0xcbf29ce484222325`; for each canonical byte, XOR it
then multiply by `0x100000001b3`, wrapping modulo 2^64. It is noncryptographic.
`Snapshot::first_difference` compares named values; `compare_traces` compares
captures in order and reports the first differing tic/field or trace length.
Compare bytes/fields even when hashes agree. Default divergence display omits
value contents; callers can inspect encoded expected/actual values privately.

`tools/reference/snapshot_goldens.py` independently encodes an explicit one-actor
setup plus recorded C coasting values. It reads no Rust output. Tests compare the
full first snapshot bytes and twelve per-tic digests/position/momentum vectors.
Longer 768-call scenarios test fresh instances, RNG wrapping, lifecycle/reuse,
pause, presentation schedules, and injected first-divergence detection. Those
long scenarios are project consistency checks, not full-engine reference traces.
Both kinds run in debug/release CI across supported platforms.

Snapshots include map/resource bytes. Traces made with private assets must remain
private and must never become public fixtures, logs, artifacts, or screenshots.
All current fixtures and CI scenarios use only original synthetic map data.

## DSIM-v4 and typed actor state

Profile 4 extends v3 with the following explicitly ordered fields. Older profiles
retain their original bytes and fixtures. Registry definitions/options are immutable.
All integers use little-endian widths; booleans are u8 0/1. Optional actor IDs use
the existing 13-byte presence/u32-slot/u64-generation representation.

- After `rng.miscellaneous`: `actors.options` = skill u8 (baby=0 through nightmare=4),
  mode u8 (only single-player=0 admitted), nomonsters u8.
- Existing per-actor `flags` now records the full admitted source u32 flags.
- After actor flags, before v3 momz/subsector/sector: `type` u16, `state` u16
  (65535 = entered null pointer; zero = direct `&states[0]` spawn assignment),
  `tics`, `health`, `reaction_time` i32 each, `sprite` u16, `frame` u32, `target`
  optional ID, `death_requested` u8, `spawnpoint` presence u8 followed, when present,
  by signed x/y/angle/type i16 and options u16 (10 bytes). Prefix remains `actor.N.`.
- Spatial link order retains all logical relinks. Block queries filter NOBLOCKMAP;
  sector list fields filter NOSECTOR. Immediate removal excludes both lists.
- After immutable map fields: `registry.states.count` u64, then `registry.state.N`;
  `registry.types.count` u64, then `registry.type.N`. State payload is name length
  u8/name UTF-8 bytes, sprite u16, frame u32, tics i32, action encoding below,
  next u16, misc1/misc2 i32. Type payload is name length/name followed by the 23
  source-order i32 properties documented by ActorTypeDef and m3c_types.csv.

Action payload: None = tag 0; original = tag 1/u16 action ID; probe = tag 2/u8
probe ID (Random=0, Enter=1, SpawnTarget=2, RemoveSelf=3, RemoveTarget=4,
TargetSelf=5). Enter/SpawnTarget additionally carry a u16 state/type operand.
Original IDs follow the generated symbolic table: zero None, remaining 74 names
in lexicographic order. Sprite/sound IDs follow original source enum order.

Event tag 12 introduces actor-state observations. Its second u8 is Initialized=0
(actor ID/type u16), Entered=1 (actor/state u16), Action=2 (actor/action payload),
Sound=3 (optional actor/sound u16), Target=4 (actor/optional target), Death=5
(actor), TargetUnavailable=6 (actor), Random=7 (actor/value u8). Actor IDs here
are the existing nonoptional 12-byte slot/generation encoding unless specified.
Last/pending observations include these events in execution order. Initialized
records direct assignment without invoking an action; Death records the explicit
controlled request, not damage processing. No sound channel/device state exists.

`m3c_snapshot_goldens.py` independently encodes C values and explicit setups:
99 captures and 24 full states cover death, null/retirement, map placement,
recursive callbacks/targets/spawning, five actions and initial zero-tic behavior.
Names and immutable definitions are serialized so different native registries
cannot accidentally produce equal canonical bytes merely by sharing indexes.
