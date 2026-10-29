//! Canonical requests with an explicitly preserved native runtime representation.
#[path = "../../../tests/support/m3f5.rs"]
mod fixtures;
use doom_sim::{identity::*, *};
use std::{hint::black_box, time::Instant};
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--measure") {
        measure();
        return;
    }
    let cases = fixtures::cases();
    for which in [1, 3] {
        let c = &cases[which];
        let mut sim = fixtures::setup(c);
        let owner = ActorId::from_parts(0, 1);
        let binding = sim.canonical_demon_binding().unwrap();
        assert_eq!(binding.states().count(), 21);
        assert_eq!(binding.original_type(), OriginalActorType::MT_SERGEANT);
        assert_eq!(
            OriginalActorType::MT_SERGEANT.states().raise,
            OriginalState::S_SARG_RAISE1
        );
        assert_eq!(binding.runtime_definition().properties[22], 0);
        assert!(
            sim.actor_registry()
                .unwrap()
                .original_actor_type(sim.actor(owner).unwrap().life().unwrap().actor_type)
                .is_err()
        );
        fixtures::assert_golden(&sim, which, 0);
        for op in 1..=c.v("tics") as usize {
            fixtures::advance(&mut sim, c, op);
            fixtures::assert_golden(&sim, which, op);
            assert_eq!(
                sim.original_demon_identity(owner).unwrap().0,
                OriginalActorType::MT_SERGEANT
            );
        }
        println!(
            "canonical_demon: {} MT_SERGEANT=12; 21 bound states; Profile20 {:016x}",
            c.label,
            sim.snapshot().digest()
        );
    }
}
fn measure() {
    let cases = fixtures::cases();
    let c = &cases[0];
    let mut parse = std::time::Duration::ZERO;
    let mut prepare = parse;
    let mut spawn = parse;
    let mut enter = parse;
    for _ in 0..100 {
        let now = Instant::now();
        let map = fixtures::map(0);
        parse += now.elapsed();
        let now = Instant::now();
        let mut sim = Simulation::canonical_demon_map(
            map,
            64,
            ActorOptions::default(),
            MapContextOptions::default(),
        )
        .unwrap();
        prepare += now.elapsed();
        let now = Instant::now();
        black_box(
            sim.spawn_original_demons(OriginalActorType::MT_SERGEANT)
                .unwrap(),
        );
        spawn += now.elapsed();
        let mut sim = fixtures::install_targets(sim, c);
        let owner = ActorId::from_parts(0, 1);
        sim.set_actor_target(owner, Some(ActorId::from_parts(1, 1)))
            .unwrap();
        let now = Instant::now();
        sim.enter_original_demon_state(owner, OriginalState::S_SARG_ATK2)
            .unwrap();
        enter += now.elapsed();
    }
    let mut sim = fixtures::setup(c);
    let owner = ActorId::from_parts(0, 1);
    let now = Instant::now();
    for _ in 0..1000 {
        black_box(sim.original_demon_identity(owner).unwrap());
    }
    let query = now.elapsed();
    let now = Instant::now();
    for op in 1..=100 {
        black_box(sim.original_demon_identity(owner).unwrap());
        fixtures::advance(&mut sim, c, op);
    }
    let step = now.elapsed();
    println!(
        "workload=canonical_api_small parse100_us={} prepare100_us={} spawn100_us={} enter100_us={} query1000_us={} step100_with_query_us={} snapshot_bytes={}",
        parse.as_micros(),
        prepare.as_micros(),
        spawn.as_micros(),
        enter.as_micros(),
        query.as_micros(),
        step.as_micros(),
        sim.snapshot().canonical_bytes().len()
    );
}
