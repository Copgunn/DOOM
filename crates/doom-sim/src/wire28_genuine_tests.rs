//! Wire 28 (`PlayerMonstersEpisode1V6`, DSIM44) against the genuine engine on the
//! synthetic wire 28 items: nine map slots of `tests/reference/wire28_wads/wire28.wad`
//! (`tools/reference/wire28_wad.py`) played from their demos. The goldens,
//! `tests/reference/wire28_state.txt`, come from `tools/reference/wire28_goldens.py`:
//! the genuine linuxdoom-1.10 runner (strict build; identical streams in the overflow
//! and address builds) and the independent DSIM44 encoder; no Rust output feeds them.
//!
//! **Tic convention.** Capture `j` is the state after `j` tics; a refusal at session
//! tic `N` keeps capture `N`. **Digest**: `tests/reference/m6a_state.txt`'s state
//! digest over DSIM44 (a wire-27 capture is first renamed: version 44, `profile` 28,
//! wire 28's context; nothing else differs, `wire28_tests`).
//!
//! - Every item equals the genuine engine at every capture up to its end: the whole
//!   demo, the genuine level's last tic (`exit:N`, sector 11's exit), or the escape
//!   witness, which refuses session tic `N` with `MapScale(Escape)` and capture `N`
//!   kept while the genuine corpse leaves the room in that tic (`escape:N`).
//! - Wire 27 (the frozen wire, renamed) equals the genuine engine on the skill-2
//!   items without a cap or an embedded monster, diverges on BITE2 at the capture
//!   after the bite that the genuine sector-11 cap keeps alive, and refuses the
//!   embedded rooms at the load; wire 28 still refuses an embedded barrel and an
//!   embedded player at the load.
//! - Planted faults in the port, through crate-private fields as `wire28_tests`
//!   does: `sk_baby` off (`PlayerRuntime::baby`: no doubling, no halving) is caught at
//!   the first pickup or blow; the escape witness removed lets the refused tic run.
//! - Altered goldens (`V` rows: the genuine engine with a rule of p_inter.c removed or
//!   moved): the port differs from each at the capture where it first differs from
//!   the genuine run, so a port with that rule removed or moved is caught there.
use crate::*;
use doom_test_evidence::sha256;
use std::path::{Path, PathBuf};

const V5: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V5;
const V6: OriginalExecutionProfile = OriginalExecutionProfile::PlayerMonstersEpisode1V6;

fn reference(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(path)
}

/// One item of the golden, its end and the genuine digest of every capture.
#[derive(Clone, Debug)]
struct Item {
    name: String,
    map: u8,
    skill: Skill,
    end: End,
    captures: Vec<String>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum End {
    Complete,
    Exit(usize),
    Escape(usize),
}
/// The items and the `V` rows (variant, item, first differing capture, digest).
fn golden() -> (Vec<Item>, Vec<(String, String, usize, String)>) {
    let text = std::fs::read_to_string(reference("wire28_state.txt")).expect("golden");
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    assert_eq!(lines.next(), Some("#schema,DOOM-WIRE28-STATE-v1"));
    let (mut items, mut variants) = (Vec::<Item>::new(), Vec::new());
    for line in lines.filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        let int = |i: usize| f[i].parse::<usize>().expect(line);
        match f[0] {
            "I" => {
                let end = match f[6].split_once(':') {
                    None => End::Complete,
                    Some(("exit", n)) => End::Exit(n.parse().expect(line)),
                    Some(("escape", n)) => End::Escape(n.parse().expect(line)),
                    _ => panic!("end {line}"),
                };
                let skill = [Skill::Baby, Skill::Easy, Skill::Medium, Skill::Hard][int(3)];
                items.push(Item {
                    name: f[1].to_owned(),
                    map: f[2].parse().expect(line),
                    skill,
                    end,
                    captures: Vec::with_capacity(int(5)),
                });
            }
            "C" => {
                let item = items.last_mut().expect("an I row first");
                assert_eq!((item.name.as_str(), item.captures.len()), (f[1], int(2)));
                item.captures.push(f[3].to_owned());
            }
            "V" => variants.push((f[1].to_owned(), f[2].to_owned(), int(3), f[4].to_owned())),
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

/// The state digest of a capture (module documentation).
fn digest(s: &Snapshot) -> String {
    let rename = s.version() == 43;
    assert!(rename || s.version() == 44, "a wire 27 or 28 capture");
    let value = |f: &StateField| -> Vec<u8> {
        match (rename, f.path()) {
            (true, "profile") => 28_u32.to_le_bytes().to_vec(),
            (true, "context.schema") => PLAYER_EPISODE1_V6_CONTEXT_SCHEMA.to_vec(),
            _ => f.value().to_vec(),
        }
    };
    let kept = |f: &&StateField| !f.path().starts_with("map.") && !f.path().starts_with("events.");
    let mut body = b"DSIM".to_vec();
    body.extend(44_u32.to_le_bytes());
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
    let lmp = std::fs::read(reference(&format!("wire28_scripts/{name}.lmp"))).expect("demo");
    demo::parse_demo_episode1_v3(&lmp, lmp[3])
        .expect("demo parse")
        .1
        .commands
}

fn level(
    map: u8,
    skill: Skill,
    policy: OriginalExecutionProfile,
) -> Result<Simulation, LevelStartError> {
    let bytes = std::fs::read(reference("wire28_wads/wire28.wad")).expect("wad");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("wad parse");
    let map_data =
        doom_map::Map::from_wad(&wad, &format!("E1M{map}"), doom_map::MapLimits::default())
            .expect("map");
    let build = if policy == V6 {
        Simulation::original_player_episode1_v6_level
    } else {
        Simulation::original_player_episode1_v5_level
    };
    build(
        map_data,
        map,
        skill,
        MapContextOptions::default(),
        LevelStart::NewGame,
    )
}

/// The outcome of one play: every digest the port took (capture 0 on), the first
/// capture that differs from `expected`, and the refusal (session tic, error).
struct Play {
    digests: Vec<String>,
    difference: Option<usize>,
    refusal: Option<(usize, SimError)>,
}

/// Plays `tics` commands (or until a refusal), comparing every capture with
/// `expected` as far as it reaches; after a refusal the kept state must be the
/// capture before it.
fn play(s: &mut Simulation, commands: &[TicCommand], tics: usize, expected: &[String]) -> Play {
    let none = TicCommand::default();
    let mut p = Play {
        digests: vec![digest(&s.snapshot())],
        difference: None,
        refusal: None,
    };
    for (tic, c) in commands.iter().take(tics).enumerate() {
        if let Err(e) = s.advance([*c, none, none, none]) {
            assert_eq!(
                digest(&s.snapshot()),
                p.digests[tic],
                "the state kept after the refusal"
            );
            p.refusal = Some((tic, e));
            break;
        }
        p.digests.push(digest(&s.snapshot()));
    }
    p.difference = p.digests.iter().zip(expected).position(|(a, b)| a != b);
    p
}

/// The number of tics an item's comparison plays: the whole demo, the genuine
/// level's tics, or through the escape tic.
fn tics(item: &Item, commands: &[TicCommand]) -> usize {
    match item.end {
        End::Complete => commands.len(),
        End::Exit(n) | End::Escape(n) => n + 1,
    }
}

fn run(item: &Item, policy: OriginalExecutionProfile, fault: impl FnOnce(&mut Simulation)) -> Play {
    let commands = demo(&item.name);
    let mut s = level(item.map, item.skill, policy).expect("the map loads");
    fault(&mut s);
    play(&mut s, &commands, tics(item, &commands), &item.captures)
}

const ESCAPE: SimError = SimError::MapScale(MapScaleError::Escape);

/// Every item equals the genuine engine at every capture up to its end; ESCAPE's
/// refusal is the escape witness at the genuine crossing tic. The altered goldens
/// each differ from the port at their first differing capture.
#[test]
fn wire_28_equals_the_genuine_engine_on_every_item() {
    let (items, variants) = golden();
    assert_eq!(items.len(), 12);
    let mut compared = 0;
    let mut digests = std::collections::BTreeMap::new();
    for item in &items {
        let p = run(item, V6, |_| {});
        assert_eq!(p.difference, None, "{}: first differing capture", item.name);
        match item.end {
            End::Escape(n) => {
                assert_eq!(p.refusal, Some((n, ESCAPE)), "{}", item.name);
                assert_eq!(p.digests.len(), n + 1);
            }
            _ => assert_eq!(p.refusal, None, "{}", item.name),
        }
        compared += p.digests.len();
        digests.insert(item.name.clone(), p.digests);
    }
    eprintln!("wire 28: 12 items, {compared} captures equal to the genuine engine");
    assert_eq!(variants.len(), 8);
    for (v, name, k, d) in &variants {
        let port = &digests[name];
        assert!(
            *k > 0 && port[*k] != *d,
            "{v} {name}: the port is not caught at {k}"
        );
        let genuine = &items.iter().find(|i| &i.name == name).unwrap().captures;
        assert_eq!(port[..*k], genuine[..*k], "{v} {name}");
        eprintln!("altered golden {v}: {name} caught at capture {k}");
    }
}

/// Wire 27 (renamed) equals the genuine engine on the skill-2 items that meet no
/// wire-28 rule, diverges on BITE2 at capture 97 (the bite of session tic 96 kills
/// the player under wire 27, the genuine cap leaves 1 health), and refuses the
/// rooms with an embedded imp at the load, as master refuses E1M7 on skill 3; wire
/// 28 refuses an embedded barrel (E1M8) and player (E1M9) with wire 27's errors.
#[test]
fn wire_27_diverges_on_the_bite_and_refuses_the_embedded_rooms() {
    let (items, _) = golden();
    let by = |n: &str| items.iter().find(|i| i.name == n).unwrap().clone();
    for name in ["AMMO2", "DROP2", "HURT2", "SPLASH2"] {
        assert_eq!(
            run(&by(name), V5, |_| {}).difference,
            None,
            "{name} under wire 27"
        );
    }
    let bite = run(&by("BITE2"), V5, |_| {});
    assert_eq!(
        (bite.difference, bite.refusal),
        (Some(97), None),
        "BITE2 under wire 27"
    );
    let embedded = Err(LevelStartError::Setup(MonsterError::Arithmetic.into()));
    for map in [6, 7] {
        assert_eq!(
            level(map, Skill::Medium, V5).map(|_| ()),
            embedded,
            "E1M{map} under wire 27"
        );
    }
    let errors = [
        MonsterError::Arithmetic.into(),
        PlayerBodyError::Binding.into(),
    ];
    for (map, error) in [(8, errors[0]), (9, errors[1])] {
        for policy in [V5, V6] {
            let got = level(map, Skill::Medium, policy).map(|_| ());
            assert_eq!(
                got,
                Err(LevelStartError::Setup(error)),
                "E1M{map} {policy:?}"
            );
        }
    }
}

/// Planted faults in the port: `sk_baby` off is caught at the first doubled pickup
/// or halved blow (the variants' captures). Without the escape witness the refused
/// tic runs: the corpse's centre ends in the void inside the vertex rectangle (the
/// room's west notch), where no other check refuses it, and the port then equals the
/// genuine engine at every capture to the demo's end, so the witness alone refuses.
#[test]
fn planted_faults_in_the_port_are_caught() {
    let (items, _) = golden();
    let by = |n: &str| items.iter().find(|i| i.name == n).unwrap().clone();
    for (name, at) in [
        ("AMMO0", 10),
        ("DROP0", 321),
        ("HURT0", 1),
        ("SPLASH0", 58),
        ("BITE0", 1),
    ] {
        let p = run(&by(name), V6, |s| {
            s.original_player.as_mut().unwrap().baby = false
        });
        assert_eq!(p.difference, Some(at), "{name} without sk_baby");
    }
    let mut item = by("ESCAPE");
    let End::Escape(n) = item.end else {
        panic!("ESCAPE ends at the witness")
    };
    item.end = End::Complete;
    let p = run(&item, V6, |s| s.world.escape_witness = None);
    assert_eq!(
        (p.refusal, p.difference),
        (None, None),
        "ESCAPE without the witness"
    );
    assert!(p.digests.len() > n + 2);
    eprintln!(
        "ESCAPE without the witness: {} captures equal, past tic {n}",
        p.digests.len()
    );
}
