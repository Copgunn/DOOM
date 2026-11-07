//! Wire 23 (`PlayerMonstersEpisode1V1`, DSIM39) on the nine M4a mechanism WADs
//! (`tests/reference/m4a_wads/`), tic for tic against the genuine engine's goldens
//! (M4a mutation-testing step).
//!
//! `tools/reference/check_player_episode1.py` performs the same comparison on the
//! release example's captures; this test makes it part of `cargo test`, through
//! the public API only (`Simulation::original_player_episode1_map`, `advance`,
//! `snapshot`, the public `SimError`/`MapScaleError`). Every expected value is
//! read from `tests/reference/m4a_synthetic_goldens.json`, which
//! `m4a_snapshot_goldens.py` projected from the dumps of the genuine
//! linuxdoom-1.10 engine (the M4a driver fork) with the independent DSIM39
//! encoder `level_snapshot.py`: never from Rust output. For each script:
//!
//! - every capture in the compared range, stripped as the comparator strips it
//!   (no `map.*`, no `events.*`; `level_compare.stripped`), has the golden
//!   per-tic SHA-256 prefix, and over a full range the sequence hash and the
//!   final digest also match;
//! - the run ends as the manifest's `rust` expectation states: `match` scripts
//!   run every demo command (an `exit` script may refuse the tic after its exit
//!   capture with `SimError::LevelExited`: `G_ExitLevel` sets `gameaction`, and
//!   the next `G_Ticker` runs `G_DoCompleted`, g_game.c:1164–1170, out of scope);
//!   a `PlayerReborn` script refuses capture `T + 2` after the reborn request of
//!   dump tic `T` (`P_DeathThink` sets `PST_REBORN` on `BT_USE`, p_user.c:226–228,
//!   and the next `G_Ticker` would run `G_DoReborn`, g_game.c:591–597); an H7
//!   witness script refuses capture `T + 1` after the blocked move of dump tic `T`
//!   (`PlatBlocked`: p_plats.c:62–82; `StairBlocked`: p_floor.c:113–119).
//!   Capture `N` is dump tic `N − 1` (DSIM tic numbering).
use doom_sim::{demo::parse_demo_episode1, *};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}

/// A minimal JSON value: enough for the committed goldens file.
#[derive(Debug)]
enum Json {
    Null,
    Bool,
    Number(i64),
    Text(String),
    List(Vec<Json>),
    Object(BTreeMap<String, Json>),
}
impl Json {
    fn get(&self, key: &str) -> &Json {
        match self {
            Json::Object(m) => m.get(key).unwrap_or_else(|| panic!("missing key {key}")),
            other => panic!("not an object: {other:?}"),
        }
    }
    fn text(&self) -> &str {
        match self {
            Json::Text(s) => s,
            other => panic!("not a string: {other:?}"),
        }
    }
    fn number(&self) -> usize {
        match self {
            Json::Number(n) => usize::try_from(*n).expect("non-negative"),
            other => panic!("not a number: {other:?}"),
        }
    }
    fn list(&self) -> &[Json] {
        match self {
            Json::List(v) => v,
            other => panic!("not a list: {other:?}"),
        }
    }
}
struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Parser<'_> {
    fn skip(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn eat(&mut self, b: u8) {
        self.skip();
        assert_eq!(self.bytes[self.at], b, "JSON at byte {}", self.at);
        self.at += 1;
    }
    fn peek(&mut self) -> u8 {
        self.skip();
        self.bytes[self.at]
    }
    fn string(&mut self) -> String {
        self.eat(b'"');
        let mut out = String::new();
        loop {
            let b = self.bytes[self.at];
            self.at += 1;
            match b {
                b'"' => return out,
                b'\\' => {
                    let e = self.bytes[self.at];
                    self.at += 1;
                    match e {
                        b'"' | b'\\' | b'/' => out.push(char::from(e)),
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hex = std::str::from_utf8(&self.bytes[self.at..self.at + 4])
                                .expect("hex");
                            self.at += 4;
                            out.push(
                                char::from_u32(u32::from_str_radix(hex, 16).expect("hex"))
                                    .unwrap_or('?'),
                            );
                        }
                        other => panic!("escape {other}"),
                    }
                }
                _ => {
                    // Multi-byte UTF-8 passes through byte by byte.
                    let start = self.at - 1;
                    let len = match b {
                        0..=0x7f => 1,
                        0xc0..=0xdf => 2,
                        0xe0..=0xef => 3,
                        _ => 4,
                    };
                    self.at = start + len;
                    out.push_str(std::str::from_utf8(&self.bytes[start..self.at]).expect("utf8"));
                }
            }
        }
    }
    fn value(&mut self) -> Json {
        match self.peek() {
            b'{' => {
                self.eat(b'{');
                let mut m = BTreeMap::new();
                if self.peek() == b'}' {
                    self.eat(b'}');
                    return Json::Object(m);
                }
                loop {
                    let k = self.string();
                    self.eat(b':');
                    m.insert(k, self.value());
                    if self.peek() == b',' {
                        self.eat(b',');
                    } else {
                        self.eat(b'}');
                        return Json::Object(m);
                    }
                }
            }
            b'[' => {
                self.eat(b'[');
                let mut v = Vec::new();
                if self.peek() == b']' {
                    self.eat(b']');
                    return Json::List(v);
                }
                loop {
                    v.push(self.value());
                    if self.peek() == b',' {
                        self.eat(b',');
                    } else {
                        self.eat(b']');
                        return Json::List(v);
                    }
                }
            }
            b'"' => Json::Text(self.string()),
            b'n' => {
                self.at += 4;
                Json::Null
            }
            b't' => {
                self.at += 4;
                Json::Bool
            }
            b'f' => {
                self.at += 5;
                Json::Bool
            }
            _ => {
                let start = self.at;
                while self
                    .bytes
                    .get(self.at)
                    .is_some_and(|b| b.is_ascii_digit() || *b == b'-')
                {
                    self.at += 1;
                }
                Json::Number(
                    std::str::from_utf8(&self.bytes[start..self.at])
                        .expect("number")
                        .parse()
                        .expect("integer"),
                )
            }
        }
    }
}
fn goldens() -> Json {
    let bytes = std::fs::read(reference("m4a_synthetic_goldens.json")).expect("goldens");
    Parser {
        bytes: &bytes,
        at: 0,
    }
    .value()
}

/// The canonical bytes of `snap` without its `map.*` and `events.*` fields
/// (`level_compare.stripped`).
fn stripped(snap: &Snapshot) -> Vec<u8> {
    let fields: Vec<_> = snap
        .fields()
        .filter(|f| !f.path().starts_with("map.") && !f.path().starts_with("events."))
        .collect();
    let mut out = b"DSIM".to_vec();
    out.extend(snap.version().to_le_bytes());
    out.extend((fields.len() as u64).to_le_bytes());
    for f in fields {
        out.extend((f.path().len() as u32).to_le_bytes());
        out.extend(f.path().as_bytes());
        out.extend((f.value().len() as u64).to_le_bytes());
        out.extend(f.value());
    }
    out
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex"))
        .collect()
}

/// The error a refused tic must carry for a manifest refusal name.
fn refusal(name: &str) -> SimError {
    match name {
        "PlayerReborn" => SimError::PlayerReborn,
        "blocked raiseToNearestAndChange plat (H7)" => MapScaleError::PlatBlocked.into(),
        "blocked stair step (H7)" => MapScaleError::StairBlocked.into(),
        other => panic!("unknown manifest refusal {other}"),
    }
}

fn run(script: &str) {
    run_with(script, |_| {});
}

/// [`run`] with the demo's commands changed by `tweak` first (the self-test).
fn run_with(script: &str, tweak: impl FnOnce(&mut Vec<TicCommand>)) {
    let all = goldens();
    assert_eq!(all.get("dsim").get("version").number(), 39);
    let g = all.get("scripts").get(script);
    let wad_file = format!("m4a_wads/{}.wad", g.get("wad").text().to_ascii_lowercase());
    let wad_bytes = std::fs::read(reference(&wad_file)).expect("WAD");
    assert_eq!(
        doom_test_evidence::sha256(&wad_bytes),
        g.get("wad_sha256").text(),
        "{script}: the WAD differs from the goldens'"
    );
    let lmp = std::fs::read(reference(&format!("m4a_scripts/{script}.lmp"))).expect("demo");
    assert_eq!(
        doom_test_evidence::sha256(&lmp),
        g.get("lmp_sha256").text(),
        "{script}: the demo differs from the goldens'"
    );
    assert_eq!(g.get("slot").text(), "E1M1");
    assert!(matches!(g.get("projection_stop"), Json::Null), "{script}");
    let digests: Vec<&str> = g.get("tic_digests").list().iter().map(Json::text).collect();
    let snapshots = g.get("snapshots").number();
    assert_eq!(digests.len(), snapshots);
    assert_eq!(g.get("tics_projected").number(), snapshots);
    // (last capture compared, the refused capture and its error).
    let (last, refused): (usize, Option<(usize, SimError)>) = match g.get("rust") {
        Json::Text(t) if t == "match" => (snapshots - 1, None),
        rust @ Json::Object(_) => {
            let name = rust.get("refuse").text();
            let t = rust.get("tic").number();
            if name == "PlayerReborn" {
                (t + 1, Some((t + 2, refusal(name))))
            } else {
                (t, Some((t + 1, refusal(name))))
            }
        }
        other => panic!("{script}: rust expectation {other:?}"),
    };
    let end = g.get("end");
    let exit_tic = (end.get("kind").text() == "exit").then(|| end.get("tic").number());

    let wad = doom_wad::Wad::parse(&wad_bytes, doom_wad::Limits::default()).expect("WAD");
    let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).expect("map");
    let (_, mut demo) = parse_demo_episode1(&lmp, 1).expect("E1M1 demo");
    tweak(&mut demo.commands);
    let mut s = Simulation::original_player_episode1_map(map, MapContextOptions::default())
        .expect("admission");
    s.spawn_map_things().expect("THINGs");
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .expect("weapons");
    s.spawn_level_specials().expect("specials");

    let mut sequence = Vec::new();
    let mut check = |tic: usize, s: &Simulation| {
        if tic > last {
            return;
        }
        let snap = s.snapshot();
        assert_eq!(snap.version(), 39);
        let digest = doom_test_evidence::sha256(&stripped(&snap));
        assert_eq!(&digest[..16], digests[tic], "{script}: capture {tic}");
        sequence.extend(hex_to_bytes(&digest));
        if tic == snapshots - 1 {
            assert_eq!(digest, g.get("final_sha256").text(), "{script}: final");
        }
    };
    check(0, &s);
    let mut ended = None;
    for (i, command) in demo.commands.iter().enumerate() {
        let tic = i + 1;
        let commands = [
            *command,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ];
        match s.advance(commands) {
            Ok(()) => check(tic, &s),
            Err(e) => {
                ended = Some((tic, e));
                break;
            }
        }
    }
    match (refused, ended) {
        (Some(want), Some(got)) => assert_eq!(got, want, "{script}: refusal"),
        (None, Some((tic, SimError::LevelExited))) if exit_tic.map(|t| t + 1) == Some(tic) => {}
        (None, None) => {}
        (want, got) => panic!("{script}: ending {got:?}, expected {want:?}"),
    }
    let compared = sequence.len() / 32;
    assert_eq!(compared, last + 1, "{script}: captures compared");
    if last == snapshots - 1 {
        assert_eq!(
            doom_test_evidence::sha256(&sequence),
            g.get("sequence_sha256").text(),
            "{script}: sequence"
        );
    }
}

/// The harness can fail: the same script with one movement command removed (the
/// first nonzero `forward`, so the player stops a tic early) no longer
/// matches the goldens.
#[test]
fn harness_detects_a_changed_command() {
    let changed = std::panic::catch_unwind(|| {
        run_with("DOORS1", |commands| {
            let first = commands
                .iter_mut()
                .find(|c| c.forward != 0)
                .expect("a moving command");
            first.forward = 0;
        })
    });
    assert!(changed.is_err());
}

macro_rules! scripts {
    ($($name:ident => $script:literal),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                run($script);
            }
        )*
        #[test]
        fn every_golden_script_has_a_test() {
            let mut listed: Vec<&str> = vec![$($script),*];
            listed.sort_unstable();
            let all = goldens();
            let mut golden: Vec<&str> = match all.get("scripts") {
                Json::Object(m) => m.keys().map(String::as_str).collect(),
                _ => panic!("scripts"),
            };
            golden.sort_unstable();
            assert_eq!(listed, golden);
        }
    };
}
scripts! {
    doors1 => "DOORS1", doors2 => "DOORS2", doors3 => "DOORS3",
    floors1 => "FLOORS1", floors2 => "FLOORS2", floors3 => "FLOORS3",
    plats1 => "PLATS1", plats2 => "PLATS2",
    stairs1 => "STAIRS1", stairs2 => "STAIRS2", donut1 => "DONUT1",
    exit1 => "EXIT1", exit2 => "EXIT2", sect11 => "SECT11", exdie => "EXDIE",
    saw1 => "SAW1", rocket1 => "ROCKET1", rockdie => "ROCKDIE",
    items1 => "ITEMS1", timers => "TIMERS", suit1 => "SUIT1", suitdie => "SUITDIE",
    invis1 => "INVIS1", turbo1 => "TURBO1", target1 => "TARGET1",
    spect1 => "SPECT1", spect2 => "SPECT2", spectd => "SPECTD",
    scale1 => "SCALE1", scale2 => "SCALE2",
}
