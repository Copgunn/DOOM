# Rust DOOM

An independent Rust-native rewrite of the original DOOM engine, focused on
deterministic simulation and source-compatible gameplay. Development is early:
the engine runs limited simulation examples without graphics, not a complete game.

> **Machine-Maintained, Human-Directed Project**
>
> This project is developed and maintained primarily by AI coding agents under
> human direction and oversight. Agents implement code, write tests, maintain
> documentation, and assist with development workflows.
>
> Changes are subject to automated validation and human-defined review
> requirements. AI-generated code can contain mistakes, and passing tests does
> not guarantee correctness. This project is not affiliated with id Software or
> Bethesda.

## Status

The Rust workspace provides WAD and classic-map parsing, fixed-point arithmetic,
original random-number streams, ordered actors and explicitly selected simulation
configurations. The Demon example covers synthetic map spawning, stand/run,
perception, pursuit, melee, pain, death, persistent corpses and map kill totals.
Historical Demon profiles retain their explicit native identity binding. An
opt-in original-ID registry/profile stores original Demon types and states directly,
using the same bounded execution services and an explicitly versioned diagnostic
contract. Scoped actor access adds explicit context admission and target-lifetime
observations over that same owner. The restricted game context remains.
An owning game-instance boundary qualifies references across execution scopes and
rejects foreign or pre-restart references. Its opaque provenance is process-local;
diagnostics remain local observations, not portable actor references or savegames.
See [compatibility](docs/COMPATIBILITY.md) for the exact boundary.

A separate opt-in policy now executes one genuine original MT_PLAYER body with
checked player/mo ownership, direct damage, pain, death and retained corpses.
It shares the actor owner and combat/stepping core; synthetic bodies remain
separate. Explicit capabilities add physical knockback and grounded collision,
then command turning/thrust and reaction-time delays through the same movement
services. A separate active-player capability lets a canonical Demon acquire that
player visually and pursue through the shared thinker pass. An additional combat
capability executes source-timed Demon melee, movement misses, physical damage,
pain, death and retained corpses. The acquisition-only policy keeps its explicit
melee rejection. A separate pistol capability adds original psprite cadence,
ammunition, refire spread, real hitscan impacts, one-sector noise alerts and
Demon damage/pain/death through that same combat loop. Older policies remain
available. The four-weapon capability adds source fallback, genuine fists, manual
switching, seven-pellet shotgun and chaingun firing in the same world. A separate world-pickup capability adds original bullet/shell supplies,
stimpacks, medikits and world shotgun/chaingun acquisition through shared collision
and inventory services. Full player-thinker/view processing, remaining weapons,
armor/powers and respawn remain excluded. See the
[player contract](docs/COMPATIBILITY.md#original-player-body-lifecycle) and
[command movement boundary](docs/COMPATIBILITY.md#original-player-command-movement) and
[active-player/Demon contract](docs/COMPATIBILITY.md#active-original-player-and-demon-acquisition)
and [autonomous melee boundary](docs/COMPATIBILITY.md#autonomous-original-demon-player-melee).
See the [genuine pistol contract](docs/COMPATIBILITY.md#genuine-player-pistol-combat)
for its separately admitted inputs, geometry, ammunition and source limits.
The [weapon expansion contract](docs/COMPATIBILITY.md#original-player-weapon-expansion)
describes controlled loadouts and the additional weapon actions. The
[world pickup contract](docs/COMPATIBILITY.md#original-world-pickups) defines the
separate source touch, collection and lifetime capability.

Call `Simulation::advance()` for a tic without an implicit diagnostic Snapshot.
`step()` retains its Snapshot return, and `snapshot()` captures explicitly in
debug or release. Both stepping APIs run the same gameplay and retain events and
recoverable-error guarantees; gameplay may still allocate. See the
[stepping contract](docs/COMPATIBILITY.md#stepping-and-diagnostics).

## What works

- Validated WAD directories and classic maps, including their spatial partitions
  (BSP) and collision grids (BLOCKMAP).
- Deterministic 35 Hz headless simulation with [canonical identity metadata](docs/ACTOR_IDENTITIES.md).
- Static-sector movement, collision, actor states and selected combat services.
- Original-C comparisons, independently encoded canonical snapshots, and safety
  tests for rejection, stale references and recoverable-error atomicity.
- Opt-in larger static geometry, including bounded Demon gameplay composition; see [supported limits](docs/COMPATIBILITY.md#expanded-map-demon-profile).
- Synthetic examples that run without commercial game data or presentation devices.

## What does not work yet

There is no game renderer, audio playback, complete player gameplay, general map
loading, complete original monster support, multiplayer or verified vanilla demos.
Doors, dynamic sectors, resurrection and Nightmare respawning are excluded.
A custom Vulkan-first rendering architecture is a future direction; rendering
runtime is not implemented. The [roadmap](docs/ROADMAP.md) describes dependencies.

## Getting started

Install Rust through rustup. The workspace uses stable Rust and supports Rust
1.85.0. From the repository root:

```sh
cargo build --workspace --locked
cargo test --workspace --locked
```

## Running a headless example

```sh
cargo run -p doom-sim --example demon_expanded_map --locked
cargo run -p doom-sim --example canonical_demon --locked
cargo run -p doom-sim --example runtime_identity --locked
cargo run -p doom-sim --example original_registry --locked
cargo run -p doom-sim --example context_ownership --locked
cargo run -p doom-sim --example game_instance --locked
cargo run -p doom-sim --example diagnostic_free --locked
cargo run -p doom-sim --example original_player --locked
cargo run -p doom-sim --example player_knockback --locked
cargo run -p doom-sim --example player_commands --locked
cargo run -p doom-sim --example player_demon --locked
cargo run -p doom-sim --example player_demon_melee --locked
cargo run -p doom-sim --example player_pistol --locked
cargo run -p doom-sim --example player_weapon_pack --locked
cargo run -p doom-sim --example player_world_pickups --locked
cargo run -p doom-sim --example map_scale --locked
cargo run -p doom-sim --example demon_map_context --locked
cargo run -p doom-sim --example demon_map_spawn --locked
cargo run -p doom-map --example parse_map --locked
```

Examples use synthetic public maps, assert expected behavior and print diagnostic
results. They require no commercial IWAD, GPU, audio device or network service.
`demon_expanded_map` runs source-tested Demon gameplay on larger synthetic maps.
`canonical_demon` drives that existing behavior with original MT_SERGEANT/SARG
identities; it does not admit Spectre or resurrection.
`runtime_identity` checks the [native identity boundary](docs/ACTOR_IDENTITIES.md#native-runtime-identity-boundary) against unchanged historical captures.
`original_registry` checks [actual original-ID storage](docs/ACTOR_IDENTITIES.md#original-id-execution-registry) and bounded v20/v21 diagnostic reading.
`context_ownership` checks scoped target operations and retirement against compact
new C observations; see the [ownership contract](docs/COMPATIBILITY.md#context-and-target-lifetime-contract).
`game_instance` demonstrates [qualified references](docs/COMPATIBILITY.md#game-instance-provenance)
and reuses an existing C scenario without admitting new gameplay.
`map_extent` separately demonstrates expanded unbound coordinates and spatial work.
See [testing](docs/TESTING.md) for all examples and reproduction commands.

## Architecture

Small crates separate numeric types, WAD parsing, validated maps and authoritative
simulation. Gameplay owns its RNG and ordered state; presentation cannot affect
outcomes. Read the [architecture guide](docs/ARCHITECTURE.md) and
[documentation index](docs/README.md).

## Compatibility and validation

Original id Software Linux DOOM 1.10 is the behavioral authority. Matching an
isolated source slice does not establish whole-game or DOS demo compatibility.
The [validation guide](docs/VALIDATION.md) distinguishes original-C evidence,
independent DSIM simulation snapshots and Rust-specific safety policies. DSIM is
a project diagnostic format, not an original demo or savegame. Historical evidence
remains available in the [archive](docs/validation/archive/README.md).

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) for the development and review workflow.
Feature work needs source evidence, appropriate tests, documentation and owner
review. Agent-specific standing rules are in [AGENTS.md](AGENTS.md).

## License and attribution

The project preserves id Software's original source, history and attribution.
The Rust workspace is GPL-2.0-only; see the preserved
[license notice](LICENSE.TXT) and [original release README](README.TXT).
The source license does not grant rights to commercial game data. Such assets
stay external, are never distributed here, and are unnecessary for public CI.
The [external asset guide](docs/ASSETS.md) describes optional local loading.
