//! Opt-in private measurement; identical instrumentation also runs on baseline.
use super::tests::fixtures as fixture;
use super::*;
use std::{hint::black_box, time::Instant};
#[test]
fn legacy_weapon_stages_preserve_preview_state_and_report_optional_costs() {
    let cases = fixture::cases();
    for which in [0usize, 5, 23, 24] {
        let commands = fixture::commands(which);
        let mut seed = fixture::setup(&cases[which]);
        // Locate the actual firing boundary, then reconstruct the before world.
        let mut probe = fixture::setup(&cases[which]);
        let warm = commands
            .iter()
            .enumerate()
            .find_map(|(i, c)| {
                probe.advance(*c).unwrap();
                probe
                    .events()
                    .iter()
                    .any(|e| matches!(e, Event::Weapon(WeaponEvent::Bullet { .. })))
                    .then_some(i)
            })
            .expect("source fixture fires");
        for command in commands.iter().take(warm) {
            seed.advance(*command).unwrap();
        }
        let command = commands[warm];
        let before = seed.snapshot();
        seed.preflight_weapon_step(&command).unwrap();
        assert_eq!(seed.snapshot(), before);
        if std::env::var_os("DOOM_WEAPON_STAGE_COST").is_none() {
            continue;
        }
        // Build independent identical worlds outside the measured interval.
        for stage in ["preview", "commit"] {
            let mut worlds: Vec<_> = (0..100)
                .map(|_| {
                    let mut s = fixture::setup(&cases[which]);
                    for c in commands.iter().take(warm) {
                        s.advance(*c).unwrap();
                    }
                    if stage == "commit" {
                        s.preflight_weapon_step(&command).unwrap();
                    }
                    s
                })
                .collect();
            println!("DOOM_ALLOC_BEGIN");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            let start = Instant::now();
            for s in &mut worlds {
                if stage == "preview" {
                    s.preflight_weapon_step(black_box(&command)).unwrap();
                } else {
                    s.weapon_tick(black_box(command));
                }
            }
            let ns = start.elapsed().as_nanos();
            println!("DOOM_ALLOC_END");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            println!(
                "weapon_stage case={which} stage={stage} calls=100 ns={ns} digest={:016x}",
                worlds[0].snapshot().digest()
            );
        }
    }
}
