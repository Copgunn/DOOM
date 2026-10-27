# Rust DOOM

An independent, Rust-native rewrite of the original DOOM engine, maintained in
[Copgunn/DOOM](https://github.com/Copgunn/DOOM). The goal is deterministic vanilla
simulation, original WAD/map compatibility, reproducible demo playback, and
cross-platform presentation with Vulkan as the primary hardware renderer.
This is an idiomatic rewrite with explicit ownership, not an official id Software project.

## Current status

M0 foundation is implemented, validated, and merged ([PR #1](https://github.com/Copgunn/DOOM/pull/1)).
M1 implements and validates bounded IWAD/PWAD parsing and classic binary map
decoding ([PR #2](https://github.com/Copgunn/DOOM/pull/2)). Acceptance evidence is
tracked in the [roadmap](docs/ROADMAP.md#m1--wad-and-map-parsing) and
[validation evidence](docs/VALIDATION.md). The Rust workspace provides libraries
and synthetic examples. It does not run a game.

Implemented:

- Immutable borrowing WAD reader, checked signed directory fields, configurable
  limits, exact eight-byte names, safe lump access, ASCII-insensitive lookup,
  original ordering, and last-match precedence across archives.
- Classic THINGS, LINEDEFS, SIDEDEFS, VERTEXES, SEGS, SSECTORS, NODES, SECTORS,
  REJECT, and BLOCKMAP decoding for E#M#/MAP## groups. Validates references,
  sentinels, ranges, BSP cycles, visibility size, and shared block lists.
- Typed errors, fallible allocation, bounded validation work, synthetic malformed
  input/adversarial regressions, and separate runnable WAD/map fuzz targets.
- Stable Linux/Windows/macOS CI, Rust 1.85 tests, and repository engineering guidance.

Planned: deterministic simulation (M2), vanilla gameplay (M3), version-specific
demo playback (M4), and rendering, input, audio, and Vulkan (M5). Authentic IWAD
compatibility, geometric correctness of BSP builders, and gameplay synchronization
are not established. Read the precise [validation policies and limits](docs/COMPATIBILITY.md).

## Build and test

Install [Rust using rustup](https://rustup.rs/), then run from the repository root:

```sh
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked
cargo run -p doom-map --example parse_map --locked
```

`rust-toolchain.toml` selects stable with rustfmt and Clippy. The minimum compiler
is Rust 1.85 (edition 2024), tested separately in CI. The stable workspace has no
third-party dependencies and needs no IWAD, graphics SDK, C compiler, or platform
libraries. Lockfiles are committed; stable follows stable releases, so record the
exact compiler for regressions. API documentation is generated in `target/doc/`.
Optional fuzzing needs nightly, cargo-fuzz, and a C++ compiler; see [TESTING.md](docs/TESTING.md).

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
crates/doom-types/     shared leaf types and 35 Hz vocabulary
crates/doom-wad/       bounded borrowed archive views; depends on doom-types
crates/doom-map/       owned validated maps; depends on doom-wad and doom-types
tests/support/        original synthetic fixture builders
fuzz/                 separate nightly package, lockfile, targets and seed corpus
docs/                 architecture, roadmap, compatibility, testing, validation
.github/              stable cross-platform CI and PR template
linuxdoom-1.10/        preserved original engine; behavioral reference
ipx/ sersrc/ sndserv/  preserved original auxiliary source
README.TXT            original release notes and attribution
LICENSE.TXT           original GPL version 2 license
```

The future simulation consumes validated map data and ordered per-tic inputs at
exactly 35 Hz. Rendering, audio, wall clocks, and I/O cannot influence simulation
outcomes. See [architecture](docs/ARCHITECTURE.md) for boundaries and ownership.

## Roadmap and compatibility

| Milestone | Scope | Status |
| --- | --- | --- |
| M0 | Workspace, CI, documentation, repository guidance | Implemented, validated, merged |
| M1 | Bounded WAD and classic map parsing, regression tests, fuzzing | Implemented and validated; evidence in PR #2 |
| M2 | Fixed-point arithmetic, RNG, ordered thinkers, movement/collision | Planned |
| M3 | Vanilla player/monster behavior, weapons, interactions, specials | Planned |
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
