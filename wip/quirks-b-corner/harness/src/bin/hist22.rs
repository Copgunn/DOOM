//! Does the public direct `Simulation::line_attack` / `aim_line_attack` accept a wire-22..25
//! simulation? If so, it uses the endpoint-requiring collector. Player body from THINGS.
//!     hist22 WAD FINE
use doom_sim::*;
use doom_types::Fixed;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let fine: u32 = a[1].parse()?;
    for wire in [22u8, 23, 24, 25] {
        let bytes = std::fs::read(&a[0])?;
        let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default())?;
        let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default())?;
        let ctx = MapContextOptions::default();
        let mut s = match wire {
            22 => Simulation::original_player_level_map(map, ctx)?,
            23 => Simulation::original_player_episode1_map(map, ctx)?,
            24 => Simulation::original_player_episode1_v2_map(map, 1, Skill::Medium, ctx)?,
            _ => Simulation::original_player_episode1_v3_map(map, 1, Skill::Medium, ctx)?,
        };
        let report = s.spawn_map_things()?;
        let body = report.player_body.ok_or("no body")?;
        let before = s.snapshot();
        let aim = s.aim_line_attack(body, fine << 19, Fixed::from_map_units(2048)).map(|r| r.trace.cells.len());
        let la = s.line_attack(body, LineAttack { angle: fine << 19, range: Fixed::from_map_units(2048), slope: Fixed::ZERO, damage: 0 }).map(|r| r.trace.cells.len());
        println!("wire {wire}: aim_line_attack {aim:?}; line_attack {la:?}; snapshot unchanged {}", before.canonical_bytes() == s.snapshot().canonical_bytes());
    }
    Ok(())
}
