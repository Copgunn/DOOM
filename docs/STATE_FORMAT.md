# Canonical diagnostic state format: DSIM versions 1–24

This is a diagnostic/replay comparison format for explicit simulation profiles,
not a DOOM savegame or network/demo format. A field schema change requires an
intentional format-version change and reviewed golden updates. All integers
below are little endian, with explicit widths and two's-complement signed values.
No Rust layout, padding, pointers, addresses, timestamps, paths, or GPU state is
encoded. Shared immutable map storage uses Arc internally; reference counts and
addresses are excluded.

M3f8's execution-scope brands and derived target availability are API observations,
not persistent fields. V20/v21, profile/registry/context schema identifiers, stored
target handles and event meanings remain unchanged. An inert target identity may
remain encoded after retirement; readers must not redirect it to a reused slot.
Decoded diagnostics do not restore a world or grant a context token. See the
[ownership contract](COMPATIBILITY.md#context-and-target-lifetime-contract).

M3f9's process-local InstanceId and qualified-reference envelopes are also excluded.
They have no numeric/byte encoding, and neither DSIM version can restore them or
an executable world. A decoded local ActorId carries no provenance and rejects
qualified import unless a caller explicitly supplies trusted local routing policy.
That provenance milestone introduced no v22, field change or reinterpretation; see the
[provenance contract](COMPATIBILITY.md#game-instance-provenance).

## Framing and field order

Stepping observation is optional: `advance` records the same gameplay/events but
does not capture a Snapshot; `step` captures after that core succeeds. Explicit
capture and encoding retain every historical byte and interpretation, including
v20/v21. No format version or field changes accompany this API split. See the
[stepping contract](COMPATIBILITY.md#stepping-and-diagnostics).

Header: ASCII `DSIM`, u32 schema version (`1` for M2, `2` for M3a, `3` for M3b, `4` for M3c, `5` for M3d1, `6` for M3d2, `7` for M3d2b, `8` for M3d3a, `9` for M3d3b, `10` for M3d3c, `11` for M3d3d, `12` for M3d3e, `13` for M3d3f, `14` for M3d3g, `15` for M3d3h, `16` for M3d3i, `17` for M3d3j, `18` for M3f1, `19` for M3f2, `20` for M3f3, `21` for the original-ID profile),
u64 field count. Each ordered field is
u32 path-byte length, UTF-8 path bytes, u64 value-byte length, then value bytes.
Generated schema paths are shorter than u32 capacity. Value/count lengths fit
u64 on supported Linux/Windows/macOS hosts. No native usize bytes are emitted.

Runtime fields precede static map fields. Runtime order:

| Field/group | Value encoding |
| --- | --- |
| `tic` | u64 completed logical tics |
| `tic_rate`, `profile` | u32 35; u32 profile 1 (M2 flat non-player XY), 2 (M3a player/solid XY), 3 (M3b static-sector XY/Z), 4 (M3c typed actor lifecycle), 5 (M3d1 bounded damage/hitscan), 6 (M3d2 player weapons/psprites), 7 (M3d2b projectiles), 8 (M3d3a perception), 9 (M3d3b pursuit), 10 (M3d3c bounded attacks), 11 (M3d3d shotgunner attacks), 12 (M3d3e chaingunner attack/refire), or 13 (M3d3f Demon melee); profile-specific additions are specified below |
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

## DSIM-v13 — Demon Melee Attack Foundations

Header/version and profile are 13. V13 composes the existing typed actors,
position/momentum/flags/health/state/timer/sprite/frame, target/threshold/reaction,
movedir/movecount, RNG indices, roster/sound targets, store generations and
thinker/spatial order, map/sector and complete registry fields. No persistent
melee result/counter or cached sight/aim is added. V1-v12 definitions and expected
bytes/generators/encoders/helpers remain unchanged.

Native 136 has named SARG Look/Chase 1/2 and copied attack 4/5/6 (sprite 39,
frames 4/5/6,tics 8/8/8,next 5/6/2). Its explicit source-player-body adaptation sets
health 150,mass 400,melee 4,missile 0,attacksound 52. Target 135 retains labelled source
body pain/death/see states 149..173. Full original Demon 12/Spectre 13 execution,
Nightmare timing and actual player binding/damage are excluded.

No new transient tag is necessary:

| Existing tag/layout | Meaning used by v13 |
| --- | --- |
| Actor-state action/layout | Original action 56 A_SargAttack; separate action 20 entries for ATK1/ATK2 |
| 18 Monster Range, kind 0 | Current source melee decision; optional distance/threshold, strict short circuit |
| 18 Monster Random | kind 0/1 separate SHADOW draws; kind 4 one damage draw |
| 18 Monster Face / Selection | Real facing and Chase melee choice before sound/state entry |
| 16 Sight/perception | Existing ordered REJECT/BSP result/visits and awareness/roster effects |
| 13 Combat | Direct Damage request, source/inflictor IDs, knockback, health, pain/kill/timer RNG, reaction/retarget/ignored |
| Existing state/sound/movement/store events | Selection sound 52 before entry 4, real Chase return and source thinker/spatial order |

Null Sarg invocation records the action but no Face/range/damage draw. A miss
records Face/shadow and evaluated range/sight, no damage draw. A hit records
Face, sight/range, kind 4, then immediate shared damage effects. ATK1/ATK2 and
internal Sarg facing remain distinct; source state already expresses 8/8/8 progress.
No hitscan Attack/Trace/Impact is generated by direct melee. The scripted reuse
fixture has a genuine explicit body spawn/retirement; it is not a melee effect.

The independent Python encoder reads original-C/public fixture/schema data only.
Its 1,309 captures/169 complete streams cover multi-tic entries, changed range,
pain/death/RNG and real lowest-free-slot generation reuse. First-field diagnostics
complement full bytes and digests. The 36 radius component vectors are outside
these capture totals. DSIM remains a project diagnostic, not a vanilla savegame,
network format or demo; hashes do not establish complete game compatibility.

## M3d3g DSIM-v14: Demon pain, death and corpse foundations

Version/profile 14 composes the same field families as v13, with explicit native
Demon lifecycle metadata and copied state aliases. All earlier headers, field
order, encoders and expected bytes remain frozen. The existing version 8+ roster
and sector sound fields are present; root profile and DSIM header both encode 14.
No persistent corpse boolean, pain result or duplicate death timer is added.

Native136 copies source Demon properties with aliases: 1 Look, 2 Chase, 4/5/6 SARG
attack, 7/8 pain, 9..14 ordinary death. PAIN2 returns to fixture Chase 2; DIE6 remains
-1 with next0. Doom number is-1, flags 6 omit COUNTKILL, raise/xdeath/missile are0.
All source pain/death sounds, mass, height, chance and other copied fields remain
visible in canonical registry records. Native135 and original blood/puff records
retain their explicit source-body/effect identities; full types 12/13 stay blocked.

Health, flags, height, XY/Z momentum/location, state/tics/sprite/frame, target,
reaction/threshold, generations, thinker/spatial/sector order and RNG already
encode all persistent outcomes. Kill sets CORPSE/DROPOFF and height 14 while SOLID
remains; Fall later clears SOLID; terminal state 14 has tics -1 and retains storage.
Old target identities may remain unresolved after retirement and never alias reuse.

Existing transient tags suffice: ActorState12 Entered/Action/Sound; Combat13
Damage/Random/Knockback/Health/Pain/Kill/DeathTimer/Reaction/Retarget/Impact/Ignored;
existing Monster/Perception/Sight and movement observations retain chronology.
No new event tag is needed to diagnose pain, scream, Fall or terminal entry.

`m3d3g_snapshot_goldens.py` independently derives3,972 captures and 727 complete
byte streams from C/public inputs/schema, including per-tic digests and first-field
comparison paths. The three injected pain-boundary components are excluded from
canonical gameplay captures. DSIM remains project diagnostic data, not a vanilla
savegame, network protocol or proof of full demo synchronization.

## DSIM-v15 — Original Demon Stand/Run State Closure

Profile 15 composes the v14 persistent layout and observation tags. Header schema
and profile numbers are15; every v1–v14 encoder/fixture byte remains unchanged.
No additional persistent actor flag, AI clock or redundant corpse state is added.
Actor state/tics/sprite/frame, original metadata, health/flags/height/momentum,
identity/target, reaction/threshold/directions, thinker/spatial/sector/roster/sound
membership and RNG already describe this behavior.

The immutable fixture registry copies source states 475/476 into1/3, and477–484
into2/15–21. Sprite39, frames0/1 and10/10 stand timing, frames0/0/1/1/2/2/3/3 and
2-tic run timing, actions/misc and every mapped next-state link are retained.
Native 136 is named FIXTURE_DEMON_STAND_RUN_136. Its property aliases, excluded
COUNTKILL/raise and admitted body/effects match the documented restricted slice.
Attack4–6 and pain7–8 return to RUN1/2; death9–14 preserves terminal-1.

Existing Entered/Action/Sound, perception/sight/movement and combat observations
record immediate nested chronology. Repeated frames still have different state
IDs and separate Chase actions. No transient tag is added merely to label a frame.
`m3d3h_snapshot_goldens.py` independently derives 7,186 captures and 1,395 complete
byte fixtures from original-C traces/public inputs/schema, including all loop,
wake/combat/lifecycle transitions and C-observed RNG-wrap boundaries. It never
reads Rust output. DSIM is project diagnostic data, not a DOOM savegame or demo.

## DSIM-v16 — Original Demon Map-Spawn Foundations

Version/profile 16 preserves every v1–v15 field layout and expected byte stream.
It composes the v15 schema and existing persistent fields: raw map THINGS,
ActorOptions, registry metadata, actor type/state/tics/sprite/frame/health/flags,
position/momentum/angle/lastlook, floor/ceiling/sector, target generation, thinker
and spatial order, RNG indexes and existing transient events. No duplicate map-
spawn, original-type, scoring or corpse flag is introduced.

Each map native has the existing `.spawnpoint`: u8 presence1 followed by exactly
10 little-endian bytes (i16 x,y,angle,type; u16 options). Unbound/effect spawns
retain presence0. Original kind3002 remains in spawnpoint/map fields, while the
explicit executable actor is136 and the registry declares the adaptation. Source
filter/THING-index observations are recorded in reference rows/report, not hidden
persistent state. Header and profile carry 16; sound/roster fields compose exactly
as in 15. Raw spawn has no entered-state action event.

`m3d3i_snapshot_goldens.py` independently composes C rows/public input/schema into
3,440 captures and 784 complete byte fixtures, including filtered/multi-THING setup,
angles/floors, source RNG wrap, ordered shared-target strikes and spawn-to-corpse.
Rust checks full field order/value/bytes and per-tic FNV1a64, with first-field
errors. The asserted 135-tic lifecycle digest is `e327c3fbf88cb132`. DSIM remains
project diagnostic evidence, not a vanilla savegame/demo or full synchronization.

## DSIM-v17 — Demon Map Context and Statistics

Profile17 composes v16 fields/ordering with a distinct registry name and source
COUNTKILL flags. Schemas1–16, their encoders, fixtures and Rust helpers remain
byte-identical. This is project diagnostic state, not an original savegame/demo.

Immediately after `actors.options`, before `store.max_slots`, encode four named
fields, each signed32-bit little-endian:

| Field | Meaning |
| --- | --- |
| `level.total_kills` | Available admitted map COUNTKILL placements |
| `level.total_items` | Source initialized item total; remains0 in this slice |
| `level.total_secrets` | Source initialized secret total; remains0 in this slice |
| `level.single_player_kill_count` | Genuine unbound single-player players[0] fallback credit |

All counters begin0. Spawn total increments after timer RNG and before angle/
ambush assignment; death credit occurs after corpse flags/quarter height and
before death-state entry/timer RNG. Neither death nor removal decrements available
kills. No redundant corpse, respawn-disabled or initial-seed field is serialized:
actor flags/state and actual owned RNG indexes already represent admitted state.
Profile17 fixes single-player normal timing and respawn disabled by admission.

Transient event tag21 has a one-byte subtype:0 map available-kill update,1 fallback
death credit. Both then encode actor identity (u32 slot,u64 generation) and signed
32-bit updated total. Events observe the source positions above. They are not
additional persistent statistics. Existing event tags/order retain their meanings.
Head-actor retirement is inside the thinker phase, before any later live Think.

Fresh restart constructs a new instance through the same validated setup path,
clears counters/tic/actors and retains actual RNG streams. Handles remain scoped
to their instance. Checked overflow and recoverable rejection are Rust policies;
unsupported player scoring, respawn, raise and full game setup reject explicitly.

The [independent encoder](../tools/reference/m3d3j_snapshot_goldens.py) derives
complete bytes/digests from C rows and documented schema, never Rust output.
See [source findings](M3D3J_REFERENCE.md), [validation](validation/M3D3J.md) and
[reproduction](TESTING.md).

## DSIM-v18 — Map Scale Foundations

Profile18 deliberately composes the **DSIM-v3 untyped static-sector field layout**,
with header version and `profile` field both u32 value18. It does not inherit typed
lifecycle fields merely because its version is numerically higher. All v1–v17
encoders, fixtures, helpers and encoded meanings remain unchanged.

Field order, integer widths and event tags are those of v3: tic/rate/profile/pause,
owned RNG indexes, store limit/slots/list links and thinker order, untyped physical
actor fields and flags, XY/Z/momentum/constraints/location, spatial link order,
BLOCKMAP policy/omission diagnostic, indexed runtime sector heights/head lists,
four unbound command slots, ordered last/pending events, then original map tables
and source BLOCKMAP words/ranges. No registry, actor-state, perception roster,
weapon or map-statistics fields are added. Existing count/index widths cover the
[declared capacities](COMPATIBILITY.md#map-scale-profile) without widening classic records.

Prepared-list caches, membership words, setup reference budget, timing measurements
and derived storage metrics are excluded. The map is immutable; a different
successful setup reference budget cannot affect later behavior. Actor slot limits
remain encoded by the existing `store.max_slots` field.

The [independent encoder](../tools/reference/m3f1_snapshot_goldens.py) consumes
original-C traces/public fixture inputs and this schema, never Rust output. The
[record](validation/M3F1.md) identifies full-byte and per-tic coverage and its
explicit exclusions. Full fixtures diagnose the first differing named field;
digests are noncryptographic diagnostics, not source-equivalence proof. This is
neither an original savegame nor a vanilla demo format.

## DSIM-v19 — Expanded Map Extents and Spatial Work

Profile19 uses the same untyped static-sector field layout described for v18 and
v3, with header version and `profile` both u32 value19. It adds no persistent fields
or new event tags. Existing physical actors, flags, XYZ/momentum/constraints,
location, thinker/spatial/sector order, owned RNG, command/event fields and original
map tables represent the expanded inputs. Numeric version order does not admit
typed lifecycle, perception, weapons or statistics.

Preparation budgets, membership, root-expansion cache, timing/storage observations
and read-only SightResult diagnostics are excluded. Actor slot limits remain in
`store.max_slots`; successful differences in other setup budgets do not alter
canonical state. All v1–v18 schemas, encoders, helpers and expected bytes remain
unchanged. [The independent encoder](../tools/reference/m3f2_snapshot_goldens.py)
uses original-C physical/order traces, public classic bytes and this schema;
expected snapshots are never generated from Rust. Full fixtures identify the first
different named field; per-tic FNV digests are diagnostics, not vanilla savegames,
demos or independent proof of complete source equivalence. Coverage and exclusions
belong in [the evidence record](validation/M3F2.md).

## DSIM-v20 — Expanded-Map Demon Gameplay Composition

Header version and the `profile` u32 are both 20. This explicitly selects the
v17 typed actor, perception, lifecycle and statistics field layout with the new
profile's admitted larger classic-map tables. It does not inherit v19's untyped
layout, nor add redundant copies of geometry, actors, targets or counters.

Field order remains: tic/rate/profile/pause/RNG; actor options and level counters;
store/slot/actor/thinker state; spatial and static-sector state with ordered sector
actors/sound targets; command bindings/commands and perception roster; last and
pending events; original map tables and registry metadata. Existing v17 widths,
optional identity encoding, spawnpoints, state/tics/sprite/frame, health/flags,
threshold/reaction/movement, XYZ/momentum and statistics/event tags are unchanged.
Sector/line capacities change admitted field counts, not integer wire widths.

No new persistent field or event tag is introduced. The actor execution plan,
transient sight failure/query budgets, root-expansion cache, prepared allocations
and timing/RSS observations are excluded. Failed operations commit no diagnostic
state. All v1–v19 schemas, encoders, helper bytes and expectations remain frozen.

The [main independent encoder](../tools/reference/m3f3_snapshot_goldens.py) and
[additional clipping encoder](../tools/reference/m3f3_clipping_snapshot_goldens.py)
use genuine C rows, public input bytes and documented native identity/state
projection. Expected bytes never come from Rust. Full fixtures diagnose the first
named field; every capture also compares complete ordered event-field bytes with
first-byte diagnostics. Event projections are separate from full DSIM fixtures.
Per-tic FNV digests provide compact checks. DSIM is not a vanilla
savegame/demo. [Evidence](validation/M3F3.md) distinguishes C-derived captures from
separate independently specified Rust-only capacity states.

## Original identity metadata observations

M3f4 changes no DSIM version, profile, field or historical fixture. Canonical
identity views are read-only metadata, not additional simulation state. Existing
registry-local identifiers remain encoded exactly as before. The separate
`DOOM-IDENTITY-v1` text observer begins with its schema tag and counts, followed
by ordered type/state/sprite records; it is not a DSIM capture or demo format.
The [mapping guide](ACTOR_IDENTITIES.md) documents interpretation, and the
[source observer](../tools/reference/m3f4_reference.py) defines emitted columns.

## Canonical Demon execution observations

M3f5 adds no DSIM schema, profile, persistent field or event tag. Raw ActorLife
and DSIM-v20 continue encoding native type 136 and historical state aliases.
Canonical definition queries are derived observations, excluded from snapshots.

The separate text protocol `DOOM-DEMON-EXEC-v1` prefixes the original-C trace and
spawn files with that schema line. Its tagged rows inherit the column definitions
in the unchanged [M3f3 observer](../tools/reference/m3f3_reference.py) and
[source extractor](../tools/reference/generate_m3d3j.py), but retain raw original
type 12/SARG identities. The [new encoder](../tools/reference/m3f5_snapshot_goldens.py)
explicitly projects only identity-bearing A/E/K/J/G fields into native Profile20
slots before invoking the unchanged independent DSIM encoder. Its provenance lists
every mapping and input/output hash. The raw C files are retained separately.
This is source-derived projected evidence, not a claim that native IDs equal
original ordinals. Historical captures remain unchanged.

## Typed native identity observations

M3f6 changes no DSIM version, profile tag, registry metadata, field path, width,
value or event encoding. NativeDemonType/NativeDemonState are derived views, excluded
from serialization. ActorLife.actor_type/state and state-bearing events retain the v20
native slots. M3f6 itself introduced no v21 stream or decoder. Historical
encoders/readers and fixtures keep their existing interpretation. The
[runtime audit](M3F6_REFERENCE.md) identifies the exact observable fields that a
opt-in original-ID contract must version rather than silently reinterpret.

## DSIM-v21 — Original-ID Demon diagnostics

Version21 changes actual definition/state/event values, supplies explicit registry
and context identifiers, and omits redundant immutable registry rows. It is an
opt-in diagnostic representation; it is not a savegame or executable world loader.
No historical version is redefined. Existing readers/helpers for 1–20 remain valid;
a v20-only reader rejects21.

### Identifiers and field layout

Framing is unchanged: DSIM + u32 version 21 + u64 field count, followed by ordered
u32 path length/UTF-8 path/u64 value length/value records. Runtime fields compose
the version 17 families, as v20 does, with these explicit differences:

| Field or family | V21 contract |
| --- | --- |
| `tic`, `tic_rate` | u64 completed tic, u32 value 35; unchanged |
| `profile` | u32 execution policy 1 = `DemonNormalV1`; independent of format21 and Rust enum ordering |
| `registry.schema` | Exact UTF-8 bytes `DOOM-INFO-1.10-v1`, immediately after profile |
| `context.schema` | Exact UTF-8 bytes `UNBOUND-BODY-v1`, immediately after registry schema |
| `.type` and initialized-state event type | u16 definition slot: source ordinals0–136 or explicit context 137; only admitted roles may occur |
| `.state` and state-bearing events | u16 actual original state ordinal; normal SARG475–495, not native1–21 |
| Absent current state | u16 65535; distinct from original S_NULL0 |
| All actor handles | u32 slot + u64 generation (12 bytes); optional handles have one presence byte plus12 bytes |
| Registry counts/records/actions | Not emitted; derived from identified immutable source/context packages |
| Remaining runtime/map/event fields | Same explicit widths and order as v20, complete map fields retained |

The registry identifier fixes all original actor/state/sprite/action metadata of
the preserved Linux DOOM1.10 package. `OriginalExecutionRegistry::admit` checks the
complete records; changed metadata requires a new schema. Action IDs in event
payloads retain the existing Rust dispatch encoding fixed by this package, not
invented original-C action ordinals. There are no serialized function pointers.
The context package fixes one synthetic unbound body, its definition slot derived
from original actor count, and the descriptor specified in
[identity mapping](ACTOR_IDENTITIES.md#original-id-execution-registry). It does not
claim original player identity. Source135/136 remain original decorations.

State-entry event subtype1, perception SeeState subtype8, monster attack selection
subtype3 and combat Pain/Killed subtypes6/7 carry source states. Initialized type
subtype0 carries the original/context definition slot. Other event payloads,
chronology, raw RNG observations/indexes, statistics and map/physics fields retain
the shared schema. Referenced handles may be stale because an event describes
history; they must never resolve to a different generation. No second canonical
identity field is added to runtime storage or diagnostics.

### Bounded encoding and reading

`Snapshot::diagnostic_bytes(DiagnosticLimits)` preflights byte/field/path/value
counts with checked arithmetic and uses a fallible reservation before encoding.
The existing trusted `canonical_bytes()` convenience API keeps its prior process
allocation policy. `Diagnostic::decode` explicitly accepts20/21;
`decode_v20` accepts only20. Unknown versions and incompatible profile/registry/
context identifiers reject, with no fallback reinterpretation. Versions1–19 use
their unchanged historical readers and are not accepted by this new reader.

Default budgets are 64 MiB total/value,262144 fields and192 path bytes. These are
diagnostic input/output budgets, not increased simulation capacities. Lengths,
UTF-8, duplicate paths, truncation/trailing data, integer overflow and allocation
failure reject. Field-count reservations require enough remaining framing bytes.
The reader borrows values and sorts only a bounded lookup index; encoded actor,
thinker and event order is preserved. Raw nested sight/collision payload counts
are not used to allocate or traverse untrusted arrays.

The read-only view validates required version/profile/schema fields, source/context
role and current-state closure, current generations/occupied slots, handle widths,
bounded handle slots, event counts/tags and the identity-bearing event fields.
V20 retains native interpretation and validates its complete embedded registry
against the legacy package. `actor(handle)` resolves only an occupied slot with
matching generation. Optional unavailable/stale target references do not redirect.
`encode` rechecks caller budgets and preserves exact input bytes/order.

This is deliberately a diagnostic reader: non-identity event bodies and physical/
map values are opaque, and successful decode does not prove canonical full-field
shape, executable state, coherent geometry or restorable simulation. Unknown field
paths can be retained for inspection; unknown format/profile/schema identifiers
cannot. It never constructs a Simulation. Error-policy, malformed-input and
namespace tests are Rust-only safety evidence, separate from C compatibility.

### Independent evidence and historical reading

[M3f7 encoding](../tools/reference/m3f7_snapshot_goldens.py) consumes unchanged
M3f5 compiled-C rows, preserving actual type 12/source SARG states. Only the
observer's explicitly synthetic body 135 binds to the context definition. Immutable
metadata is omitted by schema; profile/schema/framing are independently encoded.
A compact digest index and two complete streams avoid duplicating old raw traces
or EV20 data. Ordered event tests reuse old EV20 bytes and replace only identity
payloads from actual C rows, not native-to-original runtime tables. These are new
encodings of reused inputs, not new C behavior coverage. [Observed results](validation/M3F7.md)
own exact counts and size. All v20 expectations/readers remain byte-for-byte intact.

## DSIM-v22 — Original player body

V22 was introduced for PlayerBodyLifecycleV1 (wire policy2), because genuine
player ownership/attacker/health/state observations change the schema. The original
policy2 contract below remains unchanged; separate grounded and command policies follow. It preserves
DSIM framing and the immutable original registry ID `DOOM-INFO-1.10-v1`; context ID
is exactly `PLAYER-BODY-1-MEDIUM-NOTHRUST-v1`. Historical v20/v21 bytes, identities,
encoders and readers remain unchanged. No reinterpretation or world restoration.

Runtime fields compose the v21/version17 families with these changes:

| Ordered location | V22 representation |
| --- | --- |
| Header / profile | u32 version22 / u32 execution policy2 |
| registry.schema, context.schema | Exact IDs above, in the existing positions |
| Occupied actor after movecount, before momz | actor.N.player: u8 source player index0 if its full handle equals mo, otherwise255; derived, not another stored identity |
| Synthetic perception.player.* family | Omitted; this policy has an unlisted genuine body, no synthetic AI roster |
| After four historical command-slot families, before events.last | original_player.0.mo (optional handle), health (i32), state (u8 PST_LIVE0/PST_DEAD1), attacker (optional handle), damage_count (i32), frags (i32), in that order |
| Existing level.single_player_kill_count | Sole source players[0].killcount, no duplicate player counter |
| Definitions / states | Actual original0/12/37/38 and root-admitted source states; context137 is not executable here |

OriginalPlayer fields are present even before binding, initially mo/attacker absent,
health100, Live, damagecount/frags0. The four old `player.N.actor` command binding
families remain distinct and empty, with zero commands under policy 2. mo and attacker retain
stored handles when unavailable. The derived actor association requires exact slot
AND generation; no stale handle points at a reused body. InstanceId remains excluded.
Immutable metadata is identified, never serialized redundantly.

Event tag22 was introduced in v22 and is retained in v23. Player slot0 is fixed by the policy:

| Subtype | Ordered payload after tag/subtype |
| --- | --- |
| 0 Bound | 12-byte body handle, after existing Spawned/Initialized observations |
| 1 Damaged | i32 clamped player health, 13-byte optional attacker, i32 damagecount, i32 tactile amplitude; before existing mobj Health event; tactile period40/duration10 fixed by schema |
| 2 Died | i32 frags[0], u16 original requested S_PISTOLDOWN state; after source player-dead/solidity changes and before existing Kill/state entry |

Other ordered event bytes and identity meanings remain as previously specified.
The psprite request state is metadata for a service boundary, never permission to
execute that state as a mobj. Tactile is an observation, not a device implementation.

`Diagnostic::decode` retains its v20/v21-only contract and rejects22. Explicit
`Diagnostic::decode_player` accepts only22 through the same bounded framing parser,
validating policy/schema, admitted identities/states, handle widths/generations,
player health/state/association, available attacker roles and new event payloads.
Malformed/truncated/duplicate/over-budget data reject. Stale references remain
opaque, not redirected. `Snapshot::diagnostic_bytes` supports20–22 with existing
fallible size budgets. Trusted canonical_bytes remains available. As before,
physical/map and unrelated event data are not executable restoration proof.

[Independent encoding](../tools/reference/m3g2_snapshot_goldens.py) uses new genuine
C player/body observations and separately declared Rust birth/generation schema
policy. Complete event bytes cover every tic; selected state captures, compact
indexes and four complete fixtures avoid broad raw-trace growth. Expected bytes
never come from Rust. [The evidence record](validation/M3G2.md) owns exact counts.

## DSIM-v22 grounded-player execution policy

PlayerBodyKnockbackV1 selects wire policy 3 with exact context schema
`PLAYER-BODY-1-MEDIUM-GROUNDED-v1`. It uses the same version 22 framing, ordered
fields, source player/mo/attacker association and event widths as stationary
policy 2. The original registry schema remains `DOOM-INFO-1.10-v1`. Position,
momentum, existing movement/slide and CombatEvent::Knockback payloads already
represent every required observable; no version 23 or new event tag is necessary.

Policy 2 must pair with `PLAYER-BODY-1-MEDIUM-NOTHRUST-v1`, and policy 3 with the
new grounded schema. Mixed/unknown pairs reject. `Diagnostic::decode_player`
explicitly validates these two policy/context pairs; the historical v20/v21 decode entry
point still rejects v22. Neither old context definitions nor native identities are
reinterpreted. Existing policy 2 full bytes remain immutable.

The [new independent encoder](../tools/reference/m3g3_snapshot_goldens.py) uses
C physical/relationship/collision observations plus explicit schema policy. Its
reusable [player projection](../tools/reference/player_snapshot.py) retains full
DSIM bytes for selected fixtures. Test-only `PBE1` event evidence is compact:
4-byte magic, repeated u32 case/op/last-count/pending-count, followed by each
last then pending event's u32 length and exact payload. Field names are derived
from family/index; both counts and every ordered payload are compared. This is
a separately versioned test-fixture encoding, not a DSIM format change or a demo.

Decoding stays read-only, bounded and distinct from restoring an executable world.
Physical/map fields and unrelated event payloads retain their documented opaque
validation limits. Source equivalence is established by the declared comparisons,
not merely by successful decoding or canonical projection.


## DSIM-v22 player-command execution policy

PlayerCommandMovementV1 selects wire policy 4 and exact context schema
`PLAYER-COMMAND-1-MEDIUM-GROUNDED-v1`, with registry `DOOM-INFO-1.10-v1`. Framing,
field order/widths and original player association remain unchanged. Existing
player.0.forward/side/angle_turn/buttons contain admitted input; the remaining
three slots stay default, and historical player.N.actor bindings stay empty.
Existing actor angle/momentum/reaction/state/timer fields describe the outcome.
No persistent field, DSIM version or event tag is added.

The Players phase now precedes command work and the Thinkers phase follows it.
Existing tag4 Player(slot0,body handle) observes completed P_MovePlayer, after any
immediate RUN entry; reaction-suppressed or inactive bodies emit no such event.
All ordinary thinker/movement/combat events retain their meanings. Old policies
2/3 retain their previous bytes, phase order and exact context pairing; v20/v21
retain their historical interpretation. Mixed/unknown policy/context pairs reject
through explicit decode_player; the historical decode entry point still rejects22.
The reader validates bounded structural/identity/player contracts, not complete
command arithmetic, map physics or executable-world restoration.

The [independent command encoder](../tools/reference/m3g4_snapshot_goldens.py)
combines actual C command/phase observations with frozen field/event primitives
through a reusable player execution projection. It preserves PBE1 test framing,
reports birth/generation projection separately from C pointer behavior and never
uses Rust outcomes as expectations. See [observations](validation/M3G4.md).

## DSIM-v22 active-player Demon policy

`PlayerDemonAcquisitionV1` selects wire policy5 and exact context
`PLAYER-DEMON-1-MEDIUM-GROUNDED-NOMELEE-v1`, paired with `DOOM-INFO-1.10-v1`.
Framing, widths, field order and event tags remain DSIM22. Existing original_player
mo/health/state/attacker fields and derived actor.player association describe the
same canonical record. Fixed active source slot0 and inactive slots1–3 are part
of this immutable context contract; no independently mutable roster health/actor
copy or redundant immutable activity field is serialized. The historical
perception fixture array is not encoded in player policies.

Existing actor target, lastlook, angle/momentum, state/tics, reaction/threshold,
command, RNG, statistics and ordered events describe acquisition and nonmelee
pursuit. Players/Thinkers phase ordering and Player command events retain policy4
meanings; the existing Perception, Pursuit, Monster range, state/sound and movement
events retain their source chronology. No version increment or event reinterpretation
is required. Old v20/v21/v22 policies2/3/4 retain their bytes and identifiers.

The explicit decode_player reader recognizes only matching policy/context pairs
and validates original identities against this policy's admitted states. Unknown
or mixed pairs reject. SARG attack metadata exists but has no executable permission
under this policy. Structural reading still does not establish executable geometry,
roster liveness, provenance or a restored world. Setup and retained-reference
observations remain distinct from permission to advance.

The [independent encoder](../tools/reference/m3g5_snapshot_goldens.py) verifies
actual C roster R rows against P/AP ownership and health before using frozen field/
event primitives. Complete PBE1 event frames and selected full fixtures preserve
all ordered payloads. Birth/generation projection is independent Rust schema policy,
not C allocator identity. See [evidence](validation/M3G5.md) and
[source/harness qualifications](M3G5_REFERENCE.md).

## DSIM-v22 autonomous Demon-player melee policy

`PlayerDemonMeleeV1` selects wire policy6 and exact context
`PLAYER-DEMON-1-MEDIUM-GROUNDED-MELEE-v1`, paired with `DOOM-INFO-1.10-v1`.
Framing, field order, widths and event tags remain DSIM22. Existing player/mo,
attacker, actor target/lastlook, angle/momentum, health/state/tics, flags, reaction,
statistics, RNG and ordered Monster/Combat/PlayerBody events describe the admitted
melee without new persistent fields or redundant immutable metadata.

Fixed source activity remains player0 active and slots1–3 inactive, declared by
this context rather than serialized as another mutable roster. Death does not
change activity or mo ownership. Actor source identities and SARG states remain
original ordinals. Policy6 permits normal attack states485–487; policy5 retains its
no-melee context and rejection. All previous v20/v21/v22 bytes and meanings remain.

`decode_player` validates exact policy/context pairs and their admitted definition/
state roles. Mixed or unknown pairs reject; replacing both identifiers with the
no-melee pair still rejects a captured SARG attack state. The old decode entry
continues rejecting22. Structural decoding supplies neither safe execution,
geometry admission, incarnation provenance nor executable-world restoration.

The [independent encoder](../tools/reference/m3g6_snapshot_goldens.py) checks source
roster/player/association rows and uses frozen generic field/event primitives.
The separately named source capture `DOOM-PLAYER-DEMON-MELEE-v1` and new PBE1 event
file preserve complete event order without rewriting older captures. Birth/generation
projection and invalid-input rejection remain Rust schema/safety policy. See
[source qualification](M3G6_REFERENCE.md), [reproduction](TESTING.md#autonomous-demon-player-melee)
and [observations](validation/M3G6.md).


## DSIM-v23 genuine-player pistol

V23 is required by player-owned persistent weapon/psprite observations. It retains
the v22 genuine-player field/event families and framing, selects wire policy7 and
exact context `PLAYER-DEMON-1-MEDIUM-GROUNDED-PISTOL-v1`, with unchanged registry
`DOOM-INFO-1.10-v1`. Earlier20/21/22 layouts, readers and policy meanings are
immutable. `Diagnostic::decode_pistol` reads23 explicitly; the historical decode
and decode_player entry points continue rejecting it.

After `original_player.0.frags`, before `events.last.count`, append these fields
in the stated order. Prefix is `original_player.0.pistol`:

| Suffix | Representation |
| --- | --- |
| present | u8 bool; if0, no remaining pistol fields follow |
| bullets | u16 source am_clip in0..200 |
| attack_down | u8 source bool |
| refire | i32 source counter in0..200 |
| bob | i32 raw source fixed-point value in0..0x100000 |
| extra_light | i32 source light0/1 |
| psprite.0.state, .tics, .x, .y | u16 original state or65535 forNULL; i32 countdown and raw fixed-point XY |
| psprite.1.state, .tics, .x, .y | Same fields, flash after weapon |

Immutable ready=Pistol, pending=nochange, owned=fist+pistol and original ammo
capacity derive from policy7 rather than another serialized player record.
Original body0, Demon12 and effects37/38 retain actual source IDs. Player mobj
attack states154/155 are executable under this capability; policy8 reuses them separately. Weapon overlays
10..17 and immediate LightDone1/Null0 are not actor states or thinker instances.

Persistent weapon states are source pistol ready/up/down/attack; flash is17 or
NULL. Timers are checked against canonical source descriptors. Null flash retains
a zero timer and source-maintained coordinates; it does not restore another
weapon. Decoding bounds bullets, refire, bob, light, coordinates, channel-specific
states and exact schema/policy pairs. Death retains the body and weapon ownership,
with a lowering pistol. Setup snapshots may have present0 before initialization.

Existing event tag14 weapon subtypes and payload widths remain unchanged. V23
validates the admitted pistol state/action identities, ammo changes, source
5/10/15 bullet damage, ordered draw categories and one-sector noise payload.
Source initialization and psprite entry are real events, not synthetic firing
requests. Tag22 Died remains the same boundary, now followed immediately by
actual tag14 lowering entries/actions before existing Kill/death-state events.
An ActorState Entered S_NULL event can mark an impact thinker for removal; v23
explicitly recognizes that sentinel without granting live S_NULL execution.

Read-only source aim queries are compared separately; no monster-aim event is
invented for them. Existing Attack/Trace/Impact/Damage and player/movement/AI events
encode actual source chronology. PBE1 remains compact independent test-evidence
framing, not a savegame format. Instance provenance and context tokens remain
excluded. Diagnostic decoding observes fields and identity boundaries; map,
physical values and other nested payloads retain their declared opaque limits.
It never restores executable worlds or grants actor ownership.

[Support](COMPATIBILITY.md#genuine-player-pistol-combat),
[source qualification](M3G7_REFERENCE.md), [reproduction](TESTING.md#genuine-player-pistol)
and [evidence](validation/M3G7.md) identify the independent bytes and limits.

## DSIM-v24 original player weapon expansion

Version24 selects policy8 (`PlayerDemonWeaponsV1`) and context
`PLAYER-DEMON-1-MEDIUM-GROUNDED-WEAPONS-v1`, with the original registry schema.
This separate layout is necessary because ready/pending/owned and shell counts
are dynamic; v23's immutable pistol assumptions remain unchanged. The explicit
`Diagnostic::decode_weapons` reader handles24; older readers reject it. V20–23
bytes, readers, event meanings and execution-policy/context pairs are preserved.

Replace the v23 pistol-only group with prefix `original_player.0.weapons`, after
`original_player.0.frags` and before `events.last.count`, in this order:

| Suffix | Representation |
| --- | --- |
| present | u8 bool;0 means no other weapon fields follow |
| ready, pending | u8 original weapon ordinal0..3; pending10 means wp_nochange |
| owned | u16 original weapon bits0..3; fist/pistol required; ready/pending must be owned |
| bullets, shells | u16 am_clip0..200, am_shell0..50 |
| attack_down | u8 bool |
| refire | nonnegative i32, checked on source increment |
| bob, extra_light | i32 raw fixed0..0x100000; i32 light0..2 |
| psprite.0.state, .tics, .x, .y | u16 original state or65535 NULL; i32 countdown/raw XY |
| psprite.1.state, .tics, .x, .y | Same, flash after weapon |

There are18 fields when present. Capacity and excluded cell/rocket ammo remain
immutable policy data. Original overlays are fist2–9, pistol10–17, shotgun18–31,
chaingun49–56, with immediate Null0/LightDone1. Persisted weapon states must match
ready and canonical positive timers; flash identity/light and coordinate bounds
are checked. Source Chain3 is zero-tic and appears only as an entry/action event.
Dead-player weapon state must be the actual ready downstate. No health, actor
binding, immutable original metadata or process-local provenance is duplicated.

Existing tag14 widths/order remain. Version24/policy8 admits the four named weapon
state/action closures, ammo0/1 decrements, lights0..2, nonnegative refire and fist
or bullet damage domains. Tag22 Died keeps its width and records the actual
ready weapon's downstate. V23 retains pistol-only validation. Other nested payloads
retain their documented bounded/opaque handling. Decoding validates diagnostic
observations, never restores an executable world or grants ownership.
