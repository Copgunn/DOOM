//! Original monsters at real-map scale: source scenarios and separate measurements.
#[path = "../../../tests/support/m3h3.rs"]
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
    // Committed source rows really reach the far walls, slide along them and
    // keep more fireballs alive than the historical cap of 24 admitted.
    let (mut launches, mut slides, mut far) = (0, 0, 0);
    for c in source::cases() {
        for op in 0..=c.tics {
            for (tag, row) in source::rows(c.which, op) {
                launches += usize::from(tag == "MS" && row[0] == 5);
                slides += usize::from(tag == "SL");
                far += usize::from(tag == "A" && row[1] == 1 && row[3].abs() > 994 * 65536);
            }
        }
    }
    assert_eq!([launches, slides], [60, 339]);
    assert!(
        far > 0,
        "captured actor fields beyond the historical 994 margin"
    );
    assert_eq!(MAP_SCALE_MAX_LIVE_PROJECTILES, 192);
    assert_eq!(MAP_SCALE_ACTOR_SLOTS, 256);
    println!(
        "player_map_scale: C monsters, fireballs, slides and impulses at walls across rooms up to 2048 units, DSIM30: source/schema/policy checks complete digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step", "capture"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    // Full scripted timelines; a whole run is one measured batch.
    let name = match label {
        "stall_hit" => "stall_fireball_hit_near_wall",
        "stall_past_994" => "stall_fireball_past_994",
        "wall_slide" => "wall_slide_under_fire",
        "long_flight" => "long_fireball_flight",
        "traversal" => "full_room_traversal",
        "eight_imps_far" => "eight_imps_far_room",
        "corner_fight" => "mixed_roster_corner_fight",
        "drops_far" => "drops_far_from_origin",
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
        "player_map_scale label={label} mode={mode} calls={} warmup=0 peak_actors={peak} ns={} digest={:016x}",
        c.tics,
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
