//! Genuine active-player/Demon source scenarios and separate production measurements.
#[path = "../../../tests/support/m3g9.rs"]
mod source;
use doom_sim::*;
use std::{
    error::Error,
    hint::black_box,
    io::Write,
    time::{Duration, Instant},
};
type ResultHost<T> = Result<T, Box<dyn Error>>;
const CALLS: usize = 100;
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
    if !args.is_empty() {
        return Err("use --measure workload advance|step".into());
    }
    let digest = source::verify_parts(3);
    println!(
        "player_world_pickups: C world pickups and sustained combat: source/schema/policy checks complete digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    // Warm-up is outside both counters/timer and is part of the declared input.
    let (index, warmup) = match label {
        "idle" => (0, 16),
        "clip" => (1, 0),
        "full_ammo" => (7, 0),
        "four_ammo" => (16, 0),
        "stim" => (18, 0),
        "medikit" => (22, 0),
        "blocked_pickup" => (28, 0),
        "sixteen" => (48, 0),
        "shotgun" => (33, 0),
        "chaingun" => (34, 0),
        "world_weapon_fight" => (47, 0),
        "supplies_melee" => (46, 192),
        "after_heal" => (46, 254),
        "player_death" => (49, 412),
        "death_transition" => (50, 0),
        "full_encounter" => (46, 0),
        "losing_encounter" => (49, 0),
        _ => return Err("unknown workload".into()),
    };
    let c = &source::cases()[index];
    let calls = if matches!(label, "full_encounter" | "losing_encounter") {
        c.tics
    } else {
        CALLS
    };
    let (mut sim, _) = source::setup(c);
    for op in 1..=warmup {
        sim.advance(if label == "idle" {
            [TicCommand::default(); 4]
        } else {
            source::commands(c, op)
        })?;
    }
    let (result, time) = interval(|| -> Result<(), SimError> {
        for op in warmup + 1..=warmup + calls {
            let commands = if label == "idle" || op > c.tics {
                [TicCommand::default(); 4]
            } else {
                source::commands(c, op)
            };
            if mode == "advance" {
                sim.advance(commands)?;
            } else {
                drop(black_box(sim.step(commands)?));
            }
        }
        Ok(())
    })?;
    result?;
    println!(
        "player_world_pickups label={label} mode={mode} calls={calls} warmup={warmup} ns={} digest={:016x}",
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
