//! Explicit diagnostic capture; optional bounded host measurements are advisory.
#[path = "../../../tests/support/m3g1.rs"]
mod fixture;
use doom_sim::*;
use fixture::{HostResult, UPDATES};
use std::hint::black_box;
const COMMANDS: [TicCommand; 4] = [TicCommand {
    forward: 0,
    side: 0,
    angle_turn: 0,
    buttons: 0,
}; 4];
fn main() -> HostResult<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => demonstrate(),
        [flag, label, profile, mode] if flag == "--measure" => measure(label, profile, mode),
        [flag] if flag == "--control" || flag == "--probe" => {
            let (_, elapsed) = fixture::interval(|| {
                if flag == "--probe" {
                    let bytes = black_box(vec![0_u8; 4096]);
                    black_box(&bytes);
                    drop(bytes);
                } else {
                    black_box(());
                }
            })?;
            println!("control mode={flag} ns={}", elapsed.as_nanos());
            Ok(())
        }
        _ => Err(
            "use --measure LABEL 17|20|original step|advance|advance_snapshot|capture|encode"
                .into(),
        ),
    }
}
fn demonstrate() -> HostResult<()> {
    let case = &fixture::ownership::cases()[0];
    let mut diagnostic = fixture::ownership::setup(0, case);
    let mut production = diagnostic.clone();
    // This loop uses existing checked services and never requests a Snapshot.
    for op in 1..=usize::try_from(case.v("tics"))? {
        fixture::ownership::advance(&mut diagnostic, 0, case, op);
        fixture::ownership_tick(&mut production, 0, case, op);
    }
    // Diagnostics remain available explicitly in both debug and release.
    let actual = production.snapshot();
    assert_eq!(actual, diagnostic.snapshot());
    fixture::ownership::assert_golden(&production, 0, usize::try_from(case.v("tics"))?);
    println!(
        "diagnostic_free: explicit capture; reused C replace_clear {}",
        actual.digest()
    );
    Ok(())
}
fn measure(label: &str, profile: &str, mode: &str) -> HostResult<()> {
    if !["step", "advance", "advance_snapshot", "capture", "encode"].contains(&mode) {
        return Err("unknown measurement mode".into());
    }
    let mut workload = fixture::Workload::setup(label, profile)?;
    let frozen = ["capture", "encode"].contains(&mode);
    if frozen {
        for op in 1..=UPDATES {
            workload.before_tick(op)?;
            workload.sim.advance(COMMANDS)?;
        }
    }
    let encoded = if mode == "encode" {
        Some(workload.sim.snapshot())
    } else {
        None
    };
    let (result, elapsed) = fixture::interval(|| -> HostResult<usize> {
        let mut bytes = 0;
        for op in 1..=UPDATES {
            if !frozen {
                workload.before_tick(op)?;
            }
            match mode {
                "step" => drop(black_box(workload.sim.step(COMMANDS)?)),
                "advance" => workload.sim.advance(COMMANDS)?,
                "advance_snapshot" => {
                    workload.sim.advance(COMMANDS)?;
                    drop(black_box(workload.sim.snapshot()));
                }
                "capture" => drop(black_box(workload.sim.snapshot())),
                "encode" => {
                    let snapshot = encoded.as_ref().ok_or("encoding setup missing")?;
                    bytes += black_box(snapshot.canonical_bytes()).len();
                }
                _ => return Err("unknown measurement mode".into()),
            }
        }
        Ok(bytes)
    })?;
    let bytes = result?;
    // Final verification is outside timers/counters but included in process RSS.
    let snapshot = workload.sim.snapshot();
    println!(
        "stepping label={label} profile={profile} mode={mode} calls={UPDATES} ns={} bytes={bytes} digest={:016x}",
        elapsed.as_nanos(),
        snapshot.digest()
    );
    Ok(())
}
