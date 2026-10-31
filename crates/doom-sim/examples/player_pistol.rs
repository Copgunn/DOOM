//! Genuine active-player/Demon source scenarios and separate production measurements.
#[path = "../../../tests/support/m3g7.rs"]
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
    let digest = source::verify();
    println!(
        "player_pistol: C cases=17 tics=2488 captures=464 event_fields=35028 digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    // Warm-up is outside both counters/timer and is part of the declared input.
    let (index, warmup) = match label {
        "idle" => (1, 16),
        "first_shot" => (2, 0),
        "held_refire" => (3, 0),
        "repeated" => (0, 64),
        "miss" => (15, 0),
        "wall" => (6, 0),
        "demon_hits" => (0, 0),
        "demon_pain" => (0, 64),
        "demon_death" => (0, 160),
        "moving_fire" => (14, 0),
        "reaction" => (12, 0),
        "retaliation" => (0, 18),
        "two_way" => (0, 160),
        "player_death" => (13, 284),
        "empty_idle" => (11, 0),
        _ => return Err("unknown workload".into()),
    };
    let c = &source::cases()[index];
    let (mut sim, _) = source::setup(c);
    for op in 1..=warmup {
        sim.advance(source::commands(c, op))?;
    }
    let (result, time) = interval(|| -> Result<(), SimError> {
        for op in warmup + 1..=warmup + CALLS {
            let commands = source::commands(c, op);
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
        "player_pistol label={label} mode={mode} calls={CALLS} warmup={warmup} ns={} digest={:016x}",
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
