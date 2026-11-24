//! `DDSP` version 3's reader on states built by hand (M6a reader review, items 3, 5 and 9):
//! Rust-only tests of the format's own rules, needing no WAD. The states a session display
//! really saves are driven in `tests/m6a_display.rs`.
use super::*;
use doom_sim::PickupMessage;

/// An intermission state after its second tic: the frame drawn (no wipe due, no taint), the
/// status bar as a level left it with health 100.
fn intermission() -> SessionDisplayState {
    let mut status = StatusBarState::level_start([false; 9]);
    status.oldhealth = 100;
    SessionDisplayState {
        screenblocks: 10,
        fullscreen: false,
        wipe: false,
        status,
        hud: HudState::level_start(PickupMessage::None),
        old_gamestate: Some(GameState::Intermission),
        tic: 42,
        entered: 40,
        scene: Scene::Intermission(Carried {
            screen: vec![3; PIXELS],
            taint: vec![0; PIXELS],
            fuzzpos: 7,
            wall_steps: WallSteps::default(),
        }),
        melt: None,
    }
}

fn inconsistent(s: &SessionDisplayState) -> Option<&'static str> {
    match s.check() {
        Ok(()) => None,
        Err(SessionRestoreError::Display(DisplayRestoreError::Inconsistent(rule))) => Some(rule),
        Err(e) => panic!("{e:?}"),
    }
}

/// Item 3: a run count from the file is compared with the room left, never added to the length
/// first. The largest count is refused as any other overrun is (on a 32-bit `usize` the old sum
/// wrapped: a panic in a debug build, a wrong length in a release build); runs that fill the
/// screen exactly are read.
#[test]
fn taint_runs_are_bounded_without_overflow() {
    let good = intermission().encode();
    assert_eq!(
        SessionDisplayState::decode(&good).as_ref(),
        Ok(&intermission())
    );
    // magic 4, version 4, tic 8, entered 8, state 1, old state 1, fuzzpos 4, wall steps 16, screen.
    let runs = 26 + 4 + 16 + PIXELS;
    assert_eq!(good[runs..runs + 4], 1_u32.to_le_bytes(), "one run");
    assert_eq!(good[runs + 4..runs + 8], (PIXELS as u32).to_le_bytes());
    for count in [PIXELS as u32 + 1, u32::MAX, u32::MAX - PIXELS as u32 + 1] {
        let mut b = good.clone();
        b[runs + 4..runs + 8].copy_from_slice(&count.to_le_bytes());
        assert_eq!(
            SessionDisplayState::decode(&b),
            Err(StateDecodeError::Invalid("taint runs")),
            "count {count}"
        );
    }
    // Two runs, the second overrunning by the largest count after a first of one pixel.
    let mut b = good[..runs].to_vec();
    b.extend(2_u32.to_le_bytes());
    for (n, v) in [(1_u32, 0_u32), (u32::MAX, 0)] {
        b.extend(n.to_le_bytes());
        b.extend(v.to_le_bytes());
    }
    b.extend(&good[runs + 12..]);
    assert_eq!(
        SessionDisplayState::decode(&b),
        Err(StateDecodeError::Invalid("taint runs"))
    );
    let mut r = Reader {
        b: &[
            2, 0, 0, 0, 1, 0, 0, 0, 9, 0, 0, 0, 0xff, 0xf9, 0, 0, 0, 0, 0, 0,
        ],
        at: 0,
    };
    let taint = r.runs().expect("1 + 63,999 pixels");
    assert_eq!((taint.len(), taint[0], taint[1]), (PIXELS, 9, 0));
}

/// Item 5: the melt tag. No display produces a melt in progress, but the format carries one: the
/// encoder writes tag 1 for a hand-built state, the decoder keeps it, and `check` refuses it.
#[test]
fn the_melt_tag_is_written_for_a_hand_built_melt_and_refused_by_check() {
    let melt_at = 26 + 4 + 16 + PIXELS + 4 + 8;
    let plain = intermission();
    assert_eq!(plain.encode()[melt_at], 0);
    let mut s = intermission();
    s.melt = Some(MeltState {
        step: 42,
        start: vec![1; PIXELS],
        start_taint: vec![0; PIXELS],
    });
    let bytes = s.encode();
    assert_eq!(bytes[melt_at], 1);
    assert_eq!(SessionDisplayState::decode(&bytes).as_ref(), Ok(&s));
    assert_eq!(
        s.check(),
        Err(SessionRestoreError::NotYet(NotYet::MeltPictures))
    );
}

/// Item 9: what `check` (and with it every restore) refuses of a state that decodes. The first
/// three were refused before the review; `st_oldhealth` above 200 outside a level was not.
#[test]
fn check_refuses_states_no_session_display_reaches() {
    assert_eq!(inconsistent(&intermission()), None);
    // A cleared wipe at the entry: the entry always wipes (d_main.c:223).
    let mut s = intermission();
    s.entered = s.tic;
    assert_eq!(inconsistent(&s), Some("wipe"));
    // Carried taint with no wipe due: the frame outside a level wrote every pixel.
    let mut s = intermission();
    if let Scene::Intermission(c) = &mut s.scene {
        c.taint[PIXELS - 1] = 4;
    }
    assert_eq!(inconsistent(&s), Some("carried taint"));
    // `st_oldhealth` outside a level: the finished level's last health.
    for (health, rule) in [
        (0, None),
        (200, None),
        (201, Some("oldhealth")),
        (-1, Some("oldhealth")),
        (i32::MAX, Some("oldhealth")),
    ] {
        let mut s = intermission();
        s.status.oldhealth = health;
        assert_eq!(inconsistent(&s), rule, "oldhealth {health}");
        assert!(
            SessionDisplayState::decode(&s.encode()).is_ok() || health < -1,
            "the decoder alone keeps version 1's range for {health}"
        );
    }
}
