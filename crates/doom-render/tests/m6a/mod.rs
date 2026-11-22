//! Shared support for the M6a display tests: the 26 items of the genuine M6a goldens
//! (`tests/reference/m6a_manifest.json`, `m6a_session/*.session` DOOM-M6A-SESSION-v1 and
//! `m6a_frames/*.sb<N>.frames`, written by the genuine engine only:
//! `tools/reference/m6a_goldens.py`), their lump lists (the manifest's WAD sets, then the demo as a
//! single lump, as the M5c support does) and readers of the rows the display tests use.
#![allow(dead_code, reason = "each test target uses a subset")]

use crate::common::reference;
use doom_render::{GameMode, IntermissionView, LumpDirectory, Resources, WiState};
use std::collections::BTreeMap;
use std::sync::Arc;

/// One manifest item.
#[derive(Clone, Debug)]
pub struct Item {
    /// Name (the goldens' file stem).
    pub name: String,
    /// `registered` or `retail`.
    pub mode: String,
    /// The WAD files, as paths under the repository root.
    pub wads: Vec<String>,
    /// The demo, a path under the repository root.
    pub lmp: String,
    /// The start map and the skill byte.
    pub map: u8,
    /// See [`Item::map`].
    pub skill: u8,
}

/// The repository root.
pub fn root(path: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// Every manifest item, in manifest order.
pub fn items() -> Vec<Item> {
    let text = std::fs::read_to_string(reference("m6a_manifest.json")).expect("manifest");
    let m: serde_json::Value = serde_json::from_str(&text).expect("manifest JSON");
    assert_eq!(m["schema"], "DOOM-M6A-MANIFEST-v1");
    let sets = &m["wadsets"];
    m["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| Item {
            name: i["name"].as_str().expect("name").into(),
            mode: i["mode"].as_str().expect("mode").into(),
            wads: sets[i["wadset"].as_str().expect("wadset")]
                .as_array()
                .expect("WAD set")
                .iter()
                .map(|w| w.as_str().expect("WAD").into())
                .collect(),
            lmp: i["lmp"].as_str().expect("lmp").into(),
            map: i["map"].as_u64().expect("map") as u8,
            skill: i["skill"].as_u64().expect("skill") as u8,
        })
        .collect()
}

/// The resources of an item's lump list.
pub fn resources(item: &Item) -> Arc<Resources> {
    let files: Vec<Vec<u8>> = item
        .wads
        .iter()
        .map(|w| std::fs::read(root(w)).expect("WAD"))
        .collect();
    let mut dir = LumpDirectory::new();
    for b in &files {
        let wad = doom_wad::Wad::parse(b, doom_wad::Limits::default()).expect("WAD");
        dir.add_wad(&wad).expect("WAD lumps");
    }
    let lmp = std::fs::read(root(&item.lmp)).expect("demo");
    dir.add_single_lump(&item.lmp, &lmp).expect("demo lump");
    let mode = match item.mode.as_str() {
        "registered" => GameMode::Registered,
        "retail" => GameMode::Retail,
        m => panic!("mode {m}"),
    };
    Arc::new(Resources::load(dir, mode).expect("resources"))
}

/// The session G row of a tic (DOOM-M6A-SESSION-v1).
#[derive(Clone, Copy, Debug, Default)]
pub struct Tic {
    /// `gamestate` after the tic (0 level, 1 intermission, 2 finale).
    pub gamestate: i64,
    /// `gamemap`.
    pub map: u8,
    /// `rndindex` after the tic and its display.
    pub rndindex: u8,
    /// Cumulative draws: sound, status bar, wipe, intermission, finale, other.
    pub draws: [i64; 6],
}

/// The intermission rows of a tic: I (`wminfo`) and J (the wi_stuff.c statics after the tic and
/// its display).
#[derive(Clone, Debug)]
pub struct Wi {
    /// `state`.
    pub state: i64,
    /// `cnt`.
    pub cnt: i64,
    /// `cnt_kills`, `cnt_items`, `cnt_secret`, `cnt_time`, `cnt_par`.
    pub counts: [i32; 5],
    /// `snl_pointeron` after the display.
    pub snl_pointeron: bool,
    /// `epsd`, `didsecret`, `last`, `next`.
    pub wminfo: [i32; 4],
}

impl Wi {
    /// The drawer's view, with `snl_pointeron` as given.
    pub fn view(&self, snl_pointeron: bool) -> IntermissionView {
        let [k, i, s, t, p] = self.counts;
        IntermissionView {
            state: match self.state {
                -1 => WiState::NoState,
                0 => WiState::StatCount,
                1 => WiState::ShowNextLoc,
                s => panic!("state {s}"),
            },
            cnt_kills: k,
            cnt_items: i,
            cnt_secret: s,
            cnt_time: t,
            cnt_par: p,
            snl_pointeron,
            epsd: self.wminfo[0],
            didsecret: self.wminfo[1] != 0,
            last: self.wminfo[2],
            next: self.wminfo[3],
        }
    }
}

/// One session golden.
#[derive(Debug, Default)]
pub struct Session {
    /// G rows by tic.
    pub tics: BTreeMap<i64, Tic>,
    /// L rows: the tic of every load after the first (whose tic is -1), with its map.
    pub loads: BTreeMap<i64, u8>,
    /// I and J rows by tic.
    pub wi: BTreeMap<i64, Wi>,
}

/// Reads `tests/reference/m6a_session/<name>.session`.
pub fn read_session(name: &str) -> Session {
    let text = std::fs::read_to_string(reference(&format!("m6a_session/{name}.session")))
        .expect("session");
    let mut s = Session::default();
    if text.is_empty() {
        // NOE1M2AT stops at its first load, before any tic.
        return s;
    }
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("#schema,DOOM-M6A-SESSION-v1"));
    let mut minfo: BTreeMap<i64, [i32; 4]> = BTreeMap::new();
    for l in lines {
        let c: Vec<&str> = l.split(',').collect();
        let n = |i: usize| c[i].parse::<i64>().expect("number");
        match c[0] {
            "G" => {
                let t = Tic {
                    gamestate: n(2),
                    map: n(5) as u8,
                    rndindex: n(10) as u8,
                    draws: std::array::from_fn(|i| n(11 + i)),
                };
                assert!(s.tics.insert(n(1), t).is_none(), "{name}: {l}");
            }
            "L" => {
                s.loads.insert(n(1), n(3) as u8);
            }
            "I" => {
                minfo.insert(n(1), std::array::from_fn(|i| n(2 + i) as i32));
            }
            "J" => {
                let w = Wi {
                    state: n(2),
                    cnt: n(5),
                    counts: std::array::from_fn(|i| n(9 + i) as i32),
                    snl_pointeron: n(15) != 0,
                    wminfo: minfo[&n(1)],
                };
                s.wi.insert(n(1), w);
            }
            _ => {}
        }
    }
    s
}

/// A frame row of the M6a frames golden: the M5c F and P rows of a level frame, or the M6a G row
/// of a frame outside a level.
#[derive(Clone, Debug, Default)]
pub struct FrameRow {
    /// `gamestate` (G rows; 0 for F rows).
    pub gamestate: i64,
    /// The whole-frame digest (`undefined` for a tracker-flagged level frame).
    pub sha256: String,
    /// Palette, palette set (-1 none), `M_Random` index (the P row's for a level frame).
    pub palette: [i64; 3],
}

/// Reads `tests/reference/m6a_frames/<name>.sb<sb>.frames`.
pub fn read_frames(name: &str, sb: u8) -> BTreeMap<i64, FrameRow> {
    let text = std::fs::read_to_string(reference(&format!("m6a_frames/{name}.sb{sb}.frames")))
        .expect("frames");
    let mut out: BTreeMap<i64, FrameRow> = BTreeMap::new();
    if text.is_empty() {
        return out;
    }
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("#schema,DOOM-M5C-FRAMES-v1"));
    for l in lines {
        let c: Vec<&str> = l.split(',').collect();
        let n = |i: usize| c[i].parse::<i64>().expect("number");
        match c[0] {
            "G" => {
                let r = FrameRow {
                    gamestate: n(2),
                    sha256: c[3].into(),
                    palette: [n(4), n(5), n(6)],
                };
                assert!(out.insert(n(1), r).is_none(), "{name}: {l}");
            }
            "F" => {
                let r = FrameRow {
                    gamestate: 0,
                    sha256: c[2].into(),
                    palette: [0; 3],
                };
                assert!(out.insert(n(1), r).is_none(), "{name}: {l}");
            }
            "P" => {
                let r = out.get_mut(&n(1)).expect("P after F");
                r.palette = [n(2), n(3), n(6)];
            }
            _ => {}
        }
    }
    out
}

/// `snl_pointeron` before the display of each intermission tic, from the logic's rule alone
/// (not from the J rows): false in a fresh process (wi_stuff.c:749); on the tic that enters
/// ShowNextLoc (`cnt` 140 and no update yet, wi_stuff.c:752-759) the value the last frame left;
/// in ShowNextLoc after an update `(cnt & 31) < 20` (wi_stuff.c:768); in the other states
/// unchanged by the logic (the drawer sets it in NoState, wi_stuff.c:814).
pub fn pointer_before(w: &Wi, after_last_frame: bool) -> bool {
    match (w.state, w.cnt) {
        (1, 140) => after_last_frame,
        (1, cnt) => (cnt & 31) < 20,
        _ => after_last_frame,
    }
}
