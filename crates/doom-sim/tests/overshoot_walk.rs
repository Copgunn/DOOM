//! The block-walk overshoot (`P_PathTraverse`, p_maputl.c:841-876) against the
//! genuine engine, under wires 26, 27 and 28 (certificate audit 1, item 4).
//!
//! The goldens, `tests/reference/overshoot_state.txt`, come from
//! `tools/reference/overshoot_goldens.py`: the genuine linuxdoom-1.10 runner (the
//! strict `-fwrapv` build of wire 27's tooling) and the independent DSIM43 encoder;
//! no Rust output feeds them. Two items of `tests/reference/overshoot_wads/`:
//!
//! - **SLIDE**: at tic 38 a slide trace 11.3 units long, 3 raw units under a
//!   BLOCKMAP row line with a y step of 4 raw units, has `ystep` 0 and walks 64 cells
//!   east instead of stepping into its end cell; 3,557 units east it meets the dip
//!   line (linedef 10), whose infinite line separates the short trace's ends, and the
//!   genuine engine slides along it.
//! - **WRAP**: at tic 52 the same kind of walk meets two 2,184-unit notch edges
//!   about 5,000 units east; their `P_InterceptVector` narrows a `FixedMul` product
//!   (`wrap.den_exact.num_wrong.div2`, six calls), so wires 26, 27 and 28 refuse the
//!   tic with `MapScale(Arithmetic)`; the genuine engine drops the wrapped
//!   (negative) fractions and goes on.
//!
//! The capture digest is wire 27's (`tests/wire27_intercept.rs`): a DSIM42 or DSIM44
//! capture is renamed to DSIM43 (version, `profile`, context) first; the encoders
//! assert that nothing else differs (`level_snapshot_wire27.compare42`,
//! `level_snapshot_wire28.compare43`). The `F` rows are the genuine runner with a
//! planted fault (the walk never enters a cell past its end cell): Rust must differ
//! from them, so a reach-bounded walk would fail this test.
use doom_sim::{demo::parse_demo_episode1_v2, *};
use doom_test_evidence::sha256;
use std::path::{Path, PathBuf};

fn reference(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(path)
}

/// One item of the golden: map, tics, the expected ends under wires 26 and 27
/// (`None` complete, `Some(tic)` refused), the overshoot tic, and the strict and
/// planted-fault capture digests.
#[derive(Default)]
struct Item {
    name: String,
    map: u8,
    tics: usize,
    wire26: Option<usize>,
    wire27: Option<usize>,
    overshoot: usize,
    strict: Vec<String>,
    fault: Vec<String>,
}

fn golden() -> Vec<Item> {
    let text = std::fs::read_to_string(reference("overshoot_state.txt")).expect("golden");
    let mut items: Vec<Item> = Vec::new();
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    assert_eq!(lines.next(), Some("#schema,DOOM-OVERSHOOT-STATE-v1"));
    let end = |s: &str| match s.split(':').collect::<Vec<_>>()[..] {
        ["complete"] => None,
        ["refused", tic, ..] => Some(tic.parse().expect(s)),
        _ => panic!("end {s}"),
    };
    for line in lines.filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        let item = items.last_mut();
        match f[0] {
            "I" => items.push(Item {
                name: f[1].to_owned(),
                map: f[2].parse().expect(line),
                tics: f[3].parse().expect(line),
                wire26: end(f[5]),
                wire27: end(f[6]),
                ..Item::default()
            }),
            "O" => item.expect("I first").overshoot = f[2].parse().expect(line),
            "C" | "F" => {
                let item = item.expect("I first");
                let list = if f[0] == "C" {
                    &mut item.strict
                } else {
                    &mut item.fault
                };
                assert_eq!(
                    (item.name.as_str(), list.len()),
                    (f[1], f[2].parse().expect(line))
                );
                list.push(f[3].to_owned());
            }
            "L" | "B" | "W" | "U" => {}
            _ => panic!("row {line}"),
        }
    }
    for item in &items {
        assert_eq!(item.strict.len(), item.tics + 1, "{}", item.name);
        assert_eq!(item.fault.len(), item.tics + 1, "{}", item.name);
    }
    items
}

fn framed(out: &mut Vec<u8>, path: &str, value: &[u8]) {
    out.extend((path.len() as u32).to_le_bytes());
    out.extend(path.as_bytes());
    out.extend((value.len() as u64).to_le_bytes());
    out.extend(value);
}

/// The DSIM43 state digest of a wire 26, 27 or 28 capture (module documentation).
fn digest(s: &Snapshot) -> String {
    let rename = s.version() != 43;
    assert!(
        matches!(s.version(), 42..=44),
        "a wire 26, 27 or 28 capture"
    );
    let value = |f: &StateField| -> Vec<u8> {
        match (rename, f.path()) {
            (true, "profile") => 27_u32.to_le_bytes().to_vec(),
            (true, "context.schema") => PLAYER_EPISODE1_V5_CONTEXT_SCHEMA.to_vec(),
            _ => f.value().to_vec(),
        }
    };
    let kept = |f: &&StateField| !f.path().starts_with("map.") && !f.path().starts_with("events.");
    let mut body = b"DSIM".to_vec();
    body.extend(43_u32.to_le_bytes());
    body.extend((s.fields().filter(kept).count() as u64).to_le_bytes());
    for f in s.fields().filter(kept) {
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
    let lmp = std::fs::read(reference(&format!("overshoot_scripts/{name}.lmp"))).expect("demo");
    parse_demo_episode1_v2(&lmp, lmp[3])
        .expect("demo parse")
        .1
        .commands
}

fn level(item: &Item, wire: u8) -> Simulation {
    let bytes = std::fs::read(reference("overshoot_wads/overshoot.wad")).expect("wad");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("wad parse");
    let name = format!("E1M{}", item.map);
    let map = doom_map::Map::from_wad(&wad, &name, doom_map::MapLimits::default()).expect("map");
    let build = match wire {
        26 => Simulation::original_player_episode1_v4_level,
        27 => Simulation::original_player_episode1_v5_level,
        28 => Simulation::original_player_episode1_v6_level,
        _ => unreachable!("wires 26 to 28"),
    };
    let start = LevelStart::NewGame;
    build(
        map,
        item.map,
        Skill::Medium,
        MapContextOptions::default(),
        start,
    )
    .expect("the map loads under wires 26 to 28")
}

/// Plays `commands` and returns the first capture whose digest differs from
/// `expected` and the refusal (session tic, error) if one came first; the state
/// after a refusal must be the capture before it.
fn play(
    s: &mut Simulation,
    commands: &[TicCommand],
    expected: &[String],
) -> (Option<usize>, Option<(usize, SimError)>) {
    if digest(&s.snapshot()) != expected[0] {
        return (Some(0), None);
    }
    let none = TicCommand::default();
    for (tic, c) in commands.iter().enumerate() {
        if let Err(e) = s.advance([*c, none, none, none]) {
            let kept = digest(&s.snapshot()) == expected[tic];
            return (if kept { None } else { Some(tic) }, Some((tic, e)));
        }
        if digest(&s.snapshot()) != expected[tic + 1] {
            return (Some(tic + 1), None);
        }
    }
    (None, None)
}

/// Every item equals the genuine engine after every tic under wires 26, 27 and 28,
/// up to the refusal the witness build predicts (wire 28 takes wire 27's
/// intercept rule): SLIDE runs to its end, WRAP refuses tic 52.
#[test]
fn the_overshoot_items_equal_the_genuine_engine_under_wires_26_to_28() {
    let items = golden();
    assert_eq!(items.len(), 2);
    for item in &items {
        let commands = demo(&item.name);
        assert_eq!(commands.len(), item.tics, "{}", item.name);
        for wire in [26, 27, 28] {
            let (difference, refusal) = play(&mut level(item, wire), &commands, &item.strict);
            assert_eq!(
                difference, None,
                "{} wire {wire}: first differing capture",
                item.name
            );
            let expected = if wire == 26 { item.wire26 } else { item.wire27 };
            let arithmetic: SimError = MapScaleError::Arithmetic.into();
            assert_eq!(
                refusal,
                expected.map(|tic| (tic, arithmetic)),
                "{} wire {wire}",
                item.name
            );
        }
    }
    let wrap = &items[1];
    assert_eq!((wrap.wire26, wrap.wire27), (Some(52), Some(52)));
    assert_eq!(wrap.overshoot, 52);
}

/// The harness can fail: Rust differs from the planted-fault genuine build (the
/// reach-bounded walk) at the capture after SLIDE's overshoot tic, an altered
/// digest is reported at its capture, and SLIDE without its small forward move at
/// tic 38 differs at capture 39. On WRAP the planted fault does not change the
/// genuine run (the wrapped fractions are negative and dropped).
#[test]
fn a_reach_bounded_walk_and_planted_faults_are_detected() {
    let items = golden();
    let slide = &items[0];
    assert_eq!(slide.overshoot, 38);
    let commands = demo("SLIDE");
    for wire in [26, 27, 28] {
        let (difference, _) = play(&mut level(slide, wire), &commands, &slide.fault);
        assert_eq!(difference, Some(slide.overshoot + 1), "wire {wire}");
    }
    for at in [0, slide.overshoot + 1, slide.tics] {
        let mut bent = slide.strict.clone();
        let flipped = if bent[at].starts_with('0') { "1" } else { "0" };
        bent[at].replace_range(..1, flipped);
        assert_eq!(
            play(&mut level(slide, 27), &commands, &bent),
            (Some(at), None)
        );
    }
    let mut quiet = commands.clone();
    assert_ne!(quiet[slide.overshoot].forward, 0);
    quiet[slide.overshoot].forward = 0;
    let expected = (Some(slide.overshoot + 1), None);
    assert_eq!(play(&mut level(slide, 27), &quiet, &slide.strict), expected);
    assert_eq!(items[1].fault, items[1].strict);
}
