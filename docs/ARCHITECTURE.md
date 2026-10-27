# Architecture

## Scope and decisions

M0 establishes three library crates alongside the untouched original source.
It contains no executable, archive loader, map parser, simulation, or renderer.
The C source is an authoritative behavioral reference, not a Rust design template.

Current Cargo dependency edges (arrows mean “depends on”):

```text
doom-wad -> doom-types
doom-map (standalone until decoding needs shared types)
```

- `doom-types`: 35 Hz constant and lossless raw `LumpName`. Keep this a small
  shared leaf with no I/O. Fixed-point arithmetic belongs in M2, with explicit
  integer width, overflow, shift, rounding, and division behavior.
- `doom-wad`: exact IWAD/PWAD magic recognition and unvalidated signed directory
  metadata. In M1, own archive bytes, bound directory/lump access, and define
  ordered archive lookup. Do not treat `LumpInfo` construction as validation.
  An ignored integration probe reads signatures from `DOOM_IWAD_DIR`; this is
  test configuration only. M1 archive loading and the future client must accept
  externally supplied paths without bundling assets or embedding private paths.
- `doom-map`: signed classic `Vertex` value representation. In M1, decode map
  records and validate all references before exposing a usable map. Rust struct
  layout is never the disk format; decode little-endian bytes explicitly.

There are no third-party dependencies. Add dependencies only for a concrete need,
with rationale, license review, platform impact, and determinism implications.
Use stable Rust, edition 2024, minimum 1.85, and Cargo resolver 3. Workspace
lints currently forbid unsafe code and warn on undocumented public APIs.
Do not add empty crates for the future components below.

## Intended boundaries

| Future component | Owns | May consume |
| --- | --- | --- |
| `doom-sim` | Per-tic gameplay state, fixed-point rules, RNG, ordered thinkers | `doom-types`, validated `doom-map` data |
| `doom-demo` | Versioned demo command decoding/encoding and replay | `doom-types`; replay drives `doom-sim` through commands |
| `doom-render` | Presentation snapshots and renderer interface | Immutable simulation output and resource views |
| `doom-render-software` | Minimal diagnostic software/headless path | Renderer interface |
| `doom-render-vulkan` | Primary hardware presentation, GPU resources | Renderer interface; platform surface |
| `doom-platform` | OS events, clocks, windows, input acquisition | Platform libraries, no gameplay authority |
| `doom-audio` | Playback and mixing | Ordered sound events and resources |
| `doom-client` | Composition, startup, resource loading, scheduling | All required boundaries through explicit APIs |
| `doom-compat` | Reference profiles, state comparisons, diagnostics | Simulation/replay public APIs and legal test data |

M1 may add `doom-map -> doom-wad` and `doom-map -> doom-types` where decoding
actually needs them. Keep dependencies acyclic. Resource bytes belong to the
archive/resource layer; validated map structures belong to the map layer;
runtime actors and mutable sector state belong to simulation. Do not embed GPU,
windowing, filesystem, or audio objects in gameplay data.

## Data flow and determinism

```text
archive bytes -> bounded WAD view -> decoded/validated map -> simulation state
OS input or demo -> ordered per-tic commands -> simulation.step -> snapshots/events
snapshots -> software/headless or Vulkan presentation
events -> audio; recorded commands/state -> replay diagnostics
```

The client scheduler accrues time and executes exactly **35 logical tics per
second**. A simulation step accepts explicit commands, never frame duration or a
wall clock. Rendering frame rate, interpolation, audio timing, and I/O completion
must not affect tic outcomes. Catch-up policy and command assignment must be
specified and tested when the scheduler is implemented.

Preserve vanilla fixed-point/integer semantics and the precise RNG table,
indices, reset points, and call sequence. Keep gameplay RNG separate from
cosmetic RNG. Preserve deterministic actor/thinker insertion, update, and removal
ordering; stable identifiers alone do not guarantee stable iteration. Use
explicit ordered storage. Never depend on randomized hash iteration, pointer
addresses, thread scheduling, or unordered parallel reductions in gameplay.
Rendering can interpolate immutable snapshots but cannot mutate simulation or
consume its RNG. Vulkan is the eventual primary hardware backend; a minimal
software or headless path must support automated verification.

## Ownership and input safety

Prefer safe Rust and explicit owners over global mutable state. Future actor
relationships use validated handles/stable identifiers with a specified stale
handle policy (for example, generation checks); avoid raw pointers between actors.
Any future unsafe code requires a deliberately reviewed lint-policy change,
documented safety invariants, justification, and tests.

Treat archive and demo data as untrusted. Validate signed fields before unsigned
conversion, checked offset/size arithmetic before slicing, record widths before
decoding, allocation limits before allocating, and every index/cross-reference
before traversal. Reject malformed input with structured errors, without panic,
out-of-bounds access, or attacker-controlled unbounded allocation. Define the
permitted behavior of zero-size marker lumps and overlapping/shared ranges in
M1 rather than assuming all unusual layouts are invalid.

Compatibility is an observable behavior contract, not permission to reproduce C
undefined behavior. Specify ambiguous behavior with pinned reference evidence;
document safe deviations. See [compatibility](COMPATIBILITY.md) and
[test extension rules](TESTING.md).
