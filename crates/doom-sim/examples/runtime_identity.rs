//! A typed native view leaves stored IDs and historical captures unchanged.
#[path = "../../../tests/support/m3f5.rs"]
mod fixtures;
use doom_sim::*;
fn main() {
    let cases = fixtures::cases();
    for which in [1, 3] {
        let case = &cases[which];
        let mut sim = fixtures::setup(case);
        let owner = ActorId::from_parts(0, 1);
        for op in 0..=usize::try_from(case.v("tics")).unwrap() {
            if op != 0 {
                fixtures::advance(&mut sim, case, op);
            }
            let before = sim.snapshot();
            let (ty, state) = sim.native_demon_identity(owner).unwrap();
            let life = sim.actor(owner).unwrap().life().unwrap();
            assert_eq!(ty.runtime(), life.actor_type);
            assert_eq!(state.map(NativeDemonState::runtime), life.state);
            assert_eq!(
                sim.original_demon_identity(owner),
                Ok((ty.original(), state.map(NativeDemonState::original)))
            );
            assert_eq!(sim.snapshot(), before);
            assert_eq!(sim.snapshot().version(), 20);
            fixtures::assert_golden(&sim, which, op);
        }
        println!(
            "runtime_identity: {} native136 -> MT_SERGEANT12; DSIM20 {:016x}",
            case.label,
            sim.snapshot().digest()
        );
    }
}
