//! Scoped ownership against compact independent C evidence, plus a separate microbenchmark.
#[path = "../../../tests/support/m3f8.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
use fixtures::{BODY, OWNER, id};
use std::{hint::black_box, time::Instant};
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--measure") {
        measure(std::env::args().nth(2).as_deref() == Some("scoped"));
        return;
    }
    for (which, c) in fixtures::cases().iter().enumerate() {
        let counts = fixtures::run(which, c);
        let mut sim = fixtures::setup(which, c);
        for op in 1..=usize::try_from(c.v("tics")).unwrap() {
            fixtures::advance(&mut sim, which, c, op);
        }
        println!(
            "context_ownership: {} captures={} {:016x}",
            c.label,
            counts[4],
            sim.snapshot().digest()
        );
    }
}
fn measure(scoped: bool) {
    let mut sim = fixtures::setup(0, &fixtures::cases()[0]);
    let (query, set, retire);
    if scoped {
        (query, set, retire) = sim
            .with_execution_context(|ctx| {
                let owner = ctx.admit_local(id(0), OWNER)?;
                let a = ctx.admit_local(id(1), BODY)?;
                let b = ctx.admit_local(id(2), BODY)?;
                let start = Instant::now();
                for _ in 0..100_000 {
                    black_box(ctx.target(black_box(owner))?);
                }
                let query = start.elapsed();
                let start = Instant::now();
                for i in 0..100 {
                    ctx.set_target(owner, Some(if i % 2 == 0 { a } else { b }))?;
                }
                let set = start.elapsed();
                ctx.set_target(owner, Some(a))?;
                ctx.set_perception_player(3, None, false, 100)?;
                let mut body = ctx.admit_local(id(4), BODY)?;
                let start = Instant::now();
                for _ in 0..100 {
                    let old = body;
                    ctx.mark_remove(old)?;
                    black_box(ctx.step([TicCommand::default(); 4])?);
                    body = ctx.spawn_body(
                        [Fixed::from_map_units(-200), Fixed::from_map_units(-96)],
                        SpawnHeight::Floor,
                    )?;
                    assert_eq!(ctx.availability(old), ActorAvailability::StaleGeneration);
                }
                Ok((query, set, start.elapsed()))
            })
            .unwrap();
    } else {
        let start = Instant::now();
        for _ in 0..100_000 {
            black_box(sim.actor_target(black_box(id(0))));
        }
        query = start.elapsed();
        let start = Instant::now();
        for i in 0..100 {
            sim.set_actor_target(id(0), Some(if i % 2 == 0 { id(1) } else { id(2) }))
                .unwrap();
        }
        set = start.elapsed();
        sim.set_actor_target(id(0), Some(id(1))).unwrap();
        sim.set_perception_player(
            3,
            PerceptionPlayer {
                actor: None,
                active: false,
                health: 100,
            },
        )
        .unwrap();
        let mut body = id(4);
        let start = Instant::now();
        for _ in 0..100 {
            let old = body;
            sim.mark_remove(old).unwrap();
            black_box(sim.step([TicCommand::default(); 4]).unwrap());
            body = sim
                .spawn_context_body(
                    [Fixed::from_map_units(-200), Fixed::from_map_units(-96)],
                    SpawnHeight::Floor,
                )
                .unwrap();
            assert_eq!(
                sim.actor_availability(old),
                ActorAvailability::StaleGeneration
            );
        }
        retire = start.elapsed();
    }
    println!(
        "query100000_us={} set100_us={} retirement100_us={} digest={:016x}",
        query.as_micros(),
        set.as_micros(),
        retire.as_micros(),
        sim.snapshot().digest()
    );
}
