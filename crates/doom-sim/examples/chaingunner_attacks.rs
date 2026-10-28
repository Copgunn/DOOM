//! Assertions against original C and independent DSIM-v12, using public fixtures.
#[path = "../../../tests/support/m3d3e.rs"]
mod fixtures;
use doom_sim::*;
fn check_shot(events: &[Event]) {
    let Some(start) = events.iter().position(|e| {
        matches!(
            e,
            Event::ActorState(ActorStateEvent::Action {
                action: ActorAction::Original(ActionId(13)),
                ..
            })
        )
    }) else {
        return;
    };
    let events = &events[start..];
    let position = |test: fn(&Event) -> bool| events.iter().position(test).unwrap();
    let sound = position(|e| {
        matches!(
            e,
            Event::ActorState(ActorStateEvent::Sound { sound: 2, .. })
        )
    });
    let face = position(|e| matches!(e, Event::Monster(MonsterEvent::Face { .. })));
    let aim = position(|e| matches!(e, Event::Monster(MonsterEvent::Aim { .. })));
    let shot = position(|e| matches!(e, Event::Combat(CombatEvent::Attack { .. })));
    assert!(sound < face && face < aim && aim < shot);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Monster(MonsterEvent::Aim { .. })))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Combat(CombatEvent::Attack { .. })))
            .count(),
        1
    );
    let draws = events[..shot]
        .iter()
        .filter_map(|e| match e {
            Event::Monster(MonsterEvent::Random { kind, .. }) => Some(*kind),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(draws.ends_with(&[2, 3, 4]));
}
fn main() {
    let cases = fixtures::cases();
    for (label, expected_shots) in [
        ("thinker_hit_then_miss", 3),
        ("thinker_miss_then_hit", 3),
        ("thinker_moving_target", 8),
        ("refire_wall_occluded", 0),
        ("thinker_dead", 2),
        ("thinker_pursue", 9),
        ("thinker_shadow", 20),
        ("thinker_cycles", 31),
    ] {
        let which = cases.iter().position(|c| c.label == label).unwrap();
        let c = &cases[which];
        let mut s = fixtures::setup(c);
        let owner = ActorId::from_parts(0, 1);
        let mut trace = fixtures::SourceTrace::default();
        trace.check(&s, which, 0, s.pending_events());
        fixtures::assert_golden(&s, which, 0);
        check_shot(s.pending_events());
        let initial = s.actor(owner).unwrap().position();
        let mut all = s.pending_events().to_vec();
        let mut shot_tics = if all
            .iter()
            .any(|e| matches!(e, Event::Combat(CombatEvent::Attack { .. })))
        {
            vec![0]
        } else {
            Vec::new()
        };
        let mut moved = false;
        for op in 1..=c.v("tics") as usize {
            let pending = s.pending_events().len();
            fixtures::advance(&mut s, c, op);
            let events = if c.v("mode") == 4 {
                s.events()
            } else {
                s.pending_events()
            };
            let events = &events[pending..];
            trace.check(&s, which, op, events);
            fixtures::assert_golden(&s, which, op);
            check_shot(events);
            if events
                .iter()
                .any(|e| matches!(e, Event::Combat(CombatEvent::Attack { .. })))
            {
                shot_tics.push(s.tic());
            }
            all.extend_from_slice(events);
            moved |= s.actor(owner).unwrap().position() != initial;
        }
        assert_eq!(shot_tics.len(), expected_shots);
        if label.starts_with("thinker_") {
            assert!(shot_tics.windows(2).all(|t| matches!(t[1] - t[0], 4 | 5)));
        }
        let exit = all.iter().position(|e| {
            matches!(
                e,
                Event::Refire(RefireEvent {
                    decision: RefireDecision::DeadTarget | RefireDecision::NotVisible,
                    ..
                })
            )
        });
        if label == "refire_wall_occluded" || label == "thinker_dead" {
            let exit = exit.unwrap();
            assert!(matches!(
                all[exit + 1],
                Event::ActorState(ActorStateEvent::Entered {
                    state: StateId(2),
                    ..
                })
            ));
            assert!(all[exit + 1..].iter().any(|e| matches!(
                e,
                Event::ActorState(ActorStateEvent::Action {
                    action: ActorAction::Original(ActionId(15)),
                    ..
                })
            )));
        }
        if label == "thinker_hit_then_miss" || label == "thinker_miss_then_hit" {
            let hits = all
                .iter()
                .filter_map(|e| match e {
                    Event::Combat(CombatEvent::Trace { result }) => Some(result.hit.is_some()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                &hits[..2],
                if label == "thinker_hit_then_miss" {
                    &[true, false]
                } else {
                    &[false, true]
                }
            );
        }
        if label == "thinker_moving_target" {
            let angles = all
                .iter()
                .filter_map(|e| match e {
                    Event::Monster(MonsterEvent::Face { angles, .. }) => Some(angles[1]),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_ne!(angles[0], angles[1]);
        }
        if label == "thinker_pursue" {
            assert!(moved);
        }
        if label == "thinker_cycles" {
            assert!(s.actor(owner).unwrap().life().unwrap().target.is_some());
            assert!(
                all.iter()
                    .any(|e| matches!(e, Event::Monster(MonsterEvent::Selection { kind: 1, .. })))
            );
            assert!(all.iter().any(|e| matches!(
                e,
                Event::ActorState(ActorStateEvent::Entered {
                    state: StateId(4),
                    ..
                })
            )));
            assert!(
                all.iter()
                    .filter(|e| matches!(
                        e,
                        Event::Refire(RefireEvent {
                            decision: RefireDecision::VisibleContinue
                                | RefireDecision::EarlyContinue,
                            ..
                        })
                    ))
                    .count()
                    > 2
            );
            assert!(
                all.iter()
                    .any(|e| matches!(e, Event::Combat(CombatEvent::Health { .. })))
            );
            assert!(all.iter().any(|e| matches!(e, Event::Retired(_))));
            assert_eq!(s.snapshot().digest(), 0x54ac5f9c158c8e29);
            println!("DSIM-v12 final {:016x}", s.snapshot().digest());
        }
        println!(
            "{label}: {} tics, {expected_shots} individually aimed shots; original-C chronology and independent DSIM-v12 match",
            s.tic()
        );
    }
}
