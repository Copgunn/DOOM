//! DSIM-v5 encoded independently from C captures and explicit fixture inputs.
#[path = "../../../tests/support/m3d1.rs"]
mod fixtures;
use doom_sim::TicCommand;
#[test]
fn all_sixty_three_v5_captures_and_seventeen_byte_fixtures_match() {
    for (which, case) in fixtures::cases().iter().enumerate() {
        let mut sim = fixtures::setup(case);
        fixtures::assert_golden(&sim, which, 0);
        fixtures::execute(&mut sim, case);
        if [0, 1, 2, 3, 4, 5, 6, 7, 10, 13, 14, 38].contains(&which) {
            fixtures::assert_golden(&sim, which, 1);
        }
        if [5, 7, 38].contains(&which) {
            for _ in 0..case.n(19) {
                sim.step([TicCommand::default(); 4]).unwrap();
            }
            fixtures::assert_golden(&sim, which, case.n(19) + 1);
        }
    }
}
