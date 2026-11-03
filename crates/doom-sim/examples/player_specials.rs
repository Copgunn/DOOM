//! Original monsters with sky ceilings and static sector specials: source scenarios
//! and separate measurements.
#[path = "../../../tests/support/m3h7.rs"]
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
    if let [flag, label] = args.as_slice() {
        if flag == "--admit" {
            return admit(label);
        }
    }
    if !args.is_empty() {
        return Err("use --measure workload advance|step|capture, or --admit pools|sky".into());
    }
    let digest = source::verify();
    // Headline counts of the committed C rows (m3h7_contract.json inventory): light
    // changes per kind (LC kind 1 flash, 2 strobe, 3 glow, 4 flicker), the
    // environmental damage calls (G code 2), secrets (SE), sky removals (KR), sky
    // shots by reason (KS code 0 above the front ceiling, 1 back sector also sky),
    // P_PlayerInSpecialSector entries (SP) and light thinkers at load (TK op 0).
    let (mut changes, mut shots) = ([0usize; 5], [0usize; 2]);
    let (mut damage, mut secrets, mut removals, mut calls, mut lights) = (0, 0, 0, 0, 0);
    // The same observations from the Rust run: DSIM34 tag-25 events.
    let (mut rust_shots, mut rust_secrets, mut rust_removals) = ([0usize; 2], 0, 0);
    for c in source::cases() {
        for op in 0..=c.tics {
            let mut entered = false;
            for (tag, row) in source::rows(c.which, op) {
                match tag.as_str() {
                    "LC" => changes[usize::try_from(row[0])?] += 1,
                    // Null-source damage after this op's SP entry (the controlled
                    // between-tic damage requests precede it).
                    "G" => damage += usize::from(entered && row[0] == 2 && row[2] == -1),
                    "SE" => secrets += 1,
                    "KR" => removals += 1,
                    "KS" => shots[usize::try_from(row[0])?] += 1,
                    "SP" => {
                        calls += 1;
                        entered = true;
                    }
                    "TK" => lights += usize::from(op == 0 && (1..=4).contains(&row[1])),
                    _ => {}
                }
            }
        }
        let (mut s, a, things) = source::setup(&c);
        for op in 1..=c.tics {
            source::controlled(&mut s, &c, a, &things, op)?;
            s.advance(source::commands(&c, op))?;
            for event in s.events() {
                match event {
                    Event::Special(SpecialEvent::SkyShot { reason, .. }) => {
                        rust_shots[usize::from(*reason)] += 1;
                    }
                    Event::Special(SpecialEvent::Secret { .. }) => rust_secrets += 1,
                    Event::Special(SpecialEvent::SkyRemoval { .. }) => rust_removals += 1,
                    _ => {}
                }
            }
        }
    }
    assert_eq!(changes, [0, 156, 1001, 1897, 218]);
    assert_eq!(
        [damage, secrets, removals, calls, lights],
        [20, 1, 8, 689, 56]
    );
    assert_eq!(shots, [4, 4]);
    assert_eq!(
        (rust_shots, rust_secrets, rust_removals),
        (shots, secrets, removals)
    );
    assert_eq!(
        OriginalExecutionProfile::PlayerMonstersSpecialsV1.wire_id(),
        18
    );
    let pools = source::case_map(0);
    assert_eq!(pools.linedefs().len(), 52);
    assert_eq!(pools.sectors().len(), 15);
    println!(
        "player_specials: C pools and sky rooms, 17 scenarios, light changes flash/strobe/glow/flicker 156/1001/1897/218, 20 environmental damage calls, 1 secret, 8 sky removals, 4+4 sky shots, DSIM34: source/schema/policy checks complete digest={digest:016x}"
    );
    Ok(())
}
fn measure(label: &str, mode: &str) -> ResultHost<()> {
    if !["advance", "step", "capture"].contains(&mode) {
        return Err("unknown stepping mode".into());
    }
    // Full scripted timelines; a whole run is one measured batch.
    if ![
        "pools_lights_soak",
        "pools_light_and_fireball",
        "sky_hack_wall",
        "sky_window_order",
    ]
    .contains(&label)
    {
        return Err("unknown workload".into());
    }
    let c = source::cases()
        .into_iter()
        .find(|c| c.label == label)
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
        "player_specials label={label} mode={mode} calls={} warmup=0 peak_actors={peak} ns={} digest={:016x}",
        c.tics,
        time.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
/// Load time of a committed room: the parsed map is built outside the timer; the
/// policy's admission, the THING and body spawns and `spawn_level_specials` inside
/// it (`specials_ns` is the last of these alone).
fn admit(label: &str) -> ResultHost<()> {
    let name = match label {
        "pools" => "pools_light_and_fireball",
        "sky" => "sky_hack_wall",
        _ => return Err("unknown room".into()),
    };
    let c = source::cases()
        .into_iter()
        .find(|c| c.label == name)
        .ok_or("committed room")?;
    let map = source::case_map(c.map);
    let (lines, sectors, cells) = (
        map.linedefs().len(),
        map.sectors().len(),
        map.blockmap().cell_ranges().len(),
    );
    let (result, time) = interval(|| -> Result<_, Box<dyn Error>> {
        let mut sim = Simulation::original_player_specials_map(
            black_box(map),
            MapContextOptions {
                initial_rng: doom_types::DoomRng::from_indices(c.seed, 0),
                respawn_monsters: false,
            },
        )?;
        sim.spawn_map_things()?;
        sim.spawn_player_body(
            PlayerSlot::One,
            c.position.map(doom_types::Fixed::from_map_units),
        )?;
        let start = Instant::now();
        sim.spawn_level_specials()?;
        Ok((sim, start.elapsed()))
    })?;
    let (sim, specials) = result?;
    let thinkers = sim
        .shared_thinker_order()
        .filter(|t| t.sector().is_some())
        .count();
    println!(
        "player_specials admit={label} linedefs={lines} sectors={sectors} cells={cells} light_thinkers={thinkers} ns={} specials_ns={} digest={:016x}",
        time.as_nanos(),
        specials.as_nanos(),
        sim.snapshot().digest()
    );
    Ok(())
}
