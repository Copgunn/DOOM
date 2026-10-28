//! Asset-free demonstration checked against original C rows and independent v4.
#[path = "../../../tests/support/m3c.rs"]
mod fixtures;
use doom_sim::{ActorId, ActorStateEvent, Event, Solidity, TicCommand};
fn main() {
    let mut sim = fixtures::setup(0);
    let player = ActorId::from_parts(0, 1);
    let fog = ActorId::from_parts(1, 1);
    assert_eq!(sim.actor(player).unwrap().life().unwrap().tics, 10);
    assert_eq!(sim.actor(fog).unwrap().life().unwrap().tics, 6);
    let mut actions = 0;
    let mut transitions = 0;
    for tic in 1..=80 {
        let snapshot = sim.step([TicCommand::default(); 4]).unwrap();
        let golden = include_str!("../../../tests/reference/m3c_snapshots.csv")
            .lines()
            .skip(1)
            .find(|row| {
                let mut parts = row.split(',');
                parts.next() == Some("0") && parts.next() == Some(tic.to_string().as_str())
            })
            .unwrap();
        assert_eq!(
            format!("{:016x}", snapshot.digest()),
            golden.split(',').nth(3).unwrap()
        );
        for event in sim.events() {
            match event {
                Event::ActorState(ActorStateEvent::Action { .. }) => actions += 1,
                Event::ActorState(ActorStateEvent::Entered { .. }) => transitions += 1,
                _ => {}
            }
        }
    }
    let actor = sim.actor(player).unwrap();
    assert_eq!(actor.life().unwrap().tics, -1);
    assert_eq!(actor.solidity(), Solidity::NonSolid);
    assert!(sim.actor(fog).is_none());
    assert_eq!(actions, 2);
    assert_eq!(transitions, 19);
    println!(
        "Actor states: 80 tics, {transitions} entries, {actions} actions; terminal player object persists, fog retired."
    );
    println!("DSIM-v4 final {:016x}", sim.snapshot().digest());
}
