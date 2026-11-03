//! Private structural rollback check and opt-in host cost observations.
use super::*;
#[path = "../../../tests/support/m3g1.rs"]
mod fixture;
use std::hint::black_box;
#[test]
fn projection_mutation_is_isolated_and_component_costs_are_observable() {
    for (label, profile) in [("small_control", "20"), ("canonical_small", "original")] {
        let workload = fixture::Workload::setup(label, profile).unwrap();
        let mut sim = workload.sim;
        let before = sim.snapshot();
        let mut plan = sim.demon_plan().unwrap();
        assert_eq!(
            plan.actors.order().collect::<Vec<_>>(),
            sim.actor_order().collect::<Vec<_>>()
        );
        assert_eq!(plan.spatial.order, sim.spatial.order);
        // The shared order is staged with the slots; without sector thinkers it is
        // the actor order and the empty sector class costs no allocation.
        assert_eq!(
            plan.actors.shared_order().collect::<Vec<_>>(),
            sim.actors.shared_order().collect::<Vec<_>>()
        );
        assert!(
            plan.actors
                .shared_order()
                .eq(sim.actor_order().map(ThinkerId::Actor))
        );
        assert_eq!(plan.actors.sector_capacity(), 0);
        assert_eq!(plan.actors.sector_slots.capacity(), 0);
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

#[test]
fn specials_plans_stage_the_sector_class_in_the_committed_buffer() {
    // pools_lights_soak: eight light thinkers in the sector-thinker class.
    let mut sim = crate::specials_evidence::setup(&crate::specials_evidence::cases()[0]).0;
    assert_eq!(sim.actors.sector_slots.len(), 8);
    for _ in 0..2 {
        sim.advance([TicCommand::default(); 4]).unwrap();
    }
    // After a commit the replaced vector is kept as the next plan's buffer, so
    // staging the sector class reuses it instead of allocating.
    let spare = sim.spare_sector_slots.as_ptr();
    assert!(sim.spare_sector_slots.capacity() >= sim.actors.sector_slots.len());
    let plan = sim.demon_plan().unwrap();
    assert_eq!(plan.actors.sector_slots.as_ptr(), spare);
    assert_eq!(plan.actors.sector_slots.len(), 8);
    assert_eq!(sim.spare_sector_slots.capacity(), 0);
    let staged = sim.snapshot();
    drop(plan);
    assert_eq!(sim.snapshot(), staged);
}
