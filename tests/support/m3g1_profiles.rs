//! Bounded API-refinement inputs, reusing historical setups unchanged.
//! Equality here is Rust policy, never a new independent C oracle.
#![allow(dead_code)]
// Frozen historical helpers recursively reuse source files. Keep their bytes intact.
#![allow(clippy::duplicate_mod)]
#[path = "m3d3c.rs"]
mod attacks;
#[path = "m3d3e.rs"]
mod chaingunner;
#[path = "m3d1.rs"]
mod combat;
#[path = "m3f3.rs"]
mod expanded;
#[path = "m3f2.rs"]
mod extent;
#[path = "m3b.rs"]
mod geometry;
#[path = "m3d3g.rs"]
mod lifecycle;
#[path = "m3d3j.rs"]
mod map_context;
#[path = "m3d3i.rs"]
mod map_spawn;
#[path = "m3d3f.rs"]
mod melee;
#[path = "m3f8.rs"]
mod original;
#[path = "m3d3a.rs"]
mod perception;
#[path = "m3d2b.rs"]
mod projectiles;
#[path = "m3d3b.rs"]
mod pursuit;
#[path = "m3f1.rs"]
mod scale;
#[path = "m3d3d.rs"]
mod shotgunner;
#[path = "m3d3h.rs"]
mod stand_run;
#[path = "m3c.rs"]
mod states;
#[path = "m3d2.rs"]
mod weapons;
use doom_sim::*;
use doom_types::Fixed;

pub const PROFILES: [Profile; 21] = [
    Profile::Foundation,
    Profile::PlayerCollision,
    Profile::MultiSector,
    Profile::ActorStates,
    Profile::Combat,
    Profile::PlayerWeapons,
    Profile::Projectiles,
    Profile::Perception,
    Profile::Pursuit,
    Profile::MonsterAttacks,
    Profile::Shotgunner,
    Profile::Chaingunner,
    Profile::DemonMelee,
    Profile::DemonLifecycle,
    Profile::DemonStandRun,
    Profile::DemonMapSpawn,
    Profile::DemonMapContext,
    Profile::MapScale,
    Profile::MapExtent,
    Profile::DemonExpandedMap,
    Profile::OriginalDemon,
];

pub fn setup(profile: Profile) -> Simulation {
    let mut sim = match profile {
        Profile::Foundation => Simulation::new(geometry::map(19)).unwrap(),
        Profile::PlayerCollision => Simulation::player_collision(geometry::map(19)).unwrap(),
        Profile::MultiSector => Simulation::multisector(geometry::map(19)).unwrap(),
        Profile::ActorStates => states::setup(6),
        Profile::Combat => combat::setup(&combat::cases()[0]),
        Profile::PlayerWeapons => weapons::setup(&weapons::cases()[0]),
        Profile::Projectiles => projectiles::setup(&projectiles::cases()[0]),
        Profile::Perception => perception::setup(&perception::cases()[0]),
        Profile::Pursuit => pursuit::setup(&pursuit::cases()[0]),
        Profile::MonsterAttacks => attacks::setup(&attacks::cases()[0]),
        Profile::Shotgunner => shotgunner::setup(&shotgunner::cases()[0]),
        Profile::Chaingunner => chaingunner::setup(&chaingunner::cases()[0]),
        Profile::DemonMelee => melee::setup(&melee::cases()[0]),
        Profile::DemonLifecycle => lifecycle::setup(&lifecycle::cases()[0]),
        Profile::DemonStandRun => stand_run::setup(&stand_run::cases()[0]),
        Profile::DemonMapSpawn => map_spawn::setup(&map_spawn::cases()[0]),
        Profile::DemonMapContext => map_context::setup(&map_context::cases()[0]),
        Profile::MapScale => Simulation::map_scale(scale::map(0)).unwrap(),
        Profile::MapExtent => Simulation::map_extent(extent::map(0)).unwrap(),
        Profile::DemonExpandedMap => expanded::setup(&expanded::cases()[0]),
        Profile::OriginalDemon => original::setup(0, &original::cases()[0]),
    };
    if matches!(
        profile,
        Profile::Foundation
            | Profile::PlayerCollision
            | Profile::MultiSector
            | Profile::MapScale
            | Profile::MapExtent
    ) {
        let actor = sim
            .spawn(ActorSpec {
                radius: Fixed::from_map_units(8),
                height: Fixed::from_map_units(16),
                ..Default::default()
            })
            .unwrap();
        if matches!(
            profile,
            Profile::Foundation | Profile::PlayerCollision | Profile::MultiSector
        ) {
            sim.bind_player(0, actor).unwrap();
        }
    }
    assert_eq!(sim.profile(), profile);
    sim
}
