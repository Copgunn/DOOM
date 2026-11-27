//! Wire 29 (`PlayerMonstersEpisode1V7`, DSIM45) against the genuine engine on the
//! 19 M6b items (`tools/reference/m6b_wad.py`). The goldens,
//! `tests/reference/wire29_state.txt`, come from `tools/reference/wire29_goldens.py`:
//! the genuine linuxdoom-1.10 M6b runner (strict build; its dump sets are those
//! pinned in `tests/reference/m6b_genuine.txt`) and the independent DSIM45 encoder;
//! no Rust output feeds them.
//!
//! **Tic convention.** Capture `j` is the state after `j` session tics; a refusal at
//! session tic `N` keeps capture `N`. **Digest**: `tests/reference/m6a_state.txt`'s
//! state digest over DSIM45. Every capture the port takes must also be read by the
//! DSIM45 reader (`Diagnostic::decode_episode1_v7`).
//!
//! Each item plays its demo under wire 29 at the skill and parameters of its header
//! and must equal the genuine engine at every capture up to the end [`EXPECTED`]
//! names: the whole level, or a refusal at a session tic. The refusals are the
//! rules later steps of M6b build (temporary [`Wire29Pending`] refusals, the
//! pause command's `BT_SPECIAL` byte). A step that builds a rule moves its items'
//! ends; nothing else in this file needs to change.
use crate::*;
use doom_test_evidence::sha256;
use std::path::{Path, PathBuf};

fn reference(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(path)
}

/// One item of the golden: its header values and the genuine digest of every
/// projected capture.
#[derive(Clone, Debug)]
struct Item {
    name: String,
    skill: Skill,
    parameters: GameParameters,
    captures: Vec<String>,
}
/// A `V` row: variant, item, first differing DSIM45 capture (`None`: equal, or
/// refused by the encoder), the variant's digest there.
type Variant = (String, String, Option<usize>, String);

fn golden() -> (Vec<Item>, Vec<Variant>) {
    let text = std::fs::read_to_string(reference("wire29_state.txt")).expect("golden");
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    assert_eq!(lines.next(), Some("#schema,DOOM-WIRE29-STATE-v1"));
    let (mut items, mut variants) = (Vec::<Item>::new(), Vec::new());
    for line in lines.filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        let int = |i: usize| f[i].parse::<usize>().expect(line);
        match f[0] {
            "I" => items.push(Item {
                name: f[1].to_owned(),
                skill: [
                    Skill::Baby,
                    Skill::Easy,
                    Skill::Medium,
                    Skill::Hard,
                    Skill::Nightmare,
                ][int(3)],
                parameters: GameParameters {
                    respawn: f[4] == "1",
                    fast: f[5] == "1",
                },
                captures: Vec::with_capacity(int(9)),
            }),
            "C" => {
                let item = items.last_mut().expect("an I row first");
                assert_eq!((item.name.as_str(), item.captures.len()), (f[1], int(2)));
                item.captures.push(f[3].to_owned());
            }
            "V" => variants.push((
                f[1].to_owned(),
                f[2].to_owned(),
                f[4].parse().ok(),
                f[5].to_owned(),
            )),
            _ => panic!("row {line}"),
        }
    }
    (items, variants)
}

fn framed(out: &mut Vec<u8>, path: &str, value: &[u8]) {
    out.extend((path.len() as u32).to_le_bytes());
    out.extend(path.as_bytes());
    out.extend((value.len() as u64).to_le_bytes());
    out.extend(value);
}

/// The digest of the simulation's capture, which the DSIM45 reader must read.
fn checked(s: &Simulation) -> String {
    let snap = s.snapshot();
    let bytes = snap.canonical_bytes();
    if let Err(e) = Diagnostic::decode_episode1_v7(&bytes, DiagnosticLimits::default()) {
        panic!(
            "the DSIM45 reader refuses the capture of tic {}: {e:?}",
            s.tic()
        );
    }
    digest(&snap)
}

/// The state digest of a DSIM45 capture (module documentation).
fn digest(s: &Snapshot) -> String {
    assert_eq!(s.version(), 45, "a wire 29 capture");
    let kept = |f: &&StateField| !f.path().starts_with("map.") && !f.path().starts_with("events.");
    let mut body = b"DSIM".to_vec();
    body.extend(45_u32.to_le_bytes());
    body.extend((s.fields().filter(kept).count() as u64).to_le_bytes());
    for f in s.fields().filter(kept) {
        framed(&mut body, f.path(), f.value());
    }
    for f in s
        .fields()
        .filter(|f| f.path().starts_with("events.sounds."))
    {
        framed(&mut body, f.path(), f.value());
    }
    sha256(&body)
}

/// The demo's header byte 3 (the map) and its commands as `G_ReadDemoTiccmd`
/// reads them, every button byte kept (the wire-29 demo reader is a later step).
fn demo(name: &str) -> (u8, Vec<TicCommand>) {
    let lmp = std::fs::read(reference(&format!("m6b_scripts/{name}.lmp"))).expect("demo");
    let body = &lmp[13..lmp.len() - 1];
    assert_eq!((body.len() % 4, lmp.last()), (0, Some(&demo::DEMO_MARKER)));
    let commands = body.chunks_exact(4).map(|c| TicCommand {
        forward: c[0] as i8,
        side: c[1] as i8,
        angle_turn: (u16::from(c[2]) << 8) as i16,
        buttons: c[3],
    });
    (lmp[3], commands.collect())
}

pub(crate) fn level(
    name: &str,
    map: u8,
    skill: Skill,
    parameters: GameParameters,
) -> Result<Simulation, LevelStartError> {
    let wad = if name.starts_with("PAUSEWI") || name.starts_with("PAUSEFIN") {
        "m6a_wads/episode.wad"
    } else {
        "m6b_wads/m6b.wad"
    };
    let bytes = std::fs::read(reference(wad)).expect("wad");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("wad parse");
    let map_data =
        doom_map::Map::from_wad(&wad, &format!("E1M{map}"), doom_map::MapLimits::default())
            .expect("map");
    Simulation::original_player_episode1_v7_level_at(
        map_data,
        map,
        skill,
        parameters,
        MapContextOptions::default(),
        LevelStart::NewGame,
        0,
    )
}

/// How far the port follows an item at this step: every projected capture, or a
/// refusal of session tic `N` (capture `N` kept).
#[derive(Clone, Copy, Debug, PartialEq)]
enum End {
    /// Every projected capture; the run may then stop at the level's exit.
    Level,
    /// Session tic `N` refused with this error.
    Refused(usize, SimError),
    /// The load refused with this setup error.
    Load(SimError),
}

const ITEM_WITNESS: SimError = SimError::PlayerBody(PlayerBodyError::Invariant);
const UNBUILT_STATES: SimError = SimError::Wire29Pending(Wire29Pending::FastStates);
const UNBUILT_MISSILE: SimError = SimError::Wire29Pending(Wire29Pending::FastMissile);
const UNBUILT_CHASE: SimError = SimError::Wire29Pending(Wire29Pending::FastChase);
const UNBUILT_RESPAWN: SimError = SimError::Wire29Pending(Wire29Pending::Respawn);
const SPECIAL: SimError = SimError::UnsupportedButtons;

/// Each item's end at this step (M6b phase 2 step 1). The loads refused at skill 4
/// are the spawn witnesses of items (`world_pickups::validate_item_ref`) and
/// decorations (`e1m1_rules::validate_decoration`), which expect the type's
/// reaction time where `P_SpawnMobj` leaves 0 on Nightmare (p_mobj.c:503-504):
/// step 2. GIB4's zombieman stands in a closed door sector, taller than it
/// (`MapScale(Fit)`, at every wire). The pause command's byte is refused
/// (`UnsupportedButtons`) until step 6.
const EXPECTED: [(&str, End); 19] = [
    ("AMMO4", End::Load(ITEM_WITNESS)),
    ("BITS4", End::Level),
    ("BITS3", End::Level),
    ("CLAMP5", End::Level),
    ("REACT4", End::Refused(29, UNBUILT_MISSILE)),
    ("REACT3", End::Level),
    ("REACT3F", End::Refused(5, UNBUILT_CHASE)),
    ("FAST4", End::Refused(1, UNBUILT_STATES)),
    ("FAST3", End::Level),
    ("FAST3F", End::Refused(1, UNBUILT_STATES)),
    ("RESP4", End::Load(ITEM_WITNESS)),
    ("RESP3", End::Level),
    ("RESP3R", End::Refused(192, UNBUILT_RESPAWN)),
    ("BLOCK4", End::Load(ITEM_WITNESS)),
    ("GIB4", End::Load(SimError::MapScale(MapScaleError::Fit))),
    ("PAUSE", End::Refused(72, SPECIAL)),
    ("PAUSE0", End::Refused(0, SPECIAL)),
    ("PAUSEWI", End::Level),
    ("PAUSEFIN", End::Level),
];

/// The outcome of one play: every digest the port took (capture 0 on), the first
/// capture that differs from `expected`, and the refusal (session tic, error).
struct Play {
    digests: Vec<String>,
    difference: Option<usize>,
    refusal: Option<(usize, SimError)>,
}

/// Plays the commands while there are genuine captures to compare (or until a
/// refusal); after a refusal the kept state must be the capture before it.
fn play(s: &mut Simulation, commands: &[TicCommand], expected: &[String]) -> Play {
    let none = TicCommand::default();
    let mut p = Play {
        digests: vec![checked(s)],
        difference: None,
        refusal: None,
    };
    for (tic, c) in commands.iter().take(expected.len() - 1).enumerate() {
        if let Err(e) = s.advance([*c, none, none, none]) {
            assert_eq!(
                checked(s),
                p.digests[tic],
                "the state kept after the refusal"
            );
            p.refusal = Some((tic, e));
            break;
        }
        p.digests.push(checked(s));
    }
    p.difference = p.digests.iter().zip(expected).position(|(a, b)| a != b);
    p
}

fn run(item: &Item) -> Play {
    let (map, commands) = demo(&item.name);
    let mut s = level(&item.name, map, item.skill, item.parameters).expect("the map loads");
    play(&mut s, &commands, &item.captures)
}

/// Every item equals the genuine engine at every capture up to its end at this
/// step; the refusals are the rules not built yet, each with the state kept.
#[test]
fn wire_29_equals_the_genuine_engine_up_to_each_items_end() {
    let (items, variants) = golden();
    assert_eq!(items.len(), EXPECTED.len());
    assert_eq!(variants.len(), 33);
    let mut compared = 0;
    for (item, (name, end)) in items.iter().zip(EXPECTED) {
        assert_eq!(item.name, name);
        if let End::Load(error) = end {
            let (map, _) = demo(name);
            let got = level(name, map, item.skill, item.parameters).map(|_| ());
            assert_eq!(got, Err(LevelStartError::Setup(error)), "{name}");
            eprintln!("{name}: refused at the load, {error:?}");
            continue;
        }
        let p = run(item);
        assert_eq!(p.difference, None, "{name}: first differing capture");
        match end {
            End::Level => {
                assert_eq!(p.refusal, None, "{name}");
                assert_eq!(p.digests.len(), item.captures.len(), "{name}");
            }
            End::Refused(n, error) => {
                assert_eq!(p.refusal, Some((n, error)), "{name}");
                assert_eq!(p.digests.len(), n + 1, "{name}");
            }
            End::Load(_) => unreachable!(),
        }
        compared += p.digests.len();
        eprintln!("{name}: {} captures equal, end {end:?}", p.digests.len());
    }
    eprintln!(
        "wire 29: {} items, {compared} captures equal to the genuine engine",
        items.len()
    );
}
