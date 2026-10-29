# M3d3j evidence and review status

This record separates observed original-C comparisons, independent encoding and
Rust-specific safety policies. Local/reference/sanitizer results below are
completed observations; exact-head CI is maintained in the feature PR, and the
About metadata blocker remains explicit.

## Verified prerequisite

PR19 corrected head `1d0675f06f9f26bb0397c822c8b2319ad34707c1` passed fresh mandatory
local checks and [CI40800289490](https://github.com/Copgunn/DOOM/actions/runs/40800289490).
The authorized regular merge is `97052fb6e2491453466b64918b15f319d88958a1`, with ordered
parents `cae36bedc9de4f0ec14b571e1730fbc041ea5ba3` and that corrected head. Tree
`65e2aac09492b4d971523a036a97c1b3ef803fe0` equals the corrected feature tree.
Local/remote master matched the merge. Exact post-merge
[CI40801082798](https://github.com/Copgunn/DOOM/actions/runs/40801082798) passed Linux,
Windows, native macos-26-intel and Rust 1.85 before investigation commit `63a78ea`
or runtime implementation. All 165 original files and 3,975 historical reference/
tool/support paths were verified unchanged, together with history/licenses.

## Original C and fixture projection

Observed new comparisons pass for 351 scripts across 54 synthetic maps using all 21
geometry bases: 922,324 actor fields, 39,340 raw gameplay RNG draws including setup/
discards, 149,398 instrumentation rows and 23,188 statistics fields. There are 469
source-ordered statistics updates. A separate unadapted original spawn pass checks
11,622 actor fields and 37,742 raw draws. Seventy genuine C bodies and two verbatim
P_SetupLevel fragments are inventoried separately. This is not whole-game or branch
coverage. The 123 filter components yield 73 admitted source comparisons and 50 Rust
policy exclusions; 10 credit components separately show original player-pointer
and network branches that the runtime does not admit. One separate genuine setup-fragment
component verifies initialized counters and retained gameplay/miscellaneous RNG
indexes 211/87; it is not complete game setup.

The old COUNTKILL flag projection is removed. Original type12/state identities
still map to labeled native136/state aliases, with raise disabled. The admitted
fallback credit branch has no actual player_t pointer and does not implement full
player scoring. Source hashes, compiler flags, ABI assumptions, adapter hashes,
inputs and outputs are in [provenance](../../tests/reference/m3d3j_provenance.json).

## Independent canonical evidence

The independent encoder produced 5,797 C-derived DSIM-v17 captures and 1,128 complete
C-derived byte fixtures, plus two independently specified complete Rust-only
atomic-capacity states, with per-tic digests and first-field comparison.
Head-actor retirement
is represented inside the thinker phase before any subsequent live Think.
[Provenance](../../tests/reference/m3d3j_snapshot_provenance.json) records exact inputs
and output hashes. Expectations never come from Rust output. Historical schemas
and bytes 1–16 remain separate immutable contracts.

The asserted `demon_map_context` example covers filters, multiple spawns, source
state timing, pursuit/melee, pain/death, corpse persistence and distinct statistics.
Its final independently expected digest is `a4dee14537bb6761`.

## Rust safety policies and limitations

Tests cover overflow before mutation, late invalid setup/capacity, stale/pending/
reused identities, explicit respawn/player/raise rejection, flag preservation,
nonlethal infighting rejection, fresh reset with original RNG continuity, pause,
presentation reads and independent instances. Safe rejection is not a claim that
original C recovers from the same fatal/undefined inputs.

Complete Demon/Spectre identity/state admission, real player scoring/damage,
Nightmare respawn, resurrection, multiplayer, generalized interactions, full level
loading and presentation remain unsupported. The next proposed investigation is
original actor/state identity and remaining game-context dependencies, subject to
separate owner authorization.

## Repository About metadata blocker

Before: `DOOM Open Source Release`.

A normal authorized CLI PATCH attempt returned `Forbidden`; the connected GitHub
tools have no repository-settings mutation. A fresh independent metadata fetch
confirmed the old description remained visible, repository identity/public status
and licensing unchanged. No credential extraction or network-policy bypass was
attempted. The criterion remains **unchecked**, so delivery remains draft pending
owner action. Requested exact owner-apply description:

> Machine-maintained, human-directed Rust-native DOOM engine rewrite focused on deterministic simulation and vanilla compatibility. Early development.

## Public documentation and historical evidence

README and CONTRIBUTING disclose machine maintenance under human direction;
README places the notice directly after the introduction. AGENTS now contains
concise standing enforcement rules. The validation guide links complete historical
records instead of duplicating a chronological ledger. Unnecessary private-resource
identifiers were removed from current public instructions without rewriting history.
The [archive manifest](archive/manifest.json) records exact original bytes/hashes,
rebased navigation copies and complete section mappings. Final observed size/link
and preservation results follow after complete validation.

## Observed local and sanitizer results

Workspace checks passed 277 tests (two private opt-in tests remained ignored).
Release simulation/type checks passed 253 tests. Build, workspace/fuzz formatting,
warnings-denied Clippy and rustdoc passed. Rust 1.85.0 workspace checks also passed 277 tests. All 17 original-C/independent-encoder pairs pass `--check`, preserving v1–v16.
All 17 asserted simulation examples match in debug/release, and the README parse-map command passes.
Exact final-head cross-platform CI is recorded in the feature PR after verification.
The About metadata criterion remains unchecked; no merge is authorized for this PR.

All 16 AddressSanitizer fuzz targets built, retaining the 15 historical targets.
The new `demon_map_context` campaign completed 4,096 actual executions with no
findings, exit 0, in 695.7 seconds. It used 12 public synthetic seeds, seed 1,
maximum 65-byte inputs, eight operations/32 tics, optional 64-slot setups,
20-second per-input timeout and 2048 MiB RSS limit. The target mutates validated
public map bytes and exercises source/target identities, flags, damage, lifecycle,
restart, capacity and complete canonical-state rejection. `ASAN_OPTIONS=detect_leaks=0`;
there is no LeakSanitizer coverage or exhaustive-coverage claim.

The working native macos-26-intel label and all previous CI steps/check names
remain intact. New C/encoder, asserted example and documentation/archive checks
are added. Linux's job budget rises from 15 to 30 minutes to retain every historical
comparison plus the new encoder; Windows/macOS retain 15 minutes. No check is
removed, substituted or made optional.

## Documentation restructuring measurements

| Public entry point | Before bytes | After bytes |
| --- | --- | --- |
| `README.md` | 45,352 | 4,302 |
| `AGENTS.md` | 30,014 | 4,521 |
| `docs/VALIDATION.md` | 142,549 | 3,278 |

Size reduction is a measurement, not evidence of usability by itself. The guides
now lead with project capabilities, build/run commands and explicit limitations,
then link source findings and detailed records. README's disclosure follows its
introductory description; CONTRIBUTING distinguishes AI work, human direction,
automated checks and actual owner review.

The old ledger's complete 142,549 bytes and SHA-256
`6b2e68e2978d644fbd34c4170eaf0d0011c39f6545d01a755e6555a9613ce9a6`
remain in [the exact copy](archive/pre-m3d3j-ledger.txt), with a
[readable navigation copy](archive/pre-m3d3j-ledger.md). Twenty full historical
documents have exact copies, hashes and section mappings in the manifest; their
navigation copies rebase links, including the old README heading reference.
Checkout rules preserve all archived bytes across Windows/macOS/Linux.

Observed documentation validation resolves 449 relative links/two anchors across 54
Markdown files and verifies 20 exact archives. A current-Markdown scan finds no
unnecessary private-resource identifiers. History was not rewritten. All 165
original upstream files, 3,975 historical reference/tool/support paths, original
ancestry/licenses and both dependency locks remain unchanged. Only synthetic
public data was used; no commercial assets or unrelated private resources were
accessed.
