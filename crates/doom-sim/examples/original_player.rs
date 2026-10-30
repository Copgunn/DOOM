//! Genuine body lifecycle demonstration and separate advisory host measurements.
#[path = "../../../tests/support/m3g2.rs"]
mod source;
use doom_sim::{identity::*, *};
use doom_types::Fixed;
use std::{
    error::Error,
    hint::black_box,
    io::Write,
    time::{Duration, Instant},
};
type HostResult<T> = Result<T, Box<dyn Error>>;
const CALLS: usize = 100;
// Same host-only markers as tools/measurement/alloc_probe.c; no engine hook.
fn interval<T>(operation: impl FnOnce() -> T) -> std::io::Result<(T, Duration)> {
    let mut out = std::io::stdout().lock();
    out.write_all(b"DOOM_ALLOC_BEGIN\n")?;
    out.flush()?;
    let start = Instant::now();
    let result = operation();
    let elapsed = start.elapsed();
    out.write_all(b"DOOM_ALLOC_END\n")?;
    out.flush()?;
    Ok((result, elapsed))
}
fn setup(demon: bool) -> HostResult<(Simulation, ActorId)> {
    let mut sim = Simulation::original_player_map(source::map(), 64, MapContextOptions::default())?;
    if demon {
        sim.spawn_original_actor(OriginalActorType::MT_SERGEANT)?;
    }
    let body = sim.spawn_player_body(PlayerSlot::One, [Fixed::from_map_units(96), Fixed::ZERO])?;
    Ok((sim, body))
}
fn main() -> HostResult<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let [flag, label, mode] = args.as_slice() {
        if flag == "--measure" {
            return measure(label, mode);
        }
    }
    if !args.is_empty() {
        return Err(
            "use --measure idle|damage_death|demon_source|initialization advance|step".into(),
        );
    }
    let mut captures = 0;
    let mut events = 0;
    let mut digest = 0;
    for (which, c) in source::cases().iter().enumerate() {
        let mut sim = source::setup(which, c);
        let mut trace = source::SourceTrace::default();
        trace.check(&sim, which, 0, sim.pending_events());
        captures += usize::from(source::assert_snapshot(&sim, which, 0));
        events += source::assert_events(&sim, which, 0);
        for op in 1..=c.tics {
            let pending = sim.pending_events().len();
            source::operations(&mut sim, which, op);
            sim.advance(source::COMMANDS)?;
            trace.check(&sim, which, op, &sim.events()[pending..]);
            captures += usize::from(source::assert_snapshot(&sim, which, op));
            events += source::assert_events(&sim, which, op);
        }
        digest ^= sim.snapshot().digest();
    }
    assert_eq!((captures, events), (185, 4256));
    println!("original_player: C captures={captures} ordered_events={events} digest={digest:016x}");
    Ok(())
}
fn measure(label: &str, mode: &str) -> HostResult<()> {
    if !["advance", "step"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    if label == "initialization" {
        // Warm the immutable source policy; parsing/cloning the100 inputs is outside.
        drop(setup(false)?);
        let maps: Vec<_> = (0..CALLS).map(|_| source::map()).collect();
        let (result, elapsed) = interval(|| -> HostResult<Option<Simulation>> {
            let mut last = None;
            for map in maps {
                let mut sim =
                    Simulation::original_player_map(map, 64, MapContextOptions::default())?;
                sim.spawn_player_body(PlayerSlot::One, [Fixed::from_map_units(96), Fixed::ZERO])?;
                last = Some(sim);
            }
            Ok(last)
        })?;
        let sim = result?.ok_or("initialization workload empty")?;
        println!(
            "player label={label} mode=construct_spawn calls={CALLS} ns={} digest={:016x}",
            elapsed.as_nanos(),
            sim.snapshot().digest()
        );
        return Ok(());
    }
    if !["idle", "damage_death", "demon_source"].contains(&label) {
        return Err("unknown player workload".into());
    }
    let (mut sim, body) = setup(label == "demon_source")?;
    let (result, elapsed) = interval(|| -> Result<(), SimError> {
        for tic in 1..=CALLS {
            let request = if label == "demon_source" && tic == 1 {
                Some((Some(ActorId::from_parts(0, 1)), 20))
            } else if label == "damage_death" && tic == 10 {
                Some((None, 20))
            } else if label != "idle" && tic == 50 {
                Some((None, 80))
            } else {
                None
            };
            if let Some((source, amount)) = request {
                sim.damage_actor(DamageRequest {
                    target: body,
                    inflictor: None,
                    source,
                    amount,
                })?;
            }
            if label == "demon_source" && tic == 1 {
                sim.mark_remove(ActorId::from_parts(0, 1))?;
            }
            if mode == "advance" {
                sim.advance(source::COMMANDS)?;
            } else {
                drop(black_box(sim.step(source::COMMANDS)?));
            }
        }
        Ok(())
    })?;
    result?;
    let p = sim.original_player(PlayerSlot::One)?;
    assert_eq!(p.health, if label == "idle" { 100 } else { 0 });
    println!(
        "player label={label} mode={mode} calls={CALLS} ns={} digest={:016x}",
        elapsed.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
