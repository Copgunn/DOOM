//! Asserted original-C pursuit and independently encoded DSIM-v9 states.
#[path = "../../../tests/support/m3d3b.rs"]
mod fixtures;
use doom_sim::*;
fn main() {
    let cases = fixtures::cases();
    for label in [
        "thinker_acquire_and_pursue",
        "thinker_wall_alternates",
        "thinker_target_loss_spawn",
        "thinker_reacquisition",
    ] {
        let which = cases.iter().position(|c| c.label == label).unwrap();
        let c = &cases[which];
        let mut sim = fixtures::setup(c);
        let mut trace = fixtures::Trace::default();
        let actor = ActorId::from_parts(0, 1);
        trace.check(&sim, which, 0, sim.pending_events());
        fixtures::assert_golden(&sim, which, 0);
        let initial = sim.actor(actor).unwrap().position();
        let mut blocked = false;
        for op in 1..=c.n(13) as usize {
            let pending = sim.pending_events().len();
            let before = sim.actor(actor).unwrap().position();
            fixtures::advance(&mut sim, c, op);
            trace.check(&sim, which, op, &sim.events()[pending..]);
            fixtures::assert_golden(&sim, which, op);
            blocked |= sim.events().iter().any(|e|matches!(e,Event::SectorMovement{actor:a,attempt} if *a==actor&&!attempt.allowed()));
            let a = sim.actor(actor).unwrap();
            let life = a.life().unwrap();
            if op == 1 {
                assert_eq!(life.state, Some(StateId(2)));
                assert_eq!(life.target, Some(ActorId::from_parts(1, 1)));
                assert_ne!(a.position(), initial);
            }
            if op >= 6 && label == "thinker_target_loss_spawn" {
                assert_eq!(life.state, Some(StateId(1)));
                assert_eq!(life.target, Some(ActorId::from_parts(1, 1)));
            }
            if op == 6 && label == "thinker_reacquisition" {
                assert_eq!(life.target, Some(ActorId::from_parts(2, 1)));
                assert_eq!(a.position(), before);
            }
        }
        if label == "thinker_wall_alternates" {
            assert!(blocked);
            assert!(trace.directions > 1);
        }
        println!(
            "{label}: {} tics match C directions/movement/counters/targets/states/RNG and DSIM-v9",
            sim.tic()
        );
        if label == "thinker_reacquisition" {
            println!("DSIM-v9 final {:016x}", sim.snapshot().digest());
        }
    }
}
