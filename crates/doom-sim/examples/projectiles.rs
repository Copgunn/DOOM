//! Asserted original-C plasma lifecycle and independent DSIM-v7 snapshots.
#[path = "../../../tests/support/m3d2b.rs"]
mod fixtures;
use doom_sim::*;
fn main() {
    let cases = fixtures::cases();
    for which in [2, 5, 36, 24] {
        let c = &cases[which];
        let mut sim = fixtures::setup(c);
        let mut trace = fixtures::Trace::default();
        trace.check(&sim, which, 0, sim.pending_events());
        fixtures::assert_golden(&sim, which, 0);
        let pending = sim.pending_events().len();
        fixtures::launch(&mut sim, c);
        trace.check(&sim, which, 1, &sim.pending_events()[pending..]);
        if fixtures::has_golden(which, 1) {
            fixtures::assert_golden(&sim, which, 1);
        }
        let missile = ActorId::from_parts(2, 1);
        assert_eq!(
            sim.actor(missile).unwrap().life().unwrap().actor_type,
            ActorTypeId(34)
        );
        let mut flight = 0;
        let mut passage = false;
        let mut impact = false;
        let mut explosions = Vec::new();
        for op in 2..=c.n(16) as usize + 1 {
            let pending = sim.pending_events().len();
            sim.step([TicCommand::default(); 4]).unwrap();
            trace.check(&sim, which, op, &sim.events()[pending..]);
            if fixtures::has_golden(which, op) {
                fixtures::assert_golden(&sim, which, op);
            }
            if let Some(a) = sim.actor(missile) {
                let l = a.life().unwrap();
                if l.flags & 65536 != 0 {
                    flight += 1;
                } else if let Some(s) = l.state {
                    explosions.push(s.0);
                }
            }
            for e in sim.events() {
                match e {
                    Event::SectorMovement { actor, attempt }
                        if *actor == missile
                            && attempt.allowed()
                            && attempt.location.sector == 1 =>
                    {
                        passage = true
                    }
                    Event::Projectile(ProjectileEvent::Exploded(id)) if *id == missile => {
                        impact = true
                    }
                    _ => {}
                }
            }
        }
        assert!(flight > 1);
        assert!(impact);
        assert!(explosions.contains(&113));
        assert!(sim.actor(missile).is_none());
        if which == 5 || which == 36 {
            assert!(passage);
        }
        if which == 24 {
            assert!(
                sim.actor(ActorId::from_parts(1, 1))
                    .unwrap()
                    .life()
                    .unwrap()
                    .health
                    <= 0
            );
            fixtures::assert_golden(&sim, which, c.n(16) as usize + 1);
        }
        println!(
            "{}: {} tics; C spawn/velocity/flight/impact/explosion/removal assertions passed",
            c.label,
            sim.tic()
        );
        if which == 24 {
            println!("DSIM-v7 final {:016x}", sim.snapshot().digest());
        }
    }
}
