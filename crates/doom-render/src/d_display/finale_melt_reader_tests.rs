//! `DDSP` version 4's reader and rules on states built by hand: Rust-only tests of the format's
//! own rules, needing no WAD. The states a finale-and-melt display really saves, and their
//! restore-and-continue, are driven against the genuine goldens in
//! `crates/doom-game/tests/m6a_finale_melt_display.rs`.
use super::*;
use crate::d_display::DisplayState;
use crate::d_display::session::MeltState;
use crate::f_wipe::melt_offsets;
use crate::hu_stuff::HudState;
use crate::st_stuff::StatusBarState;
use doom_sim::PickupMessage;

/// A scene screen with some structure, so that a melt's pictures differ from step to step.
pub(crate) fn pattern(seed: u32) -> Vec<u8> {
    (0..PIXELS as u32)
        .map(|i| (i.wrapping_mul(seed) >> 7) as u8)
        .collect()
}

/// An intermission state after its second tic (no wipe due, no taint).
fn intermission() -> SessionDisplayStateV4 {
    let mut status = StatusBarState::level_start([false; 9]);
    status.oldhealth = 100;
    status.rndindex = 77;
    SessionDisplayStateV4 {
        mode: GameMode::Registered,
        session: SessionDisplayState {
            screenblocks: 10,
            fullscreen: false,
            wipe: false,
            status,
            hud: HudState::level_start(PickupMessage::None),
            old_gamestate: Some(GameState::Intermission),
            tic: 42,
            entered: 40,
            scene: Scene::Intermission(Carried {
                screen: pattern(2_654_435_761),
                taint: vec![0; PIXELS],
                fuzzpos: 7,
                wall_steps: WallSteps::default(),
            }),
            melt: None,
        },
        melt: None,
    }
}

/// The intermission's first frame, its melt in progress after `step` steps: the display that
/// started it ran one tic after the state was entered, and the index is the one after the wipe's
/// 320 draws.
pub(crate) fn melting(step: u8) -> SessionDisplayStateV4 {
    let mut s = intermission();
    s.session.tic = 41;
    let Scene::Intermission(c) = &s.session.scene else {
        unreachable!()
    };
    let end = c.screen.clone();
    let start = pattern(40_503);
    let (mut m, after) = Melt::new(&start, &end, 77u8.wrapping_sub(64)).expect("melt");
    assert_eq!(after, 77);
    for _ in 0..step {
        assert!(!m.step(), "a step before the last");
    }
    s.melt = Some(MeltProgress {
        step,
        palette: 0,
        columns: *m.columns(),
        start,
        start_taint: vec![0; PIXELS],
        end,
    });
    s
}

fn inconsistent(s: &SessionDisplayStateV4) -> Option<&'static str> {
    match s.check() {
        Ok(()) => None,
        Err(SessionRestoreError::Display(DisplayRestoreError::Inconsistent(rule))) => Some(rule),
        Err(e) => panic!("{e:?}"),
    }
}

/// Offsets in the encoding of [`melting`] (one taint run in each screen).
const MODE: usize = 8;
const SCENE: usize = 27;
pub(crate) const MELT: usize = SCENE + 4 + 16 + PIXELS + 12;
const COLUMNS: usize = MELT + 3;
const START: usize = COLUMNS + 2 * MELT_COLUMNS;
const END: usize = START + PIXELS + 12;

#[test]
fn round_trips_and_offsets() {
    for s in [intermission(), melting(0), melting(9), melting(30)] {
        let b = s.encode();
        assert_eq!(&b[..8], b"DDSP\x04\0\0\0");
        assert_eq!(SessionDisplayStateV4::decode(&b).as_ref(), Ok(&s));
        assert_eq!(s.check(), Ok(()));
    }
    let b = melting(9).encode();
    assert_eq!(b[MODE], 1, "registered");
    assert_eq!(b[MELT..MELT + 3], [1, 9, 0], "tag, steps shown, palette");
    assert_eq!(b[START + PIXELS..START + PIXELS + 4], 1u32.to_le_bytes());
    assert_eq!(
        &b[END..END + PIXELS],
        &melting(9).melt.expect("melt").end[..]
    );
    // Without a melt, version 4 is version 3's bytes after one game-mode byte.
    let s = intermission();
    let (v3, v4) = (s.session.encode(), s.encode());
    assert_eq!(v4[9..], v3[8..]);
    for (mode, code) in [
        (GameMode::Shareware, 0),
        (GameMode::Registered, 1),
        (GameMode::Retail, 2),
    ] {
        let mut s = intermission();
        s.mode = mode;
        let b = s.encode();
        assert_eq!(b[MODE], code);
        assert_eq!(
            SessionDisplayStateV4::decode(&b).expect("decoded").mode,
            mode
        );
    }
}

/// Each version's reader refuses the others: a version-3 encoding given to the version-4
/// reader and the reverse, and versions 1 and 2's reader on version 4.
#[test]
fn versions_refuse_each_other() {
    let s = intermission();
    assert_eq!(
        SessionDisplayStateV4::decode(&s.session.encode()),
        Err(StateDecodeError::Version(3))
    );
    assert_eq!(
        SessionDisplayState::decode(&s.encode()),
        Err(StateDecodeError::Version(4))
    );
    assert_eq!(
        DisplayState::decode_any(&s.encode()),
        Err(StateDecodeError::Version(4))
    );
    assert_eq!(
        DisplayState::decode(&s.encode()),
        Err(StateDecodeError::Version(4))
    );
}

/// Truncation anywhere is refused without a panic (every prefix up to the melt block and around
/// each field of it, and a stride through the screens).
#[test]
fn truncation_is_refused() {
    let b = melting(5).encode();
    let mut cuts: Vec<usize> = (0..SCENE + 64).collect();
    cuts.extend((SCENE..b.len()).step_by(997));
    for at in [MELT, COLUMNS, START, END, b.len()] {
        cuts.extend(at.saturating_sub(16)..at.min(b.len()));
    }
    cuts.extend(b.len() - 100..b.len());
    for n in cuts {
        assert!(
            SessionDisplayStateV4::decode(&b[..n]).is_err(),
            "a prefix of {n} bytes"
        );
    }
    let mut long = b.clone();
    long.push(0);
    assert_eq!(
        SessionDisplayStateV4::decode(&long),
        Err(StateDecodeError::Trailing)
    );
}

/// Values outside what any display writes are refused by the decoder, each named.
#[test]
fn out_of_domain_values_are_refused() {
    let good = melting(5).encode();
    let dec = SessionDisplayStateV4::decode;
    let with = |at: usize, bytes: &[u8]| {
        let mut b = good.clone();
        b[at..at + bytes.len()].copy_from_slice(bytes);
        dec(&b)
    };
    let inv = StateDecodeError::Invalid;
    assert_eq!(with(0, b"X"), Err(StateDecodeError::Magic));
    assert_eq!(with(4, &[5]), Err(StateDecodeError::Version(5)));
    assert_eq!(with(MODE, &[3]), Err(inv("game mode")));
    // A later tic decodes; the rules across fields judge it.
    let later = with(9, &50u64.to_le_bytes()).expect("decoded");
    assert_eq!(inconsistent(&later), Some("melt wipe"));
    assert_eq!(with(17, &99u64.to_le_bytes()), Err(inv("entered")));
    assert_eq!(with(25, &[3]), Err(inv("gamestate")));
    assert_eq!(with(26, &[7]), Err(inv("old gamestate")));
    assert_eq!(with(SCENE, &50u32.to_le_bytes()), Err(inv("fuzzpos")));
    assert_eq!(with(MELT, &[2]), Err(inv("melt")));
    assert_eq!(with(MELT + 1, &[43]), Err(inv("melt step")));
    assert_eq!(with(MELT + 2, &[14]), Err(inv("melt palette")));
    assert_eq!(
        with(COLUMNS, &201i16.to_le_bytes()),
        Err(inv("melt columns"))
    );
    assert_eq!(
        with(COLUMNS + 2 * 159, &(-16i16).to_le_bytes()),
        Err(inv("melt columns"))
    );
    // The start screen's taint runs: a count beyond the room, zero runs, a run of 0.
    let runs = START + PIXELS;
    assert_eq!(with(runs, &0u32.to_le_bytes()), Err(inv("taint runs")));
    assert_eq!(
        with(runs + 4, &(PIXELS as u32 + 1).to_le_bytes()),
        Err(inv("taint runs"))
    );
    assert_eq!(with(runs + 4, &0u32.to_le_bytes()), Err(inv("taint runs")));
    assert_eq!(
        with(runs, &(PIXELS as u32 + 1).to_le_bytes()),
        Err(inv("taint runs"))
    );
}

/// The rules across fields: version 3's, then the melt's, each named.
#[test]
fn states_no_display_produces_are_refused() {
    assert_eq!(inconsistent(&melting(7)), None);
    let mut s = melting(7);
    s.session.melt = Some(MeltState {
        step: 1,
        start: vec![0; PIXELS],
        start_taint: vec![0; PIXELS],
    });
    assert_eq!(inconsistent(&s), Some("version 3 melt"));
    // Version 3's rules apply.
    let mut s = melting(7);
    s.session.hud.message_on = true;
    s.session.hud.message_counter = 0;
    assert_eq!(inconsistent(&s), Some("message_on and message_counter"));
    let mut s = melting(7);
    s.session.entered = 39;
    assert_eq!(inconsistent(&s), Some("melt wipe"), "two tics since entry");
    let mut s = melting(7);
    s.session.wipe = true;
    assert_eq!(inconsistent(&s), Some("melt wipe"), "the frame cleared it");
    let mut s = melting(7);
    s.session.old_gamestate = None;
    assert_eq!(inconsistent(&s), Some("melt old gamestate"));
    let mut s = melting(7);
    s.melt.as_mut().expect("melt").end[100] ^= 1;
    assert_eq!(inconsistent(&s), Some("melt end"));
    let mut s = melting(7);
    s.melt.as_mut().expect("melt").palette = 1;
    assert_eq!(inconsistent(&s), Some("melt palette"));
    let mut s = melting(7);
    s.melt.as_mut().expect("melt").columns[80] += 1;
    assert_eq!(inconsistent(&s), Some("melt columns"));
    let mut s = melting(7);
    s.melt.as_mut().expect("melt").step = 8;
    assert_eq!(
        inconsistent(&s),
        Some("melt columns"),
        "one step more than shown"
    );
    let mut s = melting(7);
    s.melt.as_mut().expect("melt").start_taint.pop();
    assert_eq!(inconsistent(&s), Some("melt screens"));
    let mut s = melting(7);
    s.session.status.rndindex = 78;
    assert_eq!(
        inconsistent(&s),
        Some("melt columns"),
        "another index's melt"
    );
    // A melt whose last step was shown is over: a state with that many steps is refused.
    let index = (0..=255u8)
        .find(|i| melt_offsets(*i).0.iter().all(|y| *y >= -6))
        .expect("an index whose melt has at most 34 steps");
    let mut s = melting(0);
    s.session.status.rndindex = index.wrapping_add(64);
    let m = s.melt.as_mut().expect("melt");
    let (mut melt, _) = Melt::new(&m.start, &m.end, index).expect("melt");
    let mut n = 0u8;
    loop {
        n += 1;
        if melt.step() {
            break;
        }
    }
    assert!(n <= 34);
    m.step = n;
    m.columns = *melt.columns();
    assert_eq!(inconsistent(&s), Some("melt step"));
    m_replay_bound(&s);
}

/// Steps shown beyond [`MELT_MAX_SHOWN`] are refused before any replay.
fn m_replay_bound(s: &SessionDisplayStateV4) {
    let mut s = s.clone();
    s.melt.as_mut().expect("melt").step = MELT_MAX_SHOWN + 1;
    assert_eq!(inconsistent(&s), Some("melt step"));
}

/// The picture taint is composed as the picture is: [`compose`] over the screens equals the
/// melt's own picture after every step.
#[test]
fn compose_follows_the_melt() {
    let (start, end) = (pattern(7), pattern(13));
    let (mut m, _) = Melt::new(&start, &end, 200).expect("melt");
    loop {
        let done = m.step();
        assert_eq!(compose(&start, &end, m.columns()), m.picture());
        if done {
            break;
        }
    }
    assert_eq!(compose(&start, &end, m.columns()), end);
}
