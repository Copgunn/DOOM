//! Comparisons to separately compiled preserved C, plus explicit safe-policy edges.
use doom_types::{DoomRng, Fixed, FixedDivError};

#[test]
fn fixed_point_matches_recorded_c_vectors() {
    for (index, line) in include_str!("../../../tests/reference/fixed.csv")
        .lines()
        .skip(1)
        .enumerate()
    {
        let c = line.split(',').collect::<Vec<_>>();
        let a = c[1].parse::<i32>().unwrap();
        let b = c[2].parse::<i32>().unwrap();
        let (a, b) = (Fixed::from_bits(a), Fixed::from_bits(b));
        if c[3] == "ERR" {
            assert_eq!(
                a.div2_checked(b),
                Err(FixedDivError::Overflow),
                "helper row {}: {line}",
                index + 2
            );
            continue;
        }
        let expected = c[3].parse::<i32>().unwrap();
        let actual = match c[0] {
            "M" => (a * b).bits(),
            "D" => (a / b).bits(),
            "H" => a.div2_checked(b).unwrap().bits(),
            _ => panic!("unknown vector operation"),
        };
        assert_eq!(actual, expected, "C reference row {}: {line}", index + 2);
        if c[0] == "H" {
            let integer = (i64::from(a.bits()) * 65536) / i64::from(b.bits());
            assert_eq!(
                i64::from(expected),
                integer,
                "double/integer experiment row {}",
                index + 2
            );
        }
    }
}

#[test]
fn safe_extremes_wrapping_rounding_and_division_policy() {
    let raw = Fixed::from_bits;
    assert_eq!(Fixed::MAX + raw(1), Fixed::MIN);
    assert_eq!(Fixed::MIN - raw(1), Fixed::MAX);
    assert_eq!(-Fixed::MIN, Fixed::MIN);
    assert_eq!((raw(-1) * raw(1)).bits(), -1);
    assert_eq!(raw(-1).div2_checked(raw(3)).unwrap().bits(), -21845);
    assert_eq!(Fixed::ZERO / Fixed::ZERO, Fixed::MAX);
    assert_eq!(raw(-1) / Fixed::ZERO, Fixed::MIN);
    assert_eq!(Fixed::MIN / Fixed::MIN, Fixed::ONE);
    assert_eq!((Fixed::ONE / Fixed::MIN).bits(), -2);
    assert_eq!(Fixed::MIN / (-Fixed::ONE), Fixed::MAX);
    assert_eq!(raw(1_000_000_000) / raw(50_000), Fixed::MAX);
    assert_eq!(
        raw(1_000_000_000).div2_checked(raw(50_000)).unwrap().bits(),
        1_310_720_000
    );
    assert_eq!(
        Fixed::ZERO.div2_checked(Fixed::ZERO),
        Err(FixedDivError::DivisionByZero)
    );
    assert_eq!(
        Fixed::MIN.div2_checked(raw(-1)),
        Err(FixedDivError::Overflow)
    );
}

#[test]
fn every_rng_lookup_and_wrap_matches_original_c() {
    let mut rng = DoomRng::default();
    for line in include_str!("../../../tests/reference/rng.csv")
        .lines()
        .skip(1)
    {
        let c = line
            .split(',')
            .map(|v| v.parse::<u16>().unwrap())
            .collect::<Vec<_>>();
        let p = rng.p_random();
        let m = rng.m_random();
        assert_eq!(
            [u16::from(p), u16::from(m)],
            [c[3], c[4]],
            "first differing RNG lookup {} at expected indexes {}/{}",
            c[0],
            c[1],
            c[2]
        );
        assert_eq!(
            rng.indices(),
            [c[1] as u8, c[2] as u8],
            "RNG index after lookup {}",
            c[0]
        );
    }
}

#[test]
fn rng_stream_separation_reset_and_restore() {
    let mut baseline = DoomRng::default();
    let mut varied = DoomRng::default();
    for i in 0..1024 {
        for _ in 0..i % 7 {
            varied.m_random();
        }
        assert_eq!(
            varied.p_random(),
            baseline.p_random(),
            "gameplay lookup {i}"
        );
    }
    let mut restored = DoomRng::from_indices(varied.indices()[0], varied.indices()[1]);
    assert_eq!(restored.p_random(), varied.p_random());
    assert_eq!(restored.m_random(), varied.m_random());
    varied.reset();
    assert_eq!(varied.indices(), [0, 0]);
    assert_eq!(varied.p_random(), 8);
    assert_eq!(varied.m_random(), 8);
}
