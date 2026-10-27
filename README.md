# Rust DOOM

An independent, Rust-native rewrite of the original DOOM engine, maintained in
[Copgunn/DOOM](https://github.com/Copgunn/DOOM). The goal is deterministic vanilla
simulation, original WAD and map compatibility, reproducible demo playback, and
cross-platform presentation with Vulkan as the primary hardware renderer.

This is an idiomatic rewrite with explicit ownership and subsystem boundaries.
It is not an official id Software project.

## Current status

**M0 foundation is implemented on the development branch; delivery verification
and owner review are tracked in the [roadmap](docs/ROADMAP.md#m0--foundation).**
The Rust workspace builds libraries only. It does not run a game.

Implemented:

- Three dependency-light crates with shared 35 Hz timing vocabulary, lossless
  eight-byte lump names, WAD signature recognition and raw directory metadata,
  and a classic vertex value type. There are no third-party Rust dependencies.
- Unit and integration test structure using generated data, including invalid
  WAD signatures. These tests do not establish archive parsing coverage.
- Rust formatting, Clippy, build, test, and documentation CI for Linux, Windows,
  and macOS, plus a minimum-supported-Rust test job.
- Architecture, compatibility, testing, contribution, and agent guidance.

Planned: bounded WAD parsing and map validation (M1), simulation and gameplay
(M2–M3), demo playback (M4), and rendering, input, audio, and Vulkan (M5).

## Build and test

Install [Rust using rustup](https://rustup.rs/), then run from the repository root:

```sh
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo doc --workspace --no-deps --locked
```

`rust-toolchain.toml` selects stable Rust with rustfmt and Clippy. The minimum
supported compiler is Rust 1.85 (edition 2024); CI tests 1.85.0 separately. The
lockfile is committed. Stable intentionally follows stable releases; record the
exact compiler when reporting regressions. M0 needs no IWAD, graphics SDK, C
compiler, or platform libraries. Documentation is generated in `target/doc/`.

### Optional private IWAD directory

Keep lawfully obtained IWADs outside the checkout in a directory you control.
Set `DOOM_IWAD_DIR` to that directory in your private shell environment. M0
provides an opt-in **signature probe**, not an engine/archive loader:

```sh
cargo test -p doom-wad --test private_iwad -- --ignored
```

The probe reads four bytes from each `.wad` file, requires an IWAD signature,
opens files read-only, and emits no asset names, bytes, or locations. It does not
verify the archive, maps, game version, or authenticity. Public CI never runs it.
Do not publish private test logs or paths. Future M1 parsing and the M5 client
will use this external-directory convention; runtime loading is not implemented.

Do not download or copy private assets into a cloud workspace without explicit
authorization for that destination. Keep any source originals unchanged. Local
asset directories, WAD/demo/package files, environment files, and private test
output directories are ignored as a safeguard; ignore rules do not prevent
forced staging or replace review. Never upload commercial assets, even to private
branches, releases, CI artifacts, snapshots, or logs.

## Layout and architecture

```text
crates/doom-types/     shared leaf types; no I/O or dependencies
crates/doom-wad/       WAD vocabulary; depends on doom-types
crates/doom-map/       classic map vocabulary; no dependencies yet
docs/                 architecture, roadmap, compatibility, testing, validation
.github/              Rust CI and pull request template
linuxdoom-1.10/        preserved original engine; behavioral reference
ipx/ sersrc/ sndserv/  preserved original auxiliary source
README.TXT            original release notes and attribution
LICENSE.TXT           original GPL version 2 license
```

The future simulation consumes validated map data and ordered per-tic inputs at
exactly 35 Hz. It produces presentation snapshots and events without depending
on rendering, wall-clock timing, audio, or I/O. Future renderers consume those
snapshots; they cannot change gameplay state. See [architecture](docs/ARCHITECTURE.md).

## Roadmap and compatibility

| Milestone | Scope | Status |
| --- | --- | --- |
| M0 | Workspace, CI, engineering and repository guidance | Implemented; verification/review tracked in roadmap |
| M1 | Bounded IWAD/PWAD and classic map decoding, validation, fuzzing | Planned |
| M2 | Fixed-point arithmetic, RNG, ordered thinkers, movement/collision | Planned |
| M3 | Vanilla player/monster behavior, weapons, interactions, specials | Planned |
| M4 | Version-specific demo replay and desynchronization diagnostics | Planned |
| M5 | Presentation loop, input/audio, software diagnostics, Vulkan | Planned |

The initial behavioral reference is **Linux DOOM 1.10**, from
[id-Software/DOOM](https://github.com/id-Software/DOOM) at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. DOS DOOM 1.9 compatibility is a separate
objective requiring its own evidence. Demo version 110 does not establish DOS
demo compatibility. No Rust simulation, demo, or rendering compatibility is
currently claimed. See [compatibility](docs/COMPATIBILITY.md) and the
[milestone acceptance criteria](docs/ROADMAP.md).

## Attribution, license, and assets

The original source was released by id Software, with release notes by John
Carmack and source cleanup credited to Bernd Kreimeier. This fork preserves its
upstream history, source notices, [README.TXT](README.TXT), and
[LICENSE.TXT](LICENSE.TXT), including the ZeniMax Media copyright notice.

Code in this rewrite is licensed under **GPL version 2 only** (`GPL-2.0-only`),
consistent with the repository's GPL-2.0 license. The original README is a
historical document; the GPL license governs code use and redistribution.
The engine source license does not grant rights to proprietary game data.
Commercial DOOM IWADs, textures, sprites, music, sounds, and other proprietary
assets are not included and must never be committed. Obtain any required data
lawfully; CI uses synthetic data only. A future runnable engine will need
separately supplied, appropriately licensed assets.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md), [AGENTS.md](AGENTS.md), and the
[testing guide](docs/TESTING.md). Work on a focused branch, include meaningful
tests and relevant documentation, and open a PR against `master`. Correctness,
determinism, safety, and reviewable changes take priority over optimization.
Merging requires explicit owner approval.
