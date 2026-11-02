//! Original monsters with static heights: source scenarios and separate measurements.
#[path = "../../../tests/support/m3h4.rs"]
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
    // Committed source rows really leave the flat floor: one landing sound, height
    // refusals of every kind, noise floods beyond one sector and wall slides.
    let (mut landings, mut refusals, mut floods, mut slides, mut falls) = (0, 0, 0, 0, 0);
    for c in source::cases() {
        for op in 0..=c.tics {
            for (tag, row) in source::rows(c.which, op) {
                landings += usize::from(tag == "E" && row[0] == 7 && row[2] == 34);
                refusals += usize::from(tag == "M" && row[4] == 0 && (3..=6).contains(&row[9]));
                floods += usize::from(tag == "N");
                slides += usize::from(tag == "SL");
                falls += usize::from(tag == "Z" && row[9] == 1);
            }
        }
    }
    assert_eq!(
        [landings, refusals, floods, slides, falls],
        [1, 204, 172, 258, 147]
    );
    assert_eq!(HEIGHTS_MOMZ_UNITS, 1024);
    assert_eq!(MAP_SCALE_ACTOR_SLOTS, 320);
    println!(
        "player_heights: C steps, falls, landings, ledges, windows, fences and noise floods across static heights, DSIM31: source/schema/policy checks complete digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step", "capture"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    // Full scripted timelines; a whole run is one measured batch.
    let name = match label {
        "stairs" => "imp_pursuit_stairs",
        "ledge_drop" => "drop_landing_oof",
        "corpse_ledge" => "corpse_pushed_off_ledge",
        "window_sight" => "window_sight_pit_then_step",
        "fence_slide" => "fence_slide",
        "ceiling_fireball" => "fireball_up_ceiling",
        _ => return Err("unknown workload".into()),
    };
    let c = source::cases()
        .into_iter()
        .find(|c| c.label == name)
        .ok_or("committed workload")?;
    let (mut sim, a, things) = source::setup(&c);
    let mut peak = 0;
    let (result, time) = interval(|| -> Result<(), SimError> {
        for op in 1..=c.tics {
            source::controlled(&mut sim, &c, a, &things, op)?;
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
        "player_heights label={label} mode={mode} calls={} warmup=0 peak_actors={peak} ns={} digest={:016x}",
        c.tics,
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
