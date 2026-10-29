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
It still adapts some original actor/state identifiers and restricts game context.
See [compatibility](docs/COMPATIBILITY.md) for the exact boundary.

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
cargo run -p doom-sim --example map_scale --locked
cargo run -p doom-sim --example demon_map_context --locked
cargo run -p doom-sim --example demon_map_spawn --locked
cargo run -p doom-map --example parse_map --locked
```

Examples use synthetic public maps, assert expected behavior and print diagnostic
results. They require no commercial IWAD, GPU, audio device or network service.
`demon_expanded_map` runs source-tested Demon gameplay on larger synthetic maps.
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
