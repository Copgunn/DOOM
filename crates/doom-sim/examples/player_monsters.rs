//! Original former-human/mixed-roster source scenarios and separate measurements.
#[path = "../../../tests/support/m3h1.rs"]
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
    if !args.is_empty() {
        return Err("use --measure workload advance|step|capture".into());
    }
    let digest = source::verify();
    println!(
        "player_monsters: C former humans, mixed rosters, infighting, drops and statistics: source/schema/policy checks complete digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step", "capture"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    // Full scripted timelines; a whole run is one measured batch.
    let index = match label {
        "possessed_alone" => 0,
        "chainguy_alone" => 2,
        "killed_drop" => 5,
        "mixed_group" => 6,
        "infighting" => 7,
        "roster_eight" => 11,
        "pursuit" => 12,
        _ => return Err("unknown workload".into()),
    };
    let c = &source::cases()[index];
    let (mut sim, _) = source::setup(c);
    let (result, time) = interval(|| -> Result<(), SimError> {
        for op in 1..=c.tics {
            let commands = source::commands(c, op);
            if mode == "advance" {
                sim.advance(commands)?;
            } else if mode == "capture" {
                sim.advance(commands)?;
                drop(black_box(sim.snapshot()));
            } else {
                drop(black_box(sim.step(commands)?));
            }
        }
        Ok(())
    })?;
    result?;
    println!(
        "player_monsters label={label} mode={mode} calls={} warmup=0 ns={} digest={:016x}",
        c.tics,
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
