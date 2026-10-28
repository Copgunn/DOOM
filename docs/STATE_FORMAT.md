# Canonical simulation state, DSIM versions 1 through 12

This is a diagnostic/replay comparison format for explicit M2 through M3d3e profiles,
not a DOOM savegame or network/demo format. A field schema change requires an
intentional format-version change and reviewed golden updates. All integers
below are little endian, with explicit widths and two's-complement signed values.
No Rust layout, padding, pointers, addresses, timestamps, paths, or GPU state is
encoded. Shared immutable map storage uses Arc internally; reference counts and
addresses are excluded.

## Framing and field order

Header: ASCII `DSIM`, u32 schema version (`1` for M2, `2` for M3a, `3` for M3b, `4` for M3c, `5` for M3d1, `6` for M3d2, `7` for M3d2b, `8` for M3d3a, `9` for M3d3b, `10` for M3d3c, `11` for M3d3d, `12` for M3d3e),
u64 field count. Each ordered field is
u32 path-byte length, UTF-8 path bytes, u64 value-byte length, then value bytes.
Generated schema paths are shorter than u32 capacity. Value/count lengths fit
u64 on supported Linux/Windows/macOS hosts. No native usize bytes are emitted.

Runtime fields precede static map fields. Runtime order:

| Field/group | Value encoding |
| --- | --- |
| `tic` | u64 completed logical tics |
| `tic_rate`, `profile` | u32 35; u32 profile 1 (M2 flat non-player XY), 2 (M3a player/solid XY), 3 (M3b static-sector XY/Z), 4 (M3c typed actor lifecycle), 5 (M3d1 bounded damage/hitscan), 6 (M3d2 player weapons/psprites), 7 (M3d2b projectiles), 8 (M3d3a perception), 9 (M3d3b pursuit), 10 (M3d3c bounded attacks), 11 (M3d3d shotgunner attacks), or 12 (M3d3e chaingunner attack/refire); profile-specific additions are specified below |
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

## DSIM-v5 and bounded combat

`Simulation::combat` / `with_combat_registry` select header version/profile 5.
Versions 1 through 4 remain byte-for-byte unchanged; no savegame migration exists.
V5 retains v4 field order and adds `actor.N.threshold` (i32) immediately after
`.spawnpoint` and before `.momz`. All other implemented persistent combat state
already has explicit fields: health, flags, current state/timer, momentum, height,
reaction, targets, slot generations, linked orders and gameplay RNG. No persistent
inflictor, weapon state, attack queue or artificial combat counter is added.

Event tag 13 adds a combat subtag u8. `ID` below means the existing 12-byte actor
identity, `OptID` the 13-byte optional identity. Fixed values are raw i32 16.16;
all counts/integers have explicit little-endian encoding.

| Subtag | Ordered payload |
| --- | --- |
| 0 Attack | shooter ID, angle u32, range/slope/damage i32 |
| 1 Trace | origin X/Y then delta X/Y i32; u32 cell count then i32 X/Y pairs; u32 candidate count then objects; u32 intercept count then (fraction i32, object); hit presence u8 then optional object; position presence u8 then optional X/Y/Z i32; traversed count u32 |
| 2 Damage | target ID, inflictor OptID, source OptID, amount i32 |
| 3 Random | kind u8, value u8 |
| 4 Knockback | target ID, angle u32, thrust i32, resulting momx/momy i32 |
| 5 Health | target ID, before/after i32 |
| 6 Pain | target ID, state u16 |
| 7 Kill | target ID, state u16, extreme u8 |
| 8 DeathTimer | target ID, tics i32 |
| 9 Reaction | target ID |
| 10 Retarget | target ID, source ID |
| 11 Impact | actor ID |
| 12 Ignored | target ID |

Trace object = tag 0 + line u16, or tag 1 + actor ID. Intercepts are stable sorted
by signed fraction, preserving candidate insertion ties; candidates remain in
source callback order. Trace records precede impact/damage and include no later
result.effect/result.damage fields. Random kinds: 0 forward-fall, 1 pain, 2 kill
timer, 3/4 impact Z left/right, 5 impact timer. P_SpawnMobj lastlook remains an
implicit draw, observable in actor/RNG state and initialization order. Actor-state
entries/actions/targets still use tag 12 at their actual chronological positions.
There is no sound device, renderer state or memory address in these records.

`m3d1_snapshot_goldens.py` reads C captures, explicit inputs and source metadata,
never Rust output. It encodes 48 initial, nine damage API, three shot API and three
settled death captures (63 total), with 17 full byte fixtures and provenance hashes.
The selected due-east damage events use source branch predicates, C RNG/momentum/
state rows and original input thrust; complete shot traces use C geometry/cell/
intercept/hit rows. Settled final snapshots assert no movement/actions and encode
phase/thinker events; C link/removal observations reconstruct spatial/store order.
Field comparison precedes digest comparison for stored byte fixtures. Digests are
noncryptographic schema diagnostics, not proof of complete gameplay equivalence.

## DSIM-v6 and player weapons/psprites

`player_weapons` / `with_weapon_registry` explicitly select version/profile 6.
Versions 1..5 retain their exact field order, bytes and goldens. V6 retains v5
actor fields, including threshold, and adds the following ordered runtime fields:

- Immediately after each `sector.N.actors`: `sector.N.soundtarget`, the existing
  13-byte optional actor identity. Per-call sound traversal caches are derived.
- After all four player command groups, before events: `weapons.player.present`
  u8. When present, append fields under `weapons.player.` in this order:
  `slot` u8, `actor` 12-byte ID, `health` i32, `ready` u8, `pending` u8 (10 means
  source wp_nochange; 9 is NUMWEAPONS), `owned` u16, `ammo` four i32s, `max_ammo`
  four i32s, `attack_down` u8, `refire` i32, `bob` raw fixed i32, `extra_light` i32.
- Then `weapons.player.psprite.N.state` u16 (65535 null), `.tics` i32, `.x`/`.y`
  raw fixed i32, for N=0 weapon then N=1 flash. Sprite/frame derive from the
  immutable registry state; no ordinary thinker identity is invented for psprites.

The profile invariant admits only one LIVE player with protected health. Inventory
setup is explicit; no armor/power/pickup state or attack queue is invented. Slot,
body generation, source ownership/ammo, timers/positions, light, commands, RNG,
actor effects and sound targets capture all implemented persistent consequences.

Event tag 14 adds a u8 weapon subtag. All integer values use explicit little-endian
widths. ID below is 12-byte slot/generation; action IDs reuse the source symbolic
registry table, but dispatch eligibility is separate from actor actions.

| Subtag | Ordered payload |
| --- | --- |
| 0 Entered | slot u8, psprite u8, state u16 (zero records null entry) |
| 1 Action | slot u8, psprite u8, action u16 |
| 2 Ammo | slot u8, ammo u8, before/after i32 |
| 3 Light | slot u8, value i32 |
| 4 Fire | slot u8, weapon u8, refire i32 |
| 5 Bullet | actor ID, angle u32, slope/damage i32 |
| 6 Noise | sector u16, blocking-lines-crossed u8, target ID |
| 7 Initialized | slot u8, actor ID |
| 8 Random | kind u8 (damage=0, spread left=1, spread right=2), value u8 |

Source weapon sounds use existing tag-12 Sound observations; combat retains tag
13 exactly. Entries/actions/effects appear in chronological order. Setup queues
initialization/psprite events, which prefix the first completed tic just like actor
spawns. Read-only queries never drain observations or consume RNG.

`m3d2_snapshot_goldens.py` reads C/setup/schema inputs only and reuses historical
v5 field assembly without editing it. It explicitly adds v6 fields/events, source
player bindings/commands/color angles and sector sound targets. Selected states
are initial, raise/ready/attack-start and settled captures with no shot effects or
motion on that tic; their complete source weapon/lifecycle events are encoded.
Single-burst captures avoid slot reuse assumptions; the per-tic C test separately
checks reused identities and transient firing fields/order. 69 captures and 24 byte
fixtures have independent input/encoder/output hashes and first-field diagnostics.
Hashes remain noncryptographic schema evidence, not full gameplay synchronization.

## DSIM-v7 — Projectile Foundations

Profile/version 7 explicitly selects `Simulation::projectiles` or
`with_projectile_registry`. It composes **v5** field order: typed actor fields,
threshold, XYZ/momentum, generation/link/membership, sector geometry and immutable
registry already represent every persistent plasma property. `ActorLife.target`
is the source owner. Intended launch victim/aim is transient; no tracer or runtime
damage multiplier is introduced. Registry type 34 contains immutable damage/speed
and state definitions; flags/state/tics encode flight versus explosion.

V6-only `sector.N.soundtarget` and `weapons.player` fields are absent in v7. V6 is
a separate player-weapon profile, not a required schema prefix. No implicit profile
conversion or savegame migration exists. Versions 1..6 retain their exact old
fields/bytes/helpers. First comparison across versions reports `schema.version`.

Shared state, combat, sector-movement and Z events retain tags 12, 13, 10 and 11.
New event tag **15** is followed by one u8 discriminator:

| Discriminator | Payload in order |
| --- | --- |
| 0 explosion starts | 12-byte missile identity, before momentum/state changes |
| 1 launch velocity | missile identity; u32 BAM angle; three i32 raw momenta XYZ |
| 2 projectile RNG | u8 kind (0 spawn tics, 1/2 shadow left/right, 3 contact damage, 4 explosion timer); u8 result |
| 4 initial half-step | missile identity; three i32 raw XYZ; i32 randomized spawn tics |
| 5 launch request | source identity; u8 aim mode (0 actor, 1 autoaim); destination identity only for mode 0; u16 projectile type |

Discriminator 3 is unused/reserved and is never emitted. Launch state is queued
between tics; successful effects retain source order. Damage precedes movement
result/explosion; in-thinker explosion precedes the same actor's state countdown.
Actor and candidate references retain their original generation after retirement.

`m3d2b_snapshot_goldens.py` independently encodes 129 C/setup-derived captures
and stores 35 full streams. It covers all 41 setups, complete wall/dropoff
flight/explosion/removal, settled lethal state and both RNG-wrap batches with
slot reuse. No Rust output is read. Transient damage fields/order are tested
against C separately; canonical bytes are project schema evidence, not DOOM
savegames or full synchronization. Rust checks named fields before full bytes
and digest, preserving first-field divergence diagnostics.

## DSIM-v8 — Monster Perception and Target Acquisition

Schema/profile 8 deliberately composes v5 persistent typed actors/static-sector
state, the existing sector sound-target representation and a separate minimal
perception roster. It has no v6 weapon player/psprites or v7 projectile API/event15.
Old schemas 1..7 and their expected bytes/encoders remain unchanged. Existing
actor lastlook, target, threshold, reaction time, state, tics, health and angle
already encode perception consequences; no duplicate fields are added.

In field order, add `sector.N.soundtarget` immediately after each `sector.N.actors`
using the existing fixed 13-byte optional generation identity (u8 presence, u32
slot, u64 generation; absent is 13 zero bytes). Legacy `player.N` bindings/commands
remain absent/zero. Immediately after those four legacy slot groups and before
events, write for N=0..3:

| Path | Value |
| --- | --- |
| `perception.player.N.active` | u8 boolean |
| `perception.player.N.health` | signed i32 little-endian player-record health |
| `perception.player.N.actor` | fixed 13-byte optional generation identity |

Inactive retained IDs remain encoded even after retirement; active IDs must pass
preflight. Sound targets persist independently of assignment/wake. Local REJECT
indices, slope scratch, traversal stack, duplicate masks, validcount, sightcounts
and scan plans are not authoritative persistent fields. Diagnostic events carry
observations of completed operations and do not become inputs to later detection.
Pure sight queries append no events and preserve the entire snapshot.

Existing event tags/phases retain their layouts. Shared noise uses only tag 14,
discriminator 6 (u16 sector, u8 blocks, 12-byte target); other weapon events are not
admitted. New tag 16 is followed by the u8 discriminator below. All integers are
little-endian. `ID` is u32 slot+u64 generation; `OptID` is the fixed 13-byte form.

| Discriminator | Payload in order |
| --- | --- |
| 0 LookStart | ID actor, i32 threshold before reset |
| 1 LookSound | ID actor, OptID sound target, u8 shootable, u8 ambush |
| 2 ScanStart | ID actor, u8 allaround, u8 lastlook, u8 stop |
| 3 Slot | ID actor, u8 slot, u8 active, u8 prior active count |
| 4 Health | ID actor, u8 slot, i32 player-record health |
| 5 Facing | ID actor, u8 slot, u32 relative BAM angle, u8 distance-present, i32 distance only if present, u8 accepted |
| 6 EmptyRoster | ID actor; explicit Rust safety-policy observation |
| 7 ScanEnd | ID actor, u8 found, u8 final lastlook |
| 8 Wake | ID actor, u16 see state |
| 9 Random | ID actor, u8 gameplay result |
| 10 Sight | ID observer, ID target, u8 visible, u32 REJECT pair, u8 rejected, u8 geometry-present; if present i32 eye/bottom/top; u32 visit count then visits |

Each sight visit starts with a u8 code:

| Code | Payload in order |
| --- | --- |
| 1 NodeStart | u16 node, u8 raw side |
| 2 NodeEnd | u16 node, u8 raw side |
| 3 Subsector | u16 subsector |
| 4 Seg | u16 subsector, u32 seg, u16 line, u8 duplicate |
| 5 Sides | u16 line, u8 stage, u8 side0, u8 side1 |
| 6 Opening | u16 line, i32 top, i32 bottom |
| 7 Clip | u16 line, i32 fraction, i32 bottom, i32 top |
| 8 Blocked | u16 line, u8 reason (0 one-sided, 1 closed opening, 2 closed slopes) |

Geometry is absent on REJECT rather than serializing stale C global scratch.
Node side 2 is recorded before normalization. Action/target/sound/state-entry
observations still use established tag 12.

`m3d3a_snapshot_goldens.py` independently encodes 420 C/setup-derived captures and
45 full streams, including pending/transient sight/scan/A_Look/noise, source
thinker phases and RNG wrap. Named fields, full bytes and FNV-1a64 digest are
compared by Rust. Native fixture registry edits are encoded explicitly. This
schema is a project comparison format, not an original DOOM savegame or evidence
of complete vanilla/demo synchronization.

## DSIM-v9 — Monster Pursuit and Movement Decisions

Schema/profile 9 composes v8 typed actor/static-sector state, sector sound targets,
legacy absent/zero player bindings and the distinct perception roster. It adds
only two persistent actor fields immediately after `actor.N.threshold` and before
`actor.N.momz`, for each occupied typed actor:

| Path | Value |
| --- | --- |
| `actor.N.movedir` | u8 source direction 0..7 or NODIR (8); spawn initializes EAST (0) |
| `actor.N.movecount` | signed i32 little-endian; spawn initializes 0 |

Existing angle, flags, health, target, reactiontime, threshold, state/tics and
spatial/sector order are reused. Native registry edits (including missile-state
removal, integer speed, radius, sounds and positive Chase duration) are explicitly
encoded in existing immutable registry fields. Source skills use the existing
actors.options field. fastparm/netgame remain fixed false; no hidden option is
added. v6 weapon player, v7 projectile events/API and player damage are absent.
All schemas 1..8 retain their original fields/expected bytes/encoders.

Temporary axis candidates, movement results, query stamps and local plans are
not persistent authority. Existing events 10 (ordered sector movement),11 (Z),12
(actor state/action/target/sound),14/6 (shared noise) and 16 (perception) retain
their exact layouts. Observations do not become gameplay inputs.

New event tag 17 has a u8 discriminator, then 12-byte actor ID (u32 slot, u64
generation) and payload below, all integers little-endian:

| Discriminator | Remaining payload |
| --- | --- |
| 0 Start | u8 operation |
| 1 Return | u8 operation, u8 boolean result |
| 2 Direction | u8 assigned movedir |
| 3 Random | u8 kind, u8 original lookup value |

Operation 0=Move, 1=TryWalk, 2=NewChaseDir, 3=Chase. Void operations encode false in
Return; this does not invent a source result. Random kind 0=successful walk count,
1=axis priority, 2=fallback ordering, 3=active sound. Source call nesting, assignments,
query results, RNG, state/sound events and returns retain chronological order.
Move NODIR has no collision event. Failed direct moves leave actor constraints/
links untouched. Successful walking relinks immediately before its movement
observation; ordinary momentum XY/Z still precedes the state action.

`m3d3b_snapshot_goldens.py` independently encodes 835 C/setup-derived captures and
stores 49 full byte streams. All captures compare digests; selected streams first
compare named fields, then bytes. Captures include pending/transient callbacks,
source thinker phases, counter/angle/target changes, relinking and RNG wrap. C
ordinals map to generation 1 in these source cases; retirement/reuse is separately
verified by Rust policy tests. This is a project diagnostic comparison schema,
not a DOOM savegame/demo or evidence of complete vanilla synchronization.

## DSIM-v10 — Monster Attack Selection and Bounded Actions

Schema/profile 10 composes v9 fields in the same order: typed actors, health,
states/tics/flags, source target generations, threshold/movedir/movecount, physical
geometry, thinker/spatial/sector links, sound targets and distinct four-slot roster.
It includes shared damage/impact state and exact named registry adaptations. No
new persistent attack result, queue or cached shot plan is serialized. Every
v1-v9 schema/fixture byte remains unchanged. fastparm/netgame stay fixed false.

Existing event tags 10/11/12/13/14/16/17 retain their layouts. New tag 18 has a u8
variant and 12-byte actor identity (u32 slot,u64 generation), followed by:

| Variant | Payload after actor identity |
| --- | --- |
|0 Range |u8 kind (0 melee,1 missile),u8 accepted,u8 reason,optional i32 distance,optional i32 threshold |
|1 Random |u8 kind,u8 lookup value |
|2 Face |13-byte optional target,u32 old/new angle,u32 old/new flags |
|3 Selection |u8 kind (0 melee,1 missile),u16 state |
|4 JustAttacked |u32 resulting flags |
|5 Aim |shared combat Trace payload beginning with its variant1 byte,optional target,i32 slope,i32 bottom/top window |

Optional range integers use a presence byte followed by i32 only when present.
Melee distance/threshold are raw 16.16; missile distance is signed integer map
units after the reached adjustments (final capped value when probability runs).
Reasons 0 accepted,1 null,2 melee distance,3 sight,4 JUSTHIT,5 reaction,6 VILE far,
7 UNDEAD near,8 random rejection. Random kinds 0/1 shadow,2/3 spread,4 damage,
5 missile probability. Source sight uses tag 16 before range; pure public melee
returns evidence without adding events. Selection precedes melee sound/state;
JustAttacked follows missile state actions. Internal PosAttack facing has its own
Face/Random observations without inventing another state-action event.

Aim's reused Trace layout contains origin/delta, ordered cells/candidates, stable
fraction-sorted intercepts, optional stopped object, absent position and traversed
count, then aim target/slope/window. PosAttack uses existing tag 13 Attack/Trace,
real effect and damage observations in source order. No observation drives AI.

The independent Python encoder reads C traces, public inputs, original metadata
and this schema. It maps source birth ordinals to the lowest free slot/current
generation, retaining old references after retirement, and reproduces full events
and canonical link order.1290 captures and 72 full streams include retirement and
reuse; first differing named field precedes full bytes/digest checks. DSIM remains
a project diagnostic format, not a vanilla demo, savegame or network protocol.

## DSIM-v11 — Shotgunner Multi-Pellet Attack Foundations

Version/profile 11 composes the accepted v10 actor, health/state/frame/timer/flags,
target/generation, reaction/threshold/movedir/movecount, geometry, thinker/spatial/
sector links, sound targets and four-slot perception roster. Exact labelled SPOS
registry adaptations replace the fixture animation in this separate profile.
No weapon-player record or persistent pellet/cache/plan is added. V1-v10 fields,
expected bytes and helper schemas remain unchanged; a higher profile number does
not grant all previous gameplay capabilities.

Existing tags 10/11/12/13/14/16/17/18 keep their layouts. Source body state/action,
shotgun sound, facing/shared aim/RNG and real combat/effect events retain their
existing chronological forms. New tag 19 is u8 variant, 12-byte actor identity
(u32 slot,u64 generation), then:

| Variant | Remaining payload |
| --- | --- |
|0 Begin |None; emitted only after the source non-null check, before sound |
|1 PelletStart |u8 source ordinal 1..3, before its three attack RNG draws |
|2 Pellet |u8 ordinal,u32 BAM angle,i32 shared slope,i32 independent damage |
|3 PelletDone |u8 ordinal,u8 gameplay RNG index,13-byte optional real effect ID |
|4 Completed |u8 final gameplay RNG index |

Optional IDs retain presence+u32 slot+u64 generation, zeros when absent. All
integers are explicitly little-endian. A true miss has no effect; null emits no
shotgun event (the existing action-entry observation still records invocation).

Each pellet's tag 18 spread/damage draws precede its tag 19 parameters and existing
tag 13 Attack/Trace/impact/damage. PelletDone follows all those effects and records
the exact source RNG index before the next PelletStart. Shared aim appears once
before the loop. Earlier standalone FaceTarget and internal facing remain separate
observations; internal facing does not invent an extra state-action entry.

The independent Python encoder reads C/public inputs and documented schema only.
Aim and each of three line traces have distinct scopes, including identical miss
results, so candidates/intercepts cannot accidentally be mixed across pellets.
C birth ordinals map to lowest-free slots/current generations; full thinker and
spatial order, effects, retirements and old identities are encoded. 949 captures
and 92 full streams are compared with first-field diagnostics and byte/digest
checks. DSIM is a project diagnostic, not a DOOM savegame/demo/network protocol.

## DSIM-v12 — Chaingunner Attack and Refire Foundations

Version/profile 12 composes the v11 field order: actor state/tics/sprite/frame,
health/flags/target generations, movedir/movecount/reaction/threshold, RNG, geometry,
thinker/spatial/sector order, roster health and sound targets. Exact labelled CPOS
states replace the attack fixtures in this distinct profile. No persistent firing
counter, aim cache or preflight plan is serialized. V1-v11 formats and expected
bytes remain unchanged; profile numbers do not grant cumulative gameplay support.

Existing tags and layouts are retained. Actor-state tag 12 records CPosAttack/
CPosRefire entries using original action IDs 13/14, shotgun sound, immediate see/
Chase/Look actions and physical state changes. Tag 18 retains facing, fresh aim,
spread/damage draws and range selection; its Random kind 6 adds the refire draw
(0/1 shadow, 2/3 spread, 4 damage, 5 missile chance stay unchanged). Tag 16 carries
only reached source sight queries, and tag 13 carries actual shot/damage/effects.

New tag 20 is u8 decision followed by a 12-byte actor ID (u32 slot,u64 generation):

| Decision | Meaning |
| --- | --- |
| 0 | Random below 40: early continuation without target/health/sight evaluation |
| 1 | Null target: immediate see-state handoff |
| 2 | Mobj health <=0: handoff without refire sight |
| 3 | Healthy target failed original sight: handoff |
| 4 | Healthy visible target: natural continuation |

Chronology is action entry, facing/shadow RNG, refire draw, optional sight, decision,
then any immediate see/Chase/Look actions. Nested queries after a handoff remain
separately observable. CPOS's 10/4/4/1 phase is already represented by state/timer;
shots execute on distinct entries with current aim and source RNG.

The independent encoder consumes original-C rows/public setup/schema only and
checks 1,544 captures/110 full streams, real lowest-free-slot/generation mapping,
retirement, fields, bytes and digests. The separate injected-40 dependency probe
is not a gameplay capture and adds no schema field. DSIM remains a project
diagnostic, not a DOOM savegame/demo/network protocol.
