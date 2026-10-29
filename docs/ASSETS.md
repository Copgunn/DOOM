# External game assets

Public build/tests/examples use synthetic or legally redistributable fixtures.
No commercial IWAD is bundled, needed for CI or authorized for this milestone.
The GPL source license does not license proprietary game data.

For a separately authorized local environment, keep legally obtained IWADs outside
the checkout and configure `DOOM_IWAD_DIR` in your private shell. Existing ignored,
read-only parser validation can then be explicitly enabled:

```sh
cargo test -p doom-map --test private_iwad -- --ignored
```

The library loads supplied bytes; a complete engine client that automatically
finds IWADs is not implemented. These opt-in tests check archive/map structure,
not authenticity, gameplay, rendering or demo compatibility. They avoid printing
private filenames/paths/bytes; a corresponding doom-wad test checks signatures.

Keep original assets unchanged. Connected storage access alone does not authorize
copying assets into a cloud workspace. Never commit or upload commercial assets to
branches, PRs, releases, CI artifacts, snapshots or logs. Keep private locations,
links, credentials and output out of public documentation. Ignore rules protect
local assets and output but cannot prevent forced staging.
