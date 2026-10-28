//! Original-C and independent DSIM-v11 assertions for sequential source blasts.
#[path = "../../../tests/support/m3d3d.rs"]
mod fixtures;
use doom_sim::*;
fn assert_blast(events: &[Event]) {
    let Some(begin) = events
        .iter()
        .position(|e| matches!(e, Event::Shotgun(ShotgunEvent::Begin { .. })))
    else {
        return;
    };
    let events = &events[begin..];
    let sound = events
        .iter()
        .position(|e| {
            matches!(
                e,
                Event::ActorState(ActorStateEvent::Sound { sound: 2, .. })
            )
        })
        .unwrap();
    let face = events
        .iter()
        .position(|e| matches!(e, Event::Monster(MonsterEvent::Face { .. })))
        .unwrap();
    let aim = events
        .iter()
        .position(|e| matches!(e, Event::Monster(MonsterEvent::Aim { .. })))
        .unwrap();
    assert!(sound < face && face < aim);
    let slope = events
        .iter()
        .find_map(|e| {
            if let Event::Monster(MonsterEvent::Aim { result, .. }) = e {
                Some(result.slope)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Monster(MonsterEvent::Aim { .. })))
            .count(),
        1
    );
    let params = events
        .iter()
        .filter_map(|e| {
            if let Event::Shotgun(ShotgunEvent::Pellet { index, slope, .. }) = e {
                Some((*index, *slope))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(params, vec![(1, slope), (2, slope), (3, slope)]);
    let done = events
        .iter()
        .filter_map(|e| {
            if let Event::Shotgun(ShotgunEvent::PelletDone { index, .. }) = e {
                Some(*index)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(done, [1, 2, 3]);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Shotgun(ShotgunEvent::Completed { .. })))
            .count(),
        1
    );
}
fn main() {
    let cases = fixtures::cases();
    for label in [
        "multiple_targets_hp6_y0",
        "thinker_pursue",
        "thinker_shadow",
        "thinker_cycles",
    ] {
        let which = cases.iter().position(|c| c.label == label).unwrap();
        let c = &cases[which];
        let mut s = fixtures::setup(c);
        let mut trace = fixtures::SourceTrace::default();
        trace.check(&s, which, 0, s.pending_events());
        fixtures::assert_golden(&s, which, 0);
        let owner = ActorId::from_parts(0, 1);
        let initial = s.actor(owner).unwrap().position();
        let mut acquired = false;
        let mut moved = false;
        let mut first_face = false;
        let mut returned = false;
        let mut retired = false;
        let mut blasts = 0;
        let mut changed_health = false;
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
            assert_blast(events);
            acquired |= s.actor(owner).unwrap().life().unwrap().target.is_some();
            moved |= s.actor(owner).unwrap().position() != initial;
            first_face |= events.iter().any(|e| {
                matches!(
                    e,
                    Event::ActorState(ActorStateEvent::Entered {
                        state: StateId(4),
                        ..
                    })
                )
            });
            returned |= events.iter().any(|e| {
                matches!(
                    e,
                    Event::ActorState(ActorStateEvent::Entered {
                        state: StateId(2),
                        ..
                    })
                )
            });
            retired |= events.iter().any(|e| matches!(e, Event::Retired(_)));
            blasts += events
                .iter()
                .filter(|e| matches!(e, Event::Shotgun(ShotgunEvent::Begin { .. })))
                .count();
            changed_health |= events
                .iter()
                .any(|e| matches!(e, Event::Combat(CombatEvent::Health { .. })));
        }
        assert!(acquired && blasts > 0);
        if c.v("mode") == 4 {
            assert!(moved && first_face && returned && retired);
            assert_eq!(
                blasts,
                match label {
                    "thinker_pursue" => 1,
                    "thinker_shadow" => 2,
                    "thinker_cycles" => 3,
                    _ => unreachable!(),
                }
            );
            println!(
                "{label}: {} tics, {blasts} source blasts; acquisition/pursuit/two facing calls/shared aim/three pellets/effects/return/retirement and DSIM-v11 match",
                s.tic()
            );
        } else {
            let mut pellet = 0;
            for event in s.pending_events() {
                match event {
                    Event::Shotgun(ShotgunEvent::PelletStart { index, .. }) => pellet = *index,
                    Event::Combat(CombatEvent::Kill { target, .. }) if target.slot() == 1 => {
                        assert_eq!(pellet, 2)
                    }
                    Event::Combat(CombatEvent::Damage(request)) if request.target.slot() == 2 => {
                        assert_eq!(pellet, 3)
                    }
                    _ => {}
                }
            }
            assert!(changed_health);
            assert!(
                s.actor(ActorId::from_parts(1, 1))
                    .unwrap()
                    .life()
                    .unwrap()
                    .health
                    <= 0
            );
            assert!(
                s.actor(ActorId::from_parts(2, 1))
                    .unwrap()
                    .life()
                    .unwrap()
                    .health
                    < 100
            );
            println!(
                "{label}: source pellet 2 kills the front body; pellet 3 damages the next target"
            );
        }
        if label == "thinker_cycles" {
            assert!(changed_health);
            println!("DSIM-v11 final {:016x}", s.snapshot().digest());
        }
    }
}
