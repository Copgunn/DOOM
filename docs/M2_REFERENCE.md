# M2 source investigation and scope

Recorded before implementation. Authority: preserved Linux DOOM 1.10 at
`a77dfb96cb91780ca334d0d4cfd86957558007e0`. This document distinguishes observed
source behavior, defined Rust policies, and unimplemented mechanics.

## Confirmed observations

- `m_fixed.h/c`: signed 32-bit 16.16; multiplication widens to signed 64 bits,
  shifts right 16, then narrows. `FixedDiv` saturates when `(abs(a)>>14)>=abs(b)`;
  its active helper divides in `double`, scales by 65536 and converts toward
  zero. The guard is deliberately conservative; do not replace it with a generic
  saturation check or infer equivalence from integer algebra alone.
- `m_random.h/c`: distinct gameplay/miscellaneous eight-bit indexes, increment
  before lookup into the exact 256-entry table, reset both indexes to zero.
- `p_tick.h/c`: append thinkers at the tail. New tails can run in the current
  traversal, including when spawned by the current tail. Marked thinkers skip
  their update and are unlinked when visited. Self-removal during an update is
  not inspected again until the next traversal. Removing an already visited
  thinker likewise waits for the next traversal. Players precede thinkers,
  then specials and respawn phases; `leveltime` increments afterward. Pause
  returns before those phases. Menu/network/demo pause rules need higher layers.
- `p_mobj.c`: spawn consumes gameplay RNG for `lastlook` and appends a thinker;
  full spawning additionally needs type/state information. `P_MobjThinker`
  performs XY, Z, then state transitions/respawn. M2 implements only grounded
  non-player XY movement, not the complete spawn/state machinery.
- `P_XYMovement`: clamp each momentum component to ±30 units; split when either
  remaining component is **positive** and greater than 15 units. Position uses
  `/2` (toward zero), while the remaining component uses arithmetic `>>1`.
  This negative/positive and odd-negative asymmetry must remain. Non-player
  blocking zeros momentum. Ground friction is FixedMul by `0xe800`, with strict
  component thresholds ±`0x1000` for stopping. Players, corpses, missiles, skulls,
  airborne motion and their special responses are outside the supported profile.
- `p_map.c`: destination bounding-box overlap plus `P_BoxOnLineSide`, not swept
  collision; a crossed one-sided line blocks. `P_CheckPosition` scans BLOCKMAP
  x outermost, y innermost. `P_BlockLinesIterator` (`p_maputl.c`) visits stored
  order, including initial zero, deduplicating first occurrence by validcount.
  `P_TryMove` checks fit/step/dropoff and performs sector/special transitions.
  M2's single static sector permits only the fit check and one-sided blocking.
- `p_maputl.c`: line-side tests use axis fast paths and shifted FixedMul for
  diagonal sides; box tests preserve strict comparisons and slope orientation.
- `p_user.c`, `g_game.c`, `d_ticcmd.h`, `doomdef.h`: 35 Hz, ordered player slots,
  signed command bytes, angleturn shifted 16, ground thrust command×2048 using
  the original fine sine/cosine tables. These are command-application foundations,
  not complete player thinking, sliding, networking, weapons or demo semantics.
- `p_mobj.h`, `d_think.h`, `d_player.h`, `r_defs.h`: pointer relationships and
  storage layout are not portable state. Safe owned actors, generation checks,
  explicit processing order and canonical field encoding replace those layouts.

## Ambiguities and deliberate safe policies

- Negative signed shifts, narrowing outside i32, signed overflow, `abs(INT_MIN)`,
  negative angleturn left shift, and floating NaN-to-int conversions are not all
  defined by portable C. Rust uses explicit two's-complement wrapping for fixed
  add/sub/negation/narrowing, arithmetic right shifts, unsigned magnitudes for
  the division guard, and checked errors for direct helper zero/range failure.
  Reference comparisons exclude undefined C inputs and record compiler settings.
- `P_RunThinkers` reads `currentthinker->next` after `Z_Free(currentthinker)`.
  A reference adapter retains retired storage until traversal ends, preserving
  observed links/order without reproducing use-after-free. Rust saves the next
  handle before unlinking. An explicit visit budget bounds user callbacks.
- Identity is store-scoped slot+generation; lowest reusable slot is chosen
  deterministically, but thinker order is separate and append-only on spawning.
  Generation exhaustion retires a slot permanently. Marking removal preserves
  readable storage until traversal retirement; stale handles fail afterward.
- Command-controlled movers use the non-player XY response. Attaching an input
  slot does not make them fully implemented vanilla players. Unsupported buttons,
  portals, multi-sector motion and line specials must fail explicitly.
- Static map/BLOCKMAP geometric accuracy is not proven by M1. M2 consumes the
  validated lists faithfully; it does not replace missing spatial data with a
  different collision algorithm. Overflow-policy cases and out-of-profile maps
  must not be advertised as independently verified vanilla behavior.

## Reference evidence plan

Use a standalone harness built from the preserved fixed/RNG code and extracted
geometry/XY/thinker function bodies. Record source hashes, host/compiler/flags,
inputs and vectors under `tests/reference/`; compare those recorded vectors in
stable Rust on every CI platform. Arithmetic/RNG/isolated movement/order evidence
does not establish complete gameplay synchronization or DOS 1.9 compatibility.

Headless regression snapshots include all implemented state and static map data,
with a versioned field order and little-endian encoding. Project golden digests
are regression baselines, distinct from independently obtained C vectors.

## Implemented evidence

The isolated harness now links untouched fixed/RNG sources and extracts the
listed thinker, geometry, position/try-move, XY and thrust bodies. `shim.h`
retains retired thinker storage, defines unsigned angle shifting, and aborts
unsupported gameplay branches. Two movement layouts distinguish shared-list
behavior from x-outer/y-inner candidate ordering. A mixed removal/spawn trace
also verifies that reusing a lower Rust slot during traversal preserves append
order. Compiler/source/adapter provenance and recorded vectors are committed;
[validation notes](VALIDATION.md) distinguish local and remote observations.
