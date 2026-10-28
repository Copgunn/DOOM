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
    (p / f'public_seed_{selector}').write_bytes(
        bytes([selector]) + b''.join(bytes(op) for op in ops))
PYSEED
```

The new target and seed recipe contain no game assets. Evolving local corpus,
coverage and crash output stay ignored or outside the checkout. Report actual
ASan execution counts and leak-detection configuration after running the campaign.
