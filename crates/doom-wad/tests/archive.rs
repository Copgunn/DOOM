//! Bounded archive and ordered lookup regression suite using synthetic data.

#[path = "../../../tests/support/mod.rs"]
mod support;

use doom_wad::{Limits, LumpLocation, Wad, WadError, WadKind, WadSet};
use support::{name, set_i32, wad};

#[test]
fn empty_archives_and_exact_names() {
    for (magic, kind) in [(*b"IWAD", WadKind::Iwad), (*b"PWAD", WadKind::Pwad)] {
        let bytes = wad(magic, &[]);
        let w = Wad::parse(&bytes, Limits::default()).unwrap();
        assert_eq!(w.kind(), kind);
        assert!(w.lumps().is_empty());
        assert_eq!(w.data(usize::MAX), None);
    }
    let raw = [255, 128, 0, 7, 4, 3, 2, 1];
    let bytes = wad(*b"PWAD", &[(raw, vec![1, 2])]);
    let w = Wad::parse(&bytes, Limits::default()).unwrap();
    assert_eq!(w.lumps()[0].name().as_bytes(), &raw);
    assert_eq!(w.data(0), Some(&[1, 2][..]));
}

#[test]
fn lookup_preserves_order_and_last_match_across_archives() {
    let base = wad(
        *b"IWAD",
        &[
            (name("TEST"), vec![1]),
            (name("TeSt"), vec![2]),
            (name("EIGHTCHR"), vec![3]),
            (*b"A\0junk!!", vec![4]),
        ],
    );
    let patch = wad(*b"PWAD", &[(name("test"), vec![5])]);
    let archives = [
        Wad::parse(&base, Limits::default()).unwrap(),
        Wad::parse(&patch, Limits::default()).unwrap(),
    ];
    assert_eq!(archives[0].find("test"), Some(1));
    assert_eq!(archives[0].find("eightchrLONG"), Some(2));
    assert_eq!(archives[0].find("a"), Some(3));
    assert_eq!(archives[0].lumps()[0].name().as_bytes(), &name("TEST"));
    let set = WadSet::new(&archives);
    assert_eq!(set.data(set.find("tEsT").unwrap()), Some(&[5][..]));
    assert_eq!(set.find("missing"), None);
    assert_eq!(
        set.data(LumpLocation {
            archive: 2,
            lump: 0
        }),
        None
    );
    assert_eq!(
        set.data(LumpLocation {
            archive: 0,
            lump: usize::MAX
        }),
        None
    );
}

#[test]
fn zero_markers_shared_overlapping_and_directory_ranges_are_safe() {
    let mut b = wad(
        *b"PWAD",
        &[
            (name("A"), vec![1, 2, 3]),
            (name("B"), vec![4, 5]),
            (name("MARK"), vec![]),
        ],
    );
    let dir = i32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
    set_i32(&mut b, dir + 16, 13);
    set_i32(&mut b, dir + 32, 0);
    let w = Wad::parse(&b, Limits::default()).unwrap();
    assert_eq!(w.data(0), Some(&[1, 2, 3][..]));
    assert_eq!(w.data(1), Some(&[2, 3][..]));
    assert_eq!(w.data(2), Some(&[][..]));
    set_i32(&mut b, dir + 16, dir as i32);
    assert!(Wad::parse(&b, Limits::default()).is_ok());
    let end = b.len() as i32;
    set_i32(&mut b, dir + 32, end);
    assert!(Wad::parse(&b, Limits::default()).is_ok());
    set_i32(&mut b, dir + 32, end + 1);
    assert!(matches!(
        Wad::parse(&b, Limits::default()),
        Err(WadError::OutOfBounds { entry: Some(2), .. })
    ));
}

#[test]
fn malformed_headers_directories_and_signed_fields() {
    for size in 0..12 {
        assert_eq!(
            Wad::parse(&vec![0; size], Limits::default()).unwrap_err(),
            WadError::TruncatedHeader
        );
    }
    let valid = wad(*b"PWAD", &[(name("A"), vec![0])]);
    let dir = 13;
    for (offset, field, entry) in [
        (4, "lump count", None),
        (8, "directory offset", None),
        (dir, "lump offset", Some(0)),
        (dir + 4, "lump size", Some(0)),
    ] {
        let mut b = valid.clone();
        set_i32(&mut b, offset, -1);
        assert_eq!(
            Wad::parse(&b, Limits::default()).unwrap_err(),
            WadError::NegativeField {
                entry,
                field,
                value: -1
            }
        );
    }
    let mut b = valid.clone();
    b[0] = 0;
    assert_eq!(
        Wad::parse(&b, Limits::default()).unwrap_err(),
        WadError::InvalidMagic
    );
    for offset in [8, dir, dir + 4] {
        let mut b = valid.clone();
        set_i32(&mut b, offset, i32::MAX);
        assert!(
            Wad::parse(
                &b,
                Limits {
                    max_lump_bytes: usize::MAX,
                    ..Limits::default()
                }
            )
            .is_err()
        );
    }
    assert!(matches!(
        Wad::parse(&valid[..valid.len() - 1], Limits::default()),
        Err(WadError::OutOfBounds { entry: None, .. })
    ));
}

#[test]
fn resource_limits_are_applied_before_allocation() {
    let b = wad(*b"IWAD", &[(name("A"), vec![1, 2])]);
    for limits in [
        Limits {
            max_archive_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_lumps: 0,
            ..Limits::default()
        },
        Limits {
            max_lump_bytes: 1,
            ..Limits::default()
        },
    ] {
        assert!(matches!(
            Wad::parse(&b, limits),
            Err(WadError::LimitExceeded { .. })
        ));
    }
    let mut b = wad(*b"PWAD", &[]);
    set_i32(&mut b, 4, i32::MAX);
    assert!(matches!(
        Wad::parse(&b, Limits::default()),
        Err(WadError::LimitExceeded { .. })
    ));
    assert!(
        Wad::parse(
            &b,
            Limits {
                max_lumps: usize::MAX,
                ..Limits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn adversarial_mutations_and_truncations_never_panic() {
    let seed = wad(
        *b"PWAD",
        &[(name("TEST"), vec![1, 2, 3]), (name("END"), vec![])],
    );
    let check = |bytes: &[u8]| {
        if let Ok(w) = Wad::parse(
            bytes,
            Limits {
                max_lumps: 128,
                ..Limits::default()
            },
        ) {
            for (i, lump) in w.lumps().iter().enumerate() {
                assert_eq!(w.data(i).unwrap().len(), lump.range().len());
            }
        }
    };
    for end in 0..=seed.len() {
        check(&seed[..end]);
    }
    for i in 0..seed.len() {
        for value in [0, 127, 128, 255] {
            let mut b = seed.clone();
            b[i] = value;
            check(&b);
        }
    }
}
