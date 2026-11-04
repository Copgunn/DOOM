# Rust DOOM

An independent Rust-native rewrite of the original DOOM engine. The goal is an engine
that behaves identically to id Software's linuxdoom-1.10, tic for tic, so that it can
one day play back original demos exactly. Every admitted behavior is checked against
traces from the original C source and against independently encoded state snapshots.
Development is early: the engine runs limited simulation examples without graphics,
not a complete game.

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
static sector specials, E1M1's doors, lifts, floors, switches and exit, two-sided
scrolling walls and E1M1's seven static decoration types. The real E1M1 does not load
yet: no policy admits its exploding barrels, more than 16 pickups (it has 52) or THING
angles from 180 to 359 degrees (45 of its 91 things), and whether every THING clears
the walls at its spawn spot has not been measured. Admitted
behavior is checked against original-C source traces and independently encoded
snapshots; [compatibility](docs/COMPATIBILITY.md) states the exact limits.

`Simulation::advance()` runs one tic without capturing a diagnostic snapshot; see the
[stepping contract](docs/COMPATIBILITY.md#stepping-and-diagnostics). Identity binding,
scoped actor access and game-instance provenance are internal design, described in the
[architecture guide](docs/ARCHITECTURE.md#actors-lifecycle-and-ordering).

## Policies

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
| `PlayerMonstersE1M1V1` (20) | Two-sided scrolling walls and E1M1's seven static decoration types (lamps, pillars, candelabras, corpses and gore) | [E1M1 decorations](docs/COMPATIBILITY.md#original-monsters-with-two-sided-scrolling-walls-and-static-decorations) |

## Where it's headed

The [roadmap](docs/ROADMAP.md) holds the plan and its dependencies. In order:

- Run the real E1M1 headless and match the original C tic for tic.
- Extend that to the rest of Episode 1.
- Add a renderer and audio on top of the unchanged simulation.

## What works

- Validated WAD directories and classic maps, including their spatial partitions
  (BSP) and collision grids (BLOCKMAP).
- Deterministic 35 Hz headless simulation with [canonical identity metadata](docs/ACTOR_IDENTITIES.md).
- One original player with four weapons, pickups, armor and bonuses against the Demon,
  former humans and the Imp, on synthetic maps with heights, diagonal walls, sky,
  static specials, doors, lifts, floors, switches, the exit, two-sided scrolling walls
  and static decorations such as lamps, pillars and corpses (opt-in; see
  [policies](#policies)).
- Opt-in larger static geometry for the Demon profiles; see [supported limits](docs/COMPATIBILITY.md#expanded-map-demon-profile).
- Original-C comparisons, independently encoded canonical snapshots, and safety
  tests for rejection, stale references and recoverable-error atomicity.
- Synthetic examples that run without commercial game data or presentation devices.

## What does not work yet

There is no game renderer, audio playback, general map loading, multiplayer or
verified vanilla demo playback. Gameplay still excludes full player-thinker and view
processing, the remaining weapons and monsters, exploding barrels, other line specials
(crushers, ceilings, stairs, locked doors, teleporters), soulspheres and powers, the
intermission and next map, resurrection and Nightmare respawning. No policy loads the
real E1M1 yet. A custom Vulkan-first rendering architecture is a future direction; the
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

Three recommended examples, each on synthetic public maps with asserted results:

```sh
cargo run -p doom-sim --example player_movers --locked   # player, monsters, doors, lifts and the exit
cargo run -p doom-sim --example player_imps --locked     # the Imp and its fireball
cargo run -p doom-map --example parse_map --locked       # WAD and map parsing
```

They need no commercial IWAD, GPU, audio device or network service. Every example and
its reproduction command is listed in [testing](docs/TESTING.md#asserted-headless-examples).

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
