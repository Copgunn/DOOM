//! Actual original-ID execution and bounded read-only diagnostics.
#[path = "../../../tests/support/m3f7.rs"]
mod fixtures;
use doom_sim::{identity::*, *};
use std::{hint::black_box, time::Instant};
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--measure") {
        measure(std::env::args().nth(2).unwrap().parse().unwrap());
        return;
    }
    let cases = fixtures::cases();
    for which in [1, 3] {
        let c = &cases[which];
        let mut sim = fixtures::setup(c);
        for op in 0..=usize::try_from(c.v("tics")).unwrap() {
            if op != 0 {
                fixtures::advance(&mut sim, c, op);
            }
            fixtures::assert_golden(&sim, which, op);
            let before = sim.snapshot();
            let bytes = before
                .diagnostic_bytes(DiagnosticLimits::default())
                .unwrap();
            let view = Diagnostic::decode(&bytes, DiagnosticLimits::default()).unwrap();
            assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
            assert_eq!(
                sim.original_actor_identity(ActorId::from_parts(0, 1))
                    .unwrap()
                    .0,
                DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT)
            );
            assert_eq!(sim.snapshot(), before);
        }
        println!(
            "original_registry: {} actual MT_SERGEANT12; DSIM21/profile1 {:016x}",
            c.label,
            sim.snapshot().digest()
        );
    }
}
fn measure(which: usize) {
    let c = &fixtures::cases()[which];
    let mut parse = std::time::Duration::ZERO;
    let mut prepare = parse;
    let mut spawn = parse;
    let mut entry = parse;
    for _ in 0..100 {
        let now = Instant::now();
        let map = fixtures::map(usize::try_from(c.v("map")).unwrap());
        parse += now.elapsed();
        let now = Instant::now();
        let mut sim = Simulation::original_id_map(
            map,
            64,
            fixtures::options(c),
            MapContextOptions::default(),
        )
        .unwrap();
        prepare += now.elapsed();
        let now = Instant::now();
        black_box(
            sim.spawn_original_actor(OriginalActorType::MT_SERGEANT)
                .unwrap(),
        );
        spawn += now.elapsed();
        let mut sim = fixtures::install_targets(sim, c);
        let owner = ActorId::from_parts(0, 1);
        sim.set_actor_target(
            owner,
            Some(ActorId::from_parts(
                sim.level_statistics().unwrap().total_kills as u32,
                1,
            )),
        )
        .unwrap();
        let now = Instant::now();
        sim.enter_original_state(owner, OriginalState::S_SARG_ATK2)
            .unwrap();
        entry += now.elapsed();
    }
    let r = OriginalExecutionRegistry::new(OriginalExecutionProfile::DemonNormalV1).unwrap();
    let now = Instant::now();
    for _ in 0..100_000 {
        black_box(
            OriginalActorType::try_from(black_box(OriginalActorType::MT_SERGEANT.ordinal()))
                .unwrap(),
        );
    }
    let conversion = now.elapsed();
    let now = Instant::now();
    for _ in 0..100_000 {
        let definition = black_box(DefinitionIdentity::Original(OriginalActorType::MT_SERGEANT));
        black_box(r.registry().actor_type(r.resolve(definition)).unwrap());
    }
    let lookup = now.elapsed();
    let mut sim = fixtures::setup(c);
    let now = Instant::now();
    for _ in 0..100_000 {
        black_box(
            sim.original_actor_identity(black_box(ActorId::from_parts(0, 1)))
                .unwrap(),
        );
    }
    let query = now.elapsed();
    let now = Instant::now();
    for op in 1..=100 {
        fixtures::advance(&mut sim, c, op);
    }
    let step = now.elapsed();
    let snapshot = sim.snapshot();
    let now = Instant::now();
    let mut bytes = Vec::new();
    for _ in 0..100 {
        bytes = black_box(
            snapshot
                .diagnostic_bytes(DiagnosticLimits::default())
                .unwrap(),
        );
    }
    let encode = now.elapsed();
    let now = Instant::now();
    for _ in 0..100 {
        black_box(Diagnostic::decode(&bytes, DiagnosticLimits::default()).unwrap());
    }
    let decode = now.elapsed();
    println!(
        "workload=original_{} parse100_us={} prepare100_us={} spawn100_us={} enter100_us={} conversion100000_us={} lookup100000_us={} query100000_us={} step100_us={} encode100_us={} decode100_us={} diagnostic_bytes={}",
        c.label,
        parse.as_micros(),
        prepare.as_micros(),
        spawn.as_micros(),
        entry.as_micros(),
        conversion.as_micros(),
        lookup.as_micros(),
        query.as_micros(),
        step.as_micros(),
        encode.as_micros(),
        decode.as_micros(),
        bytes.len()
    );
}
