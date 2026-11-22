//! M6a step 9: the app plays the session (owner's rulings U2, U4–U7; M6a note §4.10). The app
//! ([`App`] over [`GameData::load_session`], doom-play's game) is driven headlessly by key events
//! made from the committed synthetic scripts (`tests/reference/m6a_scripts/*.txt`: a forward move
//! of 25 is the up arrow held, `USE` the space bar held), the way a player at the window would.
//!
//! Three comparisons, each shown able to fail by a planted fault:
//!
//! 1. **The recorded demo is the genuine item's.** The session file the app writes is byte for
//!    byte the item's committed demo (REBHELD, whole) or its first tics (CHAIN up to its first
//!    turn, which no key gives; FINREG and NOE1M2 up to their refusals), so the genuine engine
//!    ran exactly these commands (`tools/reference/m6a_goldens.py`) and the level-chain gate
//!    (`tests/m6a_session.rs`) compares their replay in full.
//! 2. **The live app against the genuine engine**, every tic at screenblocks 10: the picture the
//!    window shows (`App::frame`, its SHA-256 and palette) against the genuine frames golden, and
//!    `gamestate`, `map`, `leveltime`, `gametic` and both random indices against the genuine G
//!    rows.
//! 3. **The demo replays through the session to the same end**: a fresh `Session` replaying the
//!    saved file reports every tic as the live app did (frame, palette, wipe, kind, the `M_Random`
//!    index, the sound tic) and ends at the player's quit with the same `DSES` v1 bytes, or at the
//!    same refusal.
#![allow(clippy::unwrap_used, reason = "test code")]

mod m6a;

use doom_game::app::{
    App, AudioSink, Ending, GameData, Host, LoadError, Mode, RecordingSink, SessionConfig,
    SessionTicReport, SinkCall, Timestamp, Witness,
};
use doom_game::input::keys;
use doom_game::session::demo::{SessionDemo, mode_of_session_name, replay_mode};
use doom_game::session::{
    ModeSetting, NotYet, PlantedFault, Session, SessionMode, SessionSettings, SessionWitness,
    StartError, TicKind,
};
use doom_render::GameState;
use doom_sound::chain::ChainTic;
use doom_test_evidence::sha256;
use m6a::{Item, Rows, int};
use std::collections::BTreeMap;

/// What one session tic reported, live or replayed.
#[derive(Clone, Debug, PartialEq)]
struct Seen {
    tic: u64,
    kind: TicKind,
    frame: String,
    palette: u8,
    wiped: bool,
    gamestate: i64,
    map: u8,
    leveltime: u64,
    prndindex: u8,
    rndindex: u8,
    sound: ChainTic,
}

fn seen(s: &Session, r: &doom_game::session::TicReport, frame: &[u8]) -> Seen {
    Seen {
        tic: r.tic,
        kind: r.kind,
        frame: sha256(frame),
        palette: r.palette,
        wiped: r.wiped,
        gamestate: match s.gamestate() {
            GameState::Level => 0,
            GameState::Intermission => 1,
            GameState::Finale => 2,
        },
        map: s.map(),
        leveltime: s.level().tic(),
        prndindex: s.level().rng().indices()[0],
        rndindex: s.display().random_index(),
        sound: r.sound.clone(),
    }
}

#[derive(Default)]
struct PlayHost {
    files: BTreeMap<String, Vec<u8>>,
    sink: RecordingSink,
    seen: Vec<Seen>,
}

impl Host for PlayHost {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp {
            year: 2026,
            month: 11,
            day: 22,
            hour: 12,
            minute: 0,
            second: 0,
        }
    }
    fn exists(&mut self, name: &str) -> bool {
        self.files.contains_key(name)
    }
    fn save(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.files.insert(name.to_string(), bytes.to_vec());
        Ok(())
    }
    fn audio(&mut self) -> Option<&mut dyn AudioSink> {
        Some(&mut self.sink)
    }
    fn session_tic(&mut self, r: &SessionTicReport<'_>) {
        // the window's picture source is checked by the driver (App::frame); this is the session's
        self.seen
            .push(seen(r.session, r.report, r.session.frame().pixels()));
    }
}

/// The keys a script holds in each tic: (up arrow, space), up to its first command no key gives.
fn script_keys(item: &Item) -> Vec<(bool, bool)> {
    let txt = item.lmp.replace(".lmp", ".txt");
    let text = std::fs::read_to_string(m6a::root(&txt)).unwrap();
    let mut out = Vec::new();
    for l in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = l.split_whitespace().collect();
        let held = match f.as_slice() {
            ["map", _] => continue,
            ["idle", n] => (n.parse::<usize>().unwrap(), (false, false)),
            ["cmd", n, fwd, "0", "0", b]
                if matches!(*fwd, "0" | "25") && matches!(*b, "0" | "USE") =>
            {
                (n.parse().unwrap(), (*fwd == "25", *b == "USE"))
            }
            _ => break,
        };
        out.extend(std::iter::repeat_n(held.1, held.0));
    }
    out
}

/// The app at the window's tic rate, one tic per update, with key events between tics.
struct Driver {
    app: App,
    host: PlayHost,
    clock: u64,
    base: u64,
    held: (bool, bool),
    /// The window's picture (SHA-256 of `App::frame`, its palette) after each tic.
    shown: Vec<(String, u8)>,
}

fn at(k: u64) -> u64 {
    (k * 1_000_000).div_ceil(35)
}

impl Driver {
    fn new(data: GameData, screenblocks: u8) -> Self {
        let mut host = PlayHost::default();
        let cfg = SessionConfig {
            screenblocks,
            ..SessionConfig::default()
        };
        let mut app = App::new(data, cfg, &mut host);
        assert_eq!(app.update(0, &mut host).ran, 0);
        Self {
            app,
            host,
            clock: 0,
            base: 0,
            held: (false, false),
            shown: Vec::new(),
        }
    }

    fn set(&mut self, want: (bool, bool)) {
        if want.0 != self.held.0 {
            self.app.key(keys::KEY_UPARROW, want.0, &mut self.host);
        }
        if want.1 != self.held.1 {
            self.app.key(b' ', want.1, &mut self.host);
        }
        self.held = want;
    }

    /// Plays `script` until it ends or the session ends.
    fn play(&mut self, script: &[(bool, bool)]) {
        for &want in script {
            if self.app.mode() != &Mode::Playing {
                break;
            }
            self.set(want);
            self.clock += 1;
            let ran = self
                .app
                .update(self.base + at(self.clock), &mut self.host)
                .ran;
            if ran == 1 {
                let (px, palette) = self.app.frame().unwrap();
                self.shown.push((sha256(px), palette));
            }
        }
        self.set((false, false));
    }

    fn demo(&self) -> Vec<u8> {
        self.host.files[self.app.session_name()].clone()
    }
}

fn load(item: &Item, setting: ModeSetting) -> GameData {
    let files: Vec<Vec<u8>> = item
        .wads
        .iter()
        .map(|w| std::fs::read(m6a::root(w)).unwrap())
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    GameData::load_session(&refs, setting, item.map, item.skill).unwrap()
}

/// Comparison 2: the live tics against the item's genuine G rows and screenblocks-10 frames; the
/// number of tics compared, or the first difference.
fn against_genuine(item: &Item, seen: &[Seen], shown: &[(String, u8)]) -> Result<usize, String> {
    let rows = Rows::read(&format!("m6a_session/{}.session", item.name));
    let (frames, _) = m6a::read_frames(&item.name, 10);
    assert_eq!(seen.len(), shown.len());
    for (s, (frame, palette)) in seen.iter().zip(shown) {
        let t = s.tic as i64;
        let d = |what: String| format!("{} tic {t}: {what}", item.name);
        let g = rows
            .at("G", t)
            .ok_or_else(|| d("no genuine G row".into()))?;
        let got = [
            ("gamestate", s.gamestate),
            ("map", i64::from(s.map)),
            ("leveltime", s.leveltime as i64),
            ("gametic", t + 1),
            ("prndindex", i64::from(s.prndindex)),
            ("rndindex", i64::from(s.rndindex)),
        ];
        for (name, v) in got {
            if int(g, name) != v {
                return Err(d(format!("G {name} {v} genuine {}", g[name])));
            }
        }
        let f = frames.get(&t).ok_or_else(|| d("no genuine frame".into()))?;
        if *frame != f.sha256 || &s.frame != frame {
            return Err(d("the picture differs from the genuine frame".into()));
        }
        if i64::from(*palette) != f.status[0] {
            return Err(d(format!("palette {palette} genuine {}", f.status[0])));
        }
    }
    Ok(seen.len())
}

/// Comparison 3: the saved demo replayed through a fresh session (over the app's own data);
/// returns the session after the last command and its end, or the first tic that differs.
fn replay(
    app: &App,
    demo: &[u8],
    live: &[Seen],
    fault: Option<PlantedFault>,
) -> Result<(Session, Option<doom_game::session::Refusal>), String> {
    let d = SessionDemo::parse(demo).map_err(|e| format!("{e:?}"))?;
    let live_session = app.session().unwrap();
    let settings = live_session.settings();
    assert_eq!((d.map, d.skill), (settings.map, settings.skill));
    let mut s = Session::new(live_session.data().clone(), settings).unwrap();
    if let Some(f) = fault {
        s.plant_fault(f);
    }
    let mut got = Vec::new();
    let end = doom_game::session::demo::replay(&mut s, &d.commands, |s, r| {
        got.push(seen(s, r, s.frame().pixels()));
    });
    if let Some(i) = (0..got.len().max(live.len())).find(|&i| got.get(i) != live.get(i)) {
        return Err(format!("replay tic {i} differs from the live app"));
    }
    Ok((s, end))
}

fn commands_of(bytes: &[u8]) -> Vec<doom_game::input::TicCommand> {
    SessionDemo::parse(bytes).unwrap().commands
}

/// REBHELD (start E1M2: the death, the reborn, the exit, the intermission) and CHAIN (start E1M1:
/// two exits, two intermissions, two carried starts with the chainsaw's load sound, to its first
/// turn), played by keys and ended by the player's quit.
#[test]
fn played_sessions_record_the_genuine_demos_and_replay_to_the_quit() {
    for (name, whole) in [("REBHELD", true), ("CHAIN", false)] {
        let item = m6a::item(name);
        let data = load(&item, ModeSetting::Probe);
        assert_eq!(data.session_mode().unwrap().mode, SessionMode::Registered);
        let mut d = Driver::new(data, 10);
        let script = script_keys(&item);
        assert_eq!(script.len() == item.session_tics, whole, "{name}");
        d.play(&script);
        assert_eq!(d.app.mode(), &Mode::Playing, "{name}: no refusal");
        d.app.quit(&mut d.host);
        assert!(d.app.quit_requested());
        // 1: the demo is the genuine item's
        let name_on_disk = d.app.session_name().to_string();
        assert_eq!(
            name_on_disk,
            format!("E1M{}-skill2-registered-20261122-120000.lmp", item.map)
        );
        assert_eq!(
            mode_of_session_name(&name_on_disk),
            Some(SessionMode::Registered)
        );
        assert_eq!(
            replay_mode(&name_on_disk, None),
            Ok(SessionMode::Registered)
        );
        let genuine = std::fs::read(m6a::root(&item.lmp)).unwrap();
        let demo = d.demo();
        if whole {
            assert_eq!(demo, genuine, "{name}: the session file is the item's demo");
        } else {
            assert_eq!(demo[..13], genuine[..13], "{name}: header");
            assert_eq!(commands_of(&demo), commands_of(&genuine)[..script.len()]);
        }
        // 2: the live app against the genuine engine
        let n = against_genuine(&item, &d.host.seen, &d.shown).unwrap();
        assert_eq!(n, script.len());
        // 3: the replay through the session, to the same quit
        let (s, end) = replay(&d.app, &demo, &d.host.seen, None).unwrap();
        assert_eq!(end, None, "{name}: the replay ends at the demo's end");
        let live = d.app.session().unwrap();
        assert_eq!(
            s.save().unwrap().encode(),
            live.save().unwrap().encode(),
            "{name}: the same DSES v1 at the quit"
        );
        let kinds = |k: fn(&TicKind) -> bool| d.host.seen.iter().filter(|s| k(&s.kind)).count();
        let loads = kinds(|k| matches!(k, TicKind::Load { .. }));
        let completions = kinds(|k| *k == TicKind::Completion);
        // the exit line names the mode and where the session was (from the genuine last G row)
        let rows = Rows::read(&format!("m6a_session/{name}.session"));
        let last = rows.at("G", script.len() as i64 - 1).unwrap();
        let at = match int(last, "gamestate") {
            1 => format!("in the intermission after E1M{}", last["map"]),
            _ => format!("in E1M{}", last["map"]),
        };
        assert_eq!(
            d.app.session_summary().unwrap(),
            format!(
                "session: game mode registered (the IWAD has no fourth episode), started at \
                 E1M{} skill 2, ended {at} after {} tics",
                item.map,
                script.len()
            )
        );
        assert_eq!(d.host.sink.calls[0], SinkCall::SessionStart);
        assert!(!d.host.sink.buffers.is_empty());
        eprintln!(
            "{name}: {} tics played by keys and compared live (frames, G rows), replayed equal; \
             {loads} loads, {completions} completions, {} buffers; {}",
            script.len(),
            d.host.sink.buffers.len(),
            d.app.session_summary().unwrap()
        );
        assert!(loads >= 1 && completions >= 1, "{name} crosses a boundary");
    }
}

/// Each new comparison can fail: a picture one pixel off at one tic and an `M_Random` index one
/// off are caught by the genuine comparison at that tic; a replay with a carried start one point
/// of health short (`PlantedFault::CarriedHealth`) differs from the live app at REBHELD's carried
/// load.
#[test]
fn the_play_comparisons_can_fail() {
    let item = m6a::item("REBHELD");
    let mut d = Driver::new(load(&item, ModeSetting::Probe), 10);
    d.play(&script_keys(&item));
    d.app.quit(&mut d.host);
    let carried = d
        .host
        .seen
        .iter()
        .find(|s| matches!(s.kind, TicKind::Load { start, .. } if start == doom_sim::StartKind::Carried))
        .unwrap()
        .tic;
    let mut shown = d.shown.clone();
    let mut frame = vec![0u8; 64_000];
    frame[7] = 1;
    shown[100].0 = sha256(&frame);
    let e = against_genuine(&item, &d.host.seen, &shown).unwrap_err();
    assert!(e.contains("tic 100: the picture"), "{e}");
    let mut seen = d.host.seen.clone();
    seen[200].rndindex = seen[200].rndindex.wrapping_add(1);
    let e = against_genuine(&item, &seen, &d.shown).unwrap_err();
    assert!(e.contains("tic 200: G rndindex"), "{e}");
    let e = replay(
        &d.app,
        &d.demo(),
        &d.host.seen,
        Some(PlantedFault::CarriedHealth),
    )
    .err()
    .unwrap();
    assert_eq!(e, format!("replay tic {carried} differs from the live app"));
    eprintln!(
        "planted: picture at tic 100, rndindex at tic 200, CarriedHealth at tic {carried}: caught"
    );
}

/// Restart from the pause screen (U6: S11's restart as an app action): a new game at the start
/// map with a fresh session and a new demo file, and the same keys then give the same tics.
#[test]
fn restart_from_the_pause_screen_is_a_new_game_and_a_new_demo() {
    let item = m6a::item("CHAIN");
    let script = script_keys(&item);
    let mut d = Driver::new(load(&item, ModeSetting::Probe), 10);
    d.play(&script[..200]);
    let first = d.host.seen.clone();
    let first_name = d.app.session_name().to_string();
    assert!(
        first
            .iter()
            .any(|s| matches!(s.kind, TicKind::Load { map: 2, .. })),
        "E1M2 loaded"
    );
    d.app.key(keys::KEY_ESCAPE, true, &mut d.host);
    assert_eq!(d.app.mode(), &Mode::Paused);
    d.app.key(keys::KEY_ESCAPE, false, &mut d.host);
    d.app.key(b'r', true, &mut d.host);
    d.app.key(b'r', false, &mut d.host);
    let calls = &d.host.sink.calls;
    assert_eq!(
        &calls[calls.len() - 2..],
        &[SinkCall::Resume, SinkCall::SessionStart]
    );
    assert_eq!(d.app.mode(), &Mode::Playing);
    let s = d.app.session().unwrap();
    assert_eq!(
        (s.gametic(), s.map(), s.gamestate()),
        (0, 1, GameState::Level)
    );
    assert_eq!(d.app.recorder().len(), 0);
    assert_eq!(d.app.session_name(), first_name.replace(".lmp", "-2.lmp"));
    assert_eq!(
        commands_of(&d.host.files[&first_name]).len(),
        200,
        "the first demo kept"
    );
    // the restarted session's clock starts at its first update
    d.base += at(d.clock) + 1;
    d.clock = 0;
    assert_eq!(d.app.update(d.base, &mut d.host).ran, 0);
    d.play(&script[..200]);
    assert_eq!(&d.host.seen[200..], &first[..]);
}

/// The refusals a player can meet end the session plainly and keep the demo, which replays to
/// the same refusal: the E1M8 victory (the finale is not built yet; FINREG, and FINRET in the
/// forced retail mode) at the genuine engine's first finale tic, and E1M2's missing music at its
/// load tic (NOE1M2, where the genuine engine stops). A start the session refuses is refused at
/// load with a plain message.
#[test]
fn refusals_are_reported_plainly_and_replay_to_the_same_end() {
    let yellow = [250, 210, 60, 255];
    let red = [235, 95, 95, 255];
    for (name, setting, not_yet) in [
        ("FINREG", ModeSetting::Probe, true),
        ("FINRET", ModeSetting::Force(SessionMode::Retail), true),
        ("NOE1M2", ModeSetting::Probe, false),
    ] {
        let item = m6a::item(name);
        let mut d = Driver::new(load(&item, setting), 11);
        d.play(&script_keys(&item));
        let Mode::Ended(Ending::Refused { tic, witness }) = d.app.mode().clone() else {
            panic!("{name}: {:?}", d.app.mode());
        };
        // the genuine engine's first finale tic, or its stop
        let stop = if not_yet {
            let rows = Rows::read(&format!("m6a_session/{name}.session"));
            (0..)
                .find(|&t| rows.at("G", t).is_some_and(|g| int(g, "gamestate") == 2))
                .unwrap() as u64
        } else {
            item.session_tics as u64
        };
        assert_eq!(tic, stop, "{name}");
        let Witness::Session(w) = &witness else {
            panic!("{name}: {witness:?}")
        };
        let text = witness.to_string();
        if not_yet {
            assert_eq!(**w, SessionWitness::NotYet(NotYet::Victory));
            assert!(text.starts_with("E1M8 is complete. The finale"), "{text}");
            assert!(text.contains("is not built yet"), "{text}");
        } else {
            assert!(matches!(**w, SessionWitness::Sound(_)), "{w:?}");
            assert_eq!(
                text,
                "the music lump D_E1M2 is missing: the original stops here too"
            );
        }
        assert!(
            !text.contains('\'') && !text.contains(';'),
            "the screen font has neither"
        );
        let mut picture = vec![0u8; 320 * 200 * 4];
        d.app.picture(0, &mut picture);
        let has = |c: [u8; 4]| picture.chunks_exact(4).any(|p| p == c);
        assert_eq!(
            (has(yellow), has(red)),
            (not_yet, !not_yet),
            "{name}: the heading"
        );
        let word = match setting {
            ModeSetting::Force(_) => "retail (forced by --game-mode)",
            ModeSetting::Probe => "registered (the IWAD has no fourth episode)",
        };
        let summary = d.app.session_summary().unwrap();
        assert!(
            summary.starts_with(&format!("session: game mode {word}")),
            "{summary}"
        );
        assert!(
            summary.ends_with(&format!("refused at session tic {tic}: {text}")),
            "{summary}"
        );
        // the refused tic made no change, so its command is not in the demo; the demo is the
        // genuine item's up to it and replays to the same refusal
        let demo = d.demo();
        let genuine = std::fs::read(m6a::root(&item.lmp)).unwrap();
        assert_eq!(commands_of(&demo), commands_of(&genuine)[..stop as usize]);
        assert!(d.app.session_name().contains(match setting {
            ModeSetting::Force(_) => "-retail-",
            ModeSetting::Probe => "-registered-",
        }));
        let (mut s, end) = replay(&d.app, &demo, &d.host.seen, None).unwrap();
        assert_eq!(end, None);
        let again = s.tic(Default::default()).unwrap_err();
        assert_eq!((again.tic, &again.witness), (tic, &**w), "{name}");
        // R on the refusal screen starts a new game
        d.app.key(b'r', true, &mut d.host);
        assert_eq!(d.app.mode(), &Mode::Playing);
        assert_eq!(d.app.session().unwrap().gametic(), 0);
        eprintln!("{name}: refused at tic {tic}: {text}");
    }
    // starts refused at load, plainly
    let item = m6a::item("CHAIN");
    let files: Vec<Vec<u8>> = item
        .wads
        .iter()
        .map(|w| std::fs::read(m6a::root(w)).unwrap())
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let start = |map, skill| GameData::load_session(&refs, ModeSetting::Probe, map, skill);
    let e = start(1, 1).unwrap_err();
    assert!(matches!(
        e,
        LoadError::SessionStart(StartError::NotYet(NotYet::EasySkill))
    ));
    assert!(
        e.to_string()
            .contains("easy skills (0 and 1) are not built yet"),
        "{e}"
    );
    assert!(matches!(
        start(10, 2),
        Err(LoadError::SessionStart(StartError::Domain))
    ));
    let noe1m2 = m6a::item("NOE1M2");
    let files: Vec<Vec<u8>> = noe1m2
        .wads
        .iter()
        .map(|w| std::fs::read(m6a::root(w)).unwrap())
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let e = GameData::load_session(&refs, ModeSetting::Probe, 2, 2).unwrap_err();
    assert!(matches!(e, LoadError::SessionStart(StartError::Sound(_))));
    assert!(
        e.to_string()
            .ends_with("the original stops at this load too"),
        "{e}"
    );
    // the settings a session starts with are the data's map and skill and the app's screenblocks
    let d = Driver::new(load(&item, ModeSetting::Probe), 11);
    let want = SessionSettings {
        map: 1,
        skill: 2,
        screenblocks: 11,
    };
    assert_eq!(d.app.session().unwrap().settings(), want);
}

/// A session no committed item holds, for the local genuine check
/// (`tools/local/m6a_play_check.py`): REBHELD played by keys (start E1M2, the death, the reborn,
/// the exit, the carried E1M3), then SECRET's E1M3 part to its first turn (the secret exit, the
/// intermission, the load of E1M9), then the quit. The demo replays through the session to the same
/// quit; with `M6A_PLAY_OUT` set, the session file is written into that directory under its own
/// name.
#[test]
fn a_session_through_the_secret_exit_replays_to_its_quit() {
    let item = m6a::item("REBHELD");
    let mut script = script_keys(&item);
    script.extend(script_keys(&m6a::item("SECRET")));
    let mut d = Driver::new(load(&item, ModeSetting::Probe), 10);
    d.play(&script);
    assert_eq!(d.app.mode(), &Mode::Playing);
    d.app.quit(&mut d.host);
    let demo = d.demo();
    assert_eq!(commands_of(&demo).len(), script.len());
    let loads: Vec<u8> = d
        .host
        .seen
        .iter()
        .filter_map(|s| match s.kind {
            TicKind::Load { map, .. } => Some(map),
            _ => None,
        })
        .collect();
    assert_eq!(
        loads,
        [2, 3, 9],
        "the reborn, E1M3, then E1M9 by the secret exit"
    );
    let (s, end) = replay(&d.app, &demo, &d.host.seen, None).unwrap();
    assert_eq!(end, None);
    let live = d.app.session().unwrap();
    assert_eq!(s.save().unwrap().encode(), live.save().unwrap().encode());
    if let Some(dir) = std::env::var_os("M6A_PLAY_OUT") {
        let path = std::path::Path::new(&dir).join(d.app.session_name());
        std::fs::write(&path, &demo).unwrap();
        eprintln!("wrote {}", path.display());
    }
    eprintln!(
        "{} tics; {}",
        script.len(),
        d.app.session_summary().unwrap()
    );
}
