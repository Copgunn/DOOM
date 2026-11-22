//! Older wires (7..21) on one corner scene through the public API: construct the wire's
//! simulation from the WAD's map, spawn what the wire admits, place the player body at the
//! scene start, turn it on tic 1 to the genuine run's angle, then press ATTACK (or USE) on the
//! genuine run's tic. Prints per tic: rng.gameplay, the actor count and the non-actor thinker
//! count; on a refused tic, the error; at the end every `*.health` field.
//!
//!     wires WIRE WAD X Y TURN16 fire|use
use doom_sim::{identity::*, *};
use doom_types::Fixed;

fn field<'a>(snap: &'a Snapshot, path: &str) -> &'a [u8] {
    snap.fields().find(|f| f.path() == path).map_or(&[], StateField::value)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let wire: u32 = a[0].parse()?;
    let bytes = std::fs::read(&a[1])?;
    let pos = [a[2].parse::<i16>()?, a[3].parse::<i16>()?];
    let turn: i16 = a[4].parse::<i32>()? as i16;
    let action = a[5].as_str();
    let wad = doom_wad::Wad::parse(&bytes, doom_wad::Limits::default())?;
    let map = doom_map::Map::from_wad(&wad, "E1M1", doom_map::MapLimits::default())?;
    let ctx = MapContextOptions::default();
    let mut s = match wire {
        7 => Simulation::original_player_demon_pistol_map(map, 64, ctx)?,
        8 => Simulation::original_player_demon_weapons_map(map, 64, ctx)?,
        9 => Simulation::original_player_pickups_map(map, 64, ctx)?,
        10 => Simulation::original_player_armor_map(map, 64, ctx)?,
        11 => Simulation::original_player_bonuses_map(map, 64, ctx)?,
        12 => Simulation::original_player_monsters_map(map, 64, ctx)?,
        13 => Simulation::original_player_imps_map(map, 64, ctx)?,
        14 => Simulation::original_player_map_scale_map(map, ctx)?,
        15 => Simulation::original_player_heights_map(map, ctx)?,
        16 => Simulation::original_player_diagonal_map(map, ctx)?,
        17 => Simulation::original_player_real_map_map(map, ctx)?,
        18 => Simulation::original_player_specials_map(map, ctx)?,
        19 => Simulation::original_player_movers_map(map, ctx)?,
        20 => Simulation::original_player_e1m1_map(map, ctx)?,
        21 => Simulation::original_player_barrels_map(map, ctx)?,
        _ => return Err("wire 7..21".into()),
    };
    if wire <= 8 {
        match s.spawn_original_actor(OriginalActorType::MT_SERGEANT) {
            Ok(r) => println!("setup spawn_original_actor spawned {}", r.spawned.len()),
            Err(e) => println!("setup spawn_original_actor error {e:?}"),
        }
    } else {
        match s.spawn_map_things() {
            Ok(r) => println!("setup spawn_map_things spawned {}", r.spawned.len()),
            Err(e) => println!("setup spawn_map_things error {e:?}"),
        }
    }
    s.spawn_player_body(PlayerSlot::One, [Fixed::from_map_units(pos[0]), Fixed::from_map_units(pos[1])])?;
    if wire == 7 {
        s.initialize_player_pistol(PlayerSlot::One, 50)?;
    } else {
        s.initialize_player_weapons(PlayerSlot::One, WeaponInventory::default())?;
    }
    if wire >= 18 {
        s.spawn_level_specials()?;
    }
    let tics = if action == "fire" { 71 } else { 46 };
    let line = |s: &Simulation| {
        let snap = s.snapshot();
        let rng = field(&snap, "rng.gameplay").first().copied().unwrap_or(0);
        let actors = field(&snap, "thinker.order").len() / 12;
        let shared = field(&snap, "thinker.shared_order").len() / 13;
        let mut pos: Vec<(i32, i32)> = Vec::new();
        let mut xs = std::collections::BTreeMap::new();
        for f in snap.fields() {
            let path = f.path();
            let parts: Vec<&str> = path.split('.').collect();
            if parts.len() == 3 && parts[0] == "actor" && (parts[2] == "x" || parts[2] == "y") && f.value().len() == 4 {
                let v = i32::from_le_bytes([f.value()[0], f.value()[1], f.value()[2], f.value()[3]]);
                xs.entry(parts[1].to_string()).or_insert([0i32; 2])[usize::from(parts[2] == "y")] = v;
            }
        }
        for (_, v) in xs { pos.push((v[0], v[1])); }
        pos.sort();
        let ps: Vec<String> = pos.iter().map(|(x, y)| format!("{x},{y}")).collect();
        format!("{} {} {} {} {}", snap.tic(), rng, actors, shared, ps.join(";"))
    };
    println!("tic {}", line(&s));
    for t in 0..tics {
        let mut c = [TicCommand::default(); 4];
        if t == 0 {
            c[0].angle_turn = turn;
        }
        if action == "fire" && t == 30 {
            c[0].buttons = 1;
        }
        if action == "use" && t == 5 {
            c[0].buttons = 2;
        }
        if let Err(e) = s.advance(c) {
            println!("refused {} {e:?}", t + 1);
            break;
        }
        println!("tic {}", line(&s));
    }
    let snap = s.snapshot();
    for f in snap.fields().filter(|f| f.path().ends_with(".health")) {
        println!("health {} {:?}", f.path(), f.value());
    }
    Ok(())
}
