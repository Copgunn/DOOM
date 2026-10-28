# Seed provenance

The original parser seeds below are synthetic bytes created by this project's
`tests/support/mod.rs`, licensed GPL-2.0-only. No commercial game data is included.

- `wad/empty_iwad`: twelve-byte empty IWAD.
- `wad/duplicates`: two differently cased names in a PWAD.
- `map/classic_square`: complete classic square map, one sector and leaf, four
  walls, a thing definition, visibility byte, and shared BLOCKMAP lists.

| Seed | Bytes | SHA-256 |
| --- | ---: | --- |
| `wad/empty_iwad` | 12 | `1c752adca4dce342d362ef48c4df48371f3903a59f0ad3e4637af643fc2081a7` |
| `wad/duplicates` | 45 | `c9c09f0c9f0afe9362ed8f2a700dbb3a660acc7d7aa9a3ef71d8d036e5beefce` |
| `map/classic_square` | 497 | `8d3f63a56a68b137351751a7a61919be93ad917b4d77bdb26644aa02c4672d9b` |

Regenerate from repository root with:

```sh
cargo run -p doom-map --example generate_fuzz_seeds --locked
```

Do not add data acquired from private assets. Evolving generated corpora and
crash artifacts are ignored; review and minimize new findings before committing
them as synthetic regressions.

## Chaingunner action seeds

The six M3d3e seeds are public synthetic action bytes, licensed GPL-2.0-only.
They select ordinary, shadow, null, early-refire/dead, repeated-cycle and occluded
fixtures from the original-C synthetic corpus. Each seed contains a selector and
eight bounded API operations; later generated mutations stay ignored. Create a
fresh corpus from repository root with:

```sh
python3 - <<'PYSEED'
from pathlib import Path
p = Path('fuzz/corpus/chaingunner_attacks')
p.mkdir(parents=True, exist_ok=True)
ops = [
    [0,0,1,0,0,0,0,0], [13,0,1,0,0,0,0,0],
    [1,0,1,0,0,0,0,0], [5,0,1,0,0,0,0,7],
    [4,0,1,0,0,0,0,0], [15,0,1,0,0,0,0,0],
    [6,1,1,0,0,0,0,0], [7,0,1,0,0,0,0,1],
]
for selector in range(6):
    (p / f'public_seed_{selector}').write_bytes(
        bytes([selector]) + b''.join(bytes(op) for op in ops))
PYSEED
```

The reported campaign used the same six 65-byte seeds in a fresh temporary corpus,
not a resumed mutation corpus. No commercial or private data is involved.


## Demon melee seeds

Six public synthetic 65-byte seeds select ordinary, shadow, null, outside-range,
repeated-cycle and wall-occluded fixtures. At most eight API operations run per
input; each step operation checks up to three individual tics (24 total), with
canonical equality checked separately for every failed tic. Geometry mutation
uses only the supplied synthetic map bytes, at most eight byte changes; parser
and strict constructor errors are accepted without claiming C equivalence.
Create a fresh temporary corpus from repository root:

```sh
python3 - <<'PYSEED'
from pathlib import Path
p = Path('/tmp/doom-demon-melee-corpus')
p.mkdir(parents=True, exist_ok=True)
step = [13,0,1,0,0,0,0,2]
for selector in range(6):
    first = [5,0,1,0,0,0,0,4] if selector == 1 else [1,0,1,0,0,0,0,0]
    ops = [step]*8 if selector == 4 else [first]+[step]*7
    selector_byte = selector | (0x80 if selector in [0,1,4] else 0)
    (p / f'public_seed_{selector}').write_bytes(
        bytes([selector_byte]) + b''.join(bytes(op) for op in ops))
PYSEED
```

The high selector bit pre-fills the bounded fixture to 64 actors before the
eight mutational operations, covering capacity hits, misses and spawn errors.
The low seven bits still select the same public fixture.

The new target and seed recipe contain no game assets. Evolving local corpus,
coverage and crash output stay ignored or outside the checkout. Report actual
ASan execution counts and leak-detection configuration after running the campaign.

## M3d3g Demon lifecycle seeds

Twelve fresh public synthetic 65-byte seeds select eight source-backed setup
families: pain, ordinary death, corpse collision, hitscan, moving melee, ledge,
fall-forward and self-melee. Four seeds set the high selector bit and pre-fill
64 slots. Each input admits at most eight API operations and 32 individually
checked tics, plus eight mutations to public map bytes for constructor validation.
Every failed operation/tic checks full canonical equality. This is bounded safety
fuzzing, not a complete branch-coverage or original-C-equivalence claim.

Create the same fresh seed corpus from repository root:

```sh
DOOM_LIFECYCLE_CORPUS=$(mktemp -d)
export DOOM_LIFECYCLE_CORPUS
python3 - <<'PYSEED'
from pathlib import Path
import os
p = Path(os.environ['DOOM_LIFECYCLE_CORPUS'])
step = bytes([13,0,1,0,0,0,0,3])
hit = bytes([0,0,1,150,0,0,0,1])
move = bytes([11,1,1,0,0,252,255,3])
for i in range(12):
    selector = (i % 8) | (0x80 if i >= 8 else 0)
    ops = [step] * 8
    if i % 8 == 0:
        ops[0] = hit
    if i % 8 == 2:
        ops = [move,step,step,step,step,step,move,step]
    (p / f'public_seed_{i}').write_bytes(bytes([selector]) + b''.join(ops))
PYSEED
ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run --sanitizer address \
  demon_lifecycle "$DOOM_LIFECYCLE_CORPUS" -- \
  -runs=10000 -seed=1 -max_len=65 -timeout=20 -rss_limit_mb=2048
```

The recipe mirrors the reported fresh-corpus campaign; mutations and crash output
stay ignored or outside the checkout. No proprietary assets are involved.
`detect_leaks=0` disables leak detection: do not claim LeakSanitizer coverage.
Record actual campaign completion/counts/findings in the validation ledger.
