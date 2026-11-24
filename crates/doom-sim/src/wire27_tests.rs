//! Wire 27 (`PlayerMonstersEpisode1V5`, DSIM43), step A: unit tests of the
//! wrapping `P_InterceptVector` (p_maputl.c:231-251) and of DSIM43. Expected
//! values come from the genuine C rows (`tests/reference/m3d3a_fractions.csv`,
//! the source body of `P_InterceptVector`), from hand computation, or from
//! [`reference`], an independent transcription of the C text in 128-bit
//! integers; never from the function under test. Rust-only otherwise.
use crate::barrels_replay::{cases, room};
use crate::diagonal_room::Lumps;
use crate::intercept_vector::{
    FixedDivRefusal, InterceptCaller, policy_intercept, wire27_wraps, wrapping_intercept,
};
use crate::*;
use FixedDivRefusal::{AbsMin, Div2Range, Saturation};

type V = [i64; 2];
fn fx(v: V) -> [Fixed; 2] {
    v.map(|b| Fixed::from_bits(i32::try_from(b).unwrap()))
}
fn wrap(v: i128) -> i64 {
    let m = v.rem_euclid(1 << 32);
    (if m >= 1 << 31 { m - (1 << 32) } else { m }) as i64
}
fn fits(v: i128) -> bool {
    (i128::from(i32::MIN)..=i128::from(i32::MAX)).contains(&v)
}
/// `FixedMul` (m_fixed.c:43-49) before narrowing: the product's floor `>> 16`.
fn mul(a: i64, b: i64) -> i128 {
    (i128::from(a) * i128::from(b)).div_euclid(65536)
}
/// p_maputl.c:240-250 with `FixedDiv` (m_fixed.c:57-87) and wire 27's refusals,
/// computed exactly and reduced mod 2^32 where the C stores an `int`. Returns
/// the result and whether any step left `int` (the cases `checked_intercept`
/// refuses).
fn reference(v2s: V, v2d: V, v1s: V, v1d: V) -> (Result<i32, FixedDivRefusal>, bool) {
    let (p1, p2) = (mul(v1d[1] >> 8, v2d[0]), mul(v1d[0] >> 8, v2d[1]));
    let d = i128::from(wrap(p1)) - i128::from(wrap(p2));
    let mut wrapped = !fits(p1) || !fits(p2) || !fits(d);
    let den = wrap(d);
    if den == 0 {
        return (Ok(0), wrapped);
    }
    let x = i128::from(v1s[0]) - i128::from(v2s[0]);
    let y = i128::from(v2s[1]) - i128::from(v1s[1]);
    let (q1, q2) = (mul(wrap(x) >> 8, v1d[1]), mul(wrap(y) >> 8, v1d[0]));
    let n = i128::from(wrap(q1)) + i128::from(wrap(q2));
    wrapped |= !fits(x) || !fits(y) || !fits(q1) || !fits(q2) || !fits(n);
    let num = wrap(n);
    if num == i64::from(i32::MIN) || den == i64::from(i32::MIN) {
        return (Err(AbsMin), wrapped);
    }
    if (num.abs() >> 14) >= den.abs() {
        if wrapped {
            return (Err(Saturation), true);
        }
        return (Ok(if (num ^ den) < 0 { i32::MIN } else { i32::MAX }), false);
    }
    let c = num as f64 / den as f64 * 65536.0;
    if !(-2147483648.0..2147483648.0).contains(&c) {
        return (Err(Div2Range), wrapped);
    }
    (Ok(c as i32), wrapped)
}
fn wrapping(v2s: V, v2d: V, v1s: V, v1d: V) -> Result<i32, FixedDivRefusal> {
    wrapping_intercept(fx(v2s), fx(v2d), fx(v1s), fx(v1d)).map(Fixed::bits)
}
fn checked(v2s: V, v2d: V, v1s: V, v1d: V) -> Option<i32> {
    sight::checked_intercept(fx(v2s), fx(v2d), fx(v1s), fx(v1d)).map(Fixed::bits)
}

/// Where `checked_intercept` is `Some`, wire 27 returns that value (an `INT_MIN`
/// operand aside): over the 131 genuine C rows, every one of which is `Some`
/// (none wraps) and equals the C value, and over a deterministic sweep, which
/// also agrees with [`reference`] everywhere, wrapped or not, and never reaches
/// `Div2Range`.
#[test]
fn equals_checked_where_some_over_the_c_rows_and_a_sweep() {
    let mut rows = 0;
    for line in include_str!("../../../tests/reference/m3d3a_fractions.csv")
        .lines()
        .skip(1)
    {
        let n: Vec<i64> = line.split(',').map(|s| s.parse().unwrap()).collect();
        let a = ([n[0], n[1]], [n[2], n[3]], [n[4], n[5]], [n[6], n[7]]);
        let got = wrapping(a.0, a.1, a.2, a.3);
        assert_eq!(got, reference(a.0, a.1, a.2, a.3).0, "row {rows}");
        assert_eq!(checked(a.0, a.1, a.2, a.3).map(Ok), Some(got), "row {rows}");
        assert_eq!(got, Ok(n[8] as i32), "C row {rows}");
        rows += 1;
    }
    assert_eq!(rows, 131);
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        // A value of 0 to 31 significant bits, either sign.
        (seed as i32 >> (seed >> 59)) as i64
    };
    let mut seen = [0_u32; 5];
    for _ in 0..200_000 {
        let a = (
            [next(), next()],
            [next(), next()],
            [next(), next()],
            [next(), next()],
        );
        let (expected, wrapped) = reference(a.0, a.1, a.2, a.3);
        let got = wrapping(a.0, a.1, a.2, a.3);
        assert_eq!(got, expected, "{a:?}");
        match checked(a.0, a.1, a.2, a.3) {
            Some(v) => {
                assert!(!wrapped);
                assert!(got == Ok(v) || got == Err(AbsMin), "{a:?}");
                seen[0] += 1;
            }
            None => {
                assert!(wrapped);
                seen[match got {
                    Ok(_) => 1,
                    Err(Saturation) => 2,
                    Err(AbsMin) => 3,
                    Err(Div2Range) => 4,
                }] += 1;
            }
        }
    }
    assert!(seen[..3].iter().all(|n| *n > 1000), "{seen:?}");
    assert_eq!(seen[4], 0);
}

/// One hand-computed vector per class (p_maputl.c:240-250, m_fixed.c:43-87),
/// with `a = 0x7FFF00 = 8,388,352`. Each is refused by `checked_intercept`.
#[test]
fn hand_vectors_of_each_wrap_class() {
    let (o, big, a) = ([0, 0], 0x7fff_0000, 1 << 30);
    // Wraps that cancel: both den products are a·2^14 (minus a) past `int`;
    // den = a exactly, num = 0x7FFF0000, frac = 256.0, the exact value.
    let cancel = (o, [a, a - 65536], [1 << 24, 0], [big, big]);
    // A wrong intercept: one product, a·2^14 = 137,434,759,168, wraps to
    // -4,194,304; the exact frac 1024 (1/64) becomes -511.984375·65536.
    let wrong = (o, [a, 0], [1 << 24, 0], [big, big]);
    // den = FixedMul(2^22, 2^26) = 2^32 wraps to exactly 0: 0 (p_maputl.c:242-243),
    // where the exact den gives frac 1.
    let zero = (o, [1 << 26, 0], [1 << 20, 0], [0, a]);
    // num = 2·2,147,418,112 wraps to -131,072 over den 131,068:
    // -131072/131068·65536 = -65538.00006, truncated to -65538.
    let num = ([0, a], [65536, 0], [a, 0], [33_553_408, 33_553_408]);
    for (v, expected) in [
        (cancel, 256 << 16),
        (wrong, -33_553_408),
        (zero, 0),
        (num, -65538),
    ] {
        assert_eq!(checked(v.0, v.1, v.2, v.3), None);
        assert_eq!(wrapping(v.0, v.1, v.2, v.3), Ok(expected));
        assert_eq!(reference(v.0, v.1, v.2, v.3), (Ok(expected), true));
    }
}

/// The three `FixedDiv` refusals, each named (ruling 7).
#[test]
fn each_fixeddiv_refusal_is_named() {
    let (o, a) = ([0, 0], 1 << 30);
    // Saturation of wrapped operands: den = 2^32 + 64 wraps to 64, num = 2^20,
    // (2^20 >> 14) = 64 >= 64 (m_fixed.c:62-63).
    let v = (o, [(1 << 26) + 1, 0], [1 << 14, 0], [0, a]);
    assert_eq!(checked(v.0, v.1, v.2, v.3), None);
    assert_eq!(wrapping(v.0, v.1, v.2, v.3), Err(Saturation));
    // INT_MIN after a wrap: num = 2^30 + 2^30.
    let v = ([0, a], [65536, 0], [a, 0], [1 << 24, 1 << 24]);
    assert_eq!(checked(v.0, v.1, v.2, v.3), None);
    assert_eq!(wrapping(v.0, v.1, v.2, v.3), Err(AbsMin));
    // INT_MIN without a wrap, the one non-wrapped difference from wire 26:
    // num = -2^30 - 2^30 fits, and wire 26 keeps its unsigned-magnitude value.
    let v = ([a, -a], [65536, 0], o, [1 << 24, 1 << 24]);
    assert_eq!(checked(v.0, v.1, v.2, v.3), Some(i32::MIN));
    assert_eq!(wrapping(v.0, v.1, v.2, v.3), Err(AbsMin));
    // den = FixedMul(-2^22, 2^25) = INT_MIN without a wrap; wire 26 gives 128.
    let v = (o, [1 << 25, 0], [65536, 0], [0, -a]);
    assert_eq!(checked(v.0, v.1, v.2, v.3), Some(128));
    assert_eq!(wrapping(v.0, v.1, v.2, v.3), Err(AbsMin));
    // Div2Range is unreachable without INT_MIN: at the largest |num| the guard
    // fails, |den|·2^14 - 1, FixedDiv2 stays in range (m_fixed.c:84-85).
    for den in [1, 2, 3, 7, 100, 65535, 65536, 1 << 20, i32::MAX] {
        let top = (i64::from(den) << 14) - 1;
        let num = i32::try_from(top.min(i64::from(i32::MAX))).unwrap();
        for (n, d) in [(num, den), (-num, den), (num, -den), (-num, -den)] {
            assert!((n.unsigned_abs() >> 14) < d.unsigned_abs());
            assert!(
                Fixed::from_bits(n)
                    .div2_checked(Fixed::from_bits(d))
                    .is_ok()
            );
        }
    }
}

fn record(x: i16, y: i16, angle: i16, kind: i16, options: i16) -> Vec<u8> {
    [x, y, angle, kind, options]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
/// The wire-26 unit-test room: a 1,024 × 256 rectangle, the player at (-256, 0)
/// facing east.
fn base() -> Lumps {
    let c = &cases()[0];
    let mut l = room(c.map);
    let t = &mut l.lumps.iter_mut().find(|(n, _)| n == "THINGS").unwrap().1;
    t.splice(0..0, record(c.position[0], c.position[1], 0, 1, 7));
    l
}
fn level(l: &Lumps, v5: bool) -> Simulation {
    let build = if v5 {
        Simulation::original_player_episode1_v5_level
    } else {
        Simulation::original_player_episode1_v4_level
    };
    build(
        l.map(),
        1,
        Skill::Medium,
        MapContextOptions::default(),
        LevelStart::NewGame,
    )
    .unwrap()
}
fn command(t: u32) -> [TicCommand; 4] {
    let mut c = [TicCommand::default(); 4];
    c[0] = TicCommand {
        forward: 50,
        side: if t % 3 == 0 { 24 } else { 0 },
        angle_turn: if t < 40 { 256 } else { -512 },
        buttons: [weapons::BT_ATTACK, weapons::BT_USE, 0][(t % 3) as usize],
    };
    c
}
/// The bytes of `bytes` that differ from `other`'s.
fn differing(bytes: &[u8], other: &[u8]) -> Vec<usize> {
    assert_eq!(bytes.len(), other.len());
    (0..bytes.len())
        .filter(|i| bytes[*i] != other[*i])
        .collect()
}

/// Ruling 2: DSIM43 is DSIM42 field for field. One state, captured under wire 26
/// and wire 27 after every tic of a run that moves, turns, uses and fires (no
/// intercept wraps in this room), differs in exactly three bytes: the version
/// (offset 4: 42, 43), the `profile` value (offset 82: 26, 27) and the context
/// string's part digit (offset 181: '4', '5'); the paths and lengths around them
/// are equal, and the events are equal.
#[test]
fn dsim43_is_dsim42_but_for_the_three_values_naming_the_wire() {
    let l = base();
    let (mut v4, mut v5) = (level(&l, false), level(&l, true));
    assert!(!v4.world.wrapping_intercept && v5.world.wrapping_intercept);
    for t in 0..=70 {
        if t > 0 {
            v4.advance(command(t)).unwrap();
            v5.advance(command(t)).unwrap();
            assert_eq!(v4.events(), v5.events(), "tic {t}");
        }
        let (b4, b5) = (
            v4.snapshot().canonical_bytes(),
            v5.snapshot().canonical_bytes(),
        );
        assert_eq!(differing(&b4, &b5), [4, 82, 181], "tic {t}");
        assert_eq!(
            (&b4[..8], &b5[4..8]),
            (&b"DSIM*\0\0\0"[..], &[43, 0, 0, 0][..])
        );
        assert_eq!(&b4[63..82], b"\x07\0\0\0profile\x04\0\0\0\0\0\0\0");
        assert_eq!(
            (&b4[82..86], &b5[82..86]),
            (&[26, 0, 0, 0][..], &[27, 0, 0, 0][..])
        );
        assert_eq!(&b4[130..156], b"\x0e\0\0\0context.schema\x1d\0\0\0\0\0\0\0");
        assert_eq!(&b4[156..185], PLAYER_EPISODE1_V4_CONTEXT_SCHEMA);
        assert_eq!(&b5[156..185], PLAYER_EPISODE1_V5_CONTEXT_SCHEMA);
    }
    let (b4, b5) = (
        v4.snapshot().canonical_bytes(),
        v5.snapshot().canonical_bytes(),
    );
    let limits = DiagnosticLimits::default();
    assert_eq!(
        Diagnostic::decode_episode1_v4(&b4, limits)
            .unwrap()
            .version(),
        42
    );
    assert_eq!(
        Diagnostic::decode_episode1_v5(&b5, limits)
            .unwrap()
            .version(),
        43
    );
    assert_eq!(
        Diagnostic::decode_episode1_v4(&b5, limits).err(),
        Some(DiagnosticError::Version(43))
    );
    assert_eq!(
        Diagnostic::decode_episode1_v5(&b4, limits).err(),
        Some(DiagnosticError::Version(42))
    );
    // A version that names one wire and a profile (or context) that names the
    // other is refused.
    for (mut bytes, at, value, v5_reader) in [
        (b5.clone(), 82, 26, true),
        (b4.clone(), 82, 27, false),
        (b5.clone(), 181, b'4', true),
        (b4.clone(), 181, b'5', false),
        (b4.clone(), 4, 43, true),
        (b5.clone(), 4, 42, false),
    ] {
        bytes[at] = value;
        let decoded = if v5_reader {
            Diagnostic::decode_episode1_v5(&bytes, limits)
        } else {
            Diagnostic::decode_episode1_v4(&bytes, limits)
        };
        assert_eq!(decoded.err(), Some(DiagnosticError::Schema), "{at}");
    }
}

/// Wire 27's identity and constructor rule: policy 27, its own context string
/// of wire 26's length, and only the level-start constructors build it.
#[test]
fn wire_27_identity_and_constructor() {
    let v5 = OriginalExecutionProfile::PlayerMonstersEpisode1V5;
    assert_eq!(v5.wire_id(), 27);
    assert_eq!(v5.context_schema(), b"PLAYER-MONSTERS-EPISODE1-5-v1");
    assert_eq!(
        v5.context_schema().len(),
        PLAYER_EPISODE1_V4_CONTEXT_SCHEMA.len()
    );
    assert_eq!(
        Simulation::with_original_registry(
            base().map(),
            player_map_scale::actor_slots(v5),
            OriginalExecutionRegistry::new(v5).unwrap(),
            ActorOptions::default(),
            MapContextOptions::default(),
        )
        .err(),
        Some(PlayerBodyError::Unsupported.into())
    );
    let s = level(&base(), true);
    assert_eq!(s.snapshot().version(), 43);
    assert_eq!(s.level_start_kind(), Some(StartKind::NewGame));
}

/// Ruling 6: all five callers wrap in this step, and only under wire 27: the
/// same vector ("a wrong intercept" above) is refused through a wire-26 world.
#[test]
fn every_caller_wraps_under_wire_27_alone() {
    let l = base();
    let (v4, v5) = (level(&l, false), level(&l, true));
    let v = fx([0, 0]);
    let (d2, s1, d1) = (fx([1 << 30, 0]), fx([1 << 24, 0]), fx([0x7fff_0000; 2]));
    for caller in [
        InterceptCaller::Aim,
        InterceptCaller::ShootLine,
        InterceptCaller::ShootThing,
        InterceptCaller::Slide,
        InterceptCaller::Use,
    ] {
        assert!(wire27_wraps(caller));
        assert_eq!(policy_intercept(&v4.world, caller, v, d2, s1, d1), None);
        assert_eq!(
            policy_intercept(&v5.world, caller, v, d2, s1, d1),
            Some(Fixed::from_bits(-33_553_408))
        );
    }
}

/// The room with its east wall (line 1) stretched to (512, 7872) → (512, -128),
/// 8,000 units long, and a BLOCKMAP of 9 × 63 cells listing every line, so that
/// the Episode 1 load rules (span and line components under 8,192) still admit
/// it. A pistol shot east (range 2,048) crosses the wall: den =
/// FixedMul(-8000·256, ~2^27) ≈ -4,194,304,000 leaves `int`.
fn stretched() -> Lumps {
    let mut l = base();
    let [_, b] = l.line_vertices(0);
    l.set_vertex(b, [512, 7872]);
    let (w, h) = (9_u16, 63_u16);
    let cells = usize::from(w * h);
    let mut words = vec![(-512_i16) as u16, (-128_i16) as u16, w, h];
    words.extend((0..cells).map(|c| (4 + cells + 6 * c) as u16));
    for _ in 0..cells {
        words.extend([0, 0, 1, 2, 3, 0xffff]);
    }
    l.lumps.iter_mut().find(|(n, _)| n == "BLOCKMAP").unwrap().1 =
        words.iter().flat_map(|w| w.to_le_bytes()).collect();
    l
}

/// The E1M8 refusal in a synthetic room: the tic whose pistol shot crosses the
/// stretched wall is refused under wire 26 with `MapScale(Arithmetic)`, the
/// state kept, and completes under wire 27, where the wrapped fraction is
/// negative ("behind source", p_maputl.c:591-592) so the wall is not an
/// intercept: the shot reaches what lies beyond.
#[test]
fn a_wrapping_shot_completes_under_wire_27_where_wire_26_refuses() {
    let l = stretched();
    let (mut v4, mut v5) = (level(&l, false), level(&l, true));
    let mut fire = [TicCommand::default(); 4];
    fire[0].buttons = weapons::BT_ATTACK;
    // The pistol rises for 18 tics (A_Raise, p_pspr.c:429-434) before it fires.
    for t in 0..18 {
        v4.advance(fire).unwrap();
        v5.advance(fire).unwrap();
        assert_eq!(v4.events(), v5.events(), "tic {t}");
    }
    let before = v4.snapshot().canonical_bytes();
    assert_eq!(v4.advance(fire), Err(MapScaleError::Arithmetic.into()));
    assert_eq!(v4.snapshot().canonical_bytes(), before);
    v5.advance(fire).unwrap();
    let trace = v5
        .events()
        .iter()
        .find_map(|e| match e {
            Event::Combat(CombatEvent::Trace { result }) => Some(result.trace.clone()),
            _ => None,
        })
        .unwrap();
    assert!(trace.candidates.contains(&TraceObject::Line(1)));
    assert!(
        trace
            .intercepts
            .iter()
            .all(|i| i.object != TraceObject::Line(1))
    );
    // The same trace against line 1 by the independent transcription: wrapped,
    // and negative.
    let [ox, oy] = trace.origin.map(|v| i64::from(v.bits()));
    let [dx, dy] = trace.delta.map(|v| i64::from(v.bits()));
    let (result, wrapped) = reference(
        [ox, oy],
        [dx, dy],
        [512 << 16, 7872 << 16],
        [0, -8000 << 16],
    );
    assert!(wrapped && result.is_ok_and(|f| f < 0), "{result:?}");
}
