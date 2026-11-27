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
//! rules later steps of M6b build (the pause command's `BT_SPECIAL` byte). A step
//! that builds a rule moves its items' ends; nothing else in this file needs to
//! change.
use crate::nightmare::Fault;
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
pub(crate) fn demo(name: &str) -> (u8, Vec<TicCommand>) {
    demo_at(&format!("m6b_scripts/{name}.lmp"))
}
fn demo_at(path: &str) -> (u8, Vec<TicCommand>) {
    let lmp = std::fs::read(reference(path)).expect("demo");
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
    level_from(wad, map, skill, parameters)
}
fn level_from(
    wad: &str,
    map: u8,
    skill: Skill,
    parameters: GameParameters,
) -> Result<Simulation, LevelStartError> {
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
}

const SPECIAL: SimError = SimError::UnsupportedButtons;

/// Each item's end at this step (M6b phase 2 step 4: Nightmare's ammunition,
/// spawn bit and reaction time, the fast table, `A_Chase`'s `fastparm` half and
/// the corpse respawn built). The pause command's byte is refused
/// (`UnsupportedButtons`) until step 6.
const EXPECTED: [(&str, End); 19] = [
    ("AMMO4", End::Level),
    ("BITS4", End::Level),
    ("BITS3", End::Level),
    ("CLAMP5", End::Level),
    ("REACT4", End::Level),
    ("REACT3", End::Level),
    ("REACT3F", End::Level),
    ("FAST4", End::Level),
    ("FAST3", End::Level),
    ("FAST3F", End::Level),
    ("RESP4", End::Level),
    ("RESP3", End::Level),
    ("RESP3R", End::Level),
    ("BLOCK4", End::Level),
    ("GIB4", End::Level),
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

/// One item of `tools/reference/wire29_stale.py` at its I row's skill and
/// parameters: its golden's rows, split by comma, and the port's play of its demo against the
/// golden's captures.
fn wire29_item(name: &str, wad: &str) -> (Vec<Vec<String>>, Play) {
    let golden = format!("wire29_stale/{}_state.txt", name.to_lowercase());
    let text = std::fs::read_to_string(reference(&golden)).expect("golden");
    let rows: Vec<Vec<String>> = text
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split(',').map(str::to_owned).collect())
        .collect();
    let captures: Vec<String> = rows
        .iter()
        .filter(|f| f[0] == "C")
        .map(|f| f[3].clone())
        .collect();
    let (map, commands) = demo_at(&format!("wire29_stale/{name}.lmp"));
    let i = rows.iter().find(|f| f[0] == "I").expect("the I row");
    let skill =
        [Skill::Medium, Skill::Hard, Skill::Nightmare][i[2].parse::<usize>().expect("skill") - 2];
    let parameters = GameParameters {
        respawn: i[3] == "1",
        fast: i[4] == "1",
    };
    let wad = format!("wire29_stale/{wad}");
    let mut s = level_from(&wad, map, skill, parameters).expect("loads");
    let p = play(&mut s, &commands, &captures);
    (rows, p)
}

/// A STALE item: the genuine engine's first read of a freed corpse
/// (`first_freed_read`, a leveltime, which is the session tic: the level starts at
/// tic 0) and the port's play of its demo.
fn stale_item(name: &str, wad: &str) -> (usize, Play) {
    let (rows, p) = wire29_item(name, wad);
    let fact = |fact: &str| -> usize {
        let row = rows.iter().find(|f| f[0] == "O" && f[1] == fact);
        row.expect(fact)[2].parse().expect(fact)
    };
    let (first, reads) = (fact("first_freed_read"), fact("freed_reads"));
    assert!(
        reads > 0 && first < rows.iter().filter(|f| f[0] == "C").count(),
        "the genuine engine reads freed memory"
    );
    (first, p)
}

/// STALE (golden `tests/reference/wire29_stale/stale_state.txt`): a dead player
/// whose `attacker` is an imp's corpse that respawns. The genuine engine reads the
/// corpse through `player->attacker` in `P_DeathThink` on every tic: while it
/// is pending (still in the thinker list) and then, from the observed
/// `first_freed_read`, from a freed zone block. The port equals the genuine
/// engine at every capture before that read and refuses its tic with
/// `FreedRead`, the state kept.
#[test]
fn a_freed_attacker_is_refused_where_the_genuine_engine_first_reads_it() {
    let (first, p) = stale_item("STALE", "stale.wad");
    assert_eq!(p.difference, None, "first differing capture");
    assert_eq!(p.refusal, Some((first, SimError::FreedRead)));
    eprintln!(
        "STALE: {} captures equal, FreedRead at session tic {first}",
        p.digests.len()
    );
}

/// STALE2 (golden `tests/reference/wire29_stale/stale2_state.txt`): an imp whose
/// `target` is a zombieman's corpse goes back to its spawn state with the
/// pointer kept; the corpse respawns and is freed, and a barrel's explosion with
/// no source (p_inter.c:904-911 keeps the target) puts the imp in pain, whose
/// next state runs `A_Chase`: the genuine engine reads the freed block's flags
/// there (p_enemy.c:704-705). The port equals the genuine engine at every capture
/// before that read, the long sleep with the freed target included, and refuses
/// its tic with `FreedRead`, the state kept.
#[test]
fn a_freed_target_is_refused_where_the_genuine_engine_first_reads_it() {
    let (first, p) = stale_item("STALE2", "stale2.wad");
    assert_eq!(p.difference, None, "first differing capture");
    assert_eq!(p.refusal, Some((first, SimError::FreedRead)));
    eprintln!(
        "STALE2: {} captures equal, FreedRead at session tic {first}",
        p.digests.len()
    );
}

/// BOSS2 (golden `tests/reference/wire29_stale/boss2_state.txt`, map E1M8): two
/// Barons telefragged, respawned and telefragged again. `A_BossDeath` keeps no
/// memory (p_enemy.c:1693-1704) and calls the tag-666 `EV_DoFloor` on every last
/// Baron's death (1730-1733): the genuine engine's calls start the floor, skip it
/// while it still moves (p_floor.c's busy test) and start a mover of 0 units on the
/// lowered floor (the `O` rows `boss_calls`, `boss_started`). The port equals the
/// genuine engine at every capture.
#[test]
fn a_repeated_boss_death_re_fires_tag_666_as_the_genuine_engine_does() {
    let (rows, p) = wire29_item("BOSS2", "boss2.wad");
    let fact = |k: &str| rows.iter().find(|f| f[0] == "O" && f[1] == k).expect(k)[2].clone();
    assert_eq!(fact("boss_started"), "1 0 1", "the genuine calls");
    assert_eq!((p.difference, p.refusal), (None, None));
    assert_eq!(p.digests.len(), rows.iter().filter(|f| f[0] == "C").count());
    eprintln!(
        "BOSS2: {} captures equal, boss calls at {}",
        p.digests.len(),
        fact("boss_calls")
    );
}

/// FASTOUT (golden `tests/reference/wire29_stale/fastout_state.txt`, Nightmare):
/// an imp's fireball (20 units a tic on the fast table) passes through the map's
/// outer wall in one move (p_mobj.c:152 splits only positive moves above 15) and
/// flies outside the vertex rectangle until it meets the floor there and
/// explodes (`O` rows `first_outside`, `explode_outside`). Wire 29 follows it
/// (`escaped_missile`) and equals the genuine engine at every capture.
#[test]
fn a_fast_fireball_outside_the_map_flies_as_in_the_genuine_engine() {
    let (rows, p) = wire29_item("FASTOUT", "fastout.wad");
    let fact = |k: &str| rows.iter().find(|f| f[0] == "O" && f[1] == k).expect(k)[2].clone();
    assert!(
        fact("explode_outside").starts_with("True"),
        "the genuine escape"
    );
    assert_eq!((p.difference, p.refusal), (None, None));
    assert_eq!(p.digests.len(), rows.iter().filter(|f| f[0] == "C").count());
    eprintln!(
        "FASTOUT: {} captures equal, outside from capture {}",
        p.digests.len(),
        fact("first_outside")
    );
}

/// The planted faults of the port ([`Fault`]) and the `V` rows of the altered
/// genuine engine each makes the port behave like.
const FAULTS: [(Fault, &str); 16] = [
    (Fault::Bit, "bit"),
    (Fault::NoDouble, "nodouble"),
    (Fault::React, "react"),
    (Fault::FastStates, "fast_states"),
    (Fault::FastSpeeds, "fast_speeds"),
    (Fault::ChaseDir, "chase_dir"),
    (Fault::ChaseCount, "chase_count"),
    (Fault::RespawnFlag, "respawn_flag"),
    (Fault::Timer, "timer"),
    (Fault::Clock, "clock"),
    (Fault::Gate, "gate"),
    (Fault::NoCheck, "nocheck"),
    (Fault::FogSpot, "fog_spot"),
    (Fault::React18, "react18"),
    (Fault::Ambush, "ambush"),
    (Fault::Count, "count"),
];

/// The port with `fault` planted: its digests of captures 0 to `last`, without the
/// DSIM45 reader (a variant's state may be outside it, e.g. the slow table on
/// Nightmare).
fn planted(item: &Item, fault: Fault, last: usize) -> Vec<String> {
    crate::nightmare::with_fault(fault, || {
        let (map, commands) = demo(&item.name);
        let mut s = level(&item.name, map, item.skill, item.parameters).expect("the map loads");
        let mut digests = vec![digest(&s.snapshot())];
        let none = TicCommand::default();
        for c in &commands[..last] {
            s.advance([*c, none, none, none])
                .expect("a tic of the planted port");
            digests.push(digest(&s.snapshot()));
        }
        digests
    })
}

/// Planted faults: with a rule of the port changed as an altered genuine engine
/// changes the C, the port first differs from the genuine engine at that
/// variant's `V` row capture, and there equals the variant's DSIM45 capture.
/// `clamp` has no such row (its skill 5 is outside DSIM45; the clamp belongs to
/// the demo reader, a later step).
#[test]
fn planted_faults_are_caught_at_the_variants_captures() {
    let (items, variants) = golden();
    let mut rows = 0;
    for (fault, name) in FAULTS {
        for (_, item, k, d) in variants.iter().filter(|v| v.0 == name) {
            let k = k.expect("a differing capture");
            let item = items.iter().find(|i| i.name == *item).unwrap();
            let port = planted(item, fault, k);
            let first = port.iter().zip(&item.captures).position(|(a, b)| a != b);
            assert_eq!(first, Some(k), "{name} on {}", item.name);
            assert_eq!(
                port[k], *d,
                "{name} on {}: the variant's capture",
                item.name
            );
            eprintln!("planted {name}: {} caught at capture {k}", item.name);
            rows += 1;
        }
    }
    assert_eq!(rows, 28);
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
        }
        compared += p.digests.len();
        eprintln!("{name}: {} captures equal, end {end:?}", p.digests.len());
    }
    eprintln!(
        "wire 29: {} items, {compared} captures equal to the genuine engine",
        items.len()
    );
}
