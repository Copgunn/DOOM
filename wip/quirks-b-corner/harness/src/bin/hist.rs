//! Historical endpoint-requiring hitscan (`combat_trace::collect`, require_endpoint) through the
//! public `Simulation::aim_line_attack` / `line_attack` of the profile-5 combat simulation (Simulation::combat), on a
//! corner scene: a shooter at X Y, the genuine run's angle (fine 1248 << 19), range 2048, slope 0.
//! Prints each result and whether the canonical snapshot is unchanged by a refused attack.
//!
//!     hist WAD X Y FINE
use doom_sim::*;
use doom_types::Fixed;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&a[0])?;
    let pos = [a[1].parse::<i16>()?, a[2].parse::<i16>()?];
    let fine: u32 = a[3].parse()?;
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default())?;
    let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default())?;
    let mut s = Simulation::combat(map).map_err(|e| format!("construct {e:?}"))?;
    let shooter = s.spawn_type(
        ActorTypeId(0),
        [Fixed::from_map_units(pos[0]), Fixed::from_map_units(pos[1])],
        SpawnHeight::Floor,
    ).map_err(|e| format!("spawn {e:?}"))?;
    let angle = fine << 19;
    let range = Fixed::from_map_units(2048);
    let before = s.snapshot();
    match s.aim_line_attack(shooter, angle, range) {
        Ok(r) => println!("aim_line_attack ok cells {} target {:?}", r.trace.cells.len(), r.target),
        Err(e) => println!("aim_line_attack err {e:?}"),
    }
    let r = s.line_attack(shooter, LineAttack { angle, range, slope: Fixed::ZERO, damage: 0 });
    let after = s.snapshot();
    match r {
        Ok(r) => println!("line_attack ok cells {} distinct-last {:?}", r.trace.cells.len(), r.trace.cells.last()),
        Err(e) => println!("line_attack err {e:?}"),
    }
    println!("snapshot unchanged {}", before.canonical_bytes() == after.canonical_bytes());
    Ok(())
}
