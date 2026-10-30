//! Qualified boundary references; gameplay expectations reuse frozen C evidence.
#[path = "../../../tests/support/m3f9.rs"]
mod fixture;
use doom_sim::*;
use fixture::{
    export,
    fixtures::{BODY, OWNER, id},
};
use std::{hint::black_box, time::Instant};

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--measure") {
        let qualified = match std::env::args().nth(2).as_deref() {
            Some("scoped") => false,
            Some("qualified") => true,
            _ => {
                eprintln!("usage: game_instance --measure scoped|qualified");
                std::process::exit(2);
            }
        };
        measure(qualified);
        return;
    }
    let (counts, snapshot) = fixture::source_case();
    let (mut a, mut b) = (
        GameInstance::new(fixture::setup()),
        GameInstance::new(fixture::setup()),
    );
    let reference = export(&mut a, id(0), OWNER);
    assert_eq!(
        fixture::import(&mut b, &reference, OWNER),
        Err(ProvenanceError::InstanceMismatch)
    );
    fixture::import(&mut a, &reference, OWNER).unwrap();
    a.restart().unwrap();
    assert_eq!(
        fixture::import(&mut a, &reference, OWNER),
        Err(ProvenanceError::InstanceMismatch)
    );
    println!(
        "game_instance: reused C replace_clear captures={} {:016x}",
        counts[4],
        snapshot.digest()
    );
}

fn measure(qualified: bool) {
    let mut game = GameInstance::new(fixture::setup());
    let owner = export(&mut game, id(0), OWNER);
    let body = export(&mut game, id(1), BODY);
    let (imports, exports, updates) = game
        .with_execution_context(|ctx| {
            let owner_local = ctx.import_actor(&owner, OWNER)?;
            let body_local = ctx.import_actor(&body, BODY)?;
            let start = Instant::now();
            for _ in 0..100_000 {
                black_box(ctx.import_actor(black_box(&body), BODY)?);
            }
            let imports = start.elapsed();
            let start = Instant::now();
            for _ in 0..100_000 {
                black_box(ctx.export_actor(black_box(body_local))?);
            }
            let exports = start.elapsed();
            let start = Instant::now();
            for _ in 0..100 {
                let current = if qualified {
                    ctx.import_actor(black_box(&owner), OWNER)?
                } else {
                    owner_local
                };
                black_box(ctx.target(current)?);
                black_box(ctx.step([TicCommand::default(); 4])?);
            }
            Ok((imports, exports, start.elapsed()))
        })
        .unwrap();
    println!(
        "import100000_us={} export100000_us={} step100_us={} digest={:016x}",
        imports.as_micros(),
        exports.as_micros(),
        updates.as_micros(),
        game.simulation().snapshot().digest()
    );
}
