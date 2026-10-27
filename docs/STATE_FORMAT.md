# Canonical simulation state, DSIM version 1

This is a diagnostic/replay comparison format for the implemented M2 profile,
not a DOOM savegame or network/demo format. A field schema change requires an
intentional format-version change and reviewed golden updates. All integers
below are little endian, with explicit widths and two's-complement signed values.
No Rust layout, padding, pointers, addresses, timestamps, paths, or GPU state is
encoded. Shared immutable map storage uses Arc internally; reference counts and
addresses are excluded.

## Framing and field order

Header: ASCII `DSIM`, u32 version `1`, u64 field count. Each ordered field is
u32 path-byte length, UTF-8 path bytes, u64 value-byte length, then value bytes.
Generated schema paths are shorter than u32 capacity. Value/count lengths fit
u64 on supported Linux/Windows/macOS hosts. No native usize bytes are emitted.

Runtime fields precede static map fields. Runtime order:

| Field/group | Value encoding |
| --- | --- |
| `tic` | u64 completed logical tics |
| `tic_rate`, `profile` | u32 35; u32 profile 1 (static grounded non-solid non-player XY, mobj flags zero) |
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
