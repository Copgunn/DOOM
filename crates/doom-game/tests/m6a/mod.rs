//! Shared support for the M6a session tests: the items of the genuine M6a goldens
//! (`tests/reference/m6a_manifest.json`, `m6a_results.json`, `m6a_session/*.session`
//! DOOM-M6A-SESSION-v1 and `m6a_frames/*.sb<N>.frames`), all written by the genuine
//! linuxdoom-1.10 engine only (`tools/reference/m6a_goldens.py`); no Rust output feeds an
//! expectation. The rows are read by column name from each file's `#columns` lines.
#![allow(dead_code, reason = "each test target uses a subset")]

use doom_game::session::{ModeSetting, SessionData, SessionMode};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The repository root joined with `path`.
pub fn root(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// The directory of the goldens: `tests/reference`, or for the local real-map gates (the
/// `real_maps_*` tests, never in CI) the directory `M6A_REFERENCE_DIR` names, which
/// `tools/local/m6a_session_real.py` writes in the same layout from the genuine engine on a
/// scratch copy of the owner's IWAD.
pub fn reference(path: &str) -> PathBuf {
    match std::env::var_os("M6A_REFERENCE_DIR") {
        Some(dir) => PathBuf::from(dir).join(path),
        None => root("tests/reference").join(path),
    }
}

/// Whether the goldens are the local real-map ones (`M6A_REFERENCE_DIR` set).
pub fn real_maps() -> bool {
    std::env::var_os("M6A_REFERENCE_DIR").is_some()
}

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
    /// The start map.
    pub map: u8,
    /// The skill byte.
    pub skill: u8,
    /// How the genuine run ended: `marker` or `error:<message>` (m6a_results.json).
    pub end: String,
    /// The genuine session tics (G rows).
    pub session_tics: usize,
}

/// The 18 level-chain items of step 2 (start maps 1, 2, 3 and 7).
pub const CHAIN_ITEMS: [&str; 18] = [
    "CHAIN", "INTSTA", "INTSTB", "INTIDLE", "INTHELD", "INTFAST", "REBHELD", "REBLATE", "SECRET",
    "REBSECRT", "PROBE", "USEAT130", "USEAT131", "USEAT132", "RBXDEAD", "NOINTER", "NOE1M2",
    "NOE1M2AT",
];

fn json(path: &str) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(reference(path)).expect(path)).expect("JSON")
}

/// Every manifest item, in manifest order.
pub fn items() -> Vec<Item> {
    let m = json("m6a_manifest.json");
    assert_eq!(m["schema"], "DOOM-M6A-MANIFEST-v1");
    let results = json("m6a_results.json");
    let sets = &m["wadsets"];
    m["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| {
            let name = i["name"].as_str().expect("name").to_owned();
            let r = &results["items"][&name];
            Item {
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
                end: r["end"].as_str().expect("end").into(),
                session_tics: r["session_tics"].as_u64().expect("tics") as usize,
                name,
            }
        })
        .collect()
}

/// The items of a local real-map gate: every manifest item, or only those `M6A_REAL_ONLY` names
/// (comma-separated), so that a gate can be split into runs that each fit a time limit.
pub fn real_items() -> Vec<Item> {
    let all = items();
    match std::env::var("M6A_REAL_ONLY") {
        Ok(only) => {
            let want: Vec<&str> = only.split(',').collect();
            let out: Vec<Item> = all
                .into_iter()
                .filter(|i| want.contains(&i.name.as_str()))
                .collect();
            assert_eq!(out.len(), want.len(), "M6A_REAL_ONLY names an unknown item");
            out
        }
        Err(_) => all,
    }
}

/// The names of the items whose state golden (`m6a_state.txt`, L rows) has a carried start: a
/// level loaded by `G_DoWorldDone` with the player of the level before (the planted faults of
/// the carried start can show only there).
pub fn carried_items() -> Vec<String> {
    let text = std::fs::read_to_string(reference("m6a_state.txt")).expect("state golden");
    let mut out: Vec<String> = text
        .lines()
        .filter(|l| l.starts_with("L,"))
        .map(|l| l.split(',').collect::<Vec<_>>())
        .filter(|f| f[5] == "carried")
        .map(|f| f[1].to_owned())
        .collect();
    out.dedup();
    out
}

/// The named item.
pub fn item(name: &str) -> Item {
    items()
        .into_iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("item {name}"))
}

/// The session data of an item's lump list (the WAD set, then the demo as a single lump, as the
/// genuine runner's `-playdemo` adds it), in `mode` (None: the item's own mode, forced).
pub fn data(item: &Item, mode: Option<SessionMode>) -> Arc<SessionData> {
    let files: Vec<Vec<u8>> = item
        .wads
        .iter()
        .map(|w| std::fs::read(root(w)).expect("WAD"))
        .collect();
    let refs: Vec<&[u8]> = files.iter().map(Vec::as_slice).collect();
    let lmp = std::fs::read(root(&item.lmp)).expect("demo");
    let mode = mode.unwrap_or(match item.mode.as_str() {
        "registered" => SessionMode::Registered,
        "retail" => SessionMode::Retail,
        m => panic!("mode {m}"),
    });
    Arc::new(
        SessionData::load_with(&refs, &[(&item.lmp, &lmp)], ModeSetting::Force(mode))
            .expect("session data"),
    )
}

/// The rows of one genuine stream, by kind, as name → value maps, indexed by tic where the row
/// has a `tic` column.
#[derive(Default)]
pub struct Rows {
    by_kind: BTreeMap<String, Vec<BTreeMap<String, String>>>,
    by_tic: BTreeMap<(String, i64), usize>,
}

impl Rows {
    /// Parses a stream (the first `#columns` line of each kind wins; the frames' M6a `G` row
    /// replaces nothing: a session `G` row and a frames `G` row are in different files).
    pub fn parse(text: &str) -> Self {
        let mut columns: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut rows = Self::default();
        for line in text.lines() {
            let f: Vec<&str> = line.split(',').collect();
            if f[0] == "#columns" {
                columns
                    .entry(f[1].to_owned())
                    .or_insert_with(|| f[2..].iter().map(|s| (*s).to_owned()).collect());
                continue;
            }
            if f[0].starts_with('#') || f[0] == "Z" {
                // Z: the totals row, name-value pairs of its own width.
                continue;
            }
            let Some(names) = columns.get(f[0]) else {
                continue;
            };
            assert_eq!(names.len() + 1, f.len(), "row width: {line}");
            let row: BTreeMap<String, String> = names
                .iter()
                .zip(&f[1..])
                .map(|(n, v)| (n.clone(), (*v).to_owned()))
                .collect();
            let list = rows.by_kind.entry(f[0].to_owned()).or_default();
            if let Some(t) = row.get("tic").and_then(|t| t.parse::<i64>().ok()) {
                rows.by_tic.insert((f[0].to_owned(), t), list.len());
            }
            list.push(row);
        }
        rows
    }

    /// Reads `tests/reference/<path>`.
    pub fn read(path: &str) -> Self {
        Self::parse(&std::fs::read_to_string(reference(path)).expect(path))
    }

    /// Every row of a kind.
    pub fn of(&self, kind: &str) -> &[BTreeMap<String, String>] {
        self.by_kind.get(kind).map_or(&[], Vec::as_slice)
    }

    /// The row of a kind at a tic.
    pub fn at(&self, kind: &str, tic: i64) -> Option<&BTreeMap<String, String>> {
        let i = *self.by_tic.get(&(kind.to_owned(), tic))?;
        self.by_kind.get(kind).map(|l| &l[i])
    }
}

/// A row's integer column.
pub fn int(row: &BTreeMap<String, String>, name: &str) -> i64 {
    row.get(name)
        .unwrap_or_else(|| panic!("column {name}"))
        .parse()
        .unwrap_or_else(|_| panic!("column {name}: {:?}", row.get(name)))
}

/// A row's list column (space-separated integers).
pub fn list(row: &BTreeMap<String, String>, name: &str) -> Vec<i64> {
    row.get(name)
        .unwrap_or_else(|| panic!("column {name}"))
        .split(' ')
        .map(|v| v.parse().expect("number"))
        .collect()
}

/// One frame of an M6a frames golden (DOOM-M5C-FRAMES-v1 with the M6a `G` rows): a level frame
/// (the M5c F, M, P and Q rows) or a frame outside a level (the M6a G row).
#[derive(Clone, Debug, Default)]
pub struct GoldenFrame {
    /// `gamestate` (0 for a level frame).
    pub gamestate: i64,
    /// The whole-frame digest.
    pub sha256: String,
    /// A level frame's F row: `fuzzpos`, `mapped`, `visplanes`, `drawsegs`, `vissprites`,
    /// `openings`, `solidsegs_peak`, `drawseg_drops`, `vissprite_drop` (flagged columns and the
    /// whole-frame exclusion are 0 in every M6a golden, asserted).
    pub f: Vec<i64>,
    /// The lines newly mapped (M rows).
    pub mapped: Vec<u16>,
    /// P row: palette, palette set, face, `message_on`, `rndindex` (level frames); for a G row
    /// palette, palette set and `rndindex` only.
    pub status: Vec<i64>,
    /// Q row: the face-zeroed digest, cumulative sound and wipe draws, the face rectangle.
    pub noface: String,
    /// See [`GoldenFrame::noface`].
    pub q: [i64; 6],
    /// Real maps only (the synthetic goldens have none, asserted): the renderer's tracker
    /// flagged columns of undefined reads (U rows), and the digest with them zeroed.
    pub flagged: Vec<u16>,
    /// See [`GoldenFrame::flagged`]: the masked digest (`-` when no column is flagged).
    pub masked: String,
    /// Real maps only: a frame the genuine runner excludes as a whole (F `whole` = 1).
    pub whole_excluded: bool,
}

/// Reads `tests/reference/m6a_frames/<name>.sb<sb>.frames`: the frames by tic, and the E row
/// (the genuine engine's stop) if any.
pub fn read_frames(name: &str, sb: u8) -> (BTreeMap<i64, GoldenFrame>, Option<String>) {
    let path = format!("m6a_frames/{name}.sb{sb}.frames");
    let text = std::fs::read_to_string(reference(&path)).expect("frames");
    let real = real_maps();
    let mut out: BTreeMap<i64, GoldenFrame> = BTreeMap::new();
    let mut error = None;
    // Real maps: each F row's flagged-column count, checked against its U rows below.
    let mut columns: BTreeMap<i64, i64> = BTreeMap::new();
    for l in text.lines() {
        let c: Vec<&str> = l.split(',').collect();
        if c[0].starts_with('#') {
            continue;
        }
        let n = |i: usize| {
            c[i].parse::<i64>()
                .unwrap_or_else(|_| panic!("{name}: {l}"))
        };
        match c[0] {
            "F" => {
                let g = GoldenFrame {
                    sha256: c[2].into(),
                    masked: c[3].into(),
                    whole_excluded: c[14] == "1",
                    f: [4, 5, 7, 8, 9, 10, 11, 12]
                        .into_iter()
                        .map(n)
                        .chain([i64::from(c[13] == "1")])
                        .collect(),
                    ..GoldenFrame::default()
                };
                if real {
                    columns.insert(n(1), n(6));
                } else {
                    assert_eq!(
                        c[6], "0",
                        "{name}: no flagged column in the M6a goldens: {l}"
                    );
                    assert_eq!(c[14], "0", "{name}: no whole-frame exclusion: {l}");
                }
                assert!(out.insert(n(1), g).is_none(), "{name}: {l}");
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
                assert!(out.insert(n(1), g).is_none(), "{name}: {l}");
            }
            "E" => error = Some(c[2..].join(",")),
            "U" if real => out
                .get_mut(&n(1))
                .expect("U after F")
                .flagged
                .push(c[2].parse().expect("column")),
            "U" | "O" | "D" | "W" => panic!("{name}: unexpected row {l}"),
            _ => {}
        }
    }
    for (tic, count) in columns {
        assert_eq!(
            out[&tic].flagged.len() as i64,
            count,
            "{name}: U rows at {tic}"
        );
    }
    (out, error)
}

/// The genuine sound stream of an item (`tests/reference/m6a_sound/<name>.sound`): the S, T, K,
/// L, C and R rows by tic, the loads (LV rows: tic → map) and the E row (tic, message).
#[derive(Default)]
pub struct GenuineSound {
    /// Rows by tic.
    pub by_tic: BTreeMap<i64, Vec<String>>,
    /// Loads after the first.
    pub loads: BTreeMap<i64, u8>,
    /// The stop, if any.
    pub error: Option<(i64, String)>,
}

/// Reads an item's genuine sound stream.
pub fn read_sound(name: &str) -> GenuineSound {
    let path = format!("m6a_sound/{name}.sound");
    let mut g = GenuineSound::default();
    for l in std::fs::read_to_string(reference(&path))
        .expect("sound")
        .lines()
    {
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

/// The items of the owner's rulings of 2026-11-22 (`tools/reference/m6a_wad.py`, goldens from the
/// genuine engine like every other item): `TURBO` (ruling 1: the turbo message tests the session's
/// `gametic`, g_game.c:671-679, across a world-done load, a reborn reload and on load tics) and
/// `INTCMD` (ruling 2: intermission commands outside wire 24's level domain, which the C ignores
/// but for bits 0 and 1 of the buttons, wi_stuff.c:1470-1498); and `ATTACK` (the stale attacker: a
/// zombieman hurts the player, who exits, and the next level loads with `players[0].attacker`
/// pointing into the freed level, which the C never reads before it is overwritten or wiped; the
/// independent encoder projects it as NULL under a narrow rule, `tools/reference/m6a_state.py`).
pub const RULING_ITEMS: [&str; 3] = ["TURBO", "INTCMD", "ATTACK"];
