//! Wire 26 started as a new game against wire 25 on one demo (M6a step 4, the M5c
//! method of M6a note §4.3; Rust against Rust).
//!
//! `wire26_newgame_check --wad <path> --map E1Mn --demo <lmp>` builds wire 25
//! (`original_player_episode1_v3_map` with the setup calls of every level run) and
//! wire 26 (`original_player_episode1_v4_level` with `LevelStart::NewGame`) for the
//! map with the demo header's skill, runs the demo under both and requires at every
//! capture (tic 0 and every admitted tic) that the DSIM42 capture with its start
//! group removed and its version, `profile` and `context.schema` set back to wire
//! 25's has the DSIM41 capture's bytes and FNV-1a digest, that both read back
//! (`decode_episode1_v4`, refused by `decode_episode1_v3`), that the sound records
//! are equal and that both wires refuse at the same tic with the same error (see
//! `tests/wire26/mod.rs`). Any difference ends the run with a non-zero status. A map
//! both wires refuse at load is reported as such (status 0). It prints one line:
//! the captures compared and the refusal if any. Used by
//! `tools/local/m6a_wire26_real.py` on the 61 real runs.
//! Without `--wad` it prints its usage and exits successfully.

#[path = "../tests/wire26/mod.rs"]
mod wire26;

use doom_sim::demo::parse_demo_episode1_v2;
use std::{error::Error, path::PathBuf};

const USAGE: &str = "usage: wire26_newgame_check --wad <path> --map E1Mn --demo <lmp>";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let (mut wad, mut map, mut demo) = (None::<PathBuf>, None::<String>, None::<PathBuf>);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--wad" => wad = Some(PathBuf::from(value()?)),
            "--map" => map = Some(value()?),
            "--demo" => demo = Some(PathBuf::from(value()?)),
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => return Err(format!("unknown option {other}\n{USAGE}").into()),
        }
    }
    let Some(wad) = wad else {
        println!("{USAGE}");
        return Ok(());
    };
    let map = map.ok_or("--map is required")?;
    let number = map
        .strip_prefix("E1M")
        .and_then(|n| n.parse::<u8>().ok())
        .filter(|n| (1..=9).contains(n))
        .ok_or("--map must be E1M1 to E1M9")?;
    let bytes = std::fs::read(&wad)?;
    let parsed =
        doom_wad::Wad::parse(&bytes, doom_wad::Limits::default()).map_err(|e| format!("{e:?}"))?;
    let level = doom_map::Map::from_wad(&parsed, &map, doom_map::MapLimits::default())
        .map_err(|e| format!("{e:?}"))?;
    let lmp = std::fs::read(demo.ok_or("--demo is required")?)?;
    let (header, demo) = parse_demo_episode1_v2(&lmp, number).map_err(|e| format!("{e:?}"))?;
    let out = wire26::run_pair(&level, number, wire26::skill(header.skill), &demo.commands)?;
    println!(
        "wire26_newgame_check {map}: {} captures equal, {} tics, refused {:?}, load refused {:?}",
        out.captures, out.tics, out.refused, out.load_refused
    );
    Ok(())
}
