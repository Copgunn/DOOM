//! M6a part 2, the part 2 golden set (owner's ruling V9 of 2026-11-23): the whole Episode 1
//! finale crawl. The finale-and-melt display (`SessionConfig::FinaleAndMelt`) and the step-2
//! finale logic against the genuine linuxdoom-1.10 engine's goldens of the two full-crawl items
//! (`tests/reference/m6a2_*`, written by `tools/reference/m6a2_goldens.py`; no Rust output feeds
//! an expectation), driven exactly as `m6a_finale_melt_display.rs` drives the part 1 items.
//!
//! The items `FULLREG` (registered) and `FULLRET` (retail) run the part 1 `FINALE` demo with the
//! part 2 narrow font (`tests/reference/m6a2_wads/m6a2_font.wad`, `tools/reference/m6a2_wad.py`),
//! in which every line of E1TEXT fits in 320 pixels: the crawl shows all 15 lines, each begun by
//! its newline (f_finale.c:306-311), and all 440 characters (356 glyphs, 69 spaces drawn as
//! 4-pixel gaps, f_finale.c:314-318, and 15 newlines); the last glyph appears on finale tic 1,327
//! and the frame holds for 244 tics until stage 1 on finale tic 1,571 (f_finale.c:241) with its
//! melt to `HELP2` or `CREDIT` (f_finale.c:714-720). The part 1 goldens (FINRET, FINREG, NOHELP2)
//! stay compared by the part 1 tests: they cover the crawl's edge break (f_finale.c:321-322).
//!
//! Compared at screenblocks 10 and 11: every frame's digest, palette and palette set, the
//! `M_Random` index after every tic, the finale's stage and count on every finale tic (session F
//! rows), and every melt (start and end screens, palette, every step's picture and `done` flag)
//! from the melt golden of that screen size.
mod m6a;
mod m6a2;

use doom_render::MeltFault;
use m6a2::{
    Counts, DriverFault, Golden, LAST_GLYPH_FINALE_TIC, STAGE1_FINALE_TIC, TextFault, drive,
    full_items, session_tic_of_finale_tic,
};

/// What one full-crawl item compares at each screen size (both items alike): 1,951 session tics
/// (321 level frames of E1M8, then 1,630 finale frames: 1,570 of the text and 60 of the art
/// screen), 1,630 finale stage and count rows, 3 melts (session start, level to finale, the text
/// to the art screen) with their steps.
const TICS: usize = 1951;
const LEVEL_FRAMES: usize = 321;
const FINALE_FRAMES: usize = 1630;

/// Every frame, index, finale state and melt of both full-crawl items at both screen sizes.
#[test]
fn the_full_crawl_items_against_the_genuine_goldens() {
    let mut total = Counts::default();
    let mut golden_melts = (0usize, 0usize);
    for item in full_items() {
        for sb in [10u8, 11] {
            let golden = Golden::full(&item.name, sb);
            let gm = golden.melts.as_ref().expect("melts");
            golden_melts.0 += gm.len();
            golden_melts.1 += gm.values().map(|m| m.steps.len()).sum::<usize>();
            let (n, _) = drive(&item, sb, &golden, None, None).unwrap_or_else(|e| panic!("{e}"));
            eprintln!("{} sb{sb}: {n:?}", item.name);
            assert_eq!(
                (
                    n.tics,
                    n.level_frames,
                    n.intermission_frames,
                    n.finale_frames
                ),
                (TICS, LEVEL_FRAMES, 0, FINALE_FRAMES),
                "{} sb{sb}",
                item.name
            );
            assert_eq!(n.finale_states, FINALE_FRAMES);
            assert_eq!(n.index, n.tics);
            assert_eq!(n.melts, gm.len());
            assert_eq!(n.melts, 3, "session start, level to finale, text to art");
            total.tics += n.tics;
            total.level_frames += n.level_frames;
            total.finale_frames += n.finale_frames;
            total.finale_states += n.finale_states;
            total.index += n.index;
            total.melts += n.melts;
            total.melt_steps += n.melt_steps;
        }
    }
    eprintln!(
        "total {total:?}; melts compared {} of {}, steps {} of {}",
        total.melts, golden_melts.0, total.melt_steps, golden_melts.1
    );
    assert_eq!((total.melts, total.melt_steps), golden_melts);
    assert_eq!(total.tics, 4 * TICS);
}

/// The crawl reaches the end of the text in the genuine frames, read from the goldens alone: on
/// every finale tic of the text stage the frame changes exactly when a glyph is first counted
/// (finale tic 1, then `13 + 3i` for each drawn character `i`, f_finale.c:298), the last change
/// is finale tic 1,327 (the `'!'` of "Inferno!", character 438), and the frame then holds until
/// stage 1 on finale tic 1,571.
#[test]
fn the_genuine_crawl_reaches_the_last_character() {
    let text = doom_render::f_finale::E1TEXT;
    assert_eq!(text.len(), 440);
    let glyph = |c: u8| (b'!'..=b'_').contains(&c.to_ascii_uppercase());
    let want: Vec<i64> = std::iter::once(1)
        .chain(
            text.iter()
                .enumerate()
                .filter(|(_, c)| glyph(**c))
                .map(|(i, _)| 13 + 3 * i as i64),
        )
        .collect();
    assert_eq!(want.len(), 1 + 356);
    assert_eq!(text.iter().filter(|c| **c == b'\n').count(), 15);
    assert_eq!(
        text.iter().filter(|c| **c != b'\n' && !glyph(**c)).count(),
        69
    );
    for item in full_items() {
        for sb in [10u8, 11] {
            let g = Golden::full(&item.name, sb);
            let first = session_tic_of_finale_tic(&g.session, 1);
            let mut changes = Vec::new();
            let mut prev: Option<&str> = None;
            for n in 1..STAGE1_FINALE_TIC {
                let f = &g.frames[&(first + n - 1)];
                assert_eq!(f.gamestate, 2);
                if prev != Some(f.sha256.as_str()) {
                    changes.push(n);
                }
                prev = Some(f.sha256.as_str());
            }
            assert_eq!(changes, want, "{} sb{sb}", item.name);
            assert_eq!(changes.last(), Some(&LAST_GLYPH_FINALE_TIC));
            let art = &g.frames[&(first + STAGE1_FINALE_TIC - 1)];
            assert_ne!(Some(art.sha256.as_str()), prev, "the art screen follows");
            let f = g
                .session
                .at("F", first + STAGE1_FINALE_TIC - 1)
                .expect("F row");
            assert_eq!(f["finalestage"], "1");
        }
    }
}

/// The planted faults of the part 2 set, each named and caught at its tic:
/// a newline treated as a space and the line step off by one (planted in a test-side copy of
/// `F_TextWrite` that first equals the drawer on every one of FULLRET's 1,570 text frames)
/// are caught on finale tic 118, the first glyph of the second line; the crawl one character
/// short on the tic the last character appears is caught on finale tic 1,327 (and only there,
/// so the part 1 item FINRET, whose crawl breaks at the right edge on its second line, cannot
/// see it); a melt column's offset off by one at screenblocks 11 is caught in the first melt.
#[test]
fn the_planted_faults_are_caught() {
    let item = full_items()
        .into_iter()
        .find(|i| i.name == "FULLRET")
        .expect("FULLRET");
    let golden = Golden::full("FULLRET", 10);
    let first = session_tic_of_finale_tic(&golden.session, 1);
    let (_, l) = drive(
        &item,
        10,
        &golden,
        None,
        Some(DriverFault::TextReplicaClean),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(l.replica_equal, 1570, "the copy equals the drawer");
    let line2 = 13 + 3 * 35;
    for (fault, at) in [
        (DriverFault::Text(TextFault::NewlineAsSpace), line2),
        (DriverFault::Text(TextFault::LineStepOffByOne), line2),
        (DriverFault::LastCharacterShort, LAST_GLYPH_FINALE_TIC),
    ] {
        let e = drive(&item, 10, &golden, None, Some(fault))
            .map(|_| ())
            .expect_err("a planted fault must be caught");
        eprintln!("FULLRET {fault:?}: {e}");
        let want = format!("FULLRET sb10 tic {}: frame differs", first + at - 1);
        assert!(e.starts_with(&want), "{fault:?}: {e}");
    }
    // The part 1 item cannot see the short last character: its crawl stopped on line 2.
    let finret = m6a::item("FINRET");
    let (frames, error) = m6a2::read_frames("m6a_frames/FINRET.sb10.frames");
    let p1 = Golden {
        frames,
        error,
        session: m6a::Rows::read("m6a_session/FINRET.session"),
        melts: None,
    };
    drive(
        &finret,
        10,
        &p1,
        None,
        Some(DriverFault::LastCharacterShort),
    )
    .map(|_| ())
    .expect("FINRET's frames do not show the last character");
    // A melt column's offset off by one at screenblocks 11.
    let g11 = Golden::full("FULLRET", 11);
    let e = drive(&item, 11, &g11, Some(MeltFault::ColumnOffByOne), None)
        .map(|_| ())
        .expect_err("caught");
    eprintln!("FULLRET sb11 ColumnOffByOne: {e}");
    assert!(
        e.starts_with("FULLRET sb11 tic 0: melt step"),
        "the first melt: {e}"
    );
}
