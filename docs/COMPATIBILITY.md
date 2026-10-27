# Compatibility contract

## Reference and current limits

Initial target: **Linux DOOM 1.10**, preserved in `linuxdoom-1.10/`, upstream
[id-Software/DOOM](https://github.com/id-Software/DOOM) commit
`a77dfb96cb91780ca334d0d4cfd86957558007e0`.
M1 parses IWAD/PWAD bytes and structurally validates classic binary maps using
synthetic fixtures. It does not simulate, render, play demos, or compare reference
states. Commercial IWAD validation and gameplay compatibility are not established.

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

## Implemented M1 policies and deviations

- Signed little-endian WAD counts/offsets/lengths must be nonnegative and in
  bounds. Checked arithmetic and configurable limits precede access/allocation.
  Empty lumps at offsets zero through EOF are accepted; past-EOF empty markers
  are rejected. Shared/overlapping ranges, including header/directory overlap,
  are allowed when bounded. Directories need not be at the end of the file.
- Names retain all eight original bytes. Lookup truncates/pads queries to eight
  bytes, stops at NUL and folds ASCII case on both stored names and queries.
  `W_AddFile` uses `strncpy` (padding after NUL), while `W_CheckNumForName`
  uppercases only the query and scans backward. Symmetric folding extends lookup
  to lowercase directory names as requested; strict lowercase-directory bug
  emulation is not claimed. Non-ASCII bytes are not locale-folded. Raw
  `LumpName` equality remains exact byte equality.
- Later matching entries and later archives win. Maps select the last marker,
  then require THINGS through BLOCKMAP in reference order within one archive.
  Groups cannot spill across archives or borrow independently overridden lumps.
  A malformed overriding map errors instead of silently falling back.
- E#M# and MAP## markers are recognized. TEXTMAP identifies UDMF; BEHAVIOR
  immediately after the classic group identifies Hexen. Known XNOD/ZNOD,
  XGLN/ZGLN and xNd4/xNd5 node signatures are explicitly unsupported. No format
  inference can reliably distinguish arbitrary unmarked alternative layouts.
- Fixed records must have exact widths: things 10, lines 14, sides 30, vertices
  4, segs 12, subsectors 4, nodes 28, sectors 26 bytes. Essential geometry tables
  cannot be empty. Ordinary references follow signed nonnegative Linux 1.10
  shorts, limiting tables to 32768 entries; extended unsigned-index maps are
  unsupported. Only sidedef -1 is a sentinel, mapped to None; front sides and
  two-sided back sides must exist. Seg side selectors must address existing sides.
- Every subsector has a nonempty bounded seg range with one front sector. BSP
  children address valid nodes/leaves, all cycles are rejected, and partition
  deltas cannot both be zero. A node-free map requires exactly one subsector.
  Shared/unreachable acyclic nodes/leaves and overlapping subsector ranges are
  permitted; node ordering need not be topological. The last node is the root.
- REJECT must contain at least ceil(sectors²/8) bytes; trailing bytes are retained.
  Short-table overreads/emulation are intentionally rejected.
- BLOCKMAP uses signed positive dimensions, signed nonnegative word offsets,
  offsets beyond the complete cell table, leading zero list words, validated
  linedef references and -1 terminators. Shared lists/suffixes are allowed;
  unused trailing words are harmless. The exposed list includes the leading
  zero as linedef 0, as `P_BlockLinesIterator` does. Large unsigned/extended
  BLOCKMAP encodings and malformed-list recovery are unsupported.

These checks reject unsafe input the C implementation may overread, divide by
zero on, or traverse indefinitely. They establish structural safety, not that a
node builder's partitions/bounds match geometry, that a BLOCKMAP covers every
intersecting line, or that textures/thing types resolve. Those semantic and
runtime questions require later verification.

Default archive limits: 512 MiB input, 1,000,000 lumps, 128 MiB per lump.
Default map limits: 64 MiB per lump, 32768 records per table, 262144 BLOCKMAP
cells, 4194304 BLOCKMAP words, 128 MiB cumulative decoded/scratch allocation.
Applications can lower limits; raising them cannot enable unsupported formats.
All caller-provided input bytes must already have been acquired within the
application's own I/O and memory limits.

## Determinism objectives

- Exactly 35 logical tics per second; rendering rate and platform clocks cannot
  affect a simulation step.
- Explicit fixed-point/integer behavior, RNG sequencing, and actor/thinker order.
- Identical commands, initial state, compatibility profile, and resource bytes
  produce identical canonical state sequences across supported platforms.
- Keep input, filesystem work, audio, and rendering outside gameplay authority.
- Never use unordered containers or parallel scheduling to determine gameplay order.

## Verification to introduce

M1 tests generated archives/maps, malformed bounds/references, patch precedence,
adversarial sharing and iterative deep BSP validation. Separate WAD/map fuzz
targets use only synthetic seeds. Bounded smoke testing is not exhaustive proof
of compatibility or absence of defects. M2 introduces
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
