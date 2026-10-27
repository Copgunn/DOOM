# Rust DOOM

An independent, Rust-native rewrite of the original DOOM engine, maintained in
[Copgunn/DOOM](https://github.com/Copgunn/DOOM). The goal is deterministic vanilla
simulation, original WAD/map compatibility, reproducible demo playback, and
cross-platform presentation with Vulkan as the primary hardware renderer.
This is an idiomatic rewrite with explicit ownership, not an official id Software project.

## Current status

M0 foundation is implemented, validated, and merged ([PR #1](https://github.com/Copgunn/DOOM/pull/1)).
M1 resource/map parsing is implemented, validated, and merged
([PR #2](https://github.com/Copgunn/DOOM/pull/2)). M2 deterministic headless
foundations are implemented, validated and merged
([PR #3](https://github.com/Copgunn/DOOM/pull/3), merge `10a1106`). M3a adds
reference-tested grounded player XY movement, wall sliding and solid actor
collision in [PR #4](https://github.com/Copgunn/DOOM/pull/4), accepted for the
owner-authorized regular merge after correction-head checks.
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
- Stable debug/release Linux/Windows/macOS CI, Rust 1.85 tests, and engineering guidance.

Both profiles support one static sector, one-sided lines, no line/sector specials,
and grounded XY movement. `Simulation::new` preserves the M2 non-solid/non-player
response and DSIM-v1. `Simulation::player_collision` enables M3a: bound actors use
player sliding/friction, unbound actors use ordinary blocking, and solidity is
independent of input binding. Its bounded profile requires conservative BLOCKMAP
coverage; see [source findings and restrictions](docs/M3A_REFERENCE.md).

Complete player thinking, Z/gravity, portals/sector transitions, actor animation,
weapons, monsters, specials and gameplay respawn remain incomplete. Unsupported
profiles/buttons fail explicitly. M3b–M3e continue gameplay, M4 covers demo
playback, and M5 covers presentation/input/audio and Vulkan. Authentic IWAD
compatibility, full synchronization and DOS 1.9 behavior are not established.
Read the precise [compatibility limits](docs/COMPATIBILITY.md).

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

Run `cargo run -p doom-sim --example player_arena --locked` for a clockwise
synthetic room with an angled northeast wall, two solid blockers, and twelve
reproducible commands. It asserts every XY/momentum/angle result against isolated
C vectors, then reports four actor blocks, four wall blocks and four slide
projections. No display, audio, timer, renderer or commercial data is needed.
`python tools/reference/check_headless.py` compares debug/release output.

The original `headless -- 12` example still exercises M2 coasting and DSIM-v1.
For M3a, initialize from an already validated `Map`:

```rust
use doom_map::Map;
use doom_sim::{ActorSpec, SimError, Simulation, Snapshot, Solidity, TicCommand};

fn one_tic(map: Map) -> Result<Snapshot, SimError> {
    let mut sim = Simulation::player_collision(map)?;
    let player = sim.spawn_with_solidity(ActorSpec::default(), Solidity::Solid)?;
    sim.bind_player(0, player)?;
    let mut commands = [TicCommand::default(); 4];
    commands[0].forward = 25;
    sim.step(commands) // exactly one logical tic, with no elapsed-time argument
}
```

`spawn` creates a non-solid mover; `spawn_with_solidity` also supports `Solid`.
Like original parameterized spawning, setup may place actors overlapping; use
`check_actor_position` for an observational occupancy check. Non-solid movers
still collide with solid candidates. Removal unlinks spatially immediately and
retires thinker storage later. Successful moves reinsert at cell head, including
within the same cell; slot reuse never determines collision order.

Recoverable `step` errors preserve commands, actors, RNG, events and time.
Geometry/actor invariants are validated before stepping; allocation exhaustion
uses Rust's process-level policy. No full simulation clone is made each tic.

`run_commands` records whole command streams; `Snapshot::canonical_bytes()` and
`digest()` encode explicit state, while `compare_traces` reports the first
mismatching tic/field. Rendering can read snapshots/events without draining or
changing simulation. Hash equality alone is not proof of gameplay equivalence.
See [STATE_FORMAT.md](docs/STATE_FORMAT.md) for field widths/order and
[M2_REFERENCE.md](docs/M2_REFERENCE.md) and [M3A_REFERENCE.md](docs/M3A_REFERENCE.md)
for source findings, safe policies and excluded C cases.

`doom-types::Fixed` uses explicit wrapping arithmetic and the reference's guarded
double division; `DoomRng` owns both original RNG indexes. `ThinkerStore` separates
slot/generation identity from append order, detects stale handles, defers retirement,
and permits same-traversal spawning through restricted callbacks. The current
simulation's built-in thinkers perform only the supported XY update; full mobj
state transitions and type-driven spawning are M3 work.

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
| M3a | Grounded player XY, wall sliding, solid actor collision | Implemented/validated; authorized merge after fresh checks |
| M3b–M3e | Multi-sector/Z, states, combat, interactions and specials | Planned |
| M4 | Version-specific demo replay and desynchronization diagnostics | Planned |
| M5 | Presentation, input/audio, software diagnostics, Vulkan | Planned |

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
