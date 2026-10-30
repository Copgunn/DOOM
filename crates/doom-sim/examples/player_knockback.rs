//! Original player physical motion, C-backed assertions and advisory host timing.
#[path = "../../../tests/support/m3g3.rs"]
mod source;
use doom_sim::*;
use doom_types::Fixed;
use std::{
    error::Error,
    hint::black_box,
    io::Write,
    time::{Duration, Instant},
};
type HostResult<T> = Result<T, Box<dyn Error>>;
const CALLS: usize = 100;
fn interval<T>(op: impl FnOnce() -> T) -> std::io::Result<(T, Duration)> {
    let mut out = std::io::stdout().lock();
    out.write_all(b"DOOM_ALLOC_BEGIN\n")?;
    out.flush()?;
    let start = Instant::now();
    let result = op();
    let elapsed = start.elapsed();
    out.write_all(b"DOOM_ALLOC_END\n")?;
    out.flush()?;
    Ok((result, elapsed))
}
fn main() -> HostResult<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let [flag, label, mode] = args.as_slice() {
        if flag == "--measure" {
            return measure(label, mode);
        }
    }
    if !args.is_empty() {
        return Err("use --measure physical|friction|collision|blocker|moving_corpse|initialization advance|step".into());
    }
    let mut captures = 0;
    let mut events = 0;
    let mut digest = 0;
    let mut sums = [0; 5];
    for (which, c) in source::cases().iter().enumerate() {
        let (mut sim, a) = source::setup(c);
        let mut trace = source::SourceTrace::default();
        trace.check(&sim, which, 0, sim.pending_events(), source::rows(which, 0));
        captures += usize::from(source::assert_snapshot(&sim, which, 0));
        events += source::assert_events(&sim, which, 0);
        for op in 1..=c.tics {
            let pending = sim.pending_events().len();
            source::operations(&mut sim, a, c, op);
            sim.advance(source::COMMANDS)?;
            trace.check(
                &sim,
                which,
                op,
                &sim.events()[pending..],
                source::rows(which, op),
            );
            captures += usize::from(source::assert_snapshot(&sim, which, op));
            events += source::assert_events(&sim, which, op);
        }
        for (sum, count) in sums.iter_mut().zip(trace.counts) {
            *sum += count;
        }
        digest ^= sim.snapshot().digest();
    }
    assert_eq!(sums, [12922, 1416, 497, 247, 944]);
    assert_eq!((captures, events), (236, 6652));
    println!(
        "player_knockback: C captures={captures} ordered_events={events} digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> HostResult<()> {
    if !["advance", "step"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    if label == "initialization" {
        drop(Simulation::original_player_knockback_map(
            source::map(),
            64,
            MapContextOptions::default(),
        )?);
        let maps: Vec<_> = (0..CALLS).map(|_| source::map()).collect();
        let (result, time) = interval(|| -> HostResult<Option<Simulation>> {
            let mut last = None;
            for map in maps {
                let mut sim = Simulation::original_player_knockback_map(
                    map,
                    64,
                    MapContextOptions::default(),
                )?;
                sim.spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])?;
                last = Some(sim);
            }
            Ok(last)
        })?;
        let sim = result?.ok_or("empty construction workload")?;
        println!(
            "knockback label={label} mode=construct_spawn calls={CALLS} ns={} digest={:016x}",
            time.as_nanos(),
            sim.snapshot().digest()
        );
        return Ok(());
    }
    let which = match label {
        "physical" => 1,
        "friction" => 7,
        "collision" => 8,
        "blocker" => 9,
        "moving_corpse" => 10,
        _ => return Err("unknown knockback workload".into()),
    };
    let c = &source::cases()[which];
    let (mut sim, a) = source::setup(c);
    let (result, time) = interval(|| -> Result<(), SimError> {
        for tic in 1..=CALLS {
            source::operations(&mut sim, a, c, tic);
            if mode == "advance" {
                sim.advance(source::COMMANDS)?;
            } else {
                drop(black_box(sim.step(source::COMMANDS)?));
            }
        }
        Ok(())
    })?;
    result?;
    assert_eq!(
        sim.actor(a.body).ok_or("lost body")?.momentum(),
        [Fixed::ZERO; 2]
    );
    println!(
        "knockback label={label} mode={mode} calls={CALLS} ns={} digest={:016x}",
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
