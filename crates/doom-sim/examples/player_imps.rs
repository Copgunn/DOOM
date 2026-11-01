//! Original Imp, fireball and mixed-roster source scenarios and separate measurements.
#[path = "../../../tests/support/m3h2.rs"]
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
    // Committed source rows really launch and explode fireballs.
    let (mut launches, mut explosions) = (0, 0);
    for c in source::cases() {
        for op in 0..=c.tics {
            for (tag, row) in source::rows(c.which, op) {
                launches += usize::from(tag == "MS" && row[0] == 5);
                explosions += usize::from(tag == "MS" && row[0] == 0);
            }
        }
    }
    assert_eq!([launches, explosions], [115, 114]);
    assert_eq!(MAX_LIVE_PROJECTILES, 24);
    println!(
        "player_imps: C Imp melee/missile choice, fireball flight, contact and explosion, same-species rule, infighting and mixed rosters: source/schema/policy checks complete digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step", "capture"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    // Full scripted timelines; a whole run is one measured batch.
    let name = match label {
        "imp_alone" => "imp_missile_far",
        "melee" => "imp_melee_contact",
        "infighting" => "fireball_hits_former_human",
        "same_species" => "fireball_hits_imp",
        "pursuit" => "imps_pursuit",
        "mixed_roster" => "mixed_roster",
        "roster_eight_imps" => "roster_eight_imps",
        _ => return Err("unknown workload".into()),
    };
    let c = source::cases()
        .into_iter()
        .find(|c| c.label == name)
        .ok_or("committed workload")?;
    let (mut sim, ..) = source::setup(&c);
    let mut peak = 0;
    let (result, time) = interval(|| -> Result<(), SimError> {
        for op in 1..=c.tics {
            let commands = source::commands(&c, op);
            if mode == "advance" {
                sim.advance(commands)?;
            } else if mode == "capture" {
                sim.advance(commands)?;
                drop(black_box(sim.snapshot()));
            } else {
                drop(black_box(sim.step(commands)?));
            }
            peak = peak.max(sim.actor_order().count());
        }
        Ok(())
    })?;
    result?;
    println!(
        "player_imps label={label} mode={mode} calls={} warmup=0 peak_actors={peak} ns={} digest={:016x}",
        c.tics,
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
