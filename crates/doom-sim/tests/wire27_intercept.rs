//! Wire 27 (`PlayerMonstersEpisode1V5`, DSIM43) against the genuine engine on the
//! synthetic wrap items, eight map slots of one room layout (owner's rulings of
//! 2026-11-24, R6-R8).
//!
//! The goldens, `tests/reference/wire27_state.txt`, come from
//! `tools/reference/wire27_goldens.py`: the genuine linuxdoom-1.10 runner (strict
//! `-fwrapv` build of the genuine text; identical dumps in the overflow, address and
//! witness builds) and the independent DSIM43 encoder
//! `tools/reference/level_snapshot_wire27.py`; no Rust output feeds them. Each item
//! is one new-game level of `tests/reference/wire27_wads/wire27.wad` played from its
//! demo (`tests/reference/wire27_scripts/`), in which `P_InterceptVector`
//! (p_maputl.c:231-251) wraps, as the genuine witness build recorded per call.
//!
//! **Tic convention.** A session tic is the 0-based index of a tic (one `advance`,
//! one genuine `G_Ticker`), equal to the number of tics completed before it; capture
//! `j` is the state after `j` tics; a refusal at session tic `N` keeps capture `N`.
//!
//! - **State digest** (as `tests/reference/m6a_state.txt`): SHA-256 of the DSIM43
//!   capture without `map.*` and `events.*` in the canonical framing, then its
//!   `events.sounds.*` fields in the same per-field framing.
//! - **Wire 27**: every capture equals the genuine one; an item with a call whose
//!   class wire 27 refuses for its caller (the golden's `T` rows) equals it up to
//!   that tic, which wire 27 refuses with `MapScale(Arithmetic)` (the genuine run
//!   went on).
//! - **Wire 26** (R8, the known gap pinned): the same demo refuses at the first tic
//!   with a wrapping call, with `MapScale(Arithmetic)` and the state before it kept;
//!   every earlier capture, renamed to DSIM43 (version, `profile`, context), equals
//!   the genuine one.
//! - **Coverage**: the witness rows (`W`, `T`, per caller and class) give each
//!   wrapping caller at least one genuine wrapping call; the shoot trace against a
//!   thing has none and is not wrapped by wire 27. The items are eight map slots of
//!   one room layout.
//! - **Self-tests**: an altered golden digest and a demo without its wrapping use
//!   press are each detected at their capture.
use doom_sim::{demo::parse_demo_episode1_v2, *};
use doom_test_evidence::sha256;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn reference(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(path)
}

/// One item of the golden.
#[derive(Debug, Default, Clone)]
struct Item {
    map: u8,
    tics: usize,
    wire26_refusal: usize,
    /// The session tic wire 27 refuses, if any (`refused:N:kind`).
    wire27_refusal: Option<usize>,
    captures: Vec<String>,
}

/// The golden: items in file order, the per-item witness rows and the totals
/// with wire 27's decision (`admit` or `refuse:kind`) per caller and class.
struct Golden {
    items: Vec<(String, Item)>,
    witness: BTreeMap<(String, String, String), u64>,
    totals: BTreeMap<(String, String), (u64, String)>,
}

fn golden() -> Golden {
    let text = std::fs::read_to_string(reference("wire27_state.txt")).expect("golden");
    let mut g = Golden {
        items: Vec::new(),
        witness: BTreeMap::new(),
        totals: BTreeMap::new(),
    };
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    assert_eq!(lines.next(), Some("#schema,DOOM-WIRE27-STATE-v1"));
    for line in lines.filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        let int = |i: usize| f[i].parse::<usize>().expect(line);
        match f[0] {
            "I" => {
                let end: Vec<&str> = f[6].split(':').collect();
                let wire27_refusal = match end[..] {
                    ["complete"] => None,
                    ["refused", tic, _kind] => Some(tic.parse().expect(line)),
                    _ => panic!("end {line}"),
                };
                let item = Item {
                    map: f[2].parse().expect(line),
                    tics: int(3),
                    wire26_refusal: int(5),
                    wire27_refusal,
                    captures: Vec::with_capacity(int(4)),
                };
                g.items.push((f[1].to_owned(), item));
            }
            "C" => {
                let (name, item) = g.items.last_mut().expect("an I row first");
                assert_eq!((name.as_str(), item.captures.len()), (f[1], int(2)));
                item.captures.push(f[3].to_owned());
            }
            "W" => {
                let key = (f[1].to_owned(), f[2].to_owned(), f[3].to_owned());
                g.witness.insert(key, f[4].parse().expect(line));
            }
            "T" => {
                let key = (f[1].to_owned(), f[2].to_owned());
                g.totals
                    .insert(key, (f[3].parse().expect(line), f[4].to_owned()));
            }
            _ => panic!("row {line}"),
        }
    }
    for (name, item) in &g.items {
        assert_eq!(item.captures.len(), item.tics + 1, "{name}");
    }
    g
}

fn framed(out: &mut Vec<u8>, path: &str, value: &[u8]) {
    out.extend((path.len() as u32).to_le_bytes());
    out.extend(path.as_bytes());
    out.extend((value.len() as u64).to_le_bytes());
    out.extend(value);
}

/// The state digest of a capture (module documentation). A wire-26 (DSIM42)
/// capture is first renamed to DSIM43: version 43, `profile` 27, the wire-27
/// context; nothing else differs between the two (`wire27_tests`).
fn digest(s: &Snapshot) -> String {
    let rename = s.version() == 42;
    assert!(rename || s.version() == 43, "a wire 26 or 27 capture");
    let value = |f: &StateField| -> Vec<u8> {
        match (rename, f.path()) {
            (true, "profile") => 27_u32.to_le_bytes().to_vec(),
            (true, "context.schema") => PLAYER_EPISODE1_V5_CONTEXT_SCHEMA.to_vec(),
            _ => f.value().to_vec(),
        }
    };
    let kept: Vec<_> = s
        .fields()
        .filter(|f| !f.path().starts_with("map.") && !f.path().starts_with("events."))
        .collect();
    let mut body = b"DSIM".to_vec();
    body.extend(43_u32.to_le_bytes());
    body.extend((kept.len() as u64).to_le_bytes());
    for f in &kept {
        framed(&mut body, f.path(), &value(f));
    }
    for f in s
        .fields()
        .filter(|f| f.path().starts_with("events.sounds."))
    {
        framed(&mut body, f.path(), f.value());
    }
    sha256(&body)
}

fn demo(name: &str) -> Vec<TicCommand> {
    let lmp = std::fs::read(reference(&format!("wire27_scripts/{name}.lmp"))).expect("demo");
    parse_demo_episode1_v2(&lmp, lmp[3])
        .expect("demo parse")
        .1
        .commands
}

fn level(item: &Item, wire27: bool) -> Simulation {
    let bytes = std::fs::read(reference("wire27_wads/wire27.wad")).expect("wad");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("wad parse");
    let map = doom_map::Map::from_wad(
        &wad,
        &format!("E1M{}", item.map),
        doom_map::MapLimits::default(),
    )
    .expect("map");
    let build = if wire27 {
        Simulation::original_player_episode1_v5_level
    } else {
        Simulation::original_player_episode1_v4_level
    };
    build(
        map,
        item.map,
        Skill::Medium,
        MapContextOptions::default(),
        LevelStart::NewGame,
    )
    .expect("the map loads under wires 26 and 27")
}

fn step(s: &mut Simulation, c: TicCommand) -> Result<(), SimError> {
    let none = TicCommand::default();
    s.advance([c, none, none, none])
}

/// Plays `commands` and returns the first capture whose digest differs from
/// `expected` and the refusal (session tic, error) if one came first; captures
/// are compared up to the refusal and the state after it must be the last one.
fn play(
    s: &mut Simulation,
    commands: &[TicCommand],
    expected: &[String],
) -> (Option<usize>, Option<(usize, SimError)>) {
    if digest(&s.snapshot()) != expected[0] {
        return (Some(0), None);
    }
    for (tic, c) in commands.iter().enumerate() {
        if let Err(e) = step(s, *c) {
            let kept = digest(&s.snapshot()) == expected[tic];
            return (if kept { None } else { Some(tic) }, Some((tic, e)));
        }
        if digest(&s.snapshot()) != expected[tic + 1] {
            return (Some(tic + 1), None);
        }
    }
    (None, None)
}

/// Every item under wire 27 equals the genuine engine after every tic, up to the
/// declared saturation refusal where there is one.
#[test]
fn wire_27_equals_the_genuine_engine_on_every_item() {
    let g = golden();
    assert_eq!(g.items.len(), 8);
    for (name, item) in &g.items {
        let commands = demo(name);
        assert_eq!(commands.len(), item.tics, "{name}");
        let mut s = level(item, true);
        let (difference, refusal) = play(&mut s, &commands, &item.captures);
        assert_eq!(difference, None, "{name}: first differing capture");
        match item.wire27_refusal {
            None => assert!(refusal.is_none(), "{name}: {refusal:?}"),
            Some(tic) => assert_eq!(
                refusal,
                Some((tic, MapScaleError::Arithmetic.into())),
                "{name}"
            ),
        }
    }
}

/// Ruling R8: under wire 26 every item refuses at its first wrapping tic, with the
/// state before it kept and equal to the genuine engine's.
#[test]
fn wire_26_refuses_every_item_at_its_first_wrap() {
    for (name, item) in &golden().items {
        let mut s = level(item, false);
        let (difference, refusal) = play(&mut s, &demo(name), &item.captures);
        assert_eq!(difference, None, "{name}: first differing capture");
        assert_eq!(
            refusal,
            Some((item.wire26_refusal, MapScaleError::Arithmetic.into())),
            "{name}"
        );
        assert!(item.wire27_refusal.is_none_or(|t| t >= item.wire26_refusal));
    }
}

/// The witness coverage (R6, R7): each wrapping caller has a genuine wrapping
/// call, the totals are the sums of the items' rows, the only caller without a
/// wrapping call is the shoot trace against a thing, which wire 27 keeps checked
/// (`wire27_tests::every_caller_wraps_under_wire_27_alone`), an item refuses
/// under wire 27 exactly when one of its classes is refused for its caller, and
/// every wrap class wire 27 admits for a caller has a genuine call (the owner's
/// corrections of 2026-11-24; `wire27_tests::wire27_admits_exactly_the_witnessed_classes`
/// ties the table to `wire27_admits`).
#[test]
fn the_witness_covers_every_wrapping_caller() {
    let g = golden();
    let callers = ["aim", "shoot_line", "shoot_thing", "slide", "use"];
    assert_eq!(g.totals.len(), callers.len() * 28);
    for caller in callers {
        let wraps = (g.totals.iter())
            .any(|((c, k), (n, _))| c == caller && k.starts_with("wrap.") && *n > 0);
        assert_eq!(wraps, caller != "shoot_thing", "{caller}");
    }
    for ((caller, class), (n, _)) in &g.totals {
        let sum: u64 = (g.witness.iter())
            .filter(|((_, c, k), _)| c == caller && k == class)
            .map(|(_, v)| v)
            .sum();
        assert_eq!(sum, *n, "{caller} {class}");
    }
    for ((caller, class), (n, wire27)) in &g.totals {
        let admitted_wrap = class.starts_with("wrap.") && wire27 == "admit";
        assert!(
            !admitted_wrap || *n > 0,
            "{caller} {class}: admitted without an item"
        );
    }
    for (name, item) in &g.items {
        let refused = (g.witness.keys())
            .filter(|(i, _, _)| i == name)
            .any(|(_, c, k)| g.totals[&(c.clone(), k.clone())].1 != "admit");
        assert_eq!(refused, item.wire27_refusal.is_some(), "{name}");
    }
}

/// The comparison can fail: one golden digest altered is reported at its capture,
/// and USE without its use press (the one wrapping call of the item) differs from
/// the genuine run at the capture after that tic, under wire 27, and no longer
/// refuses under wire 26.
#[test]
fn planted_faults_are_detected_at_their_capture() {
    let g = golden();
    let (_, item) = g.items.iter().find(|(n, _)| n == "USE").expect("USE");
    let commands = demo("USE");
    for at in [0, item.wire26_refusal, item.tics] {
        let mut bent = item.captures.clone();
        let flipped = if bent[at].starts_with('0') { "1" } else { "0" };
        bent[at].replace_range(..1, flipped);
        let (difference, _) = play(&mut level(item, true), &commands, &bent);
        assert_eq!(difference, Some(at));
    }
    let press = item.wire26_refusal;
    assert_ne!(commands[press].buttons, 0);
    let mut quiet = commands.clone();
    quiet[press].buttons = 0;
    let (difference, refusal) = play(&mut level(item, true), &quiet, &item.captures);
    assert_eq!((difference, refusal), (Some(press + 1), None));
    let (difference, refusal) = play(&mut level(item, false), &quiet, &item.captures);
    assert_eq!((difference, refusal), (Some(press + 1), None));
}
