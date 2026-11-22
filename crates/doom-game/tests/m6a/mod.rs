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
    serde_json::from_str(&std::fs::read_to_string(root(path)).expect(path)).expect("JSON")
}

/// Every manifest item, in manifest order.
pub fn items() -> Vec<Item> {
    let m = json("tests/reference/m6a_manifest.json");
    assert_eq!(m["schema"], "DOOM-M6A-MANIFEST-v1");
    let results = json("tests/reference/m6a_results.json");
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
            if f[0].starts_with('#') {
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
        Self::parse(&std::fs::read_to_string(root(&format!("tests/reference/{path}"))).expect(path))
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
