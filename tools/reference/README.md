# Isolated Linux DOOM reference vectors

Run from the repository root on a supported GNU C host:

```sh
python3 tools/reference/generate.py
python3 tools/reference/generate.py --check
```

The first command intentionally regenerates vectors/tables and provenance; review
all changes. `--check` compares recorded results and generated Rust tables without
replacing host-specific provenance. Stable tests read committed vectors and need
no C compiler. Linux CI also reproduces them; Windows/macOS run Rust comparisons.

The script links the untouched `m_fixed.c` and `m_random.c`, extracts the original
point/box predicates, BLOCKMAP iterator, position/try-move, XY movement, thrust,
player-move, and thinker bodies, and builds a temporary standalone executable.
It never reads WAD assets or builds the full engine. Inputs/results are in
`tests/reference/`; the manifest records source hashes, compiler, flags, platform,
and adapter hashes. Original id Software source/table attribution and GPL-2.0-only
licensing apply to the derived fixtures and generated lookup tables.

The adapter retains thinker storage instead of freeing it before a link read.
Negative angleturn shifts use an explicit unsigned shift, defining a C-undefined
case according to the documented Rust wrapping policy. Pointer sentinel casts,
negative right shifts and out-of-range signed narrowing are GNU compiler-profile
observations, not portable C guarantees. FixedDiv inputs involving abs(INT_MIN)
are excluded; the direct floating helper excludes zero divisors and traps its
fatal range path through setjmp/longjmp. Rust extreme-value policies are tested
separately. No NaN-to-int conversion is invoked.

Movement uses the synthetic square's one static sector, no other actors, no
specials/portals and grounded non-player flags. Unsupported branches abort.
Thrust is compared separately from complete player thinking/sliding/state changes.
Candidate order and attempted-move counts are recorded, including the original
positive-only substep split and strict contact/stop thresholds. Thinker traces
record update order rather than allocator addresses or removed storage.

These are isolated reference comparisons, not full-engine synchronization,
complete gameplay verification, or DOS 1.9 compatibility. Project snapshot goldens
have separate provenance and must not be represented as C reference traces.
