//! Simulation-only timing companion to `player_level_run` (M4a, H12).
//!
//! The same public setup and demo commands run without per-tic snapshots,
//! hashing or I/O. Only successful `Simulation::advance` calls contribute to
//! `advance_ns`; a refused call has its own duration. Final capture/hash work,
//! parsing, admission and spawning are outside the interval. Clock overhead is
//! reported, not subtracted. This is a developer measurement, not a time gate.
use doom_sim::{
    MapContextOptions, PlayerSlot, Simulation, Snapshot, TicCommand, WeaponInventory,
    demo::{parse_demo, parse_demo_episode1},
};
use std::{error::Error, fs, hint::black_box, path::PathBuf, time::Instant};

type HostResult<T> = Result<T, Box<dyn Error>>;
const USAGE: &str = "player_level_advance --wad PATH --map E1Mn --demo PATH \
    [--policy level|episode1] (wire 22 default, wire 23 episode1); no captures";

#[derive(Default)]
struct Args {
    wad: Option<PathBuf>,
    demo: Option<PathBuf>,
    map: String,
    episode1: bool,
}

fn args() -> HostResult<Option<Args>> {
    let mut args = Args {
        map: "E1M1".into(),
        ..Args::default()
    };
    let mut input = std::env::args().skip(1);
    while let Some(flag) = input.next() {
        match flag.as_str() {
            "--wad" => args.wad = Some(input.next().ok_or(USAGE)?.into()),
            "--demo" => args.demo = Some(input.next().ok_or(USAGE)?.into()),
            "--map" => args.map = input.next().ok_or(USAGE)?,
            "--policy" => {
                args.episode1 = match input.next().as_deref() {
                    Some("episode1") => true,
                    Some("level") => false,
                    _ => return Err(USAGE.into()),
                };
            }
            "--help" => return Ok(None),
            _ => return Err(USAGE.into()),
        }
    }
    Ok(args.wad.is_some().then_some(args))
}

// The C comparator's projection (level_compare.py::stripped), after timing.
fn stripped(snapshot: &Snapshot) -> Vec<u8> {
    let fields: Vec<_> = snapshot
        .fields()
        .filter(|field| !field.path().starts_with("map.") && !field.path().starts_with("events."))
        .collect();
    let mut bytes = b"DSIM".to_vec();
    bytes.extend(snapshot.version().to_le_bytes());
    bytes.extend((fields.len() as u64).to_le_bytes());
    for field in fields {
        bytes.extend((field.path().len() as u32).to_le_bytes());
        bytes.extend(field.path().as_bytes());
        bytes.extend((field.value().len() as u64).to_le_bytes());
        bytes.extend(field.value());
    }
    bytes
}

fn main() -> HostResult<()> {
    let Some(args) = args()? else {
        println!("{USAGE}");
        return Ok(());
    };
    let demo_bytes = fs::read(args.demo.ok_or(USAGE)?)?;
    let demo = if args.episode1 {
        let number = match args.map.as_bytes() {
            [b'E', b'1', b'M', number @ b'1'..=b'9'] => number - b'0',
            _ => return Err("episode1 needs E1M1..E1M9".into()),
        };
        parse_demo_episode1(&demo_bytes, number)?.1
    } else {
        parse_demo(&demo_bytes)?
    };
    let bytes = fs::read(args.wad.ok_or(USAGE)?)?;
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default())?;
    let map = doom_map::Map::from_wad(&wad, &args.map, doom_map::MapLimits::default())?;
    let mut simulation = if args.episode1 {
        Simulation::original_player_episode1_map(map, MapContextOptions::default())?
    } else {
        Simulation::original_player_level_map(map, MapContextOptions::default())?
    };
    simulation.spawn_map_things()?;
    simulation.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())?;
    simulation.spawn_level_specials()?;
    doom_sim::take_level_run_stats();

    let mut clocks = [0u128; 1001];
    for clock in &mut clocks {
        *clock = Instant::now().elapsed().as_nanos();
    }
    clocks.sort_unstable();
    let (mut admitted, mut advance_ns, mut refused_ns, mut refused_at) =
        (0usize, 0u128, 0u128, None);
    let loop_start = Instant::now();
    for (index, command) in demo.commands.iter().enumerate() {
        let commands = black_box([
            *command,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ]);
        let start = Instant::now();
        let result = black_box(simulation.advance(commands));
        let elapsed = start.elapsed().as_nanos();
        if result.is_err() {
            refused_ns = elapsed;
            refused_at = Some(index + 1);
            break;
        }
        advance_ns += elapsed;
        admitted += 1;
    }
    let loop_ns = loop_start.elapsed().as_nanos();
    let snapshot = simulation.snapshot();
    let refused_at = refused_at.map_or_else(|| "null".into(), |tic| tic.to_string());
    println!(
        "{{\"commands\":{},\"admitted\":{admitted},\"refused_at\":{refused_at},\
        \"advance_ns\":{advance_ns},\"refused_ns\":{refused_ns},\"loop_ns\":{loop_ns},\
        \"clock_floor_ns\":{},\"clock_median_ns\":{},\"final_tic\":{},\
        \"final_fnv\":\"{:016x}\",\"final_sha_stripped\":\"{}\"}}",
        demo.commands.len(),
        clocks[0],
        clocks[clocks.len() / 2],
        snapshot.tic(),
        snapshot.digest(),
        doom_test_evidence::sha256(&stripped(&snapshot)),
    );
    Ok(())
}
