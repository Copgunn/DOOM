//! Mutation-survivor tests of M6a part 2 (a child of `f_finale`, so `text_write` is in reach).
use super::tests::res;
use super::*;
use crate::d_display::session::finale_melt::reader_tests::{MELT, melting};
use crate::d_display::session::finale_melt::{
    MELT_MAX_SHOWN, MeltFault, SessionDisplayStateV4, SessionStateRefusal,
};
use crate::f_wipe::{MELT_COLUMNS, Melt};
use crate::{Display, DisplayConfig, Level, LumpDirectory, SessionConfig};
use std::sync::Arc;

/// A finale-and-melt display over the synthetic room and graphics.
fn display() -> Display {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
    let mut dir = LumpDirectory::new();
    for w in [
        "m5a_wads/amp.wad",
        "m5a_gfx.wad",
        "m5b_gfx.wad",
        "m6a_wads/m6a_gfx.wad",
    ] {
        let b = std::fs::read(root.join(w)).expect("WAD");
        dir.add_wad(&doom_wad::Wad::parse(&b, doom_wad::Limits::default()).expect("WAD"))
            .expect("lumps");
    }
    let r = Resources::load(dir, GameMode::Registered).expect("resources");
    let b = std::fs::read(root.join("m5a_wads/amp.wad")).expect("WAD");
    let wad = doom_wad::Wad::parse(&b, doom_wad::Limits::default()).expect("WAD");
    let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).expect("map");
    let level = Arc::new(Level::load(&r, &map, 1).expect("level"));
    let cfg = SessionConfig::FinaleAndMelt {
        mode: GameMode::Registered,
    };
    Display::new_session_with(Arc::new(r), level, DisplayConfig { screenblocks: 10 }, cfg)
        .expect("display")
}

/// Kills finale_melt.rs:341 `melt_taint` -> None / Some(vec![]) / Some(vec![0]) / Some(vec![1]).
/// Rule (its doc comment, f_wipe.c:200-216): in column pair i with position y, rows above y
/// show the end screen, rows from y show the start screen's row r - y (y < 0: the start).
#[test]
fn melt_taint_is_the_taint_of_the_pixel_shown() {
    let mut s = melting(9);
    let m = s.melt.as_mut().expect("melt");
    m.start_taint = (0..64000u32).map(|i| i + 1).collect();
    // The intermission scene (no wipe due) carries no taint, so the end screen's taint is 0.
    let (start_t, cols) = (m.start_taint.clone(), m.columns);
    let mut d = display();
    assert_eq!(d.melt_taint(), None);
    d.restore_session_v4(s, None).expect("restore");
    let got = d.melt_taint().expect("melting");
    let (mut ends, mut starts) = (0, 0);
    for (p, &t) in got.iter().enumerate() {
        let (r, x) = (p / 320, p % 320);
        let y = cols[x / 2];
        let want = if (r as i32) < y {
            ends += 1;
            0
        } else {
            starts += 1;
            start_t[(r - y.max(0) as usize) * 320 + x]
        };
        assert_eq!(t, want, "pixel {p}");
    }
    assert!(ends > 0 && starts > 0, "both screens show");
}

/// Kills finale_melt.rs:410:52 guard `s.fault.is_some()` -> false: without it
/// `session_state_v4` (which refuses a faulted display with None) comes out as NotStarted.
/// Rule: SessionStateRefusal::PlantedFault, "the display holds a planted fault".
#[test]
fn a_faulted_display_is_refused_as_planted_fault() {
    let mut d = display();
    d.plant_melt_fault(MeltFault::WrongPalette);
    assert_eq!(
        d.session_state_v4_checked().unwrap_err(),
        SessionStateRefusal::PlantedFault
    );
}

/// Kills f_finale.rs:130:31 `||` -> `&&`: draw refuses a screen or a taint of the wrong size
/// (f_finale.c draws into screens[0], 320x200) when exactly one of the two is wrong.
#[test]
fn draw_refuses_exactly_one_wrong_slice() {
    let r = res("m6a_wads/m6a_gfx.wad", &[]);
    let font = crate::hu_stuff::load_font(&r).expect("font");
    let v = FinaleView { stage: 1, count: 0 };
    let go = |n: usize, m: usize| {
        let (mut a, mut t) = (vec![0; n], vec![0; m]);
        draw(&r, &font, GameMode::Retail, &v, &mut a, &mut t)
    };
    assert_eq!(go(63_999, 64_000), Err(DrawRefusal::Screen(63_999)));
    assert_eq!(go(64_000, 64_001), Err(DrawRefusal::Screen(64_000)));
}

/// Kills f_finale.rs:225:19 `>` -> `>=`: f_finale.c:321 `if (cx+w > SCREENWIDTH) break;` draws a
/// glyph that ends exactly at x = 320. A line of one glyph repeated, then spaces (4 wide), then
/// a last glyph, is built so that 10 + widths = 320 exactly.
#[test]
fn a_glyph_ending_at_320_is_drawn() {
    let r = res("m6a_wads/m6a_gfx.wad", &[]);
    let font = crate::hu_stuff::load_font(&r).expect("font");
    let w = |c: u8| i32::from(font[usize::from(c - b'!')].header.width);
    let run = |t: &[u8]| {
        let (mut s, mut tn) = (vec![7u8; 64000], vec![9u32; 64000]);
        text_write(&r, &font, t, 1000, &mut s, &mut tn).expect("drawn");
        s
    };
    let found = (b'!'..=b'_').find_map(|c| {
        (b'!'..=b'_').find_map(|last| {
            (0..=3i32).find_map(|gaps| {
                let rest = 310 - 4 * gaps - w(last);
                (rest >= 0 && rest % w(c) == 0).then(|| (c, last, gaps, rest / w(c)))
            })
        })
    });
    let (c, last, gaps, n) = found.expect("an exact fit");
    let mut t = vec![c; n as usize];
    t.extend(vec![b' '; gaps as usize]);
    let before = run(&t);
    t.push(last);
    assert_ne!(run(&t), before, "a glyph ending at x = 320 is drawn");
}

/// Kills f_wipe.rs:86:9 and finale_melt.rs:115:9 (Display impls -> Ok(Default)): the formatted
/// text of the type's own code, the variant's name after the prefix.
#[test]
fn the_refusals_have_their_text() {
    let mut o = [0; MELT_COLUMNS];
    o[3] = 1;
    let e = Melt::from_offsets(&[0; 64000], &[0; 64000], o).unwrap_err();
    assert_eq!(e.to_string(), format!("melt refused: {e:?}"));
    assert!(e.to_string().ends_with("{ column: 3, value: 1 }"));
    let t = SessionStateRefusal::PlantedFault.to_string();
    assert_eq!(t, "session display state refused: PlantedFault");
}

/// Kills finale_melt.rs:701:28 `>` -> `>=` in `SessionDisplayStateV4::decode`: the steps-shown
/// byte is in 0..=MELT_MAX_SHOWN (the `MeltProgress::step` doc), so the maximum decodes and one
/// more is "melt step".
#[test]
fn decode_takes_the_largest_step_and_refuses_the_next() {
    let mut b = melting(0).encode();
    assert_eq!(b[MELT], 1, "melt tag");
    b[MELT + 1] = MELT_MAX_SHOWN;
    assert!(SessionDisplayStateV4::decode(&b).is_ok());
    b[MELT + 1] = MELT_MAX_SHOWN + 1;
    assert_eq!(
        SessionDisplayStateV4::decode(&b),
        Err(crate::StateDecodeError::Invalid("melt step"))
    );
}
