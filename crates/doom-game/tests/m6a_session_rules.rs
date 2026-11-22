//! M6a step 7: the session layer's own rules (Rust-side tests; the genuine comparison is
//! `tests/m6a_session.rs`).
//!
//! - **`DSES` version 1 restore and continue**: at the first tic of every kind a session passes
//!   (its start, a level tic, an exit tic with `ga_completed` pending, the completion tic, a
//!   later intermission tic, the tic that leaves `ga_worlddone` pending, a load tic, a tic with
//!   a reborn pending, the reload), the state is saved, encoded, decoded (byte-equal round trip)
//!   and restored next to a clone of the level's simulation, and the restored session continues
//!   to the demo's end with every frame, palette, `M_Random` index, sound report and later
//!   encoding equal to the uninterrupted run's;
//! - **forgeries** of the encoding and mismatched restores are refused;
//! - **refusals** leave the session unchanged (its encoding equal before and after): the genuine
//!   stops (a missing `D_INTER` on the intermission's first tic, a missing later map's music on
//!   its load tic), `BT_SPECIAL` in a replayed command, the E1M8 victory (the finale is built after
//!   the owner's hand test), a later map that does not load or is not admitted at its load tic, and
//!   a missing intermission picture (`WI_loadData`'s `I_Error`); the turbo message whose `gametic`
//!   and `leveltime` tests disagree is no longer refused (owner's ruling 1 of 2026-11-22);
//! - **the command domain** (owner's ruling 2 of 2026-11-22): wire 24's on a level or load tic,
//!   everything but `BT_SPECIAL` on an intermission tic, the completion tic included;
//! - **`DSES` and the start `gametic`**: a level whose `level.start.gametic` is not the sound's
//!   `levelstarttic` is refused at restore;
//! - **the declared game mode** (ruling U4): the probe, the forced setting, and the mode token of
//!   a session demo's file name.
mod m6a;

use doom_game::app::Timestamp;
use doom_game::session::demo::{
    ModeTokenError, SessionDemo, mode_of_session_name, replay, replay_mode, session_mode_name,
};
use doom_game::session::sound::SoundWitness;
use doom_game::session::state::{DsesError, SessionState};
use doom_game::session::{
    MapLoadError, ModeSetting, ModeSource, NotYet, Session, SessionData, SessionMode,
    SessionSettings, SessionWitness, StartError, TicKind, probe_game_mode,
};
use doom_render::{GameState, SessionError};
use doom_sim::TicCommand;
use doom_sound::chain::ChainRefusal;
use doom_test_evidence::sha256;
use std::sync::Arc;

/// The WAD files of an item, as bytes (the demo is added by [`data_of`]).
fn wad_bytes(item: &m6a::Item) -> Vec<Vec<u8>> {
    item.wads
        .iter()
        .map(|w| std::fs::read(m6a::root(w)).expect("WAD"))
        .collect()
}

fn data_of(item: &m6a::Item, wads: &[Vec<u8>], setting: ModeSetting) -> Arc<SessionData> {
    let refs: Vec<&[u8]> = wads.iter().map(Vec::as_slice).collect();
    let lmp = std::fs::read(m6a::root(&item.lmp)).expect("demo");
    Arc::new(SessionData::load_with(&refs, &[(&item.lmp, &lmp)], setting).expect("data"))
}

fn commands(item: &m6a::Item) -> Vec<TicCommand> {
    SessionDemo::parse(&std::fs::read(m6a::root(&item.lmp)).expect("demo"))
        .expect("demo")
        .commands
}

fn new_session(item: &m6a::Item, data: Arc<SessionData>) -> Session {
    let settings = SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: 10,
    };
    Session::new(data, settings).expect("session")
}

/// The directory entry index of the lump `name` in a WAD's bytes (the last match).
fn lump_index(wad: &[u8], name: &str) -> Option<(usize, usize, usize)> {
    let n = i32::from_le_bytes(wad[4..8].try_into().unwrap()) as usize;
    let dir = i32::from_le_bytes(wad[8..12].try_into().unwrap()) as usize;
    (0..n).rev().find_map(|i| {
        let e = dir + 16 * i;
        let raw = &wad[e + 8..e + 16];
        let end = raw.iter().position(|b| *b == 0).unwrap_or(8);
        (raw[..end] == *name.as_bytes()).then(|| {
            let pos = i32::from_le_bytes(wad[e..e + 4].try_into().unwrap()) as usize;
            let size = i32::from_le_bytes(wad[e + 4..e + 8].try_into().unwrap()) as usize;
            (e, pos, size)
        })
    })
}

/// The WAD with the directory name `from` renamed `to` (same length or shorter).
fn rename(mut wad: Vec<u8>, from: &str, to: &str) -> Vec<u8> {
    let (e, _, _) = lump_index(&wad, from).unwrap_or_else(|| panic!("lump {from}"));
    let mut name = [0u8; 8];
    name[..to.len()].copy_from_slice(to.as_bytes());
    wad[e + 8..e + 16].copy_from_slice(&name);
    wad
}

/// A minimal IWAD whose directory holds the given empty lumps.
fn iwad_with(names: &[&str]) -> Vec<u8> {
    let mut w = b"IWAD".to_vec();
    w.extend_from_slice(&(names.len() as i32).to_le_bytes());
    w.extend_from_slice(&12i32.to_le_bytes());
    for n in names {
        w.extend_from_slice(&12i32.to_le_bytes());
        w.extend_from_slice(&0i32.to_le_bytes());
        let mut name = [0u8; 8];
        name[..n.len()].copy_from_slice(n.as_bytes());
        w.extend_from_slice(&name);
    }
    w
}

/// The kind of a session tic for the restore points.
fn kind_of(s: &Session, k: TicKind) -> &'static str {
    match (k, s.gamestate(), s.gameaction()) {
        (TicKind::Load { .. }, _, _) => "load",
        (TicKind::Completion, _, 8) => "completion with world done",
        (TicKind::Completion, _, _) => "completion",
        (_, GameState::Intermission, 8) => "world done pending",
        (_, GameState::Intermission, _) => "intermission",
        (_, _, 6) => "exit pending",
        _ if s.level().reborn_requested() => "reborn pending",
        _ => "level",
    }
}

/// Per tic of an uninterrupted run: the frame digest, the palette pair, the `M_Random` index,
/// the sound report and the encoding's digest after the tic.
type Trace = Vec<(String, (u8, Option<u8>), u8, String, String)>;

fn trace_tic(
    s: &Session,
    r: &doom_game::session::TicReport,
) -> (String, (u8, Option<u8>), u8, String, String) {
    (
        sha256(s.frame().pixels()),
        (r.palette, r.palette_set),
        s.display().random_index(),
        format!("{:?}", r.sound),
        sha256(&s.save().expect("save").encode()),
    )
}

#[test]
fn dses_restore_and_continue_at_every_kind_of_tic() {
    let mut points_total = 0;
    let mut kinds_seen = std::collections::BTreeSet::new();
    for name in ["INTFAST", "REBHELD", "USEAT131", "RBXDEAD", "SECRET"] {
        let item = m6a::item(name);
        let wads = wad_bytes(&item);
        let data = data_of(&item, &wads, ModeSetting::Force(SessionMode::Registered));
        let cmds = commands(&item);
        let mut s = new_session(&item, data.clone());
        // The uninterrupted run, and the states saved at the first tic of each kind.
        let mut trace: Trace = Vec::new();
        let mut points: Vec<(usize, &str, Vec<u8>, doom_sim::Simulation)> = Vec::new();
        let start = s.save().expect("save at the start").encode();
        points.push((0, "start", start, s.level().clone()));
        let mut seen = std::collections::BTreeSet::new();
        for (t, &c) in cmds.iter().enumerate() {
            let r = s.tic(c).unwrap_or_else(|e| panic!("{name} tic {t}: {e}"));
            trace.push(trace_tic(&s, &r));
            let k = kind_of(&s, r.kind);
            if seen.insert(k) {
                let st = s.save().expect("save");
                let bytes = st.encode();
                let back = SessionState::decode(&bytes).expect("decode");
                assert_eq!(back.encode(), bytes, "{name} tic {t}: round trip");
                points.push((t + 1, k, bytes, s.level().clone()));
            }
        }
        for (at, k, bytes, level) in points {
            let st = SessionState::decode(&bytes).expect("decode");
            let mut r = Session::restore(data.clone(), &st, level)
                .unwrap_or_else(|e| panic!("{name} after {at} tics ({k}): restore {e:?}"));
            assert_eq!(
                r.save().expect("save").encode(),
                bytes,
                "{name} {k}: restored encoding"
            );
            for (t, &c) in cmds.iter().enumerate().skip(at) {
                let rep = r
                    .tic(c)
                    .unwrap_or_else(|e| panic!("{name} {k}: tic {t}: {e}"));
                assert_eq!(
                    trace_tic(&r, &rep),
                    trace[t],
                    "{name} restored after {at} ({k}): tic {t}"
                );
            }
            points_total += 1;
            kinds_seen.insert(k);
        }
        eprintln!("{name}: kinds {seen:?}");
    }
    eprintln!("DSES restore and continue: {points_total} points, kinds {kinds_seen:?}");
    for k in [
        "start",
        "level",
        "exit pending",
        "completion",
        "intermission",
        "world done pending",
        "load",
        "reborn pending",
    ] {
        assert!(kinds_seen.contains(k), "kind {k} restored");
    }
}

/// A saved state of INTFAST in its intermission (tic 60) and in its first level (tic 30),
/// with the session data and the level simulations.
fn intfast_states() -> (
    Arc<SessionData>,
    Vec<u8>,
    doom_sim::Simulation,
    Vec<u8>,
    doom_sim::Simulation,
) {
    let item = m6a::item("INTFAST");
    let data = data_of(
        &item,
        &wad_bytes(&item),
        ModeSetting::Force(SessionMode::Registered),
    );
    let mut s = new_session(&item, data.clone());
    let cmds = commands(&item);
    let (mut level_state, mut level_sim) = (Vec::new(), None);
    for (t, &c) in cmds.iter().enumerate().take(60) {
        s.tic(c).expect("tic");
        if t + 1 == 30 {
            level_state = s.save().expect("save").encode();
            level_sim = Some(s.level().clone());
        }
    }
    assert_eq!(s.gamestate(), GameState::Intermission);
    let wi = s.save().expect("save").encode();
    (
        data,
        wi,
        s.level().clone(),
        level_state,
        level_sim.expect("level"),
    )
}

#[test]
fn dses_forgeries_and_mismatches_are_refused() {
    let (data, wi, frozen, lv, level) = intfast_states();
    // Offsets of the fixed head: magic 0, version 4, mode 6, source 7, map 8, skill 9,
    // screenblocks 10, gametic 11, gamestate 19, gamemap 20, gameaction 21, secretexit 22,
    // didsecret 23, wminfo 24 (epsd 24, didsecret 28, last 29, next 33, ...).
    let forged = |at: usize, v: &[u8]| {
        let mut b = wi.clone();
        b[at..at + v.len()].copy_from_slice(v);
        SessionState::decode(&b)
    };
    let cases: Vec<(&str, Result<SessionState, DsesError>)> = vec![
        ("magic", forged(0, b"DSEZ")),
        ("version 2", forged(4, &2u16.to_le_bytes())),
        ("mode 2", forged(6, &[2])),
        ("skill 1 (easy skills after the hand test)", forged(9, &[1])),
        ("screenblocks 9", forged(10, &[9])),
        ("gametic", forged(11, &1u64.to_le_bytes())),
        ("finale", forged(19, &[2])),
        ("level state with a carried player", forged(19, &[0])),
        ("gamemap 10", forged(20, &[10])),
        ("gameaction 6 in the intermission", forged(21, &[6])),
        ("gameaction 7", forged(21, &[7])),
        ("secretexit 2", forged(22, &[2])),
        ("wminfo epsd 1", forged(24, &1i32.to_le_bytes())),
        ("wminfo next", forged(33, &4i32.to_le_bytes())),
        ("truncated", SessionState::decode(&wi[..wi.len() - 1])),
        (
            "trailing",
            SessionState::decode(&[wi.as_slice(), &[0]].concat()),
        ),
    ];
    for (what, r) in cases {
        assert!(r.is_err(), "{what}: accepted");
        eprintln!("{what}: {:?}", r.err());
    }
    // The finale slot and the sound kind: found by their position after the carried player.
    let st = SessionState::decode(&wi).expect("decode");
    let level_len = st.level.len();
    let display_len = st.display.encode().len();
    let sound_len = st.sound.encode().len();
    let finale = wi.len() - (1 + 4 + sound_len) - (4 + display_len) - (4 + level_len) - 1;
    assert_eq!(wi[finale], 0);
    let mut b = wi.clone();
    b[finale] = 1;
    assert!(matches!(
        SessionState::decode(&b),
        Err(DsesError::NotYetFinale)
    ));
    let mut b = wi.clone();
    b[wi.len() - 4 - sound_len - 1] = 1;
    assert!(matches!(
        SessionState::decode(&b),
        Err(DsesError::SoundKind(1))
    ));
    // Restores that do not fit.
    let item = m6a::item("INTFAST");
    let retail = data_of(
        &item,
        &wad_bytes(&item),
        ModeSetting::Force(SessionMode::Retail),
    );
    assert!(matches!(
        Session::restore(retail, &st, frozen.clone()),
        Err(DsesError::Mode)
    ));
    assert!(matches!(
        Session::restore(data.clone(), &st, level.clone()),
        Err(DsesError::LevelMismatch)
    ));
    let lst = SessionState::decode(&lv).expect("decode");
    assert!(matches!(
        Session::restore(data.clone(), &lst, frozen.clone()),
        Err(DsesError::LevelMismatch)
    ));
    let mut dead = st.clone();
    dead.carried.as_mut().expect("carried").start = None;
    assert!(
        Session::restore(data.clone(), &dead, frozen.clone()).is_err(),
        "a dead carried player"
    );
    let mut tic = st.clone();
    tic.display.tic += 1;
    assert!(matches!(
        tic.check(),
        Err(DsesError::Inconsistent("display tic"))
    ));
    assert!(Session::restore(data, &st, frozen).is_ok());
}

/// Runs `cmds` up to `upto` tics, then asserts that the next tic is refused with the session's
/// encoding unchanged, and returns the refusal.
fn refused_at(
    mut s: Session,
    cmds: &[TicCommand],
    upto: usize,
    cmd: Option<TicCommand>,
) -> SessionWitness {
    for (t, &c) in cmds.iter().enumerate().take(upto) {
        s.tic(c).unwrap_or_else(|e| panic!("tic {t}: {e}"));
    }
    let before = s.save().expect("save").encode();
    let e = s.tic(cmd.unwrap_or(cmds[upto])).expect_err("refused");
    assert_eq!(e.tic, upto as u64);
    assert_eq!(
        s.save().expect("save").encode(),
        before,
        "the refusal changed the session"
    );
    // And the session can still be saved and the same tic refused again.
    let again = s.tic(cmd.unwrap_or(cmds[upto])).expect_err("refused again");
    assert_eq!(again, e);
    e.witness
}

#[test]
fn refusals_leave_the_session_unchanged() {
    // The genuine stops (U10 (i)): D_INTER on the intermission's first tic, a later map's music
    // at its load tic.
    for (name, music, upto) in [("NOINTER", "d_inter", 56), ("NOE1M2", "d_e1m2", 142)] {
        let item = m6a::item(name);
        let data = m6a::data(&item, None);
        let w = refused_at(new_session(&item, data), &commands(&item), upto, None);
        assert!(
            matches!(&w, SessionWitness::Sound(SoundWitness::Chain(ChainRefusal::MusicMissing { music: m, tic: Some(t) })) if m.lump_name() == music && *t == upto as u64),
            "{name}: {w:?}"
        );
    }
    let chain = m6a::item("CHAIN");
    let cmds = commands(&chain);
    let wads = wad_bytes(&chain);
    // BT_SPECIAL in a replayed command (the original's pause, M6b): refused at any tic.
    let data = data_of(&chain, &wads, ModeSetting::Force(SessionMode::Registered));
    let pause = TicCommand {
        buttons: 0x81,
        ..TicCommand::default()
    };
    for upto in [0, 130, 141] {
        let w = refused_at(new_session(&chain, data.clone()), &cmds, upto, Some(pause));
        assert_eq!(w, SessionWitness::Special);
    }
    // The turbo message where gametic and leveltime disagree (E1M2's level tic 0 is session
    // tic 142, so session tic 256 is level tic 114) is not refused (owner's ruling 1 of
    // 2026-11-22: the level tests the session's gametic; the genuine comparison is the TURBO
    // item, tests/m6a_session.rs).
    let turbo = TicCommand {
        forward: 60,
        ..TicCommand::default()
    };
    let mut s = new_session(&chain, data.clone());
    for &c in &cmds[..256] {
        s.tic(c).expect("tic");
    }
    s.tic(turbo).expect("ruling 1: no refusal");
    // A later map that does not load at its load tic (E1M2's marker renamed).
    let mut broken = wads.clone();
    broken[0] = rename(broken[0].clone(), "E1M2", "E1MX");
    let data = data_of(&chain, &broken, ModeSetting::Force(SessionMode::Registered));
    let w = refused_at(new_session(&chain, data), &cmds, 142, None);
    assert!(
        matches!(
            w,
            SessionWitness::Map {
                map: 2,
                error: MapLoadError::Map(_)
            }
        ),
        "{w:?}"
    );
    // A later map that wire 26 does not admit at its load tic (one line special 255 on E1M2).
    let mut odd = wads.clone();
    let (e, _, _) = lump_index(&odd[0], "E1M2").expect("E1M2");
    let next = e + 32; // E1M2, THINGS, LINEDEFS
    let pos = i32::from_le_bytes(odd[0][next..next + 4].try_into().unwrap()) as usize;
    odd[0][pos + 6..pos + 8].copy_from_slice(&255u16.to_le_bytes());
    let data = data_of(&chain, &odd, ModeSetting::Force(SessionMode::Registered));
    let w = refused_at(new_session(&chain, data), &cmds, 142, None);
    assert!(
        matches!(
            w,
            SessionWitness::Admission { map: 2, .. } | SessionWitness::Map { map: 2, .. }
        ),
        "{w:?}"
    );
    eprintln!("E1M2 with special 255: {w:?}");
    // A missing intermission picture: WI_loadData stops at G_DoCompleted's WI_Start, before
    // WI_Ticker's music.
    let mut nopic = wads.clone();
    nopic[3] = rename(nopic[3].clone(), "WIMAP0", "XIMAP0");
    let data = data_of(&chain, &nopic, ModeSetting::Force(SessionMode::Registered));
    let w = refused_at(new_session(&chain, data), &cmds, 126, None);
    assert!(
        matches!(w, SessionWitness::Display(SessionError::Load(_))),
        "{w:?}"
    );
    // The E1M8 victory: F_StartFinale is built after the owner's hand test.
    let fin = m6a::item("FINREG");
    let data = m6a::data(&fin, None);
    let w = refused_at(new_session(&fin, data), &commands(&fin), 321, None);
    assert_eq!(w, SessionWitness::NotYet(NotYet::Victory));
}

#[test]
fn session_starts_that_are_refused() {
    let item = m6a::item("NOE1M2AT");
    let data = m6a::data(&item, None);
    let e = Session::new(
        data,
        SessionSettings {
            map: 2,
            skill: 2,
            screenblocks: 10,
        },
    )
    .expect_err("refused");
    assert!(
        matches!(
            &e,
            StartError::Sound(SoundWitness::Chain(ChainRefusal::MusicMissing {
                tic: None,
                ..
            }))
        ),
        "{e:?}"
    );
    let chain = m6a::item("CHAIN");
    let data = m6a::data(&chain, None);
    for (map, skill, sb, want) in [
        (0, 2, 10, StartError::Domain),
        (10, 2, 10, StartError::Domain),
        (1, 2, 12, StartError::Domain),
        (1, 4, 10, StartError::Domain),
        (1, 0, 10, StartError::NotYet(NotYet::EasySkill)),
        (1, 1, 10, StartError::NotYet(NotYet::EasySkill)),
    ] {
        let e = Session::new(
            data.clone(),
            SessionSettings {
                map,
                skill,
                screenblocks: sb,
            },
        )
        .expect_err("refused");
        assert_eq!(e, want, "map {map} skill {skill} sb {sb}");
    }
    // Every map of the synthetic episode starts (ruling U14: any map 1..=9).
    for map in 1..=9 {
        Session::new(
            data.clone(),
            SessionSettings {
                map,
                skill: 2,
                screenblocks: 11,
            },
        )
        .unwrap_or_else(|e| panic!("map {map}: {e:?}"));
    }
}

#[test]
fn the_declared_game_mode() {
    // The probe: retail exactly when the IWAD's own directory has E4M1 (ruling U4; linuxdoom
    // makes no such probe, d_main.c:563-717).
    assert_eq!(
        probe_game_mode(&iwad_with(&["E1M1", "E3M9"])).unwrap(),
        SessionMode::Registered
    );
    assert_eq!(
        probe_game_mode(&iwad_with(&["E1M1", "E4M1"])).unwrap(),
        SessionMode::Retail
    );
    assert_eq!(
        probe_game_mode(&iwad_with(&["E4M10", "E4M"])).unwrap(),
        SessionMode::Registered
    );
    assert!(probe_game_mode(b"PWAD").is_err());
    // The synthetic episode has no fourth episode: probed registered; forced either way.
    let chain = m6a::item("CHAIN");
    let wads = wad_bytes(&chain);
    let probed = data_of(&chain, &wads, ModeSetting::Probe);
    assert_eq!(
        (probed.mode().mode, probed.mode().source),
        (SessionMode::Registered, ModeSource::Probed)
    );
    let forced = data_of(&chain, &wads, ModeSetting::Force(SessionMode::Retail));
    assert_eq!(
        (forced.mode().mode, forced.mode().source),
        (SessionMode::Retail, ModeSource::Forced)
    );
    // A later WAD with E4M1 does not change the probe (the IWAD alone is looked at).
    let mut with_e4 = wads.clone();
    with_e4.push(iwad_with(&["E4M1"]));
    assert_eq!(
        data_of(&chain, &with_e4, ModeSetting::Probe).mode().mode,
        SessionMode::Registered
    );
    // The file-name token next to the unchanged single-level name.
    let t = Timestamp {
        year: 2026,
        month: 11,
        day: 21,
        hour: 23,
        minute: 5,
        second: 9,
    };
    let name = session_mode_name(3, 2, SessionMode::Retail, t);
    assert_eq!(name, "E1M3-skill2-retail-20261121-230509.lmp");
    assert_eq!(
        doom_game::app::session_name(3, 2, t),
        "E1M3-skill2-20261121-230509.lmp"
    );
    assert_eq!(mode_of_session_name(&name), Some(SessionMode::Retail));
    assert_eq!(
        mode_of_session_name("dir/E1M1-skill3-registered-20261121-230509.lmp"),
        Some(SessionMode::Registered)
    );
    assert_eq!(
        mode_of_session_name("E1M3-skill2-20261121-230509.lmp"),
        None
    );
    assert_eq!(replay_mode(&name, None), Ok(SessionMode::Retail));
    assert_eq!(
        replay_mode(&name, Some(SessionMode::Retail)),
        Ok(SessionMode::Retail)
    );
    assert_eq!(
        replay_mode(&name, Some(SessionMode::Registered)),
        Err(ModeTokenError::Disagrees {
            token: SessionMode::Retail,
            flag: SessionMode::Registered
        })
    );
    assert_eq!(
        replay_mode("E1M3-skill2-20261121-230509.lmp", None),
        Err(ModeTokenError::Missing)
    );
}

/// The session demo: the header's map is the start, the commands run across every boundary, the
/// bytes round-trip through the recorder's writer, and `replay` runs the same session layer.
#[test]
fn the_session_demo_across_levels() {
    let chain = m6a::item("CHAIN");
    let lmp = std::fs::read(m6a::root(&chain.lmp)).expect("demo");
    let demo = SessionDemo::parse(&lmp).expect("parse");
    assert_eq!((demo.map, demo.skill, demo.commands.len()), (1, 2, 831));
    assert_eq!(
        demo.bytes().expect("write"),
        lmp,
        "the recorder's writer gives the same file"
    );
    let start7 =
        SessionDemo::parse(&std::fs::read(m6a::root(&m6a::item("INTFAST").lmp)).unwrap()).unwrap();
    assert_eq!(start7.map, 7);
    let mut bad = lmp.clone();
    bad[3] = 10;
    assert!(SessionDemo::parse(&bad).is_err(), "map 10");
    let data = m6a::data(&chain, None);
    let mut s = new_session(&chain, data);
    let mut loads = 0;
    let end = replay(&mut s, &demo.commands, |_, r| {
        loads += usize::from(matches!(r.kind, TicKind::Load { .. }))
    });
    assert_eq!((end, loads, s.gametic(), s.map()), (None, 7, 831, 8));
    // A replayed BT_SPECIAL ends the replay with a refusal at its tic.
    let mut cmds = demo.commands.clone();
    cmds[200].buttons |= 0x80;
    let mut s = new_session(&chain, m6a::data(&chain, None));
    let end = replay(&mut s, &cmds, |_, _| {});
    assert_eq!(
        end.map(|e| (e.tic, e.witness)),
        Some((200, SessionWitness::Special))
    );
}

/// Owner's ruling 2 of 2026-11-22: a command is judged by what reads it. On a level tic (CHAIN
/// tic 100) and a load tic (tic 142, E1M2's) a command outside wire 24's domain is refused; on
/// the completion tic (126) and every later intermission tic (127 to 141) the same kind of
/// command is accepted and ignored but for bits 0 and 1 of its buttons
/// (`WI_checkForAccelerate`, wi_stuff.c:1470-1498): CHAIN with each intermission command
/// replaced by one with a forward of 100, a side of -128, a turn with a low byte and the buttons
/// bits 2 to 6 set (its press bits kept) gives byte-equal saved sessions and frames on every tic
/// from the first intermission tic to the end of the demo. `BT_SPECIAL` stays refused in the
/// intermission (`refusals_leave_the_session_unchanged`, tics 130 and 141).
#[test]
fn the_command_domain_follows_the_reader() {
    let chain = m6a::item("CHAIN");
    let cmds = commands(&chain);
    let data = data_of(
        &chain,
        &wad_bytes(&chain),
        ModeSetting::Force(SessionMode::Registered),
    );
    let odd = |c: TicCommand| TicCommand {
        forward: 100,
        side: i8::MIN,
        angle_turn: 0x1234,
        buttons: (c.buttons & 3) | 0x7c,
    };
    for upto in [100, 142] {
        let w = refused_at(
            new_session(&chain, data.clone()),
            &cmds,
            upto,
            Some(odd(TicCommand::default())),
        );
        assert_eq!(w, SessionWitness::Command, "tic {upto}");
    }
    let (mut plain, mut bent) = (
        new_session(&chain, data.clone()),
        new_session(&chain, data.clone()),
    );
    let mut intermission = 0;
    for (t, &c) in cmds.iter().enumerate() {
        let in_wi = (126..142).contains(&t);
        plain.tic(c).expect("tic");
        bent.tic(if in_wi { odd(c) } else { c })
            .unwrap_or_else(|e| panic!("tic {t}: {e}"));
        intermission += usize::from(in_wi);
        if t >= 126 {
            assert_eq!(
                sha256(bent.frame().pixels()),
                sha256(plain.frame().pixels()),
                "tic {t}: frame"
            );
            if t <= 160 || t + 1 == cmds.len() {
                assert_eq!(
                    bent.save().expect("save").encode(),
                    plain.save().expect("save").encode(),
                    "tic {t}: saved session"
                );
            }
        }
    }
    assert_eq!(intermission, 16);
}

/// `DSES` restore checks the level's `level.start.gametic` (wire 26, owner's ruling 1 of
/// 2026-11-22) against the sound's `levelstarttic` (`levelstarttic = gametic`, g_game.c:470):
/// INTFAST's first level (E1M7, a new game at session tic 0) rebuilt at gametic 128 and run with
/// the same 30 commands has a capture that differs in that field alone, and its restore is
/// refused.
#[test]
fn dses_refuses_a_start_gametic_that_is_not_levelstarttic() {
    let (data, _wi, _frozen, lv, level) = intfast_states();
    let st = SessionState::decode(&lv).expect("decode");
    assert!(Session::restore(data.clone(), &st, level.clone()).is_ok());
    let item = m6a::item("INTFAST");
    let wads = wad_bytes(&item);
    let wad = doom_wad::Wad::parse(&wads[0], doom_wad::Limits::default()).expect("wad");
    let map = doom_map::Map::from_wad(&wad, "E1M7", doom_map::MapLimits::default()).expect("map");
    let mut sim = doom_sim::Simulation::original_player_episode1_v4_level_at(
        map,
        7,
        doom_sim::Skill::Medium,
        doom_sim::MapContextOptions::default(),
        doom_sim::LevelStart::NewGame,
        128,
    )
    .expect("level");
    let none = TicCommand::default();
    for &c in &commands(&item)[..30] {
        sim.advance([c, none, none, none]).expect("tic");
    }
    assert_eq!(level.level_start_gametic(), Some(0));
    assert_eq!(sim.level_start_gametic(), Some(128));
    let differing: Vec<String> = level
        .snapshot()
        .fields()
        .zip(sim.snapshot().fields())
        .filter(|(a, b)| a.value() != b.value())
        .map(|(a, _)| a.path().to_owned())
        .collect();
    assert_eq!(differing, ["level.start.gametic"]);
    let mut forged = st.clone();
    forged.level = sim.snapshot().canonical_bytes();
    assert!(matches!(
        Session::restore(data, &forged, sim),
        Err(DsesError::Inconsistent(
            "the level's start gametic and levelstarttic"
        ))
    ));
}
