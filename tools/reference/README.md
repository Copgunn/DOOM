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


## M3a extension (separate provenance)

```sh
python3 tools/reference/generate_m3a.py --check
python3 tools/reference/m3a_snapshot_goldens.py --check
python tools/reference/check_headless.py
```

The new adapter imports the original extraction helper without modifying the M2
script, shim, provenance, vectors or tables. It extracts original actor link and
thing iterator/predicate, fixed DDA/intercept traversal, slide projection, point
angles, XY and input bodies. A static flat sector, zero/MF_SOLID flags, live
players with zero reaction time/no cheats, state 7, and aborting unsupported effect
stubs isolate the supported mechanics. ABI/shift/binary64 assertions and the same
GNU flags make assumptions explicit. Historical C sources remain unchanged.

The 252 rows cover 21 explicit setups and twelve tics each. Inputs, outputs,
ordered attempts, thing/line candidates and slide walls are recorded. Case 17
moves a blocker at tic 2, unlinks it at tic 3 and spawns a replacement at tic 4.
Case 20 reorders complete cell lists to test equal-fraction corner ties. Radius-sum
boundaries and widened Rust extreme policies have additional Rust tests; undefined
C position differences/abs(INT_MIN) are excluded from comparison. Enlarged adapter
intercept storage avoids unchecked C writes; cases stay within the original bound.

The v2 golden encoder independently applies explicit schema additions and clockwise
synthetic map records to Python encoding rules and C coasting values. Its separate
manifest hashes inputs, encoders and outputs. Neither golden encoder reads Rust
output. The final script compares actual headless stdout across debug/release on
all CI operating systems. No complete engine, DOS, commercial data or demo run is
claimed by these isolated fixtures.
