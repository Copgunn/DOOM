//! Shared support for the tests of the M6a part 2 golden set (owner's ruling V9 of 2026-11-23 and
//! its addition of 2026-11-24): `tests/reference/m6a2_manifest.json` (`tools/reference/m6a2_wad.py`)
//! and the goldens `tools/reference/m6a2_goldens.py` writes from the genuine linuxdoom-1.10
//! engine only (`m6a2_session`, `m6a2_frames`, `m6a2_melt`, `m6a2_results.json`); no Rust output
//! feeds an expectation. The part 1 goldens are read through `m6a` as they are.
//!
//! The driver is the one of `m6a_finale_melt_display.rs` (part 2, the display steps), unchanged
//! in what it runs: each item's frozen session (wire 26, the intermission logic, its frozen
//! display) runs from the committed demo and a second display in the finale-and-melt
//! configuration (`SessionConfig::FinaleAndMelt`) is driven in lockstep from what the session
//! exposes; from the E1M8 victory, which the frozen session refuses, the step-2 finale logic
//! (`doom_game::session::finale`) drives the finale. What it adds: the finale's stage and count
//! compared with the genuine F rows on every finale tic, melts compared at either screen size
//! from a named melt file, and the planted faults of the part 2 set.
#![allow(dead_code, reason = "each test target uses a subset")]

use crate::m6a::{self, GoldenFrame, Item, Rows, int};
use doom_game::session::demo::SessionDemo;
use doom_game::session::finale::{self, FinaleView};
use doom_game::session::{NotYet, Session, SessionSettings, SessionWitness, TicKind};
use doom_render::v_video::Patch;
use doom_render::{
    Display, DisplayConfig, Frame, GameState, Level, LevelChange, MeltFault, SCREENHEIGHT,
    SCREENWIDTH, SessionConfig, SessionError,
};
use doom_sim::{CarriedStart, LevelStart, MapContextOptions, Simulation, Skill, TicCommand};
use doom_test_evidence::sha256;
use std::collections::BTreeMap;
use std::sync::Arc;

/// A map and its renderer level.
type MapLevel = (doom_map::Map, Arc<Level>);

/// The finale tic on which the last drawn character of E1TEXT is first counted with the part 2
/// font: character 438 (`'!'`), `(finalecount - 10) / 3 >= 439` (f_finale.c:298), so finalecount
/// 1,327 (`tests/reference/m6a2_manifest.json`, crawl `last_glyph_finale_tic`, computed by
/// `tools/reference/m6a2_wad.py` from the glyph widths and observed in the genuine frames).
pub const LAST_GLYPH_FINALE_TIC: i64 = 1327;
/// Stage 1 begins when `finalecount > strlen(text) * 3 + 250` (f_finale.c:241): finale tic 1,571.
pub const STAGE1_FINALE_TIC: i64 = 1571;

/// The repository's `tests/reference/<path>` (the part 2 set is not read from
/// `M6A_REFERENCE_DIR`).
fn reference(path: &str) -> std::path::PathBuf {
    m6a::root("tests/reference").join(path)
}

fn json(path: &str) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(reference(path)).expect(path)).expect("JSON")
}

/// The full-crawl items of the part 2 manifest (FULLREG, FULLRET), with their genuine end and
/// session tics from `m6a2_results.json`.
pub fn full_items() -> Vec<Item> {
    let m = json("m6a2_manifest.json");
    assert_eq!(m["schema"], "DOOM-M6A2-MANIFEST-v1");
    let results = json("m6a2_results.json");
    assert_eq!(results["schema"], "DOOM-M6A2-RESULTS-v1");
    m["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| {
            let name = i["name"].as_str().expect("name").to_owned();
            let r = &results["full_crawl"][&name];
            Item {
                mode: i["mode"].as_str().expect("mode").into(),
                wads: m["wadsets"][i["wadset"].as_str().expect("wadset")]
                    .as_array()
                    .expect("WAD set")
                    .iter()
                    .map(|w| w.as_str().expect("WAD").into())
                    .collect(),
                lmp: i["lmp"].as_str().expect("lmp").into(),
                map: i["map"].as_u64().expect("map") as u8,
                skill: i["skill"].as_u64().expect("skill") as u8,
                end: r["end"].as_str().expect("end").into(),
                session_tics: r["session_tics"].as_u64().expect("tics") as usize,
                name,
            }
        })
        .collect()
}

/// The names of the items with a melt golden at screenblocks 11 (`m6a2_manifest.json`,
/// `melt_sb11`: every part 1 item, then the full-crawl items).
pub fn melt_sb11_names() -> Vec<String> {
    json("m6a2_manifest.json")["melt_sb11"]["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|n| n.as_str().expect("name").to_owned())
        .collect()
}

/// The genuine goldens one drive compares against.
pub struct Golden {
    /// Every frame by session tic.
    pub frames: BTreeMap<i64, GoldenFrame>,
    /// The genuine engine's stop (frames E row), if any.
    pub error: Option<String>,
    /// The session stream (G rows: the `M_Random` index; F rows: the finale's stage and count).
    pub session: Rows,
    /// The melts at this screen size, when a melt golden exists for it.
    pub melts: Option<BTreeMap<i64, GoldenMelt>>,
}

impl Golden {
    /// A full-crawl item's goldens at `sb` (all in the part 2 set).
    pub fn full(name: &str, sb: u8) -> Self {
        let (frames, error) = read_frames(&format!("m6a2_frames/{name}.sb{sb}.frames"));
        Self {
            frames,
            error,
            session: Rows::parse(
                &std::fs::read_to_string(reference(&format!("m6a2_session/{name}.session")))
                    .expect("session"),
            ),
            melts: Some(melts(&format!("m6a2_melt/{name}.sb{sb}.melt"))),
        }
    }

    /// A part 1 item at screenblocks 11: the part 1 frames and session goldens and the part 2
    /// melt golden at 11.
    pub fn part1_sb11(name: &str) -> Self {
        let (frames, error) = read_frames(&format!("m6a_frames/{name}.sb11.frames"));
        Self {
            frames,
            error,
            session: Rows::parse(
                &std::fs::read_to_string(reference(&format!("m6a_session/{name}.session")))
                    .expect("session"),
            ),
            melts: Some(melts(&format!("m6a2_melt/{name}.sb11.melt"))),
        }
    }
}

/// Reads a frames golden (`tests/reference/<path>`, the synthetic set: no flagged column): the
/// frames by tic (F and P rows of a level frame, the G row of any other) and the E row.
pub fn read_frames(path: &str) -> (BTreeMap<i64, GoldenFrame>, Option<String>) {
    let text = std::fs::read_to_string(reference(path)).expect(path);
    let mut out: BTreeMap<i64, GoldenFrame> = BTreeMap::new();
    let mut error = None;
    for l in text.lines() {
        let c: Vec<&str> = l.split(',').collect();
        if c[0].starts_with('#') {
            continue;
        }
        let n = |i: usize| {
            c[i].parse::<i64>()
                .unwrap_or_else(|_| panic!("{path}: {l}"))
        };
        match c[0] {
            "F" => {
                assert_eq!(c[6], "0", "{path}: no flagged column: {l}");
                assert_eq!(c[14], "0", "{path}: no whole-frame exclusion: {l}");
                let g = GoldenFrame {
                    sha256: c[2].into(),
                    ..GoldenFrame::default()
                };
                assert!(out.insert(n(1), g).is_none(), "{path}: {l}");
            }
            "P" => out.get_mut(&n(1)).expect("P after F").status = (2..=6).map(n).collect(),
            "G" => {
                let g = GoldenFrame {
                    gamestate: n(2),
                    sha256: c[3].into(),
                    status: vec![n(4), n(5), n(6)],
                    ..GoldenFrame::default()
                };
                assert!(out.insert(n(1), g).is_none(), "{path}: {l}");
            }
            "E" => error = Some(c[2..].join(",")),
            "U" | "O" | "D" | "W" | "V" | "Y" => panic!("{path}: unexpected row {l}"),
            _ => {}
        }
    }
    (out, error)
}

/// One melt of a melt golden.
#[derive(Clone, Debug)]
pub struct GoldenMelt {
    /// The start screen's digest.
    pub start: String,
    /// The end screen's digest.
    pub end: String,
    /// The palette of the melt.
    pub palette: i64,
    /// The palette set in the display that started it (-1: none).
    pub palette_set: i64,
    /// Every step's picture digest and `done` flag.
    pub steps: Vec<(String, bool)>,
}

/// Reads a part 2 melt golden (`DOOM-M6A2-MELT-v1`, the runner's `--melt-clock unit`).
pub fn melts(path: &str) -> BTreeMap<i64, GoldenMelt> {
    let text = std::fs::read_to_string(reference(path)).expect(path);
    assert!(
        text.starts_with("#schema,DOOM-M6A2-MELT-v1\n"),
        "{path}: schema"
    );
    let rows = Rows::parse(&text);
    let mut melts: BTreeMap<i64, GoldenMelt> = rows
        .of("Y")
        .iter()
        .map(|y| {
            assert_eq!(y["clock"], "unit", "{path}: --melt-clock unit");
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
        assert_eq!(int(v, "step"), m.steps.len() as i64 + 1, "{path}: order");
        assert_eq!(int(v, "tics"), 1, "{path}: one tick per step");
        m.steps.push((v["sha256"].clone(), int(v, "done") == 1));
    }
    melts
}

/// One melt as the display showed it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShownMelt {
    start: String,
    end: String,
    palette: u8,
    palette_set: i64,
    steps: Vec<(String, bool)>,
}

/// What one tic presented.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    gamestate: i64,
    sha256: String,
    palette: u8,
    palette_set: i64,
    rndindex: u8,
    finale: Option<FinaleView>,
    melt: Option<ShownMelt>,
}

/// A defect planted in a test-side copy of `F_TextWrite` ([`TextReplica`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextFault {
    /// A newline treated as a space (f_finale.c:306-311 not taken: a 4-pixel gap instead).
    NewlineAsSpace,
    /// The line step 12 instead of 11 (f_finale.c:309).
    LineStepOffByOne,
}

/// A planted fault outside the display (the display's own are [`MeltFault`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriverFault {
    /// The crawl one character short on the tic the last character appears: the drawer is given
    /// `finalecount - 3` on finale tic [`LAST_GLYPH_FINALE_TIC`] only.
    LastCharacterShort,
    /// Every text frame drawn by the test-side copy of `F_TextWrite` with the given defect.
    Text(TextFault),
    /// Every text frame drawn by the test-side copy without a defect (to show the copy equals the
    /// drawer, so that a fault planted in it is the only difference).
    TextReplicaClean,
}

/// A test-side copy of `F_TextWrite` (f_finale.c:261-325) over the session's resources, used only
/// to plant [`TextFault`]s; without a fault it must equal `doom_render::f_finale::draw` on every
/// text frame (`the_planted_faults_are_caught` checks it on every one of FULLRET's).
pub struct TextReplica {
    flat: Vec<u8>,
    font: Vec<Patch>,
    res: doom_render::Resources,
}

impl TextReplica {
    fn new(res: &doom_render::Resources) -> Self {
        let lump = res
            .directory()
            .get_num_for_name(b"FLOOR4_8")
            .expect("FLOOR4_8");
        Self {
            flat: res.lump(lump)[..4096].to_vec(),
            font: doom_render::hu_stuff::load_font(res).expect("font"),
            res: res.clone(),
        }
    }

    /// The text frame of `finalecount` with `fault`.
    fn frame(&self, finalecount: i32, fault: Option<TextFault>) -> Vec<u8> {
        let (w, h) = (SCREENWIDTH, SCREENHEIGHT);
        let mut s = vec![0u8; w * h];
        for (y, row) in s.chunks_exact_mut(w).enumerate() {
            for (x, p) in row.iter_mut().enumerate() {
                *p = self.flat[((y & 63) << 6) + (x & 63)];
            }
        }
        let (mut cx, mut cy) = (10i32, 10i32);
        let count = usize::try_from(((finalecount - 10) / 3).max(0)).expect("count");
        for &ch in doom_render::f_finale::E1TEXT.iter().take(count) {
            let ch = if ch == b'\n' && fault == Some(TextFault::NewlineAsSpace) {
                b' '
            } else {
                ch
            };
            if ch == b'\n' {
                cx = 10;
                cy += if fault == Some(TextFault::LineStepOffByOne) {
                    12
                } else {
                    11
                };
                continue;
            }
            let c = i32::from(ch.to_ascii_uppercase()) - i32::from(b'!');
            if !(0..=63).contains(&c) {
                cx += 4;
                continue;
            }
            let p = self.font[usize::try_from(c).expect("glyph")];
            let hd = p.header;
            if cx + i32::from(hd.width) > w as i32 {
                break;
            }
            let bytes = self.res.lump(p.lump);
            let (x0, y0) = (cx - i32::from(hd.leftoffset), cy - i32::from(hd.topoffset));
            for col in 0..i32::from(hd.width) {
                let at = 8 + 4 * usize::try_from(col).expect("col");
                let mut o =
                    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("offset")) as usize;
                while bytes[o] != 0xff {
                    let (top, len) = (usize::from(bytes[o]), usize::from(bytes[o + 1]));
                    for k in 0..len {
                        let y = usize::try_from(y0).expect("y") + top + k;
                        let x = usize::try_from(x0 + col).expect("x");
                        s[y * w + x] = bytes[o + 3 + k];
                    }
                    o += len + 4;
                }
            }
            cx += i32::from(hd.width);
        }
        s
    }
}

/// One item's frozen session with the new-configuration display beside it.
#[derive(Clone)]
pub struct Lockstep {
    item: Item,
    session: Session,
    /// The finale-and-melt display under test.
    pub display: Display,
    levels: Arc<Vec<Option<MapLevel>>>,
    skill: Skill,
    commands: Arc<Vec<TicCommand>>,
    finale: Option<FinaleView>,
    frame: Frame,
    /// The next session tic.
    tic: usize,
    driver_fault: Option<DriverFault>,
    replica: Option<Arc<TextReplica>>,
    /// The test-side copy's frame of this tic, when it replaces the drawer's.
    replaced: Option<Vec<u8>>,
    /// Text frames on which the clean copy equalled the drawer.
    pub replica_equal: usize,
}

/// Why a drive stopped.
#[derive(Clone, Debug, PartialEq)]
pub enum Stop {
    /// The demo ended.
    End,
    /// The genuine run's last tic was reached.
    GenuineEnd,
    /// A refusal at a tic, with its text.
    Refused(usize, String),
}

impl Lockstep {
    /// The item at `screenblocks`, before its first tic.
    pub fn new(item: &Item, screenblocks: u8) -> Result<Self, String> {
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
            commands: Arc::new(demo.commands),
            finale: None,
            frame: Frame::new(),
            tic: 0,
            driver_fault: None,
            replica: None,
            replaced: None,
            replica_equal: 0,
        })
    }

    /// Plants a driver fault.
    pub fn plant(&mut self, fault: DriverFault) {
        if matches!(fault, DriverFault::Text(_) | DriverFault::TextReplicaClean) {
            self.replica = Some(Arc::new(TextReplica::new(self.session.data().resources())));
        }
        self.driver_fault = Some(fault);
    }

    /// The simulation before its tic 0 of the load the session is about to run, built as
    /// `Session::world_done_tic` and the reborn branch of `Session::tic` build it.
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
        self.replaced = None;
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
    pub fn step(&mut self) -> Result<Shown, Stop> {
        let (gamestate, palette, palette_set) = self.tic_to_display()?;
        Ok(self.shown(gamestate, palette, palette_set))
    }

    /// One finale tic: `F_Ticker`, its forced wipe, then `D_Display`.
    fn finale_tic(&mut self, t: usize) -> Result<(i64, u8, Option<u8>), Stop> {
        let view = self.finale.as_mut().expect("in the finale");
        let tick = finale::tick(view).expect("finale tic");
        let view = *view;
        let mut drawn = view;
        if self.driver_fault == Some(DriverFault::LastCharacterShort)
            && view.stage == 0
            && i64::from(view.count) == LAST_GLYPH_FINALE_TIC
        {
            drawn.count -= doom_render::f_finale::TEXTSPEED;
        }
        let d = &mut self.display;
        if tick.wipe {
            d.begin_finale_art().expect("forced wipe");
        }
        d.observe_non_level(0).expect("observed");
        let out = match d.draw_finale(&drawn, &mut self.frame) {
            Ok(f) => Ok((2, f.palette, f.palette_set)),
            Err(SessionError::Finale(e)) => Err(Stop::Refused(t, e.to_string())),
            Err(e) => panic!("{} tic {t}: {e:?}", self.item.name),
        };
        if let (Some(r), 0) = (&self.replica, view.stage) {
            let fault = match self.driver_fault {
                Some(DriverFault::Text(f)) => Some(f),
                _ => None,
            };
            let pic = r.frame(view.count, fault);
            if fault.is_none() {
                assert_eq!(
                    pic,
                    self.frame.pixels(),
                    "the test-side copy of F_TextWrite equals the drawer (finalecount {})",
                    view.count
                );
                self.replica_equal += 1;
            }
            self.replaced = Some(pic);
        }
        out
    }

    /// The tic's frame and, when it melts, every melt step.
    fn shown(&mut self, gamestate: i64, palette: u8, palette_set: Option<u8>) -> Shown {
        let palette_set = palette_set.map_or(-1, i64::from);
        let sha = sha256(self.replaced.as_deref().unwrap_or(self.frame.pixels()));
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
            finale: self.finale,
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

/// What a drive compared.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    /// Session tics compared.
    pub tics: usize,
    /// Level frames equal.
    pub level_frames: usize,
    /// Intermission frames equal.
    pub intermission_frames: usize,
    /// Finale frames equal.
    pub finale_frames: usize,
    /// Finale tics whose stage and count equal the genuine F row.
    pub finale_states: usize,
    /// `M_Random` indices equal.
    pub index: usize,
    /// Melts equal (start, end, palette, every step).
    pub melts: usize,
    /// Melt steps equal.
    pub melt_steps: usize,
}

/// Drives `item` at `screenblocks` against `golden`; Err names the first difference.
pub fn drive(
    item: &Item,
    screenblocks: u8,
    golden: &Golden,
    melt_fault: Option<MeltFault>,
    driver_fault: Option<DriverFault>,
) -> Result<(Counts, Lockstep), String> {
    let mut l = match Lockstep::new(item, screenblocks) {
        Ok(l) => l,
        // The genuine run stops before its first tic too (a missing music lump at the first
        // load, U10): nothing to compare.
        Err(e) if item.session_tics == 0 && golden.frames.is_empty() => {
            return Err(format!("stops before its first tic: {e}"));
        }
        Err(e) => return Err(e),
    };
    if let Some(f) = melt_fault {
        l.display.plant_melt_fault(f);
    }
    if let Some(f) = driver_fault {
        l.plant(f);
    }
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
        let g = golden
            .frames
            .get(&t)
            .ok_or_else(|| d("no genuine frame".into()))?;
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
        let row = golden
            .session
            .at("G", t)
            .ok_or_else(|| d("no session row".into()))?;
        if i64::from(s.rndindex) != int(row, "rndindex") {
            return Err(d(format!(
                "M_Random index {} genuine {}",
                s.rndindex,
                int(row, "rndindex")
            )));
        }
        // The finale's stage and count against the genuine F row of the tic.
        match (s.finale, golden.session.at("F", t)) {
            (None, None) => {}
            (Some(v), Some(f)) => {
                let want = (int(f, "finalestage"), int(f, "finalecount"));
                if (i64::from(v.stage), i64::from(v.count)) != want {
                    return Err(d(format!(
                        "finale stage and count {:?} genuine {want:?}",
                        (v.stage, v.count)
                    )));
                }
                n.finale_states += 1;
            }
            (v, f) => {
                return Err(d(format!("finale {v:?}, genuine F row {}", f.is_some())));
            }
        }
        n.index += 1;
        match s.gamestate {
            0 => n.level_frames += 1,
            1 => n.intermission_frames += 1,
            _ => n.finale_frames += 1,
        }
        n.tics += 1;
        let Some(melts) = &golden.melts else {
            continue;
        };
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
    let error = &golden.error;
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
    if n.tics != golden.frames.len() {
        return Err(format!(
            "{name} sb{screenblocks}: {} frames compared, the genuine run has {}",
            n.tics,
            golden.frames.len()
        ));
    }
    if let Some(melts) = &golden.melts {
        if seen_melts.len() != melts.len() {
            return Err(format!(
                "{name} sb{screenblocks}: {} melts shown, the genuine run has {}",
                seen_melts.len(),
                melts.len()
            ));
        }
    }
    Ok((n, l))
}

/// The session tic of finale tic `n` (finale tic 1 is the tic whose `G_Ticker` ran
/// `G_DoCompleted` and `F_StartFinale`) in a session's genuine F rows.
pub fn session_tic_of_finale_tic(session: &Rows, n: i64) -> i64 {
    let first = session
        .of("F")
        .iter()
        .map(|f| int(f, "tic"))
        .min()
        .expect("a finale");
    first + n - 1
}
