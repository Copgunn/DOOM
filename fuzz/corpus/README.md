# Seed provenance

All seeds are original synthetic bytes created by this project's
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
