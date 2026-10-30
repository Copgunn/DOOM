//! Bounded host workloads and frozen ownership input scripts, not new C adapters.
#![allow(dead_code)]
// Frozen historical helpers recursively reuse source files. Keep their bytes intact.
#![allow(clippy::duplicate_mod)]
#[path = "m3f3.rs"]
pub mod expanded;
#[path = "m3f3_clipping.rs"]
mod high_grid;
#[path = "m3f8.rs"]
pub mod ownership;
use doom_sim::*;
use doom_types::Fixed;
use ownership::{BODY, OWNER, id};
use std::{error::Error, io::Write};
pub type HostResult<T> = Result<T, Box<dyn Error>>;
pub const UPDATES: usize = 100;

// Only the stepping entry point differs from the frozen input script. C fields,
// identity projection and expected diagnostics remain in the existing helpers.
pub fn ownership_tick(sim: &mut Simulation, which: usize, c: &ownership::Case, op: usize) {
    sim.with_execution_context(|ctx| {
        match (which, op) {
            (0, 1) => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                let body = ctx.admit_local(id(2), BODY)?;
                ctx.set_target(owner, Some(body))?;
            }
            (0, 2) => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                ctx.set_target(owner, None)?;
            }
            (1, 1) => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                let body = ctx.admit_local(id(2), BODY)?;
                ctx.damage(owner, None, Some(body), 1)?;
            }
            (2, 1) => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                let body = ctx.admit_local(id(1), BODY)?;
                ctx.damage(body, None, Some(owner), 1000)?;
            }
            (3 | 7, 1) => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                let body = ctx.admit_local(id(1), BODY)?;
                ctx.damage(owner, None, Some(body), 150)?;
            }
            (4 | 5, 1) => {
                let owner = ctx.admit_local(id(0), OWNER)?;
                ctx.set_target(owner, None)?;
                ctx.set_perception_player(0, None, false, c.v("p0health"))?;
            }
            (5, 2) => {
                let body = ctx.spawn_body(
                    [Fixed::from_bits(c.v("p0x")), Fixed::from_bits(c.v("p0y"))],
                    SpawnHeight::Absolute(Fixed::from_bits(c.v("p0z"))),
                )?;
                ctx.set_perception_player(0, Some(body), true, 100)?;
                let owner = ctx.admit_local(id(0), OWNER)?;
                ctx.set_target(owner, Some(body))?;
            }
            (7, 2) => {
                let body = ctx.admit_local(id(1), BODY)?;
                ctx.set_perception_player(0, None, false, c.v("p0health"))?;
                ctx.mark_remove(body)?;
            }
            _ => {}
        }
        ctx.advance([TicCommand::default(); 4])?;
        Ok(())
    })
    .unwrap();
}

/// Two externally scripted damage requests at most; setup is outside all timers.
pub struct Workload {
    pub sim: Simulation,
    script: Vec<(usize, DamageRequest)>,
}
impl Workload {
    pub fn before_tick(&mut self, tick: usize) -> Result<(), SimError> {
        for &(at, request) in &self.script {
            if at == tick {
                self.sim.damage_actor(request)?;
            }
        }
        Ok(())
    }
    pub fn setup(label: &str, profile: &str) -> HostResult<Self> {
        if profile == "original" {
            if !["canonical_small", "actor_capacity"].contains(&label) {
                return Err("unsupported original-ID benchmark label".into());
            }
            let case = ownership::prior::cases()
                .into_iter()
                .next()
                .ok_or("missing canonical input")?;
            let mut sim = ownership::prior::setup(&case);
            if label == "actor_capacity" {
                let body = sim.perception_players()[0]
                    .actor
                    .ok_or("missing context body")?;
                let position = sim.actor(body).ok_or("missing live body")?.position();
                while sim.actor_order().count() < 64 {
                    sim.spawn_context_body(position, SpawnHeight::Floor)?;
                }
            }
            return Ok(Self {
                sim,
                script: Vec::new(),
            });
        }
        let (case, map) = if label == "high_cell_demon" {
            let c = high_grid::cases()
                .into_iter()
                .find(|c| c.label == label)
                .ok_or("missing grid input")?;
            let map = high_grid::map(usize::try_from(c.v("map"))?);
            (
                expanded::Case {
                    label: c.label,
                    values: c.values,
                },
                map,
            )
        } else {
            let c = expanded::cases()
                .into_iter()
                .find(|c| c.label == label)
                .ok_or("unknown workload label")?;
            let map = expanded::map(usize::try_from(c.v("map"))?);
            (c, map)
        };
        // These measurements intentionally exclude other historical input scripts.
        if case.v("hitscan") != 0
            || ["sound_tick", "retire_tick", "remove_corpse"]
                .iter()
                .any(|key| case.v(key) >= 0)
            || case.v("kill_all") != 0
        {
            return Err("workload requires an unmeasured external script".into());
        }
        let mut sim = match profile {
            "17" => Simulation::with_demon_map_context_registry(
                map,
                64,
                ActorRegistry::demon_map_context_fixture(),
                ActorOptions::default(),
                MapContextOptions::default(),
            )?,
            "20" => Simulation::with_demon_expanded_map_registry(
                map,
                64,
                ActorRegistry::demon_map_context_fixture(),
                ActorOptions::default(),
                MapContextOptions::default(),
            )?,
            _ => return Err("unsupported benchmark profile".into()),
        };
        sim.spawn_map_things()?;
        let sim = expanded::install_targets(sim, &case);
        let mut script = Vec::new();
        for (at, amount) in [("damage_tick", "damage"), ("kill_tick", "kill")] {
            if let Ok(tick) = usize::try_from(case.v(at)) {
                script.push((
                    tick,
                    DamageRequest {
                        target: id(0),
                        source: expanded::damage_source(&sim, case.v("source_kind")),
                        inflictor: expanded::damage_source(&sim, case.v("inflictor_kind")),
                        amount: case.v(amount),
                    },
                ));
            }
        }
        Ok(Self { sim, script })
    }
}

/// Mark only the workload thread. No marker or host time enters Simulation.
pub fn interval<T>(operation: impl FnOnce() -> T) -> std::io::Result<(T, std::time::Duration)> {
    let mut out = std::io::stdout().lock();
    out.flush()?;
    out.write_all(b"DOOM_ALLOC_BEGIN\n")?;
    out.flush()?;
    let start = std::time::Instant::now();
    let result = operation();
    let elapsed = start.elapsed();
    out.write_all(b"DOOM_ALLOC_END\n")?;
    out.flush()?;
    Ok((result, elapsed))
}
