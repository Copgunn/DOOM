//! Asserted original-C chronology and independently encoded DSIM-v15.
#[path = "../../../tests/support/m3d3h.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
fn owner() -> ActorId {
    ActorId::from_parts(0, 1)
}
fn entry(e: &Event, state: u16) -> bool {
    matches!(e,Event::ActorState(ActorStateEvent::Entered{actor,state:StateId(s)}) if *actor==owner()&&*s==state)
}
fn main() {
    let cases = fixtures::cases();
    let mut digest = 0;
    let source = ActorRegistry::vanilla();
    let look = source
        .state(source.find_state("S_SARG_STND").unwrap())
        .unwrap()
        .action;
    let chase = source
        .state(source.find_state("S_SARG_RUN1").unwrap())
        .unwrap()
        .action;
    for label in [
        "idle_loop",
        "run_cycle",
        "nested_wake_melee",
        "sound_wake",
        "target_loss_noshoot",
        "no_pain_stand1",
        "no_pain_stand2",
        "retired_source_identity",
        "corpse_collision",
        "stand_run_lifecycle",
    ] {
        let which = cases.iter().position(|c| c.label == label).unwrap();
        let c = &cases[which];
        let mut s = fixtures::setup(c);
        let mut repeat = fixtures::setup(c);
        let mut trace = fixtures::SourceTrace::default();
        trace.check(&s, which, 0, s.pending_events());
        fixtures::assert_golden(&s, which, 0);
        let mut all = s
            .pending_events()
            .iter()
            .cloned()
            .map(|e| (0usize, e))
            .collect::<Vec<_>>();
        for op in 1..=c.v("tics") as usize {
            let pending = s.pending_events().len();
            fixtures::advance(&mut s, c, op);
            fixtures::advance(&mut repeat, c, op);
            assert_eq!(s.snapshot(), repeat.snapshot());
            let events = &s.events()[pending..];
            trace.check(&s, which, op, events);
            fixtures::assert_golden(&s, which, op);
            all.extend(events.iter().cloned().map(|e| (op, e)));
            let a = s.actor(owner()).unwrap();
            let l = a.life().unwrap();
            if l.health <= 0 {
                assert_eq!(a.height(), Fixed::from_map_units(14));
                assert_eq!(l.flags & 4, 0);
                assert_eq!(l.flags & (1048576 | 1024), 1048576 | 1024);
                let state = l.state.unwrap().0;
                assert!((9..=14).contains(&state));
                assert_eq!(l.flags & 2 != 0, state < 12);
                if state == 14 {
                    assert_eq!(l.tics, -1);
                }
            }
            if label == "run_cycle" {
                assert_eq!(l.frame, [0, 0, 1, 1, 2, 2, 3, 3, 0][op / 2]);
            }
            if label == "retired_source_identity" && op >= 5 {
                assert_eq!(l.target, Some(ActorId::from_parts(1, 1)));
                assert!(s.actor_target(owner()).is_none());
                assert!(s.actor(ActorId::from_parts(1, 2)).is_some());
            }
        }
        let entered = |state| {
            all.iter()
                .filter_map(|(t, e)| entry(e, state).then_some(*t))
                .collect::<Vec<_>>()
        };
        let actions = |action| {
            all.iter().filter_map(|(t,e)|matches!(e,Event::ActorState(ActorStateEvent::Action{actor,action:a}) if *actor==owner()&&*a==action).then_some(*t)).collect::<Vec<_>>()
        };
        let sounds = |sound| {
            all.iter().filter_map(|(t,e)|matches!(e,Event::ActorState(ActorStateEvent::Sound{actor:Some(id),sound:n}) if *id==owner()&&*n==sound).then_some(*t)).collect::<Vec<_>>()
        };
        if label == "idle_loop" {
            assert_eq!(entered(3), [10, 30, 50]);
            assert_eq!(entered(1), [20, 40, 60]);
            assert_eq!(actions(look), [10, 20, 30, 40, 50, 60]);
            assert!(actions(chase).is_empty());
        }
        if label == "run_cycle" {
            assert_eq!(actions(chase), (0..=16).step_by(2).collect::<Vec<_>>());
            assert_eq!(actions(chase).into_iter().filter(|t| *t < 16).count(), 8);
            for (i, state) in [2, 15, 16, 17, 18, 19, 20, 21].into_iter().enumerate() {
                assert_eq!(entered(state)[0], i * 2);
            }
        }
        if label == "nested_wake_melee" {
            assert_eq!(actions(look)[0], 10);
            assert_eq!(actions(chase)[0], 10);
            assert_eq!(entered(2)[0], 10);
            assert_eq!(entered(4)[0], 10);
            assert_eq!(entered(5)[0], 18);
            assert_eq!(entered(6)[0], 26);
            assert_eq!(sounds(41), [10]);
        }
        if label == "sound_wake" {
            assert_eq!(entered(2)[0], 10);
            assert_eq!(sounds(41), [10]);
        }
        if label == "target_loss_noshoot" {
            assert!(entered(1).contains(&6));
            assert!(entered(3).contains(&16));
            assert!(entered(2).contains(&36));
        }
        if label.starts_with("no_pain_stand") {
            assert!(!all.iter().any(|(t, e)| *t == 0 && entry(e, 7)));
            assert_eq!(
                all.iter().any(|(t, e)| *t == 0 && entry(e, 2)),
                label == "no_pain_stand1"
            );
        }
        if label == "stand_run_lifecycle" {
            assert!(entered(7).contains(&45));
            assert_eq!(sounds(26), [46]);
            assert!(entered(2).contains(&48));
            assert!(entered(9).contains(&75));
            let timer = all
                .iter()
                .find_map(|(t, e)| match e {
                    Event::Combat(CombatEvent::DeathTimer { target, tics })
                        if *t == 75 && *target == owner() =>
                    {
                        Some(*tics)
                    }
                    _ => None,
                })
                .unwrap();
            assert!((5..=8).contains(&timer));
            let scream = 75 + timer as usize - 1;
            assert_eq!(sounds(64), [scream]);
            assert!(entered(12).contains(&(scream + 12)));
            assert!(entered(14).contains(&(scream + 20)));
            assert_eq!(
                s.actor(owner()).unwrap().life().unwrap().state,
                Some(StateId(14))
            );
            digest = s.snapshot().digest();
        }
    }
    // Derived from original-C input/trace/schema before compiling this example.
    assert_eq!(digest, 0x51463158173610c5);
    println!("Demon stand/run: 10 C-backed scenarios; DSIM-v15 {digest:016x}");
}
