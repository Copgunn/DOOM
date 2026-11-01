#![no_main]
#[path = "../../tests/support/m3f7.rs"]
mod fixtures;
use doom_sim::{identity::*, *};
use libfuzzer_sys::fuzz_target;
fn operation(s: &mut Simulation, ids: &[ActorId], c: &[u8]) {
    let raw = u16::from_le_bytes([c[2], c[3]]);
    let actor = ids[usize::from(c[1]) % ids.len()];
    let actor = if c[0] & 128 != 0 {
        ActorId::from_parts(actor.slot(), actor.generation() + 1)
    } else {
        actor
    };
    let before = s.snapshot();
    let failed = match c[0] % 6 {
        0 => s.step([TicCommand::default(); 4]).is_err(),
        1 => s.enter_actor_state(actor, StateId(raw)).is_err(),
        2 => {
            OriginalActorType::try_from(raw).map_or(true, |ty| s.spawn_original_actor(ty).is_err())
        }
        3 => s
            .damage_actor(DamageRequest {
                target: actor,
                source: None,
                inflictor: None,
                amount: i32::from(raw),
            })
            .is_err(),
        4 => s.mark_remove(actor).is_err(),
        _ => {
            if let Ok((definition, state)) = s.original_actor_identity(actor) {
                let life = s.actor(actor).unwrap().life().unwrap();
                assert_eq!(state.map(OriginalState::ordinal), life.state.map(|s| s.0));
                if let DefinitionIdentity::Original(ty) = definition {
                    assert_eq!(ty.ordinal(), life.actor_type.0);
                }
            }
            let bytes = s
                .snapshot()
                .diagnostic_bytes(DiagnosticLimits::default())
                .unwrap();
            let view = Diagnostic::decode(&bytes, DiagnosticLimits::default()).unwrap();
            assert_eq!(view.encode(DiagnosticLimits::default()).unwrap(), bytes);
            false
        }
    };
    if failed {
        assert_eq!(before, s.snapshot());
    }
}
fuzz_target!(|data: &[u8]| {
    if data.len() < 2 || data.len() > 34 {
        return;
    }
    let cases = fixtures::cases();
    let c = &cases[usize::from(data[0]) % cases.len()];
    let make = || {
        Simulation::original_id_map(
            fixtures::map(usize::try_from(c.v("map")).unwrap()),
            u32::from(data[1] % 65),
            fixtures::options(c),
            MapContextOptions::default(),
        )
        .unwrap()
    };
    let (mut a, mut b) = (make(), make());
    let before = a.snapshot();
    let ra = a.spawn_original_actor(OriginalActorType::MT_SERGEANT);
    let rb = b.spawn_original_actor(OriginalActorType::MT_SERGEANT);
    assert_eq!(ra, rb);
    assert_eq!(a.snapshot(), b.snapshot());
    if ra.is_err() {
        assert_eq!(before, a.snapshot());
        return;
    }
    if data[1] % 65 < 8 {
        return;
    }
    a = fixtures::install_targets(a, c);
    b = fixtures::install_targets(b, c);
    let ids: Vec<_> = a.actor_order().collect();
    for c in data[2..].as_chunks::<4>().0.iter().take(8) {
        operation(&mut a, &ids, c);
        operation(&mut b, &ids, c);
        assert_eq!(a.snapshot(), b.snapshot());
    }
});
