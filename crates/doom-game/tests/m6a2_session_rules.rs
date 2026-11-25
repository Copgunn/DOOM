//! M6a part 2 steps 3 and 4, the session: `DSES` version 2 restore-and-continue, the refusals of
//! the finale-and-melt session with the state they preserve, and the version 2 reader.
//!
//! Restore-and-continue is Rust-only consistency (the state is the session's own, and the run it
//! continues is the uninterrupted one, which `tests/m6a2_session.rs` compares with the genuine
//! goldens). The refusals are the genuine engine's stops at the genuine tics (`NOHELP2`,
//! `NOVICTOR`, read from `tests/reference/m6a_results.json`) and the session's own typed refusals.
mod m6a;

use doom_game::session::finale_melt::{Configuration, FinaleMeltTicKind};
use doom_game::session::sound::SoundWitness;
use doom_game::session::state::{DsesError, SessionState};
use doom_game::session::state_v2::SessionStateV2;
use doom_game::session::{
    NotYet, PlantedFault, Session, SessionData, SessionMode, SessionSettings, SessionWitness,
    StartError,
};
use doom_render::{FinaleView, Frame, GameState, SessionError, SessionStateRefusal};
use doom_sim::TicCommand;
use doom_sound::chain::ChainRefusal;
use doom_sound::chain_melt::ChainSaveRefusal;
use doom_sound::music::Music;
use doom_sound::schedule::Schedule;
use doom_test_evidence::sha256;
use std::collections::BTreeMap;
use std::sync::Arc;

/// A finale-and-melt session driven from an item's demo, one call at a time.
#[derive(Clone)]
struct Drive {
    name: String,
    data: Arc<SessionData>,
    s: Session,
    cmds: Arc<Vec<TicCommand>>,
    next: usize,
}

/// What one call did: a tic (its kind, whether it melts) or a melt step; the digest of what it
/// showed (the frame, or the melt picture) and of what it handed out (the buffer, if any).
#[derive(Clone, Debug, PartialEq)]
enum Call {
    Tic {
        kind: FinaleMeltTicKind,
        before: GameState,
        wiped: bool,
        shown: String,
        buffer: Option<String>,
    },
    Step {
        step: u8,
        done: bool,
        shown: String,
        buffer: Option<String>,
    },
}

fn buffer_digest(b: &doom_sound::mixer::Buffer) -> String {
    sha256(&doom_sound::mixer::buffer_bytes(b))
}

impl Drive {
    fn new(name: &str) -> Self {
        let item = m6a::item(name);
        let lmp = std::fs::read(m6a::root(&item.lmp)).expect("demo");
        let demo = doom_game::session::demo::SessionDemo::parse(&lmp).expect("demo");
        let data = m6a::data(&item, None);
        let settings = SessionSettings {
            map: item.map,
            skill: item.skill,
            screenblocks: 10,
        };
        let s = Session::new_finale_and_melt(data.clone(), settings).expect("session");
        Self {
            name: name.into(),
            data,
            s,
            cmds: Arc::new(demo.commands),
            next: 0,
        }
    }

    /// The next call: a melt step when a melt is in progress, else the next tic (None at the
    /// demo's end).
    fn call(&mut self) -> Option<Call> {
        if self.s.melt_in_progress() {
            let mut f = Frame::new();
            let m = self.s.melt_step(&mut f).expect("melt step");
            return Some(Call::Step {
                step: m.step,
                done: m.done,
                shown: sha256(f.pixels()),
                buffer: m.buffer.as_ref().map(buffer_digest),
            });
        }
        let cmd = *self.cmds.get(self.next)?;
        let before = self.s.gamestate();
        let r = self
            .s
            .tic_finale_and_melt(cmd)
            .unwrap_or_else(|e| panic!("{} call {}: {e}", self.name, self.next));
        self.next += 1;
        Some(Call::Tic {
            kind: r.kind,
            before,
            wiped: r.wiped,
            shown: sha256(self.s.frame().pixels()),
            buffer: r.sound.sound.buffer.as_ref().map(buffer_digest),
        })
    }

    /// Runs whole tics (each with its melt) until session tic `t` is next.
    fn run_to_tic(&mut self, t: u64) {
        while self.s.gametic() < t || self.s.melt_in_progress() {
            self.call().expect("a call");
        }
    }

    fn bytes(&self) -> Vec<u8> {
        self.s.save_v2().expect("saveable").encode()
    }

    /// A drive restored from this one's saved bytes (decoded, byte-equal round trip, restored
    /// against a copy of the level's simulation, and saving the same bytes again).
    fn restored(&self) -> Self {
        let bytes = self.bytes();
        let state = SessionStateV2::decode(&bytes).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(state.encode(), bytes, "{}: round trip", self.name);
        let s = Session::restore_v2(self.data.clone(), &state, self.s.level().clone())
            .unwrap_or_else(|e| panic!("{} at {}: restore {e}", self.name, self.s.gametic()));
        let r = Self { s, ..self.clone() };
        assert_eq!(
            r.bytes(),
            bytes,
            "{}: the restored session saves the same",
            self.name
        );
        r
    }
}

/// The kind of melt a tic's display starts (M6a note §1.11), from what the tic was.
fn melt_kind(call: &Call) -> Option<&'static str> {
    let Call::Tic {
        kind,
        before,
        wiped: true,
        ..
    } = call
    else {
        return None;
    };
    Some(match (kind, before) {
        (FinaleMeltTicKind::Level, _) => "session start",
        (FinaleMeltTicKind::Completion, _) => "level to intermission",
        (FinaleMeltTicKind::Load { .. }, GameState::Intermission) => "intermission to level",
        (FinaleMeltTicKind::Load { .. }, _) => "reborn reload",
        (FinaleMeltTicKind::Victory, _) => "level to finale",
        (FinaleMeltTicKind::Finale, _) => "finale text to art",
        _ => panic!("a melt after {kind:?}"),
    })
}

/// Continues `a` (uninterrupted) and `b` (restored from `a`) side by side until the melt in
/// progress ends and two more tics with their melts have run (or the demo ends): every call's
/// picture and buffer equal, and the saved bytes equal at the end.
fn continue_equal(mut a: Drive, mut b: Drive, what: &str) {
    let mut tics_after = 0;
    loop {
        if tics_after == 2 && !a.s.melt_in_progress() {
            break;
        }
        let (x, y) = (a.call(), b.call());
        assert_eq!(x, y, "{what}: a call after the restore");
        match x {
            None => break,
            Some(Call::Tic { .. }) => tics_after += 1,
            Some(Call::Step { .. }) => {}
        }
    }
    assert_eq!(a.bytes(), b.bytes(), "{what}: the state after continuing");
}

/// `DSES` version 2 saved and restored at every step of 17 melts of all six kinds (session start,
/// level to intermission, intermission to level, the reborn reload, level to finale, the finale's
/// text to art: at least two of each), before each of those melts' tics (a load tic, the reborn,
/// the completion, the victory and the stage-1 tic among them), at intermission tics, and in both
/// finale stages; each restored session continues exactly as the uninterrupted one.
#[test]
fn dses_v2_restore_and_continue() {
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    let (mut points, mut melts) = (0usize, 0usize);
    for (name, take) in [
        ("FINRET", 3),
        ("FINREG", 3),
        ("CHAIN", 5),
        ("REBHELD", 3),
        ("REBLATE", 3),
    ] {
        let mut d = Drive::new(name);
        let mut seen = 0;
        while seen < take {
            // Before the tic.
            let before_tic = d.clone();
            let call = d.call().expect("a call");
            let Some(kind) = melt_kind(&call) else {
                continue;
            };
            seen += 1;
            melts += 1;
            *kinds.entry(kind).or_insert(0) += 1;
            let what = format!("{name} {kind} at tic {}", before_tic.s.gametic());
            continue_equal(before_tic.clone(), before_tic.restored(), &what);
            points += 1;
            // Every step of the melt: after the tic, then after each step but the last.
            let mut k = 0;
            while d.s.melt_in_progress() {
                let shown = d.s.display().melt_steps_shown().expect("a melt");
                assert_eq!(usize::from(shown), k);
                continue_equal(d.clone(), d.restored(), &format!("{what} step {k}"));
                points += 1;
                d.call().expect("a step");
                k += 1;
            }
        }
    }
    // Intermission tics and both finale stages, outside a melt.
    for (name, tics) in [
        ("CHAIN", vec![130u64, 142, 270]),
        ("INTSTA", vec![280, 600]),
        ("FINRET", vec![400, 1000, 1890, 1895, 1940]),
    ] {
        let mut d = Drive::new(name);
        for t in tics {
            d.run_to_tic(t);
            let gs = d.s.gamestate();
            assert!(
                gs != GameState::Level,
                "{name} tic {t}: outside a level ({gs:?})"
            );
            continue_equal(d.clone(), d.restored(), &format!("{name} tic {t}"));
            points += 1;
        }
    }
    eprintln!("DSES v2 restore points: {points}, every step of {melts} melts; kinds {kinds:?}");
    assert_eq!(melts, 17);
    assert_eq!(kinds.len(), 6, "every kind of melt");
    assert!(kinds.values().all(|n| *n >= 2), "{kinds:?}");
}

/// The genuine stops, each a typed refusal with the state it must keep: `NOVICTOR`'s missing
/// `D_VICTOR` on the victory tic (session tic 321: nothing changes, the same refusal again) and
/// `NOHELP2`'s missing `HELP2` at the first stage-1 frame (session tic 1,891, finale tic 1,571:
/// rule 2, so `F_Ticker`'s tic stays committed, stage 1 count 0, while the frame, the display and
/// the sound stay at the last good tic and the session takes no further call and cannot be
/// saved).
#[test]
fn the_genuine_stops_keep_their_state() {
    let novictor = m6a::item("NOVICTOR");
    assert_eq!(novictor.session_tics, 321);
    let mut d = Drive::new("NOVICTOR");
    d.run_to_tic(321);
    let before = d.bytes();
    for _ in 0..2 {
        let e = d.s.tic_finale_and_melt(d.cmds[321]).expect_err("refused");
        assert_eq!(e.tic, 321);
        assert_eq!(
            e.witness,
            SessionWitness::Sound(SoundWitness::Chain(ChainRefusal::MusicMissing {
                music: Music::Victor,
                tic: Some(321)
            }))
        );
        assert_eq!(d.bytes(), before, "NOVICTOR: unchanged");
    }

    let nohelp2 = m6a::item("NOHELP2");
    assert_eq!(nohelp2.session_tics, 1891);
    let mut d = Drive::new("NOHELP2");
    d.run_to_tic(1891);
    assert_eq!(
        d.s.finale(),
        Some(FinaleView {
            stage: 0,
            count: 1570
        })
    );
    let frame = d.s.frame().pixels().to_vec();
    let display = d.s.display().session_state_v4().expect("state").encode();
    let sound = d.s.sound().clone();
    let e = d.s.tic_finale_and_melt(d.cmds[1891]).expect_err("refused");
    assert_eq!(e.tic, 1891);
    match &e.witness {
        SessionWitness::Display(SessionError::Finale(r)) => {
            assert_eq!(r.to_string(), "W_GetNumForName: HELP2 not found!")
        }
        w => panic!("{w:?}"),
    }
    assert_eq!(
        d.s.finale(),
        Some(FinaleView { stage: 1, count: 0 }),
        "committed"
    );
    assert_eq!(d.s.frame().pixels(), &frame[..], "the last good frame");
    assert_eq!(
        d.s.display().session_state_v4().expect("state").encode(),
        display,
        "the display at the last good tic"
    );
    assert_eq!(d.s.sound(), &sound, "the sound at the last good tic");
    assert_eq!(d.s.gametic(), 1891);
    assert_eq!(d.s.ended(), Some(&e));
    assert!(!d.s.melt_in_progress());
    let again = d.s.tic_finale_and_melt(TicCommand::default());
    assert_eq!(
        again.map(|_| ()).expect_err("ended").witness,
        SessionWitness::Ended
    );
    let step = d.s.melt_step(&mut Frame::new());
    assert_eq!(step.expect_err("ended").witness, SessionWitness::Ended);
    assert!(matches!(d.s.save_v2(), Err(DsesError::NotSaveable)));
    // Retail on the same WADs draws CREDIT there (FINRET runs on through tic 1,950).
    let mut d = Drive::new("FINRET");
    d.run_to_tic(1891);
    d.call().expect("tic 1891");
    assert_eq!(d.s.finale(), Some(FinaleView { stage: 1, count: 0 }));
    assert!(d.s.melt_in_progress(), "the stage-1 wipe melts");
}

/// The session's own typed refusals, each changing nothing: a tic during a melt, a melt step with
/// no melt, each old-version save from the new session (`DSES` version 1, `DDSP` version 3,
/// `DSND` version 2) and each new-version call on the frozen one, and skills 0 and 1 (refused as
/// the frozen session refuses them).
#[test]
fn the_session_refusals_change_nothing() {
    let mut d = Drive::new("CHAIN");
    d.call().expect("tic 0, which melts");
    assert!(d.s.melt_in_progress());
    for steps in [0, 7] {
        while d.s.display().melt_steps_shown() != Some(steps) {
            d.call().expect("step");
        }
        let before = d.bytes();
        let e = d.s.tic_finale_and_melt(d.cmds[1]).expect_err("refused");
        assert_eq!((e.tic, e.witness), (1, SessionWitness::MeltInProgress));
        assert_eq!(d.bytes(), before, "a tic during a melt");
        // Old-version saves from the new session, in a melt and (below) outside one.
        assert!(matches!(
            d.s.save(),
            Err(DsesError::Configuration(Configuration::FinaleAndMelt))
        ));
        assert_eq!(
            d.s.sound().save_v2_checked(),
            Err(ChainSaveRefusal::FinaleAndMeltSavesVersion3)
        );
        assert_eq!(
            d.s.display().session_state_v3_checked(),
            Err(SessionStateRefusal::FinaleAndMeltSavesVersion4)
        );
        assert!(d.s.display().session_state().is_none());
        let e = d.s.tic(d.cmds[1]).expect_err("refused");
        assert_eq!(
            e.witness,
            SessionWitness::Configuration(Configuration::FinaleAndMelt)
        );
        assert_eq!(d.bytes(), before, "the refused calls");
    }
    d.run_to_tic(1);
    let before = d.bytes();
    let e = d.s.melt_step(&mut Frame::new()).expect_err("no melt");
    assert_eq!((e.tic, e.witness), (1, SessionWitness::NoMelt));
    assert!(matches!(
        d.s.save(),
        Err(DsesError::Configuration(Configuration::FinaleAndMelt))
    ));
    assert_eq!(
        d.s.sound().save_v2_checked(),
        Err(ChainSaveRefusal::FinaleAndMeltSavesVersion3)
    );
    assert_eq!(
        d.s.display().session_state_v3_checked(),
        Err(SessionStateRefusal::FinaleAndMeltSavesVersion4)
    );
    assert_eq!(d.bytes(), before, "a melt step with no melt");
    // The frozen session refuses every new-version call; its own checked saves are its saves.
    let item = m6a::item("CHAIN");
    let settings = SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: 10,
    };
    let mut frozen = Session::new(d.data.clone(), settings).expect("frozen");
    frozen.tic(d.cmds[0]).expect("tic 0");
    let v1 = frozen.save().expect("version 1").encode();
    let frozen_config = SessionWitness::Configuration(Configuration::RefusesFinaleAndMelt);
    assert_eq!(
        frozen
            .tic_finale_and_melt(d.cmds[1])
            .expect_err("refused")
            .witness,
        frozen_config
    );
    assert_eq!(
        frozen
            .melt_step(&mut Frame::new())
            .expect_err("refused")
            .witness,
        frozen_config
    );
    assert!(matches!(
        frozen.save_v2(),
        Err(DsesError::Configuration(
            Configuration::RefusesFinaleAndMelt
        ))
    ));
    assert_eq!(
        frozen.sound().save_v3_checked(),
        Err(ChainSaveRefusal::RefusesMeltSavesVersion2)
    );
    assert_eq!(frozen.sound().save_v2_checked(), Ok(frozen.sound().save()));
    assert_eq!(
        frozen.display().session_state_v4_checked(),
        Err(SessionStateRefusal::RefusesFinaleAndMeltSavesVersion3)
    );
    assert_eq!(
        frozen.display().session_state_v3_checked().ok(),
        frozen.display().session_state()
    );
    assert_eq!(frozen.save().expect("version 1").encode(), v1, "unchanged");
    assert_eq!(frozen.configuration(), Configuration::RefusesFinaleAndMelt);
    assert!(!frozen.melt_in_progress());
    // Skills 0 and 1: refused at the start as the frozen session refuses them.
    for skill in [0u8, 1] {
        let s = SessionSettings { skill, ..settings };
        let a = Session::new(d.data.clone(), s)
            .map(|_| ())
            .expect_err("frozen");
        let b = Session::new_finale_and_melt(d.data.clone(), s)
            .map(|_| ())
            .expect_err("new");
        assert_eq!(a, StartError::NotYet(NotYet::EasySkill));
        assert_eq!(b, a, "skill {skill}");
    }
    // A planted fault cannot be saved.
    let mut f = Drive::new("CHAIN");
    f.s.plant_fault(PlantedFault::NoArtWipe);
    assert!(matches!(f.s.save_v2(), Err(DsesError::NotSaveable)));
}

/// The rule tying the two embedded states' melts together: a state whose `DSND` pending display
/// and `DDSP` melt in progress disagree is refused ([`DsesError::MeltAndPendingDisplay`]), both
/// ways, by `check` and by the reader; each forgery is otherwise one the embedded readers accept.
#[test]
fn a_pending_display_and_a_melt_must_agree() {
    let mut d = Drive::new("CHAIN");
    // A melt at a tic that owes no buffer, so that flipping the pending flag keeps DSND's own
    // buffer rule, and a tic outside a melt likewise.
    let (mut in_melt, mut outside) = (None, None);
    while in_melt.is_none() || outside.is_none() {
        let call = d.call().expect("a call");
        let t = d.s.gametic();
        let owes = Schedule::buffers_before(t) != Schedule::buffers_before(t - 1);
        if let Call::Tic { wiped, .. } = call {
            if !owes {
                let st = d.s.save_v2().expect("state");
                match (wiped, &in_melt, &outside) {
                    (true, None, _) => in_melt = Some(st),
                    (false, _, None) => outside = Some(st),
                    _ => {}
                }
            }
        }
    }
    for (mut st, what) in [
        (in_melt.expect("melt"), "a melt without a pending display"),
        (
            outside.expect("outside"),
            "a pending display without a melt",
        ),
    ] {
        st.check().expect("the session's own state");
        st.sound.state.melt = !st.sound.state.melt;
        assert!(
            matches!(st.check(), Err(DsesError::MeltAndPendingDisplay)),
            "{what}: {:?}",
            st.check()
        );
        let bytes = st.encode();
        doom_sound::chain_melt::ChainSoundStateV3::decode(&st.sound.encode())
            .expect("DSND version 3 accepts the forged sound alone");
        assert!(
            matches!(
                SessionStateV2::decode(&bytes),
                Err(DsesError::MeltAndPendingDisplay)
            ),
            "{what}"
        );
    }
}

/// The byte offsets of a version 2 encoding's variable part: the carried tag, the finale tag and
/// the level length.
fn offsets(st: &SessionStateV2) -> (usize, usize, usize) {
    let carried = 198;
    let carried_len = match &st.carried {
        None => 1,
        Some(c) if c.start.is_some() => 1 + 71 + 10,
        Some(_) => 1 + 10,
    };
    let finale = carried + carried_len;
    let level = finale + if st.finale.is_some() { 9 } else { 1 };
    (carried, finale, level)
}

/// The version 2 reader: round trips of the session's own states (a level, a melt, the
/// intermission, both finale stages), every version refused by the other's reader, truncation at
/// every prefix of the fixed part and around every boundary (and at a stride over the rest), bad
/// lengths, out-of-domain values, trailing bytes, and states that no session produces; none
/// panics. Restore refuses another mode and another level.
#[test]
fn the_version_2_reader() {
    let mut states: Vec<(String, SessionStateV2)> = Vec::new();
    let mut d = Drive::new("FINRET");
    for t in [0u64, 5, 321, 1000, 1891, 1900] {
        d.run_to_tic(t);
        d.call().expect("call");
        states.push((
            format!("FINRET after tic {t}"),
            d.s.save_v2().expect("state"),
        ));
    }
    let mut c = Drive::new("CHAIN");
    c.run_to_tic(60);
    states.push(("CHAIN tic 60".into(), c.s.save_v2().expect("state")));
    for (what, st) in &states {
        let bytes = st.encode();
        assert_eq!(&bytes[..6], b"DSES\x02\x00");
        eprintln!("{what}: {} bytes", bytes.len());
        let back = SessionStateV2::decode(&bytes).unwrap_or_else(|e| panic!("{what}: {e}"));
        assert_eq!(&back, st, "{what}");
        assert_eq!(back.encode(), bytes, "{what}");
        assert!(matches!(
            SessionState::decode(&bytes),
            Err(DsesError::Version(2))
        ));
        let (carried, finale, level) = offsets(st);
        assert_eq!(
            usize::from(bytes[carried]),
            match &st.carried {
                None => 0,
                Some(c) if c.start.is_some() => 1,
                Some(_) => 2,
            },
            "{what}: carried offset"
        );
        assert_eq!(bytes[finale], u8::from(st.finale.is_some()), "{what}");
        let n = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4")) as usize;
        let display = level + 4 + n(level);
        let sound = display + 4 + n(display);
        assert_eq!(bytes[sound], 3, "{what}: the sound kind");
        assert_eq!(sound + 5 + n(sound + 1), bytes.len(), "{what}: the layout");
        // Truncation.
        let mut cuts: Vec<usize> = (0..level + 8).collect();
        for b in [level, display, sound, bytes.len()] {
            cuts.extend(b.saturating_sub(3)..(b + 3).min(bytes.len()));
        }
        cuts.extend((0..bytes.len()).step_by(4093));
        for cut in cuts {
            if cut < bytes.len() {
                assert!(
                    SessionStateV2::decode(&bytes[..cut]).is_err(),
                    "{what}: {cut}"
                );
            }
        }
        // Trailing, magic, version.
        let mut more = bytes.clone();
        more.push(0);
        assert!(matches!(
            SessionStateV2::decode(&more),
            Err(DsesError::Trailing)
        ));
        let set = |at: usize, v: u8| {
            let mut b = bytes.clone();
            b[at] = v;
            SessionStateV2::decode(&b)
        };
        assert!(matches!(set(0, b'X'), Err(DsesError::Magic)));
        assert!(matches!(set(4, 3), Err(DsesError::Version(3))));
        assert!(matches!(set(4, 1), Err(DsesError::Version(1))));
        // Out of domain.
        assert!(matches!(set(6, 2), Err(DsesError::Value("mode"))));
        assert!(matches!(set(7, 2), Err(DsesError::Value("mode source"))));
        assert!(matches!(set(9, 0), Err(DsesError::Value("skill"))));
        assert!(matches!(set(9, 4), Err(DsesError::Value("skill"))));
        assert!(matches!(set(10, 12), Err(DsesError::Value("screenblocks"))));
        assert!(matches!(set(19, 3), Err(DsesError::Value("gamestate"))));
        assert!(matches!(set(21, 7), Err(DsesError::Value("gameaction"))));
        assert!(matches!(set(22, 2), Err(DsesError::Value("secretexit"))));
        assert!(matches!(set(carried, 3), Err(DsesError::Value("carried"))));
        assert!(matches!(set(finale, 2), Err(DsesError::Value("finale"))));
        assert!(matches!(set(sound, 2), Err(DsesError::SoundKind(2))));
        // Bad lengths.
        let mut huge = bytes.clone();
        huge[level..level + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            SessionStateV2::decode(&huge),
            Err(DsesError::Truncated)
        ));
        let mut longer = bytes.clone();
        longer[display..display + 4].copy_from_slice(&((n(display) + 1) as u32).to_le_bytes());
        assert!(SessionStateV2::decode(&longer).is_err(), "{what}");
        // A finale's stage and count.
        if st.finale.is_some() {
            let mut b = bytes.clone();
            b[finale + 1..finale + 5].copy_from_slice(&2i32.to_le_bytes());
            assert!(matches!(
                SessionStateV2::decode(&b),
                Err(DsesError::Value("finalestage"))
            ));
            let mut b = bytes.clone();
            b[finale + 5..finale + 9].copy_from_slice(&(-1i32).to_le_bytes());
            assert!(matches!(
                SessionStateV2::decode(&b),
                Err(DsesError::Value("finalecount"))
            ));
            let mut f = st.clone();
            f.finale = f.finale.map(|v| FinaleView {
                count: v.count + 1,
                ..v
            });
            assert!(matches!(
                SessionStateV2::decode(&f.encode()),
                Err(DsesError::Inconsistent("finalecount and the finale's tics"))
            ));
            let mut f = st.clone();
            f.finale = None;
            assert!(matches!(
                SessionStateV2::decode(&f.encode()),
                Err(DsesError::Inconsistent("finale and gamestate"))
            ));
        } else {
            let mut f = st.clone();
            f.finale = Some(FinaleView { stage: 0, count: 1 });
            assert!(SessionStateV2::decode(&f.encode()).is_err(), "{what}");
        }
        // States no session produces.
        let mut f = st.clone();
        f.gametic += 1;
        assert!(f.check().is_err(), "{what}: gametic");
        let mut f = st.clone();
        f.mode.mode = match st.mode.mode {
            SessionMode::Registered => SessionMode::Retail,
            SessionMode::Retail => SessionMode::Registered,
        };
        assert!(
            matches!(f.check(), Err(DsesError::Inconsistent("display game mode"))),
            "{what}: {:?}",
            f.check()
        );
    }
    // The other version's bytes.
    let item = m6a::item("CHAIN");
    let settings = SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: 10,
    };
    let mut frozen = Session::new(c.data.clone(), settings).expect("frozen");
    frozen.tic(c.cmds[0]).expect("tic");
    let v1 = frozen.save().expect("version 1").encode();
    assert!(matches!(
        SessionStateV2::decode(&v1),
        Err(DsesError::Version(1))
    ));
    // Restore: another declared mode, another level.
    let (_, st) = &states[2];
    let retail = m6a::data(&m6a::item("FINRET"), Some(SessionMode::Registered));
    assert!(matches!(
        Session::restore_v2(retail, st, d.s.level().clone()),
        Err(DsesError::Mode)
    ));
    assert!(matches!(
        Session::restore_v2(d.data.clone(), st, c.s.level().clone()),
        Err(DsesError::LevelMismatch)
    ));
}

/// Pins: seeds for the three part 2 fuzz targets (`m6a2_dses2`, `m6a_ddsp4`, `m6a_dsnd3`), the
/// whole `DSES` version 2 state and its embedded `DDSP` version 4 and `DSND` version 3 blobs (the
/// `display.encode()` and `sound.encode()` of the public [`SessionStateV2`] fields, the very values
/// `DSES` embeds), written under the directory `M6A2_FUZZ_SEEDS` names (outside the repository),
/// one folder per target. The states are saved from `Session::new_finale_and_melt` on the committed
/// items FINRET (a level, the intermission, melts of several kinds, both finale stages) and CHAIN,
/// at the first states of each (gamestate, finale stage) and of each melt kind, a few steps into
/// each melt, and every 300th call. Local only; it asserts only the seed counts. It lives at test
/// level because a fuzz seed is a file outside the repository that no public check can read.
#[test]
#[ignore = "local only: writes fuzz seeds under M6A2_FUZZ_SEEDS"]
fn write_fuzz_seeds_part2_local() {
    let Some(dir) = std::env::var_os("M6A2_FUZZ_SEEDS") else {
        return;
    };
    let mut seen: BTreeMap<String, u32> = BTreeMap::new();
    let mut written = 0usize;
    for item in ["FINRET", "CHAIN"] {
        let mut d = Drive::new(item);
        let mut kind = String::from("none");
        let mut calls = 0u32;
        while let Some(c) = d.call() {
            calls += 1;
            if let Some(k) = melt_kind(&c) {
                kind = k.replace(' ', "_");
            }
            let melting = d.s.melt_in_progress();
            let key = if melting {
                format!("melt_{kind}")
            } else {
                match d.s.finale() {
                    Some(f) => format!("finale{}", f.stage),
                    None => format!("{:?}", d.s.gamestate()).to_lowercase(),
                }
            };
            let n = seen.entry(format!("{item}_{key}")).or_insert(0);
            let take = if melting {
                matches!(*n, 0 | 3 | 9)
            } else {
                *n < 2 || calls % 300 == 0
            };
            *n += 1;
            if !take {
                continue;
            }
            let Ok(state) = d.s.save_v2() else { continue };
            let name = format!("{item}_{key}_{calls}");
            for (target, bytes) in [
                ("m6a2_dses2", state.encode()),
                ("m6a_ddsp4", state.display.encode()),
                ("m6a_dsnd3", state.sound.encode()),
            ] {
                let folder = std::path::Path::new(&dir).join(target);
                std::fs::create_dir_all(&folder).expect("folder");
                std::fs::write(folder.join(&name), bytes).expect("seed");
            }
            written += 1;
        }
    }
    assert!(written >= 6, "only {written} seeds");
}
