//! Genuine active-player/Demon source scenarios and separate production measurements.
#[path = "../../../tests/support/m3g8.rs"]
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
        "player_weapon_pack: C weapon expansion: source/schema/policy checks complete digest={digest:016x}"
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
        "fallback" => (1, 0),
        "zero_ammo" => (2, 0),
        "release" => (3, 0),
        "fist_hit" => (26, 0),
        "fist_kill" => (26, 400),
        "fist_miss" => (27, 0),
        "switch_moving" => (8, 0),
        "switch_death" => (10, 128),
        "shotgun" => (12, 0),
        "shotgun_held" => (13, 0),
        "shotgun_death" => (13, 128),
        "shotgun_player_death" => (28, 192),
        "shotgun_mixed" => (14, 0),
        "shotgun_fallback" => (16, 0),
        "chaingun" => (18, 0),
        "chaingun_last" => (19, 0),
        "retaliation" => (20, 0),
        "chaingun_release" => (21, 0),
        "chaingun_death" => (29, 128),
        "fallback_priority" => (23, 0),
        "pending_precedence" => (24, 0),
        "opening_switch" | "full_encounter" => (25, 0),
        "two_way" => (25, 256),
        _ => return Err("unknown workload".into()),
    };
    let c = &source::cases()[index];
    let calls = if label == "full_encounter" {
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
            let commands = if label == "idle" {
                [TicCommand::default(); 4]
            } else {
                source::commands(c, op.min(c.tics))
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
        "player_weapon_pack label={label} mode={mode} calls={calls} warmup={warmup} ns={} digest={:016x}",
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
