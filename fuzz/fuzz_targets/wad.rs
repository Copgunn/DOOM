#![no_main]
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
        for (i, lump) in wad.lumps().iter().enumerate() {
            assert_eq!(wad.data(i).unwrap().len(), lump.range().len());
        }
        let _ = wad.find("TEST");
    }
});
