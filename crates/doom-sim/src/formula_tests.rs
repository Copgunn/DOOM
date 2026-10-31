//! Component evidence from compiled original bodies, distinct from whole-game tests.
use super::*;
const OBSERVED: &str = include_str!("../../../tests/reference/weapon_formula_observations.csv");

#[test]
fn every_damage_byte_matches_compiled_original_bodies() {
    for line in OBSERVED.lines().skip(1).take(256) {
        let row: Vec<_> = line.split(',').map(|s| s.parse::<i32>().unwrap()).collect();
        let value = u8::try_from(row[0]).unwrap();
        assert_eq!(bullet_damage(value), row[1]);
        assert_eq!(punch_damage(value), row[2]);
        let mut kinds = Vec::new();
        let accurate = gun_shot(u32::MAX, true, &mut |kind| {
            kinds.push(kind);
            value
        });
        assert_eq!(accurate, (u32::MAX, row[1]));
        assert_eq!(kinds, [0]);
    }
}
fn hash_word(hash: &mut u64, value: u32) {
    for byte in value.to_le_bytes() {
        *hash = (*hash ^ u64::from(byte)).wrapping_mul(1099511628211);
    }
}
#[test]
fn all_spread_pairs_wrap_and_match_compiled_source_summaries() {
    for punch in [false, true] {
        let mut hash = 14695981039346656037u64;
        for angle in [0, 0x80000000, u32::MAX] {
            for left in 0..=255u8 {
                for right in 0..=255u8 {
                    let mut kinds = Vec::new();
                    let mut random = |kind| {
                        kinds.push(kind);
                        match kind {
                            0 => 123,
                            1 => left,
                            2 => right,
                            _ => panic!("unexpected draw kind"),
                        }
                    };
                    let (result, damage) = if punch {
                        punch_shot(angle, &mut random)
                    } else {
                        gun_shot(angle, false, &mut random)
                    };
                    assert_eq!(kinds, [0, 1, 2]);
                    // Independent widened mathematical specification of BAM wrapping.
                    let delta = (i64::from(left) - i64::from(right)) * 262144;
                    assert_eq!(
                        i64::from(result),
                        (i64::from(angle) + delta).rem_euclid(1i64 << 32)
                    );
                    hash_word(&mut hash, result);
                    hash_word(&mut hash, u32::try_from(damage).unwrap());
                    hash_word(&mut hash, 3);
                }
            }
        }
        let label = if punch {
            "punch_spread"
        } else {
            "bullet_spread"
        };
        let expected = u64::from_str_radix(
            OBSERVED
                .lines()
                .find(|l| l.starts_with(label))
                .unwrap()
                .split(',')
                .nth(1)
                .unwrap(),
            16,
        )
        .unwrap();
        assert_eq!(hash, expected);
    }
}
#[test]
fn staged_and_recorded_rng_adapters_preserve_draws_indices_and_pellet_order() {
    for start in 0..=255u16 {
        for (punch, accurate, pellets) in [
            (true, false, 1),
            (false, true, 1),
            (false, false, 1),
            (false, false, 7),
        ] {
            let mut staged = DoomRng::default();
            for _ in 0..start {
                staged.p_random();
            }
            let mut authoritative = staged;
            let mut preview = Vec::new();
            let mut committed = Vec::new();
            let mut ordered = Vec::new();
            for _ in 0..pellets {
                let mut a = |_| staged.p_random();
                preview.push(if punch {
                    punch_shot(u32::MAX, &mut a)
                } else {
                    gun_shot(u32::MAX, accurate, &mut a)
                });
                let mut b = |kind| {
                    let value = authoritative.p_random();
                    ordered.push((kind, value));
                    value
                };
                committed.push(if punch {
                    punch_shot(u32::MAX, &mut b)
                } else {
                    gun_shot(u32::MAX, accurate, &mut b)
                });
            }
            assert_eq!(preview, committed);
            assert_eq!(staged, authoritative);
            let pattern = if accurate { &[0][..] } else { &[0, 1, 2][..] };
            assert_eq!(
                ordered.iter().map(|&(kind, _)| kind).collect::<Vec<_>>(),
                pattern
                    .iter()
                    .copied()
                    .cycle()
                    .take(pattern.len() * pellets)
                    .collect::<Vec<_>>()
            );
        }
    }
}
