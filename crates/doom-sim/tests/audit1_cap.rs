//! Audit 1, item 1 (2026-11-26): the sector-11 cap of `P_DamageMobj` (p_inter.c:836-842)
//! run, path by path, against the genuine engine under wires 23 to 28.
//!
//! The goldens, `tests/reference/audit1_cap/cap_state.txt`, come from
//! `tools/reference/audit1_cap.py`: the genuine linuxdoom-1.10 runner (strict build;
//! identical streams in the overflow and address builds) and the independent DSIM44
//! encoder; no Rust output feeds them. Each item is a room whose every sector has
//! special 11 and one damage path that lands a blow of at least the player's health;
//! the genuine engine leaves health 1 and the level exits. The `L` row names the blow,
//! shown by the altered genuine engine `nocap` (the cap's condition made false), whose
//! player is dead at the capture after it (`V` rows).
//!
//! Per item and wire the verdict is **capped** (equal to the genuine engine at every
//! capture), **not capped** (first differs at the capture after the lethal blow, the
//! player dead there) or **refused** (the load or a tic refused, with its error).
//! Method: wires 26 to 28 compare the whole state (DSIM42 and DSIM43 are DSIM44 field for
//! field; a capture is renamed to wire 28's `profile` and context first); wires 23 to 25
//! compare the projection that DSIM39 to DSIM44 share (`#projected` in the golden: no
//! `profile`, `context.schema`, `level.map`, `level.start.*`, `map.*` or `events.*`).
use doom_sim::*;
use doom_test_evidence::sha256;
use std::path::{Path, PathBuf};

fn reference(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(path)
}

struct Item {
    name: String,
    wad: String,
    map: u8,
    lethal: usize,
    whole: Vec<String>,
    projected: Vec<String>,
}

/// The items and the `nocap` rows (item, first differing capture, digest there).
fn golden() -> (Vec<Item>, Vec<(String, usize, String)>) {
    let text = std::fs::read_to_string(reference("audit1_cap/cap_state.txt")).expect("golden");
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    assert_eq!(lines.next(), Some("#schema,DOOM-AUDIT1-CAP-STATE-v1"));
    let (mut items, mut variants) = (Vec::<Item>::new(), Vec::new());
    for line in lines.filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        let int = |i: usize| f[i].parse::<usize>().expect(line);
        match f[0] {
            "I" => items.push(Item {
                name: f[1].into(),
                wad: f[2].into(),
                map: int(3) as u8,
                lethal: 0,
                whole: Vec::new(),
                projected: Vec::new(),
            }),
            "L" => items.last_mut().unwrap().lethal = int(2),
            "C" => {
                let item = items.last_mut().unwrap();
                assert_eq!((item.name.as_str(), item.whole.len()), (f[1], int(2)));
                item.whole.push(f[3].into());
                item.projected.push(f[4].into());
            }
            "V" => variants.push((f[2].into(), int(3), f[4].into())),
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

/// (whole, projected) digests of a capture; whole only for DSIM42 to DSIM44.
fn digests(s: &Snapshot) -> (Option<String>, String) {
    let body = |keep: &dyn Fn(&str) -> bool, rename: bool| {
        let value = |f: &StateField| match (rename, f.path()) {
            (true, "profile") => 28_u32.to_le_bytes().to_vec(),
            (true, "context.schema") => PLAYER_EPISODE1_V6_CONTEXT_SCHEMA.to_vec(),
            _ => f.value().to_vec(),
        };
        let kept: Vec<&StateField> = s.fields().filter(|f| keep(f.path())).collect();
        let mut out = b"DSIM".to_vec();
        out.extend(44_u32.to_le_bytes());
        out.extend((kept.len() as u64).to_le_bytes());
        for f in kept {
            framed(&mut out, f.path(), &value(f));
        }
        out
    };
    let state = |p: &str| !p.starts_with("map.") && !p.starts_with("events.");
    let shared = |p: &str| {
        state(p)
            && !["profile", "context.schema", "level.map"].contains(&p)
            && !p.starts_with("level.start.")
    };
    let whole = (42..=44).contains(&s.version()).then(|| {
        let mut b = body(&state, true);
        for f in s
            .fields()
            .filter(|f| f.path().starts_with("events.sounds."))
        {
            framed(&mut b, f.path(), f.value());
        }
        sha256(&b)
    });
    (whole, sha256(&body(&shared, false)))
}

fn level(wire: u32, item: &Item) -> Result<Simulation, String> {
    let bytes = std::fs::read(reference(&item.wad["tests/reference/".len()..])).expect("wad");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("wad parse");
    let name = format!("E1M{}", item.map);
    let map = doom_map::Map::from_wad(&wad, &name, doom_map::MapLimits::default()).expect("map");
    let (n, sk, ctx, new) = (
        item.map,
        Skill::Medium,
        MapContextOptions::default(),
        LevelStart::NewGame,
    );
    let e = |e: &dyn std::fmt::Debug| format!("{e:?}");
    let mut s = match wire {
        23 => Simulation::original_player_episode1_map(map, ctx).map_err(|x| e(&x))?,
        24 => Simulation::original_player_episode1_v2_map(map, n, sk, ctx).map_err(|x| e(&x))?,
        25 => Simulation::original_player_episode1_v3_map(map, n, sk, ctx).map_err(|x| e(&x))?,
        26 => {
            return Simulation::original_player_episode1_v4_level(map, n, sk, ctx, new)
                .map_err(|x| e(&x));
        }
        27 => {
            return Simulation::original_player_episode1_v5_level(map, n, sk, ctx, new)
                .map_err(|x| e(&x));
        }
        _ => {
            return Simulation::original_player_episode1_v6_level(map, n, sk, ctx, new)
                .map_err(|x| e(&x));
        }
    };
    s.spawn_map_things().map_err(|x| e(&x))?;
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .map_err(|x| e(&x))?;
    s.spawn_level_specials().map_err(|x| e(&x))?;
    Ok(s)
}

fn field<'a>(s: &'a Snapshot, path: &str) -> &'a [u8] {
    s.fields().find(|f| f.path() == path).expect(path).value()
}
fn health(s: &Snapshot) -> i32 {
    i32::from_le_bytes(field(s, "original_player.0.health").try_into().unwrap())
}

fn commands(item: &Item) -> Vec<TicCommand> {
    let lmp = if item.name == "BITE2" {
        reference("wire28_scripts/BITE2.lmp")
    } else {
        reference(&format!("audit1_cap/{}.lmp", item.name))
    };
    let lmp = std::fs::read(lmp).expect("demo");
    demo::parse_demo_episode1_v3(&lmp, lmp[3])
        .expect("demo")
        .1
        .commands
}

#[derive(Debug, PartialEq)]
enum Verdict {
    Capped,
    NotCapped,
    Refused(String),
    /// Differs elsewhere than at the capture after the lethal blow: unexpected.
    Differs(usize),
}
use Verdict::*;

/// One play under `wire`: the verdict, the whole digests (wires 26-28) and the
/// number of captures compared. `fault` may change one command (the self-test).
fn play(item: &Item, wire: u32, fault: Option<usize>) -> (Verdict, Vec<Option<String>>, usize) {
    let mut s = match level(wire, item) {
        Ok(s) => s,
        Err(e) => return (Refused(format!("load: {e}")), Vec::new(), 0),
    };
    let expected = if wire >= 26 {
        &item.whole
    } else {
        &item.projected
    };
    let mut cmds = commands(item);
    if let Some(t) = fault {
        cmds[t].forward = 25;
    }
    let none = TicCommand::default();
    let (mut shots, mut refusal) = (vec![s.snapshot()], None);
    for (t, c) in cmds.iter().take(expected.len() - 1).enumerate() {
        if let Err(e) = s.advance([*c, none, none, none]) {
            refusal = Some(format!("session tic {t}: {e:?}"));
            break;
        }
        shots.push(s.snapshot());
    }
    let ds: Vec<_> = shots.iter().map(digests).collect();
    let got: Vec<&String> = ds
        .iter()
        .map(|(w, p)| if wire >= 26 { w.as_ref().unwrap() } else { p })
        .collect();
    let wholes = ds.iter().map(|(w, _)| w.clone()).collect();
    match got.iter().zip(expected).position(|(a, b)| *a != b) {
        None if refusal.is_some() => (Refused(refusal.unwrap()), wholes, got.len()),
        None => (Capped, wholes, got.len()),
        Some(k) if k == item.lethal + 1 && health(&shots[k]) <= 0 && health(&shots[k - 1]) > 0 => {
            (NotCapped, wholes, k + 1)
        }
        Some(k) => (Differs(k), wholes, k + 1),
    }
}

/// The verdict of every path under every wire, each established by a run.
#[test]
fn the_sector_11_cap_path_by_path_on_wires_23_to_28() {
    let (items, variants) = golden();
    assert_eq!(items.len(), 11);
    let baron = "load: MapSpawn(UnsupportedType(3003))";
    let mut compared = 0;
    for item in &items {
        for wire in 23..=28 {
            let (verdict, wholes, n) = play(item, wire, None);
            compared += n;
            let expected = match (item.name.as_str(), wire) {
                ("BARONFB" | "BARONCL", 23) => Refused(baron.into()),
                ("SPECTRE" | "BITE2", 23..=27) => NotCapped,
                _ => Capped,
            };
            eprintln!("{} wire {wire}: {verdict:?} ({n} captures)", item.name);
            assert_eq!(verdict, expected, "{} wire {wire}", item.name);
            // A port without the cap is the genuine engine without it: where an older
            // wire does not cap, its whole state at the first differing capture is the
            // `nocap` engine's.
            let (_, k, d) = variants.iter().find(|v| v.0 == item.name).unwrap();
            if verdict == NotCapped && wire >= 26 {
                assert_eq!(
                    wholes[*k].as_deref(),
                    Some(d.as_str()),
                    "{} wire {wire}",
                    item.name
                );
            }
            // The altered golden: the port equals the genuine run before `k` and is
            // caught at `k`, so a port behaving like `nocap` would be caught there.
            if wire == 28 {
                assert_eq!(*k, item.lethal + 1);
                assert_ne!(wholes[*k].as_deref(), Some(d.as_str()), "{}", item.name);
            }
        }
    }
    eprintln!("audit 1 cap: 11 items x 6 wires, {compared} captures compared");
    let bite = |n: &str| items.iter().find(|i| i.name == n).unwrap().whole.clone();
    let w28 = std::fs::read_to_string(reference("wire28_state.txt")).expect("wire 28 golden");
    let committed: Vec<String> = w28
        .lines()
        .filter(|l| l.starts_with("C,BITE2,"))
        .map(|l| l.rsplit(',').next().unwrap().to_owned())
        .collect();
    assert_eq!(
        bite("BITE2"),
        committed,
        "BITE2 regenerated equals wire 28's golden"
    );
}

/// Self-test: the harness can fail. One command changed before the lethal blow (a
/// step forward at session tic 10) is caught at the capture after it.
#[test]
fn a_planted_fault_in_the_commands_is_caught() {
    let (items, _) = golden();
    for name in ["FLOOR", "BITE2"] {
        let item = items.iter().find(|i| i.name == name).unwrap();
        for wire in [25, 28] {
            assert_eq!(
                play(item, wire, Some(10)).0,
                Differs(11),
                "{name} wire {wire}"
            );
        }
    }
}

/// Item 3 of audit 1, pinned (2026-11-26, America/Los_Angeles): the public
/// `Simulation::damage_actor` on the player body in a sector of special 11, at
/// damage equal to its health, **kills it under wire 28**; linuxdoom-1.10 leaves 1
/// health (p_inter.c:836-842). A known divergence, frozen since wire 23 (owner's
/// ruling 3 of 2026-11-26, docs/validation/WIRE28.md section 12); API only, see
/// docs/validation/AUDIT1.md. This test pins the frozen behaviour; it is not the C.
#[test]
fn damage_actor_on_the_player_in_sector_11_kills_without_the_cap() {
    let (items, _) = golden();
    let item = items.iter().find(|i| i.name == "FLOOR").unwrap();
    let mut s = level(28, item).unwrap();
    let mo = field(&s.snapshot(), "original_player.0.mo").to_vec();
    assert_eq!((mo.len(), mo[0]), (13, 1));
    let body = ActorId::from_parts(
        u32::from_le_bytes(mo[1..5].try_into().unwrap()),
        u64::from_le_bytes(mo[5..13].try_into().unwrap()),
    );
    let sector = field(&s.snapshot(), "sector.0.special").to_vec();
    assert_eq!(
        sector,
        11_i16.to_le_bytes(),
        "the room is one sector of special 11"
    );
    let r = s
        .damage_actor(DamageRequest {
            target: body,
            inflictor: None,
            source: None,
            amount: 100,
        })
        .unwrap();
    assert_eq!((r.health, r.killed), ([100, 0], true));
    assert_eq!(health(&s.snapshot()), 0, "the C would leave health 1");
}
