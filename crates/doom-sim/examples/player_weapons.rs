//! Asserted C player/weapon traces and independent final DSIM-v6 bytes.
#[path = "../../../tests/support/m3d2.rs"]
mod fixtures;
use doom_sim::*;
fn main() {
    let cases = fixtures::cases();
    for which in [0, 1, 5, 9, 11, 15, 24, 26, 16] {
        let c = &cases[which];
        let mut sim = fixtures::setup(c);
        let expected = fixtures::player_reference(which);
        assert_eq!(fixtures::player_values(&sim), expected[0]);
        fixtures::assert_golden(&sim, which, 0);
        let mut bullets = 0;
        for (tic, command) in fixtures::commands(which).into_iter().enumerate() {
            sim.step(command).unwrap();
            assert_eq!(
                fixtures::player_values(&sim),
                expected[tic + 1],
                "{} tic {}",
                c.label,
                tic + 1
            );
            bullets += sim
                .events()
                .iter()
                .filter(|e| matches!(e, Event::Weapon(WeaponEvent::Bullet { .. })))
                .count();
            if fixtures::has_golden(which, tic + 1) {
                fixtures::assert_golden(&sim, which, tic + 1);
            }
        }
        match which {
            0 => {
                assert_eq!(bullets, 1);
                assert_eq!(sim.weapon_player().unwrap().ammo[0], 49);
            }
            1 => {
                assert!(bullets > 1);
                assert!(sim.weapon_player().unwrap().refire > 0);
            }
            5 => {
                assert_eq!(bullets, 7);
                assert_eq!(sim.weapon_player().unwrap().ammo[1], 7);
            }
            9 => assert_eq!(sim.weapon_player().unwrap().ready, Weapon::Shotgun),
            11 => assert_eq!(
                sim.actor(ActorId::from_parts(1, 1))
                    .unwrap()
                    .life()
                    .unwrap()
                    .health,
                1000
            ),
            15 | 16 => assert!(
                sim.actor(ActorId::from_parts(1, 1))
                    .unwrap()
                    .life()
                    .unwrap()
                    .health
                    <= 0
            ),
            24 => {
                assert_eq!(bullets, 2);
                assert_eq!(sim.weapon_player().unwrap().ammo[0], 48);
            }
            26 => {
                assert_eq!(sim.weapon_player().unwrap().ready, Weapon::Shotgun);
                assert_eq!(sim.weapon_player().unwrap().ammo[0], 0);
            }
            _ => unreachable!(),
        }
        println!(
            "{}: {} tics, {bullets} bullets/pellets, C player/psprite/ammo/RNG assertions passed",
            c.label,
            sim.tic()
        );
        if which == 16 {
            println!("DSIM-v6 final {:016x}", sim.snapshot().digest());
        }
    }
}
