//! The victory-tic display stops against the genuine record of
//! `tools/reference/m6a2_victory_stops.py` (FINREG's demo with lumps renamed, as here).
#![allow(clippy::unwrap_used, reason = "test code")]

mod m6a;
mod m6a2_run;
#[path = "../../doom-sound/tests/common/rows.rs"]
mod rows;

use doom_game::app::*;
use doom_game::session::demo::{SessionDemo, replay_finale_and_melt};
use doom_game::session::{ModeSetting, Refusal, Session, SessionWitness, StartError};
use doom_render::{FinaleView, GameState, ResourceError::LumpNotFound, f_finale::DrawRefusal};
use doom_test_evidence::sha256;
use serde_json::Value;

/// The record of `name`; FINREG's WAD set with names starting `renamed` given '_' first.
fn load(name: &str, renamed: &str) -> (Value, Result<GameData, LoadError>) {
    let text = std::fs::read_to_string(m6a::root("tests/reference/m6a2_victory_stops/stops.json"));
    let rec = serde_json::from_str::<Value>(&text.unwrap()).unwrap()["items"][name].clone();
    let mut files = Vec::new();
    for w in m6a::item("FINREG").wads {
        let mut b = std::fs::read(m6a::root(&w)).unwrap();
        let at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize;
        let (n, off) = (at(4), at(8));
        for o in (0..n).map(|k| off + 16 * k + 8).filter(|_| w == rec["wad"]) {
            if b[o..o + 8].starts_with(renamed.as_bytes()) {
                b[o] = b'_';
            }
        }
        assert!(w != rec["wad"] || sha256(&b) == rec["wad_sha256"], "{name}");
        files.push(b);
    }
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let data = GameData::load_finale_and_melt_session(&refs, ModeSetting::Probe, 8, 2);
    (rec, data)
}

#[derive(Default)]
struct H(std::collections::BTreeMap<String, Vec<u8>>);
impl Host for H {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp::default()
    }
    fn exists(&mut self, name: &str) -> bool {
        self.0.contains_key(name)
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.0.insert(name.into(), bytes.to_vec());
        Ok(())
    }
}

/// No `FLOOR4_8`: the app, played by keys (up arrow: a forward move of 25), ends at the genuine
/// stop with the genuine text, the demo holding the stop tic's command (read by the genuine
/// `G_Ticker` before `D_Display` stopped, g_game.c:656-669, d_main.c:383-395) and replaying to it,
/// every tic before it FINREG's genuine G row and frame (the record: the genuine NOFLAT run's).
#[test]
fn without_the_flat_the_victory_tic_ends_the_session_as_the_genuine_engine() {
    let (rec, data) = load("NOFLAT", "FLOOR4_8");
    let cmds = m6a2_run::commands(&m6a::item("FINREG")).unwrap();
    let (mut h, mut held, mut k) = (H::default(), false, 0);
    let mut app = App::new(data.unwrap(), SessionConfig::default(), &mut h);
    app.update(0, &mut h);
    while app.mode() == &Mode::Playing && k < 2000 {
        let want = cmds[app.session().unwrap().gametic() as usize].forward != 0;
        if want != held {
            app.key(doom_game::input::keys::KEY_UPARROW, want, &mut h);
            held = want;
        }
        k += 1;
        app.update((k * 1_000_000u64).div_ceil(35), &mut h);
    }
    let tic = rec["stop_tic"].as_u64().unwrap();
    let w = SessionWitness::FinaleText(DrawRefusal::Lump(LumpNotFound("FLOOR4_8".into())));
    let witness = Witness::Session(Box::new(w.clone()));
    assert_eq!(app.mode(), &Mode::Ended(Ending::Refused { tic, witness }));
    let stop = rec["stop"].as_str().unwrap();
    let text =
        format!("the finale text screen cannot be drawn ({stop}). The original stops here too.");
    let s = app.session().unwrap();
    assert_eq!(s.finale(), Some(FinaleView { stage: 0, count: 1 }));
    assert_eq!((s.gamestate(), s.gametic()), (GameState::Finale, tic));
    assert_eq!(app.finale_began(), Some(tic));
    assert_eq!(app.art_screen_began().or(app.art_screen_refused()), None);
    let summary = format!(
        "session: game mode registered (the IWAD has no fourth episode), started at \
        E1M8 skill 2, ended in the finale (after E1M8) after {} tics; the finale began at session tic \
        {tic}, its art screen had not begun; refused at session tic {tic}: {text}",
        tic + 1
    );
    assert_eq!(app.session_summary().unwrap(), summary);
    let demo = SessionDemo::parse(&h.0[app.session_name()]).unwrap();
    assert_eq!(demo.commands, cmds[..=tic as usize]);
    let rows = m6a::Rows::read("m6a_session/FINREG.session");
    let frames = m6a2_run::read_frames("m6a_frames/FINREG.sb10.frames").0;
    let mut fresh = Session::new_finale_and_melt(s.data().clone(), s.settings()).unwrap();
    let mut n = 0;
    let on_tic = |s: &Session, r: &doom_game::session::finale_melt::FinaleMeltTicReport| {
        let (t, f) = (r.tic as i64, &frames[&(r.tic as i64)]);
        let g = |c| m6a::int(rows.at("G", t).unwrap(), c);
        let rnd = [s.level().rng().indices()[0], s.display().random_index()].map(i64::from);
        assert_eq!(rnd, [g("prndindex"), g("rndindex")]);
        assert_eq!(s.level().tic() as i64, g("leveltime"));
        let got = (sha256(s.frame().pixels()), i64::from(r.palette));
        assert_eq!(got, (f.sha256.clone(), f.status[0]));
        n += 1;
    };
    let end = replay_finale_and_melt(&mut fresh, &demo.commands, on_tic, |_, _, _| {});
    assert_eq!((end, n), (Some(Refusal { tic, witness: w }), tic));
    assert_eq!(rec["tics_run"], tic);
    app.key(b'q', true, &mut h);
    assert!(app.quit_requested() && h.0.len() == 1);
    app.key(b'r', true, &mut h);
    assert_eq!(app.mode(), &Mode::Playing);
}

/// A glyph of E1TEXT, or the whole font, missing: `HU_Init` stops the genuine engine before tic 0
/// (hu_stuff.c:404-410); the session does not start either, with the genuine text.
#[test]
fn without_a_font_glyph_both_stop_before_tic_0() {
    for (name, renamed) in [("NOGLYPH", "STCFN079"), ("NOFONT", "STCFN")] {
        let (rec, data) = load(name, renamed);
        assert!(rec["stop_tic"] == -1 && rec["tics_run"] == 0);
        let e = data.err().unwrap();
        assert!(matches!(e, LoadError::SessionStart(StartError::Display(_))));
        let want = format!(
            "the display could not be built: {}",
            rec["stop"].as_str().unwrap()
        );
        assert_eq!(e.to_string(), want);
    }
}
