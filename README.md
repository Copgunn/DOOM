# Rust DOOM

An independent, Rust-native rewrite of the original DOOM engine, maintained in
[Copgunn/DOOM](https://github.com/Copgunn/DOOM). The goal is deterministic vanilla
simulation, original WAD/map compatibility, reproducible demo playback, and
cross-platform presentation through a custom wgpu renderer, with Vulkan preferred
on supported platforms. Rendering remains future work.
This is an idiomatic rewrite with explicit ownership, not an official id Software project.

## Current status

M0 foundation is implemented, validated, and merged ([PR #1](https://github.com/Copgunn/DOOM/pull/1)).
M1 resource/map parsing is implemented, validated, and merged
([PR #2](https://github.com/Copgunn/DOOM/pull/2)). M2 deterministic headless
foundations are implemented, validated and merged
([PR #3](https://github.com/Copgunn/DOOM/pull/3), merge `10a1106`). M3a adds
reference-tested grounded player XY movement, wall sliding and solid actor
collision, merged in [PR #4](https://github.com/Copgunn/DOOM/pull/4) as `764dd96`.
M3b implements and validates static multi-sector movement and vertical physics
in [PR #5](https://github.com/Copgunn/DOOM/pull/5), merged as `c791ef8` after
owner-authorized correction-head and post-merge CI. M3c adds typed actor states,
spawning and controlled death on its review branch in [PR #6](https://github.com/Copgunn/DOOM/pull/6); it remains unmerged.
Acceptance and exact delivery evidence are in the [roadmap](docs/ROADMAP.md)
and [validation notes](docs/VALIDATION.md). A playable client is future work.

Implemented:

- Immutable borrowing WAD reader, checked signed directory fields, configurable
  limits, exact eight-byte names, safe lump access, ASCII-insensitive lookup,
  original ordering, and last-match precedence across archives.
- Classic THINGS, LINEDEFS, SIDEDEFS, VERTEXES, SEGS, SSECTORS, NODES, SECTORS,
  REJECT, and BLOCKMAP decoding for E#M#/MAP## groups. Validates references,
  sentinels, ranges, BSP cycles, visibility size, and shared block lists.
- Typed errors, fallible allocation, bounded validation work, synthetic malformed
  input/adversarial regressions, and separate runnable WAD/map fuzz targets.
- Explicit 16.16 fixed arithmetic, original owned RNG streams, validated actor
  handles and append-sensitive thinker scheduling.
- Grounded non-player XY motion, original line predicates and ordered BLOCKMAP
  collision, command-thrust foundations, canonical snapshots and first-divergence
  diagnostics, compared with isolated C vectors and independent encoding goldens.
- Explicit M3a profile: player-specific ground friction and blocking response,
  three-corner wall tracing, axis/angled slides, solid actor collision and
  cell-head ordering, DSIM-v2 snapshots, and an asserted headless arena.
- Explicit M3b profile: iterative original BSP lookup, two-sided openings,
  step/dropoff/clearance rules, XY/Z order, gravity/landing/ceiling contact,
  sector-head membership/order, reference-list BLOCKMAP policy and DSIM-v3.
- Explicit M3c profile: complete original actor metadata, validated executable
  subsets, ordered state actions, typed/map spawning, controlled death, safe
  targets and DSIM-v4. Metadata recognition does not imply monster gameplay.
- Stable debug/release Linux/Windows/macOS CI, Rust 1.85 tests, and engineering guidance.

Choose the compatibility profile explicitly:

| Constructor | Schema | Supported subset |
| --- | --- | --- |
| `Simulation::new` | DSIM-v1 | M2 one-sector non-solid/non-player XY response |
| `Simulation::player_collision` | DSIM-v2 | M3a grounded player slides and solid collision, conservative coverage |
| `Simulation::multisector` | DSIM-v3 | M3b static classic sectors, BSP/openings, step/dropoff and XY/Z/gravity |
| `Simulation::actor_states` | DSIM-v4 | M3c typed actors and supported state/action lifecycle on static-sector physics |

M3b consumes supplied BLOCKMAP lists in reference order. Conservative omissions
are diagnostic, not automatic rebuilds or proof of broken geometry. Missing
collision data can allow source-compatible movement through omitted walls; see
[the investigated policy](docs/M3B_REFERENCE.md). Older profiles remain unchanged.
Classic two-sided lines are gameplay openings, not stacked sectors, room-over-room
or enhanced visual portals.

Complete player thinking/view/animation, weapons, monster AI,
combat, moving doors/lifts, specials and respawn remain incomplete. M3d–M3e
continue gameplay; M4 covers demos; M5 covers presentation/input/audio and the
custom wgpu renderer. Authentic IWAD compatibility, full synchronization and DOS
1.9 behavior are unverified. Read the [compatibility limits](docs/COMPATIBILITY.md).

## Build and test

Install [Rust using rustup](https://rustup.rs/), then run from the repository root:

```sh
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p doom-sim -p doom-types --release --locked
cargo doc --workspace --no-deps --locked
cargo run -p doom-map --example parse_map --locked
cargo run -p doom-sim --example headless --locked -- 12
cargo run -p doom-sim --example player_arena --locked
cargo run -p doom-sim --example multisector_arena --locked
cargo run -p doom-sim --example actor_states --locked
```

`rust-toolchain.toml` selects stable with rustfmt and Clippy. The minimum compiler
is Rust 1.85 (edition 2024), tested separately in CI. The stable workspace has no
third-party dependencies and needs no IWAD, graphics SDK, C compiler, or platform
libraries. Lockfiles are committed; stable follows stable releases, so record the
exact compiler for regressions. API documentation is generated in `target/doc/`.
Optional fuzzing needs nightly, cargo-fuzz, and a C++ compiler. Reproducing the
isolated C vectors needs Python 3 and a supported C compiler; recorded-vector Rust
tests need neither. See [TESTING.md](docs/TESTING.md).

## Parsing APIs

The runnable `parse_map` example loads the original synthetic square fixture and
reports four vertices, four linedefs, and one sector. The same API accepts bytes
acquired by your application under its own I/O/memory limits:

```rust
use doom_map::{Map, MapLimits};
use doom_wad::{Limits, Wad, WadSet};

fn load_map(bytes: &[u8]) -> Result<Map, Box<dyn std::error::Error>> {
    let archives = [Wad::parse(bytes, Limits::default())?];
    let resources = WadSet::new(&archives);
    Ok(Map::load(&resources, "MAP01", MapLimits::default())?)
}
```

Keep byte buffers alive while using their `Wad` views. Add parsed patch archives
later in the `WadSet` sequence; later matches take precedence. `Wad::lumps()`
retains every directory entry; `find(name)` returns its last matching index and
`data(index)` returns `Option<&[u8]>`. `Map` owns its validated records and exposes
immutable slices such as `vertices()`, `linedefs()`, and `sectors()`.

Maps load from the overriding marker's complete ordered group, within one archive.
They do not assemble geometry using global individual-lump lookup. Unsupported
UDMF, Hexen, and known extended-node signatures produce explicit errors. Input
safety is stricter than the original C loader; see [compatibility](docs/COMPATIBILITY.md).

## Headless simulation APIs

Run the current demonstration:

```sh
cargo run -p doom-sim --example multisector_arena --locked
```

It runs six synthetic connected-sector scenarios (144 asserted C-derived tics):
a 24-unit step, rejected 25-unit step, exact 56-unit doorway, blocked 55-unit
doorway, allowed drop/gravity/landing, and movement through three sectors. Output
shows logical tic, raw fixed XYZ/momentum, sector and canonical digest. It requires
no window, renderer, audio, wall clock or commercial assets.

The original `headless -- 12` and `player_arena` examples preserve M2 coasting and
M3a angled/solid collision evidence. `python tools/reference/check_headless.py`
compares all four examples' debug/release output byte-for-byte.

Initialize M3b from a validated `Map`:

```rust
use doom_map::Map;
use doom_sim::{ActorSpec, SimError, Simulation, Snapshot, Solidity, TicCommand, VerticalSpec};

fn one_tic(map: Map) -> Result<Snapshot, SimError> {
    let mut sim = Simulation::multisector(map)?;
    let player = sim.spawn_vertical(
        ActorSpec::default(),
        Solidity::Solid,
        VerticalSpec { allow_dropoff: true, ..Default::default() },
    )?;
    sim.bind_player(0, player)?;
    let mut commands = [TicCommand::default(); 4];
    commands[0].forward = 25;
    sim.step(commands) // one logical 35 Hz tic; no elapsed-time argument
}
```

`VerticalSpec` selects floor/ceiling/absolute spawn Z, initial vertical momentum,
normal gravity or `Gravity::None` (MF_NOGRAVITY), and MF_DROPOFF permission. Player
binding does not silently add dropoff flags. Airborne actors turn but receive no
forward/side thrust; airborne XY skips ground friction. XY runs before Z for each
thinker. There is no jump/crouch/freelook, flying-monster or arbitrary-flag API.

`world().locate(position)` returns source BSP membership. `actor.location()` and
`actor.constraints()` distinguish the center's sector from contacted floor/ceiling
constraints while straddling a line. `sectors()` exposes read-only runtime heights;
`sector_actor_order`, `line_opening` and `check_multisector_position` expose
observational ordering/opening/destination decisions. Legacy flat collision and
dimension helpers reject M3b calls that lack actor/sector context.

`spawn` creates a non-solid mover and `spawn_with_solidity` selects zero/MF_SOLID;
in M3b these use default floor-Z setup. Setup may overlap actors like the source.
Non-solid movers still collide with solid candidates; supported ordinary actor
collision uses the original XY-only overlap predicate even when Z differs.
Removal unlinks spatial/sector membership immediately; thinker storage retires
later. Every successful move reinserts at both heads, including within a cell or
sector. Identity, thinker order and spatial/sector order remain separate.

Recoverable `step` errors preserve commands, actors, RNG, events and time.
Geometry/actor invariants are validated before stepping; allocation exhaustion
uses Rust's process-level policy. No full simulation clone is made each tic.

`run_commands` records whole command streams; `Snapshot::canonical_bytes()` and
`digest()` encode explicit state, while `compare_traces` reports the first
mismatching tic/field. Rendering can read snapshots/events without draining or
changing simulation. Hash equality alone is not proof of gameplay equivalence.
See [STATE_FORMAT.md](docs/STATE_FORMAT.md) for field widths/order and
[M2_REFERENCE.md](docs/M2_REFERENCE.md), [M3A_REFERENCE.md](docs/M3A_REFERENCE.md),
and [M3B_REFERENCE.md](docs/M3B_REFERENCE.md) for source findings and safe policies.

`doom-types::Fixed` uses explicit wrapping arithmetic and the reference's guarded
double division; `DoomRng` owns both original RNG indexes. `ThinkerStore` separates
slot/generation identity from append order, detects stale handles, defers retirement,
and permits same-traversal spawning through restricted callbacks. The current
simulation's built-in thinkers perform the selected profile's motion and, in
profile 4, admitted actor state/action processing after XY/Z.

## Typed actor lifecycle (M3c)

```sh
cargo run -p doom-sim --example actor_states --locked
```

This asset-free example checks 80 tics against independently encoded C results:
19 state entries, two supported actions, a persistent terminal player-type object,
and fog null-state removal followed by retirement. It is a headless test scenario.

`ActorRegistry::vanilla()` recognizes all 137 original types, 967 states and 74
non-null action identities. `Simulation::actor_states(map)` selects profile 4;
`spawn_type(type_id, xy, SpawnHeight::Floor)` validates an executable type before
initializing it. The registry method `find_type("MT_PLAYER")` resolves symbolic names.
Spawning assigns the initial state directly without calling its action. Subsequent
state entry/countdowns run at 35 Hz, after each actor's XY/Z motion. State zero is
explicit removal; infinite states persist. Unsupported callbacks reject before
mutation. Bounds include 1024 actors, 2048 states, 256 types and 4096 map THINGS.

In a flat 128-unit sector, 67 source types are admitted: player-type pawns, inert
props, fog and a few animation-only effects. Actual admission checks dimensions
against every sector and validates spawn/death closures. The five original actions
are `A_Fall`, `A_Pain`, `A_Scream`, `A_XScream` and `A_PlayerScream`; sound requests
are inert events. See the [admission matrix and policies](docs/M3C_REFERENCE.md).

`spawn_map_things()` performs single-player DOOM II skill/nomonsters filtering,
DoomEd lookup, source-order spawning, positive-tic randomization, angle quantization
and ambush flags. It reports player/deathmatch starts explicitly; full player setup
and multiplayer modes remain unsupported. A bad selected type rejects the batch
without partial spawning or RNG drift.

`request_actor_death(id)` enters an unbound actor's registered death state. It
implements no damage, kill scoring, drops, corpse flags, height change or randomized
kill timer. `A_Fall` clears solidity; a terminal object can persist. Targets retain
generation-aware identities through removal and resolve unavailable after retirement.
`enter_actor_state` validates explicit state requests before effects. A separate
`ProbeAction` namespace tests scheduler/service capabilities; those callbacks are
synthetic probes, not vanilla AI or a modding language.

## Optional private IWAD directory

Keep lawfully obtained IWADs outside the checkout in a directory you control and
set `DOOM_IWAD_DIR` in your private shell. After separately authorizing assets for
that environment, run the read-only opt-in archive/map validation:

```sh
cargo test -p doom-map --test private_iwad -- --ignored
```

This test checks IWAD archives and their classic map groups without printing
private filenames, paths, or bytes. A separate `doom-wad` test of the same name
checks signatures only. Both stay ignored in public CI. They do not establish
authenticity, game initialization, rendering, demo compatibility, or gameplay.
The library accepts bytes; an engine client that automatically discovers/loads
assets is future work. No private assets are bundled.

Do not copy assets into a cloud workspace without explicit destination consent,
change source originals, or publish private test output. Local asset directories,
WAD/demo/package files, environment files, and private output are ignored as a
safeguard; ignore rules do not prevent forced staging. Never upload commercial
assets to GitHub branches, PRs, releases, CI artifacts, snapshots, or public logs.

## Layout and architecture

```text
crates/doom-types/     explicit fixed arithmetic, vanilla RNG, names and 35 Hz vocabulary
crates/doom-wad/       bounded borrowed archive views; depends on doom-types
crates/doom-map/       owned validated maps; depends on doom-wad and doom-types
crates/doom-sim/       owned headless state, scheduling, movement and snapshots
tests/support/        original synthetic fixture builders
tests/reference/      recorded C vectors, provenance and independent snapshot goldens
tools/reference/      reproducible isolated C harness and golden encoder
fuzz/                 separate nightly package, lockfile, targets and seed corpus
docs/                 architecture, roadmap, compatibility, testing, validation
.github/              stable cross-platform CI and PR template
linuxdoom-1.10/        preserved original engine; behavioral reference
ipx/ sersrc/ sndserv/  preserved original auxiliary source
README.TXT            original release notes and attribution
LICENSE.TXT           original GPL version 2 license
```

The simulation consumes validated map data and ordered per-tic commands. Each
step represents 1/35 second of logical time. Rendering, audio, wall clocks, and
I/O cannot influence its outcomes. See [architecture](docs/ARCHITECTURE.md).

## Roadmap and compatibility

| Milestone | Scope | Status |
| --- | --- | --- |
| M0 | Workspace, CI, documentation, repository guidance | Implemented, validated, merged |
| M1 | Bounded WAD and classic map parsing, regression tests, fuzzing | Implemented, validated, merged |
| M2 | Fixed arithmetic, RNG, ordered thinkers, narrow movement, snapshots | Implemented, validated, merged |
| M3a | Grounded player XY, wall sliding, solid actor collision | Implemented, validated, merged |
| M3b | Static sectors/BSP, openings, step/dropoff and XY/Z/gravity | Implemented, validated, merged |
| M3c | Actor states, spawning and controlled death | Implemented on review branch; unmerged |
| M3d–M3e | Combat, interactions and moving sector specials | Planned |
| M4 | Version-specific demo replay and desynchronization diagnostics | Planned |
| M5 | Presentation, input/audio, custom wgpu renderer; Vulkan preferred | Planned |

The initial behavioral reference is **Linux DOOM 1.10**, from
[id-Software/DOOM](https://github.com/id-Software/DOOM) at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. DOS DOOM 1.9 is a separate objective
requiring its own evidence. Demo version 110 does not establish DOS compatibility.
See [milestone acceptance criteria](docs/ROADMAP.md).

## Attribution, license, and assets

The original source was released by id Software, with release notes by John
Carmack and cleanup credited to Bernd Kreimeier. This fork preserves upstream
history, source notices, [README.TXT](README.TXT), and [LICENSE.TXT](LICENSE.TXT),
including the ZeniMax Media copyright notice.

Rewrite code is **GPL version 2 only** (`GPL-2.0-only`), consistent with the
repository's GPL-2.0 license. The original README is historical; the GPL governs
code use and redistribution, including commercial distribution under its terms.
The engine source license grants no rights to proprietary game data. Commercial
IWADs, textures, sprites, music, sounds, and other proprietary assets are not
included and must never be committed. Obtain data lawfully; public CI uses only
synthetic data. A future runnable engine needs separately licensed assets.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md), [AGENTS.md](AGENTS.md), and the
[testing guide](docs/TESTING.md). Work on a focused branch with meaningful tests
and relevant documentation, and open a PR against `master`. Correctness,
determinism, safety, and reviewable changes take priority over optimization.
Merging requires explicit owner approval.
