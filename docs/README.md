# Documentation

Start with the [project README](../README.md). These guides explain the engine
without requiring familiarity with milestone names or earlier agent sessions.

| Need | Read |
| --- | --- |
| Design and crate boundaries | [Architecture](ARCHITECTURE.md) |
| Supported behavior and explicit limits | [Compatibility](COMPATIBILITY.md) |
| Current work and dependencies | [Roadmap](ROADMAP.md) |
| Reproduce tests, examples and sanitizers | [Testing](TESTING.md) |
| Interpret evidence and find detailed records | [Validation](VALIDATION.md) |
| Canonical fields and encoding | [State format](STATE_FORMAT.md) |
| Contribution and human review workflow | [Contributing](../CONTRIBUTING.md) |
| Agent enforcement rules | [AGENTS](../AGENTS.md) |
| Optional external game data | [Assets](ASSETS.md) |
| Documentation quality/evidence review | [Checklist](DOC_REVIEW_CHECKLIST.md) |

## Original-source investigations

These technical notes document source decisions, explicit adaptations and input
slices. They do not claim complete game compatibility.

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

[Reference tooling](../tools/reference/README.md) explains generation and provenance.
[Historical records](validation/archive/README.md) retain full original evidence
and former guide contents with exact-byte copies and digests.
