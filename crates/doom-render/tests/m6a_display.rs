//! M6a: the session display (`Display::new_session`, `DDSP` version 3) and the single-player
//! intermission drawer (`doom_render::wi_stuff`) against the genuine M6a goldens
//! (`tests/reference/m6a_frames`, `m6a_session`; written by the genuine engine only,
//! `tools/reference/m6a_goldens.py`).
//!
//! What each test proves, and what it does not:
//!
//! - **The drawer alone** (`intermission_frames_against_the_genuine_digests`): every intermission
//!   frame of the 26 items at screenblocks 10 and 11, drawn from the intermission state the
//!   genuine session rows record (the J and I rows: an explicit fixture projection, since the
//!   Rust intermission logic is the next step), against the genuine whole-frame digest. The
//!   pointer state `snl_pointeron` is not taken from the rows: it follows the logic's rule
//!   (`m6a::pointer_before`) and the drawer's own write, and is checked against the rows'
//!   after-display value on every frame (the first-frame quirk of M6a note §1.6).
//! - **The `M_Random` accounting across transitions** (`random_index_through_the_chain`): a
//!   session display driven through every tic of the items the wire-25 simulation can carry, with
//!   each tic's draws outside the display taken from the session rows (sound, intermission; a
//!   fixture projection like tests/m5c_display.rs), against the rows' `M_Random` index after every
//!   tic and its frame. The display itself decides where the wipes fall. Levels after the first are
//!   carried by an idle wire-25 simulation of that map (the carried start is wire 26, built in
//!   another step), so their frames are not compared; the first level of each item is the
//!   genuine start and its frames are.
//! - Rust-only: `DDSP` version 3's round trip, forgeries and restore-and-continue, and the
//!   session's typed refusals.
mod common;
mod m6a;
#[path = "../../doom-sim/tests/wire25/mod.rs"]
mod wire25;

use doom_render::d_display::DisplayRestoreError;
use doom_render::wi_stuff::{self, IntermissionLoadError, IntermissionRefusal};
use doom_render::{
    Display, DisplayConfig, DisplayObserveError, DisplayRefusal, Frame, GameState,
    IntermissionGraphics, IntermissionView, Level, LevelChange, MeltState, NotYet, RandomContract,
    Resources, Scene, SessionDisplayState, SessionError, SessionRestoreError, StateDecodeError,
    WiState,
};
use doom_sim::demo::parse_demo_episode1_v2;
use doom_sim::{Simulation, Skill, TicCommand};
use doom_test_evidence::sha256;
use m6a::{Item, Session, items, read_frames, read_session, resources, root};
use std::collections::BTreeMap;
use std::sync::Arc;

#[test]
fn intermission_frames_against_the_genuine_digests() {
    let (mut compared, mut equal, mut genuine_rows) = (0usize, 0usize, 0usize);
    let mut first_mismatch = None;
    let (mut hidden, mut shown) = (0usize, 0usize);
    for item in items() {
        let s = read_session(&item.name);
        let res = resources(&item);
        for sb in [10u8, 11] {
            let frames = read_frames(&item.name, sb);
            genuine_rows += frames.values().filter(|r| r.gamestate == 1).count();
            if s.wi.is_empty() {
                continue;
            }
            let g = IntermissionGraphics::load(&res, 0).expect("graphics");
            let mut pointer = false;
            let mut screen = vec![0u8; 64000];
            for (tic, w) in &s.wi {
                let row = &frames[tic];
                assert_eq!(row.gamestate, 1, "{} tic {tic}", item.name);
                let before = m6a::pointer_before(w, pointer);
                if (w.state, w.cnt) == (1, 140) {
                    if before { shown += 1 } else { hidden += 1 }
                }
                let f = wi_stuff::draw(&res, &g, &w.view(before), &mut screen).expect("drawn");
                assert_eq!(
                    f.snl_pointeron, w.snl_pointeron,
                    "{} tic {tic}: snl_pointeron after the frame",
                    item.name
                );
                pointer = f.snl_pointeron;
                assert_eq!(row.palette[0], 0, "{} tic {tic}: palette", item.name);
                compared += 1;
                if sha256(&screen) == row.sha256 {
                    equal += 1;
                } else if first_mismatch.is_none() {
                    first_mismatch = Some((item.name.clone(), sb, *tic));
                }
            }
        }
    }
    eprintln!(
        "intermission frames: {compared} compared, {equal} equal, first mismatch {first_mismatch:?}; \
         ShowNextLoc entries with the pointer hidden {hidden}, shown {shown}"
    );
    assert_eq!(first_mismatch, None);
    assert_eq!(compared, genuine_rows, "every genuine intermission frame");
    assert_eq!(compared, INTERMISSION_FRAMES);
    assert_eq!((hidden, shown), POINTER_ENTRIES);
}

/// Intermission frames in the committed goldens (both screen sizes), and ShowNextLoc entry frames
/// with the pointer hidden (the first of each process) and shown (later ones), both screen sizes.
const INTERMISSION_FRAMES: usize = 5774;
const POINTER_ENTRIES: (usize, usize) = (30, 46);

/// The first WAD of an item holds its maps. With `carrier`, every W1 exit line (special 52,
/// g_game.c:1002-1006 through p_spec.c:677-680) is cleared, the one line special of the synthetic
/// maps that the wire-25 simulation refuses at load (`Pursuit(Specials)`): such a map only carries
/// the display through its tics, and its frames are not compared.
fn map_of(item: &Item, n: u8, carrier: bool) -> doom_map::Map {
    let mut bytes = std::fs::read(root(&item.wads[0])).expect("WAD");
    if carrier {
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("WAD");
        let lumps = wad.lumps();
        let at = lumps
            .iter()
            .position(|l| {
                l.name()
                    .as_bytes()
                    .starts_with(format!("E1M{n}").as_bytes())
            })
            .expect("map marker");
        let lines = lumps[at..]
            .iter()
            .find(|l| l.name().as_bytes().starts_with(b"LINEDEFS"))
            .expect("LINEDEFS")
            .range();
        drop(wad);
        for l in lines.step_by(14) {
            if bytes[l + 6..l + 8] == 52i16.to_le_bytes() {
                bytes[l + 6..l + 8].copy_from_slice(&0i16.to_le_bytes());
            }
        }
    }
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("WAD");
    doom_map::Map::from_wad(&wad, &format!("E1M{n}"), Default::default()).expect("map")
}

/// One item driven through the session display, tic by tic.
#[derive(Clone)]
struct Driver {
    item: Item,
    res: Arc<Resources>,
    session: Arc<Session>,
    frames: Arc<BTreeMap<i64, m6a::FrameRow>>,
    commands: Arc<Vec<TicCommand>>,
    skill: Skill,
    display: Display,
    sim: Simulation,
    level: Arc<Level>,
    /// The next session tic.
    tic: i64,
    /// Whether the current level is the item's genuine start (its frames are compared).
    genuine: bool,
    pointer: bool,
    frame: Frame,
}

/// What the drive compared.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Counts {
    tics: usize,
    index_equal: usize,
    wipes: usize,
    intermission_frames: usize,
    level_frames: usize,
    level_flagged: usize,
    finales: usize,
}

impl Driver {
    fn new(item: &Item) -> Option<Self> {
        let lmp = std::fs::read(root(&item.lmp)).expect("demo");
        let (header, demo) = parse_demo_episode1_v2(&lmp, item.map).ok()?;
        let skill =
            [Skill::Baby, Skill::Easy, Skill::Medium, Skill::Hard][usize::from(header.skill)];
        let res = resources(item);
        let session = read_session(&item.name);
        let first = session.tics.get(&0)?.map;
        let (sim, level, carrier) = Self::level(item, &res, first, skill);
        let mut display = Display::new_session(
            res.clone(),
            level.clone(),
            DisplayConfig { screenblocks: 10 },
        )
        .expect("display");
        display.begin_level(&sim).expect("level start");
        Some(Self {
            item: item.clone(),
            frames: Arc::new(read_frames(&item.name, 10)),
            session: Arc::new(session),
            commands: Arc::new(demo.commands),
            res,
            skill,
            display,
            sim,
            level,
            tic: 0,
            genuine: !carrier,
            pointer: false,
            frame: Frame::new(),
        })
    }

    /// The simulation and level of `map`, and whether it is a carrier ([`map_of`]).
    fn level(
        item: &Item,
        res: &Resources,
        map: u8,
        skill: Skill,
    ) -> (Simulation, Arc<Level>, bool) {
        let mut carrier = false;
        let (m, sim) = loop {
            let m = map_of(item, map, carrier);
            match wire25::build(true, m.clone(), map, skill) {
                Ok(sim) => break (m, sim),
                Err(e) if !carrier && e.contains("Pursuit(Specials)") => carrier = true,
                Err(e) => panic!("{} E1M{map}: {e}", item.name),
            }
        };
        let level = Arc::new(Level::load(res, &m, 1).expect("level"));
        (sim, level, carrier)
    }

    /// Runs the next session tic; false at the end of the rows or on reaching the finale.
    fn step(&mut self, n: &mut Counts) -> bool {
        let t = self.tic;
        let name = self.item.name.clone();
        let Some(g) = self.session.tics.get(&t).copied() else {
            return false;
        };
        let prev = self.session.tics.get(&(t - 1)).copied();
        let prev_state = prev.map_or(0, |p| p.gamestate);
        let before = prev.map_or([0; 6], |p| p.draws);
        let delta: [i64; 6] = std::array::from_fn(|i| g.draws[i] - before[i]);
        // The draws the display does not make itself: sound, intermission, finale, other.
        let outside = (delta[0] + delta[3] + delta[4] + delta[5]) as u16;
        assert!(
            delta[2] == 0 || delta[2] == 320,
            "{name} tic {t}: wipe draws"
        );
        match g.gamestate {
            0 => {
                if t > 0 {
                    if let Some(&map) = self.session.loads.get(&t) {
                        let change = if prev_state == 1 {
                            LevelChange::WorldDone
                        } else {
                            LevelChange::Reborn
                        };
                        let (sim, level, _) = Self::level(&self.item, &self.res, map, self.skill);
                        self.display
                            .begin_next_level(level.clone(), &sim, change)
                            .expect("next level");
                        (self.sim, self.level, self.genuine) = (sim, level, false);
                    } else {
                        assert_eq!(prev_state, 0, "{name} tic {t}: a level without a load");
                    }
                }
                assert_eq!(delta[1], 1, "{name} tic {t}: one ST_Ticker draw");
                assert_eq!(delta[3] + delta[4] + delta[5], 0, "{name} tic {t}");
                let cmd = if self.genuine {
                    self.commands[t as usize]
                } else {
                    TicCommand::default()
                };
                self.sim
                    .advance([
                        cmd,
                        Default::default(),
                        Default::default(),
                        Default::default(),
                    ])
                    .expect("tic");
                self.display
                    .observe_after(&self.sim, &cmd, outside)
                    .expect("observed");
                let wipe = self.display.session_state().expect("state").wipe;
                assert_eq!(wipe, delta[2] == 320, "{name} tic {t}: wipe");
                n.wipes += usize::from(wipe);
                let f = self
                    .display
                    .draw(&self.sim, &mut self.frame)
                    .expect("drawn");
                if self.genuine {
                    let row = &self.frames[&t];
                    assert_eq!(row.gamestate, 0);
                    if row.sha256 == "undefined" {
                        n.level_flagged += 1;
                    } else {
                        assert_eq!(sha256(self.frame.pixels()), row.sha256, "{name} tic {t}");
                        n.level_frames += 1;
                    }
                    let set = f.palette_set.map_or(-1, i64::from);
                    assert_eq!(
                        [
                            i64::from(f.palette),
                            set,
                            i64::from(self.display.random_index())
                        ],
                        row.palette,
                        "{name} tic {t}: P row"
                    );
                }
            }
            1 => {
                if prev_state == 0 {
                    self.display.begin_intermission().expect("intermission");
                }
                self.display.observe_non_level(outside).expect("observed");
                let wipe = self.display.session_state().expect("state").wipe;
                assert_eq!(wipe, delta[2] == 320, "{name} tic {t}: wipe");
                n.wipes += usize::from(wipe);
                let w = &self.session.wi[&t];
                let view = w.view(m6a::pointer_before(w, self.pointer));
                let f = self
                    .display
                    .draw_intermission(&view, &mut self.frame)
                    .expect("drawn");
                self.pointer = f.snl_pointeron;
                let row = &self.frames[&t];
                assert_eq!(sha256(self.frame.pixels()), row.sha256, "{name} tic {t}");
                let set = f.palette_set.map_or(-1, i64::from);
                assert_eq!(
                    [
                        i64::from(f.palette),
                        set,
                        i64::from(self.display.random_index())
                    ],
                    row.palette,
                    "{name} tic {t}: G row"
                );
                n.intermission_frames += 1;
            }
            2 => {
                assert_eq!(prev_state, 0, "{name}: the finale follows a level");
                self.display.begin_finale().expect("finale");
                self.display.observe_non_level(outside).expect("observed");
                let view = doom_render::FinaleView { stage: 0, count: 1 };
                assert_eq!(
                    self.display.draw_finale(&view, &mut self.frame),
                    Err(SessionError::NotYet(NotYet::FinaleDrawer))
                );
                n.finales += 1;
                return false;
            }
            s => panic!("{name} tic {t}: gamestate {s}"),
        }
        n.tics += 1;
        assert_eq!(
            self.display.random_index(),
            g.rndindex,
            "{name} tic {t}: M_Random index"
        );
        n.index_equal += 1;
        assert_eq!(self.display.session_tic(), Some(t as u64 + 1));
        self.tic += 1;
        true
    }
}

#[test]
fn random_index_through_the_chain() {
    let mut n = Counts::default();
    let mut driven = Vec::new();
    let mut not_driven = Vec::new();
    for item in items() {
        let Some(mut d) = Driver::new(&item) else {
            not_driven.push(item.name.clone());
            continue;
        };
        while d.step(&mut n) {}
        driven.push(item.name.clone());
    }
    eprintln!(
        "driven {} items {driven:?}; not driven {not_driven:?}; {n:?}",
        driven.len()
    );
    assert_eq!(n.index_equal, n.tics);
    assert_eq!(n, CHAIN_COUNTS);
    assert_eq!(not_driven, NOT_DRIVEN);
}

/// What the drive of the 23 items compares (screenblocks 10): every tic's index, the 102 wipes up
/// to the first finale frame, every intermission frame through the display, and the level frames
/// of the items whose start map the wire-25 simulation admits unmodified.
const CHAIN_COUNTS: Counts = Counts {
    tics: 9724,
    index_equal: 9724,
    wipes: 102,
    intermission_frames: 2855,
    level_frames: 2234,
    level_flagged: 0,
    finales: 3,
};
/// Items not driven: the item that stops before its first tic (NOE1M2AT), and the two easy
/// skills, whose demos the version-2 demo parser refuses (skill bytes 0 and 1; the easy skills are
/// M6a's step 10, U12).
const NOT_DRIVEN: [&str; 3] = ["NOE1M2AT", "SKILL0", "SKILL1"];

fn intfast() -> Driver {
    let item = items()
        .into_iter()
        .find(|i| i.name == "INTFAST")
        .expect("item");
    Driver::new(&item).expect("driver")
}

/// Drives `d` to session tic `t` (not yet run).
fn to_tic(d: &mut Driver, t: i64) {
    let mut n = Counts::default();
    while d.tic < t {
        assert!(d.step(&mut n));
    }
}

/// A fresh session display of the same item, restored from `state`.
fn restored(d: &Driver, state: SessionDisplayState) -> Display {
    let mut fresh = Display::new_session(
        d.res.clone(),
        d.level.clone(),
        DisplayConfig { screenblocks: 10 },
    )
    .expect("display");
    let level = matches!(state.scene, Scene::Level(_)).then(|| (d.level.clone(), &d.sim));
    fresh.restore_session(state, level).expect("restored");
    fresh
}

/// `DDSP` version 3 round-trips at every kind of point of a session (level start, mid-level, the
/// pending intermission, the intermission, the next level's start, mid-level again), restores into
/// a fresh session display, and the restored display continues exactly as the uninterrupted one
/// (every later tic's encoding equal, frames and indices checked by the driver).
#[test]
fn session_state_round_trip_and_restore_and_continue() {
    let mut d = intfast();
    let mut points = 0;
    // INTFAST: E1M7 from tic 0, the exit on tic 55, the intermission on tics 56-68, E1M8 from 69.
    for t in [0i64, 1, 2, 30, 56, 57, 60, 69, 70, 75] {
        to_tic(&mut d, t);
        let state = d.display.session_state().expect("state");
        let bytes = state.encode();
        assert_eq!(&bytes[..8], b"DDSP\x03\0\0\0");
        let decoded = SessionDisplayState::decode(&bytes).expect("decoded");
        assert_eq!(decoded, state);
        assert_eq!(decoded.encode(), bytes);
        state.check().expect("checked");
        let mut a = d.clone();
        let mut b = d.clone();
        b.display = restored(&d, decoded);
        assert_eq!(b.display.session_state().expect("state").encode(), bytes);
        let mut n = Counts::default();
        for _ in 0..12 {
            let more = a.step(&mut n);
            assert_eq!(b.step(&mut n), more);
            assert_eq!(
                a.display.session_state().map(|s| s.encode()),
                b.display.session_state().map(|s| s.encode()),
                "continued from tic {t}"
            );
            if !more {
                break;
            }
        }
        points += 1;
    }
    assert_eq!(points, 10);
    // The pending intermission (entered, not yet observed): the wipe is due at 0 tics since entry.
    let mut p = intfast();
    to_tic(&mut p, 56);
    p.display.begin_intermission().expect("intermission");
    let state = p.display.session_state().expect("state");
    assert!(state.wipe && state.tic == state.entered);
    assert_eq!(state.scene.game_state(), GameState::Intermission);
    assert_eq!(
        SessionDisplayState::decode(&state.encode()).expect("decoded"),
        state
    );
}

/// Forged or impossible version-3 states are refused by the decoder or by `check`, and the
/// version-1/2 decoders and the version-3 decoder refuse each other's versions.
#[test]
fn session_state_forgeries() {
    let mut d = intfast();
    to_tic(&mut d, 60);
    let inter = d.display.session_state().expect("state");
    to_tic(&mut d, 75);
    let level = d.display.session_state().expect("state");
    assert_eq!((level.tic, level.entered), (75, 69));
    let incons = |s: &SessionDisplayState| match s.check() {
        Err(SessionRestoreError::Display(DisplayRestoreError::Inconsistent(e))) => e,
        r => panic!("{r:?}"),
    };
    let mut s = level.clone();
    s.wipe = true;
    assert_eq!(incons(&s), "wipe");
    let mut s = level.clone();
    s.entered = s.tic;
    assert_eq!(incons(&s), "wipe");
    let mut s = level.clone();
    s.entered -= 1;
    assert_eq!(incons(&s), "level tic");
    let mut s = level.clone();
    s.old_gamestate = Some(GameState::Intermission);
    assert_eq!(incons(&s), "old gamestate");
    let mut s = level.clone();
    s.old_gamestate = None;
    s.check().expect("a screen size change leaves -1");
    let mut s = inter.clone();
    if let Scene::Intermission(c) = &mut s.scene {
        c.taint[5] = 1;
    }
    assert_eq!(incons(&s), "carried taint");
    let mut s = inter.clone();
    s.hud.message_on = true;
    s.hud.message_counter = 0;
    assert_eq!(incons(&s), "message_on and message_counter");
    // The melt in progress: a layout that round-trips, refused as not yet built.
    let mut s = inter.clone();
    s.melt = Some(MeltState {
        step: 3,
        start: vec![7; 64000],
        start_taint: vec![0; 64000],
    });
    let bytes = s.encode();
    assert_eq!(SessionDisplayState::decode(&bytes).expect("decoded"), s);
    assert_eq!(
        s.check(),
        Err(SessionRestoreError::NotYet(NotYet::MeltPictures))
    );
    let mut fresh = restored(&d, inter.clone());
    assert_eq!(
        fresh.restore_session(s, None),
        Err(SessionRestoreError::NotYet(NotYet::MeltPictures))
    );
    // Decoder bounds.
    let good = inter.encode();
    let dec = SessionDisplayState::decode;
    let mut b = good.clone();
    b[4] = 2;
    assert_eq!(dec(&b), Err(StateDecodeError::Version(2)));
    assert_eq!(
        doom_render::DisplayState::decode_any(&good),
        Err(StateDecodeError::Version(3))
    );
    let mut b = good.clone();
    b[24] = 3;
    assert_eq!(dec(&b), Err(StateDecodeError::Invalid("gamestate")));
    let mut b = good.clone();
    b[25] = 7;
    assert_eq!(dec(&b), Err(StateDecodeError::Invalid("old gamestate")));
    let mut b = good.clone();
    b[8..16].copy_from_slice(&0u64.to_le_bytes());
    assert_eq!(dec(&b), Err(StateDecodeError::Invalid("entered")));
    let mut b = good.clone();
    b[26..30].copy_from_slice(&50u32.to_le_bytes());
    assert_eq!(dec(&b), Err(StateDecodeError::Invalid("fuzzpos")));
    let melt_at = 26 + 4 + 16 + 64000 + 4 + 8;
    assert_eq!(good[melt_at], 0, "one taint run in a drawn intermission");
    let mut b = good.clone();
    b[melt_at] = 2;
    assert_eq!(dec(&b), Err(StateDecodeError::Invalid("melt")));
    assert_eq!(
        dec(&good[..good.len() - 1]),
        Err(StateDecodeError::Truncated)
    );
    let mut b = good.clone();
    b.push(0);
    assert_eq!(dec(&b), Err(StateDecodeError::Trailing));
    let mut b = good.clone();
    b[0] = b'X';
    assert_eq!(dec(&b), Err(StateDecodeError::Magic));
    // The level argument must fit the scene.
    let mut fresh = restored(&d, inter.clone());
    assert_eq!(
        fresh.restore_session(level.clone(), None),
        Err(SessionRestoreError::Level)
    );
    assert_eq!(
        fresh.restore_session(inter.clone(), Some((d.level.clone(), &d.sim))),
        Err(SessionRestoreError::Level)
    );
    assert_eq!(fresh.session_state(), Some(inter));
}

/// The session's typed refusals, each leaving the state unchanged, and the palette rule after a
/// screen size change (d_main.c:215-220, 274-275).
#[test]
fn session_refusals_and_rules() {
    let mut d = intfast();
    to_tic(&mut d, 30);
    let enc = |d: &Display| d.session_state().expect("state").encode();
    let before = enc(&d.display);
    let mut f = Frame::new();
    let cmd = TicCommand::default();
    assert_eq!(
        d.display.begin_level(&d.sim),
        Err(DisplayObserveError::WrongGameState)
    );
    assert_eq!(
        d.display.set_contract(RandomContract::SoundFreeWipeFree),
        Err(DisplayObserveError::Contract)
    );
    assert!(
        d.display.state().is_none(),
        "a session display saves version 3 only"
    );
    let w = SessionError::WrongGameState(GameState::Level);
    assert_eq!(d.display.observe_non_level(0), Err(w.clone()));
    let view = IntermissionView {
        state: WiState::StatCount,
        cnt_kills: -1,
        cnt_items: -1,
        cnt_secret: -1,
        cnt_time: -1,
        cnt_par: -1,
        snl_pointeron: false,
        epsd: 0,
        last: 6,
        next: 7,
        didsecret: false,
    };
    assert_eq!(d.display.draw_intermission(&view, &mut f), Err(w.clone()));
    let fv = doom_render::FinaleView { stage: 0, count: 0 };
    assert_eq!(d.display.draw_finale(&fv, &mut f), Err(w.clone()));
    let (sim2, level2, _) = Driver::level(&d.item, &d.res, 8, d.skill);
    assert_eq!(
        d.display
            .begin_next_level(level2.clone(), &sim2, LevelChange::WorldDone),
        Err(w.clone())
    );
    assert_eq!(enc(&d.display), before);
    // The finale: entered from the level, observed, its drawer not built yet.
    let mut fin = d.clone();
    fin.display.begin_finale().expect("finale");
    fin.display.observe_non_level(0).expect("observed");
    let pending = enc(&fin.display);
    assert_eq!(
        fin.display.draw_finale(&fv, &mut f),
        Err(SessionError::NotYet(NotYet::FinaleDrawer))
    );
    assert_eq!(enc(&fin.display), pending, "the wipe stays due");
    fin.display.begin_finale_art().expect("forced wipe");
    assert_eq!(
        fin.display.begin_intermission(),
        Err(SessionError::WrongGameState(GameState::Finale))
    );
    // In the intermission: level calls are refused.
    to_tic(&mut d, 60);
    let before = enc(&d.display);
    assert_eq!(
        d.display.observe_after(&d.sim, &cmd, 0),
        Err(DisplayObserveError::WrongGameState)
    );
    assert_eq!(
        d.display.draw(&d.sim, &mut f).err(),
        Some(DisplayRefusal::WrongGameState)
    );
    let wi = SessionError::WrongGameState(GameState::Intermission);
    assert_eq!(d.display.begin_intermission(), Err(wi.clone()));
    assert_eq!(d.display.begin_finale_art(), Err(wi.clone()));
    assert_eq!(
        d.display
            .begin_next_level(level2, &sim2, LevelChange::Reborn),
        Err(wi)
    );
    let mut bad = view;
    bad.epsd = 1;
    assert_eq!(
        d.display.draw_intermission(&bad, &mut f),
        Err(SessionError::Intermission(IntermissionRefusal::Episode(1)))
    );
    bad.epsd = 0;
    bad.next = 9;
    assert_eq!(
        d.display.draw_intermission(&bad, &mut f),
        Err(SessionError::Intermission(IntermissionRefusal::Map("next")))
    );
    assert_eq!(enc(&d.display), before);
    // A screen size change sets oldgamestate to -1: the next frame sets palette 0 again.
    d.display.set_screenblocks(11).expect("size");
    d.display.observe_non_level(0).expect("observed");
    let nf = d.display.draw_intermission(&view, &mut f).expect("drawn");
    assert_eq!((nf.palette, nf.palette_set), (0, Some(0)));
    d.display.observe_non_level(0).expect("observed");
    let nf = d.display.draw_intermission(&view, &mut f).expect("drawn");
    assert_eq!(nf.palette_set, None);
    // A display that is not a session display.
    let mut plain = Display::new(
        d.res.clone(),
        d.level.clone(),
        DisplayConfig { screenblocks: 10 },
    )
    .expect("display");
    assert_eq!(plain.begin_intermission(), Err(SessionError::NotSession));
    assert_eq!(plain.session_state(), None);
    assert_eq!(plain.game_state(), None);
    let st = d.display.session_state().expect("state");
    assert_eq!(
        plain.restore_session(st, None),
        Err(SessionRestoreError::NotSession)
    );
}

/// The drawer's own rules on the synthetic pictures: "sucks" above 61*59 seconds
/// (wi_stuff.c:700-720), a splat that does not fit refused (wi_stuff.c:485-487), a pointer that
/// does not fit not drawn (wi_stuff.c:494-498), and a missing name refused at load in
/// `WI_loadData`'s order (wi_stuff.c:1557).
#[test]
fn drawer_rules() {
    let item = items()
        .into_iter()
        .find(|i| i.name == "INTFAST")
        .expect("item");
    let res = resources(&item);
    let g = IntermissionGraphics::load(&res, 0).expect("graphics");
    let draw = |res: &Resources, g: &IntermissionGraphics, v: &IntermissionView| {
        let mut s = vec![0u8; 64000];
        wi_stuff::draw(res, g, v, &mut s).map(|_| sha256(&s))
    };
    let mut v = IntermissionView {
        state: WiState::StatCount,
        cnt_kills: 50,
        cnt_items: 75,
        cnt_secret: 100,
        cnt_time: 3599,
        cnt_par: 30,
        snl_pointeron: false,
        epsd: 0,
        last: 0,
        next: 1,
        didsecret: false,
    };
    let at_limit = draw(&res, &g, &v).expect("drawn");
    v.cnt_time = 3600;
    let sucks = draw(&res, &g, &v).expect("drawn");
    v.cnt_time = 1_000_000;
    assert_eq!(draw(&res, &g, &v).expect("drawn"), sucks);
    assert_ne!(at_limit, sucks);
    v.cnt_time = -1;
    let hidden = draw(&res, &g, &v).expect("drawn");
    assert_ne!(hidden, sucks);
    // A patch 320 wide and one row high (every column the same one-pixel post) fits at no node.
    let mut wide = Vec::new();
    for x in [320i16, 1, 0, 0] {
        wide.extend(x.to_le_bytes());
    }
    let post: i32 = 8 + 4 * 320;
    for _ in 0..320 {
        wide.extend(post.to_le_bytes());
    }
    wide.extend([0u8, 1, 0, 4, 0, 0xff]);
    let with = |names: &[&str]| {
        let files: Vec<Vec<u8>> = item
            .wads
            .iter()
            .map(|w| std::fs::read(root(w)).expect("WAD"))
            .collect();
        let mut dir = doom_render::LumpDirectory::new();
        for b in &files {
            dir.add_wad(&doom_wad::Wad::parse(b, doom_wad::Limits::default()).expect("WAD"))
                .expect("lumps");
        }
        for n in names {
            dir.add_single_lump(n, &wide).expect("lump");
        }
        Resources::load(dir, doom_render::GameMode::Registered).expect("resources")
    };
    let v = IntermissionView {
        state: WiState::ShowNextLoc,
        snl_pointeron: true,
        ..v
    };
    let off = IntermissionView {
        snl_pointeron: false,
        ..v
    };
    assert_ne!(
        draw(&res, &g, &v),
        draw(&res, &g, &off),
        "the pointer is drawn"
    );
    // The splat's array has one element: refused, not read past.
    let r = with(&["WISPLAT"]);
    let gs = IntermissionGraphics::load(&r, 0).expect("graphics");
    assert_eq!(
        draw(&r, &gs, &v),
        Err(IntermissionRefusal::SplatDoesNotFit { node: 0 })
    );
    // The pointer's array has two: WIURH1 when WIURH0 does not fit, nothing when neither does.
    let r = with(&["WIURH0"]);
    let gy = IntermissionGraphics::load(&r, 0).expect("graphics");
    assert_ne!(
        draw(&r, &gy, &v),
        draw(&res, &g, &v),
        "WIURH1 drawn instead"
    );
    let r = with(&["WIURH0", "WIURH1"]);
    let gy = IntermissionGraphics::load(&r, 0).expect("graphics");
    assert_eq!(draw(&r, &gy, &v), draw(&res, &g, &off), "no pointer");
    // A missing picture: the episode WAD alone lacks WIMAP0, the first name WI_loadData caches.
    let mut bare = item.clone();
    bare.wads.retain(|w| !w.ends_with("m6a_gfx.wad"));
    assert_eq!(
        IntermissionGraphics::load(&resources(&bare), 0).err(),
        Some(IntermissionLoadError::Resource(
            doom_render::ResourceError::LumpNotFound("WIMAP0".into())
        ))
    );
    assert_eq!(
        IntermissionGraphics::load(&res, 1).err(),
        Some(IntermissionLoadError::Episode(1))
    );
}

/// The comparisons can fail: a planted fault in each is caught. In the drawer, the pointer drawn
/// the other way on a ShowNextLoc frame and a kill count off by two on a counted statistics
/// frame; in the accounting, a level change started as a new game (`begin_level`, which clears
/// the index as `M_ClearRandom` does) instead of keeping the index.
#[test]
fn the_comparisons_can_fail() {
    let item = items()
        .into_iter()
        .find(|i| i.name == "INTSTA")
        .expect("item");
    let (s, res) = (read_session(&item.name), resources(&item));
    let frames = read_frames(&item.name, 10);
    let g = IntermissionGraphics::load(&res, 0).expect("graphics");
    let mut screen = vec![0u8; 64000];
    let mut caught = [false; 2];
    let mut pointer = false;
    for (tic, w) in &s.wi {
        let before = m6a::pointer_before(w, pointer);
        let mut v = w.view(before);
        pointer = wi_stuff::draw(&res, &g, &v, &mut screen)
            .expect("drawn")
            .snl_pointeron;
        if v.state == WiState::ShowNextLoc {
            v.snl_pointeron = !v.snl_pointeron;
            wi_stuff::draw(&res, &g, &v, &mut screen).expect("drawn");
            caught[0] |= sha256(&screen) != frames[tic].sha256;
        } else if v.state == WiState::StatCount && v.cnt_kills >= 0 {
            v.cnt_kills += 2;
            wi_stuff::draw(&res, &g, &v, &mut screen).expect("drawn");
            caught[1] |= sha256(&screen) != frames[tic].sha256;
        }
    }
    assert_eq!(caught, [true, true]);
    // INTFAST loads E1M8 on tic 69: a new game there would clear the index.
    let mut d = intfast();
    to_tic(&mut d, 69);
    let (sim, level, _) = Driver::level(&d.item, &d.res, 8, d.skill);
    let mut kept = d.display.clone();
    kept.begin_next_level(level.clone(), &sim, LevelChange::WorldDone)
        .expect("kept");
    let mut cleared =
        Display::new_session(d.res.clone(), level, DisplayConfig { screenblocks: 10 })
            .expect("display");
    cleared.begin_level(&sim).expect("new game");
    let row = d.session.tics[&69];
    let draws = (row.draws[0] - d.session.tics[&68].draws[0]) as u16;
    let mut results = Vec::new();
    for disp in [&mut kept, &mut cleared] {
        let mut sim = sim.clone();
        let cmd = TicCommand::default();
        sim.advance([
            cmd,
            Default::default(),
            Default::default(),
            Default::default(),
        ])
        .expect("tic");
        disp.observe_after(&sim, &cmd, draws).expect("observed");
        disp.draw(&sim, &mut d.frame).expect("drawn");
        results.push(disp.random_index() == row.rndindex);
    }
    assert_eq!(results, [true, false]);
}
