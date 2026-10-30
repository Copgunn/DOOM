//! C-backed executable command sequences; production and diagnostic measurements.
#[path = "../../../tests/support/m3g4.rs"]
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
    let time = start.elapsed();
    out.write_all(b"DOOM_ALLOC_END\n")?;
    out.flush()?;
    Ok((result, time))
}
fn main() -> HostResult<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let [flag, label, mode] = args.as_slice() {
        if flag == "--measure" {
            return measure(label, mode);
        }
    }
    if !args.is_empty() {
        return Err("use --measure idle|forward|turn_strafe|combined|reaction|continuous|knockback|wall|initialization advance|step".into());
    }
    let mut captures = 0;
    let mut payload = 0;
    let mut sums = [0; 5];
    let mut motion = [0; 2];
    let mut digest = 0;
    for (which, c) in source::cases().iter().enumerate() {
        let (mut sim, a) = source::setup(c);
        let mut trace = source::SourceTrace::default();
        for op in 0..=c.tics {
            let pending = sim.pending_events().len();
            if op != 0 {
                source::operations(&mut sim, a, c, op);
                sim.advance(source::commands(c, op))?;
            }
            let events = if op == 0 {
                sim.pending_events()
            } else {
                &sim.events()[pending..]
            };
            trace.check(&sim, which, op, events, source::rows(which, op));
            let snapshot = sim.snapshot();
            let counts =
                source::check_commands_and_motion(&sim, a, &snapshot, source::rows(which, op));
            for (s, n) in motion.iter_mut().zip(counts) {
                *s += n;
            }
            captures += usize::from(source::evidence().assert_snapshot(&snapshot, which, op));
            payload += source::evidence().assert_events(&snapshot, which, op);
        }
        for (s, n) in sums.iter_mut().zip(trace.counts) {
            *s += n;
        }
        digest ^= sim.snapshot().digest();
    }
    assert_eq!(sums, [9776, 1896, 376, 39, 1264]);
    assert_eq!(motion, [19019, 5852]);
    assert_eq!((captures, payload), (316, 10530));
    println!(
        "player_commands: C captures={captures} ordered_events={payload} digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> HostResult<()> {
    if !["advance", "step"].contains(&mode) {
        return Err("unknown mode".into());
    }
    if label == "initialization" {
        drop(Simulation::original_player_command_map(
            source::map(),
            64,
            MapContextOptions::default(),
        )?);
        let maps: Vec<_> = (0..CALLS).map(|_| source::map()).collect();
        let (result, time) = interval(|| -> HostResult<Option<Simulation>> {
            let mut last = None;
            for map in maps {
                let mut sim =
                    Simulation::original_player_command_map(map, 64, MapContextOptions::default())?;
                sim.spawn_player_body(PlayerSlot::One, [Fixed::ZERO; 2])?;
                last = Some(sim);
            }
            Ok(last)
        })?;
        let sim = result?.ok_or("empty workload")?;
        println!(
            "commands label={label} mode=construct_spawn calls={CALLS} ns={} digest={:016x}",
            time.as_nanos(),
            sim.snapshot().digest()
        );
        return Ok(());
    }
    let index = match label {
        "idle" => 0,
        "forward" => 1,
        "turn_strafe" => 12,
        "combined" => 9,
        "reaction" => 21,
        "continuous" => 10,
        "knockback" => 13,
        "wall" => 15,
        _ => return Err("unknown workload".into()),
    };
    let c = &source::cases()[index];
    let (mut sim, a) = source::setup(c);
    let (result, time) = interval(|| -> Result<(), SimError> {
        for tic in 1..=CALLS {
            source::operations(&mut sim, a, c, tic);
            let commands = source::commands(c, tic);
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
        "commands label={label} mode={mode} calls={CALLS} ns={} digest={:016x}",
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
