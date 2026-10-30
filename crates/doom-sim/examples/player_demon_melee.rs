//! Genuine active-player/Demon source scenarios and separate production measurements.
#[path = "../../../tests/support/m3g6.rs"]
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
        "player_demon_melee: C cases=17 tics=1944 captures=402 event_fields=18532 digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    // Warm-up is outside both counters/timer and is part of the declared input.
    let (index, warmup) = match label {
        "acquisition_selection" => (0, 0),
        "hit_followthrough" => (1, 0),
        "miss" => (2, 0),
        "repeated" => (1, 64),
        "lethal" => (4, 0),
        "moving_pressure" => (16, 0),
        "prior_impulse" => (5, 0),
        "corpse_continuation" => (0, 300),
        "wall" => (13, 0),
        _ => return Err("unknown workload".into()),
    };
    let c = &source::cases()[index];
    let (mut sim, a) = source::setup(c);
    for op in 1..=warmup {
        source::operations(&mut sim, a, c, op);
        sim.advance(source::commands(c, op))?;
    }
    let (result, time) = interval(|| -> Result<(), SimError> {
        for op in warmup + 1..=warmup + CALLS {
            source::operations(&mut sim, a, c, op);
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
        "player_demon_melee label={label} mode={mode} calls={CALLS} warmup={warmup} ns={} digest={:016x}",
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
