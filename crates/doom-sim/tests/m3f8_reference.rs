//! NEW independent original-C observations; Rust generation policy is separate.
#[path = "../../../tests/support/m3f8.rs"]
mod fixtures;
#[test]
fn scoped_execution_matches_new_c_fields_rng_statistics_and_all_event_bytes() {
    let mut sum = [0; 5];
    for (which, c) in fixtures::cases().iter().enumerate() {
        for (total, value) in sum.iter_mut().zip(fixtures::run(which, c)) {
            *total += value;
        }
    }
    println!("NEW C actor fields/draws/statistics/event payloads/captures: {sum:?}");
    assert_eq!(sum, [11856, 63, 376, 1138, 94]);
}
