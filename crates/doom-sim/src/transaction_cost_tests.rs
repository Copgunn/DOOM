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

#[test]
fn plans_stage_sector_writes_in_a_sparse_overlay() {
    // No policy before the movers policy writes a sector height or a line special:
    // a whole tic leaves the overlay empty, and it never allocates.
    let mut sim = crate::specials_evidence::setup(&crate::specials_evidence::cases()[0]).0;
    for _ in 0..2 {
        sim.advance([TicCommand::default(); 4]).unwrap();
    }
    assert!(sim.spare_writes.is_empty());
    assert_eq!(sim.spare_writes, staging::StagedWrites::default());
    assert!(sim.line_specials.is_empty());
    let table = sim.sectors.clone();
    let before = sim.snapshot();
    let moved = SectorState {
        floor: table[0].floor + Fixed::from_map_units(8),
        ceiling: table[0].ceiling,
    };
    // A write is visible through the plan's view at once, and only there; a
    // refused (dropped) plan leaves the committed table byte-identical.
    let mut plan = sim.demon_plan().unwrap();
    assert!(plan.writes.is_empty());
    plan.writes.write_sector(&sim.sectors, 0, moved).unwrap();
    let view = plan.writes.sectors(&sim.sectors);
    assert_eq!(view[0], moved);
    assert!((1..table.len()).all(|i| view[i] == table[i]));
    assert_eq!(sim.sector_view()[0], table[0]);
    drop(plan);
    assert_eq!(sim.sectors, table);
    assert_eq!(sim.snapshot(), before);
    // A committed plan applies the overlay and keeps it as the next plan's buffer.
    let mut plan = sim.demon_plan().unwrap();
    plan.writes.write_sector(&sim.sectors, 0, moved).unwrap();
    let _ = sim.commit_demon_plan(plan);
    assert_eq!(sim.sectors[0], moved);
    assert!((1..table.len()).all(|i| sim.sectors[i] == table[i]));
    assert!(!sim.spare_writes.is_empty());
    let plan = sim.demon_plan().unwrap();
    assert!(plan.writes.is_empty());
}

/// The M3h8 §9 staging measurement (opt-in, `DOOM_STAGING_COST=1`, release): one
/// synthetic tic stages `k` sector writes, reads `reads` sectors through the staged
/// view and commits, with the sparse overlay and with a full copy of the sector table
/// into a reused spare buffer (the M3h7 `SectorSpecial` pattern), against a baseline
/// that reads the committed table directly and stages nothing. A third, bench-only
/// variant (`indexed`) adds to the sparse entries a reused per-sector position table
/// (`u16::MAX` when unwritten), so a read is one lookup instead of a scan, and
/// resets only the written positions at the next plan.
#[test]
fn staging_cost_sparse_overlay_against_full_copy() {
    if std::env::var_os("DOOM_STAGING_COST").is_none() {
        return;
    }
    const TICS: usize = 100_000;
    const REPEATS: usize = 7;
    let median = |mut runs: Vec<u128>| {
        runs.sort_unstable();
        runs[runs.len() / 2]
    };
    for sectors in [88usize, 4096] {
        let mut table: Vec<SectorState> = (0..sectors)
            .map(|i| SectorState {
                floor: Fixed::from_map_units(i as i16 % 256),
                ceiling: Fixed::from_map_units(256 + i as i16 % 256),
            })
            .collect();
        for k in [0usize, 1, 2, 4, 9, 18] {
            for reads in [0usize, 256, 2048] {
                // Deterministic spread of written and read indices.
                let written: Vec<u16> = (0..k).map(|j| ((j * 37 + 5) % sectors) as u16).collect();
                let read: Vec<usize> = (0..reads).map(|r| (r * 7919 + 13) % sectors).collect();
                let state = |t: usize| SectorState {
                    floor: Fixed::from_bits(t as i32 & 0xffff),
                    ceiling: Fixed::from_map_units(512),
                };
                let mut sparse = staging::StagedWrites::default();
                let mut spare: Vec<SectorState> = Vec::new();
                let mut lines: Vec<i16> = Vec::new();
                let (mut base_ns, mut sparse_ns, mut full_ns, mut indexed_ns) =
                    (vec![], vec![], vec![], vec![]);
                let mut position = vec![u16::MAX; sectors];
                let mut entries: Vec<(u16, SectorState)> = Vec::new();
                for _ in 0..REPEATS {
                    let start = std::time::Instant::now();
                    let mut sum = 0i64;
                    for _ in 0..TICS {
                        let view = staging::Sectors::committed(black_box(&table));
                        for &i in &read {
                            sum += i64::from(view[i].floor.bits());
                        }
                    }
                    black_box(sum);
                    base_ns.push(start.elapsed().as_nanos());

                    let start = std::time::Instant::now();
                    let mut sum = 0i64;
                    for t in 0..TICS {
                        sparse.clear();
                        for &s in &written {
                            sparse.write_sector(&table, s, state(t)).unwrap();
                        }
                        let view = sparse.sectors(black_box(&table));
                        for &i in &read {
                            sum += i64::from(view[i].floor.bits());
                        }
                        sparse.commit(&mut table, &mut lines);
                    }
                    black_box(sum);
                    sparse_ns.push(start.elapsed().as_nanos());

                    let start = std::time::Instant::now();
                    let mut sum = 0i64;
                    for t in 0..TICS {
                        spare.clear();
                        spare.try_reserve(table.len()).unwrap();
                        spare.extend_from_slice(&table);
                        for &s in &written {
                            spare[usize::from(s)] = state(t);
                        }
                        let view = black_box(spare.as_slice());
                        for &i in &read {
                            sum += i64::from(view[i].floor.bits());
                        }
                        std::mem::swap(&mut table, &mut spare);
                    }
                    black_box(sum);
                    full_ns.push(start.elapsed().as_nanos());

                    let start = std::time::Instant::now();
                    let mut sum = 0i64;
                    for t in 0..TICS {
                        for &(s, _) in &entries {
                            position[usize::from(s)] = u16::MAX;
                        }
                        entries.clear();
                        for &s in &written {
                            let at = &mut position[usize::from(s)];
                            if *at == u16::MAX {
                                *at = entries.len() as u16;
                                entries.push((s, state(t)));
                            } else {
                                entries[usize::from(*at)].1 = state(t);
                            }
                        }
                        let (table_ref, position_ref) = (black_box(&table), black_box(&position));
                        for &i in &read {
                            let at = position_ref[i];
                            let sector = if at == u16::MAX {
                                &table_ref[i]
                            } else {
                                &entries[usize::from(at)].1
                            };
                            sum += i64::from(sector.floor.bits());
                        }
                        for &(s, state) in &entries {
                            table[usize::from(s)] = state;
                        }
                    }
                    black_box(sum);
                    indexed_ns.push(start.elapsed().as_nanos());
                }
                let per_tic = |runs: Vec<u128>| median(runs) as f64 / TICS as f64;
                println!(
                    "staging sectors={sectors} writes={k} reads={reads} baseline_ns_per_tic={:.1} sparse_ns_per_tic={:.1} full_copy_ns_per_tic={:.1} indexed_ns_per_tic={:.1}",
                    per_tic(base_ns),
                    per_tic(sparse_ns),
                    per_tic(full_ns),
                    per_tic(indexed_ns)
                );
            }
        }
    }
}
