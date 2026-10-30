# Validation guide

Compatibility claims are limited to independently demonstrated input slices.
A passing test suite or matching digest does not prove a complete game, original
savegame compatibility or vanilla demo synchronization.

## Evidence families

| Evidence | What it establishes | Main records |
| --- | --- | --- |
| Genuine original C | Source decisions, fields, RNG and ordering under documented adapters | Milestone REFERENCE notes and `tests/reference/*provenance.json` |
| Fixture projection | Explicit native identity/state adaptations compared with original C | Source investigations and adapter assumptions |
| Independent DSIM | Complete bytes/per-tic digests from C observations plus documented schema; no Rust expected-output generation | [State format](STATE_FORMAT.md), independent Python encoders |
| Rust safety policies | Checked arithmetic, bounds, atomic rejection, stale handles and independent instances | Policy/unit tests, [compatibility](COMPATIBILITY.md) |
| Platform/toolchain | Recorded checks execute on supported hosts and Rust versions | Exact-head GitHub Actions, [testing](TESTING.md) |

Compiled source-function inventory is not branch coverage. Setup fragments are
reported separately from whole functions. Unadapted C, declared fixture projection,
canonical consistency and Rust-specific rejection are never interchangeable claims.

## Reproduce

[TESTING](TESTING.md) owns build, format, Clippy, workspace/release, rustdoc/MSRV,
reference/encoder, example, documentation and fuzz reproduction commands.
Cross-platform CI requires Linux, Windows, native macOS and
Rust 1.85.0. Original-C regeneration runs on Linux; other hosts compare recorded
observations and independent canonical fixtures.

Sanitizer reports name the actual target, corpus/input model, execution count,
limits, findings and leak setting. Disabled leak detection does not provide
LeakSanitizer coverage; no bounded campaign is exhaustive.

## Detailed records

- [M3g1 stepping and transaction costs](validation/M3G1.md): complete-state API
  refinement, capture-boundary regression hooks, reused C observations, and
  separately instrumented timing/allocation measurements. No new C coverage.

- [M3f9 game-instance provenance](validation/M3F9.md): Rust-specific incarnation,
  import/export, restart and trust policy; the selected C input is reused evidence,
  not new original-source coverage. Historical DSIM and gameplay remain separate.

- [M3f8 context/target ownership](validation/M3F8.md): new compact C inputs,
  retained references and logical retirement, independently encoded v21 captures,
  and separately labeled lifetime-brand/generation/atomicity policy checks.

- [M3f7 original-ID registry/profile and bounded diagnostics](validation/M3F7.md):
  new encoding of reused C inputs; actual runtime storage and exact ordered event
  comparisons remain distinct from metadata, digests and Rust-only reader policy.

- [M3f6 runtime identity audit and boundary](validation/M3F6.md), reusing independent
  C observations without claiming new capture or gameplay coverage.

- [M3f5 canonical Demon execution binding](validation/M3F5.md), including raw
  canonical C observations and separately projected DSIM-v20 comparisons.
- [M3f4 canonical identity metadata](validation/M3F4.md).

- [M3f3 combined Demon/expanded-map evidence](validation/M3F3.md).

- [M3f2 expanded-coordinate/spatial-work evidence](validation/M3F2.md).
- [M3f1 map-scale source comparisons and host measurements](validation/M3F1.md).

- [M3d3j map-context implementation and delivery evidence](validation/M3D3J.md).
- [M3d3j original-source investigation](M3D3J_REFERENCE.md).
- [Reference-tool reproduction and provenance](../tools/reference/README.md).
- [All historical engineering records](validation/archive/README.md), including
  the complete [pre-M3d3j ledger](validation/archive/pre-m3d3j-ledger.md).

## Historical evidence preservation

The old validation ledger is retained byte-for-byte in the
[archive](validation/archive/README.md). Its readable Markdown copy rebases links. The
[manifest](validation/archive/manifest.json) records source commit, hashes, sizes,
archive destinations and every original heading. Other former guides are also
archived intact, preserving detailed restrictions and evidence.

Historical SHA/CI references, source hashes, qualified counts and sanitizer limits
remain historical observations. Moving them does not assert they were rerun on a
new commit. Actual generators, encoders, fixtures and historical helper bytes remain
at their existing paths. The [documentation checklist](DOC_REVIEW_CHECKLIST.md)
governs evidence and link review.

## Keep observations separate from current status

Detailed measured results belong in milestone records, not this guide or other
entry points. ROADMAP owns the development summary; GitHub PRs/Actions own exact
review commits, statuses and CI history. Source investigations and frozen archives
describe their original observation context. Follow the
[authority map](README.md#documentation-authority) rather than copying those facts.

M3f4 adds a separate compiled-C [identity metadata observer](M3F4_REFERENCE.md),
without changing historical DSIM captures. [Measured observations](validation/M3F4.md)
separate complete metadata recognition from unchanged executable admission.

## Original-player evidence boundary

[M3g2](validation/M3G2.md) adds genuine original-player damage/death inputs, C field
and ordered-event comparisons, and independently encoded DSIM-v22 fixtures. It
reuses the existing source extraction services while separately counting executed
function entries, inherited inventory and the newly compiled P_DropWeapon body.
The association fragment and tactile/psprite request boundaries are explicit;
controls, weapons, death thinker and automatic roster/monster acquisition are not
inferred. Rust generation/provenance/atomicity tests remain policy evidence.
Historical observers, fixtures and encoders continue to run unchanged. The
[source matrix](M3G2_REFERENCE.md) and [support contract](COMPATIBILITY.md#original-player-body-lifecycle)
define what each comparison proves.

[M3g3](validation/M3G3.md) adds original-player physical impulse and actual grounded
movement, friction, axis sliding and moving-corpse observations. Genuine map-spawn
origins retain previous ownership/RNG semantics. Independent DSIM22 policy3 captures
and compact complete event frames remain distinct from Rust-only forward bounds,
provenance and rollback guarantees. Three source slide bodies extend this observer
but already occur in historical movement evidence; their inventory is not globally
new function or branch coverage. All old observers, schemas and fixtures remain
regression gates.


[M3g4](validation/M3G4.md) adds command-gate/P_MovePlayer observations, compact
per-tic motion/command rows and independently encoded DSIM22 policy4 fixtures.
Selected full captures overlap these rows; do not sum them as unique coverage.
The unsigned negative-turn adapter, literal defined cases and deliberate UB
expression probe have distinct claims. Rust admission, generation/provenance,
zero-command continuation and late transaction checks remain separate safety
policy. Every older observer, fixture and decoder remains a regression gate.


[M3g5](validation/M3G5.md) adds genuine active-player/MT_SERGEANT source scenarios.
The new observer executes original scan/Look/Chase bodies against a genuine player
association, with the explicitly bounded command gate before the thinker pass.
This is neither full PlayerThink nor full P_Ticker execution. Source roster rows
are independently checked against player/mo fields before policy5 DSIM22 encoding.
Per-tic motion/Demon/decision rows overlap selected full captures and are counted
separately. All-inactive C scanning is never run. Retired source identity uses the
declared quarantine observation; stale/foreign generations, fixed-body removal
rejection and transactional melee refusal are Rust policy. Historical evidence,
source extraction and encoders remain unchanged; new expected values never come
from Rust output. See the [source matrix](M3G5_REFERENCE.md).
