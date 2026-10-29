# Compatibility boundaries

The behavioral authority is original id Software Linux DOOM 1.10. This is an early
headless rewrite supporting specific, limited behavior, not a complete source port.
This describes the code in this checkout, not its merge/review status. This
document owns support restrictions and limits; [ROADMAP](ROADMAP.md) owns
development status and plans. The [glossary](README.md#short-glossary) explains
profile, admission and fixture terminology.
Linux 1.10 comparisons do not prove DOS 1.9 demo synchronization.

## What a compatibility claim means

| Level | Meaning |
| --- | --- |
| Recognition | An original type, state or resource is represented as metadata |
| Validated parsing | Input records and references pass structural/safety checks |
| Supported execution | An explicitly selected configuration accepts specified actors and reachable actions |
| Original-C evidence | Specific inputs match genuine source bodies under declared adapters |
| Full-game compatibility | All enabled dependencies compose in the original game context; not established |

Loading a WAD, recognizing MT_SERGEANT or matching an isolated hash does not
establish the last level. [Reference notes](README.md) describe each admitted
slice and the [validation guide](VALIDATION.md) distinguishes evidence families.

## Versioned gameplay profiles

Historical profile behavior and DSIM bytes remain frozen. New behavior is opt-in.

| Configuration (profile) | Supported foundation |
| --- | --- |
| 1 | Non-solid fixed-point headless movement and command foundations |
| 2 | Grounded player XY sliding and solid collision |
| 3 | Static multi-sector BSP/openings, XY/Z and gravity/dropoff |
| 4 | Explicit type/state lifecycle and bounded synthetic probes |
| 5 | Unbound damage, aim and hitscan |
| 6 | Bounded player weapons and psprites; complete player damage excluded |
| 7 | Bounded original plasma projectile foundations |
| 8 | Sight, ordered player scanning, sound propagation and Look |
| 9 | Non-attacking source pursuit |
| 10–12 | Separately admitted attack-selection, shotgunner and chaingunner fixtures |
| 13 | Demon melee with simplified fixture stand/run loops |
| 14 | Demon pain, ordinary death and persistent corpse lifecycle |
| 15 | Original two-state stand and eight-state run closure |
| 16 | Bounded classic Demon THING spawning, retaining the prior fixture flags |
| 17 | Map kill totals, original COUNTKILL flags and bounded death-credit context |

These are separate contracts, not modes that automatically admit all original
actors or map features. Detailed older limits remain available in the complete
[historical compatibility document](validation/archive/pre-m3d3j-compatibility.md).

## Demon support: remaining dependencies

| Dependency | Map-context configuration (profile 17) support |
| --- | --- |
| Source metadata | Original `MT_SERGEANT` type 12 / editor type 3002 is recognized unchanged |
| Map THING admission | Commercial single-player, normal timing, bounded Demon-only execution |
| Spawn compatibility | Signed fixed coordinates, BSP floor, quantized angle, raw state, original RNG/order |
| Original flags | SOLID/SHOOTABLE/COUNTKILL = 4194310 restored; ambush applied from THING |
| Level statistics | Kills/items/secrets reset to 0; admitted map COUNTKILL placements add available kills |
| Player statistics | Only genuine players[0].killcount fallback for absent/unbound sources; no player-pointer scoring |
| Damage/death | Existing shared services, pain chance 180, delayed sounds, ordinary death/Fall/quarter height |
| Terminal corpse | COUNTKILL retained; respawn disabled, no extra terminal RNG/movecount work |
| Respawn/raise | Explicitly unsupported; no Nightmare respawn or arch-vile resurrection |
| Complete original actor | Deferred: native type 136 identity, state aliases and disabled raise remain adaptations |

Only the old COUNTKILL flag projection is removed. Profile 17 uses
`ActorRegistry::demon_map_context_fixture`, not complete original actor admission.
Available `total_kills` never decreases on death/removal. The separate
`single_player_kill_count` represents source's unbound single-player fallback,
including absent sources, self damage and admitted monster-caused lethal damage.
It is not points, frags or a complete player/intermission subsystem. Other level
totals remain zero because items and sector specials are excluded.

Fresh restart resets the bounded level/actors/statistics while continuing RNG.
It returns a new simulation and does not perform complete game/input/player reload.
No API promises globally unique handles across separate simulation instances.

## Map size and processing limits

A valid parsed map may still be rejected by a simulation configuration. The bounds
below come from the linked implementation, not a promise of real-map gameplay or
unlimited geometry. Four different kinds of limits must be considered separately.

### 1. Original encoding and source semantics

The classic records have fixed-width fields. [Map types](../crates/doom-map/src/types.rs)
preserve signed 16-bit coordinates/heights, unsigned flag bit patterns and
16-bit references. The [parser](../crates/doom-map/src/parse.rs) additionally
validates references according to the selected Linux 1.10 interpretation:

- Ordinary vertex, line, sidedef and sector references must address existing
  records and fit the nonnegative signed 16-bit range. A Rust `u16` storage type
  does not automatically authorize larger source indexes.
- Node children use the high bit to mark a subsector and the lower bits for its
  index. Sidedef `0xffff` means absent; subsector first-seg/count fields have
  signed-range and contiguous-range checks.
- BLOCKMAP dimensions must be positive signed 16-bit values; referenced list
  offsets and line indexes follow signed source constraints. `0xffff` terminates
  a list. These rules can reject data well below allocation budgets.

Changing a budget cannot widen an on-disk field or remove a reserved marker.
Supporting another source interpretation or an extended map format requires an
explicit compatibility decision and, where needed, different representations.

### 2. Parser safety budgets

[MapLimits and Budget](../crates/doom-map/src/parse.rs) provide these defaults:

| Setting | Default | What it bounds |
| --- | --- | --- |
| `max_records` | 32,768 | Decoded records per table |
| `max_lump_bytes` | 64 MiB | Bytes in one map lump |
| `max_blockmap_cells` | 262,144 | Decoded collision-grid cells |
| `max_blockmap_words` | 4 Mi words | Raw BLOCKMAP header/table/list storage |
| `max_decoded_bytes` | 128 MiB | Cumulative requested decoded and scratch allocation |

These are resource ceilings, not guaranteed supported map sizes. Checked
multiplication/addition and fallible reservations enforce the allocation budget.
The parser also hard-caps every record table at 32,768 independently of the
configurable setting. Its uniform application needs table-by-table investigation:
for example, the THINGS record count comes from lump length, not a 16-bit count
field. Neither that policy nor every classic reference is interchangeable with a
universal format limit. Increasing `max_records` alone does not remove the cap.

### 3. Temporary simulation support restrictions

These narrower bounds support the existing arithmetic, ownership and preflight
proofs. They are not classic-map format limits; changing one requires checking
all dependent representations and services.

| Area | Current bound | Implementation |
| --- | --- | --- |
| Collision preparation | 128 lines; 4,096 BLOCKMAP cells; 65,536 BLOCKMAP words | [prepare_cells](../crates/doom-sim/src/movement.rs) |
| Static topology | 128 sectors; 4,096 subsectors; 4,096 nodes | [Topology::new](../crates/doom-sim/src/multisector.rs) |
| Sight-supported geometry | Absolute vertex coordinates, node origins/deltas and sector heights at most 512 map units | [validate_map](../crates/doom-sim/src/sight.rs) |
| Map Demon setup | 256 THINGS, four native Demon test actors, 64 total actor slots | [map spawning](../crates/doom-sim/src/demon_map_spawn.rs), [constructor](../crates/doom-sim/src/demon_lifecycle.rs) |
| Earlier general actor-state setup | 4,096 THINGS and 1,024 actor slots | [actor-state constructor](../crates/doom-sim/src/spawning.rs) |
| Underlying actor store | Configurable, at most 65,536 slots; individual configurations are stricter | [ThinkerStore::new](../crates/doom-sim/src/store.rs) |
| Map Demon arithmetic margins | Absolute spawn coordinates 978, owner query margin 1,008, other actor coordinates 994, general actor/query range 1,024 map units; body initial XY momentum and admitted Z momentum at most 64 units | [map preflight](../crates/doom-sim/src/demon_map_spawn.rs), [spawn checks](../crates/doom-sim/src/spawning.rs), [sight](../crates/doom-sim/src/sight.rs) |

For example, collision and hitscan services contain fixed line-work buffers tied
to the line bound; raising only the rejection constant would not be sufficient.
The [future map-scale milestone](ROADMAP.md#map-scale-limits-and-real-map-readiness)
plans a separately validated audit and expansion. This inventory does not change
runtime capacities.

### 4. Performance and traversal work limits

| Service | Current work bound | Meaning |
| --- | --- | --- |
| Sight | 8,192 units of fully expanded BSP traversal work, including repeated children/segs | [sight work validation](../crates/doom-sim/src/sight.rs) |
| Hitscan | 64 grid-cell iterations; candidate count bounded by `64 * (128 + 1024)` | [combat tracing](../crates/doom-sim/src/combat_trace.rs) |
| State actions | 128 immediate entry work units, 8,192 planned step work units, and bounded child spawning | [lifecycle planning](../crates/doom-sim/src/lifecycle.rs) |

Some work bounds also preserve source semantics: the hitscan cell loop follows
[original P_PathTraverse](../linuxdoom-1.10/p_maputl.c), so expanding it may change
vanilla behavior. Other bounds prevent pathological CPU/memory growth. The future
inventory must distinguish these reasons rather than replace every value with a
large constant. Keep deterministic traversal and checked resource use throughout.

## Demon runtime restrictions

Map-spawned Demon thinkers precede stable unbound type 135 test targets/effects.
Live Demon, perception-roster and sound targets must use that target role. Real
player bindings reject. Demon actors can enter only through the atomic map setup
batch, not ordinary in-tic spawning. Source identifiers/state aliases remain the
documented adaptation in the support matrix above. The [source investigation](M3D3I_REFERENCE.md)
explains its ownership proof. Capacity/work/checked-statistics overflow and stale
identities reject before changing canonical state.

Synthetic cross-native lethal or already-ignored damage is admitted because source
returns before retargeting. Nonlethal cross-native damage rejects; autonomous AI
still cannot acquire native targets. No generalized infighting or projectile
interaction is implied. Source/inflictor identity is validated even for ignored
requests as a Rust safety policy. Unsupported actions never silently no-op.

## Deferred systems

Full player damage/inventory/armor/powers/scoring, multiplayer, Fast/Nightmare
execution, respawning, resurrection, dynamic sectors, doors, specials, complete
map/actor spawning, generalized monster interactions, demos and presentation remain
unsupported. Spectre metadata recognition is not Spectre gameplay compatibility.
Source undefined behavior is not a compatibility target; checked Rust rejection
and adapter-defined evaluation order are recorded separately.

Read [M3D3J_REFERENCE](M3D3J_REFERENCE.md) for exact statistics chronology,
[STATE_FORMAT](STATE_FORMAT.md) for schemas, and [TESTING](TESTING.md) to reproduce.
