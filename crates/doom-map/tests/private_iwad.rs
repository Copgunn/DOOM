//! Opt-in archive/map validation. No private names, paths, or bytes are logged.

use doom_map::{Map, MapLimits, is_map_marker};
use doom_wad::{Limits, Wad, WadKind};
use std::{env, fs, io::Read};

#[test]
#[ignore = "requires separately authorized assets via DOOM_IWAD_DIR"]
fn private_iwad_maps() {
    let directory = env::var_os("DOOM_IWAD_DIR")
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| panic!("DOOM_IWAD_DIR must be set"));
    let entries = fs::read_dir(directory)
        .unwrap_or_else(|_| panic!("cannot inspect configured asset directory"));
    let limits = Limits::default();
    let mut archives = 0;
    let mut maps = 0;
    for entry in entries {
        let entry = entry.unwrap_or_else(|_| panic!("cannot inspect asset entry"));
        let path = entry.path();
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("wad"))
        {
            continue;
        }
        let file = fs::File::open(path).unwrap_or_else(|_| panic!("cannot open configured asset"));
        let size = file
            .metadata()
            .unwrap_or_else(|_| panic!("cannot inspect asset size"))
            .len();
        assert!(
            size <= limits.max_archive_bytes as u64,
            "asset exceeds archive policy"
        );
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size as usize)
            .unwrap_or_else(|_| panic!("cannot reserve asset input"));
        file.take(limits.max_archive_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .unwrap_or_else(|_| panic!("cannot read configured asset"));
        let wad = Wad::parse(&bytes, limits)
            .unwrap_or_else(|_| panic!("private archive validation failed"));
        assert_eq!(wad.kind(), WadKind::Iwad, "configured asset must be IWAD");
        archives += 1;
        for lump in wad.lumps() {
            if !is_map_marker(lump.name()) {
                continue;
            }
            let name = lump.name();
            let end = name.as_bytes().iter().position(|b| *b == 0).unwrap_or(8);
            let marker = std::str::from_utf8(&name.as_bytes()[..end])
                .unwrap_or_else(|_| panic!("invalid map marker"));
            Map::from_wad(&wad, marker, MapLimits::default())
                .unwrap_or_else(|_| panic!("private map validation failed"));
            maps += 1;
        }
    }
    assert!(
        archives > 0 && maps > 0,
        "configured assets contain no IWAD maps"
    );
}
