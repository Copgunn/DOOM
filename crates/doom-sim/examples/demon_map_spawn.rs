//! Original-C spawn/gameplay assertions and independent DSIM-v16 bytes.
#[path = "../../../tests/support/m3d3i.rs"]
mod fixtures;
use doom_sim::*;
use doom_types::Fixed;
fn main() {
    let cases = fixtures::cases();
    let owner = ActorId::from_parts(0, 1);
    let mut digest = 0;
    for label in [
        "one_idle",
        "one_wake",
        "multiple_4",
        "skill_group_0",
        "skill_group_1",
        "skill_group_2",
        "skill_group_3",
        "nomonsters_original_flags",
        "nonzero_floor",
        "shared_target_death",
        "spawn_lifecycle",
    ] {
        let which = cases.iter().position(|c| c.label == label).unwrap();
        let c = &cases[which];
        let (spawn, report) = fixtures::map_only(c);
        let mut raw = fixtures::SourceTrace::default();
        raw.check_original_spawn(&spawn, which);
        assert!(report.player_starts.is_empty() && report.deathmatch_starts.is_empty());
        assert!(
            spawn
                .pending_events()
                .iter()
                .all(|e| !matches!(e, Event::ActorState(ActorStateEvent::Action { .. })))
        );
        if label == "multiple_4" {
            assert_eq!(report.spawned.len(), 4);
            assert_eq!(spawn.actor_order().collect::<Vec<_>>(), report.spawned);
        }
        if label == "nomonsters_original_flags" {
            assert_eq!(report.filtered, 4);
            assert!(report.spawned.is_empty());
        }
        if label == "nonzero_floor" {
            assert_eq!(spawn.actor(owner).unwrap().z(), Fixed::from_map_units(80));
        }
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
            trace.check(&s, which, op, &s.events()[pending..]);
            fixtures::assert_golden(&s, which, op);
            all.extend(s.events()[pending..].iter().cloned().map(|e| (op, e)));
        }
        let entered = |state| {
            all.iter().filter_map(|(tic,e)|matches!(e,Event::ActorState(ActorStateEvent::Entered{actor,state:StateId(s)}) if *actor==owner&&*s==state).then_some(*tic)).collect::<Vec<_>>()
        };
        let sounds = |sound| {
            all.iter().filter_map(|(tic,e)|matches!(e,Event::ActorState(ActorStateEvent::Sound{actor:Some(id),sound:s}) if *id==owner&&*s==sound).then_some(*tic)).collect::<Vec<_>>()
        };
        if label == "one_idle" {
            let first = spawn.actor(owner).unwrap().life().unwrap().tics as usize;
            assert_eq!(entered(3)[0], first);
            assert_eq!(entered(1)[0], first + 10);
            assert!(entered(2).is_empty());
        }
        if label == "one_wake" {
            assert!(!entered(2).is_empty());
            assert!(!entered(4).is_empty());
            assert!(!entered(6).is_empty());
        }
        if label == "shared_target_death" {
            assert!(
                all.iter()
                    .any(|(_, e)| matches!(e, Event::Combat(CombatEvent::Ignored(_))))
            );
        }
        if label == "spawn_lifecycle" {
            assert!(entered(7).contains(&45));
            assert_eq!(sounds(26), [46]);
            assert!(entered(2).contains(&48));
            assert!(entered(9).contains(&75));
            assert_eq!(sounds(64), [81]);
            assert!(entered(12).contains(&93));
            assert!(entered(14).contains(&101));
            let a = s.actor(owner).unwrap();
            let l = a.life().unwrap();
            assert_eq!(l.state, Some(StateId(14)));
            assert_eq!(l.tics, -1);
            assert_eq!(l.flags & 6, 0);
            assert_eq!(l.flags & (1048576 | 1024), 1048576 | 1024);
            assert_eq!(a.height(), Fixed::from_map_units(14));
            digest = s.snapshot().digest();
        }
    }
    assert_eq!(digest, 0xe327c3fbf88cb132);
    println!("Demon map spawn: 11 C-backed scenarios; DSIM-v16 {digest:016x}");
}
