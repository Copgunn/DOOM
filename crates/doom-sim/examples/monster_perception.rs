//! Asserted source sight/scan/sound/ambush behavior and independent DSIM-v8 bytes.
#[path = "../../../tests/support/m3d3a.rs"]
mod fixtures;
use doom_sim::*;
fn main() {
    let cases = fixtures::cases();
    for label in [
        "scan_spawn_lastlook_three",
        "look_sound_through_wall",
        "look_ambush_blocked",
        "look_ambush_visible",
        "thinker_outside_then_visible",
    ] {
        let which = cases.iter().position(|c| c.label == label).unwrap();
        let c = &cases[which];
        let mut sim = fixtures::setup(c);
        let mut trace = fixtures::Trace::default();
        trace.check(&sim, which, 0, sim.pending_events());
        fixtures::assert_golden(&sim, which, 0);
        if label == "scan_spawn_lastlook_three" {
            assert_eq!(sim.actor(ActorId::from_parts(0, 1)).unwrap().lastlook(), 3);
        }
        if label == "look_sound_through_wall" || label == "look_ambush_blocked" {
            let before = sim.snapshot();
            assert!(
                !sim.check_sight(ActorId::from_parts(0, 1), ActorId::from_parts(1, 1))
                    .unwrap()
                    .visible
            );
            assert_eq!(sim.snapshot(), before);
        }
        for op in 1..=c.n(13) as usize {
            fixtures::flip(&mut sim, c, op);
            let pending = sim.pending_events().len();
            if op == 1 && c.n(11) >= 0 {
                sim.noise_alert(
                    ActorId::from_parts(c.n(11) as u32, 1),
                    ActorId::from_parts(c.n(12) as u32, 1),
                )
                .unwrap();
            }
            match c.n(0) {
                1 => {
                    sim.look_for_players(ActorId::from_parts(0, 1), false)
                        .unwrap();
                    trace.check(&sim, which, op, &sim.pending_events()[pending..]);
                }
                2 => {
                    let woke = sim.look(ActorId::from_parts(0, 1)).unwrap();
                    assert_eq!(woke, label != "look_ambush_blocked");
                    trace.check(&sim, which, op, &sim.pending_events()[pending..]);
                }
                3 => {
                    sim.step([TicCommand::default(); 4]).unwrap();
                    trace.check(&sim, which, op, &sim.events()[pending..]);
                }
                _ => unreachable!(),
            }
            fixtures::assert_golden(&sim, which, op);
            if label == "thinker_outside_then_visible" {
                let life = sim
                    .actor(ActorId::from_parts(0, 1))
                    .unwrap()
                    .life()
                    .unwrap();
                if op == 1 {
                    assert_eq!(life.state, Some(StateId(1)));
                    assert_eq!(life.target, None);
                } else {
                    assert_eq!(life.state, Some(StateId(2)));
                    assert_eq!(life.target, Some(ActorId::from_parts(2, 1)));
                    assert_eq!(sim.actor(ActorId::from_parts(0, 1)).unwrap().lastlook(), 1);
                }
            }
        }
        if label == "scan_spawn_lastlook_three" {
            assert_eq!(sim.actor(ActorId::from_parts(0, 1)).unwrap().lastlook(), 0);
            assert_eq!(
                sim.actor(ActorId::from_parts(0, 1))
                    .unwrap()
                    .life()
                    .unwrap()
                    .target,
                Some(ActorId::from_parts(1, 1))
            );
        }
        if label == "look_ambush_blocked" {
            assert_eq!(
                sim.actor(ActorId::from_parts(0, 1))
                    .unwrap()
                    .life()
                    .unwrap()
                    .state,
                Some(StateId(1))
            );
            assert_eq!(
                sim.actor(ActorId::from_parts(0, 1))
                    .unwrap()
                    .life()
                    .unwrap()
                    .target,
                Some(ActorId::from_parts(1, 1))
            );
        }
        println!(
            "{label}: C sight/scan/lastlook/target/state/RNG assertions passed, {} tics",
            sim.tic()
        );
        if label == "thinker_outside_then_visible" {
            println!("DSIM-v8 final {:016x}", sim.snapshot().digest());
        }
    }
}
