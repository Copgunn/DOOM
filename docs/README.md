# Documentation

Start with the [project README](../README.md). These guides explain the engine
without requiring familiarity with milestone names or earlier agent sessions.

## Documentation authority

Every information category has one authoritative home. Update that home, then
summarize briefly and link from other pages. Do not maintain parallel milestone
status tables, test-count lists, commit histories or schema definitions.

| Information | Authoritative home |
| --- | --- |
| Purpose, quickstart and high-level capabilities | [README](../README.md) |
| Current milestone summary and planned work | [ROADMAP](ROADMAP.md) |
| Exact supported behavior and runtime/parser limits | [COMPATIBILITY](COMPATIBILITY.md) |
| Architecture and subsystem ownership | [ARCHITECTURE](ARCHITECTURE.md) |
| Canonical simulation snapshot schemas | [STATE_FORMAT](STATE_FORMAT.md) |
| Build, test and reproduction commands | [TESTING](TESTING.md) |
| Validation methodology and evidence navigation | [VALIDATION](VALIDATION.md) |
| Original-C investigation and technical findings | The corresponding REFERENCE note below |
| Detailed milestone test observations | The corresponding `docs/validation/` record, such as [M3d3j](validation/M3D3J.md) |
| Exact review commits, CI, reviews and merge history | [GitHub PRs](https://github.com/Copgunn/DOOM/pulls) and [Actions](https://github.com/Copgunn/DOOM/actions) |
| Standing agent engineering rules | [AGENTS](../AGENTS.md) |
| Human contributor workflow | [CONTRIBUTING](../CONTRIBUTING.md) |
| Historical observations | [Frozen records](validation/archive/README.md) |

The [documentation checklist](DOC_REVIEW_CHECKLIST.md) applies this ownership model.
[External asset guidance](ASSETS.md) covers separately authorized local data;
public examples and checks use synthetic or redistributable inputs.

## Short glossary

- **Deterministic simulation:** the same inputs and initial state produce the same results.
- **Fixed-point arithmetic:** integer-based fractions with explicit rounding and overflow rules.
- **RNG:** random-number generator; gameplay owns its sequence rather than using global randomness.
- **BSP / BLOCKMAP:** the map's binary space partition tree / collision grid.
- **Profile:** an explicitly selected simulation configuration with documented support limits.
- **Admission:** the checks deciding which behavior a configuration actually supports.
- **Fixture / fixture projection:** a controlled test setup / a documented adaptation
  between original game data and that setup, not proof of complete game support.
- **DSIM:** this project's deterministic simulation snapshot format; its versions
  are defined in STATE_FORMAT. It is not an original demo or savegame.
- **Original-C equivalence:** matching specific behavior observed from genuine
  original source functions under the stated inputs and adapters, not whole-game compatibility.

## Original-source investigations

These technical notes retain their original investigation and observation context.
They are not live milestone-status pages; consult ROADMAP and the linked GitHub PR
for present status. Historical review statements are not new claims that work is pending.

| Foundation | Investigation |
| --- | --- |
| Numeric/headless simulation | [M2](M2_REFERENCE.md) |
| Collision and static sectors | [M3a](M3A_REFERENCE.md), [M3b](M3B_REFERENCE.md) |
| Actor lifecycle | [M3c](M3C_REFERENCE.md) |
| Shared damage and tracing | [M3d1](M3D1_REFERENCE.md) |
| Weapons and projectiles | [M3d2](M3D2_REFERENCE.md), [M3d2b](M3D2B_REFERENCE.md) |
| Perception and pursuit | [M3d3a](M3D3A_REFERENCE.md), [M3d3b](M3D3B_REFERENCE.md) |
| Selected monster attacks | [M3d3c](M3D3C_REFERENCE.md), [M3d3d](M3D3D_REFERENCE.md), [M3d3e](M3D3E_REFERENCE.md) |
| Demon melee/lifecycle | [M3d3f](M3D3F_REFERENCE.md), [M3d3g](M3D3G_REFERENCE.md) |
| Demon stand/run and map spawning | [M3d3h](M3D3H_REFERENCE.md), [M3d3i](M3D3I_REFERENCE.md) |
| Demon map totals and bounded credit | [M3d3j](M3D3J_REFERENCE.md) |
| Bounded larger static geometry | [M3f1](M3F1_REFERENCE.md), [measured evidence](validation/M3F1.md) |
| Expanded coordinates and spatial work | [M3f2](M3F2_REFERENCE.md), [measured evidence](validation/M3F2.md) |
| Demon gameplay on larger static maps | [M3f3](M3F3_REFERENCE.md), [measured evidence](validation/M3F3.md) |

[Reference tooling](../tools/reference/README.md) describes harness layout and
provenance. [Historical records](validation/archive/README.md) retain complete
original evidence and former guide contents with exact-byte copies and digests.
