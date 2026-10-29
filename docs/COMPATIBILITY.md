# Compatibility boundaries

The behavioral authority is original id Software Linux DOOM 1.10. This is an early
headless rewrite with explicitly admitted slices, not a complete source port.
Linux 1.10 comparisons do not prove DOS 1.9 demo synchronization.

## What a compatibility claim means

| Level | Meaning |
| --- | --- |
| Recognition | An original type, state or resource is represented as metadata |
| Validated parsing | Input records and references pass structural/safety checks |
| Admitted execution | A named profile accepts a bounded actor/action closure |
| Original-C evidence | Specific inputs match genuine source bodies under declared adapters |
| Full-game compatibility | All enabled dependencies compose in the original game context; not established |

Loading a WAD, recognizing MT_SERGEANT or matching an isolated hash does not
establish the last level. [Reference notes](README.md) describe each admitted
slice and the [validation guide](VALIDATION.md) distinguishes evidence families.

## Versioned gameplay profiles

Historical profile behavior and DSIM bytes remain frozen. New behavior is opt-in.

| Profile / DSIM | Admitted foundation |
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

## Demon admission: remaining dependencies

| Dependency | Profile17 status |
| --- | --- |
| Source metadata | Original MT_SERGEANT12 / DoomEd3002 is recognized unchanged |
| Map THING admission | Commercial single-player, normal timing, bounded Demon-only execution |
| Spawn compatibility | Signed fixed coordinates, BSP floor, quantized angle, raw state, original RNG/order |
| Original flags | SOLID/SHOOTABLE/COUNTKILL =4194310 restored; ambush applied from THING |
| Level statistics | Kills/items/secrets reset to0; admitted map COUNTKILL placements add available kills |
| Player statistics | Only genuine players[0].killcount fallback for absent/unbound sources; no player-pointer scoring |
| Damage/death | Existing shared services, pain180, delayed sounds, ordinary death/Fall/quarter height |
| Terminal corpse | COUNTKILL retained; respawn disabled, no extra terminal RNG/movecount work |
| Respawn/raise | Explicitly unsupported; no Nightmare respawn or arch-vile resurrection |
| Complete original actor | Deferred: native136 identity, state aliases and disabled raise remain adaptations |

Only the old COUNTKILL flag projection is removed. Profile17 uses
`ActorRegistry::demon_map_context_fixture`, not complete original actor admission.
Available `total_kills` never decreases on death/removal. The separate
`single_player_kill_count` represents source's unbound single-player fallback,
including absent sources, self damage and admitted monster-caused lethal damage.
It is not points, frags or a complete player/intermission subsystem. Other level
totals remain zero because items and sector specials are excluded.

Fresh restart resets the bounded level/actors/statistics while continuing RNG.
It returns a new simulation and does not perform complete game/input/player reload.
No API promises globally unique handles across separate simulation instances.

## Explicit runtime restrictions

Profile17 retains at most256 THINGS, four native Demons and64 actor slots. Native
thinkers form a prefix before stable unbound type135 targets/effects. Live native,
roster and sound targets must use that body role. Full player bindings reject.
Native direct spawning is excluded; only the atomic setup batch admits them.

Conservative bounds include native spawn coordinates978 map units, owner query
margin1008, other actor994, body initial XY momentum64, general query1024 and sight
work8192. Geometry and state/action closures retain prior explicit limits. See the
[source investigation](M3D3I_REFERENCE.md) for the proof behind those bounds.
Capacity/work/checked-statistics overflow and stale identities reject atomically.

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
