# Rust DOOM

A rewrite of the original DOOM engine in Rust that aims to behave exactly like id
Software's code, down to every single step of game logic.

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

## Why this exists

Most source ports (modern rewrites of a game's released source code) aim to be playable
first and accurate second. This project works the other way round.

It only accepts behavior that has been checked against the original Linux DOOM 1.10 C
code. The check compares the entire game state after every tic, the 1/35-second step in
which DOOM updates its world. Anything not yet proven identical is refused: the engine
stops with an error instead of approximating.

That makes it useful for exact-behavior work:

- playing back recorded demos (files that replay a player's inputs), which only stay in
  sync if every tic matches;
- speedrun and tool-assisted (TAS) research;
- studying how the original engine really behaves.

**It is not playable yet.** If you want to play Doom today, use an established source
port such as [Chocolate Doom](https://www.chocolate-doom.org/). It needs your own game
files, such as those from a copy of Doom you bought.

## Where it stands

The engine runs DOOM's game logic without graphics. It is driven by tests and small
example programs, on small test maps built for this project. Today it can simulate:

- one player who moves, turns, uses switches and doors, takes damage and dies;
- the fist, pistol, shotgun and chaingun, with ammunition and weapon switching;
- the Demon, the Zombieman, the Shotgun Guy, the Chaingunner and the Imp, which see,
  chase, attack and fight each other;
- ammunition, health, armor and bonus pickups;
- walls at any angle, steps and ledges, windows, sky, damaging floors, secret areas,
  flickering lights and scrolling walls;
- the doors, lifts, moving floors, switches and exit used by E1M1, the first map;
- E1M1's lamps, pillars, candelabras, corpses and gore.

What it cannot do yet:

- show graphics, play sound or offer menus;
- run the real E1M1 from start to finish, because the map has exploding barrels and 52
  pickups, and both are still beyond what the engine supports;
- handle the other monsters, the other weapons, or the remaining map features such as
  crushers and teleporters;
- play back demos.

[Compatibility](docs/COMPATIBILITY.md) states the exact limits.

## Where it's headed

The committed plan, in order (details in the [roadmap](docs/ROADMAP.md)):

1. Run the real E1M1 and match the original tic for tic.
2. Extend that to the rest of Episode 1.
3. Add a renderer and audio, so that it is actually playable.

## Where it's (maybe) going

These are ideas, not promises, grouped into three tracks. The
[roadmap](docs/ROADMAP.md#ideas-under-consideration) describes them in a little more detail.

**Track 1: Faithful.** Vanilla behavior (that of the original, unmodified game) stays
exact forever; anything that changes it lives in a separate opt-in mode.

- **Original demo playback.** Play back recorded vanilla demos (`.lmp` files) without
  desyncing. That is the strongest end-to-end proof of exactness.
- **Faster progress.** Port whole systems at once, such as all monsters or all weapons.
  Check them against the original by running real maps and demos.
- **One vanilla profile, plus a limit-removing one.** Fold the step-by-step policies into
  one complete vanilla profile. A separate profile would lift the original's hard limits,
  such as the most things or visible surfaces it can handle, without otherwise changing
  gameplay.
- **An optional "fixed" mode.** A separate opt-in mode would correct known original bugs.
  The differences and quirks already recorded become its to-do list.
- **The full game and Doom II.** After Episode 1, the remaining episodes and then Doom II,
  held to the same exactness.

**Track 2: Presentation.** How the planned renderer could look and feel. This can never
change gameplay, because the renderer only reads the game state.

- **A faithful software renderer first.** It would look exactly like DOOM did in 1993.
- **Then a modern renderer.** Smooth high frame rates by drawing extra frames between
  the game's 35 tics per second, plus modern effects such as dynamic lighting.
- **VR.** A headset handles looking around while the mouse turns and aims. That suits
  Doom, because the original has no vertical aiming.
- **A fully offline engine.** Your own game files, with no internet connection or account
  ever required.

**Track 3: Beyond vanilla.** A new engine built on the verified core. It would live in
separate opt-in modes, so vanilla is never affected.

- **Modern source-port features, done cleanly.** Features like those of ZDoom, a popular
  advanced source port, without decades-old code underneath.
- **Things the original never simulated.** Water, blood and nukage that actually flow,
  and very large crowds of monsters.
- **Scripting and more platforms.** A scripting layer for new game logic, and other
  platforms such as mobile.

## How it's verified

The original C source is the ground truth. Each supported behavior is run in both the
original code and the Rust code, and the whole game state is compared after every tic.
The expected results are recorded by a separate program written from the C output alone,
so Rust is never checked against its own output. Sanitizers catch hidden memory and
arithmetic errors while the original C code runs. Fuzzing feeds in huge numbers of random
inputs to find anything that slips through. When something is not proven identical, the
engine refuses it rather than guessing. The [validation guide](docs/VALIDATION.md)
explains the method, and [compatibility](docs/COMPATIBILITY.md) lists what is covered.

## Try it

You need Rust, installed through [rustup](https://rustup.rs/) (stable, or at least
1.85.0), and a C compiler. The C compiler builds a small compression library used only by
tests. No game files are needed, and it builds on Linux, Windows and macOS. From the
repository root:

```sh
cargo build --workspace --locked
cargo test --workspace --locked
```

Three examples worth running:

```sh
cargo run -p doom-sim --example player_movers --locked   # a player, monsters, doors, lifts and the exit
cargo run -p doom-sim --example player_imps --locked     # the Imp and its dodgeable fireball
cargo run -p doom-map --example parse_map --locked       # reading DOOM's map format
```

Each one runs a short scripted scene on a test map, checks the result and prints a
one-line summary. None needs an IWAD (the main game data file, such as `doom.wad`), a
graphics card, audio or a network.
[Testing](docs/TESTING.md#asserted-headless-examples) lists every example and check.

## Policies (for developers)

New gameplay is switched on through opt-in policies. Each policy adds one slice of Doom
on top of the one before it. Older policies never change, so earlier results stay exactly
reproducible, much like compatibility levels. The number in brackets is the policy's
identifier in saved test snapshots. The earlier numbered Demon profiles are listed under
[versioned gameplay profiles](docs/COMPATIBILITY.md#versioned-gameplay-profiles).

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

More developer reading:

- [Architecture](docs/ARCHITECTURE.md): small crates for numbers, WAD files, maps and
  simulation, and how [actors and their ordering](docs/ARCHITECTURE.md#actors-lifecycle-and-ordering) work.
- [Stepping contract](docs/COMPATIBILITY.md#stepping-and-diagnostics) and
  [actor identities](docs/ACTOR_IDENTITIES.md).
- [Larger maps for the Demon profiles](docs/COMPATIBILITY.md#expanded-map-demon-profile).
- The [documentation index](docs/README.md) and the
  [historical evidence archive](docs/validation/archive/README.md).

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow. Each change needs evidence from
the original source, tests and documentation, and owner review. The Python tools under
`tools/` are a gentle place to start. Standing rules for AI agents are in
[AGENTS.md](AGENTS.md).

## License and attribution

The project preserves id Software's original source, history and attribution. The Rust
workspace is GPL-2.0-only; see the preserved [license notice](LICENSE.TXT) and the
[original release README](README.TXT). The source license grants no rights to the
commercial game data. Those files stay external, are never distributed here, and are not
needed for the public tests. The [external asset guide](docs/ASSETS.md) describes
optional local loading.
