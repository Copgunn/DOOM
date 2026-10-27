# Compatibility contract

## Reference and current limits

Initial target: **Linux DOOM 1.10**, preserved in `linuxdoom-1.10/`, upstream
[id-Software/DOOM](https://github.com/id-Software/DOOM) commit
`a77dfb96cb91780ca334d0d4cfd86957558007e0`.
The Rust implementation currently contains only foundational vocabulary and WAD
signature recognition. It cannot load archives/maps, simulate, render, play
demos, or compare reference states. No gameplay compatibility is established.

DOS DOOM 1.9 is a separate future target. The released Linux source is not a
complete DOS specification. Linux's `VERSION = 110` and matching demo header
acceptance do not establish DOS 1.9 replay or synchronization compatibility.

## Findings from the preserved source

| Reference | Observed behavior | Engineering implication |
| --- | --- | --- |
| `doomdef.h` | `TICRATE = 35`, `VERSION = 110` | Separate simulation tics from presentation and version guarantees |
| `m_fixed.h`, `m_fixed.c` | 16 fractional bits, widened multiply, guarded division; active `FixedDiv2` uses `double` | Specify widths, overflow and rounding from evidence; do not blindly assume integer division is equivalent |
| `m_random.c` | Separate `prndindex`/`rndindex`, increment-before-read table lookup, reset to zero | Preserve gameplay call order and reset behavior independently of presentation |
| `p_tick.c` | Append thinkers, traverse list order, defer removal; players before thinkers/specials | Define deterministic scheduling and spawn/removal behavior with reference traces |
| `w_wad.h`, `w_wad.c` | Signed 32-bit directory fields, eight-byte names, backward name search for patch precedence | Bound input safely; preserve ordered duplicate/override resolution |
| `doomdata.h`, `p_setup.c` | Ordered classic map lumps, binary records, inter-record references | Decode explicit little-endian layouts and validate before use |
| `g_game.c` (`G_BeginRecording`, `G_DoPlayDemo`) | Writes version 110 and rejects a different demo version | Test version-specific headers separately from complete demo compatibility |

`LumpName` equality in M0 is byte equality, not archive name lookup. M1 must
define lookup normalization from the actual reference paths, including unusual
case/padding bytes, and test duplicate names both within and across archives.

## Determinism objectives

- Exactly 35 logical tics per second; rendering rate and platform clocks cannot
  affect a simulation step.
- Explicit fixed-point/integer behavior, RNG sequencing, and actor/thinker order.
- Identical commands, initial state, compatibility profile, and resource bytes
  produce identical canonical state sequences across supported platforms.
- Keep input, filesystem work, audio, and rendering outside gameplay authority.
- Never use unordered containers or parallel scheduling to determine gameplay order.

## Verification to introduce

M1 tests generated archives/maps, malformed bounds and references, and patch
precedence; add parser fuzz targets and preserve minimized failures. M2 introduces
canonical per-tic state encoding and repeatability checks in debug/release and
across platforms. Include RNG indices, actor identity/order, positions/momenta,
player state, sector/special state, and pending commands/events as appropriate;
exclude pointers, padding, GPU objects, and wall clocks.

M3 compares per-tic traces against a pinned, reproducibly built reference using
legal fixtures. Capture reference commit, build settings, data hashes, commands,
profile, and first divergent field/tic. M4 adds version-specific demo fixtures,
header rejection tests, complete playback, and synchronization diagnostics.
Checksums are useful summaries, but a mismatch must have field-level diagnostics.

Reference C undefined behavior, host ABI differences, and intentional safe-input
rejections require documented decisions and regression cases, not accidental
Rust behavior. Performance benchmarks come after correctness baselines and must
record toolchain, platform, build profile, dataset, and repeatability.

## Assets

CI requires no proprietary IWAD. Generate fixtures in code or use explicitly
redistributable data with provenance and license recorded. Never commit commercial
IWADs, extracted proprietary assets, or private repository content. Locally
available reference-engine helpers are environment-specific and are not evidence
of Rust compatibility unless a comparison was actually performed.
