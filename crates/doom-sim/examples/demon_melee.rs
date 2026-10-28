//! Original-C and independent DSIM-v13 assertions; synthetic public data only.
#[path = "../../../tests/support/m3d3f.rs"]
mod fixtures;
use doom_sim::*;
fn owner() -> ActorId {
    ActorId::from_parts(0, 1)
}
fn entered(e: &Event, state: u16) -> bool {
    matches!(e,Event::ActorState(ActorStateEvent::Entered{actor,state:StateId(s)}) if *actor==owner()&&*s==state)
}
fn main() {
    let cases = fixtures::cases();
    let mut final_digest = 0;
    for label in [
        "thinker_pursue",
        "thinker_shadow",
        "moving_miss",
        "lost_sight",
        "target_dies_during_animation",
        "target_lost",
        "reuse",
        "thinker_dead",
        "thinker_cycles",
    ] {
        let which = cases.iter().position(|c| c.label == label).unwrap();
        let c = &cases[which];
        let mut sim = fixtures::setup(c);
        let mut repeated = fixtures::setup(c);
        let initial = sim.actor(owner()).unwrap().position();
        let mut trace = fixtures::SourceTrace::default();
        trace.check(&sim, which, 0, sim.pending_events());
        fixtures::assert_golden(&sim, which, 0);
        let mut all = sim
            .pending_events()
            .iter()
            .cloned()
            .map(|e| (0, e))
            .collect::<Vec<_>>();
        let mut moved = false;
        for op in 1..=c.v("tics") as usize {
            let pending = sim.pending_events().len();
            fixtures::advance(&mut sim, c, op);
            fixtures::advance(&mut repeated, c, op);
            assert_eq!(sim.snapshot(), repeated.snapshot());
            let es = &sim.events()[pending..];
            trace.check(&sim, which, op, es);
            fixtures::assert_golden(&sim, which, op);
            all.extend(es.iter().cloned().map(|e| (op, e)));
            moved |= sim.actor(owner()).unwrap().position() != initial;
            let l = sim.actor(owner()).unwrap().life().unwrap();
            if let Some(StateId(state @ 4..=6)) = l.state {
                assert_eq!((l.sprite, l.frame), (39, u32::from(state)));
                assert!((1..=8).contains(&l.tics));
            }
            assert!(sim.actor_order().all(|id| matches!(
                sim.actor(id).unwrap().life().unwrap().actor_type,
                ActorTypeId(135 | 136)
            )));
            if op == 16 && matches!(label, "moving_miss" | "lost_sight") {
                assert!(es.iter().any(|e|matches!(e,Event::Monster(MonsterEvent::Range{decision,..}) if !decision.accepted)));
                assert!(!es.iter().any(|e| matches!(
                    e,
                    Event::Monster(MonsterEvent::Random { kind: 4, .. })
                        | Event::Combat(CombatEvent::Damage(_))
                )));
            }
            if op == 16 && label == "target_dies_during_animation" {
                assert!(
                    es.iter()
                        .any(|e| matches!(e, Event::Combat(CombatEvent::Ignored(_))))
                );
                assert!(
                    es.iter()
                        .any(|e| matches!(e, Event::Monster(MonsterEvent::Random { kind: 4, .. })))
                );
            }
            if op == 16 && label == "target_lost" {
                assert!(!es.iter().any(|e| matches!(
                    e,
                    Event::Monster(
                        MonsterEvent::Face { .. }
                            | MonsterEvent::Range { .. }
                            | MonsterEvent::Random { .. }
                    )
                )));
            }
        }
        assert!(!all.iter().any(|(_, e)| matches!(
            e,
            Event::Combat(
                CombatEvent::Attack { .. } | CombatEvent::Trace { .. } | CombatEvent::Impact { .. }
            ) | Event::Monster(MonsterEvent::Aim { .. })
        )));
        for (i, (_, e)) in all.iter().enumerate() {
            if matches!(e, Event::Monster(MonsterEvent::Selection { kind: 0, .. })) {
                assert!(matches!(
                    &all[i + 1].1,
                    Event::ActorState(ActorStateEvent::Sound { sound: 52, .. })
                ));
                assert!(entered(&all[i + 2].1, 4));
            }
        }
        let first4 = all.iter().position(|(_, e)| entered(e, 4)).unwrap();
        let first5 = (first4 + 1..all.len())
            .find(|&i| entered(&all[i].1, 5))
            .unwrap();
        let first6 = (first5 + 1..all.len())
            .find(|&i| entered(&all[i].1, 6))
            .unwrap();
        let back = (first6 + 1..all.len())
            .find(|&i| entered(&all[i].1, 2))
            .unwrap();
        assert_eq!(
            (
                all[first5].0 - all[first4].0,
                all[first6].0 - all[first5].0,
                all[back].0 - all[first6].0
            ),
            (8, 8, 8)
        );
        let cycle = &all[first4..back];
        let faces = cycle
            .iter()
            .filter(|(_, e)| matches!(e, Event::Monster(MonsterEvent::Face { .. })))
            .count();
        assert_eq!(
            faces,
            if label == "target_lost" {
                2
            } else if label == "target_dies_during_animation" {
                4
            } else {
                3
            }
        ); // Explicit extra source strike at tic12 adds one face.
        if label == "thinker_shadow" {
            assert_eq!(
                cycle
                    .iter()
                    .filter(|(_, e)| matches!(
                        e,
                        Event::Monster(MonsterEvent::Random { kind: 0 | 1, .. })
                    ))
                    .count(),
                6
            );
            assert!(
                all.iter()
                    .any(|(_, e)| matches!(e, Event::Combat(CombatEvent::Pain { .. })))
            );
        }
        if matches!(label, "thinker_cycles" | "thinker_pursue") {
            assert!(moved);
            assert!(all.iter().any(|(_, e)| matches!(
                e,
                Event::Perception(PerceptionEvent::ScanEnd { found: true, .. })
            )));
        }
        if matches!(label, "thinker_cycles" | "thinker_shadow") {
            assert!(all.iter().any(|(_,e)|matches!(e,Event::Combat(CombatEvent::Health{values,..}) if values[1]<values[0])));
            assert!(
                all.iter()
                    .any(|(_, e)| matches!(e, Event::Combat(CombatEvent::Knockback { .. })))
            );
            assert!(
                all.iter()
                    .filter(|(_, e)| matches!(e, Event::Combat(CombatEvent::Damage(_))))
                    .count()
                    > 1
            );
        }
        if label == "thinker_dead" {
            assert!(
                all.iter().any(|(_, e)| matches!(
                    e,
                    Event::Combat(CombatEvent::Kill { extreme: true, .. })
                ))
            );
            assert!(
                all.iter()
                    .any(|(_, e)| matches!(e, Event::Combat(CombatEvent::DeathTimer { .. })))
            );
        }
        if label == "reuse" {
            assert!(sim.actor(ActorId::from_parts(1, 1)).is_none());
            assert!(sim.actor(ActorId::from_parts(1, 2)).is_some());
        }
        if label == "thinker_cycles" {
            final_digest = sim.snapshot().digest();
            assert_eq!(final_digest, 0xb6c312bf5fbcc7dc);
        }
        println!("{label}: original-C and DSIM-v13 assertions passed");
    }
    println!("demon_melee final digest {final_digest:016x}");
}
