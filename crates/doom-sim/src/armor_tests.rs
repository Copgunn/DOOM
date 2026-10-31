use super::*;
#[test]
fn independent_original_armor_actors() {
    assert_ne!(armor_evidence::verify_range(0..8), 0);
}

#[test]
fn independent_original_armor_pickups() {
    assert_ne!(armor_evidence::verify_range(8..15), 0);
    assert_ne!(armor_evidence::verify_range(17..18), 0);
    assert_ne!(armor_evidence::verify_range(21..22), 0);
    assert_ne!(armor_evidence::verify_range(32..33), 0);
}
#[test]
fn independent_original_give_armor_components() {
    for line in include_str!("../../../tests/reference/m3g10_components.csv").lines() {
        let v: Vec<_> = line.split(',').collect();
        if v.get(1) != Some(&"G") {
            continue;
        }
        let n = |i: usize| v[i].parse::<i32>().unwrap();
        let mut armor = PlayerArmorState {
            kind: ArmorType::try_from(n(4) as u8).unwrap(),
            points: n(5),
        };
        assert_eq!(
            armor
                .grant(ArmorType::try_from(n(3) as u8).unwrap())
                .unwrap(),
            n(6) != 0,
            "{line}"
        );
        assert_eq!(
            armor,
            PlayerArmorState {
                kind: ArmorType::try_from(n(7) as u8).unwrap(),
                points: n(8)
            },
            "{line}"
        );
    }
}
