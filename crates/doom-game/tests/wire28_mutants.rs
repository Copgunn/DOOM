//! Wire 28, mutation survivors of the run on cd1f7b05: each test names the mutant it kills.
//! Rust-only rules already cited in the source (`Recorder::new_v3`, `SessionStateV3::check`).
#![allow(clippy::unwrap_used, reason = "test code")]
mod m6a;

use doom_game::app::{App, GameData, Host, SessionConfig, Timestamp};
use doom_game::app::{FinaleMeltSessionTicReport, MeltStepShown, Recorder};
use doom_game::session::state::DsesError;
use doom_game::session::state_v2::SessionStateV2;
use doom_game::session::state_v3::SessionStateV3;
use doom_game::session::{ModeSetting, Session, SessionSettings};

struct H;
impl Host for H {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, _: &str) -> bool {
        false
    }
    fn save(&mut self, _: &str, _: &[u8]) -> Result<(), String> {
        Ok(())
    }
    fn finale_melt_tic(&mut self, _: &FinaleMeltSessionTicReport<'_>) {}
    fn melt_step(&mut self, _: &MeltStepShown<'_>) {}
}

/// Kills `app/recorder.rs:83:13` (delete field `v3` in `Recorder::new_v3`): a wire 28 recording
/// of skill 0 or 1 writes its demo (skill byte 0 or 1 in the header), which the earlier writer
/// refuses; at skills 2 and 3 both writers give the same bytes.
#[test]
fn a_wire_28_recording_writes_the_easy_skills() {
    for skill in [0, 1] {
        let bytes = Recorder::new_v3(1, skill, 10)
            .bytes()
            .expect("wire 28 writer");
        assert_eq!((bytes[1], bytes[3]), (skill, 1), "skill and map bytes");
        assert!(
            Recorder::new(1, skill, 10).bytes().is_err(),
            "the earlier writer"
        );
    }
    for skill in [2, 3] {
        assert_eq!(
            Recorder::new_v3(1, skill, 10).bytes(),
            Recorder::new(1, skill, 10).bytes()
        );
    }
}

/// Kills `app/mod.rs:541:50` (`==` -> `!=` in `GameData::recorder`): the app of a wire 28 session
/// at skills 0 and 1 holds a recording the wire 28 writer writes. (`:541:35`, `&&` -> `||`, is
/// not killable: it differs only for a session of another configuration, which refuses skills 0
/// and 1 at load, and at skills 2 and 3 the two writers' bytes are equal.)
#[test]
fn the_wire_28_app_records_the_easy_skills() {
    let item = m6a::item("SKILL0");
    let files: Vec<Vec<u8>> = item
        .wads
        .iter()
        .map(|w| std::fs::read(m6a::root(w)).unwrap())
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    for skill in [0, 1] {
        let data = GameData::load_wire28_session(&refs, ModeSetting::Probe, 1, skill).unwrap();
        let app = App::new(data, SessionConfig::default(), &mut H);
        let bytes = app.recorder().bytes().expect("the wire 28 writer");
        assert_eq!(bytes[1], skill, "the demo's skill byte");
    }
}

/// Kills `session/state_v3.rs:45:9` (`SessionStateV3::check` -> `Ok(())`): `check` alone refuses a
/// state with a field out of its domain that the encoder still writes (skill 4, screenblocks 9),
/// as `decode` does; skills 0 to 3 pass.
#[test]
fn check_refuses_a_state_out_of_its_domain() {
    let item = m6a::item("SKILL0");
    let data = m6a::data(&item, None);
    let st = SessionSettings {
        map: item.map,
        skill: 0,
        screenblocks: 10,
    };
    let state = Session::new_finale_and_melt_wire28(data, st)
        .unwrap()
        .save_v3()
        .unwrap();
    assert!(state.check().is_ok());
    let mut bad = state.clone();
    bad.0.settings.skill = 4;
    assert!(matches!(bad.check(), Err(DsesError::Value("skill"))));
    assert!(matches!(
        SessionStateV3::decode(&bad.encode()),
        Err(DsesError::Value("skill"))
    ));
    let mut bad = state;
    bad.0.settings.screenblocks = 9;
    assert!(matches!(bad.check(), Err(DsesError::Value("screenblocks"))));
}

/// Owner's ruling 2 of 2026-11-26: `DSES` version 3 refuses a state whose skill byte disagrees
/// with its embedded level's skill, with `SkillAndLevel`, in `check` and in `decode`; version 2
/// still accepts the same disagreement (its acceptance is frozen). Every level of a game is set up
/// on the one `gameskill` (g_game.c:484, 1450), so no session writes such a state. With
/// `WIRE28_DSES3_SEED` set to a path the refused version 3 bytes are also written there (the fuzz
/// seed `fuzz/corpus/wire28_dses3/skill_and_level`).
#[test]
fn dses_version_3_refuses_a_skill_byte_its_level_does_not_have() {
    let settings = |skill| SessionSettings {
        map: 1,
        skill,
        screenblocks: 10,
    };
    let data = m6a::data(&m6a::item("SKILL0"), None);
    let state = Session::new_finale_and_melt_wire28(data, settings(0))
        .unwrap()
        .save_v3()
        .unwrap();
    for skill in [1, 2, 3] {
        let mut bad = state.clone();
        bad.0.settings.skill = skill;
        let refused = |r: Result<(), DsesError>| matches!(r, Err(DsesError::SkillAndLevel { skill: s, level: Some(0) }) if s == skill);
        assert!(refused(bad.check()), "{skill}");
        assert!(refused(SessionStateV3::decode(&bad.encode()).map(|_| ())));
        if let (1, Ok(path)) = (skill, std::env::var("WIRE28_DSES3_SEED")) {
            std::fs::write(path, bad.encode()).unwrap();
        }
    }
    let data = m6a::data(&m6a::item("SKILL2"), None);
    let mut v2 = Session::new_finale_and_melt(data, settings(2))
        .unwrap()
        .save_v2()
        .unwrap();
    v2.settings.skill = 3;
    assert!(v2.check().is_ok());
    assert!(SessionStateV2::decode(&v2.encode()).is_ok());
}
