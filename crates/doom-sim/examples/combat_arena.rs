//! Asserted original-C combat outcomes and independently encoded DSIM-v5.
#[path = "../../../tests/support/m3d1.rs"]
mod fixtures;
use doom_sim::*;
fn main() {
    let cases = fixtures::cases();
    let target = ActorId::from_parts(1, 1);
    let mut open = fixtures::setup(&cases[13]);
    let hit = fixtures::execute(&mut open, &cases[13]).unwrap();
    assert_eq!(hit.hit, Some(TraceObject::Actor(target)));
    assert_eq!(open.actor(target).unwrap().life().unwrap().health, 80);
    assert_eq!(open.actor(target).unwrap().life().unwrap().reaction_time, 0);
    fixtures::assert_golden(&open, 13, 1);

    let mut blocked = fixtures::setup(&cases[14]);
    let hit = fixtures::execute(&mut blocked, &cases[14]).unwrap();
    assert!(matches!(hit.hit, Some(TraceObject::Line(_))));
    assert_eq!(blocked.actor(target).unwrap().life().unwrap().health, 100);
    fixtures::assert_golden(&blocked, 14, 1);

    let mut pain = fixtures::setup(&cases[3]);
    fixtures::execute(&mut pain, &cases[3]);
    assert!(
        pain.pending_events()
            .iter()
            .any(|e| matches!(e, Event::Combat(CombatEvent::Pain { .. })))
    );
    fixtures::assert_golden(&pain, 3, 1);

    let mut lethal = fixtures::setup(&cases[38]);
    let hit = fixtures::execute(&mut lethal, &cases[38]).unwrap();
    assert!(hit.damage.unwrap().killed);
    fixtures::assert_golden(&lethal, 38, 1);
    let mut actions = 0;
    for _ in 0..80 {
        lethal.step([TicCommand::default(); 4]).unwrap();
        actions += lethal
            .events()
            .iter()
            .filter(|e| matches!(e, Event::ActorState(ActorStateEvent::Action { .. })))
            .count();
    }
    fixtures::assert_golden(&lethal, 38, 81);
    let corpse = lethal.actor(target).unwrap();
    assert_eq!(corpse.life().unwrap().health, -20);
    assert_eq!(corpse.life().unwrap().tics, -1);
    assert_eq!(corpse.solidity(), Solidity::NonSolid);
    assert!(lethal.actor(ActorId::from_parts(2, 1)).is_none());
    assert_eq!(actions, 2);
    println!(
        "Combat arena: hitscan, wall, pain, lethal hit; 80 tics, {actions} death actions; corpse persists, impact retired."
    );
    println!("DSIM-v5 final {:016x}", lethal.snapshot().digest());
}
