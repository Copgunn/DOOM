//! Classic map structure, malformed-input, and bounded-work regressions.

#[path = "../../../tests/support/mod.rs"]
mod support;

use doom_map::{ErrorKind as K, Map, MapError, MapLimits, is_map_marker};
use doom_types::LumpName;
use doom_wad::{Limits, Wad, WadSet};
use support::{Lump, classic_map, name, node, set_word, wad, words};

fn parse(lumps: &[Lump]) -> Result<Map, MapError> {
    parse_with(lumps, MapLimits::default())
}
fn parse_with(lumps: &[Lump], limits: MapLimits) -> Result<Map, MapError> {
    let bytes = wad(*b"PWAD", lumps);
    let wad = Wad::parse(&bytes, Limits::default()).unwrap();
    Map::from_wad(&wad, "MAP01", limits)
}

#[test]
fn complete_minimal_map_preserves_numeric_semantics_and_all_tables() {
    let map = parse(&classic_map("MAP01")).unwrap();
    assert_eq!(map.vertices().len(), 4);
    assert_eq!(map.vertices()[0].x, -64);
    assert_eq!(map.things()[0].position.x, -16);
    assert_eq!(map.things()[0].angle, 90);
    assert_eq!(map.things()[0].kind, 1);
    assert_eq!(map.things()[0].options, 7);
    assert_eq!(map.linedefs().len(), 4);
    assert_eq!(map.linedefs()[0].sides, [Some(0), None]);
    assert_eq!(map.sidedefs()[0].offsets, [-8, 16]);
    assert_eq!(map.sidedefs()[0].textures[2].as_bytes(), b"WALLTEST");
    assert_eq!(map.segs()[0].angle, 0x8000);
    assert_eq!(map.segs()[0].offset, -4);
    assert_eq!(map.subsectors()[0].seg_count, 4);
    assert!(map.nodes().is_empty());
    assert_eq!(map.sectors()[0].heights, [-16, 128]);
    assert_eq!(map.sectors()[0].light, 160);
    assert_eq!(map.sectors()[0].tag, 7);
    assert_eq!(map.blockmap().origin().x, -128);
    assert_eq!(map.blockmap().dimensions(), [2, 2]);
    assert_eq!(
        map.blockmap().cell_lines(1, 1).unwrap().collect::<Vec<_>>(),
        [0, 0, 1, 2, 3]
    );
    assert!(map.blockmap().cell_lines(2, 0).is_none());
    assert!(map.blockmap().cell_lines(0, usize::MAX).is_none());
    assert_eq!(map.is_rejected(0, 0), Some(false));
    assert_eq!(map.is_rejected(1, 0), None);
    assert_eq!(parse(&classic_map("MAP01")).unwrap(), map);
}

#[test]
fn marker_families_group_order_and_patch_precedence() {
    for marker in ["MAP01", "map00", "E1M1", "e9m9"] {
        assert!(is_map_marker(LumpName::from_bytes(name(marker))));
        let bytes = wad(*b"IWAD", &classic_map(marker));
        let wad = Wad::parse(&bytes, Limits::default()).unwrap();
        assert!(Map::from_wad(&wad, marker, MapLimits::default()).is_ok());
    }
    for marker in ["MAP1", "MAP001", "E1M", "EAM1", "GL_MAP01"] {
        assert!(!is_map_marker(LumpName::from_bytes(name(marker))));
    }
    let base = wad(*b"IWAD", &classic_map("MAP01"));
    let mut patch = classic_map("MAP01");
    set_word(&mut patch[4].1, 0, i16::MIN);
    let patch = wad(*b"PWAD", &patch);
    let archives = [
        Wad::parse(&base, Limits::default()).unwrap(),
        Wad::parse(&patch, Limits::default()).unwrap(),
    ];
    assert_eq!(
        Map::load(&WadSet::new(&archives), "MAP01", MapLimits::default())
            .unwrap()
            .vertices()[0]
            .x,
        i16::MIN
    );
    let broken = wad(*b"PWAD", &[(name("MAP01"), vec![])]);
    let archives = [
        Wad::parse(&base, Limits::default()).unwrap(),
        Wad::parse(&broken, Limits::default()).unwrap(),
    ];
    assert!(matches!(
        Map::load(&WadSet::new(&archives), "MAP01", MapLimits::default())
            .unwrap_err()
            .kind,
        K::MissingLump("THINGS")
    ));
    let w = &archives[0];
    assert_eq!(
        Map::from_wad(w, "MAP02", MapLimits::default())
            .unwrap_err()
            .kind,
        K::MarkerNotFound
    );
    assert_eq!(
        Map::from_wad(w, "MAP001", MapLimits::default())
            .unwrap_err()
            .kind,
        K::InvalidMarker
    );
    let mut wrong = classic_map("MAP01");
    wrong.swap(3, 4);
    assert_eq!(parse(&wrong).unwrap_err().kind, K::MissingLump("SIDEDEFS"));
}

#[test]
fn rejects_each_incomplete_record_and_empty_essential_table() {
    for i in 1..=8 {
        let mut lumps = classic_map("MAP01");
        lumps[i].1.push(0);
        let error = parse(&lumps).unwrap_err();
        assert!(matches!(error.kind, K::RecordSize { .. }), "{i}: {error}");
    }
    for i in [2, 3, 4, 5, 6, 8] {
        let mut lumps = classic_map("MAP01");
        lumps[i].1.clear();
        assert_eq!(parse(&lumps).unwrap_err().kind, K::EmptyGeometry);
    }
    let mut lumps = classic_map("MAP01");
    lumps[1].1.clear();
    assert!(parse(&lumps).unwrap().things().is_empty());
}

#[test]
fn rejects_invalid_vertex_sector_line_side_references_and_sentinels() {
    for (lump, offset, value) in [
        (2, 0, 4),
        (2, 2, -1),
        (2, 10, 4),
        (2, 12, -2),
        (3, 28, 1),
        (5, 0, 4),
        (5, 2, -1),
        (5, 6, 4),
    ] {
        let mut lumps = classic_map("MAP01");
        set_word(&mut lumps[lump].1, offset, value);
        let error = parse(&lumps).unwrap_err();
        assert!(
            matches!(error.kind, K::InvalidReference { .. }),
            "{lump}/{offset}: {error}"
        );
    }
    for (lump, offset, value) in [(2, 10, -1), (2, 4, 4), (5, 8, 2), (5, 8, 1), (5, 8, -1)] {
        let mut lumps = classic_map("MAP01");
        set_word(&mut lumps[lump].1, offset, value);
        assert!(matches!(
            parse(&lumps).unwrap_err().kind,
            K::InvalidValue { .. }
        ));
    }
    let mut valid = classic_map("MAP01");
    set_word(&mut valid[2].1, 4, 4);
    set_word(&mut valid[2].1, 12, 0);
    assert!(parse(&valid).is_ok());
}

#[test]
fn subsector_ranges_and_front_sector_consistency() {
    for (offset, value) in [(0, 0), (0, -1), (0, 5), (2, 4), (2, -1)] {
        let mut lumps = classic_map("MAP01");
        set_word(&mut lumps[6].1, offset, value);
        assert_eq!(parse(&lumps).unwrap_err().kind, K::InvalidSubsector);
    }
    let mut lumps = classic_map("MAP01");
    let second = lumps[8].1.clone();
    lumps[8].1.extend(second);
    set_word(&mut lumps[3].1, 30 + 28, 1);
    assert_eq!(parse(&lumps).unwrap_err().kind, K::InvalidSubsector);
}

#[test]
fn bsp_references_cycles_and_node_free_policy() {
    for children in [[0x8001, 0x8000], [1, 0x8000], [0xffff, 0x8000]] {
        let mut lumps = classic_map("MAP01");
        lumps[7].1 = node(children);
        assert!(matches!(
            parse(&lumps).unwrap_err().kind,
            K::InvalidReference { .. }
        ));
    }
    for nodes in [
        node([0, 0x8000]),
        [node([1, 0x8000]), node([0, 0x8000])].concat(),
        [node([0, 0x8000]), node([0x8000, 0x8000])].concat(),
    ] {
        let mut lumps = classic_map("MAP01");
        lumps[7].1 = nodes;
        assert_eq!(parse(&lumps).unwrap_err().kind, K::BspCycle);
    }
    let mut lumps = classic_map("MAP01");
    lumps[7].1 = node([0x8000, 0x8000]);
    assert_eq!(parse(&lumps).unwrap().nodes().len(), 1);
    set_word(&mut lumps[7].1, 4, 0);
    assert!(matches!(
        parse(&lumps).unwrap_err().kind,
        K::InvalidValue { .. }
    ));
    let mut lumps = classic_map("MAP01");
    lumps[6].1.extend(words(&[4, 0]));
    assert!(matches!(
        parse(&lumps).unwrap_err().kind,
        K::InvalidValue { .. }
    ));
}

#[test]
fn deep_bsp_uses_bounded_iterative_traversal() {
    let mut lumps = classic_map("MAP01");
    let count = 20_000_u16;
    for i in 0..count {
        let next = if i + 1 == count { 0x8000 } else { i + 1 };
        lumps[7].1.extend(node([next, 0x8000]));
    }
    assert_eq!(parse(&lumps).unwrap().nodes().len(), usize::from(count));
}

#[test]
fn reject_size_bits_and_trailing_data() {
    let mut lumps = classic_map("MAP01");
    lumps[9].1.clear();
    assert_eq!(
        parse(&lumps).unwrap_err().kind,
        K::RejectTooShort {
            expected: 1,
            actual: 0
        }
    );
    lumps[9].1 = vec![1, 255];
    let map = parse(&lumps).unwrap();
    assert_eq!(map.is_rejected(0, 0), Some(true));
    assert_eq!(map.reject(), &[1, 255]);
}

#[test]
fn blockmap_dimensions_offsets_indexes_and_termination() {
    for (offset, value) in [
        (4, 0),
        (6, -1),
        (8, -1),
        (8, 0),
        (8, i16::MAX),
        (16, 1),
        (18, 4),
        (26, 0),
    ] {
        let mut lumps = classic_map("MAP01");
        set_word(&mut lumps[10].1, offset, value);
        assert!(
            matches!(parse(&lumps).unwrap_err().kind, K::InvalidBlockmap(_)),
            "{offset}/{value}"
        );
    }
    for length in [0, 7, 9] {
        let mut lumps = classic_map("MAP01");
        lumps[10].1.truncate(length);
        assert!(matches!(
            parse(&lumps).unwrap_err().kind,
            K::InvalidBlockmap(_)
        ));
    }
    let mut lumps = classic_map("MAP01");
    lumps[10].1 = words(&[0, 0, 3, 3]);
    assert!(matches!(
        parse(&lumps).unwrap_err().kind,
        K::InvalidBlockmap(_)
    ));
}

#[test]
fn shared_long_lists_and_overlapping_subsector_ranges_are_linear_work() {
    let mut lumps = classic_map("MAP01");
    let seg = lumps[5].1[..12].to_vec();
    lumps[5].1 = seg.repeat(32767);
    lumps[6].1 = words(&[32767, 0]).repeat(20_000);
    lumps[7].1 = node([0x8000, 0x8000 | 19999]);
    let mut block = words(&[0, 0, 200, 100]);
    block.extend(words(&[20004]).repeat(20_000));
    block.extend(words(&[0]).repeat(32_000));
    block.extend(words(&[-1]));
    lumps[10].1 = block;
    let map = parse(&lumps).unwrap();
    assert_eq!(map.subsectors().len(), 20_000);
    assert_eq!(map.blockmap().cell_lines(199, 99).unwrap().len(), 32_000);
}

#[test]
fn all_configurable_map_limits_are_enforced() {
    for limits in [
        MapLimits {
            max_lump_bytes: 1,
            ..MapLimits::default()
        },
        MapLimits {
            max_records: 0,
            ..MapLimits::default()
        },
        MapLimits {
            max_blockmap_cells: 1,
            ..MapLimits::default()
        },
        MapLimits {
            max_blockmap_words: 1,
            ..MapLimits::default()
        },
        MapLimits {
            max_decoded_bytes: 1,
            ..MapLimits::default()
        },
    ] {
        assert!(matches!(
            parse_with(&classic_map("MAP01"), limits).unwrap_err().kind,
            K::LimitExceeded(_)
        ));
    }
}

#[test]
fn extended_map_formats_are_explicitly_rejected() {
    let mut lumps = classic_map("MAP01");
    lumps[1].0 = name("TEXTMAP");
    assert_eq!(
        parse(&lumps).unwrap_err().kind,
        K::UnsupportedFormat("UDMF")
    );
    let mut lumps = classic_map("MAP01");
    lumps.push((name("BEHAVIOR"), vec![]));
    assert_eq!(
        parse(&lumps).unwrap_err().kind,
        K::UnsupportedFormat("Hexen")
    );
    for magic in [b"XNOD", b"ZNOD", b"XGLN", b"ZGLN", b"xNd4", b"xNd5"] {
        let mut lumps = classic_map("MAP01");
        lumps[7].1 = magic.to_vec();
        assert_eq!(
            parse(&lumps).unwrap_err().kind,
            K::UnsupportedFormat("extended nodes")
        );
    }
}

#[test]
fn structured_payload_mutations_and_container_truncations_never_panic() {
    let seed = classic_map("MAP01");
    for i in 1..seed.len() {
        for p in 0..seed[i].1.len() {
            for value in [0, 127, 128, 255] {
                let mut lumps = seed.clone();
                lumps[i].1[p] = value;
                let _ = parse(&lumps);
            }
        }
    }
    let seed = wad(*b"PWAD", &seed);
    for end in 0..seed.len() {
        if let Ok(wad) = Wad::parse(&seed[..end], Limits::default()) {
            let _ = Map::from_wad(&wad, "MAP01", MapLimits::default());
        }
    }
}
