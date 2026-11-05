//! A whole level under `PlayerMonstersLevelV1` (wire 22) driven by a version-110
//! demo, for a tic-for-tic comparison with the genuine engine (M3h9c).
//!
//! `player_level_run --wad <path> --map E1M1 --demo <lmp> --out <dir> [--captures]`
//! loads the map, builds `original_player_level_map` with default options
//! (medium skill, single player, RNG indices 0), spawns the THINGs (the body at
//! its record), the default weapons and the level specials, then runs one tic
//! per demo command. It writes `digests.txt` (one line per capture: tic, SHA-256
//! of the canonical capture without its `map.*` fields, the FNV-1a digest of the
//! whole capture, the gameplay RNG index, the actor and thinker counts), with
//! `--captures` every capture as `tic_NNNNN.bin`, `stats.txt` (work maxima) and,
//! on a refused tic, `refused.txt` (the tic and the error) before stopping.
//! Captures of a commercial IWAD hold its map bytes: keep them out of Git.
//! Without `--wad` it prints its usage and exits successfully.
//! `player_level_run --write-idle <tics> --demo <lmp>` instead writes a demo of
//! `tics` zero commands with `write_demo` and exits.
use doom_sim::{
    demo::{parse_demo, write_demo},
    *,
};
use std::{error::Error, fmt::Write as _, fs, path::PathBuf};

type ResultHost<T> = Result<T, Box<dyn Error>>;

const USAGE: &str = "usage: player_level_run --wad <path> --map E1M1 --demo <lmp> --out <dir> \
     [--captures] | --write-idle <tics> --demo <lmp>";

/// The canonical bytes of `snap` without its static `map.*` fields.
fn without_map(snap: &Snapshot) -> Vec<u8> {
    let fields: Vec<_> = snap
        .fields()
        .filter(|f| !f.path().starts_with("map."))
        .collect();
    let mut out = b"DSIM".to_vec();
    out.extend(snap.version().to_le_bytes());
    out.extend((fields.len() as u64).to_le_bytes());
    for f in fields {
        out.extend((f.path().len() as u32).to_le_bytes());
        out.extend(f.path().as_bytes());
        out.extend((f.value().len() as u64).to_le_bytes());
        out.extend(f.value());
    }
    out
}

fn field<'a>(snap: &'a Snapshot, path: &str) -> &'a [u8] {
    snap.fields()
        .find(|f| f.path() == path)
        .map_or(&[], StateField::value)
}

/// One `digests.txt` line, plus the actor and thinker counts.
fn line(snap: &Snapshot) -> (String, usize, usize) {
    let actors = field(snap, "thinker.order").len() / 12;
    let thinkers = field(snap, "thinker.shared_order").len() / 13;
    let rng = field(snap, "rng.gameplay").first().copied().unwrap_or(0);
    (
        format!(
            "{} {} {:016x} {rng} {actors} {thinkers}",
            snap.tic(),
            doom_test_evidence::sha256(&without_map(snap)),
            snap.digest()
        ),
        actors,
        thinkers,
    )
}

#[derive(Default)]
struct Args {
    wad: Option<PathBuf>,
    map: String,
    demo: Option<PathBuf>,
    out: Option<PathBuf>,
    captures: bool,
    write_idle: Option<usize>,
}
fn args() -> ResultHost<Args> {
    let mut a = Args {
        map: "E1M1".into(),
        ..Args::default()
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--wad" => a.wad = it.next().map(PathBuf::from),
            "--map" => a.map = it.next().ok_or(USAGE)?,
            "--demo" => a.demo = it.next().map(PathBuf::from),
            "--out" => a.out = it.next().map(PathBuf::from),
            "--captures" => a.captures = true,
            "--write-idle" => a.write_idle = Some(it.next().ok_or(USAGE)?.parse()?),
            _ => return Err(USAGE.into()),
        }
    }
    Ok(a)
}

fn main() -> ResultHost<()> {
    let a = args()?;
    if let Some(tics) = a.write_idle {
        let demo = write_demo(&vec![TicCommand::default(); tics])?;
        fs::write(a.demo.ok_or(USAGE)?, &demo)?;
        println!("wrote an idle demo of {tics} tics ({} bytes)", demo.len());
        return Ok(());
    }
    let Some(wad_path) = a.wad else {
        println!("{USAGE}");
        return Ok(());
    };
    let (demo_path, out) = (a.demo.ok_or(USAGE)?, a.out.ok_or(USAGE)?);
    let bytes = fs::read(wad_path)?;
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default())?;
    let map = doom_map::Map::from_wad(&wad, &a.map, doom_map::MapLimits::default())?;
    let demo = parse_demo(&fs::read(demo_path)?)?;
    let mut s = Simulation::original_player_level_map(map, MapContextOptions::default())?;
    let report = s.spawn_map_things()?;
    s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())?;
    s.spawn_level_specials()?;
    fs::create_dir_all(&out)?;
    // Load work is not a tic's: the maxima cover the tics only.
    take_level_run_stats();
    let mut digests = String::new();
    let snap = s.snapshot();
    let (text, mut peak_actors, mut peak_thinkers) = line(&snap);
    writeln!(digests, "{text}")?;
    if a.captures {
        fs::write(out.join("tic_00000.bin"), snap.canonical_bytes())?;
    }
    let (mut changes, mut clips, mut admitted) = (0usize, 0usize, 0usize);
    let mut refused = None;
    let mut last = snap.digest();
    for (i, command) in demo.commands.iter().enumerate() {
        let tic = i + 1;
        if let Err(error) = s.advance([
            *command,
            TicCommand::default(),
            TicCommand::default(),
            TicCommand::default(),
        ]) {
            fs::write(out.join("refused.txt"), format!("{tic} {error:?}\n"))?;
            refused = Some(tic);
            break;
        }
        admitted += 1;
        let (c, t) = s.events().iter().fold((0, 0), |(c, t), e| match e {
            Event::Mover(MoverEvent::SectorChanged { .. }) => (c + 1, t),
            Event::Mover(MoverEvent::ThingClipped { .. }) => (c, t + 1),
            _ => (c, t),
        });
        changes = changes.max(c);
        clips = clips.max(t);
        let snap = s.snapshot();
        let (text, actors, thinkers) = line(&snap);
        peak_actors = peak_actors.max(actors);
        peak_thinkers = peak_thinkers.max(thinkers);
        writeln!(digests, "{text}")?;
        if a.captures {
            fs::write(
                out.join(format!("tic_{tic:05}.bin")),
                snap.canonical_bytes(),
            )?;
        }
        last = snap.digest();
    }
    fs::write(out.join("digests.txt"), digests)?;
    let stats = take_level_run_stats();
    fs::write(
        out.join("stats.txt"),
        format!(
            "max_intercepts {}\nmax_spechit {}\nmax_change_sector_calls_per_tic {changes}\n\
             max_change_sector_thing_clips_per_tic {clips}\npeak_actors {peak_actors}\n\
             peak_thinkers {peak_thinkers}\n",
            stats.max_intercepts, stats.max_spechit
        ),
    )?;
    println!(
        "spawned {} body {} commands {} admitted {admitted} refused {} final_fnv {last:016x}",
        report.spawned.len(),
        report.player_body.is_some(),
        demo.commands.len(),
        refused.map_or("no".into(), |t| format!("at tic {t}")),
    );
    Ok(())
}
