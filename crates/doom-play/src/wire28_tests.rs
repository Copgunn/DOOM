//! Wire 28 in doom-play (decision D3 of 2026-11-25: the session's wire 28 configuration for every
//! skill). Rust-only checks: the `--skill` values 0 to 3 accepted and nightmare refused, the help's
//! account of the skills and of the escape stop, and a skill 0 session through the app (doom-play's
//! loader) whose demo carries skill byte 0, is read back by the wire 28 reader alone and replays
//! through a fresh wire 28 session to the same state. No genuine trace is compared here.

use crate::melt_tests::at;
use crate::{HELP, parse};
use doom_game::app::{App, GameData, Host, SessionConfig, Timestamp};
use doom_game::session::demo::{SessionDemo, replay_finale_and_melt};
use doom_game::session::finale_melt::Configuration;
use doom_game::session::{ModeSetting, Session, SessionSettings};
use std::path::Path;
use std::sync::Arc;

fn args(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

/// `--skill 0` and `--skill 1` are accepted beside 2, 3, `hmp` and `uv`; 4 and `nightmare` are
/// refused with a message naming nightmare; anything else is refused as before.
#[test]
fn skills_0_to_3_are_accepted_and_nightmare_is_refused() {
    for (given, want) in [
        ("0", 0),
        ("1", 1),
        ("2", 2),
        ("3", 3),
        ("hmp", 2),
        ("uv", 3),
    ] {
        let o = parse(args(&format!("--iwad a.wad --skill {given}")))
            .unwrap()
            .unwrap();
        assert_eq!(o.skill, want, "{given}");
        assert_eq!((o.map, o.screenblocks), (1, 10), "nothing else: {given}");
    }
    for nightmare in ["4", "nightmare"] {
        let e = parse(args(&format!("--iwad a.wad --skill {nightmare}"))).unwrap_err();
        assert_eq!(e, "--skill: nightmare (4) is not built yet", "{nightmare}");
    }
    for bad in ["5", "-1", "itytd", "", "00x"] {
        assert!(
            parse(args(&format!("--iwad a.wad --skill {bad}"))).is_err(),
            "{bad}"
        );
    }
}

/// The help names skills 0 to 3, says nightmare is not built, no longer lists the easy skills or
/// E1M7 on skill 3 as refused, and names the new possible stop (a monster shoved through a
/// one-sided wall refuses the tic).
#[test]
fn the_help_lists_skills_0_to_3_and_the_escape_stop() {
    for want in [
        "0: I'm Too Young To Die; 1: Hey, Not Too Rough;",
        "2 or hmp: Hurt Me Plenty (default); 3 or uv: Ultra-Violence.",
        "Nightmare (4) is not built yet",
        "skills 0 to 3, and E1M7 on skill 3 plays.",
        "Not built yet: nightmare (skill 4), which is refused at launch.",
        "monster or a corpse is shoved through a one-sided wall",
        "the session refuses that tic",
        "TIC REFUSED screen naming the rule",
    ] {
        assert!(HELP.contains(want), "{want}");
    }
    for gone in [
        "The easy skills (0 and 1) are not built",
        "Not built yet: the easy skills",
        "E1M7 on skill 3, where the session stops",
    ] {
        assert!(!HELP.contains(gone), "{gone}");
    }
}

/// A host that keeps every file the app saves.
#[derive(Default)]
struct FileHost {
    saved: Vec<(String, Vec<u8>)>,
}

impl Host for FileHost {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, _: &str) -> bool {
        false
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.saved.push((name.to_owned(), bytes.to_vec()));
        Ok(())
    }
}

/// The synthetic episode's WADs (CHAIN's, as `melt_tests::app`).
fn wads() -> Vec<Vec<u8>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/reference");
    [
        "m6a_wads/episode.wad",
        "m5b_gfx.wad",
        "m5c_sfx.wad",
        "m6a_wads/m6a_gfx.wad",
        "m6a_wads/m6a_music.wad",
    ]
    .iter()
    .map(|w| std::fs::read(dir.join(w)).expect("WAD"))
    .collect()
}

/// doom-play's loader at skill 0: a session of the wire 28 configuration plays 120 tics with the
/// up arrow held; its demo file (named `...-skill0-...`) has skill byte 0 in its header (the
/// demo header's second byte, g_game.c:1556), the wire 28 reader reads it back to the recorded
/// commands while the earlier reader refuses it at the skill byte, and the commands replayed
/// through a fresh wire 28 session reach the same tic and the same saved state (`DSES` v3).
#[test]
fn a_skill_0_recording_round_trips_its_header_and_replays() {
    let files = wads();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let data = GameData::load_wire28_session(&refs, ModeSetting::Probe, 1, 0).expect("skill 0");
    assert_eq!(
        data.session_configuration(),
        Configuration::FinaleAndMeltWire28
    );
    let mut host = FileHost::default();
    let mut app = App::new(data, SessionConfig::default(), &mut host);
    app.key(doom_game::input::keys::KEY_UPARROW, true, &mut host);
    let mut k = 0;
    while app.tic() < 120 {
        k += 1;
        assert!(k < 1_000, "120 tics not reached");
        app.update(at(k), &mut host);
    }
    app.quit(&mut host);
    let (name, bytes) = host.saved.last().expect("a demo file").clone();
    assert!(name.starts_with("E1M1-skill0-"), "{name}");
    assert_eq!(
        (bytes[0], bytes[1], bytes[3]),
        (110, 0, 1),
        "version, skill, map"
    );
    let demo = SessionDemo::parse_v3(&bytes).expect("the wire 28 reader");
    assert_eq!((demo.map, demo.skill), (1, 0));
    assert_eq!(demo.commands, app.recorder().commands());
    assert_eq!(demo.bytes_v3().expect("written"), bytes);
    assert!(SessionDemo::parse(&bytes).is_err(), "the earlier reader");
    assert!(demo.bytes().is_err(), "the earlier writer");
    let played = app.session().expect("session");
    let session_data = Arc::clone(played.data());
    let settings = SessionSettings {
        map: 1,
        skill: 0,
        screenblocks: 10,
    };
    let mut replayed =
        Session::new_finale_and_melt_wire28(session_data, settings).expect("a fresh session");
    let end = replay_finale_and_melt(&mut replayed, &demo.commands, |_, _| {}, |_, _, _| {});
    assert_eq!(end, None);
    assert_eq!(replayed.gametic(), played.gametic());
    assert_eq!(
        replayed.save_v3().expect("saveable").encode(),
        played.save_v3().expect("saveable").encode()
    );
}
