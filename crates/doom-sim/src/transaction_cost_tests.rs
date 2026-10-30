//! Private structural rollback check and opt-in host cost observations.
use super::*;
#[path = "../../../tests/support/m3g1.rs"]
mod fixture;
use std::hint::black_box;
#[test]
fn projection_mutation_is_isolated_and_component_costs_are_observable() {
    for (label, profile) in [("small_control", "20"), ("canonical_small", "original")] {
        let workload = fixture::Workload::setup(label, profile).unwrap();
        let sim = workload.sim;
        let before = sim.snapshot();
        let mut plan = sim.demon_plan().unwrap();
        assert_eq!(
            plan.actors.order().collect::<Vec<_>>(),
            sim.actor_order().collect::<Vec<_>>()
        );
        assert_eq!(plan.spatial.order, sim.spatial.order);
        let first = plan.actors.order().next().unwrap();
        plan.actors.mark_remove(first).unwrap();
        plan.rng.p_random();
        plan.statistics.as_mut().unwrap().total_kills += 1;
        plan.bindings.fill(None);
        drop(plan);
        assert_eq!(sim.snapshot(), before);
    }
    if std::env::var_os("DOOM_STEPPING_COST").is_none() {
        return;
    }
    for (label, profile) in [
        ("small_control", "20"),
        ("four_owners", "20"),
        ("high_cell_demon", "20"),
        ("canonical_small", "original"),
        ("actor_capacity", "original"),
    ] {
        let mut sim = fixture::Workload::setup(label, profile).unwrap().sim;
        let before = sim.snapshot();
        let slot_bytes = std::mem::size_of_val(sim.actors.slots.as_slice());
        let spatial_copy_bytes = std::mem::size_of_val(sim.spatial.order.as_slice());
        let spatial_reserved_bytes = sim.actors.max_slots as usize * std::mem::size_of::<ActorId>();
        for operation in ["preflight", "actor_preflight", "staging"] {
            let (result, elapsed) = fixture::interval(|| -> Result<(), SimError> {
                for _ in 0..fixture::UPDATES {
                    match operation {
                        "preflight" => black_box(&sim).preflight_expanded_demons()?,
                        "actor_preflight" => black_box(&mut sim).preflight_actor_step()?,
                        _ => drop(black_box(sim.demon_plan()?)),
                    }
                }
                Ok(())
            })
            .unwrap();
            result.unwrap();
            assert_eq!(sim.snapshot(), before);
            println!(
                "transaction label={label} profile={profile} operation={operation} calls={} ns={} logical_slots={} slot_bytes={slot_bytes} spatial_copy_bytes={spatial_copy_bytes} spatial_reserved_bytes={spatial_reserved_bytes}",
                fixture::UPDATES,
                elapsed.as_nanos(),
                sim.actors.slots.len()
            );
        }
    }
}
