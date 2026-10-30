//! New boundary checks over an unchanged original-C scenario, not new C coverage.
#[path = "../../../tests/support/m3f9.rs"]
mod fixture;
#[test]
fn qualified_imports_delegate_to_the_existing_c_observed_replacement_chronology() {
    let (counts, _) = fixture::source_case();
    assert_eq!(counts[0], 910);
    assert_eq!(counts[2], 28);
    assert_eq!(counts[4], 7);
    println!(
        "M3f9 reused C: actor_fields={} draws={} statistics={} events={} captures={}",
        counts[0], counts[1], counts[2], counts[3], counts[4]
    );
}
