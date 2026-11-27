//! The block-walk overshoot (p_maputl.c:841-876) against the genuine engine under
//! wires 26, 27 and 28 (certificate audit 1, item 4). Goldens:
//! `tests/reference/overshoot_state.txt` from `tools/reference/overshoot_goldens.py`
//! (the genuine strict runner and the independent DSIM43 encoder, never Rust output).
//! SLIDE: at tic 38 a slide trace walks 64 cells past its end cell and the genuine
//! engine slides along a line 3,557 units east. WRAP: at tic 52 such a walk meets
//! edges whose `P_InterceptVector` narrows a `FixedMul` product; wires 26 to 28 refuse
//! the tic (`MapScale(Arithmetic)`) and the genuine engine drops the fractions. The
//! digest is wire 27's (`tests/wire27_intercept.rs`; DSIM42 and DSIM44 captures are
//! renamed to DSIM43, which the encoders show is the only difference). `F` rows: the
//! genuine runner with a planted reach-bounded walk, which Rust must not equal.
use doom_sim::{demo::parse_demo_episode1_v2, *};
use doom_test_evidence::sha256;
use std::path::{Path, PathBuf};

fn reference(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(path)
}

/// An item: the ends under wires 26 and 27 (`Some(tic)` refused) and digests.
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
            "C" => item.expect("I first").strict.push(f[3].to_owned()),
            "F" => item.expect("I first").fault.push(f[3].to_owned()),
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
    let build = [
        Simulation::original_player_episode1_v4_level,
        Simulation::original_player_episode1_v5_level,
        Simulation::original_player_episode1_v6_level,
    ][usize::from(wire - 26)];
    let (skill, options) = (Skill::Medium, MapContextOptions::default());
    build(map, item.map, skill, options, LevelStart::NewGame).expect("wires 26 to 28 load it")
}

/// The first capture differing from `expected` and the refusal (tic, error), if
/// any; the state after a refusal must be the capture before it.
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

/// Every capture equals the genuine one under wires 26 to 28, up to the refusal the
/// witness build predicts (wire 28 has wire 27's intercept rule).
#[test]
fn the_overshoot_items_equal_the_genuine_engine_under_wires_26_to_28() {
    let items = golden();
    assert_eq!(items.len(), 2);
    for item in &items {
        let commands = demo(&item.name);
        assert_eq!(commands.len(), item.tics, "{}", item.name);
        for wire in [26, 27, 28] {
            let end = if wire == 26 { item.wire26 } else { item.wire27 };
            let refused = end.map(|tic| (tic, SimError::from(MapScaleError::Arithmetic)));
            let result = play(&mut level(item, wire), &commands, &item.strict);
            assert_eq!(result, (None, refused), "{} wire {wire}", item.name);
        }
    }
    let wrap = &items[1];
    assert_eq!((wrap.wire26, wrap.wire27), (Some(52), Some(52)));
    assert_eq!(wrap.overshoot, 52);
}

/// The harness can fail: Rust differs from the planted-fault build at capture 39 of
/// SLIDE, an altered digest is reported at its capture, and SLIDE without its small
/// forward move at tic 38 differs at capture 39. (WRAP's genuine run does not depend
/// on the overshoot: the wrapped fractions are dropped.)
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
        bent[at] = format!("x{}", &bent[at][1..]);
        let result = play(&mut level(slide, 27), &commands, &bent);
        assert_eq!(result, (Some(at), None));
    }
    let mut quiet = commands.clone();
    assert_ne!(quiet[slide.overshoot].forward, 0);
    quiet[slide.overshoot].forward = 0;
    let expected = (Some(slide.overshoot + 1), None);
    assert_eq!(play(&mut level(slide, 27), &quiet, &slide.strict), expected);
    assert_eq!(items[1].fault, items[1].strict);
}
