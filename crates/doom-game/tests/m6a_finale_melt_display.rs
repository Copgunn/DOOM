//! M6a part 2 steps 3 and 4 (display): the finale-and-melt configuration of the session display
//! (`doom_render::SessionConfig::FinaleAndMelt`, `DDSP` version 4) against the committed goldens of
//! the genuine linuxdoom-1.10 engine (`tests/reference/m6a_frames`, `m6a_melt`, `m6a_session`,
//! written by `tools/reference/m6a_goldens.py`; no Rust output feeds an expectation).
//!
//! **How the display is driven.** The display under test is not yet reached by the session (that
//! is the next step), so each item's frozen session (wire 26, the intermission logic, its own
//! frozen session display) runs from the committed demo, and a second display, in the new
//! configuration, is driven in lockstep from what the session exposes after each tic: the
//! simulation (`Session::level`), the tic's sound and intermission draws (`TicReport`), the
//! intermission state (`Session::intermission`, `wminfo`), and at a load the simulation before
//! its tic 0, built here exactly as the session builds it from its carried player
//! (`Session::carried`, the `P_Random` index and `gametic`). From the E1M8 victory tic, which the
//! frozen session refuses, the finale logic (`doom_game::session::finale`, step 2) drives the
//! finale; its tics draw no `M_Random` (M6a note §1.8, asserted through the index). Nothing is
//! projected from a golden into the display; every comparison is against the genuine rows.
//!
//! Compared, at screenblocks 10: every frame's digest, palette and palette set (frames F/P and G
//! rows), the `M_Random` index after every tic (session G rows), and every melt: its tic, start
//! and end screen digests, palette and palette set (melt Y rows), its step count and every step's
//! picture digest and `done` flag (V rows); at screenblocks 11 every frame's digest and the index.
//! `NOHELP2`'s stop is the display's typed refusal at the genuine tic with the genuine message.
mod m6a;

use doom_game::session::demo::SessionDemo;
use doom_game::session::finale::{self, FinaleView};
use doom_game::session::{NotYet, Session, SessionSettings, SessionWitness, TicKind};
use doom_render::{
    Display, DisplayConfig, Frame, GameState, Level, LevelChange, MeltFault, Scene, SessionConfig,
    SessionDisplayStateV4, SessionError,
};
use doom_sim::{CarriedStart, LevelStart, MapContextOptions, Simulation, Skill, TicCommand};
use doom_test_evidence::sha256;
use m6a::{Item, Rows, int};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A map and its renderer level.
type MapLevel = (doom_map::Map, Arc<Level>);

/// One melt as the display showed it.
#[derive(Clone, Debug, Default, PartialEq)]
struct ShownMelt {
    start: String,
    end: String,
    palette: u8,
    palette_set: i64,
    steps: Vec<(String, bool)>,
}

/// What one tic presented.
#[derive(Clone, Debug, PartialEq)]
struct Shown {
    gamestate: i64,
    sha256: String,
    palette: u8,
    palette_set: i64,
    rndindex: u8,
    melt: Option<ShownMelt>,
}

/// A planted fault outside the display (the display's own are [`MeltFault`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DriverFault {
    /// The finale's art screen shown one tic early: `F_Ticker`'s stage test as `>=` instead of
    /// `>` (f_finale.c:241).
    FinaleStageEarly,
}

/// One item's frozen session with the new-configuration display beside it.
#[derive(Clone)]
struct Lockstep {
    item: Item,
    session: Session,
    display: Display,
    levels: Arc<Vec<Option<MapLevel>>>,
    skill: Skill,
    screenblocks: u8,
    commands: Arc<Vec<TicCommand>>,
    finale: Option<FinaleView>,
    frame: Frame,
    /// The next session tic.
    tic: usize,
    driver_fault: Option<DriverFault>,
}

/// Why a drive stopped.
#[derive(Clone, Debug, PartialEq)]
enum Stop {
    /// The demo ended.
    End,
    /// The genuine run's last tic was reached.
    GenuineEnd,
    /// A refusal at a tic, with its text.
    Refused(usize, String),
}

impl Lockstep {
    fn new(item: &Item, screenblocks: u8) -> Result<Self, String> {
        let lmp = std::fs::read(m6a::root(&item.lmp)).expect("demo");
        let demo = SessionDemo::parse(&lmp).map_err(|e| format!("demo refused: {e:?}"))?;
        let data = m6a::data(item, None);
        let settings = SessionSettings {
            map: item.map,
            skill: item.skill,
            screenblocks,
        };
        let session =
            Session::new(data.clone(), settings).map_err(|e| format!("no session: {e:?}"))?;
        let skill = match item.skill {
            2 => Skill::Medium,
            3 => Skill::Hard,
            s => return Err(format!("skill {s}")),
        };
        let res = data.resources().clone();
        let levels = Arc::new(levels(item, &res));
        let mode = data.mode().mode.game_mode();
        let first = levels[usize::from(item.map) - 1]
            .as_ref()
            .ok_or("no start map")?
            .1
            .clone();
        let mut display = Display::new_session_with(
            res,
            first,
            DisplayConfig { screenblocks },
            SessionConfig::FinaleAndMelt { mode },
        )
        .expect("display");
        display.begin_level(session.level()).expect("level start");
        Ok(Self {
            item: item.clone(),
            session,
            display,
            levels,
            skill,
            screenblocks,
            commands: Arc::new(demo.commands),
            finale: None,
            frame: Frame::new(),
            tic: 0,
            driver_fault: None,
        })
    }

    /// The simulation before its tic 0 of the load the session is about to run, built as
    /// `Session::world_done_tic` and the reborn branch of `Session::tic` build it, with the level
    /// change and the carried `deltaviewheight`.
    fn pending_load(&self) -> Option<(Arc<Level>, Simulation, LevelChange, i32)> {
        let s = &self.session;
        let p_random = s.level().rng().indices()[0];
        let (map, start, change, dvh) = match s.gamestate() {
            GameState::Level if s.level().reborn_requested() => (
                s.map(),
                LevelStart::Reborn { p_random },
                LevelChange::Reborn,
                0,
            ),
            GameState::Intermission if s.gameaction() == 8 => {
                let c = s.carried()?;
                let map = u8::try_from(s.wminfo().next + 1).ok()?;
                let (start, dvh) = match c.start {
                    Some(cs) => (
                        LevelStart::Carried(CarriedStart {
                            attack_down: c.latches.attackdown,
                            use_down: c.latches.usedown,
                            p_random,
                            ..cs
                        }),
                        c.deltaviewheight,
                    ),
                    None => (LevelStart::Reborn { p_random }, 0),
                };
                (map, start, LevelChange::WorldDone, dvh)
            }
            _ => return None,
        };
        let (m, level) = self
            .levels
            .get(usize::from(map).checked_sub(1)?)?
            .as_ref()?;
        let sim = Simulation::original_player_episode1_v4_level_at(
            m.clone(),
            map,
            self.skill,
            MapContextOptions::default(),
            start,
            s.gametic(),
        )
        .ok()?;
        Some((level.clone(), sim, change, dvh))
    }

    /// Runs the next session tic through both, up to and including the display's frame (a melt
    /// it starts is not stepped): the frame's game state, palette and palette set.
    fn tic_to_display(&mut self) -> Result<(i64, u8, Option<u8>), Stop> {
        let t = self.tic;
        let Some(&cmd) = self.commands.get(t) else {
            return Err(Stop::End);
        };
        if self.finale.is_none()
            && self.item.end.starts_with("error")
            && t >= self.item.session_tics
        {
            return Err(Stop::GenuineEnd);
        }
        let drawn = if self.finale.is_some() {
            self.finale_tic(t)?
        } else {
            let load = self.pending_load();
            match self.session.tic(cmd) {
                Ok(r) => {
                    let d = &mut self.display;
                    let level_draws = r.sound.sound.draws;
                    match r.kind {
                        TicKind::Load { .. } => {
                            let (level, sim, change, dvh) = load.expect("a load was predicted");
                            d.begin_next_level_carried(level, &sim, change, dvh)
                                .expect("next level");
                        }
                        TicKind::Completion => d.begin_intermission().expect("intermission"),
                        _ => assert!(load.is_none(), "{} tic {t}: no load", self.item.name),
                    }
                    if self.session.gamestate() == GameState::Level {
                        d.observe_after(self.session.level(), &cmd, level_draws)
                            .expect("observed");
                        let f = d
                            .draw(self.session.level(), &mut self.frame)
                            .expect("drawn");
                        (0, f.palette, f.palette_set)
                    } else {
                        d.observe_non_level(r.wi_draws + level_draws)
                            .expect("observed");
                        let view = self.session.intermission().view(self.session.wminfo());
                        let f = d.draw_intermission(&view, &mut self.frame).expect("drawn");
                        (1, f.palette, f.palette_set)
                    }
                }
                Err(e)
                    if e.witness == SessionWitness::NotYet(NotYet::Victory)
                        && t < self.item.session_tics =>
                {
                    self.display.begin_finale().expect("finale");
                    self.finale = Some(finale::start(1).expect("episode 1"));
                    self.finale_tic(t)?
                }
                Err(e) => return Err(Stop::Refused(t, e.to_string())),
            }
        };
        self.tic += 1;
        Ok(drawn)
    }

    /// Runs the next session tic whole: its frame and every step of the melt it starts.
    fn step(&mut self) -> Result<Shown, Stop> {
        let (gamestate, palette, palette_set) = self.tic_to_display()?;
        Ok(self.shown(gamestate, palette, palette_set))
    }

    /// One finale tic: `F_Ticker`, its forced wipe, then `D_Display`.
    fn finale_tic(&mut self, t: usize) -> Result<(i64, u8, Option<u8>), Stop> {
        let view = self.finale.as_mut().expect("in the finale");
        let mut tick = finale::tick(view).expect("finale tic");
        if self.driver_fault == Some(DriverFault::FinaleStageEarly)
            && *view
                == (FinaleView {
                    stage: 0,
                    count: finale::STAGE_THRESHOLD,
                })
        {
            *view = FinaleView { stage: 1, count: 0 };
            tick.wipe = true;
        }
        let view = *view;
        let d = &mut self.display;
        if tick.wipe {
            d.begin_finale_art().expect("forced wipe");
        }
        d.observe_non_level(0).expect("observed");
        match d.draw_finale(&view, &mut self.frame) {
            Ok(f) => Ok((2, f.palette, f.palette_set)),
            Err(SessionError::Finale(e)) => Err(Stop::Refused(t, e.to_string())),
            Err(e) => panic!("{} tic {t}: {e:?}", self.item.name),
        }
    }

    /// The tic's frame and, when it melts, every melt step.
    fn shown(&mut self, gamestate: i64, palette: u8, palette_set: Option<u8>) -> Shown {
        let palette_set = palette_set.map_or(-1, i64::from);
        let sha = sha256(self.frame.pixels());
        let melt = self.display.melt_in_progress().then(|| {
            let (s, e) = self.display.melt_screens().expect("melt");
            let mut m = ShownMelt {
                start: sha256(s),
                end: sha256(e),
                palette: self.display.melt_palette().expect("melt"),
                palette_set,
                steps: Vec::new(),
            };
            let mut picture = Frame::new();
            loop {
                let st = self.display.melt_step(&mut picture).expect("melt step");
                assert_eq!(usize::from(st.step), m.steps.len() + 1);
                m.steps.push((sha256(picture.pixels()), st.done));
                if st.done {
                    break;
                }
            }
            m
        });
        Shown {
            gamestate,
            sha256: sha,
            palette,
            palette_set,
            rndindex: self.display.random_index(),
            melt,
        }
    }
}

/// The nine maps of an item and their levels, loaded as `SessionData` loads them (None for a map
/// the WADs lack or that does not load).
fn levels(item: &Item, res: &doom_render::Resources) -> Vec<Option<MapLevel>> {
    let files: Vec<Vec<u8>> = item
        .wads
        .iter()
        .map(|w| std::fs::read(m6a::root(w)).expect("WAD"))
        .collect();
    let parsed: Vec<doom_wad::Wad> = files
        .iter()
        .map(|b| doom_wad::Wad::parse(b, doom_wad::Limits::default()).expect("WAD"))
        .collect();
    let set = doom_wad::WadSet::new(&parsed);
    (1..=9u8)
        .map(|n| {
            let map = doom_map::Map::load(&set, &format!("E1M{n}"), doom_map::MapLimits::default())
                .ok()?;
            let level = Level::load(res, &map, 1).ok()?;
            Some((map, Arc::new(level)))
        })
        .collect()
}

/// One melt of a melt golden.
#[derive(Clone, Debug)]
struct GoldenMelt {
    start: String,
    end: String,
    palette: i64,
    palette_set: i64,
    steps: Vec<(String, bool)>,
}

fn golden_melts(name: &str) -> BTreeMap<i64, GoldenMelt> {
    let rows = Rows::read(&format!("m6a_melt/{name}.melt"));
    let mut melts: BTreeMap<i64, GoldenMelt> = rows
        .of("Y")
        .iter()
        .map(|y| {
            assert_eq!(y["clock"], "unit", "{name}: --melt-clock unit");
            (
                int(y, "tic"),
                GoldenMelt {
                    start: y["start_sha256"].clone(),
                    end: y["end_sha256"].clone(),
                    palette: int(y, "palette"),
                    palette_set: int(y, "palette_set"),
                    steps: Vec::new(),
                },
            )
        })
        .collect();
    for v in rows.of("V") {
        let m = melts.get_mut(&int(v, "tic")).expect("a V row of a Y row");
        assert_eq!(int(v, "step"), m.steps.len() as i64 + 1, "{name}: order");
        assert_eq!(int(v, "tics"), 1, "{name}: one tick per step");
        m.steps.push((v["sha256"].clone(), int(v, "done") == 1));
    }
    melts
}

/// What a drive compared.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Counts {
    tics: usize,
    level_frames: usize,
    intermission_frames: usize,
    finale_frames: usize,
    index: usize,
    melts: usize,
    melt_steps: usize,
}

/// Drives `item` at `screenblocks` (melts compared only at 10, the melt goldens' size) and
/// compares every tic; Err names the first difference.
fn drive(
    item: &Item,
    screenblocks: u8,
    melt_fault: Option<MeltFault>,
    driver_fault: Option<DriverFault>,
) -> Result<Counts, String> {
    let (frames, error) = m6a::read_frames(&item.name, screenblocks);
    let mut l = match Lockstep::new(item, screenblocks) {
        Ok(l) => l,
        // The genuine run stops before its first tic too (a missing music lump at the first
        // load, U10): nothing to compare.
        Err(e) if item.session_tics == 0 && frames.is_empty() => {
            eprintln!(
                "{} sb{screenblocks}: stops before its first tic: {e}",
                item.name
            );
            return Ok(Counts::default());
        }
        Err(e) => return Err(e),
    };
    if let Some(f) = melt_fault {
        l.display.plant_melt_fault(f);
    }
    l.driver_fault = driver_fault;
    let session = Rows::read(&format!("m6a_session/{}.session", item.name));
    let melts = golden_melts(&item.name);
    let name = &item.name;
    let mut n = Counts::default();
    let mut seen_melts = Vec::new();
    let stop = loop {
        let t = l.tic as i64;
        let s = match l.step() {
            Ok(s) => s,
            Err(stop) => break stop,
        };
        let d = |what: String| format!("{name} sb{screenblocks} tic {t}: {what}");
        let g = frames.get(&t).ok_or_else(|| d("no genuine frame".into()))?;
        if g.gamestate != s.gamestate || g.sha256 != s.sha256 {
            return Err(d(format!(
                "frame differs (gamestate {} genuine {})",
                s.gamestate, g.gamestate
            )));
        }
        let (gp, gs) = (g.status[0], g.status[1]);
        if (i64::from(s.palette), s.palette_set) != (gp, gs) {
            return Err(d(format!(
                "palette {:?} genuine {:?}",
                (s.palette, s.palette_set),
                (gp, gs)
            )));
        }
        let row = session
            .at("G", t)
            .ok_or_else(|| d("no session row".into()))?;
        if i64::from(s.rndindex) != int(row, "rndindex") {
            return Err(d(format!(
                "M_Random index {} genuine {}",
                s.rndindex,
                int(row, "rndindex")
            )));
        }
        n.index += 1;
        match s.gamestate {
            0 => n.level_frames += 1,
            1 => n.intermission_frames += 1,
            _ => n.finale_frames += 1,
        }
        n.tics += 1;
        if screenblocks != 10 {
            continue;
        }
        match (&s.melt, melts.get(&t)) {
            (None, None) => {}
            (Some(m), Some(gm)) => {
                seen_melts.push(t);
                if (&m.start, &m.end) != (&gm.start, &gm.end) {
                    return Err(d("melt start or end screen differs".into()));
                }
                if (i64::from(m.palette), m.palette_set) != (gm.palette, gm.palette_set) {
                    return Err(d(format!(
                        "melt palette {:?} genuine {:?}",
                        (m.palette, m.palette_set),
                        (gm.palette, gm.palette_set)
                    )));
                }
                for (k, step) in m.steps.iter().enumerate() {
                    match gm.steps.get(k) {
                        Some(want) if want == step => {}
                        Some(_) => return Err(d(format!("melt step {} picture differs", k + 1))),
                        None => return Err(d(format!("melt has more than {k} steps"))),
                    }
                }
                if m.steps.len() != gm.steps.len() {
                    return Err(d(format!(
                        "melt has {} steps, genuine {}",
                        m.steps.len(),
                        gm.steps.len()
                    )));
                }
                n.melts += 1;
                n.melt_steps += m.steps.len();
            }
            (Some(_), None) => return Err(d("a melt the genuine run does not make".into())),
            (None, Some(_)) => return Err(d("no melt where the genuine run melts".into())),
        }
    };
    // A display stop (`HELP2`) must be the genuine one, tic and message; a stop the display
    // cannot make (a sound or session refusal) must fall on the genuine run's last tic.
    let want_stop = error.clone().map(|e| (item.session_tics, e));
    let display_stop = error
        .as_deref()
        .is_some_and(|e| e.contains("W_GetNumForName: HELP2"));
    match &stop {
        Stop::Refused(t, e) if display_stop => {
            if Some((*t, e.clone())) != want_stop {
                return Err(format!("{name}: stop {:?}, genuine {want_stop:?}", (t, e)));
            }
        }
        Stop::Refused(t, e) => {
            if error.is_none() || *t != item.session_tics {
                return Err(format!("{name}: stop {:?}, genuine {want_stop:?}", (t, e)));
            }
        }
        Stop::End | Stop::GenuineEnd if display_stop => {
            return Err(format!("{name}: no stop, genuine {want_stop:?}"));
        }
        Stop::End | Stop::GenuineEnd => {}
    }
    if n.tics != frames.len() {
        return Err(format!(
            "{name} sb{screenblocks}: {} frames compared, the genuine run has {}",
            n.tics,
            frames.len()
        ));
    }
    if screenblocks == 10 && seen_melts.len() != melts.len() {
        return Err(format!(
            "{name}: {} melts shown, the genuine run has {}",
            seen_melts.len(),
            melts.len()
        ));
    }
    Ok(n)
}

/// Items whose demo the session refuses before any tic: the easy skills (skill bytes 0 and 1,
/// the next pull request).
const NOT_DRIVEN: [&str; 2] = ["SKILL0", "SKILL1"];

/// What the drive compares at screenblocks 10 over the manifest's 29 items (26 with frames;
/// NOE1M2AT stops before its first tic, as the genuine run does; SKILL0 and SKILL1 are not driven): every genuine frame (level,
/// intermission and finale), the index after every tic, and every melt and melt step but the
/// easy skills' (the genuine rows hold 125 melts and 4,946 steps; SKILL0 and SKILL1 hold 6 and
/// 238). Each count is of genuine rows, every one compared equal (the drive requires one frame
/// per genuine frame row and one melt per genuine Y row).
const SB10: Counts = Counts {
    tics: 16197,
    level_frames: 8185,
    intermission_frames: 3182,
    finale_frames: 4830,
    index: 16197,
    melts: 119,
    melt_steps: 4708,
};
/// The same at screenblocks 11, where no melt golden exists (the runner's melt goldens are at
/// screenblocks 10).
const SB11: Counts = Counts {
    melts: 0,
    melt_steps: 0,
    ..SB10
};

/// Every item, both screen sizes: every frame, index and melt equal.
#[test]
fn every_item_against_the_genuine_goldens() {
    let mut total = [Counts::default(); 2];
    let mut not_driven = Vec::new();
    let (mut golden_melts_all, mut golden_steps_all) = (0usize, 0usize);
    let (mut uncovered_melts, mut uncovered_steps) = (0usize, 0usize);
    for item in m6a::items() {
        let gm = golden_melts(&item.name);
        let steps: usize = gm.values().map(|m| m.steps.len()).sum();
        golden_melts_all += gm.len();
        golden_steps_all += steps;
        for (i, sb) in [10u8, 11].into_iter().enumerate() {
            match drive(&item, sb, None, None) {
                Ok(n) => {
                    let t = &mut total[i];
                    t.tics += n.tics;
                    t.level_frames += n.level_frames;
                    t.intermission_frames += n.intermission_frames;
                    t.finale_frames += n.finale_frames;
                    t.index += n.index;
                    t.melts += n.melts;
                    t.melt_steps += n.melt_steps;
                    eprintln!("{} sb{sb}: {n:?}", item.name);
                }
                Err(e) if NOT_DRIVEN.contains(&item.name.as_str()) => {
                    eprintln!("{} sb{sb}: not driven: {e}", item.name);
                    if sb == 10 {
                        not_driven.push(item.name.clone());
                        uncovered_melts += gm.len();
                        uncovered_steps += steps;
                    }
                }
                Err(e) => panic!("{e}"),
            }
        }
    }
    eprintln!(
        "sb10 {:?}; sb11 {:?}; melts compared {} of {golden_melts_all}, steps {} of \
         {golden_steps_all}; not driven {not_driven:?} ({uncovered_melts} melts, \
         {uncovered_steps} steps)",
        total[0], total[1], total[0].melts, total[0].melt_steps
    );
    assert_eq!(not_driven, NOT_DRIVEN);
    assert_eq!(total, [SB10, SB11]);
    assert_eq!(
        (
            total[0].melts + uncovered_melts,
            total[0].melt_steps + uncovered_steps
        ),
        (golden_melts_all, golden_steps_all)
    );
    assert_eq!(total[0].index, total[0].tics);
}

/// The planted faults are caught, each named: three in the display ([`MeltFault`]: a melt
/// column's offset off by one, a dropped step, a wrong palette for a melt) and one in the driver
/// (the finale's art screen one tic early).
#[test]
fn the_planted_faults_are_caught() {
    let cases: [(&str, Option<MeltFault>, Option<DriverFault>, &str); 5] = [
        (
            "CHAIN",
            Some(MeltFault::ColumnOffByOne),
            None,
            "picture differs",
        ),
        (
            "CHAIN",
            Some(MeltFault::DroppedStep),
            None,
            "tic 0: melt step 5 picture differs",
        ),
        (
            "CHAIN",
            Some(MeltFault::WrongPalette),
            None,
            "tic 0: melt palette",
        ),
        (
            "FINRET",
            Some(MeltFault::ColumnOffByOne),
            None,
            "picture differs",
        ),
        (
            "FINRET",
            None,
            Some(DriverFault::FinaleStageEarly),
            "tic 1890: ",
        ),
    ];
    for (name, mf, df, want) in cases {
        let item = m6a::item(name);
        let e = drive(&item, 10, mf, df).expect_err("a planted fault must be caught");
        eprintln!("{name} {mf:?} {df:?}: {e}");
        assert!(e.contains(want), "{name} {mf:?} {df:?}: {e}");
    }
    // A display holding a planted fault cannot be saved.
    let mut l = Lockstep::new(&m6a::item("CHAIN"), 10).expect("drive");
    assert!(l.display.session_state_v4().is_some());
    l.display.plant_melt_fault(MeltFault::WrongPalette);
    assert!(l.display.session_state_v4().is_none());
}

/// A fresh finale-and-melt display restored from `state` beside `l`'s session.
fn restored(l: &Lockstep, state: SessionDisplayStateV4) -> Display {
    let res = l.session.data().resources().clone();
    let mode = l.session.data().mode().mode.game_mode();
    let level_of = |map: u8| {
        l.levels[usize::from(map) - 1]
            .as_ref()
            .expect("a loaded map")
            .1
            .clone()
    };
    let mut fresh = Display::new_session_with(
        res,
        level_of(l.item.map),
        DisplayConfig {
            screenblocks: l.screenblocks,
        },
        SessionConfig::FinaleAndMelt { mode },
    )
    .expect("display");
    let level = match &state.session.scene {
        Scene::Level(_) => Some((level_of(l.session.map()), l.session.level())),
        _ => None,
    };
    fresh.restore_session_v4(state, level).expect("restored");
    fresh
}

/// Encodes, decodes and checks `l`'s state; returns it.
fn round_trip(l: &Lockstep) -> SessionDisplayStateV4 {
    let state = l.display.session_state_v4().expect("state");
    let bytes = state.encode();
    assert_eq!(&bytes[..8], b"DDSP\x04\0\0\0");
    let decoded = SessionDisplayStateV4::decode(&bytes).expect("decoded");
    assert_eq!(decoded, state);
    assert_eq!(decoded.encode(), bytes);
    decoded.check().expect("checked");
    decoded
}

/// Runs `l` whole up to session tic `t` (not yet run).
fn to_tic(l: &mut Lockstep, t: usize) {
    while l.tic < t {
        l.step().expect("tic");
    }
}

/// `DDSP` version 4 saved and restored at every step of melts of every kind (session start,
/// level to intermission, intermission to level, reborn reload, level to finale, the finale's
/// text to art), and in both finale stages; each restored display continues exactly as the
/// uninterrupted one: every later picture, `done` flag and palette equal, and the whole state's
/// encoding equal after the melt and after the next tics.
#[test]
fn restore_at_every_melt_step_and_continue() {
    // (item, melt tic) from the genuine Y rows: every melt of FINRET and FINREG (session start,
    // level to finale, the finale's text to art); CHAIN's first five (session start, then level to
    // intermission and intermission to level twice); the first three of REBHELD
    // and REBLATE (each with a reborn reload).
    let mut cases: Vec<(&str, i64)> = Vec::new();
    for (name, take) in [
        ("FINRET", 3),
        ("FINREG", 3),
        ("CHAIN", 5),
        ("REBHELD", 3),
        ("REBLATE", 3),
    ] {
        let melts = golden_melts(name);
        let picks: Vec<i64> = melts.keys().copied().take(take).collect();
        for t in picks {
            cases.push((name, t));
        }
    }
    let (mut points, mut melts) = (0usize, 0usize);
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for (name, t) in cases {
        let item = m6a::item(name);
        let mut base = Lockstep::new(&item, 10).expect("drive");
        to_tic(&mut base, t as usize);
        let mut at_frame = base;
        at_frame.tic_to_display().expect("tic");
        assert!(at_frame.display.melt_in_progress(), "{name} tic {t}: melts");
        // The kind of melt, from the genuine frames' game states before and at the melt's tic.
        let (frames, _) = m6a::read_frames(name, 10);
        let now = frames[&t].gamestate;
        let kind = match ((t > 0).then(|| frames[&(t - 1)].gamestate), now) {
            (None, _) => "session start",
            (Some(0), 1) => "level to intermission",
            (Some(1), 0) => "intermission to level",
            (Some(0), 0) => "reborn reload",
            (Some(0), 2) => "level to finale",
            (Some(2), 2) => "finale text to art",
            _ => panic!("{name} tic {t}: an unexpected melt"),
        };
        *kinds.entry(kind).or_insert(0) += 1;
        // The uninterrupted run from the frame: every picture, then two more tics.
        let mut pictures = Vec::new();
        let mut a = at_frame.clone();
        let mut f = Frame::new();
        loop {
            let st = a.display.melt_step(&mut f).expect("step");
            pictures.push((f.pixels().to_vec(), st));
            if st.done {
                break;
            }
        }
        let after_melt = a.display.session_state_v4().expect("state").encode();
        let mut later = Vec::new();
        for _ in 0..2 {
            match a.step() {
                Ok(_) => later.push(a.display.session_state_v4().map(|s| s.encode())),
                Err(_) => break,
            }
        }
        // Restore at every step 0..N-1 and continue.
        let mut b = at_frame.clone();
        for k in 0..pictures.len() {
            let state = round_trip(&b);
            assert_eq!(state.melt.as_ref().map(|m| usize::from(m.step)), Some(k));
            let mut c = b.clone();
            c.display = restored(&b, state);
            assert_eq!(
                c.display.session_state_v4().map(|s| s.encode()),
                b.display.session_state_v4().map(|s| s.encode()),
                "{name} tic {t} step {k}: restored"
            );
            for (want, st) in &pictures[k..] {
                let got = c.display.melt_step(&mut f).expect("step");
                assert_eq!((&f.pixels().to_vec(), got), (want, *st), "{name} tic {t}");
            }
            assert!(!c.display.melt_in_progress());
            assert_eq!(
                c.display.session_state_v4().expect("state").encode(),
                after_melt,
                "{name} tic {t} step {k}: after the melt"
            );
            for want in &later {
                c.step().expect("tic");
                assert_eq!(
                    &c.display.session_state_v4().map(|s| s.encode()),
                    want,
                    "{name} tic {t} step {k}: a later tic"
                );
            }
            points += 1;
            b.display.melt_step(&mut f).expect("step");
        }
        melts += 1;
    }
    eprintln!("restore points in mid-melt: {points} over {melts} melts; kinds {kinds:?}");
    assert_eq!(melts, 17);
    assert_eq!(kinds.len(), 6, "every kind of melt");
    assert!(
        kinds.values().all(|n| *n >= 2),
        "at least two melts of each kind"
    );
    // Both finale stages, outside a melt: FINRET in the text (tics 400, 1000) and on the art
    // screen, which begins at tic 1891 (tics 1895, 1940; its demo has 1,951 tics).
    let item = m6a::item("FINRET");
    let mut l = Lockstep::new(&item, 10).expect("drive");
    let mut stages = Vec::new();
    for t in [400usize, 1000, 1895, 1940] {
        to_tic(&mut l, t);
        let state = round_trip(&l);
        assert!(state.melt.is_none());
        assert_eq!(state.session.scene.game_state(), GameState::Finale);
        stages.push(l.finale.expect("finale").stage);
        let mut c = l.clone();
        c.display = restored(&l, state);
        let mut a = l.clone();
        for _ in 0..5 {
            let (x, y) = (a.step().expect("tic"), c.step().expect("tic"));
            assert_eq!(x, y, "FINRET from tic {t}");
            assert_eq!(
                a.display.session_state_v4().map(|s| s.encode()),
                c.display.session_state_v4().map(|s| s.encode())
            );
        }
    }
    assert_eq!(stages, [0, 0, 1, 1]);
}

/// The frozen configuration beside the new one, what can be shown in the tree (master's code is
/// not in it): on every tic of CHAIN, REBHELD and FINRET up to the victory, which the frozen
/// session refuses, the frozen display (inside the frozen session) and the new one give the same
/// frame and `M_Random` index, the version-4 state's version-3 part equals the frozen display's
/// version-3 state (which never holds a melt), and the version-4 bytes are the version-3 bytes
/// after one game-mode byte. The frozen display still refuses the finale drawer and the melt
/// pictures, saves no version 4 and restores none; the new display saves no version 3 and
/// restores none. (That version 3's bytes and rules are master's is shown by the existing
/// tests, which this pull request leaves unmodified: `tests/m6a_display.rs` and
/// `session_reader_tests.rs` in doom-render, the doom-game session tests.)
#[test]
fn the_frozen_configuration_is_unchanged() {
    let mut tics = 0usize;
    for name in ["CHAIN", "REBHELD", "FINRET"] {
        let item = m6a::item(name);
        let mut l = Lockstep::new(&item, 10).expect("drive");
        assert_eq!(
            l.session.display().session_config(),
            Some(SessionConfig::RefusesFinaleAndMelt)
        );
        loop {
            let at_victory = l.clone();
            if l.step().is_err() {
                break;
            }
            if l.finale.is_some() {
                // The frozen display at the victory: the finale drawer is still refused.
                let mut frozen = at_victory.session.display().clone();
                frozen.begin_finale().expect("finale");
                frozen.observe_non_level(0).expect("observed");
                let mut f = Frame::new();
                let view = FinaleView { stage: 0, count: 1 };
                assert_eq!(
                    frozen.draw_finale(&view, &mut f),
                    Err(SessionError::NotYet(doom_render::NotYet::FinaleDrawer))
                );
                break;
            }
            let frozen = l.session.display();
            let v3 = frozen.session_state().expect("version 3");
            assert!(v3.melt.is_none());
            let v4 = l.display.session_state_v4().expect("version 4");
            assert_eq!(v4.session, v3, "{name} tic {}", l.tic - 1);
            let (b3, b4) = (v3.encode(), v4.encode());
            assert_eq!(&b3[..8], b"DDSP\x03\0\0\0");
            assert_eq!(b4[9..], b3[8..], "{name} tic {}", l.tic - 1);
            assert_eq!(l.session.frame().pixels(), l.frame.pixels());
            assert_eq!(frozen.random_index(), l.display.random_index());
            tics += 1;
        }
        // Each configuration refuses the other's calls and states, changing nothing.
        let mut frozen = l.session.display().clone();
        let v3 = frozen.session_state().expect("version 3");
        let v4 = l.display.session_state_v4().expect("version 4");
        let mut f = Frame::new();
        assert_eq!(
            frozen.melt_step(&mut f),
            Err(SessionError::NotYet(doom_render::NotYet::MeltPictures))
        );
        assert!(frozen.session_state_v4().is_none());
        assert!(!frozen.melt_in_progress());
        let level =
            matches!(v4.session.scene, Scene::Level(_)).then(|| (at_level(&l), l.session.level()));
        assert_eq!(
            frozen.restore_session_v4(v4.clone(), level.clone()),
            Err(doom_render::SessionRestoreError::WrongConfig)
        );
        assert_eq!(frozen.session_state(), Some(v3.clone()));
        let mut new = l.display.clone();
        assert!(
            new.session_state().is_none(),
            "the new display saves version 4"
        );
        assert_eq!(
            new.restore_session(v3, level),
            Err(doom_render::SessionRestoreError::WrongConfig)
        );
        assert_eq!(new.session_state_v4(), Some(v4));
    }
    eprintln!("tics with the frozen and new displays compared: {tics}");
    assert!(tics > 1000);
}

/// The level of `l`'s current map.
fn at_level(l: &Lockstep) -> Arc<Level> {
    l.levels[usize::from(l.session.map()) - 1]
        .as_ref()
        .expect("a loaded map")
        .1
        .clone()
}

/// While a melt is in progress every other display call is refused with a typed error and
/// changes nothing, in a level melt (CHAIN's session start), an intermission melt (CHAIN's first
/// level-to-intermission melt) and a finale melt (FINRET's text to art); a melt step with no melt
/// is refused; and the registered art screen without `HELP2` (NOHELP2) is a typed refusal that
/// leaves the display and the last good frame unchanged.
#[test]
fn a_melt_in_progress_refuses_every_other_call() {
    let enc = |d: &Display| d.session_state_v4().expect("state").encode();
    let chain = m6a::item("CHAIN");
    let into_intermission = *golden_melts("CHAIN")
        .keys()
        .nth(1)
        .expect("CHAIN's second melt");
    for (item, t) in [
        (chain.clone(), 0usize),
        (chain, into_intermission as usize),
        (m6a::item("FINRET"), 1891),
    ] {
        let mut l = Lockstep::new(&item, 10).expect("drive");
        to_tic(&mut l, t);
        l.tic_to_display().expect("tic");
        assert!(l.display.melt_in_progress());
        let before = enc(&l.display);
        let mut f = Frame::new();
        let level = at_level(&l);
        let d = &mut l.display;
        let sim = l.session.level();
        let cmd = TicCommand::default();
        let melting = Err(SessionError::MeltInProgress);
        assert_eq!(
            d.observe_after(sim, &cmd, 0),
            Err(doom_render::DisplayObserveError::MeltInProgress)
        );
        assert_eq!(
            d.draw(sim, &mut f).err(),
            Some(doom_render::DisplayRefusal::MeltInProgress)
        );
        assert_eq!(d.observe_non_level(0), melting);
        let view = l.session.intermission().view(l.session.wminfo());
        assert_eq!(d.draw_intermission(&view, &mut f).map(|_| ()), melting);
        let fv = FinaleView { stage: 1, count: 0 };
        assert_eq!(d.draw_finale(&fv, &mut f).map(|_| ()), melting);
        assert_eq!(d.begin_intermission(), melting);
        assert_eq!(d.begin_finale(), melting);
        assert_eq!(d.begin_finale_art(), melting);
        assert_eq!(d.begin_next_level(level, sim, LevelChange::Reborn), melting);
        assert_eq!(
            d.set_screenblocks(11),
            Err(doom_render::ConfigError::MeltInProgress)
        );
        assert_eq!(enc(d), before, "{} tic {t}: unchanged", item.name);
        while !d.melt_step(&mut f).expect("step").done {}
        let after = enc(d);
        assert_eq!(d.melt_step(&mut f), Err(SessionError::NoMelt));
        assert_eq!(enc(d), after);
    }
    // NOHELP2: the art screen's first frame in registered mode without HELP2.
    let mut l = Lockstep::new(&m6a::item("NOHELP2"), 10).expect("drive");
    to_tic(&mut l, 1891);
    let mut view = l.finale.expect("finale");
    assert!(finale::tick(&mut view).expect("tic").wipe);
    let d = &mut l.display;
    d.begin_finale_art().expect("wipe");
    d.observe_non_level(0).expect("observed");
    let (before, frame) = (enc(d), l.frame.pixels().to_vec());
    let mut out = l.frame.clone();
    for _ in 0..2 {
        match d.draw_finale(&view, &mut out) {
            Err(SessionError::Finale(e)) => {
                assert_eq!(e.to_string(), "W_GetNumForName: HELP2 not found!")
            }
            r => panic!("{r:?}"),
        }
        assert_eq!(enc(d), before);
        assert_eq!(out.pixels(), &frame[..], "the last good frame");
        assert!(!d.melt_in_progress());
    }
}
