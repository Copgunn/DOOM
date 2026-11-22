//! Shared by the wire-26 tests and `examples/wire26_newgame_check.rs` (M6a step 4):
//! a demo run under wire 25 and under wire 26 started as a new game, side by side,
//! with the M5c method's check at every capture (M6a note §4.3): the DSIM42 capture
//! with its start group (`level.start.*`) removed and its version, `profile` and
//! `context.schema` set back to wire 25's has the DSIM41 capture's bytes and digest.
//! Both runs must refuse at the same tic with the same error, if at all, and give
//! the same sound records.
#![allow(dead_code)]

use doom_sim::*;

/// What one paired run observed.
#[derive(Debug, Default)]
pub struct Outcome {
    /// Captures compared (tic 0 included).
    pub captures: usize,
    /// Admitted tics.
    pub tics: usize,
    /// The refused tic and its error, the same under both wires.
    pub refused: Option<(usize, String)>,
    /// The load refusal, the same under both wires (no capture then).
    pub load_refused: Option<String>,
    /// Wire 25's load refusal of a map wire 26 loads: only a map holding the walk
    /// exits 52 or 124, which wire 26 alone admits (no capture compared then).
    pub wire25_only_refused: Option<String>,
}

/// DSIM framing (STATE_FORMAT "Framing and field order"): `DSIM`, u32 version,
/// u64 field count, then per field u32 path length, path, u64 value length, value.
pub fn encode(version: u32, fields: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = b"DSIM".to_vec();
    out.extend(version.to_le_bytes());
    out.extend((fields.len() as u64).to_le_bytes());
    for (path, value) in fields {
        out.extend((path.len() as u32).to_le_bytes());
        out.extend(path.as_bytes());
        out.extend((value.len() as u64).to_le_bytes());
        out.extend(value);
    }
    out
}

/// FNV-1a-64 over bytes, the digest `Snapshot::digest` documents.
pub fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}

/// A DSIM42 capture as wire 25 would name it: the start group removed, the
/// version 41, `profile` 25 and the wire-25 context. Panics unless the capture is
/// version 42 with policy 26 and the wire-26 context.
pub fn as_wire25(s: &Snapshot) -> Vec<u8> {
    assert_eq!(s.version(), 42, "a wire-26 capture is DSIM42");
    let fields: Vec<(String, Vec<u8>)> = s
        .fields()
        .filter(|f| !f.path().starts_with("level.start."))
        .map(|f| {
            let value = match f.path() {
                "profile" => {
                    assert_eq!(f.value(), 26_u32.to_le_bytes(), "wire 26's profile");
                    25_u32.to_le_bytes().to_vec()
                }
                "context.schema" => {
                    assert_eq!(f.value(), PLAYER_EPISODE1_V4_CONTEXT_SCHEMA);
                    PLAYER_EPISODE1_V3_CONTEXT_SCHEMA.to_vec()
                }
                _ => f.value().to_vec(),
            };
            (f.path().to_owned(), value)
        })
        .collect();
    encode(41, &fields)
}

/// The start group's fields of a DSIM42 capture, in order.
pub fn start_group(s: &Snapshot) -> Vec<(String, Vec<u8>)> {
    s.fields()
        .filter(|f| f.path().starts_with("level.start."))
        .map(|f| (f.path().to_owned(), f.value().to_vec()))
        .collect()
}

fn check(tic: usize, a: &Simulation, b: &Simulation) -> Result<(), String> {
    let (sa, sb) = (a.snapshot(), b.snapshot());
    if sa.version() != 41 {
        return Err(format!("tic {tic}: wire 25 wrote version {}", sa.version()));
    }
    let projected = as_wire25(&sb);
    let expected = sa.canonical_bytes();
    if projected != expected || fnv(&projected) != sa.digest() {
        return Err(format!("tic {tic}: the projected DSIM42 capture differs"));
    }
    let limits = DiagnosticLimits::default();
    let bytes = sb.canonical_bytes();
    Diagnostic::decode_episode1_v4(&bytes, limits)
        .map_err(|e| format!("tic {tic}: DSIM42 reader: {e:?}"))?;
    if Diagnostic::decode_episode1_v3(&bytes, limits).is_ok() {
        return Err(format!(
            "tic {tic}: the DSIM41 reader read a DSIM42 capture"
        ));
    }
    if a.sound_records() != b.sound_records()
        || a.pending_sound_records() != b.pending_sound_records()
    {
        return Err(format!("tic {tic}: sound records differ"));
    }
    Ok(())
}

/// Wire 25 with the setup calls of every level run (`tests/wire25/mod.rs`'s).
pub fn build25(map: doom_map::Map, number: u8, skill: Skill) -> Result<Simulation, String> {
    let mut s = Simulation::original_player_episode1_v3_map(
        map,
        number,
        skill,
        MapContextOptions::default(),
    )
    .map_err(|e| format!("load: {e:?}"))?;
    s.spawn_map_things().map_err(|e| format!("things: {e:?}"))?;
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())
        .map_err(|e| format!("weapons: {e:?}"))?;
    s.spawn_level_specials()
        .map_err(|e| format!("specials: {e:?}"))?;
    Ok(s)
}

/// Wire 26 at `start`.
pub fn build26(
    map: doom_map::Map,
    number: u8,
    skill: Skill,
    start: LevelStart,
) -> Result<Simulation, String> {
    Simulation::original_player_episode1_v4_level(
        map,
        number,
        skill,
        MapContextOptions::default(),
        start,
    )
    .map_err(|e| format!("wire 26 load: {e:?}"))
}

/// Runs `commands` under wire 25 and wire 26 (new game), checking every capture.
pub fn run_pair(
    map: &doom_map::Map,
    number: u8,
    skill: Skill,
    commands: &[TicCommand],
) -> Result<Outcome, String> {
    let mut out = Outcome::default();
    let (mut a, mut b) = match (
        build25(map.clone(), number, skill),
        Simulation::original_player_episode1_v4_level(
            map.clone(),
            number,
            skill,
            MapContextOptions::default(),
            LevelStart::NewGame,
        ),
    ) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(x), Err(y)) => {
            // The same refusal: wire 25's error inside wire 26's load error.
            let y = format!("{y:?}");
            let inner = x.split_once(": ").map_or(x.as_str(), |(_, e)| e);
            if !y.contains(inner) {
                return Err(format!("load: wire 25 {x}, wire 26 {y}"));
            }
            out.load_refused = Some(x);
            return Ok(out);
        }
        (Err(x), Ok(_))
            if map.linedefs().iter().any(|l| matches!(l.special, 52 | 124))
                && x.contains("Pursuit(Specials)") =>
        {
            out.wire25_only_refused = Some(x);
            return Ok(out);
        }
        (x, y) => {
            return Err(format!(
                "load: wire 25 {:?}, wire 26 {:?}",
                x.err(),
                y.err()
            ));
        }
    };
    check(0, &a, &b)?;
    out.captures += 1;
    for (i, c) in commands.iter().enumerate() {
        let tic = i + 1;
        let cmd = [
            *c,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ];
        match (a.advance(cmd), b.advance(cmd)) {
            (Ok(()), Ok(())) => {}
            (Err(x), Err(y)) if x == y => {
                out.refused = Some((tic, format!("{x:?}")));
                break;
            }
            (x, y) => return Err(format!("tic {tic}: wire 25 {x:?}, wire 26 {y:?}")),
        }
        out.tics += 1;
        check(tic, &a, &b)?;
        out.captures += 1;
    }
    Ok(out)
}

/// The skill of a version-110 demo header byte (2 or 3 only, as wire 25).
pub fn skill(byte: u8) -> Skill {
    match byte {
        2 => Skill::Medium,
        3 => Skill::Hard,
        other => panic!("skill {other} is not admitted by wires 25 and 26"),
    }
}
