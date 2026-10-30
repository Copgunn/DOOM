//! Genuine active-player/Demon source scenarios and separate production measurements.
#[path = "../../../tests/support/m3g5.rs"]
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
        "player_demon: C cases=25 tics=1124 captures=373 event_fields=12886 digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    let index = match label {
        "active_player" => 0,
        "standing_demon" | "repeated_search" => 5,
        "failed_sight" => 10,
        "acquisition" => 1,
        "stop_slot" => 2,
        "moving_player" => 14,
        "pursuit" => 17,
        "target_death" => 19,
        "target_loss" => 20,
        "retirement" => 23,
        _ => return Err("unknown workload".into()),
    };
    let c = &source::cases()[index];
    let (mut sim, a) = source::setup(c);
    let (result, time) = interval(|| -> Result<(), SimError> {
        for op in 1..=CALLS {
            source::operations(&mut sim, a, c, op);
            if label == "repeated_search" {
                sim.look_for_players(a.origin.ok_or(MapSpawnError::Target)?, false)?;
            }
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
        "player_demon label={label} mode={mode} calls={CALLS} ns={} digest={:016x}",
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
