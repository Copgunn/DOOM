//! Independent C/setup DSIM-v6 bytes and digests, preserving historical schemas.
#[path = "../../../tests/support/m3d2.rs"]
mod fixtures;
#[test]
fn v6_captures_match_independent_encoding() {
    let mut captures = 0;
    for (which, c) in fixtures::cases().iter().enumerate() {
        let mut sim = fixtures::setup(c);
        fixtures::assert_golden(&sim, which, 0);
        captures += 1;
        for (i, cmd) in fixtures::commands(which).into_iter().enumerate() {
            sim.step(cmd).unwrap();
            if fixtures::has_golden(which, i + 1) {
                fixtures::assert_golden(&sim, which, i + 1);
                captures += 1;
            }
        }
    }
    assert_eq!(captures, 69);
}
