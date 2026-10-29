# Documentation guide

This directory is the starting point for the Rust DOOM engineering documentation.
The repository root [README](../README.md) is the project overview. This guide
explains where to find authoritative details without maintaining another copy of
milestone status, test counts, commit IDs, or CI results.

**For the current state of a milestone, use the [roadmap](ROADMAP.md), the
[validation ledger](VALIDATION.md), and the live GitHub pull request.** A feature
branch may describe work that has not yet reached the default branch.

## Start here

| Document | Use it for |
| --- | --- |
| [Root README](../README.md) | Overview, supported entry points, example commands, project scope |
| [Contributing](../CONTRIBUTING.md) | Setup, Rust checks, PR workflow and review expectations |
| [Engineering instructions](../AGENTS.md) | Standing constraints for Codex and other contributors |
| [Architecture](ARCHITECTURE.md) | Module boundaries, ownership, deterministic simulation and future rendering |
| [Compatibility](COMPATIBILITY.md) | What the implementation supports, rejects or has not verified |
| [Roadmap](ROADMAP.md) | Milestone sequence, declared acceptance criteria and review boundaries |
| [Testing](TESTING.md) | Reproduction commands, synthetic fixtures, fuzzing and private-test policies |
| [Validation](VALIDATION.md) | Observed evidence, exact commits, CI runs, merges and test limitations |
| [State format](STATE_FORMAT.md) | Versioned DSIM schema, field/event order and diagnostic digests |
| [Documentation review checklist](DOC_REVIEW_CHECKLIST.md) | Low-conflict documentation edits and evidence-quality checks |

The [reference-tool guide](../tools/reference/README.md) explains how the
isolated original-C generators and independent snapshot encoders are reproduced.
The [fuzz corpus guide](../fuzz/corpus/README.md) covers public synthetic seeds.

## Original-source investigations

These notes describe the original C behavior, the explicitly admitted Rust
subset, compatibility limits, and independent comparison evidence. They are
historical milestone records, not claims that the entire original game works.

| Milestone | Source investigation |
| --- | --- |
| M2 — deterministic simulation foundation | [M2_REFERENCE](M2_REFERENCE.md) |
| M3a — player movement and solid collision | [M3A_REFERENCE](M3A_REFERENCE.md) |
| M3b — static sectors, BSP and vertical movement | [M3B_REFERENCE](M3B_REFERENCE.md) |
| M3c — actor metadata, state machines and spawning | [M3C_REFERENCE](M3C_REFERENCE.md) |
| M3d1 — direct damage and hitscan | [M3D1_REFERENCE](M3D1_REFERENCE.md) |
| M3d2 — player weapons and psprites | [M3D2_REFERENCE](M3D2_REFERENCE.md) |
| M3d2b — plasma projectile foundations | [M3D2B_REFERENCE](M3D2B_REFERENCE.md) |
| M3d3a — monster sight, perception and target selection | [M3D3A_REFERENCE](M3D3A_REFERENCE.md) |
| M3d3b — monster pursuit and movement decisions | [M3D3B_REFERENCE](M3D3B_REFERENCE.md) |
| M3d3c — attack selection and bounded actions | [M3D3C_REFERENCE](M3D3C_REFERENCE.md) |
| M3d3d — sequential shotgunner attacks | [M3D3D_REFERENCE](M3D3D_REFERENCE.md) |
| M3d3e — chaingunner attack and refire | [M3D3E_REFERENCE](M3D3E_REFERENCE.md) |
| M3d3f — Demon melee foundations | [M3D3F_REFERENCE](M3D3F_REFERENCE.md) |

For later milestones, find their source notes through the
[roadmap](ROADMAP.md). Do not treat a future subphase as implemented merely
because its predecessor is merged.

## Understanding the evidence

Engineering reviews should distinguish three independent questions:

1. **Original-C behavioral comparison.** Does the Rust implementation produce
   the same observed results as original Linux DOOM 1.10 function bodies for the
   **declared tested inputs**? See the milestone reference note and
   [reference-tool guide](../tools/reference/README.md). A synthetic adapter,
   excluded branch or defined replacement for undefined C behavior limits the
   equivalence claim.
2. **Canonical state and determinism.** Do independently specified DSIM bytes
   and named fields agree, and do repeated simulations remain deterministic?
   See [STATE_FORMAT.md](STATE_FORMAT.md). DSIM is a project diagnostic format:
   it is **not** an original DOOM savegame, demo, or network protocol.
3. **Rust-specific safety policies.** Do invalid handles, malformed inputs,
   capacity limits, unsupported actions and other recoverable failures reject
   safely and atomically? These tests do **not** expand original-C equivalence.

A successful CI run proves its executed checks passed on a specific commit,
not complete vanilla compatibility. Linux reference reproduction may have a
different execution path from the Windows/macOS stable test jobs; consult
the [testing guide](TESTING.md) and the actual CI job steps.

**Profiles are explicitly selected, not automatically cumulative.** For
example, a later projectile profile does not necessarily admit the previous
player-weapon profile's full behavior. Consult [Compatibility](COMPATIBILITY.md)
and the precise [DSIM schema](STATE_FORMAT.md) before asserting a capability.

## Common checks

From the repository root:

~~~sh
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p doom-sim -p doom-types --release --locked
cargo doc --workspace --no-deps --locked
~~~

The minimum supported Rust compiler is tested separately by CI. Original-C
reference generators, independent encoders, fuzz builds and debug/release example
checks are documented in [TESTING.md](TESTING.md); do not guess their current
number or mark them passed without actually running them.

Public tests use synthetic or otherwise redistributable data. Do not stage,
publish, provision, or log private/commercial IWADs, textures, sound, credentials
or private repository material. The engine's GPL-2.0-only source license does
not license proprietary game assets.

## Keeping documentation in sync

- Update [README](../README.md) for user-facing capabilities and examples.
- Update [ARCHITECTURE.md](ARCHITECTURE.md) for module boundaries and ownership.
- Update [COMPATIBILITY.md](COMPATIBILITY.md) for admission, exclusions and
  deliberately different safe behavior.
- Update [ROADMAP.md](ROADMAP.md) for acceptance and milestone progress.
- Append observed, reproducible evidence to [VALIDATION.md](VALIDATION.md),
  including the exact commit and completed CI run; avoid claiming that a commit
  contains its **own** SHA.
- Update [STATE_FORMAT.md](STATE_FORMAT.md) only for reviewed canonical changes.
  Keep old version fixtures and generators intact.
- Update milestone source notes when reference findings or their limits change.
- Follow the [review checklist](DOC_REVIEW_CHECKLIST.md) for docs-only edits.

Keep focused documentation changes on their own branch and PR, especially while
a gameplay milestone is active. Do not merge or rewrite history without owner
authorization. Historical upstream code, [original release notes](../README.TXT)
and [license](../LICENSE.TXT) must remain untouched.

- [M3d3g Demon pain/death investigation](M3D3G_REFERENCE.md): source timing, RNG, corpse collision and admission proof obligations.
- [M3d3h Demon stand/run investigation](M3D3H_REFERENCE.md): original state closure, source timing and explicit admission plan.
- [M3d3i source investigation](M3D3I_REFERENCE.md): bounded original Demon THING spawning and explicit actor admission.
