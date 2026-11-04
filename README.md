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

The engine parses WAD files and classic maps and runs a deterministic, headless
35 tic/second simulation with fixed-point arithmetic, original random-number streams
and ordered actors; it has no renderer or audio. Gameplay is admitted only through
explicitly selected configurations: frozen numbered profiles for the earlier Demon
work, and opt-in original-ID execution policies, each a separate contract that leaves
every older policy unchanged. The newest policy runs one genuine original player with
four weapons, pickups, armor and bonuses against the Demon, former humans and the Imp,
on synthetic maps up to classic-map size with static heights, diagonal walls, sky,
static sector specials and E1M1's doors, lifts, floors, switches and exit. Admitted
behavior is checked against original-C source traces and independently encoded
snapshots; [compatibility](docs/COMPATIBILITY.md) states the exact limits.

Numbered profiles 1–20 are listed under
[versioned gameplay profiles](docs/COMPATIBILITY.md#versioned-gameplay-profiles).
The opt-in original-ID policies follow in wire-ID order. Policy 1 stands alone;
each later policy extends the one above it.

| Policy (wire ID) | Adds | Contract |
| --- | --- | --- |
| `DemonNormalV1` (1) | The bounded Demon closure with original actor types and states stored directly | [Original-ID Demon](docs/COMPATIBILITY.md#original-id-demon-profile) |
| `PlayerBodyLifecycleV1` (2) | One genuine original player body: direct damage, pain, death and a retained corpse | [Player body](docs/COMPATIBILITY.md#original-player-body-lifecycle) |
| `PlayerBodyKnockbackV1` (3) | Physical knockback and grounded wall collision for that player | [Knockback](docs/COMPATIBILITY.md#original-player-grounded-knockback) |
| `PlayerCommandMovementV1` (4) | Player commands (turning, thrust) and the source reaction-time delay | [Command movement](docs/COMPATIBILITY.md#original-player-command-movement) |
| `PlayerDemonAcquisitionV1` (5) | One Demon that sees and pursues the active player, without melee | [Acquisition](docs/COMPATIBILITY.md#active-original-player-and-demon-acquisition) |
| `PlayerDemonMeleeV1` (6) | Source-timed Demon melee against the player, with damage, pain and death | [Demon melee](docs/COMPATIBILITY.md#autonomous-original-demon-player-melee) |
| `PlayerDemonPistolV1` (7) | The genuine pistol: psprites, ammunition, refire spread, hitscan and noise alerts | [Pistol](docs/COMPATIBILITY.md#genuine-player-pistol-combat) |
| `PlayerDemonWeaponsV1` (8) | Fists, shotgun, chaingun, weapon fallback and manual switching | [Weapons](docs/COMPATIBILITY.md#original-player-weapon-expansion) |
| `PlayerDemonPickupsV1` (9) | World ammunition, stimpacks, medikits and shotgun/chaingun pickups | [Pickups](docs/COMPATIBILITY.md#original-world-pickups) |
| `PlayerDemonArmorV1` (10) | Green and blue armor with source absorption and depletion | [Armor](docs/COMPATIBILITY.md#original-armor-and-damage-absorption) |
| `PlayerDemonBonusesV1` (11) | Health and armor bonuses (+1 up to 200) and counted-item statistics | [Bonuses](docs/COMPATIBILITY.md#original-health-and-armor-bonuses) |
| `PlayerMonstersV1` (12) | Former humans beside the Demon, up to eight monsters, infighting and item drops | [Former humans](docs/COMPATIBILITY.md#original-former-humans-and-multi-monster-encounters) |
| `PlayerMonstersImpV1` (13) | The Imp and its dodgeable fireball | [Imps](docs/COMPATIBILITY.md#original-imps-and-fireballs) |
| `PlayerMonstersMapScaleV1` (14) | Travel bounded by the loaded walls in walled rooms up to 2,048 units | [Map scale](docs/COMPATIBILITY.md#original-monsters-at-real-map-scale) |
| `PlayerMonstersHeightsV1` (15) | Static floor/ceiling heights joined by two-sided lines: steps, ledges, windows, falls | [Static heights](docs/COMPATIBILITY.md#original-monsters-with-static-heights) |
| `PlayerMonstersDiagonalV1` (16) | Linedefs of any direction inside a closed boundary; one corner jump refused | [Diagonal walls](docs/COMPATIBILITY.md#original-monsters-among-diagonal-walls) |
| `PlayerMonstersRealMapV1` (17) | WAD-format geometry limits (E1M1-sized) under a per-map arithmetic certificate | [Classic-map scale](docs/COMPATIBILITY.md#original-monsters-at-classic-map-scale) |
| `PlayerMonstersSpecialsV1` (18) | Sky ceilings, light thinkers, damage floors, secrets and one-sided scrolling walls | [Sky and specials](docs/COMPATIBILITY.md#original-monsters-with-sky-and-static-specials) |
| `PlayerMonstersMoversV1` (19) | E1M1's seven door, lift, floor, switch and exit line types; the tic after the exit is refused | [Movers](docs/COMPATIBILITY.md#original-monsters-with-doors-lifts-floors-switches-and-the-exit) |

`Simulation::advance()` runs one tic without capturing a diagnostic snapshot; see the
[stepping contract](docs/COMPATIBILITY.md#stepping-and-diagnostics). Identity binding,
scoped actor access and game-instance provenance are internal design, described in the
[architecture guide](docs/ARCHITECTURE.md#actors-lifecycle-and-ordering).

## What works

- Validated WAD directories and classic maps, including their spatial partitions
  (BSP) and collision grids (BLOCKMAP).
- Deterministic 35 Hz headless simulation with [canonical identity metadata](docs/ACTOR_IDENTITIES.md).
- One original player with four weapons, pickups, armor and bonuses against the Demon,
  former humans and the Imp, on synthetic maps with heights, diagonal walls, sky,
  static specials, doors, lifts, floors, switches and the exit (opt-in; see the table above).
- Opt-in larger static geometry for the Demon profiles; see [supported limits](docs/COMPATIBILITY.md#expanded-map-demon-profile).
- Original-C comparisons, independently encoded canonical snapshots, and safety
  tests for rejection, stale references and recoverable-error atomicity.
- Synthetic examples that run without commercial game data or presentation devices.

## What does not work yet

There is no game renderer, audio playback, general map loading, multiplayer or
verified vanilla demo playback. Gameplay still excludes full player-thinker and view
processing, the remaining weapons and monsters, other line specials (crushers,
ceilings, stairs, locked doors, teleporters), two-sided scrolling walls,
soulspheres and powers, the intermission and next map, resurrection and Nightmare
respawning. A custom Vulkan-first rendering architecture is a future direction; the
[roadmap](docs/ROADMAP.md) describes dependencies.

## Getting started

Install Rust through rustup. The workspace uses stable Rust and supports Rust
1.85.0. Full workspace checks also need a host C compiler for the bundled
test-only zstd library; no external zstd program is needed. From the repository root:

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
cargo run -p doom-sim --example player_bonuses --locked
cargo run -p doom-sim --example player_monsters --locked
cargo run -p doom-sim --example player_imps --locked
cargo run -p doom-sim --example player_heights --locked
cargo run -p doom-sim --example player_diagonal --locked
cargo run -p doom-sim --example player_real_map --locked
cargo run -p doom-sim --example player_specials --locked
cargo run -p doom-sim --example player_movers --locked
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

Historical source traces use bounded offline zstd loading, and preview/execution
share original weapon formulas. These maintenance changes preserve gameplay and
DSIM bytes. See the [storage contract](docs/EVIDENCE_STORAGE.md) and
[formula contract](docs/WEAPON_FORMULAS.md).

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
