//! Wire 28 in the session (decisions D2 and D3 of 2026-11-25): the session's third configuration,
//! `Configuration::FinaleAndMeltWire28` (`Session::new_finale_and_melt_wire28`), is the
//! finale-and-melt configuration over wire 28 (DSIM44), saved as `DSES` version 3.
//!
//! What is checked, and of which kind:
//!
//! - **Admission** (Rust-only safety): the wire 28 session starts on the synthetic m6a items
//!   SKILL0 to SKILL3 (skills 0 to 3) and refuses skill 4; the two older configurations still
//!   refuse skills 0 and 1 with master's error (`StartError::NotYet(NotYet::EasySkill)`), in the
//!   session and in the app. The SKILL0 to SKILL3 demos replay through the wire 28 session without
//!   a refusal to the genuine engine's session tic count (`m6a_results.json`, written by the
//!   genuine linuxdoom-1.10 engine alone); the streams themselves are compared with the genuine
//!   goldens by the next step, not here.
//! - **Wire 27 against wire 28 on skills 2 and 3** (an explicit fixture projection): on a handful
//!   of items both sessions give the same reports, frames, `M_Random` index, sound, mixer buffers,
//!   melt steps and pictures, refusals and saved states; the level captures are equal field for
//!   field once the three values that name the wire (the version, `profile`, `context.schema`) are
//!   read under wire 27's names, as `m6a2_run::wire26_fields` does for 43 against 42.
//! - **`DSES` version 3** (Rust-only safety): a round trip mid-level at skill 0; restore and
//!   continue equals the uninterrupted run; version 3 refuses version 2's bytes and a DSIM43
//!   capture inside, version 2 refuses version 3's bytes, a DSIM44 capture inside and skill 0,
//!   version 1 refuses version 3; each configuration saves only its own version.
//!
//! Not here: an escape-witness refusal inside a session. The witness's situation (a monster's box
//! across a one-sided wall, then a blow strong enough to clear it) is built in doom-sim's own
//! tests by setting a momentum through crate-private fields, which doom-game cannot reach; the
//! next step builds a synthetic WAD that reaches it through the session.
mod m6a;

use doom_game::app::{GameData, LoadError};
use doom_game::session::demo::{SessionDemo, replay_finale_and_melt};
use doom_game::session::finale_melt::{
    Configuration, FinaleMeltTicKind, FinaleMeltTicReport, MeltStepReport,
};
use doom_game::session::state::{DsesError, SessionState};
use doom_game::session::state_v2::SessionStateV2;
use doom_game::session::state_v3::SessionStateV3;
use doom_game::session::{
    ModeSetting, NotYet, Session, SessionSettings, SessionWitness, StartError,
};
use doom_render::Frame;
use doom_sim::TicCommand;
use doom_sim::{PLAYER_EPISODE1_V5_CONTEXT_SCHEMA, PLAYER_EPISODE1_V6_CONTEXT_SCHEMA, Snapshot};

fn settings(item: &m6a::Item, skill: u8) -> SessionSettings {
    SessionSettings {
        map: item.map,
        skill,
        screenblocks: 10,
    }
}

/// The commands of an item's demo, read by the wire 28 reader (skills 0 to 3).
fn commands(item: &m6a::Item) -> Vec<TicCommand> {
    let lmp = std::fs::read(m6a::root(&item.lmp)).expect("demo");
    let demo = SessionDemo::parse_v3(&lmp).expect("the wire 28 reader");
    assert_eq!(
        (demo.map, demo.skill),
        (item.map, item.skill),
        "{}",
        item.name
    );
    demo.commands
}

/// A DSIM44 capture's fields under wire 27's names (the explicit fixture projection of the
/// module documentation); a DSIM43 capture's as they are.
fn wire27_fields(s: &Snapshot) -> Vec<(String, Vec<u8>)> {
    let rename = s.version() == 44;
    assert!(rename || s.version() == 43, "a wire 27 or 28 capture");
    s.fields()
        .map(|f| {
            let value = match (rename, f.path()) {
                (true, "profile") => {
                    assert_eq!(f.value(), 28_u32.to_le_bytes(), "wire 28's profile");
                    27_u32.to_le_bytes().to_vec()
                }
                (true, "context.schema") => {
                    assert_eq!(f.value(), PLAYER_EPISODE1_V6_CONTEXT_SCHEMA);
                    PLAYER_EPISODE1_V5_CONTEXT_SCHEMA.to_vec()
                }
                _ => f.value().to_vec(),
            };
            (f.path().to_owned(), value)
        })
        .collect()
}

/// One session tic and every melt step its display started: the tic's report (its buffer, when
/// the tic melts, taken from the melt's last step) and the melt's steps with their pictures.
type Tic = Result<(FinaleMeltTicReport, Vec<(MeltStepReport, Vec<u8>)>), SessionWitness>;

fn tic(s: &mut Session, cmd: TicCommand) -> Tic {
    let mut r = s.tic_finale_and_melt(cmd).map_err(|e| e.witness)?;
    let mut steps = Vec::new();
    let mut picture = Frame::new();
    while s.melt_in_progress() {
        let m = s.melt_step(&mut picture).map_err(|e| e.witness)?;
        if m.done {
            r.sound.sound.buffer = m.buffer;
        }
        steps.push((m, picture.pixels().to_vec()));
    }
    Ok((r, steps))
}

/// Two ticks' outcomes are the same (every reported field, the steps and pictures).
fn assert_same_tic(a: &Tic, b: &Tic, what: &str) {
    match (a, b) {
        (Ok((a, sa)), Ok((b, sb))) => {
            assert_eq!(
                (a.tic, a.kind, a.palette, a.palette_set, a.wiped, a.wi_draws),
                (b.tic, b.kind, b.palette, b.palette_set, b.wiped, b.wi_draws),
                "{what}: report"
            );
            assert_eq!(a.level_frame, b.level_frame, "{what}: level frame");
            assert_eq!(a.wi_starts, b.wi_starts, "{what}: intermission starts");
            assert_eq!(a.sound, b.sound, "{what}: sound");
            assert_eq!(a.finale, b.finale, "{what}: finale");
            assert_eq!(sa, sb, "{what}: melt steps and pictures");
        }
        (Err(a), Err(b)) => assert_eq!(a, b, "{what}: refusal"),
        _ => panic!(
            "{what}: only one refused: {a:?} / {b:?}",
            a = a.is_err(),
            b = b.is_err()
        ),
    }
}

/// Admission (Rust-only safety): skills 0 to 3 start under wire 28 (DSIM44 at tic 0), 4 is
/// refused; the two older configurations refuse 0 and 1 with master's error and still start 2
/// and 3; the app's loaders likewise. The SKILL0 to SKILL3 demos (the wire 28 reader; the earlier
/// reader refuses SKILL0 and SKILL1) replay without a refusal to the genuine session tic count.
#[test]
fn the_wire_28_session_starts_on_skills_0_to_3_and_the_older_ones_refuse_0_and_1() {
    for (name, skill) in [("SKILL0", 0), ("SKILL1", 1), ("SKILL2", 2), ("SKILL3", 3)] {
        let item = m6a::item(name);
        assert_eq!(item.skill, skill);
        let data = m6a::data(&item, None);
        let st = settings(&item, skill);
        let mut s = Session::new_finale_and_melt_wire28(data.clone(), st).expect(name);
        assert_eq!(s.configuration(), Configuration::FinaleAndMeltWire28);
        assert_eq!(s.level().snapshot().version(), 44, "{name}");
        let older = [
            Session::new(data.clone(), st).map(|_| ()),
            Session::new_finale_and_melt(data.clone(), st).map(|_| ()),
        ];
        for r in older {
            if skill < 2 {
                assert_eq!(r, Err(StartError::NotYet(NotYet::EasySkill)), "{name}");
            } else {
                assert_eq!(r, Ok(()), "{name}");
            }
        }
        let lmp = std::fs::read(m6a::root(&item.lmp)).expect("demo");
        assert_eq!(SessionDemo::parse(&lmp).is_ok(), skill >= 2, "{name}");
        let cmds = commands(&item);
        let end = replay_finale_and_melt(&mut s, &cmds, |_, _| {}, |_, _, _| {});
        assert_eq!(end, None, "{name}");
        assert_eq!(item.end, "marker");
        assert_eq!(s.gametic() as usize, item.session_tics, "{name}");
        // A wire 28 session runs no frozen tic and saves no version 1 or 2 state.
        let refused = s.tic(TicCommand::default()).expect_err("the frozen tic");
        assert_eq!(
            refused.witness,
            SessionWitness::Configuration(Configuration::FinaleAndMeltWire28)
        );
        for config in [s.save().map(|_| ()), s.save_v2().map(|_| ())] {
            assert!(matches!(
                config,
                Err(DsesError::Configuration(Configuration::FinaleAndMeltWire28))
            ));
        }
    }
    let item = m6a::item("SKILL0");
    for skill in [4, 5, 255] {
        let r = Session::new_finale_and_melt_wire28(m6a::data(&item, None), settings(&item, skill));
        assert_eq!(r.map(|_| ()), Err(StartError::Domain), "skill {skill}");
    }
    let files: Vec<Vec<u8>> = item
        .wads
        .iter()
        .map(|w| std::fs::read(m6a::root(w)).expect("WAD"))
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    for skill in [0, 1] {
        let new =
            GameData::load_wire28_session(&refs, ModeSetting::Probe, 1, skill).expect("wire 28");
        assert_eq!(
            new.session_configuration(),
            Configuration::FinaleAndMeltWire28
        );
        for old in [
            GameData::load_session(&refs, ModeSetting::Probe, 1, skill),
            GameData::load_finale_and_melt_session(&refs, ModeSetting::Probe, 1, skill),
        ] {
            assert!(matches!(
                old,
                Err(LoadError::SessionStart(StartError::NotYet(
                    NotYet::EasySkill
                )))
            ));
        }
    }
    assert!(matches!(
        GameData::load_wire28_session(&refs, ModeSetting::Probe, 1, 4),
        Err(LoadError::SessionStart(StartError::Domain))
    ));
}

/// Wire 27 against wire 28 on skills 2 and 3 (explicit fixture projection, module documentation):
/// SKILL2, SKILL3, INTFAST (intermissions), REBHELD (a reborn), SECRET (the secret exit),
/// NOVICTOR (the missing D_VICTOR refusal) and FINRET (the victory, the finale and its melts),
/// every tic and melt step; the saved states at every 35th tic and at every load, completion
/// and victory tic.
#[test]
fn the_wire_28_session_equals_the_wire_27_session_on_skills_2_and_3() {
    let (mut tics, mut saves) = (0usize, 0usize);
    for name in [
        "SKILL2", "SKILL3", "INTFAST", "REBHELD", "SECRET", "NOVICTOR", "FINRET",
    ] {
        let item = m6a::item(name);
        let cmds = commands(&item);
        let lmp = std::fs::read(m6a::root(&item.lmp)).expect("demo");
        let old = SessionDemo::parse(&lmp).expect("the earlier reader");
        assert_eq!(old.commands, cmds, "{name}: both readers");
        assert_eq!(
            old.bytes_v3().expect("v3"),
            old.bytes().expect("v2"),
            "{name}"
        );
        let data = m6a::data(&item, None);
        let st = settings(&item, item.skill);
        let mut a = Session::new_finale_and_melt(data.clone(), st).expect(name);
        let mut b = Session::new_finale_and_melt_wire28(data, st).expect(name);
        for (t, &cmd) in cmds.iter().enumerate() {
            let what = format!("{name} tic {t}");
            let (ra, rb) = (tic(&mut a, cmd), tic(&mut b, cmd));
            assert_same_tic(&ra, &rb, &what);
            assert_eq!(a.frame().pixels(), b.frame().pixels(), "{what}: frame");
            assert_eq!(
                a.display().random_index(),
                b.display().random_index(),
                "{what}: index"
            );
            let Ok((r, _)) = &ra else {
                assert_eq!(name, "NOVICTOR", "{what}: the only refusal");
                assert!(matches!(ra, Err(SessionWitness::Sound(_))), "{what}");
                break;
            };
            tics += 1;
            let boundary = matches!(
                r.kind,
                FinaleMeltTicKind::Load { .. }
                    | FinaleMeltTicKind::Completion
                    | FinaleMeltTicKind::Victory
            );
            if t % 35 != 0 && !boundary {
                continue;
            }
            let (v2, v3) = (a.save_v2().expect("v2"), b.save_v3().expect("v3"));
            let (la, lb) = (a.level().snapshot(), b.level().snapshot());
            assert_eq!((la.version(), lb.version()), (43, 44), "{what}: wires");
            assert_eq!(wire27_fields(&la), wire27_fields(&lb), "{what}: level");
            // Self-test of the projection: unprojected, exactly the two values naming the wire
            // differ, so the comparison above sees every other byte.
            let differing: Vec<&str> = la
                .fields()
                .zip(lb.fields())
                .filter(|(x, y)| x.path() != y.path() || x.value() != y.value())
                .map(|(x, _)| x.path())
                .collect();
            assert_eq!(differing, ["profile", "context.schema"], "{what}");
            assert_eq!(v3.0.level, lb.canonical_bytes(), "{what}: saved level");
            let (mut x, mut y) = (v2.clone(), v3.0.clone());
            x.level.clear();
            y.level.clear();
            assert_eq!(x, y, "{what}: every other saved field");
            // The encodings: the version and the level blob alone differ.
            let (e2, e3) = (v2.encode(), v3.encode());
            assert_eq!((e2[4], e3[4]), (2, 3), "{what}: version");
            x.level = v2.level.clone();
            y.level = v2.level.clone();
            assert_eq!(x.encode()[6..], SessionStateV3(y).encode()[6..], "{what}");
            saves += 1;
        }
    }
    eprintln!("wire 27 and wire 28: {tics} tics equal, {saves} saved states equal");
    assert!(tics > 3_000, "{tics}");
}

/// `DSES` version 3 (Rust-only safety): a wire 28 session of SKILL0 saved mid-level, encoded,
/// decoded and restored saves the same bytes, and continued to the demo's end equals the
/// uninterrupted session tic for tic and in its final saved state; the version and wire refusals
/// of the module documentation.
#[test]
fn dses_version_3_round_trips_restores_and_refuses_the_other_versions() {
    let item = m6a::item("SKILL0");
    let cmds = commands(&item);
    let data = m6a::data(&item, None);
    let mut s = Session::new_finale_and_melt_wire28(data.clone(), settings(&item, 0)).unwrap();
    let mid = 150;
    for &cmd in &cmds[..mid] {
        tic(&mut s, cmd).expect("admitted");
    }
    assert_eq!(s.gamestate(), doom_render::GameState::Level, "mid-level");
    let state = s.save_v3().expect("saveable");
    assert_eq!(state.0.settings.skill, 0);
    let bytes = state.encode();
    assert_eq!(&bytes[..6], b"DSES\x03\x00");
    let decoded = SessionStateV3::decode(&bytes).expect("decodes");
    assert_eq!(decoded, state);
    let mut r = Session::restore_v3(data.clone(), &decoded, s.level().clone()).expect("restores");
    assert_eq!(r.configuration(), Configuration::FinaleAndMeltWire28);
    assert_eq!(r.save_v3().expect("saveable").encode(), bytes);
    for (t, &cmd) in cmds.iter().enumerate().skip(mid) {
        assert_same_tic(&tic(&mut s, cmd), &tic(&mut r, cmd), &format!("tic {t}"));
    }
    assert_eq!(r.gametic() as usize, item.session_tics);
    assert_eq!(
        r.save_v3().unwrap().encode(),
        s.save_v3().unwrap().encode(),
        "restored and continued equals uninterrupted"
    );

    // Each reader refuses the other versions.
    assert!(matches!(
        SessionStateV2::decode(&bytes),
        Err(DsesError::Version(3))
    ));
    assert!(matches!(
        SessionState::decode(&bytes),
        Err(DsesError::Version(3))
    ));
    // Version 2 refuses skill 0 (before its level reader runs).
    assert!(matches!(
        SessionStateV2::decode(&state.0.encode()),
        Err(DsesError::Value("skill"))
    ));

    // SKILL2 under both configurations at the same mid-level tic.
    let item = m6a::item("SKILL2");
    let cmds = commands(&item);
    let data = m6a::data(&item, None);
    let st = settings(&item, 2);
    let mut a = Session::new_finale_and_melt(data.clone(), st).unwrap();
    let mut b = Session::new_finale_and_melt_wire28(data.clone(), st).unwrap();
    for &cmd in &cmds[..mid] {
        tic(&mut a, cmd).expect("wire 27");
        tic(&mut b, cmd).expect("wire 28");
    }
    let (v2, v3) = (a.save_v2().unwrap(), b.save_v3().unwrap());
    assert!(matches!(
        SessionStateV3::decode(&v2.encode()),
        Err(DsesError::Version(2))
    ));
    assert!(matches!(
        SessionStateV2::decode(&v3.encode()),
        Err(DsesError::Version(3))
    ));
    // A DSIM43 capture inside version 3, a DSIM44 capture inside version 2.
    let mut wrong3 = v3.clone();
    wrong3.0.level = v2.level.clone();
    assert!(matches!(
        SessionStateV3::decode(&wrong3.encode()),
        Err(DsesError::Level(_))
    ));
    let mut wrong2 = v2.clone();
    wrong2.level = v3.0.level.clone();
    assert!(matches!(
        SessionStateV2::decode(&wrong2.encode()),
        Err(DsesError::Level(_))
    ));
    // Each configuration saves only its own version, and restores only its own level.
    assert!(matches!(
        a.save_v3(),
        Err(DsesError::Configuration(Configuration::FinaleAndMelt))
    ));
    assert!(matches!(
        Session::restore_v3(data.clone(), &v3, a.level().clone()),
        Err(DsesError::LevelMismatch)
    ));
    let back = Session::restore_v3(data, &v3, b.level().clone()).expect("its own level");
    assert_eq!(back.save_v3().unwrap().encode(), v3.encode());
}
