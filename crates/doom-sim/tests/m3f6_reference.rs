//! Reused, independently compiled C observations; no new C inputs or captures.
#[path = "../../../tests/support/m3f5.rs"]
mod fixtures;
use doom_sim::{identity::*, *};
use std::collections::BTreeSet;

fn check_native(
    sim: &Simulation,
    which: usize,
    op: usize,
    spawn: bool,
    seen: &mut BTreeSet<OriginalState>,
) -> usize {
    let actors: Vec<_> = sim.actor_order().collect();
    let mut fields = 0;
    for (_, row) in fixtures::rows(which, op, spawn)
        .iter()
        .filter(|(tag, r)| tag == "A" && r[1] != 0 && r[2] == 12)
    {
        let actor = actors[usize::try_from(row[0]).unwrap()];
        let life = sim.actor(actor).unwrap().life().unwrap();
        let (ty, state) = sim.native_demon_identity(actor).unwrap();
        assert_eq!(ty.runtime(), life.actor_type);
        assert_eq!(i64::from(ty.original().ordinal()), row[2]);
        assert_ne!(i64::from(ty.runtime().0), row[2]);
        assert_eq!(state.map(NativeDemonState::runtime), life.state);
        assert_eq!(
            state.map_or(-1, |s| i64::from(s.original().ordinal())),
            row[10]
        );
        if let Some(state) = state {
            seen.insert(state.original());
        }
        fields += 2;
    }
    fields
}

#[test]
fn native_boundary_matches_c_identity_and_frozen_capture_chronology() {
    let mut seen = BTreeSet::new();
    let (mut identities, mut spawn_ids, mut captures, mut events) = (0, 0, 0, 0);
    for (which, case) in fixtures::cases().iter().enumerate() {
        let map = fixtures::map_only(case);
        spawn_ids += check_native(&map, which, 0, true, &mut seen);
        let mut sim = fixtures::setup(case);
        for op in 0..=usize::try_from(case.v("tics")).unwrap() {
            if op != 0 {
                fixtures::advance(&mut sim, case, op);
            }
            identities += check_native(&sim, which, op, false, &mut seen);
            events += fixtures::assert_golden(&sim, which, op);
            captures += 1;
        }
    }
    assert_eq!(
        (identities, spawn_ids, captures, events),
        (1906, 26, 664, 14063)
    );
    assert_eq!(seen.len(), 21);
    println!(
        "M3f6 reused C boundary: {identities} identity fields + {spawn_ids} raw-spawn fields; {captures} existing captures; {events} ordered event fields; {} normal states",
        seen.len()
    );
}
