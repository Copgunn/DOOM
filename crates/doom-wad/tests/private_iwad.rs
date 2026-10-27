//! Opt-in signature probe for externally supplied assets. Never runs in CI.
//!
//! No asset bytes, filenames, or locations are logged. This is not an archive
//! parser or a compatibility test. Provisioning assets requires separate consent.

use std::{env, fs, io::Read, path::Path};

use doom_wad::WadKind;

#[test]
#[ignore = "requires separately authorized external assets via DOOM_IWAD_DIR"]
fn private_iwad_signatures() {
    let directory = env::var_os("DOOM_IWAD_DIR")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| panic!("DOOM_IWAD_DIR must be set"));
    let entries = fs::read_dir(Path::new(&directory))
        .unwrap_or_else(|_| panic!("cannot inspect configured asset directory"));
    let mut found = false;
    for entry in entries {
        let entry = entry.unwrap_or_else(|_| panic!("cannot inspect asset entry"));
        let path = entry.path();
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("wad"))
        {
            continue;
        }
        let mut file =
            fs::File::open(&path).unwrap_or_else(|_| panic!("cannot open configured asset"));
        let mut magic = [0; 4];
        file.read_exact(&mut magic)
            .unwrap_or_else(|_| panic!("cannot read asset signature"));
        assert!(
            WadKind::from_magic(magic) == Some(WadKind::Iwad),
            "configured WAD does not have an IWAD signature"
        );
        found = true;
    }
    assert!(found, "configured directory contains no WAD files");
}
