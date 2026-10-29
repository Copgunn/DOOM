#![no_main]
#[path = "../../tests/support/m3f5.rs"]
mod fixtures;
use doom_sim::{identity::*, *};
use libfuzzer_sys::fuzz_target;

fn operation(sim: &mut Simulation, ids: &[ActorId], c: &[u8]) {
    let mut actor = ids[usize::from(c[1]) % ids.len()];
    if c[0] & 128 != 0 {
        actor = ActorId::from_parts(actor.slot(), actor.generation() + 1);
    }
    let before = sim.snapshot();
    let ordinal = u16::from_le_bytes([c[2], c[3]]);
    let failed = match c[0] % 7 {
        0 => sim.step([TicCommand::default(); 4]).is_err(),
        1 => match OriginalState::try_from(ordinal) {
            Ok(state) => sim.enter_original_demon_state(actor, state).is_err(),
            Err(_) => true,
        },
        2 => match OriginalActorType::try_from(ordinal) {
            Ok(kind) => sim.spawn_original_demons(kind).is_err(),
            Err(_) => true,
        },
        3 => {
            let _ = sim.original_demon_identity(actor);
            let binding = sim.canonical_demon_binding().unwrap();
            if let Ok(state) = OriginalState::try_from(ordinal) {
                if let Ok(runtime) = binding.runtime_state(state) {
                    assert_eq!(binding.original_state(runtime), Ok(state));
                }
            }
            assert_eq!(before, sim.snapshot());
            false
        }
        4 => sim
            .damage_actor(DamageRequest {
                target: actor,
                source: None,
                inflictor: None,
                amount: i32::from(ordinal),
            })
            .is_err(),
        5 => sim.mark_remove(actor).is_err(),
        _ => {
            let binding = sim.canonical_demon_binding().unwrap();
            let _ = binding.original_state(StateId(ordinal));
            if let Ok(kind) = OriginalActorType::try_from(ordinal) {
                assert_eq!(
                    binding.runtime_type(kind).is_ok(),
                    kind == OriginalActorType::MT_SERGEANT
                );
            }
            assert_eq!(before, sim.snapshot());
            false
        }
    };
    if failed {
        assert_eq!(before, sim.snapshot());
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 || data.len() > 35 {
        return;
    }
    let cases = fixtures::cases();
    let c = &cases[usize::from(data[0]) % cases.len()];
    let make = || {
        Simulation::canonical_demon_map(
            fixtures::map(c.v("map") as usize),
            u32::from(data[1] % 65),
            fixtures::options(c),
            MapContextOptions::default(),
        )
    };
    let (mut a, mut b) = (make().unwrap(), make().unwrap());
    let before = a.snapshot();
    let ra = a.spawn_original_demons(OriginalActorType::MT_SERGEANT);
    let rb = b.spawn_original_demons(OriginalActorType::MT_SERGEANT);
    assert_eq!(ra, rb);
    assert_eq!(a.snapshot(), b.snapshot());
    if ra.is_err() {
        assert_eq!(before, a.snapshot());
        return;
    }
    // Installing the four fixture bodies requires room; low limits above only
    // exercise batch admission/rollback, without inventing an execution setup.
    if data[1] % 65 < 8 {
        return;
    }
    a = fixtures::install_targets(a, c);
    b = fixtures::install_targets(b, c);
    let ids: Vec<_> = a.actor_order().collect();
    assert_eq!(a.snapshot(), b.snapshot());
    for c in data[2..].chunks_exact(4).take(8) {
        operation(&mut a, &ids, c);
        operation(&mut b, &ids, c);
        assert_eq!(a.snapshot(), b.snapshot());
    }
});
