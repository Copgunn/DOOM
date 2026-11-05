//! Wire 22 (`PlayerMonstersLevelV1`, DSIM38) against the independent encoder's synthetic
//! goldens (M3h9c). The fixtures `tests/reference/m3h9c_level_*.bin` and
//! `m3h9c_synthetic_goldens.json` were projected by tools/reference/level_snapshot.py
//! from the genuine engine's dumps of the synthetic WAD (m3h9c_snapshot_goldens.py), never
//! from Rust output. Here every fixture decodes with `Diagnostic::decode_level`, the load
//! fixture equals Rust's own load capture field for field (map included; `events.*` is
//! not projected by the encoder, decision G5), and SYNIDLE's demo replayed through
//! `doom_sim::demo` gives the golden per-tic digest sequence.
use doom_sim::{demo::parse_demo, *};
use std::path::{Path, PathBuf};

fn reference(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/reference")
        .join(name)
}

/// (path, value) pairs of a canonical DSIM capture.
fn fields(bytes: &[u8]) -> (u32, Vec<(String, Vec<u8>)>) {
    assert_eq!(&bytes[..4], b"DSIM");
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let count = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let (mut at, mut out) = (16, Vec::new());
    for _ in 0..count {
        let len = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        let path = String::from_utf8(bytes[at + 4..at + 4 + len].to_vec()).unwrap();
        at += 4 + len;
        let len = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()) as usize;
        out.push((path, bytes[at + 8..at + 8 + len].to_vec()));
        at += 8 + len;
    }
    assert_eq!(at, bytes.len());
    (version, out)
}

/// The comparator's stripped form: the DSIM framing of the fields outside `map.*` and,
/// unless `keep_map`, `events.*`.
fn stripped(version: u32, all: &[(String, Vec<u8>)], keep_map: bool) -> Vec<u8> {
    let kept: Vec<_> = all
        .iter()
        .filter(|(p, _)| !p.starts_with("events.") && (keep_map || !p.starts_with("map.")))
        .collect();
    let mut out = b"DSIM".to_vec();
    out.extend(version.to_le_bytes());
    out.extend((kept.len() as u64).to_le_bytes());
    for (p, v) in kept {
        out.extend((p.len() as u32).to_le_bytes());
        out.extend(p.as_bytes());
        out.extend((v.len() as u64).to_le_bytes());
        out.extend(v);
    }
    out
}

fn value<'a>(all: &'a [(String, Vec<u8>)], path: &str) -> &'a [u8] {
    &all.iter().find(|(p, _)| p == path).expect(path).1
}

fn level() -> Simulation {
    let bytes = std::fs::read(reference("m3h9c_synthetic.wad")).expect("synthetic WAD");
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).expect("WAD");
    let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default()).expect("E1M1");
    let mut s = Simulation::original_player_level_map(map, MapContextOptions::default())
        .expect("wire-22 admission");
    s.spawn_map_things().expect("THINGs");
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .expect("weapons");
    s.spawn_level_specials().expect("specials");
    s
}

fn fixtures() -> Vec<PathBuf> {
    let mut out: Vec<_> = std::fs::read_dir(reference(""))
        .expect("tests/reference")
        .map(|e| e.expect("entry").path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("m3h9c_level_") && n.ends_with(".bin"))
        })
        .collect();
    out.sort();
    out
}

#[test]
fn synthetic_level_fixtures_decode() {
    let paths = fixtures();
    assert!(paths.len() >= 9, "the committed fixtures");
    for path in paths {
        let bytes = std::fs::read(&path).expect("fixture");
        Diagnostic::decode_level(&bytes, DiagnosticLimits::default())
            .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        let (version, all) = fields(&bytes);
        assert_eq!(version, 38);
        assert_eq!(value(&all, "profile"), 22u32.to_le_bytes());
        assert_eq!(value(&all, "context.schema"), PLAYER_LEVEL_CONTEXT_SCHEMA);
        assert_eq!(value(&all, "store.max_slots"), 384u32.to_le_bytes());
        let slots = u64::from_le_bytes(value(&all, "store.slot_count").try_into().unwrap());
        assert!(slots <= 384);
        let mo = value(&all, "original_player.0.mo");
        assert_eq!(mo[0], 1, "the body is present");
        let slot = u32::from_le_bytes(mo[1..5].try_into().unwrap());
        assert_eq!(value(&all, &format!("actor.{slot}.occupied")), [1]);
        assert_eq!(value(&all, &format!("actor.{slot}.player")), [0]);
        assert_eq!(value(&all, &format!("actor.{slot}.spawnpoint")), [0]);
        assert_eq!(value(&all, &format!("actor.{slot}.angle")).len(), 4);
    }
}

#[test]
fn synthetic_load_equals_the_encoder_fixture() {
    let expected = std::fs::read(reference("m3h9c_level_SYNIDLE_0.bin")).expect("load fixture");
    let (ev, ef) = fields(&expected);
    let snap = level().snapshot();
    let (rv, rf) = fields(&snap.canonical_bytes());
    assert_eq!(stripped(rv, &rf, true), stripped(ev, &ef, true));
}

fn golden(script: &str, key: &str) -> String {
    let text = std::fs::read_to_string(reference("m3h9c_synthetic_goldens.json")).expect("goldens");
    let at = text.find(&format!("\"{script}\": {{")).expect("script");
    let tail = &text[at..];
    let needle = format!("\"{key}\": ");
    let start = tail.find(&needle).expect(key) + needle.len();
    tail[start..]
        .split([',', '\n'])
        .next()
        .unwrap()
        .trim()
        .trim_matches('"')
        .to_string()
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn synidle_replay_matches_the_encoder_goldens() {
    let demo = parse_demo(&std::fs::read(reference("m3h9c_scripts/SYNIDLE.lmp")).expect("demo"))
        .expect("version-110 demo");
    let tics: usize = golden("SYNIDLE", "tics_projected").parse().unwrap();
    assert_eq!(demo.commands.len() + 1, tics);
    let mut s = level();
    let digest = |s: &Simulation| {
        let (v, f) = fields(&s.snapshot().canonical_bytes());
        unhex(&doom_test_evidence::sha256(&stripped(v, &f, false)))
    };
    let mut sequence = digest(&s);
    let mut last = sequence.clone();
    for command in &demo.commands {
        s.advance([
            *command,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ])
        .expect("an admitted tic");
        last = digest(&s);
        sequence.extend(&last);
    }
    assert_eq!(
        doom_test_evidence::sha256(&sequence),
        golden("SYNIDLE", "sequence_sha256")
    );
    assert_eq!(
        last.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        golden("SYNIDLE", "final_sha256")
    );
}
