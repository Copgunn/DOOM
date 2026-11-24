//! The gate of the finale-and-melt session (`Session::new_finale_and_melt`,
//! `Session::tic_finale_and_melt`, `Session::melt_step`; M6a part 2 steps 3 and 4, the session
//! part) against the committed goldens of the genuine linuxdoom-1.10 engine, shared by the
//! `m6a2_session*` test targets. No Rust output feeds an expectation: every compared value is a
//! genuine row (`tools/reference/m6a_goldens.py`, `m6a_mixer.py`, `m6a_state.py`,
//! `m6a2_goldens.py`), the only inputs the committed demo and WADs.
//!
//! One run drives one item's whole demo through the session, the same calls as the app makes (a
//! tic per command, then every step of the melt the tic's display started), and compares on every
//! tic, at either screen size:
//!
//! - the session rows: **G** (every column the frozen gate compares, the finale's included), **C**
//!   (the console player; outside a level, the intermission's and the finale's, the finished
//!   player as `G_PlayerFinishLevel` leaves it), **I** and **J** (the intermission) and **F**
//!   (`finalestage`, `finalecount`, the flat and the text: f_finale.c:52-54, 117-121);
//! - every frame: a level frame's whole and face-zeroed digests, renderer counters, newly mapped
//!   lines, palette, palette set, face, `message_on`, `M_Random` index and cumulative sound and
//!   wipe draws; a frame outside a level (intermission and finale) whole, with palette, palette
//!   set and index;
//! - every melt: its tic, start and end screens, palette and palette set (Y rows), its step count
//!   and every step's picture and `done` flag (V rows); and, Rust-side, that a tic asked for
//!   during the melt is refused with nothing changed and that no melt step but the last changes
//!   the sound;
//! - every sound row (S, T, K, C, L, R; actor origins matched to the genuine serials by a
//!   bijection over the item) and every mixer buffer (the session's starts and buffers written as
//!   the genuine mixer driver writes them, the buffer of a melting tic after its last step, drained
//!   after the end), row for row;
//! - at screenblocks 10, where `m6a_state.txt` has the item, the level's whole state after every
//!   tic that ends in a level (`m6a_state.rs`'s digests);
//! - the end: the demo's end, or the genuine stop as the typed refusal at the same tic.
#![allow(dead_code, reason = "each test target uses a subset")]

use crate::m6a::{self, GenuineSound, Item, Rows, int};
use crate::rows::{Carry, GoldenWriter, drain, stream_rows_with};
use doom_game::session::demo::SessionDemo;
use doom_game::session::finale_melt::{FinaleMeltTicKind, FinaleMeltTicReport};
use doom_game::session::{
    NotYet, PlantedFault, Refusal, Session, SessionSettings, SessionWitness, StartError,
};
use doom_render::{Frame, GameState, SCREENHEIGHT, SCREENWIDTH, SessionError};
use doom_sim::{ActorId, PlayerBodyState, PlayerSlot, Simulation, Snapshot, TicCommand};
use doom_sound::chain::ChainRefusal;
use doom_sound::logic::{SoundOrigin, SoundRecord};
use doom_test_evidence::sha256;
use std::collections::BTreeMap;

/// Where an item's goldens are.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Source {
    /// The part 1 set (`tests/reference/m6a_*`), with its melts at screenblocks 11 from the part 2
    /// set (`m6a2_melt/<ITEM>.sb11.melt`).
    Part1,
    /// The part 2 full-crawl items (`tests/reference/m6a2_*`).
    Full,
}

/// One drivable item and where its goldens are.
#[derive(Clone, Debug)]
pub struct Spec {
    /// The item.
    pub item: Item,
    /// Its golden set.
    pub source: Source,
}

/// Every item of both golden sets: the 29 of `m6a_manifest.json`, then FULLREG and FULLRET.
pub fn specs() -> Vec<Spec> {
    let mut out: Vec<Spec> = m6a::items()
        .into_iter()
        .map(|item| Spec {
            item,
            source: Source::Part1,
        })
        .collect();
    out.extend(full_items().into_iter().map(|item| Spec {
        item,
        source: Source::Full,
    }));
    out
}

/// The named item of either set.
pub fn spec(name: &str) -> Spec {
    specs()
        .into_iter()
        .find(|s| s.item.name == name)
        .unwrap_or_else(|| panic!("item {name}"))
}

fn reference(path: &str) -> std::path::PathBuf {
    m6a::root("tests/reference").join(path)
}

fn read(path: &str) -> String {
    std::fs::read_to_string(reference(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn json(path: &str) -> serde_json::Value {
    serde_json::from_str(&read(path)).expect("JSON")
}

/// FULLREG and FULLRET (`m6a2_manifest.json`, `m6a2_results.json`).
fn full_items() -> Vec<Item> {
    let m = json("m6a2_manifest.json");
    assert_eq!(m["schema"], "DOOM-M6A2-MANIFEST-v1");
    let results = json("m6a2_results.json");
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

/// One frame of a frames golden (the synthetic sets: no flagged column or exclusion, asserted).
#[derive(Clone, Debug, Default)]
pub struct GoldenFrame {
    /// `gamestate` (0 for a level frame).
    pub gamestate: i64,
    /// Whole-frame digest.
    pub sha256: String,
    /// F row counters (as `m6a::GoldenFrame::f`).
    pub f: Vec<i64>,
    /// Newly mapped lines.
    pub mapped: Vec<u16>,
    /// P row (level) or G row (other) status.
    pub status: Vec<i64>,
    /// Q row: face-zeroed digest.
    pub noface: String,
    /// Q row numbers.
    pub q: [i64; 6],
}

/// Reads a frames golden (`tests/reference/<path>`): frames by tic and the E row.
pub fn read_frames(path: &str) -> (BTreeMap<i64, GoldenFrame>, Option<String>) {
    let text = read(path);
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
                    f: [4, 5, 7, 8, 9, 10, 11, 12]
                        .into_iter()
                        .map(n)
                        .chain([i64::from(c[13] == "1")])
                        .collect(),
                    ..GoldenFrame::default()
                };
                assert!(out.insert(n(1), g).is_none(), "{path}: {l}");
            }
            "M" => {
                out.get_mut(&n(1)).expect("M after F").mapped =
                    c[2].split(' ').map(|v| v.parse().expect("line")).collect();
            }
            "P" => out.get_mut(&n(1)).expect("P after F").status = (2..=6).map(n).collect(),
            "Q" => {
                let g = out.get_mut(&n(1)).expect("Q after F");
                g.noface = c[2].into();
                g.q = std::array::from_fn(|i| n(3 + i));
            }
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
    /// Start screen digest.
    pub start: String,
    /// End screen digest.
    pub end: String,
    /// Palette.
    pub palette: i64,
    /// Palette set in the display that started it (-1: none).
    pub palette_set: i64,
    /// Every step's picture digest and `done`.
    pub steps: Vec<(String, bool)>,
}

/// Reads a melt golden of either set (DOOM-M6A-MELT-v1 or DOOM-M6A2-MELT-v1, both written by the
/// genuine runner under `--melt-clock unit`).
pub fn read_melts(path: &str) -> BTreeMap<i64, GoldenMelt> {
    let text = read(path);
    assert!(
        text.starts_with("#schema,DOOM-M6A-MELT-v1\n")
            || text.starts_with("#schema,DOOM-M6A2-MELT-v1\n"),
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

/// The genuine sound stream at `path` (as `m6a::read_sound` reads `m6a_sound`).
fn read_sound(path: &str) -> GenuineSound {
    let mut g = GenuineSound::default();
    for l in read(path).lines() {
        if l.starts_with('#') || l.starts_with("Z,") || l.starts_with("W,") {
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        let tic: i64 = f[1].parse().expect("tic");
        match f[0] {
            "LV" => {
                g.loads.insert(tic, f[3].parse().expect("map"));
            }
            "E" => g.error = Some((tic, f[2..].join(","))),
            _ => g.by_tic.entry(tic).or_default().push(l.to_owned()),
        }
    }
    g
}

/// One level of the state golden (`m6a_state.txt`, DOOM-M6A-STATE-v1).
#[derive(Debug, Default, Clone)]
pub struct GoldenLevel {
    tic: i64,
    map: u8,
    kind: String,
    captures: usize,
    map_sha: String,
    sequence: String,
    points: BTreeMap<usize, (i64, String, String)>,
}

/// The state golden's levels of `name`, in load order (None: the item is not in it).
pub fn golden_levels(name: &str) -> Option<Vec<GoldenLevel>> {
    let text = read("m6a_state.txt");
    assert!(text.starts_with("#schema,DOOM-M6A-STATE-v1\n"));
    let mut levels: Vec<GoldenLevel> = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        if f[1] != name {
            continue;
        }
        let load: usize = f[2].parse().expect("load");
        match f[0] {
            "L" => {
                assert_eq!(levels.len(), load, "{line}");
                levels.push(GoldenLevel {
                    tic: f[3].parse().expect("tic"),
                    map: f[4].parse().expect("map"),
                    kind: f[5].to_owned(),
                    captures: f[6].parse().expect("captures"),
                    map_sha: f[7].to_owned(),
                    sequence: f[8].to_owned(),
                    points: BTreeMap::new(),
                });
            }
            "D" => {
                let leveltime: usize = f[4].parse().expect("leveltime");
                let point = (f[3].parse().expect("tic"), f[5].to_owned(), f[6].to_owned());
                levels[load].points.insert(leveltime, point);
            }
            _ => panic!("row {line}"),
        }
    }
    (!levels.is_empty()).then_some(levels)
}

fn framed(out: &mut Vec<u8>, path: &str, value: &[u8]) {
    out.extend((path.len() as u32).to_le_bytes());
    out.extend(path.as_bytes());
    out.extend((value.len() as u64).to_le_bytes());
    out.extend(value);
}

/// The state digest and the map digest of a capture (`m6a_state.rs`'s, unchanged).
fn digests(s: &Snapshot) -> (String, String) {
    assert_eq!(s.version(), 42, "wire 26 captures are DSIM 42");
    let kept: Vec<_> = s
        .fields()
        .filter(|f| !f.path().starts_with("map.") && !f.path().starts_with("events."))
        .collect();
    let mut body = b"DSIM".to_vec();
    body.extend(42_u32.to_le_bytes());
    body.extend((kept.len() as u64).to_le_bytes());
    for f in &kept {
        framed(&mut body, f.path(), f.value());
    }
    let mut map = Vec::new();
    for f in s.fields() {
        if f.path().starts_with("events.sounds.") {
            framed(&mut body, f.path(), f.value());
        } else if f.path().starts_with("map.") {
            framed(&mut map, f.path(), f.value());
        }
    }
    (sha256(&body), sha256(&map))
}

fn unhex(h: &str) -> Vec<u8> {
    (0..h.len() / 2)
        .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

/// The genuine goldens of one run.
pub struct Golden {
    session: Rows,
    frames: BTreeMap<i64, GoldenFrame>,
    sound: GenuineSound,
    mixer: Vec<String>,
    melts: BTreeMap<i64, GoldenMelt>,
    state: Option<Vec<GoldenLevel>>,
}

impl Golden {
    /// The goldens of `spec` at screenblocks `sb`.
    pub fn of(spec: &Spec, sb: u8) -> Self {
        let name = &spec.item.name;
        let (session, frames, sound, mixer_name, melts) = match spec.source {
            Source::Part1 => (
                format!("m6a_session/{name}.session"),
                format!("m6a_frames/{name}.sb{sb}.frames"),
                format!("m6a_sound/{name}.sound"),
                name.clone(),
                if sb == 10 {
                    format!("m6a_melt/{name}.melt")
                } else {
                    format!("m6a2_melt/{name}.sb{sb}.melt")
                },
            ),
            Source::Full => {
                // The mixer golden is a function of the sound stream's starts and the sound
                // lumps only (tools/reference/m5c_mixer_driver.c). FULLREG and FULLRET differ from
                // FINREG and FINRET only by the font WAD, which holds no sound lump, and their
                // genuine sound streams are byte-identical (asserted here), so the genuine mixer
                // goldens of FINREG and FINRET are theirs.
                let twin = name.replace("FULL", "FIN");
                assert_eq!(
                    read(&format!("m6a2_sound/{name}.sound")),
                    read(&format!("m6a_sound/{twin}.sound")),
                    "{name}: the genuine sound streams of {name} and {twin}"
                );
                (
                    format!("m6a2_session/{name}.session"),
                    format!("m6a2_frames/{name}.sb{sb}.frames"),
                    format!("m6a2_sound/{name}.sound"),
                    twin,
                    format!("m6a2_melt/{name}.sb{sb}.melt"),
                )
            }
        };
        let mixer = read(&format!("m6a_mixer/goldens/{mixer_name}.golden"))
            .lines()
            .filter(|l| !l.starts_with("d ") && !l.starts_with("q "))
            .map(str::to_string)
            .collect();
        Self {
            session: Rows::parse(&read(&session)),
            frames: read_frames(&frames).0,
            sound: read_sound(&sound),
            mixer,
            melts: read_melts(&melts),
            state: (sb == 10 && spec.source == Source::Part1)
                .then(|| golden_levels(name))
                .flatten(),
        }
    }

    /// The genuine melts.
    pub fn melts(&self) -> &BTreeMap<i64, GoldenMelt> {
        &self.melts
    }

    /// The genuine buffers (the mixer golden's `b` rows and the repeats of its `r` rows).
    pub fn buffers(&self) -> u64 {
        self.mixer
            .iter()
            .map(|l| match l.split(' ').collect::<Vec<_>>().as_slice() {
                ["b", ..] => 1,
                ["r", n, ..] => n.parse().expect("repeat"),
                _ => 0,
            })
            .sum()
    }

    /// The genuine sound rows.
    pub fn sound_rows(&self) -> usize {
        self.sound.by_tic.values().map(Vec::len).sum()
    }
}

/// What one run compared.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Session tics.
    pub tics: usize,
    /// Level tics (a load included).
    pub level_tics: usize,
    /// Intermission tics.
    pub intermission_tics: usize,
    /// Finale tics (the victory included).
    pub finale_tics: usize,
    /// Loads.
    pub loads: usize,
    /// Wipes (each a melt).
    pub wipes: usize,
    /// G rows.
    pub g_rows: usize,
    /// C rows.
    pub c_rows: usize,
    /// I and J rows.
    pub wi_rows: usize,
    /// F rows.
    pub f_rows: usize,
    /// Level frames.
    pub level_frames: usize,
    /// Intermission frames.
    pub intermission_frames: usize,
    /// Finale frames.
    pub finale_frames: usize,
    /// Sound rows.
    pub sound_rows: usize,
    /// S rows whose listener columns the record does not carry.
    pub s_listener_unchecked: usize,
    /// Mixer buffers.
    pub buffers: u64,
    /// Mixer golden rows.
    pub mixer_rows: usize,
    /// Melts.
    pub melts: usize,
    /// Melt steps.
    pub melt_steps: usize,
    /// Tics asked for during a melt and refused with nothing changed.
    pub tics_refused_in_melt: usize,
    /// State golden levels.
    pub state_levels: usize,
    /// State captures (every level tic).
    pub state_captures: usize,
}

impl Counts {
    /// Adds `o`.
    pub fn add(&mut self, o: &Counts) {
        self.tics += o.tics;
        self.level_tics += o.level_tics;
        self.intermission_tics += o.intermission_tics;
        self.finale_tics += o.finale_tics;
        self.loads += o.loads;
        self.wipes += o.wipes;
        self.g_rows += o.g_rows;
        self.c_rows += o.c_rows;
        self.wi_rows += o.wi_rows;
        self.f_rows += o.f_rows;
        self.level_frames += o.level_frames;
        self.intermission_frames += o.intermission_frames;
        self.finale_frames += o.finale_frames;
        self.sound_rows += o.sound_rows;
        self.s_listener_unchecked += o.s_listener_unchecked;
        self.buffers += o.buffers;
        self.mixer_rows += o.mixer_rows;
        self.melts += o.melts;
        self.melt_steps += o.melt_steps;
        self.tics_refused_in_melt += o.tics_refused_in_melt;
        self.state_levels += o.state_levels;
        self.state_captures += o.state_captures;
    }
}

/// A fault planted in the test's driver (the session's own are `PlantedFault`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriverFault {
    /// A demo command consumed (and dropped) at every melt step.
    CommandInMelt,
    /// The next tic asked for after one melt step instead of after the last.
    TicAfterOneMeltStep,
}

/// The options of one run.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Screenblocks.
    pub sb: u8,
    /// A session fault.
    pub session_fault: Option<PlantedFault>,
    /// A driver fault.
    pub driver_fault: Option<DriverFault>,
}

impl Options {
    /// No fault at screenblocks `sb`.
    pub fn at(sb: u8) -> Self {
        Self {
            sb,
            session_fault: None,
            driver_fault: None,
        }
    }
}

/// The outcome of one run.
pub struct Outcome {
    /// What was compared.
    pub counts: Counts,
    /// The first difference, if any.
    pub first_difference: Option<String>,
    /// The refusal that ended the run, if any.
    pub end: Option<Refusal>,
    /// Why the item is not driven (the easy skills), if so.
    pub not_driven: Option<String>,
}

/// The bijection of Rust actor handles and genuine serials (as `m6a_session.rs`).
#[derive(Default)]
struct Serials {
    to_rust: BTreeMap<String, String>,
    to_c: BTreeMap<String, String>,
}

impl Serials {
    fn compare(&mut self, rust: &str, genuine: &str) -> Result<(), String> {
        let r: Vec<&str> = rust.split(',').collect();
        let g: Vec<&str> = genuine.split(',').collect();
        if r.len() != g.len() {
            return Err("field count".into());
        }
        for (i, (a, b)) in r.iter().zip(&g).enumerate() {
            if let Some(h) = a.strip_prefix('A') {
                let (h, s) = (h.to_string(), b.to_string());
                match (self.to_c.get(&h), self.to_rust.get(&s)) {
                    (None, None) => {
                        self.to_c.insert(h.clone(), s.clone());
                        self.to_rust.insert(s, h);
                    }
                    (Some(c), Some(r)) if *c == s && *r == h => {}
                    _ => return Err(format!("field {i}: handle {h} against serial {s}")),
                }
            } else if a != b {
                return Err(format!("field {i}"));
            }
        }
        Ok(())
    }
}

/// For an S row with no placement on either side, the genuine row with its listener columns
/// replaced by `-` (the record carries no listener for an unplaced start, s_sound.c:304-324).
fn unplaced_listener(rust: &str, genuine: &str) -> Option<String> {
    let r: Vec<&str> = rust.split(',').collect();
    let g: Vec<&str> = genuine.split(',').collect();
    if r.first() != Some(&"S") || g.first() != Some(&"S") || r.len() != 22 || g.len() != 22 {
        return None;
    }
    if r[17..22].iter().any(|f| *f != "-") || g[17] != "-" || g[18] != "-" {
        return None;
    }
    let mut out: Vec<&str> = g[..19].to_vec();
    out.extend(["-", "-", "-"]);
    Some(out.join(","))
}

/// Tags every actor handle with the level ordinal `n`.
fn tag(rows: Vec<String>, n: usize) -> Vec<String> {
    rows.into_iter()
        .map(|r| {
            r.split(',')
                .map(|f| match f.strip_prefix('A') {
                    Some(h) => format!("A{n}/{h}"),
                    None => f.to_string(),
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect()
}

/// The `C` columns of the simulation's player (as `m6a_session.rs`).
fn player_columns(s: &Simulation, outside: bool) -> BTreeMap<&'static str, i64> {
    let p = s.original_player(PlayerSlot::One).expect("player");
    let w = s
        .original_player_weapons(PlayerSlot::One)
        .expect("weapons")
        .expect("pack");
    let armor = s.original_player_armor(PlayerSlot::One).expect("armor");
    let powers = s.original_player_powers(PlayerSlot::One).expect("powers");
    let pickups = s.original_player_pickups(PlayerSlot::One).expect("pickups");
    let finish = s.level_finish().expect("finish");
    let stats = s.level_statistics().expect("statistics");
    let state = match (p.state, finish.reborn) {
        (PlayerBodyState::Live, _) => 0,
        (PlayerBodyState::Dead, false) => 1,
        (PlayerBodyState::Dead, true) => 2,
    };
    let mut m = BTreeMap::new();
    m.insert("playerstate", state);
    m.insert("health", i64::from(p.health));
    m.insert("armorpoints", i64::from(armor.points));
    m.insert("armortype", i64::from(armor.kind.wire_id()));
    m.insert("readyweapon", w.ready as i64);
    m.insert("pendingweapon", w.pending.map_or(10, |x| x as i64));
    m.insert("weaponowned", i64::from(w.owned));
    m.insert("ammo0", i64::from(w.bullets));
    m.insert("ammo1", i64::from(w.shells));
    m.insert("ammo2", i64::from(w.cells));
    m.insert("ammo3", i64::from(w.rockets));
    let max = [200_i64, 50, 300, 50].map(|v| v << u32::from(w.backpack));
    for (k, v) in max.iter().enumerate() {
        m.insert(["maxammo0", "maxammo1", "maxammo2", "maxammo3"][k], *v);
    }
    m.insert("backpack", i64::from(w.backpack));
    // G_PlayerFinishLevel (g_game.c:785-791) clears cards, powers and both palette counts.
    let keep = i64::from(!outside);
    let cards: i64 = powers
        .cards
        .iter()
        .enumerate()
        .map(|(k, c)| i64::from(*c) << k)
        .sum();
    m.insert("cards", cards * keep);
    for (k, v) in powers.powers.iter().enumerate() {
        m.insert(
            ["pw0", "pw1", "pw2", "pw3", "pw4", "pw5"][k],
            i64::from(*v) * keep,
        );
    }
    m.insert("cheats", 0);
    m.insert("killcount", i64::from(stats.single_player_kill_count));
    m.insert("itemcount", i64::from(pickups.item_count));
    m.insert(
        "secretcount",
        i64::from(s.original_player_secret_count().expect("secrets")),
    );
    m.insert("refire", i64::from(w.refire));
    m.insert("damagecount", i64::from(p.damage_count) * keep);
    m.insert("bonuscount", i64::from(pickups.bonus_count) * keep);
    if !outside {
        m.insert("attackdown", i64::from(w.attack_down));
        let usedown = match finish.carried {
            Some(c) => c.use_down,
            None => s
                .snapshot()
                .fields()
                .find(|f| f.path() == "original_player.0.use_down")
                .is_some_and(|f| f.value() == [1]),
        };
        m.insert("usedown", i64::from(usedown));
    }
    m
}

/// One Rust level for the state comparison.
#[derive(Debug, Default)]
struct RustLevel {
    tic: i64,
    map: u8,
    kind: String,
    captures: Vec<(String, i64)>,
    map_sha: Option<String>,
}

/// One run in progress.
struct Run<'a> {
    name: &'a str,
    sb: u8,
    golden: &'a Golden,
    counts: Counts,
    serials: Serials,
    carry: Carry,
    writer: GoldenWriter,
    level_ordinal: usize,
    st_draws: i64,
    wipe_draws: i64,
    snd_draws: i64,
    wi_draws: i64,
    listener: Option<ActorId>,
    levels: Vec<RustLevel>,
}

impl Run<'_> {
    fn diff(&self, tic: i64, what: impl std::fmt::Display) -> String {
        format!("{} sb{} tic {tic}: {what}", self.name, self.sb)
    }

    fn g_row(&mut self, s: &Session, tic: i64) -> Result<(), String> {
        let g = self
            .golden
            .session
            .at("G", tic)
            .ok_or_else(|| self.diff(tic, "a tic the genuine run does not have"))?;
        let gamestate = match s.gamestate() {
            GameState::Level => 0,
            GameState::Intermission => 1,
            GameState::Finale => 2,
        };
        let got: [(&str, i64); 19] = [
            ("gamestate", gamestate),
            ("gameaction", i64::from(s.gameaction())),
            ("episode", 1),
            ("map", i64::from(s.map())),
            ("leveltime", s.level().tic() as i64),
            ("gametic", s.gametic() as i64),
            ("levelstarttic", s.levelstarttic() as i64),
            ("prndindex", i64::from(s.level().rng().indices()[0])),
            ("rndindex", i64::from(s.display().random_index())),
            ("snd_draws", self.snd_draws),
            ("st_draws", self.st_draws),
            ("wipe_draws", self.wipe_draws),
            ("wi_draws", self.wi_draws),
            ("fin_draws", 0),
            ("other_draws", 0),
            ("wipegamestate", gamestate),
            ("secretexit", i64::from(s.secretexit())),
            ("loads", self.counts.loads as i64 + 1),
            ("wipes", self.counts.wipes as i64),
        ];
        for (name, v) in got {
            if int(g, name) != v {
                return Err(self.diff(tic, format!("G {name} {v} genuine {}", g[name])));
            }
        }
        self.counts.g_rows += 1;
        Ok(())
    }

    fn c_row(&mut self, s: &Session, tic: i64) -> Result<(), String> {
        let Some(c) = self.golden.session.at("C", tic) else {
            return Ok(());
        };
        let outside = s.gamestate() != GameState::Level;
        let mut m = player_columns(s.level(), outside);
        if outside {
            let carried = s
                .carried()
                .ok_or_else(|| self.diff(tic, "no carried player"))?;
            m.insert("attackdown", i64::from(carried.latches.attackdown));
            m.insert("usedown", i64::from(carried.latches.usedown));
            m.insert("viewheight", i64::from(carried.viewheight));
            m.insert("deltaviewheight", i64::from(carried.deltaviewheight));
        } else {
            let v = s.display().level_view().expect("a level view");
            m.insert("viewheight", i64::from(v.viewheight));
            m.insert("deltaviewheight", i64::from(v.deltaviewheight));
        }
        m.insert("didsecret", i64::from(s.didsecret()));
        for (name, v) in m {
            if int(c, name) != v {
                return Err(self.diff(tic, format!("C {name} {v} genuine {}", c[name])));
            }
        }
        self.counts.c_rows += 1;
        Ok(())
    }

    fn wi_rows(&mut self, s: &Session, tic: i64) -> Result<(), String> {
        let (Some(i), Some(j)) = (
            self.golden.session.at("I", tic),
            self.golden.session.at("J", tic),
        ) else {
            if s.gamestate() == GameState::Intermission {
                return Err(self.diff(tic, "no genuine I or J row in the intermission"));
            }
            return Ok(());
        };
        let w = s.wminfo();
        let wi = s.intermission();
        let got: [(&str, i64, &BTreeMap<String, String>); 30] = [
            ("epsd", i64::from(w.epsd), i),
            ("didsecret", i64::from(w.didsecret), i),
            ("last", i64::from(w.last), i),
            ("next", i64::from(w.next), i),
            ("maxkills", i64::from(w.maxkills), i),
            ("maxitems", i64::from(w.maxitems), i),
            ("maxsecret", i64::from(w.maxsecret), i),
            ("maxfrags", 0, i),
            ("partime", i64::from(w.partime), i),
            ("pnum", 0, i),
            ("in", 1, i),
            ("skills", i64::from(w.plyr.skills), i),
            ("sitems", i64::from(w.plyr.sitems), i),
            ("ssecret", i64::from(w.plyr.ssecret), i),
            ("stime", i64::from(w.plyr.stime), i),
            ("score", 0, i),
            ("state", i64::from(wi.state), j),
            ("acceleratestage", i64::from(wi.acceleratestage), j),
            ("me", 0, j),
            ("cnt", i64::from(wi.cnt), j),
            ("bcnt", i64::from(wi.bcnt), j),
            ("firstrefresh", i64::from(wi.firstrefresh), j),
            ("sp_state", i64::from(wi.sp_state), j),
            ("cnt_kills", i64::from(wi.cnt_kills), j),
            ("cnt_items", i64::from(wi.cnt_items), j),
            ("cnt_secret", i64::from(wi.cnt_secret), j),
            ("cnt_time", i64::from(wi.cnt_time), j),
            ("cnt_par", i64::from(wi.cnt_par), j),
            ("cnt_pause", i64::from(wi.cnt_pause), j),
            ("snl_pointeron", i64::from(wi.snl_pointeron), j),
        ];
        for (name, v, row) in got {
            if int(row, name) != v {
                return Err(self.diff(tic, format!("{name} {v} genuine {}", row[name])));
            }
        }
        let ctr: Vec<String> = wi.anims.iter().map(|a| a.ctr.to_string()).collect();
        let next: Vec<String> = wi.anims.iter().map(|a| a.nexttic.to_string()).collect();
        if j["anim_ctr"] != ctr.join(" ")
            || j["anim_nexttic"] != next.join(" ")
            || j["nanims"] != "10"
            || j["wbs_is_wminfo"] != "1"
        {
            return Err(self.diff(tic, "animations differ"));
        }
        self.counts.wi_rows += 2;
        Ok(())
    }

    /// The F row: `finalestage`, `finalecount`, and Episode 1's flat and text (f_finale.c:117-121).
    fn f_row(&mut self, s: &Session, tic: i64) -> Result<(), String> {
        match (s.finale(), self.golden.session.at("F", tic)) {
            (None, None) => Ok(()),
            (Some(v), Some(f)) => {
                let want = (int(f, "finalestage"), int(f, "finalecount"));
                if (i64::from(v.stage), i64::from(v.count)) != want
                    || f["flat"] != "FLOOR4_8"
                    || f["text"] != "1"
                {
                    return Err(self.diff(
                        tic,
                        format!(
                            "F stage and count {:?} genuine {want:?}",
                            (v.stage, v.count)
                        ),
                    ));
                }
                self.counts.f_rows += 1;
                Ok(())
            }
            (v, f) => Err(self.diff(tic, format!("F {v:?}, genuine F row {}", f.is_some()))),
        }
    }

    fn frame(&mut self, s: &Session, r: &FinaleMeltTicReport, tic: i64) -> Result<(), String> {
        let g = self
            .golden
            .frames
            .get(&tic)
            .ok_or_else(|| self.diff(tic, "a frame the genuine run does not have"))?;
        let px = s.frame().pixels();
        match &r.level_frame {
            Some(f) => {
                if g.gamestate != 0 || g.f.is_empty() {
                    return Err(self.diff(tic, "a level frame where the genuine one is not"));
                }
                if sha256(px) != g.sha256 {
                    return Err(self.diff(tic, "level frame differs"));
                }
                let q = g.q;
                let mut nf = px.to_vec();
                for y in q[3] as usize..(q[5] as usize).min(SCREENHEIGHT) {
                    for x in q[2] as usize..(q[4] as usize).min(SCREENWIDTH) {
                        nf[y * SCREENWIDTH + x] = 0;
                    }
                }
                if sha256(&nf) != g.noface {
                    return Err(self.diff(tic, "face-zeroed digest differs"));
                }
                if [q[0], q[1]] != [self.snd_draws, self.wipe_draws] {
                    return Err(self.diff(tic, "Q draws differ"));
                }
                let c = f.view.counters;
                let got = vec![
                    i64::from(f.view.fuzzpos),
                    i64::from(c.mapped),
                    i64::from(c.visplanes),
                    i64::from(c.drawsegs),
                    i64::from(c.vissprites),
                    i64::from(c.openings),
                    i64::from(c.solidsegs_peak),
                    i64::from(c.drawseg_drops),
                    i64::from(c.vissprite_drop),
                ];
                if got != g.f
                    || f.view.newly_mapped != g.mapped
                    || f.view.switch_parity_unknown
                    || !f.view.flagged_columns.is_empty()
                {
                    return Err(self.diff(tic, format!("F {got:?} genuine {:?}", g.f)));
                }
                let status = vec![
                    i64::from(f.palette),
                    f.palette_set.map_or(-1, i64::from),
                    i64::from(f.face),
                    i64::from(f.message_on),
                    i64::from(s.display().random_index()),
                ];
                if status != g.status {
                    return Err(self.diff(tic, format!("P {status:?} genuine {:?}", g.status)));
                }
                self.counts.level_frames += 1;
            }
            None => {
                let state = match s.gamestate() {
                    GameState::Intermission => 1,
                    GameState::Finale => 2,
                    GameState::Level => 0,
                };
                if g.gamestate != state || state == 0 {
                    return Err(self.diff(
                        tic,
                        format!("a frame of game state {state}, genuine {}", g.gamestate),
                    ));
                }
                if sha256(px) != g.sha256 {
                    return Err(self.diff(tic, format!("game state {state} frame differs")));
                }
                let status = vec![
                    i64::from(r.palette),
                    r.palette_set.map_or(-1, i64::from),
                    i64::from(s.display().random_index()),
                ];
                if status != g.status {
                    return Err(self.diff(tic, format!("G {status:?} genuine {:?}", g.status)));
                }
                if state == 1 {
                    self.counts.intermission_frames += 1;
                } else {
                    self.counts.finale_frames += 1;
                }
            }
        }
        Ok(())
    }

    /// The tic's sound rows and its mixer starts (and its buffer when it did not melt).
    fn sound(
        &mut self,
        s: &Session,
        r: &FinaleMeltTicReport,
        tic: i64,
        old_listener: Option<ActorId>,
        new_level: bool,
    ) -> Result<(), String> {
        let level = r.level_frame.is_some();
        let listener = s.level().sound_listener().map(|l| l.actor);
        let records: Vec<SoundRecord> = if level {
            s.level().sound_records().to_vec()
        } else {
            r.wi_starts
                .iter()
                .map(|w| SoundRecord::Start {
                    sfx: w.sfx(),
                    origin: SoundOrigin::None,
                    at: None,
                })
                .collect()
        };
        let first_seq = self.carry.seq;
        let rows = stream_rows_with(
            &r.sound.sound,
            &records,
            s.sound().sound(),
            listener,
            &mut self.carry,
            r.sound.sound.rndindex.wrapping_add(1),
            s.display().random_index(),
        );
        let origin = |o: SoundOrigin| match o {
            SoundOrigin::None => "none,-1".to_string(),
            SoundOrigin::Actor(a) => format!(
                "{},A{}:{}",
                if Some(a) == old_listener {
                    "player"
                } else {
                    "mobj"
                },
                a.slot(),
                a.generation()
            ),
            SoundOrigin::Sector(x) => format!("sector,{x}"),
            SoundOrigin::Button(b) => format!("button,{b}"),
        };
        let k_rows: Vec<String> = r
            .sound
            .level_start
            .iter()
            .map(|k| {
                format!(
                    "K,{tic},0,{},{},{},{},level_start,{}",
                    k.cnum,
                    k.entry.sfx.get(),
                    origin(k.entry.origin),
                    k.entry.handle,
                    u8::from(k.playing)
                )
            })
            .collect();
        let before = if new_level {
            self.level_ordinal - 1
        } else {
            self.level_ordinal
        };
        let mut rows = tag(rows, self.level_ordinal);
        rows.splice(0..0, tag(k_rows, before));
        let last = rows.last_mut().expect("an R row");
        let mut f: Vec<String> = last.split(',').map(str::to_string).collect();
        f[6] = self.st_draws.to_string();
        f[7] = self.wipe_draws.to_string();
        if !level {
            f[2] = "-1".into();
            f[3] = "-1".into();
        }
        *last = f.join(",");
        let genuine = self
            .golden
            .sound
            .by_tic
            .get(&tic)
            .cloned()
            .unwrap_or_default();
        for i in 0..rows.len().max(genuine.len()) {
            let rr = rows.get(i).map_or("<none>", String::as_str);
            let gg = genuine.get(i).map_or("<none>", String::as_str);
            let patched;
            let gg = match unplaced_listener(rr, gg) {
                Some(p) => {
                    self.counts.s_listener_unchecked += 1;
                    patched = p;
                    patched.as_str()
                }
                None => gg,
            };
            if let Err(e) = self.serials.compare(rr, gg) {
                return Err(self.diff(
                    tic,
                    format!("sound row {i} ({e}):\n  genuine: {gg}\n  rust:    {rr}"),
                ));
            }
            self.counts.sound_rows += 1;
        }
        self.writer.starts(&r.sound.sound, first_seq);
        if let Some(b) = &r.sound.sound.buffer {
            self.writer
                .buffer(r.sound.sound.tic, b, s.sound().sound().mixer().mask());
        }
        Ok(())
    }

    /// A capture of the level for the state comparison (screenblocks 10, items of the golden).
    fn capture(&mut self, s: &Session, r: &FinaleMeltTicReport, tic: i64) {
        if self.golden.state.is_none() {
            return;
        }
        match r.kind {
            FinaleMeltTicKind::Load { map, start } => self.levels.push(RustLevel {
                tic,
                map,
                kind: format!("{start:?}").to_lowercase(),
                ..RustLevel::default()
            }),
            FinaleMeltTicKind::Level => {}
            _ => return,
        }
        let level = self.levels.last_mut().expect("level");
        let (state, map) = digests(&s.level().snapshot());
        level.map_sha.get_or_insert(map);
        level.captures.push((state, tic));
    }

    fn compare_state(&mut self) -> Result<(), String> {
        let Some(golden) = &self.golden.state else {
            return Ok(());
        };
        let name = self.name;
        if self.levels.len() != golden.len() {
            return Err(format!(
                "{name}: {} levels, the genuine run {}",
                self.levels.len(),
                golden.len()
            ));
        }
        for (k, (r, g)) in self.levels.iter().zip(golden).enumerate() {
            let head = (r.tic, r.map, r.kind.as_str(), r.captures.len());
            if head != (g.tic, g.map, g.kind.as_str(), g.captures) {
                return Err(format!("{name} load {k}: {head:?}, genuine {g:?}"));
            }
            if r.map_sha.as_deref() != Some(g.map_sha.as_str()) {
                return Err(format!("{name} load {k}: map group differs"));
            }
            for (&lt, (tic, why, want)) in &g.points {
                let (got, rtic) = &r.captures[lt - 1];
                if rtic != tic || got != want {
                    return Err(format!(
                        "{name} load {k}: session tic {tic} ({why}): state differs"
                    ));
                }
            }
            let all: Vec<u8> = r.captures.iter().flat_map(|c| unhex(&c.0)).collect();
            if sha256(&all) != g.sequence {
                return Err(format!("{name} load {k}: the sequence differs"));
            }
            self.counts.state_levels += 1;
            self.counts.state_captures += r.captures.len();
        }
        Ok(())
    }
}

/// Whether `end` is the genuine stop of `item` (a missing music lump at the genuine tic, or the
/// `HELP2` display stop at finale tic 1,571).
fn expected_stop(item: &Item, end: &Refusal) -> bool {
    if end.tic != item.session_tics as u64 {
        return false;
    }
    if item.end == "error:W_GetNumForName: HELP2 not found!" {
        return matches!(&end.witness, SessionWitness::Display(SessionError::Finale(e))
            if e.to_string() == "W_GetNumForName: HELP2 not found!");
    }
    let music = item
        .end
        .strip_prefix("error:W_GetNumForName: ")
        .and_then(|m| m.strip_suffix(" not found!"));
    let Some(music) = music else { return false };
    matches!(&end.witness, SessionWitness::Sound(doom_game::session::sound::SoundWitness::Chain(ChainRefusal::MusicMissing { music: m, tic: Some(t) })) if m.lump_name() == music && *t == end.tic)
}

/// The session settings of an item at `sb`.
pub fn settings(item: &Item, sb: u8) -> SessionSettings {
    SessionSettings {
        map: item.map,
        skill: item.skill,
        screenblocks: sb,
    }
}

/// The demo commands of an item, or why its demo is refused (the easy skills).
pub fn commands(item: &Item) -> Result<Vec<TicCommand>, String> {
    let lmp = std::fs::read(m6a::root(&item.lmp)).expect("demo");
    let demo = SessionDemo::parse(&lmp).map_err(|e| format!("demo refused: {e:?}"))?;
    assert_eq!((demo.map, demo.skill), (item.map, item.skill));
    Ok(demo.commands)
}

/// A new finale-and-melt session of `item` at `sb`, or why it does not start.
pub fn new_session(item: &Item, sb: u8) -> Result<Session, StartError> {
    Session::new_finale_and_melt(m6a::data(item, None), settings(item, sb))
}

/// Runs `spec` with `opts` against its goldens (module documentation).
pub fn run(spec: &Spec, opts: Options) -> Outcome {
    let item = &spec.item;
    let mut out = Outcome {
        counts: Counts::default(),
        first_difference: None,
        end: None,
        not_driven: None,
    };
    let commands = match commands(item) {
        Ok(c) => c,
        Err(e) => {
            out.not_driven = Some(e);
            return out;
        }
    };
    let mut s = match new_session(item, opts.sb) {
        Ok(s) => s,
        Err(StartError::Sound(w)) => {
            // NOE1M2AT: the first map's music at the session's load, before any tic.
            let ok = item.session_tics == 0
                && matches!(&w, doom_game::session::sound::SoundWitness::Chain(ChainRefusal::MusicMissing { music, tic: None }) if item.end.contains(&music.lump_name()));
            out.first_difference = (!ok).then(|| format!("{}: start refused {w:?}", item.name));
            return out;
        }
        Err(StartError::NotYet(NotYet::EasySkill)) => {
            out.not_driven = Some("the easy skills are refused".into());
            return out;
        }
        Err(e) => {
            out.first_difference = Some(format!("{}: start {e:?}", item.name));
            return out;
        }
    };
    if let Some(f) = opts.session_fault {
        s.plant_fault(f);
    }
    let golden = Golden::of(spec, opts.sb);
    let mut run = Run {
        name: &item.name,
        sb: opts.sb,
        golden: &golden,
        counts: Counts::default(),
        serials: Serials::default(),
        carry: Carry::default(),
        writer: GoldenWriter::new(s.sound().sound().mixer().sound_set()),
        level_ordinal: 0,
        st_draws: 0,
        wipe_draws: 0,
        snd_draws: 0,
        wi_draws: 0,
        listener: s.level().sound_listener().map(|l| l.actor),
        levels: vec![RustLevel {
            tic: -1,
            map: item.map,
            kind: "new".into(),
            ..RustLevel::default()
        }],
    };
    let mut next = 0usize;
    let result: Result<(), String> = (|| {
        while let Some(&cmd) = commands.get(next) {
            next += 1;
            let tic = s.gametic() as i64;
            let before = s.gametic();
            let r = match s.tic_finale_and_melt(cmd) {
                Ok(r) => r,
                Err(e) => {
                    if !matches!(e.witness, SessionWitness::Display(_)) && s.gametic() != before {
                        return Err(format!("{}: a refusal changed gametic", item.name));
                    }
                    out.end = Some(e);
                    break;
                }
            };
            let old_listener = run.listener;
            let new_level = matches!(r.kind, FinaleMeltTicKind::Load { .. });
            if new_level {
                run.counts.loads += 1;
                run.level_ordinal += 1;
            }
            run.counts.tics += 1;
            match r.kind {
                FinaleMeltTicKind::Level | FinaleMeltTicKind::Load { .. } => {
                    run.counts.level_tics += 1;
                    run.st_draws += 1;
                }
                FinaleMeltTicKind::Completion | FinaleMeltTicKind::Intermission => {
                    run.counts.intermission_tics += 1;
                }
                FinaleMeltTicKind::Victory | FinaleMeltTicKind::Finale => {
                    run.counts.finale_tics += 1;
                }
            }
            if r.wiped {
                run.counts.wipes += 1;
                run.wipe_draws += 320;
            }
            run.snd_draws += i64::from(r.sound.sound.draws);
            run.wi_draws += i64::from(r.wi_draws);
            run.listener = s.level().sound_listener().map(|l| l.actor);
            run.g_row(&s, tic)?;
            run.c_row(&s, tic)?;
            run.wi_rows(&s, tic)?;
            run.f_row(&s, tic)?;
            run.frame(&s, &r, tic)?;
            run.sound(&s, &r, tic, old_listener, new_level)?;
            run.capture(&s, &r, tic);
            melt(&mut run, &mut s, &r, tic, opts, &mut next)?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        out.first_difference = Some(e);
        out.counts = run.counts;
        return out;
    }
    let genuine_end_ok = match &out.end {
        None => item.end == "marker" && run.counts.tics == item.session_tics,
        Some(e) => expected_stop(item, e),
    };
    if !genuine_end_ok {
        out.first_difference = Some(format!(
            "{} sb{}: ended {:?} after {} tics; the genuine run: {} after {} tics",
            item.name, opts.sb, out.end, run.counts.tics, item.end, item.session_tics
        ));
    } else if run.counts.melts != golden.melts.len() {
        out.first_difference = Some(format!(
            "{} sb{}: {} melts, the genuine run has {}",
            item.name,
            opts.sb,
            run.counts.melts,
            golden.melts.len()
        ));
    } else if let Err(e) = run.compare_state() {
        out.first_difference = Some(e);
    } else {
        drain(s.sound().sound(), &mut run.writer);
        run.counts.buffers = run.writer.buffers();
        let want = &golden.mixer;
        let got = std::mem::replace(
            &mut run.writer,
            GoldenWriter::new(s.sound().sound().mixer().sound_set()),
        )
        .finish();
        let n = want.len().max(got.len());
        if let Some(i) = (0..n).find(|i| want.get(*i) != got.get(*i)) {
            out.first_difference = Some(format!(
                "{} sb{} mixer row {i}:\n  golden: {:?}\n  rust:   {:?}",
                item.name,
                opts.sb,
                want.get(i),
                got.get(i)
            ));
        }
        run.counts.mixer_rows = want.len();
    }
    out.counts = run.counts;
    out
}

/// The melt the tic's display started, if any: every step against the genuine V rows; a tic asked
/// for during it refused with nothing changed; the sound unchanged by every step but the last; the
/// owed buffer after the last step.
fn melt(
    run: &mut Run<'_>,
    s: &mut Session,
    r: &FinaleMeltTicReport,
    tic: i64,
    opts: Options,
    next: &mut usize,
) -> Result<(), String> {
    let genuine = run.golden.melts.get(&tic);
    if !r.wiped {
        return match (s.melt_in_progress(), genuine) {
            (false, None) => Ok(()),
            (true, _) => Err(run.diff(tic, "a melt without a wipe")),
            (false, Some(_)) => Err(run.diff(tic, "no melt where the genuine run melts")),
        };
    }
    let gm = genuine.ok_or_else(|| run.diff(tic, "a melt the genuine run does not make"))?;
    if !s.melt_in_progress() {
        return Err(run.diff(tic, "a wipe without a melt"));
    }
    let (start, end) = s.display().melt_screens().expect("a melt");
    if (sha256(start), sha256(end)) != (gm.start.clone(), gm.end.clone()) {
        return Err(run.diff(tic, "melt start or end screen differs"));
    }
    let palette = s.display().melt_palette().expect("a melt");
    let palette_set = r.palette_set.map_or(-1, i64::from);
    if (i64::from(palette), palette_set) != (gm.palette, gm.palette_set) {
        return Err(run.diff(
            tic,
            format!(
                "melt palette {:?} genuine {:?}",
                (palette, palette_set),
                (gm.palette, gm.palette_set)
            ),
        ));
    }
    if opts.session_fault != Some(PlantedFault::TicInMelt) {
        // A tic asked for during the melt: refused, nothing changed.
        let (g, shown, sound) = (
            s.gametic(),
            s.display().melt_steps_shown(),
            s.sound().clone(),
        );
        match s.tic_finale_and_melt(TicCommand::default()) {
            Err(e) if e.witness == SessionWitness::MeltInProgress => {}
            other => {
                return Err(run.diff(
                    tic,
                    format!("a tic during the melt: {:?}", other.map(|r| r.tic)),
                ));
            }
        }
        if (s.gametic(), s.display().melt_steps_shown(), s.sound()) != (g, shown, &sound) {
            return Err(run.diff(tic, "a refused tic changed the session"));
        }
        run.counts.tics_refused_in_melt += 1;
    }
    let mut picture = Frame::new();
    let mut steps: Vec<(String, bool)> = Vec::new();
    loop {
        let sound = s.sound().clone();
        let m = s
            .melt_step(&mut picture)
            .map_err(|e| run.diff(tic, format!("melt step refused: {e}")))?;
        steps.push((sha256(picture.pixels()), m.done));
        if usize::from(m.step) != steps.len() || m.palette != palette || m.tic != tic as u64 {
            return Err(run.diff(tic, format!("melt step report {m:?}")));
        }
        if opts.driver_fault == Some(DriverFault::CommandInMelt) {
            *next += 1;
        }
        if m.done {
            if let Some(b) = &m.buffer {
                run.writer
                    .buffer(tic as u64, b, s.sound().sound().mixer().mask());
            }
            break;
        }
        if s.sound() != &sound || m.buffer.is_some() {
            return Err(run.diff(tic, format!("melt step {} changed the sound", m.step)));
        }
        if opts.driver_fault == Some(DriverFault::TicAfterOneMeltStep) {
            break;
        }
    }
    for (k, step) in steps.iter().enumerate() {
        match gm.steps.get(k) {
            Some(want) if want == step => {}
            Some(_) => return Err(run.diff(tic, format!("melt step {} picture differs", k + 1))),
            None => return Err(run.diff(tic, format!("melt has more than {k} steps"))),
        }
    }
    if steps.len() != gm.steps.len() {
        return Err(run.diff(
            tic,
            format!("melt has {} steps, genuine {}", steps.len(), gm.steps.len()),
        ));
    }
    run.counts.melts += 1;
    run.counts.melt_steps += steps.len();
    Ok(())
}
