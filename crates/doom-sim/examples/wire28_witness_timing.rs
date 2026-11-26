//! Wire 27 against wire 28 replay time (section 1.17 condition 3): reported, not gated.
//! Same level and demo commands, no capture or hashing in the timed region; runs alternate
//! A B A B after one untimed warm-up of each. Run alone on an idle machine.
use doom_sim::{
    LevelStart, MapContextOptions, Simulation, Skill, TicCommand, demo::parse_demo_episode1_v2,
};
use std::{error::Error, fs, hint::black_box, time::Instant};

type HostResult<T> = Result<T, Box<dyn Error>>;
const USAGE: &str = "wire28_witness_timing --wad PATH --map E1Mn --demo PATH [--runs N=20] \
    (demo skill 2 or 3; wire 27 = DSIM43, no witness; wire 28 = DSIM44)";

fn level(wad: &[u8], map: &str, n: u8, skill: Skill, wire28: bool) -> HostResult<Simulation> {
    let wad = doom_wad::Wad::parse(wad, doom_wad::Limits::default())?;
    let map = doom_map::Map::from_wad(&wad, map, doom_map::MapLimits::default())?;
    let build = if wire28 {
        Simulation::original_player_episode1_v6_level
    } else {
        Simulation::original_player_episode1_v5_level
    };
    let context = MapContextOptions::default();
    Ok(build(map, n, skill, context, LevelStart::NewGame)?)
}

fn main() -> HostResult<()> {
    let (mut wad, mut demo, mut map, mut runs) = (None, None, "E1M1".to_string(), 20usize);
    let mut input = std::env::args().skip(1);
    while let Some(flag) = input.next() {
        let value = input.next().ok_or(USAGE)?;
        match flag.as_str() {
            "--wad" => wad = Some(fs::read(value)?),
            "--demo" => demo = Some(fs::read(value)?),
            "--map" => map = value,
            "--runs" => runs = value.parse::<usize>()?.max(1),
            _ => return Err(USAGE.into()),
        }
    }
    let (wad, demo) = (wad.ok_or(USAGE)?, demo.ok_or(USAGE)?);
    let n = match map.as_bytes() {
        [b'E', b'1', b'M', n @ b'1'..=b'9'] => n - b'0',
        _ => return Err("--map must be E1M1..E1M9".into()),
    };
    let (header, demo) = parse_demo_episode1_v2(&demo, n)?;
    let skill = match header.skill {
        2 => Skill::Medium,
        3 => Skill::Hard,
        other => return Err(format!("demo skill {other}: both wires admit only 2 or 3").into()),
    };
    let quiet = TicCommand::default();
    let cmds: Vec<_> = demo
        .commands
        .iter()
        .map(|c| [*c, quiet, quiet, quiet])
        .collect();
    let (mut times, mut ends) = ([vec![], vec![]], [None, None]);
    for round in 0..=runs {
        for w in [false, true] {
            let mut sim = level(&wad, &map, n, skill, w)?;
            let start = Instant::now();
            for command in &cmds {
                black_box(sim.advance(black_box(*command)))?;
            }
            let elapsed = start.elapsed().as_nanos();
            if round > 0 {
                times[usize::from(w)].push(elapsed);
            }
            ends[usize::from(w)] = Some(sim);
        }
    }
    // Outside the timed region: the same game under both wires; only the names may differ.
    let [Some(a), Some(b)] = ends else {
        unreachable!()
    };
    let (fa, fb) = (a.snapshot(), b.snapshot());
    let differ: Vec<_> = (fa.fields().zip(fb.fields()))
        .filter(|(x, y)| (x.path(), x.value()) != (y.path(), y.value()))
        .map(|(x, _)| x.path().to_string())
        .collect();
    println!(
        "equality: tics {} {}, rng equal {}, differing fields {differ:?}",
        fa.tic(),
        fb.tic(),
        a.rng() == b.rng()
    );
    let named = differ == ["profile", "context.schema"];
    assert!(
        fa.tic() == fb.tic() && a.rng() == b.rng() && named,
        "the wires diverged"
    );
    println!("witness checks per run: not counted (no counter in the library)");
    let (tics, mut median) = (cmds.len() as f64, [0.0; 2]);
    for (i, name) in ["wire 27", "wire 28"].iter().enumerate() {
        times[i].sort_unstable();
        median[i] = times[i][runs / 2] as f64;
        let (min, max) = (times[i][0], times[i][runs - 1]);
        println!(
            "{name}: runs {runs} tics {tics} ns/run min {min} median {} max {max}; {:.1} ns/tic",
            median[i],
            median[i] / tics
        );
    }
    let d = median[1] - median[0];
    println!(
        "wire 28 - wire 27: {:.1} ns/tic, {:+.2} %",
        d / tics,
        100.0 * d / median[0]
    );
    Ok(())
}
