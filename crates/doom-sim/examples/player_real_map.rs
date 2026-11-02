//! Original monsters in rooms of classic-map size: source scenarios and separate measurements.
#[path = "../../../tests/support/m3h6.rs"]
mod source;
use doom_sim::*;
use std::{
    error::Error,
    hint::black_box,
    io::Write,
    time::{Duration, Instant},
};
type ResultHost<T> = Result<T, Box<dyn Error>>;
fn interval<T>(run: impl FnOnce() -> T) -> std::io::Result<(T, Duration)> {
    let mut output = std::io::stdout().lock();
    output.write_all(b"DOOM_ALLOC_BEGIN\n")?;
    output.flush()?;
    let start = Instant::now();
    let result = run();
    let elapsed = start.elapsed();
    output.write_all(b"DOOM_ALLOC_END\n")?;
    output.flush()?;
    Ok((result, elapsed))
}
fn main() -> ResultHost<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let [flag, label, mode] = args.as_slice() {
        if flag == "--measure" {
            return measure(label, mode);
        }
    }
    if let [flag, label] = args.as_slice() {
        if flag == "--admit" {
            return admit(label);
        }
    }
    if !args.is_empty() {
        return Err("use --measure workload advance|step|capture, or --admit town|wide".into());
    }
    let digest = source::verify();
    // Committed source rows really exercise real-map scale: slides, blocked moves,
    // hitscan impacts, noise floods across 74 sectors, and traces of up to 73
    // intercepts (the TI observer rows), none reaching the source's 128.
    let (mut slides, mut blocked, mut impacts, mut floods, mut largest) = (0, 0, 0, 0, 0);
    for c in source::cases() {
        for op in 0..=c.tics {
            for (tag, row) in source::rows(c.which, op) {
                slides += usize::from(tag == "SL");
                blocked += usize::from(tag == "M" && row[4] == 0);
                impacts += usize::from(tag == "X" && row[0] == 1 && row[1] == 1);
                floods += usize::from(tag == "N");
                if tag == "TI" {
                    largest = largest.max(row[1]);
                }
            }
        }
    }
    assert_eq!([slides, blocked, impacts, floods], [615, 1508, 136, 5693]);
    assert_eq!(largest, 73);
    assert!(source::EXPECTED_ESCAPES.is_empty());
    assert_eq!(
        OriginalExecutionProfile::PlayerMonstersRealMapV1.wire_id(),
        17
    );
    let town = source::case_map(0);
    assert_eq!(town.linedefs().len(), 516);
    assert_eq!(town.sectors().len(), 74);
    println!(
        "player_real_map: C rooms of 516 linedefs, 74 sectors, a 3,850-norm wall and a 60 x 60 cell BLOCKMAP, traces of up to 73 intercepts, DSIM33: source/schema/policy checks complete digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step", "capture"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    // Full scripted timelines; a whole run is one measured batch.
    let name = match label {
        "town_patrol" => "town_patrol",
        "town_fight" => "town_fight",
        "gallery_shot" => "gallery_monster_shot",
        "wide_sight" => "wide_sight",
        _ => return Err("unknown workload".into()),
    };
    let c = source::cases()
        .into_iter()
        .find(|c| c.label == name)
        .ok_or("committed workload")?;
    let (mut sim, a, things) = source::setup(&c);
    let mut peak = 0;
    let (result, time) = interval(|| -> Result<(), SimError> {
        for op in 1..=c.tics {
            source::controlled(&mut sim, &c, a, &things, op)?;
            let commands = source::commands(&c, op);
            if mode == "advance" {
                sim.advance(commands)?;
            } else if mode == "capture" {
                sim.advance(commands)?;
                drop(black_box(sim.snapshot()));
            } else {
                drop(black_box(sim.step(commands)?));
            }
            peak = peak.max(sim.actor_order().count());
        }
        Ok(())
    })?;
    result?;
    println!(
        "player_real_map label={label} mode={mode} calls={} warmup=0 peak_actors={peak} ns={} digest={:016x}",
        c.tics,
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
/// Admission time of a committed room: the parsed map is built outside the timer,
/// the policy's load-time checks and preparation inside it.
fn admit(label: &str) -> ResultHost<()> {
    let name = match label {
        "town" => "town_fight",
        "wide" => "wide_pursuit",
        "gallery" => "gallery_monster_shot",
        _ => return Err("unknown room".into()),
    };
    let c = source::cases()
        .into_iter()
        .find(|c| c.label == name)
        .ok_or("committed room")?;
    let map = source::case_map(c.map);
    let (lines, sectors, cells) = (
        map.linedefs().len(),
        map.sectors().len(),
        map.blockmap().cell_ranges().len(),
    );
    let (result, time) = interval(|| {
        Simulation::original_player_real_map_map(
            black_box(map),
            MapContextOptions {
                initial_rng: doom_types::DoomRng::from_indices(6, 0),
                respawn_monsters: false,
            },
        )
    })?;
    let sim = result?;
    println!(
        "player_real_map admit={label} linedefs={lines} sectors={sectors} cells={cells} ns={} digest={:016x}",
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
