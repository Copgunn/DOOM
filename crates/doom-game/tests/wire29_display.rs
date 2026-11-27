//! The pause display's own methods (M6b step 9, mutation survivors in `doom-render`): reached
//! through a wire 29 session's display, because a pause display needs a session's lumps. Rules
//! from `d_display/pause.rs`'s module documentation (cited there against d_main.c:303-311 and
//! g_game.c:704-709): `paused` follows the tic's toggle; a version 5 state is the pause display's
//! only (version 4 the other finale-and-melt displays'); a level scene shown paused after a tic
//! has at least one paused tic; no paused tic outside a level; a new game clears `paused`. The
//! genuine `paused` column of the committed session streams supplies the pause stretches.
mod m6a;

use doom_game::session::demo::SessionDemo;
use doom_game::session::{Session, SessionSettings};
use doom_render::{Frame, PauseFields, SessionDisplayStateV5, SessionStateRefusal};
use doom_sim::TicCommand;
use m6a::{Rows, int};

const SHARED: [&str; 5] = [
    "tests/reference/m5b_gfx.wad",
    "tests/reference/m5c_sfx.wad",
    "tests/reference/m6a_wads/m6a_gfx.wad",
    "tests/reference/m6a_wads/m6a_music.wad",
    "tests/reference/m6b_wads/m6b_gfx.wad",
];

/// A wire 29 session of the item at the start, its commands and the genuine `paused` column.
fn start(name: &str) -> (Session, Vec<TicCommand>, Vec<bool>) {
    let episode = name == "PAUSEWI";
    let lmp = format!("tests/reference/m6b_scripts/{name}.lmp");
    let (demo, parameters) =
        SessionDemo::parse_v4(&std::fs::read(m6a::root(&lmp)).unwrap()).expect("reads");
    let first = if episode {
        "tests/reference/m6a_wads/episode.wad"
    } else {
        "tests/reference/m6b_wads/m6b.wad"
    };
    let item = m6a::Item {
        name: name.into(),
        mode: "registered".into(),
        wads: std::iter::once(first)
            .chain(SHARED)
            .map(String::from)
            .collect(),
        lmp,
        map: demo.map,
        skill: demo.skill,
        end: "marker".into(),
        session_tics: demo.commands.len(),
    };
    let settings = SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: 10,
    };
    let s = Session::new_finale_and_melt_wire29(m6a::data(&item, None), settings, parameters)
        .expect("starts");
    let g = Rows::read(&format!("m6b_session/{name}.session"));
    let paused = g.of("G").iter().map(|r| int(r, "paused") == 1).collect();
    (s, demo.commands, paused)
}

/// Runs `tics` tics (melts stepped).
fn run(s: &mut Session, cmds: &[TicCommand], tics: usize) {
    let mut unseen = Frame::new();
    for &c in &cmds[..tics] {
        s.tic_finale_and_melt(c).expect("a tic");
        while s.melt_in_progress() {
            s.melt_step(&mut unseen).expect("a melt step");
        }
    }
}

/// `pause_fields` follows the genuine `paused` column and the paused tics so far; the state kinds
/// are each display's own.
#[test]
fn the_pause_display_reports_its_fields_and_saves_as_version_5_only() {
    let (mut s, cmds, paused) = start("PAUSE");
    let mut unseen = Frame::new();
    for (t, &c) in cmds.iter().enumerate().take(130) {
        s.tic_finale_and_melt(c).expect("a tic");
        while s.melt_in_progress() {
            s.melt_step(&mut unseen).expect("a melt step");
        }
        let d = s.display();
        let want = PauseFields {
            paused: paused[t],
            level_paused: paused[..=t].iter().filter(|&&p| p).count() as u64,
        };
        assert_eq!(d.pause_fields(), Some(want), "tic {t}");
        let v5 = d
            .session_state_v5()
            .expect("a pause display saves as version 5");
        assert_eq!(v5.pause, want, "tic {t}");
        assert_eq!(v5.pause, s.save_v4().unwrap().v4.display, "tic {t}");
    }
    let d = s.display();
    assert!(d.session_state_v4().is_none());
    assert_eq!(
        d.session_state_v4_checked().err(),
        Some(SessionStateRefusal::PauseSavesVersion5)
    );
    // The wire 28 session's display is a finale-and-melt display: version 4, never 5.
    let lmp = "tests/reference/m6b_scripts/BITS3.lmp";
    let (demo, _) = SessionDemo::parse_v4(&std::fs::read(m6a::root(lmp)).unwrap()).unwrap();
    let item = m6a::Item {
        name: "BITS3".into(),
        mode: "registered".into(),
        wads: std::iter::once("tests/reference/m6b_wads/m6b.wad")
            .chain(SHARED)
            .map(String::from)
            .collect(),
        lmp: lmp.into(),
        map: demo.map,
        skill: demo.skill,
        end: "marker".into(),
        session_tics: demo.commands.len(),
    };
    let settings = SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: 10,
    };
    let mut w28 = Session::new_finale_and_melt_wire28(m6a::data(&item, None), settings).unwrap();
    run(&mut w28, &demo.commands, 3);
    let d = w28.display();
    assert_eq!(d.pause_fields(), None);
    assert!(d.session_state_v5().is_none());
    assert_eq!(
        d.session_state_v5_checked().err(),
        Some(SessionStateRefusal::NotPauseDisplay)
    );
    assert!(d.session_state_v4().is_some());
    assert!(d.session_state_v4_checked().is_ok());
}

/// `observe_paused` is `observe_paused_with` with both tickers on (the pause's `ST_Ticker` and
/// `HU_Ticker`, g_game.c:729-734) and differs from one with them off; a new game
/// (`begin_level`) clears `paused`; the plant-fault picture is one row lower.
#[test]
fn a_paused_tic_observed_directly_a_new_game_and_the_lowered_picture() {
    let (mut s, cmds, paused) = start("PAUSE");
    run(&mut s, &cmds, 80);
    assert!(paused[79], "tic 79 is inside the genuine pause");
    let (sim, cmd) = (s.level(), &cmds[79]);
    let (mut a, mut b, mut off) = (
        s.display().clone(),
        s.display().clone(),
        s.display().clone(),
    );
    let ra = a.observe_paused(sim, cmd, 0);
    let rb = b.observe_paused_with(sim, cmd, 0, [true, true]);
    assert_eq!(ra, rb);
    assert_eq!(a.session_state_v5(), b.session_state_v5());
    off.observe_paused_with(sim, cmd, 0, [false, false])
        .unwrap();
    assert_ne!(
        off.session_state_v5(),
        b.session_state_v5(),
        "ST_Ticker and HU_Ticker change the display's state"
    );
    // The picture's place: one row lower when planted.
    let (x, y) = s.display().pause_origin().expect("a pause display");
    let mut low = s.display().clone();
    low.plant_pause_fault();
    assert_eq!(low.pause_origin(), Some((x, y + 1)));
    // A new game on a display at tic 0 clears `paused`.
    let (fresh, _, _) = start("PAUSE");
    let mut d = fresh.display().clone();
    d.set_paused(true).expect("M_PAUSE is in the WAD");
    assert_eq!(d.pause_fields().unwrap().paused, true);
    d.begin_level(fresh.level()).expect("a new game at tic 0");
    assert_eq!(d.pause_fields().unwrap().paused, false);
}

/// `SessionDisplayStateV5::check` (the rules above): `paused` after a level tic with no paused
/// tic is refused, and so is a paused tic outside a level (PAUSEWI is in the intermission at
/// tic 137, the genuine `paused` column holds there).
#[test]
fn a_version_5_display_state_is_checked_for_paused_tics() {
    let state = |name: &str, tic: usize| {
        let (mut s, cmds, paused) = start(name);
        run(&mut s, &cmds, tic);
        let v4 = s.save_v4().unwrap();
        (
            SessionDisplayStateV5 {
                display: v4.base.display.clone(),
                pause: v4.v4.display,
            },
            paused[tic - 1],
        )
    };
    let (ok, _) = state("PAUSE", 60);
    assert_eq!(ok.pause, PauseFields::default());
    assert!(ok.check().is_ok());
    let mut bad = ok.clone();
    bad.pause.paused = true;
    assert!(bad.check().is_err(), "paused with no paused tic");
    let (inter, was_paused) = state("PAUSEWI", 138);
    assert!(was_paused && inter.check().is_ok());
    assert_eq!(inter.pause.level_paused, 0);
    let mut bad = inter.clone();
    bad.pause.level_paused = 1;
    assert!(bad.check().is_err(), "a paused tic outside a level");
}
