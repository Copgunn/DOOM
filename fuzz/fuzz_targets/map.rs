#![no_main]
use doom_map::{Map, MapLimits};
use doom_wad::{Limits, Wad};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(wad) = Wad::parse(
        data,
        Limits {
            max_archive_bytes: 1 << 20,
            max_lumps: 4096,
            max_lump_bytes: 1 << 20,
        },
    ) {
        for marker in ["MAP01", "E1M1"] {
            let _ = Map::from_wad(
                &wad,
                marker,
                MapLimits {
                    max_lump_bytes: 1 << 20,
                    max_records: 4096,
                    max_blockmap_cells: 4096,
                    max_blockmap_words: 65536,
                    max_decoded_bytes: 8 << 20,
                },
            );
        }
    }
});
