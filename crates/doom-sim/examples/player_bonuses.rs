//! Genuine active-player/Demon source scenarios and separate production measurements.
#[path = "../../../tests/support/m3g11.rs"]
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
        return Err("use --measure workload advance|step|capture".into());
    }
    let digest = source::verify();
    println!(
        "player_bonuses: C bonus animation, resources, counted items, absorption and sustained combat: source/schema/policy checks complete digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step", "capture"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    if label == "initialize" {
        let c = &source::cases()[7];
        // Separately labeled100 complete map/player/item/weapon initializations,
        // not100 gameplay updates. Case parsing remains outside the interval.
        let (last, elapsed) = interval(|| {
            let mut last = None;
            for _ in 0..CALLS {
                last = Some(black_box(source::setup(c).0));
            }
            last.unwrap()
        })?;
        println!(
            "player_bonuses label={label} mode={mode} calls={CALLS} warmup=0 ns={} digest={:016x}",
            elapsed.as_nanos(),
            last.snapshot().digest()
        );
        return Ok(());
    }
    // Warm-up is outside both counters/timer and is part of the declared input.
    let (index, warmup) = match label {
        "none_idle" => (0, 0),
        "health_idle" => (1, 0),
        "armor_idle" => (4, 0),
        "sixteen" => (7, 0),
        "collect_health" => (2, 0),
        "collect_armor" => (5, 0),
        "multiple_counted" => (11, 0),
        "green_above100" => (21, 8),
        "health_above100" => (19, 8),
        "mixed_resources" => (23, 0),
        "full_encounter" => (19, 0),
        "full_depletion" => (24, 0),
        "frozen_snapshot" => (7, 0),
        _ => return Err("unknown workload".into()),
    };
    let c = &source::cases()[index];
    let calls = if matches!(label, "full_encounter" | "full_depletion") {
        c.tics
    } else {
        CALLS
    };
    let (mut sim, _) = source::setup(c);
    for op in 1..=warmup {
        sim.advance(if label.ends_with("idle") {
            [TicCommand::default(); 4]
        } else {
            source::commands(c, op)
        })?;
    }
    let (result, time) = interval(|| -> Result<(), SimError> {
        for op in warmup + 1..=warmup + calls {
            let commands = if label.ends_with("idle") || op > c.tics {
                [TicCommand::default(); 4]
            } else {
                source::commands(c, op)
            };
            if label == "frozen_snapshot" {
                drop(black_box(sim.snapshot()));
            } else if mode == "advance" {
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
        "player_bonuses label={label} mode={mode} calls={calls} warmup={warmup} ns={} digest={:016x}",
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
