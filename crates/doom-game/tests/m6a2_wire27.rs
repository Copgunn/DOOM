//! The finale-and-melt session runs on wire 27 (`PlayerMonstersEpisode1V5`, captures DSIM43;
//! owner's ruling of 2026-11-24 for M6a part 2), the frozen session on wire 26: the eight
//! synthetic wrap items of wire 27 driven through both sessions and compared with the genuine
//! engine's state after every tic.
//!
//! The goldens are `tests/reference/wire27_state.txt` (`tools/reference/wire27_goldens.py`: the
//! genuine linuxdoom-1.10 strict build and the independent DSIM43 encoder; no Rust output feeds
//! them), the inputs the committed WAD set and demos of wire 27. The digest is
//! `crates/doom-sim/tests/wire27_intercept.rs`'s: SHA-256 of the DSIM43 capture without `map.*`
//! and `events.*`, then its `events.sounds.*` fields. Here the new session's captures are DSIM43
//! themselves, so nothing is renamed for it.
//!
//! - **New session**: every tic of every item runs (each with its melt), and the level's capture
//!   after it equals the genuine one, past the tic at which wire 26 refuses.
//! - **Frozen session**: equal up to that tic (its DSIM42 captures renamed to DSIM43), which it
//!   refuses with `MapScale(Arithmetic)` and the state before it kept: part 1 is unchanged.
//! - **`DSES` version 2 embeds DSIM43**: a state saved after the wrap restores and saves the same
//!   bytes; the same state with the frozen session's DSIM42 capture in its place is refused.
//! - **Self-test**: an altered golden digest is reported at its capture.
use doom_game::session::demo::SessionDemo;
use doom_game::session::state::DsesError;
use doom_game::session::state_v2::SessionStateV2;
use doom_game::session::{
    ModeSetting, Session, SessionData, SessionMode, SessionSettings, SessionWitness,
};
use doom_render::Frame;
use doom_sim::{MapScaleError, PLAYER_EPISODE1_V5_CONTEXT_SCHEMA, SimError, Snapshot, TicCommand};
use doom_test_evidence::sha256;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn reference(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(path)
}

/// One item of the golden: its map slot, the session tic wire 26 refuses and the genuine
/// digest after every tic (capture `j` is the state after `j` tics).
struct Item {
    name: String,
    map: u8,
    wire26_refusal: u64,
    captures: Vec<String>,
}

fn golden() -> Vec<Item> {
    let text = std::fs::read_to_string(reference("wire27_state.txt")).expect("golden");
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    assert_eq!(lines.next(), Some("#schema,DOOM-WIRE27-STATE-v1"));
    let mut items: Vec<Item> = Vec::new();
    for line in lines {
        let f: Vec<&str> = line.split(',').collect();
        match f[0] {
            "I" => {
                assert_eq!(f[6], "complete", "{line}");
                items.push(Item {
                    name: f[1].to_owned(),
                    map: f[2].parse().expect(line),
                    wire26_refusal: f[5].parse().expect(line),
                    captures: Vec::new(),
                });
            }
            "C" => {
                let item = items.last_mut().expect("an I row first");
                assert_eq!(
                    (item.name.as_str(), item.captures.len()),
                    (f[1], f[2].parse().expect(line))
                );
                item.captures.push(f[3].to_owned());
            }
            _ => {}
        }
    }
    items
}

fn framed(out: &mut Vec<u8>, path: &str, value: &[u8]) {
    out.extend((path.len() as u32).to_le_bytes());
    out.extend(path.as_bytes());
    out.extend((value.len() as u64).to_le_bytes());
    out.extend(value);
}

/// The state digest of a DSIM43 capture; a DSIM42 capture (the frozen session's) is first
/// renamed to DSIM43 as `wire27_intercept.rs` does.
fn digest(s: &Snapshot) -> String {
    let rename = s.version() == 42;
    assert!(rename || s.version() == 43, "a wire 26 or 27 capture");
    let mut body = b"DSIM".to_vec();
    body.extend(43_u32.to_le_bytes());
    let kept = |p: &str| !p.starts_with("map.") && !p.starts_with("events.");
    body.extend((s.fields().filter(|f| kept(f.path())).count() as u64).to_le_bytes());
    for f in s.fields().filter(|f| kept(f.path())) {
        match (rename, f.path()) {
            (true, "profile") => framed(&mut body, f.path(), &27_u32.to_le_bytes()),
            (true, "context.schema") => {
                framed(&mut body, f.path(), PLAYER_EPISODE1_V5_CONTEXT_SCHEMA);
            }
            _ => framed(&mut body, f.path(), f.value()),
        }
    }
    for f in s
        .fields()
        .filter(|f| f.path().starts_with("events.sounds."))
    {
        framed(&mut body, f.path(), f.value());
    }
    sha256(&body)
}

/// The session data of wire 27's WAD set (`tools/reference/wire27_wad.py`'s `WADSET`) and the
/// item's commands.
fn load(item: &Item) -> (Arc<SessionData>, SessionSettings, Vec<TicCommand>) {
    let files: Vec<Vec<u8>> = [
        "wire27_wads/wire27.wad",
        "m5b_gfx.wad",
        "m5c_sfx.wad",
        "m6a_wads/m6a_gfx.wad",
        "m6a_wads/m6a_music.wad",
    ]
    .iter()
    .map(|w| std::fs::read(reference(w)).expect("WAD"))
    .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let lmp = std::fs::read(reference(&format!("wire27_scripts/{}.lmp", item.name))).expect("demo");
    let demo = SessionDemo::parse(&lmp).expect("demo parse");
    assert_eq!((demo.map, demo.skill), (item.map, 2), "{}", item.name);
    let data = SessionData::load_with(&refs, &[], ModeSetting::Force(SessionMode::Retail))
        .expect("session data");
    let settings = SessionSettings {
        map: item.map,
        skill: 2,
        screenblocks: 10,
    };
    assert_eq!(
        demo.commands.len() + 1,
        item.captures.len(),
        "{}",
        item.name
    );
    (Arc::new(data), settings, demo.commands)
}

/// One whole tic of the new session: the tic, then every step of its melt.
fn whole_tic(s: &mut Session, cmd: TicCommand) -> Result<(), SessionWitness> {
    s.tic_finale_and_melt(cmd).map_err(|e| e.witness)?;
    let mut picture = Frame::new();
    while s.melt_in_progress() {
        s.melt_step(&mut picture).expect("melt step");
    }
    Ok(())
}

/// Drives the new session and returns the first capture that differs from `expected`.
fn first_difference(item: &Item, expected: &[String]) -> Option<usize> {
    let (data, settings, commands) = load(item);
    let mut s = Session::new_finale_and_melt(data, settings).expect("the new session starts");
    if digest(&s.level().snapshot()) != expected[0] {
        return Some(0);
    }
    for (tic, &cmd) in commands.iter().enumerate() {
        whole_tic(&mut s, cmd).unwrap_or_else(|w| panic!("{} tic {tic}: {w}", item.name));
        assert_eq!(s.level().snapshot().version(), 43, "{}", item.name);
        if digest(&s.level().snapshot()) != expected[tic + 1] {
            return Some(tic + 1);
        }
    }
    None
}

#[test]
fn the_new_session_equals_the_genuine_engine_past_wire_26s_refusal() {
    let items = golden();
    assert_eq!(items.len(), 8);
    let mut tics = 0;
    for item in &items {
        assert_eq!(
            first_difference(item, &item.captures),
            None,
            "{}",
            item.name
        );
        assert!((item.wire26_refusal as usize) < item.captures.len() - 1);
        tics += item.captures.len() - 1;
    }
    eprintln!("wire 27 through the finale-and-melt session: 8 items, {tics} tics equal");
    assert_eq!(tics, 261);
}

#[test]
fn the_frozen_session_still_refuses_at_the_first_wrap() {
    for item in &golden() {
        let (data, settings, commands) = load(item);
        let mut s = Session::new(data, settings).expect("the frozen session starts");
        assert_eq!(digest(&s.level().snapshot()), item.captures[0]);
        let mut refused = None;
        for (tic, &cmd) in commands.iter().enumerate() {
            if let Err(e) = s.tic(cmd) {
                refused = Some((e.tic, e.witness));
                break;
            }
            assert_eq!(
                digest(&s.level().snapshot()),
                item.captures[tic + 1],
                "{} {tic}",
                item.name
            );
        }
        let arithmetic: SimError = MapScaleError::Arithmetic.into();
        assert_eq!(
            refused,
            Some((item.wire26_refusal, SessionWitness::Simulation(arithmetic))),
            "{}",
            item.name
        );
        let kept = &item.captures[item.wire26_refusal as usize];
        assert_eq!(
            &digest(&s.level().snapshot()),
            kept,
            "{}: state kept",
            item.name
        );
        assert_eq!(s.level().snapshot().version(), 42, "{}", item.name);
    }
}

#[test]
fn dses_version_2_embeds_dsim43_and_refuses_dsim42() {
    for item in &golden() {
        let (data, settings, commands) = load(item);
        let mut new = Session::new_finale_and_melt(data.clone(), settings).expect("new");
        let mut frozen = Session::new(data.clone(), settings).expect("frozen");
        // One tic past the wrap in the new session; the frozen one stops before it.
        for &cmd in &commands[..=item.wire26_refusal as usize] {
            whole_tic(&mut new, cmd).expect("the new session runs it");
        }
        for &cmd in &commands[..item.wire26_refusal as usize] {
            frozen.tic(cmd).expect("before the wrap");
        }
        let state = new.save_v2().expect("saveable");
        assert_eq!(&state.level[..8], b"DSIM\x2b\0\0\0", "{}", item.name);
        let bytes = state.encode();
        let decoded = SessionStateV2::decode(&bytes).expect("decodes");
        let restored = Session::restore_v2(data.clone(), &decoded, new.level().clone())
            .unwrap_or_else(|e| panic!("{}: restore {e}", item.name));
        assert_eq!(restored.save_v2().expect("saveable").encode(), bytes);
        // The same state holding a wire-26 capture: refused by the reader's check, and a
        // wire-26 level is refused for a wire-27 state.
        let mut swapped = state.clone();
        swapped.level = frozen.level().snapshot().canonical_bytes();
        assert_eq!(&swapped.level[..8], b"DSIM\x2a\0\0\0");
        assert!(
            matches!(
                SessionStateV2::decode(&swapped.encode()),
                Err(DsesError::Level(_))
            ),
            "{}: a DSIM42 capture in DSES version 2",
            item.name
        );
        assert!(matches!(
            Session::restore_v2(data, &decoded, frozen.level().clone()),
            Err(DsesError::LevelMismatch)
        ));
    }
}

#[test]
fn an_altered_golden_digest_is_reported_at_its_capture() {
    let items = golden();
    let item = items.iter().find(|i| i.name == "USE").expect("USE");
    for at in [0, item.wire26_refusal as usize + 1, item.captures.len() - 1] {
        let mut bent = item.captures.clone();
        let flipped = if bent[at].starts_with('0') { "1" } else { "0" };
        bent[at].replace_range(..1, flipped);
        assert_eq!(first_difference(item, &bent), Some(at));
    }
}
