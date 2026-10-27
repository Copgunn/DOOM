//! Display-free synthetic headless runner; no IWADs, clocks or rendering APIs.
use doom_map::{Map, MapLimits};
use doom_sim::{ActorSpec, Simulation, TicCommand, run_commands};
use doom_types::Fixed;
use doom_wad::{Limits, Wad};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tics = std::env::args()
        .nth(1)
        .map_or(Ok(12), |s| s.parse::<usize>())?;
    let wad = Wad::parse(
        include_bytes!("../../../fuzz/corpus/map/classic_square"),
        Limits::default(),
    )?;
    let map = Map::from_wad(&wad, "MAP01", MapLimits::default())?;
    let mut simulation = Simulation::new(map)?;
    simulation.spawn(ActorSpec {
        momentum: [Fixed::ONE, Fixed::ZERO],
        ..Default::default()
    })?;
    let trace = run_commands(
        &mut simulation,
        (0..tics).map(|_| [TicCommand::default(); 4]),
    )?;
    for snapshot in trace {
        println!("tic={}, fnv1a64={:016x}", snapshot.tic(), snapshot.digest());
    }
    Ok(())
}
