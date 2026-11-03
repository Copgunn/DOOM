//! Private tests of the sparse sector overlay and the load-time sector `blockbox`.
use super::*;
use crate::diagonal_room::Lumps;
use crate::multisector::{SectorBlockBox, sector_block_boxes};
use doom_types::Fixed;

mod movers_evidence {
    include!("../../../tests/reference/m3h8_maps.rs");
}

fn state(floor: i16, ceiling: i16) -> SectorState {
    SectorState {
        floor: Fixed::from_map_units(floor),
        ceiling: Fixed::from_map_units(ceiling),
    }
}

#[test]
fn the_overlay_reads_through_replaces_and_commits_only_on_request() {
    let committed = vec![state(0, 128), state(8, 120), state(16, 112)];
    let mut writes = StagedWrites::default();
    let view = writes.sectors(&committed);
    assert_eq!(view.len(), 3);
    assert!((0..3).all(|i| view[i] == committed[i]));
    assert_eq!(view.get(3), None);

    writes.write_sector(&committed, 1, state(4, 120)).unwrap();
    writes.write_sector(&committed, 2, state(16, 64)).unwrap();
    // A second write of one sector replaces its entry instead of adding one.
    writes.write_sector(&committed, 1, state(2, 120)).unwrap();
    let view = writes.sectors(&committed);
    assert_eq!(
        [view[0], view[1], view[2]],
        [committed[0], state(2, 120), state(16, 64)]
    );
    assert_eq!(writes.sectors.len(), 2);
    // Out of range: refused, overlay unchanged.
    let staged = writes.clone();
    assert_eq!(
        writes.write_sector(&committed, 3, state(0, 0)),
        Err(GeometryError::InvalidReference.into())
    );
    assert_eq!(writes, staged);

    // Dropping the overlay leaves the table as it was; committing applies it.
    let mut table = committed.clone();
    let mut lines: Vec<i16> = Vec::new();
    writes.commit(&mut table, &mut lines);
    assert_eq!(table, [committed[0], state(2, 120), state(16, 64)]);
    // Clearing keeps the capacity for the next plan.
    let capacity = writes.sectors.capacity();
    writes.clear();
    assert!(writes.is_empty());
    assert_eq!(writes.sectors.capacity(), capacity);
    assert!(Sectors::committed(&table).get(1) == Some(&state(2, 120)));
}

#[test]
fn line_specials_are_staged_like_sectors() {
    let committed: Vec<i16> = vec![0, 36, 88, 11];
    let mut writes = StagedWrites::default();
    assert_eq!(writes.line_special(&committed, 1), Some(36));
    assert_eq!(writes.line_special(&committed, 4), None);
    writes.write_line_special(&committed, 1, 0).unwrap();
    writes.write_line_special(&committed, 3, 0).unwrap();
    assert_eq!(writes.line_special(&committed, 1), Some(0));
    assert_eq!(writes.line_special(&committed, 2), Some(88));
    assert_eq!(
        writes.write_line_special(&committed, 4, 0),
        Err(GeometryError::InvalidReference.into())
    );
    // Without runtime line specials (every policy before movers) nothing is writable.
    assert!(writes.write_line_special(&[], 0, 0).is_err());
    let mut lines = committed.clone();
    writes.commit(&mut [], &mut lines);
    assert_eq!(lines, [0, 0, 88, 0]);
}

/// An independent transliteration of `P_GroupLines` (p_setup.c 538–575): sector by
/// sector, every line in order whose front or back sector it is, the `M_ClearBox`
/// sentinels and the `M_AddToBox` `else if`, then the block arithmetic in `i64` with
/// each `fixed_t` step required to fit.
fn source_block_boxes(map: &doom_map::Map) -> Vec<Option<SectorBlockBox>> {
    const BOXTOP: usize = 0;
    const BOXBOTTOM: usize = 1;
    const BOXLEFT: usize = 2;
    const BOXRIGHT: usize = 3;
    let fit = |v: i64| i32::try_from(v).ok().map(i64::from);
    let origin = map.blockmap().origin();
    let (orgx, orgy) = (i64::from(origin.x) * 65536, i64::from(origin.y) * 65536);
    let [width, height] = map.blockmap().dimensions().map(i64::from);
    (0..map.sectors().len())
        .map(|sector| {
            let mut bbox = [i32::MIN, i32::MAX, i32::MAX, i32::MIN].map(i64::from);
            for line in map.linedefs() {
                let side = |n: usize| {
                    line.sides[n].map(|s| usize::from(map.sidedefs()[usize::from(s)].sector))
                };
                if side(0) == Some(sector) || side(1) == Some(sector) {
                    for v in line.vertices {
                        let v = map.vertices()[usize::from(v)];
                        let (x, y) = (i64::from(v.x) * 65536, i64::from(v.y) * 65536);
                        if x < bbox[BOXLEFT] {
                            bbox[BOXLEFT] = x;
                        } else if x > bbox[BOXRIGHT] {
                            bbox[BOXRIGHT] = x;
                        }
                        if y < bbox[BOXBOTTOM] {
                            bbox[BOXBOTTOM] = y;
                        } else if y > bbox[BOXTOP] {
                            bbox[BOXTOP] = y;
                        }
                    }
                }
            }
            let block =
                |b: i64, org: i64, r: i64| fit(fit(b - org)? + r).map(|v| v.div_euclid(1 << 23));
            let r = 32 * 65536;
            let top = block(bbox[BOXTOP], orgy, r)?;
            let bottom = block(bbox[BOXBOTTOM], orgy, -r)?;
            let right = block(bbox[BOXRIGHT], orgx, r)?;
            let left = block(bbox[BOXLEFT], orgx, -r)?;
            Some(SectorBlockBox {
                top: (if top >= height { height - 1 } else { top }) as i32,
                bottom: (if bottom < 0 { 0 } else { bottom }) as i32,
                right: (if right >= width { width - 1 } else { right }) as i32,
                left: (if left < 0 { 0 } else { left }) as i32,
            })
        })
        .collect()
}

#[test]
fn sector_block_boxes_follow_p_group_lines() {
    for bytes in movers_evidence::MAP_BYTES {
        let map = Lumps::parse(bytes).map();
        let boxes = sector_block_boxes(&map).unwrap();
        assert_eq!(boxes, source_block_boxes(&map));
        assert_eq!(boxes.len(), map.sectors().len());
        // Every committed room's sectors have lines and boxes on the grid.
        let [width, height] = map.blockmap().dimensions().map(i32::from);
        for b in boxes {
            let b = b.unwrap();
            assert!(0 <= b.left && b.left <= b.right && b.right < width);
            assert!(0 <= b.bottom && b.bottom <= b.top && b.top < height);
        }
    }
}
