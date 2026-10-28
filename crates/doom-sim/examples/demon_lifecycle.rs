//! Original-C fields and independently encoded DSIM-v14; public synthetic maps.
#[path = "../../../tests/support/m3d3g.rs"]
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
    for label in [
        "idle_see_handoff",
        "look_wake",
        "corpse_collision",
        "corpse_ledge_2",
        "retired_source_identity",
        "hitscan_lethal",
        "fall_forward_taken",
        "self_melee_death",
        "pain_then_death",
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
            if label == "retired_source_identity" && op >= 5 {
                assert_eq!(l.target, Some(ActorId::from_parts(1, 1)));
                assert!(s.actor_target(owner()).is_none());
                assert!(s.actor(ActorId::from_parts(1, 2)).is_some());
            }
        }
        if label == "idle_see_handoff" {
            assert!(!all.iter().any(
                |(_, e)| matches!(e,Event::Combat(CombatEvent::Pain{target,..}) if *target==owner())
            ));
            assert!(all.iter().any(|(t, e)| *t == 0 && entry(e, 2)));
        }
        if label == "pain_then_death" {
            let sounds = |sound| {
                all.iter().filter_map(|(t,e)|matches!(e,Event::ActorState(ActorStateEvent::Sound{actor:Some(id),sound:n}) if *id==owner()&&*n==sound).then_some(*t)).collect::<Vec<_>>()
            };
            assert!(all.iter().any(|(t, e)| *t == 0 && entry(e, 7)));
            assert_eq!(sounds(26), [2]);
            assert!(all.iter().any(|(t, e)| *t == 4 && entry(e, 2)));
            assert!(all.iter().any(|(t, e)| *t == 8 && entry(e, 9)));
            let timer = all
                .iter()
                .find_map(|(t, e)| match e {
                    Event::Combat(CombatEvent::DeathTimer { target, tics })
                        if *t == 8 && *target == owner() =>
                    {
                        Some(*tics)
                    }
                    _ => None,
                })
                .unwrap();
            assert!((5..=8).contains(&timer));
            let scream = 8 + timer as usize - 1;
            assert_eq!(sounds(64), [scream]);
            assert!(all.iter().any(|(t, e)| *t == scream + 12 && entry(e, 12)));
            assert!(all.iter().any(|(t, e)| *t == scream + 20 && entry(e, 14)));
            assert_eq!(
                s.actor(owner()).unwrap().life().unwrap().state,
                Some(StateId(14))
            );
            digest = s.snapshot().digest();
        }
    }
    // Expected digest was generated from original-C traces before this example.
    assert_eq!(digest, 0xeecc4b4dcc4fbca5);
    println!("Demon lifecycle: 9 C-backed scenarios; DSIM-v14 {digest:016x}");
}
