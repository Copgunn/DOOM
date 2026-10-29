# Contributing

Rust DOOM is an early independent Rust-native engine rewrite. Start with the
[README](README.md), [compatibility boundaries](docs/COMPATIBILITY.md) and
[roadmap](docs/ROADMAP.md) before proposing a feature.

## Machine maintenance and human direction

AI coding agents perform most implementation, tests, documentation and workflow
maintenance under human direction and oversight. Automated checks enforce
human-defined requirements; owner review and merge authorization are separate
steps. This does not mean every line has been manually reviewed. AI-generated
changes can be wrong, and passing tests does not guarantee correctness.

## A useful contribution

Explain the concrete behavior being changed, its source authority and its
specific supported inputs and behavior. Original Linux DOOM 1.10 determines
gameplay behavior; its C architecture need not be mechanically translated.
Unfamiliar terms are in the [glossary](docs/README.md#short-glossary). Keep metadata recognition,
fixture execution and full-game compatibility claims separate. Read the relevant
[source investigation](docs/README.md) before modifying a subsystem.

Use safe, idiomatic Rust, owned RNG/state and validated generational actor handles.
Preserve integer/fixed-point semantics, actor and spatial ordering, state-entry
chronology and recoverable-error atomicity. Reuse existing services rather than
adding another actor store, scheduler or damage engine. Reject unsupported
branches explicitly.

## Workflow

1. Inspect master, local changes and open PRs; work on a focused feature branch.
2. Commit source investigation before new runtime behavior. Separate substantial
   implementation, reference tooling, fixtures and documentation logically.
3. Add independent original-C evidence for admitted behavior and meaningful Rust
   safety tests. Preserve historical canonical formats, helpers and fixture bytes.
4. Run the [complete validation commands](docs/TESTING.md), including release,
   Rust 1.85.0, references, examples, relevant fuzzing and sanitizer checks.
5. Update affected guides and the [documentation checklist](docs/DOC_REVIEW_CHECKLIST.md).
6. Open a PR explaining behavior, evidence and remaining unsupported branches.
   Keep blocked work draft. Require exact-head Linux, Windows, native macOS and
   Rust 1.85 CI before requesting readiness.

Do not merge without explicit owner authorization, enable auto-merge, force-push
shared history or remove active branches. An implementation PR stays open for its
own owner review. Agent-specific enforcement rules are in [AGENTS.md](AGENTS.md).

Before changing documentation, use the [authority map](docs/README.md#documentation-authority)
to find the document responsible for that fact. Update it and link from other
pages; do not repeat milestone status, commit hashes, test totals or implementation logs.

## Evidence and reporting

Original-C comparisons verify only their declared adapters and inputs. Independent
DSIM byte checks verify the documented diagnostic schema; DSIM is not an original
savegame or demo. Atomic rejection, bounds, stale handles and presentation
independence are Rust-specific safety policies. Report these evidence families
separately. Record observed counts, immutable commit/CI references and actual
sanitizer settings; disabled leak detection provides no LeakSanitizer evidence.
Historical records belong in the [validation archive](docs/validation/archive/README.md).

## Source, assets and licensing

Keep the repository public and GPL-2.0-only. Preserve original upstream source,
history, notices and attribution. The project is independent of id Software and
Bethesda. Use only synthetic or redistributable fixtures.
Do not access unrelated private repositories, private cloud resources,
credentials or proprietary assets. Never commit commercial IWADs or publish
private paths, links, data or test output. See [external assets](docs/ASSETS.md).
