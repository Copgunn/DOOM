//! Asserted original-C attack lifecycles with independently encoded DSIM-v10.
#[path = "../../../tests/support/m3d3c.rs"]
mod fixtures;
use doom_sim::*;
fn main() {
    let cases = fixtures::cases();
    for label in [
        "thinker_pursue_then_attack",
        "thinker_shadow_double_face",
        "thinker_attack_cycles",
    ] {
        let which = cases.iter().position(|c| c.label == label).unwrap();
        let c = &cases[which];
        let mut s = fixtures::setup(c);
        let mut trace = fixtures::SourceTrace::default();
        trace.check(&s, which, 0, s.pending_events());
        fixtures::assert_golden(&s, which, 0);
        let owner = ActorId::from_parts(0, 1);
        let initial = s.actor(owner).unwrap().position();
        let mut flags = [false; 10];
        for op in 1..=c.v("tics") as usize {
            let pending = s.pending_events().len();
            fixtures::advance(&mut s, c, op);
            let events = &s.events()[pending..];
            trace.check(&s, which, op, events);
            fixtures::assert_golden(&s, which, op);
            flags[0] |= s.actor(owner).unwrap().life().unwrap().target.is_some();
            flags[1] |= s.actor(owner).unwrap().position() != initial;
            flags[2]|=events.iter().any(|e|matches!(e,Event::Monster(MonsterEvent::Range{decision,..}) if !decision.accepted));
            flags[3] |= events.iter().any(|e| {
                matches!(
                    e,
                    Event::Monster(MonsterEvent::Selection {
                        kind: 1,
                        state: StateId(4),
                        ..
                    })
                )
            });
            flags[4] |= events.iter().any(|e| {
                matches!(
                    e,
                    Event::Monster(MonsterEvent::Face {
                        target: Some(_),
                        ..
                    })
                )
            });
            flags[5] |= events.iter().any(|e| {
                matches!(
                    e,
                    Event::ActorState(ActorStateEvent::Sound { sound: 1, .. })
                )
            });
            flags[6] |= events
                .iter()
                .any(|e| matches!(e, Event::Monster(MonsterEvent::Random { kind: 4, .. })));
            flags[7] |= events
                .iter()
                .any(|e| matches!(e, Event::Combat(CombatEvent::Impact { .. })));
            flags[8] |= s.actor(owner).unwrap().life().unwrap().state == Some(StateId(2));
            flags[9] |= events.iter().any(|e| matches!(e, Event::Retired(_)));
        }
        for (i, flag) in flags.iter().enumerate() {
            if i != 2 {
                assert!(flag, "{label} observation {i}");
            }
        }
        if label == "thinker_pursue_then_attack" {
            assert!(flags[2]);
        }
        println!(
            "{label}: {} tics; source acquisition/walking/selection/facing/sound/RNG/impact/return/retirement and DSIM-v10 match",
            s.tic()
        );
        if label == "thinker_attack_cycles" {
            assert!(
                s.actor(ActorId::from_parts(1, 1))
                    .unwrap()
                    .life()
                    .unwrap()
                    .health
                    < 1000
            );
            println!("DSIM-v10 final {:016x}", s.snapshot().digest());
        }
    }
}
