//! M6a part 2 step 4, the app: the app in the session's finale-and-melt configuration
//! ([`GameData::load_finale_and_melt_session`], what doom-play plays; owner's ruling V1 of
//! 2026-11-23) driven headlessly by key events made from the committed synthetic scripts
//! (`tests/reference/m6a_scripts/*.txt`: a forward move of 25 is the up arrow held, `USE` the
//! space bar, `ATTACK` the Ctrl key, the vanilla bindings), one update per tic of wall time.
//!
//! Compared with the genuine linuxdoom-1.10 goldens only (no Rust output feeds an expectation):
//!
//! 1. **The recorded demo is the genuine item's**: the session file is byte for byte the item's
//!    committed demo (or its first tics where a script stops at a command no key gives, or where
//!    the genuine engine stops), so the app recorded one command per game tic and none per melt
//!    step.
//! 2. **Every tic and every melt step against the genuine engine**: each tic's composed frame and
//!    palette against the frames golden and its G row (game state, map, `leveltime`, `gametic`,
//!    both random indices); each melt step's picture, `done` and palette against the melt golden's
//!    V and Y rows; and **the picture presented after every update** (what the window shows:
//!    `App::frame`) against the genuine sequence under the declared schedule (M6a note §1.16.1):
//!    a tic that does not wipe shows its frame; a tic that wipes shows its melt's first step in
//!    the same update, then one step per update, then the next tic. That also proves the pacing:
//!    one slot per tic of wall time, no step skipped, no burst after a melt.
//! 3. **The demo replays through a fresh session** (`replay_finale_and_melt`) to the same tics,
//!    melt steps and `DSES` version 2 bytes, or to the same refusal.
#![allow(clippy::unwrap_used, reason = "test code")]

mod m6a;
mod m6a2_run;
#[path = "../../doom-sound/tests/common/rows.rs"]
mod rows;

use doom_game::app::{
    App, AppFault, AudioSink, Ending, FinaleMeltSessionTicReport, GameData, Host, MeltStepShown,
    Mode, RecordingSink, SessionConfig, SinkCall, Timestamp, Witness,
};
use doom_game::input::{Event, MouseButton, keys};
use doom_game::session::demo::{SessionDemo, replay_finale_and_melt};
use doom_game::session::finale_melt::{Configuration, FinaleMeltTicKind};
use doom_game::session::{ModeSetting, Session, SessionMode, SessionWitness};
use doom_render::{GameState, SessionError};
use doom_sim::TicCommand;
use doom_test_evidence::sha256;
use m6a::{Item, Rows, int};
use m6a2_run::{GoldenFrame, GoldenMelt, Source, read_frames, read_melts, spec};
use std::collections::BTreeMap;

/// One committed tic, as the host saw it.
#[derive(Clone, Debug, PartialEq)]
struct TicSeen {
    tic: u64,
    kind: FinaleMeltTicKind,
    events: Vec<Event>,
    executed: TicCommand,
    frame: String,
    palette: u8,
    wiped: bool,
    gamestate: i64,
    map: u8,
    leveltime: u64,
    prndindex: u8,
    rndindex: u8,
    buffer: bool,
}

/// One melt step, as the host saw it.
#[derive(Clone, Debug, PartialEq)]
struct StepSeen {
    tic: u64,
    step: u8,
    done: bool,
    palette: u8,
    picture: String,
    buffer: bool,
}

fn gamestate(s: &Session) -> i64 {
    match s.gamestate() {
        GameState::Level => 0,
        GameState::Intermission => 1,
        GameState::Finale => 2,
    }
}

fn tic_seen(s: &Session, r: &doom_game::session::finale_melt::FinaleMeltTicReport) -> TicSeen {
    TicSeen {
        tic: r.tic,
        kind: r.kind,
        events: Vec::new(),
        executed: TicCommand::default(),
        frame: sha256(s.frame().pixels()),
        palette: r.palette,
        wiped: r.wiped,
        gamestate: gamestate(s),
        map: s.map(),
        leveltime: s.level().tic(),
        prndindex: s.level().rng().indices()[0],
        rndindex: s.display().random_index(),
        buffer: r.sound.sound.buffer.is_some(),
    }
}

#[derive(Default)]
struct PlayHost {
    files: BTreeMap<String, Vec<u8>>,
    sink: RecordingSink,
    tics: Vec<TicSeen>,
    steps: Vec<StepSeen>,
}

impl Host for PlayHost {
    fn timestamp(&mut self) -> Timestamp {
        Timestamp {
            year: 2026,
            month: 11,
            day: 24,
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
    fn finale_melt_tic(&mut self, r: &FinaleMeltSessionTicReport<'_>) {
        let mut t = tic_seen(r.session, r.report);
        t.events = r.events.to_vec();
        t.executed = r.executed;
        self.tics.push(t);
    }
    fn melt_step(&mut self, r: &MeltStepShown<'_>) {
        self.steps.push(StepSeen {
            tic: r.report.tic,
            step: r.report.step,
            done: r.report.done,
            palette: r.report.palette,
            picture: sha256(r.picture.pixels()),
            buffer: r.report.buffer.is_some(),
        });
    }
}

/// The keys held in one tic: up arrow, space (use), Ctrl (fire).
type Keys = (bool, bool, bool);

/// The keys a script holds in each tic, up to its first command no key gives.
fn script_keys(item: &Item) -> Vec<Keys> {
    let txt = item.lmp.replace(".lmp", ".txt");
    let text = std::fs::read_to_string(m6a::root(&txt))
        .unwrap()
        .replace("\r\n", "\n");
    let mut out = Vec::new();
    for l in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = l.split_whitespace().collect();
        let held = match f.as_slice() {
            ["map", _] => continue,
            ["idle", n] => (n.parse::<usize>().unwrap(), (false, false, false)),
            ["cmd", n, fwd, "0", "0", b]
                if matches!(*fwd, "0" | "25") && matches!(*b, "0" | "USE" | "ATTACK") =>
            {
                (
                    n.parse().unwrap(),
                    (*fwd == "25", *b == "USE", *b == "ATTACK"),
                )
            }
            _ => break,
        };
        out.extend(std::iter::repeat_n(held.1, held.0));
    }
    out
}

fn at(k: u64) -> u64 {
    (k * 1_000_000).div_ceil(35)
}

/// What one update did and the picture it left on the window.
#[derive(Clone, Debug, PartialEq)]
struct Presented {
    ran: u64,
    melt_steps: u64,
    dropped: u64,
    picture: String,
    palette: u8,
}

/// The app at the window's tic rate: one update per tic of wall time, key events between them.
struct Driver {
    app: App,
    host: PlayHost,
    clock: u64,
    base: u64,
    held: Keys,
    presented: Vec<Presented>,
}

impl Driver {
    fn new(data: GameData, cfg: SessionConfig) -> Self {
        let mut host = PlayHost::default();
        let mut app = App::new(data, cfg, &mut host);
        assert_eq!(
            app.update(0, &mut host).ran,
            0,
            "the first update starts the clock"
        );
        Self {
            app,
            host,
            clock: 0,
            base: 0,
            held: (false, false, false),
            presented: Vec::new(),
        }
    }

    fn set(&mut self, want: Keys) {
        let pairs = [
            (want.0, self.held.0, keys::KEY_UPARROW),
            (want.1, self.held.1, b' '),
            (want.2, self.held.2, keys::KEY_RCTRL),
        ];
        for (w, h, code) in pairs {
            if w != h {
                self.app.key(code, w, &mut self.host);
            }
        }
        self.held = want;
    }

    /// One update one tic of wall time after the last.
    fn update(&mut self) -> Presented {
        self.clock += 1;
        let u = self.app.update(self.base + at(self.clock), &mut self.host);
        let (picture, palette) = match self.app.frame() {
            Some((px, p)) => (sha256(px), p),
            None => (String::new(), 0),
        };
        let p = Presented {
            ran: u.ran,
            melt_steps: u.melt_steps,
            dropped: u.dropped,
            picture,
            palette,
        };
        // the window presents the game's picture unless the session ended in this update (the
        // end screen is shown then)
        let ended = matches!(self.app.mode(), Mode::Ended(_));
        if (u.ran > 0 || u.melt_steps > 0) && !ended {
            self.presented.push(p.clone());
        }
        p
    }

    /// After a pause or a restart, the app's clock starts again at its next update: the driver's
    /// clock is re-based there so that every later update is exactly one tic of wall time later.
    fn restart_clock(&mut self) {
        self.base += at(self.clock) + 1;
        self.clock = 0;
        assert_eq!(self.app.update(self.base, &mut self.host).ran, 0);
    }

    /// Plays `script`, one game tic per entry (melt steps between them), until it ends or the
    /// session stops playing. The pacing itself is checked by [`paced`].
    fn play(&mut self, script: &[Keys]) {
        for &want in script {
            for guard in 0.. {
                assert!(guard < 100, "no tic in 100 updates");
                if self.app.mode() != &Mode::Playing {
                    return;
                }
                self.set(want);
                let tics = self.host.tics.len();
                let p = self.update();
                assert!(p.ran <= 1, "one tic per tic of wall time: {p:?}");
                if self.host.tics.len() > tics {
                    break;
                }
            }
            // a load clears the engine's held keys (g_game.c:490-496): press them again
            if matches!(
                self.host.tics.last(),
                Some(t) if matches!(t.kind, FinaleMeltTicKind::Load { .. })
            ) {
                self.set((false, false, false));
            }
        }
        // the last tic's melt, if any, to its end
        while self.app.melt_in_progress() {
            self.set(script.last().copied().unwrap_or_default());
            self.update();
        }
        self.set((false, false, false));
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
    let data =
        GameData::load_finale_and_melt_session(&refs, setting, item.map, item.skill).unwrap();
    assert_eq!(data.session_configuration(), Configuration::FinaleAndMelt);
    data
}

fn setting_of(item: &Item) -> ModeSetting {
    match item.mode.as_str() {
        "retail" => ModeSetting::Force(SessionMode::Retail),
        _ => ModeSetting::Probe,
    }
}

fn cfg(screenblocks: u8) -> SessionConfig {
    SessionConfig {
        screenblocks,
        ..SessionConfig::default()
    }
}

/// The genuine goldens of an item at a screen size.
struct Genuine {
    rows: Rows,
    frames: BTreeMap<i64, GoldenFrame>,
    melts: BTreeMap<i64, GoldenMelt>,
}

fn genuine(name: &str, sb: u8) -> Genuine {
    let source = spec(name).source;
    let (session, frames, melts) = match source {
        Source::Part1 => (
            format!("m6a_session/{name}.session"),
            format!("m6a_frames/{name}.sb{sb}.frames"),
            if sb == 10 {
                format!("m6a_melt/{name}.melt")
            } else {
                format!("m6a2_melt/{name}.sb{sb}.melt")
            },
        ),
        Source::Full => (
            format!("m6a2_session/{name}.session"),
            format!("m6a2_frames/{name}.sb{sb}.frames"),
            format!("m6a2_melt/{name}.sb{sb}.melt"),
        ),
    };
    Genuine {
        rows: Rows::read(&session),
        frames: read_frames(&frames).0,
        melts: read_melts(&melts),
    }
}

/// What a comparison covered.
#[derive(Debug, Default, PartialEq)]
struct Covered {
    tics: usize,
    melts: usize,
    melt_steps: usize,
    presented: usize,
}

/// Comparison 2 (module documentation): every tic, melt step and presented picture of the live
/// app against the genuine goldens; the counts compared, or the first difference.
fn against_genuine(name: &str, g: &Genuine, d: &Driver) -> Result<Covered, String> {
    let mut c = Covered::default();
    let mut want_presented: Vec<(String, u8)> = Vec::new();
    let mut steps = d.host.steps.iter();
    for t in &d.host.tics {
        let tic = t.tic as i64;
        let diff = |what: String| format!("{name} tic {tic}: {what}");
        let row = g
            .rows
            .at("G", tic)
            .ok_or_else(|| diff("no genuine G row".into()))?;
        for (col, v) in [
            ("gamestate", t.gamestate),
            ("map", i64::from(t.map)),
            ("leveltime", t.leveltime as i64),
            ("gametic", tic + 1),
            ("prndindex", i64::from(t.prndindex)),
            ("rndindex", i64::from(t.rndindex)),
        ] {
            if int(row, col) != v {
                return Err(diff(format!("G {col} {v} genuine {}", row[col])));
            }
        }
        let f = g
            .frames
            .get(&tic)
            .ok_or_else(|| diff("no genuine frame".into()))?;
        if t.frame != f.sha256 || i64::from(t.palette) != f.status[0] {
            return Err(diff("the frame or its palette differs".into()));
        }
        c.tics += 1;
        match (t.wiped, g.melts.get(&tic)) {
            (false, None) => want_presented.push((f.sha256.clone(), t.palette)),
            (true, Some(m)) => {
                for (k, (sha, done)) in m.steps.iter().enumerate() {
                    let s = steps
                        .next()
                        .ok_or_else(|| diff(format!("melt step {} not shown", k + 1)))?;
                    if (s.tic, usize::from(s.step)) != (t.tic, k + 1)
                        || &s.picture != sha
                        || s.done != *done
                        || i64::from(s.palette) != m.palette
                    {
                        return Err(diff(format!("melt step {} differs: {s:?}", k + 1)));
                    }
                    if s.buffer && !s.done {
                        return Err(diff(format!("a buffer at melt step {}", k + 1)));
                    }
                    want_presented.push((sha.clone(), s.palette));
                    c.melt_steps += 1;
                }
                c.melts += 1;
            }
            (w, m) => return Err(diff(format!("wiped {w}, genuine melt {}", m.is_some()))),
        }
    }
    if let Some(s) = steps.next() {
        return Err(format!(
            "{name}: a melt step the genuine run does not have: {s:?}"
        ));
    }
    let got: Vec<(String, u8)> = d
        .presented
        .iter()
        .map(|p| (p.picture.clone(), p.palette))
        .collect();
    if let Some(i) =
        (0..got.len().max(want_presented.len())).find(|&i| got.get(i) != want_presented.get(i))
    {
        return Err(format!(
            "{name}: presented picture {i} of {} differs from the genuine schedule",
            want_presented.len()
        ));
    }
    c.presented = got.len();
    Ok(c)
}

/// The pacing: every update one slot, none dropped, at most one game tic and one melt step (the
/// melt's first step in its tic's slot), and the number of slots is the tics plus the melt steps
/// that do not share a tic's slot.
fn paced(d: &Driver) -> Result<(), String> {
    for (i, p) in d.presented.iter().enumerate() {
        let ok = matches!((p.ran, p.melt_steps), (1, 0) | (1, 1) | (0, 1)) && p.dropped == 0;
        if !ok {
            return Err(format!("update {i}: {p:?}"));
        }
    }
    let melts = d.host.tics.iter().filter(|t| t.wiped).count();
    let slots = d.host.tics.len() + d.host.steps.len() - melts;
    if d.presented.len() != slots {
        return Err(format!("{} updates for {slots} slots", d.presented.len()));
    }
    Ok(())
}

/// The sink's calls: no buffer between a melt's start and its end, one start and one end per
/// melt, and every buffer the session reported handed over in order.
fn sound_paced(d: &Driver) -> Result<(), String> {
    let mut melting = false;
    let (mut starts, mut ends) = (0, 0);
    for c in &d.host.sink.calls {
        match c {
            SinkCall::MeltStart if !melting => (melting, starts) = (true, starts + 1),
            SinkCall::MeltEnd if melting => (melting, ends) = (false, ends + 1),
            SinkCall::Buffer(i) if melting => return Err(format!("buffer {i} during a melt")),
            SinkCall::MeltStart | SinkCall::MeltEnd => return Err(format!("{c:?} out of order")),
            _ => {}
        }
    }
    let melts = d.host.tics.iter().filter(|t| t.wiped).count();
    let ended = d.host.steps.iter().filter(|s| s.done).count();
    if (starts, ends) != (melts, ended) {
        return Err(format!("{starts} starts and {ends} ends, {melts} melts"));
    }
    let reported = d.host.tics.iter().filter(|t| t.buffer).count()
        + d.host.steps.iter().filter(|s| s.buffer).count();
    if reported != d.host.sink.buffers.len() {
        return Err(format!(
            "{reported} buffers reported, {} handed",
            d.host.sink.buffers.len()
        ));
    }
    Ok(())
}

fn commands_of(bytes: &[u8]) -> Vec<TicCommand> {
    SessionDemo::parse(bytes).unwrap().commands
}

/// Comparison 3: the saved demo through a fresh session of the app's own data; the session, its
/// end, and whether every tic and melt step equals the live app's.
fn replay(
    d: &Driver,
    demo: &[u8],
) -> Result<(Session, Option<doom_game::session::Refusal>), String> {
    let parsed = SessionDemo::parse(demo).map_err(|e| format!("{e:?}"))?;
    let live = d.app.session().unwrap();
    let mut s = Session::new_finale_and_melt(live.data().clone(), live.settings()).unwrap();
    let (mut tics, mut steps) = (Vec::new(), Vec::new());
    let end = replay_finale_and_melt(
        &mut s,
        &parsed.commands,
        |s, r| tics.push(tic_seen(s, r)),
        |_, m, px| {
            steps.push(StepSeen {
                tic: m.tic,
                step: m.step,
                done: m.done,
                palette: m.palette,
                picture: sha256(px.pixels()),
                buffer: m.buffer.is_some(),
            });
        },
    );
    let live_tics: Vec<TicSeen> = d
        .host
        .tics
        .iter()
        .map(|t| TicSeen {
            events: Vec::new(),
            executed: TicCommand::default(),
            ..t.clone()
        })
        .collect();
    if tics != live_tics {
        let i = (0..).find(|&i| tics.get(i) != live_tics.get(i)).unwrap();
        return Err(format!("replay tic {i} differs from the live app"));
    }
    if steps != d.host.steps {
        return Err("the replay's melt steps differ from the live app's".into());
    }
    Ok((s, end))
}

/// CHAIN (two exits, two intermissions, two carried loads; to its first turn), REBHELD (the
/// death, the reborn, the exit and the intermission, whole), FINRET (E1M8 into the finale in
/// retail mode, whole: the text, the stage-1 wipe and CREDIT) and FULLREG (the same commands with
/// the narrow font in registered mode: the whole crawl and HELP2), played by keys, every tic, melt
/// step and presented picture against the genuine goldens, the demo against the committed one and
/// its replay to the same `DSES` version 2 bytes.
#[test]
fn played_sessions_match_the_genuine_tics_melt_steps_and_demos() {
    let mut total = Covered::default();
    for (name, sb, whole) in [
        ("CHAIN", 10, false),
        ("REBHELD", 11, true),
        ("FINRET", 10, true),
        ("FULLREG", 11, true),
    ] {
        let item = spec(name).item;
        let script = script_keys(&item);
        assert_eq!(script.len() == item.session_tics, whole, "{name}");
        let mut d = Driver::new(load(&item, setting_of(&item)), cfg(sb));
        d.play(&script);
        assert_eq!(d.app.mode(), &Mode::Playing, "{name}: no refusal");
        assert_eq!(d.host.tics.len(), script.len(), "{name}");
        d.app.quit(&mut d.host);
        // 1: one command per game tic, none per melt step; the genuine demo
        let demo = d.demo();
        let committed = std::fs::read(m6a::root(&item.lmp)).unwrap();
        assert_eq!(d.app.recorder().len(), script.len(), "{name}");
        if whole {
            assert_eq!(
                demo, committed,
                "{name}: the session file is the item's demo"
            );
        } else {
            assert_eq!(demo[..13], committed[..13], "{name}: header");
            assert_eq!(commands_of(&demo), commands_of(&committed)[..script.len()]);
        }
        // 2: every tic, melt step and presented picture
        let c = against_genuine(name, &genuine(name, sb), &d).unwrap();
        paced(&d).unwrap();
        sound_paced(&d).unwrap();
        assert_eq!(d.app.melts() as usize, c.melts, "{name}");
        // 3: the replay to the same state
        let (s, end) = replay(&d, &demo).unwrap();
        assert_eq!(end, None, "{name}");
        assert_eq!(
            s.save_v2().unwrap().encode(),
            d.app.session().unwrap().save_v2().unwrap().encode(),
            "{name}: the same DSES version 2 bytes"
        );
        eprintln!(
            "{name} sb{sb}: {} tics, {} melts, {} melt steps, {} presented pictures equal to the \
             genuine engine; {}",
            c.tics,
            c.melts,
            c.melt_steps,
            c.presented,
            d.app.session_summary().unwrap()
        );
        total.tics += c.tics;
        total.melts += c.melts;
        total.melt_steps += c.melt_steps;
        total.presented += c.presented;
    }
    eprintln!("in all: {total:?}");
    assert_eq!(total.tics, total.presented + total.melts - total.melt_steps);
}

/// The finale never ends by itself (rulings U5, Q-f_finale-207): FINRET played to the end of its
/// demo and on for 1,000 idle tics more stays on the art screen, every frame the genuine last
/// frame (one unchanging CREDIT picture), playing until the player quits; the exit line names the
/// finale's first tic and its art screen's (both from the genuine rows). The pause screen works
/// in the finale, and R on it starts a new game whose first update shows the genuine first step
/// of the session-start melt.
#[test]
fn the_finale_stays_until_quit_with_pause_and_restart() {
    let item = spec("FINRET").item;
    let g = genuine("FINRET", 10);
    let script = script_keys(&item);
    let mut d = Driver::new(load(&item, setting_of(&item)), cfg(10));
    d.play(&script);
    let last = (script.len() - 1) as i64;
    let art = &g.frames[&last].sha256;
    let idle = vec![(false, false, false); 1000];
    d.play(&idle);
    assert_eq!(d.app.mode(), &Mode::Playing);
    let after: Vec<&TicSeen> = d.host.tics[script.len()..].iter().collect();
    assert_eq!(after.len(), 1000);
    assert!(
        after
            .iter()
            .all(|t| t.frame == *art && !t.wiped && t.kind == FinaleMeltTicKind::Finale)
    );
    let victory = (0..)
        .find(|&t| g.rows.at("G", t).is_some_and(|r| int(r, "gamestate") == 2))
        .unwrap();
    let stage1 = (0..)
        .find(|&t| {
            g.rows
                .at("F", t)
                .is_some_and(|r| int(r, "finalestage") == 1)
        })
        .unwrap();
    assert_eq!(d.app.finale_began(), Some(victory as u64));
    assert_eq!(d.app.art_screen_began(), Some(stage1 as u64));
    let summary = d.app.session_summary().unwrap();
    assert_eq!(
        summary,
        format!(
            "session: game mode retail (forced by --game-mode), started at E1M8 skill 2, ended \
             in the finale (after E1M8) after {} tics; the finale began at session tic {victory}, \
             its art screen (CREDIT) began at session tic {stage1}",
            script.len() + 1000
        )
    );
    // the pause in the finale: no tic runs, then the resume
    d.app.key(keys::KEY_ESCAPE, true, &mut d.host);
    d.app.key(keys::KEY_ESCAPE, false, &mut d.host);
    assert_eq!(d.app.mode(), &Mode::Paused);
    let n = d.host.tics.len();
    assert_eq!(d.update().ran, 0);
    d.app.key(keys::KEY_PAUSE, true, &mut d.host);
    assert_eq!(d.app.mode(), &Mode::Playing);
    d.restart_clock();
    assert_eq!(d.update().ran, 1);
    assert_eq!(d.host.tics.len(), n + 1);
    assert_eq!(d.host.tics[n].frame, *art);
    // R on the pause screen: a new game and a new demo; the first demo is kept whole
    let first = d.app.session_name().to_string();
    d.app.key(keys::KEY_ESCAPE, true, &mut d.host);
    d.app.key(b'r', true, &mut d.host);
    assert_eq!(d.app.mode(), &Mode::Playing);
    assert_eq!(d.app.session_name(), first.replace(".lmp", "-2.lmp"));
    assert_eq!(
        commands_of(&d.host.files[&first]).len(),
        script.len() + 1001
    );
    assert_eq!(
        (d.app.finale_began(), d.app.art_screen_began()),
        (None, None)
    );
    let s = d.app.session().unwrap();
    assert_eq!((s.gametic(), s.map()), (0, 8));
    d.restart_clock();
    // FINALE's script holds the up arrow from tic 0 (its first command moves 25)
    d.set(script[0]);
    let p = d.update();
    assert_eq!(
        (p.ran, p.melt_steps),
        (1, 1),
        "tic 0 and its melt's first step"
    );
    assert_eq!(p.picture, g.melts[&0].steps[0].0, "the genuine first step");
}

/// Registered mode without `HELP2` (NOHELP2, ruling U4 (3)): the session ends at the genuine
/// stop (the first stage-1 frame) with the refusal screen, the demo saved whole (the refused tic's
/// command kept, as the genuine engine ran its `F_Ticker`), equal to the committed demo's first
/// tics, and its replay ends with the same refusal. The exit line says where the finale began and
/// that its art screen was refused there; the screen says so plainly in red; R restarts.
#[test]
fn registered_without_help2_ends_at_the_genuine_tic_with_the_demo_saved() {
    let item = spec("NOHELP2").item;
    let g = genuine("NOHELP2", 11);
    let mut d = Driver::new(load(&item, setting_of(&item)), cfg(11));
    d.play(&script_keys(&item));
    let Mode::Ended(Ending::Refused { tic, witness }) = d.app.mode().clone() else {
        panic!("{:?}", d.app.mode());
    };
    assert_eq!(tic, item.session_tics as u64, "the genuine stop");
    assert_eq!(item.end, "error:W_GetNumForName: HELP2 not found!");
    let Witness::Session(w) = &witness else {
        panic!("{witness:?}")
    };
    assert!(
        matches!(**w, SessionWitness::Display(SessionError::Finale(_))),
        "{w:?}"
    );
    let text = witness.to_string();
    assert_eq!(
        text,
        "the finale art screen cannot be drawn (W_GetNumForName: HELP2 not found!). The original \
         stops here too. In registered mode its art screen is HELP2, in retail mode CREDIT."
    );
    assert!(!text.contains('\'') && !text.contains(';') && !text.contains("not built"));
    let demo = d.demo();
    let genuine_demo = std::fs::read(m6a::root(&item.lmp)).unwrap();
    assert_eq!(
        commands_of(&demo),
        commands_of(&genuine_demo)[..=tic as usize]
    );
    let c = against_genuine("NOHELP2", &g, &d).unwrap();
    paced(&d).unwrap();
    sound_paced(&d).unwrap();
    let victory = (0..)
        .find(|&t| g.rows.at("G", t).is_some_and(|r| int(r, "gamestate") == 2))
        .unwrap();
    assert_eq!(d.app.art_screen_refused(), Some(tic));
    assert_eq!(
        d.app.session_summary().unwrap(),
        format!(
            "session: game mode registered (the IWAD has no fourth episode), started at E1M8 skill \
             2, ended in the finale (after E1M8) after {} tics; the finale began at session tic \
             {victory}, its art screen (HELP2) was refused at session tic {tic}; refused at \
             session tic {tic}: {text}",
            tic + 1
        )
    );
    let mut picture = vec![0u8; 320 * 200 * 4];
    d.app.picture(0, &mut picture);
    let has = |c: [u8; 4]| picture.chunks_exact(4).any(|p| p == c);
    assert!(
        has([235, 95, 95, 255]) && !has([250, 210, 60, 255]),
        "a red TIC REFUSED heading"
    );
    let (_, end) = replay(&d, &demo).unwrap();
    let end = end.unwrap();
    assert_eq!((end.tic, &end.witness), (tic, &**w));
    d.app.key(b'r', true, &mut d.host);
    assert_eq!(d.app.mode(), &Mode::Playing);
    eprintln!(
        "NOHELP2: {} tics, {} melts, {} steps equal; refused at tic {tic}",
        c.tics, c.melts, c.melt_steps
    );
}

/// S21's cap applies in the finale: FINRET with a cap of 400 tics ends at it, in the finale's
/// text; the melt steps take no tic of the cap.
#[test]
fn the_tic_cap_applies_in_the_finale() {
    let item = spec("FINRET").item;
    let config = SessionConfig {
        tic_cap: 400,
        ..cfg(10)
    };
    let mut d = Driver::new(load(&item, setting_of(&item)), config);
    d.play(&script_keys(&item));
    assert_eq!(d.app.mode(), &Mode::Ended(Ending::TicCap));
    assert_eq!(d.app.recorder().len(), 400);
    assert!(d.host.steps.len() > 60, "two melts' steps ran besides");
    let summary = d.app.session_summary().unwrap();
    assert!(
        summary.ends_with(
            "after 400 tics; the finale began at session tic 321, its art screen had not begun; \
             at the session length cap"
        ),
        "{summary}"
    );
}

/// A fresh app on CHAIN's WADs in the new configuration, its clock started, with tic 0 run: the
/// session-start melt (tic 0's display wipes) has shown its first step and is in progress.
fn in_first_melt(fault: Option<AppFault>) -> Driver {
    let item = m6a::item("CHAIN");
    let mut d = Driver::new(load(&item, ModeSetting::Probe), cfg(10));
    d.app.plant_fault(fault);
    let p = d.update();
    assert_eq!((p.ran, p.melt_steps), (1, 1));
    assert!(d.app.melt_in_progress());
    d
}

/// Runs the melt to its end with no input; the updates it took. No command is built, none
/// recorded and no buffer handed over meanwhile, unless a fault says otherwise (then the error).
fn finish_melt(d: &mut Driver) -> Result<usize, String> {
    let (recorded, buffers) = (d.app.recorder().len(), d.host.sink.buffers.len());
    let mut n = 0;
    while d.app.melt_in_progress() {
        let p = d.update();
        n += 1;
        if p.ran != 0 || d.app.recorder().len() > recorded {
            return Err(format!("a command during the melt: {p:?}"));
        }
        if d.app.melt_in_progress() && d.host.sink.buffers.len() != buffers {
            return Err("a buffer during the melt".into());
        }
    }
    Ok(n)
}

/// The first tic after the melt.
fn tic_after(d: &mut Driver) -> TicSeen {
    let n = d.host.tics.len();
    let p = d.update();
    assert_eq!(p.ran, 1, "the next tic in the next slot");
    d.host.tics[n].clone()
}

/// Keys and buttons during a melt are held and reach the game, in order, on the first tic after
/// it (the owner's ruling of 2026-11-23): the up arrow pressed during the session-start melt moves
/// the player in tic 1, not before (no command is built during the melt). Several events in one
/// melt keep their order (the up arrow, then the space bar, then the left button, posted as the
/// layer posts a pump: keys in order, then one mouse event). A key pressed and released inside the
/// melt (the space bar): S6's same-pump rule holds the release back one command, so the press is in
/// tic 1's command and the release in tic 2's (M6a note §1.16.5; COMPATIBILITY).
#[test]
fn keys_and_buttons_during_a_melt_reach_the_first_tic_after_it_in_order() {
    let mut d = in_first_melt(None);
    d.app.key(keys::KEY_UPARROW, true, &mut d.host);
    assert_eq!(d.app.held_inputs(), 1);
    let steps = finish_melt(&mut d).unwrap();
    assert!(steps >= 33, "{steps}");
    let t1 = tic_after(&mut d);
    assert_eq!(t1.tic, 1);
    assert_eq!(t1.events, [Event::KeyDown(keys::KEY_UPARROW)]);
    assert_eq!(t1.executed.forward, 25);
    assert_eq!(
        d.host.tics[0].executed.forward, 0,
        "tic 0's command was built before"
    );
    assert_eq!(d.app.held_inputs(), 0);

    let mut d = in_first_melt(None);
    d.app.key(keys::KEY_UPARROW, true, &mut d.host);
    d.app.key(b' ', true, &mut d.host);
    d.app.mouse_button(MouseButton::Left, true);
    d.app.key(b' ', false, &mut d.host);
    assert_eq!(d.app.held_inputs(), 4);
    finish_melt(&mut d).unwrap();
    let t1 = tic_after(&mut d);
    assert_eq!(
        t1.events,
        [
            Event::KeyDown(keys::KEY_UPARROW),
            Event::KeyDown(b' '),
            Event::Mouse {
                buttons: 1,
                dx: 0,
                dy: 0
            }
        ]
    );
    // forward, use and fire in tic 1
    assert_eq!((t1.executed.forward, t1.executed.buttons & 3), (25, 3));
    let t2 = tic_after(&mut d);
    assert_eq!(
        t2.events,
        [Event::KeyUp(b' ')],
        "the release one command later"
    );
    assert_eq!((t2.executed.forward, t2.executed.buttons & 3), (25, 1));
}

/// Mouse motion during a melt is ignored, not accumulated (the owner's ruling of 2026-11-21):
/// motion during the session-start melt turns nothing in tic 1, while the up arrow pressed in the
/// same melt is kept (the two rules differ within one melt). Outside a melt the same motion turns
/// the player (the control).
#[test]
fn motion_during_a_melt_is_ignored_while_keys_are_kept() {
    let mut d = in_first_melt(None);
    d.app.mouse_motion(100, 0);
    d.app.key(keys::KEY_UPARROW, true, &mut d.host);
    d.app.mouse_motion(100, 0);
    assert_eq!(d.app.held_inputs(), 1, "the motion is not held");
    finish_melt(&mut d).unwrap();
    let t1 = tic_after(&mut d);
    assert_eq!(t1.events, [Event::KeyDown(keys::KEY_UPARROW)]);
    assert_eq!((t1.executed.forward, t1.executed.angle_turn), (25, 0));
    // the control: the same motion before tic 2 turns the player
    d.app.mouse_motion(100, 0);
    let t2 = tic_after(&mut d);
    assert!(
        matches!(t2.events[..], [Event::Mouse { dx: 400, .. }]),
        "{:?}",
        t2.events
    );
    assert_ne!(t2.executed.angle_turn, 0);
}

/// The app's own keys during a melt are held like any key and act when the melt ends: Esc pauses
/// then (the melt finishes first, no tic runs after it), and R after it restarts, as on the pause
/// screen. A focus loss during a melt pauses when it ends. Closing the window (`App::quit`) works
/// during a melt.
#[test]
fn the_apps_own_keys_during_a_melt_act_when_it_ends() {
    let mut d = in_first_melt(None);
    d.app.key(keys::KEY_ESCAPE, true, &mut d.host);
    d.app.key(keys::KEY_UPARROW, true, &mut d.host);
    assert_eq!(d.app.mode(), &Mode::Playing, "held");
    finish_melt(&mut d).unwrap();
    assert_eq!(
        d.app.mode(),
        &Mode::Paused,
        "the pause took effect at the melt's end"
    );
    assert_eq!(d.update().ran, 0);
    assert_eq!(d.host.tics.len(), 1);
    let calls = &d.host.sink.calls;
    let end = calls.iter().position(|c| *c == SinkCall::MeltEnd).unwrap();
    assert_eq!(
        calls.last(),
        Some(&SinkCall::Pause),
        "after the melt's end: {calls:?}"
    );
    assert!(end < calls.len() - 1);

    let mut d = in_first_melt(None);
    let name = d.app.session_name().to_string();
    d.app.key(keys::KEY_ESCAPE, true, &mut d.host);
    d.app.key(b'r', true, &mut d.host);
    finish_melt(&mut d).unwrap();
    assert_eq!(d.app.mode(), &Mode::Playing);
    assert_eq!(
        d.app.session_name(),
        name.replace(".lmp", "-2.lmp"),
        "restarted"
    );
    assert_eq!(d.app.session().unwrap().gametic(), 0);

    let mut d = in_first_melt(None);
    d.app.focus_lost();
    assert_eq!(d.app.mode(), &Mode::Playing);
    finish_melt(&mut d).unwrap();
    assert_eq!(d.app.mode(), &Mode::Paused);

    let mut d = in_first_melt(None);
    d.app.quit(&mut d.host);
    assert!(d.app.quit_requested());
    assert_eq!(d.update().melt_steps, 0, "nothing runs after the quit");
    assert_eq!(commands_of(&d.demo()).len(), 1, "tic 0 saved");
}

/// No catch-up burst after a melt: with the host on time the first update after the melt runs one
/// tic, and so does each after it; a late host drops slots and never runs two melt steps in one
/// update.
#[test]
fn no_burst_after_a_melt_and_a_late_host_skips_no_melt_step() {
    let mut d = in_first_melt(None);
    finish_melt(&mut d).unwrap();
    for _ in 0..5 {
        assert_eq!(d.update().ran, 1);
    }
    // a late host during a melt: three slots due, one step shown, two dropped
    let mut d = in_first_melt(None);
    let shown = d.host.steps.len();
    d.clock += 2;
    let p = d.update();
    assert_eq!((p.ran, p.melt_steps, p.dropped), (0, 1, 2));
    assert_eq!(d.host.steps.len(), shown + 1);
}

/// The planted faults, each caught by the checks above: a key dropped during a melt, motion
/// accumulated across a melt, a command built during a melt step, a melt step skipped by the
/// pacing and a catch-up burst after a melt.
#[test]
fn the_planted_faults_are_caught() {
    // a key dropped during a melt: tic 1 does not move
    let mut d = in_first_melt(Some(AppFault::DropKeyInMelt));
    d.app.key(keys::KEY_UPARROW, true, &mut d.host);
    finish_melt(&mut d).unwrap();
    assert_eq!(
        tic_after(&mut d).executed.forward,
        0,
        "caught: the key was dropped"
    );
    // motion accumulated across the melt: tic 1 turns
    let mut d = in_first_melt(Some(AppFault::MotionInMelt));
    d.app.mouse_motion(100, 0);
    finish_melt(&mut d).unwrap();
    assert_ne!(
        tic_after(&mut d).executed.angle_turn,
        0,
        "caught: the motion reached tic 1"
    );
    // a command built during a melt step: the melt check fails, and so does the demo
    let mut d = in_first_melt(Some(AppFault::CommandInMeltStep));
    let e = finish_melt(&mut d).unwrap_err();
    assert!(e.starts_with("a command during the melt"), "{e}");
    // a melt step skipped by the pacing, and a catch-up burst: REBHELD by keys against the
    // genuine goldens
    let item = m6a::item("REBHELD");
    let script = script_keys(&item);
    let g = genuine("REBHELD", 10);
    for (fault, want) in [
        (AppFault::SkipMeltStep, "differs from the genuine schedule"),
        (AppFault::CatchUpAfterMelt, "one tic per tic of wall time"),
    ] {
        let mut d = Driver::new(load(&item, ModeSetting::Probe), cfg(10));
        d.app.plant_fault(Some(fault));
        let played = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| d.play(&script)));
        let e = match played {
            Err(p) => p
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| "a panic".into()),
            Ok(()) => against_genuine("REBHELD", &g, &d)
                .and_then(|_| paced(&d))
                .err()
                .unwrap_or_else(|| panic!("{fault:?}: not caught")),
        };
        assert!(e.contains(want), "{fault:?}: caught elsewhere: {e}");
        eprintln!("{fault:?}: caught: {e}");
    }
    // the same checks pass without a fault
    let mut d = Driver::new(load(&item, ModeSetting::Probe), cfg(10));
    d.play(&script);
    against_genuine("REBHELD", &g, &d).unwrap();
    paced(&d).unwrap();
}

/// The frozen app is still offered by the library and untouched: data loaded with
/// `GameData::load_session` keeps the frozen configuration (its tests, `tests/m6a_play.rs`, pin
/// its behaviour, the "not built yet" stop at E1M8's exit included).
#[test]
fn the_frozen_app_keeps_its_configuration() {
    let item = m6a::item("CHAIN");
    let files: Vec<Vec<u8>> = item
        .wads
        .iter()
        .map(|w| std::fs::read(m6a::root(w)).unwrap())
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let data = GameData::load_session(&refs, ModeSetting::Probe, 1, 2).unwrap();
    assert_eq!(
        data.session_configuration(),
        Configuration::RefusesFinaleAndMelt
    );
    let mut host = PlayHost::default();
    let mut app = App::new(data, cfg(10), &mut host);
    app.update(0, &mut host);
    let u = app.update(at(1), &mut host);
    assert_eq!(
        (u.ran, u.melt_steps),
        (1, 0),
        "the frozen app shows no melt"
    );
    assert!(!app.melt_in_progress());
    assert!(host.tics.is_empty() && host.steps.is_empty());
}
