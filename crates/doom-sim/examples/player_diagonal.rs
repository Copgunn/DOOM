//! Original monsters among diagonal walls: source scenarios and separate measurements.
#[path = "../../../tests/support/m3h5.rs"]
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
    // Committed source rows really meet walls of every slope: slides, blocked moves
    // at lines, hitscan impacts on walls, noise floods and the one controlled push.
    let (mut slides, mut blocked, mut impacts, mut floods, mut pushes) = (0, 0, 0, 0, 0);
    for c in source::cases() {
        pushes += usize::from(c.push_at != 0);
        for op in 0..=c.tics {
            for (tag, row) in source::rows(c.which, op) {
                slides += usize::from(tag == "SL");
                blocked += usize::from(tag == "M" && row[4] == 0);
                impacts += usize::from(tag == "X" && row[0] == 1 && row[1] == 1);
                floods += usize::from(tag == "N");
            }
        }
    }
    assert_eq!(
        [slides, blocked, impacts, floods, pushes],
        [1272, 1742, 22, 70, 1]
    );
    assert_eq!(source::EXPECTED_ESCAPES, &[("corner_jump", 1)]);
    assert_eq!(
        OriginalExecutionProfile::PlayerMonstersDiagonalV1.wire_id(),
        16
    );
    println!(
        "player_diagonal: C wall slides at every slope, diagonal corners, risers, windows, a pillar and a corridor, the refused corner jump, DSIM32: source/schema/policy checks complete digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step", "capture"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    // Full scripted timelines; a whole run is one measured batch.
    let name = match label {
        "wedge_slide" => "slide_45",
        "pillar_chase" => "imp_around_pillar",
        "corridor_pursuit" => "corridor_pursuit",
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
        "player_diagonal label={label} mode={mode} calls={} warmup=0 peak_actors={peak} ns={} digest={:016x}",
        c.tics,
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
